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
    ALL_LAYERS, Anisotropic, BEAM_MESH, Bloom, Color, Grading, Light, LightKind, MaterialMaps,
    MeshData, MeshGlow, MeshInstance, Mirror, Projection, ReflectionProbe, RenderMode,
    ShadowQuality, Shadows, Sky, TextureData, TextureFilter, TextureWrap, Tonemapping, WorldCamera,
    WorldQuad, WorldTexture, WorldView, primitive_id, viewport_size,
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
    /// Shows the sky behind everything (Unity's skybox clear flag).
    skybox: bool,
    /// The layers it draws, a bit each (Unity's `cullingMask`).
    culling_mask: u32,
    /// Per layer: things farther than this aren't drawn (0: only the far plane).
    cull_distances: [f32; 32],
    /// Cull distances measured straight from the camera, not along its view.
    cull_spherical: bool,
    /// FXAA over the screen (Unity's camera `antialiasing`).
    fxaa: bool,
    /// Draws into this render texture instead of the screen (`targetTexture`).
    target: Option<u32>,
}

impl Camera {
    /// The fraction (0 to 1, past 1 when bigger) of the view's height something `size`
    /// metres across takes up `distance` away, as LOD groups measure it.
    fn screen_height(&self, size: f32, distance: f32) -> f32 {
        match self.projection {
            Projection::Perspective { fov_y_degrees } => {
                let half = (fov_y_degrees.clamp(1.0, 179.0).to_radians() / 2.0).tan();
                size / (2.0 * distance.max(1e-4) * half)
            }
            Projection::Orthographic { half_height } => size / (2.0 * half_height.max(1e-4)),
        }
    }

    /// Whether this camera at `position` facing `forward` draws something on `layer` at `at`.
    fn shows(&self, layer: u8, at: Vec3, position: Vec3, forward: Vec3) -> bool {
        if self.culling_mask & (1 << layer) == 0 {
            return false;
        }
        let limit = self.cull_distances[layer as usize];
        if limit <= 0.0 {
            return true;
        }
        let distance = if self.cull_spherical {
            at.distance(position)
        } else {
            (at - position).dot(forward)
        };
        distance <= limit
    }
}

/// Something drawn for an object: its layer and where it is, for cameras to cull.
struct Tagged<T> {
    layer: u8,
    at: Vec3,
    item: T,
    /// The MeshRenderer it came from, for LOD groups.
    renderer: Option<u32>,
}

impl<T> Tagged<T> {
    /// `None` for unknown objects.
    fn new(scene: &ScriptScene, object: ScriptObject, item: T) -> Option<Self> {
        let o = scene.object(object)?;
        Some(Self {
            layer: scene.layer(object),
            at: scene.scene.world_matrix(o).translation.into(),
            item,
            renderer: None,
        })
    }
}

/// A `LODGroup`: renderers per level of detail, the most detailed first, picked by how
/// tall the group looks on screen.
#[derive(Clone, Debug)]
struct LodGroup {
    object: ScriptObject,
    /// Its size in metres before the object's scale (Unity's `size`).
    size: f32,
    /// Per level: the screen height fraction (0 to 1) below which the next one takes
    /// over; below the last, nothing is drawn.
    heights: Vec<f32>,
    /// Per level: its MeshRenderers' component ids.
    renderers: Vec<Vec<u32>>,
    /// A level shown whatever the distance (`ForceLOD`).
    forced: Option<usize>,
}

impl LodGroup {
    /// The level to show at a relative screen height (`None`: culled).
    fn level(&self, height: f32) -> Option<usize> {
        if let Some(f) = self.forced {
            return Some(f.min(self.renderers.len().saturating_sub(1)));
        }
        self.heights.iter().position(|&h| height >= h)
    }
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
    metallic: f32,
    unlit: bool,
    texture: Option<u32>,
    tiling: [f32; 4],
    cast_shadows: bool,
    extras: MaterialExtras,
}

/// A skinned renderer's bones (objects, in the mesh's joint order) and their inverse
/// bind poses.
#[derive(Clone, Debug)]
struct Skin {
    bones: Vec<ScriptObject>,
    bindposes: Vec<Mat4>,
}

/// A material's settings past the basic ones.
#[derive(Clone, Copy, Debug, Default)]
struct MaterialExtras {
    maps: MaterialMaps,
    specular: Option<[f32; 3]>,
    mode: RenderMode,
    alpha_cutoff: f32,
    double_sided: bool,
}

/// A material's maps and modes from `meshRenderer`'s last argument:
/// `[normal|null normalScale metallicGloss|null gltfPacking occlusion|null
/// occlusionStrength emission|null detail|null [detail sx sy ox oy] [specular r g b]|null
/// mode(opaque|cutout|fade|transparent) alphaCutoff doubleSided]`.
fn material_extras(v: Option<&Value>) -> Result<MaterialExtras, String> {
    let Some(Value::Array(a)) = v else {
        return Ok(MaterialExtras {
            alpha_cutoff: 0.5,
            ..Default::default()
        });
    };
    let texture = |i: usize| -> Option<WorldTexture> {
        a.get(i)
            .and_then(Value::as_f64)
            .filter(|v| v.is_finite() && *v >= 0.0)
            .map(|v| WorldTexture::Asset(v as u32))
    };
    let number = |i: usize, default: f32| {
        a.get(i)
            .and_then(Value::as_f64)
            .map(|v| v as f32)
            .filter(|v| v.is_finite())
            .unwrap_or(default)
    };
    let maps = MaterialMaps {
        normal: texture(0),
        normal_scale: number(1, 1.0),
        metallic_gloss: texture(2),
        gltf_packing: matches!(a.get(3), Some(Value::Bool(true))),
        occlusion: texture(4),
        occlusion_strength: number(5, 1.0),
        emission: texture(6),
        detail: texture(7),
        detail_tiling: nums::<4>(a.get(8)).unwrap_or([1.0, 1.0, 0.0, 0.0]),
        reflection: None,
    };
    let mode = match a.get(10).and_then(Value::as_str) {
        None | Some("opaque") => RenderMode::Opaque,
        Some("cutout") => RenderMode::Cutout,
        Some("fade") => RenderMode::Fade,
        Some("transparent") => RenderMode::Transparent,
        Some(other) => return Err(format!("unknown rendering mode `{other}`")),
    };
    Ok(MaterialExtras {
        maps,
        specular: nums::<3>(a.get(9)),
        mode,
        alpha_cutoff: number(11, 0.5),
        double_sided: matches!(a.get(12), Some(Value::Bool(true))),
    })
}

