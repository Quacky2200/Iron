//! The 3D world pass (M5.a): what a camera sees, drawn before the screen UI.
//!
//! For now the world is quads: coloured or textured planes, such as UI panels in the
//! world (a texture from [`crate::UiRenderer::render_to_texture`]). Meshes and lights
//! come with M5.a2. Quads are drawn back to front with depth testing, so see-through
//! ones blend over what's behind them.

use std::collections::HashMap;

use bytemuck::{Pod, Zeroable};
use iron_core::glam::{Mat4, Vec3, Vec4Swizzles};

use crate::{Color, Gpu, RenderTarget};

const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

/// What the camera sees from where: `view_proj` maps world space to clip space.
#[derive(Clone, Copy, Debug)]
pub struct WorldCamera {
    pub view_proj: Mat4,
    /// The camera's position, for sorting see-through quads.
    pub position: Vec3,
    pub clear: Color,
}

impl WorldCamera {
    /// A perspective camera at `world` (its transform: looking down its -z), with a
    /// vertical field of view in degrees, for a `width`×`height` target.
    pub fn perspective(
        world: Mat4,
        fov_y_degrees: f32,
        near: f32,
        far: f32,
        size: (u32, u32),
        clear: Color,
    ) -> Self {
        let aspect = size.0.max(1) as f32 / size.1.max(1) as f32;
        let proj = iron_core::glam::camera::rh::proj::directx::perspective(
            fov_y_degrees.clamp(1.0, 179.0).to_radians(),
            aspect,
            near.max(1e-4),
            far.max(near + 1e-3),
        );
        Self {
            view_proj: proj * world.inverse(),
            position: world.w_axis.xyz(),
            clear,
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

pub struct WorldRenderer {
    layout: wgpu::PipelineLayout,
    module: wgpu::ShaderModule,
    pipelines: HashMap<wgpu::TextureFormat, wgpu::RenderPipeline>,
    globals_layout: wgpu::BindGroupLayout,
    texture_layout: wgpu::BindGroupLayout,
    globals: wgpu::Buffer,
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
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
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
        let globals = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("world globals"),
            size: 64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
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
            quads,
            sampler,
            white: white.create_view(&Default::default()),
            depth: None,
        }
    }

    fn quad_buffer(gpu: &Gpu, count: usize) -> wgpu::Buffer {
        gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("world quads"),
            size: (count * std::mem::size_of::<GpuQuad>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    fn pipeline(&mut self, gpu: &Gpu, format: wgpu::TextureFormat) -> wgpu::RenderPipeline {
        self.pipelines
            .entry(format)
            .or_insert_with(|| {
                gpu.device
                    .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                        label: Some("world quads"),
                        layout: Some(&self.layout),
                        vertex: wgpu::VertexState {
                            module: &self.module,
                            entry_point: Some("vs_quad"),
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
                            depth_compare: Some(wgpu::CompareFunction::Less),
                            stencil: Default::default(),
                            bias: Default::default(),
                        }),
                        multisample: Default::default(),
                        fragment: Some(wgpu::FragmentState {
                            module: &self.module,
                            entry_point: Some("fs_quad"),
                            compilation_options: Default::default(),
                            targets: &[Some(wgpu::ColorTargetState {
                                format,
                                blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                                write_mask: wgpu::ColorWrites::ALL,
                            })],
                        }),
                        multiview_mask: None,
                        cache: None,
                    })
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

    /// Clears `target` to the camera's colour and draws `quads` into it. `textures` are
    /// the textures quads refer to by index.
    pub fn render(
        &mut self,
        gpu: &Gpu,
        target: &RenderTarget<'_>,
        camera: &WorldCamera,
        quads: &[WorldQuad],
        textures: &[&wgpu::TextureView],
    ) {
        let (view, format, size) = (target.view, target.format, target.size);
        if size.0 == 0 || size.1 == 0 {
            return;
        }
        let pipeline = self.pipeline(gpu, format);
        let depth = self.depth_view(gpu, size);

        // Back to front by distance from the camera, so see-through quads blend over
        // what's behind them; then grouped into runs that share a texture.
        let mut order: Vec<&WorldQuad> = quads.iter().collect();
        let distance = |q: &WorldQuad| q.model.w_axis.xyz().distance_squared(camera.position);
        order.sort_by(|a, b| distance(b).total_cmp(&distance(a)));
        let data: Vec<GpuQuad> = order
            .iter()
            .map(|q| {
                let [r, g, b, a] = q.color;
                GpuQuad {
                    model: q.model.to_cols_array_2d(),
                    color: [r * a, g * a, b * a, a],
                    uv: q.uv,
                }
            })
            .collect();
        if self.quads.size() < (data.len().max(1) * std::mem::size_of::<GpuQuad>()) as u64 {
            self.quads = Self::quad_buffer(gpu, data.len().next_power_of_two());
        }
        gpu.queue.write_buffer(
            &self.globals,
            0,
            bytemuck::cast_slice(&camera.view_proj.to_cols_array()),
        );
        if !data.is_empty() {
            gpu.queue
                .write_buffer(&self.quads, 0, bytemuck::cast_slice(&data));
        }
        let globals = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("world globals"),
            layout: &self.globals_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.globals.as_entire_binding(),
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
                    view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(camera.clear.into()),
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
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &globals, &[]);
            let mut start = 0;
            while start < order.len() {
                let texture = order[start].texture;
                let mut end = start + 1;
                while end < order.len() && order[end].texture == texture {
                    end += 1;
                }
                let group = texture.map_or(0, |t| (t + 1).min(groups.len() - 1));
                pass.set_bind_group(1, &groups[group], &[]);
                pass.draw(0..6, start as u32..end as u32);
                start = end;
            }
        }
        gpu.queue.submit([encoder.finish()]);
    }
}
