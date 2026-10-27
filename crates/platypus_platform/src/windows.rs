// SPDX-License-Identifier: Apache-2.0

//! Window types for games with more than one window: what a window asks for ([`WindowSpec`]), what that
//! means on this OS ([`plan`], decided without a display so it can be tested), where a new window goes
//! ([`place`]) and who owns whom ([`WindowTree`]). Design: `docs/research/platypus-windows.md` section 7.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;

use winit::dpi::{LogicalPosition, LogicalSize};
use winit::event_loop::ActiveEventLoop;
use winit::window::{Icon, Window, WindowAttributes, WindowButtons, WindowId, WindowLevel};

use crate::fake::VirtualBackend;
use crate::input::InputEvent;
use crate::platform::{PlatformApp, set_black_background};

/// A window's id in the platform; `MAIN` is the window that owns the process.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WindowKey(pub u32);

impl WindowKey {
    pub const MAIN: WindowKey = WindowKey(1);
}

/// How a window looks and behaves (design section 4).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum WindowKind {
    #[default]
    Normal,
    Tool,
    Dialog,
    Popup,
    Borderless,
    Child,
}

impl WindowKind {
    pub fn parse(s: &str) -> Option<WindowKind> {
        Some(match s {
            "normal" => WindowKind::Normal,
            "tool" => WindowKind::Tool,
            "dialog" => WindowKind::Dialog,
            "popup" => WindowKind::Popup,
            "borderless" => WindowKind::Borderless,
            "child" => WindowKind::Child,
            _ => return None,
        })
    }

    pub fn name(self) -> &'static str {
        match self {
            WindowKind::Normal => "normal",
            WindowKind::Tool => "tool",
            WindowKind::Dialog => "dialog",
            WindowKind::Popup => "popup",
            WindowKind::Borderless => "borderless",
            WindowKind::Child => "child",
        }
    }
}

/// What a window asked for. Sizes and positions are logical (UI pixels).
#[derive(Clone, Debug, PartialEq)]
pub struct WindowSpec {
    pub title: String,
    /// Client size.
    pub size: (f64, f64),
    /// Outer top-left; `None`: placed by [`place`].
    pub position: Option<(f64, f64)>,
    pub min_size: Option<(f64, f64)>,
    pub kind: WindowKind,
    pub parent: Option<WindowKey>,
    pub resizable: bool,
    pub minimizable: bool,
    pub maximizable: bool,
    pub closable: bool,
    pub always_on_top: bool,
    pub visible: bool,
    /// Takes keyboard focus when shown.
    pub active: bool,
    /// RGBA8 pixels, width, height.
    pub icon: Option<(Vec<u8>, u32, u32)>,
}

impl Default for WindowSpec {
    fn default() -> Self {
        WindowSpec {
            title: String::new(),
            size: (400.0, 300.0),
            position: None,
            min_size: None,
            kind: WindowKind::Normal,
            parent: None,
            resizable: true,
            minimizable: true,
            maximizable: true,
            closable: true,
            always_on_top: false,
            visible: true,
            active: true,
            icon: None,
        }
    }
}

impl WindowSpec {
    pub fn main(title: &str, w: f64, h: f64) -> WindowSpec {
        WindowSpec {
            title: title.to_string(),
            size: (w, h),
            ..WindowSpec::default()
        }
    }
}

/// The operating system / window system, for the rules that differ.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Os {
    Windows,
    MacOs,
    X11,
    Wayland,
    Other,
    /// Platforms where a game has one screen: secondary windows cannot exist at all.
    Ios,
    Android,
    Web,
}

impl Os {
    /// Whether secondary windows can exist (false on iOS, Android and the web).
    pub fn supports_windows(self) -> bool {
        !matches!(self, Os::Ios | Os::Android | Os::Web)
    }

    /// The lowercase platform name used in error texts.
    pub fn name(self) -> &'static str {
        match self {
            Os::Windows => "windows",
            Os::MacOs => "macos",
            Os::X11 => "x11",
            Os::Wayland => "wayland",
            Os::Other => "other",
            Os::Ios => "ios",
            Os::Android => "android",
            Os::Web => "web",
        }
    }

    pub fn current() -> Os {
        if cfg!(target_os = "ios") {
            Os::Ios
        } else if cfg!(target_os = "android") {
            Os::Android
        } else if cfg!(target_arch = "wasm32") {
            Os::Web
        } else if cfg!(windows) {
            Os::Windows
        } else if cfg!(target_os = "macos") {
            Os::MacOs
        } else if cfg!(target_os = "linux") {
            // winit picks Wayland when WAYLAND_DISPLAY is set unless told to use X11.
            let wayland = std::env::var_os("WAYLAND_DISPLAY").is_some_and(|v| !v.is_empty());
            let forced_x11 = std::env::var("WINIT_UNIX_BACKEND").is_ok_and(|v| v == "x11");
            if wayland && !forced_x11 {
                Os::Wayland
            } else {
                Os::X11
            }
        } else {
            Os::Other
        }
    }
}

