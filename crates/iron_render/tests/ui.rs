//! UI renderer tests (M2-DRW-01..08): scenes are built through the real UiTree
//! from styles, rendered offscreen, and checked with pixel assertions that
//! encode the CSS behaviour, plus golden images for regressions.

use iron_core::{Commit, Value};
use iron_render::testing::{assert_golden, shared_gpu};
use iron_render::{Color, OffscreenTarget, RenderStats, RenderTarget, UiRenderer};
use iron_ui::UiTree;
use iron_ui::opcodes::*;

/// Golden comparisons allow small differences between GPUs and drivers.
const TOLERANCE: u8 = 3;

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

    /// Adds a box under `parent` (0 = root) and returns its id.
    fn add(&mut self, parent: u32, style: &[(&str, Value)]) -> u32 {
        let id = self.next;
        self.next += 1;
        let n = |v: u32| Value::Number(v.into());
        self.ops
            .push(Value::Array(vec![n(CREATE), n(id), "box".into()]));
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
}

/// Absolute position and size, plus extra style.
fn at(x: f64, y: f64, w: f64, h: f64, extra: &[(&str, Value)]) -> Vec<(&'static str, Value)> {
    let mut v: Vec<(&'static str, Value)> = vec![
        ("position", "absolute".into()),
        ("left", x.into()),
        ("top", y.into()),
        ("width", w.into()),
        ("height", h.into()),
    ];
    for (k, val) in extra {
        // Style names in these tests are static strings.
        let k: &'static str = Box::leak(k.to_string().into_boxed_str());
        v.push((k, val.clone()));
    }
    v
}

struct Frame {
    pixels: Vec<u8>,
    width: u32,
    stats: RenderStats,
}

impl Frame {
    fn px(&self, x: u32, y: u32) -> [u8; 4] {
        let i = ((y * self.width + x) * 4) as usize;
        self.pixels[i..i + 4].try_into().unwrap()
    }

    fn luma(&self, x: u32, y: u32) -> f32 {
        let [r, g, b, _] = self.px(x, y);
        0.2126 * f32::from(r) + 0.7152 * f32::from(g) + 0.0722 * f32::from(b)
    }
}

fn render(scene: Scene, logical: (u32, u32), scale: f32, clear: Color) -> Frame {
    render_with_assets(scene, logical, scale, clear, None)
}

fn render_with_assets(
    scene: Scene,
    logical: (u32, u32),
    scale: f32,
    clear: Color,
    assets: Option<&std::path::Path>,
) -> Frame {
    let gpu = shared_gpu();
    let mut tree = UiTree::new(logical.0 as f32, logical.1 as f32);
    let errors = tree.apply_commit(&Commit { ops: scene.ops });
    assert!(errors.is_empty(), "{errors:?}");
    let cmds = tree.draw_list();

    let (w, h) = (
        (logical.0 as f32 * scale) as u32,
        (logical.1 as f32 * scale) as u32,
    );
    let target = OffscreenTarget::with_format(&gpu, w, h, wgpu::TextureFormat::Rgba8UnormSrgb);
    let mut ui = UiRenderer::new(&gpu);
    if let Some(root) = assets {
        ui.set_asset_root(root);
    }
    let view = target.view().unwrap();
    let stats = ui.render(
        &gpu,
        &RenderTarget {
            view: &view,
            format: target.format(),
            size: (w, h),
            scale,
        },
        clear,
        &cmds,
    );
    Frame {
        pixels: target.read_pixels(&gpu).unwrap(),
        width: w,
        stats,
    }
}

use iron_render::wgpu;

const BLACK: Color = Color::rgb(0.0, 0.0, 0.0);

fn three_boxes() -> Scene {
    let mut s = Scene::new();
    s.add(
        0,
        &at(
            0.0,
            0.0,
            32.0,
            32.0,
            &[("backgroundColor", "#ff0000".into())],
        ),
    );
    s.add(
        0,
        &at(
            32.0,
            0.0,
            32.0,
            32.0,
            &[("backgroundColor", "#00ff00".into())],
        ),
    );
    s.add(
        0,
        &at(
            64.0,
            0.0,
            32.0,
            32.0,
            &[("backgroundColor", "#0000ff".into())],
        ),
    );
    s
}

