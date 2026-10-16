//! Pointer events end to end (M3-EVT-02, M3-EVT-03): the Rust pointer state machine
//! hit-tests, `@iron/react` runs the handlers inside QuickJS, and the re-render
//! comes back as a commit. Uses `tests/fixtures/events.js`, built by Bun from
//! `tests/fixtures/src/events.tsx`.

use iron_core::{NodeId, Value};
use iron_script::conformance::capture_logs;
use iron_script::{QuickJsRuntime, ScriptRuntime};
use iron_ui::{Pointer, ROOT, Rgba, UiEvent, UiTree};

const EVENTS: &str = include_str!("../../../tests/fixtures/events.js");

struct Game {
    rt: QuickJsRuntime,
    tree: UiTree,
    pointer: Pointer,
    button: NodeId,
    button_label: NodeId,
    panel_label: NodeId,
}

impl Game {
    fn start() -> Self {
        let mut rt = QuickJsRuntime::new();
        let logs = capture_logs(&mut rt);
        rt.load_bundle("events.js", EVENTS)
            .unwrap_or_else(|e| panic!("bundle failed: {e}\n{:?}", e.stack));
        assert!(logs.borrow().is_empty(), "{:?}", logs.borrow());
        let mut tree = UiTree::new(400.0, 300.0);
        tree.add_stylesheet(".button:hover { background-color: #00ff00 }");
        let mut game = Self {
            rt,
            tree,
            pointer: Pointer::new(),
            button: NodeId(0),
            button_label: NodeId(0),
            panel_label: NodeId(0),
        };
        game.apply_commits();
        let panel = game.tree.children(ROOT)[0];
        let [button, panel_label] = game.tree.children(panel) else {
            panic!("the panel has a button and a label");
        };
        game.button = *button;
        game.panel_label = *panel_label;
        game.button_label = game.tree.children(game.button)[0];
        game
    }

    fn apply_commits(&mut self) {
        for commit in self.rt.drain_commits() {
            let errors = self.tree.apply_commit(&commit);
            assert!(errors.is_empty(), "{errors:?}");
        }
    }

    /// Dispatches like the app does, returning whether each event was prevented.
    fn dispatch(&mut self, events: Vec<UiEvent>) -> Vec<(&'static str, bool)> {
        let out = events
            .iter()
            .map(|e| {
                let prevented = self
                    .rt
                    .call_global("__iron_ui_event", &e.script_args())
                    .unwrap_or_else(|err| panic!("{err}"));
                (e.kind.name(), prevented == Value::Bool(true))
            })
            .collect();
        self.apply_commits();
        out
    }

    fn click(&mut self, x: f32, y: f32) -> Vec<(&'static str, bool)> {
        let mut events = self.pointer.move_to(&mut self.tree, x, y);
        events.extend(self.pointer.down(&mut self.tree, 0, 0.0));
        events.extend(self.pointer.up(&mut self.tree, 0));
        self.dispatch(events)
    }

    fn labels(&self) -> (String, String) {
        (
            self.tree.text_content(self.button_label),
            self.tree.text_content(self.panel_label),
        )
    }
}

// M3-EVT-03
#[test]
fn clicking_a_button_runs_its_on_click_and_re_renders() {
    let mut game = Game::start();
    assert_eq!(game.labels(), ("clicks 0".into(), "panel 0".into()));

    let events = game.click(50.0, 30.0);
    assert_eq!(
        events,
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

    let events = game.pointer.move_to(&mut game.tree, 50.0, 30.0);
    game.dispatch(events);
    game.tree.layout_if_needed();
    assert_eq!(game.tree.visual(button).unwrap().background, green);
    assert_eq!(
        game.pointer.cursor(&game.tree),
        "pointer",
        "set on the button, inherited by its label"
    );

    let events = game.pointer.move_to(&mut game.tree, 300.0, 200.0);
    game.dispatch(events);
    game.tree.layout_if_needed();
    assert_ne!(game.tree.visual(button).unwrap().background, green);
    assert_eq!(game.pointer.cursor(&game.tree), "default");
}
