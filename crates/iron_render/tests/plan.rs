// SPDX-License-Identifier: Apache-2.0

//! The backend-neutral frame plan (`iron_render::plan`): draw lists from the real `UiTree`,
//! planned with fake resources: no GPU, no wgpu device, the same plan any renderer executes
//! (M2-PLN-01..06).

use std::collections::HashMap;

use iron_core::{Commit, Value};
use iron_render::Color;
use iron_render::glyphs::{GlyphCache, GlyphEntry};
use iron_render::plan::{
    FramePlan, ImageInfo, KIND_GLYPH, PlanParams, PlanResources, Step, needs_backdrop, plan,
};
use iron_ui::opcodes::*;
use iron_ui::{DrawCmd, UiTree};

/// Resources with no GPU behind them: a real glyph cache (swash and packing run; the pixels
/// are queued, nobody writes them), images of fixed sizes, no SVG.
#[derive(Default)]
struct Fake {
    glyphs: GlyphCache,
    images: HashMap<String, (u32, u32)>,
    glyph_calls: u32,
}

impl PlanResources for Fake {
    fn glyph(
        &mut self,
        run: &iron_ui::GlyphRun,
        glyph: u32,
        size_px: f32,
        subpixel: u8,
        stroke_px: f32,
    ) -> Option<GlyphEntry> {
        self.glyph_calls += 1;
        self.glyphs.glyph(run, glyph, size_px, subpixel, stroke_px)
    }

    fn image(&mut self, url: &str) -> Option<ImageInfo> {
        self.images.get(url).map(|&size| ImageInfo {
            size,
            key: url.to_owned(),
        })
    }

    fn svg_at(&mut self, _: &str, _: (f32, f32)) -> Option<(String, (u32, u32))> {
        None
    }
}

fn n(v: u32) -> Value {
    Value::Number(v.into())
}

/// An element: its parent, its kind and its style pairs.
type Element<'a> = (u32, &'a str, &'a [(&'a str, &'a str)]);

/// A tree of `elements` (parent, kind, style pairs; ids from 1 in order) plus text nodes.
fn tree(elements: &[Element], texts: &[(u32, &str)], css: &str) -> UiTree {
    let mut ops = Vec::new();
    let mut id = 0;
    for (parent, kind, style) in elements {
        id += 1;
        ops.push(Value::Array(vec![n(CREATE), n(id), (*kind).into()]));
        let mut set = vec![n(SET_STYLE), n(id)];
        for (k, v) in *style {
            set.push((*k).into());
            set.push((*v).into());
        }
        ops.push(Value::Array(set));
        ops.push(Value::Array(vec![n(APPEND_CHILD), n(*parent), n(id)]));
    }
    for (parent, text) in texts {
        id += 1;
        ops.push(Value::Array(vec![n(CREATE_TEXT), n(id), (*text).into()]));
        ops.push(Value::Array(vec![n(APPEND_CHILD), n(*parent), n(id)]));
    }
    let mut t = UiTree::new(200.0, 100.0);
    let errors = t.apply_commit(&Commit { ops });
    assert!(errors.is_empty(), "{errors:?}");
    if !css.is_empty() {
        let added = t.add_stylesheet(css);
        assert!(added.warnings.is_empty(), "{:?}", added.warnings);
    }
    t.layout_if_needed();
    t
}

const CLEAR: Color = Color::rgb(0.1, 0.2, 0.3);

fn plan_of(cmds: &[DrawCmd], seeded: bool, res: &mut Fake) -> FramePlan {
    plan(
        cmds,
        &PlanParams {
            size: (400, 200),
            scale: 2.0,
            clear: CLEAR,
            seeded,
        },
        res,
    )
}

// M2-PLN-01
#[test]
fn boxes_become_one_batch_over_a_cleared_canvas() {
    let mut t = tree(
        &[
            (
                0,
                "box",
                &[
                    ("width", "50"),
                    ("height", "20"),
                    ("backgroundColor", "red"),
                ],
            ),
            (
                0,
                "box",
                &[
                    ("width", "50"),
                    ("height", "20"),
                    ("backgroundColor", "blue"),
                ],
            ),
        ],
        &[],
        "",
    );
    let p = plan_of(&t.draw_list(), false, &mut Fake::default());
    assert_eq!(p.instances.len(), 2);
    assert_eq!((p.textures, p.layers, p.clips.len()), (1, 0, 0));
    assert!(matches!(&p.steps[0], Step::Clear { target: 0, color } if *color == CLEAR));
    assert!(
        matches!(&p.steps[1], Step::Draw { target: 0, range, image: None } if *range == (0..2))
    );
    assert_eq!(p.steps.len(), 2);
}

// M2-PLN-02
#[test]
fn a_seeded_canvas_is_not_cleared() {
    let mut t = tree(
        &[(
            0,
            "box",
            &[
                ("width", "50"),
                ("height", "20"),
                ("backgroundColor", "red"),
            ],
        )],
        &[],
        "",
    );
    let p = plan_of(&t.draw_list(), true, &mut Fake::default());
    assert!(!p.steps.iter().any(|s| matches!(s, Step::Clear { .. })));
    assert!(matches!(&p.steps[0], Step::Draw { .. }));
}

