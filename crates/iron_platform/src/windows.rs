//! Window types for games with more than one window: what a window asks for ([`WindowSpec`]), what that
//! means on this OS ([`plan`], decided without a display so it can be tested), where a new window goes
//! ([`place`]) and who owns whom ([`WindowTree`]). Design: `docs/research/iron-windows.md` section 7.

use std::collections::BTreeMap;
use std::sync::Arc;

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
}

impl Os {
    pub fn current() -> Os {
        if cfg!(windows) {
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
}

fn os_name(os: Os) -> &'static str {
    match os {
        Os::Windows => "Windows",
        Os::MacOs => "macOS",
        Os::X11 => "X11",
        Os::Wayland => "Wayland",
        Os::Other => "this platform",
    }
}

/// What `spec` means on `os` (design section 4), decided without a display.
pub fn plan(spec: &WindowSpec, os: Os) -> NativePlan {
    let mut kind = spec.kind;
    let mut warning = None;
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
}
