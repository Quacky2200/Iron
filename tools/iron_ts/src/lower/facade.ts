// The TypeScript `@iron/engine` members the native engine API has in another shape, written over
// the native functions. A chunk is appended to a program that names one of its triggers (the text
// after `when`, separated by `|`), when the engine API has the names after an optional `@`
// (`//# when Spline @ Vector3`). The naming scheme is in `facade.rs`. A chunk may also be plain
// TypeScript classes (`Shapes`, `Spline`, `SplineAnimate`); their declarations for `tsc` are in
// `engine_ts.d.ts`. Keep module-level state out of a chunk: a top-level initialiser of the
// program may run a chunk's code before the chunk's own `const`s are set.

//# when input.gamepads()|input.on(
interface Gamepad {
  id: number;
  name: string;
  connected: boolean;
  buttons: Record<string, number>;
  axes: Record<string, number>;
}
interface GamepadEvent {
  gamepad: Gamepad;
}
// A pad as the TypeScript API has it: the standard buttons and axes by name (0 when not reported).
function __pad(id: number): Gamepad {
  const buttons: Record<string, number> = {};
  for (const b of ["South", "East", "West", "North", "C", "Z", "LeftBumper", "RightBumper", "LeftTrigger", "RightTrigger", "Select", "Start", "Mode", "LeftStick", "RightStick", "DPadUp", "DPadDown", "DPadLeft", "DPadRight"]) buttons[b] = input.gamepadButton(id, b);
  const axes: Record<string, number> = {};
  for (const a of ["LeftStickX", "LeftStickY", "RightStickX", "RightStickY", "LeftZ", "RightZ", "DPadX", "DPadY"]) axes[a] = input.gamepadAxis(id, a);
  return { id, name: input.gamepadName(id), connected: input.gamepadConnected(id), buttons, axes };
}
function __xt_input_gamepads(): Gamepad[] {
  const out: Gamepad[] = [];
  const n = input.gamepadCount();
  for (let i = 0; i < n; i++) out.push(__pad(input.gamepadId(i)));
  return out;
}
function __xt_input_on(type: string, fn: (e: GamepadEvent) => void): () => void {
  const token = type === "gamepaddisconnected" ? input.onGamepadDisconnected((id: number) => { fn({ gamepad: __pad(id) }); }) : input.onGamepadConnected((id: number) => { fn({ gamepad: __pad(id) }); });
  return () => { input.off(token); };
}

//# when postMessage&Window|onMessage&Window|onRequest&Window|.request<&Window|.request(&Window|broadcast&Window
// Messages between windows travel as JSON of the static type (the compiler refuses handles in it).
function __xm_Window_postMessage<T>(self: Window, data: T): boolean {
  return self.postMessageJson(JSON.stringify(data));
}
function __xm_Window_onMessage<T>(self: Window, f: (from: Window, data: T) => void): number {
  return self.onMessageJson((from: Window, text: string) => {
    const data: T = JSON.parse(text);
    f(from, data);
  });
}
function __xm_Window_onRequest<T, R, U = number>(self: Window, f: (from: Window, data: T, reply: (answer: R) => void) => U): number {
  return self.onRequestJson((from: Window, text: string, id: number) => {
    const data: T = JSON.parse(text);
    f(from, data, (answer: R) => {
      Window.replyJson(id, JSON.stringify(answer));
    });
  });
}
// An async handler (it returns a promise, found by the compiler): a rejection fails the asker's promise.
function __xm_Window_onRequestP<T, R>(self: Window, f: (from: Window, data: T, reply: (answer: R) => void) => Promise<void>): number {
  return self.onRequestJson((from: Window, text: string, id: number) => {
    const data: T = JSON.parse(text);
    f(from, data, (answer: R) => {
      Window.replyJson(id, JSON.stringify(answer));
    }).then(() => {}, (e: Error) => { Window.replyError(id, e.message); });
  });
}
function __xt_Window_onMessage<T>(f: (from: Window, data: T) => void): number {
  return Window.onMessageJson((from: Window, text: string) => {
    const data: T = JSON.parse(text);
    f(from, data);
  });
}
function __xt_Window_onRequest<T, R, U = number>(f: (from: Window, data: T, reply: (answer: R) => void) => U): number {
  return Window.onRequestJson((from: Window, text: string, id: number) => {
    const data: T = JSON.parse(text);
    f(from, data, (answer: R) => {
      Window.replyJson(id, JSON.stringify(answer));
    });
  });
}
function __xt_Window_onRequestP<T, R>(f: (from: Window, data: T, reply: (answer: R) => void) => Promise<void>): number {
  return Window.onRequestJson((from: Window, text: string, id: number) => {
    const data: T = JSON.parse(text);
    f(from, data, (answer: R) => {
      Window.replyJson(id, JSON.stringify(answer));
    }).then(() => {}, (e: Error) => { Window.replyError(id, e.message); });
  });
}
function __xt_Window_broadcast<T>(data: T): number {
  return Window.broadcastJson(JSON.stringify(data));
}
// `request<R>(data)` is written per reply type by facade.rs (a promise class exists per written type).

