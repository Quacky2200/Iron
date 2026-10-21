//! Gamepad glue: events for the game's `input` API (`__iron_input` in @iron/engine)
//! and gamepad UI navigation (d-pad / left stick move focus, South clicks).

use std::collections::HashMap;

use iron_core::Value;
use iron_platform::GamepadEvent;
use iron_ui::{Direction, Input};

/// A stick counts as pushed past this, and released below the lower value, so one
/// push moves focus once.
const STICK_PRESS: f32 = 0.6;
const STICK_RELEASE: f32 = 0.3;

/// Arguments for `__iron_input` for a gamepad event.
pub fn script_args(id: u32, event: &GamepadEvent) -> Vec<Value> {
    let id = Value::Number(id.into());
    match event {
        GamepadEvent::Connected { name } => {
            vec!["gamepadconnected".into(), id, name.as_str().into()]
        }
        GamepadEvent::Disconnected => vec!["gamepaddisconnected".into(), id],
        GamepadEvent::ButtonDown(b) => vec!["gamepadbutton".into(), id, (*b).into(), 1.0.into()],
        GamepadEvent::ButtonUp(b) => vec!["gamepadbutton".into(), id, (*b).into(), 0.0.into()],
        GamepadEvent::ButtonValue(b, v) => {
            vec![
                "gamepadbutton".into(),
                id,
                (*b).into(),
                f64::from(*v).into(),
            ]
        }
        GamepadEvent::Axis(a, v) => {
            vec!["gamepadaxis".into(), id, (*a).into(), f64::from(*v).into()]
        }
    }
}

/// Applies a `__iron_input` argument list to the native `input` state (and so to the listeners
/// compiled code registered). The scene must be current.
pub fn apply_native(args: &[Value]) {
    use iron_engine_api::input as i;
    let Some(kind) = args.first().and_then(Value::as_str) else {
        return;
    };
    let s = |n: usize| args.get(n).and_then(Value::as_str).unwrap_or("");
    let f = |n: usize| args.get(n).and_then(Value::as_f64).unwrap_or(0.0);
    match kind {
        "keydown" => i::key_down(s(1), matches!(args.get(3), Some(Value::Bool(true)))),
        "keyup" => i::key_up_event(s(1)),
        "mousemove" => i::mouse_move(f(1) as f32, f(2) as f32),
        "mousedelta" => i::mouse_delta(f(1), f(2)),
        "wheel" => i::wheel(f(1), f(2)),
        "blur" => i::blur(),
        "gamepadconnected" => i::gamepad_connected(f(1) as i64, s(2)),
        "gamepaddisconnected" => i::gamepad_disconnected(f(1) as i64),
        "gamepadbutton" => i::gamepad_button(f(1) as i64, s(2), f(3)),
        "gamepadaxis" => i::gamepad_axis(f(1) as i64, s(2), f(3)),
        _ => {}
    }
}

/// Turns gamepad events into UI navigation.
#[derive(Debug, Default)]
pub struct GamepadNav {
    /// Which way each pad's left stick is pushed on each axis (-1, 0, 1).
    stick: HashMap<(u32, &'static str), i8>,
}

impl GamepadNav {
    pub fn ui_input(&mut self, id: u32, event: &GamepadEvent) -> Option<Input> {
        match event {
            GamepadEvent::ButtonDown("DPadUp") => Some(Input::Navigate(Direction::Up)),
            GamepadEvent::ButtonDown("DPadDown") => Some(Input::Navigate(Direction::Down)),
            GamepadEvent::ButtonDown("DPadLeft") => Some(Input::Navigate(Direction::Left)),
            GamepadEvent::ButtonDown("DPadRight") => Some(Input::Navigate(Direction::Right)),
            GamepadEvent::ButtonDown("South") => Some(Input::Activate),
            GamepadEvent::Axis(axis @ ("LeftStickX" | "LeftStickY"), value) => {
                let state = self.stick.entry((id, axis)).or_insert(0);
                let pushed = if value.abs() >= STICK_PRESS {
                    value.signum() as i8
                } else if value.abs() < STICK_RELEASE {
                    0
                } else {
                    *state
                };
                let fresh = pushed != 0 && pushed != *state;
                *state = pushed;
                if !fresh {
                    return None;
                }
                // Stick y is positive up.
                Some(Input::Navigate(match (*axis, pushed) {
                    ("LeftStickX", 1) => Direction::Right,
                    ("LeftStickX", _) => Direction::Left,
                    (_, 1) => Direction::Up,
                    _ => Direction::Down,
                }))
            }
            GamepadEvent::Disconnected => {
                self.stick.retain(|(pad, _), _| *pad != id);
                None
            }
            _ => None,
        }
    }
}

// M3-PAD-01
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn d_pad_and_south_navigate() {
        let mut nav = GamepadNav::default();
        assert_eq!(
            nav.ui_input(0, &GamepadEvent::ButtonDown("DPadLeft")),
            Some(Input::Navigate(Direction::Left))
        );
        assert_eq!(
            nav.ui_input(0, &GamepadEvent::ButtonDown("South")),
            Some(Input::Activate)
        );
        assert_eq!(nav.ui_input(0, &GamepadEvent::ButtonUp("South")), None);
        assert_eq!(nav.ui_input(0, &GamepadEvent::ButtonDown("North")), None);
    }

