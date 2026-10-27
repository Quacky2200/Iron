// SPDX-License-Identifier: Apache-2.0

//! UI events end to end (M3-EVT-02, M3-EVT-03, M3-EVT-04): [`Interaction`] hit-tests
//! and tracks focus, `@platypus/react` runs the handlers inside QuickJS, and re-renders
//! come back as commits; `ref.focus()` / `ref.blur()` come back as ops. Uses
//! `tests/fixtures/events.js`, built by Bun from `tests/fixtures/src/events.tsx`.

use platypus_core::{NodeId, Value};
use platypus_script::conformance::capture_logs;
use platypus_script::{QuickJsRuntime, ScriptRuntime};
use platypus_ui::{Input, Interaction, ROOT, Rgba, UiTree};

const EVENTS: &str = include_str!("../../../tests/fixtures/events.js");

struct Game {
    rt: QuickJsRuntime,
    tree: UiTree,
    ui: Interaction,
    button: NodeId,
    button_label: NodeId,
    panel_label: NodeId,
    field: NodeId,
    field_label: NodeId,
    name: NodeId,
}

impl Game {
    fn start() -> Self {
        let mut rt = QuickJsRuntime::new();
        let logs = capture_logs(&mut rt);
        rt.load_bundle("events.js", EVENTS)
            .unwrap_or_else(|e| panic!("bundle failed: {e}\n{:?}", e.stack));
        assert!(logs.borrow().is_empty(), "{:?}", logs.borrow());
        let mut tree = UiTree::new(400.0, 300.0);
        tree.add_stylesheet(
            ".button:hover { background-color: #00ff00 }
             .field:focus-visible { background-color: #0000ff }",
        );
        let mut game = Self {
            rt,
            tree,
            ui: Interaction::new(),
            button: NodeId(0),
            button_label: NodeId(0),
            panel_label: NodeId(0),
            field: NodeId(0),
            field_label: NodeId(0),
            name: NodeId(0),
        };
        game.apply_commits();
        let panel = game.tree.children(ROOT)[0];
        let [button, panel_label, field, name] = game.tree.children(panel) else {
            panic!("the panel has a button, a label, a field and a name input");
        };
        game.name = *name;
        (game.button, game.panel_label, game.field) = (*button, *panel_label, *field);
        game.button_label = game.tree.children(game.button)[0];
        game.field_label = game.tree.children(game.field)[0];
        game
    }

    fn apply_commits(&mut self) {
        for commit in self.rt.drain_commits() {
            let errors = self.tree.apply_commit(&commit);
            assert!(errors.is_empty(), "{errors:?}");
        }
    }

    /// Handles one input like the app: events go to `__platypus_ui_event`, commits are
    /// applied, then `focus` / `blur` ops are handled. Returns each event's type and
    /// whether it was prevented.
    fn handle(&mut self, input: Input) -> Vec<(&'static str, bool)> {
        let mut out = Vec::new();
        let rt = &mut self.rt;
        self.ui.handle(&mut self.tree, input, &mut |e| {
            let prevented = rt
                .call_global("__platypus_ui_event", &e.script_args())
                .unwrap_or_else(|err| panic!("{err}"))
                == Value::Bool(true);
            out.push((e.kind.name(), prevented));
            prevented
        });
        self.apply_commits();
        // The engine's own ops (`render.quality` on load) don't concern the UI.
        for op in self.rt.drain_ops() {
            if op.name.starts_with("render.") {
                continue;
            }
            let id = NodeId(op.args[0].as_f64().unwrap() as u32);
            let input = match op.name.as_str() {
                "focus" => Input::Focus(id),
                "blur" => Input::Blur(id),
                other => panic!("unexpected op {other}"),
            };
            out.extend(self.handle(input));
        }
        out
    }

    fn click(&mut self, x: f32, y: f32) -> Vec<(&'static str, bool)> {
        let mut events = self.handle(Input::PointerMove { x, y });
        events.extend(self.handle(Input::PointerDown {
            button: 0,
            time_ms: 0.0,
        }));
        events.extend(self.handle(Input::PointerUp { button: 0 }));
        events
    }

    fn key(&mut self, key: &str) -> Vec<(&'static str, bool)> {
        self.handle(Input::KeyDown {
            key: key.into(),
            code: String::new(),
            repeat: false,
            text: None,
        })
    }

    fn labels(&self) -> (String, String) {
        (
            self.tree.text_content(self.button_label),
            self.tree.text_content(self.panel_label),
        )
    }

    fn field_text(&self) -> String {
        self.tree.text_content(self.field_label)
    }
}

// M3-EVT-03
#[test]
fn clicking_a_button_runs_its_on_click_and_re_renders() {
    let mut game = Game::start();
    assert_eq!(game.labels(), ("clicks 0".into(), "panel 0".into()));

    let events = game.click(50.0, 30.0);
    assert_eq!(
        &events[..7],
        [
            ("pointerenter", false),
            ("pointerenter", false),
            ("pointerenter", false),
            ("pointermove", false),
            ("pointerdown", true),
            ("pointerup", false),
            ("click", false),
        ],
        "panel, button, then the label text are entered; pointerdown is prevented"
    );
    assert_eq!(
        game.labels(),
        ("clicks 1".into(), "panel 0".into()),
        "the button stopped the click from bubbling to the panel"
    );

    game.click(50.0, 30.0);
    assert_eq!(game.labels(), ("clicks 2".into(), "panel 0".into()));

    // Below the button: the panel gets it.
    game.click(300.0, 200.0);
    assert_eq!(game.labels(), ("clicks 2".into(), "panel 1".into()));
}

// M3-EVT-03
#[test]
fn hover_styles_and_cursor_follow_the_pointer() {
    let mut game = Game::start();
    let green = Rgba::parse("#00ff00").unwrap();
    let button = game.button;

    game.handle(Input::PointerMove { x: 50.0, y: 30.0 });
    game.tree.layout_if_needed();
    assert_eq!(game.tree.visual(button).unwrap().background, green);
    assert_eq!(
        game.ui.cursor(&game.tree),
        "pointer",
        "set on the button, inherited by its label"
    );

    game.handle(Input::PointerMove { x: 300.0, y: 200.0 });
    game.tree.layout_if_needed();
    assert_ne!(game.tree.visual(button).unwrap().background, green);
    assert_eq!(game.ui.cursor(&game.tree), "default");
}

// M3-EVT-04
#[test]
fn tab_focuses_the_field_and_typed_keys_reach_it() {
    let mut game = Game::start();
    let blue = Rgba::parse("#0000ff").unwrap();
    assert_eq!(game.field_text(), "idle: ");

    assert_eq!(game.key("Tab"), [("focusin", false)]);
    assert_eq!(game.ui.focused(), Some(game.field));
    assert_eq!(game.field_text(), "focused: ", "onFocus ran");
    game.tree.layout_if_needed();
    assert_eq!(
        game.tree.visual(game.field).unwrap().background,
        blue,
        ":focus-visible after Tab"
    );

    for key in ["h", "i", "!", "Backspace", "Shift"] {
        game.key(key);
    }
    assert_eq!(game.field_text(), "focused: hi");

    let events = game.key("Escape");
    assert_eq!(
        events,
        [("keydown", false), ("focusout", false)],
        "Escape calls ref.blur(), applied through an op"
    );
    assert_eq!(game.ui.focused(), None);
    assert_eq!(game.field_text(), "idle: hi", "onBlur ran");
}

// M3-EVT-04
#[test]
fn a_button_can_focus_the_field_without_taking_focus_itself() {
    let mut game = Game::start();
    game.click(50.0, 30.0);
    assert_eq!(
        game.ui.focused(),
        Some(game.field),
        "the click handler called ref.focus()"
    );
    assert!(
        !game.tree.state(game.field).focus_visible,
        "no keyboard use, so no focus ring"
    );
    game.key("x");
    assert_eq!(game.field_text(), "focused: x");

    // Pressing elsewhere blurs.
    game.click(300.0, 250.0);
    assert_eq!(game.ui.focused(), None);
}

// M3-INP-01
#[test]
fn typing_into_a_controlled_input_goes_through_react() {
    let mut game = Game::start();
    let name = game.name;
    game.handle(Input::Focus(name));
    for c in ["a", "1", "b"] {
        let events = game.handle(Input::KeyDown {
            key: c.into(),
            code: String::new(),
            repeat: false,
            text: Some(c.into()),
        });
        assert!(events.contains(&("input", false)), "{events:?}");
    }
    assert_eq!(
        game.tree.editor_value(name).unwrap(),
        "AB",
        "upper-cased by onChange, and the digit was refused"
    );
    game.handle(Input::KeyDown {
        key: "c".into(),
        code: String::new(),
        repeat: false,
        text: Some("c".into()),
    });
    assert_eq!(
        game.tree.editor_value(name).unwrap(),
        "ABC",
        "the caret stayed at the end"
    );
}
