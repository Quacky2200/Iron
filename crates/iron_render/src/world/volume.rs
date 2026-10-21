//! Light shafts' froxel volume: 3D textures over each view's frustum (screen cells by depth
//! slices). Two compute passes fill them once per view:
//! - `cs_volume_light` lights each cell: the light the fog there scatters towards the camera,
//!   taken at a point that moves around inside the cell from frame to frame and blended
//!   with what the cell held last frame (found by reprojecting it). Over a few frames each
//!   cell holds its average over its whole extent, so beams through bars and leaves come out
//!   smooth instead of in stair steps, for one shadow lookup per cell per frame.
//! - `cs_volume` adds the lit fog up from the eye out to each slice, with the fog's
//!   transmittance, into the volume draws read.
//!
//! Everything the view draws then reads it with trilinear filtering, so shafts cost the same
//! however much overdraw there is.

use bytemuck::{Pod, Zeroable};
use iron_core::glam::{Mat4, Vec3};

use crate::Gpu;

/// Screen cells across and down, and depth slices, for each detail level (low, medium,
/// high, ultra): finer volumes make sharper beams and cost more to fill.
pub const SIZES: [(u32, u32, u32); 4] = [
    (160, 90, 48),
    (240, 135, 64),
    (320, 180, 96),
    (480, 270, 128),
];
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
/// Where in its cell each frame samples: a Halton (2, 3, 5) sequence, so any run of frames
/// spreads its samples evenly over the cell. Frame 0 takes the middle.
const JITTER: usize = 64;

fn halton(mut i: usize, base: usize) -> f32 {
    let (mut f, mut r) = (1.0, 0.0);
    while i > 0 {
        f /= base as f32;
        r += f * (i % base) as f32;
        i /= base;
    }
    r
}

fn jitter(frame: u64) -> [f32; 3] {
    let i = (frame % JITTER as u64) as usize;
    if i == 0 {
        return [0.5; 3];
    }
    [halton(i, 2), halton(i, 3), halton(i, 5)]
}

/// The camera a volume is filled for, to reproject the next frame's cells into it.
#[derive(Clone, Copy, PartialEq)]
pub struct VolumeCamera {
    pub view_proj: Mat4,
    pub position: Vec3,
    pub forward: Vec3,
    pub orthographic: bool,
    /// How far the volume reaches, in metres.
    pub distance: f32,
    /// Which lights light the fog and which of them have shadows: when that changes (a
    /// light turned on, shadows arriving on the first frames) last frame's cells are wrong
    /// everywhere, so they're dropped rather than blended out over a second.
    pub lights: u64,
    /// Blending with last frame's cells (`Denoising::temporal`); without it each frame
    /// samples the cells' middles and stands alone.
    pub reprojection: bool,
    /// How much of each new frame goes into a cell (1 - the history weight).
    pub blend: f32,
    /// Shadow samples per cell each frame, and for cells whose view moved.
    pub samples: u32,
    pub motion_samples: u32,
    /// The spot and point lights in the volume (up to `LOCAL_POSES`): where they are, which
    /// way they point and their range. When one moves (a sweeping beam) the history counts
    /// less, by how far its beam moved.
    pub local_poses: [LocalPose; LOCAL_POSES],
    pub local_count: usize,
}

/// How many moving lights the volume follows; any more count as moving a long way.
pub const LOCAL_POSES: usize = 4;

/// A spot or point light's place in the volume, to tell how far its beam moved.
#[derive(Clone, Copy, PartialEq, Default)]
pub struct LocalPose {
    pub position: Vec3,
    pub direction: Vec3,
    pub range: f32,
}

/// How far (metres) the furthest-moved light's beam swept since `last`: its move plus its
/// turn at the end of its range. A light more or less counts as a long way.
fn light_shift(last: &VolumeCamera, now: &VolumeCamera) -> f32 {
    if last.local_count != now.local_count || now.local_count > LOCAL_POSES {
        return 1.0;
    }
    (0..now.local_count)
        .map(|i| {
            let (a, b) = (last.local_poses[i], now.local_poses[i]);
            let turn = a.direction.dot(b.direction).clamp(-1.0, 1.0).acos();
            (a.position - b.position).length() + turn * b.range
        })
        .fold(0.0, f32::max)
}

