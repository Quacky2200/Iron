//! The `Platform` layer: windowing, input and gamepads.
//!
//! Everything above this crate sees Iron's own [`InputEvent`]s, so `winit` + `gilrs`
//! can be swapped (e.g. for SDL3) without touching the engine (DESIGN.md §10.1).

pub mod display;
mod gamepad;
mod input;
mod platform;

pub use display::{DisplaySettings, FullScreenMode, Resolution};
pub use gamepad::{GamepadEvent, axis_name, button_name, convert_gamepad};
pub use input::{
    ImeEvent, InputEvent, Modifiers, MouseButton, WheelDelta, convert_key, convert_window_event,
};
pub use platform::{
    CursorRgba, CustomCursorRequest, Platform, PlatformApp, PlatformError, WindowConfig,
};
pub use winit::dpi;
pub use winit::window::{CursorGrabMode, CursorIcon, Icon, Theme, Window};
