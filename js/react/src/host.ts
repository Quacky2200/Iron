// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// react-reconciler host config: turns React's tree mutations into UI ops.
//
// Ops are collected during render/commit and sent to the engine with ONE
// `__platypus_commit` call per React commit (D1 in TODO.md). Rust owns the real
// tree; JS only keeps node IDs and the last props needed for diffing.

import { op } from "@platypus/engine";
import { createContext } from "react";
import createReconciler from "react-reconciler";
import { DefaultEventPriority, NoEventPriority } from "react-reconciler/constants";

import {
  APPEND_CHILD,
  CREATE,
  CREATE_TEXT,
  INSERT_BEFORE,
  type Op,
  REMOVE_CHILD,
  ROOT_ID,
  SET_ATTRS,
  SET_STYLE,
  SET_TEXT,
} from "./opcodes.ts";
import { attrDiff, attrEntries, SVG_TYPES } from "./attrs.ts";
import { type PlatypusStyle, styleDiff, styleEntries } from "./style.ts";

/** Host element types; `text` and `span` are rendered by the `Text` and `Span` components. */
export type ElementType = string;

export interface Props {
  style?: PlatypusStyle;
  children?: unknown;
}

/** An element. Refs (`useRef<PlatypusElement>(null)`) point at these. */
export class Instance {
  constructor(
    readonly id: number,
    readonly type: ElementType,
    public props: Props,
  ) {
    const p = props as { value?: unknown; defaultValue?: unknown };
    if (type === "input" || type === "textarea") this.value = String(p.value ?? p.defaultValue ?? "");
  }

  /** A text field's current text, like the DOM's `input.value` (`e.target.value` in `onChange`). */
  value?: string;

  /** Gives the element keyboard focus, if it is focusable (has `tabIndex`). Applied next frame. */
  focus(): void {
    op("focus", this.id);
  }

  /** Removes keyboard focus from the element, if it has it. Applied next frame. */
  blur(): void {
    op("blur", this.id);
  }

  /**
   * Sends the pointer's moves and its release to this element wherever the pointer goes
   * (drag a slider or window), until the button is released. Call it from `onPointerDown`;
   * it applies on the next frame, followed by `onGotPointerCapture`. Platypus has one pointer,
   * so `pointerId` is ignored.
   */
  setPointerCapture(_pointerId?: number): void {
    op("setPointerCapture", this.id);
  }

  /** Ends this element's pointer capture (`onLostPointerCapture` follows). */
  releasePointerCapture(_pointerId?: number): void {
    op("releasePointerCapture", this.id);
  }

  /** Whether this element has the pointer captured (as of the last capture event). */
  hasPointerCapture(_pointerId?: number): boolean {
    return pointerCapture.element === this;
  }

  /**
   * Scrolls the element's content to (`left`, `top`), clamped to its range, or as the
   * DOM's `{ left, top, behavior }` (an axis left out stays put). Works on
   * `overflow: hidden` too. Applied next frame; `onScroll` reports the result.
   * `behavior: "smooth"` (or `scroll-behavior: smooth` on the element) eases over 250 to
   * 500 ms.
   */
  scrollTo(left: number | ScrollToOptions, top?: number): void {
    const o = typeof left === "object" ? left : { left, top };
    op("scrollTo", this.id, o.left ?? null, o.top ?? null, o.behavior ?? "auto");
  }

  /** Scrolls the element's content by (`dx`, `dy`), or the DOM's `{ left, top, behavior }`. */
  scrollBy(dx: number | ScrollToOptions, dy?: number): void {
    const o = typeof dx === "object" ? dx : { left: dx, top: dy };
    op("scrollBy", this.id, o.left ?? 0, o.top ?? 0, o.behavior ?? "auto");
  }

  /**
   * Scrolls the element's scroll containers to show it, as in the DOM: `true` or nothing
   * puts it at the top (`block: "start"`), `false` at the bottom, or say where with
   * `{ block, inline }` (`"start"`, `"center"`, `"end"` or `"nearest"`, which scrolls only
   * as far as needed; `inline` defaults to `"nearest"`). `behavior: "smooth"` eases there
   * (as does `"auto"` in a container with `scroll-behavior: smooth`).
   */
  scrollIntoView(arg: boolean | ScrollIntoViewOptions = true): void {
    const {
      block = "start",
      inline = "nearest",
      behavior = "auto",
    } = typeof arg === "boolean" ? { block: arg ? ("start" as const) : ("end" as const) } : arg;
    op("scrollIntoView", this.id, block, inline, behavior);
  }
}

/** Where `scrollIntoView` puts an element, per axis (the DOM's `ScrollLogicalPosition`). */
export type ScrollLogicalPosition = "start" | "center" | "end" | "nearest";

/** How a script's scroll moves: `auto` follows the container's `scroll-behavior`. */
export type ScrollBehavior = "auto" | "instant" | "smooth";

export interface ScrollIntoViewOptions {
  block?: ScrollLogicalPosition;
  inline?: ScrollLogicalPosition;
  behavior?: ScrollBehavior;
}

