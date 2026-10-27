// SPDX-License-Identifier: Apache-2.0

//! Drag and drop end to end (M5-DND-09): [`Interaction`] runs the drag, `@iron/react`
//! runs the handlers inside QuickJS (the `dataTransfer` carries the data, preventing
//! `dragover` accepts), and the re-render comes back as a commit. Uses
//! `tests/fixtures/dnd.js`, built by Bun from `tests/fixtures/src/dnd.tsx`.

use iron_core::{NodeId, Value};
use iron_script::conformance::capture_logs;
use iron_script::{QuickJsRuntime, ScriptRuntime};
use iron_ui::{Input, Interaction, ROOT, UiTree};

const DND: &str = include_str!("../../../tests/fixtures/dnd.js");

struct Game {
    rt: QuickJsRuntime,
    tree: UiTree,
    ui: Interaction,
    got: NodeId,
    status: NodeId,
    now: f64,
}

impl Game {
    fn start() -> Self {
        let mut rt = QuickJsRuntime::new();
        let logs = capture_logs(&mut rt);
        rt.load_bundle("dnd.js", DND)
            .unwrap_or_else(|e| panic!("bundle failed: {e}\n{:?}", e.stack));
        assert!(logs.borrow().is_empty(), "{:?}", logs.borrow());
        let mut game = Self {
            rt,
            tree: UiTree::new(400.0, 300.0),
            ui: Interaction::new(),
            got: NodeId(0),
            status: NodeId(0),
            now: 0.0,
        };
        game.apply_commits();
        let app = game.tree.children(ROOT)[0];
        let labels = game.tree.children(app)[2];
        let [got, status] = game.tree.children(labels) else {
            panic!("two labels");
        };
        (game.got, game.status) = (*got, *status);
        game
    }

    fn apply_commits(&mut self) {
        for commit in self.rt.drain_commits() {
            let errors = self.tree.apply_commit(&commit);
            assert!(errors.is_empty(), "{errors:?}");
        }
    }

    /// Returns the drag events with whether each was prevented.
    fn handle(&mut self, input: Input) -> Vec<(&'static str, bool)> {
        let mut out = Vec::new();
        let rt = &mut self.rt;
        self.ui.handle(&mut self.tree, input, &mut |e| {
            let prevented = rt
                .call_global("__iron_ui_event", &e.script_args())
                .unwrap_or_else(|err| panic!("{err}"))
                == Value::Bool(true);
            if e.kind.is_drag() {
                out.push((e.kind.name(), prevented));
            }
            prevented
        });
        // Continuous updates (dragover) render on the next tick.
        self.now += 16.0;
        let errors = self.rt.tick(self.now);
        assert!(errors.is_empty(), "{errors:?}");
        self.apply_commits();
        out
    }

    fn labels(&self) -> (String, String) {
        (
            self.tree.text_content(self.got),
            self.tree.text_content(self.status),
        )
    }
}

// M5-DND-09
#[test]
fn an_element_dragged_onto_an_accepting_target_drops_its_data() {
    let mut game = Game::start();
    assert_eq!(game.labels(), ("nothing".into(), "idle".into()));
    game.handle(Input::PointerMove { x: 10.0, y: 10.0 });
    game.handle(Input::PointerDown {
        button: 0,
        time_ms: 0.0,
    });
    let events = game.handle(Input::PointerMove { x: 110.0, y: 10.0 });
    assert_eq!(
        events,
        [
            ("dragstart", false),
            ("drag", false),
            ("dragenter", false),
            ("dragover", true)
        ]
    );
    assert_eq!(game.labels().1, "dragging");
    let events = game.handle(Input::PointerUp { button: 0 });
    assert_eq!(events, [("drop", false), ("dragend", false)]);
    assert_eq!(game.labels(), ("item".into(), "dropped".into()));
}

// M5-DND-09
#[test]
fn files_dropped_from_the_system_reach_the_drop_handler() {
    let mut game = Game::start();
    let files = vec!["C:/in/a.png".to_string(), "/tmp/b.txt".to_string()];
    let events = game.handle(Input::FileDragEnter {
        files: files.clone(),
        x: 120.0,
        y: 20.0,
    });
    assert_eq!(events, [("dragenter", false), ("dragover", true)]);
    let events = game.handle(Input::FileDrop { files });
    assert_eq!(events, [("drop", false)]);
    assert_eq!(game.labels(), ("a.png,b.txt".into(), "idle".into()));
}
