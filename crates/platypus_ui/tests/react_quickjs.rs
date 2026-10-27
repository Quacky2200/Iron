// SPDX-License-Identifier: Apache-2.0

//! React running inside the engine's script runtime, driving the Rust UI tree
//! (M2-INT-01, M2-INT-02). Uses `tests/fixtures/boxes.js`, built by Bun from
//! `tests/fixtures/src/boxes.tsx`.

use platypus_core::{Commit, NodeId, Value};
use platypus_script::conformance::capture_logs;
use platypus_script::{QuickJsRuntime, ScriptRuntime};
use platypus_ui::opcodes::{CREATE, REMOVE_CHILD};
use platypus_ui::{DrawCmd, ROOT, Rect, UiTree};

const BOXES: &str = include_str!("../../../tests/fixtures/boxes.js");

fn count(commit: &Commit, opcode: u32) -> usize {
    commit
        .ops
        .iter()
        .filter(
            |op| matches!(op, Value::Array(a) if a.first() == Some(&Value::Number(opcode.into()))),
        )
        .count()
}

fn apply_all(tree: &mut UiTree, commits: &[Commit]) {
    for commit in commits {
        let errors = tree.apply_commit(commit);
        assert!(errors.is_empty(), "commit had errors: {errors:?}");
    }
}

fn rects(tree: &UiTree, ids: &[NodeId]) -> Vec<Rect> {
    ids.iter().map(|id| tree.rect(*id).unwrap()).collect()
}

/// Loads the fixture, returning the runtime, the tree and the IDs of
/// (outer box, header, grid body).
fn start() -> (QuickJsRuntime, UiTree, [NodeId; 3]) {
    let mut rt = QuickJsRuntime::new();
    let logs = capture_logs(&mut rt);
    rt.load_bundle("boxes.js", BOXES)
        .unwrap_or_else(|e| panic!("bundle failed: {e}\n{:?}", e.stack));
    assert!(
        logs.borrow().is_empty(),
        "React logged something: {:?}",
        logs.borrow()
    );

    let commits = rt.drain_commits();
    assert_eq!(commits.len(), 1, "the first render is one commit");

    let mut tree = UiTree::new(400.0, 300.0);
    apply_all(&mut tree, &commits);
    let outer = tree.children(ROOT)[0];
    let [header, body] = tree.children(outer) else {
        panic!("outer box should have a header and a body")
    };
    let (header, body) = (*header, *body);
    (rt, tree, [outer, header, body])
}

// M2-INT-01
#[test]
fn react_bundle_renders_into_the_ui_tree() {
    let (_rt, mut tree, [outer, header, body]) = start();
    assert_eq!(tree.len(), 5, "outer, header, body and two cells");
    assert_eq!(tree.kind(header), Some("box"));

    tree.layout_if_needed();
    // 400x300 root; outer box has 10px padding and a 10px column gap.
    assert_eq!(tree.rect(outer), Some(Rect::new(0.0, 0.0, 400.0, 300.0)));
    assert_eq!(tree.rect(header), Some(Rect::new(10.0, 10.0, 380.0, 50.0)));
    assert_eq!(tree.rect(body), Some(Rect::new(10.0, 70.0, 380.0, 220.0)));
    // Two 1fr columns with a 10px gap; one auto row stretched to the body height.
    assert_eq!(
        rects(&tree, tree.children(body)),
        [
            Rect::new(10.0, 70.0, 185.0, 220.0),
            Rect::new(205.0, 70.0, 185.0, 220.0),
        ]
    );

    // Styles came through: background + rounded header + two cells with shadows.
    let cmds = tree.draw_list();
    let shadows = cmds
        .iter()
        .filter(|c| matches!(c, DrawCmd::BoxShadow(_)))
        .count();
    assert_eq!(shadows, 2);
    let header_quad = cmds.iter().find_map(|c| match c {
        DrawCmd::Quad(q) if q.rect.height == 50.0 => Some(q),
        _ => None,
    });
    assert_eq!(
        header_quad.unwrap().radii,
        platypus_ui::Radii::circular([8.0; 4])
    );
}

// M2-INT-02
#[test]
fn timer_driven_state_change_commits_incrementally() {
    let (mut rt, mut tree, [outer, header, body]) = start();
    let cells_before = tree.children(body).to_vec();

    // Not due yet: no commit.
    assert!(rt.tick(50.0).is_empty());
    assert!(rt.drain_commits().is_empty());

    // The 100ms timer fires, React re-renders and commits within the same tick.
    let errors = rt.tick(100.0);
    assert!(errors.is_empty(), "{errors:?}");
    let commits = rt.drain_commits();
    assert_eq!(commits.len(), 1);
    assert_eq!(
        count(&commits[0], CREATE),
        2,
        "only the two new cells are created"
    );
    assert_eq!(count(&commits[0], REMOVE_CHILD), 0);
    apply_all(&mut tree, &commits);

    // Existing nodes keep their IDs; two cells were appended.
    assert_eq!(tree.children(ROOT), [outer]);
    assert_eq!(tree.children(outer), [header, body]);
    assert_eq!(&tree.children(body)[..2], cells_before.as_slice());
    assert_eq!(tree.children(body).len(), 4);

    // The grid now has two rows sharing the height: (220 - 10) / 2 = 105.
    tree.layout_if_needed();
    assert_eq!(
        rects(&tree, tree.children(body)),
        [
            Rect::new(10.0, 70.0, 185.0, 105.0),
            Rect::new(205.0, 70.0, 185.0, 105.0),
            Rect::new(10.0, 185.0, 185.0, 105.0),
            Rect::new(205.0, 185.0, 185.0, 105.0),
        ]
    );

    // Nothing else is scheduled.
    assert!(rt.tick(10_000.0).is_empty());
    assert!(rt.drain_commits().is_empty());
}
