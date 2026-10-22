//! A platform without a display, for tests (and the offscreen windows of screenshot runs).
//!
//! [`FakePlatform`] drives any [`PlatformApp`] through the same `run_commands` / `request_close`
//! path the winit runner uses, over a [`VirtualBackend`] that records windows instead of
//! making them. Nothing here touches winit's event loop, a GPU or the OS.

use std::collections::BTreeMap;

use crate::input::InputEvent;
use crate::platform::PlatformApp;
use crate::windows::{
    Backend, CloseOutcome, NativePlan, Os, WindowCommand, WindowHost, WindowKey, WindowSpec,
    WindowTree, plan, request_close, run_commands, show_windows,
};

/// A window the fake platform "made": what it was asked for and what has been done to it since.
#[derive(Clone, Debug)]
pub struct FakeWindow {
    /// The spec, updated by setters.
    pub spec: WindowSpec,
    /// What the spec means on the fake platform's OS (recomputed by `SetKind` / `SetFlags`).
    pub plan: NativePlan,
    /// Client size in physical pixels.
    pub physical: (u32, u32),
    /// Outer top-left, logical.
    pub position: (f64, f64),
    pub scale: f64,
    pub focused: bool,
    pub minimized: bool,
    pub maximized: bool,
    /// The platform showed it (`take_windows_to_show`).
    pub shown: bool,
}

/// Records windows instead of making them; setters update the record and answer with the events
/// the OS would send (`Resized`, `Moved`, `Focused`).
pub(crate) struct VirtualBackend {
    os: Os,
    scale: f64,
    pub(crate) windows: BTreeMap<WindowKey, FakeWindow>,
    events: Vec<(WindowKey, InputEvent)>,
}

fn physical(size: (f64, f64), scale: f64) -> (u32, u32) {
    (
        (size.0 * scale).round().max(0.0) as u32,
        (size.1 * scale).round().max(0.0) as u32,
    )
}

impl VirtualBackend {
    pub fn new(os: Os, scale: f64) -> Self {
        VirtualBackend {
            os,
            scale,
            windows: BTreeMap::new(),
            events: Vec::new(),
        }
    }

    /// The scale new windows get (the main window's).
    pub fn set_scale(&mut self, scale: f64) {
        self.scale = scale;
    }

    pub fn has(&self, key: WindowKey) -> bool {
        self.windows.contains_key(&key)
    }

    /// Moves focus to `key`: `Focused(false)` for the window that had it, `Focused(true)` here.
    pub fn focus_to(&mut self, key: WindowKey) {
        if !self.windows.contains_key(&key) {
            return;
        }
        let old = self
            .windows
            .iter()
            .find(|(k, w)| w.focused && **k != key)
            .map(|(k, _)| *k);
        if let Some(old) = old {
            self.windows.get_mut(&old).unwrap().focused = false;
            self.events.push((old, InputEvent::Focused(false)));
        }
        let w = self.windows.get_mut(&key).unwrap();
        if !w.focused {
            w.focused = true;
            self.events.push((key, InputEvent::Focused(true)));
        }
    }
}

impl Backend for VirtualBackend {
    fn open(
        &mut self,
        key: WindowKey,
        spec: &WindowSpec,
        plan: &NativePlan,
    ) -> Result<WindowHost, String> {
        let phys = physical(spec.size, self.scale);
        self.windows.insert(
            key,
            FakeWindow {
                spec: spec.clone(),
                plan: plan.clone(),
                physical: phys,
                position: spec.position.unwrap_or((0.0, 0.0)),
                scale: self.scale,
                focused: false,
                minimized: false,
                maximized: false,
                shown: false,
            },
        );
        Ok(WindowHost::Virtual {
            width: phys.0,
            height: phys.1,
            scale: self.scale,
        })
    }

    fn apply(&mut self, key: WindowKey, cmd: &WindowCommand) {
        let os = self.os;
        if let WindowCommand::Focus(_) = cmd {
            self.focus_to(key);
            return;
        }
        let Some(w) = self.windows.get_mut(&key) else {
            return;
        };
        match cmd {
            WindowCommand::SetTitle(_, t) => w.spec.title.clone_from(t),
            WindowCommand::SetSize(_, width, height) => {
                w.spec.size = (*width, *height);
                w.physical = physical(w.spec.size, w.scale);
                self.events.push((
                    key,
                    InputEvent::Resized {
                        width: w.physical.0,
                        height: w.physical.1,
                    },
                ));
            }
            WindowCommand::SetPosition(_, x, y) => {
                w.spec.position = Some((*x, *y));
                w.position = (*x, *y);
                self.events.push((
                    key,
                    InputEvent::Moved {
                        x: (*x * w.scale).round() as i32,
                        y: (*y * w.scale).round() as i32,
                    },
                ));
            }
            WindowCommand::SetKind(_, kind) => {
                w.spec.kind = *kind;
                w.plan = plan(&w.spec, os);
            }
            WindowCommand::SetFlags(_, flags) => {
                w.spec.resizable = flags.resizable;
                w.spec.minimizable = flags.minimizable;
                w.spec.maximizable = flags.maximizable;
                w.spec.closable = flags.closable;
                w.spec.always_on_top = flags.always_on_top;
                w.spec.visible = flags.visible;
                w.plan = plan(&w.spec, os);
            }
            WindowCommand::SetMinimized(_, m) => w.minimized = *m,
            WindowCommand::SetMaximized(_, m) => w.maximized = *m,
            // Centring needs the parent's frame; the fake keeps the position.
            WindowCommand::Center(_)
            | WindowCommand::Focus(_)
            | WindowCommand::Open(..)
            | WindowCommand::Close(_) => {}
        }
    }

