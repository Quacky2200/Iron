//! The `Platform` layer: windowing, input and gamepads.
//!
//! Everything above this crate sees Iron's own [`InputEvent`]s, so `winit` + `gilrs`
//! can be swapped (e.g. for SDL3) without touching the engine (DESIGN.md §10.1).

mod input;
mod platform;

pub use input::{InputEvent, MouseButton, convert_key, convert_window_event};
pub use platform::{Platform, PlatformApp, PlatformError, WindowConfig};
pub use winit::window::Window;
