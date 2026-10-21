//! Secondary windows in the app (design: `docs/research/iron-windows.md` section 8): the script's
//! `Window` requests become platform commands, each opened window gets its own UI tree, interaction
//! state and surface, and the platform's window events are routed to them.
//!
//! The main window is not an [`AppWindow`]: the screen UI, its `Interaction` and `Gfx.surface` stay
//! as they are, so single-window games run the same code as before.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use iron_engine_api::window::{self as win, Style, WindowRef, WindowRequest, WindowState};
use iron_platform::{
    InputEvent, WheelDelta, WindowCommand, WindowHost, WindowKey, WindowKind, WindowSpec,
};
use iron_render::{OffscreenTarget, WindowSurface};
use iron_ui::{Input, Interaction, UiTree, events};

use super::*;

/// One secondary window.
#[allow(dead_code)] // `key` and `drawn` are read by the drawing code (step 8)
pub struct AppWindow {
    /// Dropped before `host`: a wgpu surface must go before the OS window it draws to.
    pub surface: Option<WindowSurface>,
    /// Virtual windows (screenshot runs) draw into this instead.
    pub offscreen: Option<OffscreenTarget>,
    pub host: WindowHost,
    pub key: WindowKey,
    pub tree: UiTree,
    pub interaction: Interaction,
    /// Stylesheets added to this tree, by the app's sheet key.
    pub sheets: HashMap<u32, u32>,
    /// The pointer in window pixels, while it is over the window.
    pub pointer_px: Option<(f32, f32)>,
    pub buttons_down: u32,
    pub ime_allowed: bool,
    pub cursor: &'static str,
    /// Client size in physical pixels, and physical pixels per UI pixel.
    pub physical: (u32, u32),
    pub scale: f64,
    /// A frame was drawn (the platform shows the window after the first one).
    pub drawn: bool,
}

/// The open secondary windows, and which of the game's windows have OS focus.
#[derive(Default)]
pub struct AppWindows {
    pub map: BTreeMap<WindowKey, AppWindow>,
    /// Windows with OS focus (the main window is in it from the start).
    pub focused: BTreeSet<WindowKey>,
    /// The position each window was opened at (the requests resolved it).
    pub placed: HashMap<WindowKey, Option<(f64, f64)>>,
    /// React commits for windows that are not open yet (QuickJS games).
    pub pending_commits: HashMap<u32, Vec<iron_core::Commit>>,
    /// Titles in use for screenshot file names.
    pub slugs: HashMap<WindowKey, String>,
}

impl AppWindows {
    pub fn new() -> Self {
        AppWindows {
            focused: BTreeSet::from([WindowKey::MAIN]),
            ..AppWindows::default()
        }
    }
}

/// UI pixels of a physical size at `scale` physical pixels per UI pixel.
pub fn to_ui(physical: (u32, u32), scale: f64) -> (f64, f64) {
    let s = if scale > 0.0 { scale } else { 1.0 };
    (f64::from(physical.0) / s, f64::from(physical.1) / s)
}

/// The game counts as focused while any of its windows is.
pub fn app_focused(focused: &BTreeSet<WindowKey>) -> bool {
    !focused.is_empty()
}

