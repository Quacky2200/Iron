// SPDX-License-Identifier: Apache-2.0

//! Post-process passes: fragment shaders run in order over a view's finished picture (after
//! bloom, exposure, tonemapping and FXAA, so on the colours the screen shows).
//!
//! A [`PostShader`] is WGSL that defines `fn effect(uv: vec2<f32>, color: vec4<f32>) -> vec4<f32>`;
//! the renderer adds the vertex stage, the bindings, a few helpers (`scene`, `scene_depth`,
//! `scene_normal`, `view_uv`, `post_time`, ..) and one private variable per declared parameter.
//! A [`PostEffect`] is a shader with its parameter values; a view carries its effects in order
//! ([`WorldView::post`](super::WorldView::post)) and each runs over that view's part of the
//! target. Effects that need the scene's depth get it as a linear distance in world units, and
//! normals rebuilt from it, resolved once per frame from the world pass's depth buffer.
//!
//! The picture ping-pongs between two textures; the last pass of a full-target view draws
//! straight into the target. A shader that doesn't compile is logged once and skipped.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use bytemuck::{Pod, Zeroable};

use super::{WorldView, viewport_px};
use crate::Gpu;

/// How many parameters (a `vec4` slot each) an effect has.
pub const POST_PARAMS: usize = 8;

/// The type of a shader parameter, as WGSL sees the private variable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PostParamType {
    Float,
    Vec2,
    Vec3,
    Vec4,
}

impl PostParamType {
    fn wgsl(self) -> &'static str {
        match self {
            PostParamType::Float => "f32",
            PostParamType::Vec2 => "vec2<f32>",
            PostParamType::Vec3 => "vec3<f32>",
            PostParamType::Vec4 => "vec4<f32>",
        }
    }

    fn swizzle(self) -> &'static str {
        match self {
            PostParamType::Float => ".x",
            PostParamType::Vec2 => ".xy",
            PostParamType::Vec3 => ".xyz",
            PostParamType::Vec4 => "",
        }
    }
}

/// A post-process fragment shader.
#[derive(Debug)]
pub struct PostShader {
    /// Same source and parameters, same id: pipelines are cached by it.
    pub id: u64,
    /// For the log.
    pub name: String,
    /// WGSL defining `fn effect(uv: vec2<f32>, color: vec4<f32>) -> vec4<f32>`.
    pub source: String,
    /// The parameters in slot order: each is a private variable of this name and type, set
    /// from the effect's values before `effect` runs.
    pub params: Vec<(String, PostParamType)>,
    /// How many times it runs over the picture (1 to 4); `pass_index()` says which.
    pub passes: u32,
    /// Reads `scene_depth` / `scene_normal`.
    pub depth: bool,
}

impl PostShader {
    pub fn new(
        name: &str,
        source: &str,
        params: Vec<(String, PostParamType)>,
        passes: u32,
        depth: bool,
    ) -> Self {
        let mut h = std::collections::hash_map::DefaultHasher::new();
        source.hash(&mut h);
        for (n, t) in &params {
            n.hash(&mut h);
            (*t as u8).hash(&mut h);
        }
        Self {
            id: h.finish(),
            name: name.to_string(),
            source: source.to_string(),
            params,
            passes: passes.clamp(1, 4),
            depth,
        }
    }
}

/// A shader and its parameter values (a `vec4` per slot; a float uses `x`).
#[derive(Clone, Debug)]
pub struct PostEffect {
    pub shader: Arc<PostShader>,
    pub params: [[f32; 4]; POST_PARAMS],
}

impl PostEffect {
    pub fn new(shader: Arc<PostShader>) -> Self {
        Self {
            shader,
            params: [[0.0; 4]; POST_PARAMS],
        }
    }
}

