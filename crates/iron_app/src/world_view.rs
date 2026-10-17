//! What scripts put in the 3D world for the renderer: `Camera`, `PlaneRenderer`,
//! `MeshRenderer` and `Light` components (`render.*` ops, keyed by component id), read
//! with their GameObjects' world transforms each frame, plus script meshes
//! (`render.mesh`) waiting to be uploaded and the ambient light.

use std::collections::BTreeMap;

use iron_core::Value;
use iron_core::glam::{Mat4, Quat, Vec3};
use iron_render::{
    Color, Light, LightKind, MeshData, MeshInstance, Projection, WorldCamera, WorldQuad, WorldView,
    primitive_id, viewport_size,
};
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

#[derive(Clone, Copy, Debug)]
struct Renderer {
    object: ScriptObject,
    /// A primitive's id or a script mesh's.
    mesh: u32,
    color: [f32; 4],
    emission: [f32; 3],
    smoothness: f32,
    unlit: bool,
}

#[derive(Clone, Copy, Debug)]
struct LightComponent {
    object: ScriptObject,
    kind: LightKind,
    /// Colour × intensity.
    color: [f32; 3],
    range: f32,
}

pub struct WorldViews {
    cameras: BTreeMap<u32, Camera>,
    planes: BTreeMap<u32, Plane>,
    renderers: BTreeMap<u32, Renderer>,
    lights: BTreeMap<u32, LightComponent>,
    ambient: [f32; 3],
    /// Script meshes to upload (`None`: remove), applied by the app before drawing.
    pub mesh_uploads: Vec<(u32, Option<MeshData>)>,
}

impl Default for WorldViews {
    fn default() -> Self {
        Self {
            cameras: BTreeMap::new(),
            planes: BTreeMap::new(),
            renderers: BTreeMap::new(),
            lights: BTreeMap::new(),
            ambient: [0.2, 0.2, 0.22],
            mesh_uploads: Vec::new(),
        }
    }
}

/// A flat number array (`None` if it isn't one).
fn flat(v: Option<&Value>) -> Option<Vec<f32>> {
    match v {
        Some(Value::Array(items)) => items.iter().map(|x| x.as_f64().map(|n| n as f32)).collect(),
        _ => None,
    }
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
            // `mesh id [positions xyz...] [normals] [uvs] [indices]`; `mesh id null` removes it.
            "mesh" => {
                if matches!(args.get(1), Some(Value::Null) | None) {
                    self.mesh_uploads.push((id, None));
                    return Ok(());
                }
                let positions = flat(args.get(1)).ok_or("mesh needs positions")?;
                let normals = flat(args.get(2)).unwrap_or_default();
                let uvs = flat(args.get(3)).unwrap_or_default();
                let indices = flat(args.get(4)).ok_or("mesh needs indices")?;
                let data = MeshData {
                    positions: positions.as_chunks::<3>().0.to_vec(),
                    normals: normals.as_chunks::<3>().0.to_vec(),
                    uvs: uvs.as_chunks::<2>().0.to_vec(),
                    indices: indices.into_iter().map(|i| i as u32).collect(),
                }
                .complete();
                if !data.is_valid() {
                    return Err(format!(
                        "mesh {id}: indices out of range or not whole triangles"
                    ));
                }
                self.mesh_uploads.push((id, Some(data)));
            }
            // `meshRenderer component object mesh(name | id) [r g b a] [er eg eb] smoothness unlit`
            "meshRenderer" => {
                let mesh = match args.get(2) {
                    Some(Value::String(name)) => {
                        primitive_id(name).ok_or_else(|| format!("no primitive `{name}`"))?
                    }
                    _ => id_at(2)?,
                };
                let renderer = Renderer {
                    object: id_at(1)?,
                    mesh,
                    color: nums::<4>(args.get(3)).unwrap_or([1.0; 4]),
                    emission: nums::<3>(args.get(4)).unwrap_or([0.0; 3]),
                    smoothness: num(5).unwrap_or(0.5),
                    unlit: matches!(args.get(6), Some(Value::Bool(true))),
                };
                self.renderers.insert(id, renderer);
            }
            // `light component object type [r g b] intensity range spotAngle innerSpotAngle`
            "light" => {
                let kind = match args.get(2).and_then(Value::as_str) {
                    Some("directional") => LightKind::Directional,
                    Some("point") => LightKind::Point,
                    Some("spot") => LightKind::Spot {
                        outer_degrees: num(6).unwrap_or(30.0),
                        inner_degrees: num(7).unwrap_or(21.8),
                    },
                    other => return Err(format!("unknown light type {other:?}")),
                };
                let [r, g, b] = nums::<3>(args.get(3)).unwrap_or([1.0; 3]);
                let intensity = num(4).unwrap_or(1.0);
                let light = LightComponent {
                    object: id_at(1)?,
                    kind,
                    color: [r * intensity, g * intensity, b * intensity],
                    range: num(5).unwrap_or(10.0),
                };
                self.lights.insert(id, light);
            }
            // `ambient 0 [r g b]`
            "ambient" => {
                self.ambient = nums::<3>(args.get(1)).ok_or("ambient needs [r, g, b]")?;
            }
            "remove" => {
                self.cameras.remove(&id);
                self.planes.remove(&id);
                self.renderers.remove(&id);
                self.lights.remove(&id);
            }
            other => return Err(format!("unknown render op `{other}`")),
        }
        Ok(())
    }

    /// Forgets components of destroyed objects.
    pub fn forget_objects(&mut self, objects: &[ScriptObject]) {
        self.cameras.retain(|_, c| !objects.contains(&c.object));
        self.planes.retain(|_, p| !objects.contains(&p.object));
        self.renderers.retain(|_, r| !objects.contains(&r.object));
        self.lights.retain(|_, l| !objects.contains(&l.object));
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
        let meshes: Vec<MeshInstance> = self
            .renderers
            .values()
            .filter(|r| scene.active(r.object))
            .filter_map(|r| {
                Some(MeshInstance {
                    mesh: r.mesh,
                    model: Mat4::from(world(r.object)?),
                    color: r.color,
                    emission: r.emission,
                    smoothness: r.smoothness,
                    unlit: r.unlit,
                    texture: None,
                })
            })
            .collect();
        let lights: Vec<Light> = self
            .lights
            .values()
            .filter(|l| scene.active(l.object))
            .filter_map(|l| {
                let (_, rotation, position) = world(l.object)?.to_scale_rotation_translation();
                Some(Light {
                    kind: l.kind,
                    color: l.color,
                    position,
                    // Lights shine along their object's forward (-z), like Unity's.
                    direction: rotation * Vec3::NEG_Z,
                    range: l.range,
                })
            })
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
                    meshes: meshes.clone(),
                    lights: lights.clone(),
                    ambient: self.ambient,
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