    #[test]
    fn a_stick_push_navigates_once_until_released() {
        let mut nav = GamepadNav::default();
        let mut push = |v: f32| nav.ui_input(1, &GamepadEvent::Axis("LeftStickY", v));
        assert_eq!(push(0.4), None, "not far enough");
        assert_eq!(push(0.7), Some(Input::Navigate(Direction::Up)));
        assert_eq!(push(0.9), None, "held");
        assert_eq!(push(0.45), None, "still held (hysteresis)");
        assert_eq!(push(0.1), None, "released");
        assert_eq!(push(-0.8), Some(Input::Navigate(Direction::Down)));
    }

    #[test]
    fn script_args_name_the_event() {
        assert_eq!(
            script_args(3, &GamepadEvent::ButtonDown("South")),
            vec![
                Value::from("gamepadbutton"),
                Value::Number(3.0),
                Value::from("South"),
                Value::Number(1.0)
            ]
        );
        assert_eq!(
            script_args(3, &GamepadEvent::Connected { name: "Pad".into() })[0],
            Value::from("gamepadconnected")
        );
    }

    #[test]
    fn gamepad_events_reach_script_listeners() {
        use iron_script::{QuickJsRuntime, ScriptRuntime};
        let mut rt = QuickJsRuntime::new();
        rt.load_bundle("input.js", include_str!("../../../tests/fixtures/input.js"))
            .unwrap();
        for event in [
            GamepadEvent::Connected { name: "Pad".into() },
            GamepadEvent::ButtonDown("South"),
            GamepadEvent::Axis("LeftStickX", 0.5),
        ] {
            rt.call_global("__iron_input", &script_args(0, &event))
                .unwrap();
        }
        let keydown = ["keydown".into(), "KeyW".into(), "w".into(), false.into()];
        rt.call_global("__iron_input", &keydown).unwrap();
        assert_eq!(
            rt.eval("__input_log()").unwrap(),
            Value::from("connected Pad, down South, LeftStickX 0.5, key KeyW, held true")
        );
    }
    /// Sends `__iron_input` the way `App::send_game_input_in` does: with domain `d` entered.
    fn send_in(rt: &mut iron_script::QuickJsRuntime, d: u32, args: &[Value]) {
        use iron_script::ScriptRuntime;
        let _g = iron_engine_api::window::enter_domain(iron_engine_api::window::DomainId(d));
        rt.call_global("__iron_input", args).unwrap();
    }

    fn key(code: &str, down: bool) -> Vec<Value> {
        vec![
            if down { "keydown" } else { "keyup" }.into(),
            code.into(),
            code.into(),
            false.into(),
        ]
    }

    // M9-API-1500
    #[test]
    fn quickjs_input_reads_the_current_event_domain() {
        use iron_script::{QuickJsRuntime, ScriptRuntime};
        iron_engine_api::window::reset();
        let mut rt = QuickJsRuntime::new();
        rt.add_native(iron_engine_api::window::register_quickjs)
            .unwrap();
        rt.load_bundle(
            "input_domains.js",
            include_str!("../../../tests/fixtures/input_domains.js"),
        )
        .unwrap();
        // An isolated window (no parent) is the root of domain 2; main is domain 1.
        rt.eval("globalThis.insp = Window.create({ title: 'Inspector', key: 'insp' });")
            .unwrap();
        assert_eq!(rt.eval("insp.domain").unwrap(), Value::Number(2.0));
        // Main holds KeyA; the Inspector's domain holds KeyB.
        rt.call_global("__iron_input", &key("KeyA", true)).unwrap();
        send_in(&mut rt, 2, &key("KeyB", true));
        send_in(&mut rt, 2, &["mousemove".into(), 40.0.into(), 7.0.into()]);
        // Main-game code reads main's keys, whatever the other window holds.
        assert_eq!(
            rt.eval("__probe()").unwrap(),
            Value::from("A true, B false, A! false, x 0")
        );
        // The window's `run` callback (its domain current) reads its own.
        rt.eval("insp.run(() => { globalThis.inside = __probe(); });")
            .unwrap();
        iron_engine_api::window::run_queued(iron_engine_api::window::DomainId(2));
        assert_eq!(
            rt.eval("globalThis.inside").unwrap(),
            Value::from("A false, B true, A! false, x 40")
        );
        // Frames end per domain: main's tick makes its press "this frame", the Inspector's own does
        // the same for its keys only.
        rt.call_global("__iron_tick", &[0.016.into()]).unwrap();
        assert_eq!(
            rt.eval("__probe()").unwrap(),
            Value::from("A true, B false, A! true, x 0")
        );
        // A promise job carries no domain tag: it reads the main domain's input.
        rt.eval(
            "insp.run(() => { Promise.resolve().then(() => { globalThis.job = __probe(); }); });",
        )
        .unwrap();
        iron_engine_api::window::run_queued(iron_engine_api::window::DomainId(2));
        rt.eval("0").unwrap();
        assert_eq!(
            rt.eval("globalThis.job").unwrap(),
            Value::from("A true, B false, A! true, x 0")
        );
        // Focus loss clears only that domain's keys.
        send_in(&mut rt, 2, &["blur".into()]);
        rt.eval("insp.run(() => { globalThis.after = __probe(); });")
            .unwrap();
        iron_engine_api::window::run_queued(iron_engine_api::window::DomainId(2));
        assert_eq!(
            rt.eval("globalThis.after").unwrap(),
            Value::from("A false, B false, A! false, x 40")
        );
        assert_eq!(
            rt.eval("__probe()").unwrap(),
            Value::from("A true, B false, A! true, x 0")
        );
        // Listeners belong to the domain that registered them: main's saw only main's press.
        assert_eq!(rt.eval("__listened()").unwrap(), Value::from("KeyA"));
        iron_engine_api::window::reset();
    }
}