/// What every post shader sees: the bindings and helpers (WGSL).
const PRELUDE: &str = r#"
struct Fx {
    p: array<vec4<f32>, 8>,
    // The view's part of the target, in uv: x, y, width, height.
    rect: vec4<f32>,
    // The target's size in pixels, and one over it.
    size: vec4<f32>,
    // Seconds, this pass, the passes, the frame.
    info: vec4<f32>,
};
@group(0) @binding(0) var<uniform> fx: Fx;
@group(0) @binding(1) var src_tex: texture_2d<f32>;
@group(0) @binding(2) var src_samp: sampler;
@group(0) @binding(3) var depth_tex: texture_2d<f32>;
@group(0) @binding(4) var normal_tex: texture_2d<f32>;

struct Out {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs(@builtin(vertex_index) vi: u32) -> Out {
    let p = vec2<f32>(f32((vi << 1u) & 2u), f32(vi & 2u));
    var o: Out;
    o.pos = vec4<f32>(p * 2.0 - 1.0, 0.0, 1.0);
    o.uv = vec2<f32>(p.x, 1.0 - p.y);
    return o;
}

// The picture before this pass at `uv` (bilinear, edges clamped).
fn scene(uv: vec2<f32>) -> vec4<f32> {
    return textureSampleLevel(src_tex, src_samp, uv, 0.0);
}
// The same at a pixel.
fn scene_px(p: vec2<i32>) -> vec4<f32> {
    let dims = vec2<i32>(textureDimensions(src_tex));
    return textureLoad(src_tex, clamp(p, vec2<i32>(0), dims - vec2<i32>(1)), 0);
}
fn clamp_px(uv: vec2<f32>) -> vec2<i32> {
    let dims = vec2<i32>(textureDimensions(depth_tex));
    return clamp(vec2<i32>(uv * fx.size.xy), vec2<i32>(0), dims - vec2<i32>(1));
}
// Distance along the view direction to what is drawn at `uv`, in world units (the far plane
// where nothing is). Only with `depth` set on the shader.
fn scene_depth(uv: vec2<f32>) -> f32 {
    return textureLoad(depth_tex, clamp_px(uv), 0).x;
}
// The world-space normal of what is drawn at `uv` (zero where nothing is), rebuilt from depth.
fn scene_normal(uv: vec2<f32>) -> vec3<f32> {
    return textureLoad(normal_tex, clamp_px(uv), 0).xyz;
}
// `uv` across the view's own part of the target, 0..1.
fn view_uv(uv: vec2<f32>) -> vec2<f32> {
    return (uv - fx.rect.xy) / fx.rect.zw;
}
// One pixel in uv.
fn pixel() -> vec2<f32> {
    return fx.size.zw;
}
// Width over height of the view.
fn aspect() -> f32 {
    return (fx.size.x * fx.rect.z) / (fx.size.y * fx.rect.w);
}
fn post_time() -> f32 {
    return fx.info.x;
}
fn pass_index() -> u32 {
    return u32(fx.info.y);
}
fn frame_index() -> u32 {
    return u32(fx.info.w);
}
fn luma(c: vec3<f32>) -> f32 {
    return dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
}
// Linear light to the gamma the screen shows, and back (for effects judged by how they look).
fn to_gamma(c: vec3<f32>) -> vec3<f32> {
    return pow(max(c, vec3<f32>(0.0)), vec3<f32>(1.0 / 2.2));
}
fn to_linear(c: vec3<f32>) -> vec3<f32> {
    return pow(max(c, vec3<f32>(0.0)), vec3<f32>(2.2));
}
// A hash of a point: white noise in 0..1.
fn hash21(p: vec2<f32>) -> f32 {
    // An integer hash of the float's bits: the usual fract-of-sine style hashes streak at large p.
    var h = bitcast<u32>(p.x) * 747796405u + bitcast<u32>(p.y) * 2891336453u + 2891336453u;
    h = ((h >> ((h >> 28u) + 4u)) ^ h) * 277803737u;
    h = (h >> 22u) ^ h;
    return f32(h >> 8u) / 16777216.0;
}
"#;

/// Copies the picture on (a pass that does nothing).
const BLIT: &str = "fn effect(uv: vec2<f32>, color: vec4<f32>) -> vec4<f32> { return color; }";

/// Linear distance and normals from the world pass's depth, for one view's part of the target.
const GEO: &str = r#"
struct GeoView {
    inv: mat4x4<f32>,
    // Position, then 1 when orthographic.
    cam: vec4<f32>,
    fwd: vec4<f32>,
    // The view's rectangle in pixels.
    rect: vec4<f32>,
};
@group(0) @binding(0) var<uniform> v: GeoView;
@group(0) @binding(1) var depth_in: DEPTH_TYPE;

fn raw(p: vec2<i32>) -> f32 {
    let dims = vec2<i32>(textureDimensions(depth_in));
    return textureLoad(depth_in, clamp(p, vec2<i32>(0), dims - vec2<i32>(1)) SAMPLE);
}

fn world(p: vec2<i32>) -> vec3<f32> {
    let ndc = vec2<f32>(
        (f32(p.x) + 0.5 - v.rect.x) / v.rect.z * 2.0 - 1.0,
        1.0 - (f32(p.y) + 0.5 - v.rect.y) / v.rect.w * 2.0,
    );
    let h = v.inv * vec4<f32>(ndc, raw(p), 1.0);
    return h.xyz / h.w;
}

fn along(a: vec3<f32>) -> f32 {
    return dot(a, v.fwd.xyz);
}

struct GeoOut {
    @location(0) depth: f32,
    @location(1) normal: vec4<f32>,
};

@vertex
fn vs(@builtin(vertex_index) vi: u32) -> @builtin(position) vec4<f32> {
    let p = vec2<f32>(f32((vi << 1u) & 2u), f32(vi & 2u));
    return vec4<f32>(p * 2.0 - 1.0, 0.0, 1.0);
}

@fragment
fn fs(@builtin(position) pos: vec4<f32>) -> GeoOut {
    let p = vec2<i32>(pos.xy);
    let c = world(p);
    var o: GeoOut;
    o.depth = along(c - v.cam.xyz);
    if (raw(p) >= 1.0) {
        o.normal = vec4<f32>(0.0);
        return o;
    }
    // Along each axis, the neighbour on the nearer side: silhouettes don't bend the normal.
    let r = world(p + vec2<i32>(1, 0));
    let l = world(p - vec2<i32>(1, 0));
    let d = world(p + vec2<i32>(0, 1));
    let u = world(p - vec2<i32>(0, 1));
    var dx = c - l;
    if (abs(along(r - c)) < abs(along(c - l))) { dx = r - c; }
    var dy = c - u;
    if (abs(along(d - c)) < abs(along(c - u))) { dy = d - c; }
    var n = normalize(cross(dx, dy));
    var to_cam = v.cam.xyz - c;
    if (v.cam.w > 0.5) { to_cam = -v.fwd.xyz; }
    if (dot(n, to_cam) < 0.0) { n = -n; }
    o.normal = vec4<f32>(n, 1.0);
    return o;
}
"#;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct FxUniform {
    p: [[f32; 4]; POST_PARAMS],
    rect: [f32; 4],
    size: [f32; 4],
    info: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct GeoUniform {
    inv: [[f32; 4]; 4],
    cam: [f32; 4],
    fwd: [f32; 4],
    rect: [f32; 4],
}

/// Uniform slots are this far apart (the usual minimum for a dynamic offset).
const STRIDE: u64 = 256;

const GEO_DEPTH: wgpu::TextureFormat = wgpu::TextureFormat::R32Float;
const GEO_NORMAL: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

/// The two textures the picture ping-pongs between.
struct Ping {
    textures: [wgpu::Texture; 2],
    views: [wgpu::TextureView; 2],
}

/// What a frame's passes are run over.
pub(crate) struct PostFrame<'a> {
    pub views: &'a [WorldView],
    /// Where the result goes, and its format and size.
    pub target: &'a wgpu::TextureView,
    pub format: wgpu::TextureFormat,
    pub size: (u32, u32),
    /// The world pass's depth buffer (kept for effects that read it), and whether it has
    /// MSAA samples.
    pub depth: &'a wgpu::TextureView,
    pub multisampled: bool,
    pub time: f32,
    pub frame: u32,
}

