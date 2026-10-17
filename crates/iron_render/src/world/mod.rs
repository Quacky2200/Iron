//! The 3D world pass (M5): what the cameras see, drawn before the screen UI.
//!
//! The world is lit meshes ([`MeshInstance`], built-in primitives or meshes set with
//! [`WorldRenderer::set_mesh`]) and quads: coloured or textured planes, such as UI
//! panels in the world. Each camera is a [`WorldView`] with its own viewport and lights;
//! views are drawn in order, each first filling its viewport with its background (which
//! also resets depth there), so later cameras draw over earlier ones, like Unity's camera
//! depth. Opaque meshes go first, then see-through meshes and quads back to front, so
//! they blend over what's behind them. Textures are either the frame's (UI panels,
//! passed to each render) or kept by the renderer ([`WorldRenderer::set_texture`]).

pub mod bloom;
pub mod cull;
pub mod environment;
pub mod mesh;
pub mod texture;
mod upscale;

use std::collections::HashMap;
use std::ops::Range;

use bytemuck::{Pod, Zeroable};
use iron_core::glam::{Mat3, Mat4, Vec3, Vec4Swizzles};

use crate::{Color, Gpu, RenderTarget};
pub use bloom::Bloom;
pub use environment::Sky;
pub use mesh::{MeshData, primitive};
use texture::SamplerKey;
pub use texture::{Anisotropic, TextureData, TextureFilter, TextureWrap};

/// A texture a quad or mesh is drawn with.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WorldTexture {
    /// Index into the `textures` given to [`WorldRenderer::render_views`].
    Frame(usize),
    /// One set with [`WorldRenderer::set_texture`] (the missing checker until it exists).
    Asset(u32),
    /// A mirror's reflection, rendered by the renderer this frame (set internally).
    Reflection(usize),
}

/// A reflection probe (Unity's): a cubemap of the scene seen from `position`, which
/// meshes inside its box reflect instead of the sky.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReflectionProbe {
    /// Identifies it across frames (its cubemap is kept between renders).
    pub id: u32,
    pub position: Vec3,
    /// Half the size of its box of influence, which is centred on `position`.
    pub half_extents: Vec3,
    /// The cubemap's face size in texels (a power of two).
    pub resolution: u32,
    /// Reflections are corrected for the box's walls (rooms), not taken as infinitely far.
    pub box_projection: bool,
    /// Render it this frame (it's also rendered when first seen or resized).
    pub render: bool,
    /// The layers it captures, a bit each (Unity's `cullingMask`).
    pub culling_mask: u32,
}

/// A flat mirror: the plane a mesh reflects the scene in.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mirror {
    /// The plane's normal, pointing out of the reflecting side (world space).
    pub normal: Vec3,
    /// A point on the plane.
    pub point: Vec3,
    /// The layers it reflects, a bit each.
    pub culling_mask: u32,
}

/// A culling mask with every layer.
pub const ALL_LAYERS: u32 = u32::MAX;

/// Whether `mask` has `layer`'s bit.
pub fn in_mask(mask: u32, layer: u8) -> bool {
    mask & (1u32 << (layer & 31)) != 0
}

const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

/// The open cone light beams are drawn with ([`mesh::beam`]).
pub const BEAM_MESH: u32 = PRIMITIVE_BASE + 0xFFFE;

/// How a surface covers what's behind it (Unity's Standard shader rendering modes).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum RenderMode {
    /// Solid. A colour alpha below 1 still draws it as [`RenderMode::Fade`] (scripts'
    /// see-through colours keep working without picking a mode).
    #[default]
    Opaque,
    /// Solid, with holes where the alpha (texture × colour) is below the cut-off: leaves,
    /// fences. Casts shadows with the same holes.
    Cutout,
    /// Everything fades with the alpha, highlights and reflections too: fading objects out.
    Fade,
    /// See-through, but highlights and reflections stay: glass, clear plastic.
    Transparent,
}

/// A material's textures besides the main one (Unity's Standard shader maps). Each is
/// sampled with the main texture's tiling, except the detail map.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MaterialMaps {
    /// A tangent-space normal map (green up), read as linear data.
    pub normal: Option<WorldTexture>,
    /// How strong the normal map's bumps are (Unity's `bumpScale`).
    pub normal_scale: f32,
    /// Metallic in red and smoothness in alpha (Unity's `metallicGlossMap`); with it the
    /// material's metallic is the map's and its smoothness scales the map's. With the
    /// specular workflow, the specular colour (rgb) and smoothness (a) instead.
    pub metallic_gloss: Option<WorldTexture>,
    /// Read `metallic_gloss` the glTF way: roughness in green, metallic in blue, both
    /// multiplying the material's values; occlusion in red.
    pub gltf_packing: bool,
    /// Ambient occlusion (green; red with glTF packing), darkening sky light and
    /// reflections only.
    pub occlusion: Option<WorldTexture>,
    pub occlusion_strength: f32,
    /// Multiplies the emission colour.
    pub emission: Option<WorldTexture>,
    /// Extra detail multiplied over the albedo (x2 in gamma space, Unity's
    /// `detailAlbedoMap`), tiled on its own.
    pub detail: Option<WorldTexture>,
    /// The detail map's tiling: scale (xy) and offset (zw).
    pub detail_tiling: [f32; 4],
    /// A mirror's reflection this frame (set by the renderer, not by callers).
    pub reflection: Option<WorldTexture>,
}

impl Default for MaterialMaps {
    fn default() -> Self {
        Self {
            normal: None,
            normal_scale: 1.0,
            metallic_gloss: None,
            gltf_packing: false,
            occlusion: None,
            occlusion_strength: 1.0,
            emission: None,
            detail: None,
            detail_tiling: [1.0, 1.0, 0.0, 0.0],
            reflection: None,
        }
    }
}

/// How a mesh adds light instead of covering what's behind it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MeshGlow {
    /// Lit (or unlit) and opaque or see-through by its colour's alpha.
    #[default]
    None,
    /// Unlit, adds its colour × alpha (glowing shapes).
    Additive,
    /// Additive with soft edges and a fade towards the open end: a light beam.
    Beam,
}

/// A magenta and black checker, drawn wherever a texture is missing (never set, failed
/// to load, or removed), so missing assets stand out instead of passing as white.
pub const MISSING_TEXTURE: u32 = u32::MAX - 1;

/// A soft round spot every renderer keeps as a texture (particles, halos).
pub const DEFAULT_PARTICLE_TEXTURE: u32 = u32::MAX;

/// The built-in meshes, registered by every renderer at [`primitive_id`].
pub const PRIMITIVES: [&str; 6] = ["cube", "sphere", "capsule", "cylinder", "plane", "quad"];
/// Mesh ids from here on are the built-in primitives; scripts use ids below it.
pub const PRIMITIVE_BASE: u32 = 0xFFFF_0000;

/// The mesh id of a built-in primitive by name.
pub fn primitive_id(name: &str) -> Option<u32> {
    PRIMITIVES
        .iter()
        .position(|p| *p == name)
        .map(|i| PRIMITIVE_BASE + i as u32)
}

/// How a camera projects the world.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Projection {
    /// Vertical field of view in degrees.
    Perspective { fov_y_degrees: f32 },
    /// Half the view's height in world units (Unity's `orthographicSize`): 2D games.
    Orthographic { half_height: f32 },
}

/// What a camera sees from where: `view_proj` maps world space to clip space.
#[derive(Clone, Copy, Debug)]
pub struct WorldCamera {
    pub view_proj: Mat4,
    /// The camera's position, for sorting see-through things and specular highlights.
    pub position: Vec3,
    /// Which way it looks (its -z).
    pub forward: Vec3,
    pub orthographic: bool,
    pub clear: Color,
}

impl WorldCamera {
    /// A camera at `world` (its transform: looking down its -z) for a viewport of `size`
    /// pixels. Uses the same projections as `Matrix4x4` in `@iron/engine` (right-handed,
    /// depth 0 to 1).
    pub fn new(
        world: Mat4,
        projection: Projection,
        near: f32,
        far: f32,
        size: (u32, u32),
        clear: Color,
    ) -> Self {
        let aspect = size.0.max(1) as f32 / size.1.max(1) as f32;
        let (near, far) = (near.max(1e-4), far.max(near.max(1e-4) + 1e-3));
        let proj = match projection {
            Projection::Perspective { fov_y_degrees } => {
                iron_core::glam::camera::rh::proj::directx::perspective(
                    fov_y_degrees.clamp(1.0, 179.0).to_radians(),
                    aspect,
                    near,
                    far,
                )
            }
            Projection::Orthographic { half_height } => {
                let h = half_height.max(1e-4);
                let w = h * aspect;
                iron_core::glam::camera::rh::proj::directx::orthographic(-w, w, -h, h, near, far)
            }
        };
        Self {
            view_proj: proj * world.inverse(),
            position: world.w_axis.xyz(),
            forward: (-world.z_axis.xyz()).normalize_or(Vec3::NEG_Z),
            orthographic: matches!(projection, Projection::Orthographic { .. }),
            clear,
        }
    }

    /// A perspective camera with a vertical field of view in degrees.
    pub fn perspective(
        world: Mat4,
        fov_y_degrees: f32,
        near: f32,
        far: f32,
        size: (u32, u32),
        clear: Color,
    ) -> Self {
        Self::new(
            world,
            Projection::Perspective { fov_y_degrees },
            near,
            far,
            size,
            clear,
        )
    }
}

/// A light (Unity's `Light` types).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LightKind {
    /// Like the sun: parallel rays along `direction`, everywhere.
    Directional,
    /// From `position` in all directions, fading out at `range`.
    Point,
    /// A cone from `position` along `direction`, with soft edges between the inner and
    /// outer half-angles (degrees), fading out at `range`.
    Spot {
        inner_degrees: f32,
        outer_degrees: f32,
    },
}

#[derive(Clone, Copy, Debug)]
pub struct Light {
    pub kind: LightKind,
    /// Linear colour already multiplied by the intensity.
    pub color: [f32; 3],
    pub position: Vec3,
    pub direction: Vec3,
    pub range: f32,
    /// Shadows it casts (only the first directional light with them does, for now).
    pub shadows: Option<Shadows>,
    /// The layers it lights (and whose objects cast its shadows), a bit each (Unity's
    /// `cullingMask`).
    pub culling_mask: u32,
}

/// Which shadows are drawn at all (Unity's `QualitySettings.shadows`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ShadowQuality {
    /// No shadows.
    Disable,
    /// Soft shadows are drawn hard (one sample).
    HardOnly,
    /// As each light asks.
    #[default]
    All,
}