/// What a spec means on one OS.
#[derive(Clone, Debug, PartialEq)]
pub struct NativePlan {
    /// After fallbacks (`Child` becomes `Tool` on macOS, Wayland and others).
    pub kind: WindowKind,
    pub decorations: bool,
    pub skip_taskbar: bool,
    /// `WS_EX_TOOLWINDOW` (the small caption, not in Alt-Tab).
    pub tool_caption: bool,
    /// Windows owner / macOS child window.
    pub owner: Option<WindowKey>,
    /// `WS_CHILD` / X11 reparent.
    pub embed_in: Option<WindowKey>,
    pub level_on_top: bool,
    pub active: bool,
    /// Close, minimise, maximise.
    pub buttons: (bool, bool, bool),
    pub resizable: bool,
    /// "utility", "dialog" or "popupmenu".
    pub x11_type: Option<&'static str>,
    pub warning: Option<String>,
    /// Set where secondary windows cannot exist at all (iOS, Android, web): the window must not be
    /// opened; the text is for `window_open_failed` and the log.
    pub error: Option<String>,
}

/// The start of every platform-restriction message.
pub const RESTRICTED: &str = "Cannot open Window due to platform restrictions";

/// The error for platforms with no secondary windows.
pub fn unsupported_error(os: Os) -> String {
    format!("{RESTRICTED} (platform: {})", os.name())
}

fn os_name(os: Os) -> &'static str {
    match os {
        Os::Windows => "Windows",
        Os::MacOs => "macOS",
        Os::X11 => "X11",
        Os::Wayland => "Wayland",
        Os::Other => "this platform",
        Os::Ios => "iOS",
        Os::Android => "Android",
        Os::Web => "the web",
    }
}

/// What `spec` means on `os` (design section 4), decided without a display.
pub fn plan(spec: &WindowSpec, os: Os) -> NativePlan {
    let mut kind = spec.kind;
    let mut warning = None;
    let error = (!os.supports_windows()).then(|| unsupported_error(os));
    // Owned windows are plain top-level windows on Wayland (winit 0.30 has no parents there).
    if os == Os::Wayland
        && matches!(
            kind,
            WindowKind::Tool | WindowKind::Dialog | WindowKind::Popup
        )
    {
        warning = Some(format!(
            "{RESTRICTED}: {} windows are not supported on Wayland",
            kind.name()
        ));
    }
    if kind == WindowKind::Child && !matches!(os, Os::Windows | Os::X11) {
        kind = WindowKind::Tool;
        warning = Some(format!(
            "style \"child\" is not available on {}; using \"tool\"",
            os_name(os)
        ));
    }
    let win = os == Os::Windows;
    let owner = spec.parent.unwrap_or(WindowKey::MAIN);
    let mut p = NativePlan {
        kind,
        decorations: true,
        skip_taskbar: false,
        tool_caption: false,
        owner: None,
        embed_in: None,
        level_on_top: spec.always_on_top,
        active: spec.active,
        buttons: (spec.closable, false, false),
        resizable: spec.resizable,
        x11_type: None,
        warning,
        error,
    };
    match kind {
        WindowKind::Normal => p.buttons = (spec.closable, spec.minimizable, spec.maximizable),
        WindowKind::Borderless => {
            p.decorations = false;
            p.buttons = (spec.closable, spec.minimizable, spec.maximizable);
        }
        WindowKind::Tool => {
            p.owner = Some(owner);
            p.tool_caption = win;
            p.skip_taskbar = win;
            p.x11_type = Some("utility");
        }
        WindowKind::Dialog => {
            p.owner = Some(owner);
            p.skip_taskbar = win;
            p.x11_type = Some("dialog");
        }
        WindowKind::Popup => {
            p.decorations = false;
            p.owner = Some(owner);
            p.tool_caption = win;
            p.skip_taskbar = win;
            // A popup never takes focus when shown (`focus()` still works).
            p.active = false;
            p.x11_type = Some("popupmenu");
        }
        WindowKind::Child => p.embed_in = Some(owner),
    }
    p
}

/// Where a window with no position goes: centred on the parent, plus 24 px per axis for the
/// n-th open window of that parent (wrapping after 8), so new windows do not stack exactly.
pub fn place(
    parent_pos: (f64, f64),
    parent_size: (f64, f64),
    size: (f64, f64),
    n: u32,
) -> (f64, f64) {
    let off = 24.0 * f64::from(n % 8);
    (
        parent_pos.0 + (parent_size.0 - size.0) / 2.0 + off,
        parent_pos.1 + (parent_size.1 - size.1) / 2.0 + off,
    )
}

/// Parent links of open windows: close order and family questions.
#[derive(Default, Debug, Clone)]
pub struct WindowTree {
    parent: BTreeMap<WindowKey, Option<WindowKey>>,
    /// Creation order, for `children`.
    order: Vec<WindowKey>,
}

impl WindowTree {
    /// A parent that is the key itself or one of its descendants (a cycle) is dropped.
    pub fn insert(&mut self, key: WindowKey, parent: Option<WindowKey>) {
        let parent = parent.filter(|p| *p != key && !self.is_descendant(*p, key));
        if self.parent.insert(key, parent).is_none() {
            self.order.push(key);
        }
    }

    /// Is `key` open (inserted and not removed)?
    pub fn contains(&self, key: WindowKey) -> bool {
        self.parent.contains_key(&key)
    }

    pub fn remove(&mut self, key: WindowKey) {
        self.parent.remove(&key);
        self.order.retain(|k| *k != key);
    }