/// `VolumeFrame` in world.wgsl.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct GpuFrame {
    prev_view_proj: [[f32; 4]; 4],
    prev_camera: [f32; 4],
    prev_view_dir: [f32; 4],
    jitter: [f32; 4],
    /// x: samples per cell; y: for cells whose view moved.
    samples: [f32; 4],
}

/// One view slot's textures and what it last held.
struct Slot {
    size: (u32, u32, u32),
    /// Lit cells, this frame's and last frame's (swapped every frame).
    light: [wgpu::TextureView; 2],
    /// The added-up volume draws read.
    volume: wgpu::TextureView,
    frame: wgpu::Buffer,
    /// Which of `light` holds last frame's cells.
    current: usize,
    /// The camera last frame's cells were lit for (none: start over).
    last: Option<VolumeCamera>,
    frames: u64,
}

pub struct VolumePass {
    /// What draws read: the volume and its sampler (group 3).
    pub read_layout: wgpu::BindGroupLayout,
    light_layout: wgpu::BindGroupLayout,
    sum_layout: wgpu::BindGroupLayout,
    light_pipeline: wgpu::ComputePipeline,
    sum_pipeline: wgpu::ComputePipeline,
    sampler: wgpu::Sampler,
    /// A black 1-texel volume for views without shafts.
    empty: wgpu::BindGroup,
    /// One set per view slot, made on first use (and remade when its size changes).
    slots: Vec<Option<Slot>>,
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

fn view(binding: u32, v: &wgpu::TextureView) -> wgpu::BindGroupEntry<'_> {
    wgpu::BindGroupEntry {
        binding,
        resource: wgpu::BindingResource::TextureView(v),
    }
}

fn entry(binding: u32, ty: wgpu::BindingType) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty,
        count: None,
    }
}

fn sampled() -> wgpu::BindingType {
    wgpu::BindingType::Texture {
        sample_type: wgpu::TextureSampleType::Float { filterable: true },
        view_dimension: wgpu::TextureViewDimension::D3,
        multisampled: false,
    }
}

fn storage() -> wgpu::BindingType {
    wgpu::BindingType::StorageTexture {
        access: wgpu::StorageTextureAccess::WriteOnly,
        format: FORMAT,
        view_dimension: wgpu::TextureViewDimension::D3,
    }
}