//# when .emit(&Window|.emit<&Window|.on(&Window|.on<&Window|.ask(&Window|.ask<&Window|.answer(&Window|.answer<&Window|emitAll
// Named events: sugar over the raw messages. The wire form is `{"$e":<name>,"d":<payload>}` (no `d`
// without a payload). The host delivers envelopes only to the `on` / `answer` listeners of their name;
// raw `onMessage` / `onRequest` listeners never see them.
function __win_event_text(name: string, payload: string | null): string {
  if (name === "") throw new Error("Window event names must not be empty");
  const head = '{"$e":' + JSON.stringify(name);
  return payload === null ? head + "}" : head + ',"d":' + payload + "}";
}
function __xm_Window_emit<T>(self: Window, name: string, payload: T): boolean {
  return self.postMessageJson(__win_event_text(name, JSON.stringify(payload)));
}
function __xm_Window_emit0<U>(self: Window, name: string): boolean {
  return self.postMessageJson(__win_event_text(name, null));
}
function __xt_Window_emitAll<T>(name: string, payload: T): number {
  return Window.broadcastJson(__win_event_text(name, JSON.stringify(payload)));
}
function __xt_Window_emitAll0<U>(name: string): number {
  return Window.broadcastJson(__win_event_text(name, null));
}
function __xm_Window_on<T>(self: Window, name: string, f: (payload: T, from: Window) => void): number {
  if (name === "") throw new Error("Window event names must not be empty");
  return self.onEventJson(name, (from: Window, body: string) => {
    const data: T = JSON.parse(body === "" ? "null" : body);
    f(data, from);
  });
}
function __xm_Window_answer<T, R>(self: Window, name: string, f: (payload: T, from: Window) => R): number {
  if (name === "") throw new Error("Window event names must not be empty");
  return self.onAnswerJson(name, (from: Window, body: string, id: number) => {
    const data: T = JSON.parse(body === "" ? "null" : body);
    const answer: R = f(data, from);
    Window.replyJson(id, JSON.stringify(answer));
  });
}
// An async handler (it returns a promise, found by the compiler): the reply is sent when it resolves, a
// rejection fails the asker's promise with the error's message.
function __xm_Window_answerP<T, R>(self: Window, name: string, f: (payload: T, from: Window) => Promise<R>): number {
  if (name === "") throw new Error("Window event names must not be empty");
  return self.onAnswerJson(name, (from: Window, body: string, id: number) => {
    const data: T = JSON.parse(body === "" ? "null" : body);
    f(data, from).then((answer: R) => {
      Window.replyJson(id, JSON.stringify(answer));
    }, (e: Error) => { Window.replyError(id, e.message); });
  });
}
function __xm_Window_answer0P<R>(self: Window, name: string, f: () => Promise<R>): number {
  if (name === "") throw new Error("Window event names must not be empty");
  return self.onAnswerJson(name, (from: Window, body: string, id: number) => {
    f().then((answer: R) => {
      Window.replyJson(id, JSON.stringify(answer));
    }, (e: Error) => { Window.replyError(id, e.message); });
  });
}
// A handler without parameters (`win.on("ping", () => ..)`): there is no payload type to decode.
function __xm_Window_on0<U>(self: Window, name: string, f: () => void): number {
  if (name === "") throw new Error("Window event names must not be empty");
  return self.onEventJson(name, (from: Window, body: string) => {
    f();
  });
}
function __xm_Window_answer0<R>(self: Window, name: string, f: () => R): number {
  if (name === "") throw new Error("Window event names must not be empty");
  return self.onAnswerJson(name, (from: Window, body: string, id: number) => {
    const answer: R = f();
    Window.replyJson(id, JSON.stringify(answer));
  });
}
// `ask<R>(name, payload?, timeoutMs?)` is written per reply type by facade.rs.

