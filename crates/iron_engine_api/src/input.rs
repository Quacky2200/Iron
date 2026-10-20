//! `input`: keyboard, mouse and gamepad state for gameplay code, as `js/engine/src/input.ts` has
//! it. Keys are named by physical code (`"Space"`, `"KeyV"`); mouse buttons are keys named
//! `MouseLeft`, `MouseRight`, `MouseMiddle` (then `Mouse3`...). The host reports events as they
//! arrive; at the start of each frame ([`tick`], which [`crate::time::advance`] calls) what arrived
//! since the last tick becomes "this frame", so `getKeyDown` is true for exactly one frame.
//!
//! Listeners: `input.onKeyDown(fn)` and friends return a token for `input.off(token)` (the
//! TypeScript's `input.on(type, fn)` returned an unsubscribe function).

use iron_api::callback::{Listeners, emit};
use iron_api::{Callback, iron_api};
use iron_math::V2;
use std::cell::RefCell;
use std::collections::HashSet;

/// One gamepad. Buttons and axes by name, in the order they were first reported.
#[derive(Default, Clone)]
struct Pad {
    id: i64,
    name: String,
    connected: bool,
    buttons: Vec<(String, f64)>,
    axes: Vec<(String, f64)>,
}

fn value_of(list: &[(String, f64)], name: &str) -> f64 {
    list.iter().find(|e| e.0 == name).map_or(0.0, |e| e.1)
}

fn set_value(list: &mut Vec<(String, f64)>, name: &str, v: f64) {
    match list.iter_mut().find(|e| e.0 == name) {
        Some(e) => e.1 = v,
        None => list.push((name.to_string(), v)),
    }
}

#[derive(Default)]
struct State {
    keys: HashSet<String>,
    pressed_pending: HashSet<String>,
    released_pending: HashSet<String>,
    pressed: HashSet<String>,
    released: HashSet<String>,
    mouse: V2,
    delta_pending: (f64, f64),
    wheel_pending: (f64, f64),
    delta: (f64, f64),
    wheel: (f64, f64),
    pads: Vec<Pad>,
    on_key_down: Listeners<(String, bool)>,
    on_key_up: Listeners<(String,)>,
    on_pad_connected: Listeners<(f64,)>,
    on_pad_disconnected: Listeners<(f64,)>,
    on_pad_button_down: Listeners<(f64, String)>,
    on_pad_button_up: Listeners<(f64, String)>,
    on_pad_axis: Listeners<(f64, String, f64)>,
}

impl State {
    fn pad(&mut self, id: i64, name: &str) -> &mut Pad {
        if let Some(i) = self.pads.iter().position(|p| p.id == id) {
            return &mut self.pads[i];
        }
        self.pads.push(Pad {
            id,
            name: name.to_string(),
            connected: true,
            ..Pad::default()
        });
        self.pads.last_mut().expect("just pushed")
    }

    fn connected(&self, id: i64) -> Option<&Pad> {
        self.pads.iter().find(|p| p.id == id && p.connected)
    }

    fn button_down_on_a_pad(&self, button: &str) -> bool {
        self.pads
            .iter()
            .filter(|p| p.connected)
            .any(|p| value_of(&p.buttons, button) > 0.5)
    }
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

/// A key went down (a repeat of a held key is not a new press).
pub fn key_down_event(code: &str) {
    key_down(code, false);
}

/// A key went down; `repeat` is the OS auto-repeat.
pub fn key_down(code: &str, repeat: bool) {
    let cbs = STATE.with(|s| {
        let mut s = s.borrow_mut();
        if !repeat && !s.keys.contains(code) {
            s.pressed_pending.insert(code.to_string());
        }
        s.keys.insert(code.to_string());
        s.on_key_down.snapshot()
    });
    emit(&cbs, (code.to_string(), repeat));
}

pub fn key_up_event(code: &str) {
    let cbs = STATE.with(|s| {
        let mut s = s.borrow_mut();
        if s.keys.remove(code) {
            s.released_pending.insert(code.to_string());
        }
        s.on_key_up.snapshot()
    });
    emit(&cbs, (code.to_string(),));
}

/// The name a mouse button has as a key: Unity's numbers 0 left, 1 right, 2 middle.
fn mouse_code(button: usize) -> String {
    match button {
        0 => "MouseLeft".into(),
        1 => "MouseRight".into(),
        2 => "MouseMiddle".into(),
        n => format!("Mouse{n}"),
    }
}

pub fn mouse_button_event(button: usize, down: bool) {
    if down {
        key_down(&mouse_code(button), false);
    } else {
        key_up_event(&mouse_code(button));
    }
}

pub fn mouse_move(x: f32, y: f32) {
    STATE.with(|s| {
        s.borrow_mut().mouse = V2 {
            x: f64::from(x),
            y: f64::from(y),
        }
    });
}

/// Raw mouse movement (it keeps coming while the cursor is locked).
pub fn mouse_delta(dx: f64, dy: f64) {
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        s.delta_pending.0 += dx;
        s.delta_pending.1 += dy;
    });
}

