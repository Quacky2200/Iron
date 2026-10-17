// UI events from the engine (DESIGN D6). Rust hit-tests and calls
// `__iron_ui_event(type, path, data)` with the target's path (target first, then
// its ancestors); this runs React-style capture and bubble phases over the
// elements' `on*` / `on*Capture` props and reports `preventDefault()` back.

import { ContinuousEventPriority, DiscreteEventPriority } from "react-reconciler/constants";

import { flush, type Instance, instances, pending, reconciler, withUpdatePriority } from "./host.ts";
import { SET_ATTRS } from "./opcodes.ts";

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
  keydown: ["onKeyDown"],
  keyup: ["onKeyUp"],
  // React's onFocus/onBlur bubble, like the DOM's focusin/focusout.
  focusin: ["onFocus"],
  focusout: ["onBlur"],
  scroll: ["onScroll"],
  // React's onChange fires on every edit, like the DOM's `input` event.
  input: ["onChange", "onInput"],
  // Dialogs (Escape, backdrop clicks): `cancel` can be prevented, then `close`.
  cancel: ["onCancel"],
  close: ["onClose"],
};

// `scroll` doesn't bubble in React 17+ either.
const NON_BUBBLING: ReadonlySet<string> = new Set(["pointerenter", "pointerleave", "scroll", "cancel", "close"]);

// Like React DOM: frequent events render with continuous priority (batched into the
// next tick), the rest synchronously before the call returns.
const CONTINUOUS: ReadonlySet<string> = new Set(["pointermove", "pointerenter", "pointerleave", "wheel", "scroll"]);

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
    const m = modifiers(mods);
    this.shiftKey = m.shiftKey;
    this.ctrlKey = m.ctrlKey;
    this.altKey = m.altKey;
    this.metaKey = m.metaKey;
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

/** Modifier keys from the data array's bitmask. */
function modifiers(bits: number) {
  return { shiftKey: (bits & 1) !== 0, ctrlKey: (bits & 2) !== 0, altKey: (bits & 4) !== 0, metaKey: (bits & 8) !== 0 };
}

/** `onKeyDown` / `onKeyUp`, sent to the focused element. */
export class IronKeyboardEvent extends IronEvent {
  /** What the key means in the current layout: `"a"`, `"A"`, `"Enter"`, `" "`. */
  readonly key: string;
  /** The physical key, layout independent: `"KeyA"`, `"Enter"`, `"Space"`. */
  readonly code: string;
  /** Held down and auto-repeating. */
  readonly repeat: boolean;
  readonly shiftKey: boolean;
  readonly ctrlKey: boolean;
  readonly altKey: boolean;
  readonly metaKey: boolean;

  constructor(type: string, target: Instance, data: readonly number[], strings: readonly string[]) {
    super(type, target, true);
    this.key = strings[0] ?? "";
    this.code = strings[1] ?? "";
    this.repeat = data[8] === 1;
    const m = modifiers(data[7] ?? 0);
    this.shiftKey = m.shiftKey;
    this.ctrlKey = m.ctrlKey;
    this.altKey = m.altKey;
    this.metaKey = m.metaKey;
  }
}

/** `onFocus` / `onBlur`. */
export class IronFocusEvent extends IronEvent {
  /** For `onFocus`, the element that lost focus; for `onBlur`, the one gaining it. */
  readonly relatedTarget: Instance | null;

  constructor(type: string, target: Instance, data: readonly number[]) {
    super(type, target, true);
    this.relatedTarget = instances.get(data[9] ?? 0) ?? null;
  }
}

/** `onChange` / `onInput` on a text field. `target.value` (or `value`) is the new text. */
export class IronInputEvent extends IronEvent {
  readonly value: string;

  constructor(type: string, target: Instance, strings: readonly string[]) {
    super(type, target, true);
    this.value = strings[0] ?? "";
  }
}

/** `onScroll`: the element's new scroll position and sizes (logical pixels). */
export class IronScrollEvent extends IronEvent {
  readonly scrollLeft: number;
  readonly scrollTop: number;
  /** Content size. */
  readonly scrollWidth: number;
  readonly scrollHeight: number;
  /** Visible (padding box) size. */
  readonly clientWidth: number;
  readonly clientHeight: number;

  constructor(type: string, target: Instance, data: readonly number[]) {
    super(type, target, false);
    this.scrollLeft = data[10] ?? 0;
    this.scrollTop = data[11] ?? 0;
    this.scrollWidth = data[12] ?? 0;
    this.scrollHeight = data[13] ?? 0;
    this.clientWidth = data[14] ?? 0;
    this.clientHeight = data[15] ?? 0;
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

function createEvent(
  type: string,
  target: Instance,
  bubbles: boolean,
  data: readonly number[],
  strings: readonly string[],
): IronEvent {
  switch (type) {
    case "wheel":
      return new IronWheelEvent(type, target, bubbles, data);
    case "keydown":
    case "keyup":
      return new IronKeyboardEvent(type, target, data, strings);
    case "focusin":
    case "focusout":
      return new IronFocusEvent(type, target, data);
    case "scroll":
      return new IronScrollEvent(type, target, data);
    case "input":
      return new IronInputEvent(type, target, strings);
    default:
      return new IronPointerEvent(type, target, bubbles, data);
  }
}

/**
 * Dispatches an engine event. `path` is the target then its ancestors; `data` is
 * `[x, y, button, buttons, detail, deltaX, deltaY, modifiers, repeat, related,
 * scrollLeft, scrollTop, scrollWidth, scrollHeight, clientWidth, clientHeight]` and
 * `strings` is `[key, code]` for keyboard events and `[value]` for input events.
 * Returns whether a handler called
 * `preventDefault()`.
 */
export function dispatchUiEvent(
  type: string,
  path: readonly number[],
  data: readonly number[],
  strings: readonly string[] = [],
): boolean {
  const names = HANDLERS[type];
  const target = instances.get(path[0] ?? -1);
  if (!names || !target) return false;
  const nodes = path.map((id) => instances.get(id)).filter((n): n is Instance => n !== undefined);
  const bubbles = !NON_BUBBLING.has(type);
  const event = createEvent(type, target, bubbles, data, strings);
  // Like the DOM, the field already holds the new text when handlers run.
  if (event instanceof IronInputEvent) target.value = event.value;
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
  if (event instanceof IronInputEvent) restoreControlledValue(target);
  return event.defaultPrevented;
}

/**
 * A controlled field (`value` prop) whose handler didn't accept the edit goes back to
 * its `value`, like React DOM; otherwise the engine's text would drift from the prop.
 */
function restoreControlledValue(target: Instance) {
  const controlled = (target.props as { value?: unknown }).value;
  if (controlled === undefined || controlled === null) return;
  const value = String(controlled);
  if (target.value === value) return;
  target.value = value;
  pending.push([SET_ATTRS, target.id, "value", value]);
  flush();
}

globalThis.__iron_ui_event = dispatchUiEvent;
