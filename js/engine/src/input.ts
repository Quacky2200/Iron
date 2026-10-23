// SPDX-License-Identifier: Apache-2.0

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

/**
 * Input state of one event domain (one per window family; 1 is the main game's). Reads resolve against
 * the domain whose code is running (`__iron_domain()`, the host's current domain: inside a window's
 * `run`, its timers, messages and UI events it is that window's). Code with no domain tag, such as
 * promise jobs after the host left the domain, reads the main domain's.
 */
class DomainInput {
  keys = new Set<string>();
  // Presses and releases since the last tick, and those of the current tick (what
  // `getKeyDown`/`getKeyUp` report, Unity style).
  pressedPending = new Set<string>();
  releasedPending = new Set<string>();
  pressedThisTick = new Set<string>();
  releasedThisTick = new Set<string>();
  // The mouse: where it is, and movement and wheel since the last tick / this tick.
  mouseX = 0;
  mouseY = 0;
  deltaPending = { x: 0, y: 0 };
  wheelPending = { x: 0, y: 0 };
  deltaThisTick = { x: 0, y: 0 };
  wheelThisTick = { x: 0, y: 0 };
  pads = new Map<number, Gamepad>();
  listeners: { [K in keyof InputEvents]?: Set<Listener<K>> } = {};

  tick() {
    this.pressedThisTick = this.pressedPending;
    this.releasedThisTick = this.releasedPending;
    this.pressedPending = new Set();
    this.releasedPending = new Set();
    this.deltaThisTick = this.deltaPending;
    this.wheelThisTick = this.wheelPending;
    this.deltaPending = { x: 0, y: 0 };
    this.wheelPending = { x: 0, y: 0 };
  }
}

const MAIN_DOMAIN = 1;
const domains = new Map<number, DomainInput>();

function domainState(d: number): DomainInput {
  let s = domains.get(d);
  if (!s) {
    s = new DomainInput();
    domains.set(d, s);
  }
  return s;
}

/** The current domain's input (the main one when the host has no domains). */
function cur(): DomainInput {
  const f = (globalThis as { __iron_domain?: () => number }).__iron_domain;
  return domainState(typeof f === "function" ? f() : MAIN_DOMAIN);
}

// The main game's input frame ends with `Time`'s tick; isolated domains' end in `__iron_input("tick")`.
onTick(() => domainState(MAIN_DOMAIN).tick());

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

function emit<K extends keyof InputEvents>(type: K, event: InputEvents[K]) {
  for (const fn of cur().listeners[type] ?? []) {
    try {
      fn(event);
    } catch (e) {
      const err = e instanceof Error ? e : new Error(String(e));
      console.error(`[input] error in ${type} listener: ${err.message}${err.stack ? `\n${err.stack}` : ""}`);
    }
  }
}

function pad(id: number, name = ""): Gamepad {
  const pads = cur().pads;
  let p = pads.get(id);
  if (!p) {
    p = { id, name, connected: true, buttons: {}, axes: {} };
    pads.set(id, p);
  }
  return p;
}

/**
 * Keyboard, mouse and gamepad state. **Input belongs to the window's event domain**: it reads the keys of
 * the window family whose code is running. Main-game code (components' `onUpdate`, ...) always reads the
 * main window's input, even while another window is focused; code inside an isolated window (`win.run`,
 * its timers, message handlers and UI events) reads that window's own. An isolated window's keys never
 * reach the main `input`: send a message instead. Focus loss releases a domain's held keys. Promise
 * jobs (`await`) carry no domain and read the main domain's. See docs/windows.md.
 */
