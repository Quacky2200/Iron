// SPDX-License-Identifier: Apache-2.0

//! Flow layout drawn by the UI renderer (M3-FLT-20..): floats with text wrapping around
//! them, checked by golden images (and by eye when they change).

use iron_core::{Commit, Value};
use iron_render::testing::{assert_golden, shared_gpu};
use iron_render::{Color, OffscreenTarget, RenderTarget, UiRenderer, wgpu};
use iron_ui::UiTree;
use iron_ui::opcodes::*;

const TOLERANCE: u8 = 3;

const LOREM: &str = "The quick brown fox jumps over the lazy dog while the five boxing wizards \
    jump quickly. Pack my box with five dozen liquor jugs, and sphinx of black quartz, judge my \
    vow. How vexingly quick daft zebras jump over a field of bright flowers in the summer sun.";

#[derive(Default)]
struct Scene {
    ops: Vec<Value>,
    next: u32,
}

type Style<'a> = &'a [(&'a str, Value)];

impl Scene {
    fn new() -> Self {
        Self {
            ops: Vec::new(),
            next: 1,
        }
    }

    fn element(&mut self, parent: u32, kind: &str, style: Style) -> u32 {
        let id = self.next;
        self.next += 1;
        let n = |v: u32| Value::Number(v.into());
        self.ops
            .push(Value::Array(vec![n(CREATE), n(id), kind.into()]));
        let mut set = vec![n(SET_STYLE), n(id)];
        for (k, v) in style {
            set.push((*k).into());
            set.push(v.clone());
        }
        self.ops.push(Value::Array(set));
        self.ops
            .push(Value::Array(vec![n(APPEND_CHILD), n(parent), n(id)]));
        id
    }

    fn text(&mut self, parent: u32, text: &str) {
        let id = self.next;
        self.next += 1;
        let n = |v: u32| Value::Number(v.into());
        self.ops
            .push(Value::Array(vec![n(CREATE_TEXT), n(id), text.into()]));
        self.ops
            .push(Value::Array(vec![n(APPEND_CHILD), n(parent), n(id)]));
    }

    fn label(&mut self, parent: u32, text: &str, style: Style) {
        let id = self.element(parent, "text", style);
        self.text(id, text);
    }
}

fn px(v: f64) -> Value {
    Value::Number(v)
}

fn render(scene: Scene, size: (u32, u32)) -> Vec<u8> {
    let gpu = shared_gpu();
    let mut tree = UiTree::new(size.0 as f32, size.1 as f32);
    let errors = tree.apply_commit(&Commit { ops: scene.ops });
    assert!(errors.is_empty(), "{errors:?}");
    tree.layout_if_needed();
    let target =
        OffscreenTarget::with_format(&gpu, size.0, size.1, wgpu::TextureFormat::Rgba8UnormSrgb);
    let mut ui = UiRenderer::new(&gpu);
    let cmds = tree.draw_list();
    let view = target.view().unwrap();
    ui.render(
        &gpu,
        &RenderTarget {
            view: &view,
            format: target.format(),
            size,
            scale: 1.0,
        },
        Color::rgb(0.0, 0.0, 0.0),
        &cmds,
    );
    target.read_pixels(&gpu).unwrap()
}

// M3-FLT-20
#[test]
fn text_wraps_around_left_and_right_floats() {
    let mut s = Scene::new();
    let page = s.element(
        0,
        "box",
        &[
            ("display", "block".into()),
            ("width", px(380.0)),
            ("padding", px(10.0)),
            ("backgroundColor", "#f4f1ea".into()),
        ],
    );
    s.label(
        page,
        "Floats",
        &[
            ("color", "#222".into()),
            ("fontSize", px(22.0)),
            ("fontWeight", "bold".into()),
            ("marginBottom", px(6.0)),
        ],
    );
    // A picture on the left and a pull-quote on the right; the paragraph flows between and
    // under them.
    s.element(
        page,
        "box",
        &[
            ("float", "left".into()),
            ("width", px(90.0)),
            ("height", px(110.0)),
            ("margin", "0 12px 6px 0".into()),
            ("backgroundColor", "#3a7ca5".into()),
            ("borderRadius", px(8.0)),
        ],
    );
    let quote = s.element(
        page,
        "box",
        &[
            ("float", "right".into()),
            ("display", "block".into()),
            ("width", px(100.0)),
            ("margin", "0 0 6px 12px".into()),
            ("padding", px(8.0)),
            ("backgroundColor", "#d9a441".into()),
            ("borderRadius", px(6.0)),
        ],
    );
    s.label(
        quote,
        "\"Jump quickly.\"",
        &[("color", "#222".into()), ("fontSize", px(14.0))],
    );
    s.label(
        page,
        LOREM,
        &[
            ("color", "#333".into()),
            ("fontSize", px(14.0)),
            ("lineHeight", px(1.4)),
        ],
    );
    s.element(
        page,
        "box",
        &[
            ("clear", "both".into()),
            ("height", px(14.0)),
            ("marginTop", px(8.0)),
            ("backgroundColor", "#7a9e7e".into()),
        ],
    );
    let pixels = render(s, (400, 300));
    assert_golden("M3-FLT-20", 400, 300, &pixels, TOLERANCE);
}
