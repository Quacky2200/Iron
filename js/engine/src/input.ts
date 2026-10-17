// Game input: keyboard and gamepad state plus events, for gameplay code. (UI
// elements get their own events through @iron/react; keys typed into a focused text
// field don't reach here.) Rust calls `__iron_input(type, ...args)`.

import { onTick } from "./clock.ts";
import { Vector2 } from "./math.ts";

/** One connected (or recently disconnected) gamepad. */
export interface Gamepad {
  readonly id: number;
  readonly name: string;
  connected: boolean;
  /**
   * Button values by name, 0 to 1 (digital buttons are 0 or 1): `South` (A), `East` (B),
   * `West` (X), `North` (Y), `LeftBumper`, `RightBumper`, `LeftTrigger`, `RightTrigger`,
   * `Select`, `Start`, `Mode`, `LeftStick`, `RightStick`, `DPadUp/Down/Left/Right`.
   */
  readonly buttons: Record<string, number>;
  /** Axis values by name, -1 to 1, up and right positive: `LeftStickX/Y`, `RightStickX/Y`, ... */
  readonly axes: Record<string, number>;
}

export interface KeyEvent {
  /** Physical key, layout independent: `"KeyW"`, `"Space"`, `"ArrowUp"`. */
  code: string;
  /** What the key means in the current layout: `"w"`, `" "`. */
  key: string;
  repeat: boolean;
}

export interface InputEvents {
  keydown: KeyEvent;
  keyup: KeyEvent;
  gamepadconnected: { gamepad: Gamepad };
  gamepaddisconnected: { gamepad: Gamepad };
  gamepadbuttondown: { gamepad: Gamepad; button: string };
  gamepadbuttonup: { gamepad: Gamepad; button: string };
  gamepadaxis: { gamepad: Gamepad; axis: string; value: number };
}

type Listener<K extends keyof InputEvents> = (event: InputEvents[K]) => void;

const keys = new Set<string>();
// Presses and releases since the last tick, and those of the current tick (what
// `getKeyDown`/`getKeyUp` report, Unity style).
let pressedPending = new Set<string>();
let releasedPending = new Set<string>();
let pressedThisTick = new Set<string>();
let releasedThisTick = new Set<string>();
// The mouse: where it is, and movement and wheel since the last tick / this tick.
let mouseX = 0;
let mouseY = 0;
let deltaPending = { x: 0, y: 0 };
let wheelPending = { x: 0, y: 0 };
let deltaThisTick = { x: 0, y: 0 };
let wheelThisTick = { x: 0, y: 0 };
onTick(() => {
  pressedThisTick = pressedPending;
  releasedThisTick = releasedPending;
  pressedPending = new Set();
  releasedPending = new Set();
  deltaThisTick = deltaPending;
  wheelThisTick = wheelPending;
  deltaPending = { x: 0, y: 0 };
  wheelPending = { x: 0, y: 0 };
});

/** Mouse buttons by Unity's numbers: 0 left, 1 right, 2 middle. */
const MOUSE_BUTTONS = ["MouseLeft", "MouseRight", "MouseMiddle"];
const mouseCode = (button: number) => MOUSE_BUTTONS[button] ?? `Mouse${button}`;

/** Raw mouse movement per "Mouse X" / "Mouse Y" unit (Unity's default sensitivity). */
const MOUSE_SENSITIVITY = 0.1;

/** Keys and pad parts that make up the virtual axes (Unity's defaults). */
const AXES: Record<string, { neg: string[]; pos: string[]; stick?: string }> = {
  Horizontal: { neg: ["KeyA", "ArrowLeft"], pos: ["KeyD", "ArrowRight"], stick: "LeftStickX" },
  Vertical: { neg: ["KeyS", "ArrowDown"], pos: ["KeyW", "ArrowUp"], stick: "LeftStickY" },
};
/** Virtual buttons: keys or gamepad buttons. */
const BUTTONS: Record<string, { keys: string[]; pad: string[] }> = {
  Jump: { keys: ["Space"], pad: ["South"] },
  Fire1: { keys: ["ControlLeft", "MouseLeft"], pad: ["RightTrigger"] },
  Fire2: { keys: ["AltLeft", "MouseRight"], pad: ["LeftTrigger"] },
  Submit: { keys: ["Enter", "Space"], pad: ["South"] },
  Cancel: { keys: ["Escape"], pad: ["East"] },
};
const pads = new Map<number, Gamepad>();
const listeners: { [K in keyof InputEvents]?: Set<Listener<K>> } = {};

