# Windows: a `Window` class for games (design)

Status: design, 2026-10-21. Steps: [iron-windows-plan.md](iron-windows-plan.md). Test IDs: `M9-API-1400..1499`,
`M9-TS-2800..2899`.

## 1. What the user asked for

- The **game** names its window. The manifest title (`iron.title` in `package.json` today, `title` in `project.json` once
  that lands) is only the title the window opens with; `Window.main.title = "..."` changes it at any time.
- Games make **more windows**: tool windows (palettes, inspectors) owned by the main window, dialogs, popups, borderless
  windows, each with its **own UI** (`createRoot(win).render(<Panel/>)`).
- A window can be asked for its parent (`win.parent`), whether it is a child (`win.isChild`), its handle (`win.handle`,
  `win.nativeHandle`), its style (get and set), and can be closed.
- An example, `examples/windows`, that creates and manages tool windows.

Constraints that shape the design:

- **GPU caution.** The user's PC has hard-reset with the Iron window open (an NVIDIA instability older than Iron). More windows
  means more swapchains. Every automated check in the plan is a fake-backend test or a `--screenshot` run, and screenshot runs
  make **no OS window for secondary windows** (they render offscreen, section 7.4). Nothing in the plan opens an interactive
  window.
- **Rust owns the world, JS owns behaviour.** Scripts hold `Window` handles (numbers inside); the OS windows, surfaces and UI
  trees live in Rust.
- **One API for both runtimes.** Declared once with `#[iron_api(namespace = "Window", quickjs, handles(Window = WindowRef))]`
  in `iron_engine_api`, like `Screen` and `Cursor`; compiled games (`iron_ts`) and QuickJS both get it.

## 2. Findings: what the stack gives us

### 2.1 winit 0.30.13 (Cargo.lock)

| Need | winit | Notes |
|---|---|---|
| Many windows, one loop | `ActiveEventLoop::create_window(attrs)` at any callback; `window_event(.., WindowId, ..)` | Creation needs the `ActiveEventLoop`, so scripts' requests are queued and run by the platform in its callbacks. |
| Title, size, position | `with_title`, `with_inner_size`, `with_min_inner_size`, `with_position`; `set_title`, `request_inner_size`, `set_outer_position`, `outer_position`, `inner_size` | Positions are of the outer frame, sizes of the client area. |
| Buttons, resizing | `with_resizable`, `with_enabled_buttons(WindowButtons)`; setters | |
| Always on top | `WindowLevel::{AlwaysOnTop, Normal, AlwaysOnBottom}` | Win32 `WS_EX_TOPMOST`; macOS `kCGFloatingWindowLevel`; X11 `_NET_WM_STATE_ABOVE`. |
| Borderless | `with_decorations(false)` | |
| Visible, minimise, maximise, focus | `set_visible`, `set_minimized`, `set_maximized`, `focus_window`, `has_focus`; `with_active(false)` opens without taking focus | |
| **Owner (Windows)** | `WindowAttributesExtWindows::with_owner_window(HWND)` | Owned windows stay above the owner, hide when it is minimised, are destroyed with it, and have no taskbar button. |
| Taskbar | `with_skip_taskbar(bool)` / `set_skip_taskbar` (Windows, `ITaskbarList::DeleteTab`) | Not on X11, Wayland or macOS. |
| Modal (Windows) | `WindowExtWindows::set_enable(false)` on the owner | Later (D2). |
| **Parent** | `unsafe with_parent_window(Option<RawWindowHandle>)` | Windows: `WS_CHILD`, confined to the parent's client area. X11: reparented inside the parent. **macOS: `addChildWindow:ordered:`** (an owned floating window that moves with its parent, not an embedded one). Wayland, iOS, Android, web: unsupported. |
| X11 window types | `WindowAttributesExtX11::with_x11_window_type(vec![WindowType::Utility])` | No `WM_TRANSIENT_FOR` in winit 0.30, so X11 tool windows are not tied to the main window. |
| Wayland | nothing for parents | No `xdg_toplevel.set_parent` in winit 0.30; tool windows are plain toplevels. |
| macOS panels | no `NSPanel`; `WindowAttributesExtMacOS` has title bar options only | `NSWindowStyleMaskUtilityWindow` needs an `NSPanel`; out of scope. |
| Events | `CloseRequested`, `Destroyed`, `Resized`, `Moved`, `Focused`, `ScaleFactorChanged`, `Occluded`, keyboard, mouse, IME, file drops | All per `WindowId`. |
| Per-window DPI | `Window::scale_factor()` and `ScaleFactorChanged` per window | |
| IME | `set_ime_allowed`, `set_ime_cursor_area` per window | |
| Cursor | `set_cursor`, `set_cursor_grab` per window | |