/// How a light casts shadows (Unity's `LightShadows`: hard or soft) and how dark they are.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shadows {
    /// Filtered edges (3 × 3 PCF) instead of one sample.
    pub soft: bool,
    /// 0 (none) to 1 (fully dark where shadowed).
    pub strength: f32,
}

/// One mesh drawn in the world.
#[derive(Clone, Copy, Debug)]
pub struct MeshInstance {
    /// A built-in ([`primitive_id`]) or a mesh set with [`WorldRenderer::set_mesh`].
    pub mesh: u32,
    pub model: Mat4,
    /// Linear RGBA, straight alpha.
    pub color: [f32; 4],
    /// Linear light it gives off on its own (not lighting anything else).
    pub emission: [f32; 3],
    /// 0 (matte) to 1 (mirror): 1 - roughness, Unity's name.
    pub smoothness: f32,
    /// 0 (plastic, stone, wood) to 1 (metal: tinted reflections, no diffuse).
    pub metallic: f32,
    /// Ignores lights: just its colour (and emission).
    pub unlit: bool,
    /// Multiplies the colour; `None` is plain colour.
    pub texture: Option<WorldTexture>,
    /// Texture tiling: scale (xy) and offset (zw), Unity's `mainTextureScale` /
    /// `mainTextureOffset` (in its bottom-up UV space).
    pub tiling: [f32; 4],
    pub glow: MeshGlow,
    /// Whether it casts shadows (Unity's `shadowCastingMode` on / off).
    pub cast_shadows: bool,
    /// Reflects the scene exactly in this plane (a planar mirror) instead of the sky.
    pub mirror: Option<Mirror>,
    /// Its object's layer (0 to 31): lights, probes and mirrors with masks skip it.
    pub layer: u8,
    /// Normal, metallic, occlusion, emission and detail maps.
    pub maps: MaterialMaps,
    /// The specular workflow (Unity's "Standard (Specular setup)"): this colour is the
    /// reflectance head-on, and `metallic` is ignored.
    pub specular: Option<[f32; 3]>,
    pub mode: RenderMode,
    /// With [`RenderMode::Cutout`]: alpha below this is a hole.
    pub alpha_cutoff: f32,
    /// Draws its back faces too (leaves, cloth, open meshes), lit from that side.
    pub double_sided: bool,
}

impl Default for MeshInstance {
    fn default() -> Self {
        Self {
            mesh: PRIMITIVE_BASE,
            model: Mat4::IDENTITY,
            color: [1.0; 4],
            emission: [0.0; 3],
            smoothness: 0.5,
            metallic: 0.0,
            unlit: false,
            texture: None,
            tiling: [1.0, 1.0, 0.0, 0.0],
            glow: MeshGlow::None,
            cast_shadows: true,
            mirror: None,
            layer: 0,
            maps: MaterialMaps::default(),
            specular: None,
            mode: RenderMode::Opaque,
            alpha_cutoff: 0.5,
            double_sided: false,
        }
    }
}

impl MeshInstance {
    /// Drawn in the see-through pass (blended back to front) rather than the opaque one.
    pub fn blended(&self) -> bool {
        self.glow != MeshGlow::None
            || match self.mode {
                RenderMode::Opaque => self.color[3] < 1.0,
                RenderMode::Cutout => false,
                RenderMode::Fade | RenderMode::Transparent => true,
            }
    }

    /// The textures it's drawn with, as a key for grouping and bind groups.
    fn material_key(&self) -> MaterialKey {
        MaterialKey {
            base: self.texture,
            normal: self.maps.normal,
            metallic_gloss: self.maps.metallic_gloss,
            // The specular colour map holds colours (sRGB); metallic maps hold data.
            metallic_srgb: self.specular.is_some(),
            occlusion: self.maps.occlusion,
            emission: self.maps.emission,
            detail: self.maps.detail,
            reflection: self.maps.reflection,
        }
    }
}

/// The textures a draw binds (group 1): the main texture and a material's maps.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct MaterialKey {
    base: Option<WorldTexture>,
    normal: Option<WorldTexture>,
    metallic_gloss: Option<WorldTexture>,
    metallic_srgb: bool,
    occlusion: Option<WorldTexture>,
    emission: Option<WorldTexture>,
    detail: Option<WorldTexture>,
    reflection: Option<WorldTexture>,
}

impl MaterialKey {
    fn texture(base: Option<WorldTexture>) -> Self {
        Self {
            base,
            ..Default::default()
        }
    }

    /// Uses the frame's textures (panels, reflections), so its bind group can't be kept.
    fn per_frame(&self) -> bool {
        [
            self.base,
            self.normal,
            self.metallic_gloss,
            self.occlusion,
            self.emission,
            self.detail,
            self.reflection,
        ]
        .iter()
        .any(|t| {
            matches!(
                t,
                Some(WorldTexture::Frame(_) | WorldTexture::Reflection(_))
            )
        })
    }
}

/// One camera's view: where on the target it draws and what it sees.
#[derive(Clone, Debug)]
pub struct WorldView {
    pub camera: WorldCamera,
    /// The part of the target it covers: x, y, width, height from the top-left, 0 to 1.
    pub viewport: [f32; 4],
    pub quads: Vec<WorldQuad>,
    pub meshes: Vec<MeshInstance>,
    pub lights: Vec<Light>,
    /// Linear light everything gets regardless of lights.
    pub ambient: [f32; 3],
    /// Fill the background with the sky ([`WorldRenderer::set_sky`]) instead of the
    /// camera's colour (Unity's skybox clear flag).
    pub skybox: bool,
    /// Draws only what's on the positive side of this plane (`[nx, ny, nz, d]`: kept where
    /// `n · p + d >= 0`), for mirror views.
    pub clip_plane: Option<[f32; 4]>,
    /// Reflection probes in this view's world (rendered for the main view).
    pub probes: Vec<ReflectionProbe>,
}

impl WorldView {
    /// A view of nothing yet (add quads, meshes and lights).
    pub fn new(camera: WorldCamera, viewport: [f32; 4]) -> Self {
        Self {
            camera,
            viewport,
            quads: Vec::new(),
            meshes: Vec::new(),
            lights: Vec::new(),
            ambient: [0.2, 0.2, 0.22],
            skybox: false,
            clip_plane: None,
            probes: Vec::new(),
        }
    }
}

/// One plane in the world.
#[derive(Clone, Copy, Debug)]
pub struct WorldQuad {
    /// Places the unit quad (-0.5..0.5 in x and y, facing +z) in the world.
    pub model: Mat4,
    /// Linear RGBA, straight alpha (multiplies the texture).
    pub color: [f32; 4],
    /// Multiplies the colour; `None` is plain colour.
    pub texture: Option<WorldTexture>,
    /// The part of the texture shown: origin and size in UV space.
    pub uv: [f32; 4],
    /// Adds light instead of covering what's behind (glows, fire, sparks).
    pub additive: bool,
    /// Draw order among see-through things: higher draws later (over), whatever the
    /// distance (Unity's `sortingOrder`; meshes are 0).
    pub order: i32,
    /// Its object's layer (0 to 31), for probes' and mirrors' masks.
    pub layer: u8,
}

