//! The 3D world pass (M5.a): planes through a perspective camera, with screen UI
//! blended over them.

use iron_core::glam::{Mat4, Quat, Vec3};
use iron_core::{Commit, Value};
use iron_render::testing::{assert_golden, shared_gpu};
use iron_render::{
    Color, OffscreenTarget, RenderTarget, TextureData, TextureFilter, TextureWrap, UiRenderer,
    WorldCamera, WorldQuad, WorldRenderer, WorldTexture,
};
use iron_ui::UiTree;
use iron_ui::opcodes::*;

fn op(code: u32, args: &[Value]) -> Value {
    let mut v = vec![Value::Number(code.into())];
    v.extend_from_slice(args);
    Value::Array(v)
}

fn place(position: Vec3, rotation: Quat, size: (f32, f32)) -> Mat4 {
    Mat4::from_scale_rotation_translation(Vec3::new(size.0, size.1, 1.0), rotation, position)
}

/// A 4×4 checker, red and white, as a texture.
fn checker(gpu: &iron_render::Gpu) -> wgpu::TextureView {
    use iron_render::wgpu;
    let mut pixels = Vec::new();
    for y in 0..4 {
        for x in 0..4 {
            pixels.extend(if (x + y) % 2 == 0 {
                [230, 40, 40, 255]
            } else {
                [255, 255, 255, 255]
            });
        }
    }
    let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("checker"),
        size: wgpu::Extent3d {
            width: 4,
            height: 4,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    gpu.queue.write_texture(
        texture.as_image_copy(),
        &pixels,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(16),
            rows_per_image: None,
        },
        texture.size(),
    );
    texture.create_view(&Default::default())
}
use iron_render::wgpu;

// M5-RND-05
#[test]
fn planes_in_perspective_with_ui_over_them() {
    let gpu = shared_gpu();
    let size = (320, 200);
    let target =
        OffscreenTarget::with_format(&gpu, size.0, size.1, wgpu::TextureFormat::Rgba8UnormSrgb);
    let view = target.view().unwrap();
    let rt = RenderTarget {
        view: &view,
        format: target.format(),
        size,
        scale: 1.0,
    };

    // A camera 4 m back and 2 m up, looking at the origin.
    let eye = Vec3::new(0.0, 2.0, 4.0);
    let camera_world =
        iron_core::glam::camera::rh::view::look_at_mat4(eye, Vec3::ZERO, Vec3::Y).inverse();
    let camera = WorldCamera::perspective(
        camera_world,
        60.0,
        0.1,
        100.0,
        size,
        Color::rgb(0.05, 0.07, 0.12),
    );

    let flat = Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2);
    let quads = [
        // A grey floor, 8 m square.
        WorldQuad::colored(place(Vec3::ZERO, flat, (8.0, 8.0)), [0.25, 0.27, 0.3, 1.0]),
        // A green wall behind, partly hidden by the blue panel in front of it.
        WorldQuad::colored(
            place(Vec3::new(-0.8, 0.75, -1.0), Quat::IDENTITY, (1.5, 1.5)),
            [0.1, 0.7, 0.3, 1.0],
        ),
        // A half-transparent blue panel nearer the camera: the green shows through.
        WorldQuad::colored(
            place(
                Vec3::new(-0.2, 0.6, 0.2),
                Quat::from_rotation_y(0.4),
                (1.2, 1.0),
            ),
            [0.2, 0.4, 1.0, 0.5],
        ),
        // A textured plane turned towards the camera, right of centre.
        WorldQuad {
            model: place(
                Vec3::new(1.3, 0.6, -0.3),
                Quat::from_rotation_y(-0.5),
                (1.0, 1.0),
            ),
            color: [1.0; 4],
            texture: Some(WorldTexture::Frame(0)),
            uv: [0.0, 0.0, 1.0, 1.0],
            additive: false,
            order: 0,
        },
    ];
    let checker = checker(&gpu);
    let mut world = WorldRenderer::new(&gpu);
    world.render(&gpu, &rt, &camera, &quads, &[&checker]);

    // Screen UI over the world: a translucent bar along the top.
    let mut tree = UiTree::new(320.0, 200.0);
    let errors = tree.apply_commit(&Commit {
        ops: vec![
            op(CREATE, &[Value::Number(1.0), "box".into()]),
            op(
                SET_STYLE,
                &[
                    Value::Number(1.0),
                    "width".into(),
                    Value::Number(320.0),
                    "height".into(),
                    Value::Number(30.0),
                    "backgroundColor".into(),
                    "rgba(255, 200, 0, 0.6)".into(),
                ],
            ),
            op(APPEND_CHILD, &[Value::Number(0.0), Value::Number(1.0)]),
        ],
    });
    assert!(errors.is_empty(), "{errors:?}");
    tree.layout_if_needed();
    let cmds = tree.draw_list();
    UiRenderer::new(&gpu).render_overlay(&gpu, &rt, &cmds);

    let pixels = target.read_pixels(&gpu).unwrap();
    assert_golden("M5-RND-05", size.0, size.1, &pixels, 2);

    let at = |x: u32, y: u32| {
        let i = ((y * size.0 + x) * 4) as usize;
        [pixels[i], pixels[i + 1], pixels[i + 2]]
    };
    // The top bar is yellow over the dark sky; below it, the sky shows.
    let bar = at(5, 10);
    assert!(bar[0] > 150 && bar[1] > 120 && bar[2] < 80, "bar {bar:?}");
    let sky = at(5, 40);
    assert!(sky[0] < 80 && sky[2] < 120, "sky {sky:?}");
}

