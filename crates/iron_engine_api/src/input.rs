//! `input`: keys by code (`"Space"`, `"KeyV"`, `"Digit1"`) and mouse buttons, with Unity's
//! "this frame" rules: `getKeyDown` is true on the frame the key went down. The host reports
//! events as they arrive and calls [`end_frame`] after the scripts ran.

use glam::Vec2;
use iron_api::iron_api;
use std::cell::RefCell;
use std::collections::HashSet;

#[derive(Default)]
struct State {
    keys: HashSet<String>,
    pressed: HashSet<String>,
    released: HashSet<String>,
    buttons: [bool; 5],
    buttons_pressed: [bool; 5],
    buttons_released: [bool; 5],
    mouse: Vec2,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

/// A key went down (a repeat of a held key is not a new press).
pub fn key_down_event(code: &str) {
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        if s.keys.insert(code.to_string()) {
            s.pressed.insert(code.to_string());
        }
    });
}

pub fn key_up_event(code: &str) {
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        if s.keys.remove(code) {
            s.released.insert(code.to_string());
        }
    });
}

pub fn mouse_button_event(button: usize, down: bool) {
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        let Some(held) = s.buttons.get(button).copied() else {
            return;
        };
        if held != down {
            s.buttons[button] = down;
            if down {
                s.buttons_pressed[button] = true;
            } else {
                s.buttons_released[button] = true;
            }
        }
    });
}

pub fn mouse_move(x: f32, y: f32) {
    STATE.with(|s| s.borrow_mut().mouse = Vec2::new(x, y));
}

/// Ends the frame: the "went down / went up this frame" sets are cleared.
pub fn end_frame() {
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        s.pressed.clear();
        s.released.clear();
        s.buttons_pressed = [false; 5];
        s.buttons_released = [false; 5];
    });
}

/// Forgets everything (a test, or losing focus).
pub fn reset() {
    STATE.with(|s| *s.borrow_mut() = State::default());
}

#[iron_api(namespace = "input", quickjs, values(Vector2 = Vec2 [x: f32, y: f32]))]
pub mod input_api {
    use super::STATE;
    use glam::Vec2;

    /// Is the key held? `code` is a `KeyboardEvent.code` name.
    pub fn get_key(code: &str) -> bool {
        STATE.with(|s| s.borrow().keys.contains(code))
    }

    /// Did the key go down this frame?
    pub fn get_key_down(code: &str) -> bool {
        STATE.with(|s| s.borrow().pressed.contains(code))
    }

    /// Did the key go up this frame?
    pub fn get_key_up(code: &str) -> bool {
        STATE.with(|s| s.borrow().released.contains(code))
    }

    /// The pointer, in pixels.
    #[property]
    pub fn mouse_position() -> Vec2 {
        STATE.with(|s| s.borrow().mouse)
    }

    pub fn get_mouse_button(button: u32) -> bool {
        STATE.with(|s| {
            s.borrow()
                .buttons
                .get(button as usize)
                .copied()
                .unwrap_or(false)
        })
    }

    pub fn get_mouse_button_down(button: u32) -> bool {
        STATE.with(|s| {
            s.borrow()
                .buttons_pressed
                .get(button as usize)
                .copied()
                .unwrap_or(false)
        })
    }

    pub fn get_mouse_button_up(button: u32) -> bool {
        STATE.with(|s| {
            s.borrow()
                .buttons_released
                .get(button as usize)
                .copied()
                .unwrap_or(false)
        })
    }
}
