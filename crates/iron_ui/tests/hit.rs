//! Hit testing (M3-EVT-01): the topmost element under a point, respecting paint
//! order, rounded corners, `pointerEvents` and `display: none`.

use iron_core::{Commit, NodeId, Value};
use iron_ui::UiTree;
use iron_ui::opcodes::*;

fn n(id: u32) -> Value {
    Value::Number(id.into())
}

fn op(code: u32, args: &[Value]) -> Value {
    let mut v = vec![Value::Number(code.into())];
    v.extend_from_slice(args);
    Value::Array(v)
}

struct Dom {
    ops: Vec<Value>,
    next: u32,
}

impl Dom {
    fn new() -> Self {
        Self {
            ops: Vec::new(),
            next: 1,
        }
    }

    /// A box with inline styles, appended to `parent`.
    fn add(&mut self, parent: u32, kind: &str, style: &[(&str, Value)]) -> u32 {
        let id = self.next;
        self.next += 1;
        self.ops.push(op(CREATE, &[n(id), kind.into()]));
        if !style.is_empty() {
            let mut a = vec![n(id)];
            for (k, v) in style {
                a.push((*k).into());
                a.push(v.clone());
            }
            self.ops.push(op(SET_STYLE, &a));
        }
        self.ops.push(op(APPEND_CHILD, &[n(parent), n(id)]));
        id
    }

    fn text(&mut self, parent: u32, text: &str) -> u32 {
        let id = self.next;
        self.next += 1;
        self.ops.push(op(CREATE_TEXT, &[n(id), text.into()]));
        self.ops.push(op(APPEND_CHILD, &[n(parent), n(id)]));
        id
    }

    fn build(self, css: &str) -> UiTree {
        let mut tree = UiTree::new(400.0, 400.0);
        let errors = tree.apply_commit(&Commit { ops: self.ops });
        assert!(errors.is_empty(), "{errors:?}");
        if !css.is_empty() {
            let added = tree.add_stylesheet(css);
            assert!(added.warnings.is_empty(), "{:?}", added.warnings);
        }
        tree.layout_if_needed();
        tree
    }
}

fn px(v: f64) -> Value {
    Value::Number(v)
}

fn hit(tree: &mut UiTree, x: f32, y: f32) -> Option<u32> {
    tree.hit_test(x, y).map(|id| id.0)
}

#[test]
fn topmost_child_wins_and_misses_fall_through() {
    let mut d = Dom::new();
    let outer = d.add(
        0,
        "box",
        &[
            ("width", px(200.0)),
            ("height", px(200.0)),
            ("padding", px(20.0)),
        ],
    );
    let inner = d.add(outer, "box", &[("width", px(50.0)), ("height", px(50.0))]);
    // An absolutely positioned later sibling overlapping `inner`: painted later, so on top.
    let over = d.add(
        outer,
        "box",
        &[
            ("position", "absolute".into()),
            ("left", px(40.0)),
            ("top", px(40.0)),
            ("width", px(50.0)),
            ("height", px(50.0)),
        ],
    );
    let mut tree = d.build("");
    assert_eq!(hit(&mut tree, 25.0, 25.0), Some(inner));
    assert_eq!(
        hit(&mut tree, 45.0, 45.0),
        Some(over),
        "later sibling is on top"
    );
    assert_eq!(hit(&mut tree, 150.0, 150.0), Some(outer));
    assert_eq!(hit(&mut tree, 300.0, 300.0), None, "only the root");
    assert_eq!(
        tree.event_path(NodeId(over)),
        vec![NodeId(over), NodeId(outer)]
    );
}

#[test]
fn children_outside_their_parent_still_hit() {
    let mut d = Dom::new();
    let parent = d.add(0, "box", &[("width", px(50.0)), ("height", px(50.0))]);
    let child = d.add(
        parent,
        "box",
        &[
            ("position", "absolute".into()),
            ("left", px(100.0)),
            ("width", px(20.0)),
            ("height", px(20.0)),
        ],
    );
    let mut tree = d.build("");
    assert_eq!(hit(&mut tree, 110.0, 10.0), Some(child));
}