// M2-DRW-01
#[test]
fn coloured_boxes() {
    let f = render(three_boxes(), (96, 32), 1.0, BLACK);
    assert_eq!(f.px(16, 16), [255, 0, 0, 255]);
    assert_eq!(f.px(48, 16), [0, 255, 0, 255]);
    assert_eq!(f.px(80, 16), [0, 0, 255, 255]);
    // Edges are crisp on pixel boundaries.
    assert_eq!(f.px(31, 0), [255, 0, 0, 255]);
    assert_eq!(f.px(32, 0), [0, 255, 0, 255]);
    assert_eq!(f.stats.draw_calls, 1);
    assert_golden("M2-DRW-01", 96, 32, &f.pixels, TOLERANCE);
}

// M2-DRW-02
#[test]
fn rounded_corners_and_borders() {
    let mut s = Scene::new();
    s.add(
        0,
        &at(
            8.0,
            8.0,
            48.0,
            48.0,
            &[
                ("backgroundColor", "white".into()),
                ("border", "4px solid #0000ff".into()),
                ("borderRadius", 12.0.into()),
            ],
        ),
    );
    let f = render(s, (64, 64), 1.0, BLACK);
    assert_eq!(f.px(32, 32), [255, 255, 255, 255], "fill");
    assert_eq!(f.px(9, 32), [0, 0, 255, 255], "left border band (x 8..12)");
    assert_eq!(f.px(32, 54), [0, 0, 255, 255], "bottom border band");
    assert_eq!(f.px(9, 9), [0, 0, 0, 255], "outside the rounded corner");
    assert_eq!(f.px(4, 32), [0, 0, 0, 255], "outside the box");
    // The corner edge is anti-aliased: somewhere along the diagonal is partial.
    let partial = (8..20).any(|i| {
        let [_, _, b, _] = f.px(i, i);
        b > 10 && b < 245
    });
    assert!(partial, "rounded corner should be anti-aliased");
    assert_golden("M2-DRW-02", 64, 64, &f.pixels, TOLERANCE);
}

// M2-DRW-03
#[test]
fn many_boxes_are_one_draw_call() {
    let mut s = Scene::new();
    for i in 0..2000u32 {
        let (x, y) = (f64::from(i % 50) * 4.0, f64::from(i / 50) * 4.0);
        s.add(
            0,
            &at(x, y, 3.0, 3.0, &[("backgroundColor", "#888".into())]),
        );
    }
    let f = render(s, (200, 160), 1.0, BLACK);
    assert_eq!(f.stats.instances, 2000);
    assert_eq!(f.stats.draw_calls, 1);
    assert_eq!(f.stats.passes, 2, "one quad pass and the blit");
}

// M2-DRW-04
#[test]
fn scale_factor_two_renders_twice_the_pixels() {
    let one = render(three_boxes(), (96, 32), 1.0, BLACK);
    let two = render(three_boxes(), (96, 32), 2.0, BLACK);
    assert_eq!(two.width, 192);
    for (x, y) in [(16, 16), (48, 16), (80, 16), (31, 5), (32, 5)] {
        assert_eq!(two.px(x * 2, y * 2), one.px(x, y), "({x}, {y})");
        assert_eq!(two.px(x * 2 + 1, y * 2 + 1), one.px(x, y), "({x}, {y})");
    }
    assert_golden("M2-DRW-04", 192, 64, &two.pixels, TOLERANCE);
}

