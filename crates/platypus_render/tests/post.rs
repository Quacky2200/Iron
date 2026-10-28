// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// SPDX-License-Identifier: Apache-2.0

//! Post-process passes (M7-FX): the built-in effects, custom WGSL, order and per-view parts.

use std::sync::Arc;

use platypus_core::glam::{Mat4, Vec3};
use platypus_render::testing::{assert_golden, shared_gpu};
use platypus_render::{
    ALL_LAYERS, Color, Light, LightKind, MeshInstance, OffscreenTarget, PostEffect, PostParamType,
    PostShader, RenderTarget, Sky, WorldCamera, WorldRenderer, WorldView, primitive_id, wgpu,
};
use platypus_render_api::{EFFECTS, ParamType, effect_named};

const SIZE: (u32, u32) = (160, 120);

/// A built-in effect from the API crate's table, with some parameters changed.
fn builtin(name: &str, set: &[(&str, [f32; 4])]) -> PostEffect {
    let def = &EFFECTS[effect_named(name).expect("a built-in effect")];
    let params = def
        .params
        .iter()
        .map(|p| {
            let ty = match p.ty {
                ParamType::Float => PostParamType::Float,
                ParamType::Vec2 => PostParamType::Vec2,
                ParamType::Vec3 => PostParamType::Vec3,
                ParamType::Vec4 => PostParamType::Vec4,
            };
            (p.name.to_string(), ty)
        })
        .collect();
    let shader = PostShader::new(
        def.name,
        &def.sources.join("\n"),
        params,
        def.passes,
        def.depth,
    );
    let mut effect = PostEffect::new(Arc::new(shader));
    for (slot, p) in def.params.iter().enumerate() {
        effect.params[slot] = set
            .iter()
            .find(|(n, _)| *n == p.name)
            .map_or(p.default, |(_, v)| *v);
    }
    effect
}

fn custom(source: &str) -> PostEffect {
    PostEffect::new(Arc::new(PostShader::new(
        "custom",
        source,
        Vec::new(),
        1,
        source.contains("scene_depth") || source.contains("scene_normal"),
    )))
}

/// Balls receding in a line over a floor, a bright thin bar and a few hard edges.
fn scene(viewport: [f32; 4]) -> WorldView {
    let size = platypus_render::viewport_size(viewport, SIZE);
    let eye = Vec3::new(0.0, 1.3, 5.0);
    let camera = WorldCamera::perspective(
        platypus_core::glam::camera::rh::view::look_at_mat4(
            eye,
            Vec3::new(0.0, 0.5, -2.0),
            Vec3::Y,
        )
        .inverse(),
        50.0,
        0.1,
        100.0,
        size,
        Color::rgb(0.0, 0.0, 0.0),
    );
    let mut v = WorldView::new(camera, viewport);
    v.skybox = true;
    let ball = |x: f32, z: f32, color: [f32; 4]| MeshInstance {
        mesh: primitive_id("sphere").unwrap(),
        model: Mat4::from_translation(Vec3::new(x, 0.5, z)),
        color,
        smoothness: 0.6,
        ..Default::default()
    };
    v.meshes = vec![
        MeshInstance {
            mesh: primitive_id("plane").unwrap(),
            color: [0.18, 0.2, 0.22, 1.0],
            ..Default::default()
        },
        ball(-1.0, 2.5, [1.0, 0.25, 0.1, 1.0]),
        ball(0.0, -1.0, [0.2, 1.0, 0.3, 1.0]),
        ball(1.2, -6.0, [0.2, 0.4, 1.0, 1.0]),
        MeshInstance {
            mesh: primitive_id("cube").unwrap(),
            model: Mat4::from_translation(Vec3::new(0.0, 1.5, -1.0))
                * Mat4::from_rotation_z(0.2)
                * Mat4::from_scale(Vec3::new(3.0, 0.06, 0.06)),
            color: [1.0, 1.0, 1.0, 1.0],
            unlit: true,
            ..Default::default()
        },
    ];
    v.lights = vec![Light {
        kind: LightKind::Directional,
        color: [1.5, 1.45, 1.3],
        position: Vec3::ZERO,
        direction: Vec3::new(-0.3, -0.8, -0.5).normalize(),
        range: 0.0,
        shadows: None,
        culling_mask: ALL_LAYERS,
        volumetric: 1.0,
    }];
    v
}

struct Rig {
    target: OffscreenTarget,
    world: WorldRenderer,
}

impl Rig {
    fn new(gpu: &platypus_render::Gpu) -> Self {
        let target =
            OffscreenTarget::with_format(gpu, SIZE.0, SIZE.1, wgpu::TextureFormat::Rgba8UnormSrgb);
        let mut world = WorldRenderer::new(gpu);
        world.set_sky(Some(Sky::default()));
        Self { target, world }
    }

