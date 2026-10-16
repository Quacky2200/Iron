// react-reconciler host config: turns React's tree mutations into UI ops.
//
// Ops are collected during render/commit and sent to the engine with ONE
// `__iron_commit` call per React commit (D1 in TODO.md). Rust owns the real
// tree; JS only keeps node IDs and the last props needed for diffing.

import { createContext } from "react";
import createReconciler from "react-reconciler";
import { DefaultEventPriority, NoEventPriority } from "react-reconciler/constants";

import {
  APPEND_CHILD,
  CREATE,
  INSERT_BEFORE,
  type Op,
  REMOVE_CHILD,
  ROOT_ID,
  SET_STYLE,
} from "./opcodes.ts";
import { type IronStyle, styleDiff, styleEntries } from "./style.ts";

export type ElementType = "box";

export interface Props {
  style?: IronStyle;
  children?: unknown;
}

export interface Instance {
  readonly id: number;
  readonly type: ElementType;
  props: Props;
}

export interface Container {
  readonly id: typeof ROOT_ID;
  /** Root children, so `clearContainer` can remove them. */
  readonly children: Set<number>;
}

type TextInstance = never;

/** Ops waiting for the next commit. Exposed for tests. */
export const pending: Op[] = [];

// IDs are never reused within a session (M2-REC-05). 0 is the root container.
let nextId = 1;

function flush() {
  if (pending.length === 0) return;
  const ops = pending.splice(0, pending.length);
  __iron_commit(ops);
}

const ELEMENT_TYPES: ReadonlySet<string> = new Set(["box"]);

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
    return { id, type: type as ElementType, props };
  },

  createTextInstance(text: string): TextInstance {
    throw new Error(
      `Text isn't supported yet (arrives in M3); got ${JSON.stringify(text)}. Only <box> elements can be rendered.`,
    );
  },

  appendInitialChild(parent: Instance, child: Instance) {
    pending.push([APPEND_CHILD, parent.id, child.id]);
  },
  appendChild(parent: Instance, child: Instance) {
    pending.push([APPEND_CHILD, parent.id, child.id]);
  },
  appendChildToContainer(container: Container, child: Instance) {
    container.children.add(child.id);
    pending.push([APPEND_CHILD, ROOT_ID, child.id]);
  },
  insertBefore(parent: Instance, child: Instance, before: Instance) {
    pending.push([INSERT_BEFORE, parent.id, child.id, before.id]);
  },
  insertInContainerBefore(container: Container, child: Instance, before: Instance) {
    container.children.add(child.id);
    pending.push([INSERT_BEFORE, ROOT_ID, child.id, before.id]);
  },
  removeChild(parent: Instance, child: Instance) {
    pending.push([REMOVE_CHILD, parent.id, child.id]);
  },
  removeChildFromContainer(container: Container, child: Instance) {
    container.children.delete(child.id);
    pending.push([REMOVE_CHILD, ROOT_ID, child.id]);
  },
  clearContainer(container: Container) {
    for (const id of container.children) pending.push([REMOVE_CHILD, ROOT_ID, id]);
    container.children.clear();
  },

  commitUpdate(instance: Instance, _type: string, prevProps: Props, nextProps: Props) {
    const changes = styleDiff(prevProps.style, nextProps.style);
    if (changes.length > 0) pending.push([SET_STYLE, instance.id, ...changes]);
    instance.props = nextProps;
  },

  // Suspense hides and reveals subtrees with these.
  hideInstance(instance: Instance) {
    pending.push([SET_STYLE, instance.id, "display", "none"]);
  },
  unhideInstance(instance: Instance, props: Props) {
    pending.push([SET_STYLE, instance.id, "display", props.style?.display ?? null]);
  },
  hideTextInstance() {},
  unhideTextInstance() {},

  finalizeInitialChildren: () => false,
  shouldSetTextContent: () => false,
  getRootHostContext: () => ({}),
  getChildHostContext: (parent: object) => parent,
  getPublicInstance: (instance: Instance) => instance,
  prepareForCommit: () => null,
  resetAfterCommit: () => flush(),
  preparePortalMount: () => {},
  detachDeletedInstance: () => {},
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

/** Resets ID allocation and pending ops. Only for tests. */
export function resetForTests() {
  pending.length = 0;
  nextId = 1;
}
