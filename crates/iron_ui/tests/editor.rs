//! Text fields (M3-INP-01; the IME part is M3-IME-01): sizing, typing
//! and editing keys, selection by keyboard and pointer, controlled `value`,
//! `readOnly` / `disabled`, clipboard requests, drawing and keeping the caret in view.

mod common;

use common::{Dom, n, op, px};
use iron_core::{Commit, NodeId, Value};
use iron_ui::events::Modifiers;
use iron_ui::opcodes::SET_ATTRS;
use iron_ui::{Clipboard, DrawCmd, Input, Interaction, Rect, UiEvent, UiTree};

struct Field {
    tree: UiTree,
    ui: Interaction,
    id: NodeId,
    events: Vec<UiEvent>,
    prevent: Vec<&'static str>,
}

impl Field {
    /// A field of `kind` with inline `style` and attributes `attrs`, focused.
    fn new(kind: &str, style: &[(&str, Value)], attrs: &[(&str, Value)]) -> Self {
        let mut d = Dom::new();
        let id = d.add(0, kind, style);
        if !attrs.is_empty() {
            let mut a = vec![n(id)];
            for (k, v) in attrs {
                a.push((*k).into());
                a.push(v.clone());
            }
            d.ops.push(op(SET_ATTRS, &a));
        }
        let mut f = Self {
            tree: d.build(""),
            ui: Interaction::new(),
            id: NodeId(id),
            events: Vec::new(),
            prevent: Vec::new(),
        };
        f.input(Input::Focus(f.id));
        f
    }

    fn input(&mut self, input: Input) {
        let (events, prevent) = (&mut self.events, &self.prevent);
        self.ui.handle(&mut self.tree, input, &mut |e| {
            events.push(e.clone());
            prevent.contains(&e.kind.name())
        });
        self.tree.layout_if_needed();
    }

    fn key(&mut self, key: &str) {
        self.input(Input::KeyDown {
            key: key.into(),
            code: String::new(),
            repeat: false,
            text: None,
        });
    }

    fn type_text(&mut self, text: &str) {
        for c in text.chars() {
            self.input(Input::KeyDown {
                key: c.to_string(),
                code: String::new(),
                repeat: false,
                text: Some(c.to_string()),
            });
        }
    }

    fn mods(&mut self, ctrl: bool, shift: bool) {
        self.input(Input::Modifiers(Modifiers {
            ctrl,
            shift,
            ..Modifiers::default()
        }));
    }

    fn chord(&mut self, key: &str) {
        self.mods(true, false);
        self.key(key);
        self.mods(false, false);
    }

    fn value(&self) -> String {
        self.tree.editor_value(self.id).unwrap()
    }

    fn set_attr(&mut self, name: &str, value: Value) {
        let errors = self.tree.apply_commit(&Commit {
            ops: vec![op(SET_ATTRS, &[n(self.id.0), name.into(), value])],
        });
        assert!(errors.is_empty(), "{errors:?}");
        self.tree.layout_if_needed();
    }

    /// Values sent with `input` events, in order.
    fn input_values(&mut self) -> Vec<String> {
        std::mem::take(&mut self.events)
            .into_iter()
            .filter(|e| e.kind.name() == "input")
            .map(|e| e.value.unwrap())
            .collect()
    }

    fn click(&mut self, x: f32, y: f32, t: f64) {
        self.input(Input::PointerMove { x, y });
        self.input(Input::PointerDown {
            button: 0,
            time_ms: t,
        });
        self.input(Input::PointerUp { button: 0 });
    }
}

#[test]
fn fields_size_from_the_font_unless_sized() {
    // (Not stretched by the root's flex row.)
    let top = || ("alignSelf", Value::from("flex-start"));
    let f = Field::new("input", &[top()], &[]);
    let r = f.tree.rect(f.id).unwrap();
    assert_eq!(r.width, 160.0, "about 20 characters wide by default");
    assert!((18.0..=22.0).contains(&r.height), "one line: {}", r.height);

    let f = Field::new("textarea", &[top()], &[("rows", px(4.0))]);
    let r = f.tree.rect(f.id).unwrap();
    assert_eq!(r.width, 240.0);
    assert!(
        (72.0..=88.0).contains(&r.height),
        "four lines: {}",
        r.height
    );

    let f = Field::new("input", &[("width", px(80.0)), ("padding", px(4.0))], &[]);
    assert_eq!(f.tree.rect(f.id).unwrap().width, 80.0);
}