    fn render(&mut self, gpu: &platypus_render::Gpu, views: &[WorldView]) -> Vec<u8> {
        let view = self.target.view().unwrap();
        let rt = RenderTarget {
            view: &view,
            format: self.target.format(),
            size: SIZE,
            scale: 1.0,
        };
        self.world.render_views(gpu, &rt, views, &[]);
        self.target.read_pixels(gpu).unwrap()
    }

    fn with(&mut self, gpu: &platypus_render::Gpu, effects: Vec<PostEffect>) -> Vec<u8> {
        let mut v = scene([0.0, 0.0, 1.0, 1.0]);
        v.post = effects;
        self.render(gpu, &[v])
    }
}

fn at(pixels: &[u8], x: u32, y: u32) -> [u8; 4] {
    let i = ((y * SIZE.0 + x) * 4) as usize;
    [pixels[i], pixels[i + 1], pixels[i + 2], pixels[i + 3]]
}

fn luma(p: [u8; 4]) -> f32 {
    0.2126 * f32::from(p[0]) + 0.7152 * f32::from(p[1]) + 0.0722 * f32::from(p[2])
}

/// How much neighbouring pixels differ in a rectangle: sharp detail scores high.
fn detail(pixels: &[u8], x0: u32, y0: u32, x1: u32, y1: u32) -> f32 {
    let mut sum = 0.0;
    for y in y0..y1 {
        for x in x0..x1 {
            sum += (luma(at(pixels, x, y)) - luma(at(pixels, x + 1, y))).abs();
            sum += (luma(at(pixels, x, y)) - luma(at(pixels, x, y + 1))).abs();
        }
    }
    sum
}

fn differing(a: &[u8], b: &[u8], over: u8) -> usize {
    a.chunks(4)
        .zip(b.chunks(4))
        .filter(|(p, q)| p.iter().zip(*q).any(|(x, y)| x.abs_diff(*y) > over))
        .count()
}

/// Pictures in a grid, `columns` across.
fn sheet(shots: &[&Vec<u8>], columns: usize) -> (u32, u32, Vec<u8>) {
    let rows = shots.len().div_ceil(columns);
    let (w, h) = (SIZE.0 as usize, SIZE.1 as usize);
    let mut out = vec![0u8; columns * w * rows * h * 4];
    for (i, shot) in shots.iter().enumerate() {
        let (cx, cy) = (i % columns, i / columns);
        for y in 0..h {
            let from = y * w * 4;
            let to = ((cy * h + y) * columns * w + cx * w) * 4;
            out[to..to + w * 4].copy_from_slice(&shot[from..from + w * 4]);
        }
    }
    ((columns * w) as u32, (rows * h) as u32, out)
}

// M7-FX-05
#[test]
fn scene_depth_and_normals_reach_the_shader() {
    let gpu = shared_gpu();
    let mut rig = Rig::new(&gpu);
    let depth = rig.with(
        &gpu,
        vec![custom(
            "fn effect(uv: vec2<f32>, c: vec4<f32>) -> vec4<f32> { let d = scene_depth(uv) / 10.0; return vec4<f32>(d, d, d, 1.0); }",
        )],
    );
    let normals = rig.with(
        &gpu,
        vec![custom(
            "fn effect(uv: vec2<f32>, c: vec4<f32>) -> vec4<f32> { return vec4<f32>(scene_normal(uv) * 0.5 + 0.5, 1.0); }",
        )],
    );
    let (w, h, pixels) = sheet(&[&depth, &normals], 2);
    assert_golden("M7-FX-05", w, h, &pixels, 3);
}