// M5-RND-06
#[test]
fn cameras_draw_into_their_viewports_by_depth() {
    use iron_render::{Projection, WorldView, viewport_size};
    let gpu = shared_gpu();
    let size = (320, 160);
    let target =
        OffscreenTarget::with_format(&gpu, size.0, size.1, wgpu::TextureFormat::Rgba8UnormSrgb);
    let view = target.view().unwrap();
    let rt = RenderTarget {
        view: &view,
        format: target.format(),
        size,
        scale: 1.0,
    };
    let flat = Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2);
    let quads = vec![
        WorldQuad::colored(place(Vec3::ZERO, flat, (6.0, 6.0)), [0.3, 0.3, 0.35, 1.0]),
        WorldQuad::colored(
            place(Vec3::new(0.0, 0.5, 0.0), Quat::IDENTITY, (1.0, 1.0)),
            [0.9, 0.5, 0.1, 1.0],
        ),
    ];
    let look = |eye: Vec3| {
        iron_core::glam::camera::rh::view::look_at_mat4(eye, Vec3::ZERO, Vec3::Y).inverse()
    };
    // Left half: perspective from the front; right half: orthographic from above.
    let left = [0.0, 0.0, 0.5, 1.0];
    let right = [0.5, 0.0, 0.5, 1.0];
    let views = [
        WorldView {
            meshes: Vec::new(),
            lights: Vec::new(),
            ambient: [0.2, 0.2, 0.22],
            skybox: false,
            camera: WorldCamera::new(
                look(Vec3::new(0.0, 2.0, 4.0)),
                Projection::Perspective {
                    fov_y_degrees: 60.0,
                },
                0.1,
                50.0,
                viewport_size(left, size),
                Color::rgb(0.1, 0.05, 0.05),
            ),
            viewport: left,
            quads: quads.clone(),
        },
        WorldView {
            meshes: Vec::new(),
            lights: Vec::new(),
            ambient: [0.2, 0.2, 0.22],
            skybox: false,
            camera: WorldCamera::new(
                Mat4::from_rotation_translation(flat, Vec3::new(0.0, 5.0, 0.0)),
                Projection::Orthographic { half_height: 4.0 },
                0.1,
                50.0,
                viewport_size(right, size),
                Color::rgb(0.05, 0.05, 0.1),
            ),
            viewport: right,
            // A red marker on the floor towards +x (east), seen from above.
            quads: [
                quads.as_slice(),
                &[WorldQuad::colored(
                    place(Vec3::new(2.0, 0.01, 0.0), flat, (1.0, 1.0)),
                    [0.9, 0.1, 0.1, 1.0],
                )],
            ]
            .concat(),
        },
        // A picture-in-picture inset over the left view (drawn after it, so on top).
        WorldView {
            meshes: Vec::new(),
            lights: Vec::new(),
            ambient: [0.2, 0.2, 0.22],
            skybox: false,
            camera: WorldCamera::new(
                look(Vec3::new(3.0, 1.0, 0.0)),
                Projection::Perspective {
                    fov_y_degrees: 50.0,
                },
                0.1,
                50.0,
                viewport_size([0.05, 0.05, 0.15, 0.3], size),
                Color::rgb(0.0, 0.2, 0.0),
            ),
            viewport: [0.05, 0.05, 0.15, 0.3],
            quads: vec![],
        },
    ];
    WorldRenderer::new(&gpu).render_views(&gpu, &rt, &views, &[]);
    let pixels = target.read_pixels(&gpu).unwrap();
    assert_golden("M5-RND-06", size.0, size.1, &pixels, 2);
    let at = |x: u32, y: u32| {
        let i = ((y * size.0 + x) * 4) as usize;
        [pixels[i], pixels[i + 1], pixels[i + 2]]
    };
    // The inset's green background covers the left view's top-left corner.
    assert!(at(20, 20)[1] > at(20, 20)[0] + 40, "{:?}", at(20, 20));
    // From above, the 6 m floor is a square inside the 8 m tall ortho view: the top edge
    // is background, and the marker is right of centre (east is +x on screen).
    let edge = at(240, 3);
    assert!(edge[2] > edge[0], "{edge:?}");
    let marker = at(240 + 40, 80);
    assert!(marker[0] > 150 && marker[1] < 120, "{marker:?}");
}