#[test]
fn rounded_corners_are_respected() {
    let mut d = Dom::new();
    let circle = d.add(
        0,
        "box",
        &[
            ("width", px(100.0)),
            ("height", px(100.0)),
            ("borderRadius", "50%".into()),
        ],
    );
    let mut tree = d.build("");
    assert_eq!(hit(&mut tree, 50.0, 50.0), Some(circle));
    assert_eq!(hit(&mut tree, 50.0, 2.0), Some(circle), "edge midpoint");
    assert_eq!(hit(&mut tree, 5.0, 5.0), None, "outside the top-left curve");
    assert_eq!(
        hit(&mut tree, 95.0, 95.0),
        None,
        "outside the bottom-right curve"
    );
}

#[test]
fn pointer_events_none_is_inherited_and_can_be_reenabled() {
    let mut d = Dom::new();
    let base = d.add(0, "box", &[("width", px(300.0)), ("height", px(300.0))]);
    let overlay = d.add(
        base,
        "box",
        &[
            ("position", "absolute".into()),
            ("inset", px(0.0)),
            ("pointerEvents", "none".into()),
        ],
    );
    let passive = d.add(
        overlay,
        "box",
        &[("width", px(100.0)), ("height", px(100.0))],
    );
    let button = d.add(
        overlay,
        "box",
        &[
            ("width", px(100.0)),
            ("height", px(100.0)),
            ("pointerEvents", "auto".into()),
        ],
    );
    let mut tree = d.build("");
    let _ = passive;
    assert_eq!(
        hit(&mut tree, 50.0, 50.0),
        Some(base),
        "overlay and its child pass through"
    );
    assert_eq!(
        hit(&mut tree, 150.0, 50.0),
        Some(button),
        "child opted back in"
    );
    assert_eq!(hit(&mut tree, 250.0, 250.0), Some(base));
}

#[test]
fn display_none_subtrees_are_skipped() {
    let mut d = Dom::new();
    let base = d.add(0, "box", &[("width", px(100.0)), ("height", px(100.0))]);
    let hidden = d.add(
        base,
        "box",
        &[
            ("position", "absolute".into()),
            ("inset", px(0.0)),
            ("display", "none".into()),
        ],
    );
    d.add(
        hidden,
        "box",
        &[("width", px(100.0)), ("height", px(100.0))],
    );
    let mut tree = d.build("");
    assert_eq!(hit(&mut tree, 50.0, 50.0), Some(base));
}

#[test]
fn text_hits_its_text_element_and_generated_content_its_owner() {
    let mut d = Dom::new();
    let label = d.add(0, "text", &[]);
    d.text(label, "Hello");
    let icon = d.add(0, "box", &[("width", px(100.0)), ("height", px(40.0))]);
    let mut tree =
        d.build(r#"box::before { content: ""; width: 30px; height: 30px; background-color: red }"#);
    let text_rect = tree.rect(NodeId(label)).unwrap();
    assert_eq!(
        hit(&mut tree, text_rect.x + 5.0, text_rect.y + 5.0),
        Some(label)
    );
    let icon_rect = tree.rect(NodeId(icon)).unwrap();
    assert_eq!(
        hit(&mut tree, icon_rect.x + 10.0, icon_rect.y + 10.0),
        Some(icon),
        "the ::before box belongs to its element"
    );
}

#[test]
fn opacity_zero_still_hits() {
    // CSS: transparent elements are still targets (unlike `visibility: hidden`).
    let mut d = Dom::new();
    let ghost = d.add(
        0,
        "box",
        &[
            ("width", px(100.0)),
            ("height", px(100.0)),
            ("opacity", px(0.0)),
        ],
    );
    let mut tree = d.build("");
    assert_eq!(hit(&mut tree, 50.0, 50.0), Some(ghost));
}
