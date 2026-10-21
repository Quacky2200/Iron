use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{DeviceEvent, DeviceId, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{CustomCursor, Window, WindowId};

use crate::fake::VirtualBackend;
use crate::gamepad::convert_gamepad;
use crate::input::{InputEvent, convert_window_event};
use crate::windows::{
    CloseOutcome, Os, RunnerBackend, WindowCommand, WindowHost, WindowKey, WindowTree,
    WinitWindows, close_windows, request_close, run_commands, show_windows,
};

#[derive(Debug, thiserror::Error)]
pub enum PlatformError {
    #[error("event loop error: {0}")]
    EventLoop(#[from] winit::error::EventLoopError),
    #[error("could not create window: {0}")]
    Window(#[from] winit::error::OsError),
}

#[derive(Clone, Debug)]
pub struct WindowConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
    /// The window and taskbar icon: RGBA pixels, width, height.
    pub icon: Option<(Vec<u8>, u32, u32)>,
    /// Windows other than the main one are not made by the OS: `Open` answers with an
    /// offscreen [`WindowHost::Virtual`] (screenshot runs).
    pub virtual_windows: bool,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: "Iron".into(),
            width: 1280,
            height: 720,
            icon: None,
            virtual_windows: false,
        }
    }
}

/// A custom cursor for the window (see [`PlatformApp::take_custom_cursor`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CustomCursorRequest {
    /// Identifies the image; the platform keeps each cursor it creates under it.
    pub key: String,
    /// The pixels, needed only the first time a key is used.
    pub image: Option<CursorRgba>,
}

/// Straight-alpha RGBA pixels for a cursor, with its hotspot in pixels.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CursorRgba {
    pub rgba: Vec<u8>,
    pub width: u16,
    pub height: u16,
    pub hotspot: (u16, u16),
}

/// What the engine implements to be driven by the platform.
pub trait PlatformApp {
    /// The window exists (called once; on mobile this is where the surface becomes valid).
    fn window_ready(&mut self, window: Arc<Window>);
    fn input(&mut self, event: InputEvent);
    /// Called once per displayed frame.
    fn frame(&mut self);
    /// Checked after each frame; returning true closes the window and ends `run`.
    fn should_exit(&self) -> bool {
        false
    }
    /// A custom cursor to show, checked after input and frames. Custom cursors need
    /// the event loop, so the platform creates them; keyword cursors the app sets on
    /// the window itself.
    fn take_custom_cursor(&mut self) -> Option<CustomCursorRequest> {
        None
    }
    /// The shortest time between frames (Unity's `targetFrameRate`): the loop waits
    /// out the rest. `None`: as often as presenting allows (vsync paces it, or nothing).
    fn frame_interval(&self) -> Option<Duration> {
        None
    }
    /// Whether the game is paused (Unity's `runInBackground` off and the window in the
    /// background): no frames are requested, the loop sleeps until events arrive and
    /// calls [`PlatformApp::idle`] now and then.
    fn paused(&self) -> bool {
        false
    }
    /// Called about every [`IDLE_POLL`] while paused, for work that can't wait for
    /// focus (watching files for hot reload).
    fn idle(&mut self) {}

    // ---- More than one window (docs/platform.md, "Multi-window"). Every method has a
    // ---- default, so a one-window app compiles and behaves as before.

    /// Window commands for the platform to run inside the event loop: asked after input
    /// events, after each frame and after `events_done`. Windows can only be made in the
    /// loop's callbacks, so scripts' requests wait here until then.
    fn take_window_commands(&mut self) -> Vec<WindowCommand> {
        Vec::new()
    }
    /// A window from [`WindowCommand::Open`] exists (hidden); its physical size is in the host.
    fn window_opened(&mut self, _key: WindowKey, _host: WindowHost) {}
    /// A window from [`WindowCommand::Open`] could not be made.
    fn window_open_failed(&mut self, _key: WindowKey, _error: String) {}
    /// An event of any window. The default sends the main window's to [`PlatformApp::input`]
    /// and drops the rest. Closing is not an event for secondary windows: see
    /// [`PlatformApp::close_requested`] (the main window's `CloseRequested` is still sent first,
    /// for apps written for one window).
    fn window_input(&mut self, key: WindowKey, event: InputEvent) {
        if key == WindowKey::MAIN {
            self.input(event);
        }
    }
    /// The user asked to close `key`; true lets it close. Closing the main window ends `run`.
    fn close_requested(&mut self, _key: WindowKey) -> bool {
        true
    }
    /// `key` is closing: drop its surface and every clone of its OS window now, before the
    /// platform drops its own. Called deepest-first for a window's children, and also for
    /// windows the OS destroyed itself.
    fn window_closed(&mut self, _key: WindowKey) {}
    /// Called once per loop iteration after all queued events, when focus has settled.
    fn events_done(&mut self) {}
    /// Windows whose first frame is drawn: the platform shows them (they are made hidden).
    fn take_windows_to_show(&mut self) -> Vec<WindowKey> {
        Vec::new()
    }
}