    /// `key` and everything under it, deepest first (the order surfaces must be dropped).
    pub fn close_order(&self, key: WindowKey) -> Vec<WindowKey> {
        let mut out = Vec::new();
        self.collect(key, &mut out);
        out.push(key);
        out
    }

    fn collect(&self, key: WindowKey, out: &mut Vec<WindowKey>) {
        for c in self.children(key) {
            self.collect(c, out);
            out.push(c);
        }
    }

    /// Children in creation order.
    pub fn children(&self, key: WindowKey) -> Vec<WindowKey> {
        self.order
            .iter()
            .copied()
            .filter(|k| self.parent.get(k) == Some(&Some(key)))
            .collect()
    }

    pub fn parent(&self, key: WindowKey) -> Option<WindowKey> {
        self.parent.get(&key).copied().flatten()
    }

    /// The top of `key`'s family (itself when it has no parent).
    pub fn root(&self, key: WindowKey) -> WindowKey {
        let mut k = key;
        while let Some(p) = self.parent(k) {
            k = p;
        }
        k
    }

    /// Is `a` below `b`?
    fn is_descendant(&self, a: WindowKey, b: WindowKey) -> bool {
        let mut k = a;
        while let Some(p) = self.parent(k) {
            if p == b {
                return true;
            }
            k = p;
        }
        false
    }
}

/// What the platform is asked to do to windows, in order.
#[derive(Clone, Debug, PartialEq)]
pub enum WindowCommand {
    Open(WindowKey, WindowSpec),
    SetTitle(WindowKey, String),
    SetSize(WindowKey, f64, f64),
    SetPosition(WindowKey, f64, f64),
    Center(WindowKey),
    SetKind(WindowKey, WindowKind),
    /// The flag fields of the spec are read.
    SetFlags(WindowKey, WindowSpec),
    SetMinimized(WindowKey, bool),
    SetMaximized(WindowKey, bool),
    Focus(WindowKey),
    Close(WindowKey),
}

/// The window behind a key: an OS window, or an offscreen one (screenshot runs, the fake platform).
#[derive(Clone)]
pub enum WindowHost {
    Os(Arc<winit::window::Window>),
    Virtual { width: u32, height: u32, scale: f64 },
}

// ---------------------------------------------------------------------------------------------
// Running window commands: one path for the winit runner and the fake platform
// ---------------------------------------------------------------------------------------------

/// What `run_commands` needs from a window system: the winit runner's [`RunnerBackend`] and the
/// fake platform's `VirtualBackend` implement it, so tests exercise the real command path.
pub(crate) trait Backend {
    /// Makes a hidden window. The owner / embedding parent (if the plan has one) is already open.
    fn open(
        &mut self,
        key: WindowKey,
        spec: &WindowSpec,
        plan: &NativePlan,
    ) -> Result<WindowHost, String>;
    /// Any command but `Open` and `Close`.
    fn apply(&mut self, key: WindowKey, cmd: &WindowCommand);
    /// Drops the window (the app has already dropped its side).
    fn destroy(&mut self, key: WindowKey);
    /// Makes a hidden window visible (its first frame is drawn).
    fn show(&mut self, key: WindowKey);
    /// Events the backend itself produced (a virtual window answering a resize): sent to
    /// `window_input` by `run_commands`.
    fn take_events(&mut self) -> Vec<(WindowKey, InputEvent)> {
        Vec::new()
    }
}

/// Rounds of "commands, then the events they caused" `run_commands` goes through at most.
const COMMAND_ROUNDS: usize = 8;

/// Drains `app.take_window_commands()`: `Open` plans and makes a window, `Close` closes it and
/// its children (deepest first), everything else goes to the backend. Events the backend
/// produced go to `window_input`, and the commands those cause run too (a few rounds at most).
pub(crate) fn run_commands<A: PlatformApp, B: Backend>(
    app: &mut A,
    backend: &mut B,
    tree: &mut WindowTree,
    os: Os,
) {
    for _ in 0..COMMAND_ROUNDS {
        let commands = app.take_window_commands();
        let had_commands = !commands.is_empty();
        for cmd in commands {
            match cmd {
                WindowCommand::Open(key, spec) => open_window(app, backend, tree, os, key, &spec),
                WindowCommand::Close(key) => close_windows(app, backend, tree, key),
                cmd => {
                    if let Some(key) = command_key(&cmd)
                        && tree.contains(key)
                    {
                        backend.apply(key, &cmd);
                    }
                }
            }
        }
        let events = backend.take_events();
        let had_events = !events.is_empty();
        for (key, event) in events {
            if tree.contains(key) {
                app.window_input(key, event);
            }
        }
        if !had_commands && !had_events {
            break;
        }
    }
}

fn command_key(cmd: &WindowCommand) -> Option<WindowKey> {
    Some(match cmd {
        WindowCommand::Open(k, _) | WindowCommand::Close(k) => return Some(*k),
        WindowCommand::SetTitle(k, _)
        | WindowCommand::SetSize(k, _, _)
        | WindowCommand::SetPosition(k, _, _)
        | WindowCommand::Center(k)
        | WindowCommand::SetKind(k, _)
        | WindowCommand::SetFlags(k, _)
        | WindowCommand::SetMinimized(k, _)
        | WindowCommand::SetMaximized(k, _)
        | WindowCommand::Focus(k) => *k,
    })
}

