//! Post-processing for the world pass: with bloom, exposure or tonemapping on, the world
//! draws into an HDR texture ([`HDR_FORMAT`]), whose bright parts are blurred down and up
//! a chain of half-size textures and added back as it's copied to the target, with
//! exposure and tonemapping applied on the way.

use std::collections::HashMap;

use bytemuck::{Pod, Zeroable};

use crate::Gpu;

/// The world's colour format while bloom is on: room for light brighter than white.
pub const HDR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

/// Unity's Bloom settings.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bloom {
    /// Linear brightness where blooming starts (1: only what's brighter than white).
    pub threshold: f32,
    /// How strongly the glow is added back.
    pub intensity: f32,
    /// 0..1: how far the glow spreads.
    pub scatter: f32,
}

impl Default for Bloom {
    fn default() -> Self {
        Self {
            threshold: 0.9,
            intensity: 1.0,
            scatter: 0.7,
        }
    }
}

/// How HDR light becomes screen colours (Unity's `TonemappingMode`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tonemapping {
    /// Clipped at white.
    #[default]
    None,
    /// Colours kept as they are until a soft shoulder (Khronos PBR Neutral).
    Neutral,
    /// Filmic: more contrast, highlights roll off (ACES).
    Aces,
}

/// Exposure and tonemapping, applied after bloom.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Grading {
    /// Multiplies the light (2 to the post exposure's stops).
    pub exposure: f32,
    pub tonemapping: Tonemapping,
}

impl Default for Grading {
    fn default() -> Self {
        Self {
            exposure: 1.0,
            tonemapping: Tonemapping::None,
        }
    }
}

