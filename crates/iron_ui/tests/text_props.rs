//! Text properties from M3d group 4: `text-transform`, `text-overflow`, wrapping,
//! fonts, decorations and strokes.

mod common;

use common::{Dom, px};
use iron_core::Value;
use iron_ui::{DrawCmd, TextCmd, UiTree};

/// The text command of the only text block in the tree.
fn text_cmd(tree: &mut UiTree) -> TextCmd {
    tree.draw_list()
        .into_iter()
        .find_map(|c| match c {
            DrawCmd::Text(t) => Some(t),
            _ => None,
        })
        .expect("a text command")
}

fn glyph_ids(cmd: &TextCmd) -> Vec<u32> {
    cmd.runs
        .iter()
        .flat_map(|r| r.glyphs.iter().map(|g| g.id))
        .collect()
}

/// A text element with `style` holding `text`.
fn label(text: &str, style: &[(&str, Value)]) -> UiTree {
    let mut d = Dom::new();
    let t = d.add(0, "text", style);
    d.text(t, text);
    d.build("")
}

// M3-TXT-10
#[test]
fn text_transform_changes_the_shaped_text() {
    let upper = glyph_ids(&text_cmd(&mut label(
        "Hello world",
        &[("textTransform", "uppercase".into())],
    )));
    assert_eq!(upper, glyph_ids(&text_cmd(&mut label("HELLO WORLD", &[]))));
    let caps = glyph_ids(&text_cmd(&mut label(
        "hello wORLD",
        &[("textTransform", "capitalize".into())],
    )));
    assert_eq!(caps, glyph_ids(&text_cmd(&mut label("Hello WORLD", &[]))));
}

// M3-TXT-10
#[test]
fn ellipsis_cuts_overflowing_lines_where_overflow_clips() {
    let style = |overflow: &str| {
        vec![
            ("width", px(80.0)),
            ("whiteSpace", "nowrap".into()),
            ("textOverflow", "ellipsis".into()),
            ("overflow", overflow.into()),
        ]
    };
    let text = "The quick brown fox jumps";
    let full = text_cmd(&mut label(text, &style("visible")));
    let cut = text_cmd(&mut label(text, &style("hidden")));
    let glyphs: Vec<_> = cut.runs.iter().flat_map(|r| &r.glyphs).collect();
    let right = glyphs
        .iter()
        .map(|g| g.x + g.advance)
        .fold(f32::MIN, f32::max);
    assert!(right <= 80.5, "everything fits in the box: {right}");
    assert!(
        glyphs.len() < glyph_ids(&full).len(),
        "visible overflow isn't cut"
    );
    // The last glyph is the ellipsis, shaped on its own.
    let dots = glyph_ids(&text_cmd(&mut label("\u{2026}", &[])));
    assert_eq!(cut.runs.last().unwrap().glyphs.len(), dots.len());
    assert_eq!(glyphs.last().unwrap().id, dots[0]);

    // Text that fits is left alone.
    let short = text_cmd(&mut label("Fox", &style("hidden")));
    assert_eq!(
        glyph_ids(&short),
        glyph_ids(&text_cmd(&mut label("Fox", &[])))
    );
}

// M3-TXT-10
#[test]
fn rtl_ellipsis_goes_on_the_left() {
    let cut = text_cmd(&mut label(
        "The quick brown fox jumps",
        &[
            ("width", px(80.0)),
            ("direction", "rtl".into()),
            ("whiteSpace", "nowrap".into()),
            ("textOverflow", "ellipsis".into()),
            ("overflow", "hidden".into()),
        ],
    ));
    let glyphs: Vec<_> = cut.runs.iter().flat_map(|r| &r.glyphs).collect();
    let left = glyphs.iter().map(|g| g.x).fold(f32::MAX, f32::min);
    assert!(left >= -0.5, "nothing left of the box: {left}");
    let dots = cut.runs.last().unwrap().glyphs[0].x;
    assert!(
        glyphs.iter().all(|g| g.x >= dots - 0.01),
        "the ellipsis is leftmost"
    );
}
