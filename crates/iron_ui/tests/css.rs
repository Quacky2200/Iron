//! Stylesheets applied to the UI tree: selectors, cascade and restyle
//! (M3-CSS-02, -03, -04, -08, -09, -10).

use iron_core::{Commit, NodeId, Value};
use iron_ui::opcodes::*;
use iron_ui::{ElementState, Rgba, UiTree};

fn n(id: u32) -> Value {
    Value::Number(id.into())
}

fn op(code: u32, args: &[Value]) -> Value {
    let mut v = vec![Value::Number(code.into())];
    v.extend_from_slice(args);
    Value::Array(v)
}

/// Builds trees from a compact description.
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

    /// An element with attributes (`class`, `id`, `data-*`, `disabled`, `checked`).
    fn el(&mut self, parent: u32, kind: &str, attrs: &[(&str, Value)]) -> u32 {
        let id = self.next;
        self.next += 1;
        self.ops.push(op(CREATE, &[n(id), kind.into()]));
        if !attrs.is_empty() {
            let mut a = vec![n(id)];
            for (k, v) in attrs {
                a.push((*k).into());
                a.push(v.clone());
            }
            self.ops.push(op(SET_ATTRS, &a));
        }
        self.ops.push(op(APPEND_CHILD, &[n(parent), n(id)]));
        id
    }

    fn class(&mut self, parent: u32, class: &str) -> u32 {
        self.el(parent, "box", &[("className", class.into())])
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
        let added = tree.add_stylesheet(css);
        assert!(added.warnings.is_empty(), "{:?}", added.warnings);
        tree.layout_if_needed();
        tree
    }
}

fn bg(tree: &UiTree, id: u32) -> Rgba {
    tree.visual(NodeId(id)).unwrap().background
}

const RED: Rgba = Rgba::new(1.0, 0.0, 0.0, 1.0);
const GREEN: Rgba = Rgba::new(0.0, 128.0 / 255.0, 0.0, 1.0);
const BLUE: Rgba = Rgba::new(0.0, 0.0, 1.0, 1.0);
const NONE: Rgba = Rgba::TRANSPARENT;

// M3-CSS-08
#[test]
fn simple_selectors_and_specificity() {
    let mut d = Dom::new();
    let a = d.el(
        0,
        "box",
        &[("className", "card big".into()), ("id", "main".into())],
    );
    let b = d.el(
        0,
        "box",
        &[("className", "card".into()), ("data-kind", "enemy".into())],
    );
    let c = d.el(0, "text", &[]);
    let tree = d.build(
        r#"
        * { opacity: 0.5 }
        box { background-color: red }
        .card { background-color: green }
        #main { background-color: blue }
        .card.big { background-color: red }          /* lower specificity than #main */
        [data-kind="enemy"] { border: 1px solid red }
        text { opacity: 1 }
        "#,
    );
    assert_eq!(bg(&tree, a), BLUE, "#id beats .class.class");
    assert_eq!(bg(&tree, b), GREEN, ".class beats type");
    assert_eq!(
        tree.visual(NodeId(b)).unwrap().border_color,
        RED,
        "attribute selector"
    );
    assert_eq!(tree.visual(NodeId(a)).unwrap().opacity, 0.5, "universal");
    assert_eq!(
        tree.visual(NodeId(c)).unwrap().opacity,
        1.0,
        "type beats universal"
    );
}

// M3-CSS-02
#[test]
fn combinators() {
    let mut d = Dom::new();
    let list = d.class(0, "list");
    let first = d.class(list, "item");
    let second = d.class(list, "item special");
    let third = d.class(list, "item");
    let fourth = d.class(list, "item");
    let deep_parent = d.class(list, "wrap");
    let deep = d.class(deep_parent, "item");
    let tree = d.build(
        r#"
        .list .item { background-color: red }          /* descendant: all items */
        .list > .item { background-color: green }      /* child: not the deep one */
        .special + .item { background-color: blue }    /* next sibling */
        .special ~ .item { border: 2px solid blue }    /* following siblings */
        "#,
    );
    assert_eq!(bg(&tree, first), GREEN);
    assert_eq!(bg(&tree, deep), RED, "descendant but not child");
    assert_eq!(bg(&tree, third), BLUE, "+ picks the next sibling only");
    assert_eq!(bg(&tree, fourth), GREEN);
    for id in [third, fourth] {
        assert_eq!(
            tree.visual(NodeId(id)).unwrap().border_color,
            BLUE,
            "~ picks all following"
        );
    }
    assert_eq!(tree.visual(NodeId(second)).unwrap().border_color, NONE);
}

