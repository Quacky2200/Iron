use iron_core::{Commit, NodeId, Value};
use iron_ui::opcodes::*;
use iron_ui::{DrawCmd, Quad, ROOT, Rect, Rgba, UiTree};

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

fn quads(cmds: &[DrawCmd]) -> Vec<&Quad> {
    cmds.iter()
        .filter_map(|c| match c {
            DrawCmd::Quad(q) => Some(q),
            _ => None,
        })
        .collect()
}

/// A compact description of the command sequence, for order assertions.
fn shape(cmds: &[DrawCmd]) -> Vec<String> {
    cmds.iter()
        .map(|c| match c {
            DrawCmd::BackdropBlur { blur, .. } => format!("backdrop {blur}"),
            DrawCmd::BoxShadow(s) => format!("shadow {}", s.offset_y),
            DrawCmd::Quad(q) => format!("quad {}", q.rect.width),
            DrawCmd::InsetShadow { shadow, .. } => format!("inset {}", shadow.spread),
            DrawCmd::PushLayer { blur, opacity, .. } => {
                format!("push blur={blur} opacity={opacity}")
            }
            DrawCmd::PopLayer => "pop".into(),
        })
        .collect()
}

fn sized(id: u32, w: f64, h: f64, extra: &[(&str, Value)]) -> Value {
    let mut props = vec![("width", w.into()), ("height", h.into())];
    props.extend_from_slice(extra);
    style(id, &props)
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
            sized(1, 100.0, 100.0, &[("backgroundColor", red.clone())]),
            create(2), // no background or border: invisible, but its children still draw
            sized(2, 50.0, 50.0, &[]),
            create(3),
            sized(
                3,
                10.0,
                10.0,
                &[("borderWidth", 2.0.into()), ("borderColor", "blue".into())],
            ),
            create(4), // display: none hides its subtree
            style(
                4,
                &[("display", "none".into()), ("backgroundColor", red.clone())],
            ),
            create(5),
            sized(5, 5.0, 5.0, &[("backgroundColor", red.clone())]),
            create(6),
            sized(6, 20.0, 20.0, &[("backgroundColor", red.clone())]),
            append(2, 3),
            append(4, 5),
            append(1, 2),
            append(1, 4),
            append(1, 6),
            append(0, 1),
        ],
    );
    let cmds = tree.draw_list();
    let q = quads(&cmds);
    assert_eq!(
        cmds.len(),
        q.len(),
        "plain boxes only produce quads: {cmds:#?}"
    );
    assert_eq!(q.len(), 3);
    // Parent first, then children in order: 1, 3 (inside 2), 6.
    assert_eq!(q[0].rect, Rect::new(0.0, 0.0, 100.0, 100.0));
    assert_eq!(q[0].background, Rgba::new(1.0, 0.0, 0.0, 1.0));
    assert_eq!(q[1].rect, Rect::new(0.0, 0.0, 10.0, 10.0));
    assert_eq!(q[1].border, [2.0; 4]);
    assert_eq!(q[1].background.a, 0.0);
    assert_eq!(q[2].rect.x, 50.0, "box 6 follows box 2 in the row");
}

// M2-VIS-07
#[test]
fn effects_are_emitted_in_css_paint_order() {
    let mut tree = UiTree::new(200.0, 200.0);
    apply_ok(
        &mut tree,
        vec![
            create(1),
            sized(
                1,
                100.0,
                100.0,
                &[
                    ("backgroundColor", "white".into()),
                    (
                        "boxShadow",
                        "0 1px 2px black, inset 0 0 0 3px red, 0 2px 4px black".into(),
                    ),
                    ("filter", "blur(4px)".into()),
                    ("backdropFilter", "blur(10px)".into()),
                    ("borderRadius", "8px 0".into()),
                ],
            ),
            create(2),
            sized(2, 10.0, 10.0, &[("backgroundColor", "black".into())]),
            append(1, 2),
            append(0, 1),
        ],
    );
    let cmds = tree.draw_list();
    assert_eq!(
        shape(&cmds),
        [
            "backdrop 10",
            "push blur=4 opacity=1",
            "shadow 2", // last shadow in the list is painted first (bottom-most)
            "shadow 1",
            "quad 100",
            "inset 3",
            "quad 10", // the child, inside the layer
            "pop",
        ]
    );
    let DrawCmd::Quad(q) = &cmds[4] else { panic!() };
    assert_eq!(q.radii, [8.0, 0.0, 8.0, 0.0]);
    // The layer covers the larger shadow (box offset 2 down, grown by its blur of 4:
    // x -4..104, y -2..106) plus 3x the filter blur (12) on every side.
    let DrawCmd::PushLayer { bounds, .. } = cmds[1] else {
        panic!()
    };
    assert_eq!(
        bounds,
        Rect::new(-4.0 - 12.0, -2.0 - 12.0, 108.0 + 24.0, 108.0 + 24.0)
    );
}

