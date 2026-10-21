// Windows: `Window.main` is the original window (title, size, events); `Window.create({ .. })` opens more,
// each with its own UI (`createRoot(win).render(<Panel />)` from `@iron/react`). The class itself is native:
// the app installs it as the global `Window`, with the same members compiled games have (design:
// docs/research/iron-windows.md). This module has the types and re-exports the global.

/** How a window looks and behaves: "normal" (own taskbar button), "tool" (owned palette, small caption, no taskbar
 *  button), "dialog", "popup" (undecorated, never takes focus), "borderless", "child" (embedded where the platform
 *  can, else "tool"). */
export type WindowStyle = "normal" | "tool" | "dialog" | "popup" | "borderless" | "child";

export interface WindowOptions {
  /** The title bar text. Default "". */
  title?: string;
  /** Client area size in UI pixels. Default 400 x 300. */
  width?: number;
  height?: number;
  /** Top-left of the outer frame in desktop UI pixels. Left out: centred on the parent, cascading. */
  x?: number;
  y?: number;
  minWidth?: number;
  minHeight?: number;
  /** Default "normal". */
  style?: WindowStyle;
  /** The window this one belongs to: it joins the parent's family and event domain. Tool, dialog, popup and child
   *  windows default to the main window; a "normal" or "borderless" window without a parent is the root of an
   *  isolated family with its own event domain. */
  parent?: Window | null;
  /** Root windows of isolated families only: keep the domain's timers running while unfocused. Default true. */
  runInBackground?: boolean;
  resizable?: boolean;
  minimizable?: boolean;
  maximizable?: boolean;
  /** Whether the title bar has a close button. */
  closable?: boolean;
  alwaysOnTop?: boolean;
  visible?: boolean;
  /** Takes keyboard focus when shown; popups never do. */
  focus?: boolean;
  /** A name that keeps the window across a hot reload and finds it again (`Window.find`). */
  key?: string;
}

/** One window (a handle: the OS window and its UI live in the engine). */
export interface Window {
  /** Iron's id: 1 for the main window, then 2, 3, ... never reused in a run. */
  readonly handle: number;
  /** The OS handle as a number (HWND, NSView pointer, X11 window id); 0 when there is none. */
  readonly nativeHandle: number;
  /** "win32", "appkit", "x11", "wayland", "virtual" (offscreen) or "" (not made yet). */
  readonly platform: string;
  readonly key: string;
  readonly isMain: boolean;
  /** False once closed (or when the open failed); a closed window's members read its last values. */
  readonly isOpen: boolean;
  /** Why the window could not be opened (`Cannot open Window due to platform restrictions (platform: android)`),
   *  null when it could. */
  readonly openError: string | null;
  /** The owner or embedding parent; undefined for the main window and unowned windows. */
  readonly parent: Window | undefined;
  /** True when the window has a parent. */
  readonly isChild: boolean;
  /** The root of its family (itself for a root). */
  readonly root: Window;
  /** True for the root of a family other than the main one: its own event domain. */
  readonly isIsolated: boolean;
  /** The event domain's id: the root's handle (1 for the main family). */
  readonly domain: number;
  readonly isEmbedded: boolean;
  readonly childCount: number;
  childAt(index: number): Window | undefined;

  title: string;
  style: WindowStyle;
  resizable: boolean;
  minimizable: boolean;
  maximizable: boolean;
  closable: boolean;
  alwaysOnTop: boolean;
  visible: boolean;
  /** Outer top-left, desktop UI pixels. */
  x: number;
  y: number;
  /** Client size, UI pixels. Setting asks the OS; the value changes when the OS answers (`onResized`). */
  width: number;
  height: number;
  setPosition(x: number, y: number): void;
  setSize(width: number, height: number): void;
  center(): void;
  /** Physical pixels per UI pixel. */
  readonly scale: number;
  minimized: boolean;
  maximized: boolean;
  readonly focused: boolean;
  focus(): void;
  /** Closes it (and its children first) without `onCloseRequested`; on the main window it quits the game. */
  close(): void;
  /** Called from an `onCloseRequested` listener: the window stays open. */
  cancelClose(): void;