// M3-CSS-02
#[test]
fn structural_pseudo_classes() {
    let mut d = Dom::new();
    let list = d.class(0, "list");
    // Text nodes between elements must not count as siblings.
    d.text(list, " ");
    let items: Vec<u32> = (0..5)
        .map(|i| d.class(list, if i == 2 { "item x" } else { "item" }))
        .collect();
    let only_parent = d.class(0, "solo");
    let only = d.el(only_parent, "text", &[]);
    let tree = d.build(
        r#"
        .item:first-child { background-color: red }
        .item:last-child { background-color: blue }
        .item:nth-child(2) { opacity: 0.2 }
        .item:nth-child(odd of .item) { border: 1px solid green }
        .item:nth-last-child(2) { opacity: 0.4 }
        text:only-child { opacity: 0.6 }
        box:first-of-type { flex-grow: 3 }
        :root { opacity: 0.9 }
        "#,
    );
    assert_eq!(bg(&tree, items[0]), RED);
    assert_eq!(bg(&tree, items[4]), BLUE);
    assert_eq!(tree.visual(NodeId(items[1])).unwrap().opacity, 0.2);
    assert_eq!(tree.visual(NodeId(items[3])).unwrap().opacity, 0.4);
    for (i, id) in items.iter().enumerate() {
        let expected = if i % 2 == 0 { GREEN } else { NONE };
        assert_eq!(
            tree.visual(NodeId(*id)).unwrap().border_color,
            expected,
            "item {i}"
        );
    }
    assert_eq!(tree.visual(NodeId(only)).unwrap().opacity, 0.6);
    assert_eq!(
        tree.visual(iron_ui::ROOT).unwrap().opacity,
        0.9,
        ":root is the root container"
    );
}

// M3-CSS-03
#[test]
fn logical_pseudo_classes() {
    let mut d = Dom::new();
    let plain = d.class(0, "a");
    let other = d.class(0, "b");
    let parent_with = d.class(0, "p");
    d.class(parent_with, "child");
    let parent_without = d.class(0, "p");
    d.class(parent_without, "other");
    let before_sib = d.class(0, "q");
    d.class(0, "marker");
    let tree = d.build(
        r#"
        box:not(.a) { opacity: 0.5 }
        :is(.a, .b) { background-color: red }
        :where(.b) { background-color: green }        /* zero specificity: loses to :is(.b) */
        .p:has(> .child) { background-color: blue }
        .q:has(+ .marker) { border: 1px solid red }
        "#,
    );
    assert_eq!(tree.visual(NodeId(plain)).unwrap().opacity, 1.0);
    assert_eq!(tree.visual(NodeId(other)).unwrap().opacity, 0.5);
    assert_eq!(bg(&tree, plain), RED);
    assert_eq!(bg(&tree, other), RED, ":where has zero specificity");
    assert_eq!(bg(&tree, parent_with), BLUE);
    assert_eq!(bg(&tree, parent_without), NONE);
    assert_eq!(tree.visual(NodeId(before_sib)).unwrap().border_color, RED);
}

// M3-CSS-09
#[test]
fn empty_and_blank() {
    let mut d = Dom::new();
    let empty = d.class(0, "t");
    let blank = d.class(0, "t");
    d.text(blank, "  \n ");
    let texty = d.class(0, "t");
    d.text(texty, "hi");
    let parent = d.class(0, "t");
    d.class(parent, "kid");
    let tree = d.build(
        r#"
        .t:empty { background-color: red }
        .t:blank { border: 1px solid blue }
        "#,
    );
    assert_eq!(bg(&tree, empty), RED);
    assert_eq!(bg(&tree, blank), NONE, "whitespace text isn't :empty");
    assert_eq!(tree.visual(NodeId(empty)).unwrap().border_color, BLUE);
    assert_eq!(
        tree.visual(NodeId(blank)).unwrap().border_color,
        BLUE,
        "whitespace text is :blank"
    );
    for id in [texty, parent] {
        assert_eq!(bg(&tree, id), NONE);
        assert_eq!(tree.visual(NodeId(id)).unwrap().border_color, NONE);
    }
}

