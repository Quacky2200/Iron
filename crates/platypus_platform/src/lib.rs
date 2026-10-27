// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

//! The `Platform` layer: windowing, input and gamepads.
//!
//! Everything above this crate sees Platypus's own [`InputEvent`]s, so `winit` + `gilrs`
//! can be swapped (e.g. for SDL3) without touching the engine (DESIGN.md §10.1).

pub mod display;
pub mod fake;
mod gamepad;
mod input;
mod platform;
pub mod windows;

pub use display::{DisplaySettings, FullScreenMode, Resolution};
pub use fake::{FakePlatform, FakeWindow};
pub use gamepad::{GamepadEvent, axis_name, button_name, convert_gamepad};
pub use input::{
    ImeEvent, InputEvent, Modifiers, MouseButton, WheelDelta, convert_key, convert_window_event,
};
pub use platform::{
    CursorRgba, CustomCursorRequest, Platform, PlatformApp, PlatformError, WindowConfig,
    cursor_in_window,
};
pub use windows::{
    NativePlan, Os, WindowCommand, WindowHost, WindowKey, WindowKind, WindowSpec, WindowTree,
    attributes, native_handle, place, plan,
};
pub use winit::dpi;
pub use winit::window::{CursorGrabMode, CursorIcon, Icon, Theme, Window};
