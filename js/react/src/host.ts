// react-reconciler host config: turns React's tree mutations into UI ops.
//
// Ops are collected during render/commit and sent to the engine with ONE
// `__iron_commit` call per React commit (D1 in TODO.md). Rust owns the real
// tree; JS only keeps node IDs and the last props needed for diffing.

import { op } from "@iron/engine";
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
import { attrDiff, attrEntries } from "./attrs.ts";
import { type IronStyle, styleDiff, styleEntries } from "./style.ts";

/** Host element types; `text` and `span` are rendered by the `Text` and `Span` components. */
export type ElementType = "box" | "text" | "span" | "input" | "textarea";

export interface Props {
  style?: IronStyle;
  children?: unknown;
}

/** An element. Refs (`useRef<IronElement>(null)`) point at these. */
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
   * Scrolls the element's content to (`left`, `top`), clamped to its range. Works on
   * `overflow: hidden` too. Applied next frame; `onScroll` reports the result.
   */
  scrollTo(left: number, top: number): void {
    op("scrollTo", this.id, left, top);
  }

  /** Scrolls the element's content by (`dx`, `dy`). */
  scrollBy(dx: number, dy: number): void {
    op("scrollBy", this.id, dx, dy);
  }

  /** Scrolls the element's scroll containers just enough to show it. */
  scrollIntoView(): void {
    op("scrollIntoView", this.id);
  }
}

export interface Container {
  readonly id: typeof ROOT_ID;
  /** Root children, so `clearContainer` can remove them. */
  readonly children: Set<number>;
}

/** A run of characters (React string children). */
export interface TextInstance {
  readonly id: number;
  readonly type: "#text";
  text: string;
}

type Node = Instance | TextInstance;

/** Live elements by ID, so engine events can find their props. */
export const instances = new Map<number, Instance>();

/** Ops waiting for the next commit. Exposed for tests. */
export const pending: Op[] = [];

// IDs are never reused within a session (M2-REC-05). 0 is the root container.
let nextId = 1;

/** Sends pending ops now (outside a React commit). */
export function flush() {
  if (pending.length === 0) return;
  const ops = pending.splice(0, pending.length);
  __iron_commit(ops);
}

const ELEMENT_TYPES: ReadonlySet<string> = new Set(["box", "text", "span", "input", "textarea", "image", "list", "dialog"]);

let currentUpdatePriority: number = NoEventPriority;

// React 19 needs a context to expose form/transition status; Iron has no forms.
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
  rendererPackageName: "@iron/react",
  extraDevToolsConfig: null,
  // Used by React's dev build to replay logs from server components; just forward them.
  bindToConsole(methodName: string, args: unknown[], badgeName: string) {
    const log = (console as unknown as Record<string, (...a: unknown[]) => void>)[methodName] ?? console.log;
    return log.bind(console, `[${badgeName}]`, ...args);
  },

  createInstance(type: string, props: Props): Instance {
    if (!ELEMENT_TYPES.has(type)) {
      throw new Error(`<${type}> is not an Iron element; available: ${[...ELEMENT_TYPES].join(", ")}`);
    }
    const id = nextId++;
    pending.push([CREATE, id, type]);
    const entries = styleEntries(props.style);
    if (entries.length > 0) pending.push([SET_STYLE, id, ...entries]);
    const attrs = attrEntries(props as Record<string, unknown>);
    if (attrs.length > 0) pending.push([SET_ATTRS, id, ...attrs]);
    const instance = new Instance(id, type as ElementType, props);
    instances.set(id, instance);
    return instance;
  },

  createTextInstance(text: string): TextInstance {
    const id = nextId++;
    pending.push([CREATE_TEXT, id, text]);
    return { id, type: "#text", text };
  },
  commitTextUpdate(instance: TextInstance, _oldText: string, newText: string) {
    instance.text = newText;
    pending.push([SET_TEXT, instance.id, newText]);
  },

  appendInitialChild(parent: Instance, child: Node) {
    pending.push([APPEND_CHILD, parent.id, child.id]);
  },
  appendChild(parent: Instance, child: Node) {
    pending.push([APPEND_CHILD, parent.id, child.id]);
  },
  appendChildToContainer(container: Container, child: Node) {
    container.children.add(child.id);
    pending.push([APPEND_CHILD, ROOT_ID, child.id]);
  },
  insertBefore(parent: Instance, child: Node, before: Node) {
    pending.push([INSERT_BEFORE, parent.id, child.id, before.id]);
  },
  insertInContainerBefore(container: Container, child: Node, before: Node) {
    container.children.add(child.id);
    pending.push([INSERT_BEFORE, ROOT_ID, child.id, before.id]);
  },
  removeChild(parent: Instance, child: Node) {
    pending.push([REMOVE_CHILD, parent.id, child.id]);
  },
  removeChildFromContainer(container: Container, child: Node) {
    container.children.delete(child.id);
    pending.push([REMOVE_CHILD, ROOT_ID, child.id]);
  },
  clearContainer(container: Container) {
    for (const id of container.children) pending.push([REMOVE_CHILD, ROOT_ID, id]);
    container.children.clear();
  },

  commitUpdate(instance: Instance, _type: string, prevProps: Props, nextProps: Props) {
    const value = (nextProps as { value?: unknown }).value;
    if (value !== undefined && value !== null) instance.value = String(value);
    const changes = styleDiff(prevProps.style, nextProps.style);
    if (changes.length > 0) pending.push([SET_STYLE, instance.id, ...changes]);
    const attrs = attrDiff(prevProps as Record<string, unknown>, nextProps as Record<string, unknown>);
    if (attrs.length > 0) pending.push([SET_ATTRS, instance.id, ...attrs]);
    instance.props = nextProps;
  },

  // Suspense hides and reveals subtrees with these.
  hideInstance(instance: Instance) {
    pending.push([SET_STYLE, instance.id, "display", "none"]);
  },
  unhideInstance(instance: Instance, props: Props) {
    pending.push([SET_STYLE, instance.id, "display", props.style?.display ?? null]);
  },
  hideTextInstance(instance: TextInstance) {
    pending.push([SET_TEXT, instance.id, ""]);
  },
  unhideTextInstance(instance: TextInstance, text: string) {
    instance.text = text;
    pending.push([SET_TEXT, instance.id, text]);
  },

  finalizeInitialChildren: () => false,
  shouldSetTextContent: () => false,
  getRootHostContext: () => ({}),
  getChildHostContext: (parent: object) => parent,
  getPublicInstance: (instance: Instance) => instance,
  prepareForCommit: () => null,
  resetAfterCommit: () => flush(),
  preparePortalMount: () => {},
  detachDeletedInstance(node: Node) {
    instances.delete(node.id);
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

  // Transitions and forms (unused by Iron).
  NotPendingTransition: null,
  HostTransitionContext: HostTransitionContext as never,
  resetFormInstance: () => {},
  shouldAttemptEagerTransition: () => false,
  trackSchedulerEvent: () => {},
  resolveEventType: () => null,
  resolveEventTimeStamp: () => -1.1,
  requestPostPaintCallback: () => {},

  // Suspending commits (for loading resources); Iron never suspends a commit.
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
  pending.length = 0;
  instances.clear();
  nextId = 1;
}