// M5-RND-07
#[test]
fn lit_primitives_with_lights() {
    use iron_render::{Light, LightKind, MeshInstance, primitive_id};
    let gpu = shared_gpu();
    let size = (320, 200);
    let target =
        OffscreenTarget::with_format(&gpu, size.0, size.1, wgpu::TextureFormat::Rgba8UnormSrgb);
    let view = target.view().unwrap();
    let rt = RenderTarget {
        view: &view,
        format: target.format(),
        size,
        scale: 1.0,
    };
    let eye = Vec3::new(0.0, 2.5, 6.0);
    let camera = WorldCamera::perspective(
        iron_core::glam::camera::rh::view::look_at_mat4(eye, Vec3::new(0.0, 0.5, 0.0), Vec3::Y)
            .inverse(),
        50.0,
        0.1,
        50.0,
        size,
        Color::rgb(0.02, 0.02, 0.03),
    );
    let mesh = |name: &str, at: Vec3, color: [f32; 4]| MeshInstance {
        mesh: primitive_id(name).unwrap(),
        model: Mat4::from_rotation_translation(Quat::from_rotation_y(0.6), at),
        color,
        emission: [0.0; 3],
        smoothness: 0.6,
        metallic: 0.0,
        unlit: false,
        texture: None,
        tiling: [1.0, 1.0, 0.0, 0.0],
        glow: iron_render::MeshGlow::None,
        cast_shadows: true,
    };
    let mut v = iron_render::WorldView::new(camera, [0.0, 0.0, 1.0, 1.0]);
    v.meshes = vec![
        MeshInstance {
            model: Mat4::from_scale(Vec3::splat(0.6)),
            smoothness: 0.0,
            metallic: 0.0,
            ..mesh("plane", Vec3::ZERO, [0.5, 0.5, 0.5, 1.0])
        },
        mesh("cube", Vec3::new(-1.8, 0.5, 0.0), [0.9, 0.3, 0.2, 1.0]),
        mesh("sphere", Vec3::new(0.0, 0.5, 0.0), [0.2, 0.5, 0.9, 1.0]),
        mesh("capsule", Vec3::new(1.8, 1.0, 0.0), [0.3, 0.8, 0.3, 1.0]),
        // An emissive, unlit marker and a see-through cylinder in front.
        MeshInstance {
            unlit: true,
            emission: [1.0, 0.8, 0.2],
            ..mesh("cube", Vec3::new(0.0, 2.2, 0.0), [0.2, 0.2, 0.2, 1.0])
        },
        mesh("cylinder", Vec3::new(0.9, 0.5, 1.5), [1.0, 1.0, 1.0, 0.35]),
    ];
    v.lights = vec![
        Light {
            kind: LightKind::Directional,
            color: [1.0, 0.95, 0.9],
            position: Vec3::ZERO,
            direction: Vec3::new(-0.5, -1.0, -0.6).normalize(),
            range: 0.0,
            shadows: None,
        },
        Light {
            kind: LightKind::Point,
            color: [6.0, 2.0, 1.0],
            position: Vec3::new(-1.0, 1.2, 1.2),
            direction: Vec3::ZERO,
            range: 4.0,
            shadows: None,
        },
    ];
    WorldRenderer::new(&gpu).render_views(&gpu, &rt, &[v], &[]);
    let pixels = target.read_pixels(&gpu).unwrap();
    assert_golden("M5-RND-07", size.0, size.1, &pixels, 3);
}

