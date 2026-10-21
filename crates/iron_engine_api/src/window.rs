//! `Window`: the game's windows as scripts see them (design: `docs/research/iron-windows.md`). The
//! main window exists from the start; `Window.create({ .. })` makes more, each with its own UI root.
//!
//! Per-thread state with host functions, like `screen.rs`. The host (the app, through the platform)
//! turns what scripts asked for ([`take_requests`]) into OS windows and reports back ([`opened`],
//! [`resized`], [`moved`], [`focus_changed`], [`close_requested`], [`closed`], ...). Scripts hold
//! [`WindowRef`] handles (numbers); a handle read after the window closed still gives its last values
//! and its setters do nothing. No winit types here: [`Style`] has its own enum.
//!
//! Families and event domains (design section 3a): a window without a parent (the main window, or a
//! "normal" / "borderless" window made without one) is the root of a family; every window below it
//! shares its [`DomainId`], the root's handle. Code of one domain may read the windows of another but
//! not change them (the way across is messages); the exception is the setup of a new isolated root by
//! its creator, until [`end_frame`].

use crate::log::{Level, emit as log_emit};
use iron_api::callback::{Listeners, emit};
use iron_api::{Callback, Handle, iron_api};
use std::cell::RefCell;
use std::collections::BTreeMap;

/// A window handle: 1 is the main window, then 2, 3, ... never reused in a run.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WindowRef(pub u32);

impl Handle for WindowRef {
    fn to_raw(self) -> u64 {
        u64::from(self.0)
    }
    fn from_raw(raw: u64) -> Self {
        WindowRef(raw as u32)
    }
}

impl WindowRef {
    pub const NONE: WindowRef = WindowRef(0);
}

/// The window that owns the process.
pub const MAIN: WindowRef = WindowRef(1);

/// An event domain: the handle of its family's root window (1 for the main family).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DomainId(pub u32);

impl DomainId {
    pub const MAIN: DomainId = DomainId(1);

    /// The root window of the domain.
    pub fn root(self) -> WindowRef {
        WindowRef(self.0)
    }
}

/// How a window looks and behaves (design section 4); the host maps it to `iron_platform`'s kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Style {
    #[default]
    Normal,
    Tool,
    Dialog,
    Popup,
    Borderless,
    Child,
}

impl Style {
    pub fn parse(s: &str) -> Option<Style> {
        Some(match s {
            "normal" => Style::Normal,
            "tool" => Style::Tool,
            "dialog" => Style::Dialog,
            "popup" => Style::Popup,
            "borderless" => Style::Borderless,
            "child" => Style::Child,
            _ => return None,
        })
    }

