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
pub mod mesh;
pub mod texture;

use std::collections::HashMap;
use std::ops::Range;

use bytemuck::{Pod, Zeroable};
use iron_core::glam::{Mat3, Mat4, Vec3, Vec4Swizzles};

use crate::{Color, Gpu, RenderTarget};
pub use bloom::Bloom;
pub use mesh::{MeshData, primitive};
pub use texture::{TextureData, TextureFilter, TextureWrap};

/// A texture a quad or mesh is drawn with.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WorldTexture {
    /// Index into the `textures` given to [`WorldRenderer::render_views`].
    Frame(usize),
    /// One set with [`WorldRenderer::set_texture`] (the missing checker until it exists).
    Asset(u32),
}

const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

/// The open cone light beams are drawn with ([`mesh::beam`]).
pub const BEAM_MESH: u32 = PRIMITIVE_BASE + 0xFFFE;

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
    /// 0 (matte) to 1 (glossy): how sharp highlights are.
    pub smoothness: f32,
    /// Ignores lights: just its colour (and emission).
    pub unlit: bool,
    /// Multiplies the colour; `None` is plain colour.
    pub texture: Option<WorldTexture>,
    /// Texture tiling: scale (xy) and offset (zw), Unity's `mainTextureScale` /
    /// `mainTextureOffset` (in its bottom-up UV space).
    pub tiling: [f32; 4],
    pub glow: MeshGlow,
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
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct GpuLight {
    position: [f32; 4],
    direction: [f32; 4],
    color: [f32; 4],
    spot: [f32; 4],
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
    /// metres (for the normal offset); w: 1 for soft.
    shadow: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
    position: [f32; 3],
    normal: [f32; 3],
    uv: [f32; 2],
}

/// A mesh's buffers on the GPU.
struct MeshBuffers {
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    count: u32,
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
}

/// A view's opaque runs, its blended runs (`true`: a mesh) and its lights.
type ViewRuns = (Vec<Run>, Vec<(bool, Run)>, Range<u32>);

/// A run of instances drawn with one mesh or texture.
struct Run {
    mesh: Option<u32>,
    texture: Option<WorldTexture>,
    /// Additive quads (drawn without depth writes).
    glow: bool,
    range: Range<u32>,
}

pub struct WorldRenderer {
    layout: wgpu::PipelineLayout,
    module: wgpu::ShaderModule,
    pipelines: HashMap<wgpu::TextureFormat, Pipelines>,
    globals_layout: wgpu::BindGroupLayout,
    shadow_layout: wgpu::BindGroupLayout,
    shadow_sampler: wgpu::Sampler,
    shadow_pipeline: wgpu::RenderPipeline,
    /// The directional shadow map, made at `shadow_resolution`.
    shadow_map: Option<(wgpu::TextureView, u32)>,
    /// How far from the camera shadows reach (metres; Unity's `shadowDistance`).
    pub shadow_distance: f32,
    /// The shadow map's size in texels.
    pub shadow_resolution: u32,
    texture_layout: wgpu::BindGroupLayout,
    /// One `GpuGlobals` per view, `stride` bytes apart (a dynamic offset picks one).
    globals: wgpu::Buffer,
    stride: u64,
    quads: wgpu::Buffer,
    mesh_instances: wgpu::Buffer,
    meshes: HashMap<u32, MeshBuffers>,
    textures: HashMap<u32, AssetTexture>,
    samplers: HashMap<(TextureFilter, TextureWrap), wgpu::Sampler>,
    sampler: wgpu::Sampler,
    white: wgpu::TextureView,
    depth: Option<(wgpu::TextureView, (u32, u32))>,
    /// Bloom over every view (`None`: off).
    pub bloom: Option<Bloom>,
    /// Made the first time bloom is on.
    bloom_pass: Option<bloom::BloomPass>,
}

