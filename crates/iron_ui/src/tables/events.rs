//! UI events as a script sees them: the fields (and where they come from in the event's number and
//! string lists), the event interfaces and the `on*` props.
//!
//! The slots index `UiEvent::script_args()` (`iron_ui::events`): `data` is `[x, y, button, buttons,
//! detail, deltaX, deltaY, modifiers, repeat, related, scrollLeft, scrollTop, scrollWidth,
//! scrollHeight, clientWidth, clientHeight]`, `strings` is `[key, code]` for keyboard events and
//! `[value]` for input events. `crates/iron_ui/tests/ui_tables.rs` checks the slots against it.

/// Where a field's value comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Init {
    /// The event type (`"click"`).
    Kind,
    /// The target (`{ id, type, value }`).
    Target,
    /// `data[i]`.
    Num(usize),
    /// `data[i] & mask != 0`.
    Flag(usize, u32),
    /// `data[i] == 1`.
    One(usize),
    /// `strings[i]`.
    Str(usize),
    /// A constant (TypeScript expression text).
    Const(&'static str),
}

#[derive(Clone, Copy, Debug)]
pub struct EventField {
    pub name: &'static str,
    /// The TypeScript type.
    pub ts: &'static str,
    pub init: Init,
}

const fn f(name: &'static str, ts: &'static str, init: Init) -> EventField {
    EventField { name, ts, init }
}

use Init::*;

/// Every field any event interface has, in the order the compiled `UiEvent` class declares them.
pub const EVENT_FIELDS: &[EventField] = &[
    f("type", "string", Kind),
    f(
        "target",
        "{ readonly id: number; readonly value: string; readonly type: string }",
        Target,
    ),
    f("clientX", "number", Num(0)),
    f("clientY", "number", Num(1)),
    f("x", "number", Num(0)),
    f("y", "number", Num(1)),
    f("button", "number", Num(2)),
    f("buttons", "number", Num(3)),
    f("detail", "number", Num(4)),
    f("deltaX", "number", Num(5)),
    f("deltaY", "number", Num(6)),
    f("shiftKey", "boolean", Flag(7, 1)),
    f("ctrlKey", "boolean", Flag(7, 2)),
    f("altKey", "boolean", Flag(7, 4)),
    f("metaKey", "boolean", Flag(7, 8)),
    f("repeat", "boolean", One(8)),
    f("scrollLeft", "number", Num(10)),
    f("scrollTop", "number", Num(11)),
    f("scrollWidth", "number", Num(12)),
    f("scrollHeight", "number", Num(13)),
    f("clientWidth", "number", Num(14)),
    f("clientHeight", "number", Num(15)),
    f("pointerId", "number", Const("1")),
    f("key", "string", Str(0)),
    f("code", "string", Str(1)),
    f("value", "string", Str(0)),
];

/// The methods every event has, with the `Ui` call the compiled class makes.
pub const EVENT_METHODS: &[(&str, &str)] = &[
    ("stopPropagation", "Ui.evStop()"),
    ("preventDefault", "Ui.evPrevent()"),
];

#[derive(Clone, Copy, Debug)]
pub struct EventInterface {
    pub name: &'static str,
    pub extends: Option<&'static str>,
    /// Names from [`EVENT_FIELDS`].
    pub fields: &'static [&'static str],
}

pub const EVENT_INTERFACES: &[EventInterface] = &[
    EventInterface {
        name: "IronEvent",
        extends: None,
        fields: &["type", "target"],
    },
    EventInterface {
        name: "IronPointerEvent",
        extends: Some("IronEvent"),
        fields: &[
            "clientX",
            "clientY",
            "x",
            "y",
            "button",
            "buttons",
            "detail",
            "shiftKey",
            "ctrlKey",
            "altKey",
            "metaKey",
            "pointerId",
        ],
    },
    EventInterface {
        name: "IronWheelEvent",
        extends: Some("IronPointerEvent"),
        fields: &["deltaX", "deltaY"],
    },
    EventInterface {
        name: "IronKeyboardEvent",
        extends: Some("IronEvent"),
        fields: &[
            "key", "code", "repeat", "shiftKey", "ctrlKey", "altKey", "metaKey",
        ],
    },
    EventInterface {
        name: "IronInputEvent",
        extends: Some("IronEvent"),
        fields: &["value"],
    },
    EventInterface {
        name: "IronFocusEvent",
        extends: Some("IronEvent"),
        fields: &[],
    },
    EventInterface {
        name: "IronScrollEvent",
        extends: Some("IronEvent"),
        fields: &[
            "scrollLeft",
            "scrollTop",
            "scrollWidth",
            "scrollHeight",
            "clientWidth",
            "clientHeight",
        ],
    },
];

/// An `on*` prop: the event type (`EventKind::name`) it listens to and the interface of its argument.
#[derive(Clone, Copy, Debug)]
pub struct EventProp {
    pub prop: &'static str,
    pub kind: &'static str,
    pub interface: &'static str,
}

const fn ep(prop: &'static str, kind: &'static str, interface: &'static str) -> EventProp {
    EventProp {
        prop,
        kind,
        interface,
    }
}

/// The event props. `onChange` is React's name for `input`; `onFocus` / `onBlur` bubble, so they
/// listen to `focusin` / `focusout`. A `...Capture` suffix (`onClickCapture`) listens in the capture
/// phase.
pub const EVENT_PROPS: &[EventProp] = &[
    ep("onClick", "click", "IronPointerEvent"),
    ep("onDoubleClick", "dblclick", "IronPointerEvent"),
    ep("onContextMenu", "contextmenu", "IronPointerEvent"),
    ep("onPointerDown", "pointerdown", "IronPointerEvent"),
    ep("onPointerUp", "pointerup", "IronPointerEvent"),
    ep("onPointerMove", "pointermove", "IronPointerEvent"),
    ep("onPointerEnter", "pointerenter", "IronPointerEvent"),
    ep("onPointerLeave", "pointerleave", "IronPointerEvent"),
    ep(
        "onGotPointerCapture",
        "gotpointercapture",
        "IronPointerEvent",
    ),
    ep(
        "onLostPointerCapture",
        "lostpointercapture",
        "IronPointerEvent",
    ),
    ep("onMouseDown", "pointerdown", "IronPointerEvent"),
    ep("onMouseUp", "pointerup", "IronPointerEvent"),
    ep("onWheel", "wheel", "IronWheelEvent"),
    ep("onKeyDown", "keydown", "IronKeyboardEvent"),
    ep("onKeyUp", "keyup", "IronKeyboardEvent"),
    ep("onChange", "input", "IronInputEvent"),
    ep("onInput", "input", "IronInputEvent"),
    ep("onFocus", "focusin", "IronFocusEvent"),
    ep("onBlur", "focusout", "IronFocusEvent"),
    ep("onScroll", "scroll", "IronScrollEvent"),
    ep("onCancel", "cancel", "IronEvent"),
    ep("onClose", "close", "IronEvent"),
];

/// The event type and capture flag of an `on*` prop name (`onClickCapture` listens to `click` while
/// capturing).
pub fn kind_of_prop(prop: &str) -> Option<(&'static str, bool)> {
    // Exact first: `onGotPointerCapture` ends in "Capture" without being a capture-phase prop.
    if let Some(p) = EVENT_PROPS.iter().find(|p| p.prop == prop) {
        return Some((p.kind, false));
    }
    let base = prop.strip_suffix("Capture")?;
    EVENT_PROPS
        .iter()
        .find(|p| p.prop == base)
        .map(|p| (p.kind, true))
}