struct Geo {
    size: (u32, u32),
    depth: wgpu::TextureView,
    normal: wgpu::TextureView,
}

pub(crate) struct PostChain {
    layout: wgpu::BindGroupLayout,
    pipeline_layout: wgpu::PipelineLayout,
    sampler: wgpu::Sampler,
    /// By shader and target format; `None` for one that didn't compile.
    pipelines: HashMap<(u64, wgpu::TextureFormat), Option<wgpu::RenderPipeline>>,
    blit: Arc<PostShader>,
    pings: HashMap<((u32, u32), wgpu::TextureFormat), Ping>,
    geo: Option<Geo>,
    geo_layouts: [wgpu::BindGroupLayout; 2],
    geo_pipelines: [Option<wgpu::RenderPipeline>; 2],
    uniforms: wgpu::Buffer,
    geo_uniforms: wgpu::Buffer,
    /// 1x1 stand-ins for the depth and normals of an effect that doesn't read them.
    empty: (wgpu::TextureView, wgpu::TextureView),
}

fn buffer(gpu: &Gpu, slots: u64) -> wgpu::Buffer {
    gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("post uniforms"),
        size: slots.max(4) * STRIDE,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

fn texture(
    gpu: &Gpu,
    label: &str,
    size: (u32, u32),
    format: wgpu::TextureFormat,
    copy: bool,
) -> wgpu::Texture {
    let mut usage = wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING;
    if copy {
        usage |= wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::COPY_DST;
    }
    gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width: size.0.max(1),
            height: size.1.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage,
        view_formats: &[],
    })
}