/// Rendering quality scripts chose (`QualitySettings`), applied to the renderer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Quality {
    pub msaa: u32,
    pub render_scale: f32,
    pub shadows: ShadowQuality,
    pub soft_radius: u32,
    /// LOD groups pick levels as if things were this many times taller on screen.
    pub lod_bias: f32,
    /// Frames wait for the display's refresh (`QualitySettings.vSyncCount` > 0).
    pub vsync: bool,
    pub anisotropic: Anisotropic,
    /// Top mips skipped (`QualitySettings.globalTextureMipmapLimit`).
    pub texture_mip_limit: u32,
}

impl Default for Quality {
    fn default() -> Self {
        Self {
            msaa: 1,
            render_scale: 1.0,
            shadows: ShadowQuality::All,
            soft_radius: 1,
            lod_bias: 1.0,
            vsync: true,
            anisotropic: Anisotropic::Enable,
            texture_mip_limit: 0,
        }
    }
}

/// A `ReflectionProbe` component.
#[derive(Clone, Copy, Debug)]
struct Probe {
    object: ScriptObject,
    half_extents: Vec3,
    resolution: u32,
    box_projection: bool,
    every_frame: bool,
    culling_mask: u32,
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
    shadows: Option<Shadows>,
    culling_mask: u32,
}

