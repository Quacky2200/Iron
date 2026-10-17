//! The 3D world pass (M5.a): planes through a perspective camera, with screen UI
//! blended over them.

use iron_core::glam::{Mat4, Quat, Vec3};
use iron_core::{Commit, Value};
use iron_render::testing::{assert_golden, shared_gpu};
use iron_render::{
    Color, OffscreenTarget, RenderTarget, UiRenderer, WorldCamera, WorldQuad, WorldRenderer,
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
            texture: Some(0),
            uv: [0.0, 0.0, 1.0, 1.0],
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