impl Grading {
    /// Changes nothing, so no post pass is needed for it.
    pub fn is_identity(&self) -> bool {
        self.tonemapping == Tonemapping::None && (self.exposure - 1.0).abs() < 1e-6
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Params {
    threshold: f32,
    knee: f32,
    intensity: f32,
    exposure: f32,
    tonemap: u32,
    _pad: [u32; 3],
}

/// The most half-size levels the blur goes down.
const MAX_LEVELS: usize = 7;

pub(crate) struct BloomPass {
    module: wgpu::ShaderModule,
    group_layout: wgpu::BindGroupLayout,
    layout: wgpu::PipelineLayout,
    sampler: wgpu::Sampler,
    params: wgpu::Buffer,
    prefilter: wgpu::RenderPipeline,
    down: wgpu::RenderPipeline,
    up: wgpu::RenderPipeline,
    composite: HashMap<wgpu::TextureFormat, wgpu::RenderPipeline>,
    /// Exposure and tonemapping alone, by target format.
    grade: HashMap<wgpu::TextureFormat, wgpu::RenderPipeline>,
    /// The HDR image and the blur levels, by target size (the screen and render
    /// textures).
    textures: HashMap<(u32, u32), (wgpu::TextureView, Vec<wgpu::TextureView>)>,
}

fn pipeline(
    gpu: &Gpu,
    module: &wgpu::ShaderModule,
    layout: &wgpu::PipelineLayout,
    fragment: &str,
    format: wgpu::TextureFormat,
    blend: Option<wgpu::BlendState>,
) -> wgpu::RenderPipeline {
    gpu.device
        .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("bloom"),
            layout: Some(layout),
            vertex: wgpu::VertexState {
                module,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module,
                entry_point: Some(fragment),
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
}

impl BloomPass {
    pub fn new(gpu: &Gpu) -> Self {
        let device = &gpu.device;
        let module = device.create_shader_module(wgpu::include_wgsl!("bloom.wgsl"));
        let texture = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        };
        let group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bloom"),
            entries: &[
                texture(0),
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                texture(3),
            ],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("bloom"),
            bind_group_layouts: &[Some(&group_layout)],
            immediate_size: 0,
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("bloom"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let params = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("bloom"),
            size: std::mem::size_of::<Params>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        // Upsampling mixes the blurrier level into the sharper one by `scatter`
        // (the blend constant).
        let mix = wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::Constant,
                dst_factor: wgpu::BlendFactor::OneMinusConstant,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent::REPLACE,
        };
        Self {
            prefilter: pipeline(gpu, &module, &layout, "fs_prefilter", HDR_FORMAT, None),
            down: pipeline(gpu, &module, &layout, "fs_down", HDR_FORMAT, None),
            up: pipeline(gpu, &module, &layout, "fs_up", HDR_FORMAT, Some(mix)),
            composite: HashMap::new(),
            grade: HashMap::new(),
            module,
            group_layout,
            layout,
            sampler,
            params,
            textures: HashMap::new(),
        }
    }

    /// The HDR texture to draw the world into, made (or remade) for `size`.
    pub fn hdr_view(&mut self, gpu: &Gpu, size: (u32, u32)) -> wgpu::TextureView {
        if !self.textures.contains_key(&size) {
            // A few sizes at most; a resized window leaves old ones behind.
            if self.textures.len() >= 4 {
                self.textures.clear();
            }
            let make = |w: u32, h: u32| {
                gpu.device
                    .create_texture(&wgpu::TextureDescriptor {
                        label: Some("bloom"),
                        size: wgpu::Extent3d {
                            width: w.max(1),
                            height: h.max(1),
                            depth_or_array_layers: 1,
                        },
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: wgpu::TextureDimension::D2,
                        format: HDR_FORMAT,
                        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                            | wgpu::TextureUsages::TEXTURE_BINDING,
                        view_formats: &[],
                    })
                    .create_view(&Default::default())
            };
            let mut levels = Vec::new();
            let (mut w, mut h) = size;
            while levels.len() < MAX_LEVELS && w > 2 && h > 2 {
                w /= 2;
                h /= 2;
                levels.push(make(w, h));
            }
            self.textures.insert(size, (make(size.0, size.1), levels));
        }
        self.textures[&size].0.clone()
    }

    /// Blurs the HDR image's bright parts (with `settings`) and writes the image plus the
    /// glow to `target`, graded.
    pub fn encode(
        &mut self,
        gpu: &Gpu,
        encoder: &mut wgpu::CommandEncoder,
        // Where the result goes (its size picks the HDR image).
        out: &crate::RenderTarget<'_>,
        settings: Option<&Bloom>,
        grading: Grading,
    ) {
        let (target, format, size) = (out.view, out.format, out.size);
        let Some((hdr, levels)) = self.textures.get(&size) else {
            return;
        };
        let bloom = settings.copied().unwrap_or(Bloom {
            intensity: 0.0,
            ..Default::default()
        });
        let knee = (bloom.threshold * 0.5).max(1e-3);
        gpu.queue.write_buffer(
            &self.params,
            0,
            bytemuck::bytes_of(&Params {
                threshold: bloom.threshold,
                knee,
                intensity: bloom.intensity,
                exposure: grading.exposure.max(0.0),
                tonemap: match grading.tonemapping {
                    Tonemapping::None => 0,
                    Tonemapping::Neutral => 1,
                    Tonemapping::Aces => 2,
                },
                _pad: [0; 3],
            }),
        );
        let (module, layout) = (&self.module, &self.layout);
        let composite = self
            .composite
            .entry(format)
            .or_insert_with(|| pipeline(gpu, module, layout, "fs_composite", format, None));
        let grade = self
            .grade
            .entry(format)
            .or_insert_with(|| pipeline(gpu, module, layout, "fs_grade", format, None));
        let group = |src: &wgpu::TextureView| {
            gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("bloom"),
                layout: &self.group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(src),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: self.params.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(hdr),
                    },
                ],
            })
        };
        let draw = |encoder: &mut wgpu::CommandEncoder,
                    out: &wgpu::TextureView,
                    src: &wgpu::TextureView,
                    pipeline: &wgpu::RenderPipeline,
                    load: wgpu::LoadOp<wgpu::Color>,
                    constant: f64| {
            let bind = group(src);
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("bloom"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: out,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, &bind, &[]);
            pass.set_blend_constant(wgpu::Color {
                r: constant,
                g: constant,
                b: constant,
                a: constant,
            });
            pass.draw(0..3, 0..1);
        };
        let clear = wgpu::LoadOp::Clear(wgpu::Color::BLACK);
        if settings.is_none() {
            draw(encoder, target, hdr, grade, clear, 0.0);
            return;
        }
        if levels.is_empty() {
            draw(encoder, target, hdr, composite, clear, 0.0);
            return;
        }
        draw(encoder, &levels[0], hdr, &self.prefilter, clear, 0.0);
        for i in 1..levels.len() {
            draw(encoder, &levels[i], &levels[i - 1], &self.down, clear, 0.0);
        }
        let scatter = f64::from(bloom.scatter.clamp(0.0, 1.0));
        for i in (0..levels.len() - 1).rev() {
            draw(
                encoder,
                &levels[i],
                &levels[i + 1],
                &self.up,
                wgpu::LoadOp::Load,
                scatter,
            );
        }
        draw(encoder, target, &levels[0], composite, clear, 0.0);
    }
}