fn open_window<A: PlatformApp, B: Backend>(
    app: &mut A,
    backend: &mut B,
    tree: &mut WindowTree,
    os: Os,
    key: WindowKey,
    spec: &WindowSpec,
) {
    if tree.contains(key) {
        app.window_open_failed(key, format!("window {} is already open", key.0));
        return;
    }
    let mut plan = plan(spec, os);
    if let Some(e) = plan.error.take() {
        // The app logs the error (`Window::open_failed`), once.
        tracing::debug!("window {}: {e}", key.0);
        app.window_open_failed(key, e);
        return;
    }
    if let Some(w) = &plan.warning {
        tracing::warn!("window {}: {w}", key.0);
    }
    // An owner that is gone (closed in the same round) cannot own anything.
    for slot in [&mut plan.owner, &mut plan.embed_in] {
        if slot.is_some_and(|k| !tree.contains(k)) {
            tracing::warn!(
                "window {}: its parent is not open; opening it unowned",
                key.0
            );
            *slot = None;
        }
    }
    match backend.open(key, spec, &plan) {
        Ok(host) => {
            let parent = spec
                .parent
                .filter(|p| tree.contains(*p))
                .or(plan.owner)
                .or(plan.embed_in);
            tree.insert(key, parent);
            app.window_opened(key, host);
        }
        Err(e) => app.window_open_failed(key, e),
    }
}

/// Closes `key` and everything under it, deepest first: the app drops its surface and clones of
/// the OS window (`window_closed`), then the backend drops the window. The main window never
/// closes this way (`Application.quit` / `request_close` end the run), and a key that is not
/// open does nothing, so a `Close` after the parent took it along is harmless.
pub(crate) fn close_windows<A: PlatformApp, B: Backend>(
    app: &mut A,
    backend: &mut B,
    tree: &mut WindowTree,
    key: WindowKey,
) {
    if key == WindowKey::MAIN || !tree.contains(key) {
        return;
    }
    for k in tree.close_order(key) {
        app.window_closed(k);
        backend.destroy(k);
        tree.remove(k);
    }
}

/// What came of a close request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CloseOutcome {
    /// The app said no.
    Kept,
    /// A secondary window (and its children) closed.
    Closed,
    /// The main window's close was allowed: its children are closed, the run ends.
    Exit,
}

/// The user asked to close `key` (title bar button, Alt+F4): ask the app, then close.
pub(crate) fn request_close<A: PlatformApp, B: Backend>(
    app: &mut A,
    backend: &mut B,
    tree: &mut WindowTree,
    key: WindowKey,
) -> CloseOutcome {
    if !tree.contains(key) {
        return CloseOutcome::Kept;
    }
    if key == WindowKey::MAIN {
        // Apps written for one window learn of the close through `input`.
        app.window_input(key, InputEvent::CloseRequested);
    }
    if !app.close_requested(key) {
        return CloseOutcome::Kept;
    }
    if key == WindowKey::MAIN {
        for child in tree.children(WindowKey::MAIN) {
            close_windows(app, backend, tree, child);
        }
        CloseOutcome::Exit
    } else {
        close_windows(app, backend, tree, key);
        CloseOutcome::Closed
    }
}

/// Shows the windows whose first frame the app says is drawn.
pub(crate) fn show_windows<A: PlatformApp, B: Backend>(app: &mut A, backend: &mut B) {
    for key in app.take_windows_to_show() {
        backend.show(key);
    }
}

// ---------------------------------------------------------------------------------------------
// winit
// ---------------------------------------------------------------------------------------------

/// The OS windows the runner made, by key.
#[derive(Default)]
pub(crate) struct WinitWindows {
    pub windows: BTreeMap<WindowKey, Arc<Window>>,
    ids: HashMap<WindowId, WindowKey>,
    specs: BTreeMap<WindowKey, WindowSpec>,
    plans: BTreeMap<WindowKey, NativePlan>,
    /// Windows that were made visible once (`visible = true` may show them again).
    shown: BTreeSet<WindowKey>,
}

impl WinitWindows {
    pub fn register(&mut self, key: WindowKey, window: Arc<Window>) {
        self.ids.insert(window.id(), key);
        self.windows.insert(key, window);
    }

    pub fn key_of(&self, id: WindowId) -> Option<WindowKey> {
        self.ids.get(&id).copied()
    }

    fn remove(&mut self, key: WindowKey) {
        if let Some(w) = self.windows.remove(&key) {
            self.ids.remove(&w.id());
        }
        self.specs.remove(&key);
        self.plans.remove(&key);
        self.shown.remove(&key);
    }
}

/// The runner's backend for one call: the winit event loop (windows can only be made inside its
/// callbacks), the OS windows, and the offscreen ones of a screenshot run.
pub(crate) struct RunnerBackend<'a> {
    pub event_loop: &'a ActiveEventLoop,
    pub native: &'a mut WinitWindows,
    pub virtual_backend: &'a mut VirtualBackend,
    /// Screenshot runs: secondary windows are offscreen records, not OS windows.
    pub virtual_windows: bool,
}

