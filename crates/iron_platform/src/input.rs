use winit::event::{ElementState, WindowEvent};
use winit::keyboard::PhysicalKey;

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
    Resized {
        width: u32,
        height: u32,
    },
    Focused(bool),
    CloseRequested,
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