/// A texture the renderer keeps, bound with its own sampler.
struct AssetTexture {
    group: wgpu::BindGroup,
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
        let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("world texture"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
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
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x2],
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
            shadow_map: None,
            shadow_distance: 25.0,
            shadow_resolution: 2048,
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
            textures: HashMap::new(),
            samplers: HashMap::new(),
            sampler,
            white: white.create_view(&Default::default()),
            depth: None,
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
            },
        );
        true
    }

    pub fn remove_mesh(&mut self, id: u32) {
        if id < PRIMITIVE_BASE {
            self.meshes.remove(&id);
        }
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
                let next = texture::half(px, *w, *h);
                levels.push(next);
            }
        }
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
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
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
        let sampler = self
            .samplers
            .entry((data.filter, data.wrap))
            .or_insert_with(|| texture::sampler(&gpu.device, data.filter, data.wrap));
        let view = texture.create_view(&Default::default());
        let group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("world texture"),
            layout: &self.texture_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        });
        self.textures.insert(
            id,
            AssetTexture {
                group,
                size: (data.width, data.height),
            },
        );
        true
    }

    pub fn remove_texture(&mut self, id: u32) {
        if id == MISSING_TEXTURE || id == DEFAULT_PARTICLE_TEXTURE {
            return;
        }
        self.textures.remove(&id);
    }

    /// A kept texture's size in pixels.
    pub fn texture_size(&self, id: u32) -> Option<(u32, u32)> {
        self.textures.get(&id).map(|t| t.size)
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

    fn pipelines(&mut self, gpu: &Gpu, format: wgpu::TextureFormat) -> Pipelines {
        let (layout, module) = (&self.layout, &self.module);
        let blend = Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING);
        let vertex_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x2],
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
                    multisample: Default::default(),
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
            .entry(format)
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
            })
            .clone()
    }

    fn depth_view(&mut self, gpu: &Gpu, size: (u32, u32)) -> wgpu::TextureView {
        if let Some((view, s)) = &self.depth
            && *s == size
        {
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
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = texture.create_view(&Default::default());
        self.depth = Some((view.clone(), size));
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
        let pipelines = self.pipelines(gpu, color_format);
        let depth = self.depth_view(gpu, size);
        let shadow_view = self.shadow_view(gpu);

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
                        0.0,
                        0.0,
                    ],
                    tiling: m.tiling,
                }
            };
            let known = |m: &&MeshInstance| self.meshes.contains_key(&m.mesh);
            // Opaque meshes, grouped by mesh and texture (instancing).
            let mut opaque: Vec<&MeshInstance> = view
                .meshes
                .iter()
                .filter(known)
                .filter(|m| m.color[3] >= 1.0 && m.glow == MeshGlow::None)
                .collect();
            opaque.sort_by_key(|m| (m.mesh, m.texture));
            let mut opaque_runs: Vec<Run> = Vec::new();
            for m in opaque {
                let i = meshes.len() as u32;
                meshes.push(gpu_mesh(m));
                match opaque_runs.last_mut() {
                    Some(r)
                        if r.mesh == Some(m.mesh) && r.texture == m.texture && r.range.end == i =>
                    {
                        r.range.end += 1
                    }
                    _ => opaque_runs.push(Run {
                        mesh: Some(m.mesh),
                        texture: m.texture,
                        glow: false,
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
                .filter(|m| m.color[3] < 1.0 || m.glow != MeshGlow::None)
                .map(|m| (0, depth_of(m.model), Item::Mesh(m)))
                .chain(
                    view.quads
                        .iter()
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
                                texture: m.texture,
                                glow: m.glow != MeshGlow::None,
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
                                if run.texture == q.texture
                                    && run.glow == q.additive
                                    && run.range.end == i =>
                            {
                                run.range.end += 1
                            }
                            _ => blended.push((
                                false,
                                Run {
                                    mesh: None,
                                    texture: q.texture,
                                    glow: q.additive,
                                    range: i..i + 1,
                                },
                            )),
                        }
                    }
                }
            }
            let first_light = lights.len() as u32;
            lights.extend(view.lights.iter().map(|l| {
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
                }
            }));
            per_view.push((opaque_runs, blended, first_light..lights.len() as u32));
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
        let shadow_texel = 2.0 * self.shadow_distance / 2.0 / self.shadow_resolution.max(1) as f32;
        for (i, view) in views.iter().enumerate() {
            let c = view.camera.clear;
            let p = view.camera.position;
            let count = per_view[i].2.len() as f32;
            // Light indices are per view; every view has the same lights.
            let (shadow_view_proj, shadow_params) = match &shadow {
                Some((_, index, s, matrix)) => (
                    *matrix,
                    [
                        *index as f32,
                        s.strength.clamp(0.0, 1.0),
                        shadow_texel,
                        f32::from(u8::from(s.soft)),
                    ],
                ),
                None => (Mat4::IDENTITY, [-1.0, 0.0, 0.0, 0.0]),
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
            .map(|(_, _, range)| {
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
        let texture_group = |view: &wgpu::TextureView| {
            gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("world texture"),
                layout: &self.texture_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                ],
            })
        };
        let groups: Vec<wgpu::BindGroup> = std::iter::once(&self.white)
            .chain(textures.iter().copied())
            .map(texture_group)
            .collect();
        let group_for = |texture: Option<WorldTexture>| match texture {
            None => &groups[0],
            Some(WorldTexture::Frame(t)) => &groups[(t + 1).min(groups.len() - 1)],
            Some(WorldTexture::Asset(id)) => {
                let missing = &self.textures[&MISSING_TEXTURE].group;
                self.textures.get(&id).map_or(missing, |t| &t.group)
            }
        };

        let shadow_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("world shadows"),
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
            ],
        });
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
            pass.set_pipeline(&self.shadow_pipeline);
            pass.set_bind_group(
                0,
                &globals_groups[main],
                &[(self.stride * main as u64) as u32],
            );
            for run in &per_view[main].0 {
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
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: color_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
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
            for (i, (view, (opaque, blended, _))) in views.iter().zip(&per_view).enumerate() {
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
                pass.set_bind_group(1, &groups[0], &[]);
                pass.set_bind_group(2, &shadow_group, &[]);
                pass.set_pipeline(&pipelines.fill);
                pass.draw(0..3, 0..1);

                let draw = |pass: &mut wgpu::RenderPass<'_>, run: &Run, blend: bool| {
                    pass.set_bind_group(1, group_for(run.texture), &[]);
                    match run.mesh.and_then(|m| self.meshes.get(&m)) {
                        Some(mesh) => {
                            pass.set_pipeline(if run.glow {
                                &pipelines.mesh_glow
                            } else if blend {
                                &pipelines.mesh_blend
                            } else {
                                &pipelines.mesh
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
                for run in opaque {
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