// M3-INP-01
#[test]
fn typing_and_editing_keys_change_the_text_and_send_input_events() {
    let mut f = Field::new("input", &[], &[]);
    f.type_text("hello world");
    assert_eq!(f.value(), "hello world");
    let values = f.input_values();
    assert_eq!(values.len(), 11, "one input event per typed character");
    assert_eq!(values.last().unwrap(), "hello world");

    f.key("Backspace");
    assert_eq!(f.value(), "hello worl");
    f.mods(true, false);
    f.key("Backspace");
    f.mods(false, false);
    assert_eq!(f.value(), "hello ", "Ctrl+Backspace deletes a word");

    f.key("Home");
    f.key("Delete");
    assert_eq!(f.value(), "ello ");
    f.mods(false, true);
    f.key("End");
    f.mods(false, false);
    f.type_text("!");
    assert_eq!(f.value(), "!", "typing replaces the selection");

    f.chord("a");
    f.type_text("x");
    assert_eq!(f.value(), "x", "Ctrl+A selects everything");

    f.input_values();
    f.key("Enter");
    assert_eq!(f.value(), "x", "Enter doesn't add lines to an input");
    assert!(f.input_values().is_empty());
}

#[test]
fn textareas_take_enter_and_move_between_lines() {
    let mut f = Field::new("textarea", &[], &[]);
    f.type_text("ab");
    f.key("Enter");
    f.type_text("cd");
    assert_eq!(f.value(), "ab\ncd");
    f.key("ArrowUp");
    f.type_text("X");
    assert_eq!(f.value(), "abX\ncd");
}

#[test]
fn a_prevented_keydown_doesnt_edit() {
    let mut f = Field::new("input", &[], &[]);
    f.prevent.push("keydown");
    f.type_text("abc");
    assert_eq!(f.value(), "");
}

// M3-INP-01
#[test]
fn value_controls_the_text_and_default_value_only_starts_it() {
    let mut f = Field::new("input", &[], &[("defaultValue", "start".into())]);
    assert_eq!(f.value(), "start");
    f.type_text("!");
    assert_eq!(f.value(), "start!", "the caret starts at the end");
    f.set_attr("defaultValue", "ignored".into());
    assert_eq!(f.value(), "start!", "defaultValue only fills a new field");

    f.set_attr("value", "set by script".into());
    assert_eq!(f.value(), "set by script");
    f.set_attr("value", "set by script".into());
    f.type_text("?");
    assert_eq!(
        f.value(),
        "set by script?",
        "the same value keeps the caret"
    );
}

#[test]
fn read_only_fields_select_but_dont_edit_and_disabled_ones_are_inert() {
    let mut f = Field::new(
        "input",
        &[],
        &[("value", "fixed".into()), ("readOnly", true.into())],
    );
    f.type_text("x");
    f.key("Backspace");
    assert_eq!(f.value(), "fixed");
    f.chord("a");
    f.chord("c");
    assert_eq!(f.ui.take_clipboard(), Some(Clipboard::Copy("fixed".into())));

    let mut f = Field::new("input", &[], &[("disabled", true.into())]);
    assert_eq!(f.ui.focused(), None, "disabled fields can't take focus");
    f.type_text("x");
    assert_eq!(f.value(), "");
}

#[test]
fn fields_are_in_the_tab_order_by_default() {
    let mut d = Dom::new();
    let a = d.add(0, "input", &[]);
    let b = d.add(0, "textarea", &[]);
    let mut tree = d.build("");
    let mut ui = Interaction::new();
    let tab = Input::KeyDown {
        key: "Tab".into(),
        code: "Tab".into(),
        repeat: false,
        text: None,
    };
    ui.handle(&mut tree, tab.clone(), &mut |_| false);
    assert_eq!(ui.focused(), Some(NodeId(a)));
    ui.handle(&mut tree, tab, &mut |_| false);
    assert_eq!(
        ui.focused(),
        Some(NodeId(b)),
        "Tab isn't typed into the field"
    );
    assert_eq!(tree.editor_value(NodeId(a)).unwrap(), "");
}

