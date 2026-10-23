// SPDX-License-Identifier: Apache-2.0

//! FXAA (fast approximate anti-aliasing, Timothy Lottes' reduced form): a post pass over
//! the finished, tonemapped image that blends along edges it finds by contrast. Cheaper
//! than MSAA and smooths what MSAA can't (texture and shading edges), a little blurrier.

use std::collections::HashMap;

use crate::Gpu;

const SHADER: &str = r#"
@group(0) @binding(0) var src: texture_2d<f32>;
@group(0) @binding(1) var samp: sampler;

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

// Perceived brightness, in gamma space (edges are judged as they look).
fn luma(c: vec3<f32>) -> f32 {
    return dot(sqrt(max(c, vec3<f32>(0.0))), vec3<f32>(0.299, 0.587, 0.114));
}

const REDUCE_MIN: f32 = 1.0 / 128.0;
const REDUCE_MUL: f32 = 1.0 / 8.0;
const SPAN_MAX: f32 = 8.0;

@fragment
fn fs(in: Out) -> @location(0) vec4<f32> {
    let px = 1.0 / vec2<f32>(textureDimensions(src));
    let m = textureSample(src, samp, in.uv);
    let nw = luma(textureSample(src, samp, in.uv + vec2<f32>(-1.0, -1.0) * px).rgb);
    let ne = luma(textureSample(src, samp, in.uv + vec2<f32>(1.0, -1.0) * px).rgb);
    let sw = luma(textureSample(src, samp, in.uv + vec2<f32>(-1.0, 1.0) * px).rgb);
    let se = luma(textureSample(src, samp, in.uv + vec2<f32>(1.0, 1.0) * px).rgb);
    let lm = luma(m.rgb);
    let lo = min(lm, min(min(nw, ne), min(sw, se)));
    let hi = max(lm, max(max(nw, ne), max(sw, se)));
    // The edge runs across the steepest change.
    var dir = vec2<f32>(-((nw + ne) - (sw + se)), (nw + sw) - (ne + se));
    let reduce = max((nw + ne + sw + se) * 0.25 * REDUCE_MUL, REDUCE_MIN);
    let scale = 1.0 / (min(abs(dir.x), abs(dir.y)) + reduce);
    dir = clamp(dir * scale, vec2<f32>(-SPAN_MAX), vec2<f32>(SPAN_MAX)) * px;
    let a = 0.5 * (
        textureSample(src, samp, in.uv + dir * (1.0 / 3.0 - 0.5)).rgb +
        textureSample(src, samp, in.uv + dir * (2.0 / 3.0 - 0.5)).rgb
    );
    let b = a * 0.5 + 0.25 * (
        textureSample(src, samp, in.uv - dir * 0.5).rgb +
        textureSample(src, samp, in.uv + dir * 0.5).rgb
    );
    // The wider blend overshot the local range: keep the narrower one.
    let lb = luma(b);
    if (lb < lo || lb > hi) {
        return vec4<f32>(a, m.a);
    }
    return vec4<f32>(b, m.a);
}
"#;

pub(crate) struct Fxaa {
    module: wgpu::ShaderModule,
    layout: wgpu::BindGroupLayout,
    pipeline_layout: wgpu::PipelineLayout,
    sampler: wgpu::Sampler,
    pipelines: HashMap<wgpu::TextureFormat, wgpu::RenderPipeline>,
    /// The image before FXAA, by size and format (the screen and render textures).
    inputs: HashMap<((u32, u32), wgpu::TextureFormat), wgpu::TextureView>,
}

impl Fxaa {
    pub fn new(gpu: &Gpu) -> Self {
        let device = &gpu.device;
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("fxaa"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("fxaa"),
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
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("fxaa"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("fxaa"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        Self {
            module,
            layout,
            pipeline_layout,
            sampler,
            pipelines: HashMap::new(),
            inputs: HashMap::new(),
        }
    }

    /// The texture the image goes into before FXAA, made (or remade) for the target.
    pub fn input(
        &mut self,
        gpu: &Gpu,
        size: (u32, u32),
        format: wgpu::TextureFormat,
    ) -> wgpu::TextureView {
        if !self.inputs.contains_key(&(size, format)) {
            if self.inputs.len() >= 4 {
                self.inputs.clear();
            }
            let view = gpu
                .device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some("fxaa input"),
                    size: wgpu::Extent3d {
                        width: size.0.max(1),
                        height: size.1.max(1),
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                })
                .create_view(&Default::default());
            self.inputs.insert((size, format), view);
        }
        self.inputs[&(size, format)].clone()
    }

    /// Smooths the input's edges into `target`.
    pub fn encode(
        &mut self,
        gpu: &Gpu,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        size: (u32, u32),
        format: wgpu::TextureFormat,
    ) {
        let Some(input) = self.inputs.get(&(size, format)) else {
            return;
        };
        let pipeline = self.pipelines.entry(format).or_insert_with(|| {
            gpu.device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("fxaa"),
                    layout: Some(&self.pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &self.module,
                        entry_point: Some("vs"),
                        compilation_options: Default::default(),
                        buffers: &[],
                    },
                    primitive: Default::default(),
                    depth_stencil: None,
                    multisample: Default::default(),
                    fragment: Some(wgpu::FragmentState {
                        module: &self.module,
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
                })
        });
        let group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("fxaa"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(input),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("fxaa"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, &group, &[]);
        pass.draw(0..3, 0..1);
    }
}