impl Backend for RunnerBackend<'_> {
    fn open(
        &mut self,
        key: WindowKey,
        spec: &WindowSpec,
        plan: &NativePlan,
    ) -> Result<WindowHost, String> {
        if self.virtual_windows {
            return self.virtual_backend.open(key, spec, plan);
        }
        let native = &mut *self.native;
        let get = |k: Option<WindowKey>| k.and_then(|k| native.windows.get(&k).cloned());
        let owner = get(plan.owner);
        let embed = get(plan.embed_in);
        let attrs = attributes(spec, plan, owner.as_deref(), embed.as_deref());
        let window = self
            .event_loop
            .create_window(attrs)
            .map_err(|e| e.to_string())?;
        let window = Arc::new(window);
        set_black_background(&window);
        if plan.tool_caption {
            win32::set_tool_window(&window, true);
        }
        native.register(key, window.clone());
        native.specs.insert(key, spec.clone());
        native.plans.insert(key, plan.clone());
        Ok(WindowHost::Os(window))
    }

    fn apply(&mut self, key: WindowKey, cmd: &WindowCommand) {
        if self.virtual_backend.has(key) {
            self.virtual_backend.apply(key, cmd);
            return;
        }
        let native = &mut *self.native;
        let Some(window) = native.windows.get(&key).cloned() else {
            return;
        };
        match cmd {
            WindowCommand::SetTitle(_, t) => {
                window.set_title(t);
                if let Some(s) = native.specs.get_mut(&key) {
                    s.title.clone_from(t);
                }
            }
            WindowCommand::SetSize(_, w, h) => {
                // The OS may answer with another size; `Resized` tells the app.
                let _ = window.request_inner_size(LogicalSize::new(*w, *h));
                if let Some(s) = native.specs.get_mut(&key) {
                    s.size = (*w, *h);
                }
            }
            WindowCommand::SetPosition(_, x, y) => {
                window.set_outer_position(LogicalPosition::new(*x, *y));
                if let Some(s) = native.specs.get_mut(&key) {
                    s.position = Some((*x, *y));
                }
            }
            WindowCommand::Center(_) => center(native, key, &window),
            WindowCommand::SetKind(_, kind) => set_kind(native, key, &window, *kind),
            WindowCommand::SetFlags(_, flags) => set_flags(native, key, &window, flags),
            WindowCommand::SetMinimized(_, m) => window.set_minimized(*m),
            WindowCommand::SetMaximized(_, m) => window.set_maximized(*m),
            WindowCommand::Focus(_) => window.focus_window(),
            WindowCommand::Open(..) | WindowCommand::Close(_) => {}
        }
    }

    fn destroy(&mut self, key: WindowKey) {
        if self.virtual_backend.has(key) {
            self.virtual_backend.destroy(key);
        } else {
            // The last `Arc`: the OS window is destroyed here (the app dropped its clones in
            // `window_closed`).
            self.native.remove(key);
        }
    }

    fn show(&mut self, key: WindowKey) {
        if self.virtual_backend.has(key) {
            self.virtual_backend.show(key);
            return;
        }
        let native = &mut *self.native;
        let visible = native.specs.get(&key).is_none_or(|s| s.visible);
        if let Some(w) = native.windows.get(&key)
            && visible
        {
            w.set_visible(true);
            native.shown.insert(key);
        }
    }

    fn take_events(&mut self) -> Vec<(WindowKey, InputEvent)> {
        self.virtual_backend.take_events()
    }
}

/// Centres `window` on its parent (else the main window), as `place` would.
fn center(native: &WinitWindows, key: WindowKey, window: &Window) {
    let parent = native
        .specs
        .get(&key)
        .and_then(|s| s.parent)
        .and_then(|k| native.windows.get(&k))
        .or_else(|| native.windows.get(&WindowKey::MAIN));
    let Some(parent) = parent.filter(|p| p.id() != window.id()) else {
        return;
    };
    let Ok(pos) = parent.outer_position() else {
        return;
    };
    let scale = parent.scale_factor();
    let pos = pos.to_logical::<f64>(scale);
    let size = parent.outer_size().to_logical::<f64>(scale);
    let me = window.outer_size().to_logical::<f64>(window.scale_factor());
    let (x, y) = place(
        (pos.x, pos.y),
        (size.width, size.height),
        (me.width, me.height),
        0,
    );
    window.set_outer_position(LogicalPosition::new(x, y));
}

fn set_kind(native: &mut WinitWindows, key: WindowKey, window: &Window, kind: WindowKind) {
    let (Some(spec), Some(old)) = (native.specs.get(&key), native.plans.get(&key)) else {
        return;
    };
    let mut spec = spec.clone();
    spec.kind = kind;
    let new = plan(&spec, Os::current());
    // An OS window's owner is fixed when it is made.
    if new.owner != old.owner || new.embed_in != old.embed_in {
        tracing::warn!(
            "window {}: style \"{}\" needs another owner than the window has; the style was not changed",
            key.0,
            kind.name()
        );
        return;
    }
    apply_plan(window, old, &new);
    native.specs.insert(key, spec);
    native.plans.insert(key, new);
}

