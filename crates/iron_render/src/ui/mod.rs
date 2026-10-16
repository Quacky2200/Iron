//! Draws `iron_ui` draw lists: rounded boxes, borders, gradients, box shadows,
//! blur filters, group opacity and backdrop blur (DESIGN.md §5.2).
//!
//! A frame is drawn into an internal canvas texture, then blitted to the target.
//! Plain boxes and shadows are batched into one instanced draw; each layer or
//! backdrop blur adds a few full-screen passes limited by scissor rects.

mod atlas;
mod gradient;
mod images;

use std::collections::HashMap;
use std::ops::Range;
use std::path::PathBuf;

use iron_ui::{DrawCmd, Rect, Rgba};
use wgpu::util::DeviceExt;

use crate::{Color, Gpu};
use atlas::{GlyphAtlas, SUBPIXEL_BINS};
use gradient::GpuGradient;
use images::ImageCache;

/// Format of the canvas and layer textures. sRGB storage, linear blending.
pub const CANVAS_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

/// Largest blur kernel (taps either side) a single pass will use.
const MAX_BLUR_RADIUS: f32 = 128.0;

const KIND_QUAD: f32 = 0.0;
const KIND_OUTER_SHADOW: f32 = 1.0;
const KIND_INSET_SHADOW: f32 = 2.0;
const KIND_IMAGE: f32 = 3.0;
const KIND_GLYPH: f32 = 4.0;

/// Where a frame is drawn.
pub struct RenderTarget<'a> {
    pub view: &'a wgpu::TextureView,
    pub format: wgpu::TextureFormat,
    /// Physical pixels.
    pub size: (u32, u32),
    /// Physical pixels per logical pixel (the window's scale factor).
    pub scale: f32,
}

/// Counts for the most recent frame (used by tests and future profiling).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RenderStats {
    /// Instanced draws of boxes and shadows.
    pub draw_calls: u32,
    /// Render passes of any kind (including blur, composite and blit).
    pub passes: u32,
    pub layers: u32,
    pub instances: u32,
}

