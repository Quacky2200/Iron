//! The GLES executor against the wgpu one (M2-GLES-02..06): the same scenes, built through the
//! real `UiTree`, drawn by both renderers into `SRGB8_ALPHA8` targets and compared pixel for
//! pixel. Needs an EGL driver (ANGLE's folder in `IRON_ANGLE_DIR`) and a GPU for wgpu; skipped
//! without an EGL driver.

use std::path::PathBuf;

use glow::HasContext;
use iron_core::{Commit, Value};
use iron_render::testing::shared_gpu;
use iron_render::{Color, OffscreenTarget, RenderTarget, UiRenderer, wgpu};
use iron_render_gles::egl::EglContext;
use iron_render_gles::{GlesRenderer, Target};
use iron_ui::UiTree;
use iron_ui::opcodes::*;

const CLEAR: Color = Color::rgb(0.02, 0.03, 0.05);

fn n(v: u32) -> Value {
    Value::Number(v.into())
}

struct Scene {
    ops: Vec<Value>,
    next: u32,
}

impl Scene {
    fn new() -> Self {
        Self {
            ops: Vec::new(),
            next: 1,
        }
    }

    /// An absolutely placed element of `kind` under `parent` (0 = the root).
    fn at(&mut self, parent: u32, kind: &str, rect: [f64; 4], style: &[(&str, &str)]) -> u32 {
        let id = self.next;
        self.next += 1;
        self.ops
            .push(Value::Array(vec![n(CREATE), n(id), kind.into()]));
        let mut set = vec![n(SET_STYLE), n(id)];
        for (k, v) in [
            ("position", Value::from("absolute")),
            ("left", rect[0].into()),
            ("top", rect[1].into()),
            ("width", rect[2].into()),
            ("height", rect[3].into()),
        ] {
            set.push(k.into());
            set.push(v);
        }
        for (k, v) in style {
            set.push((*k).into());
            set.push((*v).into());
        }
        self.ops.push(Value::Array(set));
        self.ops
            .push(Value::Array(vec![n(APPEND_CHILD), n(parent), n(id)]));
        id
    }

    fn text(&mut self, parent: u32, text: &str) {
        let id = self.next;
        self.next += 1;
        self.ops
            .push(Value::Array(vec![n(CREATE_TEXT), n(id), text.into()]));
        self.ops
            .push(Value::Array(vec![n(APPEND_CHILD), n(parent), n(id)]));
    }

    fn cmds(self, size: (f32, f32)) -> Vec<iron_ui::DrawCmd> {
        let mut t = UiTree::new(size.0, size.1);
        let errors = t.apply_commit(&Commit { ops: self.ops });
        assert!(errors.is_empty(), "{errors:?}");
        t.layout_if_needed();
        t.draw_list()
    }
}

struct Gles {
    // Declared first: the renderer goes before the context it draws with.
    renderer: GlesRenderer,
    gl: std::rc::Rc<glow::Context>,
    _cx: EglContext,
}

fn gles() -> Option<Gles> {
    let dir = std::env::var_os("IRON_ANGLE_DIR").map(PathBuf::from);
    let cx = match EglContext::headless(dir.as_deref()) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("skipped: {e}");
            return None;
        }
    };
    let renderer = GlesRenderer::new(cx.gl()).expect("renderer");
    Some(Gles {
        renderer,
        gl: std::rc::Rc::new(cx.gl()),
        _cx: cx,
    })
}

/// The wgpu renderer's pixels (RGBA8, top row first).
fn wgpu_pixels(cmds: &[iron_ui::DrawCmd], size: (u32, u32), scale: f32) -> Vec<u8> {
    let gpu = shared_gpu();
    let target =
        OffscreenTarget::with_format(&gpu, size.0, size.1, wgpu::TextureFormat::Rgba8UnormSrgb);
    let mut ui = UiRenderer::new(&gpu);
    let view = target.view().unwrap();
    ui.render(
        &gpu,
        &RenderTarget {
            view: &view,
            format: target.format(),
            size,
            scale,
        },
        CLEAR,
        cmds,
    );
    target.read_pixels(&gpu).unwrap()
}

