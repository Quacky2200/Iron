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