/// Matches `struct Instance` in `quad.wgsl`.
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Instance {
    rect: [f32; 4],
    radii: [f32; 4],
    p0: [f32; 4],
    p1: [f32; 4],
    color0: [f32; 4],
    color1: [f32; 4],
    params: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Globals {
    viewport: [f32; 2],
    scale: f32,
    _pad: f32,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct BlurParams {
    dir: [f32; 2],
    sigma: f32,
    radius: f32,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct CompositeParams {
    clip_rect: [f32; 4],
    clip_radii: [f32; 4],
    opacity: f32,
    scale: f32,
    use_clip: f32,
    _pad: f32,
}

/// A scissor rectangle in physical pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
struct PxRect {
    x: u32,
    y: u32,
    w: u32,
    h: u32,
}

/// One step of a frame, recorded after all instances are known.
enum Step {
    Clear {
        target: usize,
        color: wgpu::Color,
    },
    /// Instanced quads; with `image`, one background image draw using that texture.
    Draw {
        target: usize,
        range: Range<u32>,
        image: Option<String>,
    },
    Copy {
        src: usize,
        dst: usize,
    },
    Blur {
        tex: usize,
        temp: usize,
        area: PxRect,
        sigma: f32,
    },
    Composite {
        src: usize,
        dst: usize,
        area: PxRect,
        opacity: f32,
        clip: Option<(Rect, [f32; 4])>,
    },
}

struct PooledTexture {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    size: (u32, u32),
}

pub struct UiRenderer {
    quad_pipeline: wgpu::RenderPipeline,
    quad_layout: wgpu::BindGroupLayout,
    image_pipeline: wgpu::RenderPipeline,
    image_sampler: wgpu::Sampler,
    images: ImageCache,
    atlas: GlyphAtlas,
    blur_pipeline: wgpu::RenderPipeline,
    blur_layout: wgpu::BindGroupLayout,
    composite_pipeline: wgpu::RenderPipeline,
    composite_layout: wgpu::BindGroupLayout,
    blit_layout: wgpu::BindGroupLayout,
    blit_pipeline_layout: wgpu::PipelineLayout,
    blit_pipelines: HashMap<wgpu::TextureFormat, wgpu::RenderPipeline>,
    effects_module: wgpu::ShaderModule,
    sampler: wgpu::Sampler,
    globals: wgpu::Buffer,
    instances: wgpu::Buffer,
    gradients: wgpu::Buffer,
    pool: Vec<PooledTexture>,
}

impl UiRenderer {
    pub fn new(gpu: &Gpu) -> Self {
        let device = &gpu.device;
        let quad_module = device.create_shader_module(wgpu::include_wgsl!("quad.wgsl"));
        let effects_module = device.create_shader_module(wgpu::include_wgsl!("effects.wgsl"));

        let uniform = |binding, visibility| wgpu::BindGroupLayoutEntry {
            binding,
            visibility,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };
        let texture = wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        };
        let sampler_entry = wgpu::BindGroupLayoutEntry {
            binding: 1,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            count: None,
        };

        let quad_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ui quad"),
            entries: &[
                uniform(0, wgpu::ShaderStages::VERTEX_FRAGMENT),
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    ..texture
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    ..texture
                },
            ],
        });
        let blur_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ui blur"),
            entries: &[
                texture,
                sampler_entry,
                uniform(2, wgpu::ShaderStages::FRAGMENT),
            ],
        });
        let composite_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ui composite"),
            entries: &[
                texture,
                sampler_entry,
                uniform(3, wgpu::ShaderStages::FRAGMENT),
            ],
        });
        let blit_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ui blit"),
            entries: &[texture, sampler_entry],
        });

        let pipeline_layout = |layout: &wgpu::BindGroupLayout| {
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[Some(layout)],
                immediate_size: 0,
            })
        };

        let instance_attributes = wgpu::vertex_attr_array![
            0 => Float32x4, 1 => Float32x4, 2 => Float32x4, 3 => Float32x4,
            4 => Float32x4, 5 => Float32x4, 6 => Float32x4,
        ];
        let premultiplied = wgpu::ColorTargetState {
            format: CANVAS_FORMAT,
            blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
            write_mask: wgpu::ColorWrites::ALL,
        };
        let quad_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("ui quad"),
            layout: Some(&pipeline_layout(&quad_layout)),
            vertex: wgpu::VertexState {
                module: &quad_module,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Instance>() as u64,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &instance_attributes,
                })],
            },
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &quad_module,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(premultiplied.clone())],
            }),
            multiview_mask: None,
            cache: None,
        });

        let image_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("ui image"),
            layout: Some(
                &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: None,
                    bind_group_layouts: &[Some(&quad_layout), Some(&blit_layout)],
                    immediate_size: 0,
                }),
            ),
            vertex: wgpu::VertexState {
                module: &quad_module,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Instance>() as u64,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &instance_attributes,
                })],
            },
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &quad_module,
                entry_point: Some("fs_image"),
                compilation_options: Default::default(),
                targets: &[Some(premultiplied.clone())],
            }),
            multiview_mask: None,
            cache: None,
        });
        let image_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("ui images"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let fullscreen =
            |label, layout: &wgpu::PipelineLayout, entry, target: wgpu::ColorTargetState| {
                device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some(label),
                    layout: Some(layout),
                    vertex: wgpu::VertexState {
                        module: &effects_module,
                        entry_point: Some("vs_fullscreen"),
                        compilation_options: Default::default(),
                        buffers: &[],
                    },
                    primitive: Default::default(),
                    depth_stencil: None,
                    multisample: Default::default(),
                    fragment: Some(wgpu::FragmentState {
                        module: &effects_module,
                        entry_point: Some(entry),
                        compilation_options: Default::default(),
                        targets: &[Some(target)],
                    }),
                    multiview_mask: None,
                    cache: None,
                })
            };
        let replace = wgpu::ColorTargetState {
            format: CANVAS_FORMAT,
            blend: None,
            write_mask: wgpu::ColorWrites::ALL,
        };
        let blur_pipeline = fullscreen(
            "ui blur",
            &pipeline_layout(&blur_layout),
            "fs_blur",
            replace,
        );
        let composite_pipeline = fullscreen(
            "ui composite",
            &pipeline_layout(&composite_layout),
            "fs_composite",
            premultiplied,
        );
        let blit_pipeline_layout = pipeline_layout(&blit_layout);

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("ui"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let globals = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ui globals"),
            size: std::mem::size_of::<Globals>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let instances = Self::buffer(
            gpu,
            "ui instances",
            1024 * std::mem::size_of::<Instance>(),
            wgpu::BufferUsages::VERTEX,
        );
        let gradients = Self::buffer(
            gpu,
            "ui gradients",
            16 * std::mem::size_of::<GpuGradient>(),
            wgpu::BufferUsages::STORAGE,
        );

        Self {
            quad_pipeline,
            quad_layout,
            image_pipeline,
            image_sampler,
            images: ImageCache::default(),
            atlas: GlyphAtlas::new(gpu),
            blur_pipeline,
            blur_layout,
            composite_pipeline,
            composite_layout,
            blit_layout,
            blit_pipeline_layout,
            blit_pipelines: HashMap::new(),
            effects_module,
            sampler,
            globals,
            instances,
            gradients,
            pool: Vec::new(),
        }
    }

    fn buffer(gpu: &Gpu, label: &str, size: usize, usage: wgpu::BufferUsages) -> wgpu::Buffer {
        gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: size as u64,
            usage: usage | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    /// Folder that `url(...)` background images are loaded from (the game folder).
    /// URLs must be relative paths inside it.
    pub fn set_asset_root(&mut self, root: impl Into<PathBuf>) {
        self.images.set_root(root.into());
    }

    /// Draws `cmds` over `clear` into `target`.
    pub fn render(
        &mut self,
        gpu: &Gpu,
        target: &RenderTarget<'_>,
        clear: Color,
        cmds: &[DrawCmd],
    ) -> RenderStats {
        let (size, scale) = (target.size, target.scale);
        let mut stats = RenderStats::default();
        if size.0 == 0 || size.1 == 0 {
            return stats;
        }

        self.atlas.begin_frame();

        // --- Plan: turn commands into instances, gradients and steps. ---
        let mut frame_textures = vec![self.take_texture(gpu, size)];
        let mut instances: Vec<Instance> = Vec::new();
        let mut gradients: Vec<GpuGradient> = Vec::new();
        let mut steps = vec![Step::Clear {
            target: 0,
            color: clear.into(),
        }];
        let mut current = 0usize;
        let mut batch_start = 0u32;
        let mut layer_stack: Vec<(usize, Rect, f32, f32)> = Vec::new();

        let flush = |steps: &mut Vec<Step>,
                     instances: &Vec<Instance>,
                     batch_start: &mut u32,
                     target: usize| {
            let end = instances.len() as u32;
            if end > *batch_start {
                steps.push(Step::Draw {
                    target,
                    range: *batch_start..end,
                    image: None,
                });
                *batch_start = end;
            }
        };
        let to_px = |r: Rect| -> Option<PxRect> {
            let x0 = (r.x * scale).floor().clamp(0.0, size.0 as f32) as u32;
            let y0 = (r.y * scale).floor().clamp(0.0, size.1 as f32) as u32;
            let x1 = ((r.x + r.width) * scale).ceil().clamp(0.0, size.0 as f32) as u32;
            let y1 = ((r.y + r.height) * scale).ceil().clamp(0.0, size.1 as f32) as u32;
            (x1 > x0 && y1 > y0).then_some(PxRect {
                x: x0,
                y: y0,
                w: x1 - x0,
                h: y1 - y0,
            })
        };

        for cmd in cmds {
            match cmd {
                DrawCmd::Quad(q) => {
                    let mut gradient = -1.0;
                    let mut image = None;
                    if let Some(fill) = &q.image {
                        if let Some(g) = gradient::build(&fill.image, q.rect) {
                            gradients.push(g);
                            gradient = (gradients.len() - 1) as f32;
                        } else if let iron_ui::BackgroundImage::Url(url) = &*fill.image
                            && let Some(loaded) =
                                self.images
                                    .get(gpu, &self.blit_layout, &self.image_sampler, url)
                        {
                            let natural = (loaded.size.0 as f32, loaded.size.1 as f32);
                            let tile = images::tile_rect(q.rect, natural, fill.size, fill.position);
                            let (rx, ry) = match fill.repeat {
                                iron_ui::BackgroundRepeat::Repeat => (1.0, 1.0),
                                iron_ui::BackgroundRepeat::RepeatX => (1.0, 0.0),
                                iron_ui::BackgroundRepeat::RepeatY => (0.0, 1.0),
                                iron_ui::BackgroundRepeat::NoRepeat => (0.0, 0.0),
                            };
                            image = Some((url.clone(), tile, rx, ry));
                        }
                    }
                    let quad =
                        |background: [f32; 4], border_color: [f32; 4], gradient: f32| Instance {
                            rect: rect4(q.rect),
                            radii: q.radii,
                            p0: q.border,
                            p1: [0.0; 4],
                            color0: background,
                            color1: border_color,
                            params: [KIND_QUAD, 0.0, gradient, 0.0],
                        };
                    match image {
                        None => instances.push(quad(
                            linear_premultiplied(q.background),
                            linear_premultiplied(q.border_color),
                            gradient,
                        )),
                        // Background colour, then the image (its own draw, since it
                        // needs its texture), then the border on top.
                        Some((url, tile, rx, ry)) => {
                            instances.push(quad(
                                linear_premultiplied(q.background),
                                [0.0; 4],
                                -1.0,
                            ));
                            flush(&mut steps, &instances, &mut batch_start, current);
                            instances.push(Instance {
                                rect: rect4(q.rect),
                                radii: q.radii,
                                p0: rect4(tile),
                                p1: [0.0; 4],
                                color0: [0.0; 4],
                                color1: [0.0; 4],
                                params: [KIND_IMAGE, rx, ry, 0.0],
                            });
                            let end = instances.len() as u32;
                            steps.push(Step::Draw {
                                target: current,
                                range: batch_start..end,
                                image: Some(url),
                            });
                            batch_start = end;
                            instances.push(quad(
                                [0.0; 4],
                                linear_premultiplied(q.border_color),
                                -1.0,
                            ));
                        }
                    }
                }
                DrawCmd::Text(text) => {
                    for (rect, color) in &text.decorations {
                        instances.push(Instance {
                            rect: rect4(*rect),
                            radii: [0.0; 4],
                            p0: [0.0; 4],
                            p1: [0.0; 4],
                            color0: linear_premultiplied(*color),
                            color1: [0.0; 4],
                            params: [KIND_QUAD, 0.0, -1.0, 0.0],
                        });
                    }
                    for run in &text.runs {
                        let color = linear_premultiplied(run.color);
                        let size_px = run.font_size * scale;
                        for g in &run.glyphs {
                            // Snap to whole pixels vertically and to quarter pixels
                            // horizontally (the subpixel offset is baked into the glyph).
                            let px = g.x * scale;
                            let mut x = px.floor();
                            let mut bin = ((px - x) * f32::from(SUBPIXEL_BINS)).round() as u8;
                            if bin == SUBPIXEL_BINS {
                                x += 1.0;
                                bin = 0;
                            }
                            let y = (g.y * scale).round();
                            let Some(e) = self.atlas.glyph(gpu, run, g.id, size_px, bin) else {
                                continue;
                            };
                            let dest_x = x + e.left as f32;
                            let dest_y = y - e.top as f32;
                            instances.push(Instance {
                                rect: [
                                    dest_x / scale,
                                    dest_y / scale,
                                    e.width as f32 / scale,
                                    e.height as f32 / scale,
                                ],
                                radii: [0.0; 4],
                                p0: [e.x as f32, e.y as f32, e.width as f32, e.height as f32],
                                p1: [0.0; 4],
                                color0: color,
                                color1: [0.0; 4],
                                params: [KIND_GLYPH, if e.color { 1.0 } else { 0.0 }, -1.0, 0.0],
                            });
                        }
                    }
                }
                DrawCmd::BoxShadow(s) => {
                    let shape = s.rect.offset(s.offset_x, s.offset_y).expand(s.spread);
                    instances.push(Instance {
                        rect: rect4(clamp_size(shape)),
                        radii: s.radii.map(|r| {
                            if r > 0.0 {
                                (r + s.spread).max(0.0)
                            } else {
                                0.0
                            }
                        }),
                        p0: rect4(s.rect),
                        p1: s.radii,
                        color0: linear_premultiplied(s.color),
                        color1: [0.0; 4],
                        params: [KIND_OUTER_SHADOW, s.blur / 2.0, -1.0, 0.0],
                    });
                }
                DrawCmd::InsetShadow {
                    shadow: s,
                    border: b,
                } => {
                    let [top, right, bottom, left] = *b;
                    let padding = clamp_size(Rect::new(
                        s.rect.x + left,
                        s.rect.y + top,
                        s.rect.width - left - right,
                        s.rect.height - top - bottom,
                    ));
                    let [tl, tr, br, bl] = s.radii;
                    let inner = [
                        (tl - top.max(left)).max(0.0),
                        (tr - top.max(right)).max(0.0),
                        (br - bottom.max(right)).max(0.0),
                        (bl - bottom.max(left)).max(0.0),
                    ];
                    let hole = clamp_size(padding.offset(s.offset_x, s.offset_y).expand(-s.spread));
                    instances.push(Instance {
                        rect: rect4(padding),
                        radii: inner,
                        p0: rect4(hole),
                        p1: inner.map(|r| (r - s.spread).max(0.0)),
                        color0: linear_premultiplied(s.color),
                        color1: [0.0; 4],
                        params: [KIND_INSET_SHADOW, s.blur / 2.0, -1.0, 0.0],
                    });
                }
                DrawCmd::PushLayer {
                    bounds,
                    blur,
                    opacity,
                } => {
                    flush(&mut steps, &instances, &mut batch_start, current);
                    let layer = frame_textures.len();
                    frame_textures.push(self.take_texture(gpu, size));
                    steps.push(Step::Clear {
                        target: layer,
                        color: wgpu::Color::TRANSPARENT,
                    });
                    layer_stack.push((current, *bounds, *blur, *opacity));
                    current = layer;
                    stats.layers += 1;
                }
                DrawCmd::PopLayer => {
                    let Some((parent, bounds, blur, opacity)) = layer_stack.pop() else {
                        tracing::warn!("PopLayer without PushLayer; ignored");
                        continue;
                    };
                    flush(&mut steps, &instances, &mut batch_start, current);
                    if let Some(area) = to_px(bounds) {
                        if blur > 0.0 {
                            let temp = frame_textures.len();
                            frame_textures.push(self.take_texture(gpu, size));
                            steps.push(Step::Blur {
                                tex: current,
                                temp,
                                area,
                                sigma: blur * scale,
                            });
                        }
                        steps.push(Step::Composite {
                            src: current,
                            dst: parent,
                            area,
                            opacity,
                            clip: None,
                        });
                    }
                    current = parent;
                }
                DrawCmd::BackdropBlur { rect, radii, blur } => {
                    flush(&mut steps, &instances, &mut batch_start, current);
                    let (Some(area), Some(blur_area)) =
                        (to_px(*rect), to_px(rect.expand(3.0 * blur)))
                    else {
                        continue;
                    };
                    let copy = frame_textures.len();
                    frame_textures.push(self.take_texture(gpu, size));
                    let temp = frame_textures.len();
                    frame_textures.push(self.take_texture(gpu, size));
                    steps.push(Step::Copy {
                        src: current,
                        dst: copy,
                    });
                    steps.push(Step::Blur {
                        tex: copy,
                        temp,
                        area: blur_area,
                        sigma: blur * scale,
                    });
                    steps.push(Step::Composite {
                        src: copy,
                        dst: current,
                        area,
                        opacity: 1.0,
                        clip: Some((*rect, *radii)),
                    });
                }
            }
        }
        flush(&mut steps, &instances, &mut batch_start, current);
        // Close layers left open by an unbalanced list.
        while let Some((parent, bounds, _, opacity)) = layer_stack.pop() {
            tracing::warn!("PushLayer without PopLayer; closing it");
            if let Some(area) = to_px(bounds) {
                steps.push(Step::Composite {
                    src: current,
                    dst: parent,
                    area,
                    opacity,
                    clip: None,
                });
            }
            current = parent;
        }

        // --- Upload. ---
        gpu.queue.write_buffer(
            &self.globals,
            0,
            bytemuck::bytes_of(&Globals {
                viewport: [size.0 as f32, size.1 as f32],
                scale,
                _pad: 0.0,
            }),
        );
        if !instances.is_empty() {
            let bytes: &[u8] = bytemuck::cast_slice(&instances);
            if self.instances.size() < bytes.len() as u64 {
                self.instances = Self::buffer(
                    gpu,
                    "ui instances",
                    bytes.len().next_power_of_two(),
                    wgpu::BufferUsages::VERTEX,
                );
            }
            gpu.queue.write_buffer(&self.instances, 0, bytes);
        }
        if gradients.is_empty() {
            gradients.push(GpuGradient::default()); // storage bindings can't be empty
        }
        let gradient_bytes: &[u8] = bytemuck::cast_slice(&gradients);
        if self.gradients.size() < gradient_bytes.len() as u64 {
            self.gradients = Self::buffer(
                gpu,
                "ui gradients",
                gradient_bytes.len().next_power_of_two(),
                wgpu::BufferUsages::STORAGE,
            );
        }
        gpu.queue.write_buffer(&self.gradients, 0, gradient_bytes);
        stats.instances = instances.len() as u32;

        // --- Record. ---
        let device = &gpu.device;
        let quad_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ui quad"),
            layout: &self.quad_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.globals.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.gradients.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(self.atlas.mask_view()),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(self.atlas.color_view()),
                },
            ],
        });
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("ui frame"),
        });
        let mut pending_clear: HashMap<usize, wgpu::Color> = HashMap::new();
        let load_for =
            |pending_clear: &mut HashMap<usize, wgpu::Color>, target: usize| match pending_clear
                .remove(&target)
            {
                Some(c) => wgpu::LoadOp::Clear(c),
                None => wgpu::LoadOp::Load,
            };
        let attachment = |view, load| wgpu::RenderPassColorAttachment {
            view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load,
                store: wgpu::StoreOp::Store,
            },
        };
        // Applies any clear still pending on `target` before it's read.
        let settle = |encoder: &mut wgpu::CommandEncoder,
                      pending_clear: &mut HashMap<usize, wgpu::Color>,
                      stats: &mut RenderStats,
                      target: usize| {
            if let Some(c) = pending_clear.remove(&target) {
                encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("ui clear"),
                    color_attachments: &[Some(attachment(
                        &frame_textures[target].view,
                        wgpu::LoadOp::Clear(c),
                    ))],
                    ..Default::default()
                });
                stats.passes += 1;
            }
        };

        for step in &steps {
            match step {
                Step::Clear { target, color } => {
                    pending_clear.insert(*target, *color);
                }
                Step::Draw {
                    target,
                    range,
                    image,
                } => {
                    let image_bind_group = match image {
                        Some(url) => {
                            match self
                                .images
                                .get(gpu, &self.blit_layout, &self.image_sampler, url)
                            {
                                Some(loaded) => Some(&loaded.bind_group),
                                None => continue,
                            }
                        }
                        None => None,
                    };
                    let load = load_for(&mut pending_clear, *target);
                    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("ui quads"),
                        color_attachments: &[Some(attachment(&frame_textures[*target].view, load))],
                        ..Default::default()
                    });
                    match image_bind_group {
                        Some(bind_group) => {
                            pass.set_pipeline(&self.image_pipeline);
                            pass.set_bind_group(1, bind_group, &[]);
                        }
                        None => pass.set_pipeline(&self.quad_pipeline),
                    }
                    pass.set_bind_group(0, &quad_bind_group, &[]);
                    pass.set_vertex_buffer(0, self.instances.slice(..));
                    pass.draw(0..6, range.clone());
                    stats.draw_calls += 1;
                    stats.passes += 1;
                }
                Step::Copy { src, dst } => {
                    settle(&mut encoder, &mut pending_clear, &mut stats, *src);
                    pending_clear.remove(dst);
                    encoder.copy_texture_to_texture(
                        frame_textures[*src].texture.as_image_copy(),
                        frame_textures[*dst].texture.as_image_copy(),
                        frame_textures[*src].texture.size(),
                    );
                }
                Step::Blur {
                    tex,
                    temp,
                    area,
                    sigma,
                } => {
                    settle(&mut encoder, &mut pending_clear, &mut stats, *tex);
                    let radius = (3.0 * sigma).ceil().min(MAX_BLUR_RADIUS);
                    // Horizontal into `temp` (taller than `area`, so the vertical pass has
                    // valid rows to read), then vertical back into `tex`.
                    let r = radius as u32;
                    let tall = PxRect {
                        x: area.x,
                        y: area.y.saturating_sub(r),
                        w: area.w,
                        h: (area.y + area.h + r).min(size.1) - area.y.saturating_sub(r),
                    };
                    for (src, dst, dir, scissor, load) in [
                        (
                            *tex,
                            *temp,
                            [1.0, 0.0],
                            tall,
                            wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        ),
                        (*temp, *tex, [0.0, 1.0], *area, wgpu::LoadOp::Load),
                    ] {
                        let params = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("ui blur params"),
                            contents: bytemuck::bytes_of(&BlurParams {
                                dir,
                                sigma: sigma.max(0.01),
                                radius,
                            }),
                            usage: wgpu::BufferUsages::UNIFORM,
                        });
                        let bind_group = self.texture_bind_group(
                            gpu,
                            &self.blur_layout,
                            &frame_textures[src].view,
                            Some((2, &params)),
                        );
                        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: Some("ui blur"),
                            color_attachments: &[Some(attachment(&frame_textures[dst].view, load))],
                            ..Default::default()
                        });
                        pass.set_pipeline(&self.blur_pipeline);
                        pass.set_bind_group(0, &bind_group, &[]);
                        pass.set_scissor_rect(scissor.x, scissor.y, scissor.w, scissor.h);
                        pass.draw(0..3, 0..1);
                        stats.passes += 1;
                    }
                }
                Step::Composite {
                    src,
                    dst,
                    area,
                    opacity,
                    clip,
                } => {
                    settle(&mut encoder, &mut pending_clear, &mut stats, *src);
                    let (clip_rect, clip_radii, use_clip) = match clip {
                        Some((r, radii)) => (rect4(*r), *radii, 1.0),
                        None => ([0.0; 4], [0.0; 4], 0.0),
                    };
                    let params = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("ui composite params"),
                        contents: bytemuck::bytes_of(&CompositeParams {
                            clip_rect,
                            clip_radii,
                            opacity: *opacity,
                            scale,
                            use_clip,
                            _pad: 0.0,
                        }),
                        usage: wgpu::BufferUsages::UNIFORM,
                    });
                    let bind_group = self.texture_bind_group(
                        gpu,
                        &self.composite_layout,
                        &frame_textures[*src].view,
                        Some((3, &params)),
                    );
                    let load = load_for(&mut pending_clear, *dst);
                    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("ui composite"),
                        color_attachments: &[Some(attachment(&frame_textures[*dst].view, load))],
                        ..Default::default()
                    });
                    pass.set_pipeline(&self.composite_pipeline);
                    pass.set_bind_group(0, &bind_group, &[]);
                    pass.set_scissor_rect(area.x, area.y, area.w, area.h);
                    pass.draw(0..3, 0..1);
                    stats.passes += 1;
                }
            }
        }
        settle(&mut encoder, &mut pending_clear, &mut stats, 0);

        // --- Blit the canvas to the target. ---
        let blit = self.blit_pipeline(gpu, target.format);
        let bind_group =
            self.texture_bind_group(gpu, &self.blit_layout, &frame_textures[0].view, None);
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("ui blit"),
                color_attachments: &[Some(attachment(
                    target.view,
                    wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                ))],
                ..Default::default()
            });
            pass.set_pipeline(&blit);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.draw(0..3, 0..1);
            stats.passes += 1;
        }
        gpu.queue.submit([encoder.finish()]);

        // Commands are submitted in order, so textures can be reused next frame.
        self.pool.extend(frame_textures);
        stats
    }

    fn texture_bind_group(
        &self,
        gpu: &Gpu,
        layout: &wgpu::BindGroupLayout,
        view: &wgpu::TextureView,
        uniform: Option<(u32, &wgpu::Buffer)>,
    ) -> wgpu::BindGroup {
        let mut entries = vec![
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&self.sampler),
            },
        ];
        if let Some((binding, buffer)) = uniform {
            entries.push(wgpu::BindGroupEntry {
                binding,
                resource: buffer.as_entire_binding(),
            });
        }
        gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout,
            entries: &entries,
        })
    }

    fn blit_pipeline(&mut self, gpu: &Gpu, format: wgpu::TextureFormat) -> wgpu::RenderPipeline {
        self.blit_pipelines
            .entry(format)
            .or_insert_with(|| {
                gpu.device
                    .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                        label: Some("ui blit"),
                        layout: Some(&self.blit_pipeline_layout),
                        vertex: wgpu::VertexState {
                            module: &self.effects_module,
                            entry_point: Some("vs_fullscreen"),
                            compilation_options: Default::default(),
                            buffers: &[],
                        },
                        primitive: Default::default(),
                        depth_stencil: None,
                        multisample: Default::default(),
                        fragment: Some(wgpu::FragmentState {
                            module: &self.effects_module,
                            entry_point: Some("fs_blit"),
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
            })
            .clone()
    }

    /// Takes a canvas-format texture of `size` from the pool, or creates one.
    fn take_texture(&mut self, gpu: &Gpu, size: (u32, u32)) -> PooledTexture {
        if let Some(i) = self.pool.iter().position(|t| t.size == size) {
            return self.pool.swap_remove(i);
        }
        // Drop textures of other sizes (the window was resized).
        self.pool.retain(|t| t.size == size);
        let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("ui canvas"),
            size: wgpu::Extent3d {
                width: size.0,
                height: size.1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: CANVAS_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let view = texture.create_view(&Default::default());
        PooledTexture {
            texture,
            view,
            size,
        }
    }
}

fn rect4(r: Rect) -> [f32; 4] {
    [r.x, r.y, r.width, r.height]
}

fn clamp_size(r: Rect) -> Rect {
    Rect::new(r.x, r.y, r.width.max(0.0), r.height.max(0.0))
}

fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// sRGB straight alpha → linear premultiplied, as the shaders expect.
fn linear_premultiplied(c: Rgba) -> [f32; 4] {
    [
        srgb_to_linear(c.r) * c.a,
        srgb_to_linear(c.g) * c.a,
        srgb_to_linear(c.b) * c.a,
        c.a,
    ]
}
