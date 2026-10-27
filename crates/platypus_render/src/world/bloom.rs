// SPDX-License-Identifier: Apache-2.0

//! Post-processing for the world pass: with bloom, exposure or tonemapping on, the world
//! draws into an HDR texture ([`HDR_FORMAT`]), whose bright parts are blurred down and up
//! a chain of half-size textures and added back as it's copied to the target, with
//! exposure and tonemapping applied on the way.
//!
//! Exposure is per camera ("eye"): each view's part of the image is metered and graded
//! with its camera's own state, so split screens and render textures adapt on their own.
//! With auto exposure on, a compute pass builds a histogram of each view's brightness and
//! eases its exposure towards one that shows the chosen part of it as mid grey, like eyes
//! adapting. Auto exposure blends in and out against a manual exposure over a transition,
//! and scripts can read each camera's exposure back (a frame or two late) and reset it.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

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

/// Which parts of the view auto exposure looks at (HDRP's metering modes).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Metering {
    /// The whole view alike.
    Average,
    /// The whole view, the middle counting most (a camera's default).
    #[default]
    CenterWeighted,
    /// Only a small circle in the middle (a camera's spot meter).
    Spot,
}

/// Points of a compensation curve: (metered EV100, stops added), at most this many.
pub const MAX_CURVE_POINTS: usize = 8;

/// Auto exposure (HDRP's Automatic mode, with its histogram): the view's brightness is
/// metered into a histogram, the brightest and darkest parts set aside, and the exposure
/// eases towards one that shows the rest's average as mid grey. So a dark tunnel brightens
/// until the opening at its end burns white, and stepping out into daylight is blinding for
/// a moment, then settles.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AutoExposure {
    /// The darkest and brightest average the exposure follows, in EV100: past them it
    /// stops adapting (a dark cellar stays dark, the sun doesn't turn everything grey).
    pub min_ev: f32,
    pub max_ev: f32,
    /// Stops added after metering (positive: brighter).
    pub compensation: f32,
    /// How fast it adapts (per second, roughly the share of the way covered) when the
    /// view gets brighter, and when it gets darker. Both 3 by default: about a second for a
    /// big change, as games do (eyes take far longer to see in the dark, which is frustrating
    /// to wait for).
    pub speed_dark_to_light: f32,
    pub speed_light_to_dark: f32,
    pub metering: Metering,
    /// The part of the histogram averaged, in percent from the darkest: below `low` and
    /// above `high` is ignored (HDRP suggests 40 and 90: a little sun or a few black
    /// pixels then can't swing it). 0 and 100 average everything.
    pub low_percent: f32,
    pub high_percent: f32,
    /// More stops added by how bright the view is (HDRP's curve mapping): points of
    /// (metered EV100, stops), joined by straight lines, flat past the ends. For example
    /// (-2, -1.5), (6, 0) keeps dark places darker than full adaptation would, without
    /// a hard wall like `min_ev`.
    pub curve: [[f32; 2]; MAX_CURVE_POINTS],
    pub curve_points: u8,
    /// A soft floor instead of `min_ev`'s wall: below `dark_below` (EV100) the eyes only
    /// adapt `dark_adaptation` (0..1) of the way, so a dark room brightens some but still
    /// reads as dark, and darker still looks darker. 1: adapt fully (the default).
    pub dark_adaptation: f32,
    pub dark_below: f32,
    /// Stops adapting: the exposure holds where it is.
    pub hold: bool,
}

impl Default for AutoExposure {
    fn default() -> Self {
        Self {
            min_ev: -10.0,
            max_ev: 20.0,
            compensation: 0.0,
            speed_dark_to_light: 3.0,
            speed_light_to_dark: 3.0,
            metering: Metering::CenterWeighted,
            low_percent: 0.0,
            high_percent: 100.0,
            curve: [[0.0; 2]; MAX_CURVE_POINTS],
            curve_points: 0,
            dark_adaptation: 1.0,
            dark_below: 5.0,
            hold: false,
        }
    }
}

