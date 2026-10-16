use std::sync::Arc;

use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};

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
            return;
        }
        if let Some(input) = convert_window_event(&event) {
            let close = input == InputEvent::CloseRequested;
            let resized = matches!(input, InputEvent::Resized { .. });
            self.app.input(input);
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
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
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