/// How often a paused app's [`PlatformApp::idle`] runs.
pub const IDLE_POLL: Duration = Duration::from_millis(250);

/// Owns gamepad input and, when not headless, runs the window event loop.
pub struct Platform {
    gamepads: Option<gilrs::Gilrs>,
}

impl Platform {
    /// A platform with gamepad support. Gamepads are optional: if `gilrs` can't start
    /// (e.g. no input subsystem on a CI machine) Iron carries on without them.
    pub fn new() -> Self {
        let gamepads = match gilrs::Gilrs::new() {
            Ok(g) => Some(g),
            Err(e) => {
                tracing::warn!("gamepads unavailable: {e}");
                None
            }
        };
        Self { gamepads }
    }

    /// A platform with no window and no devices, for tests and tools (M1-PLT-01).
    pub fn headless() -> Self {
        Self { gamepads: None }
    }

    pub fn has_gamepads(&self) -> bool {
        self.gamepads.is_some()
    }

    /// Drains pending gamepad events as [`InputEvent::Gamepad`]s.
    pub fn poll_gamepads(&mut self) -> Vec<InputEvent> {
        let Some(g) = &mut self.gamepads else {
            return Vec::new();
        };
        let mut events = Vec::new();
        while let Some(ev) = g.next_event() {
            let id: usize = ev.id.into();
            let name = || g.gamepad(ev.id).name().to_owned();
            if let Some(event) = convert_gamepad(id, &ev.event, name) {
                events.push(event);
            }
        }
        events
    }

    /// Opens a window and runs until it's closed. Must be called from the main thread.
    pub fn run<A: PlatformApp>(self, config: WindowConfig, app: A) -> Result<(), PlatformError> {
        let event_loop = EventLoop::new()?;
        let mut runner = Runner {
            platform: self,
            config,
            app,
            native: WinitWindows::default(),
            virtual_backend: VirtualBackend::new(Os::current(), 1.0),
            tree: WindowTree::default(),
            error: None,
            cursors: HashMap::new(),
            last_frame: None,
        };
        event_loop.run_app(&mut runner)?;
        match runner.error {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }
}

impl Default for Platform {
    fn default() -> Self {
        Self::new()
    }
}

struct Runner<A> {
    platform: Platform,
    config: WindowConfig,
    /// Declared before `native`, so the app (and its surfaces) drops before the OS windows.
    app: A,
    native: WinitWindows,
    /// Secondary windows of a screenshot run (`WindowConfig::virtual_windows`).
    virtual_backend: VirtualBackend,
    tree: WindowTree,
    error: Option<PlatformError>,
    /// Custom cursors created so far, by request key.
    cursors: HashMap<String, CustomCursor>,
    /// When the last frame started, for the frame rate cap.
    last_frame: Option<Instant>,
}

impl<A: PlatformApp> Runner<A> {
    fn main_window(&self) -> Option<&Arc<Window>> {
        self.native.windows.get(&WindowKey::MAIN)
    }

    /// Runs the app's window commands (making, changing, closing windows) and shows the
    /// windows whose first frame is drawn. `run_commands` is shared with the fake platform.
    fn pump_windows(&mut self, event_loop: &ActiveEventLoop) {
        let mut backend = RunnerBackend {
            event_loop,
            native: &mut self.native,
            virtual_backend: &mut self.virtual_backend,
            virtual_windows: self.config.virtual_windows,
        };
        run_commands(&mut self.app, &mut backend, &mut self.tree, Os::current());
        show_windows(&mut self.app, &mut backend);
    }

