use iron_core::{Commit, NodeId, Value};
use iron_ui::opcodes::*;
use iron_ui::{ROOT, Rect, Rgba, UiTree};

fn op(code: u32, args: &[Value]) -> Value {
    let mut v = vec![Value::Number(code.into())];
    v.extend_from_slice(args);
    Value::Array(v)
}

fn n(id: u32) -> Value {
    Value::Number(id.into())
}

fn create(id: u32) -> Value {
    op(CREATE, &[n(id), "box".into()])
}

fn style(id: u32, props: &[(&str, Value)]) -> Value {
    let mut args = vec![n(id)];
    for (k, v) in props {
        args.push((*k).into());
        args.push(v.clone());
    }
    op(SET_STYLE, &args)
}

fn append(parent: u32, child: u32) -> Value {
    op(APPEND_CHILD, &[n(parent), n(child)])
}

fn commit(ops: Vec<Value>) -> Commit {
    Commit { ops }
}

fn apply_ok(tree: &mut UiTree, ops: Vec<Value>) {
    let errors = tree.apply_commit(&commit(ops));
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

fn ids(list: &[NodeId]) -> Vec<u32> {
    list.iter().map(|n| n.0).collect()
}

// M2-UI-01
#[test]
fn builds_the_tree_from_ops() {
    let mut tree = UiTree::new(100.0, 100.0);
    apply_ok(
        &mut tree,
        vec![
            create(1),
            create(2),
            create(3),
            append(1, 2),
            append(1, 3),
            append(0, 1),
        ],
    );
    assert_eq!(tree.len(), 3);
    assert_eq!(ids(tree.children(ROOT)), [1]);
    assert_eq!(ids(tree.children(NodeId(1))), [2, 3]);
    assert_eq!(tree.parent(NodeId(3)), Some(NodeId(1)));
    assert_eq!(tree.kind(NodeId(2)), Some("box"));

    // Insert before, then move an existing child to the end.
    apply_ok(
        &mut tree,
        vec![
            create(4),
            op(INSERT_BEFORE, &[n(1), n(4), n(2)]),
            append(1, 2),
        ],
    );
    assert_eq!(ids(tree.children(NodeId(1))), [4, 3, 2]);

    // Removing destroys the whole subtree.
    apply_ok(
        &mut tree,
        vec![create(5), append(3, 5), op(REMOVE_CHILD, &[n(1), n(3)])],
    );
    assert_eq!(ids(tree.children(NodeId(1))), [4, 2]);
    assert!(!tree.contains(NodeId(3)) && !tree.contains(NodeId(5)));
}

// M2-UI-02
#[test]
fn bad_ops_are_reported_and_skipped() {
    let mut tree = UiTree::new(100.0, 100.0);
    let errors = tree.apply_commit(&commit(vec![
        create(1),
        append(0, 99),                     // unknown child
        append(42, 1),                     // unknown parent
        create(1),                         // duplicate id
        op(77, &[]),                       // unknown opcode
        Value::String("not an op".into()), // not an array
        op(CREATE, &[n(2)]),               // missing type
        create(2),
        append(1, 2),
        append(2, 1),                                                  // cycle
        op(INSERT_BEFORE, &[n(0), n(2), n(1)]), // `before` not a child of root
        op(REMOVE_CHILD, &[n(0), n(2)]),        // not a child of root
        style(1, &[("width", 10.0.into()), ("colour", "red".into())]), // one bad prop
        append(0, 1),
    ]));
    assert_eq!(errors.len(), 10, "{errors:#?}");
    assert!(
        errors
            .iter()
            .any(|e| e.0.contains("unknown style property `colour`"))
    );
    // Everything valid still applied, including the good half of the style op.
    assert_eq!(ids(tree.children(ROOT)), [1]);
    assert_eq!(ids(tree.children(NodeId(1))), [2]);
    tree.layout_if_needed();
    assert_eq!(tree.rect(NodeId(1)).unwrap().width, 10.0);
}

// M2-UI-03
#[test]
fn resizing_the_root_relayouts() {
    let mut tree = UiTree::new(200.0, 100.0);
    apply_ok(
        &mut tree,
        vec![
            create(1),
            style(1, &[("width", "50%".into()), ("height", "100%".into())]),
            append(0, 1),
        ],
    );
    tree.layout_if_needed();
    assert_eq!(
        tree.rect(NodeId(1)),
        Some(Rect::new(0.0, 0.0, 100.0, 100.0))
    );

    tree.set_size(400.0, 300.0);
    assert!(tree.layout_if_needed());
    assert_eq!(
        tree.rect(NodeId(1)),
        Some(Rect::new(0.0, 0.0, 200.0, 300.0))
    );
}

// M2-LAY-04
#[test]
fn layout_only_runs_when_something_changed() {
    let mut tree = UiTree::new(100.0, 100.0);
    apply_ok(&mut tree, vec![create(1), append(0, 1)]);
    assert!(tree.layout_if_needed());
    assert!(!tree.layout_if_needed(), "nothing changed");
    tree.set_size(100.0, 100.0);
    assert!(!tree.layout_if_needed(), "same size is not a change");
    apply_ok(&mut tree, vec![style(1, &[("width", 50.0.into())])]);
    assert!(tree.layout_if_needed());
}

// M2-UI-04
#[test]
fn draw_list_is_in_paint_order_and_skips_invisible_boxes() {
    let mut tree = UiTree::new(100.0, 100.0);
    let red: Value = "#ff0000".into();
    apply_ok(
        &mut tree,
        vec![
            create(1),
            style(
                1,
                &[
                    ("width", 100.0.into()),
                    ("height", 100.0.into()),
                    ("backgroundColor", red.clone()),
                ],
            ),
            create(2), // no background or border: invisible, but its children still draw
            style(2, &[("width", 50.0.into()), ("height", 50.0.into())]),
            create(3),
            style(
                3,
                &[
                    ("width", 10.0.into()),
                    ("height", 10.0.into()),
                    ("borderWidth", 2.0.into()),
                    ("borderColor", "blue".into()),
                ],
            ),
            create(4), // display: none hides its subtree
            style(
                4,
                &[("display", "none".into()), ("backgroundColor", red.clone())],
            ),
            create(5),
            style(
                5,
                &[
                    ("width", 5.0.into()),
                    ("height", 5.0.into()),
                    ("backgroundColor", red.clone()),
                ],
            ),
            create(6), // opacity multiplies down the tree
            style(
                6,
                &[
                    ("width", 20.0.into()),
                    ("height", 20.0.into()),
                    ("opacity", 0.5.into()),
                    ("backgroundColor", red.clone()),
                ],
            ),
            create(7),
            style(
                7,
                &[
                    ("width", 10.0.into()),
                    ("height", 10.0.into()),
                    ("opacity", 0.5.into()),
                    ("backgroundColor", "#00ff00".into()),
                    ("borderRadius", 4.0.into()),
                ],
            ),
            append(2, 3),
            append(4, 5),
            append(6, 7),
            append(1, 2),
            append(1, 4),
            append(1, 6),
            append(0, 1),
        ],
    );
    let quads = tree.draw_list();
    assert_eq!(quads.len(), 4, "{quads:#?}");
    // Parent first, then children in order: 1, 3 (inside 2), 6, 7.
    assert_eq!(quads[0].rect, Rect::new(0.0, 0.0, 100.0, 100.0));
    assert_eq!(quads[0].background, Rgba::new(1.0, 0.0, 0.0, 1.0));
    assert_eq!(quads[1].rect, Rect::new(0.0, 0.0, 10.0, 10.0));
    assert_eq!(quads[1].border, [2.0; 4]);
    assert_eq!(quads[1].background.a, 0.0);
    assert_eq!(quads[2].rect.x, 50.0, "box 6 follows box 2 in the row");
    assert_eq!(quads[2].background.a, 0.5);
    assert_eq!(quads[3].background.a, 0.25);
    assert_eq!(quads[3].radius, 4.0);
}