/// A file-name part for a window's screenshot: the title lowercased, runs of characters other
/// than `a-z0-9` as `-`, trimmed. Empty becomes `window<handle>`; one already in `used` gets
/// `-<handle>`.
#[allow(dead_code)] // used by the screenshot code (step 8)
pub fn slug(title: &str, handle: u32, used: &mut HashSet<String>) -> String {
    let mut out = String::new();
    for c in title.to_lowercase().chars() {
        if c.is_ascii_lowercase() || c.is_ascii_digit() {
            out.push(c);
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    let mut s = out.trim_matches('-').to_string();
    if s.is_empty() {
        s = format!("window{handle}");
    }
    if used.contains(&s) {
        s = format!("{s}-{handle}");
    }
    used.insert(s.clone());
    s
}

fn key(w: WindowRef) -> WindowKey {
    WindowKey(w.0)
}

fn kind_of(style: Style) -> WindowKind {
    WindowKind::parse(style.name()).unwrap_or_default()
}

fn spec_flags(spec: &mut WindowSpec, st: &WindowState) {
    spec.resizable = st.flags.resizable;
    spec.minimizable = st.flags.minimizable;
    spec.maximizable = st.flags.maximizable;
    spec.closable = st.flags.closable;
    spec.always_on_top = st.flags.always_on_top;
    spec.visible = st.flags.visible;
}

/// The spec a `Create` asks for. A window without a position is centred on its parent, each
/// further child 24 px lower and to the right.
fn open_spec(
    st: &WindowState,
    state: &impl Fn(WindowRef) -> Option<WindowState>,
    open: &[WindowRef],
    me: WindowRef,
) -> WindowSpec {
    let position = st.position.or_else(|| {
        let parent = st.parent?;
        let p = state(parent)?;
        let n = open
            .iter()
            .take_while(|w| **w != me)
            .filter(|w| state(**w).is_some_and(|s| s.parent == Some(parent)))
            .count() as u32;
        Some(iron_platform::place(
            p.position.unwrap_or((0.0, 0.0)),
            p.size,
            st.size,
            n,
        ))
    });
    let mut spec = WindowSpec {
        title: st.title.clone(),
        size: st.size,
        position,
        min_size: st.min_size,
        kind: kind_of(st.style),
        parent: st.parent.map(key),
        active: st.flags.focus_on_show,
        ..WindowSpec::default()
    };
    spec_flags(&mut spec, st);
    spec
}

/// What scripts asked for, as platform commands, and the main window's new title if it changed.
/// `open` is the open windows in creation order.
pub fn requests_to_commands(
    reqs: Vec<WindowRequest>,
    state: impl Fn(WindowRef) -> Option<WindowState>,
    open: &[WindowRef],
) -> (Vec<WindowCommand>, Option<String>) {
    let mut cmds = Vec::new();
    let mut main_title = None;
    for r in reqs {
        match r {
            WindowRequest::Create(w) => {
                if let Some(st) = state(w) {
                    cmds.push(WindowCommand::Open(key(w), open_spec(&st, &state, open, w)));
                }
            }
            WindowRequest::Title(w, t) => {
                if w == win::MAIN {
                    main_title = Some(t);
                } else {
                    cmds.push(WindowCommand::SetTitle(key(w), t));
                }
            }
            WindowRequest::Style(w, s) => cmds.push(WindowCommand::SetKind(key(w), kind_of(s))),
            WindowRequest::Position(w, x, y) => cmds.push(WindowCommand::SetPosition(key(w), x, y)),
            WindowRequest::Size(w, x, y) => cmds.push(WindowCommand::SetSize(key(w), x, y)),
            WindowRequest::Center(w) => cmds.push(WindowCommand::Center(key(w))),
            WindowRequest::Flags(w, f) => {
                if let Some(mut st) = state(w) {
                    st.flags = f;
                    let mut spec = WindowSpec::default();
                    spec_flags(&mut spec, &st);
                    cmds.push(WindowCommand::SetFlags(key(w), spec));
                }
            }
            WindowRequest::Minimized(w, v) => cmds.push(WindowCommand::SetMinimized(key(w), v)),
            WindowRequest::Maximized(w, v) => cmds.push(WindowCommand::SetMaximized(key(w), v)),
            WindowRequest::Focus(w) => cmds.push(WindowCommand::Focus(key(w))),
            // The main window ends with the game (`Application.quit`), not with a command.
            WindowRequest::Close(w) if w == win::MAIN => {}
            WindowRequest::Close(w) => cmds.push(WindowCommand::Close(key(w))),
        }
    }
    // The main window's other setters (size, position, flags) are the player's and the OS's.
    cmds.retain(|c| !matches!(c, WindowCommand::SetTitle(k, _) if *k == WindowKey::MAIN));
    cmds.retain(|c| match c {
        WindowCommand::SetKind(k, _)
        | WindowCommand::SetPosition(k, ..)
        | WindowCommand::SetSize(k, ..)
        | WindowCommand::Center(k)
        | WindowCommand::SetFlags(k, _)
        | WindowCommand::SetMinimized(k, _)
        | WindowCommand::SetMaximized(k, _) => *k != WindowKey::MAIN,
        _ => true,
    });
    (cmds, main_title)
}

impl App {
    /// Turns the window requests scripts made into platform commands (and sets the main title).
    /// Called after every point that runs scripts.
    pub(super) fn pump_windows(&mut self) {
        let reqs = win::take_requests();
        if reqs.is_empty() {
            return;
        }
        let open = win::open_windows();
        let (cmds, title) = requests_to_commands(reqs, win::state, &open);
        if let (Some(t), Some(gfx)) = (title, &self.gfx) {
            gfx.window.set_title(&t);
        }
        for c in &cmds {
            if let WindowCommand::Open(k, spec) = c {
                self.windows.placed.insert(*k, spec.position);
            }
        }
        self.window_commands.extend(cmds);
    }

    /// Runs `f` (a call into the game's window listeners) with the scene current, then what
    /// follows a call into scripts.
    pub(super) fn in_game<R>(&mut self, f: impl FnOnce() -> R) -> R {
        let r = {
            let _scope = iron_scene::api::enter(&mut self.scene.scene);
            f()
        };
        if let Some(n) = self.native.as_mut() {
            n.settle(&mut self.scene.scene, &mut self.ui);
            self.native_after();
        } else {
            self.apply_script_output();
        }
        r
    }

    /// The platform made the window `key`: gives it a tree and a surface (or an offscreen target
    /// for a virtual one) and tells scripts.
    pub(super) fn window_open(&mut self, key: WindowKey, host: WindowHost) {
        let w = WindowRef(key.0);
        let (physical, scale, native) = match &host {
            WindowHost::Os(window) => {
                let size = window.inner_size();
                let (handle, platform) = iron_platform::native_handle(window);
                (
                    (size.width, size.height),
                    window.scale_factor(),
                    (handle, platform),
                )
            }
            WindowHost::Virtual {
                width,
                height,
                scale,
            } => ((*width, *height), *scale, (0, "virtual")),
        };
        let Some(gfx) = &self.gfx else {
            self.in_game(|| win::open_failed(w, "the renderer is not available"));
            self.window_commands.push(WindowCommand::Close(key));
            return;
        };
        let (mut surface, mut offscreen) = (None, None);
        match &host {
            WindowHost::Os(window) => {
                match gfx
                    .gpu
                    .create_window_surface(window.clone(), physical.0, physical.1)
                {
                    Ok(s) => surface = Some(s),
                    Err(e) => {
                        let msg = format!("no surface: {e}");
                        self.in_game(|| win::open_failed(w, &msg));
                        self.window_commands.push(WindowCommand::Close(key));
                        return;
                    }
                }
            }
            WindowHost::Virtual { .. } => {
                offscreen = Some(OffscreenTarget::with_format(
                    &gfx.gpu,
                    physical.0.max(1),
                    physical.1.max(1),
                    wgpu::TextureFormat::Rgba8UnormSrgb,
                ));
            }
        }
        let (mut tree, sheets) =
            new_panel_tree(&self.asset_root, &self.font_sources, &self.sheet_sources);
        let ui = to_ui(physical, scale);
        tree.set_size(ui.0 as f32, ui.1 as f32);
        let position = self
            .windows
            .placed
            .remove(&key)
            .flatten()
            .or_else(|| win::state(w).and_then(|s| s.position))
            .unwrap_or((0.0, 0.0));
        self.windows.map.insert(
            key,
            AppWindow {
                surface,
                offscreen,
                host,
                key,
                tree,
                interaction: Interaction::new(),
                sheets,
                pointer_px: None,
                buttons_down: 0,
                ime_allowed: false,
                cursor: "default",
                physical,
                scale,
                drawn: false,
            },
        );
        // QuickJS commits that came before the window did.
        if let Some(commits) = self.windows.pending_commits.remove(&key.0)
            && let Some(window) = self.windows.map.get_mut(&key)
        {
            for commit in commits {
                for e in window.tree.apply_commit(&commit) {
                    tracing::warn!(target: "ui", "{e}");
                }
            }
        }
        let opened = win::Opened {
            position,
            size: ui,
            scale,
            native_handle: native.0,
            platform: native.1.to_string(),
        };
        self.in_game(|| win::opened(w, opened));
    }

    /// An event of a window other than the main one.
    pub(super) fn window_event(&mut self, key: WindowKey, event: InputEvent) {
        let w = WindowRef(key.0);
        let Some(scale) = self.windows.map.get(&key).map(|x| x.scale) else {
            return;
        };
        let ui = scale as f32;
        let _current = win::enter_current(w);
        let surface = Surface::Window(key.0);
        match event {
            InputEvent::Focused(f) => {
                if f {
                    self.windows.focused.insert(key);
                } else {
                    self.windows.focused.remove(&key);
                    if let Some(x) = self.windows.map.get_mut(&key) {
                        x.buttons_down = 0;
                    }
                    self.handle_on(surface, Input::WindowBlur);
                }
                self.in_game(|| win::focus_changed(w, f));
            }
            InputEvent::Resized { width, height } => {
                let Some(x) = self.windows.map.get_mut(&key) else {
                    return;
                };
                x.physical = (width, height);
                if let (Some(s), Some(gfx)) = (x.surface.as_mut(), &self.gfx) {
                    s.resize(&gfx.gpu, width, height);
                }
                if let (Some(o), Some(gfx)) = (x.offscreen.as_mut(), &self.gfx) {
                    o.resize(&gfx.gpu, width.max(1), height.max(1));
                }
                let size = to_ui((width, height), x.scale);
                x.tree.set_size(size.0 as f32, size.1 as f32);
                self.in_game(|| win::resized(w, size.0, size.1));
            }
            InputEvent::Moved { x, y } => {
                let (lx, ly) = (f64::from(x) / f64::from(ui), f64::from(y) / f64::from(ui));
                self.in_game(|| win::moved(w, lx, ly));
            }
            InputEvent::ScaleChanged(s) => {
                if let Some(x) = self.windows.map.get_mut(&key) {
                    x.scale = s;
                }
                self.in_game(|| win::scale_changed(w, s));
            }
            InputEvent::MouseMove { x, y } => {
                if let Some(p) = self.windows.map.get_mut(&key) {
                    p.pointer_px = Some((x as f32, y as f32));
                }
                self.handle_on(
                    surface,
                    Input::PointerMove {
                        x: x as f32 / ui,
                        y: y as f32 / ui,
                    },
                );
            }
            InputEvent::MouseLeave => {
                if let Some(p) = self.windows.map.get_mut(&key) {
                    p.pointer_px = None;
                }
                self.handle_on(surface, Input::PointerLeave);
            }
            InputEvent::MouseDown { button } => {
                if let Some(p) = self.windows.map.get_mut(&key) {
                    p.buttons_down += 1;
                }
                let time_ms = self.now_ms();
                self.handle_on(
                    surface,
                    Input::PointerDown {
                        button: dom_button(button),
                        time_ms,
                    },
                );
            }
            InputEvent::MouseUp { button } => {
                self.handle_on(
                    surface,
                    Input::PointerUp {
                        button: dom_button(button),
                    },
                );
                if let Some(p) = self.windows.map.get_mut(&key) {
                    p.buttons_down = p.buttons_down.saturating_sub(1);
                }
            }
            InputEvent::MouseWheel(delta) => {
                let (dx, dy) = match delta {
                    WheelDelta::Lines { x, y } => (x * WHEEL_LINE, y * WHEEL_LINE),
                    WheelDelta::Pixels { x, y } => {
                        ((x / f64::from(ui)) as f32, (y / f64::from(ui)) as f32)
                    }
                };
                self.handle_on(surface, Input::Wheel { dx, dy });
            }
            InputEvent::KeyDown {
                code,
                key: k,
                repeat,
                text,
            } => self.handle_on(
                surface,
                Input::KeyDown {
                    key: k,
                    code,
                    repeat,
                    text,
                },
            ),
            InputEvent::KeyUp { code, key: k } => {
                self.handle_on(surface, Input::KeyUp { key: k, code })
            }
            InputEvent::Modifiers(m) => self.handle_on(
                surface,
                Input::Modifiers(events::Modifiers {
                    shift: m.shift,
                    ctrl: m.ctrl,
                    alt: m.alt,
                    meta: m.meta,
                }),
            ),
            InputEvent::Ime(iron_platform::ImeEvent::Preedit(text, cursor)) => {
                self.handle_on(surface, Input::ImePreedit { text, cursor })
            }
            InputEvent::Ime(iron_platform::ImeEvent::Commit(text)) => {
                self.handle_on(surface, Input::ImeCommit(text))
            }
            InputEvent::Ime(iron_platform::ImeEvent::Disabled) => self.handle_on(
                surface,
                Input::ImePreedit {
                    text: String::new(),
                    cursor: None,
                },
            ),
            // File drops, raw motion, gamepads and the rest belong to the main window.
            _ => {}
        }
    }

    /// The user asked to close `key`: listeners may cancel.
    pub(super) fn window_close_requested(&mut self, key: WindowKey) -> bool {
        self.in_game(|| win::close_requested(WindowRef(key.0)))
    }

    /// `key` is closing: its reconciler, tree and surface go (in that order), then scripts hear
    /// `onClosed`.
    pub(super) fn window_gone(&mut self, key: WindowKey) {
        if key == WindowKey::MAIN {
            return;
        }
        {
            let _scope = iron_scene::api::enter(&mut self.scene.scene);
            native_render::drop_window_runtime(key.0);
        }
        self.windows.map.remove(&key);
        self.windows.focused.remove(&key);
        self.windows.placed.remove(&key);
        self.windows.pending_commits.remove(&key.0);
        self.windows.slugs.remove(&key);
        if matches!(self.keys_on, Surface::Window(k) if k == key.0) {
            self.keys_on = Surface::Screen;
        }
        let w = WindowRef(key.0);
        self.in_game(|| win::closed(w));
    }

    /// Once per loop iteration, after the events: the game's focus is any of its windows'.
    pub(super) fn window_events_done(&mut self) {
        let focused = app_focused(&self.windows.focused);
        if self.main_focus_sent != Some(focused) {
            self.main_focus_sent = Some(focused);
            self.lifecycle.set_focused(focused);
            self.sync_lifecycle();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iron_engine_api::window::Flags;

    fn state(title: &str, style: Style, parent: Option<WindowRef>) -> WindowState {
        WindowState {
            key: String::new(),
            title: title.into(),
            style,
            parent,
            position: None,
            size: (200.0, 100.0),
            min_size: None,
            scale: 1.0,
            flags: Flags::default(),
            minimized: false,
            maximized: false,
            focused: false,
            created: false,
            open: true,
            open_error: None,
            embedded: false,
            run_in_background: true,
            native_handle: 0,
            platform: String::new(),
            order: 0,
        }
    }

    fn main_state() -> WindowState {
        WindowState {
            position: Some((100.0, 50.0)),
            size: (800.0, 600.0),
            ..state("Iron", Style::Normal, None)
        }
    }

    // M9-API-1460
    #[test]
    fn script_requests_become_platform_commands() {
        let tool = state("Palette", Style::Tool, Some(win::MAIN));
        let lookup = |w: WindowRef| match w.0 {
            1 => Some(main_state()),
            2 => Some(tool.clone()),
            _ => None,
        };
        let reqs = vec![
            WindowRequest::Create(WindowRef(2)),
            WindowRequest::Title(win::MAIN, "Game".into()),
            WindowRequest::Title(WindowRef(2), "P".into()),
            WindowRequest::Close(win::MAIN),
            WindowRequest::Close(WindowRef(2)),
        ];
        let (cmds, title) = requests_to_commands(reqs, lookup, &[win::MAIN, WindowRef(2)]);
        assert_eq!(title.as_deref(), Some("Game"));
        assert_eq!(
            cmds.len(),
            3,
            "Close(MAIN) and the main title are not commands"
        );
        let WindowCommand::Open(k, spec) = &cmds[0] else {
            panic!("{:?}", cmds[0]);
        };
        assert_eq!(*k, WindowKey(2));
        assert_eq!(spec.kind, WindowKind::Tool);
        assert_eq!(spec.parent, Some(WindowKey::MAIN));
        assert_eq!(spec.title, "Palette");
        assert!(
            matches!(&cmds[1], WindowCommand::SetTitle(k, t) if *k == WindowKey(2) && t == "P")
        );
        assert!(matches!(&cmds[2], WindowCommand::Close(k) if *k == WindowKey(2)));
    }

    // M9-API-1461
    #[test]
    fn windows_without_a_position_are_placed_on_their_parent() {
        let a = state("A", Style::Tool, Some(win::MAIN));
        let b = state("B", Style::Tool, Some(win::MAIN));
        let c = WindowState {
            position: Some((5.0, 6.0)),
            ..state("C", Style::Tool, Some(win::MAIN))
        };
        let lookup = |w: WindowRef| match w.0 {
            1 => Some(main_state()),
            2 => Some(a.clone()),
            3 => Some(b.clone()),
            4 => Some(c.clone()),
            _ => None,
        };
        let open = [win::MAIN, WindowRef(2), WindowRef(3), WindowRef(4)];
        let reqs = (2..=4)
            .map(|n| WindowRequest::Create(WindowRef(n)))
            .collect();
        let (cmds, _) = requests_to_commands(reqs, lookup, &open);
        let pos = |i: usize| match &cmds[i] {
            WindowCommand::Open(_, s) => s.position,
            _ => None,
        };
        // Centred on the main window (100, 50, 800 x 600), 24 px further for the second child.
        assert_eq!(pos(0), Some((100.0 + 300.0, 50.0 + 250.0)));
        assert_eq!(pos(1), Some((100.0 + 300.0 + 24.0, 50.0 + 250.0 + 24.0)));
        assert_eq!(pos(2), Some((5.0, 6.0)));
    }

    // M9-API-1462
    #[test]
    fn screenshot_names_are_slugs_of_the_titles_and_unique() {
        let mut used = HashSet::new();
        assert_eq!(slug("Counter 1", 2, &mut used), "counter-1");
        assert_eq!(slug("!!", 3, &mut used), "window3");
        assert_eq!(slug("Counter 1", 4, &mut used), "counter-1-4");
        assert_eq!(
            slug("  --Tool: Palette (x)--", 5, &mut used),
            "tool-palette-x"
        );
    }

    // M9-API-1463
    #[test]
    fn the_game_is_focused_while_any_of_its_windows_is() {
        let mut f = BTreeSet::from([WindowKey::MAIN]);
        assert!(app_focused(&f));
        // Focus moving to a tool window: Focused(false) then Focused(true) in one batch.
        f.remove(&WindowKey::MAIN);
        f.insert(WindowKey(2));
        assert!(app_focused(&f));
        f.clear();
        assert!(!app_focused(&f));
    }

    // M9-API-1464
    #[test]
    fn window_targets_do_not_collide_with_panels() {
        let panels = [0u32, 1, 7, 4096, 1 << 20];
        for w in [2u32, 3, 500] {
            let t = iron_core::WINDOW_TARGET | w;
            assert!(t >= iron_core::WINDOW_TARGET);
            assert_eq!(t - iron_core::WINDOW_TARGET, w);
            assert!(!panels.contains(&t));
        }
        assert_eq!(to_ui((1600, 1200), 2.0), (800.0, 600.0));
    }
}