/// The GLES renderer's pixels, top row first.
fn gles_pixels(g: &mut Gles, cmds: &[iron_ui::DrawCmd], size: (u32, u32), scale: f32) -> Vec<u8> {
    let gl = &g.gl;
    unsafe {
        let texture = gl.create_texture().unwrap();
        gl.bind_texture(glow::TEXTURE_2D, Some(texture));
        gl.tex_storage_2d(
            glow::TEXTURE_2D,
            1,
            glow::SRGB8_ALPHA8,
            size.0 as i32,
            size.1 as i32,
        );
        let fbo = gl.create_framebuffer().unwrap();
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(fbo));
        gl.framebuffer_texture_2d(
            glow::FRAMEBUFFER,
            glow::COLOR_ATTACHMENT0,
            glow::TEXTURE_2D,
            Some(texture),
            0,
        );
        g.renderer.render(
            &Target {
                framebuffer: Some(fbo),
                size,
                scale,
                srgb: true,
            },
            CLEAR,
            cmds,
        );
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(fbo));
        let mut bottom_up = vec![0u8; (size.0 * size.1 * 4) as usize];
        gl.pixel_store_i32(glow::PACK_ALIGNMENT, 1);
        gl.read_pixels(
            0,
            0,
            size.0 as i32,
            size.1 as i32,
            glow::RGBA,
            glow::UNSIGNED_BYTE,
            glow::PixelPackData::Slice(Some(&mut bottom_up)),
        );
        let row = (size.0 * 4) as usize;
        bottom_up.chunks(row).rev().flatten().copied().collect()
    }
}

struct Diff {
    max: u8,
    /// Pixels whose channels differ by more than the tolerance.
    off: usize,
    pixels: usize,
}

fn diff(a: &[u8], b: &[u8], tolerance: u8) -> Diff {
    assert_eq!(a.len(), b.len());
    let mut d = Diff {
        max: 0,
        off: 0,
        pixels: a.len() / 4,
    };
    for (pa, pb) in a.chunks(4).zip(b.chunks(4)) {
        let m = pa
            .iter()
            .zip(pb)
            .map(|(x, y)| x.abs_diff(*y))
            .max()
            .unwrap();
        d.max = d.max.max(m);
        if m > tolerance {
            d.off += 1;
        }
    }
    d
}

/// Draws `scene` with both renderers and requires the pixels to agree: at most `allowed_off`
/// pixels may differ by more than `tolerance`.
fn same(
    name: &str,
    scene: Scene,
    logical: (u32, u32),
    scale: f32,
    tolerance: u8,
    allowed_off: f32,
) {
    let Some(mut g) = gles() else { return };
    let size = (
        (logical.0 as f32 * scale) as u32,
        (logical.1 as f32 * scale) as u32,
    );
    let cmds = scene.cmds((logical.0 as f32, logical.1 as f32));
    let a = wgpu_pixels(&cmds, size, scale);
    let b = gles_pixels(&mut g, &cmds, size, scale);
    // Two blank images would match: the scene has to have drawn something.
    let blank = a.chunks(4).filter(|p| *p == &a[..4]).count();
    assert!(
        blank < a.len() / 4 * 9 / 10,
        "{name}: the wgpu image is blank"
    );
    if let Some(dir) = std::env::var_os("IRON_GLES_DUMP") {
        for (which, pixels) in [("wgpu", &a), ("gles", &b)] {
            let path = PathBuf::from(&dir).join(format!(
                "{}-{which}.png",
                name.replace(' ', "_").replace('+', "and")
            ));
            image::save_buffer(path, pixels, size.0, size.1, image::ColorType::Rgba8).unwrap();
        }
    }
    let d = diff(&a, &b, tolerance);
    eprintln!(
        "{name}: max diff {}, {} of {} pixels off by more than {tolerance}",
        d.max, d.off, d.pixels
    );
    assert!(
        (d.off as f32) <= allowed_off * d.pixels as f32,
        "{name}: {} of {} pixels differ by more than {tolerance} (max {})",
        d.off,
        d.pixels,
        d.max
    );
}

