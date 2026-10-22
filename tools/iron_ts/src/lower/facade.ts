// The TypeScript `@iron/engine` members the native engine API has in another shape, written over
// the native functions. A chunk is appended to a program that names one of its triggers (the text
// after `when`, separated by `|`), when the engine API has the names after an optional `@`
// (`//# when Spline @ Vector3`). The naming scheme is in `facade.rs`. A chunk may also be plain
// TypeScript classes (`Shapes`, `Spline`, `SplineAnimate`); their declarations for `tsc` are in
// `engine_ts.d.ts`. Keep module-level state out of a chunk: a top-level initialiser of the
// program may run a chunk's code before the chunk's own `const`s are set.

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