pub struct WorldViews {
    cameras: BTreeMap<u32, Camera>,
    planes: BTreeMap<u32, Plane>,
    renderers: BTreeMap<u32, Renderer>,
    lights: BTreeMap<u32, LightComponent>,
    sprites: BTreeMap<u32, Sprite>,
    /// Planar mirrors: their objects, the local normals of their planes and the layers
    /// they reflect.
    mirrors: BTreeMap<u32, (ScriptObject, Vec3, u32)>,
    /// Reflection probes by component.
    probes: BTreeMap<u32, Probe>,
    lod_groups: BTreeMap<u32, LodGroup>,
    /// Probes asked to render on the next frame (`render.probeRender`, or just added).
    probe_requests: std::collections::HashSet<u32>,
    particles: BTreeMap<u32, ParticleSystem>,
    /// Bloom over the world (`render.bloom`), applied by the app before drawing.
    pub bloom: Option<Bloom>,
    /// Shadow distance (metres), map size and the sun's cascades (`render.shadowSettings`).
    pub shadow_settings: (f32, u32, u32),
    /// Anti-aliasing, render scale and shadow quality (`render.quality`).
    pub quality: Quality,
    /// Exposure and tonemapping (`render.post`).
    pub grading: Grading,
    /// The sky (`render.sky`), applied by the app before drawing.
    pub sky: Option<Sky>,
    /// Reflection / sky light intensity, and whether ambient light comes from the sky.
    pub environment: (f32, bool),
    /// `emit` requests waiting for the next particle update (which knows where objects are).
    pending_emits: Vec<(u32, u32)>,
    ambient: [f32; 3],
    /// Script meshes to upload (`None`: remove), applied by the app before drawing.
    pub mesh_uploads: Vec<(u32, Option<MeshData>)>,
    /// Every script and model mesh's data, for mesh colliders (`MeshCollider.sharedMesh`).
    pub mesh_data: HashMap<u32, MeshData>,
    /// Model meshes and textures get engine ids from here.
    model_ids: crate::models::EngineIds,
    /// Models by file (asset key): the path scripts gave and the engine ids they got.
    model_files: HashMap<String, (String, Vec<u32>)>,
    /// Textures loaded from files: their file (asset key) and op arguments, by id.
    texture_files: HashMap<u32, (String, Vec<Value>)>,
    /// Textures to upload (`None`: remove), applied by the app before drawing.
    pub texture_uploads: Vec<(u32, Option<TextureData>)>,
    /// Render textures to make or resize, with their sizes.
    pub render_texture_uploads: Vec<(u32, (u32, u32))>,
    /// Render textures' sizes, for their cameras' aspect.
    render_textures: HashMap<u32, (u32, u32)>,
    /// Skinned renderers' bones, by component.
    skins: HashMap<u32, Skin>,
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
            mirrors: BTreeMap::new(),
            probes: BTreeMap::new(),
            lod_groups: BTreeMap::new(),
            probe_requests: std::collections::HashSet::new(),
            particles: BTreeMap::new(),
            bloom: None,
            shadow_settings: (25.0, 2048, 1),
            quality: Quality::default(),
            grading: Grading::default(),
            sky: None,
            environment: (1.0, true),
            pending_emits: Vec::new(),
            ambient: [0.2, 0.2, 0.22],
            mesh_uploads: Vec::new(),
            mesh_data: HashMap::new(),
            model_ids: crate::models::EngineIds::default(),
            model_files: HashMap::new(),
            texture_files: HashMap::new(),
            texture_uploads: Vec::new(),
            render_texture_uploads: Vec::new(),
            render_textures: HashMap::new(),
            skins: HashMap::new(),
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

/// A culling mask (through `f64`, so all 32 bits survive); missing or null: every layer.
fn mask(v: Option<&Value>) -> u32 {
    v.and_then(Value::as_f64).map_or(ALL_LAYERS, |m| m as u32)
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
    /// `model id src`: loads a glTF model, queuing its meshes and textures for upload.
    /// Returns its description for scripts.
    pub fn model_op(&mut self, root: &Path, args: &[Value]) -> Result<serde_json::Value, String> {
        let src = args
            .get(1)
            .and_then(Value::as_str)
            .ok_or("a model needs a path")?;
        self.load_model(root, src, &[])
    }

    /// Loads a model, giving it `reuse`'s ids first, and remembers its file and ids.
    fn load_model(
        &mut self,
        root: &Path,
        src: &str,
        reuse: &[u32],
    ) -> Result<serde_json::Value, String> {
        self.model_ids.record(reuse);
        let model = crate::models::load(root, src, &mut self.model_ids);
        let ids = self.model_ids.take_recorded();
        let model = model?;
        self.model_files
            .insert(crate::reload::asset_key(src), (src.to_owned(), ids));
        for (id, data) in model.meshes {
            self.mesh_data.insert(id, data.clone());
            self.mesh_uploads.push((id, Some(data)));
        }
        for (id, data) in model.textures {
            self.texture_sizes.insert(id, (data.width, data.height));
            self.texture_uploads.push((id, Some(data)));
        }
        Ok(model.description)
    }

    /// Loads again what came from a changed file (textures, models). Returns textures'
    /// new sizes and models' new descriptions for scripts. A model whose parts changed
    /// in number keeps its old ids for the first ones; placed copies keep their old
    /// materials and nodes (place it again to see those).
    #[allow(clippy::type_complexity)]
    pub fn reload_asset(
        &mut self,
        root: &Path,
        key: &str,
    ) -> (Vec<(u32, u32, u32)>, Vec<(String, serde_json::Value)>) {
        let mut sizes = Vec::new();
        let textures: Vec<(u32, Vec<Value>)> = self
            .texture_files
            .iter()
            .filter(|(_, (k, _))| k == key)
            .map(|(id, (_, args))| (*id, args.clone()))
            .collect();
        for (id, args) in textures {
            match self.texture_op("texture", root, &args) {
                Ok(Some(size)) => sizes.push(size),
                Ok(None) => {}
                Err(e) => tracing::warn!("reloading texture {id}: {e}"),
            }
        }
        let mut models = Vec::new();
        if let Some((src, ids)) = self.model_files.get(key).cloned() {
            match self.load_model(root, &src, &ids) {
                Ok(description) => models.push((src, description)),
                Err(e) => tracing::warn!("reloading model `{src}`: {e}"),
            }
        }
        (sizes, models)
    }

    /// A texture that failed to load: sprites using it draw the renderer's missing
    /// checker (an 8 × 8 texel image) in its place.
    pub fn mark_missing(&mut self, id: u32) {
        self.texture_sizes.insert(id, (8, 8));
    }

    /// Rewrites a `physics.collider` op whose shape is `renderMesh` (`[mesh id, convex
    /// 0/1, scale x y z]`) into a `mesh` or `convex` one with that mesh's data.
    pub fn resolve_mesh_collider(&self, args: &mut [Value]) -> Result<(), String> {
        if args.get(2).and_then(Value::as_str) != Some("renderMesh") {
            return Ok(());
        }
        let dims = flat(args.get(3)).ok_or("renderMesh needs [mesh, convex, sx, sy, sz]")?;
        let &[_, convex, sx, sy, sz] = dims
            .first_chunk::<5>()
            .ok_or("renderMesh needs [mesh, convex, sx, sy, sz]")?;
        // The id through `f64`: engine ids (from 2^30) don't fit an `f32`.
        let Some(Value::Array(raw)) = args.get(3) else {
            unreachable!("checked above")
        };
        let mesh = raw.first().and_then(Value::as_f64).unwrap_or(0.0) as u32;
        let data = self
            .mesh_data
            .get(&mesh)
            .ok_or_else(|| format!("no mesh {mesh} for a mesh collider"))?;
        let points: Vec<Value> = data
            .positions
            .iter()
            .flat_map(|p| [p[0] * sx, p[1] * sy, p[2] * sz])
            .map(|v| Value::from(f64::from(v)))
            .collect();
        args[2] = Value::String(if convex > 0.5 { "convex" } else { "mesh" }.into());
        args[3] = Value::Array(points);
        if convex <= 0.5 {
            // The triangles go with the collider's other options.
            let indices = Value::Array(data.indices.iter().map(|&i| Value::from(i)).collect());
            match args.get_mut(6) {
                Some(Value::Array(opts)) => {
                    opts.resize(6, Value::Null);
                    opts[5] = indices;
                }
                _ => return Err("collider options missing".into()),
            }
        }
        Ok(())
    }

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
            self.texture_files.remove(&id);
            self.texture_sizes.remove(&id);
            self.render_textures.remove(&id);
            self.texture_uploads.push((id, None));
            return Ok(None);
        }
        // Textures from files reload when the file changes.
        if let Some(Value::String(src)) = args.get(1)
            && !src.trim_start().starts_with("data:")
        {
            self.texture_files
                .insert(id, (crate::reload::asset_key(src), args.to_vec()));
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
            anisotropy: args
                .get(8)
                .and_then(Value::as_f64)
                .map_or(1, |a| a.clamp(0.0, 16.0) as u16),
            srgb: !matches!(args.get(9), Some(Value::Bool(true))),
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
            // orthographicSize [x y w h] clearFlags(skybox|color) cullingMask
            // [32 cull distances]|null cullSpherical`
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
                    skybox: args.get(10).and_then(Value::as_str) == Some("skybox"),
                    culling_mask: mask(args.get(11)),
                    cull_distances: nums::<32>(args.get(12)).unwrap_or([0.0; 32]),
                    cull_spherical: matches!(args.get(13), Some(Value::Bool(true))),
                    fxaa: args.get(14).and_then(Value::as_str) == Some("fxaa"),
                    target: match args.get(15) {
                        Some(Value::Null) | None => None,
                        _ => Some(id_at(15)?),
                    },
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
            // `mesh id [positions xyz...] [normals] [uvs] [indices] [joints 4 per vertex]
            // [weights 4 per vertex]`; `mesh id null` removes it.
            "mesh" => {
                if matches!(args.get(1), Some(Value::Null) | None) {
                    self.mesh_uploads.push((id, None));
                    self.mesh_data.remove(&id);
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
                    tangents: Vec::new(),
                    joints: flat(args.get(5))
                        .unwrap_or_default()
                        .as_chunks::<4>()
                        .0
                        .iter()
                        .map(|j| j.map(|i| i.clamp(0.0, 65535.0) as u16))
                        .collect(),
                    weights: flat(args.get(6))
                        .unwrap_or_default()
                        .as_chunks::<4>()
                        .0
                        .to_vec(),
                    indices: indices.into_iter().map(|i| i as u32).collect(),
                }
                .complete();
                if !data.is_valid() {
                    return Err(format!(
                        "mesh {id}: indices out of range or not whole triangles"
                    ));
                }
                self.mesh_data.insert(id, data.clone());
                self.mesh_uploads.push((id, Some(data)));
            }
            // `meshRenderer component object mesh(name | id) [r g b a] [er eg eb] smoothness
            // unlit texture|null [scale x y, offset x y] castShadows metallic [maps and
            // modes] (see `material_extras`) [bone objects]|null [bindposes, 16 each]`
            "meshRenderer" => {
                let extras = material_extras(args.get(11))?;
                match args.get(12) {
                    Some(Value::Array(bones)) => {
                        let bones: Vec<ScriptObject> = bones
                            .iter()
                            .map(|b| b.as_f64().map_or(u32::MAX, |b| b as u32))
                            .collect();
                        let flat = flat(args.get(13)).unwrap_or_default();
                        let bindposes: Vec<Mat4> = flat
                            .as_chunks::<16>()
                            .0
                            .iter()
                            .map(Mat4::from_cols_array)
                            .collect();
                        if bindposes.len() != bones.len() {
                            return Err(format!(
                                "{} bones but {} bind poses",
                                bones.len(),
                                bindposes.len()
                            ));
                        }
                        self.skins.insert(id, Skin { bones, bindposes });
                    }
                    _ => {
                        self.skins.remove(&id);
                    }
                }
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
                    metallic: num(10).unwrap_or(0.0).clamp(0.0, 1.0),
                    unlit: matches!(args.get(6), Some(Value::Bool(true))),
                    texture: match args.get(7) {
                        Some(Value::Null) | None => None,
                        _ => Some(id_at(7)?),
                    },
                    tiling: nums::<4>(args.get(8)).unwrap_or([1.0, 1.0, 0.0, 0.0]),
                    cast_shadows: !matches!(args.get(9), Some(Value::Bool(false))),
                    extras,
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
            // haloSize beamIntensity shadows(none|hard|soft) shadowStrength cullingMask`
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
                    shadows: match args.get(10).and_then(Value::as_str) {
                        Some(mode @ ("hard" | "soft")) => Some(Shadows {
                            soft: mode == "soft",
                            strength: num(11).unwrap_or(1.0).clamp(0.0, 1.0),
                        }),
                        _ => None,
                    },
                    culling_mask: mask(args.get(12)),
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
            // `sky 0 [zenith rgb] [horizon rgb] [ground rgb] [sun dir xyz]|null [sun rgb]
            // sunSize intensity ambientFromSky`; `sky 0 null` removes it.
            "sky" => {
                if matches!(args.get(1), Some(Value::Null) | None) {
                    self.sky = None;
                    return Ok(());
                }
                let d = Sky::default();
                self.sky = Some(Sky {
                    zenith: nums::<3>(args.get(1)).unwrap_or(d.zenith),
                    horizon: nums::<3>(args.get(2)).unwrap_or(d.horizon),
                    ground: nums::<3>(args.get(3)).unwrap_or(d.ground),
                    sun_direction: nums::<3>(args.get(4)).map(Vec3::from_array),
                    sun_color: nums::<3>(args.get(5)).unwrap_or(d.sun_color),
                    sun_size: num(6).unwrap_or(d.sun_size),
                });
                self.environment = (
                    num(7).unwrap_or(1.0).max(0.0),
                    !matches!(args.get(8), Some(Value::Bool(false))),
                );
            }
            // `probe component object [sx sy sz] resolution boxProjection everyFrame
            // cullingMask`
            "probe" => {
                let [x, y, z] = nums::<3>(args.get(2)).unwrap_or([10.0; 3]);
                let probe = Probe {
                    object: id_at(1)?,
                    half_extents: Vec3::new(x, y, z).abs() / 2.0,
                    resolution: num(3).unwrap_or(128.0).clamp(16.0, 1024.0) as u32,
                    box_projection: matches!(args.get(4), Some(Value::Bool(true))),
                    every_frame: matches!(args.get(5), Some(Value::Bool(true))),
                    culling_mask: mask(args.get(6)),
                };
                self.probes.insert(id, probe);
                self.probe_requests.insert(id);
            }
            // `probeRender component`: render it again on the next frame.
            "probeRender" => {
                self.probe_requests.insert(id);
            }
            // `mirror component object [nx ny nz] cullingMask`: the object's MeshRenderer
            // reflects the scene in the plane through its origin with this (local) normal.
            "mirror" => {
                let normal = nums::<3>(args.get(2)).map_or(Vec3::Y, Vec3::from_array);
                self.mirrors
                    .insert(id, (id_at(1)?, normal, mask(args.get(3))));
            }
            // `lodGroup component object size [heights] [[renderer ids] per level] forced|-1`
            "lodGroup" => {
                let Some(Value::Array(levels)) = args.get(4) else {
                    return Err("lodGroup needs [[renderers], ...]".into());
                };
                let renderers = levels
                    .iter()
                    .map(|level| match level {
                        Value::Array(ids) => ids
                            .iter()
                            .map(|v| v.as_f64().map(|v| v as u32).ok_or("renderer ids"))
                            .collect::<Result<Vec<u32>, _>>(),
                        _ => Err("renderer ids"),
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let heights = flat(args.get(3)).unwrap_or_default();
                if heights.len() != renderers.len() {
                    return Err("lodGroup needs a height per level".into());
                }
                let forced = num(5).ok().filter(|f| *f >= 0.0).map(|f| f as usize);
                self.lod_groups.insert(
                    id,
                    LodGroup {
                        object: id_at(1)?,
                        size: num(2).unwrap_or(1.0).max(1e-4),
                        heights,
                        renderers,
                        forced,
                    },
                );
            }
            // `quality 0 antiAliasing renderScale shadows(disable|hardOnly|all) softRadius
            // lodBias vSyncCount anisotropic(disable|enable|forceEnable) textureMipLimit`
            "quality" => {
                self.quality = Quality {
                    msaa: num(1).unwrap_or(1.0).clamp(1.0, 8.0) as u32,
                    render_scale: num(2).unwrap_or(1.0).clamp(0.25, 2.0),
                    shadows: match args.get(3).and_then(Value::as_str) {
                        Some("disable") => ShadowQuality::Disable,
                        Some("hardOnly") => ShadowQuality::HardOnly,
                        _ => ShadowQuality::All,
                    },
                    soft_radius: num(4).unwrap_or(1.0).clamp(1.0, 2.0) as u32,
                    lod_bias: num(5).unwrap_or(1.0).max(0.01),
                    vsync: num(6).ok().is_none_or(|v| v >= 1.0),
                    anisotropic: match args.get(7).and_then(Value::as_str) {
                        Some("disable") => Anisotropic::Disable,
                        Some("forceEnable") => Anisotropic::ForceEnable,
                        _ => Anisotropic::Enable,
                    },
                    texture_mip_limit: num(8).unwrap_or(0.0).clamp(0.0, 3.0) as u32,
                };
            }
            // `shadowSettings 0 distance resolution cascades` (Unity's quality settings).
            "shadowSettings" => {
                let resolution = num(2).unwrap_or(2048.0).clamp(256.0, 8192.0) as u32;
                let cascades = num(3).unwrap_or(1.0).clamp(1.0, 4.0) as u32;
                self.shadow_settings = (num(1)?.max(1.0), resolution, cascades);
            }
            // `renderTexture id width height`: a texture cameras draw into (removed with
            // `textureRemove`).
            "renderTexture" => {
                let size = (
                    num(1)?.clamp(1.0, 16384.0) as u32,
                    num(2)?.clamp(1.0, 16384.0) as u32,
                );
                self.render_textures.insert(id, size);
                self.texture_sizes.insert(id, size);
                self.render_texture_uploads.push((id, size));
            }
            // `post 0 postExposure(stops) tonemapping(none|neutral|aces)`.
            "post" => {
                self.grading = Grading {
                    exposure: 2f32.powf(num(1).unwrap_or(0.0).clamp(-10.0, 10.0)),
                    tonemapping: match args.get(2).and_then(Value::as_str) {
                        Some("neutral") => Tonemapping::Neutral,
                        Some("aces") => Tonemapping::Aces,
                        _ => Tonemapping::None,
                    },
                };
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
                self.skins.remove(&id);
                self.lights.remove(&id);
                self.sprites.remove(&id);
                self.mirrors.remove(&id);
                self.probes.remove(&id);
                self.particles.remove(&id);
                self.lod_groups.remove(&id);
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
        let renderers = &self.renderers;
        self.skins.retain(|id, _| renderers.contains_key(id));
        self.lights.retain(|_, l| !objects.contains(&l.object));
        self.sprites.retain(|_, s| !objects.contains(&s.object));
        self.mirrors.retain(|_, m| !objects.contains(&m.0));
        self.probes.retain(|_, p| !objects.contains(&p.object));
        self.lod_groups.retain(|_, g| !objects.contains(&g.object));
        self.particles.retain(|_, p| !objects.contains(&p.object));
    }

    /// The probes asked for have been rendered (call after drawing a frame).
    pub fn probes_rendered(&mut self) {
        self.probe_requests.clear();
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
    /// each seeing the planes and `extra` quads (with their objects) on the layers it
    /// draws; empty without cameras (then only screen UI is drawn).
    pub fn views(
        &self,
        scene: &ScriptScene,
        size: (u32, u32),
        extra: &[(ScriptObject, WorldQuad)],
    ) -> Vec<WorldView> {
        let world = |o: ScriptObject| scene.object(o).map(|id| scene.scene.world_matrix(id));
        let quads: Vec<Tagged<WorldQuad>> = self
            .planes
            .values()
            .filter(|p| scene.active(p.object))
            .filter_map(|p| {
                let quad =
                    WorldQuad::colored(Mat4::from(world(p.object)?) * lay_flat(p.size), p.color);
                Tagged::new(scene, p.object, quad)
            })
            .chain(
                self.sprites
                    .values()
                    .filter(|s| scene.active(s.object))
                    .filter_map(|s| {
                        // Drawn once its texture has loaded.
                        let size = *self.texture_sizes.get(&s.texture)?;
                        let (local, uv) = sprite_quad(s, size);
                        let quad = WorldQuad {
                            model: Mat4::from(world(s.object)?) * local,
                            color: s.color,
                            texture: Some(WorldTexture::Asset(s.texture)),
                            uv,
                            additive: false,
                            order: s.order,
                            layer: 0,
                        };
                        Tagged::new(scene, s.object, quad)
                    }),
            )
            .chain(
                extra
                    .iter()
                    .filter_map(|&(object, quad)| Tagged::new(scene, object, quad)),
            )
            .collect();
        // Reflection probes where their objects are.
        let probes: Vec<ReflectionProbe> = self
            .probes
            .iter()
            .filter(|(_, p)| scene.active(p.object))
            .filter_map(|(&id, p)| {
                Some(ReflectionProbe {
                    id,
                    position: world(p.object)?.translation.into(),
                    half_extents: p.half_extents,
                    resolution: p.resolution,
                    box_projection: p.box_projection,
                    render: p.every_frame || self.probe_requests.contains(&id),
                    culling_mask: p.culling_mask,
                })
            })
            .collect();
        // Mirrors: their objects' planes in the world.
        let mirror_planes: HashMap<ScriptObject, Mirror> = self
            .mirrors
            .values()
            .filter_map(|&(object, normal, culling_mask)| {
                let w = Mat4::from(world(object)?);
                Some((
                    object,
                    Mirror {
                        normal: w.transform_vector3(normal).normalize_or(Vec3::Y),
                        point: w.transform_point3(Vec3::ZERO),
                        culling_mask,
                    },
                ))
            })
            .collect();
        // Skinned renderers' bone matrices, for every view: each bone's world matrix times
        // its inverse bind pose (a missing bone holds its bind pose).
        let mut bones: Vec<Mat4> = Vec::new();
        let mut skinned: HashMap<u32, (u32, u32)> = HashMap::new();
        for (&id, skin) in &self.skins {
            let start = bones.len() as u32;
            for (b, inverse) in skin.bones.iter().zip(&skin.bindposes) {
                let world_bone = world(*b).map_or(inverse.inverse(), Mat4::from);
                bones.push(world_bone * *inverse);
            }
            skinned.insert(id, (start, skin.bones.len() as u32));
        }
        let meshes: Vec<Tagged<MeshInstance>> = self
            .renderers
            .iter()
            .filter(|(_, r)| scene.active(r.object))
            .filter_map(|(&id, r)| {
                let mesh = MeshInstance {
                    mesh: r.mesh,
                    model: Mat4::from(world(r.object)?),
                    color: r.color,
                    emission: r.emission,
                    smoothness: r.smoothness,
                    metallic: r.metallic,
                    unlit: r.unlit,
                    texture: r.texture.map(WorldTexture::Asset),
                    tiling: r.tiling,
                    glow: MeshGlow::None,
                    cast_shadows: r.cast_shadows,
                    mirror: mirror_planes.get(&r.object).copied(),
                    layer: 0,
                    maps: r.extras.maps,
                    specular: r.extras.specular,
                    mode: r.extras.mode,
                    alpha_cutoff: r.extras.alpha_cutoff,
                    double_sided: r.extras.double_sided,
                    bones: skinned.get(&id).copied(),
                };
                Some(Tagged {
                    renderer: Some(id),
                    ..Tagged::new(scene, r.object, mesh)?
                })
            })
            .collect();
        // LOD groups on active objects: where they are, how big, and every renderer of
        // theirs (each camera shows one level's).
        let lods: Vec<(Vec3, f32, &LodGroup)> = self
            .lod_groups
            .values()
            .filter(|g| scene.active(g.object))
            .filter_map(|g| {
                let (scale, _, at) = world(g.object)?.to_scale_rotation_translation();
                Some((at, g.size * scale.abs().max_element(), g))
            })
            .collect();
        // Visible spot light beams: cones as long as the light's range.
        let meshes: Vec<Tagged<MeshInstance>> = meshes
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
                        let beam = MeshInstance {
                            mesh: BEAM_MESH,
                            model: Mat4::from_scale_rotation_translation(
                                Vec3::new(radius, radius, l.range),
                                rotation,
                                position,
                            ),
                            color: [r / peak, g / peak, b / peak, 1.0],
                            emission: [0.0; 3],
                            smoothness: 0.0,
                            metallic: 0.0,
                            unlit: true,
                            texture: None,
                            tiling: [1.0, 1.0, 0.0, 0.0],
                            glow: MeshGlow::Beam,
                            cast_shadows: false,
                            mirror: None,
                            layer: 0,
                            ..Default::default()
                        };
                        Tagged::new(scene, l.object, beam)
                    }),
            )
            .collect();
        // Light halos: a glow at each light, facing each camera.
        let halos: Vec<Tagged<(f32, [f32; 4])>> = self
            .lights
            .values()
            .filter(|l| l.halo > 0.0 && scene.active(l.object))
            .filter_map(|l| {
                let [r, g, b] = l.color;
                // The light's hue at a steady brightness, whatever its intensity.
                let peak = r.max(g).max(b).max(1e-6);
                Tagged::new(
                    scene,
                    l.object,
                    (l.halo, [r / peak, g / peak, b / peak, 0.8]),
                )
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
                    shadows: l.shadows,
                    culling_mask: l.culling_mask,
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
                let forward = rotation * Vec3::NEG_Z;
                let shows = |layer: u8, at: Vec3| camera.shows(layer, at, position, forward);
                // Renderers in a LOD group's other levels (or all of them when it's culled).
                let mut lod_hidden = std::collections::HashSet::new();
                for &(at, size, group) in &lods {
                    let height =
                        camera.screen_height(size, at.distance(position)) * self.quality.lod_bias;
                    let level = group.level(height);
                    for (i, level_renderers) in group.renderers.iter().enumerate() {
                        if Some(i) != level {
                            lod_hidden.extend(level_renderers.iter().copied());
                        }
                    }
                    // Ones also in the shown level stay.
                    if let Some(l) = level {
                        for r in &group.renderers[l] {
                            lod_hidden.remove(r);
                        }
                    }
                }
                let mut quads: Vec<WorldQuad> = quads
                    .iter()
                    .filter(|q| shows(q.layer, q.at))
                    .map(|q| WorldQuad {
                        layer: q.layer,
                        ..q.item
                    })
                    .collect();
                // Particles and halos face this camera.
                for p in self.particles.values().filter(|p| scene.active(p.object)) {
                    if let Some(w) = world(p.object)
                        && shows(scene.layer(p.object), w.translation.into())
                    {
                        let first = quads.len();
                        p.quads(&w, rotation, &mut quads);
                        let layer = scene.layer(p.object);
                        for q in &mut quads[first..] {
                            q.layer = layer;
                        }
                    }
                }
                for halo in halos.iter().filter(|h| shows(h.layer, h.at)) {
                    let (at, (size, color)) = (halo.at, halo.item);
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
                        layer: halo.layer,
                    });
                }
                // A camera with a missing render texture draws nothing.
                let (target, target_size) = match camera.target {
                    Some(t) => (Some(t), *self.render_textures.get(&t)?),
                    None => (None, size),
                };
                let [r, g, b, a] = camera.clear;
                let clear = Color {
                    r: r as f64,
                    g: g as f64,
                    b: b as f64,
                    a: a as f64,
                };
                Some(WorldView {
                    meshes: meshes
                        .iter()
                        .filter(|m| shows(m.layer, m.at))
                        .filter(|m| m.renderer.is_none_or(|r| !lod_hidden.contains(&r)))
                        .map(|m| MeshInstance {
                            layer: m.layer,
                            ..m.item
                        })
                        .collect(),
                    lights: lights.clone(),
                    ambient: self.ambient,
                    skybox: camera.skybox,
                    clip_plane: None,
                    probes: probes.clone(),
                    fxaa: camera.fxaa,
                    target_texture: target,
                    bones: bones.clone(),
                    camera: WorldCamera::new(
                        Mat4::from_rotation_translation(rotation, position),
                        camera.projection,
                        camera.near,
                        camera.far,
                        viewport_size(camera.rect, target_size),
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

    // M5-AST-01
    #[test]
    fn changed_textures_and_models_reload_under_their_ids() {
        let dir = std::env::temp_dir().join(format!("iron-reload-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("assets")).unwrap();
        let png = |w: u32, h: u32| {
            let mut bytes = Vec::new();
            image::RgbaImage::new(w, h)
                .write_to(
                    &mut std::io::Cursor::new(&mut bytes),
                    image::ImageFormat::Png,
                )
                .unwrap();
            bytes
        };
        std::fs::write(dir.join("assets/crate.png"), png(4, 4)).unwrap();
        let fixtures =
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures");
        std::fs::copy(
            fixtures.join("assets/worm.glb"),
            dir.join("assets/worm.glb"),
        )
        .unwrap();
        let mut views = WorldViews::default();
        let args = [Value::from(5.0), Value::String("./assets/crate.png".into())];
        assert_eq!(
            views.texture_op("texture", &dir, &args).unwrap(),
            Some((5, 4, 4))
        );
        views
            .model_op(
                &dir,
                &[Value::from(1.0), Value::String("assets/worm.glb".into())],
            )
            .unwrap();
        let mesh_ids: Vec<u32> = views.mesh_uploads.drain(..).map(|(id, _)| id).collect();
        views.texture_uploads.clear();
        // The texture file grows: it loads again under id 5 with its new size.
        std::fs::write(dir.join("assets/crate.png"), png(8, 2)).unwrap();
        let (sizes, models) = views.reload_asset(&dir, "assets/crate.png");
        assert_eq!(sizes, [(5, 8, 2)]);
        assert!(models.is_empty());
        assert!(matches!(views.texture_uploads.as_slice(), [(5, Some(_))]));
        // The model loads again with the same mesh ids.
        let (_, models) = views.reload_asset(&dir, "assets/worm.glb");
        assert_eq!(models.len(), 1);
        let again: Vec<u32> = views.mesh_uploads.drain(..).map(|(id, _)| id).collect();
        assert_eq!(again, mesh_ids);
        // Unrelated files change nothing.
        let (sizes, models) = views.reload_asset(&dir, "assets/other.png");
        assert!(sizes.is_empty() && models.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

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

    // M5-GLB-01
    #[test]
    fn mesh_colliders_take_a_render_mesh_scaled() {
        let mut views = WorldViews::default();
        let tri = |v: &[f64]| Value::Array(v.iter().map(|x| Value::from(*x)).collect());
        views
            .apply(
                "mesh",
                &[
                    // An engine id, past what an f32 holds exactly.
                    Value::from(1_073_741_827.0),
                    tri(&[0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0]),
                    Value::Null,
                    Value::Null,
                    tri(&[0.0, 1.0, 2.0]),
                ],
            )
            .unwrap();
        let opts = Value::Array(vec![Value::from(0.5); 5]);
        let mut args = vec![
            Value::from(1.0),
            Value::from(1.0),
            Value::String("renderMesh".into()),
            tri(&[1_073_741_827.0, 0.0, 2.0, 2.0, 2.0]),
            Value::Null,
            Value::Null,
            opts,
        ];
        views.resolve_mesh_collider(&mut args).unwrap();
        assert_eq!(args[2], Value::String("mesh".into()));
        assert_eq!(args[3], tri(&[0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 2.0, 0.0]));
        let Value::Array(opts) = &args[6] else {
            panic!()
        };
        assert_eq!(opts[5], tri(&[0.0, 1.0, 2.0]));
        // Unknown meshes are refused.
        args[2] = Value::String("renderMesh".into());
        args[3] = tri(&[9.0, 1.0, 1.0, 1.0, 1.0]);
        assert!(views.resolve_mesh_collider(&mut args).is_err());
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

    // M5-LAY-02
    #[test]
    fn cameras_draw_only_their_layers_within_each_layers_distance() {
        let n = Value::from;
        let at = |x: f64, z: f64| Value::Array(vec![n(x), n(0.0), n(z)]);
        let mut scene = ScriptScene::new();
        let mut views = WorldViews::default();
        // A camera at the origin looking down -z; cubes ahead on layers 0, 3 and 5.
        scene
            .apply("create", &[n(1.0), "camera".into(), Value::Null])
            .unwrap();
        for (object, x, z, layer) in [
            (2.0, 0.0, -5.0, 0.0),
            (3.0, 0.0, -20.0, 3.0),
            (4.0, 8.0, -5.0, 5.0),
        ] {
            scene
                .apply("create", &[n(object), "cube".into(), Value::Null, at(x, z)])
                .unwrap();
            scene.apply("layer", &[n(object), n(layer)]).unwrap();
            views
                .apply(
                    "meshRenderer",
                    &[n(object + 10.0), n(object), "cube".into()],
                )
                .unwrap();
        }
        scene.scene.update_world();
        let camera = |mask: Option<f64>, distances: Option<[f64; 2]>, spherical: bool| {
            let mut far = vec![n(0.0); 32];
            if let Some([layer3, layer5]) = distances {
                far[3] = n(layer3);
                far[5] = n(layer5);
            }
            vec![
                n(100.0),
                n(1.0),
                n(60.0),
                n(0.1),
                n(1000.0),
                Value::Null,
                n(0.0),
                Value::Bool(false),
                n(5.0),
                Value::Null,
                "color".into(),
                mask.map_or(Value::Null, n),
                Value::Array(far),
                Value::Bool(spherical),
            ]
        };
        let seen = |views: &WorldViews| -> Vec<Vec<f32>> {
            views
                .views(&scene, (64, 64), &[])
                .iter()
                .map(|v| {
                    let mut x: Vec<f32> = v.meshes.iter().map(|m| m.model.w_axis.z).collect();
                    x.sort_by(f32::total_cmp);
                    x
                })
                .collect()
        };
        // Default: every layer, no distances.
        views.apply("camera", &camera(None, None, false)).unwrap();
        assert_eq!(seen(&views), vec![vec![-20.0, -5.0, -5.0]]);
        // Layer 5 off (Unity's ~(1 << 5) as an unsigned mask).
        let mask = f64::from(!(1u32 << 5));
        views
            .apply("camera", &camera(Some(mask), None, false))
            .unwrap();
        assert_eq!(seen(&views), vec![vec![-20.0, -5.0]]);
        // Layer 3 only out to 10 m: the far cube goes.
        views
            .apply("camera", &camera(None, Some([10.0, 0.0]), false))
            .unwrap();
        assert_eq!(seen(&views), vec![vec![-5.0, -5.0]]);
        // Layer 5 out to 6 m: its cube is 5 m along the view but 9.4 m away, so only a
        // spherical distance hides it.
        views
            .apply("camera", &camera(None, Some([0.0, 6.0]), false))
            .unwrap();
        assert_eq!(seen(&views), vec![vec![-20.0, -5.0, -5.0]]);
        views
            .apply("camera", &camera(None, Some([0.0, 6.0]), true))
            .unwrap();
        assert_eq!(seen(&views), vec![vec![-20.0, -5.0]]);
    }

    // M5-LOD-01
    #[test]
    fn lod_groups_show_one_level_by_screen_height() {
        let n = Value::from;
        let mut scene = ScriptScene::new();
        let mut views = WorldViews::default();
        scene
            .apply("create", &[n(1.0), "camera".into(), Value::Null])
            .unwrap();
        scene
            .apply("create", &[n(2.0), "tree".into(), Value::Null])
            .unwrap();
        // A 60 degree camera at the origin looking down -z.
        views
            .apply("camera", &[n(10.0), n(1.0), n(60.0), n(0.1), n(1000.0)])
            .unwrap();
        let group = |views: &mut WorldViews, forced: f64| {
            let levels = Value::Array(vec![
                Value::Array(vec![n(20.0)]),
                Value::Array(vec![n(21.0)]),
            ]);
            let heights = Value::Array(vec![n(0.5), n(0.1)]);
            views
                .apply(
                    "lodGroup",
                    &[n(30.0), n(2.0), n(2.0), heights, levels, n(forced)],
                )
                .unwrap();
        };
        group(&mut views, -1.0);
        // Two renderers on the tree, told apart by colour: detailed (20, red) and
        // simple (21, blue).
        views
            .apply(
                "meshRenderer",
                &[
                    n(20.0),
                    n(2.0),
                    "cube".into(),
                    Value::Array(vec![n(1.0), n(0.0), n(0.0), n(1.0)]),
                ],
            )
            .unwrap();
        views
            .apply(
                "meshRenderer",
                &[
                    n(21.0),
                    n(2.0),
                    "cube".into(),
                    Value::Array(vec![n(0.0), n(0.0), n(1.0), n(1.0)]),
                ],
            )
            .unwrap();
        let level = |scene: &mut ScriptScene, views: &WorldViews, z: f64| -> Vec<&str> {
            let at = Value::Array(vec![n(0.0), n(0.0), n(z)]);
            scene.apply("local", &[n(2.0), at]).unwrap();
            scene.scene.update_world();
            views
                .views(scene, (100, 100), &[])
                .remove(0)
                .meshes
                .iter()
                .map(|m| {
                    if m.color[0] > 0.5 {
                        "detailed"
                    } else {
                        "simple"
                    }
                })
                .collect()
        };
        // 2 m tall: 1.15 of the view at 1.5 m, 0.35 at 5 m, 0.06 at 30 m.
        assert_eq!(level(&mut scene, &views, -1.5), ["detailed"]);
        assert_eq!(level(&mut scene, &views, -5.0), ["simple"]);
        assert!(
            level(&mut scene, &views, -30.0).is_empty(),
            "culled when tiny"
        );
        // A bias of 2 counts it twice as tall: detailed at 5 m.
        views
            .apply(
                "quality",
                &[n(0.0), n(1.0), n(1.0), "all".into(), n(1.0), n(2.0)],
            )
            .unwrap();
        assert_eq!(level(&mut scene, &views, -5.0), ["detailed"]);
        // Forced levels win at any distance.
        group(&mut views, 1.0);
        assert_eq!(level(&mut scene, &views, -1.5), ["simple"]);
        assert_eq!(level(&mut scene, &views, -300.0), ["simple"]);
    }
}