// M3-INP-01
#[test]
fn pointer_presses_place_the_caret_and_select_words_and_drags() {
    let mut f = Field::new(
        "input",
        &[("width", px(300.0))],
        &[("value", "one two three".into())],
    );
    let r = f.tree.rect(f.id).unwrap();
    let y = r.y + r.height / 2.0;

    // Far right of the text: the caret goes to the end.
    f.click(290.0, y, 0.0);
    f.type_text("!");
    assert_eq!(f.value(), "one two three!");

    // Double-click on "two" selects it.
    f.set_attr("value", "one two three".into());
    let two_x = 45.0;
    f.click(two_x, y, 1000.0);
    f.click(two_x, y, 1100.0);
    f.type_text("2");
    assert_eq!(f.value(), "one 2 three");

    // Drag from the start across "one".
    f.set_attr("value", "one two three".into());
    f.input(Input::PointerMove { x: 1.0, y });
    f.input(Input::PointerDown {
        button: 0,
        time_ms: 5000.0,
    });
    f.input(Input::PointerMove { x: 30.0, y });
    f.input(Input::PointerUp { button: 0 });
    f.chord("c");
    assert_eq!(f.ui.take_clipboard(), Some(Clipboard::Copy("one".into())));
}

// M3-INP-01
#[test]
fn cut_copy_and_paste_go_through_the_app() {
    let mut f = Field::new("input", &[], &[("value", "abc".into())]);
    f.chord("a");
    f.chord("x");
    assert_eq!(f.ui.take_clipboard(), Some(Clipboard::Copy("abc".into())));
    assert_eq!(f.value(), "", "cut deletes");
    f.chord("v");
    assert_eq!(f.ui.take_clipboard(), Some(Clipboard::Paste));
    f.input_values();
    f.input(Input::Paste("line one\nline two".into()));
    assert_eq!(
        f.value(),
        "line one line two",
        "inputs flatten pasted lines"
    );
    assert_eq!(f.input_values(), ["line one line two"]);
}

// M3-INP-02
#[test]
fn shortcuts_follow_the_physical_key_on_non_latin_layouts() {
    let mut f = Field::new("input", &[], &[("value", "abc".into())]);
    let key_down = |key: &str, code: &str| Input::KeyDown {
        key: key.into(),
        code: code.into(),
        repeat: false,
        text: Some(key.into()),
    };
    f.mods(true, false);
    f.input(key_down("ф", "KeyA"));
    f.input(key_down("с", "KeyC"));
    f.mods(false, false);
    assert_eq!(
        f.ui.take_clipboard(),
        Some(Clipboard::Copy("abc".into())),
        "Ctrl+ф selects all and Ctrl+с copies"
    );
    f.type_text("x");
    assert_eq!(f.value(), "x", "the selection covered everything");

    // AltGr (Ctrl+Alt) types instead (Polish ą is AltGr+A).
    f.input(Input::Modifiers(Modifiers {
        ctrl: true,
        alt: true,
        ..Modifiers::default()
    }));
    f.input(key_down("ą", "KeyA"));
    f.mods(false, false);
    assert_eq!(f.value(), "xą");
}

fn draw(f: &mut Field) -> Vec<DrawCmd> {
    f.tree.draw_list()
}

#[test]
fn focused_fields_draw_text_selection_and_a_blinking_caret() {
    let mut f = Field::new("input", &[("padding", px(4.0))], &[("value", "hi".into())]);
    let quads = |cmds: &[DrawCmd]| {
        cmds.iter()
            .filter_map(|c| match c {
                DrawCmd::Quad(q) => Some(q.rect),
                _ => None,
            })
            .collect::<Vec<Rect>>()
    };
    let cmds = draw(&mut f);
    assert!(cmds.iter().any(|c| matches!(c, DrawCmd::Text(_))));
    assert!(
        matches!(cmds.first(), Some(DrawCmd::PushClip(_))),
        "clipped to the padding box"
    );
    let caret = *quads(&cmds).last().expect("a caret");
    assert!(caret.width <= 2.0 && caret.height > 10.0, "{caret:?}");

    f.tree.set_time(600.0);
    assert!(quads(&draw(&mut f)).is_empty(), "the caret blinks off");
    f.tree.set_time(1100.0);
    assert_eq!(quads(&draw(&mut f)).len(), 1, "and on again");

    f.chord("a");
    let selected = quads(&draw(&mut f));
    assert_eq!(selected.len(), 2, "selection and caret");
    assert!(selected[0].width > 5.0);

    f.input(Input::Blur(f.id));
    assert!(
        quads(&draw(&mut f)).is_empty(),
        "no caret or selection without focus"
    );
}