    pub fn name(self) -> &'static str {
        match self {
            Style::Normal => "normal",
            Style::Tool => "tool",
            Style::Dialog => "dialog",
            Style::Popup => "popup",
            Style::Borderless => "borderless",
            Style::Child => "child",
        }
    }

    /// Styles that always belong to a window (the owner defaults to the main one).
    fn needs_parent(self) -> bool {
        matches!(
            self,
            Style::Tool | Style::Dialog | Style::Popup | Style::Child
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Flags {
    pub resizable: bool,
    pub minimizable: bool,
    pub maximizable: bool,
    pub closable: bool,
    pub always_on_top: bool,
    pub visible: bool,
    /// Takes keyboard focus when shown (false for popups, which never do).
    pub focus_on_show: bool,
}

impl Default for Flags {
    fn default() -> Self {
        Flags {
            resizable: true,
            minimizable: true,
            maximizable: true,
            closable: true,
            always_on_top: false,
            visible: true,
            focus_on_show: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct WindowState {
    pub key: String,
    pub title: String,
    pub style: Style,
    pub parent: Option<WindowRef>,
    /// Outer top-left (None: let the host place it) and client size, UI pixels.
    pub position: Option<(f64, f64)>,
    pub size: (f64, f64),
    pub min_size: Option<(f64, f64)>,
    pub scale: f64,
    pub flags: Flags,
    pub minimized: bool,
    pub maximized: bool,
    pub focused: bool,
    /// The host made the OS (or offscreen) window.
    pub created: bool,
    pub open: bool,
    /// Drawn inside its parent's client area (set by the host, [`set_embedded`]).
    pub embedded: bool,
    /// Roots of isolated families: keep the domain's timers running while it is unfocused.
    pub run_in_background: bool,
    pub native_handle: u64,
    pub platform: String,
    /// Creation order, for `count` / `at` and the cascade offset.
    pub order: u32,
}

/// What scripts asked for since the last [`take_requests`], in order. Setters of the same field on the
/// same window coalesce (the last value wins) except `Create` and `Close`, which keep their place.
#[derive(Clone, Debug, PartialEq)]
pub enum WindowRequest {
    /// Read the spec with [`state`].
    Create(WindowRef),
    Title(WindowRef, String),
    Style(WindowRef, Style),
    Position(WindowRef, f64, f64),
    Size(WindowRef, f64, f64),
    Center(WindowRef),
    Flags(WindowRef, Flags),
    Minimized(WindowRef, bool),
    Maximized(WindowRef, bool),
    Focus(WindowRef),
    /// The host closes children first and reports each with [`closed`].
    Close(WindowRef),
}

impl WindowRequest {
    fn window(&self) -> WindowRef {
        match self {
            WindowRequest::Create(w)
            | WindowRequest::Title(w, _)
            | WindowRequest::Style(w, _)
            | WindowRequest::Position(w, ..)
            | WindowRequest::Size(w, ..)
            | WindowRequest::Center(w)
            | WindowRequest::Flags(w, _)
            | WindowRequest::Minimized(w, _)
            | WindowRequest::Maximized(w, _)
            | WindowRequest::Focus(w)
            | WindowRequest::Close(w) => *w,
        }
    }
}

/// What the host learned when it made a window.
#[derive(Clone, Debug, PartialEq)]
pub struct Opened {
    pub position: (f64, f64),
    pub size: (f64, f64),
    pub scale: f64,
    pub native_handle: u64,
    pub platform: String,
}

pub const MIN_SIZE: f64 = 1.0;
pub const MAX_SIZE: f64 = 16384.0;

#[derive(Default)]
struct WindowListeners {
    close_requested: Listeners<()>,
    closed: Listeners<()>,
    resized: Listeners<(f64, f64)>,
    moved: Listeners<(f64, f64)>,
    focus: Listeners<(bool,)>,
    scale: Listeners<(f64,)>,
}

impl WindowListeners {
    fn remove(&mut self, token: u32) -> bool {
        self.close_requested.remove(token)
            || self.closed.remove(token)
            || self.resized.remove(token)
            || self.moved.remove(token)
            || self.focus.remove(token)
            || self.scale.remove(token)
    }
}

struct State {
    windows: BTreeMap<WindowRef, WindowState>,
    next: u32,
    requests: Vec<WindowRequest>,
    listeners: BTreeMap<WindowRef, WindowListeners>,
    current: Option<WindowRef>,
    cancel: bool,
    in_close_request: Option<WindowRef>,
    /// Creation counter for `WindowState::order`.
    order: u32,
    /// The domain whose code runs now.
    domain: DomainId,
    /// `run` callbacks waiting for their domain's next dispatch.
    run_queue: BTreeMap<DomainId, Vec<Callback<()>>>,
    /// New windows a domain other than their own may still set up this frame: window to creator.
    setup: BTreeMap<WindowRef, DomainId>,
    /// (focused, paused) per domain, as the host last said.
    domain_focus: BTreeMap<DomainId, (bool, bool)>,
    domain_listeners: BTreeMap<DomainId, Listeners<(bool, bool)>>,
}

fn main_state() -> WindowState {
    WindowState {
        key: String::new(),
        title: "Iron".into(),
        style: Style::Normal,
        parent: None,
        position: None,
        size: (1280.0, 720.0),
        min_size: None,
        scale: 1.0,
        flags: Flags::default(),
        minimized: false,
        maximized: false,
        focused: false,
        created: true,
        open: true,
        embedded: false,
        run_in_background: true,
        native_handle: 0,
        platform: String::new(),
        order: 0,
    }
}

impl State {
    fn new() -> State {
        State {
            windows: BTreeMap::from([(MAIN, main_state())]),
            next: 2,
            requests: Vec::new(),
            listeners: BTreeMap::new(),
            current: None,
            cancel: false,
            in_close_request: None,
            order: 1,
            domain: DomainId::MAIN,
            run_queue: BTreeMap::new(),
            setup: BTreeMap::new(),
            domain_focus: BTreeMap::new(),
            domain_listeners: BTreeMap::new(),
        }
    }

    /// The root of `w`'s family (the window itself when it has no parent); `w` when unknown.
    fn root_of(&self, w: WindowRef) -> WindowRef {
        let mut cur = w;
        // A parent chain is short; the bound only guards against a broken state.
        for _ in 0..1024 {
            match self.windows.get(&cur).and_then(|st| st.parent) {
                Some(p) => cur = p,
                None => break,
            }
        }
        cur
    }

    fn domain_of(&self, w: WindowRef) -> DomainId {
        if self.windows.contains_key(&w) {
            DomainId(self.root_of(w).0)
        } else {
            DomainId::MAIN
        }
    }

    /// May the running domain change `w`? Warns when not.
    fn may_change(&self, w: WindowRef) -> bool {
        let d = self.domain_of(w);
        if d == self.domain {
            return true;
        }
        warn(&format!(
            "window {} belongs to another family (domain {}) and cannot be changed from domain {}; send it a message instead",
            w.0, d.0, self.domain.0
        ));
        false
    }

    /// Like [`State::may_change`], and true for the creator of `w` during setup.
    fn may_set_up(&self, w: WindowRef) -> bool {
        self.setup.get(&w) == Some(&self.domain) || self.may_change(w)
    }

    /// Queues a request; a request for the same field of the same window replaces the pending one.
    fn push(&mut self, req: WindowRequest) {
        let coalesces = !matches!(req, WindowRequest::Create(_) | WindowRequest::Close(_));
        if coalesces
            && let Some(old) = self.requests.iter_mut().rev().find(|r| {
                r.window() == req.window()
                    && std::mem::discriminant(*r) == std::mem::discriminant(&req)
            })
        {
            *old = req;
            return;
        }
        self.requests.push(req);
    }

    fn open_in_order(&self) -> Vec<WindowRef> {
        let mut v: Vec<(u32, WindowRef)> = self
            .windows
            .iter()
            .filter(|(_, s)| s.open)
            .map(|(w, s)| (s.order, *w))
            .collect();
        v.sort();
        v.into_iter().map(|(_, w)| w).collect()
    }

    fn children_of(&self, w: WindowRef) -> Vec<WindowRef> {
        self.open_in_order()
            .into_iter()
            .filter(|c| self.windows[c].parent == Some(w))
            .collect()
    }
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::new());
}

fn with<R>(f: impl FnOnce(&mut State) -> R) -> R {
    STATE.with(|s| f(&mut s.borrow_mut()))
}

fn warn(message: &str) {
    log_emit(Level::Warn, message);
}

/// Calls the listeners of `w` with its domain current.
fn emit_in<A: Clone + 'static>(w: WindowRef, cbs: &[Callback<A>], args: A) {
    let _g = enter_domain(domain_of(w));
    emit(cbs, args);
}

// ---- Host functions ----

/// At startup, before scripts run: the main window's title (the manifest's), client size (UI pixels)
/// and scale. Called again when the real scale is known.
pub fn set_main(title: &str, size: (f64, f64), scale: f64) {
    with(|s| {
        let m = s.windows.get_mut(&MAIN).expect("the main window exists");
        m.title = title.to_string();
        m.size = size;
        m.scale = scale;
    });
}

/// What scripts asked for since the last call, in order.
pub fn take_requests() -> Vec<WindowRequest> {
    with(|s| std::mem::take(&mut s.requests))
}

pub fn state(w: WindowRef) -> Option<WindowState> {
    with(|s| s.windows.get(&w).cloned())
}

/// Open windows, main first, then in creation order.
pub fn open_windows() -> Vec<WindowRef> {
    with(|s| s.open_in_order())
}

/// The host made the window `w` (an OS window, or an offscreen one).
pub fn opened(w: WindowRef, info: Opened) {
    with(|s| {
        if let Some(st) = s.windows.get_mut(&w).filter(|st| st.open) {
            st.created = true;
            st.position = Some(info.position);
            st.size = info.size;
            st.scale = info.scale;
            st.native_handle = info.native_handle;
            st.platform = info.platform;
        }
    });
}

/// The host could not make the window: logs, then as [`closed`].
pub fn open_failed(w: WindowRef, error: &str) {
    warn(&format!("window {} could not be opened: {error}", w.0));
    closed(w);
}

/// The host says whether a `child` window ended up embedded in its parent (it falls back to a
/// tool window on some platforms).
pub fn set_embedded(w: WindowRef, embedded: bool) {
    with(|s| {
        if let Some(st) = s.windows.get_mut(&w) {
            st.embedded = embedded;
        }
    });
}

/// The client area changed (UI pixels): runs `onResized` when the size differs from the stored one.
pub fn resized(w: WindowRef, width: f64, height: f64) {
    let calls = with(|s| {
        let st = s.windows.get_mut(&w)?;
        if !st.open || st.size == (width, height) {
            return None;
        }
        st.size = (width, height);
        Some(s.listeners.get(&w)?.resized.snapshot())
    });
    if let Some(cbs) = calls {
        emit_in(w, &cbs, (width, height));
    }
}

/// The outer top-left moved (desktop UI pixels).
pub fn moved(w: WindowRef, x: f64, y: f64) {
    let calls = with(|s| {
        let st = s.windows.get_mut(&w)?;
        if !st.open || st.position == Some((x, y)) {
            return None;
        }
        st.position = Some((x, y));
        Some(s.listeners.get(&w)?.moved.snapshot())
    });
    if let Some(cbs) = calls {
        emit_in(w, &cbs, (x, y));
    }
}

/// The OS focus of one window changed.
pub fn focus_changed(w: WindowRef, focused: bool) {
    let calls = with(|s| {
        let st = s.windows.get_mut(&w)?;
        if !st.open || st.focused == focused {
            return None;
        }
        st.focused = focused;
        Some(s.listeners.get(&w)?.focus.snapshot())
    });
    if let Some(cbs) = calls {
        emit_in(w, &cbs, (focused,));
    }
}

/// The window moved to a monitor with another scale (physical pixels per UI pixel).
pub fn scale_changed(w: WindowRef, scale: f64) {
    let calls = with(|s| {
        let st = s.windows.get_mut(&w)?;
        if !st.open || st.scale == scale {
            return None;
        }
        st.scale = scale;
        Some(s.listeners.get(&w)?.scale.snapshot())
    });
    if let Some(cbs) = calls {
        emit_in(w, &cbs, (scale,));
    }
}

pub fn minimized_changed(w: WindowRef, minimized: bool) {
    with(|s| {
        if let Some(st) = s.windows.get_mut(&w) {
            st.minimized = minimized;
        }
    });
}

/// The user asked to close `w`: runs `onCloseRequested`; true unless a listener called `cancelClose()`.
pub fn close_requested(w: WindowRef) -> bool {
    let (cbs, prev) = with(|s| {
        let cbs = s
            .listeners
            .get(&w)
            .map(|l| l.close_requested.snapshot())
            .unwrap_or_default();
        let prev = (s.in_close_request.replace(w), s.cancel);
        s.cancel = false;
        (cbs, prev)
    });
    emit_in(w, &cbs, ());
    with(|s| {
        let allowed = !s.cancel;
        s.in_close_request = prev.0;
        s.cancel = prev.1;
        allowed
    })
}

/// `w` is gone: marks it closed, runs `onClosed` once, then drops its listeners.
pub fn closed(w: WindowRef) {
    let calls = with(|s| {
        let st = s.windows.get_mut(&w)?;
        if !st.open {
            return None;
        }
        st.open = false;
        st.focused = false;
        s.requests.retain(|r| r.window() != w);
        Some(
            s.listeners
                .get(&w)
                .map(|l| l.closed.snapshot())
                .unwrap_or_default(),
        )
    });
    if let Some(cbs) = calls {
        emit_in(w, &cbs, ());
        with(|s| s.listeners.remove(&w));
    }
}

/// Restores the previous `Window.current` when dropped.
pub struct CurrentGuard {
    prev: Option<WindowRef>,
}

impl Drop for CurrentGuard {
    fn drop(&mut self) {
        let prev = self.prev;
        with(|s| s.current = prev);
    }
}

/// While the host dispatches an event of `w` (UI or window callback): `Window.current` is `w`.
pub fn enter_current(w: WindowRef) -> CurrentGuard {
    CurrentGuard {
        prev: with(|s| s.current.replace(w)),
    }
}

/// Restores the previous domain (and `Window.current`) when dropped.
pub struct DomainGuard {
    prev: (DomainId, Option<WindowRef>),
}

impl Drop for DomainGuard {
    fn drop(&mut self) {
        let prev = self.prev;
        with(|s| {
            s.domain = prev.0;
            s.current = prev.1;
        });
    }
}

/// Runs code as domain `d`: `current_domain()` is `d`, and `Window.current` is `d`'s root unless a
/// window event sets it ([`enter_current`]).
pub fn enter_domain(d: DomainId) -> DomainGuard {
    DomainGuard {
        prev: with(|s| (std::mem::replace(&mut s.domain, d), s.current.take())),
    }
}

/// The domain whose code runs now (the main one outside any [`enter_domain`]).
pub fn current_domain() -> DomainId {
    with(|s| s.domain)
}

/// The domain of a window: its family root's handle (the main domain for an unknown window).
pub fn domain_of(w: WindowRef) -> DomainId {
    with(|s| s.domain_of(w))
}

/// Domains with an open root: the main one first, then by root creation order.
pub fn domains() -> Vec<DomainId> {
    with(|s| {
        s.open_in_order()
            .into_iter()
            .filter(|w| s.windows[w].parent.is_none())
            .map(|w| DomainId(w.0))
            .collect()
    })
}

/// Whether a domain keeps running while unfocused (its root's `runInBackground`; true for the main
/// one, whose rule is `Application.runInBackground`).
pub fn run_in_background(d: DomainId) -> bool {
    with(|s| {
        s.windows
            .get(&d.root())
            .is_none_or(|st| st.run_in_background)
    })
}

/// The host says a domain's focus / pause changed (it aggregates OS focus per family): runs
/// `onDomainFocusChanged` of that domain.
pub fn domain_focus_changed(d: DomainId, focused: bool, paused: bool) {
    let cbs = with(|s| {
        s.domain_focus.insert(d, (focused, paused));
        s.domain_listeners
            .get(&d)
            .map(|l| l.snapshot())
            .unwrap_or_default()
    });
    let _g = enter_domain(d);
    emit(&cbs, (focused, paused));
}

/// The host calls this once per frame: windows made this frame stop being open to setup by their
/// creator's domain.
pub fn end_frame() {
    with(|s| s.setup.clear());
}

/// Runs the callbacks `Window.run` queued for `d`, each with `d` current; returns how many.
pub fn run_queued(d: DomainId) -> usize {
    let cbs = with(|s| s.run_queue.remove(&d).unwrap_or_default());
    let _g = enter_domain(d);
    for c in &cbs {
        c.call(());
    }
    cbs.len()
}

/// Whether the running domain may set `w` up: it is in `w`'s domain, or it made `w` this frame.
/// The app asks before it mounts a root for `createRoot(win)`.
pub fn can_set_up(w: WindowRef) -> bool {
    with(|s| s.setup.get(&w) == Some(&s.domain) || s.domain_of(w) == s.domain)
}

/// Tests: only the main window, no listeners, no requests.
pub fn reset() {
    with(|s| *s = State::new());
}

// ---- API helpers ----

/// The state of an open window, for a read.
fn read<R>(w: WindowRef, f: impl FnOnce(&WindowState) -> R) -> Option<R> {
    with(|s| s.windows.get(&w).map(f))
}

/// A write to an open window (a closed or unknown one is left alone); returns whether it ran.
fn write(w: WindowRef, f: impl FnOnce(&mut State, WindowRef)) -> bool {
    with(|s| {
        if s.windows.get(&w).is_some_and(|st| st.open) && s.may_change(w) {
            f(s, w);
            true
        } else {
            false
        }
    })
}

fn clamp_size(v: f64) -> f64 {
    v.clamp(MIN_SIZE, MAX_SIZE)
}

/// The window's outer top-left as scripts read it: what the OS reported, else (before the host
/// places it) centred on the parent with the cascade offset, so a pending window has final numbers.
fn effective_position(s: &State, w: WindowRef) -> (f64, f64) {
    let Some(st) = s.windows.get(&w) else {
        return (0.0, 0.0);
    };
    if let Some(p) = st.position {
        return p;
    }
    let Some(parent) = st.parent.and_then(|p| s.windows.get(&p)) else {
        return (0.0, 0.0);
    };
    let pp = parent.position.unwrap_or((0.0, 0.0));
    let n = s
        .children_of(st.parent.unwrap_or(MAIN))
        .iter()
        .position(|c| *c == w)
        .unwrap_or(0) as u32;
    let off = 24.0 * f64::from(n % 8);
    (
        pp.0 + (parent.size.0 - st.size.0) / 2.0 + off,
        pp.1 + (parent.size.1 - st.size.1) / 2.0 + off,
    )
}

#[allow(clippy::too_many_arguments)]
fn create_window(
    title: &str,
    width: f64,
    height: f64,
    x: f64,
    y: f64,
    min_width: f64,
    min_height: f64,
    style: &str,
    parent: WindowRef,
    flags: Flags,
    key: &str,
    run_in_background: bool,
) -> WindowRef {
    let style_v = if style.is_empty() {
        Style::Normal
    } else {
        Style::parse(style).unwrap_or_else(|| {
            warn(&format!(
                "Window.create: unknown style \"{style}\"; using \"normal\""
            ));
            Style::Normal
        })
    };
    with(|s| {
        // A parent must be an open window of the creating domain (or one it is setting up this
        // frame); none means the root of the creating domain for the styles that always have an
        // owner (the main window, in the main domain), and a new isolated root for the others.
        let cur_root = s.domain.root();
        let cur_root = if s.windows.get(&cur_root).is_some_and(|st| st.open) {
            cur_root
        } else {
            MAIN
        };
        let wanted = Some(parent)
            .filter(|p| *p != WindowRef::NONE)
            .filter(|p| s.windows.get(p).is_some_and(|st| st.open));
        let wanted = wanted.filter(|p| {
            let ok = s.setup.get(p) == Some(&s.domain) || s.domain_of(*p) == s.domain;
            if !ok {
                warn(&format!(
                    "Window.create: window {} belongs to another family and cannot be a parent from domain {}; ignored",
                    p.0, s.domain.0
                ));
            }
            ok
        });
        let parent = wanted.or(style_v.needs_parent().then_some(cur_root));
        let size = (
            if width.is_nan() {
                400.0
            } else {
                clamp_size(width)
            },
            if height.is_nan() {
                300.0
            } else {
                clamp_size(height)
            },
        );
        let position = (!x.is_nan() && !y.is_nan()).then_some((x, y));
        let min_size = (!min_width.is_nan() || !min_height.is_nan()).then(|| {
            (
                if min_width.is_nan() { 0.0 } else { min_width },
                if min_height.is_nan() { 0.0 } else { min_height },
            )
        });
        if !key.is_empty() && s.windows.values().any(|st| st.open && st.key == key) {
            warn(&format!(
                "Window.create: the key \"{key}\" is already taken; Window.find returns the first window"
            ));
        }
        let mut flags = flags;
        flags.focus_on_show &= style_v != Style::Popup;
        let w = WindowRef(s.next);
        s.next += 1;
        let order = s.order;
        s.order += 1;
        s.windows.insert(
            w,
            WindowState {
                key: key.to_string(),
                title: title.to_string(),
                style: style_v,
                parent,
                position,
                size,
                min_size,
                scale: s.windows[&MAIN].scale,
                flags,
                minimized: false,
                maximized: false,
                focused: false,
                created: false,
                open: true,
                embedded: false,
                run_in_background,
                native_handle: 0,
                platform: String::new(),
                order,
            },
        );
        // The creator may set up a window of another domain (a new isolated root, or a window
        // under one) until the end of the frame.
        if s.domain_of(w) != s.domain {
            let creator = s.domain;
            s.setup.insert(w, creator);
        }
        s.push(WindowRequest::Create(w));
        w
    })
}

fn push_flags(s: &mut State, w: WindowRef) {
    let f = s.windows[&w].flags.clone();
    s.push(WindowRequest::Flags(w, f));
}

fn add_listener<A: Clone + 'static>(
    w: WindowRef,
    f: Callback<A>,
    pick: impl FnOnce(&mut WindowListeners) -> &mut Listeners<A>,
) -> u32 {
    with(|s| {
        if !s.windows.get(&w).is_some_and(|st| st.open) || !s.may_set_up(w) {
            return 0;
        }
        pick(s.listeners.entry(w).or_default()).add(f)
    })
}

/// The game's windows. `Window.main` is the original one; `Window.create({ .. })` makes more.
#[iron_api(namespace = "Window", quickjs, handles(Window = WindowRef))]
pub mod window_api {
    use super::*;

    impl WindowRef {
        /// Opens a window. It exists (and has a handle) at once; the OS window appears at the end of the
        /// frame. Until then its size and position are the ones asked for. Every option is optional:
        /// `Window.create({ title: "Palette", style: "tool", width: 220, height: 320, key: "palette" })`.
        #[options]
        #[optional(
            title,
            width,
            height,
            x,
            y,
            min_width,
            min_height,
            style,
            parent,
            resizable = true,
            minimizable = true,
            maximizable = true,
            closable = true,
            always_on_top = false,
            visible = true,
            focus = true,
            key,
            run_in_background = true
        )]
        #[allow(clippy::too_many_arguments)]
        pub fn create(
            title: &str,
            width: f64,
            height: f64,
            x: f64,
            y: f64,
            min_width: f64,
            min_height: f64,
            style: &str,
            parent: WindowRef,
            resizable: bool,
            minimizable: bool,
            maximizable: bool,
            closable: bool,
            always_on_top: bool,
            visible: bool,
            focus: bool,
            key: &str,
            run_in_background: bool,
        ) -> WindowRef {
            create_window(
                title,
                width,
                height,
                x,
                y,
                min_width,
                min_height,
                style,
                parent,
                Flags {
                    resizable,
                    minimizable,
                    maximizable,
                    closable,
                    always_on_top,
                    visible,
                    focus_on_show: focus,
                },
                key,
                run_in_background,
            )
        }

        /// A reference to the original window, the one that owns the process. Always there while the
        /// game runs; closing it ends the game.
        #[property]
        pub fn main() -> WindowRef {
            MAIN
        }

        /// The window with keyboard focus, if one of the game's has it.
        #[property]
        pub fn focused() -> WindowRef {
            with(|s| {
                s.open_in_order()
                    .into_iter()
                    .find(|w| s.windows[w].focused)
                    .unwrap_or(WindowRef::NONE)
            })
        }

        /// The window whose event is being handled now (a UI handler, a window callback), else the main
        /// window.
        #[property]
        pub fn current() -> WindowRef {
            with(|s| s.current.unwrap_or(s.domain.root()))
        }

        /// Open windows, the main one first.
        #[property]
        pub fn count() -> u32 {
            with(|s| s.open_in_order().len() as u32)
        }

        /// The `index`-th open window, the main one first, then in creation order.
        pub fn at(index: f64) -> WindowRef {
            with(|s| {
                let v = s.open_in_order();
                if index.is_nan() || index < 0.0 {
                    return WindowRef::NONE;
                }
                v.get(index as usize).copied().unwrap_or(WindowRef::NONE)
            })
        }

        /// The open window made with this `key`.
        pub fn find(key: &str) -> WindowRef {
            if key.is_empty() {
                return WindowRef::NONE;
            }
            with(|s| {
                s.open_in_order()
                    .into_iter()
                    .find(|w| s.windows[w].key == key)
                    .unwrap_or(WindowRef::NONE)
            })
        }

        /// The open window with this `handle` (for handles received in messages).
        pub fn by_handle(handle: f64) -> WindowRef {
            with(|s| {
                if handle.is_nan() || handle < 1.0 || handle > f64::from(u32::MAX) {
                    return WindowRef::NONE;
                }
                let w = WindowRef(handle as u32);
                if s.windows.get(&w).is_some_and(|st| st.open) {
                    w
                } else {
                    WindowRef::NONE
                }
            })
        }

        // ---- Identity ----

        /// Iron's id: 1 for the main window, then 2, 3, ... never reused in a run.
        #[property]
        pub fn handle(self) -> f64 {
            f64::from(self.0)
        }

        /// The OS handle as a number: HWND on Windows, the NSView pointer on macOS, the X11 window id;
        /// 0 when there is none (a screenshot run's offscreen window, Wayland, before the OS window
        /// exists). For passing to native plugins.
        #[property]
        pub fn native_handle(self) -> f64 {
            read(self, |s| s.native_handle as f64).unwrap_or(0.0)
        }

        /// "win32", "appkit", "x11", "wayland", "virtual" (offscreen) or "" (not made yet).
        #[property]
        pub fn platform(self) -> String {
            read(self, |s| s.platform.clone()).unwrap_or_default()
        }

        #[property]
        pub fn key(self) -> String {
            read(self, |s| s.key.clone()).unwrap_or_default()
        }

        #[property]
        pub fn is_main(self) -> bool {
            self == MAIN
        }

        /// False once closed; a closed window's members read its last values and its setters do nothing.
        #[property]
        pub fn is_open(self) -> bool {
            read(self, |s| s.open).unwrap_or(false)
        }

        // ---- Family ----

        /// The owner or embedding parent; none for the main window and unowned windows.
        #[property]
        pub fn parent(self) -> WindowRef {
            read(self, |s| s.parent)
                .flatten()
                .unwrap_or(WindowRef::NONE)
        }

        /// True when the window has a parent: tool windows, dialogs, popups, children, and normal windows
        /// given a parent.
        #[property]
        pub fn is_child(self) -> bool {
            read(self, |s| s.parent.is_some()).unwrap_or(false)
        }

        /// True for style "child" only: drawn inside its parent's client area (where the platform can).
        #[property]
        pub fn is_embedded(self) -> bool {
            read(self, |s| s.embedded).unwrap_or(false)
        }

        /// The root of its family: follow `parent` to the end (itself for a root).
        /// `Window.main.root` is `Window.main`.
        #[property]
        pub fn root(self) -> WindowRef {
            with(|s| {
                if s.windows.contains_key(&self) {
                    s.root_of(self)
                } else {
                    WindowRef::NONE
                }
            })
        }

        /// True for the root of a family other than the main one: its own event domain.
        #[property]
        pub fn is_isolated(self) -> bool {
            with(|s| self != MAIN && s.windows.contains_key(&self) && s.root_of(self) == self)
        }

        /// The event domain's id: the root's handle (1 for the main family).
        #[property]
        pub fn domain(self) -> f64 {
            with(|s| f64::from(s.domain_of(self).0))
        }

        /// Runs `f` once in this window's domain at the start of its next dispatch (the setup of an
        /// isolated domain). Allowed for the window's own domain at any time, and for its creator
        /// during setup. False when refused or the window is closed.
        pub fn run(self, f: Callback<()>) -> bool {
            with(|s| {
                if !s.windows.get(&self).is_some_and(|st| st.open) || !s.may_set_up(self) {
                    return false;
                }
                let d = s.domain_of(self);
                s.run_queue.entry(d).or_default().push(f);
                true
            })
        }

        /// Whether the domain of the code running now has focus (one of its family's windows is
        /// focused).
        #[property]
        pub fn domain_focused() -> bool {
            with(|s| s.domain_focus.get(&s.domain).is_some_and(|f| f.0))
        }

        /// Calls `f(focused, paused)` when the current domain's focus changes; `paused` is true when
        /// that stops its timers.
        pub fn on_domain_focus_changed(f: Callback<(bool, bool)>) -> u32 {
            with(|s| {
                let d = s.domain;
                s.domain_listeners.entry(d).or_default().add(f)
            })
        }

        /// Removes a static listener by token.
        #[js_name = "off"]
        pub fn off_static(token: f64) -> bool {
            if token.is_nan() || token < 1.0 {
                return false;
            }
            with(|s| {
                let t = token as u32;
                let d = s.domain;
                s.domain_listeners.get_mut(&d).is_some_and(|l| l.remove(t))
            })
        }

        /// Open windows whose parent this is.
        #[property]
        pub fn child_count(self) -> u32 {
            with(|s| s.children_of(self).len() as u32)
        }

        pub fn child_at(self, index: f64) -> WindowRef {
            with(|s| {
                if index.is_nan() || index < 0.0 {
                    return WindowRef::NONE;
                }
                s.children_of(self)
                    .get(index as usize)
                    .copied()
                    .unwrap_or(WindowRef::NONE)
            })
        }

        // ---- Look ----

        /// Changing it changes the OS title. The main window opens with the manifest title.
        #[property]
        pub fn title(self) -> String {
            read(self, |s| s.title.clone()).unwrap_or_default()
        }

        pub fn set_title(self, t: &str) {
            write(self, |s, w| {
                s.windows.get_mut(&w).expect("open").title = t.to_string();
                s.push(WindowRequest::Title(w, t.to_string()));
            });
        }

        /// Changing it re-applies decorations, taskbar and z-order rules; "child" cannot be set or unset
        /// after creation.
        #[property]
        pub fn style(self) -> String {
            read(self, |s| s.style.name().to_string()).unwrap_or_default()
        }

        pub fn set_style(self, v: &str) {
            let Some(style) = Style::parse(v) else {
                warn(&format!("Window.style: unknown style \"{v}\"; ignored"));
                return;
            };
            write(self, |s, w| {
                let st = s.windows.get_mut(&w).expect("open");
                if style == st.style {
                    return;
                }
                if style == Style::Child || st.style == Style::Child {
                    warn("Window.style: \"child\" cannot be set or unset after creation; ignored");
                    return;
                }
                st.style = style;
                s.push(WindowRequest::Style(w, style));
            });
        }

        #[property]
        pub fn resizable(self) -> bool {
            read(self, |s| s.flags.resizable).unwrap_or(false)
        }

        pub fn set_resizable(self, v: bool) {
            write(self, |s, w| {
                s.windows.get_mut(&w).expect("open").flags.resizable = v;
                push_flags(s, w);
            });
        }

        #[property]
        pub fn minimizable(self) -> bool {
            read(self, |s| s.flags.minimizable).unwrap_or(false)
        }

        pub fn set_minimizable(self, v: bool) {
            write(self, |s, w| {
                s.windows.get_mut(&w).expect("open").flags.minimizable = v;
                push_flags(s, w);
            });
        }

        #[property]
        pub fn maximizable(self) -> bool {
            read(self, |s| s.flags.maximizable).unwrap_or(false)
        }

        pub fn set_maximizable(self, v: bool) {
            write(self, |s, w| {
                s.windows.get_mut(&w).expect("open").flags.maximizable = v;
                push_flags(s, w);
            });
        }

        /// Whether the title bar has a close button.
        #[property]
        pub fn closable(self) -> bool {
            read(self, |s| s.flags.closable).unwrap_or(false)
        }

        pub fn set_closable(self, v: bool) {
            write(self, |s, w| {
                s.windows.get_mut(&w).expect("open").flags.closable = v;
                push_flags(s, w);
            });
        }

        /// Above every normal window, not only its owner.
        #[property]
        pub fn always_on_top(self) -> bool {
            read(self, |s| s.flags.always_on_top).unwrap_or(false)
        }

        pub fn set_always_on_top(self, v: bool) {
            write(self, |s, w| {
                s.windows.get_mut(&w).expect("open").flags.always_on_top = v;
                push_flags(s, w);
            });
        }

        #[property]
        pub fn visible(self) -> bool {
            read(self, |s| s.flags.visible).unwrap_or(false)
        }

        pub fn set_visible(self, v: bool) {
            write(self, |s, w| {
                s.windows.get_mut(&w).expect("open").flags.visible = v;
                push_flags(s, w);
            });
        }

        // ---- Place ----

        /// Outer top-left, desktop UI pixels (physical / this window's scale).
        #[property]
        pub fn x(self) -> f64 {
            with(|s| effective_position(s, self).0)
        }

        pub fn set_x(self, v: f64) {
            if !v.is_nan() {
                set_pos(self, Some(v), None);
            }
        }

        #[property]
        pub fn y(self) -> f64 {
            with(|s| effective_position(s, self).1)
        }

        pub fn set_y(self, v: f64) {
            if !v.is_nan() {
                set_pos(self, None, Some(v));
            }
        }

        /// Client size, UI pixels. Setting asks the OS; the value changes when the OS answers
        /// (`onResized`), because the OS may clamp it. (A window that is not made yet reads it at once.)
        #[property]
        pub fn width(self) -> f64 {
            read(self, |s| s.size.0).unwrap_or(0.0)
        }

        pub fn set_width(self, v: f64) {
            if !v.is_nan() {
                set_dim(self, Some(v), None);
            }
        }

        #[property]
        pub fn height(self) -> f64 {
            read(self, |s| s.size.1).unwrap_or(0.0)
        }

        pub fn set_height(self, v: f64) {
            if !v.is_nan() {
                set_dim(self, None, Some(v));
            }
        }

        pub fn set_position(self, x: f64, y: f64) {
            if !x.is_nan() && !y.is_nan() {
                set_pos(self, Some(x), Some(y));
            }
        }

        pub fn set_size(self, width: f64, height: f64) {
            if !width.is_nan() && !height.is_nan() {
                set_dim(self, Some(width), Some(height));
            }
        }

        /// Centres it on its parent, or on the main window's monitor.
        pub fn center(self) {
            write(self, |s, w| s.push(WindowRequest::Center(w)));
        }

        /// Physical pixels per UI pixel (the monitor's DPI scale).
        #[property]
        pub fn scale(self) -> f64 {
            read(self, |s| s.scale).unwrap_or(1.0)
        }

        #[property]
        pub fn minimized(self) -> bool {
            read(self, |s| s.minimized).unwrap_or(false)
        }

        pub fn set_minimized(self, v: bool) {
            write(self, |s, w| {
                s.windows.get_mut(&w).expect("open").minimized = v;
                s.push(WindowRequest::Minimized(w, v));
            });
        }

        #[property]
        pub fn maximized(self) -> bool {
            read(self, |s| s.maximized).unwrap_or(false)
        }

        pub fn set_maximized(self, v: bool) {
            write(self, |s, w| {
                s.windows.get_mut(&w).expect("open").maximized = v;
                s.push(WindowRequest::Maximized(w, v));
            });
        }

        // ---- Focus and closing ----

        /// Whether this window has keyboard focus (`Window.focused` is the one that has it).
        #[property]
        #[js_name = "focused"]
        pub fn focused_now(self) -> bool {
            read(self, |s| s.focused).unwrap_or(false)
        }

        /// Brings it to the front with keyboard focus (the OS may only flash it if another program is
        /// active).
        pub fn focus(self) {
            write(self, |s, w| s.push(WindowRequest::Focus(w)));
        }

        /// Closes it (and its children first) without `onCloseRequested`. On the main window it quits
        /// the game.
        pub fn close(self) {
            if self == MAIN {
                if with(|s| s.may_change(self)) {
                    crate::app::application_api::quit();
                }
                return;
            }
            write(self, |s, w| {
                if !s
                    .requests
                    .iter()
                    .any(|r| matches!(r, WindowRequest::Close(c) if *c == w))
                {
                    s.push(WindowRequest::Close(w));
                }
            });
        }

        /// Called from an `onCloseRequested` listener: the window stays open. Ignored elsewhere.
        pub fn cancel_close(self) {
            with(|s| {
                if s.in_close_request == Some(self) {
                    s.cancel = true;
                }
            });
        }

        // ---- Events (each returns a token for `off`; 0 for a closed window) ----

        /// The user asked to close it (title bar button, Alt+F4, Cmd+W). Listeners may call
        /// `cancelClose()`.
        pub fn on_close_requested(self, f: Callback<()>) -> u32 {
            add_listener(self, f, |l| &mut l.close_requested)
        }

        /// It closed, by `close()`, the user, or its parent closing. Called once.
        pub fn on_closed(self, f: Callback<()>) -> u32 {
            add_listener(self, f, |l| &mut l.closed)
        }

        pub fn on_resized(self, f: Callback<(f64, f64)>) -> u32 {
            add_listener(self, f, |l| &mut l.resized)
        }

        pub fn on_moved(self, f: Callback<(f64, f64)>) -> u32 {
            add_listener(self, f, |l| &mut l.moved)
        }

        pub fn on_focus_changed(self, f: Callback<(bool,)>) -> u32 {
            add_listener(self, f, |l| &mut l.focus)
        }

        pub fn on_scale_changed(self, f: Callback<(f64,)>) -> u32 {
            add_listener(self, f, |l| &mut l.scale)
        }

        /// Removes any of the listeners above by token.
        pub fn off(self, token: f64) -> bool {
            if token.is_nan() || token < 1.0 {
                return false;
            }
            with(|s| {
                s.may_set_up(self)
                    && s.listeners
                        .get_mut(&self)
                        .is_some_and(|l| l.remove(token as u32))
            })
        }
    }
}

/// `x` / `y` setters: the other coordinate stays as it reads now.
fn set_pos(w: WindowRef, x: Option<f64>, y: Option<f64>) {
    write(w, |s, w| {
        let cur = effective_position(s, w);
        let p = (x.unwrap_or(cur.0), y.unwrap_or(cur.1));
        s.windows.get_mut(&w).expect("open").position = Some(p);
        s.push(WindowRequest::Position(w, p.0, p.1));
    });
}

/// Size setters: a request now; the stored size changes when the OS answers ([`resized`]), except for
/// a window the host has not made yet (nothing can clamp it).
fn set_dim(w: WindowRef, width: Option<f64>, height: Option<f64>) {
    write(w, |s, w| {
        let st = s.windows.get_mut(&w).expect("open");
        let size = (
            clamp_size(width.unwrap_or(st.size.0)),
            clamp_size(height.unwrap_or(st.size.1)),
        );
        if !st.created {
            st.size = size;
        }
        s.push(WindowRequest::Size(w, size.0, size.1));
    });
}