// M2-PLN-03
#[test]
fn a_blurred_layer_gets_its_textures_and_passes() {
    let mut t = tree(
        &[(
            0,
            "box",
            &[
                ("width", "50"),
                ("height", "20"),
                ("backgroundColor", "red"),
                ("filter", "blur(4px)"),
                ("opacity", "0.5"),
            ],
        )],
        &[],
        "",
    );
    let p = plan_of(&t.draw_list(), false, &mut Fake::default());
    assert_eq!(p.layers, 1);
    // The canvas, the layer, and a scratch texture for the blur.
    assert_eq!(p.textures, 3);
    let blur = p.steps.iter().find_map(|s| match s {
        Step::Blur { sigma, .. } => Some(*sigma),
        _ => None,
    });
    assert_eq!(blur, Some(8.0), "4 logical px at scale 2");
    let composite = p.steps.iter().find_map(|s| match s {
        Step::Composite {
            src, dst, opacity, ..
        } => Some((*src, *dst, *opacity)),
        _ => None,
    });
    assert_eq!(
        composite,
        Some((1, 0, 0.5)),
        "the layer goes onto the canvas at its opacity"
    );
}

// M2-PLN-04
#[test]
fn a_clip_goes_in_the_table_and_instances_carry_its_index() {
    let mut t = tree(
        &[
            (
                0,
                "box",
                &[("width", "40"), ("height", "40"), ("overflow", "hidden")],
            ),
            (
                1,
                "box",
                &[
                    ("width", "100"),
                    ("height", "10"),
                    ("backgroundColor", "red"),
                ],
            ),
        ],
        &[],
        "",
    );
    let p = plan_of(&t.draw_list(), false, &mut Fake::default());
    assert_eq!(p.clips.len(), 1);
    assert_eq!(p.instances.len(), 1);
    assert_eq!(p.instances[0].params[3], 1.0, "clip index 1 (0 is none)");
}

// M2-PLN-05
#[test]
fn text_places_one_glyph_instance_per_visible_glyph_and_queues_its_pixels() {
    let mut t = tree(
        &[(0, "text", &[("color", "white"), ("fontSize", "16")])],
        &[(1, "Hi!")],
        "",
    );
    let cmds = t.draw_list();
    let mut res = Fake::default();
    let p = plan_of(&cmds, false, &mut res);
    let glyphs = p
        .instances
        .iter()
        .filter(|i| i.params[0] == KIND_GLYPH)
        .count();
    assert_eq!(glyphs, 3, "H, i and !");
    assert!(res.glyph_calls >= 3);
    // Each glyph's pixels wait in the cache for whichever renderer owns the page.
    let uploads = res.glyphs.take_uploads();
    assert_eq!(uploads.len(), 3);
    assert!(
        uploads
            .iter()
            .all(|u| !u.color && u.data.len() == (u.width * u.height) as usize)
    );
    // Asking again uses the cache: nothing new to upload.
    let _ = plan_of(&cmds, false, &mut res);
    assert!(res.glyphs.take_uploads().is_empty());
}

// M2-PLN-06
#[test]
fn an_image_fill_draws_with_its_texture_and_an_unknown_image_is_skipped() {
    let css = ".a { background-image: url(a.png); background-size: 10px 10px; } .b { background-image: url(b.png); }";
    let mut ops = Vec::new();
    for (id, class) in [(1u32, "a"), (2, "b")] {
        ops.push(Value::Array(vec![n(CREATE), n(id), "box".into()]));
        ops.push(Value::Array(vec![
            n(SET_ATTRS),
            n(id),
            "className".into(),
            class.into(),
        ]));
        ops.push(Value::Array(vec![
            n(SET_STYLE),
            n(id),
            "width".into(),
            "30".into(),
            "height".into(),
            "20".into(),
        ]));
        ops.push(Value::Array(vec![n(APPEND_CHILD), n(0), n(id)]));
    }
    let mut t = UiTree::new(200.0, 100.0);
    assert!(t.apply_commit(&Commit { ops }).is_empty());
    t.add_stylesheet(css);
    t.layout_if_needed();
    let mut res = Fake::default();
    res.images.insert("a.png".into(), (20, 20));
    let p = plan_of(&t.draw_list(), false, &mut res);
    let drawn: Vec<&str> = p
        .steps
        .iter()
        .filter_map(|s| match s {
            Step::Draw {
                image: Some((url, _)),
                ..
            } => Some(url.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(drawn, ["a.png"], "b.png has no size, so it is skipped");
}

// M2-PLN-07
#[test]
fn backdrop_effects_are_found_without_planning() {
    let mut t = tree(
        &[(
            0,
            "box",
            &[
                ("width", "50"),
                ("height", "20"),
                ("backdropFilter", "blur(4px)"),
            ],
        )],
        &[],
        "",
    );
    assert!(needs_backdrop(&t.draw_list()));
    let mut plain = tree(&[(0, "box", &[("width", "50"), ("height", "20")])], &[], "");
    assert!(!needs_backdrop(&plain.draw_list()));
}