#[test]
fn empty_fields_show_the_placeholder_dimmed() {
    let mut f = Field::new(
        "input",
        &[("color", "#ffffff".into())],
        &[("placeholder", "Name".into())],
    );
    let text = |cmds: Vec<DrawCmd>| {
        cmds.into_iter().find_map(|c| match c {
            DrawCmd::Text(t) => Some(t),
            _ => None,
        })
    };
    let placeholder = text(draw(&mut f)).expect("placeholder text");
    assert_eq!(placeholder.runs[0].glyphs.len(), 4);
    assert_eq!(placeholder.runs[0].color.a, 0.5);
    f.type_text("x");
    let typed = text(draw(&mut f)).unwrap();
    assert_eq!(typed.runs[0].glyphs.len(), 1);
    assert_eq!(typed.runs[0].color.a, 1.0);
}

/// The last quad drawn: the caret.
fn last_quad(f: &mut Field) -> Rect {
    draw(f)
        .into_iter()
        .rev()
        .find_map(|c| match c {
            DrawCmd::Quad(q) => Some(q.rect),
            _ => None,
        })
        .unwrap()
}

#[test]
fn long_text_scrolls_to_keep_the_caret_visible() {
    let mut f = Field::new("input", &[("width", px(60.0))], &[]);
    f.type_text("a fairly long line of text");
    let r = f.tree.rect(f.id).unwrap();
    let caret = last_quad(&mut f);
    assert!(
        caret.x >= r.x && caret.x + caret.width <= r.x + r.width + 0.01,
        "caret {caret:?} inside {r:?}"
    );
    f.key("Home");
    let caret = last_quad(&mut f);
    assert!((caret.x - r.x).abs() < 1.0, "back at the start: {caret:?}");
}

#[test]
fn fields_show_a_text_cursor() {
    let f = Field::new("input", &[], &[]);
    assert_eq!(f.tree.cursor(Some(f.id)), "text");
}

// M3-IME-01
#[test]
fn ime_preedit_shows_underlined_and_commit_inserts() {
    let mut f = Field::new("input", &[("width", px(200.0))], &[("value", "ab".into())]);
    let area = f.ui.ime_area(&f.tree).expect("an editable field wants IME");
    let r = f.tree.rect(f.id).unwrap();
    assert!(
        area.x >= r.x && area.x + area.width <= r.x + r.width && area.height > 10.0,
        "on the caret's line, inside the field: {area:?}"
    );

    f.input_values();
    f.input(Input::ImePreedit {
        text: "ka".into(),
        cursor: Some((2, 2)),
    });
    assert_eq!(f.value(), "ab", "pre-edit text isn't part of the value yet");
    assert!(f.input_values().is_empty(), "and sends no input events");
    let underlined = draw(&mut f).into_iter().any(|c| match c {
        DrawCmd::Text(t) => !t.decorations.is_empty(),
        _ => false,
    });
    assert!(underlined, "the pre-edit text is underlined");
    let moved = f.ui.ime_area(&f.tree).unwrap();
    assert!(moved.x >= area.x, "the candidate area follows the pre-edit");

    f.input(Input::ImePreedit {
        text: String::new(),
        cursor: None,
    });
    f.input(Input::ImeCommit("か".into()));
    assert_eq!(f.value(), "abか");
    assert_eq!(f.input_values(), ["abか"]);
}

// M3-IME-01
#[test]
fn leaving_a_field_drops_its_preedit_and_read_only_fields_get_no_ime() {
    let mut f = Field::new("input", &[], &[("value", "x".into())]);
    f.input(Input::ImePreedit {
        text: "abc".into(),
        cursor: Some((3, 3)),
    });
    f.input(Input::Blur(f.id));
    f.input(Input::Focus(f.id));
    f.type_text("y");
    assert_eq!(f.value(), "xy", "the dropped pre-edit left nothing behind");
    assert!(f.ui.ime_area(&f.tree).is_some());
    f.input(Input::Blur(f.id));
    assert_eq!(f.ui.ime_area(&f.tree), None, "nothing focused: no IME");

    let f = Field::new("input", &[], &[("readOnly", true.into())]);
    assert_eq!(f.ui.ime_area(&f.tree), None);
}