// M2-DRW-05
#[test]
fn per_corner_radii_and_gradients() {
    let mut s = Scene::new();
    s.add(
        0,
        &at(
            0.0,
            0.0,
            100.0,
            40.0,
            &[
                (
                    "backgroundImage",
                    "linear-gradient(to right, #ff0000, #0000ff)".into(),
                ),
                ("borderRadius", "0 20px".into()),
            ],
        ),
    );
    s.add(
        0,
        &at(
            0.0,
            50.0,
            50.0,
            50.0,
            &[(
                "backgroundImage",
                "radial-gradient(circle, white, black)".into(),
            )],
        ),
    );
    let f = render(s, (100, 100), 1.0, BLACK);

    let [r0, _, b0, _] = f.px(1, 20);
    let [r1, _, b1, _] = f.px(98, 20);
    assert!(r0 > 240 && b0 < 15, "left end is red: {:?}", f.px(1, 20));
    assert!(b1 > 240 && r1 < 15, "right end is blue: {:?}", f.px(98, 20));
    let [rm, _, bm, _] = f.px(50, 20);
    assert!(
        rm > 100 && bm > 100,
        "middle mixes both: {:?}",
        f.px(50, 20)
    );
    // Radii "0 20px": top-left square, top-right rounded.
    assert_eq!(f.px(0, 0)[3], 255);
    assert!(f.px(0, 0)[0] > 240, "top-left corner is square");
    assert_eq!(
        f.px(99, 0),
        [0, 0, 0, 255],
        "top-right corner is rounded away"
    );

    // Radial: white centre fading to black at the corners.
    assert!(f.luma(25, 75) > 240.0, "radial centre is white");
    assert!(f.luma(1, 51) < f.luma(10, 60), "darker towards the corner");
    assert_golden("M2-DRW-05", 100, 100, &f.pixels, TOLERANCE);
}

// M2-DRW-06
#[test]
fn outer_and_inset_shadows() {
    let mut s = Scene::new();
    s.add(
        0,
        &at(
            0.0,
            0.0,
            200.0,
            100.0,
            &[("backgroundColor", "#eeeeee".into())],
        ),
    );
    s.add(
        0,
        &at(
            30.0,
            20.0,
            40.0,
            40.0,
            &[
                ("backgroundColor", "white".into()),
                ("borderRadius", 8.0.into()),
                ("boxShadow", "0 8px 16px rgba(0,0,0,0.6)".into()),
            ],
        ),
    );
    s.add(
        0,
        &at(
            120.0,
            20.0,
            60.0,
            60.0,
            &[
                ("backgroundColor", "white".into()),
                ("boxShadow", "inset 0 0 12px 2px rgba(0,0,0,0.8)".into()),
            ],
        ),
    );
    let f = render(s, (200, 100), 1.0, BLACK);
    let bg = f.luma(5, 5);
    assert!(f.luma(50, 64) < bg - 20.0, "shadow below the box");
    assert!(
        f.luma(50, 64) < f.luma(50, 85),
        "shadow fades with distance"
    );
    assert!(
        (f.luma(100, 5) - bg).abs() < 2.0,
        "far from the box is untouched"
    );
    assert_eq!(
        f.px(50, 40),
        [255, 255, 255, 255],
        "the box covers its own shadow"
    );
    // Inset: dark at the inner edge, white in the middle.
    // At the inner edge the hole (shrunk by the 2px spread) is ~2px away; with
    // sigma = 6 its blurred coverage is ~0.37, so the shadow is ~0.8 * 0.63 = 50%
    // black over white: sRGB ~188.
    let edge = f.luma(120, 50);
    assert!(
        (175.0..205.0).contains(&edge),
        "inset shadow at the edge: {edge}"
    );
    assert!(
        f.luma(120, 50) < f.luma(126, 50) && f.luma(126, 50) < f.luma(134, 50),
        "inset shadow fades inwards"
    );
    assert!(f.luma(150, 50) > 250.0, "inset shadow fades to the centre");
    assert_golden("M2-DRW-06", 200, 100, &f.pixels, TOLERANCE);
}