const SAMPLER: wgpu::BindingType = wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering);

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
                    ty: sampled(),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: SAMPLER,
                    count: None,
                },
            ],
        });
        // The compute passes bind at other numbers: one module can't bind two variables to
        // the same slot.
        let light_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("fog volume (light)"),
            entries: &[
                entry(2, SAMPLER),
                entry(3, sampled()),
                entry(
                    4,
                    wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                ),
                entry(6, storage()),
            ],
        });
        let sum_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("fog volume (sum)"),
            entries: &[entry(1, storage()), entry(2, SAMPLER), entry(5, sampled())],
        });
        let pipeline = |label: &str, entry_point: &str, layout: &wgpu::BindGroupLayout| {
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some(label),
                bind_group_layouts: &[
                    Some(globals_layout),
                    None,
                    Some(shadow_layout),
                    Some(layout),
                ],
                immediate_size: 0,
            });
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(label),
                layout: Some(&layout),
                module,
                entry_point: Some(entry_point),
                compilation_options: Default::default(),
                cache: None,
            })
        };
        let light_pipeline = pipeline("fog volume (light)", "cs_volume_light", &light_layout);
        let sum_pipeline = pipeline("fog volume (sum)", "cs_volume", &sum_layout);
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
            light_layout,
            sum_layout,
            light_pipeline,
            sum_pipeline,
            sampler,
            empty,
            slots: Vec::new(),
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

    /// Fills slot `slot`'s volume (`size` cells) for the view whose globals are at `offset`
    /// in `globals`, seen by `camera`, and returns the group its draws read it through.
    #[allow(clippy::too_many_arguments)]
    pub fn encode(
        &mut self,
        gpu: &Gpu,
        encoder: &mut wgpu::CommandEncoder,
        slot: usize,
        size: (u32, u32, u32),
        camera: VolumeCamera,
        globals: &wgpu::BindGroup,
        offset: u32,
        lighting: &wgpu::BindGroup,
    ) -> wgpu::BindGroup {
        let device = &gpu.device;
        while self.slots.len() <= slot {
            self.slots.push(None);
        }
        if self.slots[slot].as_ref().is_none_or(|s| s.size != size) {
            self.slots[slot] = Some(Slot {
                size,
                light: [volume_texture(device, size), volume_texture(device, size)],
                volume: volume_texture(device, size),
                frame: device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("fog volume frame"),
                    size: size_of::<GpuFrame>() as u64,
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                }),
                current: 0,
                last: None,
                frames: 0,
            });
        }
        let s = self.slots[slot].as_mut().expect("made above");
        let [jx, jy, jz] = if camera.reprojection {
            jitter(s.frames)
        } else {
            [0.5; 3]
        };
        // Without last frame's cells, this frame's stand alone.
        let (prev, blend) = match s.last {
            Some(last) if last.lights == camera.lights && camera.reprojection => {
                (last, camera.blend.clamp(0.02, 1.0))
            }
            _ => (camera, 1.0),
        };
        let p = prev.position;
        let f = prev.forward;
        let frame = GpuFrame {
            prev_view_proj: prev.view_proj.to_cols_array_2d(),
            prev_camera: [p.x, p.y, p.z, f32::from(u8::from(prev.orthographic))],
            prev_view_dir: [f.x, f.y, f.z, prev.distance],
            jitter: [jx, jy, jz, blend],
            samples: [
                camera.samples.max(1) as f32,
                camera.motion_samples as f32,
                s.last.map_or(0.0, |l| light_shift(&l, &camera)),
                0.0,
            ],
        };
        gpu.queue
            .write_buffer(&s.frame, 0, bytemuck::bytes_of(&frame));
        let (history, lit) = (&s.light[s.current], &s.light[1 - s.current]);
        let sampler = wgpu::BindGroupEntry {
            binding: 2,
            resource: wgpu::BindingResource::Sampler(&self.sampler),
        };
        let light_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("fog volume (light)"),
            layout: &self.light_layout,
            entries: &[
                sampler.clone(),
                view(3, history),
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: s.frame.as_entire_binding(),
                },
                view(6, lit),
            ],
        });
        let sum_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("fog volume (sum)"),
            layout: &self.sum_layout,
            entries: &[view(1, &s.volume), sampler, view(5, lit)],
        });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("fog volume"),
                timestamp_writes: None,
            });
            pass.set_bind_group(0, globals, &[offset]);
            pass.set_bind_group(2, lighting, &[]);
            pass.set_pipeline(&self.light_pipeline);
            pass.set_bind_group(3, &light_group, &[]);
            pass.dispatch_workgroups(size.0.div_ceil(4), size.1.div_ceil(4), size.2.div_ceil(4));
            pass.set_pipeline(&self.sum_pipeline);
            pass.set_bind_group(3, &sum_group, &[]);
            pass.dispatch_workgroups(size.0.div_ceil(8), size.1.div_ceil(8), 1);
        }
        s.current = 1 - s.current;
        s.last = Some(camera);
        s.frames += 1;
        Self::read_group(device, &self.read_layout, &s.volume, &self.sampler)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jitter_starts_in_the_middle_and_spreads_over_the_cell() {
        assert_eq!(jitter(0), [0.5; 3]);
        let mean: f32 = (0..JITTER as u64).map(|i| jitter(i)[0]).sum::<f32>() / JITTER as f32;
        assert!((mean - 0.5).abs() < 0.05, "{mean}");
        assert!((1..JITTER as u64).all(|i| jitter(i).iter().all(|v| (0.0..1.0).contains(v))));
    }
}