/// Exposure and tonemapping, applied after bloom.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Grading {
    /// Multiplies the light (2 to the post exposure's stops).
    pub exposure: f32,
    pub tonemapping: Tonemapping,
    /// Adapts the exposure to the image (on top of `exposure`).
    pub auto_exposure: Option<AutoExposure>,
    /// The manual exposure, in EV100 (HDRP's Fixed mode), used without auto exposure and
    /// blended with it while it fades in or out. None: light as it is (EV100 -log2(1.2)).
    pub fixed_ev: Option<f32>,
    /// Seconds auto exposure takes to blend in when it's turned on, and out (back to the
    /// manual exposure) when it's turned off, and a change of `fixed_ev` takes to ease to
    /// the new value; 0: at once.
    pub transition: f32,
    /// Seconds since the last frame, for auto exposure to adapt by.
    pub delta_time: f32,
}

impl Default for Grading {
    fn default() -> Self {
        Self {
            exposure: 1.0,
            tonemapping: Tonemapping::None,
            auto_exposure: None,
            fixed_ev: None,
            transition: 0.0,
            delta_time: 1.0 / 60.0,
        }
    }
}

impl Grading {
    /// Changes nothing, so no post pass is needed for it.
    pub fn is_identity(&self) -> bool {
        self.tonemapping == Tonemapping::None
            && (self.exposure - 1.0).abs() < 1e-6
            && self.auto_exposure.is_none()
            && self.fixed_ev.is_none()
    }
}

/// What a camera's exposure was, read back from the GPU.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ExposureReading {
    /// The exposure in use, in EV100 (higher: darker, as a camera's EV setting).
    pub ev: f32,
    /// How bright the view metered, in EV100 (NaN until auto exposure has metered it).
    pub scene_ev: f32,
}

/// The stops `fixed_ev` multiplies light by (0: as it is).
fn fixed_stops(fixed_ev: Option<f32>) -> f32 {
    fixed_ev.map_or(0.0, |ev| -ev - 1.2f32.log2())
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Params {
    threshold: f32,
    knee: f32,
    intensity: f32,
    exposure: f32,
    tonemap: u32,
    metering: u32,
    delta_time: f32,
    compensation: f32,
    min_ev: f32,
    max_ev: f32,
    dark_to_light: f32,
    light_to_dark: f32,
    low: f32,
    high: f32,
    fixed_stops: f32,
    curve_points: u32,
    dark_adaptation: f32,
    dark_below: f32,
    _pad: [f32; 2],
    curve: [[f32; 4]; MAX_CURVE_POINTS / 2],
}

/// A view's part of the image and its camera's blend between manual and auto exposure.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct ViewParams {
    /// The view's rectangle in the image, 0..1: x, y, width, height.
    rect: [f32; 4],
    /// 0: the manual exposure, 1: auto exposure's.
    weight: f32,
    /// 1: jump straight to the metered exposure this frame.
    snap: u32,
    /// 1: don't adapt (hold where it is).
    hold: u32,
    /// What the world was multiplied by on its way in (pre-exposure; 1 outside HD).
    pre: f32,
    /// The manual exposure's stops, as eased so far towards `Grading::fixed_ev`.
    fixed: f32,
    _pad: [f32; 3],
}

/// One camera's exposure state.
struct Eye {
    /// x: the stops auto exposure has adapted to; y: 1 once set; z: the metered EV100.
    meter: wgpu::Buffer,
    view: wgpu::Buffer,
    weight: f32,
    snap: bool,
    /// Where `meter` is copied to be read back, whether a read is under way, and when it's
    /// ready.
    staging: wgpu::Buffer,
    reading: bool,
    ready: Arc<AtomicBool>,
    copied: bool,
    reading_value: Option<ExposureReading>,
    /// The frame it was last drawn in (eyes no camera uses for a while are dropped).
    last_used: u64,
    /// The pre-exposure this frame's world was drawn with (see `pre_exposure`).
    pre: f32,
    /// The manual exposure (stops): where it is now, and the ease it's on (from, to, and how
    /// far along, 0..1): a change to `fixed_ev` takes the transition's time, as turning auto
    /// exposure on or off does.
    fixed: f32,
    fixed_from: f32,
    fixed_to: f32,
    fixed_t: f32,
}

