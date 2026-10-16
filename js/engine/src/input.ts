// Game input: keyboard and gamepad state plus events, for gameplay code. (UI
// elements get their own events through @iron/react; keys typed into a focused text
// field don't reach here.) Rust calls `__iron_input(type, ...args)`.

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
  pads.clear();
  for (const k of Object.keys(listeners)) delete (listeners as Record<string, unknown>)[k];
}

globalThis.__iron_input = (type: string, ...args: unknown[]) => {
  switch (type) {
    case "keydown": {
      const [code, key, repeat] = args as [string, string, boolean];
      keys.add(code);
      emit("keydown", { code, key, repeat: !!repeat });
      break;
    }
    case "keyup": {
      const [code, key] = args as [string, string];
      keys.delete(code);
      emit("keyup", { code, key, repeat: false });
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
