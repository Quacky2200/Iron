// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

//! Environment cube maps for reflections and sky light: HDR (`Rgba16Float`) cubes with
//! a mip chain, each mip blurrier, so rough surfaces sample a lower mip. The sky's is
//! drawn procedurally ([`Sky`]); reflection probes copy six renders into theirs.

use bytemuck::{Pod, Zeroable};
use platypus_core::glam::Vec3;

use crate::Gpu;

pub const ENV_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

/// A procedural sky (Unity's procedural skybox, simplified): linear HDR colours.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sky {
    /// Straight up.
    pub zenith: [f32; 3],
    /// At the horizon.
    pub horizon: [f32; 3],
    /// Below the horizon.
    pub ground: [f32; 3],
    /// Towards the sun (`None`: no sun disc).
    pub sun_direction: Option<Vec3>,
    /// The disc's colour (HDR: brighter than 1 blooms and lights shiny things).
    pub sun_color: [f32; 3],
    /// The disc's angular diameter in degrees (the real sun is about 0.5).
    pub sun_size: f32,
}

impl Default for Sky {
    fn default() -> Self {
        Self {
            zenith: [0.16, 0.32, 0.62],
            horizon: [0.62, 0.72, 0.82],
            ground: [0.22, 0.2, 0.18],
            sun_direction: Some(Vec3::new(0.4, 0.6, 0.3).normalize()),
            sun_color: [20.0, 18.0, 15.0],
            sun_size: 1.5,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct GpuSky {
    zenith: [f32; 4],
    horizon: [f32; 4],
    ground: [f32; 4],
    sun_dir: [f32; 4],
    sun_color: [f32; 4],
}

/// A cube map and its views: the whole cube for sampling, and each face of each mip
/// as a render target.
pub struct EnvMap {
    pub size: u32,
    pub mips: u32,
    _texture: wgpu::Texture,
    pub cube: wgpu::TextureView,
    /// `faces[mip][face]`.
    faces: Vec<Vec<wgpu::TextureView>>,
}

impl EnvMap {
    pub fn new(gpu: &Gpu, size: u32) -> Self {
        let mips = size.max(1).ilog2() + 1;
        let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("environment"),
            size: wgpu::Extent3d {
                width: size,
                height: size,
                depth_or_array_layers: 6,
            },
            mip_level_count: mips,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: ENV_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let cube = texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::Cube),
            ..Default::default()
        });
        let faces = (0..mips)
            .map(|mip| {
                (0..6)
                    .map(|face| {
                        texture.create_view(&wgpu::TextureViewDescriptor {
                            dimension: Some(wgpu::TextureViewDimension::D2),
                            base_mip_level: mip,
                            mip_level_count: Some(1),
                            base_array_layer: face,
                            array_layer_count: Some(1),
                            ..Default::default()
                        })
                    })
                    .collect()
            })
            .collect();
        Self {
            size,
            mips,
            _texture: texture,
            cube,
            faces,
        }
    }

    pub fn face(&self, mip: u32, face: u32) -> &wgpu::TextureView {
        &self.faces[mip as usize][face as usize]
    }
}

/// Pipelines that fill environment maps.
pub struct EnvPass {
    sky_layout: wgpu::BindGroupLayout,
    blit_layout: wgpu::BindGroupLayout,
    sky: wgpu::RenderPipeline,
    down: wgpu::RenderPipeline,
    copy_flip: wgpu::RenderPipeline,
    sampler: wgpu::Sampler,
    /// One `GpuSky` per face, 256 bytes apart (a dynamic offset picks one).
    sky_buffer: wgpu::Buffer,
}

const SKY_STRIDE: u64 = 256;

