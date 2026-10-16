use winit::event::{ElementState, MouseScrollDelta, WindowEvent};
use winit::keyboard::{ModifiersState, PhysicalKey};

/// Iron's platform-independent input events.
#[derive(Clone, Debug, PartialEq)]
pub enum InputEvent {
    /// `key` is the physical key name, e.g. `"KeyW"` or `"Space"` (layout independent).
    KeyDown {
        key: String,
        repeat: bool,
    },
    KeyUp {
        key: String,
    },
    MouseMove {
        x: f64,
        y: f64,
    },
    MouseDown {
        button: MouseButton,
    },
    MouseUp {
        button: MouseButton,
    },
    /// The cursor left the window.
    MouseLeave,
    MouseWheel(WheelDelta),
    /// The held modifier keys changed.
    Modifiers(Modifiers),
    Resized {
        width: u32,
        height: u32,
    },
    Focused(bool),
    CloseRequested,
}

/// A scroll amount, in the DOM's direction: positive `y` scrolls down (content moves up).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WheelDelta {
    /// Notched wheels: lines (the app picks the line height).
    Lines { x: f32, y: f32 },
    /// Touchpads: physical pixels.
    Pixels { x: f64, y: f64 },
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Modifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    /// The Windows / Command key.
    pub meta: bool,
}

impl From<ModifiersState> for Modifiers {
    fn from(m: ModifiersState) -> Self {
        Self {
            shift: m.shift_key(),
            ctrl: m.control_key(),
            alt: m.alt_key(),
            meta: m.super_key(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Other(u16),
}

impl From<winit::event::MouseButton> for MouseButton {
    fn from(b: winit::event::MouseButton) -> Self {
        use winit::event::MouseButton as W;
        match b {
            W::Left => MouseButton::Left,
            W::Right => MouseButton::Right,
            W::Middle => MouseButton::Middle,
            W::Back => MouseButton::Other(3),
            W::Forward => MouseButton::Other(4),
            W::Other(n) => MouseButton::Other(n),
        }
    }
}

/// Converts a winit window event into an Iron event. Events Iron doesn't use yet return `None`.
pub fn convert_window_event(event: &WindowEvent) -> Option<InputEvent> {
    match event {
        WindowEvent::CloseRequested => Some(InputEvent::CloseRequested),
        WindowEvent::Resized(size) => Some(InputEvent::Resized {
            width: size.width,
            height: size.height,
        }),
        WindowEvent::Focused(f) => Some(InputEvent::Focused(*f)),
        WindowEvent::CursorMoved { position, .. } => Some(InputEvent::MouseMove {
            x: position.x,
            y: position.y,
        }),
        WindowEvent::MouseInput { state, button, .. } => Some(match state {
            ElementState::Pressed => InputEvent::MouseDown {
                button: (*button).into(),
            },
            ElementState::Released => InputEvent::MouseUp {
                button: (*button).into(),
            },
        }),
        WindowEvent::CursorLeft { .. } => Some(InputEvent::MouseLeave),
        // winit's deltas are positive when scrolling up/left; the DOM's are the opposite.
        WindowEvent::MouseWheel { delta, .. } => Some(InputEvent::MouseWheel(match delta {
            MouseScrollDelta::LineDelta(x, y) => WheelDelta::Lines { x: -x, y: -y },
            MouseScrollDelta::PixelDelta(p) => WheelDelta::Pixels { x: -p.x, y: -p.y },
        })),
        WindowEvent::ModifiersChanged(m) => Some(InputEvent::Modifiers(m.state().into())),
        WindowEvent::KeyboardInput { event, .. } => {
            convert_key(event.physical_key, event.state, event.repeat)
        }
        _ => None,
    }
}

/// Converts a physical key press/release. Split out from [`convert_window_event`] because
/// winit's `KeyEvent` can't be constructed outside winit, so this is the testable part.
pub fn convert_key(key: PhysicalKey, state: ElementState, repeat: bool) -> Option<InputEvent> {
    let PhysicalKey::Code(code) = key else {
        return None;
    };
    let key = format!("{code:?}");
    Some(match state {
        ElementState::Pressed => InputEvent::KeyDown { key, repeat },
        ElementState::Released => InputEvent::KeyUp { key },
    })
}

// M1-PLT-02
#[cfg(test)]
mod tests {
    use super::*;
    use winit::dpi::{PhysicalPosition, PhysicalSize};
    use winit::event::DeviceId;
    use winit::keyboard::{KeyCode, NativeKeyCode};

    #[test]
    fn converts_close_and_resize() {
        assert_eq!(
            convert_window_event(&WindowEvent::CloseRequested),
            Some(InputEvent::CloseRequested)
        );
        assert_eq!(
            convert_window_event(&WindowEvent::Resized(PhysicalSize::new(800, 600))),
            Some(InputEvent::Resized {
                width: 800,
                height: 600
            })
        );
    }

    #[test]
    fn converts_mouse_move_and_buttons() {
        let device_id = DeviceId::dummy();
        assert_eq!(
            convert_window_event(&WindowEvent::CursorMoved {
                device_id,
                position: PhysicalPosition::new(10.0, 20.5),
            }),
            Some(InputEvent::MouseMove { x: 10.0, y: 20.5 })
        );
        assert_eq!(
            convert_window_event(&WindowEvent::MouseInput {
                device_id,
                state: ElementState::Pressed,
                button: winit::event::MouseButton::Left,
            }),
            Some(InputEvent::MouseDown {
                button: MouseButton::Left
            })
        );
        assert_eq!(
            convert_window_event(&WindowEvent::MouseInput {
                device_id,
                state: ElementState::Released,
                button: winit::event::MouseButton::Right,
            }),
            Some(InputEvent::MouseUp {
                button: MouseButton::Right
            })
        );
    }

    #[test]
    fn converts_wheel_to_dom_direction_and_cursor_leave() {
        let device_id = DeviceId::dummy();
        assert_eq!(
            convert_window_event(&WindowEvent::MouseWheel {
                device_id,
                delta: MouseScrollDelta::LineDelta(0.0, 1.0),
                phase: winit::event::TouchPhase::Moved,
            }),
            Some(InputEvent::MouseWheel(WheelDelta::Lines {
                x: -0.0,
                y: -1.0
            }))
        );
        assert_eq!(
            convert_window_event(&WindowEvent::CursorLeft { device_id }),
            Some(InputEvent::MouseLeave)
        );
        let m = Modifiers::from(ModifiersState::SHIFT | ModifiersState::CONTROL);
        assert!(m.shift && m.ctrl && !m.alt && !m.meta);
    }

    #[test]
    fn converts_keys_by_physical_name() {
        assert_eq!(
            convert_key(
                PhysicalKey::Code(KeyCode::KeyW),
                ElementState::Pressed,
                false
            ),
            Some(InputEvent::KeyDown {
                key: "KeyW".into(),
                repeat: false
            })
        );
        assert_eq!(
            convert_key(
                PhysicalKey::Code(KeyCode::Space),
                ElementState::Released,
                false
            ),
            Some(InputEvent::KeyUp {
                key: "Space".into()
            })
        );
        assert_eq!(
            convert_key(
                PhysicalKey::Unidentified(NativeKeyCode::Unidentified),
                ElementState::Pressed,
                false
            ),
            None
        );
    }

    #[test]
    fn ignores_unhandled_events() {
        assert_eq!(convert_window_event(&WindowEvent::Destroyed), None);
    }
}