//# when Random.pick
function __xt_Random_pick<T>(items: T[]): T | undefined {
  return items.length > 0 ? items[Random.rangeInt(0, items.length)] : undefined;
}

//# when illuminanceAt|measureIlluminance
function __light_lux(l: Light, point: Vector3, normal: Vector3 | undefined): number {
  if (!l.gameObject.activeInHierarchy) return 0;
  const t = l.gameObject.transform;
  const forward = t.forward;
  if (l.type === "directional") {
    const cos = normal !== undefined ? Math.max(0, -(normal.x * forward.x + normal.y * forward.y + normal.z * forward.z)) : 1;
    return l.intensity * cos;
  }
  const p = t.position;
  const dx = point.x - p.x;
  const dy = point.y - p.y;
  const dz = point.z - p.z;
  const d = Math.sqrt(dx * dx + dy * dy + dz * dz);
  if (d >= l.range || d === 0) return 0;
  // As the renderer: a smooth fade to zero at the range.
  const x = d / l.range;
  const fade = (1 - x * x * x * x) * (1 - x * x * x * x);
  // Lumens to candela, all round (HDRP's spots without reflector).
  let candela = l.intensity / (4 * Math.PI);
  if (l.type === "spot") {
    const c = (dx * forward.x + dy * forward.y + dz * forward.z) / d;
    const outer = Math.cos(((l.spotAngle / 2) * Math.PI) / 180);
    const inner = Math.cos(((l.innerSpotAngle / 2) * Math.PI) / 180);
    const u = Math.min(1, Math.max(0, (c - outer) / Math.max(inner - outer, 1e-4)));
    candela *= u * u * (3 - 2 * u);
  }
  const cos = normal !== undefined ? Math.max(0, -(normal.x * dx + normal.y * dy + normal.z * dz) / d) : 1;
  return ((candela * fade) / (d * d)) * cos;
}
function __light_meter(point: Vector3, normal: Vector3 | undefined, shadows: boolean): number {
  let sum = 0;
  const n = Light.count();
  for (let i = 0; i < n; i++) {
    const l = Light.at(i);
    let lux = __light_lux(l, point, normal);
    if (shadows && lux > 0) {
      const t = l.gameObject.transform;
      let dir = new Vector3(0, 1, 0);
      let distance = 10000;
      if (l.type === "directional") {
        const f = t.forward;
        dir = new Vector3(-f.x, -f.y, -f.z);
      } else {
        const p = t.position;
        dir = new Vector3(p.x - point.x, p.y - point.y, p.z - point.z);
        distance = dir.magnitude - 0.05;
      }
      // Start a little off the point, so the surface it is on does not block it.
      const len = dir.magnitude > 0 ? dir.magnitude : 1;
      const from = new Vector3(point.x + (dir.x / len) * 0.02, point.y + (dir.y / len) * 0.02, point.z + (dir.z / len) * 0.02);
      if (Physics.raycast(from, dir, distance) !== null) lux = 0;
    }
    sum += lux;
  }
  return sum;
}
function __xt_Light_illuminanceAt(point: Vector3, normal?: Vector3): number {
  return __light_meter(point, normal, false);
}
function __xt_Light_measureIlluminance(point: Vector3, normal?: Vector3): Promise<number> {
  // Resolves after the next physics step (the rays need the colliders as the world has them then).
  return new Promise<number>((resolve) => {
    Physics.afterNextStep(() => {
      resolve(__light_meter(point, normal, true));
    });
  });
}