// M3-CSS-10
#[test]
fn state_and_form_pseudo_classes() {
    let mut d = Dom::new();
    let button = d.class(0, "btn");
    let off = d.el(
        0,
        "box",
        &[("className", "btn".into()), ("disabled", true.into())],
    );
    let check = d.el(
        0,
        "box",
        &[("className", "check".into()), ("checked", true.into())],
    );
    let form = d.class(0, "form");
    let field = d.class(form, "field");
    let mut tree = d.build(
        r#"
        .btn { background-color: green }
        .btn:hover { background-color: blue }
        .btn:active { background-color: red }
        .btn:enabled { opacity: 1 }
        .btn:disabled { opacity: 0.3 }
        .check:checked { background-color: red }
        .field:focus { border: 2px solid blue }
        .field:focus-visible { opacity: 0.8 }
        .form:focus-within { background-color: blue }
        "#,
    );
    assert_eq!(bg(&tree, button), GREEN);
    assert_eq!(tree.visual(NodeId(off)).unwrap().opacity, 0.3);
    assert_eq!(bg(&tree, check), RED);

    tree.set_state(
        NodeId(button),
        ElementState {
            hover: true,
            ..Default::default()
        },
    );
    tree.layout_if_needed();
    assert_eq!(bg(&tree, button), BLUE, ":hover");
    tree.set_state(
        NodeId(button),
        ElementState {
            hover: true,
            active: true,
            ..Default::default()
        },
    );
    tree.layout_if_needed();
    assert_eq!(bg(&tree, button), RED, ":active comes later in the sheet");
    tree.set_state(NodeId(button), ElementState::default());
    tree.layout_if_needed();
    assert_eq!(bg(&tree, button), GREEN, "back to normal");

    tree.set_state(
        NodeId(field),
        ElementState {
            focus: true,
            focus_visible: true,
            ..Default::default()
        },
    );
    tree.layout_if_needed();
    assert_eq!(tree.visual(NodeId(field)).unwrap().border_color, BLUE);
    assert_eq!(tree.visual(NodeId(field)).unwrap().opacity, 0.8);
    assert_eq!(bg(&tree, form), BLUE, ":focus-within on the ancestor");

    // Toggling an attribute restyles.
    let errors = tree.apply_commit(&Commit {
        ops: vec![op(SET_ATTRS, &[n(off), "disabled".into(), Value::Null])],
    });
    assert!(errors.is_empty());
    tree.layout_if_needed();
    assert_eq!(tree.visual(NodeId(off)).unwrap().opacity, 1.0);
}

// M3-CSS-04
#[test]
fn cascade_order_inline_and_important() {
    let mut d = Dom::new();
    let a = d.class(0, "a");
    let b = d.class(0, "b");
    let mut tree = d.build(
        r#"
        .a { background-color: red; width: 10px }
        .a { background-color: green }               /* later rule wins at equal specificity */
        .b { background-color: green !important; width: 30px }
        "#,
    );
    assert_eq!(bg(&tree, a), GREEN);

    // Inline styles beat normal rules but not !important ones.
    let errors = tree.apply_commit(&Commit {
        ops: vec![
            op(
                SET_STYLE,
                &[
                    n(a),
                    "backgroundColor".into(),
                    "blue".into(),
                    "width".into(),
                    20.0.into(),
                ],
            ),
            op(
                SET_STYLE,
                &[
                    n(b),
                    "backgroundColor".into(),
                    "blue".into(),
                    "width".into(),
                    40.0.into(),
                ],
            ),
        ],
    });
    assert!(errors.is_empty());
    tree.layout_if_needed();
    assert_eq!(bg(&tree, a), BLUE);
    assert_eq!(tree.rect(NodeId(a)).unwrap().width, 20.0);
    assert_eq!(bg(&tree, b), GREEN, "!important beats inline");
    assert_eq!(
        tree.rect(NodeId(b)).unwrap().width,
        40.0,
        "inline beats a normal rule"
    );

    // Removing an inline style falls back to the stylesheet.
    let errors = tree.apply_commit(&Commit {
        ops: vec![op(
            SET_STYLE,
            &[n(a), "backgroundColor".into(), Value::Null],
        )],
    });
    assert!(errors.is_empty());
    tree.layout_if_needed();
    assert_eq!(bg(&tree, a), GREEN);
}

// M3-CSS-04: inherited properties from stylesheets flow down to text.
#[test]
fn stylesheet_text_properties_inherit() {
    let mut d = Dom::new();
    let panel = d.class(0, "panel");
    let label = d.el(panel, "text", &[]);
    d.text(label, "hello");
    let tree = d.build(".panel { color: blue; font-size: 24px }");
    let computed = tree.computed_text(NodeId(label));
    assert_eq!(computed.color, BLUE);
    assert_eq!(computed.font_size, 24.0);
    assert!(
        tree.rect(NodeId(label)).unwrap().height > 24.0,
        "re-laid out at the new size"
    );
}

#[test]
fn adding_and_removing_sheets_restyles() {
    let mut d = Dom::new();
    let a = d.class(0, "a");
    let mut tree = d.build(".a { background-color: red }");
    let second = tree.add_stylesheet(".a { background-color: blue }");
    tree.layout_if_needed();
    assert_eq!(bg(&tree, a), BLUE, "later sheets win");
    assert!(tree.remove_stylesheet(second.id));
    tree.layout_if_needed();
    assert_eq!(bg(&tree, a), RED);

    let warned = tree.add_stylesheet(".a { colour: red; width: 10px; background-color: nope }");
    assert_eq!(warned.warnings.len(), 2, "{:?}", warned.warnings);
}