export interface ScrollToOptions {
  left?: number;
  top?: number;
  behavior?: ScrollBehavior;
}

export interface Container {
  readonly id: typeof ROOT_ID;
  /** The engine UI tree it renders into: 0 is the screen, others are world panels. */
  readonly target: number;
  /** Root children, so `clearContainer` can remove them. */
  readonly children: Set<number>;
  /** Ops waiting for this root's next commit. */
  readonly pending: Op[];
}

/** A run of characters (React string children). */
export interface TextInstance {
  readonly id: number;
  readonly type: "#text";
  text: string;
}

type Node = Instance | TextInstance;

/** The element holding pointer capture, kept by the capture events (events.ts). */
export const pointerCapture: { element: Instance | null } = { element: null };

/** Live elements by ID, so engine events can find their props. */
export const instances = new Map<number, Instance>();

/** The screen's root container; world panels get their own (`containerFor`). */
export const screenContainer: Container = { id: ROOT_ID, target: 0, children: new Set(), pending: [] };
const containers = new Map<number, Container>([[0, screenContainer]]);

/** The container for an engine UI tree (0: the screen). */
export function containerFor(target: number): Container {
  let c = containers.get(target);
  if (!c) {
    c = { id: ROOT_ID, target, children: new Set(), pending: [] };
    containers.set(target, c);
  }
  return c;
}

/** Ops waiting for the screen's next commit. Exposed for tests. */
export const pending: Op[] = screenContainer.pending;

/** Which container each node belongs to, so its ops go to the right tree. */
const owners = new Map<number, Container>();

/** The op queue for a node's tree. */
export function queueFor(id: number): Op[] {
  return (owners.get(id) ?? screenContainer).pending;
}

// IDs are never reused within a session (M2-REC-05). 0 is the root container.
let nextId = 1;

/** Sends a container's pending ops as one commit to its tree. */
function flushContainer(c: Container) {
  if (c.pending.length === 0) return;
  const ops = c.pending.splice(0, c.pending.length);
  if (c.target === 0) __platypus_commit(ops);
  else __platypus_commit(ops, c.target);
}

/** Sends pending ops now (outside a React commit), each root to its own tree. */
export function flush() {
  for (const c of containers.values()) flushContainer(c);
}

const ELEMENT_TYPES: ReadonlySet<string> = new Set([
  "box",
  "text",
  "span",
  "input",
  "textarea",
  "image",
  "list",
  "dialog",
  ...SVG_TYPES,
]);

let currentUpdatePriority: number = NoEventPriority;

// React 19 needs a context to expose form/transition status; Platypus has no forms.
const HostTransitionContext = createContext<null>(null);

