use winit::event::{ElementState, Ime, MouseScrollDelta, WindowEvent};
use winit::keyboard::{Key, ModifiersState, NamedKey, PhysicalKey};

/// Iron's platform-independent input events.
#[derive(Clone, Debug, PartialEq)]
pub enum InputEvent {
    /// Named like the DOM's `KeyboardEvent`: `code` is the physical key (`"KeyW"`, layout
    /// independent), `key` the layout's meaning (`"w"`, `"Enter"`, `" "`), and `text` the
    /// characters the press types, if any.
    KeyDown {
        code: String,
        key: String,
        text: Option<String>,
        repeat: bool,
    },
    KeyUp {
        code: String,
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
    /// Raw mouse movement in device units, unclamped by the window's edges (it keeps
    /// coming while the cursor is locked): for mouse look.
    MouseMotion {
        dx: f64,
        dy: f64,
    },
    MouseWheel(WheelDelta),
    /// The held modifier keys changed.
    Modifiers(Modifiers),
    /// Input method (IME) composition, while the app allows it.
    Ime(ImeEvent),
    /// A gamepad event; `id` stays the same while the pad is connected.
    Gamepad {
        id: u32,
        event: crate::gamepad::GamepadEvent,
    },
    Resized {
        width: u32,
        height: u32,
    },
    Focused(bool),
    /// The window became fully hidden (minimised, or covered on platforms that tell)
    /// or visible again.
    Occluded(bool),
    CloseRequested,
}

/// Input method events (Japanese, Chinese, Korean, ... input).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImeEvent {
    Enabled,
    /// The text being composed and the IME's caret / selection in it (byte offsets;
    /// `None` hides the caret). Empty text ends the pre-edit.
    Preedit(String, Option<(usize, usize)>),
    /// Finished text to insert.
    Commit(String),
    Disabled,
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
        WindowEvent::Occluded(o) => Some(InputEvent::Occluded(*o)),
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
        WindowEvent::Ime(ime) => Some(InputEvent::Ime(match ime {
            Ime::Enabled => ImeEvent::Enabled,
            Ime::Preedit(text, cursor) => ImeEvent::Preedit(text.clone(), *cursor),
            Ime::Commit(text) => ImeEvent::Commit(text.clone()),
            Ime::Disabled => ImeEvent::Disabled,
        })),
        WindowEvent::KeyboardInput { event, .. } => Some(convert_key(
            event.physical_key,
            &event.logical_key,
            event.text.as_deref(),
            event.state,
            event.repeat,
        )),
        _ => None,
    }
}

/// Converts a key press/release. Split out from [`convert_window_event`] because
/// winit's `KeyEvent` can't be constructed outside winit, so this is the testable part.
pub fn convert_key(
    physical: PhysicalKey,
    logical: &Key,
    text: Option<&str>,
    state: ElementState,
    repeat: bool,
) -> InputEvent {
    let code = match physical {
        PhysicalKey::Code(code) => format!("{code:?}"),
        PhysicalKey::Unidentified(_) => "Unidentified".to_owned(),
    };
    let key = dom_key(logical);
    match state {
        ElementState::Pressed => InputEvent::KeyDown {
            code,
            key,
            // Control characters (Enter, Tab, Backspace, Ctrl+letter) aren't typed text.
            text: text
                .filter(|t| !t.chars().any(char::is_control))
                .map(str::to_owned),
            repeat,
        },
        ElementState::Released => InputEvent::KeyUp { code, key },
    }
}

/// The DOM `KeyboardEvent.key` value. winit's named keys follow the DOM names except
/// for these few.
fn dom_key(key: &Key) -> String {
    match key {
        Key::Character(c) => c.to_string(),
        Key::Named(NamedKey::Space) => " ".to_owned(),
        Key::Named(NamedKey::Super) => "Meta".to_owned(),
        Key::Named(named) => format!("{named:?}"),
        Key::Dead(_) => "Dead".to_owned(),
        Key::Unidentified(_) => "Unidentified".to_owned(),
    }
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

    // M5-APP-05
    #[test]
    fn converts_focus_and_occlusion() {
        assert_eq!(
            convert_window_event(&WindowEvent::Focused(false)),
            Some(InputEvent::Focused(false))
        );
        assert_eq!(
            convert_window_event(&WindowEvent::Occluded(true)),
            Some(InputEvent::Occluded(true))
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
    fn converts_keys_with_code_key_and_text() {
        assert_eq!(
            convert_key(
                PhysicalKey::Code(KeyCode::KeyW),
                &Key::Character("w".into()),
                Some("w"),
                ElementState::Pressed,
                false
            ),
            InputEvent::KeyDown {
                code: "KeyW".into(),
                key: "w".into(),
                text: Some("w".into()),
                repeat: false
            }
        );
        assert_eq!(
            convert_key(
                PhysicalKey::Code(KeyCode::Space),
                &Key::Named(NamedKey::Space),
                Some(" "),
                ElementState::Released,
                false
            ),
            InputEvent::KeyUp {
                code: "Space".into(),
                key: " ".into()
            }
        );
        assert_eq!(
            convert_key(
                PhysicalKey::Code(KeyCode::Enter),
                &Key::Named(NamedKey::Enter),
                Some("\r"),
                ElementState::Pressed,
                true
            ),
            InputEvent::KeyDown {
                code: "Enter".into(),
                key: "Enter".into(),
                text: None,
                repeat: true
            },
            "Enter types a control character, which isn't text"
        );
        assert_eq!(
            convert_key(
                PhysicalKey::Unidentified(NativeKeyCode::Unidentified),
                &Key::Named(NamedKey::Super),
                None,
                ElementState::Pressed,
                false
            ),
            InputEvent::KeyDown {
                code: "Unidentified".into(),
                key: "Meta".into(),
                text: None,
                repeat: false
            }
        );
    }

    // M3-IME-01
    #[test]
    fn converts_ime_events() {
        assert_eq!(
            convert_window_event(&WindowEvent::Ime(Ime::Preedit("にほ".into(), Some((6, 6))))),
            Some(InputEvent::Ime(ImeEvent::Preedit(
                "にほ".into(),
                Some((6, 6))
            )))
        );
        assert_eq!(
            convert_window_event(&WindowEvent::Ime(Ime::Commit("日本".into()))),
            Some(InputEvent::Ime(ImeEvent::Commit("日本".into())))
        );
    }

    #[test]
    fn ignores_unhandled_events() {
        assert_eq!(convert_window_event(&WindowEvent::Destroyed), None);
    }
}