/// An `n` × `n` two-colour checker with a red top-left texel (to see orientation).
fn checker_data(
    n: u32,
    a: [u8; 4],
    b: [u8; 4],
    filter: TextureFilter,
    mipmaps: bool,
) -> TextureData {
    let mut rgba = Vec::new();
    for y in 0..n {
        for x in 0..n {
            let c = if x == 0 && y == 0 {
                [230, 40, 40, 255]
            } else if (x + y) % 2 == 0 {
                a
            } else {
                b
            };
            rgba.extend(c);
        }
    }
    TextureData {
        width: n,
        height: n,
        rgba,
        filter,
        wrap: TextureWrap::Repeat,
        mipmaps,
    }
}

// M5-RND-08
#[test]
fn textured_meshes_and_sprites() {
    use iron_render::{Light, LightKind, MeshGlow, MeshInstance, primitive_id};
    let gpu = shared_gpu();
    let size = (320, 200);
    let target =
        OffscreenTarget::with_format(&gpu, size.0, size.1, wgpu::TextureFormat::Rgba8UnormSrgb);
    let view = target.view().unwrap();
    let rt = RenderTarget {
        view: &view,
        format: target.format(),
        size,
        scale: 1.0,
    };
    let mut world = WorldRenderer::new(&gpu);
    let light = [235, 235, 225, 255];
    assert!(world.set_texture(
        &gpu,
        1,
        &checker_data(8, light, [60, 90, 160, 255], TextureFilter::Trilinear, true)
    ));
    // Pixel art: point-filtered, with a see-through half.
    assert!(world.set_texture(
        &gpu,
        2,
        &checker_data(
            4,
            [250, 200, 60, 255],
            [0, 0, 0, 0],
            TextureFilter::Point,
            false
        )
    ));
    assert!(!world.set_texture(
        &gpu,
        3,
        &TextureData {
            rgba: vec![0; 3],
            ..checker_data(1, light, light, TextureFilter::Point, false)
        }
    ));
    assert_eq!(world.texture_size(1), Some((8, 8)));
    assert_eq!(world.texture_size(3), None);

    let eye = Vec3::new(0.0, 2.5, 6.0);
    let camera = WorldCamera::perspective(
        iron_core::glam::camera::rh::view::look_at_mat4(eye, Vec3::new(0.0, 0.5, 0.0), Vec3::Y)
            .inverse(),
        50.0,
        0.1,
        50.0,
        size,
        Color::rgb(0.02, 0.02, 0.03),
    );
    let mesh = |name: &str, model: Mat4, tiling: [f32; 4]| MeshInstance {
        mesh: primitive_id(name).unwrap(),
        model,
        color: [1.0; 4],
        emission: [0.0; 3],
        smoothness: 0.2,
        metallic: 0.0,
        unlit: false,
        texture: Some(WorldTexture::Asset(1)),
        tiling,
        glow: MeshGlow::None,
        cast_shadows: true,
    };
    let mut v = iron_render::WorldView::new(camera, [0.0, 0.0, 1.0, 1.0]);
    v.meshes = vec![
        // A floor tiled 6 times: mipmaps keep the distance from shimmering.
        mesh(
            "plane",
            Mat4::from_scale(Vec3::splat(0.8)),
            [6.0, 6.0, 0.0, 0.0],
        ),
        mesh(
            "cube",
            Mat4::from_rotation_translation(Quat::from_rotation_y(0.6), Vec3::new(-1.5, 0.5, 0.0)),
            [1.0, 1.0, 0.0, 0.0],
        ),
        // Half a texture per face, shifted.
        mesh(
            "sphere",
            Mat4::from_translation(Vec3::new(0.0, 0.6, 0.5)),
            [2.0, 1.0, 0.25, 0.0],
        ),
        // A see-through cube using the pixel art's alpha.
        MeshInstance {
            color: [1.0, 1.0, 1.0, 0.99],
            texture: Some(WorldTexture::Asset(2)),
            ..mesh(
                "cube",
                Mat4::from_rotation_translation(
                    Quat::from_rotation_y(-0.4),
                    Vec3::new(1.6, 0.5, -0.5),
                ),
                [1.0; 4],
            )
        },
        // A texture that doesn't exist draws the magenta missing checker.
        MeshInstance {
            texture: Some(WorldTexture::Asset(99)),
            ..mesh(
                "cube",
                Mat4::from_scale_rotation_translation(
                    Vec3::splat(0.3),
                    Quat::IDENTITY,
                    Vec3::new(0.0, 1.8, 0.0),
                ),
                [1.0; 4],
            )
        },
    ];
    // Sprites: the pixel art upright, and flipped horizontally.
    let sprite = |x: f32, uv: [f32; 4]| WorldQuad {
        model: place(Vec3::new(x, 1.6, 1.0), Quat::IDENTITY, (0.8, 0.8)),
        color: [1.0; 4],
        texture: Some(WorldTexture::Asset(2)),
        uv,
        additive: false,
        order: 0,
    };
    v.quads = vec![
        sprite(-0.6, [0.0, 0.0, 1.0, 1.0]),
        sprite(0.6, [1.0, 0.0, -1.0, 1.0]),
    ];
    v.lights = vec![Light {
        kind: LightKind::Directional,
        color: [1.0, 0.97, 0.92],
        position: Vec3::ZERO,
        direction: Vec3::new(-0.4, -1.0, -0.5).normalize(),
        range: 0.0,
        shadows: None,
    }];
    world.render_views(&gpu, &rt, &[v], &[]);
    let pixels = target.read_pixels(&gpu).unwrap();
    assert_golden("M5-RND-08", size.0, size.1, &pixels, 3);
}

