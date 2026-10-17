//! What scripts put in the 3D world for the renderer: `Camera` and `PlaneRenderer`
//! components (`render.*` ops, keyed by component id), read with their GameObjects'
//! world transforms each frame.

use std::collections::BTreeMap;

use iron_core::Value;
use iron_core::glam::{Mat4, Quat, Vec3};
use iron_render::{Color, Projection, WorldCamera, WorldQuad, WorldView, viewport_size};
use iron_scene::{ScriptObject, ScriptScene};

#[derive(Clone, Copy, Debug)]
struct Camera {
    object: ScriptObject,
    projection: Projection,
    near: f32,
    far: f32,
    /// Linear RGBA.
    clear: [f32; 4],
    /// Draw order: lower first, so higher ones draw over them (Unity's `depth`).
    depth: f32,
    /// x, y, width, height from the top-left, 0 to 1.
    rect: [f32; 4],
}

#[derive(Clone, Copy, Debug)]
struct Plane {
    object: ScriptObject,
    size: (f32, f32),
    color: [f32; 4],
}

#[derive(Default)]
pub struct WorldViews {
    cameras: BTreeMap<u32, Camera>,
    planes: BTreeMap<u32, Plane>,
}

fn nums<const N: usize>(v: Option<&Value>) -> Option<[f32; N]> {
    let Some(Value::Array(items)) = v else {
        return None;
    };
    let mut out = [0.0; N];
    for (i, slot) in out.iter_mut().enumerate() {
        *slot = items.get(i)?.as_f64()? as f32;
    }
    Some(out)
}

/// Lays a unit quad (facing +z) flat in an object's XZ plane, facing up (+y) like
/// Unity's Plane, `size` across.
pub fn lay_flat(size: (f32, f32)) -> Mat4 {
    Mat4::from_scale_rotation_translation(
        Vec3::new(size.0, size.1, 1.0),
        Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2),
        Vec3::ZERO,
    )
}

impl WorldViews {
    /// Runs one `render.*` op.
    pub fn apply(&mut self, name: &str, args: &[Value]) -> Result<(), String> {
        let num = |i: usize| -> Result<f32, String> {
            args.get(i)
                .and_then(Value::as_f64)
                .map(|v| v as f32)
                .filter(|v| v.is_finite())
                .ok_or_else(|| format!("argument {i} must be a number"))
        };
        // Ids through `f64`: `f32` can't hold integers past 2^24.
        let id_at = |i: usize| -> Result<u32, String> {
            args.get(i)
                .and_then(Value::as_f64)
                .filter(|v| v.is_finite() && *v >= 0.0)
                .map(|v| v as u32)
                .ok_or_else(|| format!("argument {i} must be an id"))
        };
        let id = id_at(0)?;
        match name {
            // `camera component object fov near far [r g b a] depth orthographic
            // orthographicSize [x y w h]`
            "camera" => {
                let orthographic = matches!(args.get(7), Some(Value::Bool(true)));
                let projection = if orthographic {
                    Projection::Orthographic {
                        half_height: num(8).unwrap_or(5.0),
                    }
                } else {
                    Projection::Perspective {
                        fov_y_degrees: num(2)?,
                    }
                };
                let camera = Camera {
                    object: id_at(1)?,
                    projection,
                    near: num(3)?,
                    far: num(4)?,
                    clear: nums::<4>(args.get(5)).unwrap_or([0.05, 0.06, 0.09, 1.0]),
                    depth: num(6).unwrap_or(0.0),
                    rect: nums::<4>(args.get(9)).unwrap_or([0.0, 0.0, 1.0, 1.0]),
                };
                self.cameras.insert(id, camera);
            }
            // `plane component object [w h] [r g b a]`
            "plane" => {
                let [w, h] = nums::<2>(args.get(2)).ok_or("plane needs a size")?;
                let plane = Plane {
                    object: id_at(1)?,
                    size: (w, h),
                    color: nums::<4>(args.get(3)).unwrap_or([1.0; 4]),
                };
                self.planes.insert(id, plane);
            }
            "remove" => {
                self.cameras.remove(&id);
                self.planes.remove(&id);
            }
            other => return Err(format!("unknown render op `{other}`")),
        }
        Ok(())
    }

    /// Forgets components of destroyed objects.
    pub fn forget_objects(&mut self, objects: &[ScriptObject]) {
        self.cameras.retain(|_, c| !objects.contains(&c.object));
        self.planes.retain(|_, p| !objects.contains(&p.object));
    }

    /// Every active camera's view (lowest depth first, then the order they were added),
    /// each seeing the planes and `extra` quads; empty without cameras (then only screen
    /// UI is drawn).
    pub fn views(
        &self,
        scene: &ScriptScene,
        size: (u32, u32),
        extra: &[WorldQuad],
    ) -> Vec<WorldView> {
        let world = |o: ScriptObject| scene.object(o).map(|id| scene.scene.world_matrix(id));
        let quads: Vec<WorldQuad> = self
            .planes
            .values()
            .filter(|p| scene.active(p.object))
            .filter_map(|p| {
                Some(WorldQuad::colored(
                    Mat4::from(world(p.object)?) * lay_flat(p.size),
                    p.color,
                ))
            })
            .chain(extra.iter().copied())
            .collect();
        let mut cameras: Vec<&Camera> = self
            .cameras
            .values()
            .filter(|c| scene.active(c.object))
            .collect();
        cameras.sort_by(|a, b| a.depth.total_cmp(&b.depth));
        cameras
            .into_iter()
            .filter_map(|camera| {
                // Cameras ignore scale: only where they are and which way they face.
                let (_, rotation, position) = world(camera.object)?.to_scale_rotation_translation();
                let [r, g, b, a] = camera.clear;
                let clear = Color {
                    r: r as f64,
                    g: g as f64,
                    b: b as f64,
                    a: a as f64,
                };
                Some(WorldView {
                    camera: WorldCamera::new(
                        Mat4::from_rotation_translation(rotation, position),
                        camera.projection,
                        camera.near,
                        camera.far,
                        viewport_size(camera.rect, size),
                        clear,
                    ),
                    viewport: camera.rect,
                    quads: quads.clone(),
                })
            })
            .collect()
    }
}