function emit<K extends keyof InputEvents>(type: K, event: InputEvents[K]) {
  for (const fn of listeners[type] ?? []) {
    try {
      fn(event);
    } catch (e) {
      const err = e instanceof Error ? e : new Error(String(e));
      console.error(`[input] error in ${type} listener: ${err.message}${err.stack ? `\n${err.stack}` : ""}`);
    }
  }
}

function pad(id: number, name = ""): Gamepad {
  let p = pads.get(id);
  if (!p) {
    p = { id, name, connected: true, buttons: {}, axes: {} };
    pads.set(id, p);
  }
  return p;
}

export const input = {
  /** Whether a key (by physical `code`, e.g. `"KeyW"`) is held. */
  isKeyDown(code: string): boolean {
    return keys.has(code);
  },

  /** Unity's `GetKey`: held. */
  getKey(code: string): boolean {
    return keys.has(code);
  },

  /** Unity's `GetKeyDown`: pressed since the last tick (true for one tick). */
  getKeyDown(code: string): boolean {
    return pressedThisTick.has(code);
  },

  /** Unity's `GetKeyUp`: released since the last tick. */
  getKeyUp(code: string): boolean {
    return releasedThisTick.has(code);
  },

  /**
   * Unity's `GetAxis` for `"Horizontal"` (A/D, arrows, left stick X) and `"Vertical"`
   * (S/W, arrows, left stick Y): -1 to 1. Keys snap; the stick wins when pushed further.
   */
  /** Where the mouse is, in UI pixels from the top-left (like `Camera.screenPointToRay`). */
  get mousePosition(): Vector2 {
    return new Vector2(mouseX, mouseY);
  },

  /**
   * How far the mouse moved this frame in raw device units, right and up positive. It
   * keeps coming while the cursor is locked (`Cursor.lockState`): use it for mouse look.
   */
  get mouseDelta(): Vector2 {
    return new Vector2(deltaThisTick.x, 0 - deltaThisTick.y);
  },

  /** Wheel movement this frame, in lines: y up (away from you) positive. */
  get mouseScrollDelta(): Vector2 {
    return new Vector2(wheelThisTick.x, wheelThisTick.y);
  },

  /** Whether mouse `button` (0 left, 1 right, 2 middle) is held. */
  getMouseButton(button: number): boolean {
    return keys.has(mouseCode(button));
  },

  /** Whether mouse `button` went down this frame. */
  getMouseButtonDown(button: number): boolean {
    return pressedThisTick.has(mouseCode(button));
  },

  /** Whether mouse `button` came up this frame. */
  getMouseButtonUp(button: number): boolean {
    return releasedThisTick.has(mouseCode(button));
  },

  getAxis(name: "Horizontal" | "Vertical" | "Mouse X" | "Mouse Y" | "Mouse ScrollWheel"): number {
    // Unity's mouse axes: raw movement scaled by its sensitivity, and the wheel.
    if (name === "Mouse X") return deltaThisTick.x * MOUSE_SENSITIVITY;
    if (name === "Mouse Y") return -deltaThisTick.y * MOUSE_SENSITIVITY;
    if (name === "Mouse ScrollWheel") return wheelThisTick.y * MOUSE_SENSITIVITY;
    const axis = AXES[name];
    if (!axis) return 0;
    const keyValue = (axis.pos.some((k) => keys.has(k)) ? 1 : 0) - (axis.neg.some((k) => keys.has(k)) ? 1 : 0);
    let stick = 0;
    for (const p of input.gamepads()) {
      const v = p.axes[axis.stick ?? ""] ?? 0;
      if (Math.abs(v) > Math.abs(stick)) stick = v;
    }
    return Math.abs(stick) > Math.abs(keyValue) ? stick : keyValue;
  },

  /** Unity's `GetButton` for `"Jump"`, `"Fire1"`, `"Fire2"`, `"Submit"`, `"Cancel"`: keys or pad. */
  getButton(name: "Jump" | "Fire1" | "Fire2" | "Submit" | "Cancel"): boolean {
    const b = BUTTONS[name];
    return !!b && (b.keys.some((k) => keys.has(k)) || b.pad.some((p) => input.isButtonDown(p)));
  },

  /** Pressed since the last tick (keys only for now). */
  getButtonDown(name: "Jump" | "Fire1" | "Fire2" | "Submit" | "Cancel"): boolean {
    const b = BUTTONS[name];
    return !!b && b.keys.some((k) => pressedThisTick.has(k));
  },

  /** Connected gamepads. */
  gamepads(): Gamepad[] {
    return [...pads.values()].filter((p) => p.connected);
  },

  /** A gamepad by id, if connected. */
  gamepad(id: number): Gamepad | undefined {
    const p = pads.get(id);
    return p?.connected ? p : undefined;
  },

  /** Whether `button` is held on any connected gamepad. */
  isButtonDown(button: string): boolean {
    return input.gamepads().some((p) => (p.buttons[button] ?? 0) > 0.5);
  },

  /** Listens for an input event; returns a function that stops listening. */
  on<K extends keyof InputEvents>(type: K, fn: Listener<K>): () => void {
    let set = listeners[type] as Set<Listener<K>> | undefined;
    if (!set) {
      set = new Set();
      (listeners as Record<string, Set<Listener<K>>>)[type] = set;
    }
    set.add(fn);
    return () => set.delete(fn);
  },
};

