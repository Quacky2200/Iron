//! A GLES 3 executor for the UI frame plan (`iron_render::plan`), for `iron-app`: the same
//! draw list and plan as the wgpu renderer, drawn with GLES through EGL (ANGLE on Windows).
//!
//! [`GlesRenderer`] plans a draw list with `plan::plan`, then runs the plan: the instances go
//! in a vertex buffer, the gradient and clip tables in float data textures (GLES 3.0 has no
//! storage buffers), glyph pixels from the shared `GlyphCache` into two page textures, and the
//! `Clear` and `Draw` steps draw into `SRGB8_ALPHA8` canvases, like wgpu's, so blending is in
//! linear light. Not done yet: images, layers (`Copy`, `Blur`, `Composite` steps) and the
//! effects built on them (blur, opacity groups, backdrop filters, masks); each skipped step is
//! logged once.

pub mod egl;
mod shaders;

use std::path::PathBuf;

use glow::HasContext;
use iron_render::glyphs::{COLOR_SIZE, GlyphCache, GlyphEntry, MASK_SIZE};
use iron_render::plan::{self, FramePlan, ImageInfo, Instance, PlanParams, PlanResources, Step};
use iron_render::{Color, RenderStats, UiBackend};
use iron_ui::DrawCmd;

/// Floats per gradient table entry (`GpuGradient`: 12 `vec4`s) and per clip table entry
/// (`GpuClip`: 5), as texels of the data textures.
const GRADIENT_TEXELS: i32 = 12;
const CLIP_TEXELS: i32 = 5;
const INSTANCE_BYTES: i32 = std::mem::size_of::<Instance>() as i32;

/// Where a frame is drawn.
#[derive(Clone, Copy, Debug)]
pub struct Target {
    /// The framebuffer (`None`: the window's default one).
    pub framebuffer: Option<glow::Framebuffer>,
    /// Physical pixels.
    pub size: (u32, u32),
    /// Physical pixels per logical pixel.
    pub scale: f32,
    /// Whether the framebuffer encodes sRGB on write (an `SRGB8_ALPHA8` attachment does; a
    /// window's default framebuffer usually doesn't).
    pub srgb: bool,
}

/// An offscreen texture of the frame's size a step draws into.
struct Canvas {
    texture: glow::Texture,
    framebuffer: glow::Framebuffer,
}

struct Program {
    program: glow::Program,
}

pub struct GlesRenderer {
    gl: glow::Context,
    quad: Program,
    blit: Program,
    vao: glow::VertexArray,
    instances: glow::Buffer,
    clip_table: glow::Texture,
    gradient_table: glow::Texture,
    glyph_mask: glow::Texture,
    glyph_color: glow::Texture,
    canvases: Vec<Canvas>,
    canvas_size: (u32, u32),
    glyphs: GlyphCache,
    asset_root: Option<PathBuf>,
    /// Things already reported as not drawn.
    reported: Vec<&'static str>,
}

/// What planning asks of this renderer: glyphs from the shared cache; no images yet.
struct Resources<'a> {
    glyphs: &'a mut GlyphCache,
    missing_images: bool,
}

impl PlanResources for Resources<'_> {
    fn glyph(
        &mut self,
        run: &iron_ui::GlyphRun,
        glyph: u32,
        size_px: f32,
        subpixel: u8,
        stroke_px: f32,
    ) -> Option<GlyphEntry> {
        self.glyphs.glyph(run, glyph, size_px, subpixel, stroke_px)
    }

    fn image(&mut self, _: &str) -> Option<ImageInfo> {
        self.missing_images = true;
        None
    }

    fn svg_at(&mut self, _: &str, _: (f32, f32)) -> Option<(String, (u32, u32))> {
        None
    }
}

fn compile(gl: &glow::Context, vertex: &str, fragment: &str) -> Result<glow::Program, String> {
    unsafe {
        let program = gl.create_program()?;
        let mut shaders = Vec::new();
        for (kind, source) in [
            (glow::VERTEX_SHADER, vertex),
            (glow::FRAGMENT_SHADER, fragment),
        ] {
            let shader = gl.create_shader(kind)?;
            gl.shader_source(shader, source);
            gl.compile_shader(shader);
            if !gl.get_shader_compile_status(shader) {
                let log = gl.get_shader_info_log(shader);
                return Err(format!("GLSL compile error: {log}"));
            }
            gl.attach_shader(program, shader);
            shaders.push(shader);
        }
        gl.link_program(program);
        if !gl.get_program_link_status(program) {
            return Err(format!(
                "GLSL link error: {}",
                gl.get_program_info_log(program)
            ));
        }
        for shader in shaders {
            gl.detach_shader(program, shader);
            gl.delete_shader(shader);
        }
        Ok(program)
    }
}