impl WorldQuad {
    pub fn colored(model: Mat4, color: [f32; 4]) -> Self {
        Self {
            model,
            color,
            texture: None,
            uv: [0.0, 0.0, 1.0, 1.0],
            additive: false,
            order: 0,
            layer: 0,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct GpuQuad {
    model: [[f32; 4]; 4],
    color: [f32; 4],
    uv: [f32; 4],
    /// x: 1 when the texture holds straight alpha (kept textures) rather than
    /// premultiplied (the frame's, such as rendered UI).
    params: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct GpuMesh {
    model: [[f32; 4]; 4],
    normal: [[f32; 4]; 4],
    color: [f32; 4],
    emission: [f32; 4],
    params: [f32; 4],
    tiling: [f32; 4],
    /// The reflection probe it uses: centre (xyz) and mip count (w, 0: none).
    probe: [f32; 4],
    /// The probe's half extents (xyz) and 1 for box projection (w).
    probe_box: [f32; 4],
    /// x: its layer's bit, for lights' masks; y: which maps it has ([`map_flags`]);
    /// z: its [`RenderMode`] (0 opaque, 1 cutout, 2 fade, 3 transparent).
    layer: [u32; 4],
    /// rgb: the specular colour; w: 1 for the specular workflow.
    specular: [f32; 4],
    /// x: normal map strength; y: occlusion strength; z: alpha cut-off.
    maps: [f32; 4],
    /// The detail map's tiling: scale (xy), offset (zw).
    detail: [f32; 4],
}

/// Bits in `GpuMesh::layer.y` for the maps a mesh has.
mod map_flags {
    pub const NORMAL: u32 = 1;
    pub const METALLIC_GLOSS: u32 = 2;
    pub const GLTF: u32 = 4;
    pub const OCCLUSION: u32 = 8;
    pub const EMISSION: u32 = 16;
    pub const DETAIL: u32 = 32;
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct GpuLight {
    position: [f32; 4],
    direction: [f32; 4],
    color: [f32; 4],
    spot: [f32; 4],
    /// x: the layers it lights.
    mask: [u32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct GpuGlobals {
    view_proj: [[f32; 4]; 4],
    background: [f32; 4],
    camera: [f32; 4],
    ambient: [f32; 4],
    /// The shadow map's view-projection.
    shadow_view_proj: [[f32; 4]; 4],
    /// x: index of the shadowed light (-1: none); y: strength; z: a shadow-map texel in
    /// metres (for the normal offset); w: the soft filter's radius in texels (0: hard).
    shadow: [f32; 4],
    /// Screen pixel to world direction, for the skybox.
    inv_view_proj: [[f32; 4]; 4],
    /// The view's viewport in target pixels: x, y, width, height.
    viewport: [f32; 4],
    /// x: 1 with an environment map; y: its intensity; z: its last mip; w: 1 when ambient
    /// light comes from it.
    env: [f32; 4],
    /// x: 1 to draw the sky as the background; yz: the target's size in pixels.
    flags: [f32; 4],
    /// Fragments where `dot(xyz, p) + w < 0` are dropped (zero: none).
    clip: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
    position: [f32; 3],
    normal: [f32; 3],
    uv: [f32; 2],
    tangent: [f32; 4],
}

/// The mesh vertex layout every mesh pipeline uses.
const VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 4] = wgpu::vertex_attr_array![
    0 => Float32x3, 1 => Float32x3, 2 => Float32x2, 3 => Float32x4
];

/// A mesh's buffers on the GPU.
struct MeshBuffers {
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    count: u32,
    /// The local bounding box, for frustum culling.
    bounds: (Vec3, Vec3),
}

/// What the last frame drew and what culling skipped (every view, mirror and probe
/// face counted), like Unity's rendering statistics.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WorldStats {
    pub meshes: usize,
    pub culled_meshes: usize,
    pub quads: usize,
    pub culled_quads: usize,
    pub lights: usize,
    pub culled_lights: usize,
}

/// Quad, background-fill, opaque-mesh and see-through-mesh pipelines for one format.
#[derive(Clone)]
struct Pipelines {
    quad: wgpu::RenderPipeline,
    /// Additive quads: they light what's behind and hide nothing, so no depth writes.
    quad_glow: wgpu::RenderPipeline,
    /// Additive meshes, seen from inside too (a beam around the camera).
    mesh_glow: wgpu::RenderPipeline,
    fill: wgpu::RenderPipeline,
    mesh: wgpu::RenderPipeline,
    mesh_blend: wgpu::RenderPipeline,
    /// Double-sided materials: both faces drawn.
    mesh_double: wgpu::RenderPipeline,
    mesh_blend_double: wgpu::RenderPipeline,
}

/// A view's opaque runs, its blended runs (`true`: a mesh) and its lights.
/// Opaque runs, see-through runs, the view's lights in the light buffer, and which of
/// the view's lights those are (culled ones left out).
type ViewRuns = (Vec<Run>, Vec<(bool, Run)>, Range<u32>, Vec<usize>);

/// A run of instances drawn with one mesh or texture.
struct Run {
    mesh: Option<u32>,
    material: MaterialKey,
    /// Both faces drawn.
    double_sided: bool,
    /// Casts shadows with holes where its alpha is below the cut-off.
    cutout: bool,
    /// The reflection probe its meshes use (`None`: the sky).
    probe: Option<u32>,
    /// Additive quads (drawn without depth writes).
    glow: bool,
    /// Opaque meshes that cast shadows (drawn into the shadow map).
    casts: bool,
    /// In the camera's view; casters outside it are only drawn into the shadow map.
    visible: bool,
    range: Range<u32>,
}

pub struct WorldRenderer {
    layout: wgpu::PipelineLayout,
    module: wgpu::ShaderModule,
    /// By colour format and sample count.
    pipelines: HashMap<(wgpu::TextureFormat, u32), Pipelines>,
    /// Multisampled colour targets the main pass resolves from, by size, format, samples.
    msaa_targets: HashMap<((u32, u32), wgpu::TextureFormat, u32), wgpu::TextureView>,
    /// The world's resolution relative to the target (0.25 to 2; Unity URP's `renderScale`):
    /// below 1 is faster and blurrier, above 1 supersamples. Screen UI isn't scaled.
    pub render_scale: f32,
    upscale: Option<upscale::Upscale>,
    /// Anti-aliasing for the main world pass: 1 (off), 2, 4 or 8 samples per pixel
    /// (Unity's `antiAliasing`), lowered to what the GPU supports. Mirrors and probes
    /// render without it.
    pub msaa: u32,
    globals_layout: wgpu::BindGroupLayout,
    shadow_layout: wgpu::BindGroupLayout,
    shadow_sampler: wgpu::Sampler,
    shadow_pipeline: wgpu::RenderPipeline,
    shadow_cutout_pipeline: wgpu::RenderPipeline,
    /// The directional shadow map, made at `shadow_resolution`.
    shadow_map: Option<(wgpu::TextureView, u32)>,
    /// How far from the camera shadows reach (metres; Unity's `shadowDistance`).
    pub shadow_distance: f32,
    /// The shadow map's size in texels.
    pub shadow_resolution: u32,
    /// Which shadows are drawn.
    pub shadow_quality: ShadowQuality,
    /// Soft shadows' filter radius in texels: 1 (3 × 3 samples) or 2 (5 × 5, smoother).
    pub soft_shadow_radius: u32,
    /// Fills environment maps (made on first use).
    env_pass: Option<environment::EnvPass>,
    /// The sky's environment map; a 1-texel black cube until there's a sky.
    env_map: environment::EnvMap,
    sky: Option<Sky>,
    sky_dirty: bool,
    /// Mirrors' reflection textures, by mirror, with their sizes.
    reflection_targets: Vec<Option<(wgpu::TextureView, (u32, u32))>>,
    env_sampler: wgpu::Sampler,
    /// How bright reflections and sky light are (Unity's `reflectionIntensity`).
    pub environment_intensity: f32,
    /// Ambient light from the sky (Unity's `ambientMode` skybox) rather than each view's
    /// `ambient` colour, when there's a sky.
    pub ambient_from_sky: bool,
    texture_layout: wgpu::BindGroupLayout,
    /// One `GpuGlobals` per view, `stride` bytes apart (a dynamic offset picks one).
    globals: wgpu::Buffer,
    stride: u64,
    quads: wgpu::Buffer,
    mesh_instances: wgpu::Buffer,
    meshes: HashMap<u32, MeshBuffers>,
    /// What the last frame drew and culled.
    stats: WorldStats,
    textures: HashMap<u32, AssetTexture>,
    samplers: HashMap<SamplerKey, wgpu::Sampler>,
    /// Material bind groups made from kept textures only, reused across frames (cleared
    /// when a texture or the texture quality changes).
    material_groups: HashMap<MaterialKey, wgpu::BindGroup>,
    /// Unity's `QualitySettings.anisotropicFiltering`.
    pub anisotropic: Anisotropic,
    /// Top mip levels skipped when sampling kept textures (Unity's
    /// `globalTextureMipmapLimit`: 0 full size, 1 half, 2 quarter, 3 eighth).
    pub texture_mip_limit: u32,
    /// The quality the material groups were made with.
    texture_quality: (Anisotropic, u32),
    sampler: wgpu::Sampler,
    white: wgpu::TextureView,
    /// Depth buffers by size and sample count (the screen, mirrors and probe faces).
    depth: HashMap<((u32, u32), u32), wgpu::TextureView>,
    /// Reflection probes' cubemaps, by probe id.
    probe_maps: HashMap<u32, environment::EnvMap>,
    /// 2D targets probe faces are drawn into before copying, by face size.
    probe_faces: HashMap<u32, wgpu::TextureView>,
    /// Bloom over every view (`None`: off).
    pub bloom: Option<Bloom>,
    /// Made the first time bloom is on.
    bloom_pass: Option<bloom::BloomPass>,
}

/// A texture the renderer keeps: views reading it as colour (sRGB) and as data
/// (linear, for normal and metallic maps), and its sampling settings.
struct AssetTexture {
    srgb: wgpu::TextureView,
    linear: wgpu::TextureView,
    filter: TextureFilter,
    wrap: TextureWrap,
    anisotropy: u16,
    size: (u32, u32),
}

fn storage_entry(binding: u32, visibility: wgpu::ShaderStages) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only: true },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

impl WorldRenderer {
    pub fn new(gpu: &Gpu) -> Self {
        let device = &gpu.device;
        let module = device.create_shader_module(wgpu::include_wgsl!("world.wgsl"));
        let globals_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("world globals"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        min_binding_size: wgpu::BufferSize::new(
                            std::mem::size_of::<GpuGlobals>() as u64
                        ),
                    },
                    count: None,
                },
                storage_entry(1, wgpu::ShaderStages::VERTEX),
                storage_entry(2, wgpu::ShaderStages::VERTEX_FRAGMENT),
                storage_entry(3, wgpu::ShaderStages::FRAGMENT),
            ],
        });
        // A draw's textures: the main one and its sampler, then the material's maps
        // (normal, metallic / specular, occlusion, emission, detail) and a mirror's
        // reflection.
        let texture_entry = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        };
        let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("world material"),
            entries: &[
                texture_entry(0),
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                texture_entry(2),
                texture_entry(3),
                texture_entry(4),
                texture_entry(5),
                texture_entry(6),
                texture_entry(7),
            ],
        });
        let shadow_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("world shadows"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                    count: None,
                },
                // The environment cube, for reflections, sky light and the skybox.
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::Cube,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("world"),
            bind_group_layouts: &[
                Some(&globals_layout),
                Some(&texture_layout),
                Some(&shadow_layout),
            ],
            immediate_size: 0,
        });
        // Casters are drawn into the shadow map with only the globals (meshes, matrix).
        let shadow_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("world shadow casters"),
                bind_group_layouts: &[Some(&globals_layout)],
                immediate_size: 0,
            });
        let shadow_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("world shadow casters"),
            layout: Some(&shadow_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs_shadow"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &VERTEX_ATTRIBUTES,
                })],
            },
            // Both sides cast: planes and open meshes have no back to cast from.
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: Default::default(),
                // Slope-scaled bias against shadow acne.
                bias: wgpu::DepthBiasState {
                    constant: 2,
                    slope_scale: 2.0,
                    clamp: 0.0,
                },
            }),
            multisample: Default::default(),
            fragment: None,
            multiview_mask: None,
            cache: None,
        });
        // Cut-out casters test their texture's alpha, so their holes let light through.
        let cutout_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("world cut-out casters"),
            bind_group_layouts: &[Some(&globals_layout), Some(&texture_layout)],
            immediate_size: 0,
        });
        let shadow_cutout_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("world cut-out casters"),
                layout: Some(&cutout_layout),
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: Some("vs_shadow_cutout"),
                    compilation_options: Default::default(),
                    buffers: &[Some(wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<Vertex>() as u64,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &VERTEX_ATTRIBUTES,
                    })],
                },
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: DEPTH_FORMAT,
                    depth_write_enabled: Some(true),
                    depth_compare: Some(wgpu::CompareFunction::Less),
                    stencil: Default::default(),
                    bias: wgpu::DepthBiasState {
                        constant: 2,
                        slope_scale: 2.0,
                        clamp: 0.0,
                    },
                }),
                multisample: Default::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &module,
                    entry_point: Some("fs_shadow_cutout"),
                    compilation_options: Default::default(),
                    targets: &[],
                }),
                multiview_mask: None,
                cache: None,
            });
        let shadow_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("world shadows"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            compare: Some(wgpu::CompareFunction::LessEqual),
            ..Default::default()
        });
        let align = u64::from(device.limits().min_uniform_buffer_offset_alignment);
        let stride = (std::mem::size_of::<GpuGlobals>() as u64).div_ceil(align) * align;
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("world"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let white = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("world white"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        gpu.queue.write_texture(
            white.as_image_copy(),
            &[255; 4],
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4),
                rows_per_image: None,
            },
            white.size(),
        );
        let mut renderer = Self {
            layout,
            module,
            pipelines: HashMap::new(),
            globals_layout,
            shadow_layout,
            shadow_sampler,
            shadow_pipeline,
            shadow_cutout_pipeline,
            shadow_map: None,
            shadow_distance: 25.0,
            shadow_resolution: 2048,
            shadow_quality: ShadowQuality::All,
            soft_shadow_radius: 1,
            env_pass: None,
            env_map: environment::EnvMap::new(gpu, 1),
            sky: None,
            sky_dirty: false,
            reflection_targets: Vec::new(),
            env_sampler: device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("environment"),
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                mipmap_filter: wgpu::MipmapFilterMode::Linear,
                ..Default::default()
            }),
            environment_intensity: 1.0,
            ambient_from_sky: true,
            texture_layout,
            globals: Self::buffer(
                gpu,
                "world globals",
                stride * 4,
                wgpu::BufferUsages::UNIFORM,
            ),
            stride,
            quads: Self::storage::<GpuQuad>(gpu, "world quads", 64),
            mesh_instances: Self::storage::<GpuMesh>(gpu, "world meshes", 64),
            meshes: HashMap::new(),
            stats: WorldStats::default(),
            textures: HashMap::new(),
            samplers: HashMap::new(),
            material_groups: HashMap::new(),
            anisotropic: Anisotropic::Enable,
            texture_mip_limit: 0,
            texture_quality: (Anisotropic::Enable, 0),
            sampler,
            white: white.create_view(&Default::default()),
            depth: HashMap::new(),
            msaa_targets: HashMap::new(),
            msaa: 1,
            render_scale: 1.0,
            upscale: None,
            probe_maps: HashMap::new(),
            probe_faces: HashMap::new(),
            bloom: None,
            bloom_pass: None,
        };
        for (i, name) in PRIMITIVES.iter().enumerate() {
            let data = primitive(name).expect("built in");
            renderer.set_mesh(gpu, PRIMITIVE_BASE + i as u32, &data);
        }
        renderer.set_texture(gpu, DEFAULT_PARTICLE_TEXTURE, &texture::soft_spot(64));
        renderer.set_texture(gpu, MISSING_TEXTURE, &texture::missing());
        renderer.set_mesh(gpu, BEAM_MESH, &mesh::beam());
        renderer
    }

    fn buffer(gpu: &Gpu, label: &str, size: u64, usage: wgpu::BufferUsages) -> wgpu::Buffer {
        gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: size.max(16),
            usage: usage | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    fn storage<T>(gpu: &Gpu, label: &str, count: usize) -> wgpu::Buffer {
        Self::buffer(
            gpu,
            label,
            (count.max(1) * std::mem::size_of::<T>()) as u64,
            wgpu::BufferUsages::STORAGE,
        )
    }

    /// Uploads `data` to `buffer`, growing it first if it's too small.
    fn upload<T: Pod>(gpu: &Gpu, buffer: &mut wgpu::Buffer, label: &str, data: &[T]) {
        let bytes = std::mem::size_of_val(data) as u64;
        if buffer.size() < bytes {
            *buffer = Self::storage::<T>(gpu, label, data.len().next_power_of_two());
        }
        if !data.is_empty() {
            gpu.queue
                .write_buffer(buffer, 0, bytemuck::cast_slice(data));
        }
    }

    /// Sets (or replaces) a mesh scripts can draw by `id` (below [`PRIMITIVE_BASE`]).
    /// Incomplete data gets normals and texture coordinates made up; invalid data is
    /// refused.
    pub fn set_mesh(&mut self, gpu: &Gpu, id: u32, data: &MeshData) -> bool {
        let data = data.clone().complete();
        if !data.is_valid() || data.indices.is_empty() {
            return false;
        }
        let vertices: Vec<Vertex> = (0..data.positions.len())
            .map(|i| Vertex {
                position: data.positions[i],
                normal: data.normals[i],
                uv: data.uvs[i],
                tangent: data.tangents[i],
            })
            .collect();
        let make = |label: &str, bytes: &[u8], usage: wgpu::BufferUsages| {
            let buffer = Self::buffer(gpu, label, bytes.len() as u64, usage);
            gpu.queue.write_buffer(&buffer, 0, bytes);
            buffer
        };
        self.meshes.insert(
            id,
            MeshBuffers {
                vertices: make(
                    "mesh vertices",
                    bytemuck::cast_slice(&vertices),
                    wgpu::BufferUsages::VERTEX,
                ),
                indices: make(
                    "mesh indices",
                    bytemuck::cast_slice(&data.indices),
                    wgpu::BufferUsages::INDEX,
                ),
                count: data.indices.len() as u32,
                bounds: cull::bounds(&data.positions).unwrap_or_default(),
            },
        );
        true
    }

    pub fn remove_mesh(&mut self, id: u32) {
        if id < PRIMITIVE_BASE {
            self.meshes.remove(&id);
        }
    }

    /// What the last [`WorldRenderer::render_views`] drew and what culling skipped.
    pub fn stats(&self) -> WorldStats {
        self.stats
    }

    pub fn has_mesh(&self, id: u32) -> bool {
        self.meshes.contains_key(&id)
    }

    /// Sets (or replaces) texture `id`, drawn with [`WorldTexture::Asset`]. Invalid
    /// data is refused.
    pub fn set_texture(&mut self, gpu: &Gpu, id: u32, data: &TextureData) -> bool {
        if !data.is_valid() {
            return false;
        }
        let max = gpu.device.limits().max_texture_dimension_2d;
        if data.width > max || data.height > max {
            tracing::warn!(
                "texture {id} is {}x{}, over the GPU's {max}",
                data.width,
                data.height
            );
            return false;
        }
        let mut levels = vec![(data.rgba.clone(), data.width, data.height)];
        if data.mipmaps {
            while let Some((px, w, h)) = levels.last()
                && (*w > 1 || *h > 1)
            {
                let next = texture::half(px, *w, *h, data.srgb);
                levels.push(next);
            }
        }
        // sRGB textures can also be read as data (a photo used as a bump map), and data
        // textures are the same either way.
        let format = if data.srgb {
            wgpu::TextureFormat::Rgba8UnormSrgb
        } else {
            wgpu::TextureFormat::Rgba8Unorm
        };
        let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("world texture"),
            size: wgpu::Extent3d {
                width: data.width,
                height: data.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: levels.len() as u32,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[wgpu::TextureFormat::Rgba8Unorm],
        });
        for (level, (px, w, h)) in levels.iter().enumerate() {
            gpu.queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: level as u32,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                px,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(w * 4),
                    rows_per_image: None,
                },
                wgpu::Extent3d {
                    width: *w,
                    height: *h,
                    depth_or_array_layers: 1,
                },
            );
        }
        let srgb = texture.create_view(&Default::default());
        let linear = texture.create_view(&wgpu::TextureViewDescriptor {
            format: Some(wgpu::TextureFormat::Rgba8Unorm),
            ..Default::default()
        });
        self.textures.insert(
            id,
            AssetTexture {
                srgb,
                linear,
                filter: data.filter,
                wrap: data.wrap,
                anisotropy: data.anisotropy,
                size: (data.width, data.height),
            },
        );
        self.material_groups.clear();
        true
    }

    pub fn remove_texture(&mut self, id: u32) {
        if id == MISSING_TEXTURE || id == DEFAULT_PARTICLE_TEXTURE {
            return;
        }
        self.textures.remove(&id);
        self.material_groups.clear();
    }

    /// A kept texture's size in pixels.
    pub fn texture_size(&self, id: u32) -> Option<(u32, u32)> {
        self.textures.get(&id).map(|t| t.size)
    }

    /// Sets (or clears) the sky: the environment reflections and sky light come from, and
    /// what views with `skybox` show behind everything. Redrawn on the next render.
    pub fn set_sky(&mut self, sky: Option<Sky>) {
        if self.sky != sky {
            self.sky = sky;
            self.sky_dirty = true;
        }
    }

    pub fn sky(&self) -> Option<&Sky> {
        self.sky.as_ref()
    }

    /// Draws the sky into a fresh environment map (or drops it with no sky).
    fn redraw_sky(&mut self, gpu: &Gpu) {
        let Some(sky) = self.sky else {
            self.env_map = environment::EnvMap::new(gpu, 1);
            return;
        };
        let pass = self
            .env_pass
            .get_or_insert_with(|| environment::EnvPass::new(gpu));
        let map = environment::EnvMap::new(gpu, 128);
        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        pass.draw_sky(gpu, &mut encoder, &map, &sky);
        gpu.queue.submit([encoder.finish()]);
        self.env_map = map;
    }

    /// The shadow map, made (or remade) at `shadow_resolution`.
    fn shadow_view(&mut self, gpu: &Gpu) -> wgpu::TextureView {
        let res = self
            .shadow_resolution
            .clamp(16, gpu.device.limits().max_texture_dimension_2d);
        if self.shadow_map.as_ref().is_none_or(|m| m.1 != res) {
            let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("world shadow map"),
                size: wgpu::Extent3d {
                    width: res,
                    height: res,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: DEPTH_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            self.shadow_map = Some((texture.create_view(&Default::default()), res));
        }
        self.shadow_map.as_ref().expect("just made").0.clone()
    }

    /// The light's view of a box `shadow_distance` across in front of the camera,
    /// moved only in whole texels so shadow edges don't crawl as the camera moves.
    fn shadow_matrix(&self, camera: &WorldCamera, direction: Vec3) -> Mat4 {
        let radius = (self.shadow_distance / 2.0).max(0.5);
        let center = camera.position + camera.forward * radius;
        let dir = direction.normalize_or(Vec3::NEG_Y);
        let up = if dir.y.abs() > 0.99 { Vec3::Z } else { Vec3::Y };
        let eye = center - dir * radius * 3.0;
        let view = iron_core::glam::camera::rh::view::look_at_mat4(eye, center, up);
        let texel = 2.0 * radius / self.shadow_resolution.max(1) as f32;
        let at = view.transform_point3(center);
        let snap = Vec3::new(
            (at.x / texel).round() * texel - at.x,
            (at.y / texel).round() * texel - at.y,
            0.0,
        );
        let proj = iron_core::glam::camera::rh::proj::directx::orthographic(
            -radius,
            radius,
            -radius,
            radius,
            0.01,
            radius * 6.0,
        );
        proj * Mat4::from_translation(snap) * view
    }

    /// The most samples per pixel up to `wanted` that the GPU can render `format` (and
    /// depth) with.
    fn supported_samples(gpu: &Gpu, format: wgpu::TextureFormat, wanted: u32) -> u32 {
        // Without the adapter-specific feature, only WebGPU's guaranteed 1 and 4 work.
        let specific = gpu
            .device
            .features()
            .contains(wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES);
        let color = gpu.adapter.get_texture_format_features(format).flags;
        let depth = gpu.adapter.get_texture_format_features(DEPTH_FORMAT).flags;
        [8, 4, 2]
            .into_iter()
            .filter(|&n| specific || n == 4)
            .find(|&n| {
                n <= wanted && color.sample_count_supported(n) && depth.sample_count_supported(n)
            })
            .unwrap_or(1)
    }

    /// The multisampled colour target for `size` and `format`.
    fn msaa_target(
        &mut self,
        gpu: &Gpu,
        size: (u32, u32),
        format: wgpu::TextureFormat,
        samples: u32,
    ) -> wgpu::TextureView {
        if self.msaa_targets.len() > 4 && !self.msaa_targets.contains_key(&(size, format, samples))
        {
            self.msaa_targets.clear();
        }
        self.msaa_targets
            .entry((size, format, samples))
            .or_insert_with(|| {
                gpu.device
                    .create_texture(&wgpu::TextureDescriptor {
                        label: Some("world msaa"),
                        size: wgpu::Extent3d {
                            width: size.0,
                            height: size.1,
                            depth_or_array_layers: 1,
                        },
                        mip_level_count: 1,
                        sample_count: samples,
                        dimension: wgpu::TextureDimension::D2,
                        format,
                        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                        view_formats: &[],
                    })
                    .create_view(&Default::default())
            })
            .clone()
    }

    fn pipelines(&mut self, gpu: &Gpu, format: wgpu::TextureFormat, samples: u32) -> Pipelines {
        let (layout, module) = (&self.layout, &self.module);
        let blend = Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING);
        let vertex_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &VERTEX_ATTRIBUTES,
        };
        let make = |label: &str,
                    entry: (&str, &str),
                    buffers: &[Option<wgpu::VertexBufferLayout<'_>>],
                    cull: Option<wgpu::Face>,
                    depth: (wgpu::CompareFunction, bool),
                    blend: Option<wgpu::BlendState>| {
            gpu.device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some(label),
                    layout: Some(layout),
                    vertex: wgpu::VertexState {
                        module,
                        entry_point: Some(entry.0),
                        compilation_options: Default::default(),
                        buffers,
                    },
                    primitive: wgpu::PrimitiveState {
                        cull_mode: cull,
                        ..Default::default()
                    },
                    depth_stencil: Some(wgpu::DepthStencilState {
                        format: DEPTH_FORMAT,
                        depth_write_enabled: Some(depth.1),
                        depth_compare: Some(depth.0),
                        stencil: Default::default(),
                        bias: Default::default(),
                    }),
                    multisample: wgpu::MultisampleState {
                        count: samples,
                        ..Default::default()
                    },
                    fragment: Some(wgpu::FragmentState {
                        module,
                        entry_point: Some(entry.1),
                        compilation_options: Default::default(),
                        targets: &[Some(wgpu::ColorTargetState {
                            format,
                            blend,
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                    }),
                    multiview_mask: None,
                    cache: None,
                })
        };
        let less = wgpu::CompareFunction::Less;
        // Quads at the same depth (2D sprites) draw in their sorted order: a later one
        // isn't hidden by the depth an earlier one's soft edge wrote.
        let less_equal = wgpu::CompareFunction::LessEqual;
        self.pipelines
            .entry((format, samples))
            .or_insert_with(|| Pipelines {
                // Planes are seen from both sides.
                quad: make(
                    "world quads",
                    ("vs_quad", "fs_quad"),
                    &[],
                    None,
                    (less_equal, true),
                    blend,
                ),
                quad_glow: make(
                    "world glowing quads",
                    ("vs_quad", "fs_quad"),
                    &[],
                    None,
                    (less_equal, false),
                    blend,
                ),
                // The background replaces colour and resets depth in the viewport.
                fill: make(
                    "world background",
                    ("vs_fill", "fs_fill"),
                    &[],
                    None,
                    (wgpu::CompareFunction::Always, true),
                    None,
                ),
                mesh: make(
                    "world meshes",
                    ("vs_mesh", "fs_mesh"),
                    &[Some(vertex_layout.clone())],
                    Some(wgpu::Face::Back),
                    (less, true),
                    blend,
                ),
                mesh_glow: make(
                    "world glowing meshes",
                    ("vs_mesh", "fs_mesh"),
                    &[Some(vertex_layout.clone())],
                    None,
                    (less, false),
                    blend,
                ),
                // See-through meshes don't hide what's drawn after them.
                mesh_blend: make(
                    "world see-through meshes",
                    ("vs_mesh", "fs_mesh"),
                    &[Some(vertex_layout.clone())],
                    Some(wgpu::Face::Back),
                    (less, false),
                    blend,
                ),
                mesh_double: make(
                    "world double-sided meshes",
                    ("vs_mesh", "fs_mesh"),
                    &[Some(vertex_layout.clone())],
                    None,
                    (less, true),
                    blend,
                ),
                mesh_blend_double: make(
                    "world double-sided see-through meshes",
                    ("vs_mesh", "fs_mesh"),
                    &[Some(vertex_layout.clone())],
                    None,
                    (less, false),
                    blend,
                ),
            })
            .clone()
    }

    fn depth_view(&mut self, gpu: &Gpu, size: (u32, u32), samples: u32) -> wgpu::TextureView {
        if let Some(view) = self.depth.get(&(size, samples)) {
            return view.clone();
        }
        let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("world depth"),
            size: wgpu::Extent3d {
                width: size.0,
                height: size.1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: samples,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = texture.create_view(&Default::default());
        // A few sizes at most: the screen, its mirrors, probe faces.
        if self.depth.len() > 8 {
            self.depth.clear();
        }
        self.depth.insert((size, samples), view.clone());
        view
    }

    /// Clears `target` to the camera's colour and draws `quads` into all of it.
    pub fn render(
        &mut self,
        gpu: &Gpu,
        target: &RenderTarget<'_>,
        camera: &WorldCamera,
        quads: &[WorldQuad],
        textures: &[&wgpu::TextureView],
    ) {
        let mut view = WorldView::new(*camera, [0.0, 0.0, 1.0, 1.0]);
        view.quads = quads.to_vec();
        self.render_views(gpu, target, &[view], textures);
    }

    /// Draws each camera's view into its viewport, in order. `target` is cleared to
    /// black first; parts no view covers stay black. `textures` are the textures
    /// quads and meshes refer to by index.
    pub fn render_views(
        &mut self,
        gpu: &Gpu,
        target: &RenderTarget<'_>,
        views: &[WorldView],
        textures: &[&wgpu::TextureView],
    ) {
        self.stats = WorldStats::default();
        let scale = if self.render_scale.is_finite() {
            self.render_scale.clamp(0.25, 2.0)
        } else {
            1.0
        };
        let scaled = (
            ((target.size.0 as f32 * scale).round() as u32).max(1),
            ((target.size.1 as f32 * scale).round() as u32).max(1),
        );
        if scaled == target.size {
            self.render_views_unscaled(gpu, target, views, textures);
            return;
        }
        // Drawn at the scaled size, then stretched onto the target.
        let up = self
            .upscale
            .get_or_insert_with(|| upscale::Upscale::new(gpu));
        let texture = up.target(gpu, scaled, target.format);
        let rt = RenderTarget {
            view: &texture,
            format: target.format,
            size: scaled,
            scale: target.scale * scale,
        };
        self.render_views_unscaled(gpu, &rt, views, textures);
        if let Some(up) = self.upscale.as_mut() {
            up.blit(gpu, &texture, target.view, target.format);
        }
    }

    fn render_views_unscaled(
        &mut self,
        gpu: &Gpu,
        target: &RenderTarget<'_>,
        views: &[WorldView],
        textures: &[&wgpu::TextureView],
    ) {
        self.update_probes(gpu, views, textures);
        if !views
            .iter()
            .any(|v| v.meshes.iter().any(|m| m.mirror.is_some()))
        {
            self.render_views_with(gpu, target, views, textures, &[], self.msaa);
            return;
        }
        // Mirrors: each mirror in the main view (the biggest) gets the scene rendered from
        // its reflected camera first; other views see the sky in them.
        let main = (0..views.len())
            .max_by(|&a, &b| {
                let area = |v: &WorldView| v.viewport[2] * v.viewport[3];
                area(&views[a]).total_cmp(&area(&views[b]))
            })
            .unwrap_or(0);
        let mut views = views.to_vec();
        let mut reflections = Vec::new();
        let bloom = self.bloom.take();
        for i in 0..views[main].meshes.len() {
            let Some(mirror) = views[main].meshes[i].mirror else {
                continue;
            };
            let k = reflections.len();
            let view = reflection_view(&views[main], mirror);
            let texture = self.reflection_target(gpu, k, target.size);
            let rt = RenderTarget {
                view: &texture,
                format: bloom::HDR_FORMAT,
                size: target.size,
                scale: target.scale,
            };
            self.render_views_with(gpu, &rt, &[view], textures, &[], 1);
            views[main].meshes[i].maps.reflection = Some(WorldTexture::Reflection(k));
            reflections.push(texture);
        }
        self.bloom = bloom;
        for (j, v) in views.iter_mut().enumerate() {
            if j != main {
                for m in &mut v.meshes {
                    m.mirror = None;
                }
            }
        }
        self.render_views_with(gpu, target, &views, textures, &reflections, self.msaa);
    }

    /// Renders the main view's reflection probes that need it: six 90° views from each
    /// probe into a 2D target, copied into its cubemap's faces, then blurred down.
    fn update_probes(&mut self, gpu: &Gpu, views: &[WorldView], textures: &[&wgpu::TextureView]) {
        let main = (0..views.len()).max_by(|&a, &b| {
            let area = |v: &WorldView| v.viewport[2] * v.viewport[3];
            area(&views[a]).total_cmp(&area(&views[b]))
        });
        let Some(main) = main else {
            return;
        };
        let main_view = &views[main];
        self.probe_maps
            .retain(|id, _| main_view.probes.iter().any(|p| p.id == *id));
        for probe in main_view.probes.clone() {
            let res = probe.resolution.clamp(16, 1024).next_power_of_two();
            let current = self
                .probe_maps
                .get(&probe.id)
                .is_some_and(|m| m.size == res);
            if current && !probe.render {
                continue;
            }
            let map = match self.probe_maps.remove(&probe.id) {
                Some(m) if m.size == res => m,
                _ => environment::EnvMap::new(gpu, res),
            };
            let face_target = self.probe_face_target(gpu, res);
            let bloom = self.bloom.take();
            for face in 0..6u32 {
                let (forward, up) = environment::face_camera(face);
                let world = iron_core::glam::camera::rh::view::look_at_mat4(
                    probe.position,
                    probe.position + forward,
                    up,
                )
                .inverse();
                let mut v = main_view.clone();
                v.camera = WorldCamera::new(
                    world,
                    Projection::Perspective {
                        fov_y_degrees: 90.0,
                    },
                    0.05,
                    1000.0,
                    (res, res),
                    main_view.camera.clear,
                );
                v.viewport = [0.0, 0.0, 1.0, 1.0];
                v.clip_plane = None;
                // Probes don't see probes or mirrors (those reflect the sky here), nor
                // layers outside their mask.
                v.probes.clear();
                v.meshes.retain(|m| in_mask(probe.culling_mask, m.layer));
                v.quads.retain(|q| in_mask(probe.culling_mask, q.layer));
                for m in &mut v.meshes {
                    m.mirror = None;
                }
                let rt = RenderTarget {
                    view: &face_target,
                    format: bloom::HDR_FORMAT,
                    size: (res, res),
                    scale: 1.0,
                };
                self.render_views_with(gpu, &rt, &[v], textures, &[], 1);
                let pass = self
                    .env_pass
                    .get_or_insert_with(|| environment::EnvPass::new(gpu));
                let mut encoder = gpu.device.create_command_encoder(&Default::default());
                pass.copy_face(gpu, &mut encoder, &face_target, &map, face);
                gpu.queue.submit([encoder.finish()]);
            }
            let pass = self
                .env_pass
                .get_or_insert_with(|| environment::EnvPass::new(gpu));
            let mut encoder = gpu.device.create_command_encoder(&Default::default());
            pass.blur_mips(gpu, &mut encoder, &map);
            gpu.queue.submit([encoder.finish()]);
            self.bloom = bloom;
            self.probe_maps.insert(probe.id, map);
        }
    }

    fn probe_face_target(&mut self, gpu: &Gpu, res: u32) -> wgpu::TextureView {
        self.probe_faces
            .entry(res)
            .or_insert_with(|| {
                gpu.device
                    .create_texture(&wgpu::TextureDescriptor {
                        label: Some("probe face"),
                        size: wgpu::Extent3d {
                            width: res,
                            height: res,
                            depth_or_array_layers: 1,
                        },
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: wgpu::TextureDimension::D2,
                        format: bloom::HDR_FORMAT,
                        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                            | wgpu::TextureUsages::TEXTURE_BINDING,
                        view_formats: &[],
                    })
                    .create_view(&Default::default())
            })
            .clone()
    }

    /// The bind group for a material's textures. Its sampler is the main texture's
    /// (with the quality's anisotropy and mip limit); maps share it.
    fn material_group(
        &mut self,
        gpu: &Gpu,
        key: MaterialKey,
        frame: &[&wgpu::TextureView],
        reflections: &[wgpu::TextureView],
    ) -> wgpu::BindGroup {
        let sampler_key = match key.base {
            Some(WorldTexture::Asset(id)) => {
                let t = self
                    .textures
                    .get(&id)
                    .unwrap_or(&self.textures[&MISSING_TEXTURE]);
                Some(SamplerKey {
                    filter: t.filter,
                    wrap: t.wrap,
                    anisotropy: self.anisotropic.level(t.anisotropy),
                    mip_limit: self.texture_mip_limit,
                })
            }
            _ => None,
        };
        let sampler = match sampler_key {
            Some(k) => self
                .samplers
                .entry(k)
                .or_insert_with(|| texture::sampler(&gpu.device, k))
                .clone(),
            None => self.sampler.clone(),
        };
        let view = |t: Option<WorldTexture>, linear: bool| -> &wgpu::TextureView {
            match t {
                None => &self.white,
                Some(WorldTexture::Frame(i)) => frame.get(i).copied().unwrap_or(&self.white),
                Some(WorldTexture::Asset(id)) => {
                    let a = self
                        .textures
                        .get(&id)
                        .unwrap_or(&self.textures[&MISSING_TEXTURE]);
                    if linear { &a.linear } else { &a.srgb }
                }
                Some(WorldTexture::Reflection(k)) => reflections.get(k).unwrap_or(&self.white),
            }
        };
        let views = [
            (0, view(key.base, false)),
            (2, view(key.normal, true)),
            (3, view(key.metallic_gloss, !key.metallic_srgb)),
            (4, view(key.occlusion, true)),
            (5, view(key.emission, false)),
            (6, view(key.detail, false)),
            (7, view(key.reflection, false)),
        ];
        let mut entries: Vec<wgpu::BindGroupEntry<'_>> = views
            .iter()
            .map(|&(binding, v)| wgpu::BindGroupEntry {
                binding,
                resource: wgpu::BindingResource::TextureView(v),
            })
            .collect();
        entries.push(wgpu::BindGroupEntry {
            binding: 1,
            resource: wgpu::BindingResource::Sampler(&sampler),
        });
        gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("world material"),
            layout: &self.texture_layout,
            entries: &entries,
        })
    }

    /// The texture mirror `k` reflects into, made (or remade) at `size`.
    fn reflection_target(&mut self, gpu: &Gpu, k: usize, size: (u32, u32)) -> wgpu::TextureView {
        if self.reflection_targets.len() <= k {
            self.reflection_targets.resize(k + 1, None);
        }
        if let Some((view, s)) = &self.reflection_targets[k]
            && *s == size
        {
            return view.clone();
        }
        let view = gpu
            .device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("mirror reflection"),
                size: wgpu::Extent3d {
                    width: size.0,
                    height: size.1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: bloom::HDR_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            })
            .create_view(&Default::default());
        self.reflection_targets[k] = Some((view.clone(), size));
        view
    }

    fn render_views_with(
        &mut self,
        gpu: &Gpu,
        target: &RenderTarget<'_>,
        views: &[WorldView],
        textures: &[&wgpu::TextureView],
        reflections: &[wgpu::TextureView],
        samples: u32,
    ) {
        let size = target.size;
        if size.0 == 0 || size.1 == 0 {
            return;
        }
        // With bloom, the world goes into an HDR texture first.
        let bloom = self.bloom.filter(|b| b.intensity > 0.0);
        let hdr = bloom.map(|_| {
            self.bloom_pass
                .get_or_insert_with(|| bloom::BloomPass::new(gpu))
                .hdr_view(gpu, size)
        });
        let (color_view, color_format) = match &hdr {
            Some(view) => (view, bloom::HDR_FORMAT),
            None => (target.view, target.format),
        };
        let samples = Self::supported_samples(gpu, color_format, samples);
        let pipelines = self.pipelines(gpu, color_format, samples);
        let depth = self.depth_view(gpu, size, samples);
        let msaa = (samples > 1).then(|| self.msaa_target(gpu, size, color_format, samples));
        let shadow_view = self.shadow_view(gpu);
        if self.sky_dirty {
            self.sky_dirty = false;
            self.redraw_sky(gpu);
        }

        // Everything for every view in shared buffers, with runs per view: opaque meshes
        // (grouped by mesh), then see-through meshes and quads back to front.
        let mut quads: Vec<GpuQuad> = Vec::new();
        let mut meshes: Vec<GpuMesh> = Vec::new();
        let mut lights: Vec<GpuLight> = Vec::new();
        let mut per_view: Vec<ViewRuns> = Vec::new();
        for view in views {
            let eye = view.camera.position;
            // How far along the view something is: straight-line distance in
            // perspective, distance along the view direction in orthographic (where
            // everything at one depth is equally far, like Unity's 2D sorting).
            let forward = view.camera.forward;
            let orthographic = view.camera.orthographic;
            let depth_of = |model: Mat4| {
                let at = model.w_axis.xyz();
                if orthographic {
                    (at - eye).dot(forward)
                } else {
                    at.distance_squared(eye)
                }
            };
            // The probe a mesh reflects: the smallest rendered one whose box holds its
            // centre.
            let probe_for = |m: &MeshInstance| -> Option<&ReflectionProbe> {
                let at = m.model.w_axis.xyz();
                view.probes
                    .iter()
                    .filter(|p| self.probe_maps.contains_key(&p.id))
                    .filter(|p| ((at - p.position).abs() - p.half_extents).max_element() <= 0.0)
                    .min_by(|a, b| {
                        let size = |p: &ReflectionProbe| {
                            p.half_extents.x * p.half_extents.y * p.half_extents.z
                        };
                        size(a).total_cmp(&size(b))
                    })
            };
            let gpu_mesh = |m: &MeshInstance| {
                let normal = Mat3::from_mat4(m.model).inverse().transpose();
                GpuMesh {
                    model: m.model.to_cols_array_2d(),
                    normal: Mat4::from_mat3(normal).to_cols_array_2d(),
                    color: m.color,
                    emission: [m.emission[0], m.emission[1], m.emission[2], m.smoothness],
                    params: [
                        if m.unlit { 1.0 } else { 0.0 },
                        match m.glow {
                            MeshGlow::None => 0.0,
                            MeshGlow::Additive => 1.0,
                            MeshGlow::Beam => 2.0,
                        },
                        m.metallic,
                        f32::from(u8::from(m.mirror.is_some())),
                    ],
                    tiling: m.tiling,
                    probe: probe_for(m).map_or([0.0; 4], |p| {
                        let mips = self.probe_maps.get(&p.id).map_or(0, |map| map.mips);
                        [p.position.x, p.position.y, p.position.z, mips as f32]
                    }),
                    probe_box: probe_for(m).map_or([0.0; 4], |p| {
                        let h = p.half_extents;
                        [h.x, h.y, h.z, f32::from(u8::from(p.box_projection))]
                    }),
                    layer: [
                        1 << (m.layer & 31),
                        [
                            (m.maps.normal.is_some(), map_flags::NORMAL),
                            (m.maps.metallic_gloss.is_some(), map_flags::METALLIC_GLOSS),
                            (m.maps.gltf_packing, map_flags::GLTF),
                            (m.maps.occlusion.is_some(), map_flags::OCCLUSION),
                            (m.maps.emission.is_some(), map_flags::EMISSION),
                            (m.maps.detail.is_some(), map_flags::DETAIL),
                        ]
                        .iter()
                        .filter(|(on, _)| *on)
                        .fold(0, |bits, (_, bit)| bits | bit),
                        match m.mode {
                            RenderMode::Opaque if m.color[3] < 1.0 => 2,
                            RenderMode::Opaque => 0,
                            RenderMode::Cutout => 1,
                            RenderMode::Fade => 2,
                            RenderMode::Transparent => 3,
                        },
                        0,
                    ],
                    specular: m.specular.map_or([0.0; 4], |[r, g, b]| [r, g, b, 1.0]),
                    maps: [
                        m.maps.normal_scale,
                        m.maps.occlusion_strength.clamp(0.0, 1.0),
                        m.alpha_cutoff,
                        0.0,
                    ],
                    detail: m.maps.detail_tiling,
                }
            };
            // Only layers in the shadowing light's mask cast its shadows.
            let shadow_mask = view
                .lights
                .iter()
                .find(|l| {
                    l.kind == LightKind::Directional && l.shadows.is_some_and(|s| s.strength > 0.0)
                })
                .map_or(ALL_LAYERS, |l| l.culling_mask);
            let casts = |m: &MeshInstance| m.cast_shadows && in_mask(shadow_mask, m.layer);
            let known = |m: &&MeshInstance| self.meshes.contains_key(&m.mesh);
            let frustum = cull::Frustum::from_view_proj(view.camera.view_proj);
            let sees = |m: &MeshInstance| {
                self.meshes
                    .get(&m.mesh)
                    .is_none_or(|b| frustum.transformed(b.bounds, m.model))
            };
            // Opaque meshes, grouped by mesh and texture (instancing).
            let mut opaque: Vec<&MeshInstance> = view
                .meshes
                .iter()
                .filter(known)
                .filter(|m| !m.blended())
                // Out of view, they may still cast shadows into it.
                .filter(|m| sees(m) || casts(m))
                .collect();
            let cutout = |m: &MeshInstance| m.mode == RenderMode::Cutout;
            opaque.sort_by_key(|m| {
                (
                    m.mesh,
                    m.material_key(),
                    m.double_sided,
                    cutout(m),
                    casts(m),
                    sees(m),
                    probe_for(m).map(|p| p.id),
                )
            });
            let mut opaque_runs: Vec<Run> = Vec::new();
            for m in opaque {
                let i = meshes.len() as u32;
                meshes.push(gpu_mesh(m));
                match opaque_runs.last_mut() {
                    Some(r)
                        if r.mesh == Some(m.mesh)
                            && r.material == m.material_key()
                            && r.double_sided == m.double_sided
                            && r.cutout == cutout(m)
                            && r.casts == casts(m)
                            && r.visible == sees(m)
                            && r.probe == probe_for(m).map(|p| p.id)
                            && r.range.end == i =>
                    {
                        r.range.end += 1
                    }
                    _ => opaque_runs.push(Run {
                        mesh: Some(m.mesh),
                        material: m.material_key(),
                        double_sided: m.double_sided,
                        cutout: cutout(m),
                        probe: probe_for(m).map(|p| p.id),
                        glow: false,
                        casts: casts(m),
                        visible: sees(m),
                        range: i..i + 1,
                    }),
                }
            }
            // See-through meshes and quads, back to front together.
            enum Item<'a> {
                Mesh(&'a MeshInstance),
                Quad(&'a WorldQuad),
            }
            let mut items: Vec<(i32, f32, Item)> = view
                .meshes
                .iter()
                .filter(known)
                .filter(|m| m.blended())
                .filter(|m| sees(m))
                .map(|m| (0, depth_of(m.model), Item::Mesh(m)))
                .chain(
                    view.quads
                        .iter()
                        .filter(|q| frustum.transformed(cull::QUAD_BOUNDS, q.model))
                        .map(|q| (q.order, depth_of(q.model), Item::Quad(q))),
                )
                .collect();
            // Lower orders first, then far to near.
            items.sort_by(|a, b| a.0.cmp(&b.0).then(b.1.total_cmp(&a.1)));
            let mut blended: Vec<(bool, Run)> = Vec::new();
            for (_, _, item) in items {
                match item {
                    Item::Mesh(m) => {
                        let i = meshes.len() as u32;
                        meshes.push(gpu_mesh(m));
                        blended.push((
                            true,
                            Run {
                                mesh: Some(m.mesh),
                                material: m.material_key(),
                                double_sided: m.double_sided,
                                cutout: false,
                                probe: probe_for(m).map(|p| p.id),
                                glow: m.glow != MeshGlow::None,
                                casts: false,
                                visible: true,
                                range: i..i + 1,
                            },
                        ));
                    }
                    Item::Quad(q) => {
                        let i = quads.len() as u32;
                        let [r, g, b, a] = q.color;
                        // Premultiplied with no coverage: blending then just adds it.
                        let cover = if q.additive { 0.0 } else { a };
                        quads.push(GpuQuad {
                            model: q.model.to_cols_array_2d(),
                            color: [r * a, g * a, b * a, cover],
                            uv: q.uv,
                            params: [
                                f32::from(u8::from(matches!(
                                    q.texture,
                                    Some(WorldTexture::Asset(_))
                                ))),
                                0.0,
                                0.0,
                                0.0,
                            ],
                        });
                        match blended.last_mut() {
                            Some((false, run))
                                if run.material == MaterialKey::texture(q.texture)
                                    && run.glow == q.additive
                                    && run.range.end == i =>
                            {
                                run.range.end += 1
                            }
                            _ => blended.push((
                                false,
                                Run {
                                    mesh: None,
                                    material: MaterialKey::texture(q.texture),
                                    double_sided: true,
                                    cutout: false,
                                    probe: None,
                                    glow: q.additive,
                                    casts: false,
                                    visible: true,
                                    range: i..i + 1,
                                },
                            )),
                        }
                    }
                }
            }
            // Point and spot lights whose range doesn't reach the view are left out.
            let kept: Vec<usize> = view
                .lights
                .iter()
                .enumerate()
                .filter(|(_, l)| {
                    l.kind == LightKind::Directional || frustum.sphere(l.position, l.range)
                })
                .map(|(i, _)| i)
                .collect();
            let drawn = opaque_runs
                .iter()
                .filter(|r| r.visible)
                .chain(blended.iter().filter(|(mesh, _)| *mesh).map(|(_, r)| r))
                .map(|r| r.range.len())
                .sum::<usize>();
            let all_meshes = view.meshes.iter().filter(known).count();
            let drawn_quads = blended
                .iter()
                .filter(|(mesh, _)| !*mesh)
                .map(|(_, r)| r.range.len())
                .sum::<usize>();
            self.stats.meshes += drawn;
            self.stats.culled_meshes += all_meshes - drawn;
            self.stats.quads += drawn_quads;
            self.stats.culled_quads += view.quads.len() - drawn_quads;
            self.stats.lights += kept.len();
            self.stats.culled_lights += view.lights.len() - kept.len();
            let first_light = lights.len() as u32;
            lights.extend(kept.iter().map(|&k| &view.lights[k]).map(|l| {
                let (kind, spot) = match l.kind {
                    LightKind::Directional => (0.0, [0.0; 4]),
                    LightKind::Point => (1.0, [0.0; 4]),
                    LightKind::Spot {
                        inner_degrees,
                        outer_degrees,
                    } => (
                        2.0,
                        [
                            (inner_degrees.min(outer_degrees).to_radians() / 2.0).cos(),
                            (outer_degrees.to_radians() / 2.0).cos(),
                            0.0,
                            0.0,
                        ],
                    ),
                };
                GpuLight {
                    position: [l.position.x, l.position.y, l.position.z, kind],
                    direction: [l.direction.x, l.direction.y, l.direction.z, l.range],
                    color: [l.color[0], l.color[1], l.color[2], 0.0],
                    spot,
                    mask: [l.culling_mask, 0, 0, 0],
                }
            }));
            per_view.push((opaque_runs, blended, first_light..lights.len() as u32, kept));
        }
        Self::upload(gpu, &mut self.quads, "world quads", &quads);
        Self::upload(gpu, &mut self.mesh_instances, "world meshes", &meshes);
        if self.globals.size() < self.stride * views.len().max(1) as u64 {
            self.globals = Self::buffer(
                gpu,
                "world globals",
                self.stride * views.len().next_power_of_two() as u64,
                wgpu::BufferUsages::UNIFORM,
            );
        }
        // Shadows: the first directional light that casts them, fitted to the camera
        // with the biggest viewport (the main view), shared by every view.
        let main_view = (0..views.len()).max_by(|&a, &b| {
            let area = |v: &WorldView| v.viewport[2] * v.viewport[3];
            area(&views[a]).total_cmp(&area(&views[b]))
        });
        let shadow = main_view.and_then(|m| {
            let view = &views[m];
            if self.shadow_quality == ShadowQuality::Disable {
                return None;
            }
            let (index, light) = view.lights.iter().enumerate().find(|(_, l)| {
                l.kind == LightKind::Directional && l.shadows.is_some_and(|s| s.strength > 0.0)
            })?;
            Some((
                m,
                index,
                light.shadows?,
                self.shadow_matrix(&view.camera, light.direction),
            ))
        });
        let shadow_texel = self.shadow_distance / self.shadow_resolution.max(1) as f32;
        let has_env = self.sky.is_some();
        let env = [
            f32::from(u8::from(has_env)),
            self.environment_intensity,
            (self.env_map.mips - 1) as f32,
            f32::from(u8::from(has_env && self.ambient_from_sky)),
        ];
        for (i, view) in views.iter().enumerate() {
            let c = view.camera.clear;
            let p = view.camera.position;
            let count = per_view[i].2.len() as f32;
            // The shadowing light (directional, so never culled) by its place among this
            // view's kept lights.
            let index = shadow
                .as_ref()
                .and_then(|(_, index, ..)| per_view[i].3.iter().position(|k| k == index));
            let (shadow_view_proj, shadow_params) = match (&shadow, index) {
                (Some((_, _, s, matrix)), Some(index)) => (
                    *matrix,
                    [
                        index as f32,
                        s.strength.clamp(0.0, 1.0),
                        shadow_texel,
                        // The PCF radius in texels: 0 is hard.
                        if s.soft && self.shadow_quality == ShadowQuality::All {
                            self.soft_shadow_radius.clamp(1, 2) as f32
                        } else {
                            0.0
                        },
                    ],
                ),
                _ => (Mat4::IDENTITY, [-1.0, 0.0, 0.0, 0.0]),
            };
            let globals = GpuGlobals {
                view_proj: view.camera.view_proj.to_cols_array_2d(),
                background: [
                    (c.r * c.a) as f32,
                    (c.g * c.a) as f32,
                    (c.b * c.a) as f32,
                    c.a as f32,
                ],
                camera: [p.x, p.y, p.z, 1.0],
                ambient: [view.ambient[0], view.ambient[1], view.ambient[2], count],
                shadow_view_proj: shadow_view_proj.to_cols_array_2d(),
                shadow: shadow_params,
                inv_view_proj: view.camera.view_proj.inverse().to_cols_array_2d(),
                viewport: viewport_px(view.viewport, size)
                    .map_or([0.0, 0.0, 1.0, 1.0], |p| p.map(|v| v as f32)),
                env,
                flags: [
                    f32::from(u8::from(view.skybox && has_env)),
                    size.0 as f32,
                    size.1 as f32,
                    0.0,
                ],
                clip: view.clip_plane.unwrap_or([0.0; 4]),
            };
            gpu.queue.write_buffer(
                &self.globals,
                self.stride * i as u64,
                bytemuck::bytes_of(&globals),
            );
        }
        // One lights buffer per view (they differ), bound with the view.
        let light_buffers: Vec<wgpu::Buffer> = per_view
            .iter()
            .map(|(_, _, range, _)| {
                let slice = &lights[range.start as usize..range.end as usize];
                let mut buffer = Self::storage::<GpuLight>(gpu, "world lights", slice.len());
                Self::upload(gpu, &mut buffer, "world lights", slice);
                buffer
            })
            .collect();
        let globals_groups: Vec<wgpu::BindGroup> = light_buffers
            .iter()
            .map(|lights| {
                gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("world globals"),
                    layout: &self.globals_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                                buffer: &self.globals,
                                offset: 0,
                                size: wgpu::BufferSize::new(
                                    std::mem::size_of::<GpuGlobals>() as u64
                                ),
                            }),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: self.quads.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: self.mesh_instances.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 3,
                            resource: lights.as_entire_binding(),
                        },
                    ],
                })
            })
            .collect();
        // Material bind groups: those of kept textures only are reused across frames;
        // ones with the frame's textures (panels, reflections) are made each time.
        let quality = (self.anisotropic, self.texture_mip_limit);
        if self.texture_quality != quality {
            self.texture_quality = quality;
            self.material_groups.clear();
        }
        let keys: std::collections::BTreeSet<MaterialKey> = per_view
            .iter()
            .flat_map(|(opaque, blended, ..)| opaque.iter().chain(blended.iter().map(|(_, r)| r)))
            .map(|r| r.material)
            .chain(std::iter::once(MaterialKey::default()))
            .collect();
        let mut frame_groups: HashMap<MaterialKey, wgpu::BindGroup> = HashMap::new();
        for key in keys {
            let per_frame = key.per_frame();
            if !per_frame && self.material_groups.contains_key(&key) {
                continue;
            }
            let group = self.material_group(gpu, key, textures, reflections);
            if per_frame {
                frame_groups.insert(key, group);
            } else {
                self.material_groups.insert(key, group);
            }
        }
        let group_for = |key: &MaterialKey| {
            frame_groups
                .get(key)
                .or_else(|| self.material_groups.get(key))
                .expect("made above")
        };
        let plain = group_for(&MaterialKey::default());

        // Shadows and the environment: the sky's, and one per reflection probe.
        let lighting_group = |cube: &wgpu::TextureView| {
            gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("world lighting"),
                layout: &self.shadow_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&shadow_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.shadow_sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(cube),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::Sampler(&self.env_sampler),
                    },
                ],
            })
        };
        let shadow_group = lighting_group(&self.env_map.cube);
        let probe_groups: HashMap<u32, wgpu::BindGroup> = self
            .probe_maps
            .iter()
            .map(|(id, map)| (*id, lighting_group(&map.cube)))
            .collect();
        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        if let Some((main, ..)) = shadow {
            // The casters: the main view's opaque meshes, from the light.
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("world shadow map"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &shadow_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            pass.set_bind_group(
                0,
                &globals_groups[main],
                &[(self.stride * main as u64) as u32],
            );
            for run in per_view[main].0.iter().filter(|r| r.casts) {
                if run.cutout {
                    pass.set_pipeline(&self.shadow_cutout_pipeline);
                    pass.set_bind_group(1, group_for(&run.material), &[]);
                } else {
                    pass.set_pipeline(&self.shadow_pipeline);
                }
                if let Some(mesh) = run.mesh.and_then(|m| self.meshes.get(&m)) {
                    pass.set_vertex_buffer(0, mesh.vertices.slice(..));
                    pass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint32);
                    pass.draw_indexed(0..mesh.count, 0, run.range.clone());
                }
            }
        }
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("world"),
                // With MSAA, draw into the multisampled target and resolve into the colour one.
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: msaa.as_ref().unwrap_or(color_view),
                    depth_slice: None,
                    resolve_target: msaa.as_ref().map(|_| color_view),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: if msaa.is_some() {
                            wgpu::StoreOp::Discard
                        } else {
                            wgpu::StoreOp::Store
                        },
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            for (i, (view, (opaque, blended, ..))) in views.iter().zip(&per_view).enumerate() {
                let Some(px) = viewport_px(view.viewport, size) else {
                    continue;
                };
                pass.set_viewport(
                    px[0] as f32,
                    px[1] as f32,
                    px[2] as f32,
                    px[3] as f32,
                    0.0,
                    1.0,
                );
                pass.set_scissor_rect(px[0], px[1], px[2], px[3]);
                pass.set_bind_group(0, &globals_groups[i], &[(self.stride * i as u64) as u32]);
                pass.set_bind_group(1, plain, &[]);
                pass.set_bind_group(2, &shadow_group, &[]);
                pass.set_pipeline(&pipelines.fill);
                pass.draw(0..3, 0..1);

                let draw = |pass: &mut wgpu::RenderPass<'_>, run: &Run, blend: bool| {
                    pass.set_bind_group(1, group_for(&run.material), &[]);
                    let lighting = run.probe.and_then(|id| probe_groups.get(&id));
                    pass.set_bind_group(2, lighting.unwrap_or(&shadow_group), &[]);
                    match run.mesh.and_then(|m| self.meshes.get(&m)) {
                        Some(mesh) => {
                            pass.set_pipeline(match (run.glow, blend, run.double_sided) {
                                (true, ..) => &pipelines.mesh_glow,
                                (false, true, false) => &pipelines.mesh_blend,
                                (false, true, true) => &pipelines.mesh_blend_double,
                                (false, false, false) => &pipelines.mesh,
                                (false, false, true) => &pipelines.mesh_double,
                            });
                            pass.set_vertex_buffer(0, mesh.vertices.slice(..));
                            pass.set_index_buffer(
                                mesh.indices.slice(..),
                                wgpu::IndexFormat::Uint32,
                            );
                            pass.draw_indexed(0..mesh.count, 0, run.range.clone());
                        }
                        None if run.mesh.is_none() => {
                            pass.set_pipeline(if run.glow {
                                &pipelines.quad_glow
                            } else {
                                &pipelines.quad
                            });
                            pass.draw(0..6, run.range.clone());
                        }
                        None => {}
                    }
                };
                for run in opaque.iter().filter(|r| r.visible) {
                    draw(&mut pass, run, false);
                }
                for (_, run) in blended {
                    draw(&mut pass, run, true);
                }
            }
        }
        if let (Some(settings), Some(pass)) = (bloom, self.bloom_pass.as_mut()) {
            pass.encode(gpu, &mut encoder, target.view, target.format, &settings);
        }
        gpu.queue.submit([encoder.finish()]);
    }
}