// M5-RND-09
#[test]
fn glows_bloom_and_spot_beams() {
    use iron_render::{
        BEAM_MESH, Bloom, DEFAULT_PARTICLE_TEXTURE, Light, LightKind, MeshGlow, MeshInstance,
        primitive_id,
    };
    let gpu = shared_gpu();
    let size = (320, 200);
    let target =
        OffscreenTarget::with_format(&gpu, size.0, size.1, wgpu::TextureFormat::Rgba8UnormSrgb);
    let view = target.view().unwrap();
    let rt = RenderTarget {
        view: &view,
        format: target.format(),
        size,
        scale: 1.0,
    };
    let eye = Vec3::new(0.0, 2.0, 6.0);
    let camera = WorldCamera::perspective(
        iron_core::glam::camera::rh::view::look_at_mat4(eye, Vec3::new(0.0, 1.0, 0.0), Vec3::Y)
            .inverse(),
        50.0,
        0.1,
        50.0,
        size,
        Color::rgb(0.01, 0.01, 0.02),
    );
    let mesh = |name: &str, model: Mat4, color: [f32; 4]| MeshInstance {
        mesh: primitive_id(name).unwrap(),
        model,
        color,
        emission: [0.0; 3],
        smoothness: 0.3,
        metallic: 0.0,
        unlit: false,
        texture: None,
        tiling: [1.0, 1.0, 0.0, 0.0],
        glow: MeshGlow::None,
        cast_shadows: true,
    };
    // A spot light shining down at an angle from the upper left, with its beam.
    let spot_at = Vec3::new(-1.6, 3.0, 0.0);
    let spot_dir = Vec3::new(0.45, -1.0, 0.0).normalize();
    let spot_rot = Quat::from_rotation_arc(Vec3::NEG_Z, spot_dir);
    let mut v = iron_render::WorldView::new(camera, [0.0, 0.0, 1.0, 1.0]);
    v.ambient = [0.05, 0.05, 0.06];
    v.meshes = vec![
        mesh(
            "plane",
            Mat4::from_scale(Vec3::splat(0.8)),
            [0.5, 0.5, 0.5, 1.0],
        ),
        // A hot emissive bar (brighter than white: it blooms) and a dull cube.
        MeshInstance {
            emission: [4.0, 1.2, 0.3],
            unlit: true,
            ..mesh(
                "cube",
                Mat4::from_scale_rotation_translation(
                    Vec3::new(1.2, 0.15, 0.15),
                    Quat::IDENTITY,
                    Vec3::new(1.3, 1.6, 0.0),
                ),
                [0.1, 0.1, 0.1, 1.0],
            )
        },
        mesh(
            "cube",
            Mat4::from_translation(Vec3::new(1.4, 0.5, -0.5)),
            [0.3, 0.5, 0.8, 1.0],
        ),
        // An additive sphere: it lights what's behind it.
        MeshInstance {
            glow: MeshGlow::Additive,
            unlit: true,
            ..mesh(
                "sphere",
                Mat4::from_scale_rotation_translation(
                    Vec3::splat(0.5),
                    Quat::IDENTITY,
                    Vec3::new(0.2, 1.8, 1.0),
                ),
                [0.2, 0.9, 0.4, 0.6],
            )
        },
        MeshInstance {
            mesh: BEAM_MESH,
            glow: MeshGlow::Beam,
            cast_shadows: false,
            unlit: true,
            ..mesh(
                "cube",
                Mat4::from_scale_rotation_translation(Vec3::new(1.2, 1.2, 3.5), spot_rot, spot_at),
                [1.0, 0.9, 0.7, 0.5],
            )
        },
    ];
    // Glowing additive spots of different sizes, one partly behind the dull cube.
    let glow = |at: Vec3, size: f32, color: [f32; 4]| WorldQuad {
        model: place(at, Quat::IDENTITY, (size, size)),
        color,
        texture: Some(WorldTexture::Asset(DEFAULT_PARTICLE_TEXTURE)),
        uv: [0.0, 0.0, 1.0, 1.0],
        additive: true,
        order: 0,
    };
    v.quads = vec![
        glow(Vec3::new(-0.6, 0.6, 0.8), 0.8, [1.0, 0.5, 0.2, 1.0]),
        glow(Vec3::new(-0.3, 0.9, 0.9), 0.4, [0.4, 0.6, 1.0, 1.0]),
        glow(Vec3::new(1.4, 0.9, -1.2), 1.4, [1.0, 0.2, 0.8, 1.0]),
    ];
    v.lights = vec![Light {
        kind: LightKind::Spot {
            inner_degrees: 25.0,
            outer_degrees: 38.0,
        },
        color: [8.0, 7.0, 5.5],
        position: spot_at,
        direction: spot_dir,
        range: 6.0,
        shadows: None,
    }];
    let mut world = WorldRenderer::new(&gpu);
    world.bloom = Some(Bloom {
        threshold: 0.9,
        intensity: 1.2,
        scatter: 0.7,
    });
    world.render_views(&gpu, &rt, &[v], &[]);
    let pixels = target.read_pixels(&gpu).unwrap();
    assert_golden("M5-RND-09", size.0, size.1, &pixels, 3);
}