// M2-DRW-07
#[test]
fn filter_blur_blurs_the_box_and_children() {
    let mut s = Scene::new();
    s.add(
        0,
        &at(
            10.0,
            20.0,
            40.0,
            40.0,
            &[("backgroundColor", "white".into())],
        ),
    );
    let blurred = s.add(
        0,
        &at(
            80.0,
            20.0,
            40.0,
            40.0,
            &[
                ("backgroundColor", "white".into()),
                ("filter", "blur(4px)".into()),
            ],
        ),
    );
    // A child inside the blurred box is blurred too.
    s.add(
        blurred,
        &at(10.0, 10.0, 20.0, 20.0, &[("backgroundColor", "red".into())]),
    );
    let f = render(s, (140, 80), 1.0, BLACK);

    assert_eq!(f.stats.layers, 1);
    // Sharp box: hard edge.
    assert_eq!(f.px(9, 40), [0, 0, 0, 255]);
    assert_eq!(f.px(10, 40), [255, 255, 255, 255]);
    // Blurred box: the edge is soft on both sides.
    let outside = f.luma(78, 40);
    let inside = f.luma(82, 40);
    assert!(
        outside > 20.0 && outside < 200.0,
        "blur spills outside: {outside}"
    );
    assert!(
        inside > 50.0 && inside < 250.0,
        "blur softens inside: {inside}"
    );
    // The red child is softened but still red-dominant at its centre.
    let [r, g, _, _] = f.px(100, 40);
    assert!(r > g + 60, "child is still red: {:?}", f.px(100, 40));
    assert!(
        g > 10,
        "child edges blend with the white box: {:?}",
        f.px(100, 40)
    );
    // Well outside the layer bounds nothing changes.
    assert_eq!(f.px(135, 5), [0, 0, 0, 255]);
    assert_golden("M2-DRW-07", 140, 80, &f.pixels, TOLERANCE);
}

// M2-DRW-08
#[test]
fn backdrop_blur_blurs_only_behind_the_box() {
    let mut s = Scene::new();
    // Black and white stripes, 4px wide.
    for i in 0..30 {
        s.add(
            0,
            &at(
                f64::from(i) * 4.0,
                0.0,
                4.0,
                80.0,
                &[(
                    "backgroundColor",
                    (if i % 2 == 0 { "white" } else { "black" }).into(),
                )],
            ),
        );
    }
    // Frosted glass over the middle, with rounded corners.
    s.add(
        0,
        &at(
            30.0,
            20.0,
            60.0,
            40.0,
            &[
                ("backdropFilter", "blur(6px)".into()),
                ("backgroundColor", "rgba(255,255,255,0.1)".into()),
                ("borderRadius", 10.0.into()),
            ],
        ),
    );
    let f = render(s, (120, 80), 1.0, BLACK);
    let contrast = |y: u32, xs: std::ops::Range<u32>| {
        let l: Vec<f32> = xs.map(|x| f.luma(x, y)).collect();
        l.iter().cloned().fold(f32::MIN, f32::max) - l.iter().cloned().fold(f32::MAX, f32::min)
    };
    assert!(
        contrast(5, 40..80) > 200.0,
        "stripes outside the glass stay sharp"
    );
    assert!(
        contrast(40, 40..80) < 40.0,
        "stripes behind the glass are blurred"
    );
    // The rounded corner clips the blur: just inside the box's corner square but
    // outside the radius, stripes are still sharp.
    assert!(
        [(30, 20), (31, 20), (30, 21)]
            .iter()
            .any(|&(x, y)| f.luma(x, y) > 250.0 || f.luma(x, y) < 5.0),
        "corner outside the radius is unblurred"
    );
    assert_golden("M2-DRW-08", 120, 80, &f.pixels, TOLERANCE);
}

// M2-VIS-08 on the GPU: group opacity composites the group once.
#[test]
fn group_opacity_blends_the_group_as_a_whole() {
    let mut s = Scene::new();
    let parent = s.add(
        0,
        &at(
            0.0,
            0.0,
            40.0,
            40.0,
            &[
                ("backgroundColor", "#ff0000".into()),
                ("opacity", 0.5.into()),
            ],
        ),
    );
    s.add(
        parent,
        &at(
            10.0,
            10.0,
            20.0,
            20.0,
            &[("backgroundColor", "#0000ff".into())],
        ),
    );
    let f = render(s, (40, 40), 1.0, BLACK);
    // Group opacity: the blue child fully covers red inside the group, then the
    // whole group is 50% over black. (Per-quad alpha would show red through blue.)
    let [r, _, b, _] = f.px(20, 20);
    assert!(r < 5, "no red shows through the child: {:?}", f.px(20, 20));
    assert!(
        (180..=192).contains(&b),
        "blue at 50% over black (sRGB ~188): {:?}",
        f.px(20, 20)
    );
    let [r, _, _, _] = f.px(5, 5);
    assert!(
        (180..=192).contains(&r),
        "red at 50% over black: {:?}",
        f.px(5, 5)
    );
}

