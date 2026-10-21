// The Window API from compiled code: options objects, static properties, listeners and
// `createRoot(win)`. The Rust test (tests/all/windows.rs) plays the host (opened, resized, closed).

let resizedCount = 0;
let closedCount = 0;
let closeRequests = 0;
let cancelNext = false;
let lastWidth = 0;
let lastHeight = 0;

/** The open window with this handle, through `Window.at`. */
function byHandle(h: number): Window | undefined {
  for (let i = 0; i < Window.count; i++) {
    const w = Window.at(i);
    if (w !== undefined && w.handle === h) return w;
  }
  return undefined;
}

export function createTool(): number {
  const w = Window.create({ title: "Palette", style: "tool", width: 220, height: 320, key: "palette" });
  return w.handle;
}

export function createDialog(parentHandle: number): number {
  const p = byHandle(parentHandle);
  if (p === undefined) return 0;
  const w = Window.create({ title: "Sure?", style: "dialog", parent: p, resizable: false });
  return w.handle;
}

export function describe(h: number): string {
  const w = byHandle(h);
  if (w === undefined) return "none";
  return `${w.title}|${w.style}|${w.isChild}|${w.parent?.title ?? "-"}|${w.width}x${w.height}`;
}

export function setMainTitle(t: string): void {
  Window.main.title = t;
}

export function mainTitle(): string {
  return Window.main.title;
}

export function mainInfo(): string {
  return `${Window.main.handle}|${Window.main.isMain}|${Window.main.isChild}|${Window.current.handle}`;
}

export function focusState(h: number): string {
  const w = byHandle(h);
  if (w === undefined) return "none";
  const f = Window.focused;
  return `${f === undefined}|${w.focused}|${f !== undefined ? f.handle : 0}`;
}

export function setCancel(on: boolean): void {
  cancelNext = on;
}

export function listen(h: number): void {
  const found = byHandle(h);
  if (found === undefined) return;
  const w: Window = found;
  w.onResized((width, height) => {
    resizedCount++;
    lastWidth = width;
    lastHeight = height;
  });
  w.onClosed(() => {
    closedCount++;
  });
  w.onCloseRequested(() => {
    closeRequests++;
    if (cancelNext) w.cancelClose();
  });
}

export function counts(): string {
  return `${resizedCount}|${closedCount}|${closeRequests}|${lastWidth}x${lastHeight}`;
}

export function list(): string {
  let out = "";
  for (let i = 0; i < Window.count; i++) {
    const w = Window.at(i);
    if (w !== undefined) out += (i > 0 ? "," : "") + w.title;
  }
  return out;
}

export function mount(h: number): void {
  const w = byHandle(h);
  if (w === undefined) return;
  createRoot(w).render(
    <box>
      <text>{w.title}</text>
    </box>,
  );
}

export function mountMain(): void {
  createRoot(Window.main).render(<box />);
}

// ---- Families and messages --------------------------------------------------------------------

interface Select {
  kind: string;
  p: Vector3;
  ids: number[];
  name: string;
}

let received = "";
let domainLog = "";
let askResult = "";
let reqLog = "";

/** A "normal" window without a parent: the root of its own family. */
export function createInspector(): number {
  const w = Window.create({ title: "Inspector", style: "normal", width: 320, height: 480 });
  return w.handle;
}

export function postSelect(h: number): boolean {
  const w = byHandle(h);
  if (w === undefined) return false;
  const msg: Select = { kind: "select", p: new Vector3(1, 2, 3), ids: [1, 2], name: "Player" };
  return w.postMessage(msg);
}

export function listenSelect(h: number): void {
  const found = byHandle(h);
  if (found === undefined) return;
  const w: Window = found;
  w.onMessage<Select>((from, msg) => {
    received += `${from.handle}:${msg.kind}|${msg.p.y}|${msg.ids.length}|${msg.name};`;
  });
}

/** The type comes from the annotation of the listener's `msg`. */
export function listenInferred(h: number): void {
  const found = byHandle(h);
  if (found === undefined) return;
  const w: Window = found;
  w.onMessage((from: Window, msg: Select) => {
    received += `inferred ${msg.ids.length};`;
  });
}

export function receivedMessages(): string {
  return received;
}

/// Answers `{ kind }` with `{ n: kind.length }`.
export function serve(h: number): void {
  const found = byHandle(h);
  if (found === undefined) return;
  const w: Window = found;
  w.onRequest<{ kind: string }, { n: number }>((from, req, reply) => {
    reqLog += `${from.handle}:${req.kind};`;
    reply({ n: req.kind.length });
  });
}

/** Takes requests and never answers. */
export function serveQuiet(h: number): void {
  const found = byHandle(h);
  if (found === undefined) return;
  const w: Window = found;
  w.onRequest<{ kind: string }, { n: number }>((from, req, reply) => {
    reqLog += `quiet ${req.kind};`;
  });
}

export function requestLog(): string {
  return reqLog;
}

export async function ask(h: number, timeoutMs: number): Promise<number> {
  const w = byHandle(h);
  if (w === undefined) return -1;
  try {
    const r = await w.request<{ n: number }>({ kind: "stats" }, timeoutMs);
    askResult = `ok ${r.n}`;
  } catch (e) {
    askResult = `error ${(e as Error).message}`;
  }
  return 0;
}