// M5-RND-10
#[test]
fn directional_shadows() {
    use iron_render::{Light, LightKind, MeshGlow, MeshInstance, Shadows, primitive_id};
    let gpu = shared_gpu();
    let size = (320, 200);
    let target =
        OffscreenTarget::with_format(&gpu, size.0, size.1, wgpu::TextureFormat::Rgba8UnormSrgb);
    let view = target.view().unwrap();
    let rt = RenderTarget {
        view: &view,
        format: target.format(),
        size,
        scale: 1.0,
    };
    let eye = Vec3::new(0.0, 3.0, 6.5);
    let camera = WorldCamera::perspective(
        iron_core::glam::camera::rh::view::look_at_mat4(eye, Vec3::new(0.0, 0.5, 0.0), Vec3::Y)
            .inverse(),
        50.0,
        0.1,
        50.0,
        size,
        Color::rgb(0.03, 0.03, 0.05),
    );
    let mesh = |name: &str, model: Mat4, color: [f32; 4]| MeshInstance {
        mesh: primitive_id(name).unwrap(),
        model,
        color,
        emission: [0.0; 3],
        smoothness: 0.3,
        metallic: 0.0,
        unlit: false,
        texture: None,
        tiling: [1.0, 1.0, 0.0, 0.0],
        glow: MeshGlow::None,
        cast_shadows: true,
    };
    let mut v = iron_render::WorldView::new(camera, [0.0, 0.0, 1.0, 1.0]);
    v.ambient = [0.12, 0.12, 0.14];
    v.meshes = vec![
        mesh(
            "plane",
            Mat4::from_scale(Vec3::splat(0.8)),
            [0.7, 0.7, 0.72, 1.0],
        ),
        mesh(
            "cube",
            Mat4::from_rotation_translation(Quat::from_rotation_y(0.5), Vec3::new(-1.6, 0.5, 0.0)),
            [0.85, 0.35, 0.25, 1.0],
        ),
        mesh(
            "sphere",
            Mat4::from_translation(Vec3::new(0.0, 0.8, 0.3)),
            [0.3, 0.55, 0.9, 1.0],
        ),
        mesh(
            "capsule",
            Mat4::from_translation(Vec3::new(1.6, 1.0, -0.3)),
            [0.35, 0.8, 0.4, 1.0],
        ),
        // A floating plank: its shadow falls apart from it.
        mesh(
            "cube",
            Mat4::from_scale_rotation_translation(
                Vec3::new(1.4, 0.08, 0.5),
                Quat::IDENTITY,
                Vec3::new(0.2, 2.0, 1.4),
            ),
            [0.9, 0.85, 0.6, 1.0],
        ),
    ];
    v.lights = vec![Light {
        kind: LightKind::Directional,
        color: [1.0, 0.97, 0.9],
        position: Vec3::ZERO,
        direction: Vec3::new(-0.5, -1.0, -0.35).normalize(),
        range: 0.0,
        shadows: Some(Shadows {
            soft: true,
            strength: 0.9,
        }),
    }];
    let mut world = WorldRenderer::new(&gpu);
    world.shadow_distance = 14.0;
    world.render_views(&gpu, &rt, &[v], &[]);
    let pixels = target.read_pixels(&gpu).unwrap();
    assert_golden("M5-RND-10", size.0, size.1, &pixels, 3);
}

