//! Light shafts' froxel volume: a 3D texture over each view's frustum (screen cells by depth
//! slices) holding the light the fog scatters towards the camera, added up from the eye
//! out to each slice. A compute pass (`cs_volume` in world.wgsl) fills it once per view;
//! everything the view draws then reads it with trilinear filtering, so shafts are smooth
//! (no per-pixel noise or banding) and cost the same however much overdraw there is.

use crate::Gpu;

/// Screen cells across and down, and depth slices.
pub const SIZE: (u32, u32, u32) = (160, 90, 64);
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

pub struct VolumePass {
    /// What draws read: the volume and its sampler (group 3).
    pub read_layout: wgpu::BindGroupLayout,
    write_layout: wgpu::BindGroupLayout,
    pipeline: wgpu::ComputePipeline,
    sampler: wgpu::Sampler,
    /// A black 1-texel volume for views without shafts.
    empty: wgpu::BindGroup,
    /// One volume per view slot, made on first use.
    volumes: Vec<wgpu::TextureView>,
}

fn volume_texture(device: &wgpu::Device, size: (u32, u32, u32)) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("fog volume"),
            size: wgpu::Extent3d {
                width: size.0,
                height: size.1,
                depth_or_array_layers: size.2,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D3,
            format: FORMAT,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::STORAGE_BINDING,
            view_formats: &[],
        })
        .create_view(&Default::default())
}

impl VolumePass {
    pub fn new(
        device: &wgpu::Device,
        module: &wgpu::ShaderModule,
        globals_layout: &wgpu::BindGroupLayout,
        shadow_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let read_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("fog volume (read)"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D3,
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
        // The compute pass writes at another binding: one module can't bind two variables
        // to the same slot.
        let write_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("fog volume (write)"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::StorageTexture {
                    access: wgpu::StorageTextureAccess::WriteOnly,
                    format: FORMAT,
                    view_dimension: wgpu::TextureViewDimension::D3,
                },
                count: None,
            }],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("fog volume"),
            bind_group_layouts: &[
                Some(globals_layout),
                None,
                Some(shadow_layout),
                Some(&write_layout),
            ],
            immediate_size: 0,
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("fog volume"),
            layout: Some(&layout),
            module,
            entry_point: Some("cs_volume"),
            compilation_options: Default::default(),
            cache: None,
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("fog volume"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let black = volume_texture(device, (1, 1, 1));
        let empty = Self::read_group(device, &read_layout, &black, &sampler);
        Self {
            read_layout,
            write_layout,
            pipeline,
            sampler,
            empty,
            volumes: Vec::new(),
        }
    }

    fn read_group(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        view: &wgpu::TextureView,
        sampler: &wgpu::Sampler,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("fog volume (read)"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        })
    }

    /// For views without shafts: reads as no light.
    pub fn empty(&self) -> &wgpu::BindGroup {
        &self.empty
    }

    /// Fills slot `slot`'s volume for the view whose globals are at `offset` in
    /// `globals`, and returns the group its draws read it through.
    pub fn encode(
        &mut self,
        gpu: &Gpu,
        encoder: &mut wgpu::CommandEncoder,
        slot: usize,
        globals: &wgpu::BindGroup,
        offset: u32,
        lighting: &wgpu::BindGroup,
    ) -> wgpu::BindGroup {
        while self.volumes.len() <= slot {
            self.volumes.push(volume_texture(&gpu.device, SIZE));
        }
        let view = &self.volumes[slot];
        let write = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("fog volume (write)"),
            layout: &self.write_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(view),
            }],
        });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("fog volume"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, globals, &[offset]);
            pass.set_bind_group(2, lighting, &[]);
            pass.set_bind_group(3, &write, &[]);
            pass.dispatch_workgroups(SIZE.0.div_ceil(8), SIZE.1.div_ceil(8), 1);
        }
        Self::read_group(&gpu.device, &self.read_layout, view, &self.sampler)
    }
}