/// Wheel movement in lines.
pub fn wheel(dx: f64, dy: f64) {
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        s.wheel_pending.0 += dx;
        s.wheel_pending.1 += dy;
    });
}

/// The window lost focus: releases may never arrive.
pub fn blur() {
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        s.keys.clear();
        for p in &mut s.pads {
            for b in &mut p.buttons {
                b.1 = 0.0;
            }
        }
    });
}

pub fn gamepad_connected(id: i64, name: &str) {
    let cbs = STATE.with(|s| {
        let mut s = s.borrow_mut();
        s.pad(id, name).connected = true;
        s.on_pad_connected.snapshot()
    });
    emit(&cbs, (id as f64,));
}

pub fn gamepad_disconnected(id: i64) {
    let cbs = STATE.with(|s| {
        let mut s = s.borrow_mut();
        s.pad(id, "").connected = false;
        s.on_pad_disconnected.snapshot()
    });
    emit(&cbs, (id as f64,));
}

pub fn gamepad_button(id: i64, button: &str, value: f64) {
    let (was, down, up) = STATE.with(|s| {
        let mut s = s.borrow_mut();
        let pad = s.pad(id, "");
        let was = value_of(&pad.buttons, button) > 0.5;
        set_value(&mut pad.buttons, button, value);
        (
            was,
            s.on_pad_button_down.snapshot(),
            s.on_pad_button_up.snapshot(),
        )
    });
    let now = value > 0.5;
    if now && !was {
        emit(&down, (id as f64, button.to_string()));
    } else if !now && was {
        emit(&up, (id as f64, button.to_string()));
    }
}

pub fn gamepad_axis(id: i64, axis: &str, value: f64) {
    let cbs = STATE.with(|s| {
        let mut s = s.borrow_mut();
        set_value(&mut s.pad(id, "").axes, axis, value);
        s.on_pad_axis.snapshot()
    });
    emit(&cbs, (id as f64, axis.to_string(), value));
}

/// Subscribes a Rust function to key presses (the host, or a test); returns the token for [`off`].
pub fn on_key_down(f: impl Fn(&str, bool) + 'static) -> u32 {
    STATE.with(|s| {
        s.borrow_mut()
            .on_key_down
            .add(Callback::from_fn(move |(c, r): (String, bool)| f(&c, r)))
    })
}

/// Removes any listener by its token; false when it is unknown.
pub fn off(token: u32) -> bool {
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        s.on_key_down.remove(token)
            | s.on_key_up.remove(token)
            | s.on_pad_connected.remove(token)
            | s.on_pad_disconnected.remove(token)
            | s.on_pad_button_down.remove(token)
            | s.on_pad_button_up.remove(token)
            | s.on_pad_axis.remove(token)
    })
}

/// Starts a frame: what arrived since the last tick is what happened "this frame".
pub fn tick() {
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        s.pressed = std::mem::take(&mut s.pressed_pending);
        s.released = std::mem::take(&mut s.released_pending);
        s.delta = std::mem::take(&mut s.delta_pending);
        s.wheel = std::mem::take(&mut s.wheel_pending);
    });
}

/// Forgets everything (a test).
pub fn reset() {
    STATE.with(|s| *s.borrow_mut() = State::default());
}

/// Keys and pad parts of the virtual axes (Unity's defaults): negative keys, positive keys, the
/// stick axis.
fn axis_def(
    name: &str,
) -> Option<(
    &'static [&'static str],
    &'static [&'static str],
    &'static str,
)> {
    match name {
        "Horizontal" => Some((
            &["KeyA", "ArrowLeft"],
            &["KeyD", "ArrowRight"],
            "LeftStickX",
        )),
        "Vertical" => Some((&["KeyS", "ArrowDown"], &["KeyW", "ArrowUp"], "LeftStickY")),
        _ => None,
    }
}

/// Virtual buttons: keys and gamepad buttons.
fn button_def(name: &str) -> Option<(&'static [&'static str], &'static [&'static str])> {
    match name {
        "Jump" => Some((&["Space"], &["South"])),
        "Fire1" => Some((&["ControlLeft", "MouseLeft"], &["RightTrigger"])),
        "Fire2" => Some((&["AltLeft", "MouseRight"], &["LeftTrigger"])),
        "Submit" => Some((&["Enter", "Space"], &["South"])),
        "Cancel" => Some((&["Escape"], &["East"])),
        _ => None,
    }
}