export const reconciler = createReconciler({
  supportsMutation: true,
  supportsPersistence: false,
  supportsHydration: false,
  isPrimaryRenderer: true,
  supportsMicrotasks: true,
  scheduleMicrotask: (fn: () => unknown) => queueMicrotask(() => void fn()),
  scheduleTimeout: (fn: (...args: unknown[]) => unknown, delay?: number) => setTimeout(fn, delay),
  cancelTimeout: (id: number) => clearTimeout(id),
  noTimeout: -1 as const,
  rendererVersion: "0.1.0",
  rendererPackageName: "@platypus/react",
  extraDevToolsConfig: null,
  // Used by React's dev build to replay logs from server components; just forward them.
  bindToConsole(methodName: string, args: unknown[], badgeName: string) {
    const log = (console as unknown as Record<string, (...a: unknown[]) => void>)[methodName] ?? console.log;
    return log.bind(console, `[${badgeName}]`, ...args);
  },

  createInstance(type: string, props: Props, root: Container): Instance {
    if (!ELEMENT_TYPES.has(type)) {
      throw new Error(`<${type}> is not a Platypus element; available: ${[...ELEMENT_TYPES].join(", ")}`);
    }
    const id = nextId++;
    owners.set(id, root);
    const pending = root.pending;
    pending.push([CREATE, id, type]);
    const entries = styleEntries(props.style);
    if (entries.length > 0) pending.push([SET_STYLE, id, ...entries]);
    const attrs = attrEntries(props as Record<string, unknown>, type);
    if (attrs.length > 0) pending.push([SET_ATTRS, id, ...attrs]);
    const instance = new Instance(id, type as ElementType, props);
    instances.set(id, instance);
    return instance;
  },

  createTextInstance(text: string, root: Container): TextInstance {
    const id = nextId++;
    owners.set(id, root);
    root.pending.push([CREATE_TEXT, id, text]);
    return { id, type: "#text", text };
  },
  commitTextUpdate(instance: TextInstance, _oldText: string, newText: string) {
    instance.text = newText;
    queueFor(instance.id).push([SET_TEXT, instance.id, newText]);
  },

  appendInitialChild(parent: Instance, child: Node) {
    queueFor(parent.id).push([APPEND_CHILD, parent.id, child.id]);
  },
  appendChild(parent: Instance, child: Node) {
    queueFor(parent.id).push([APPEND_CHILD, parent.id, child.id]);
  },
  appendChildToContainer(container: Container, child: Node) {
    container.children.add(child.id);
    container.pending.push([APPEND_CHILD, ROOT_ID, child.id]);
  },
  insertBefore(parent: Instance, child: Node, before: Node) {
    queueFor(parent.id).push([INSERT_BEFORE, parent.id, child.id, before.id]);
  },
  insertInContainerBefore(container: Container, child: Node, before: Node) {
    container.children.add(child.id);
    container.pending.push([INSERT_BEFORE, ROOT_ID, child.id, before.id]);
  },
  removeChild(parent: Instance, child: Node) {
    queueFor(parent.id).push([REMOVE_CHILD, parent.id, child.id]);
  },
  removeChildFromContainer(container: Container, child: Node) {
    container.children.delete(child.id);
    container.pending.push([REMOVE_CHILD, ROOT_ID, child.id]);
  },
  clearContainer(container: Container) {
    for (const id of container.children) container.pending.push([REMOVE_CHILD, ROOT_ID, id]);
    container.children.clear();
  },

  commitUpdate(instance: Instance, type: string, prevProps: Props, nextProps: Props) {
    const value = (nextProps as { value?: unknown }).value;
    if (value !== undefined && value !== null) instance.value = String(value);
    const changes = styleDiff(prevProps.style, nextProps.style);
    if (changes.length > 0) queueFor(instance.id).push([SET_STYLE, instance.id, ...changes]);
    const attrs = attrDiff(prevProps as Record<string, unknown>, nextProps as Record<string, unknown>, type);
    if (attrs.length > 0) queueFor(instance.id).push([SET_ATTRS, instance.id, ...attrs]);
    instance.props = nextProps;
  },

  // Suspense hides and reveals subtrees with these.
  hideInstance(instance: Instance) {
    queueFor(instance.id).push([SET_STYLE, instance.id, "display", "none"]);
  },
  unhideInstance(instance: Instance, props: Props) {
    queueFor(instance.id).push([SET_STYLE, instance.id, "display", props.style?.display ?? null]);
  },
  hideTextInstance(instance: TextInstance) {
    queueFor(instance.id).push([SET_TEXT, instance.id, ""]);
  },
  unhideTextInstance(instance: TextInstance, text: string) {
    instance.text = text;
    queueFor(instance.id).push([SET_TEXT, instance.id, text]);
  },

  finalizeInitialChildren: () => false,
  shouldSetTextContent: () => false,
  getRootHostContext: () => ({}),
  getChildHostContext: (parent: object) => parent,
  getPublicInstance: (instance: Instance) => instance,
  prepareForCommit: () => null,
  resetAfterCommit: (container: Container) => flushContainer(container),
  preparePortalMount: () => {},
  detachDeletedInstance(node: Node) {
    instances.delete(node.id);
    owners.delete(node.id);
  },
  getInstanceFromNode: () => null,
  beforeActiveInstanceBlur: () => {},
  afterActiveInstanceBlur: () => {},
  prepareScopeUpdate: () => {},
  getInstanceFromScope: () => null,

  // Update priorities.
  setCurrentUpdatePriority(priority: number) {
    currentUpdatePriority = priority;
  },
  getCurrentUpdatePriority: () => currentUpdatePriority,
  resolveUpdatePriority: () =>
    currentUpdatePriority !== NoEventPriority ? currentUpdatePriority : DefaultEventPriority,

  // Transitions and forms (unused by Platypus).
  NotPendingTransition: null,
  HostTransitionContext: HostTransitionContext as never,
  resetFormInstance: () => {},
  shouldAttemptEagerTransition: () => false,
  trackSchedulerEvent: () => {},
  resolveEventType: () => null,
  resolveEventTimeStamp: () => -1.1,
  requestPostPaintCallback: () => {},

  // Suspending commits (for loading resources); Platypus never suspends a commit.
  maySuspendCommit: () => false,
  maySuspendCommitOnUpdate: () => false,
  maySuspendCommitInSyncRender: () => false,
  preloadInstance: () => true,
  startSuspendingCommit: () => null,
  suspendInstance: () => {},
  suspendOnActiveViewTransition: () => {},
  waitForCommitToBeReady: () => null,
  getSuspendedCommitReason: () => null,
});

// Lets React's devtools hook see this renderer. Fast Refresh (dev builds, refresh.ts)
// finds the roots to refresh through it; without the hook this does nothing.
reconciler.injectIntoDevTools();

/** Runs `fn` with React updates at `priority` (an event priority). */
export function withUpdatePriority(priority: number, fn: () => void) {
  const previous = currentUpdatePriority;
  currentUpdatePriority = priority;
  try {
    fn();
  } finally {
    currentUpdatePriority = previous;
  }
}

/** Resets ID allocation and pending ops. Only for tests. */
export function resetForTests() {
  for (const c of containers.values()) c.pending.length = 0;
  for (const [t] of containers) if (t !== 0) containers.delete(t);
  screenContainer.children.clear();
  owners.clear();
  instances.clear();
  nextId = 1;
}
