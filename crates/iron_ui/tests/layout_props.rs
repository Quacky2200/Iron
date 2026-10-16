//! Layout properties from M3d group 2: `place-*` shorthands, grid extras
//! (`grid-template-areas`, `grid-area`, `grid-template`, auto tracks, line
//! longhands) and logical properties.

mod common;

use common::{Dom, px};
use iron_core::{NodeId, Value};
use iron_ui::{Rect, UiTree};

fn rect(tree: &mut UiTree, id: u32) -> Rect {
    tree.layout_if_needed();
    tree.rect(NodeId(id)).unwrap()
}

/// A 300x200 grid container with `container` styles and one child per entry of
/// `children`.
fn grid(container: &[(&str, Value)], children: &[&[(&str, Value)]]) -> (UiTree, Vec<u32>) {
    let mut d = Dom::new();
    let mut style = vec![
        ("display", Value::from("grid")),
        ("width", px(300.0)),
        ("height", px(200.0)),
    ];
    style.extend_from_slice(container);
    let g = d.add(0, "box", &style);
    let kids = children.iter().map(|s| d.add(g, "box", s)).collect();
    (d.build(""), kids)
}

#[test]
fn template_areas_and_grid_area_place_items_by_name() {
    let (mut tree, k) = grid(
        &[
            ("gridTemplateAreas", "\"head head\" \"side main\"".into()),
            ("gridTemplateColumns", "100px 1fr".into()),
            ("gridTemplateRows", "50px 1fr".into()),
        ],
        &[
            &[("gridArea", "main".into())],
            &[("gridArea", "head".into())],
            &[("gridArea", "side".into())],
        ],
    );
    assert_eq!(
        rect(&mut tree, k[0]),
        Rect::new(100.0, 50.0, 200.0, 150.0),
        "main"
    );
    assert_eq!(
        rect(&mut tree, k[1]),
        Rect::new(0.0, 0.0, 300.0, 50.0),
        "head spans both columns"
    );
    assert_eq!(
        rect(&mut tree, k[2]),
        Rect::new(0.0, 50.0, 100.0, 150.0),
        "side"
    );
}

#[test]
fn grid_template_shorthand_with_areas_and_sizes() {
    let (mut tree, k) = grid(
        &[("gridTemplate", "\"a b\" 40px \"c c\" 1fr / 1fr 2fr".into())],
        &[&[("gridArea", "b".into())], &[("gridArea", "c".into())]],
    );
    assert_eq!(rect(&mut tree, k[0]), Rect::new(100.0, 0.0, 200.0, 40.0));
    assert_eq!(rect(&mut tree, k[1]), Rect::new(0.0, 40.0, 300.0, 160.0));

    let (mut tree, k) = grid(
        &[("gridTemplate", "60px 1fr / 1fr 1fr 1fr".into())],
        &[&[], &[], &[], &[]],
    );
    assert_eq!(
        rect(&mut tree, k[3]),
        Rect::new(0.0, 60.0, 100.0, 140.0),
        "4th item wraps to row 2"
    );
}

#[test]
fn grid_area_with_lines_and_line_longhands() {
    let (mut tree, k) = grid(
        &[
            ("gridTemplateColumns", "repeat(3, 100px)".into()),
            ("gridTemplateRows", "repeat(2, 100px)".into()),
        ],
        &[
            &[("gridArea", "2 / 1 / 3 / 3".into())],
            &[
                ("gridColumnStart", "3".into()),
                ("gridRowStart", "1".into()),
                ("gridRowEnd", "span 2".into()),
            ],
        ],
    );
    assert_eq!(rect(&mut tree, k[0]), Rect::new(0.0, 100.0, 200.0, 100.0));
    assert_eq!(rect(&mut tree, k[1]), Rect::new(200.0, 0.0, 100.0, 200.0));
}

#[test]
fn auto_tracks_size_implicit_rows() {
    let (mut tree, k) = grid(
        &[
            ("gridTemplateColumns", "1fr 1fr".into()),
            ("gridAutoRows", "30px".into()),
            ("alignContent", "start".into()),
        ],
        &[&[], &[], &[]],
    );
    assert_eq!(rect(&mut tree, k[2]).y, 30.0);
    assert_eq!(rect(&mut tree, k[2]).height, 30.0);
}

#[test]
fn place_shorthands_set_both_axes() {
    let (mut tree, k) = grid(
        &[
            ("placeItems", "center".into()),
            ("gridTemplateColumns", "1fr".into()),
        ],
        &[&[("width", px(50.0)), ("height", px(20.0))]],
    );
    assert_eq!(rect(&mut tree, k[0]), Rect::new(125.0, 90.0, 50.0, 20.0));

    let (mut tree, k) = grid(
        &[],
        &[&[
            ("width", px(50.0)),
            ("height", px(20.0)),
            ("placeSelf", "end start".into()),
        ]],
    );
    assert_eq!(
        rect(&mut tree, k[0]),
        Rect::new(0.0, 180.0, 50.0, 20.0),
        "align end, justify start"
    );
}

#[test]
fn bad_grid_values_are_rejected() {
    let mut d = Dom::new();
    let a = d.add(0, "box", &[]);
    let mut tree = d.build("");
    for (prop, value) in [
        ("gridTemplateAreas", "\"a b\" \"b a\""),
        ("gridTemplateAreas", "\"a a\" \"b\""),
        ("gridArea", "1 / 2 / 3 / 4 / 5"),
        ("placeItems", "center start end"),
        ("marginInline", "1px 2px 3px"),
    ] {
        let errors = tree.apply_commit(&iron_core::Commit {
            ops: vec![common::op(
                iron_ui::opcodes::SET_STYLE,
                &[common::n(a), prop.into(), value.into()],
            )],
        });
        assert!(!errors.is_empty(), "{prop}: {value} should be rejected");
    }
}

#[test]
fn logical_properties_map_left_to_right() {
    let mut d = Dom::new();
    let col = d.add(
        0,
        "box",
        &[
            ("flexDirection", "column".into()),
            ("alignItems", "flex-start".into()),
        ],
    );
    let a = d.add(
        col,
        "box",
        &[
            ("inlineSize", px(80.0)),
            ("blockSize", px(20.0)),
            ("marginBlock", "5px 7px".into()),
            ("marginInlineStart", px(11.0)),
            ("paddingInline", px(4.0)),
        ],
    );
    let b = d.add(
        col,
        "box",
        &[("inlineSize", px(10.0)), ("blockSize", px(10.0))],
    );
    let mut tree = d.build("");
    assert_eq!(rect(&mut tree, a), Rect::new(11.0, 5.0, 80.0, 20.0));
    assert_eq!(rect(&mut tree, b).y, 32.0, "5 + 20 + 7");
}
