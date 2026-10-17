//! The 3D world pass (M5.a): what the cameras see, drawn before the screen UI.
//!
//! For now the world is quads: coloured or textured planes, such as UI panels in the
//! world. Meshes and lights come with M5.a2. Each camera is a [`WorldView`] with its
//! own viewport; views are drawn in order, each first filling its viewport with its
//! background (which also resets depth there), so later cameras draw over earlier
//! ones, like Unity's camera depth. Quads are drawn back to front with depth testing,
//! so see-through ones blend over what's behind them.

use std::collections::HashMap;
use std::ops::Range;

use bytemuck::{Pod, Zeroable};
use iron_core::glam::{Mat4, Vec3, Vec4Swizzles};

use crate::{Color, Gpu, RenderTarget};

const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

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
    /// The camera's position, for sorting see-through quads.
    pub position: Vec3,
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

/// One camera's view: where on the target it draws and what it sees.
#[derive(Clone, Debug)]
pub struct WorldView {
    pub camera: WorldCamera,
    /// The part of the target it covers: x, y, width, height from the top-left, 0 to 1.
    pub viewport: [f32; 4],
    pub quads: Vec<WorldQuad>,
}

/// One plane in the world.
#[derive(Clone, Copy, Debug)]
pub struct WorldQuad {
    /// Places the unit quad (-0.5..0.5 in x and y, facing +z) in the world.
    pub model: Mat4,
    /// Linear RGBA, straight alpha (multiplies the texture).
    pub color: [f32; 4],
    /// Index into the `textures` given to [`WorldRenderer::render`]; `None` is plain colour.
    pub texture: Option<usize>,
    /// The part of the texture shown: origin and size in UV space.
    pub uv: [f32; 4],
}