fn set_flags(native: &mut WinitWindows, key: WindowKey, window: &Window, flags: &WindowSpec) {
    let mut spec = native
        .specs
        .get(&key)
        .cloned()
        .unwrap_or_else(|| flags.clone());
    spec.resizable = flags.resizable;
    spec.minimizable = flags.minimizable;
    spec.maximizable = flags.maximizable;
    spec.closable = flags.closable;
    spec.always_on_top = flags.always_on_top;
    spec.visible = flags.visible;
    let new = plan(&spec, Os::current());
    match native.plans.get(&key) {
        Some(old) => apply_plan(window, old, &new),
        None => {
            // The main window: its plan was never recorded.
            window.set_resizable(new.resizable);
            window.set_enabled_buttons(buttons(&new));
            window.set_window_level(level(&new));
        }
    }
    if !spec.visible {
        window.set_visible(false);
    } else if native.shown.contains(&key) {
        window.set_visible(true);
    }
    native.specs.insert(key, spec);
    native.plans.insert(key, new);
}

/// Applies what differs between two plans of one window.
fn apply_plan(window: &Window, old: &NativePlan, new: &NativePlan) {
    if old.decorations != new.decorations {
        window.set_decorations(new.decorations);
    }
    if old.resizable != new.resizable {
        window.set_resizable(new.resizable);
    }
    if old.buttons != new.buttons {
        window.set_enabled_buttons(buttons(new));
    }
    if old.level_on_top != new.level_on_top {
        window.set_window_level(level(new));
    }
    #[cfg(windows)]
    {
        use winit::platform::windows::WindowExtWindows;
        if old.skip_taskbar != new.skip_taskbar {
            window.set_skip_taskbar(new.skip_taskbar);
        }
    }
    if old.tool_caption != new.tool_caption {
        win32::set_tool_window(window, new.tool_caption);
    }
}

fn buttons(plan: &NativePlan) -> WindowButtons {
    let mut b = WindowButtons::empty();
    b.set(WindowButtons::CLOSE, plan.buttons.0);
    b.set(WindowButtons::MINIMIZE, plan.buttons.1);
    b.set(WindowButtons::MAXIMIZE, plan.buttons.2);
    b
}

fn level(plan: &NativePlan) -> WindowLevel {
    if plan.level_on_top {
        WindowLevel::AlwaysOnTop
    } else {
        WindowLevel::Normal
    }
}

/// The OS handle of a window as a number and the window system's name: HWND ("win32"), the NSView
/// pointer ("appkit"), the X11 window id ("x11"); 0 for Wayland ("wayland") and unknown systems.
pub fn native_handle(window: &Window) -> (u64, &'static str) {
    use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
    match window.window_handle().map(|h| h.as_raw()) {
        Ok(RawWindowHandle::Win32(h)) => (h.hwnd.get() as u64, "win32"),
        Ok(RawWindowHandle::AppKit(h)) => (h.ns_view.as_ptr() as u64, "appkit"),
        Ok(RawWindowHandle::Xlib(h)) => (u64::from(h.window), "x11"),
        Ok(RawWindowHandle::Xcb(h)) => (u64::from(h.window.get()), "x11"),
        Ok(RawWindowHandle::Wayland(_)) => (0, "wayland"),
        _ => (0, ""),
    }
}

/// The winit attributes for a new window (design section 7.3). It starts hidden; the app shows
/// it once its first frame is drawn. `owner` / `embed` are the OS windows of `plan.owner` /
/// `plan.embed_in`.
pub fn attributes(
    spec: &WindowSpec,
    plan: &NativePlan,
    owner: Option<&Window>,
    embed: Option<&Window>,
) -> WindowAttributes {
    let (w, h) = spec.size;
    let mut a = Window::default_attributes()
        .with_title(spec.title.clone())
        .with_inner_size(LogicalSize::new(w, h))
        .with_resizable(plan.resizable)
        .with_enabled_buttons(buttons(plan))
        .with_decorations(plan.decorations)
        .with_window_level(level(plan))
        .with_visible(false)
        .with_active(plan.active);
    if let Some((w, h)) = spec.min_size {
        a = a.with_min_inner_size(LogicalSize::new(w, h));
    }
    if let Some((x, y)) = spec.position {
        a = a.with_position(LogicalPosition::new(x, y));
    }
    if let Some((rgba, w, h)) = &spec.icon {
        match Icon::from_rgba(rgba.clone(), *w, *h) {
            Ok(icon) => a = a.with_window_icon(Some(icon)),
            Err(e) => tracing::warn!("window icon: {e}"),
        }
    }
    platform_attributes(a, plan, owner, embed)
}

#[cfg(any(windows, target_os = "macos", all(unix, not(target_vendor = "apple"))))]
fn raw_handle(w: &Window) -> Option<winit::raw_window_handle::RawWindowHandle> {
    use winit::raw_window_handle::HasWindowHandle;
    w.window_handle().ok().map(|h| h.as_raw())
}

/// Windows: `with_owner_window` (above the owner, hidden with it, destroyed with it, no
/// taskbar button), `with_skip_taskbar`, `WS_CHILD` for embedded children.
#[cfg(windows)]
fn platform_attributes(
    mut a: WindowAttributes,
    plan: &NativePlan,
    owner: Option<&Window>,
    embed: Option<&Window>,
) -> WindowAttributes {
    use winit::platform::windows::WindowAttributesExtWindows;
    use winit::raw_window_handle::RawWindowHandle;
    if plan.skip_taskbar {
        a = a.with_skip_taskbar(true);
    }
    if let Some(RawWindowHandle::Win32(h)) = owner.and_then(raw_handle) {
        a = a.with_owner_window(h.hwnd.get());
    }
    if let Some(h) = embed.and_then(raw_handle) {
        // SAFETY: `embed` is a live window of ours and outlives the child (children close first).
        a = unsafe { a.with_parent_window(Some(h)) };
    }
    a
}

