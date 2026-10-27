// SPDX-License-Identifier: Apache-2.0

// UI events from the engine (DESIGN D6). Rust hit-tests and calls
// `__platypus_ui_event(type, path, data)` with the target's path (target first, then
// its ancestors); this runs React-style capture and bubble phases over the
// elements' `on*` / `on*Capture` props and reports `preventDefault()` back.

import { op } from "@platypus/engine";
import { ContinuousEventPriority, DiscreteEventPriority } from "react-reconciler/constants";

import {
  flush,
  type Instance,
  instances,
  pointerCapture,
  queueFor,
  reconciler,
  withUpdatePriority,
} from "./host.ts";
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
  transitionrun: ["onTransitionRun"],
  transitionstart: ["onTransitionStart"],
  transitionend: ["onTransitionEnd"],
  transitioncancel: ["onTransitionCancel"],
  animationstart: ["onAnimationStart"],
  animationiteration: ["onAnimationIteration"],
  animationend: ["onAnimationEnd"],
  animationcancel: ["onAnimationCancel"],
  gotpointercapture: ["onGotPointerCapture"],
  lostpointercapture: ["onLostPointerCapture"],
  dragstart: ["onDragStart"],
  drag: ["onDrag"],
  dragenter: ["onDragEnter"],
  dragover: ["onDragOver"],
  dragleave: ["onDragLeave"],
  drop: ["onDrop"],
  dragend: ["onDragEnd"],
};

// `scroll` doesn't bubble in React 17+ either.
const NON_BUBBLING: ReadonlySet<string> = new Set(["pointerenter", "pointerleave", "scroll", "cancel", "close"]);

// Like React DOM: frequent events render with continuous priority (batched into the
// next tick), the rest synchronously before the call returns.
const CONTINUOUS: ReadonlySet<string> = new Set(["pointermove", "pointerenter", "pointerleave", "wheel", "scroll", "drag", "dragover"]);

/** Base event, shaped like React's synthetic events. */
export class PlatypusEvent {
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
export class PlatypusPointerEvent extends PlatypusEvent {
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
export class PlatypusWheelEvent extends PlatypusPointerEvent {
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
export class PlatypusKeyboardEvent extends PlatypusEvent {
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
export class PlatypusFocusEvent extends PlatypusEvent {
  /** For `onFocus`, the element that lost focus; for `onBlur`, the one gaining it. */
  readonly relatedTarget: Instance | null;

  constructor(type: string, target: Instance, data: readonly number[]) {
    super(type, target, true);
    this.relatedTarget = instances.get(data[9] ?? 0) ?? null;
  }
}

/** `onChange` / `onInput` on a text field. `target.value` (or `value`) is the new text. */
export class PlatypusInputEvent extends PlatypusEvent {
  readonly value: string;

  constructor(type: string, target: Instance, strings: readonly string[]) {
    super(type, target, true);
    this.value = strings[0] ?? "";
  }
}

/** `onTransition*`: which property (kebab-case) and how long it ran, in seconds. */
export class PlatypusTransitionEvent extends PlatypusEvent {
  readonly propertyName: string;
  readonly elapsedTime: number;
  /** Always empty: transitions of `::before` / `::after` are reported on their element. */
  readonly pseudoElement = "";

  constructor(type: string, target: Instance, data: readonly number[], strings: readonly string[]) {
    super(type, target, true);
    this.propertyName = strings[0] ?? "";
    this.elapsedTime = data[5] ?? 0;
  }
}

/** `onAnimation*`: which `@keyframes` animation and how long it has run, in seconds. */
export class PlatypusAnimationEvent extends PlatypusEvent {
  readonly animationName: string;
  readonly elapsedTime: number;
  /** Always empty: animations of `::before` / `::after` are reported on their element. */
  readonly pseudoElement = "";

  constructor(type: string, target: Instance, data: readonly number[], strings: readonly string[]) {
    super(type, target, true);
    this.animationName = strings[0] ?? "";
    this.elapsedTime = data[5] ?? 0;
  }
}

/** A file dragged in from the system. */
export interface DroppedFile {
  /** The full path on this machine. */
  readonly path: string;
  /** The file (or folder) name. */
  readonly name: string;
}

/**
 * What a drag carries, like the DOM's `DataTransfer`. For a drag of an element it is
 * one object from `dragstart` to `dragend`, holding what `setData` put in; for files
 * from the system, `files` lists them (and `types` includes `"Files"`).
 */
export class PlatypusDataTransfer {
  /** Set in `dragover` to say what a drop would do (shown nowhere yet; read in `drop`). */
  dropEffect: "none" | "copy" | "move" | "link" = "none";
  effectAllowed: "none" | "copy" | "move" | "link" | "copyMove" | "copyLink" | "linkMove" | "all" | "uninitialized" =
    "uninitialized";
  readonly files: readonly DroppedFile[];
  private readonly data = new Map<string, string>();

  constructor(files: readonly string[] = []) {
    this.files = files.map((path) => ({ path, name: path.split(/[\\/]/).filter(Boolean).pop() ?? path }));
  }

  get types(): string[] {
    const types = [...this.data.keys()];
    if (this.files.length) types.push("Files");
    return types;
  }

  setData(format: string, data: string): void {
    this.data.set(format.toLowerCase(), String(data));
  }