    /// The user asked to close `key`. True when the whole app should exit.
    fn close(&mut self, event_loop: &ActiveEventLoop, key: WindowKey) -> bool {
        let mut backend = RunnerBackend {
            event_loop,
            native: &mut self.native,
            virtual_backend: &mut self.virtual_backend,
            virtual_windows: self.config.virtual_windows,
        };
        let outcome = request_close(&mut self.app, &mut backend, &mut self.tree, key);
        outcome == CloseOutcome::Exit
    }

    /// The OS destroyed `key` itself: the app drops its side, then ours (nobody is asked).
    /// True when the whole app should exit (the main window went).
    fn close_destroyed(&mut self, event_loop: &ActiveEventLoop, key: WindowKey) -> bool {
        if key == WindowKey::MAIN {
            return true;
        }
        let mut backend = RunnerBackend {
            event_loop,
            native: &mut self.native,
            virtual_backend: &mut self.virtual_backend,
            virtual_windows: self.config.virtual_windows,
        };
        close_windows(&mut self.app, &mut backend, &mut self.tree, key);
        false
    }

    /// Shows the app's requested custom cursor, creating it the first time.
    fn update_cursor(&mut self, event_loop: &ActiveEventLoop) {
        let Some(request) = self.app.take_custom_cursor() else {
            return;
        };
        if self.main_window().is_none() {
            return;
        }
        if !self.cursors.contains_key(&request.key) {
            let Some(image) = request.image else {
                return;
            };
            let (x, y) = image.hotspot;
            match CustomCursor::from_rgba(image.rgba, image.width, image.height, x, y) {
                Ok(source) => {
                    let cursor = event_loop.create_custom_cursor(source);
                    self.cursors.insert(request.key.clone(), cursor);
                }
                Err(e) => {
                    tracing::warn!("cursor `{}` rejected: {e}", request.key);
                    return;
                }
            }
        }
        if let (Some(cursor), Some(window)) = (self.cursors.get(&request.key), self.main_window()) {
            window.set_cursor(cursor.clone());
        }
    }
}

impl<A: PlatformApp> ApplicationHandler for Runner<A> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.main_window().is_some() {
            return;
        }
        // Hidden until the first frame is drawn, so the window never flashes its blank
        // (white) background.
        let icon = self.config.icon.take().and_then(|(rgba, w, h)| {
            winit::window::Icon::from_rgba(rgba, w, h)
                .inspect_err(|e| tracing::warn!("window icon: {e}"))
                .ok()
        });
        let attrs = Window::default_attributes()
            .with_visible(false)
            .with_title(self.config.title.clone())
            .with_window_icon(icon)
            .with_inner_size(LogicalSize::new(self.config.width, self.config.height));
        match event_loop.create_window(attrs) {
            Ok(window) => {
                let window = Arc::new(window);
                self.native.register(WindowKey::MAIN, window.clone());
                self.tree.insert(WindowKey::MAIN, None);
                self.virtual_backend.set_scale(window.scale_factor());
                self.app.window_ready(window.clone());
                set_black_background(&window);
                // Hidden windows get no redraw events, so draw the first frame now and
                // only then show the window.
                self.app.frame();
                window.set_visible(true);
                self.pump_windows(event_loop);
            }
            Err(e) => {
                self.error = Some(e.into());
                event_loop.exit();
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        let Some(key) = self.native.key_of(id) else {
            return;
        };
        match event {
            WindowEvent::RedrawRequested => {
                // The other windows are drawn by the app after each main frame.
                if key == WindowKey::MAIN {
                    self.last_frame = Some(Instant::now());
                    self.app.frame();
                    self.update_cursor(event_loop);
                    self.pump_windows(event_loop);
                }
            }
            WindowEvent::CloseRequested => {
                if self.close(event_loop, key) {
                    event_loop.exit();
                }
            }
            // An owned window the OS destroyed with its owner: the same path as a close.
            WindowEvent::Destroyed => {
                if self.close_destroyed(event_loop, key) {
                    event_loop.exit();
                }
            }
            event => {
                let Some(input) = convert_window_event(&event) else {
                    return;
                };
                let resized = key == WindowKey::MAIN && matches!(input, InputEvent::Resized { .. });
                self.app.window_input(key, input);
                self.update_cursor(event_loop);
                self.pump_windows(event_loop);
                if resized {
                    // Draw at the new size right away: while the user drags the border,
                    // Windows runs its own loop and would show the window's blank (white)
                    // background in the new area until our next frame.
                    self.app.frame();
                }
            }
        }
    }

    fn device_event(&mut self, _: &ActiveEventLoop, _: DeviceId, event: DeviceEvent) {
        if let DeviceEvent::MouseMotion { delta: (dx, dy) } = event {
            self.app.input(InputEvent::MouseMotion { dx, dy });
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.app.events_done();
        self.pump_windows(event_loop);
        if self.app.should_exit() {
            event_loop.exit();
            return;
        }
        for event in self.platform.poll_gamepads() {
            self.app.input(event);
        }
        self.update_cursor(event_loop);
        // Paused: no redraws, just a slow wake-up for `idle` (events still wake us).
        if self.app.paused() {
            self.app.idle();
            event_loop.set_control_flow(ControlFlow::WaitUntil(Instant::now() + IDLE_POLL));
            return;
        }
        // A frame rate cap: sleep until the next frame is due (events still wake us).
        if let (Some(interval), Some(last)) = (self.app.frame_interval(), self.last_frame) {
            let due = last + interval;
            if Instant::now() < due {
                event_loop.set_control_flow(ControlFlow::WaitUntil(due));
                return;
            }
        }
        event_loop.set_control_flow(ControlFlow::Wait);
        if let Some(window) = self.main_window() {
            window.request_redraw();
        }
    }
}

/// Windows paints the area a resize exposes with the window class's background before
/// the next frame arrives; winit's class has none, which shows as white flashes. A
/// black brush makes those few frames black instead.
#[cfg(windows)]
pub(crate) fn set_black_background(window: &Window) {
    use windows_sys::Win32::Graphics::Gdi::{BLACK_BRUSH, GetStockObject};
    use windows_sys::Win32::UI::WindowsAndMessaging::{GCLP_HBRBACKGROUND, SetClassLongPtrW};
    use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
    let Ok(handle) = window.window_handle() else {
        return;
    };
    if let RawWindowHandle::Win32(h) = handle.as_raw() {
        // SAFETY: `hwnd` is our live window; the stock brush is never freed.
        unsafe {
            SetClassLongPtrW(
                h.hwnd.get(),
                GCLP_HBRBACKGROUND,
                GetStockObject(BLACK_BRUSH),
            );
        }
    }
}

#[cfg(not(windows))]
pub(crate) fn set_black_background(_window: &Window) {}

/// Where the cursor is in the window (physical pixels), asked of the system. During
/// a drag from another program the window gets no cursor moves, so this is how the
/// drop point is followed. `None` where the platform can't say.
#[cfg(windows)]
pub fn cursor_in_window(window: &Window) -> Option<(f64, f64)> {
    use windows_sys::Win32::Foundation::POINT;
    use windows_sys::Win32::Graphics::Gdi::ScreenToClient;
    use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;
    use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
    let handle = window.window_handle().ok()?;
    let RawWindowHandle::Win32(h) = handle.as_raw() else {
        return None;
    };
    let mut p = POINT { x: 0, y: 0 };
    // SAFETY: `p` is a valid out pointer and `hwnd` is our live window.
    unsafe {
        if GetCursorPos(&mut p) == 0 || ScreenToClient(h.hwnd.get() as _, &mut p) == 0 {
            return None;
        }
    }
    Some((f64::from(p.x), f64::from(p.y)))
}

#[cfg(not(windows))]
pub fn cursor_in_window(_window: &Window) -> Option<(f64, f64)> {
    None
}

// M1-PLT-01
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn headless_platform_creates_and_drops_without_a_window() {
        let mut platform = Platform::headless();
        assert!(!platform.has_gamepads());
        platform.poll_gamepads();
        drop(platform);
    }
}