  /** Each `on...` returns a token for `off`. */
  onOpened(f: (width: number, height: number, scale: number) => void): number;
  /** The window could not be opened (see `openError`); the window closes. `Window.create` itself never throws. */
  onOpenFailed(f: (error: string) => void): number;
  onCloseRequested(f: () => void): number;
  onClosed(f: () => void): number;
  onResized(f: (width: number, height: number) => void): number;
  onMoved(f: (x: number, y: number) => void): number;
  onFocusChanged(f: (focused: boolean) => void): number;
  onScaleChanged(f: (scale: number) => void): number;
  /** A message posted to this window. Runs in this window's domain. */
  onMessage<T = unknown>(f: (from: Window, data: T) => void): number;
  /** A request posted to this window; call `reply` once (a second call is ignored). */
  onRequest<T = unknown, R = unknown>(f: (from: Window, data: T, reply: (answer: R) => void) => void): number;
  off(token: number): boolean;

  /** Queues `data` (structured values only: no handles or functions) for this window's domain, delivered at the start
   *  of its next dispatch, in order per sender. False when refused (over 1 MiB, closed window). */
  postMessage(data: unknown): boolean;
  /** Runs `f` once in this window's domain at the start of its next dispatch (the setup of an isolated domain). */
  run(f: () => void): boolean;
  /** Posts `data` as a request and resolves with the reply; rejects when the window closes, nobody listens, or
   *  nobody replies within `timeoutMs` (default 10000). */
  request<R = unknown>(data: unknown, timeoutMs?: number): Promise<R>;

  /** Named events on top of the raw messages: sends event `name` (with an optional payload) to this window.
   *  Same limits as `postMessage` (1 MiB as JSON, no handles); returns false when refused. Throws on an empty name. */
  emit(name: string, payload?: unknown): boolean;
  /** Runs `f(payload, from)` for each event `name` posted to this window, in this window's domain. Other names are
   *  ignored. Returns a token for `off`. Compiled code decodes the payload as the handler's parameter type. */
  on<T = unknown>(name: string, f: (payload: T, from: Window) => void): number;
  /** Asks event `name` and resolves with what the window's `answer(name, ..)` handler returned. Rejects like
   *  `request` (window closed, no reply within `timeoutMs`, default 10000), or at once with `no answer for "name"`
   *  when the window has no `answer` for it. Events are a layer apart: raw `onMessage` / `onRequest` never see them. */
  ask<R = unknown>(name: string, payload?: unknown, timeoutMs?: number): Promise<R>;
  /** Answers the asks of event `name` with the handler's return value (JSON). Returns a token for `off`. */
  answer<T = unknown, R = unknown>(name: string, f: (payload: T, from: Window) => R): number;
}

/** The static side of `Window`. */
export interface WindowStatic {
  /** The original window, the one that owns the process; closing it ends the game. */
  readonly main: Window;
  /** The window with keyboard focus, if one of the game's has it (the engine answers a "none" handle as null or
   *  undefined). */
  readonly focused: Window | null | undefined;
  /** The window whose event is being handled now, else the main window. */
  readonly current: Window;
  /** Open windows, the main one first. */
  readonly count: number;
  /** Whether the platform can open secondary windows (false on iOS, Android and the web). */
  readonly isSupported: boolean;
  /** Whether the current event domain has focus. */
  readonly domainFocused: boolean;
  create(options?: WindowOptions): Window;
  at(index: number): Window | undefined;
  find(key: string): Window | undefined;
  byHandle(handle: number): Window | undefined;
  onDomainFocusChanged(f: (focused: boolean, paused: boolean) => void): number;
  /** Messages to any window of the current domain (after the target's own listeners). */
  onMessage<T = unknown>(f: (from: Window, data: T) => void): number;
  onRequest<T = unknown, R = unknown>(f: (from: Window, data: T, reply: (answer: R) => void) => void): number;
  /** Posts `data` to the root window of every other family. Returns how many were queued. */
  broadcast(data: unknown): number;
  /** `broadcast` of a named event (see `Window.emit`). */
  emitAll(name: string, payload?: unknown): number;
  off(token: number): boolean;
}

/** The engine's global `Window`. */
export const Window: WindowStatic = (globalThis as unknown as { Window: WindowStatic }).Window;
