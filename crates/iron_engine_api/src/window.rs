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
use iron_api::callback::{HandleArg, Listeners, emit};
use iron_api::{Callback, Handle, iron_api};
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

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
    /// Why the host could not open it (`openError`).
    pub open_error: Option<String>,
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

/// The most a message may weigh: UTF-8 bytes of its JSON.
pub const MAX_MESSAGE_BYTES: usize = 1 << 20;
/// How long a request waits for its reply when the caller gives no timeout.
pub const DEFAULT_REQUEST_TIMEOUT_MS: f64 = 10_000.0;

pub const MIN_SIZE: f64 = 1.0;
pub const MAX_SIZE: f64 = 16384.0;

#[derive(Default)]
struct WindowListeners {
    opened: Listeners<(f64, f64, f64)>,
    open_failed: Listeners<(String,)>,
    close_requested: Listeners<()>,
    closed: Listeners<()>,
    resized: Listeners<(f64, f64)>,
    moved: Listeners<(f64, f64)>,
    focus: Listeners<(bool,)>,
    scale: Listeners<(f64,)>,
    message: Listeners<(HandleArg<WindowRef>, String)>,
    request: Listeners<(HandleArg<WindowRef>, String, f64)>,
}

/// One message in a domain's queue: a message (`request` 0, `reply_to` 0), a request (`request`
/// is its id) or a reply to the request `reply_to` (`error` set when it failed).
struct Message {
    from: WindowRef,
    to: WindowRef,
    text: String,
    request: u64,
    reply_to: u64,
    error: String,
}

/// A request that has not been answered yet.
struct PendingRequest {
    to: WindowRef,
    deadline_ms: f64,
    /// The domain whose `onReplyJson` listeners get the answer.
    sender_domain: DomainId,
    /// A reply (or a failure) is on its way.
    replied: bool,
}

/// Listeners of a domain (`Window.onMessage` and friends).
#[derive(Default)]
struct DomainListeners {
    focus: Listeners<(bool, bool)>,
    message: Listeners<(HandleArg<WindowRef>, String)>,
    request: Listeners<(HandleArg<WindowRef>, String, f64)>,
    reply: Listeners<(f64, String, String)>,
}

impl DomainListeners {
    fn remove(&mut self, token: u32) -> bool {
        self.focus.remove(token)
            || self.message.remove(token)
            || self.request.remove(token)
            || self.reply.remove(token)
    }
}

