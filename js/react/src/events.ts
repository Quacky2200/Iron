// UI events from the engine (DESIGN D6). Rust hit-tests and calls
// `__iron_ui_event(type, path, data)` with the target's path (target first, then
// its ancestors); this runs React-style capture and bubble phases over the
// elements' `on*` / `on*Capture` props and reports `preventDefault()` back.

import { ContinuousEventPriority, DiscreteEventPriority } from "react-reconciler/constants";

import { type Instance, instances, reconciler, withUpdatePriority } from "./host.ts";

/** Event props for each engine event type. Mouse aliases run alongside the pointer ones. */
const HANDLERS: Record<string, readonly string[]> = {
  pointermove: ["onPointerMove", "onMouseMove"],
  pointerdown: ["onPointerDown", "onMouseDown"],
  pointerup: ["onPointerUp", "onMouseUp"],
  pointerenter: ["onPointerEnter", "onMouseEnter"],
  pointerleave: ["onPointerLeave", "onMouseLeave"],
  click: ["onClick"],
  dblclick: ["onDoubleClick"],
  contextmenu: ["onContextMenu"],
  wheel: ["onWheel"],
};

const NON_BUBBLING: ReadonlySet<string> = new Set(["pointerenter", "pointerleave"]);

// Like React DOM: frequent events render with continuous priority (batched into the
// next tick), the rest synchronously before the call returns.
const CONTINUOUS: ReadonlySet<string> = new Set(["pointermove", "pointerenter", "pointerleave", "wheel"]);

/** Base event, shaped like React's synthetic events. */
export class IronEvent {
  /** 1 capturing, 2 at the target, 3 bubbling. */
  eventPhase = 0;
  currentTarget: Instance | null = null;
  defaultPrevented = false;
  private stopped = false;

  constructor(
    readonly type: string,
    readonly target: Instance,
    readonly bubbles: boolean,
  ) {}

  preventDefault(): void {
    this.defaultPrevented = true;
  }
  stopPropagation(): void {
    this.stopped = true;
  }
  isDefaultPrevented(): boolean {
    return this.defaultPrevented;
  }
  isPropagationStopped(): boolean {
    return this.stopped;
  }
}

/** A mouse pointer event (`onClick`, `onPointerDown`, ...). Coordinates are logical pixels. */
export class IronPointerEvent extends IronEvent {
  readonly clientX: number;
  readonly clientY: number;
  /** 0 primary, 1 middle, 2 secondary, 3 back, 4 forward. */
  readonly button: number;
  /** Held buttons: 1 primary, 2 secondary, 4 middle, 8 back, 16 forward. */
  readonly buttons: number;
  /** Click count. */
  readonly detail: number;
  readonly shiftKey: boolean;
  readonly ctrlKey: boolean;
  readonly altKey: boolean;
  readonly metaKey: boolean;
  readonly pointerType = "mouse";
  readonly pointerId = 1;
  readonly isPrimary = true;

  constructor(type: string, target: Instance, bubbles: boolean, data: readonly number[]) {
    super(type, target, bubbles);
    const [x = 0, y = 0, button = 0, buttons = 0, detail = 0, , , mods = 0] = data;
    this.clientX = x;
    this.clientY = y;
    this.button = button;
    this.buttons = buttons;
    this.detail = detail;
    this.shiftKey = (mods & 1) !== 0;
    this.ctrlKey = (mods & 2) !== 0;
    this.altKey = (mods & 4) !== 0;
    this.metaKey = (mods & 8) !== 0;
  }

  get x(): number {
    return this.clientX;
  }
  get y(): number {
    return this.clientY;
  }
}

/** `onWheel`. Deltas are logical pixels; positive `deltaY` scrolls down. */
export class IronWheelEvent extends IronPointerEvent {
  readonly deltaX: number;
  readonly deltaY: number;
  readonly deltaZ = 0;
  /** Always pixels. */
  readonly deltaMode = 0;

  constructor(type: string, target: Instance, bubbles: boolean, data: readonly number[]) {
    super(type, target, bubbles, data);
    this.deltaX = data[5] ?? 0;
    this.deltaY = data[6] ?? 0;
  }
}

type Handler = (event: IronEvent) => void;

function invoke(node: Instance, names: readonly string[], event: IronEvent, phase: number) {
  const props = node.props as Record<string, unknown>;
  for (const name of names) {
    const handler = props[name];
    if (typeof handler !== "function") continue;
    event.currentTarget = node;
    event.eventPhase = phase;
    try {
      (handler as Handler)(event);
    } catch (e) {
      // Like a DOM listener: report, and keep dispatching.
      const err = e instanceof Error ? e : new Error(String(e));
      console.error(`[react] error in ${name}: ${err.message}${err.stack ? `\n${err.stack}` : ""}`);
    }
  }
}

/**
 * Dispatches an engine event. `path` is the target then its ancestors; `data` is
 * `[x, y, button, buttons, detail, deltaX, deltaY, modifiers]`. Returns whether a
 * handler called `preventDefault()`.
 */
export function dispatchUiEvent(type: string, path: readonly number[], data: readonly number[]): boolean {
  const names = HANDLERS[type];
  const target = instances.get(path[0] ?? -1);
  if (!names || !target) return false;
  const nodes = path.map((id) => instances.get(id)).filter((n): n is Instance => n !== undefined);
  const bubbles = !NON_BUBBLING.has(type);
  const event =
    type === "wheel"
      ? new IronWheelEvent(type, target, bubbles, data)
      : new IronPointerEvent(type, target, bubbles, data);
  const capture = names.map((n) => `${n}Capture`);
  const priority = CONTINUOUS.has(type) ? ContinuousEventPriority : DiscreteEventPriority;

  withUpdatePriority(priority, () => {
    // Capture: outermost ancestor down to the target. Stopping finishes the current
    // element's handlers, like React.
    for (let i = nodes.length - 1; i >= 0 && !event.isPropagationStopped(); i--) {
      invoke(nodes[i]!, capture, event, i === 0 ? 2 : 1);
    }
    const last = bubbles ? nodes.length : 1;
    for (let i = 0; i < last && !event.isPropagationStopped(); i++) {
      invoke(nodes[i]!, names, event, i === 0 ? 2 : 3);
    }
  });
  event.currentTarget = null;
  event.eventPhase = 0;
  // Discrete updates commit now, so the next frame shows them.
  reconciler.flushSyncWork();
  return event.defaultPrevented;
}

globalThis.__iron_ui_event = dispatchUiEvent;