fn float_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    }
}

fn dynamic_uniform(binding: u32, size: u64) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: true,
            min_binding_size: wgpu::BufferSize::new(size),
        },
        count: None,
    }
}

impl PostChain {
    pub fn new(gpu: &Gpu) -> Self {
        let device = &gpu.device;
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("post"),
            entries: &[
                dynamic_uniform(0, std::mem::size_of::<FxUniform>() as u64),
                float_entry(1),
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                // Depth is R32Float: not filterable, read with `textureLoad`.
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("post"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("post"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let geo_layout = |multisampled: bool| {
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("post geometry"),
                entries: &[
                    dynamic_uniform(0, std::mem::size_of::<GeoUniform>() as u64),
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Depth,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled,
                        },
                        count: None,
                    },
                ],
            })
        };
        let empty = |format| {
            texture(gpu, "post empty", (1, 1), format, false).create_view(&Default::default())
        };
        Self {
            layout,
            pipeline_layout,
            sampler,
            pipelines: HashMap::new(),
            blit: Arc::new(PostShader::new("blit", BLIT, Vec::new(), 1, false)),
            pings: HashMap::new(),
            geo: None,
            geo_layouts: [geo_layout(false), geo_layout(true)],
            geo_pipelines: [None, None],
            uniforms: buffer(gpu, 16),
            geo_uniforms: buffer(gpu, 4),
            empty: (empty(GEO_DEPTH), empty(GEO_NORMAL)),
        }
    }

    fn ping(&mut self, gpu: &Gpu, size: (u32, u32), format: wgpu::TextureFormat) -> &Ping {
        if !self.pings.contains_key(&(size, format)) {
            if self.pings.len() >= 4 {
                self.pings.clear();
            }
            let textures = [
                texture(gpu, "post picture", size, format, true),
                texture(gpu, "post picture", size, format, true),
            ];
            let views = [
                textures[0].create_view(&Default::default()),
                textures[1].create_view(&Default::default()),
            ];
            self.pings.insert((size, format), Ping { textures, views });
        }
        &self.pings[&(size, format)]
    }

    /// The texture the finished picture goes into before the passes run.
    pub fn input(
        &mut self,
        gpu: &Gpu,
        size: (u32, u32),
        format: wgpu::TextureFormat,
    ) -> wgpu::TextureView {
        self.ping(gpu, size, format).views[0].clone()
    }

    fn pipeline(
        &mut self,
        gpu: &Gpu,
        shader: &PostShader,
        format: wgpu::TextureFormat,
    ) -> Option<&wgpu::RenderPipeline> {
        let key = (shader.id, format);
        if !self.pipelines.contains_key(&key) {
            let made = self.compile(gpu, shader, format);
            self.pipelines.insert(key, made);
        }
        self.pipelines[&key].as_ref()
    }

    /// Builds the pipeline for a shader; errors (the game's WGSL) go to the log, not a panic.
    fn compile(
        &self,
        gpu: &Gpu,
        shader: &PostShader,
        format: wgpu::TextureFormat,
    ) -> Option<wgpu::RenderPipeline> {
        let source = assemble(shader);
        let scope = gpu.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let module = gpu
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(&shader.name),
                source: wgpu::ShaderSource::Wgsl(source.into()),
            });
        let pipeline = gpu
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(&shader.name),
                layout: Some(&self.pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: Some("vs"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                primitive: Default::default(),
                depth_stencil: None,
                multisample: Default::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &module,
                    entry_point: Some("fs"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            });
        match pollster::block_on(scope.pop()) {
            Some(error) => {
                tracing::error!(shader = %shader.name, "post-process shader skipped: {error}");
                None
            }
            None => Some(pipeline),
        }
    }

    /// Resolves the world pass's depth into linear distance and normals for the views that
    /// have an effect reading them.
    fn resolve_geometry(
        &mut self,
        gpu: &Gpu,
        encoder: &mut wgpu::CommandEncoder,
        frame: &PostFrame<'_>,
    ) {
        let PostFrame {
            views,
            size,
            depth,
            multisampled,
            ..
        } = *frame;
        let m = usize::from(multisampled);
        if self.geo_pipelines[m].is_none() {
            let source = GEO
                .replace(
                    "DEPTH_TYPE",
                    if multisampled {
                        "texture_depth_multisampled_2d"
                    } else {
                        "texture_depth_2d"
                    },
                )
                // The mip level, or sample, to read.
                .replace("SAMPLE", ", 0");
            let module = gpu
                .device
                .create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some("post geometry"),
                    source: wgpu::ShaderSource::Wgsl(source.into()),
                });
            let layout = gpu
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("post geometry"),
                    bind_group_layouts: &[Some(&self.geo_layouts[m])],
                    immediate_size: 0,
                });
            let target = |format| {
                Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })
            };
            self.geo_pipelines[m] = Some(gpu.device.create_render_pipeline(
                &wgpu::RenderPipelineDescriptor {
                    label: Some("post geometry"),
                    layout: Some(&layout),
                    vertex: wgpu::VertexState {
                        module: &module,
                        entry_point: Some("vs"),
                        compilation_options: Default::default(),
                        buffers: &[],
                    },
                    primitive: Default::default(),
                    depth_stencil: None,
                    multisample: Default::default(),
                    fragment: Some(wgpu::FragmentState {
                        module: &module,
                        entry_point: Some("fs"),
                        compilation_options: Default::default(),
                        targets: &[target(GEO_DEPTH), target(GEO_NORMAL)],
                    }),
                    multiview_mask: None,
                    cache: None,
                },
            ));
        }
        if self.geo.as_ref().is_none_or(|g| g.size != size) {
            self.geo = Some(Geo {
                size,
                depth: texture(gpu, "post depth", size, GEO_DEPTH, false)
                    .create_view(&Default::default()),
                normal: texture(gpu, "post normals", size, GEO_NORMAL, false)
                    .create_view(&Default::default()),
            });
        }
        let wanted: Vec<(usize, [u32; 4])> = views
            .iter()
            .enumerate()
            .filter(|(_, v)| v.post.iter().any(|e| e.shader.depth))
            .filter_map(|(i, v)| Some((i, viewport_px(v.viewport, size)?)))
            .collect();
        if (self.geo_uniforms.size() / STRIDE) < wanted.len() as u64 {
            self.geo_uniforms = buffer(gpu, wanted.len().next_power_of_two() as u64);
        }
        let geo = self.geo.as_ref().expect("made above");
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("post geometry"),
            color_attachments: &[
                Some(wgpu::RenderPassColorAttachment {
                    view: &geo.depth,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        // The far side of the world where no view draws.
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 1.0e6,
                            g: 0.0,
                            b: 0.0,
                            a: 0.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                }),
                Some(wgpu::RenderPassColorAttachment {
                    view: &geo.normal,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                }),
            ],
            ..Default::default()
        });
        pass.set_pipeline(self.geo_pipelines[m].as_ref().expect("made above"));
        for (slot, (i, rect)) in wanted.iter().enumerate() {
            let cam = &views[*i].camera;
            let uniform = GeoUniform {
                inv: cam.view_proj.inverse().to_cols_array_2d(),
                cam: [
                    cam.position.x,
                    cam.position.y,
                    cam.position.z,
                    f32::from(u8::from(cam.orthographic)),
                ],
                fwd: [cam.forward.x, cam.forward.y, cam.forward.z, 0.0],
                rect: rect.map(|v| v as f32),
            };
            gpu.queue.write_buffer(
                &self.geo_uniforms,
                slot as u64 * STRIDE,
                bytemuck::bytes_of(&uniform),
            );
            let group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("post geometry"),
                layout: &self.geo_layouts[m],
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                            buffer: &self.geo_uniforms,
                            offset: 0,
                            size: wgpu::BufferSize::new(std::mem::size_of::<GeoUniform>() as u64),
                        }),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(depth),
                    },
                ],
            });
            pass.set_bind_group(0, &group, &[(slot as u64 * STRIDE) as u32]);
            pass.set_viewport(
                rect[0] as f32,
                rect[1] as f32,
                rect[2] as f32,
                rect[3] as f32,
                0.0,
                1.0,
            );
            pass.set_scissor_rect(rect[0], rect[1], rect[2], rect[3]);
            pass.draw(0..3, 0..1);
        }
    }

    /// Runs every view's effects over the picture in [`input`](Self::input)'s texture and leaves
    /// the result in the frame's target.
    pub fn encode(&mut self, gpu: &Gpu, encoder: &mut wgpu::CommandEncoder, frame: &PostFrame<'_>) {
        let PostFrame {
            views,
            target,
            format,
            size,
            time,
            frame: frame_index,
            ..
        } = *frame;
        let blit = self.blit.clone();
        // The passes in order: each view's effects, each run `passes` times; shaders that
        // didn't compile are left out.
        struct Step<'a> {
            effect: &'a PostEffect,
            rect: [u32; 4],
            pass: u32,
        }
        let mut steps: Vec<Step<'_>> = Vec::new();
        for view in views {
            let Some(rect) = viewport_px(view.viewport, size) else {
                continue;
            };
            for effect in &view.post {
                if self.pipeline(gpu, &effect.shader, format).is_none() {
                    continue;
                }
                for pass in 0..effect.shader.passes {
                    steps.push(Step { effect, rect, pass });
                }
            }
        }
        let blit_effect = PostEffect::new(blit.clone());
        if self.pipeline(gpu, &blit, format).is_none() {
            return;
        }
        let full = [0, 0, size.0, size.1];
        if views.iter().any(|v| v.post.iter().any(|e| e.shader.depth)) {
            self.resolve_geometry(gpu, encoder, frame);
        }
        // A last pass over the whole target draws into it; otherwise a copy follows.
        let drawn_to_target = steps.last().is_some_and(|s| s.rect == full);
        let total = steps.len() + usize::from(!drawn_to_target);
        if (self.uniforms.size() / STRIDE) < total as u64 {
            self.uniforms = buffer(gpu, (total as u64).next_power_of_two());
        }
        let (width, height) = (size.0 as f32, size.1 as f32);
        // The picture's two textures; `cur` holds it now.
        self.ping(gpu, size, format);
        let ping = &self.pings[&(size, format)];
        let mut cur = 0usize;
        let geo = self.geo.as_ref();
        for slot in 0..total {
            let (effect, rect, pass, count) = match steps.get(slot) {
                Some(s) => (s.effect, s.rect, s.pass, s.effect.shader.passes),
                None => (&blit_effect, full, 0, 1),
            };
            let covers = rect == full;
            let into_target = slot + 1 == total;
            let uniform = FxUniform {
                p: effect.params,
                rect: [
                    rect[0] as f32 / width,
                    rect[1] as f32 / height,
                    rect[2] as f32 / width,
                    rect[3] as f32 / height,
                ],
                size: [width, height, 1.0 / width, 1.0 / height],
                info: [time, pass as f32, count as f32, frame_index as f32],
            };
            gpu.queue.write_buffer(
                &self.uniforms,
                slot as u64 * STRIDE,
                bytemuck::bytes_of(&uniform),
            );
            let (depth_view, normal_view) = match geo {
                Some(g) if effect.shader.depth => (&g.depth, &g.normal),
                _ => (&self.empty.0, &self.empty.1),
            };
            let group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("post"),
                layout: &self.layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                            buffer: &self.uniforms,
                            offset: 0,
                            size: wgpu::BufferSize::new(std::mem::size_of::<FxUniform>() as u64),
                        }),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&ping.views[cur]),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(depth_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: wgpu::BindingResource::TextureView(normal_view),
                    },
                ],
            });
            let Some(pipeline) = self
                .pipelines
                .get(&(effect.shader.id, format))
                .and_then(Option::as_ref)
            else {
                continue;
            };
            let to = if into_target {
                target
            } else {
                // Outside the view's part the next picture is the last one.
                if !covers {
                    encoder.copy_texture_to_texture(
                        ping.textures[cur].as_image_copy(),
                        ping.textures[1 - cur].as_image_copy(),
                        ping.textures[cur].size(),
                    );
                }
                &ping.views[1 - cur]
            };
            let mut render = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("post"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: to,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: if covers {
                            wgpu::LoadOp::Clear(wgpu::Color::BLACK)
                        } else {
                            wgpu::LoadOp::Load
                        },
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            render.set_pipeline(pipeline);
            render.set_bind_group(0, &group, &[(slot as u64 * STRIDE) as u32]);
            render.set_scissor_rect(rect[0], rect[1], rect[2], rect[3]);
            render.draw(0..3, 0..1);
            drop(render);
            cur = 1 - cur;
        }
    }
}

/// The whole WGSL for a shader: the prelude, its parameters, its code and the entry point.
fn assemble(shader: &PostShader) -> String {
    let mut s = String::from(PRELUDE);
    for (name, ty) in &shader.params {
        let _ = writeln!(s, "var<private> {name}: {};", ty.wgsl());
    }
    s.push_str(&shader.source);
    s.push_str("\n@fragment\nfn fs(in: Out) -> @location(0) vec4<f32> {\n");
    for (i, (name, ty)) in shader.params.iter().enumerate().take(POST_PARAMS) {
        let _ = writeln!(s, "    {name} = fx.p[{i}]{};", ty.swizzle());
    }
    s.push_str("    return effect(in.uv, scene(in.uv));\n}\n");
    s
}