// M2-GLES-02
#[test]
fn boxes_borders_and_gradients_match_wgpu() {
    let mut s = Scene::new();
    s.at(
        0,
        "box",
        [8.0, 8.0, 60.0, 50.0],
        &[
            ("backgroundColor", "white"),
            ("border", "4px solid #0000ff"),
            ("borderRadius", "12px"),
        ],
    );
    s.at(
        0,
        "box",
        [80.0, 8.0, 60.0, 50.0],
        &[
            ("backgroundColor", "#204060"),
            ("border", "3px dashed #ff8800"),
        ],
    );
    s.at(
        0,
        "box",
        [152.0, 8.0, 60.0, 50.0],
        &[
            ("backgroundColor", "#204060"),
            ("border", "4px dotted #88ff00"),
            ("borderRadius", "8px"),
        ],
    );
    s.at(
        0,
        "box",
        [8.0, 70.0, 100.0, 40.0],
        &[
            (
                "backgroundImage",
                "linear-gradient(to right, #ff0000, #0000ff)",
            ),
            ("borderRadius", "0 20px"),
        ],
    );
    s.at(
        0,
        "box",
        [120.0, 70.0, 50.0, 50.0],
        &[("backgroundImage", "radial-gradient(circle, white, black)")],
    );
    s.at(
        0,
        "box",
        [180.0, 70.0, 50.0, 50.0],
        &[
            (
                "backgroundImage",
                "conic-gradient(red, yellow, lime, aqua, blue, red)",
            ),
            ("borderRadius", "50%"),
        ],
    );
    s.at(
        0,
        "box",
        [8.0, 124.0, 80.0, 26.0],
        &[
            ("border", "2px solid"),
            ("borderColor", "red lime blue yellow"),
        ],
    );
    same("shapes", s, (240, 156), 1.0, 6, 0.002);
}

// M2-GLES-03
#[test]
fn shadows_match_wgpu() {
    let mut s = Scene::new();
    s.at(
        0,
        "box",
        [20.0, 20.0, 70.0, 50.0],
        &[
            ("backgroundColor", "#c0c0c0"),
            ("borderRadius", "10px"),
            ("boxShadow", "0 8px 16px rgba(0,0,0,0.6)"),
        ],
    );
    s.at(
        0,
        "box",
        [120.0, 20.0, 70.0, 50.0],
        &[
            ("backgroundColor", "#c0c0c0"),
            ("borderRadius", "10px"),
            ("boxShadow", "inset 0 0 12px 2px rgba(0,0,0,0.8)"),
        ],
    );
    same("shadows", s, (220, 100), 1.0, 6, 0.002);
}

// M2-GLES-04
#[test]
fn clips_match_wgpu() {
    let mut s = Scene::new();
    let parent = s.at(
        0,
        "box",
        [20.0, 20.0, 80.0, 60.0],
        &[
            ("overflow", "hidden"),
            ("borderRadius", "20px"),
            ("backgroundColor", "#303030"),
        ],
    );
    s.at(
        parent,
        "box",
        [-20.0, 30.0, 150.0, 60.0],
        &[("backgroundColor", "orange")],
    );
    s.at(
        parent,
        "box",
        [40.0, -10.0, 30.0, 90.0],
        &[("backgroundColor", "teal")],
    );
    same("clips", s, (120, 100), 1.0, 6, 0.002);
}

// M2-GLES-05
#[test]
fn text_matches_wgpu() {
    let mut s = Scene::new();
    let a = s.at(
        0,
        "text",
        [8.0, 6.0, 200.0, 24.0],
        &[("color", "white"), ("fontSize", "18")],
    );
    s.text(a, "Hello GLES 123 !?");
    let b = s.at(
        0,
        "text",
        [8.0, 36.0, 200.0, 40.0],
        &[("color", "#ffcc44"), ("fontSize", "32")],
    );
    s.text(b, "Iron UI");
    let c = s.at(
        0,
        "box",
        [8.0, 84.0, 200.0, 26.0],
        &[("backgroundColor", "#e8e8e8")],
    );
    let d = s.at(
        c,
        "text",
        [4.0, 2.0, 190.0, 22.0],
        &[("color", "#101010"), ("fontSize", "16")],
    );
    s.text(d, "dark text on light");
    // Glyph placement depends on the scale: both at 1 and 2.
    same("text x1", s, (216, 116), 1.0, 8, 0.01);
}

// M2-GLES-06
#[test]
fn text_and_boxes_at_scale_two_and_a_transformed_plane_match_wgpu() {
    let mut s = Scene::new();
    s.at(
        0,
        "box",
        [10.0, 10.0, 60.0, 40.0],
        &[("backgroundColor", "crimson"), ("borderRadius", "8px")],
    );
    let t = s.at(
        0,
        "text",
        [10.0, 56.0, 100.0, 20.0],
        &[("color", "white"), ("fontSize", "14")],
    );
    s.text(t, "scale two");
    s.at(
        0,
        "box",
        [90.0, 20.0, 50.0, 40.0],
        &[("backgroundColor", "gold"), ("transform", "rotate(12deg)")],
    );
    same("scale 2 + transform", s, (150, 90), 2.0, 8, 0.01);
}