/// Raw mouse movement per "Mouse X" / "Mouse Y" unit (Unity's default sensitivity).
const MOUSE_SENSITIVITY: f64 = 0.1;

#[iron_api(namespace = "input", quickjs, values(Vector2 = V2 [x: f64, y: f64]))]
pub mod input_api {
    use super::{MOUSE_SENSITIVITY, STATE, axis_def, button_def, mouse_code, value_of};
    use iron_api::Callback;
    use iron_math::V2;

    /// Calls `f(code, repeat)` when a key goes down; returns a token for `off`.
    pub fn on_key_down(f: Callback<(String, bool)>) -> u32 {
        STATE.with(|s| s.borrow_mut().on_key_down.add(f))
    }

    /// Calls `f(code)` when a key goes up; returns a token for `off`.
    pub fn on_key_up(f: Callback<(String,)>) -> u32 {
        STATE.with(|s| s.borrow_mut().on_key_up.add(f))
    }

    /// Calls `f(id)` when a gamepad connects.
    pub fn on_gamepad_connected(f: Callback<(f64,)>) -> u32 {
        STATE.with(|s| s.borrow_mut().on_pad_connected.add(f))
    }

    /// Calls `f(id)` when a gamepad disconnects.
    pub fn on_gamepad_disconnected(f: Callback<(f64,)>) -> u32 {
        STATE.with(|s| s.borrow_mut().on_pad_disconnected.add(f))
    }

    /// Calls `f(id, button)` when a gamepad button goes down.
    pub fn on_gamepad_button_down(f: Callback<(f64, String)>) -> u32 {
        STATE.with(|s| s.borrow_mut().on_pad_button_down.add(f))
    }

    /// Calls `f(id, button)` when a gamepad button goes up.
    pub fn on_gamepad_button_up(f: Callback<(f64, String)>) -> u32 {
        STATE.with(|s| s.borrow_mut().on_pad_button_up.add(f))
    }

    /// Calls `f(id, axis, value)` when a gamepad axis moves.
    pub fn on_gamepad_axis(f: Callback<(f64, String, f64)>) -> u32 {
        STATE.with(|s| s.borrow_mut().on_pad_axis.add(f))
    }

    /// Stops a listener given by one of the `on...` functions (the TypeScript's `on` returned an
    /// unsubscribe function).
    pub fn off(token: u32) -> bool {
        super::off(token)
    }

    /// Whether a key (by physical `code`, e.g. `"KeyW"`) is held.
    pub fn is_key_down(code: &str) -> bool {
        STATE.with(|s| s.borrow().keys.contains(code))
    }

    /// Unity's `GetKey`: held.
    pub fn get_key(code: &str) -> bool {
        STATE.with(|s| s.borrow().keys.contains(code))
    }

    /// Unity's `GetKeyDown`: pressed since the last tick (true for one frame).
    pub fn get_key_down(code: &str) -> bool {
        STATE.with(|s| s.borrow().pressed.contains(code))
    }

    /// Unity's `GetKeyUp`: released since the last tick.
    pub fn get_key_up(code: &str) -> bool {
        STATE.with(|s| s.borrow().released.contains(code))
    }

    /// Where the mouse is, in UI pixels from the top-left.
    #[property]
    pub fn mouse_position() -> V2 {
        STATE.with(|s| s.borrow().mouse)
    }

    /// How far the mouse moved this frame in raw device units, right and up positive. It keeps
    /// coming while the cursor is locked: use it for mouse look.
    #[property]
    pub fn mouse_delta() -> V2 {
        STATE.with(|s| {
            let d = s.borrow().delta;
            V2 {
                x: d.0,
                y: 0.0 - d.1,
            }
        })
    }

    /// Wheel movement this frame, in lines: y up (away from you) positive.
    #[property]
    pub fn mouse_scroll_delta() -> V2 {
        STATE.with(|s| {
            let w = s.borrow().wheel;
            V2 { x: w.0, y: w.1 }
        })
    }

    /// Whether mouse `button` (0 left, 1 right, 2 middle) is held.
    pub fn get_mouse_button(button: u32) -> bool {
        STATE.with(|s| s.borrow().keys.contains(&mouse_code(button as usize)))
    }

    /// Whether mouse `button` went down this frame.
    pub fn get_mouse_button_down(button: u32) -> bool {
        STATE.with(|s| s.borrow().pressed.contains(&mouse_code(button as usize)))
    }