/// macOS: a child window (`addChildWindow`, above the owner and moving with it). X11: the
/// window type, and reparenting for embedded children. Wayland has nothing for parents.
#[cfg(not(windows))]
fn platform_attributes(
    a: WindowAttributes,
    plan: &NativePlan,
    owner: Option<&Window>,
    embed: Option<&Window>,
) -> WindowAttributes {
    #[cfg(target_os = "macos")]
    let a = match owner.and_then(raw_handle) {
        // SAFETY: `owner` is a live window of ours and outlives the child (children close first).
        Some(h) => unsafe { a.with_parent_window(Some(h)) },
        None => a,
    };
    #[cfg(all(unix, not(target_vendor = "apple")))]
    let a = {
        use winit::platform::x11::{WindowAttributesExtX11, WindowType};
        let mut a = a;
        let ty = match plan.x11_type {
            Some("utility") => Some(WindowType::Utility),
            Some("dialog") => Some(WindowType::Dialog),
            Some("popupmenu") => Some(WindowType::PopupMenu),
            _ => None,
        };
        if let Some(ty) = ty {
            a = a.with_x11_window_type(vec![ty]);
        }
        if let Some(h) = embed.and_then(raw_handle) {
            // SAFETY: `embed` is a live window of ours and outlives the child.
            a = unsafe { a.with_parent_window(Some(h)) };
        }
        a
    };
    let _ = (plan, owner, embed);
    a
}

/// Raw Win32 rules winit has no option for.
mod win32 {
    use winit::window::Window;