  getData(format: string): string {
    return this.data.get(format.toLowerCase()) ?? "";
  }

  clearData(format?: string): void {
    if (format === undefined) this.data.clear();
    else this.data.delete(format.toLowerCase());
  }

  /**
   * Shows `element` under the pointer while dragging, held at (x, y) inside it (the
   * default is the dragged element where it was pressed); `null` shows none. Unlike
   * the DOM's snapshot, the element is drawn live, so it must stay rendered.
   */
  setDragImage(element: { readonly id: number } | null, x = 0, y = 0): void {
    op("dragImage", element?.id ?? 0, x, y);
  }
}

// The transfer of the element drag in progress (from `dragstart` to `dragend`), and
// the last one made for system files.
let elementDrag: PlatypusDataTransfer | null = null;
let fileDrag: { key: string; transfer: PlatypusDataTransfer } | null = null;

function transferFor(type: string, files: readonly string[]): PlatypusDataTransfer {
  if (files.length) {
    const key = files.join("\n");
    if (fileDrag?.key !== key) fileDrag = { key, transfer: new PlatypusDataTransfer(files) };
    return fileDrag.transfer;
  }
  if (type === "dragstart" || !elementDrag) elementDrag = new PlatypusDataTransfer();
  return elementDrag;
}

/**
 * Drag and drop (`onDragStart`, `onDragOver`, `onDrop`, ...). Prevent `dragover` to
 * accept a drop there; prevent `dragstart` to cancel the drag. In `onDragEnd`,
 * `dropped` says whether it was dropped somewhere that accepted it.
 */
export class PlatypusDragEvent extends PlatypusPointerEvent {
  readonly dataTransfer: PlatypusDataTransfer;
  readonly dropped: boolean;

  constructor(type: string, target: Instance, data: readonly number[], transfer: PlatypusDataTransfer) {
    super(type, target, true, data);
    this.dataTransfer = transfer;
    this.dropped = type === "dragend" && data[4] === 1;
    if (type === "dragend" && !this.dropped) transfer.dropEffect = "none";
  }
}

/** `onScroll`: the element's new scroll position and sizes (logical pixels). */
export class PlatypusScrollEvent extends PlatypusEvent {
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

type Handler = (event: PlatypusEvent) => void;

function invoke(node: Instance, names: readonly string[], event: PlatypusEvent, phase: number) {
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
): PlatypusEvent {
  switch (type) {
    case "wheel":
      return new PlatypusWheelEvent(type, target, bubbles, data);
    case "keydown":
    case "keyup":
      return new PlatypusKeyboardEvent(type, target, data, strings);
    case "focusin":
    case "focusout":
      return new PlatypusFocusEvent(type, target, data);
    case "scroll":
      return new PlatypusScrollEvent(type, target, data);
    case "input":
      return new PlatypusInputEvent(type, target, strings);
    case "transitionrun":
    case "transitionstart":
    case "transitionend":
    case "transitioncancel":
      return new PlatypusTransitionEvent(type, target, data, strings);
    case "animationstart":
    case "animationiteration":
    case "animationend":
    case "animationcancel":
      return new PlatypusAnimationEvent(type, target, data, strings);
    case "dragstart":
    case "drag":
    case "dragenter":
    case "dragover":
    case "dragleave":
    case "drop":
    case "dragend":
      return new PlatypusDragEvent(type, target, data, transferFor(type, strings));
    default:
      return new PlatypusPointerEvent(type, target, bubbles, data);
  }
}

/**
 * Dispatches an engine event. `path` is the target then its ancestors; `data` is
 * `[x, y, button, buttons, detail, deltaX, deltaY, modifiers, repeat, related,
 * scrollLeft, scrollTop, scrollWidth, scrollHeight, clientWidth, clientHeight]` and
 * `strings` is `[key, code]` for keyboard events, `[value]` for input events and the
 * file paths for drag events.
 * Returns whether a handler called
 * `preventDefault()`.
 */
export function dispatchUiEvent(
  type: string,
  path: readonly number[],
  data: readonly number[],
  strings: readonly string[] = [],
): boolean {
  // `hasPointerCapture()` follows the engine's capture events.
  if (type === "gotpointercapture") pointerCapture.element = instances.get(path[0] ?? -1) ?? null;
  if (type === "lostpointercapture") pointerCapture.element = null;
  const names = HANDLERS[type];
  const target = instances.get(path[0] ?? -1);
  if (!names || !target) {
    // An element drag is over even when its element is gone.
    if (type === "dragend") elementDrag = null;
    return false;
  }
  const nodes = path.map((id) => instances.get(id)).filter((n): n is Instance => n !== undefined);
  const bubbles = !NON_BUBBLING.has(type);
  const event = createEvent(type, target, bubbles, data, strings);
  // Like the DOM, the field already holds the new text when handlers run.
  if (event instanceof PlatypusInputEvent) target.value = event.value;
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
  if (type === "dragend") elementDrag = null;
  // Discrete updates commit now, so the next frame shows them.
  reconciler.flushSyncWork();
  if (event instanceof PlatypusInputEvent) restoreControlledValue(target);
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
  queueFor(target.id).push([SET_ATTRS, target.id, "value", value]);
  flush();
}

globalThis.__platypus_ui_event = dispatchUiEvent;