export function askLog(): string {
  return askResult;
}

export function resetAsk(): void {
  askResult = "";
}

/** Listens for every message of the current domain. */
export function listenDomain(): void {
  Window.onMessage<{ n: number }>((from, msg) => {
    domainLog += `${from.handle}:${msg.n};`;
  });
}

export function domainMessages(): string {
  return domainLog;
}

export function broadcastHi(): number {
  return Window.broadcast({ n: 7 });
}

export function familyInfo(h: number): string {
  const w = byHandle(h);
  if (w === undefined) return "none";
  return `${w.root.handle}|${w.isIsolated}|${w.domain}|${Window.domainFocused}`;
}

// ---- Timers per domain ------------------------------------------------------------------------

let timerLog = "";
let tickId = 0;

/** A one-shot timer; the host calls this with a domain entered. */
export function timerOnce(): void {
  setTimeout(() => {
    timerLog += `t${Window.current.domain};`;
  }, 10);
}

/** A repeating timer in the current domain. */
export function timerEvery(): void {
  tickId = setInterval(() => {
    timerLog += `i${Window.current.domain};`;
  }, 10);
}

export function timerStop(): void {
  clearInterval(tickId);
}

/** A promise job queued in the current domain. */
export function microtaskHere(): void {
  Promise.resolve(0).then(() => {
    timerLog += `m${Window.current.domain};`;
  });
}

export function timerLogText(): string {
  return timerLog;
}

export function timerLogClear(): void {
  timerLog = "";
}

let failures: string[] = [];

export function createAndWatchOpen(): number {
  failures = [];
  const w = Window.create({ title: "Doomed" });
  w.onOpenFailed((e: string) => {
    failures.push(e);
  });
  return w.handle;
}

/** `failure count|first failure|isSupported|main has no error`. */
export function openReport(): string {
  return `${failures.length}|${failures[0] ?? "-"}|${Window.isSupported}|${Window.main.openError === null}`;
}

/** The open error of the second open window, or "gone" when there is none (a failed window closed). */
export function openErrorOfSecond(): string {
  const w = Window.at(1);
  if (w === undefined) return "gone";
  return `${w.openError}`;
}

// ---- Typed replies and named events ------------------------------------------------------------

interface Stats {
  n: number;
  tags: string[];
  inner: { ok: boolean; vals: number[] };
}

type Pair = { a: number; b: string };

let typedLog = "";

export function typedLogText(): string {
  return typedLog;
}

export function typedLogClear(): void {
  typedLog = "";
}

/** Answers a request with a declared interface, nested objects and arrays. */
export function serveStats(h: number): void {
  const found = byHandle(h);
  if (found === undefined) return;
  const w: Window = found;
  w.onRequest<{ kind: string }, Stats>((from, req, reply) => {
    reply({ n: req.kind.length, tags: [req.kind, "x"], inner: { ok: true, vals: [1, 2.5] } });
  });
}

export function servePair(h: number): void {
  const found = byHandle(h);
  if (found === undefined) return;
  const w: Window = found;
  w.onRequest<{ kind: string }, Pair>((from, req, reply) => {
    reply({ a: 4, b: req.kind });
  });
}

export function servePairs(h: number): void {
  const found = byHandle(h);
  if (found === undefined) return;
  const w: Window = found;
  w.onRequest<{ kind: string }, Pair[]>((from, req, reply) => {
    reply([{ a: 1, b: "one" }, { a: 2, b: "two" }]);
  });
}

export function serveNums(h: number): void {
  const found = byHandle(h);
  if (found === undefined) return;
  const w: Window = found;
  w.onRequest<{ kind: string }, number[]>((from, req, reply) => {
    reply([1, 2, 3]);
  });
}

export async function askStats(h: number): Promise<number> {
  const w = byHandle(h);
  if (w === undefined) return -1;
  const r = await w.request<Stats>({ kind: "stats" });
  typedLog += `iface ${r.n}|${r.tags.join(",")}|${r.inner.ok}|${r.inner.vals[1]};`;
  return 0;
}

export async function askPair(h: number): Promise<number> {
  const w = byHandle(h);
  if (w === undefined) return -1;
  const a = await w.request<Pair>({ kind: "pair" });
  typedLog += `alias ${a.a}|${a.b};`;
  return 0;
}

export async function askPairs(h: number): Promise<number> {
  const w = byHandle(h);
  if (w === undefined) return -1;
  const arr = await w.request<Pair[]>({ kind: "pairs" });
  typedLog += `arr ${arr.length}|${arr[1]!.b};`;
  return 0;
}

export async function askNums(h: number): Promise<number> {
  const w = byHandle(h);
  if (w === undefined) return -1;
  const nums = await w.request<number[]>({ kind: "nums" });
  typedLog += `nums ${nums.length};`;
  return 0;
}