export const input = {
  /** Whether a key (by physical `code`, e.g. `"KeyW"`) is held. */
  isKeyDown(code: string): boolean {
    return cur().keys.has(code);
  },

  /** Unity's `GetKey`: held. */
  getKey(code: string): boolean {
    return cur().keys.has(code);
  },

  /** Unity's `GetKeyDown`: pressed since the last tick (true for one tick). */
  getKeyDown(code: string): boolean {
    return cur().pressedThisTick.has(code);
  },

  /** Unity's `GetKeyUp`: released since the last tick. */
  getKeyUp(code: string): boolean {
    return cur().releasedThisTick.has(code);
  },

  /**
   * Unity's `GetAxis` for `"Horizontal"` (A/D, arrows, left stick X) and `"Vertical"`
   * (S/W, arrows, left stick Y): -1 to 1. Keys snap; the stick wins when pushed further.
   */
  /** Where the mouse is, in UI pixels from the top-left (like `Camera.screenPointToRay`). */
  get mousePosition(): Vector2 {
    return new Vector2(cur().mouseX, cur().mouseY);
  },

  /**
   * How far the mouse moved this frame in raw device units, right and up positive. It
   * keeps coming while the cursor is locked (`Cursor.lockState`): use it for mouse look.
   */
  get mouseDelta(): Vector2 {
    return new Vector2(cur().deltaThisTick.x, 0 - cur().deltaThisTick.y);
  },

  /** Wheel movement this frame, in lines: y up (away from you) positive. */
  get mouseScrollDelta(): Vector2 {
    return new Vector2(cur().wheelThisTick.x, cur().wheelThisTick.y);
  },

  /** Whether mouse `button` (0 left, 1 right, 2 middle) is held. */
  getMouseButton(button: number): boolean {
    return cur().keys.has(mouseCode(button));
  },

  /** Whether mouse `button` went down this frame. */
  getMouseButtonDown(button: number): boolean {
    return cur().pressedThisTick.has(mouseCode(button));
  },

  /** Whether mouse `button` came up this frame. */
  getMouseButtonUp(button: number): boolean {
    return cur().releasedThisTick.has(mouseCode(button));
  },

  getAxis(name: "Horizontal" | "Vertical" | "Mouse X" | "Mouse Y" | "Mouse ScrollWheel"): number {
    // Unity's mouse axes: raw movement scaled by its sensitivity, and the wheel.
    if (name === "Mouse X") return cur().deltaThisTick.x * MOUSE_SENSITIVITY;
    if (name === "Mouse Y") return -cur().deltaThisTick.y * MOUSE_SENSITIVITY;
    if (name === "Mouse ScrollWheel") return cur().wheelThisTick.y * MOUSE_SENSITIVITY;
    const axis = AXES[name];
    if (!axis) return 0;
    const keyValue = (axis.pos.some((k) => cur().keys.has(k)) ? 1 : 0) - (axis.neg.some((k) => cur().keys.has(k)) ? 1 : 0);
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
    return !!b && (b.keys.some((k) => cur().keys.has(k)) || b.pad.some((p) => input.isButtonDown(p)));
  },

  /** Pressed since the last tick (keys only for now). */
  getButtonDown(name: "Jump" | "Fire1" | "Fire2" | "Submit" | "Cancel"): boolean {
    const b = BUTTONS[name];
    return !!b && b.keys.some((k) => cur().pressedThisTick.has(k));
  },

  /** Connected gamepads. */
  gamepads(): Gamepad[] {
    return [...cur().pads.values()].filter((p) => p.connected);
  },

  /** A gamepad by id, if connected. */
  gamepad(id: number): Gamepad | undefined {
    const p = cur().pads.get(id);
    return p?.connected ? p : undefined;
  },

  /** Whether `button` is held on any connected gamepad. */
  isButtonDown(button: string): boolean {
    return input.gamepads().some((p) => (p.buttons[button] ?? 0) > 0.5);
  },

  /** Listens for an input event; returns a function that stops listening. */
  on<K extends keyof InputEvents>(type: K, fn: Listener<K>): () => void {
    let set = cur().listeners[type] as Set<Listener<K>> | undefined;
    if (!set) {
      set = new Set();
      (cur().listeners as Record<string, Set<Listener<K>>>)[type] = set;
    }
    set.add(fn);
    return () => set.delete(fn);
  },
};

/** Clears all state and listeners, of every domain. Only for tests. */
export function _resetInputForTests() {
  domains.clear();
}

globalThis.__iron_input = (type: string, ...args: unknown[]) => {
  switch (type) {
    // An isolated domain ends its input frame (the main one's ends with `Time`'s tick).
    case "tick":
      cur().tick();
      break;
    case "keydown": {
      const [code, key, repeat] = args as [string, string, boolean];
      if (!repeat && !cur().keys.has(code)) cur().pressedPending.add(code);
      cur().keys.add(code);
      emit("keydown", { code, key, repeat: !!repeat });
      break;
    }
    case "keyup": {
      const [code, key] = args as [string, string];
      if (cur().keys.has(code)) cur().releasedPending.add(code);
      cur().keys.delete(code);
      emit("keyup", { code, key, repeat: false });
      break;
    }
    case "mousemove":
      [cur().mouseX, cur().mouseY] = args as [number, number];
      break;
    case "mousedelta": {
      const [dx, dy] = args as [number, number];
      cur().deltaPending.x += dx;
      cur().deltaPending.y += dy;
      break;
    }
    case "wheel": {
      const [dx, dy] = args as [number, number];
      cur().wheelPending.x += dx;
      cur().wheelPending.y += dy;
      break;
    }
    // The window lost focus: releases may never arrive.
    case "blur":
      cur().keys.clear();
      for (const p of cur().pads.values()) for (const b of Object.keys(p.buttons)) p.buttons[b] = 0;
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
