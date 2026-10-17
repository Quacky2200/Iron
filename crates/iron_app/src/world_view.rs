//! What scripts put in the 3D world for the renderer: `Camera`, `PlaneRenderer`,
//! `MeshRenderer`, `SpriteRenderer`, `ParticleSystem` and `Light` components (`render.*`
//! ops, keyed by component id), read with their GameObjects' world transforms each frame, plus script
//! meshes (`render.mesh`) and textures (`render.texture`) waiting to be uploaded and
//! the ambient light.

use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use iron_core::Value;
use iron_core::glam::{Mat4, Quat, Vec3};
use iron_render::{
    BEAM_MESH, Bloom, Color, Light, LightKind, MeshData, MeshGlow, MeshInstance, Projection,
    TextureData, TextureFilter, TextureWrap, WorldCamera, WorldQuad, WorldTexture, WorldView,
    primitive_id, viewport_size,
};
use iron_scene::{ScriptObject, ScriptScene};

use crate::particles::{self, ParticleSystem};

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
    texture: Option<u32>,
    tiling: [f32; 4],
}

#[derive(Clone, Copy, Debug)]
struct Sprite {
    object: ScriptObject,
    texture: u32,
    /// x, y, width, height in pixels from the texture's bottom-left; `None`: all of it.
    rect: Option<[f32; 4]>,
    /// 0..1 across the rect from its bottom-left: where the object's origin sits.
    pivot: [f32; 2],
    pixels_per_unit: f32,
    color: [f32; 4],
    flip: (bool, bool),
    /// Higher draws over lower (Unity's `sortingOrder`).
    order: i32,
}

#[derive(Clone, Copy, Debug)]
struct LightComponent {
    object: ScriptObject,
    kind: LightKind,
    /// Colour × intensity.
    color: [f32; 3],
    range: f32,
    /// A glow drawn around the light this many metres across (0: none), Unity's halo.
    halo: f32,
    /// How bright a spot light's visible beam is (0: none).
    beam: f32,
}

pub struct WorldViews {
    cameras: BTreeMap<u32, Camera>,
    planes: BTreeMap<u32, Plane>,
    renderers: BTreeMap<u32, Renderer>,
    lights: BTreeMap<u32, LightComponent>,
    sprites: BTreeMap<u32, Sprite>,
    particles: BTreeMap<u32, ParticleSystem>,
    /// Bloom over the world (`render.bloom`), applied by the app before drawing.
    pub bloom: Option<Bloom>,
    /// `emit` requests waiting for the next particle update (which knows where objects are).
    pending_emits: Vec<(u32, u32)>,
    ambient: [f32; 3],
    /// Script meshes to upload (`None`: remove), applied by the app before drawing.
    pub mesh_uploads: Vec<(u32, Option<MeshData>)>,
    /// Textures to upload (`None`: remove), applied by the app before drawing.
    pub texture_uploads: Vec<(u32, Option<TextureData>)>,
    /// Loaded textures' sizes in pixels, for sprites.
    texture_sizes: HashMap<u32, (u32, u32)>,
}