    /// Whether mouse `button` came up this frame.
    pub fn get_mouse_button_up(button: u32) -> bool {
        STATE.with(|s| s.borrow().released.contains(&mouse_code(button as usize)))
    }

    /// Unity's `GetAxis` for `"Horizontal"` (A/D, arrows, left stick X), `"Vertical"` (S/W, arrows,
    /// left stick Y), `"Mouse X"`, `"Mouse Y"` and `"Mouse ScrollWheel"`: -1 to 1 for the keys
    /// (the stick wins when pushed further).
    pub fn get_axis(name: &str) -> f64 {
        STATE.with(|s| {
            let s = s.borrow();
            match name {
                "Mouse X" => return s.delta.0 * MOUSE_SENSITIVITY,
                "Mouse Y" => return -s.delta.1 * MOUSE_SENSITIVITY,
                "Mouse ScrollWheel" => return s.wheel.1 * MOUSE_SENSITIVITY,
                _ => {}
            }
            let Some((neg, pos, stick_axis)) = axis_def(name) else {
                return 0.0;
            };
            let held = |ks: &[&str]| ks.iter().any(|k| s.keys.contains(*k));
            let key_value = f64::from(u8::from(held(pos))) - f64::from(u8::from(held(neg)));
            let mut stick = 0.0f64;
            for p in s.pads.iter().filter(|p| p.connected) {
                let v = value_of(&p.axes, stick_axis);
                if v.abs() > stick.abs() {
                    stick = v;
                }
            }
            if stick.abs() > key_value.abs() {
                stick
            } else {
                key_value
            }
        })
    }

    /// Unity's `GetButton` for `"Jump"`, `"Fire1"`, `"Fire2"`, `"Submit"`, `"Cancel"`: keys or
    /// gamepad buttons.
    pub fn get_button(name: &str) -> bool {
        STATE.with(|s| {
            let s = s.borrow();
            button_def(name).is_some_and(|(keys, pad)| {
                keys.iter().any(|k| s.keys.contains(*k))
                    || pad.iter().any(|b| s.button_down_on_a_pad(b))
            })
        })
    }

    /// Pressed since the last tick (keys only for now).
    pub fn get_button_down(name: &str) -> bool {
        STATE.with(|s| {
            let s = s.borrow();
            button_def(name).is_some_and(|(keys, _)| keys.iter().any(|k| s.pressed.contains(*k)))
        })
    }

    /// How many gamepads are connected.
    pub fn gamepad_count() -> f64 {
        STATE.with(|s| s.borrow().pads.iter().filter(|p| p.connected).count() as f64)
    }

    /// The id of the i-th connected gamepad (-1 past the end).
    pub fn gamepad_id(index: f64) -> f64 {
        STATE.with(|s| {
            s.borrow()
                .pads
                .iter()
                .filter(|p| p.connected)
                .nth(index as usize)
                .map_or(-1.0, |p| p.id as f64)
        })
    }

    /// Whether a gamepad is connected.
    pub fn gamepad_connected(id: f64) -> bool {
        STATE.with(|s| s.borrow().connected(id as i64).is_some())
    }

    /// A connected gamepad's name ("" if none).
    pub fn gamepad_name(id: f64) -> String {
        STATE.with(|s| {
            s.borrow()
                .connected(id as i64)
                .map(|p| p.name.clone())
                .unwrap_or_default()
        })
    }

    /// A button's value on a connected gamepad, 0 to 1 (`South`, `East`, `West`, `North`,
    /// `LeftBumper`, `RightBumper`, `LeftTrigger`, `RightTrigger`, `Select`, `Start`, `Mode`,
    /// `LeftStick`, `RightStick`, `DPadUp/Down/Left/Right`); 0 when there is none.
    pub fn gamepad_button(id: f64, button: &str) -> f64 {
        STATE.with(|s| {
            s.borrow()
                .connected(id as i64)
                .map_or(0.0, |p| value_of(&p.buttons, button))
        })
    }

    /// An axis' value on a connected gamepad, -1 to 1, up and right positive (`LeftStickX/Y`,
    /// `RightStickX/Y`, ...); 0 when there is none.
    pub fn gamepad_axis(id: f64, axis: &str) -> f64 {
        STATE.with(|s| {
            s.borrow()
                .connected(id as i64)
                .map_or(0.0, |p| value_of(&p.axes, axis))
        })
    }

    /// Whether `button` is held on any connected gamepad.
    pub fn is_button_down(button: &str) -> bool {
        STATE.with(|s| s.borrow().button_down_on_a_pad(button))
    }
}