fn images_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/images")
}

// M2-DRW-05 (images): size, position, repeat and clipping to the box shape.
#[test]
fn background_images() {
    let mut s = Scene::new();
    // checker.png is 4x4: 2x2 red/blue squares. Sized to 20px and repeated.
    s.add(
        0,
        &at(
            0.0,
            0.0,
            40.0,
            40.0,
            &[
                ("backgroundImage", "url(checker.png)".into()),
                ("backgroundSize", "20px".into()),
            ],
        ),
    );
    // wide.png is 8x4 (left green, right yellow): `contain` letterboxes it.
    s.add(
        0,
        &at(
            40.0,
            0.0,
            40.0,
            40.0,
            &[
                ("backgroundColor", "#333333".into()),
                ("backgroundImage", "url('wide.png')".into()),
                ("backgroundSize", "contain".into()),
                ("backgroundPosition", "center".into()),
                ("backgroundRepeat", "no-repeat".into()),
            ],
        ),
    );
    // `cover` crops it to the middle; a circle clips the corners.
    s.add(
        0,
        &at(
            80.0,
            0.0,
            40.0,
            40.0,
            &[
                ("backgroundImage", "url(wide.png)".into()),
                ("backgroundSize", "cover".into()),
                ("backgroundPosition", "center".into()),
                ("borderRadius", "50%".into()),
            ],
        ),
    );
    // Missing and unsafe URLs fall back to the background colour.
    s.add(
        0,
        &at(
            0.0,
            40.0,
            20.0,
            20.0,
            &[
                ("backgroundColor", "white".into()),
                ("backgroundImage", "url(missing.png)".into()),
            ],
        ),
    );
    s.add(
        0,
        &at(
            20.0,
            40.0,
            20.0,
            20.0,
            &[
                ("backgroundColor", "white".into()),
                ("backgroundImage", "url(../images/wide.png)".into()),
            ],
        ),
    );
    let f = render_with_assets(s, (120, 60), 1.0, BLACK, Some(&images_dir()));

    let red = [255, 0, 0, 255];
    let blue = [0, 0, 255, 255];
    let green = [0, 255, 0, 255];
    let yellow = [255, 255, 0, 255];
    // Tiles repeat every 20px: red, blue, red...
    assert_eq!(f.px(5, 5), red);
    assert_eq!(f.px(15, 5), blue);
    assert_eq!(f.px(25, 5), red);
    assert_eq!(f.px(5, 15), blue);
    assert_eq!(f.px(35, 35), red);
    // contain: a 40x20 band in the middle, background colour above and below.
    assert_eq!(f.px(50, 20), green);
    assert_eq!(f.px(70, 20), yellow);
    assert_eq!(f.px(60, 4), [0x33, 0x33, 0x33, 255]);
    assert_eq!(f.px(60, 36), [0x33, 0x33, 0x33, 255]);
    // No-repeat edges don't pick up colour from the opposite edge of the image.
    assert_eq!(f.px(40, 20), green, "left edge of a no-repeat image");
    assert_eq!(f.px(79, 20), yellow, "right edge of a no-repeat image");
    // cover: the middle half of the image fills the circle.
    assert_eq!(f.px(90, 20), green);
    assert_eq!(f.px(110, 20), yellow);
    assert_eq!(f.px(81, 1), [0, 0, 0, 255], "outside the circle");
    // Fallbacks.
    assert_eq!(f.px(10, 50), [255, 255, 255, 255]);
    assert_eq!(f.px(30, 50), [255, 255, 255, 255]);
    assert_golden("M2-DRW-05-images", 120, 60, &f.pixels, TOLERANCE);
}