/// A view in this frame's image: whose exposure it uses and where it is (pixels).
#[derive(Clone, Copy, Debug)]
pub(crate) struct ExposureView {
    pub key: u32,
    pub rect: [u32; 4],
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
    meter_layout: wgpu::BindGroupLayout,
    meter: wgpu::ComputePipeline,
    /// Exposure and tonemapping alone, by target format.
    grade: HashMap<wgpu::TextureFormat, wgpu::RenderPipeline>,
    /// The HDR image and the blur levels, by target size (the screen and render
    /// textures).
    textures: HashMap<(u32, u32), (wgpu::TextureView, Vec<wgpu::TextureView>)>,
    /// Each camera's exposure.
    eyes: HashMap<u32, Eye>,
    frame: u64,
    /// The last grading, to tell whether auto exposure is still fading out.
    last: Grading,
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

fn buffer_entry(
    binding: u32,
    visibility: wgpu::ShaderStages,
    ty: wgpu::BufferBindingType,
) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility,
        ty: wgpu::BindingType::Buffer {
            ty,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

impl BloomPass {
    pub fn new(gpu: &Gpu) -> Self {
        let device = &gpu.device;
        let module = device.create_shader_module(wgpu::include_wgsl!("bloom.wgsl"));
        let texture = |binding, visibility| wgpu::BindGroupLayoutEntry {
            binding,
            visibility,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        };
        let fragment = wgpu::ShaderStages::FRAGMENT;
        let compute = wgpu::ShaderStages::COMPUTE;
        let uniform = wgpu::BufferBindingType::Uniform;
        let group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bloom"),
            entries: &[
                texture(0, fragment),
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: fragment,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                buffer_entry(2, fragment, uniform),
                texture(3, fragment),
                buffer_entry(
                    4,
                    fragment,
                    wgpu::BufferBindingType::Storage { read_only: true },
                ),
                buffer_entry(6, fragment, uniform),
            ],
        });
        let meter_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("auto exposure"),
            entries: &[
                buffer_entry(2, compute, uniform),
                texture(3, compute),
                buffer_entry(
                    5,
                    compute,
                    wgpu::BufferBindingType::Storage { read_only: false },
                ),
                buffer_entry(6, compute, uniform),
            ],
        });
        let meter = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("auto exposure"),
            layout: Some(
                &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("auto exposure"),
                    bind_group_layouts: &[Some(&meter_layout)],
                    immediate_size: 0,
                }),
            ),
            module: &module,
            entry_point: Some("cs_meter"),
            compilation_options: Default::default(),
            cache: None,
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
            meter_layout,
            meter,
            module,
            group_layout,
            layout,
            sampler,
            params,
            textures: HashMap::new(),
            eyes: HashMap::new(),
            frame: 0,
            last: Grading::default(),
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

    /// Whether auto exposure is still blending out after being turned off: the post pass
    /// must keep running until it has.
    pub fn fading(&self) -> bool {
        self.last.auto_exposure.is_none() && self.eyes.values().any(|e| e.weight > 0.0)
    }

    /// The camera `key`'s exposure as last read back.
    pub fn exposure(&self, key: u32) -> Option<ExposureReading> {
        self.eyes.get(&key).and_then(|e| e.reading_value)
    }

    /// The multiplier camera `key`'s world is pre-exposed by this frame (real-world light units): its
    /// last read-back exposure, kept inside a sane range. Remembered, so the post pass divides
    /// by the same value.
    pub fn pre_exposure(&mut self, key: u32) -> Option<f32> {
        let eye = self.eyes.get_mut(&key)?;
        let reading = eye.reading_value?;
        let k = (1.0 / (1.2 * 2f32.powf(reading.ev))).clamp(1e-6, 1e6);
        eye.pre = k;
        Some(k)
    }

    /// Makes camera `key`'s auto exposure jump straight to the view next frame (a cut).
    pub fn reset_exposure(&mut self, key: u32) {
        if let Some(e) = self.eyes.get_mut(&key) {
            e.snap = true;
        }
    }

    /// Picks up finished read-backs and starts new ones: after the frame is submitted.
    pub fn after_submit(&mut self, gpu: &Gpu) {
        for eye in self.eyes.values_mut() {
            if eye.copied && !eye.reading {
                eye.copied = false;
                eye.reading = true;
                let ready = eye.ready.clone();
                eye.staging
                    .slice(..)
                    .map_async(wgpu::MapMode::Read, move |r| {
                        if r.is_ok() {
                            ready.store(true, Ordering::Release);
                        }
                    });
            }
        }
        let _ = gpu.device.poll(wgpu::PollType::Poll);
    }

    /// Reads back what finished mapping: each eye's stops and metered EV.
    fn collect(&mut self) {
        for eye in self.eyes.values_mut() {
            let fixed = eye.fixed;
            if eye.reading && eye.ready.swap(false, Ordering::Acquire) {
                if let Ok(data) = eye.staging.slice(..).get_mapped_range() {
                    let v: &[f32] = bytemuck::cast_slice(&data[..16]);
                    let stops = if v[1] > 0.5 {
                        fixed + (v[0] - fixed) * eye.weight
                    } else {
                        fixed
                    };
                    eye.reading_value = Some(ExposureReading {
                        ev: -stops - 1.2f32.log2(),
                        scene_ev: if v[1] > 0.5 { v[2] } else { f32::NAN },
                    });
                }
                eye.staging.unmap();
                eye.reading = false;
            }
        }
    }

    fn eye(&mut self, gpu: &Gpu, key: u32, on: bool) -> &mut Eye {
        self.eyes.entry(key).or_insert_with(|| {
            let buffer = |label, size, usage| {
                gpu.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some(label),
                    size,
                    usage,
                    mapped_at_creation: false,
                })
            };
            Eye {
                meter: buffer(
                    "auto exposure",
                    16,
                    wgpu::BufferUsages::STORAGE
                        | wgpu::BufferUsages::COPY_SRC
                        | wgpu::BufferUsages::COPY_DST,
                ),
                view: buffer(
                    "auto exposure view",
                    std::mem::size_of::<ViewParams>() as u64,
                    wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                ),
                // A new camera starts adapted: no fade in when a game starts.
                weight: if on { 1.0 } else { 0.0 },
                snap: true,
                staging: buffer(
                    "auto exposure read-back",
                    16,
                    wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                ),
                reading: false,
                ready: Arc::new(AtomicBool::new(false)),
                copied: false,
                reading_value: None,
                last_used: 0,
                pre: 1.0,
                fixed: f32::NAN,
                fixed_from: 0.0,
                fixed_to: 0.0,
                fixed_t: 1.0,
            }
        })
    }

    /// Blurs the HDR image's bright parts (with `settings`) and writes the image plus the
    /// glow to `target`, graded, each of `views` with its own camera's exposure (the first
    /// is drawn over the whole image, the rest over their own rectangles).
    pub fn encode(
        &mut self,
        gpu: &Gpu,
        encoder: &mut wgpu::CommandEncoder,
        // Where the result goes (its size picks the HDR image).
        out: &crate::RenderTarget<'_>,
        settings: Option<&Bloom>,
        grading: Grading,
        views: &[ExposureView],
    ) {
        let (target, format, size) = (out.view, out.format, out.size);
        if !self.textures.contains_key(&size) {
            return;
        }
        self.collect();
        self.frame += 1;
        self.last = grading;
        let auto = grading.auto_exposure.unwrap_or_default();
        let on = grading.auto_exposure.is_some();
        // Capped: a hitch (loading, compiling shaders) mustn't swing it at once.
        let dt = grading.delta_time.clamp(0.0, 1.0 / 15.0);
        let bloom = settings.copied().unwrap_or(Bloom {
            intensity: 0.0,
            ..Default::default()
        });
        let knee = (bloom.threshold * 0.5).max(1e-3);
        let mut curve = [[0.0f32; 4]; MAX_CURVE_POINTS / 2];
        for (i, p) in auto
            .curve
            .iter()
            .take(usize::from(auto.curve_points))
            .enumerate()
        {
            curve[i / 2][(i % 2) * 2] = p[0];
            curve[i / 2][(i % 2) * 2 + 1] = p[1];
        }
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
                metering: match auto.metering {
                    Metering::Average => 0,
                    Metering::CenterWeighted => 1,
                    Metering::Spot => 2,
                },
                delta_time: dt,
                compensation: auto.compensation,
                min_ev: auto.min_ev,
                max_ev: auto.max_ev.max(auto.min_ev),
                dark_to_light: auto.speed_dark_to_light.max(0.0),
                light_to_dark: auto.speed_light_to_dark.max(0.0),
                low: auto.low_percent.clamp(0.0, 100.0),
                high: auto.high_percent.clamp(0.0, 100.0).max(auto.low_percent),
                fixed_stops: fixed_stops(grading.fixed_ev),
                curve_points: u32::from(auto.curve_points).min(MAX_CURVE_POINTS as u32),
                dark_adaptation: auto.dark_adaptation.clamp(0.0, 1.0),
                dark_below: auto.dark_below,
                _pad: [0.0; 2],
                curve,
            }),
        );
        // Each camera's blend moves towards auto exposure (on) or the manual one (off).
        let step = if grading.transition > 0.0 {
            dt / grading.transition
        } else {
            1.0
        };
        let frame = self.frame;
        let views: Vec<ExposureView> = if views.is_empty() {
            vec![ExposureView {
                key: 0,
                rect: [0, 0, size.0, size.1],
            }]
        } else {
            views.to_vec()
        };
        for v in &views {
            let eye = self.eye(gpu, v.key, on);
            eye.last_used = frame;
            let goal = if on { 1.0 } else { 0.0 };
            eye.weight = if eye.weight < goal {
                (eye.weight + step).min(goal)
            } else {
                (eye.weight - step).max(goal)
            };
            // The manual exposure eases to a new value over the transition (a new camera
            // starts there).
            let target = fixed_stops(grading.fixed_ev);
            if eye.fixed.is_nan() {
                (eye.fixed, eye.fixed_from, eye.fixed_to, eye.fixed_t) =
                    (target, target, target, 1.0);
            } else if target != eye.fixed_to {
                (eye.fixed_from, eye.fixed_to, eye.fixed_t) = (eye.fixed, target, 0.0);
            }
            eye.fixed_t = (eye.fixed_t + step).min(1.0);
            let u = eye.fixed_t * eye.fixed_t * (3.0 - 2.0 * eye.fixed_t);
            eye.fixed = eye.fixed_from + (eye.fixed_to - eye.fixed_from) * u;
            let [x, y, w, h] = v.rect;
            let (sw, sh) = (size.0.max(1) as f32, size.1.max(1) as f32);
            let params = ViewParams {
                rect: [x as f32 / sw, y as f32 / sh, w as f32 / sw, h as f32 / sh],
                weight: eye.weight,
                snap: u32::from(eye.snap),
                hold: u32::from(auto.hold),
                pre: eye.pre,
                fixed: eye.fixed,
                _pad: [0.0; 3],
            };
            gpu.queue
                .write_buffer(&eye.view, 0, bytemuck::bytes_of(&params));
        }
        // Cameras gone for a while take their state with them.
        self.eyes
            .retain(|_, e| frame.saturating_sub(e.last_used) < 600 || e.reading);
        let (hdr, levels) = &self.textures[&size];
        if on {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&self.meter);
            for v in &views {
                let eye = &self.eyes[&v.key];
                let group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("auto exposure"),
                    layout: &self.meter_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: self.params.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 3,
                            resource: wgpu::BindingResource::TextureView(hdr),
                        },
                        wgpu::BindGroupEntry {
                            binding: 5,
                            resource: eye.meter.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 6,
                            resource: eye.view.as_entire_binding(),
                        },
                    ],
                });
                pass.set_bind_group(0, &group, &[]);
                pass.dispatch_workgroups(1, 1, 1);
            }
        }
        for v in &views {
            if let Some(eye) = self.eyes.get_mut(&v.key) {
                // Snapped once metered.
                if on {
                    eye.snap = false;
                }
                if !eye.reading && !eye.copied {
                    encoder.copy_buffer_to_buffer(&eye.meter, 0, &eye.staging, 0, 16);
                    eye.copied = true;
                }
            }
        }
        let (module, layout) = (&self.module, &self.layout);
        let composite = self
            .composite
            .entry(format)
            .or_insert_with(|| pipeline(gpu, module, layout, "fs_composite", format, None));
        let grade = self
            .grade
            .entry(format)
            .or_insert_with(|| pipeline(gpu, module, layout, "fs_grade", format, None));
        let group = |src: &wgpu::TextureView, eye: &Eye| {
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
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: eye.meter.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 6,
                        resource: eye.view.as_entire_binding(),
                    },
                ],
            })
        };
        let eyes = &self.eyes;
        // Draws `src` through `pipeline` into `out` (`out_size` pixels). With `per_view`,
        // once per view with its camera's exposure: the first over everything, the rest
        // over their own rectangles (scaled to `out`).
        let draw = |encoder: &mut wgpu::CommandEncoder,
                    out: &wgpu::TextureView,
                    out_size: (u32, u32),
                    src: &wgpu::TextureView,
                    pipeline: &wgpu::RenderPipeline,
                    load: wgpu::LoadOp<wgpu::Color>,
                    constant: f64,
                    per_view: bool| {
            let parts = if per_view { views.len() } else { 1 };
            let binds: Vec<wgpu::BindGroup> = (0..parts)
                .map(|i| group(src, &eyes[&views[i].key]))
                .collect();
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
            pass.set_blend_constant(wgpu::Color {
                r: constant,
                g: constant,
                b: constant,
                a: constant,
            });
            for (i, bind) in binds.iter().enumerate() {
                if i > 0 {
                    let [x, y, w, h] = views[i].rect;
                    let sx = |v: u32| {
                        (u64::from(v) * u64::from(out_size.0) / u64::from(size.0.max(1))) as u32
                    };
                    let sy = |v: u32| {
                        (u64::from(v) * u64::from(out_size.1) / u64::from(size.1.max(1))) as u32
                    };
                    let (x0, y0) = (sx(x).min(out_size.0), sy(y).min(out_size.1));
                    let (x1, y1) = (sx(x + w).min(out_size.0), sy(y + h).min(out_size.1));
                    if x1 <= x0 || y1 <= y0 {
                        continue;
                    }
                    pass.set_scissor_rect(x0, y0, x1 - x0, y1 - y0);
                }
                pass.set_bind_group(0, bind, &[]);
                pass.draw(0..3, 0..1);
            }
        };
        let clear = wgpu::LoadOp::Clear(wgpu::Color::BLACK);
        if settings.is_none() {
            draw(encoder, target, size, hdr, grade, clear, 0.0, true);
            return;
        }
        if levels.is_empty() {
            draw(encoder, target, size, hdr, composite, clear, 0.0, true);
            return;
        }
        let half = ((size.0 / 2).max(1), (size.1 / 2).max(1));
        draw(
            encoder,
            &levels[0],
            half,
            hdr,
            &self.prefilter,
            clear,
            0.0,
            true,
        );
        for i in 1..levels.len() {
            draw(
                encoder,
                &levels[i],
                (0, 0),
                &levels[i - 1],
                &self.down,
                clear,
                0.0,
                false,
            );
        }
        let scatter = f64::from(bloom.scatter.clamp(0.0, 1.0));
        for i in (0..levels.len() - 1).rev() {
            draw(
                encoder,
                &levels[i],
                (0, 0),
                &levels[i + 1],
                &self.up,
                wgpu::LoadOp::Load,
                scatter,
                false,
            );
        }
        draw(
            encoder, target, size, &levels[0], composite, clear, 0.0, true,
        );
    }
}