/// A page-sized texture the glyph atlas fills.
unsafe fn page(gl: &glow::Context, size: u32, internal: u32) -> Result<glow::Texture, String> {
    unsafe {
        let texture = gl.create_texture()?;
        gl.bind_texture(glow::TEXTURE_2D, Some(texture));
        gl.tex_storage_2d(glow::TEXTURE_2D, 1, internal, size as i32, size as i32);
        for (p, v) in [
            (glow::TEXTURE_MIN_FILTER, glow::NEAREST),
            (glow::TEXTURE_MAG_FILTER, glow::NEAREST),
            (glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE),
            (glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE),
        ] {
            gl.tex_parameter_i32(glow::TEXTURE_2D, p, v as i32);
        }
        Ok(texture)
    }
}

/// An empty float data texture (nearest, clamped), to be sized by the frame.
unsafe fn table(gl: &glow::Context) -> Result<glow::Texture, String> {
    unsafe {
        let texture = gl.create_texture()?;
        gl.bind_texture(glow::TEXTURE_2D, Some(texture));
        for (p, v) in [
            (glow::TEXTURE_MIN_FILTER, glow::NEAREST),
            (glow::TEXTURE_MAG_FILTER, glow::NEAREST),
            (glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE),
            (glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE),
        ] {
            gl.tex_parameter_i32(glow::TEXTURE_2D, p, v as i32);
        }
        Ok(texture)
    }
}

impl GlesRenderer {
    /// A renderer on the current GL context (`gl`, from [`egl::EglContext::gl`]).
    pub fn new(gl: glow::Context) -> Result<Self, String> {
        unsafe {
            let quad = compile(&gl, &shaders::quad_vertex(), &shaders::quad_fragment())?;
            let blit = compile(&gl, shaders::BLIT_VERTEX, shaders::BLIT_FRAGMENT)?;
            let vao = gl.create_vertex_array()?;
            let instances = gl.create_buffer()?;
            let clip_table = table(&gl)?;
            let gradient_table = table(&gl)?;
            let glyph_mask = page(&gl, MASK_SIZE, glow::R8)?;
            let glyph_color = page(&gl, COLOR_SIZE, glow::SRGB8_ALPHA8)?;
            Ok(Self {
                gl,
                quad: Program { program: quad },
                blit: Program { program: blit },
                vao,
                instances,
                clip_table,
                gradient_table,
                glyph_mask,
                glyph_color,
                canvases: Vec::new(),
                canvas_size: (0, 0),
                glyphs: GlyphCache::new(),
                asset_root: None,
                reported: Vec::new(),
            })
        }
    }

    fn report(&mut self, what: &'static str) {
        if !self.reported.contains(&what) {
            self.reported.push(what);
            tracing::warn!("GLES renderer: {what} is not drawn yet");
        }
    }

    /// Draws `cmds` over `clear` into `target`.
    pub fn render(&mut self, target: &Target, clear: Color, cmds: &[DrawCmd]) -> RenderStats {
        if target.size.0 == 0 || target.size.1 == 0 {
            return RenderStats::default();
        }
        self.glyphs.begin_frame();
        let mut resources = Resources {
            glyphs: &mut self.glyphs,
            missing_images: false,
        };
        let frame = plan::plan(
            cmds,
            &PlanParams {
                size: target.size,
                scale: target.scale,
                clear,
                seeded: false,
            },
            &mut resources,
        );
        if resources.missing_images {
            self.report("images");
        }
        self.execute(target, &frame)
    }