export function serveNested(h: number): void {
  const found = byHandle(h);
  if (found === undefined) return;
  const w: Window = found;
  w.onRequest<{ kind: string }, { a: { b: number[]; c: string | null } }>((from, req, reply) => {
    reply({ a: { b: [7, 8], c: null } });
  });
}

export async function askNested(h: number): Promise<number> {
  const w = byHandle(h);
  if (w === undefined) return -1;
  const r = await w.request<{ a: { b: number[]; c: string | null } }>({ kind: "nested" });
  typedLog += `nested ${r.a.b.length}|${r.a.b[1]}|${r.a.c === null};`;
  return 0;
}

// ---- Named events ------------------------------------------------------------------------------

let eventLog = "";
let evToken = 0;

export function eventLogText(): string {
  return eventLog;
}

export function eventLogClear(): void {
  eventLog = "";
}

/** Listens for "stats" (a declared interface), "ping" (no payload) and "tags" (an array). */
export function listenEvents(h: number): void {
  const found = byHandle(h);
  if (found === undefined) return;
  const w: Window = found;
  evToken = w.on("stats", (s: Stats, from: Window) => {
    eventLog += `stats ${s.n}|${s.tags.join(",")}|${s.inner.vals[1]}|from ${from.handle};`;
  });
  w.on("ping", () => {
    eventLog += "ping;";
  });
  w.on<string[]>("tags", (t) => {
    eventLog += `tags ${t.length};`;
  });
}

export function stopStats(h: number): boolean {
  const w = byHandle(h);
  if (w === undefined) return false;
  return w.off(evToken);
}

export function emitStats(h: number): boolean {
  const w = byHandle(h);
  if (w === undefined) return false;
  const s: Stats = { n: 3, tags: ["a", "b"], inner: { ok: true, vals: [1, 2.5] } };
  return w.emit("stats", s);
}

export function emitPing(h: number): boolean {
  const w = byHandle(h);
  if (w === undefined) return false;
  return w.emit("ping");
}

export function emitTags(h: number): boolean {
  const w = byHandle(h);
  if (w === undefined) return false;
  return w.emit("tags", ["x", "y", "z"]);
}

export function emitOther(h: number): boolean {
  const w = byHandle(h);
  if (w === undefined) return false;
  return w.emit("stats2", { q: 1 });
}

export function emitEmpty(h: number): boolean {
  const w = byHandle(h);
  if (w === undefined) return false;
  return w.emit("", 1);
}

export function onEmpty(h: number): string {
  const w = byHandle(h);
  if (w === undefined) return "none";
  try {
    w.on("", (x: number) => {});
    return "no error";
  } catch (e) {
    return (e as Error).message;
  }
}

export function emitHuge(h: number): boolean {
  const w = byHandle(h);
  if (w === undefined) return false;
  let big = "";
  for (let i = 0; i < 20; i++) big += "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
  let huge = "";
  for (let i = 0; i < 16; i++) huge += big.repeat(1024);
  return w.emit("big", huge);
}

export function emitAllHi(): number {
  return Window.emitAll("hi", { n: 7 });
}

export function emitAllBare(): number {
  return Window.emitAll("bare");
}

/** Answers "stats" with a declared interface, "pair" with an anonymous type; "count" takes an array. */
export function answerEvents(h: number): void {
  const found = byHandle(h);
  if (found === undefined) return;
  const w: Window = found;
  w.answer("stats", (req: { kind: string }, from: Window): Stats => {
    return { n: req.kind.length, tags: [req.kind], inner: { ok: from.handle > 0, vals: [1, 2.5] } };
  });
  w.answer("pair", (req: { a: number }): { a: number; label: string } => {
    return { a: req.a * 2, label: "twice" };
  });
  w.answer<number[], number>("count", (xs) => xs.length);
  w.answer("whoami", (): string => "inspector");
}

export async function askEvents(h: number): Promise<number> {
  const w = byHandle(h);
  if (w === undefined) return -1;
  const s = await w.ask<Stats>("stats", { kind: "abc" });
  eventLog += `ask stats ${s.n}|${s.tags[0]}|${s.inner.ok};`;
  const p = await w.ask<{ a: number; label: string }>("pair", { a: 21 });
  eventLog += `ask pair ${p.a}|${p.label};`;
  const c = await w.ask<number>("count", [5, 6, 7, 8]);
  eventLog += `ask count ${c};`;
  const me = await w.ask<string>("whoami");
  eventLog += `ask whoami ${me};`;
  return 0;
}

/** Asks an unanswered name; the answerer is there, so it times out. */
export async function askUnknown(h: number): Promise<number> {
  const w = byHandle(h);
  if (w === undefined) return -1;
  try {
    await w.ask<number>("nobody", undefined, 50);
    eventLog += "answered;";
  } catch (e) {
    eventLog += `ask error ${(e as Error).message};`;
  }
  return 0;
}

export async function askHuge(h: number): Promise<number> {
  const w = byHandle(h);
  if (w === undefined) return -1;
  let huge = "";
  for (let i = 0; i < 1100; i++) huge += "x".repeat(1024);
  try {
    await w.ask<number>("count", huge);
  } catch (e) {
    eventLog += `ask error ${(e as Error).message};`;
  }
  return 0;
}
