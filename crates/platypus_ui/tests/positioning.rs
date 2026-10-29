// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

//! Containing blocks of absolutely positioned boxes and flex sizing inside `overflow: hidden`
//! clips, checked against CSS (the cases the Fluid desktop reported, `examples/paint-repro`).

mod common;

use common::{Dom, px};
use platypus_core::{NodeId, Value};
use platypus_ui::{Rect, UiTree};

fn rect(tree: &mut UiTree, id: u32) -> Rect {
    tree.layout_if_needed();
    tree.rect(NodeId(id)).unwrap()
}

/// Fluid's menu bar: a fixed 134x40 relative box > an absolute bar (left/top/right 0, height
/// 28, a centred row with 10px side padding) > a row group > a title (height 22, 9px side
/// padding, `position` set to `title`) > an absolute 8x8 box with `insets`. Returns the tree, the
/// bar, the title and the 8x8 box.
fn menu_bar(title: &str, insets: &[(&str, Value)]) -> (UiTree, u32, u32, u32) {
    let mut d = Dom::new();
    let outer = d.add(
        0,
        "box",
        &[
            ("position", "relative".into()),
            ("width", px(134.0)),
            ("height", px(40.0)),
        ],
    );
    let bar = d.add(
        outer,
        "box",
        &[
            ("position", "absolute".into()),
            ("left", px(0.0)),
            ("top", px(0.0)),
            ("right", px(0.0)),
            ("height", px(28.0)),
            ("flexDirection", "row".into()),
            ("justifyContent", "space-between".into()),
            ("alignItems", "center".into()),
            ("padding", "0 10px".into()),
        ],
    );
    let group = d.add(
        bar,
        "box",
        &[
            ("flexDirection", "row".into()),
            ("alignItems", "center".into()),
        ],
    );
    let item = d.add(
        group,
        "box",
        &[
            ("position", title.into()),
            ("height", px(22.0)),
            ("padding", "0 9px".into()),
            ("flexDirection", "row".into()),
            ("alignItems", "center".into()),
        ],
    );
    d.add(item, "box", &[("width", px(20.0)), ("height", px(10.0))]);
    let mut style = vec![
        ("position", Value::from("absolute")),
        ("width", px(8.0)),
        ("height", px(8.0)),
    ];
    style.extend_from_slice(insets);
    let ear = d.add(item, "box", &style);
    (d.build(""), bar, item, ear)
}

fn at(r: Rect) -> (f32, f32) {
    (r.x, r.y)
}

// M2-LAY-08
#[test]
fn an_absolute_box_in_a_static_parent_uses_the_nearest_positioned_ancestor() {
    // The title has no position, so the containing block is the absolute bar: `left: 100%`
    // is the bar's right edge (a percentage of its padding box) and `bottom: 0` its bottom.
    let (mut t, bar, _, ear) = menu_bar("static", &[("left", "100%".into()), ("bottom", px(0.0))]);
    let b = rect(&mut t, bar);
    assert_eq!(b, Rect::new(0.0, 0.0, 134.0, 28.0));
    assert_eq!(rect(&mut t, ear), Rect::new(134.0, 20.0, 8.0, 8.0));

    // With no insets it stays at its static position: the start of its parent's content box,
    // wherever the containing block is (here the root).
    let mut d = Dom::new();
    let parent = d.add(
        0,
        "box",
        &[
            ("marginLeft", px(50.0)),
            ("marginTop", px(30.0)),
            ("padding", px(5.0)),
            ("width", px(100.0)),
            ("height", px(100.0)),
        ],
    );
    let child = d.add(
        parent,
        "box",
        &[
            ("position", "absolute".into()),
            ("width", px(8.0)),
            ("height", px(8.0)),
        ],
    );
    let mut t = d.build("");
    assert_eq!(rect(&mut t, child), Rect::new(55.0, 35.0, 8.0, 8.0));
}

/// Insets, and where they put the 8x8 box's top-left corner given the title's rectangle.
type Placement<'a> = (&'a [(&'a str, Value)], fn(Rect) -> (f32, f32));

// M2-LAY-09
#[test]
fn insets_resolve_against_a_relative_title_in_a_centred_row() {
    let cases: [Placement; 4] = [
        (&[("left", "100%".into()), ("bottom", px(0.0))], |r| {
            (r.x + r.width, r.y + r.height - 8.0)
        }),
        (&[("right", px(-8.0)), ("top", px(0.0))], |r| {
            (r.x + r.width, r.y)
        }),
        (&[("right", px(0.0)), ("bottom", px(0.0))], |r| {
            (r.x + r.width - 8.0, r.y + r.height - 8.0)
        }),
        (&[("left", px(0.0)), ("top", px(0.0))], at),
    ];
    for (insets, want) in cases {
        let (mut t, _, item, ear) = menu_bar("relative", insets);
        let title = rect(&mut t, item);
        // The title: 10px into the bar, centred in its 28px height, 9 + 20 + 9 wide.
        assert_eq!(title, Rect::new(10.0, 3.0, 38.0, 22.0));
        assert_eq!(at(rect(&mut t, ear)), want(title), "{insets:?}");
    }
}

/// A 48x48 `overflow: hidden` box holding a 96x96 child (styles `child`), the clip styled
/// with `clip` as well. Returns the child's rectangle.
fn clipped_child(clip: &[(&str, Value)], child: &[(&str, Value)]) -> Rect {
    let mut d = Dom::new();
    let mut style = vec![
        ("width", px(48.0)),
        ("height", px(48.0)),
        ("overflow", Value::from("hidden")),
    ];
    style.extend_from_slice(clip);
    let c = d.add(0, "box", &style);
    let mut style = vec![("width", px(96.0)), ("height", px(96.0))];
    style.extend_from_slice(child);
    let k = d.add(c, "box", &style);
    rect(&mut d.build(""), k)
}

// M2-LAY-10
#[test]
fn an_empty_flex_item_wider_than_its_clip_shrinks_like_css() {
    // Boxes are flex rows by default: an empty item's automatic minimum width is 0, so it
    // shrinks to the clip's width, as in a browser with `display: flex` (its height stays).
    assert_eq!(clipped_child(&[], &[]), Rect::new(0.0, 0.0, 48.0, 96.0));
    // Without shrinking, or in a block container, it keeps its size.
    assert_eq!(
        clipped_child(&[], &[("flexShrink", px(0.0))]),
        Rect::new(0.0, 0.0, 96.0, 96.0)
    );
    assert_eq!(
        clipped_child(&[("display", "block".into())], &[]),
        Rect::new(0.0, 0.0, 96.0, 96.0)
    );
}
