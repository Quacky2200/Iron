use std::collections::HashMap;
use std::sync::Arc;

use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{CustomCursor, Window, WindowId};

use crate::gamepad::convert_gamepad;
use crate::input::{InputEvent, convert_window_event};

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
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: "Iron".into(),
            width: 1280,
            height: 720,
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
}

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
            window: None,
            error: None,
            cursors: HashMap::new(),
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
    app: A,
    window: Option<Arc<Window>>,
    error: Option<PlatformError>,
    /// Custom cursors created so far, by request key.
    cursors: HashMap<String, CustomCursor>,
}

impl<A: PlatformApp> Runner<A> {
    /// Shows the app's requested custom cursor, creating it the first time.
    fn update_cursor(&mut self, event_loop: &ActiveEventLoop) {
        let (Some(request), Some(window)) = (self.app.take_custom_cursor(), &self.window) else {
            return;
        };
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
        if let Some(cursor) = self.cursors.get(&request.key) {
            window.set_cursor(cursor.clone());
        }
    }
}

impl<A: PlatformApp> ApplicationHandler for Runner<A> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        // Hidden until the first frame is drawn, so the window never flashes its blank
        // (white) background.
        let attrs = Window::default_attributes()
            .with_visible(false)
            .with_title(self.config.title.clone())
            .with_inner_size(LogicalSize::new(self.config.width, self.config.height));
        match event_loop.create_window(attrs) {
            Ok(window) => {
                let window = Arc::new(window);
                self.app.window_ready(window.clone());
                set_black_background(&window);
                // Hidden windows get no redraw events, so draw the first frame now and
                // only then show the window.
                self.app.frame();
                window.set_visible(true);
                self.window = Some(window);
            }
            Err(e) => {
                self.error = Some(e.into());
                event_loop.exit();
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        if let WindowEvent::RedrawRequested = event {
            self.app.frame();
            self.update_cursor(event_loop);
            return;
        }
        if let Some(input) = convert_window_event(&event) {
            let close = input == InputEvent::CloseRequested;
            let resized = matches!(input, InputEvent::Resized { .. });
            self.app.input(input);
            self.update_cursor(event_loop);
            if close {
                event_loop.exit();
            } else if resized {
                // Draw at the new size right away: while the user drags the border,
                // Windows runs its own loop and would show the window's blank (white)
                // background in the new area until our next frame.
                self.app.frame();
            }
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.app.should_exit() {
            event_loop.exit();
            return;
        }
        for event in self.platform.poll_gamepads() {
            self.app.input(event);
        }
        self.update_cursor(event_loop);
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

/// Windows paints the area a resize exposes with the window class's background before
/// the next frame arrives; winit's class has none, which shows as white flashes. A
/// black brush makes those few frames black instead.
#[cfg(windows)]
fn set_black_background(window: &Window) {
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
fn set_black_background(_window: &Window) {}

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