/** Clears all state and listeners. Only for tests. */
export function _resetInputForTests() {
  keys.clear();
  pressedPending.clear();
  releasedPending.clear();
  pressedThisTick.clear();
  releasedThisTick.clear();
  pads.clear();
  mouseX = mouseY = 0;
  deltaPending = { x: 0, y: 0 };
  wheelPending = { x: 0, y: 0 };
  deltaThisTick = { x: 0, y: 0 };
  wheelThisTick = { x: 0, y: 0 };
  for (const k of Object.keys(listeners)) delete (listeners as Record<string, unknown>)[k];
}

globalThis.__iron_input = (type: string, ...args: unknown[]) => {
  switch (type) {
    case "keydown": {
      const [code, key, repeat] = args as [string, string, boolean];
      if (!repeat && !keys.has(code)) pressedPending.add(code);
      keys.add(code);
      emit("keydown", { code, key, repeat: !!repeat });
      break;
    }
    case "keyup": {
      const [code, key] = args as [string, string];
      if (keys.has(code)) releasedPending.add(code);
      keys.delete(code);
      emit("keyup", { code, key, repeat: false });
      break;
    }
    case "mousemove":
      [mouseX, mouseY] = args as [number, number];
      break;
    case "mousedelta": {
      const [dx, dy] = args as [number, number];
      deltaPending.x += dx;
      deltaPending.y += dy;
      break;
    }
    case "wheel": {
      const [dx, dy] = args as [number, number];
      wheelPending.x += dx;
      wheelPending.y += dy;
      break;
    }
    // The window lost focus: releases may never arrive.
    case "blur":
      keys.clear();
      for (const p of pads.values()) for (const b of Object.keys(p.buttons)) p.buttons[b] = 0;
      break;
    case "gamepadconnected": {
      const [id, name] = args as [number, string];
      const p = pad(id, name);
      p.connected = true;
      emit("gamepadconnected", { gamepad: p });
      break;
    }
    case "gamepaddisconnected": {
      const p = pad(args[0] as number);
      p.connected = false;
      emit("gamepaddisconnected", { gamepad: p });
      break;
    }
    case "gamepadbutton": {
      const [id, button, value] = args as [number, string, number];
      const p = pad(id);
      const was = (p.buttons[button] ?? 0) > 0.5;
      p.buttons[button] = value;
      const now = value > 0.5;
      if (now && !was) emit("gamepadbuttondown", { gamepad: p, button });
      if (!now && was) emit("gamepadbuttonup", { gamepad: p, button });
      break;
    }
    case "gamepadaxis": {
      const [id, axis, value] = args as [number, string, number];
      const p = pad(id);
      p.axes[axis] = value;
      emit("gamepadaxis", { gamepad: p, axis, value });
      break;
    }
  }
};