impl WorldQuad {
    pub fn colored(model: Mat4, color: [f32; 4]) -> Self {
        Self {
            model,
            color,
            texture: None,
            uv: [0.0, 0.0, 1.0, 1.0],
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct GpuQuad {
    model: [[f32; 4]; 4],
    color: [f32; 4],
    uv: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct GpuGlobals {
    view_proj: [[f32; 4]; 4],
    background: [f32; 4],
}

pub struct WorldRenderer {
    layout: wgpu::PipelineLayout,
    module: wgpu::ShaderModule,
    /// Quad and background-fill pipelines by target format.
    pipelines: HashMap<wgpu::TextureFormat, (wgpu::RenderPipeline, wgpu::RenderPipeline)>,
    globals_layout: wgpu::BindGroupLayout,
    texture_layout: wgpu::BindGroupLayout,
    /// One `GpuGlobals` per view, `stride` bytes apart (a dynamic offset picks one).
    globals: wgpu::Buffer,
    stride: u64,
    quads: wgpu::Buffer,
    sampler: wgpu::Sampler,
    white: wgpu::TextureView,
    depth: Option<(wgpu::TextureView, (u32, u32))>,
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
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
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
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("world"),
            bind_group_layouts: &[Some(&globals_layout), Some(&texture_layout)],
            immediate_size: 0,
        });
        let align = u64::from(device.limits().min_uniform_buffer_offset_alignment);
        let stride = (std::mem::size_of::<GpuGlobals>() as u64).div_ceil(align) * align;
        let globals = Self::globals_buffer(gpu, stride, 4);
        let quads = Self::quad_buffer(gpu, 64);
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
        Self {
            layout,
            module,
            pipelines: HashMap::new(),
            globals_layout,
            texture_layout,
            globals,
            stride,
            quads,
            sampler,
            white: white.create_view(&Default::default()),
            depth: None,
        }
    }

    fn globals_buffer(gpu: &Gpu, stride: u64, views: usize) -> wgpu::Buffer {
        gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("world globals"),
            size: stride * views as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    fn quad_buffer(gpu: &Gpu, count: usize) -> wgpu::Buffer {
        gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("world quads"),
            size: (count * std::mem::size_of::<GpuQuad>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    fn pipelines(
        &mut self,
        gpu: &Gpu,
        format: wgpu::TextureFormat,
    ) -> (wgpu::RenderPipeline, wgpu::RenderPipeline) {
        let (layout, module) = (&self.layout, &self.module);
        let make = |label: &str,
                    vs: &str,
                    fs: &str,
                    depth_compare: wgpu::CompareFunction,
                    blend: Option<wgpu::BlendState>| {
            gpu.device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some(label),
                    layout: Some(layout),
                    vertex: wgpu::VertexState {
                        module,
                        entry_point: Some(vs),
                        compilation_options: Default::default(),
                        buffers: &[],
                    },
                    // Planes are seen from both sides.
                    primitive: wgpu::PrimitiveState {
                        cull_mode: None,
                        ..Default::default()
                    },
                    depth_stencil: Some(wgpu::DepthStencilState {
                        format: DEPTH_FORMAT,
                        depth_write_enabled: Some(true),
                        depth_compare: Some(depth_compare),
                        stencil: Default::default(),
                        bias: Default::default(),
                    }),
                    multisample: Default::default(),
                    fragment: Some(wgpu::FragmentState {
                        module,
                        entry_point: Some(fs),
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
        self.pipelines
            .entry(format)
            .or_insert_with(|| {
                (
                    make(
                        "world quads",
                        "vs_quad",
                        "fs_quad",
                        wgpu::CompareFunction::Less,
                        Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    ),
                    // The background replaces colour and resets depth in the viewport.
                    make(
                        "world background",
                        "vs_fill",
                        "fs_fill",
                        wgpu::CompareFunction::Always,
                        None,
                    ),
                )
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
        let view = WorldView {
            camera: *camera,
            viewport: [0.0, 0.0, 1.0, 1.0],
            quads: quads.to_vec(),
        };
        self.render_views(gpu, target, &[view], textures);
    }

    /// Draws each camera's view into its viewport, in order. `target` is cleared to
    /// black first; parts no view covers stay black. `textures` are the textures
    /// quads refer to by index.
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
        let (quad_pipeline, fill_pipeline) = self.pipelines(gpu, target.format);
        let depth = self.depth_view(gpu, size);

        // Every view's quads in one buffer: back to front by distance from that view's
        // camera (so see-through quads blend over what's behind them).
        let mut data: Vec<GpuQuad> = Vec::new();
        let mut ranges: Vec<(Range<u32>, Vec<Option<usize>>)> = Vec::new();
        for view in views {
            let mut order: Vec<&WorldQuad> = view.quads.iter().collect();
            let eye = view.camera.position;
            let distance = |q: &WorldQuad| q.model.w_axis.xyz().distance_squared(eye);
            order.sort_by(|a, b| distance(b).total_cmp(&distance(a)));
            let start = data.len() as u32;
            data.extend(order.iter().map(|q| {
                let [r, g, b, a] = q.color;
                GpuQuad {
                    model: q.model.to_cols_array_2d(),
                    color: [r * a, g * a, b * a, a],
                    uv: q.uv,
                }
            }));
            let textures = order.iter().map(|q| q.texture).collect();
            ranges.push((start..data.len() as u32, textures));
        }
        if self.quads.size() < (data.len().max(1) * std::mem::size_of::<GpuQuad>()) as u64 {
            self.quads = Self::quad_buffer(gpu, data.len().next_power_of_two());
        }
        if !data.is_empty() {
            gpu.queue
                .write_buffer(&self.quads, 0, bytemuck::cast_slice(&data));
        }
        if self.globals.size() < self.stride * views.len().max(1) as u64 {
            self.globals = Self::globals_buffer(gpu, self.stride, views.len().next_power_of_two());
        }
        for (i, view) in views.iter().enumerate() {
            let c = view.camera.clear;
            let globals = GpuGlobals {
                view_proj: view.camera.view_proj.to_cols_array_2d(),
                background: [
                    (c.r * c.a) as f32,
                    (c.g * c.a) as f32,
                    (c.b * c.a) as f32,
                    c.a as f32,
                ],
            };
            gpu.queue.write_buffer(
                &self.globals,
                self.stride * i as u64,
                bytemuck::bytes_of(&globals),
            );
        }
        let globals = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("world globals"),
            layout: &self.globals_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &self.globals,
                        offset: 0,
                        size: wgpu::BufferSize::new(std::mem::size_of::<GpuGlobals>() as u64),
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.quads.as_entire_binding(),
                },
            ],
        });
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

        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("world"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target.view,
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
            for (i, (view, (range, quad_textures))) in views.iter().zip(&ranges).enumerate() {
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
                pass.set_bind_group(0, &globals, &[(self.stride * i as u64) as u32]);
                pass.set_bind_group(1, &groups[0], &[]);
                pass.set_pipeline(&fill_pipeline);
                pass.draw(0..3, 0..1);

                pass.set_pipeline(&quad_pipeline);
                let mut start = 0;
                while start < quad_textures.len() {
                    let texture = quad_textures[start];
                    let mut end = start + 1;
                    while end < quad_textures.len() && quad_textures[end] == texture {
                        end += 1;
                    }
                    let group = texture.map_or(0, |t| (t + 1).min(groups.len() - 1));
                    pass.set_bind_group(1, &groups[group], &[]);
                    pass.draw(0..6, range.start + start as u32..range.start + end as u32);
                    start = end;
                }
            }
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