    /// (Re)creates `count` canvases of `size`, keeping the ones that fit.
    fn canvases(&mut self, count: usize, size: (u32, u32)) -> Result<(), String> {
        if self.canvas_size != size {
            unsafe {
                for c in self.canvases.drain(..) {
                    self.gl.delete_framebuffer(c.framebuffer);
                    self.gl.delete_texture(c.texture);
                }
            }
            self.canvas_size = size;
        }
        while self.canvases.len() < count {
            unsafe {
                let gl = &self.gl;
                let texture = gl.create_texture()?;
                gl.bind_texture(glow::TEXTURE_2D, Some(texture));
                gl.tex_storage_2d(
                    glow::TEXTURE_2D,
                    1,
                    glow::SRGB8_ALPHA8,
                    size.0 as i32,
                    size.1 as i32,
                );
                for (p, v) in [
                    (glow::TEXTURE_MIN_FILTER, glow::NEAREST),
                    (glow::TEXTURE_MAG_FILTER, glow::NEAREST),
                    (glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE),
                    (glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE),
                ] {
                    gl.tex_parameter_i32(glow::TEXTURE_2D, p, v as i32);
                }
                let framebuffer = gl.create_framebuffer()?;
                gl.bind_framebuffer(glow::FRAMEBUFFER, Some(framebuffer));
                gl.framebuffer_texture_2d(
                    glow::FRAMEBUFFER,
                    glow::COLOR_ATTACHMENT0,
                    glow::TEXTURE_2D,
                    Some(texture),
                    0,
                );
                if gl.check_framebuffer_status(glow::FRAMEBUFFER) != glow::FRAMEBUFFER_COMPLETE {
                    return Err("the canvas framebuffer is incomplete".into());
                }
                self.canvases.push(Canvas {
                    texture,
                    framebuffer,
                });
            }
        }
        Ok(())
    }