impl EnvPass {
    pub fn new(gpu: &Gpu) -> Self {
        let device = &gpu.device;
        let module = device.create_shader_module(wgpu::include_wgsl!("env.wgsl"));
        let sky_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("sky"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: wgpu::BufferSize::new(std::mem::size_of::<GpuSky>() as u64),
                },
                count: None,
            }],
        });
        let blit_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("environment blit"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let pipeline = |layout: &wgpu::BindGroupLayout, fragment: &str| {
            let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("environment"),
                bind_group_layouts: &[Some(layout)],
                immediate_size: 0,
            });
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("environment"),
                layout: Some(&pl),
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
                    entry_point: Some(fragment),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: ENV_FORMAT,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            })
        };
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("environment"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let sky_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sky"),
            size: SKY_STRIDE * 6,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self {
            sky: pipeline(&sky_layout, "fs_sky"),
            down: pipeline(&blit_layout, "fs_down"),
            copy_flip: pipeline(&blit_layout, "fs_copy_flip"),
            sky_layout,
            blit_layout,
            sampler,
            sky_buffer,
        }
    }

    fn draw(
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        pipeline: &wgpu::RenderPipeline,
        group: &wgpu::BindGroup,
        offsets: &[u32],
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("environment"),
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
        pass.set_bind_group(0, group, offsets);
        pass.draw(0..3, 0..1);
    }

    fn blit_group(&self, gpu: &Gpu, src: &wgpu::TextureView) -> wgpu::BindGroup {
        gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("environment blit"),
            layout: &self.blit_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(src),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        })
    }

    /// Draws `sky` into `map`'s six faces and blurs its mips.
    pub fn draw_sky(&self, gpu: &Gpu, encoder: &mut wgpu::CommandEncoder, map: &EnvMap, sky: &Sky) {
        let (sun, cos) = match sky.sun_direction {
            Some(d) => (
                d.normalize_or(Vec3::Y),
                (sky.sun_size.max(0.01).to_radians() / 2.0).cos(),
            ),
            None => (Vec3::Y, 2.0),
        };
        for face in 0..6u32 {
            let s = GpuSky {
                zenith: [sky.zenith[0], sky.zenith[1], sky.zenith[2], 0.0],
                horizon: [sky.horizon[0], sky.horizon[1], sky.horizon[2], 0.0],
                ground: [sky.ground[0], sky.ground[1], sky.ground[2], 0.0],
                sun_dir: [sun.x, sun.y, sun.z, cos],
                sun_color: [
                    sky.sun_color[0],
                    sky.sun_color[1],
                    sky.sun_color[2],
                    face as f32,
                ],
            };
            gpu.queue.write_buffer(
                &self.sky_buffer,
                SKY_STRIDE * u64::from(face),
                bytemuck::bytes_of(&s),
            );
        }
        let group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sky"),
            layout: &self.sky_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &self.sky_buffer,
                    offset: 0,
                    size: wgpu::BufferSize::new(std::mem::size_of::<GpuSky>() as u64),
                }),
            }],
        });
        for face in 0..6u32 {
            let offset = (SKY_STRIDE * u64::from(face)) as u32;
            Self::draw(encoder, map.face(0, face), &self.sky, &group, &[offset]);
        }
        self.blur_mips(gpu, encoder, map);
    }

    /// Copies a 2D render (a probe camera's view) into face `face` of `map`'s top mip.
    pub fn copy_face(
        &self,
        gpu: &Gpu,
        encoder: &mut wgpu::CommandEncoder,
        src: &wgpu::TextureView,
        map: &EnvMap,
        face: u32,
    ) {
        let group = self.blit_group(gpu, src);
        Self::draw(encoder, map.face(0, face), &self.copy_flip, &group, &[]);
    }

    /// Fills mips 1.. from the one above, each face on its own (seams blur a little
    /// differently at face edges; fine for reflections).
    pub fn blur_mips(&self, gpu: &Gpu, encoder: &mut wgpu::CommandEncoder, map: &EnvMap) {
        for mip in 1..map.mips {
            for face in 0..6 {
                let group = self.blit_group(gpu, map.face(mip - 1, face));
                Self::draw(encoder, map.face(mip, face), &self.down, &group, &[]);
            }
        }
    }
}

/// Which way a reflection probe's camera looks for each cube face, and its up: a camera
/// with these renders the face mirrored left to right (`copy_face` flips it back).
pub fn face_camera(face: u32) -> (Vec3, Vec3) {
    match face {
        0 => (Vec3::X, Vec3::Y),
        1 => (Vec3::NEG_X, Vec3::Y),
        2 => (Vec3::Y, Vec3::NEG_Z),
        3 => (Vec3::NEG_Y, Vec3::Z),
        4 => (Vec3::Z, Vec3::Y),
        _ => (Vec3::NEG_Z, Vec3::Y),
    }
}
