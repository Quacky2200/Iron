//! Gamepads through gilrs, as [`InputEvent::Gamepad`] events with stable names for
//! buttons and axes (the Xbox-style layout gilrs maps every pad to).

use gilrs::{Axis, Button, EventType};

use crate::input::InputEvent;

/// What happened on one gamepad.
#[derive(Clone, Debug, PartialEq)]
pub enum GamepadEvent {
    Connected {
        name: String,
    },
    Disconnected,
    ButtonDown(&'static str),
    ButtonUp(&'static str),
    /// Analog buttons (triggers) report how far they're pressed, 0 to 1.
    ButtonValue(&'static str, f32),
    /// Sticks report -1 to 1; up and right are positive.
    Axis(&'static str, f32),
}

/// A button's name: `South` (A on Xbox), `East` (B), `West` (X), `North` (Y),
/// `LeftBumper`, `RightBumper`, `LeftTrigger`, `RightTrigger`, `Select`, `Start`,
/// `Mode`, `LeftStick`, `RightStick` (stick clicks) and `DPadUp/Down/Left/Right`.
pub fn button_name(button: Button) -> Option<&'static str> {
    Some(match button {
        Button::South => "South",
        Button::East => "East",
        Button::West => "West",
        Button::North => "North",
        Button::C => "C",
        Button::Z => "Z",
        Button::LeftTrigger => "LeftBumper",
        Button::RightTrigger => "RightBumper",
        Button::LeftTrigger2 => "LeftTrigger",
        Button::RightTrigger2 => "RightTrigger",
        Button::Select => "Select",
        Button::Start => "Start",
        Button::Mode => "Mode",
        Button::LeftThumb => "LeftStick",
        Button::RightThumb => "RightStick",
        Button::DPadUp => "DPadUp",
        Button::DPadDown => "DPadDown",
        Button::DPadLeft => "DPadLeft",
        Button::DPadRight => "DPadRight",
        Button::Unknown => return None,
    })
}

/// An axis's name: `LeftStickX/Y`, `RightStickX/Y`, `LeftZ`, `RightZ`, `DPadX/Y`.
pub fn axis_name(axis: Axis) -> Option<&'static str> {
    Some(match axis {
        Axis::LeftStickX => "LeftStickX",
        Axis::LeftStickY => "LeftStickY",
        Axis::RightStickX => "RightStickX",
        Axis::RightStickY => "RightStickY",
        Axis::LeftZ => "LeftZ",
        Axis::RightZ => "RightZ",
        Axis::DPadX => "DPadX",
        Axis::DPadY => "DPadY",
        Axis::Unknown => return None,
    })
}

/// Converts a gilrs event. `name` gives the pad's name for connections. Repeats,
/// dropped events and force-feedback notices are ignored.
pub fn convert_gamepad(
    id: usize,
    event: &EventType,
    name: impl FnOnce() -> String,
) -> Option<InputEvent> {
    let event = match event {
        EventType::Connected => GamepadEvent::Connected { name: name() },
        EventType::Disconnected => GamepadEvent::Disconnected,
        EventType::ButtonPressed(b, _) => GamepadEvent::ButtonDown(button_name(*b)?),
        EventType::ButtonReleased(b, _) => GamepadEvent::ButtonUp(button_name(*b)?),
        EventType::ButtonChanged(b, v, _) => GamepadEvent::ButtonValue(button_name(*b)?, *v),
        EventType::AxisChanged(a, v, _) => GamepadEvent::Axis(axis_name(*a)?, *v),
        // Repeats, dropped events, force-feedback notices (and future kinds).
        _ => return None,
    };
    Some(InputEvent::Gamepad {
        id: id as u32,
        event,
    })
}

/// Drops a `Connected` for a pad that is already announced (`known`), and forgets one
/// that disconnects, so scripts hear of each connection once.
pub fn announce_once(
    known: &mut std::collections::HashSet<usize>,
    event: InputEvent,
) -> Option<InputEvent> {
    if let InputEvent::Gamepad { id, event: e } = &event {
        let id = *id as usize;
        match e {
            GamepadEvent::Connected { .. } if !known.insert(id) => return None,
            GamepadEvent::Disconnected => {
                known.remove(&id);
            }
            _ => {}
        }
    }
    Some(event)
}

// M3-PAD-01 (conversion; gilrs events can't be built in tests, so names are checked
// here and `convert_gamepad` with the events that can)
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buttons_and_axes_have_stable_names() {
        assert_eq!(button_name(Button::South), Some("South"));
        assert_eq!(button_name(Button::LeftTrigger), Some("LeftBumper"));
        assert_eq!(button_name(Button::RightTrigger2), Some("RightTrigger"));
        assert_eq!(button_name(Button::DPadLeft), Some("DPadLeft"));
        assert_eq!(button_name(Button::Unknown), None);
        assert_eq!(axis_name(Axis::LeftStickY), Some("LeftStickY"));
        assert_eq!(axis_name(Axis::Unknown), None);
    }

    #[test]
    fn connections_convert_with_the_pad_name() {
        assert_eq!(
            convert_gamepad(2, &EventType::Connected, || "Xbox Controller".into()),
            Some(InputEvent::Gamepad {
                id: 2,
                event: GamepadEvent::Connected {
                    name: "Xbox Controller".into()
                }
            })
        );
        assert_eq!(
            convert_gamepad(2, &EventType::Disconnected, String::new),
            Some(InputEvent::Gamepad {
                id: 2,
                event: GamepadEvent::Disconnected
            })
        );
        assert_eq!(convert_gamepad(0, &EventType::Dropped, String::new), None);
    }

    // M3-PAD-02
    #[test]
    fn each_connection_is_announced_once() {
        let mut known = std::collections::HashSet::new();
        let connect = || convert_gamepad(1, &EventType::Connected, || "Pad".into()).unwrap();
        let gone = || convert_gamepad(1, &EventType::Disconnected, String::new).unwrap();
        // Announced at startup, then gilrs reports it again: dropped.
        known.insert(1);
        assert_eq!(announce_once(&mut known, connect()), None);
        assert!(announce_once(&mut known, gone()).is_some());
        assert!(
            announce_once(&mut known, connect()).is_some(),
            "plugged back in"
        );
    }
}