// M5-RND-11
#[test]
fn pbr_material_grid_under_a_sky() {
    use iron_render::{Light, LightKind, MeshGlow, MeshInstance, Sky, primitive_id};
    let gpu = shared_gpu();
    let size = (320, 200);
    let target =
        OffscreenTarget::with_format(&gpu, size.0, size.1, wgpu::TextureFormat::Rgba8UnormSrgb);
    let view = target.view().unwrap();
    let rt = RenderTarget {
        view: &view,
        format: target.format(),
        size,
        scale: 1.0,
    };
    let eye = Vec3::new(0.0, 1.6, 7.0);
    let camera = WorldCamera::perspective(
        iron_core::glam::camera::rh::view::look_at_mat4(eye, Vec3::new(0.0, 1.1, 0.0), Vec3::Y)
            .inverse(),
        42.0,
        0.1,
        100.0,
        size,
        Color::rgb(0.0, 0.0, 0.0),
    );
    let ball = |x: usize, y: usize| {
        let smoothness = x as f32 / 4.0;
        let metallic = y as f32 / 2.0;
        MeshInstance {
            mesh: primitive_id("sphere").unwrap(),
            model: Mat4::from_scale_rotation_translation(
                Vec3::splat(0.8),
                Quat::IDENTITY,
                Vec3::new(-2.0 + x as f32, 0.4 + y as f32 * 0.9, 0.0),
            ),
            color: [0.9, 0.55, 0.25, 1.0],
            emission: [0.0; 3],
            smoothness,
            metallic,
            unlit: false,
            texture: None,
            tiling: [1.0, 1.0, 0.0, 0.0],
            glow: MeshGlow::None,
            cast_shadows: true,
        }
    };
    let mut v = iron_render::WorldView::new(camera, [0.0, 0.0, 1.0, 1.0]);
    v.skybox = true;
    // Columns: rough to smooth; rows: dielectric, half, metal.
    v.meshes = (0..5)
        .flat_map(|x| (0..3).map(move |y| ball(x, y)))
        .collect();
    // A dark, fairly smooth floor reflecting the sky.
    v.meshes.push(MeshInstance {
        mesh: primitive_id("plane").unwrap(),
        model: Mat4::from_translation(Vec3::new(0.0, -0.01, 0.0)),
        color: [0.08, 0.08, 0.09, 1.0],
        smoothness: 0.85,
        metallic: 0.0,
        ..ball(0, 0)
    });
    v.lights = vec![Light {
        kind: LightKind::Directional,
        color: [2.0, 1.9, 1.7],
        position: Vec3::ZERO,
        direction: Vec3::new(-0.4, -0.6, -0.3).normalize(),
        range: 0.0,
        shadows: None,
    }];
    let mut world = WorldRenderer::new(&gpu);
    world.set_sky(Some(Sky::default()));
    world.render_views(&gpu, &rt, &[v], &[]);
    let pixels = target.read_pixels(&gpu).unwrap();
    assert_golden("M5-RND-11", size.0, size.1, &pixels, 3);
}