// M2-VIS-08
#[test]
fn opacity_groups_children_but_scales_simple_leaves() {
    let mut tree = UiTree::new(200.0, 200.0);
    apply_ok(
        &mut tree,
        vec![
            create(1), // parent with a child: one layer, children keep full alpha
            sized(
                1,
                100.0,
                100.0,
                &[("opacity", 0.5.into()), ("backgroundColor", "red".into())],
            ),
            create(2),
            sized(2, 10.0, 10.0, &[("backgroundColor", "lime".into())]),
            create(3), // leaf with a single quad: alpha is scaled, no layer
            sized(
                3,
                10.0,
                10.0,
                &[("opacity", 0.25.into()), ("backgroundColor", "blue".into())],
            ),
            create(4), // leaf with a shadow and a quad: needs a layer so they don't double-blend
            sized(
                4,
                10.0,
                10.0,
                &[
                    ("opacity", 0.5.into()),
                    ("backgroundColor", "blue".into()),
                    ("boxShadow", "0 0 4px black".into()),
                ],
            ),
            create(5), // fully transparent: skipped entirely
            sized(
                5,
                10.0,
                10.0,
                &[("opacity", 0.0.into()), ("backgroundColor", "blue".into())],
            ),
            append(1, 2),
            append(0, 1),
            append(0, 3),
            append(0, 4),
            append(0, 5),
        ],
    );
    let cmds = tree.draw_list();
    assert_eq!(
        shape(&cmds),
        [
            "push blur=0 opacity=0.5",
            "quad 100",
            "quad 10",
            "pop",
            "quad 10",
            "push blur=0 opacity=0.5",
            "shadow 0",
            "quad 10",
            "pop",
        ]
    );
    let q = quads(&cmds);
    assert_eq!(
        q[0].background.a, 1.0,
        "inside the layer, colours stay opaque"
    );
    assert_eq!(q[1].background.a, 1.0);
    assert_eq!(
        q[2].background.a, 0.25,
        "a simple leaf just scales its alpha"
    );
}

#[test]
fn backgrounds_and_shadows_without_a_fill_still_draw() {
    let mut tree = UiTree::new(100.0, 100.0);
    apply_ok(
        &mut tree,
        vec![
            create(1),
            sized(
                1,
                50.0,
                50.0,
                &[
                    ("backgroundImage", "linear-gradient(red, blue)".into()),
                    ("backgroundSize", "cover".into()),
                ],
            ),
            create(2), // transparent box that only casts a shadow
            sized(
                2,
                20.0,
                20.0,
                &[("boxShadow", "0 0 8px rgba(0,0,0,0.5)".into())],
            ),
            append(0, 1),
            append(0, 2),
        ],
    );
    let cmds = tree.draw_list();
    assert_eq!(shape(&cmds), ["quad 50", "shadow 0"]);
    let DrawCmd::Quad(q) = &cmds[0] else { panic!() };
    let fill = q.image.as_ref().expect("gradient fill");
    assert_eq!(fill.size, iron_ui::BackgroundSize::Cover);
    assert!(matches!(
        *fill.image,
        iron_ui::BackgroundImage::Linear { .. }
    ));
}