// M7-FX-04
#[test]
fn built_in_effects_change_the_picture_the_way_they_should() {
    let gpu = shared_gpu();
    let mut rig = Rig::new(&gpu);
    let f = |v: f32| [v, 0.0, 0.0, 0.0];
    let plain = rig.with(&gpu, Vec::new());
    let vignette = rig.with(&gpu, vec![builtin("vignette", &[])]);
    let chromatic = rig.with(
        &gpu,
        vec![builtin("chromaticAberration", &[("amount", f(0.08))])],
    );
    let blur = rig.with(&gpu, vec![builtin("blur", &[("radius", f(8.0))])]);
    let pixelate = rig.with(&gpu, vec![builtin("pixelate", &[("size", f(8.0))])]);
    let grain = rig.with(&gpu, vec![builtin("filmGrain", &[("intensity", f(0.4))])]);
    let grey = rig.with(
        &gpu,
        vec![builtin("colorGrading", &[("saturation", f(0.0))])],
    );
    let warm = rig.with(
        &gpu,
        vec![builtin(
            "colorGrading",
            &[("balance", [0.8, 0.0, 0.0, 0.0])],
        )],
    );
    let outline = rig.with(&gpu, vec![builtin("outline", &[])]);
    let crt = rig.with(&gpu, vec![builtin("crt", &[])]);
    let posterize = rig.with(
        &gpu,
        vec![builtin(
            "posterize",
            &[("levels", f(3.0)), ("softness", f(0.01))],
        )],
    );
    let tilt = rig.with(&gpu, vec![builtin("tiltShift", &[])]);
    // A fast lens on a 120 pixel tall picture: the near ball (3 m) is soft, the middle one (6 m)
    // in focus.
    let dof = rig.with(
        &gpu,
        vec![builtin(
            "depthOfField",
            &[
                ("focus_distance", f(6.0)),
                ("aperture", f(0.25)),
                ("max_blur", f(12.0)),
            ],
        )],
    );
    let all = [
        &plain, &vignette, &chromatic, &blur, &pixelate, &grain, &grey, &warm, &outline, &crt,
        &posterize, &tilt, &dof,
    ];
    let (w, h, pixels) = sheet(&all, 4);
    assert_golden("M7-FX-04", w, h, &pixels, 3);

    assert!(luma(at(&vignette, 1, 1)) < luma(at(&plain, 1, 1)) * 0.8);
    assert!((luma(at(&vignette, 80, 60)) - luma(at(&plain, 80, 60))).abs() < 2.0);

    assert!(
        at(&chromatic, 80, 60)
            .iter()
            .zip(at(&plain, 80, 60))
            .all(|(a, b)| a.abs_diff(b) <= 3)
    );
    assert!(differing(&chromatic, &plain, 12) > 150);

    assert!(detail(&blur, 0, 0, 150, 110) < detail(&plain, 0, 0, 150, 110) * 0.6);

    for (x, y) in [(3, 3), (10, 9), (77, 50)] {
        let corner = at(&pixelate, x / 8 * 8, y / 8 * 8);
        assert_eq!(at(&pixelate, x, y), corner, "block of ({x}, {y})");
    }
    assert!(differing(&pixelate, &plain, 6) > 400);

    let moved = differing(&grain, &plain, 6);
    assert!(moved > 4000 && moved < 19200, "grain moved {moved} pixels");

    assert!(
        grey.chunks(4)
            .all(|p| p[0].abs_diff(p[1]) <= 2 && p[1].abs_diff(p[2]) <= 2)
    );
    let (a, b) = (at(&warm, 80, 100), at(&plain, 80, 100));
    assert!(a[0] > b[0] && a[2] < b[2], "warmer: {a:?} against {b:?}");

    let inked = outline
        .chunks(4)
        .zip(plain.chunks(4))
        .filter(|(o, p)| luma([o[0], o[1], o[2], 255]) + 50.0 < luma([p[0], p[1], p[2], 255]))
        .count();
    assert!(inked > 150 && inked < 4000, "outline inked {inked} pixels");

    assert_eq!(&at(&crt, 0, 0)[..3], &[0, 0, 0]);

    let mut levels: Vec<u8> = posterize
        .chunks(4)
        .map(|p| luma([p[0], p[1], p[2], 255]) as u8 / 24)
        .collect();
    levels.sort_unstable();
    levels.dedup();
    assert!(
        levels.len() < 14,
        "posterize left {} brightness steps",
        levels.len()
    );

    assert!(detail(&dof, 2, 60, 55, 112) < detail(&plain, 2, 60, 55, 112) * 0.7);
    let focused = detail(&plain, 66, 50, 96, 72);
    assert!((detail(&dof, 66, 50, 96, 72) - focused).abs() < focused * 0.3);
    // The white bar is outside the sharp band.
    let peak = |p: &[u8]| {
        (28..40)
            .flat_map(|y| (98..112).map(move |x| (x, y)))
            .map(|(x, y)| luma(at(p, x, y)))
            .fold(0.0, f32::max)
    };
    // A thin line spreads out, so it's dimmer.
    assert!(
        peak(&tilt) + 8.0 < peak(&plain),
        "{} against {}",
        peak(&tilt),
        peak(&plain)
    );
    assert!(
        (detail(&tilt, 40, 55, 120, 65) - detail(&plain, 40, 55, 120, 65)).abs()
            < detail(&plain, 40, 55, 120, 65) * 0.2
    );
}