    /// Small caption, no taskbar button, not in Alt-Tab. winit rewrites `GWL_EXSTYLE` whenever
    /// one of its flags changes, so the style is kept by a subclass that adds `WS_EX_TOOLWINDOW`
    /// (and removes `WS_EX_APPWINDOW`) in `WM_STYLECHANGING`. `on = false` undoes both.
    #[cfg(windows)]
    pub(super) fn set_tool_window(window: &Window, on: bool) {
        use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
        use windows_sys::Win32::UI::Shell::{
            DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass,
        };
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            GWL_EXSTYLE, GetWindowLongPtrW, STYLESTRUCT, SWP_FRAMECHANGED, SWP_NOACTIVATE,
            SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SetWindowLongPtrW, SetWindowPos, WM_NCDESTROY,
            WM_STYLECHANGING, WS_EX_APPWINDOW, WS_EX_TOOLWINDOW,
        };
        use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};

        const TOOL_SUBCLASS_ID: usize = 0x4952_544F; // "IRTO"

        unsafe extern "system" fn tool_proc(
            hwnd: HWND,
            msg: u32,
            wparam: WPARAM,
            lparam: LPARAM,
            id: usize,
            _data: usize,
        ) -> LRESULT {
            if msg == WM_STYLECHANGING && wparam as i32 == GWL_EXSTYLE {
                // SAFETY: for WM_STYLECHANGING `lparam` points to the window's STYLESTRUCT.
                let style = unsafe { &mut *(lparam as *mut STYLESTRUCT) };
                style.styleNew = (style.styleNew | WS_EX_TOOLWINDOW) & !WS_EX_APPWINDOW;
            } else if msg == WM_NCDESTROY {
                // SAFETY: removes this very subclass from the window being destroyed.
                unsafe { RemoveWindowSubclass(hwnd, Some(tool_proc), id) };
            }
            // SAFETY: passes the message on unchanged.
            unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) }
        }

        let Ok(handle) = window.window_handle() else {
            return;
        };
        let RawWindowHandle::Win32(h) = handle.as_raw() else {
            return;
        };
        let hwnd = h.hwnd.get() as HWND;
        // SAFETY: `hwnd` is our live window; the subclass proc and id are valid for its lifetime.
        unsafe {
            if on {
                SetWindowSubclass(hwnd, Some(tool_proc), TOOL_SUBCLASS_ID, 0);
            } else {
                RemoveWindowSubclass(hwnd, Some(tool_proc), TOOL_SUBCLASS_ID);
            }
            let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
            let ex = if on {
                (ex | WS_EX_TOOLWINDOW) & !WS_EX_APPWINDOW
            } else {
                ex & !WS_EX_TOOLWINDOW
            };
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex as isize);
            SetWindowPos(
                hwnd,
                0,
                0,
                0,
                0,
                0,
                SWP_FRAMECHANGED | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
            );
        }
    }

    #[cfg(not(windows))]
    pub(super) fn set_tool_window(_window: &Window, _on: bool) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tool() -> WindowSpec {
        WindowSpec {
            kind: WindowKind::Tool,
            ..WindowSpec::default()
        }
    }

    // M9-API-1400
    #[test]
    fn window_kinds_parse_and_name_round_trip() {
        for n in ["normal", "tool", "dialog", "popup", "borderless", "child"] {
            assert_eq!(WindowKind::parse(n).unwrap().name(), n);
        }
        assert_eq!(WindowKind::parse("Tool"), None);
        assert_eq!(WindowKind::parse(""), None);
    }

    // M9-API-1401
    #[test]
    fn tool_windows_are_owned_skip_the_taskbar_and_get_the_small_caption_on_windows() {
        let p = plan(&tool(), Os::Windows);
        assert_eq!(p.owner, Some(WindowKey::MAIN));
        assert!(p.tool_caption && p.skip_taskbar && p.decorations);
        assert_eq!(p.buttons, (true, false, false));
        assert_eq!(p.x11_type, Some("utility"));
    }

    // M9-API-1402
    #[test]
    fn tool_windows_on_macos_are_child_windows_without_taskbar_rules() {
        let p = plan(&tool(), Os::MacOs);
        assert_eq!(p.owner, Some(WindowKey::MAIN));
        assert!(!p.tool_caption && !p.skip_taskbar);
    }

    // M9-API-1403
    #[test]
    fn embedded_children_fall_back_to_tool_where_the_platform_has_none() {
        let spec = WindowSpec {
            kind: WindowKind::Child,
            parent: Some(WindowKey(5)),
            ..WindowSpec::default()
        };
        let p = plan(&spec, Os::Windows);
        assert_eq!(p.kind, WindowKind::Child);
        assert_eq!(p.embed_in, Some(WindowKey(5)));
        assert!(p.warning.is_none() && p.owner.is_none());
        for os in [Os::MacOs, Os::Wayland, Os::Other] {
            let p = plan(&spec, os);
            assert_eq!(p.kind, WindowKind::Tool);
            assert_eq!(p.embed_in, None);
            assert_eq!(p.owner, Some(WindowKey(5)));
            assert!(p.warning.unwrap().contains("\"child\""));
        }
        assert_eq!(plan(&spec, Os::X11).embed_in, Some(WindowKey(5)));
    }

    // M9-API-1404
    #[test]
    fn popups_have_no_decorations_and_do_not_take_focus() {
        let spec = WindowSpec {
            kind: WindowKind::Popup,
            active: true,
            ..WindowSpec::default()
        };
        let p = plan(&spec, Os::Windows);
        assert!(!p.decorations && !p.active && p.skip_taskbar && p.tool_caption);
        assert_eq!(p.x11_type, Some("popupmenu"));
        assert!(plan(&WindowSpec::default(), Os::Windows).active);
    }

    // M9-API-1405
    #[test]
    fn new_windows_are_centred_on_their_parent_and_cascade() {
        let at = |n| place((100.0, 50.0), (800.0, 600.0), (200.0, 100.0), n);
        assert_eq!(at(0), (400.0, 300.0));
        assert_eq!(at(1), (424.0, 324.0));
        assert_eq!(at(8), (400.0, 300.0));
    }

    // M9-API-1406
    #[test]
    fn closing_a_window_closes_its_children_deepest_first() {
        let k = WindowKey;
        let mut t = WindowTree::default();
        t.insert(k(1), None);
        t.insert(k(2), Some(k(1)));
        t.insert(k(3), Some(k(2)));
        t.insert(k(4), Some(k(1)));
        assert_eq!(t.close_order(k(2)), vec![k(3), k(2)]);
        assert_eq!(t.close_order(k(1)), vec![k(3), k(2), k(4), k(1)]);
        assert_eq!(t.children(k(1)), vec![k(2), k(4)]);
        assert_eq!(t.parent(k(3)), Some(k(2)));
        assert_eq!(t.root(k(3)), k(1));
        t.remove(k(3));
        assert_eq!(t.close_order(k(1)), vec![k(2), k(4), k(1)]);
    }

    // M9-API-1407
    #[test]
    fn a_parent_that_would_make_a_cycle_is_dropped() {
        let k = WindowKey;
        let mut t = WindowTree::default();
        t.insert(k(2), None);
        t.insert(k(3), Some(k(2)));
        t.insert(k(2), Some(k(3)));
        assert_eq!(t.parent(k(2)), None);
        t.insert(k(5), Some(k(5)));
        assert_eq!(t.parent(k(5)), None);
        assert_eq!(t.close_order(k(2)), vec![k(3), k(2)]);
    }

    // M9-API-1447
    #[test]
    fn winit_attributes_follow_the_plan() {
        let spec = WindowSpec {
            title: "Palette".into(),
            size: (220.0, 320.0),
            kind: WindowKind::Tool,
            always_on_top: true,
            ..WindowSpec::default()
        };
        let p = plan(&spec, Os::Windows);
        let a = attributes(&spec, &p, None, None);
        assert_eq!(a.title, "Palette");
        assert_eq!(a.inner_size, Some(LogicalSize::new(220.0, 320.0).into()));
        assert!(a.decorations && a.resizable && a.active);
        assert!(
            !a.visible,
            "windows start hidden and are shown after their first frame"
        );
        assert_eq!(a.window_level, WindowLevel::AlwaysOnTop);
        assert_eq!(a.enabled_buttons, WindowButtons::CLOSE);

        let popup = WindowSpec {
            kind: WindowKind::Popup,
            ..WindowSpec::default()
        };
        let a = attributes(&popup, &plan(&popup, Os::Windows), None, None);
        assert!(!a.decorations && !a.active);
        assert_eq!(a.window_level, WindowLevel::Normal);
    }
}