/// `view` seen in `mirror`: the camera reflected in its plane, and everything behind the
/// plane clipped away. The projection is flipped left to right so triangles keep their
/// winding (the reflection flips it once, the flip back); the mirror samples it flipped.
fn reflection_view(view: &WorldView, mirror: Mirror) -> WorldView {
    let n = mirror.normal.normalize_or(Vec3::Y);
    let d = -n.dot(mirror.point);
    // Reflection in the plane: x - 2 (n · x + d) n.
    let reflect = Mat4::from_cols(
        (Vec3::X - 2.0 * n.x * n).extend(0.0),
        (Vec3::Y - 2.0 * n.y * n).extend(0.0),
        (Vec3::Z - 2.0 * n.z * n).extend(0.0),
        (-2.0 * d * n).extend(1.0),
    );
    let flip = Mat4::from_scale(Vec3::new(-1.0, 1.0, 1.0));
    let mut v = view.clone();
    v.meshes
        .retain(|m| m.mirror.is_none() && in_mask(mirror.culling_mask, m.layer));
    v.quads.retain(|q| in_mask(mirror.culling_mask, q.layer));
    v.camera.view_proj = flip * view.camera.view_proj * reflect;
    v.camera.position = reflect.transform_point3(view.camera.position);
    v.camera.forward = reflect.transform_vector3(view.camera.forward);
    // Keep only what's in front of the mirror; a hair past the plane hides the seam.
    v.clip_plane = Some([n.x, n.y, n.z, d + 1e-3]);
    v
}

/// A normalized viewport in whole pixels inside the target (`None` when empty).
fn viewport_px(v: [f32; 4], size: (u32, u32)) -> Option<[u32; 4]> {
    let (w, h) = (size.0 as f32, size.1 as f32);
    let x0 = (v[0] * w).round().clamp(0.0, w);
    let y0 = (v[1] * h).round().clamp(0.0, h);
    let x1 = ((v[0] + v[2]) * w).round().clamp(0.0, w);
    let y1 = ((v[1] + v[3]) * h).round().clamp(0.0, h);
    (x1 > x0 && y1 > y0).then_some([x0 as u32, y0 as u32, (x1 - x0) as u32, (y1 - y0) as u32])
}

/// A normalized viewport's size in pixels, for a camera's aspect ratio.
pub fn viewport_size(v: [f32; 4], size: (u32, u32)) -> (u32, u32) {
    viewport_px(v, size).map_or((1, 1), |p| (p[2], p[3]))
}