// M7-FX-06
#[test]
fn custom_wgsl_runs_and_a_broken_shader_is_skipped() {
    let gpu = shared_gpu();
    let mut rig = Rig::new(&gpu);
    let plain = rig.with(&gpu, Vec::new());
    let invert = rig.with(
        &gpu,
        vec![custom(
            "fn effect(uv: vec2<f32>, c: vec4<f32>) -> vec4<f32> { return vec4<f32>(vec3<f32>(1.0) - c.rgb, c.a); }",
        )],
    );
    // Light inverted: 1 - light, seen through the sRGB curve.
    let srgb = |v: u8| (f32::from(v) / 255.0).powf(2.2);
    for (x, y) in [(5, 5), (80, 100), (20, 90)] {
        let (a, b) = (at(&plain, x, y), at(&invert, x, y));
        assert!(
            (0..3).all(|k| {
                let want = (1.0 - srgb(a[k])).max(0.0).powf(1.0 / 2.2) * 255.0;
                (want - f32::from(b[k])).abs() <= 6.0
            }),
            "({x}, {y}): {a:?} and {b:?}"
        );
    }
    // No `effect` function, and a type error: logged, and the picture is left as it was.
    for source in [
        "fn nothing() {}",
        "fn effect(uv: vec2<f32>, c: vec4<f32>) -> vec4<f32> { return 1.0; }",
    ] {
        let broken = rig.with(&gpu, vec![custom(source)]);
        assert_eq!(differing(&broken, &plain, 1), 0, "{source}");
    }
    // A working pass after a broken one still runs.
    let after = rig.with(
        &gpu,
        vec![
            custom("fn effect() {"),
            builtin("colorGrading", &[("saturation", [0.0; 4])]),
        ],
    );
    assert!(after.chunks(4).all(|p| p[0].abs_diff(p[1]) <= 2));
}

// M7-FX-07
#[test]
fn passes_run_in_order_over_their_own_views() {
    let gpu = shared_gpu();
    let mut rig = Rig::new(&gpu);
    let f = |v: f32| [v, 0.0, 0.0, 0.0];
    let both = |first: &str, second: &str| {
        let a = builtin(first, &[("size", f(10.0)), ("radius", f(6.0))]);
        let b = builtin(second, &[("size", f(10.0)), ("radius", f(6.0))]);
        vec![a, b]
    };
    let plain = rig.with(&gpu, Vec::new());
    let pixelate_blur = rig.with(&gpu, both("pixelate", "blur"));
    let blur_pixelate = rig.with(&gpu, both("blur", "pixelate"));
    // Blocks of blurred or blocks of crisp: pixelating last leaves flat blocks, blurring last
    // smooths their steps.
    let block = |p: &[u8]| at(p, 40, 30) == at(p, 47, 37) && at(p, 41, 100) == at(p, 48, 105);
    assert!(block(&blur_pixelate));
    assert!(!block(&pixelate_blur));
    assert!(differing(&pixelate_blur, &blur_pixelate, 4) > 300);

    // Two cameras side by side; only the left one has an effect, and it stays in its half.
    let mut left = scene([0.0, 0.0, 0.5, 1.0]);
    left.post = vec![builtin("colorGrading", &[("saturation", [0.0; 4])])];
    let right = scene([0.5, 0.0, 0.5, 1.0]);
    let split = rig.render(&gpu, &[left, right]);
    let unfiltered = rig.render(
        &gpu,
        &[scene([0.0, 0.0, 0.5, 1.0]), scene([0.5, 0.0, 0.5, 1.0])],
    );
    for y in (0..SIZE.1).step_by(7) {
        for x in 0..SIZE.0 {
            let (a, b) = (at(&split, x, y), at(&unfiltered, x, y));
            if x < 80 {
                assert!(
                    a[0].abs_diff(a[1]) <= 2 && a[1].abs_diff(a[2]) <= 2,
                    "grey at ({x}, {y})"
                );
            } else {
                assert_eq!(a, b, "untouched at ({x}, {y})");
            }
        }
    }
    assert!(differing(&split, &plain, 20) > 1000);
}

// M7-FX-08
#[test]
fn depth_effects_work_with_msaa_and_after_tonemapping() {
    use platypus_render::{Grading, Tonemapping};
    let gpu = shared_gpu();
    let mut rig = Rig::new(&gpu);
    let outline = || vec![builtin("outline", &[])];
    let plain = rig.with(&gpu, outline());
    rig.world.msaa = 4;
    let msaa = rig.with(&gpu, outline());
    // Inked silhouettes with 4x MSAA's multisampled depth too, about where they were.
    let inked = |p: &[u8]| {
        p.chunks(4)
            .filter(|q| luma([q[0], q[1], q[2], 255]) < 60.0)
            .count()
    };
    assert!(inked(&msaa) > 100, "{} inked pixels", inked(&msaa));
    assert!(inked(&msaa).abs_diff(inked(&plain)) < inked(&plain) / 2 + 60);
    // With grading on the world goes through the HDR texture first; the passes still apply.
    rig.world.msaa = 1;
    rig.world.grading = Grading {
        exposure: 1.0,
        tonemapping: Tonemapping::Aces,
        ..Default::default()
    };
    let graded = rig.with(&gpu, outline());
    assert!(inked(&graded) > 100);
}