    fn destroy(&mut self, key: WindowKey) {
        self.windows.remove(&key);
    }

    fn show(&mut self, key: WindowKey) {
        if let Some(w) = self.windows.get_mut(&key) {
            w.shown = true;
        }
    }

    fn take_events(&mut self) -> Vec<(WindowKey, InputEvent)> {
        std::mem::take(&mut self.events)
    }
}

/// Runs a [`PlatformApp`] without winit: the main window is a virtual one, and windows the app
/// opens are records ([`FakeWindow`]).
pub struct FakePlatform<A: PlatformApp> {
    app: A,
    backend: VirtualBackend,
    tree: WindowTree,
    os: Os,
    exited: bool,
}

impl<A: PlatformApp> FakePlatform<A> {
    /// Opens the main window (as a virtual one: the app gets `window_opened(MAIN, ..)`, not
    /// `window_ready`) with `scale` physical pixels per UI pixel.
    pub fn new(app: A, main: WindowSpec, scale: f64) -> Self {
        let mut p = FakePlatform {
            app,
            backend: VirtualBackend::new(Os::Windows, scale),
            tree: WindowTree::default(),
            os: Os::Windows,
            exited: false,
        };
        p.open_main(main);
        p
    }

    /// Like [`FakePlatform::new`], but the app is not told about the main window: it draws its
    /// own (an offscreen target for a windowless benchmark run). Its secondary windows are
    /// virtual, as in screenshot runs.
    pub fn with_app_main(app: A, main: WindowSpec, scale: f64) -> Self {
        let mut p = FakePlatform {
            app,
            backend: VirtualBackend::new(Os::current(), scale),
            tree: WindowTree::default(),
            os: Os::current(),
            exited: false,
        };
        p.register_main(&main);
        p
    }

    fn register_main(&mut self, main: &WindowSpec) -> WindowHost {
        let plan = plan(main, self.os);
        self.backend.os = self.os;
        let host = self
            .backend
            .open(WindowKey::MAIN, main, &plan)
            .expect("the virtual backend always opens");
        self.tree.insert(WindowKey::MAIN, None);
        host
    }

    fn open_main(&mut self, main: WindowSpec) {
        let host = self.register_main(&main);
        self.app.window_opened(WindowKey::MAIN, host);
    }

    /// Which OS's rules `plan` applies (default Windows). Call it before the app opens windows.
    pub fn os(mut self, os: Os) -> Self {
        self.os = os;
        self.backend.os = os;
        if let Some(w) = self.backend.windows.get_mut(&WindowKey::MAIN) {
            w.plan = plan(&w.spec, os);
        }
        self
    }

    fn pump(&mut self) {
        run_commands(&mut self.app, &mut self.backend, &mut self.tree, self.os);
    }

    /// One loop iteration: commands, `frame()`, commands, the windows to show, `events_done()`,
    /// then the commands that caused.
    pub fn step(&mut self) {
        if self.exited {
            return;
        }
        self.pump();
        self.app.frame();
        self.pump();
        show_windows(&mut self.app, &mut self.backend);
        self.app.events_done();
        self.pump();
        show_windows(&mut self.app, &mut self.backend);
        if self.app.should_exit() {
            self.exited = true;
        }
    }

    /// Sends an event of `key` to `window_input` (ignored for a window that is not open), then
    /// runs the commands it caused.
    pub fn send(&mut self, key: WindowKey, event: InputEvent) {
        if self.exited || !self.tree.contains(key) {
            return;
        }
        self.app.window_input(key, event);
        self.pump();
    }

    /// The user clicks the close button: `close_requested`, then the close. True when the
    /// window closed (the main window closing ends the run: [`FakePlatform::exited`]).
    pub fn user_close(&mut self, key: WindowKey) -> bool {
        match request_close(&mut self.app, &mut self.backend, &mut self.tree, key) {
            CloseOutcome::Kept => false,
            CloseOutcome::Closed => true,
            CloseOutcome::Exit => {
                self.exited = true;
                true
            }
        }
    }

    /// Moves focus to `key`: `Focused(false)` to the window that had it, `Focused(true)` to `key`.
    pub fn focus(&mut self, key: WindowKey) {
        self.backend.focus_to(key);
        self.pump();
    }

    pub fn window(&self, key: WindowKey) -> Option<&FakeWindow> {
        self.backend.windows.get(&key)
    }

    /// Open windows, in key order (the main window first).
    pub fn keys(&self) -> Vec<WindowKey> {
        self.backend.windows.keys().copied().collect()
    }

    pub fn tree(&self) -> &WindowTree {
        &self.tree
    }

    pub fn exited(&self) -> bool {
        self.exited
    }

    pub fn app(&mut self) -> &mut A {
        &mut self.app
    }
}