impl WindowListeners {
    fn remove(&mut self, token: u32) -> bool {
        self.opened.remove(token)
            || self.open_failed.remove(token)
            || self.close_requested.remove(token)
            || self.closed.remove(token)
            || self.resized.remove(token)
            || self.moved.remove(token)
            || self.focus.remove(token)
            || self.scale.remove(token)
            || self.message.remove(token)
            || self.request.remove(token)
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
    domain_listeners: BTreeMap<DomainId, DomainListeners>,
    /// Messages waiting for each domain's next delivery, in posting order.
    queues: BTreeMap<DomainId, VecDeque<Message>>,
    pending: BTreeMap<u64, PendingRequest>,
    next_request: u64,
    /// The latest `now_ms` the host gave `deliver`; request deadlines count from it.
    now_ms: f64,
    /// Whether the platform can open secondary windows (the host says; true until told).
    supported: bool,
    /// Keyed windows a hot reload kept that the new code has not claimed yet (`create` with their key).
    unclaimed: BTreeSet<WindowRef>,
    /// What `reload_begin` took out, until the new code's first frame (`reload_end`) or a failed
    /// reload (`reload_abort`) puts it back.
    stash: Option<Stash>,
}

/// The old code's window state a reload set aside.
struct Stash {
    listeners: BTreeMap<WindowRef, WindowListeners>,
    domain_listeners: BTreeMap<DomainId, DomainListeners>,
    run_queue: BTreeMap<DomainId, Vec<Callback<()>>>,
    queues: BTreeMap<DomainId, VecDeque<Message>>,
    pending: BTreeMap<u64, PendingRequest>,
    requests: Vec<WindowRequest>,
    setup: BTreeMap<WindowRef, DomainId>,
    /// The windows that existed, so a failed reload can forget the ones its code made.
    windows: BTreeSet<WindowRef>,
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
        open_error: None,
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
            queues: BTreeMap::new(),
            pending: BTreeMap::new(),
            next_request: 1,
            now_ms: 0.0,
            supported: true,
            unclaimed: BTreeSet::new(),
            stash: None,
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
    let calls = with(|s| {
        let st = s.windows.get_mut(&w).filter(|st| st.open)?;
        st.created = true;
        st.position = Some(info.position);
        st.size = info.size;
        st.scale = info.scale;
        st.native_handle = info.native_handle;
        st.platform = info.platform;
        let args = (st.size.0, st.size.1, st.scale);
        Some((s.listeners.get(&w)?.opened.snapshot(), args))
    });
    if let Some((cbs, args)) = calls {
        emit_in(w, &cbs, args);
    }
}

/// The host says whether the platform can open secondary windows (`Window.isSupported`).
pub fn set_supported(supported: bool) {
    with(|s| s.supported = supported);
}

/// The host could not make the window: logs an error (once), stores it as `openError`, runs
/// `onOpenFailed`, then as [`closed`]. `Window.create` has long returned, so nothing throws.
pub fn open_failed(w: WindowRef, error: &str) {
    let calls = with(|s| {
        let st = s.windows.get_mut(&w).filter(|st| st.open)?;
        st.open_error = Some(error.to_string());
        Some(
            s.listeners
                .get(&w)
                .map(|l| l.open_failed.snapshot())
                .unwrap_or_default(),
        )
    });
    if calls.is_none() {
        return;
    }
    if error.starts_with("Cannot open Window") {
        log_emit(Level::Error, error);
    } else {
        log_emit(
            Level::Error,
            &format!("window {} could not be opened: {error}", w.0),
        );
    }
    if let Some(cbs) = calls {
        emit_in(w, &cbs, (error.to_string(),));
    }
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

thread_local! {
    /// Guards of the domains `runtime_enter` entered, innermost last.
    static ENTERED: RefCell<Vec<DomainGuard>> = const { RefCell::new(Vec::new()) };
}

/// For a script runtime's timers and microtasks (`ts_rt_set_domain_hooks`): the current domain.
pub extern "C" fn runtime_current_domain() -> u32 {
    current_domain().0
}

/// Enters domain `d` for a timer or microtask callback; undone by [`runtime_leave_domain`].
pub extern "C" fn runtime_enter_domain(d: u32) {
    let g = enter_domain(DomainId(d));
    ENTERED.with(|e| e.borrow_mut().push(g));
}

pub extern "C" fn runtime_leave_domain() {
    // Dropped outside the borrow: the guard's drop writes the window state, not `ENTERED`.
    let g = ENTERED.with(|e| e.borrow_mut().pop());
    drop(g);
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
            .map(|l| l.focus.snapshot())
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

/// Messages waiting for `d` (tests, the phase B channel).
pub fn pending(d: DomainId) -> usize {
    with(|s| s.queues.get(&d).map_or(0, |q| q.len()))
}

/// Delivers what is queued for `d` (call it with `d` current; it enters `d` itself otherwise): first
/// the `run` callbacks, then the messages and requests that were queued **before** the call (what
/// listeners post meanwhile waits for the next delivery) to the target's listeners and then the
/// domain's, then replies to `onReplyJson`, then the requests of `d` that expired (`"timed out"`) or
/// whose target closed (`"window closed"`). A request nobody listens to fails with
/// `"no request listener"`.
pub fn deliver(d: DomainId, now_ms: f64) {
    run_queued(d);
    let batch: Vec<Message> = with(|s| {
        if now_ms > s.now_ms {
            s.now_ms = now_ms;
        }
        s.queues
            .get_mut(&d)
            .map(|q| q.drain(..).collect())
            .unwrap_or_default()
    });
    let _g = enter_domain(d);
    let (replies, incoming): (Vec<Message>, Vec<Message>) =
        batch.into_iter().partition(|m| m.reply_to != 0);
    for m in incoming {
        deliver_one(d, m);
    }
    for m in replies {
        let cbs = with(|s| {
            s.pending.remove(&m.reply_to)?;
            Some(
                s.domain_listeners
                    .get(&d)
                    .map(|l| l.reply.snapshot())
                    .unwrap_or_default(),
            )
        });
        if let Some(cbs) = cbs {
            emit(&cbs, (m.reply_to as f64, m.text, m.error));
        }
    }
    // Requests of this domain that ran out of time or lost their target.
    let failed: Vec<(u64, &'static str)> = with(|s| {
        let now = s.now_ms;
        let mut out = Vec::new();
        for (id, p) in &s.pending {
            if p.sender_domain != d || p.replied {
                continue;
            }
            if p.deadline_ms <= now {
                out.push((*id, "timed out"));
            } else if !s.windows.get(&p.to).is_some_and(|st| st.open) {
                out.push((*id, "window closed"));
            }
        }
        for (id, _) in &out {
            s.pending.remove(id);
        }
        out
    });
    if !failed.is_empty() {
        let cbs = with(|s| {
            s.domain_listeners
                .get(&d)
                .map(|l| l.reply.snapshot())
                .unwrap_or_default()
        });
        for (id, err) in failed {
            emit(&cbs, (id as f64, String::new(), err.to_string()));
        }
    }
}

/// One message or request to its target window's listeners, then the domain's.
fn deliver_one(d: DomainId, m: Message) {
    let (open, win_msg, dom_msg, win_req, dom_req) = with(|s| {
        let open = s.windows.get(&m.to).is_some_and(|st| st.open);
        let wl = s.listeners.get(&m.to);
        let dl = s.domain_listeners.get(&d);
        (
            open,
            wl.map(|l| l.message.snapshot()).unwrap_or_default(),
            dl.map(|l| l.message.snapshot()).unwrap_or_default(),
            wl.map(|l| l.request.snapshot()).unwrap_or_default(),
            dl.map(|l| l.request.snapshot()).unwrap_or_default(),
        )
    });
    // A message to a window that closed on the way is dropped (a request fails in its sender's
    // domain when it sees the closed target).
    if !open {
        return;
    }
    let _c = enter_current(m.to);
    if m.request == 0 {
        for cb in win_msg.iter().chain(&dom_msg) {
            cb.call((HandleArg(m.from), m.text.clone()));
        }
        return;
    }
    if win_req.is_empty() && dom_req.is_empty() {
        reject(m.request, "no request listener");
        return;
    }
    for cb in win_req.iter().chain(&dom_req) {
        cb.call((HandleArg(m.from), m.text.clone(), m.request as f64));
        if with(|s| s.pending.get(&m.request).is_none_or(|p| p.replied)) {
            break;
        }
    }
}

/// Fails the request `id` in its sender's domain (a reply message with an error).
fn reject(id: u64, error: &str) {
    with(|s| {
        let Some(p) = s.pending.get_mut(&id) else {
            return;
        };
        if p.replied {
            return;
        }
        p.replied = true;
        let (to, d) = (p.to, p.sender_domain);
        s.queues.entry(d).or_default().push_back(Message {
            from: to,
            to: d.root(),
            text: String::new(),
            request: 0,
            reply_to: id,
            error: error.to_string(),
        });
    });
}

/// Whether the running domain may set `w` up: it is in `w`'s domain, or it made `w` this frame.
/// The app asks before it mounts a root for `createRoot(win)`.
pub fn can_set_up(w: WindowRef) -> bool {
    with(|s| s.setup.get(&w) == Some(&s.domain) || s.domain_of(w) == s.domain)
}

/// Hot reload, before the old code goes: drops every listener (they hold the old code's closures),
/// queued messages, pending requests and `run` callbacks, and returns the open windows without a key
/// (not the main one): the host closes them (children first, no `onCloseRequested`; `onClosed` does
/// not run, its listeners are gone). Windows with a key stay open and wait for the new code to claim
/// them with `Window.create({ key })`; [`reload_end`] closes what nobody claimed.
pub fn reload_begin() -> Vec<WindowRef> {
    with(|s| {
        s.stash = Some(Stash {
            listeners: std::mem::take(&mut s.listeners),
            domain_listeners: std::mem::take(&mut s.domain_listeners),
            run_queue: std::mem::take(&mut s.run_queue),
            queues: std::mem::take(&mut s.queues),
            pending: std::mem::take(&mut s.pending),
            requests: std::mem::take(&mut s.requests),
            setup: std::mem::take(&mut s.setup),
            windows: s.windows.keys().copied().collect(),
        });
        s.current = None;
        s.cancel = false;
        s.in_close_request = None;
        s.unclaimed = s
            .windows
            .iter()
            .filter(|(w, st)| **w != MAIN && st.open && !st.key.is_empty())
            .map(|(w, _)| *w)
            .collect();
        s.open_in_order()
            .into_iter()
            .filter(|w| *w != MAIN && s.windows[w].key.is_empty())
            .collect()
    })
}

/// Hot reload that failed after [`reload_begin`]: the old code keeps running, so its listeners,
/// queued messages and requests come back and what the failed code registered or created is
/// dropped. The windows the host was told to close stay open.
pub fn reload_abort() {
    with(|s| {
        let Some(old) = s.stash.take() else { return };
        s.listeners = old.listeners;
        s.domain_listeners = old.domain_listeners;
        s.run_queue = old.run_queue;
        s.queues = old.queues;
        s.pending = old.pending;
        s.requests = old.requests;
        s.setup = old.setup;
        s.windows.retain(|w, _| old.windows.contains(w));
        s.unclaimed.clear();
        s.current = None;
    });
}

/// Hot reload, after the new code's first frame: the keyed windows it did not claim, to close.
pub fn reload_end() -> Vec<WindowRef> {
    with(|s| {
        s.stash = None;
        std::mem::take(&mut s.unclaimed)
            .into_iter()
            .filter(|w| s.windows.get(w).is_some_and(|st| st.open))
            .collect()
    })
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

/// Queues a message (or, with a timeout, a request) for the domain of `to`; the request id, or 0 for
/// a plain message, on success.
fn post(to: WindowRef, text: &str, request_timeout: Option<f64>) -> Option<u64> {
    if text.len() > MAX_MESSAGE_BYTES {
        warn(&format!(
            "Window.postMessage: the message is over {MAX_MESSAGE_BYTES} bytes as JSON; refused"
        ));
        return None;
    }
    with(|s| {
        if !s.windows.get(&to).is_some_and(|st| st.open) {
            warn(&format!(
                "Window.postMessage: window {} is closed; the message is dropped",
                to.0
            ));
            return None;
        }
        let from = s.current.unwrap_or(s.domain.root());
        let d = s.domain_of(to);
        let mut request = 0;
        if let Some(timeout) = request_timeout {
            request = s.next_request;
            s.next_request += 1;
            let sender_domain = s.domain;
            let deadline_ms = s.now_ms + timeout;
            s.pending.insert(
                request,
                PendingRequest {
                    to,
                    deadline_ms,
                    sender_domain,
                    replied: false,
                },
            );
        }
        s.queues.entry(d).or_default().push_back(Message {
            from,
            to,
            text: text.to_string(),
            request,
            reply_to: 0,
            error: String::new(),
        });
        Some(request)
    })
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
    // After a hot reload, the key of a window that stayed open gives that window back.
    if !key.is_empty()
        && let Some(w) = with(|s| {
            s.unclaimed
                .iter()
                .copied()
                .find(|w| s.windows.get(w).is_some_and(|st| st.open && st.key == key))
        })
    {
        return claim(
            w,
            title,
            style_v,
            parent,
            &flags,
            run_in_background,
            style.is_empty(),
        );
    }
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
                open_error: None,
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

/// `create` with the key of a window a reload kept: that window, with the new code's title, style
/// and flags (its size and position stay: the player may have moved it). A different parent is
/// ignored with a warning. The creator may set it up (`createRoot`, listeners, `run`) this frame.
fn claim(
    w: WindowRef,
    title: &str,
    style: Style,
    parent: WindowRef,
    flags: &Flags,
    run_in_background: bool,
    style_not_given: bool,
) -> WindowRef {
    with(|s| {
        s.unclaimed.remove(&w);
        let st = s.windows.get_mut(&w).expect("claimed windows exist");
        let wanted_parent = Some(parent).filter(|p| *p != WindowRef::NONE);
        if wanted_parent.is_some() && wanted_parent != st.parent {
            warn(&format!(
                "Window.create: window {} was kept by the reload with another parent; the new parent is ignored",
                w.0
            ));
        }
        let mut changed = Vec::new();
        if st.title != title {
            st.title = title.to_string();
            changed.push(WindowRequest::Title(w, title.to_string()));
        }
        if !style_not_given
            && st.style != style
            && st.style != Style::Child
            && style != Style::Child
        {
            st.style = style;
            changed.push(WindowRequest::Style(w, style));
        }
        let mut flags = flags.clone();
        flags.focus_on_show &= st.style != Style::Popup;
        if st.flags != flags {
            st.flags = flags;
            changed.push(WindowRequest::Flags(w, st.flags.clone()));
        }
        st.run_in_background = run_in_background;
        for r in changed {
            s.push(r);
        }
        if s.domain_of(w) != s.domain {
            let creator = s.domain;
            s.setup.insert(w, creator);
        }
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

/// JavaScript over the `...Json` natives for QuickJS games: the typed-looking `postMessage`, `request`,
/// `onMessage`, `onRequest`, the named events (`emit`, `on`, `ask`, `answer`, `Window.emitAll`) and `Window.broadcast` / `Window.onMessage` / `Window.onRequest`. The wire
/// form is JSON text: `JSON.stringify` with a replacer that throws a `TypeError` for functions and engine
/// handle objects (`__iron_is_handle`), `JSON.parse` on the way in. Compiled code lowers the same calls
/// onto the same natives (`tools/iron_ts/src/lower/facade.ts`), with the types checked at compile time.
pub const QUICKJS_PRELUDE: &str = r#"
(function () {
  const W = globalThis.Window;
  const isHandle = globalThis.__iron_is_handle;
  const where = (key) => (key === '' ? 'the message' : '"' + key + '"');
  const replacer = function (key, v) {
    if (typeof v === 'function') {
      throw new TypeError('Window messages carry structured data only: ' + where(key) + ' is a function');
    }
    if (v !== null && typeof v === 'object' && isHandle(v)) {
      throw new TypeError('Window messages carry structured data only: ' + where(key) + ' is a ' +
        Object.prototype.toString.call(v).slice(8, -1) + ' handle; send a number such as win.handle and look it up on the other side');
    }
    return v;
  };
  const toJson = (data) => {
    const text = JSON.stringify(data, replacer);
    if (text === undefined) {
      throw new TypeError('Window messages carry structured data only: undefined cannot be sent');
    }
    return text;
  };
  const def = (target, name, value) =>
    Object.defineProperty(target, name, { value, configurable: true, writable: true });
  const requestListener = (f) => (from, text, id) =>
    f(from, JSON.parse(text), (answer) => { W.replyJson(id, toJson(answer)); });

  Object.defineProperty(W.prototype, 'openError', {
    get() { const t = this.openErrorText; return t === '' ? null : t; },
    configurable: true,
  });
  def(W.prototype, 'postMessage', function (data) { return this.postMessageJson(toJson(data)); });
  def(W.prototype, 'onMessage', function (f) {
    return this.onMessageJson((from, text) => f(from, JSON.parse(text)));
  });
  def(W.prototype, 'onRequest', function (f) { return this.onRequestJson(requestListener(f)); });
  def(W, 'onMessage', function (f) {
    return W.onMessageJson((from, text) => f(from, JSON.parse(text)));
  });
  def(W, 'onRequest', function (f) { return W.onRequestJson(requestListener(f)); });
  def(W, 'broadcast', function (data) { return W.broadcastJson(toJson(data)); });

  // Named events: sugar over the raw messages. The wire form is `{"$e":<name>,"d":<payload>}` (no `d`
  // without a payload), the same text the compiled code writes; a listener for a name ignores every
  // other text without parsing it.
  const eventHead = (name) => {
    if (typeof name !== 'string' || name === '') {
      throw new TypeError('Window event names must not be empty');
    }
    return '{"$e":' + JSON.stringify(name);
  };
  const eventText = (name, payload) => {
    const head = eventHead(name);
    return payload === undefined ? head + '}' : head + ',"d":' + toJson(payload) + '}';
  };
  // The payload of event `head` in `text`: null when it is another text, '' without a payload.
  const eventBody = (text, head) => {
    if (!text.startsWith(head)) return null;
    const rest = text.slice(head.length);
    if (rest === '}') return '';
    if (rest.startsWith(',"d":') && rest.endsWith('}')) return rest.slice(5, -1);
    return null;
  };
  const decode = (body) => (body === '' ? undefined : JSON.parse(body));
  def(W.prototype, 'emit', function (name, payload) { return this.postMessageJson(eventText(name, payload)); });
  def(W, 'emitAll', function (name, payload) { return W.broadcastJson(eventText(name, payload)); });
  def(W.prototype, 'on', function (name, f) {
    const head = eventHead(name);
    return this.onMessageJson((from, text) => {
      const body = eventBody(text, head);
      if (body !== null) f(decode(body), from);
    });
  });
  def(W.prototype, 'answer', function (name, f) {
    const head = eventHead(name);
    return this.onRequestJson((from, text, id) => {
      const body = eventBody(text, head);
      if (body === null) return;
      const answer = f(decode(body), from);
      W.replyJson(id, toJson(answer === undefined ? null : answer));
    });
  });

  // `request`: a promise settled by the domain's `onReplyJson` listener (one per domain).
  const pending = new Map();
  const listening = new Set();
  const listen = () => {
    const d = W.current.domain;
    if (listening.has(d)) return;
    listening.add(d);
    W.onReplyJson((id, text, error) => {
      const p = pending.get(id);
      if (p === undefined) return;
      pending.delete(id);
      if (error !== '') {
        p.reject(new Error(error));
        return;
      }
      try {
        p.resolve(JSON.parse(text));
      } catch (e) {
        p.reject(e);
      }
    });
  };
  const requestText = (win, text, timeoutMs) => {
    listen();
    return new Promise((resolve, reject) => {
      const id = win.postRequestJson(text, timeoutMs === undefined ? NaN : timeoutMs);
      if (id === 0) {
        reject(new Error('request refused: the window is closed or the message is over 1 MiB'));
      } else {
        pending.set(id, { resolve, reject });
      }
    });
  };
  def(W.prototype, 'request', function (data, timeoutMs) {
    return requestText(this, toJson(data), timeoutMs);
  });
  def(W.prototype, 'ask', function (name, payload, timeoutMs) {
    return requestText(this, eventText(name, payload), timeoutMs);
  });
})();
"#;

/// Installs the `Window` class and its JavaScript helpers in a QuickJS context (use this rather than
/// `window_api::register_quickjs` alone: that one has no `postMessage` / `request` / `onMessage`).
pub fn register_quickjs<'js>(ctx: &iron_api::rquickjs::Ctx<'js>) -> iron_api::rquickjs::Result<()> {
    window_api::register_quickjs(ctx)?;
    ctx.globals().set(
        "__iron_is_handle",
        iron_api::rquickjs::Function::new(ctx.clone(), |v: iron_api::rquickjs::Value<'_>| {
            iron_api::qjs::is_handle(&v)
        })?,
    )?;
    ctx.eval::<(), _>(QUICKJS_PRELUDE)?;
    // The timers prelude (`iron_script`) tags timers with the domain current when they were set.
    ctx.globals().set(
        "__iron_domain",
        iron_api::rquickjs::Function::new(ctx.clone(), || runtime_current_domain())?,
    )?;
    ctx.globals().set(
        "__iron_enter_domain",
        iron_api::rquickjs::Function::new(ctx.clone(), |d: u32| runtime_enter_domain(d))?,
    )?;
    ctx.globals().set(
        "__iron_leave_domain",
        iron_api::rquickjs::Function::new(ctx.clone(), || runtime_leave_domain())?,
    )?;
    Ok(())
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
                s.domain_listeners.entry(d).or_default().focus.add(f)
            })
        }

        /// Messages to any window of the current domain (after the per-window listeners of the target).
        /// The wire form is JSON text; the typed `onMessage<T>` wraps this.
        #[js_name = "onMessageJson"]
        pub fn on_message_json_all(f: Callback<(HandleArg<WindowRef>, String)>) -> u32 {
            with(|s| {
                let d = s.domain;
                s.domain_listeners.entry(d).or_default().message.add(f)
            })
        }

        /// Requests to any window of the current domain; `replyJson(id, text)` answers the sender's
        /// promise (once).
        #[js_name = "onRequestJson"]
        pub fn on_request_json_all(f: Callback<(HandleArg<WindowRef>, String, f64)>) -> u32 {
            with(|s| {
                let d = s.domain;
                s.domain_listeners.entry(d).or_default().request.add(f)
            })
        }

        /// The sender side of `request`: called with (request id, reply JSON, error text; one of the two
        /// empty) in the domain that sent it.
        pub fn on_reply_json(f: Callback<(f64, String, String)>) -> u32 {
            with(|s| {
                let d = s.domain;
                s.domain_listeners.entry(d).or_default().reply.add(f)
            })
        }

        /// Answers request `request` (once; later calls are false) with JSON text.
        pub fn reply_json(request: f64, text: &str) -> bool {
            if request.is_nan() || request < 1.0 {
                return false;
            }
            if text.len() > MAX_MESSAGE_BYTES {
                warn("Window.reply: the reply is over 1 MiB; refused");
                return false;
            }
            let id = request as u64;
            with(|s| {
                let Some(p) = s.pending.get_mut(&id) else {
                    return false;
                };
                if p.replied {
                    return false;
                }
                p.replied = true;
                let (to, d) = (p.to, p.sender_domain);
                s.queues.entry(d).or_default().push_back(Message {
                    from: to,
                    to: d.root(),
                    text: text.to_string(),
                    request: 0,
                    reply_to: id,
                    error: String::new(),
                });
                true
            })
        }

        /// Posts JSON text to the root window of every other family. Returns how many were queued.
        pub fn broadcast_json(text: &str) -> u32 {
            if text.len() > MAX_MESSAGE_BYTES {
                warn("Window.broadcast: the message is over 1 MiB; refused");
                return 0;
            }
            with(|s| {
                let from = s.current.unwrap_or(s.domain.root());
                let me = s.domain;
                let roots: Vec<DomainId> = s
                    .open_in_order()
                    .into_iter()
                    .filter(|w| s.windows[w].parent.is_none())
                    .map(|w| DomainId(w.0))
                    .filter(|d| *d != me)
                    .collect();
                for d in &roots {
                    s.queues.entry(*d).or_default().push_back(Message {
                        from,
                        to: d.root(),
                        text: text.to_string(),
                        request: 0,
                        reply_to: 0,
                        error: String::new(),
                    });
                }
                roots.len() as u32
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

        /// The host made the window: called with its client size (UI pixels) and scale. Register it
        /// right after `create`; it fires at the end of the frame.
        pub fn on_opened(self, f: Callback<(f64, f64, f64)>) -> u32 {
            add_listener(self, f, |l| &mut l.opened)
        }

        /// The window could not be opened (see `openError`): called once with the error text. The
        /// window is then closed (`isOpen` false). `Window.create` itself never throws.
        pub fn on_open_failed(self, f: Callback<(String,)>) -> u32 {
            add_listener(self, f, |l| &mut l.open_failed)
        }

        /// Why the window could not be opened, "" when it could (compiled and QuickJS code read it as
        /// `openError`: a string, or null when fine).
        #[property]
        pub fn open_error_text(self) -> String {
            read(self, |s| s.open_error.clone().unwrap_or_default()).unwrap_or_default()
        }

        /// Whether the platform can open secondary windows at all (false on iOS, Android and the
        /// web). Check it before `create` where the game must work there.
        #[property]
        pub fn is_supported() -> bool {
            with(|s| s.supported)
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

        /// A message posted to this window, as JSON text. Runs in this window's domain.
        pub fn on_message_json(self, f: Callback<(HandleArg<WindowRef>, String)>) -> u32 {
            add_listener(self, f, |l| &mut l.message)
        }

        /// A request posted to this window: `(from, JSON text, request id)`; answer with
        /// `Window.replyJson(id, text)`.
        pub fn on_request_json(self, f: Callback<(HandleArg<WindowRef>, String, f64)>) -> u32 {
            add_listener(self, f, |l| &mut l.request)
        }

        /// Queues JSON text for this window's domain; delivered at the start of that domain's next
        /// dispatch, in order per sender. False (and a warning) when the text is over 1 MiB or the
        /// window is closed.
        pub fn post_message_json(self, text: &str) -> bool {
            post(self, text, None).is_some()
        }

        /// Posts `text` as a request and returns its id (0: refused). The reply, a failure or the
        /// timeout (default 10000 ms) arrives through `Window.onReplyJson` in the sender's domain.
        pub fn post_request_json(self, text: &str, timeout_ms: f64) -> f64 {
            let timeout = if timeout_ms.is_finite() && timeout_ms > 0.0 {
                timeout_ms
            } else {
                DEFAULT_REQUEST_TIMEOUT_MS
            };
            post(self, text, Some(timeout)).map_or(0.0, |id| id as f64)
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