    /// Writes the glyph pixels placed since the last frame into the page textures.
    fn upload_glyphs(&mut self) {
        let gl = &self.gl;
        unsafe {
            gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);
            for u in self.glyphs.take_uploads() {
                let (texture, format) = if u.color {
                    (self.glyph_color, glow::RGBA)
                } else {
                    (self.glyph_mask, glow::RED)
                };
                gl.bind_texture(glow::TEXTURE_2D, Some(texture));
                gl.tex_sub_image_2d(
                    glow::TEXTURE_2D,
                    0,
                    u.x as i32,
                    u.y as i32,
                    u.width as i32,
                    u.height as i32,
                    format,
                    glow::UNSIGNED_BYTE,
                    glow::PixelUnpackData::Slice(Some(&u.data)),
                );
            }
        }
    }

    /// Runs a plan: uploads its tables, draws its steps into the canvases and copies canvas 0
    /// to the target.
    pub fn execute(&mut self, target: &Target, frame: &FramePlan) -> RenderStats {
        let mut stats = RenderStats {
            layers: frame.layers,
            instances: frame.instances.len() as u32,
            clips: frame.clips.len() as u32,
            ..RenderStats::default()
        };
        if let Err(e) = self.canvases(frame.textures, target.size) {
            tracing::error!("GLES renderer: {e}");
            return stats;
        }
        self.upload_glyphs();
        let size = target.size;
        let gl = &self.gl;
        unsafe {
            // The instance buffer and the two tables.
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.instances));
            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                bytemuck::cast_slice(&frame.instances),
                glow::DYNAMIC_DRAW,
            );
            let default_gradient = [iron_render::plan::GpuGradient::default()];
            let gradients: &[_] = if frame.gradients.is_empty() {
                &default_gradient
            } else {
                &frame.gradients
            };
            gl.bind_texture(glow::TEXTURE_2D, Some(self.gradient_table));
            gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 4);
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGBA32F as i32,
                GRADIENT_TEXELS,
                gradients.len() as i32,
                0,
                glow::RGBA,
                glow::FLOAT,
                glow::PixelUnpackData::Slice(Some(bytemuck::cast_slice(gradients))),
            );
            let default_clip = [iron_render::plan::GpuClip::default()];
            let clips: &[_] = if frame.clips.is_empty() {
                &default_clip
            } else {
                &frame.clips
            };
            gl.bind_texture(glow::TEXTURE_2D, Some(self.clip_table));
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGBA32F as i32,
                CLIP_TEXELS,
                clips.len() as i32,
                0,
                glow::RGBA,
                glow::FLOAT,
                glow::PixelUnpackData::Slice(Some(bytemuck::cast_slice(clips))),
            );

            // State: premultiplied alpha blending, nothing else.
            gl.disable(glow::DEPTH_TEST);
            gl.disable(glow::CULL_FACE);
            gl.disable(glow::SCISSOR_TEST);
            gl.enable(glow::BLEND);
            gl.blend_func_separate(
                glow::ONE,
                glow::ONE_MINUS_SRC_ALPHA,
                glow::ONE,
                glow::ONE_MINUS_SRC_ALPHA,
            );
            gl.use_program(Some(self.quad.program));
            gl.uniform_4_f32(
                gl.get_uniform_location(self.quad.program, "u_globals")
                    .as_ref(),
                size.0 as f32,
                size.1 as f32,
                target.scale,
                0.0,
            );
            for (name, unit, texture) in [
                ("u_clips", 0, self.clip_table),
                ("u_grads", 1, self.gradient_table),
                ("u_mask", 2, self.glyph_mask),
                ("u_color", 3, self.glyph_color),
            ] {
                gl.active_texture(glow::TEXTURE0 + unit);
                gl.bind_texture(glow::TEXTURE_2D, Some(texture));
                gl.uniform_1_i32(
                    gl.get_uniform_location(self.quad.program, name).as_ref(),
                    unit as i32,
                );
            }
            gl.bind_vertex_array(Some(self.vao));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.instances));
        }

        let mut pending_clear: Vec<Option<Color>> = vec![None; self.canvases.len()];
        for step in &frame.steps {
            match step {
                Step::Clear { target, color } => pending_clear[*target] = Some(*color),
                Step::Draw {
                    target,
                    range,
                    image,
                } => {
                    if image.is_some() {
                        self.report("images");
                        continue;
                    }
                    let gl = &self.gl;
                    unsafe {
                        gl.bind_framebuffer(
                            glow::FRAMEBUFFER,
                            Some(self.canvases[*target].framebuffer),
                        );
                        gl.viewport(0, 0, size.0 as i32, size.1 as i32);
                        if let Some(c) = pending_clear[*target].take() {
                            gl.clear_color(c.r as f32, c.g as f32, c.b as f32, c.a as f32);
                            gl.clear(glow::COLOR_BUFFER_BIT);
                            stats.passes += 1;
                        }
                        // One vec4 attribute per instance field, read from this range.
                        for location in 0..9u32 {
                            gl.enable_vertex_attrib_array(location);
                            gl.vertex_attrib_pointer_f32(
                                location,
                                4,
                                glow::FLOAT,
                                false,
                                INSTANCE_BYTES,
                                range.start as i32 * INSTANCE_BYTES + location as i32 * 16,
                            );
                            gl.vertex_attrib_divisor(location, 1);
                        }
                        gl.draw_arrays_instanced(
                            glow::TRIANGLES,
                            0,
                            6,
                            (range.end - range.start) as i32,
                        );
                    }
                    stats.draw_calls += 1;
                    stats.passes += 1;
                }
                Step::Copy { .. } => self.report("layers (copies)"),
                Step::Blur { .. } => self.report("blur"),
                Step::Composite { .. } => self.report("layers (composites)"),
            }
        }
        let gl = &self.gl;
        unsafe {
            // A canvas nothing drew into still shows its clear.
            if let Some(c) = pending_clear.first_mut().and_then(Option::take) {
                gl.bind_framebuffer(glow::FRAMEBUFFER, Some(self.canvases[0].framebuffer));
                gl.clear_color(c.r as f32, c.g as f32, c.b as f32, c.a as f32);
                gl.clear(glow::COLOR_BUFFER_BIT);
                stats.passes += 1;
            }
            // The canvas, pixel for pixel, onto the target.
            gl.bind_framebuffer(glow::FRAMEBUFFER, target.framebuffer);
            gl.viewport(0, 0, size.0 as i32, size.1 as i32);
            gl.disable(glow::BLEND);
            gl.use_program(Some(self.blit.program));
            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, Some(self.canvases[0].texture));
            gl.uniform_1_i32(
                gl.get_uniform_location(self.blit.program, "u_src").as_ref(),
                0,
            );
            gl.uniform_1_f32(
                gl.get_uniform_location(self.blit.program, "u_encode")
                    .as_ref(),
                if target.srgb { 0.0 } else { 1.0 },
            );
            gl.draw_arrays(glow::TRIANGLES, 0, 3);
            stats.passes += 1;
        }
        stats
    }
}

impl UiBackend for GlesRenderer {
    type Context = ();
    type Target<'a> = Target;

    fn set_asset_root(&mut self, root: PathBuf) {
        self.asset_root = Some(root);
    }

    fn image_size(&mut self, _: &(), _: &str) -> Option<(u32, u32)> {
        None
    }

    fn render(&mut self, _: &(), target: &Target, clear: Color, cmds: &[DrawCmd]) -> RenderStats {
        GlesRenderer::render(self, target, clear, cmds)
    }

    fn render_overlay(&mut self, _: &(), target: &Target, cmds: &[DrawCmd]) -> RenderStats {
        // Over what the target holds: not done yet (the canvas is cleared, then copied).
        GlesRenderer::render(self, target, Color::TRANSPARENT, cmds)
    }
}