impl Default for WorldViews {
    fn default() -> Self {
        Self {
            cameras: BTreeMap::new(),
            planes: BTreeMap::new(),
            renderers: BTreeMap::new(),
            lights: BTreeMap::new(),
            sprites: BTreeMap::new(),
            particles: BTreeMap::new(),
            bloom: None,
            pending_emits: Vec::new(),
            ambient: [0.2, 0.2, 0.22],
            mesh_uploads: Vec::new(),
            texture_uploads: Vec::new(),
            texture_sizes: HashMap::new(),
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

/// Where a sprite's quad goes in its object's space, and the part of the texture it
/// shows (origin and size in top-down UV space).
fn sprite_quad(s: &Sprite, (tw, th): (u32, u32)) -> (Mat4, [f32; 4]) {
    let (tw, th) = (tw as f32, th as f32);
    let [x, y, w, h] = s.rect.unwrap_or([0.0, 0.0, tw, th]);
    let ppu = s.pixels_per_unit.max(1e-6);
    let size = Vec3::new(w / ppu, h / ppu, 1.0);
    let offset = Vec3::new(
        (0.5 - s.pivot[0]) * size.x,
        (0.5 - s.pivot[1]) * size.y,
        0.0,
    );
    let mut uv = [x / tw, 1.0 - (y + h) / th, w / tw, h / th];
    if s.flip.0 {
        uv[0] += uv[2];
        uv[2] = -uv[2];
    }
    if s.flip.1 {
        uv[1] += uv[3];
        uv[3] = -uv[3];
    }
    (
        Mat4::from_scale_rotation_translation(size, Quat::IDENTITY, offset),
        uv,
    )
}

impl WorldViews {
    /// `texture id src filter wrap mipmaps [width height [r g b a ...]]`: loads image
    /// file `src` (relative to the game folder) or, with `src` null, takes the pixels
    /// given (sRGB bytes, top row first). `textureRemove id` drops one. Returns the
    /// loaded texture's id and size.
    pub fn texture_op(
        &mut self,
        name: &str,
        root: &Path,
        args: &[Value],
    ) -> Result<Option<(u32, u32, u32)>, String> {
        let id = args
            .first()
            .and_then(Value::as_f64)
            .filter(|v| v.is_finite() && *v >= 0.0)
            .ok_or("a texture needs an id")? as u32;
        if name == "textureRemove" {
            self.texture_sizes.remove(&id);
            self.texture_uploads.push((id, None));
            return Ok(None);
        }
        let filter = match args.get(2).and_then(Value::as_str) {
            Some(f) => TextureFilter::parse(f).ok_or_else(|| format!("unknown filter `{f}`"))?,
            None => TextureFilter::default(),
        };
        let wrap = match args.get(3).and_then(Value::as_str) {
            Some(w) => TextureWrap::parse(w).ok_or_else(|| format!("unknown wrap `{w}`"))?,
            None => TextureWrap::default(),
        };
        let mipmaps = !matches!(args.get(4), Some(Value::Bool(false)));
        let (width, height, rgba) = match args.get(1) {
            Some(Value::String(src)) => {
                let bytes = if src.trim_start().starts_with("data:") {
                    iron_core::decode_data_url(src).ok_or("bad data URL")?
                } else {
                    let path = iron_core::resolve_asset(root, src).ok_or_else(|| {
                        format!("`{src}` must be a relative path inside the game folder")
                    })?;
                    std::fs::read(&path).map_err(|e| format!("`{src}`: {e}"))?
                };
                let image = image::load_from_memory(&bytes)
                    .map_err(|e| format!("`{src}`: {e}"))?
                    .to_rgba8();
                let (w, h) = image.dimensions();
                (w, h, image.into_raw())
            }
            _ => {
                let w = args.get(5).and_then(Value::as_f64).unwrap_or(0.0) as u32;
                let h = args.get(6).and_then(Value::as_f64).unwrap_or(0.0) as u32;
                let px = flat(args.get(7)).ok_or("a texture needs a file or pixels")?;
                (
                    w,
                    h,
                    px.into_iter().map(|v| v.clamp(0.0, 255.0) as u8).collect(),
                )
            }
        };
        let data = TextureData {
            width,
            height,
            rgba,
            filter,
            wrap,
            mipmaps,
        };
        if !data.is_valid() {
            return Err(format!(
                "texture {id}: {width}x{height} doesn't match its pixels"
            ));
        }
        self.texture_sizes.insert(id, (width, height));
        self.texture_uploads.push((id, Some(data)));
        Ok(Some((id, width, height)))
    }

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
            // `meshRenderer component object mesh(name | id) [r g b a] [er eg eb] smoothness
            // unlit texture|null [scale x y, offset x y]`
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
                    texture: match args.get(7) {
                        Some(Value::Null) | None => None,
                        _ => Some(id_at(7)?),
                    },
                    tiling: nums::<4>(args.get(8)).unwrap_or([1.0, 1.0, 0.0, 0.0]),
                };
                self.renderers.insert(id, renderer);
            }
            // `sprite component object texture [x y w h]|null [pivot x y] pixelsPerUnit
            // [r g b a] flipX flipY sortingOrder`
            "sprite" => {
                let sprite = Sprite {
                    object: id_at(1)?,
                    texture: id_at(2)?,
                    rect: nums::<4>(args.get(3)),
                    pivot: nums::<2>(args.get(4)).unwrap_or([0.5, 0.5]),
                    pixels_per_unit: num(5).unwrap_or(100.0),
                    color: nums::<4>(args.get(6)).unwrap_or([1.0; 4]),
                    flip: (
                        matches!(args.get(7), Some(Value::Bool(true))),
                        matches!(args.get(8), Some(Value::Bool(true))),
                    ),
                    order: num(9).unwrap_or(0.0) as i32,
                };
                self.sprites.insert(id, sprite);
            }
            // `light component object type [r g b] intensity range spotAngle innerSpotAngle
            // haloSize beamIntensity`
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
                    halo: num(8).unwrap_or(0.0).max(0.0),
                    beam: num(9).unwrap_or(0.0).max(0.0),
                };
                self.lights.insert(id, light);
            }
            // `particles component object [key, value, ...] playOnAwake`: new settings keep
            // a system's particles.
            "particles" => {
                let Some(Value::Array(pairs)) = args.get(2) else {
                    return Err("particles needs [key, value, ...]".into());
                };
                let settings = particles::Settings::parse(pairs)?;
                match self.particles.get_mut(&id) {
                    Some(p) => p.set_settings(settings),
                    None => {
                        let mut p = ParticleSystem::new(id_at(1)?, settings);
                        if matches!(args.get(3), Some(Value::Bool(true))) {
                            p.play();
                        }
                        self.particles.insert(id, p);
                    }
                }
            }
            // `particleControl component play|stop|stopClear|pause|clear|emit [count]`
            "particleControl" => {
                let Some(p) = self.particles.get_mut(&id) else {
                    return Err(format!("no particle system {id}"));
                };
                match args.get(1).and_then(Value::as_str) {
                    Some("play") => p.play(),
                    Some("stop") => p.stop(),
                    Some("stopClear") => {
                        p.stop();
                        p.clear();
                    }
                    Some("pause") => p.pause(),
                    Some("clear") => p.clear(),
                    Some("emit") => {
                        // Emitted where the object is when the frame's particles update.
                        self.pending_emits
                            .push((id, num(2).unwrap_or(1.0).max(0.0) as u32));
                    }
                    other => return Err(format!("unknown particle action {other:?}")),
                }
            }
            // `bloom 0 threshold intensity scatter`; `bloom 0 null` turns it off.
            "bloom" => {
                self.bloom = match args.get(1) {
                    Some(Value::Null) | None => None,
                    _ => Some(Bloom {
                        threshold: num(1)?.max(0.0),
                        intensity: num(2)?.max(0.0),
                        scatter: num(3)?.clamp(0.0, 1.0),
                    }),
                };
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
                self.sprites.remove(&id);
                self.particles.remove(&id);
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
        self.sprites.retain(|_, s| !objects.contains(&s.object));
        self.particles.retain(|_, p| !objects.contains(&p.object));
    }

    /// Advances particle systems on active objects by `dt` (scaled) seconds, emitting
    /// from where their objects are now. Returns `[id, count, alive (0/1), ...]` for
    /// systems whose count or state changed, for `__iron_particles`.
    pub fn update_particles(&mut self, dt: f32, scene: &ScriptScene) -> Vec<f64> {
        let world = |o: ScriptObject| scene.object(o).map(|id| scene.scene.world_matrix(id));
        for (id, count) in self.pending_emits.drain(..) {
            if let Some(p) = self.particles.get_mut(&id)
                && let Some(w) = world(p.object)
            {
                p.emit(count, &w);
            }
        }
        for p in self.particles.values_mut() {
            if !scene.active(p.object) {
                continue;
            }
            if let Some(w) = world(p.object) {
                p.update(dt, &w);
            }
        }
        let mut changed = Vec::new();
        for (id, p) in &mut self.particles {
            if let Some((count, alive)) = p.take_change() {
                changed.extend([f64::from(*id), count as f64, f64::from(u8::from(alive))]);
            }
        }
        changed
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
            .chain(
                self.sprites
                    .values()
                    .filter(|s| scene.active(s.object))
                    .filter_map(|s| {
                        // Drawn once its texture has loaded.
                        let size = *self.texture_sizes.get(&s.texture)?;
                        let (local, uv) = sprite_quad(s, size);
                        Some(WorldQuad {
                            model: Mat4::from(world(s.object)?) * local,
                            color: s.color,
                            texture: Some(WorldTexture::Asset(s.texture)),
                            uv,
                            additive: false,
                            order: s.order,
                        })
                    }),
            )
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
                    texture: r.texture.map(WorldTexture::Asset),
                    tiling: r.tiling,
                    glow: MeshGlow::None,
                })
            })
            .collect();
        // Visible spot light beams: cones as long as the light's range.
        let meshes: Vec<MeshInstance> = meshes
            .into_iter()
            .chain(
                self.lights
                    .values()
                    .filter(|l| l.beam > 0.0 && scene.active(l.object))
                    .filter_map(|l| {
                        let LightKind::Spot { outer_degrees, .. } = l.kind else {
                            return None;
                        };
                        let (_, rotation, position) =
                            world(l.object)?.to_scale_rotation_translation();
                        let radius = l.range * (outer_degrees.min(170.0).to_radians() / 2.0).tan();
                        let [r, g, b] = l.color;
                        let peak = r.max(g).max(b).max(1e-6) / l.beam;
                        Some(MeshInstance {
                            mesh: BEAM_MESH,
                            model: Mat4::from_scale_rotation_translation(
                                Vec3::new(radius, radius, l.range),
                                rotation,
                                position,
                            ),
                            color: [r / peak, g / peak, b / peak, 1.0],
                            emission: [0.0; 3],
                            smoothness: 0.0,
                            unlit: true,
                            texture: None,
                            tiling: [1.0, 1.0, 0.0, 0.0],
                            glow: MeshGlow::Beam,
                        })
                    }),
            )
            .collect();
        // Light halos: a glow at each light, facing each camera.
        let halos: Vec<(Vec3, f32, [f32; 4])> = self
            .lights
            .values()
            .filter(|l| l.halo > 0.0 && scene.active(l.object))
            .filter_map(|l| {
                let [r, g, b] = l.color;
                // The light's hue at a steady brightness, whatever its intensity.
                let peak = r.max(g).max(b).max(1e-6);
                let position = world(l.object)?.translation.into();
                Some((position, l.halo, [r / peak, g / peak, b / peak, 0.8]))
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
                // Particles and halos face this camera.
                let mut quads = quads.clone();
                for p in self.particles.values().filter(|p| scene.active(p.object)) {
                    if let Some(w) = world(p.object) {
                        p.quads(&w, rotation, &mut quads);
                    }
                }
                for &(at, size, color) in &halos {
                    // Pulled half its size towards the camera, so walls and panes near
                    // the light don't slice the glow in two.
                    let towards = (position - at).normalize_or_zero();
                    let at = at + towards * (size * 0.5).min(at.distance(position) * 0.5);
                    quads.push(WorldQuad {
                        model: Mat4::from_scale_rotation_translation(
                            Vec3::new(size, size, 1.0),
                            rotation,
                            at,
                        ),
                        color,
                        texture: Some(WorldTexture::Asset(particles::DEFAULT_TEXTURE)),
                        uv: [0.0, 0.0, 1.0, 1.0],
                        additive: true,
                        order: 0,
                    });
                }
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
                    quads,
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sprite(rect: Option<[f32; 4]>, pivot: [f32; 2], flip: (bool, bool)) -> Sprite {
        Sprite {
            object: 1,
            texture: 1,
            rect,
            pivot,
            pixels_per_unit: 16.0,
            color: [1.0; 4],
            flip,
            order: 0,
        }
    }

    // M5-TEX-02
    #[test]
    fn sprites_are_sized_by_pixels_per_unit_and_sit_on_their_pivot() {
        // A 16 × 32 cell from the bottom-right of a 64 × 64 sheet, pivot at its feet.
        let (m, uv) = sprite_quad(
            &sprite(Some([48.0, 0.0, 16.0, 32.0]), [0.5, 0.0], (false, false)),
            (64, 64),
        );
        let (scale, _, pos) = m.to_scale_rotation_translation();
        assert_eq!(scale, Vec3::new(1.0, 2.0, 1.0));
        // The quad's centre is a metre above the origin, so its bottom edge is on it.
        assert_eq!(pos, Vec3::new(0.0, 1.0, 0.0));
        // Bottom-up pixels become top-down UVs: rows 0..32 from the bottom are v 0.5..1.
        assert_eq!(uv, [0.75, 0.5, 0.25, 0.5]);
        let (_, flipped) = sprite_quad(&sprite(None, [0.5, 0.5], (true, true)), (64, 64));
        assert_eq!(flipped, [1.0, 1.0, -1.0, -1.0]);
    }

    // M5-TEX-01
    #[test]
    fn textures_from_pixels_queue_uploads_and_report_their_size() {
        let mut views = WorldViews::default();
        let root = Path::new(".");
        let px = Value::Array((0..8).map(|v| Value::from(f64::from(v))).collect());
        let args = [
            Value::from(4.0),
            Value::Null,
            Value::String("point".into()),
            Value::String("clamp".into()),
            Value::Bool(false),
            Value::from(1.0),
            Value::from(2.0),
            px,
        ];
        assert_eq!(
            views.texture_op("texture", root, &args),
            Ok(Some((4, 1, 2)))
        );
        let (id, data) = views.texture_uploads.pop().unwrap();
        let data = data.unwrap();
        assert_eq!(
            (id, data.filter, data.wrap, data.mipmaps),
            (4, TextureFilter::Point, TextureWrap::Clamp, false)
        );
        // Pixels that don't fill the size are refused, as are paths outside the game.
        let mut bad = args.clone();
        bad[6] = Value::from(3.0);
        assert!(views.texture_op("texture", root, &bad).is_err());
        let mut outside = args.clone();
        outside[1] = Value::String("../secret.png".into());
        assert!(views.texture_op("texture", root, &outside).is_err());
        assert_eq!(
            views.texture_op("textureRemove", root, &[Value::from(4.0)]),
            Ok(None)
        );
        assert!(matches!(views.texture_uploads.pop(), Some((4, None))));
    }
}
