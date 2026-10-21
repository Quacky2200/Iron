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