**`WS_EX_TOOLWINDOW` needs raw Win32.** winit has no option for the small tool caption. Worse, winit rewrites
`GWL_EXSTYLE` from its own flags in `WindowFlags::apply_diff` (`window_state.rs`: `SetWindowLongW(window, GWL_EXSTYLE,
style_ex)`) every time a flag changes (`set_visible`, `set_resizable`, maximise, decorations...), so setting the style once
after creation would be undone at the first `set_visible(true)`. The fix: **subclass the window** with
`SetWindowSubclass` (comctl32, `windows-sys` feature `Win32_UI_Shell`, already in the registry at 0.52) and, in
`WM_STYLECHANGING` for `GWL_EXSTYLE`, OR `WS_EX_TOOLWINDOW` into `STYLESTRUCT.styleNew` (and drop `WS_EX_APPWINDOW`). The
style is applied once with `SetWindowLongPtrW` + `SetWindowPos(SWP_FRAMECHANGED | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER |
SWP_NOACTIVATE)` and the subclass keeps it. `iron_platform` already uses `windows-sys` 0.52 (winit's version) for the black
class brush; the plan adds the `Win32_UI_Shell` feature. No new crate.

Alt-Tab and the taskbar on Windows: a window shows in both when it is unowned or has `WS_EX_APPWINDOW`, and is hidden from both
when it has `WS_EX_TOOLWINDOW`. An owned window without `WS_EX_APPWINDOW` already has no taskbar button; `WS_EX_TOOLWINDOW`
also removes it from Alt-Tab and gives the small caption. `with_skip_taskbar(true)` is set too, harmlessly, for the cases winit
re-adds `WS_EX_APPWINDOW` (it does when `ON_TASKBAR` is set).

### 2.2 wgpu 30 and `iron_render`

- `Gpu` holds one `Instance`, `Adapter`, `Device`, `Queue`; `Gpu::with_surface` makes the instance, the first surface and the
  adapter together. Further surfaces come from `gpu.instance.create_surface(Arc<Window>)`; `adapter.is_surface_supported` checks
  them (on desktop one adapter drives every window on its monitors; a window moved to a monitor on another GPU still works through
  the OS's copy path).
- `WindowSurface` (`surface.rs`) already tolerates 0 x 0 (minimised) and outdated surfaces and owns its present mode and
  `desired_maximum_frame_latency` (1, or 2 on DX12). Each secondary window gets one more `WindowSurface`.
- `UiRenderer` already renders into targets of different formats (the screen, panel textures in `Rgba8UnormSrgb`, offscreen
  screenshots), so **one `UiRenderer` and one `WorldRenderer` serve every window**: glyph atlas, images and pipelines are shared.
- **Frame pacing.** Vsync is off by default (`AutoNoVsync`: mailbox or immediate), so presenting N swapchains in a row does not
  block. With vsync on, presenting several FIFO swapchains in one thread can wait a vblank per window on some drivers (the frame
  rate divides by N). So **secondary windows always present with vsync off**; the main window follows
  `QualitySettings.vSyncCount` as today. Secondary windows draw once per main frame; a minimised, hidden or 0 x 0 window is
  skipped by `WindowSurface` itself.

### 2.3 Iron today (base `c3ffc1b`)

- `iron_platform::Platform::run(WindowConfig, app)` makes **one** window in `resumed`, hidden until its first frame, and calls
  `PlatformApp::{window_ready, input, frame, ...}`. `window_event` ignores the `WindowId`; `CloseRequested` exits the loop.
- `iron_app::App` has one `Gfx { gpu, surface, window, renderer, world }`, the screen `UiTree` (`self.ui`) with its
  `Interaction`, and `Surface::{Screen, Panel(id)}` routing for the pointer and keys between the screen and world panels.
- World panels (`panels.rs`) are the model for per-window UI: their own `UiTree`, `Interaction`, stylesheet and font copies
  (`new_panel_tree`), QuickJS commits with a target id, and for compiled games their own `iron_ui_native::Runtime`
  (`native_render.rs`) fed by `UiRoot.mount(panel, element)` (`iron_ts_ui::mount`).
- The `Screen` API (`iron_engine_api::screen`) is per thread with host functions (`update`, `set_display`, `take_requests`).
  The `Window` module copies that pattern.
- `--screenshot` renders the main window offscreen at its size and writes one PNG.

## 3. The TypeScript API

What scripts see (the generated `.d.ts` plus the hand-written supplement in `engine_ts.d.ts`). The QuickJS column of section 6
lists where QuickJS differs.

```ts
/** How a window looks and behaves; see the table in section 4. */
type WindowStyle = "normal" | "tool" | "dialog" | "popup" | "borderless" | "child";

interface WindowOptions {
  /** The title bar text (and the taskbar's, for normal windows). Default "". */
  title?: string;
  /** Client area size in UI pixels (logical, before DPI). Default 400 x 300. */
  width?: number;
  height?: number;
  /** Top-left of the outer frame in desktop UI pixels. Left out: centred on the parent (or the main window),
   *  each further window 24 px lower and to the right, so new windows don't stack exactly. */
  x?: number;
  y?: number;
  /** Smallest client size the user can drag to. Default none. */
  minWidth?: number;
  minHeight?: number;
  /** Default "normal". */
  style?: WindowStyle;
  /** The owner (tool, dialog, popup) or the embedding parent ("child"). Default: the main window for
   *  "tool", "dialog", "popup" and "child"; none for "normal" and "borderless". */
  parent?: Window | null;
  /** Default true. Styles that have no such button ignore it (a tool window is never minimisable). */
  resizable?: boolean;
  minimizable?: boolean;
  maximizable?: boolean;
  /** Whether the title bar has a close button. Default true. */
  closable?: boolean;
  /** Above every normal window, not only its owner. Default false. */
  alwaysOnTop?: boolean;
  /** Shown once its first frame is drawn. Default true. */
  visible?: boolean;
  /** Takes keyboard focus when shown. Default true; popups never do. */
  focus?: boolean;
  /** A name that keeps the window across a hot reload (section 9) and finds it again (`Window.find`). Default "". */
  key?: string;
}

declare class Window {
  private constructor();

  // ---- The windows there are ----
  /** The game's first window: always there while the game runs. */
  static readonly main: Window;
  /** The window with keyboard focus, if one of the game's has it. */
  static readonly focused: Window | undefined;
  /** The window whose event is being handled now (a UI handler, a window callback), else the main window.
   *  A component shared by several windows' UIs uses it to know where it is. */
  static readonly current: Window;
  /** Open windows, the main one first, then in creation order. */
  static readonly count: number;
  static at(index: number): Window | undefined;
  /** The open window made with this `key`. */
  static find(key: string): Window | undefined;

  /** Opens a window. It exists (and has a handle) at once; the OS window appears at the end of the frame.
   *  Until then its size and position are the ones asked for. */
  static create(options?: WindowOptions): Window;

  // ---- Identity ----
  /** Iron's id: 1 for the main window, then 2, 3, ... never reused in a run. */
  readonly handle: number;
  /** The OS handle as a number: HWND on Windows, the NSView pointer on macOS, the X11 window id; 0 when there is
   *  none (a screenshot run's offscreen window, Wayland, before the OS window exists). For passing to native plugins. */
  readonly nativeHandle: number;
  /** "win32", "appkit", "x11", "wayland", "virtual" (offscreen) or "" (not made yet). */
  readonly platform: string;
  readonly key: string;
  readonly isMain: boolean;
  /** False once closed; a closed window's members read its last values and its setters do nothing. */
  readonly isOpen: boolean;

  // ---- Family ----
  /** The owner or embedding parent; undefined for the main window and unowned windows. */
  readonly parent: Window | undefined;
  /** True when the window belongs to another one (`parent` is set): tool windows, dialogs, popups, children. */
  readonly isChild: boolean;
  /** True for style "child" only: drawn inside its parent's client area (where the platform can). */
  readonly isEmbedded: boolean;
  /** Open windows whose parent this is. */
  readonly childCount: number;
  childAt(index: number): Window | undefined;

  // ---- Look ----
  /** Changing it changes the OS title. The main window opens with the manifest title. */
  title: string;
  /** Changing it re-applies decorations, taskbar and z-order rules; "child" cannot be set or unset after creation. */
  style: WindowStyle;
  resizable: boolean;
  minimizable: boolean;
  maximizable: boolean;
  closable: boolean;
  alwaysOnTop: boolean;
  visible: boolean;

  // ---- Place ----
  /** Outer top-left, desktop UI pixels (physical / this window's scale). */
  x: number;
  y: number;
  /** Client size, UI pixels. Setting asks the OS; the value changes when the OS answers (`onResized`). */
  width: number;
  height: number;
  setPosition(x: number, y: number): void;
  setSize(width: number, height: number): void;
  /** Centres it on its parent, or on the main window's monitor. */
  center(): void;
  /** Physical pixels per UI pixel (the monitor's DPI scale). */
  readonly scale: number;
  minimized: boolean;
  maximized: boolean;

  // ---- Focus and closing ----
  readonly focused: boolean;
  /** Brings it to the front with keyboard focus (the OS may only flash it if another program is active). */
  focus(): void;
  /** Closes it (and its children first) without `onCloseRequested`. On the main window it quits the game. */
  close(): void;
  /** Called from an `onCloseRequested` listener: the window stays open. Ignored elsewhere. */
  cancelClose(): void;

  // ---- Events (each returns a token for `off`) ----
  /** The user asked to close it (title bar button, Alt+F4, Cmd+W). Listeners may call `cancelClose()`. */
  onCloseRequested(f: () => void): number;
  /** It closed, by `close()`, the user, or its parent closing. Called once. */
  onClosed(f: () => void): number;
  onResized(f: (width: number, height: number) => void): number;
  onMoved(f: (x: number, y: number) => void): number;
  onFocusChanged(f: (focused: boolean) => void): number;
  onScaleChanged(f: (scale: number) => void): number;
  /** Removes any of the listeners above by token. */
  off(token: number): boolean;
}

/** `createRoot(target)`: the screen (no argument or `Window.main`), a world panel, or another window. */
declare function createRoot(target?: WorldPanel | Window | number): { render(node: ReactNode): void; unmount(): void };
```

Usage:

```tsx
Window.main.title = `My Game (level ${level})`;

const palette = Window.create({ title: "Palette", style: "tool", width: 220, height: 320, key: "palette" });
createRoot(palette).render(<Palette />);
palette.onCloseRequested(() => { palette.cancelClose(); palette.visible = false; }); // hide instead of closing

if (palette.isChild) console.log(`owned by ${palette.parent?.title}`);
```

Rules:

- **`Window.main` is never closed by scripts**: `Window.main.close()` is `Application.quit()`.
- **Screen API stays the main window's.** `Screen.width` / `height` / `fullScreen` / `setResolution` keep working on the main
  window only; a tool window's size is `win.width` / `height`. Fullscreen secondary windows are out of scope.
- **Units.** Sizes and positions are UI pixels (winit `Logical*` with that window's scale factor). With monitors of different
  DPI, a position is converted with the scale of the monitor the window is on, as winit does.
- **Handles.** `WindowRef(u32)` as raw `u64`; 1 is the main window and ids grow from 2 (`Handle` rule: never 0).

## 4. Styles per platform

| Style | Meaning | Windows | macOS | X11 | Wayland |
|---|---|---|---|---|---|
| `normal` | An ordinary top-level window with its own taskbar button | default attributes | default | default | default |
| `tool` | Palette or inspector: small caption, no taskbar button, not in Alt-Tab, stays above its owner, hidden while the owner is minimised, closed with it | `with_owner_window(owner)`, `with_skip_taskbar(true)`, `WS_EX_TOOLWINDOW` (subclass, 2.1), buttons: close only | `with_parent_window(owner AppKit handle)` = `addChildWindow` (above the owner, moves with it); normal caption | `with_x11_window_type([Utility])`; not tied to the owner | plain toplevel |
| `dialog` | A question or settings box over its owner: full caption, no taskbar button, not resizable by default | `with_owner_window`, `with_skip_taskbar(true)`, buttons: close | child window as `tool` | `[Dialog]` | plain |
| `popup` | Undecorated owned window (menus, tooltips, splash); does not take focus by default | `with_owner_window`, `with_decorations(false)`, `with_skip_taskbar(true)`, `WS_EX_TOOLWINDOW`, `with_active(false)` | child window, no decorations | `[PopupMenu]`, no decorations | plain, no decorations |
| `borderless` | Undecorated top-level window with a taskbar button (launchers, overlays) | `with_decorations(false)` | same | same | same |
| `child` | Embedded in the parent's client area, moving and clipping with it | `with_parent_window(Win32)` = `WS_CHILD` | **not available**: falls back to `tool` | `with_parent_window(Xlib)` | **not available**: falls back to `tool` |

- A fallback logs one warning naming the style and the platform; `isEmbedded` is then false.
- `child` is the riskiest (a second swapchain over the parent's; airspace and flicker issues on some drivers) and only matters
  for embedding (the editor's Scene view could use it). It is in the API but built last, behind a user decision (D1).
- **`alwaysOnTop`** adds `WindowLevel::AlwaysOnTop` to any style.
- **Transparent windows** are out of scope for now (`with_transparent` plus a non-opaque `alpha_mode` on the surface); the
  option is not in `WindowOptions` until built (D4).

## 5. The Rust `#[iron_api]` shape (`crates/iron_engine_api/src/window.rs`)

Per-thread state with host functions, like `screen.rs`. No winit types: the app translates.

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WindowRef(pub u32);
impl Handle for WindowRef { /* to_raw: u64::from(self.0); from_raw: WindowRef(raw as u32) */ }
pub const MAIN: WindowRef = WindowRef(1);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Style { #[default] Normal, Tool, Dialog, Popup, Borderless, Child }
impl Style { pub fn parse(s: &str) -> Option<Style>; pub fn name(self) -> &'static str; }

#[derive(Clone, Debug, PartialEq)]
pub struct Flags { pub resizable: bool, pub minimizable: bool, pub maximizable: bool, pub closable: bool,
                   pub always_on_top: bool, pub visible: bool, pub focus_on_show: bool }

#[derive(Clone, Debug, PartialEq)]
pub struct WindowState {
    pub key: String, pub title: String, pub style: Style, pub parent: Option<WindowRef>,
    /// Outer top-left (None: let the host place it) and client size, UI pixels.
    pub position: Option<(f64, f64)>, pub size: (f64, f64), pub min_size: Option<(f64, f64)>,
    pub scale: f64, pub flags: Flags,
    pub minimized: bool, pub maximized: bool, pub focused: bool,
    /// The host made the OS (or offscreen) window.
    pub created: bool, pub open: bool,
    pub native_handle: u64, pub platform: String,
    /// Creation order, for `count` / `at` and the cascade offset.
    pub order: u32,
}

/// What scripts asked for since the last `take_requests`, in order. Setters of the same field on the same window
/// coalesce (the last value wins) except `Create` and `Close`, which keep their place.
#[derive(Clone, Debug, PartialEq)]
pub enum WindowRequest {
    Create(WindowRef),               // read the spec with `state(w)`
    Title(WindowRef, String),
    Style(WindowRef, Style),
    Position(WindowRef, f64, f64),
    Size(WindowRef, f64, f64),
    Center(WindowRef),
    Flags(WindowRef, Flags),
    Minimized(WindowRef, bool),
    Maximized(WindowRef, bool),
    Focus(WindowRef),
    Close(WindowRef),                // the host closes children first and reports each with `closed`
}

/// What the host learned when it made a window.
#[derive(Clone, Debug, PartialEq)]
pub struct Opened { pub position: (f64, f64), pub size: (f64, f64), pub scale: f64,
                    pub native_handle: u64, pub platform: String }

// ---- Host functions ----
pub fn set_main(title: &str, size: (f64, f64), scale: f64);      // at startup, before scripts run
pub fn take_requests() -> Vec<WindowRequest>;
pub fn state(w: WindowRef) -> Option<WindowState>;
pub fn open_windows() -> Vec<WindowRef>;                          // main first, creation order
pub fn opened(w: WindowRef, info: Opened);
pub fn open_failed(w: WindowRef, error: &str);                   // logs, then as `closed`
pub fn resized(w: WindowRef, width: f64, height: f64);           // runs onResized when changed
pub fn moved(w: WindowRef, x: f64, y: f64);
pub fn focus_changed(w: WindowRef, focused: bool);
pub fn scale_changed(w: WindowRef, scale: f64);
pub fn minimized_changed(w: WindowRef, minimized: bool);
/// The user asked to close `w`: runs onCloseRequested; true unless a listener called `cancelClose()`.
pub fn close_requested(w: WindowRef) -> bool;
/// `w` is gone: marks it closed, runs onClosed once, then drops its listeners.
pub fn closed(w: WindowRef);
/// While the host dispatches an event of `w` (UI or window callback): `Window.current` is `w`.
pub fn enter_current(w: WindowRef) -> CurrentGuard;
pub fn reset();                                                  // tests: only the main window, no listeners
pub fn reload_begin() -> Vec<WindowRef>;   // hot reload, section 9: drops listeners; returns keyless windows to close
pub fn reload_end() -> Vec<WindowRef>;     // keyed windows the new code did not claim: to close

#[iron_api(namespace = "Window", quickjs, handles(Window = WindowRef))]
pub mod window_api {
    use super::*;
    impl WindowRef {
        #[options]   // QuickJS: the optional parameters may come as one object (plan step 3)
        #[optional(title, width, height, x, y, min_width, min_height, style, parent,
                   resizable = true, minimizable = true, maximizable = true, closable = true,
                   always_on_top = false, visible = true, focus = true, key)]
        pub fn create(title: &str, width: f64, height: f64, x: f64, y: f64, min_width: f64, min_height: f64,
                      style: &str, parent: WindowRef, resizable: bool, minimizable: bool, maximizable: bool,
                      closable: bool, always_on_top: bool, visible: bool, focus: bool, key: &str) -> WindowRef;
        pub fn main() -> WindowRef;                 // `Window.main` (the facade reads a no-argument static as a property)
        pub fn focused() -> WindowRef;              // 0 = none
        pub fn current() -> WindowRef;
        pub fn count() -> u32;
        pub fn at(index: f64) -> WindowRef;         // 0 past the end
        pub fn find(key: &str) -> WindowRef;
        #[property] pub fn handle(self) -> f64;
        #[property] pub fn native_handle(self) -> f64;
        #[property] pub fn platform(self) -> String;
        #[property] pub fn key(self) -> String;
        #[property] pub fn is_main(self) -> bool;
        #[property] pub fn is_open(self) -> bool;
        #[property] pub fn parent(self) -> WindowRef;
        #[property] pub fn is_child(self) -> bool;
        #[property] pub fn is_embedded(self) -> bool;
        #[property] pub fn child_count(self) -> u32;
        pub fn child_at(self, index: f64) -> WindowRef;
        #[property] pub fn title(self) -> String;           pub fn set_title(self, t: &str);
        #[property] pub fn style(self) -> String;           pub fn set_style(self, s: &str);
        #[property] pub fn resizable(self) -> bool;         pub fn set_resizable(self, v: bool);
        #[property] pub fn minimizable(self) -> bool;       pub fn set_minimizable(self, v: bool);
        #[property] pub fn maximizable(self) -> bool;       pub fn set_maximizable(self, v: bool);
        #[property] pub fn closable(self) -> bool;          pub fn set_closable(self, v: bool);
        #[property] pub fn always_on_top(self) -> bool;     pub fn set_always_on_top(self, v: bool);
        #[property] pub fn visible(self) -> bool;           pub fn set_visible(self, v: bool);
        #[property] pub fn x(self) -> f64;                  pub fn set_x(self, v: f64);
        #[property] pub fn y(self) -> f64;                  pub fn set_y(self, v: f64);
        #[property] pub fn width(self) -> f64;              pub fn set_width(self, v: f64);
        #[property] pub fn height(self) -> f64;             pub fn set_height(self, v: f64);
        pub fn set_position(self, x: f64, y: f64);
        pub fn set_size(self, width: f64, height: f64);
        pub fn center(self);
        #[property] pub fn scale(self) -> f64;
        #[property] pub fn minimized(self) -> bool;         pub fn set_minimized(self, v: bool);
        #[property] pub fn maximized(self) -> bool;         pub fn set_maximized(self, v: bool);
        #[property] pub fn focused_now(self) -> bool;  // #[js_name = "focused"]: instance getter vs static `focused()`
        pub fn focus(self);
        pub fn close(self);
        pub fn cancel_close(self);
        pub fn on_close_requested(self, f: Callback<()>) -> u32;
        pub fn on_closed(self, f: Callback<()>) -> u32;
        pub fn on_resized(self, f: Callback<(f64, f64)>) -> u32;
        pub fn on_moved(self, f: Callback<(f64, f64)>) -> u32;
        pub fn on_focus_changed(self, f: Callback<(bool,)>) -> u32;
        pub fn on_scale_changed(self, f: Callback<(f64,)>) -> u32;
        pub fn off(self, token: f64) -> bool;
    }
}
```

Notes:

- **`focused` twice.** The static `Window.focused` and the instance `win.focused` share a script name. The macro picks by
  receiver (statics are on the class, instance getters on the prototype), but Rust needs two names: the instance getter is
  `focused_now` with `#[js_name = "focused"]`. The plan's first test checks both resolve (`M9-TS-2802`); if the manifest
  rejects a static and an instance member with one name, the instance one becomes `isFocused` and the d.ts says so.
- **Validation.** Unknown style names are ignored with a warning (`create` falls back to `normal`); sizes are clamped to
  `[1, 16384]`; a `parent` that is closed or the window itself is ignored; `child` set after creation is ignored.
- **Closed windows.** Getters read the last state; setters and `focus` do nothing; `on*` returns 0.
- **Events are synchronous** from the host's calls, emitted after the state borrow is released from a snapshot
  (`iron_api::callback::{Listeners, emit}`), so listeners may call the API, open windows or close them.
- **Pending windows.** Between `create` and `opened`, `created` is false, `nativeHandle` 0, `platform` "", and the size and
  position read back as asked for (or the cascade position the API computes, so scripts see final numbers at once:
  `position: None` is resolved by the host; the API answers the parent's centre until then).

## 6. Runtimes

| | Compiled (`iron_ts`) | QuickJS |
|---|---|---|
| Declarations | Manifest + `engine_ts.d.ts` supplement: `declare namespace Window { const main: Window; const focused: Window \| undefined; const current: Window; const count: number; function create(options?: WindowOptions): Window; }`, `interface WindowOptions`, `type WindowStyle` | Same globals (installed by `window_api::register_quickjs`) |
| `Window.main` | `facade.rs::ext_static_prop`: a no-argument static read as a property (as `Camera.main`) | A function: `Window.main()` (QuickJS has no static properties on handle classes; listed in `tools/iron_ts/TODO.md` with `AudioListener.volume`) |
| `Window.create({..})` | `call_ext_args` maps a trailing object literal onto the optional parameters by name, honouring the manifest defaults | New `#[options]` attribute (plan step 3): a single plain object at the first optional position is read by camelCase name |
| `createRoot(win)` | `UiRoot.mount(win, el)`: a third overload `ts_ui_mount_window(window: handle:Window, element)` into its own queue, `iron_ts_ui::take_window_mounts()` | `@iron/react` `createRoot(target)`: a `Window` object (it has `handle`) becomes the target id `iron_core::WINDOW_TARGET \| handle`; the main window is target 0 |
| Callbacks | `Callback<..>` from compiled closures | `Callback<..>` from JS functions (`iron_api::qjs::keep_function`) |
| Hot reload | `Native::reload` | QuickJS reload in `reload_scripts` |

QuickJS games run the `@iron/engine` path and the app does not install `iron_native` there. The plan installs **only**
`window_api::register_quickjs` into the app's QuickJS runtime (`QuickJsRuntime::add_native`), so bun-built games get the same
global `Window`; `js/engine` adds `window.ts` (types and a re-export of `globalThis.Window`).

## 7. Platform abstraction (`iron_platform`)

### 7.1 Types (`crates/iron_platform/src/windows.rs`, no winit in the pure part)

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WindowKey(pub u32);
impl WindowKey { pub const MAIN: WindowKey = WindowKey(1); }

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum WindowKind { #[default] Normal, Tool, Dialog, Popup, Borderless, Child }

#[derive(Clone, Debug, PartialEq)]
pub struct WindowSpec {
    pub title: String,
    pub size: (f64, f64),                 // logical client size
    pub position: Option<(f64, f64)>,     // logical outer top-left; None: placed by `place`
    pub min_size: Option<(f64, f64)>,
    pub kind: WindowKind,
    pub parent: Option<WindowKey>,
    pub resizable: bool, pub minimizable: bool, pub maximizable: bool, pub closable: bool,
    pub always_on_top: bool, pub visible: bool, pub active: bool,
    pub icon: Option<(Vec<u8>, u32, u32)>,
}

/// What a spec means on this OS, decided without a display (testable).
#[derive(Clone, Debug, PartialEq)]
pub struct NativePlan {
    pub kind: WindowKind,                 // after fallbacks (`Child` -> `Tool` on macOS / Wayland)
    pub decorations: bool, pub skip_taskbar: bool, pub tool_caption: bool,   // WS_EX_TOOLWINDOW
    pub owner: Option<WindowKey>,         // Windows owner / macOS child window
    pub embed_in: Option<WindowKey>,      // WS_CHILD / X11 reparent
    pub level_on_top: bool, pub active: bool,
    pub buttons: (bool, bool, bool),      // close, minimise, maximise
    pub resizable: bool,
    pub x11_type: Option<&'static str>,   // "utility", "dialog", "popupmenu"
    pub warning: Option<String>,
}
pub fn plan(spec: &WindowSpec, os: Os) -> NativePlan;     // `Os::{Windows, MacOs, X11, Wayland, Other}`; `Os::current()`

/// Where a window with no position goes: centred on the parent (outer pos + (parent size - size) / 2),
/// plus 24 px x (n % 8) for the n-th open window of that parent.
pub fn place(parent_pos: (f64, f64), parent_size: (f64, f64), size: (f64, f64), n: u32) -> (f64, f64);

/// Parent links of open windows: close order and family questions.
#[derive(Default)]
pub struct WindowTree { /* key -> parent */ }
impl WindowTree {
    pub fn insert(&mut self, key: WindowKey, parent: Option<WindowKey>);  // a parent that would make a cycle is dropped
    pub fn remove(&mut self, key: WindowKey);
    /// `key` and everything under it, deepest first (the order surfaces must be dropped).
    pub fn close_order(&self, key: WindowKey) -> Vec<WindowKey>;
    pub fn children(&self, key: WindowKey) -> Vec<WindowKey>;
    pub fn parent(&self, key: WindowKey) -> Option<WindowKey>;
}

pub enum WindowCommand {
    Open(WindowKey, WindowSpec),
    SetTitle(WindowKey, String), SetSize(WindowKey, f64, f64), SetPosition(WindowKey, f64, f64), Center(WindowKey),
    SetKind(WindowKey, WindowKind), SetFlags(WindowKey, WindowSpec /* flag fields read */),
    SetMinimized(WindowKey, bool), SetMaximized(WindowKey, bool), Focus(WindowKey),
    Close(WindowKey),
}

/// The window behind a key: an OS window, or an offscreen one (screenshot runs, the fake platform).
#[derive(Clone)]
pub enum WindowHost { Os(Arc<winit::window::Window>), Virtual { width: u32, height: u32, scale: f64 } }
```

### 7.2 `PlatformApp` additions (default methods: one-window apps compile and behave as before)

```rust
/// Window commands for the platform to run on the event loop (after input, after each frame).
fn take_window_commands(&mut self) -> Vec<WindowCommand> { Vec::new() }
/// A window from `WindowCommand::Open` exists. Physical size is in the host.
fn window_opened(&mut self, _key: WindowKey, _host: WindowHost) {}
fn window_open_failed(&mut self, _key: WindowKey, _error: String) {}
/// An event of any window. The default sends the main window's to `input` and drops the rest.
fn window_input(&mut self, key: WindowKey, event: InputEvent) { if key == WindowKey::MAIN { self.input(event) } }
/// The user asked to close `key`; true lets it close. Closing the main window ends `run`.
fn close_requested(&mut self, _key: WindowKey) -> bool { true }
/// `key` is closing: drop its surface and every clone of its OS window now, before the platform drops its own.
/// Called deepest-first for a window's children.
fn window_closed(&mut self, _key: WindowKey) {}
/// Called once per loop iteration after all queued events (focus is settled then, see 8.3).
fn events_done(&mut self) {}
/// Windows whose first frame is drawn: the platform shows them (they are made hidden).
fn take_windows_to_show(&mut self) -> Vec<WindowKey> { Vec::new() }
```

New `InputEvent` variants: `Moved { x: i32, y: i32 }` (physical outer position) and `ScaleChanged(f64)`.
`convert_window_event` maps `WindowEvent::Moved` and `ScaleFactorChanged`.

### 7.3 The winit runner

`Runner` keeps `ids: HashMap<WindowId, WindowKey>`, `windows: BTreeMap<WindowKey, Arc<Window>>`, `tree: WindowTree`.

- `resumed` makes the main window as today (key `MAIN`, `window_ready`).
- `window_event(id, ..)` looks up the key; `RedrawRequested` of the main window runs `frame()` as today, of others nothing
  (they draw each main frame); `CloseRequested` asks `close_requested(key)`; if true, closes (below); the main window then
  exits the loop. `Destroyed` of a window we did not close (the OS destroyed an owned window with its owner) runs the same
  close path for that key.
- After input handling (`about_to_wait`, before requesting the redraw) and after each frame, `run_commands(event_loop)` drains
  `take_window_commands()`:
  - `Open`: `plan(spec, Os::current())`, then `attributes(spec, plan, owner_window)`: `WindowAttributes` (title, inner size,
    min size, position, resizable, buttons, decorations, level, `with_visible(false)`, `with_active`) plus, on Windows,
    `with_owner_window(owner hwnd)` / `with_skip_taskbar`, or `unsafe with_parent_window(Some(owner.window_handle().as_raw()))`
    on macOS (owned) and for embedded children; on X11 `with_x11_window_type`. Then `create_window`, `set_black_background`,
    `win32::set_tool_window(&window, true)` when `plan.tool_caption`, then `window_opened(key, WindowHost::Os(arc))`. The
    window stays hidden; after each frame the runner calls `take_windows_to_show()` and runs `set_visible(true)` on those
    (the app lists a window once its first frame was presented and `spec.visible` is set), so a new window never flashes
    its blank background, as the main window does today.
  - Setters call the winit setters. `SetKind` recomputes the plan and applies decorations, level, skip taskbar and the tool
    style (`win32::set_tool_window(&w, on)`); owner changes need recreating the window and are not supported (the request is
    ignored with a warning).
  - `Close(key)`: `tree.close_order(key)`; for each `k`: `app.window_closed(k)`, remove from maps (dropping our `Arc`, which
    destroys the OS window). The main window never closes this way (`Application.quit` covers it).
- **Screenshot runs** (`WindowConfig::virtual_windows = true`): `Open` makes no OS window; it calls
  `window_opened(key, WindowHost::Virtual { width, height, scale: main's scale })` with the physical size, and setters update
  the virtual record and echo `Resized` / `Moved` to `window_input`.

`win32` helpers (`#[cfg(windows)]`, in `windows.rs` or `platform.rs` next to `set_black_background`):

```rust
/// Small caption, no taskbar button, not in Alt-Tab; kept through winit's style rewrites by a subclass that adds
/// WS_EX_TOOLWINDOW (and removes WS_EX_APPWINDOW) in WM_STYLECHANGING for GWL_EXSTYLE.
pub(crate) fn set_tool_window(window: &Window, on: bool);
```

`SetWindowSubclass(hwnd, Some(tool_proc), TOOL_SUBCLASS_ID, 0)`; `tool_proc` handles `WM_STYLECHANGING` with `wparam ==
GWL_EXSTYLE`, and `WM_NCDESTROY` by `RemoveWindowSubclass`; everything goes on to `DefSubclassProc`. `on = false` removes the
subclass and clears the bit the same way.

### 7.4 The fake platform (`crates/iron_platform/src/fake.rs`)

Drives any `PlatformApp` without winit, a display or a GPU, for unit tests:

```rust
pub struct FakePlatform<A: PlatformApp> { app: A, windows: BTreeMap<WindowKey, FakeWindow>, tree: WindowTree, exited: bool }
pub struct FakeWindow { pub spec: WindowSpec, pub plan: NativePlan, pub physical: (u32, u32), pub scale: f64,
                        pub focused: bool, pub minimized: bool, pub maximized: bool }
impl<A: PlatformApp> FakePlatform<A> {
    pub fn new(app: A, main: WindowSpec, scale: f64) -> Self;  // opens MAIN as Virtual (window_opened, not window_ready)
    pub fn os(self, os: Os) -> Self;                            // which platform's `plan` to apply (default Windows)
    pub fn step(&mut self);              // run_commands, frame(), run_commands, events_done()
    pub fn send(&mut self, key: WindowKey, event: InputEvent);  // window_input
    pub fn user_close(&mut self, key: WindowKey) -> bool;       // CloseRequested path; true if closed
    pub fn focus(&mut self, key: WindowKey);                    // Focused(false) to the old, Focused(true) to the new
    pub fn window(&self, key: WindowKey) -> Option<&FakeWindow>;
    pub fn keys(&self) -> Vec<WindowKey>;
    pub fn exited(&self) -> bool;
    pub fn app(&mut self) -> &mut A;
}
```

The same `run_commands` logic is shared: a private `trait Backend { fn open(..) -> Result<WindowHost, String>; fn apply(..);
fn destroy(..); }` with `WinitBackend<'a>(&'a ActiveEventLoop, ...)` and `FakeBackend`, and one generic
`fn run_commands<B: Backend>(app, backend, tree, cmds)`, so the fake tests the real command path, not a copy.

## 8. The app (`crates/iron_app`)

### 8.1 Per-window state (`crates/iron_app/src/windows.rs`)

```rust
pub struct AppWindow {
    pub key: WindowKey,
    pub host: WindowHost,
    pub surface: Option<WindowSurface>,       // Os windows
    pub offscreen: Option<OffscreenTarget>,   // Virtual windows
    pub tree: UiTree,
    pub interaction: Interaction,
    pub sheets: HashMap<u32, u32>,            // app sheet key -> tree id (as panels)
    pub pointer_px: Option<(f32, f32)>,
    pub buttons_down: u32,
    pub cursor: &'static str,
    pub physical: (u32, u32),
    pub scale: f64,
    pub drawn: bool,
}
pub struct AppWindows { pub map: BTreeMap<WindowKey, AppWindow>, focused: BTreeSet<WindowKey> }
```

The main window is **not** an `AppWindow`: `self.ui`, `self.interaction`, `Gfx.surface` stay as they are, so single-window
games run the same code. `Surface` gains `Window(u32)` for the routing helpers (`handle_on`, `update_cursor`, `update_ime`).

### 8.2 Flow per frame

1. Scripts run (`tick`, UI events). `window::take_requests()` -> `WindowCommand`s (`requests_to_commands`, pure, tested),
   queued for `take_window_commands`; `Title(MAIN, t)` calls `gfx.window.set_title(t)` directly.
2. The platform opens windows: `window_opened` makes the `AppWindow` (tree from `new_panel_tree` with the screen's fonts and
   stylesheets; surface from `Gpu::create_window_surface`, or an `OffscreenTarget` for `Virtual`) and reports
   `window::opened(w, Opened { .. })`.
3. Mounts: QuickJS commits with target `WINDOW_TARGET | handle` go to that window's tree (a commit before the window opened is
   kept in a pending list, as panels do with commits before their component starts); compiled `take_window_mounts()` render
   with a per-window `iron_ui_native::Runtime` (`native_render.rs` keeps `runtimes` for panels; windows get a second map),
   flushed every frame.
4. `frame()`: after the main window presents, each open `AppWindow` with a visible, non-zero size draws its tree:
   `tree.set_size(physical / scale)`, `load_images`, `tree.draw_list()`, `renderer.render(gpu, target, CLEAR, cmds)` into its
   surface (`present_with`, vsync always off) or offscreen target. A window's first draw sets `drawn`, and the platform shows it.

### 8.3 Input routing

| Event from | Goes to |
|---|---|
| Main window | As today (screen, world panels, game input) |
| Secondary window: pointer, wheel | That window's `Interaction` (its own hover, active, capture: `buttons_down` per window), dispatched through `__iron_ui_event` with the window's target (QuickJS) or the window's native runtime |
| Secondary window: keys, IME | That window's `Interaction` (keyboard focus is per OS window, so the OS already sent them to the right one); then **also** to game `input` unless typing in a text field (decision U2) |
| Modifiers | Every tree of that window, plus the main window's when the main window is focused |
| Gamepads, raw mouse motion | Global as today (raw motion only while the main window is focused) |
| File drops | Main window only (secondary windows log and ignore them for now) |
| Focus | `focused` set per window; `events_done()` sets `Lifecycle::set_focused(!focused.is_empty())` once per batch, so moving focus from the main window to a tool window (Focused(false) then Focused(true)) does not pause and resume the game for an instant. `window::focus_changed` runs per window |
| Resized / Moved / ScaleChanged | `WindowSurface::resize`, `window::resized/moved/scale_changed` (UI pixels: physical / scale) |
| CloseRequested | `close_requested` -> `window::close_requested(w)` inside the script scope; main window: if allowed, the game ends as today |

`Window.current` is set with `window::enter_current(w)` around every dispatch of a window's events and callbacks.
`Cursor.lockState` and custom cursors stay the main window's; CSS `cursor` of a secondary window's UI sets that window's cursor.

### 8.4 Closing

- User close: `close_requested` (listeners may cancel). Then the platform's `Close` path: children first; for each
  `window_closed(k)`: drop the native runtime (its effects' cleanups run), drop the tree, the surface, the `Arc<Window>`,
  then `window::closed(w)` (onClosed). Order matters: a wgpu surface must be dropped before its HWND is destroyed.
- `win.close()`: the same without `onCloseRequested`.
- The main window: cancelable `onCloseRequested`; otherwise the game ends (children are not closed one by one; `App::drop`
  drops everything before the event loop ends).

### 8.5 Screenshot runs

`--screenshot out.png` keeps writing the main window to `out.png` and also writes every open secondary window, drawn offscreen at
its size and the main window's scale, to `out-<slug>.png`: `slug(title)` lowercases, turns every run of characters other than
`a-z0-9` into `-`, trims `-`; an empty slug is `window<handle>`; a slug already used in this run gets `-<handle>`. With
`--frames N` the files are written on the same frame. No OS window and no swapchain is made for them.

### 8.6 The main window's title

`set_main(manifest.title, ..)` at startup; the window opens with it (unchanged code path). `Window.main.title = t` calls
`set_title`. A hot reload keeps the current title (the new code may set it again). `Application.productName` stays the
manifest title: it is the product, the window title is the game's to change.

## 9. Hot reload

- **Before** the old code goes (`Native::reload`, QuickJS `reload_scripts`): `window::reload_begin()` drops every window
  listener (they hold the old code's closures), drops the window UI roots (the new code mounts again), and returns the open
  windows that have **no key**: the host closes them (children first, no `onCloseRequested`; `onClosed` is not run either,
  since its listeners are gone). Windows **with a key** stay open, with their position and size, marked unclaimed.
- The new code runs. `Window.create({ key })` with the key of an unclaimed window returns **that** window (its handle, its OS
  window): the new title, style and flags apply, the size and position are kept (the player may have moved it). A different
  `parent` is ignored with a warning. `createRoot(win)` mounts the new UI.
- After the first frame of the new code: `window::reload_end()` returns keyed windows nobody claimed; the host closes them.
- The main window always stays; its listeners are dropped and re-registered by the new code.
- Fast Refresh (QuickJS dev builds) keeps React roots; window roots behave like panel roots there.

`window::reset()` (tests, shutdown) closes nothing on the OS side: it is per-thread script state only. `Native::shutdown` and
the QuickJS teardown call it with the other resets.

## 10. A 3D view in a window (designed, built later: D3)

`Camera.targetWindow: Window | null` (`camera.rs`: `target_window: u32`, 0 = the main window). `render_sync` and `world_view`
group cameras by target; `WorldViews::views(scene, size, quads)` gets a `target` filter. In `frame()`, a secondary window with
cameras draws `draw_frame(gpu, renderer, world, target, views, textures, cmds)` instead of the UI-only path, so its UI blends
over its own world view. Shadows are drawn once a frame (already the case), so a second view adds a main pass, not a shadow pass.
Render textures (`Camera.targetTexture`) already work and are the fallback: a tool window can show a camera through an `<image>`
of a render texture with no new renderer code.

## 11. The editor

`docs/editor.md` lists "panels cannot float in windows of their own". With this API, undocking a dock panel is
`Window.create({ style: "tool", parent: Window.main, title, x, y, width, height, key: "dock:" + id })` plus mounting the panel's
content with `createRoot(win)`; docking back closes the window. Keys keep floating panels across reloads, and the layout saved in
PlayerPrefs (`editor.layout`) can store their rectangles. Drag-out (a tab dragged outside the window) needs the cursor position
outside the window: `cursor_in_window` exists for Windows. Not in this plan.

## 12. Out of scope

- Fullscreen secondary windows, per-window `Screen`/display modes, multiple monitors' selection (`docs/platform.md` limit).
- Transparent and click-through windows (D4), modal dialogs that disable the owner (D2), embedded `child` windows (D1),
  `Camera.targetWindow` (D3).
- macOS `NSPanel` utility windows, X11 `WM_TRANSIENT_FOR`, Wayland parents (winit 0.30 has none).
- File drops onto secondary windows, drag and drop between windows, per-window UI scale (`setUiScale` is the main window's).
- Windows on mobile (iOS / Android): `Window.create` returns a window that never opens; `open_failed` closes it with a
  warning.
- A redraw-on-change mode for idle tool windows (they draw every main frame; with vsync off that is as often as the main window).

## 13. Risks

| Risk | Mitigation |
|---|---|
| GPU instability with more swapchains | Every automated check is fake-backend or screenshot (virtual windows, no swapchain). Secondary windows present with vsync off and are skipped when hidden or minimised. The plan never opens an interactive window; the manual check in the window is left to the user (decision U6). |
| winit rewrites `GWL_EXSTYLE` | The subclass keeps `WS_EX_TOOLWINDOW` (2.1). |
| Surface used after its HWND died (owned windows die with their owner) | `close_order` drops descendants' surfaces first; `Destroyed` of an unknown close runs the same path. |
| Pausing flicker when focus moves between the game's windows | Focus is aggregated in `events_done()` (8.3). |
| Vsync with N FIFO swapchains divides the frame rate | Secondary windows never use vsync (2.2). |
| `focused` as a static and an instance member | `#[js_name]`, checked by `M9-TS-2802`, fallback `isFocused` (5). |
| QuickJS options objects | New `#[options]` macro attribute with its own tests (plan step 3). |
| Large `iron_app` changes | Main-window code paths are untouched; new code lives in `windows.rs`; `PlatformApp` methods have defaults. |

## 14. Decisions for the user

- **U1** `isChild` means "has a parent" (tool windows are children); `isEmbedded` means `WS_CHILD`. (Alternative: `isChild` only
  for embedded windows.)
- **U2** Keys typed into a tool window also reach game `input` (except into text fields). (Alternative: only the main window's.)
- **U3** The game counts as focused while any of its windows is (no pause when a tool window has focus).
- **U4** Hot reload closes keyless windows and keeps keyed ones (section 9).
- **U5** Secondary windows always present with vsync off.
- **U6** Screenshot runs make no OS windows for secondary windows. Optional (step D5): keep the main window hidden in screenshot
  runs too (the runner would call `frame()` from `about_to_wait`), which removes the visible window from every automated check.
- **D1** Build the `child` (embedded) style on Windows and X11, or drop it from the API.
- **D2** Modal dialogs (`modal: true`: disable the owner while open; Windows only through winit).
- **D3** `Camera.targetWindow`.
- **D4** Transparent windows.
- QuickJS has `Window.main()` as a function where compiled code has `Window.main`; acceptable until QuickJS gets static
  properties on handle classes?
