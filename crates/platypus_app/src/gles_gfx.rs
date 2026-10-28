// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

//! The main window drawn by the GLES executor (`platypus_render_gles`) through EGL: ANGLE on
//! Windows, Mesa and friends elsewhere. This is `platypus-app`'s renderer, so a UI-only program
//! loads no Vulkan or wgpu. There is no 3D world, world panels or secondary windows on this
//! path; if EGL (or the windowing system) is unavailable `platypus-app` falls back to wgpu.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context as _, Result};
use platypus_platform::Window;
use platypus_render::UiBackend as _;
use platypus_render_gles::egl::EglContext;
use platypus_render_gles::{GlesRenderer, Target};

use super::App;

/// Frames the loop runs at most per second when the swap interval doesn't pace it (a frame that
/// changed nothing is not swapped, so nothing waits for the display).
pub(crate) const FRAME_CAP: Duration = Duration::from_micros(16_667);

pub(crate) struct GlesGfx {
    pub(crate) window: Arc<Window>,
    // Declared before `context`: the renderer's GL objects go first.
    renderer: GlesRenderer,
    context: EglContext,
    /// Frames and render time since the last report.
    report: (u32, Duration, Instant),
    /// What the window shows now (size, scale, draw lists), so an unchanged frame isn't redrawn.
    shown: Option<Shown>,
    /// Shown instead of the UI once the game asks for the 3D world (built on first use).
    notice: Option<super::notice::Notice>,
}

type Shown = (
    (u32, u32),
    u32,
    Vec<platypus_ui::DrawCmd>,
    Option<Vec<platypus_ui::DrawCmd>>,
);

/// Where ANGLE's DLLs are looked for first (`PLATYPUS_ANGLE_DIR`).
fn angle_dir() -> Option<PathBuf> {
    std::env::var_os("PLATYPUS_ANGLE_DIR").map(PathBuf::from)
}

impl GlesGfx {
    /// A context and renderer on `window`, or why there isn't one.
    fn new(window: Arc<Window>, asset_root: &Path) -> Result<Self, String> {
        let dir = angle_dir();
        let context = EglContext::window(dir.as_deref(), &*window)?;
        context.set_vsync(true);
        let mut renderer = GlesRenderer::new(context.gl())?;
        renderer.set_asset_root(asset_root.to_path_buf());
        Ok(Self {
            window,
            renderer,
            context,
            report: (0, Duration::ZERO, Instant::now()),
            shown: None,
            notice: None,
        })
    }

    fn target(&self, size: (u32, u32), scale: f32) -> Target {
        Target {
            framebuffer: None,
            size,
            scale,
            srgb: false,
        }
    }
}

impl App {
    /// Starts drawing `window` with the GLES executor. `false`: it can't (the caller falls back).
    pub(crate) fn install_gles(&mut self, window: Arc<Window>) -> bool {
        use platypus_script::ScriptRuntime as _;
        let gles = match GlesGfx::new(window.clone(), &self.asset_root) {
            Ok(g) => g,
            Err(e) => {
                tracing::warn!("GLES renderer unavailable: {e}");
                return false;
            }
        };
        tracing::info!("renderer ready (GLES through EGL)");
        let size = window.inner_size();
        let dpi = window.scale_factor();
        self.info.set_system(serde_json::json!({
            "graphicsDeviceName": "GLES (EGL)",
            "graphicsDeviceType": "Gles",
            "graphicsDeviceKind": "Gles",
            "screenWidth": size.width,
            "screenHeight": size.height,
            "screenRefreshRate": 0.0,
            "dpi": dpi * 96.0,
        }));
        self.info.set(
            "display",
            super::display::state(Some(&window), &self.display),
        );
        if let Err(e) = self.scripts.eval(&self.info.script()) {
            tracing::error!("system info: {e}");
        }
        let title = platypus_engine_api::window::state(platypus_engine_api::window::MAIN)
            .map_or_else(String::new, |s| s.title);
        platypus_engine_api::window::set_main(
            &title,
            super::windows::to_ui((size.width, size.height), dpi),
            dpi,
        );
        if title != window.title() {
            window.set_title(&title);
        }
        self.gles = Some(gles);
        true
    }

    /// Tells the UI the size of every image it asked for (layout needs it).
    fn load_gles_images(&mut self) {
        let Some(gles) = &mut self.gles else {
            return;
        };
        for src in self.ui.take_image_requests() {
            let size = gles.renderer.image_size(&(), &src);
            self.ui.set_image_size(&src, size);
        }
    }

    /// The FPS counter's commands: at window resolution and scale, over the UI.
    fn gles_fps_commands(
        &mut self,
        size: (u32, u32),
        dpi: f64,
    ) -> Option<Vec<platypus_ui::DrawCmd>> {
        self.fps_on.then(|| {
            let text = self.fps.text(self.started.elapsed().as_secs_f64());
            self.fps_overlay
                .draw_list(&text, size.0, size.1, dpi as f32)
        })
    }

    /// The UI's draw list, or the "3D not supported" screen when the game uses the world.
    fn gles_ui_commands(&mut self, size: (u32, u32), scale: f32) -> Vec<platypus_ui::DrawCmd> {
        if self.world_view.needs_world()
            && let Some(gles) = &mut self.gles
        {
            return gles
                .notice
                .get_or_insert_with(|| super::notice::Notice::new("3D Not Supported"))
                .draw_list(size.0, size.1, scale);
        }
        self.ui.draw_list()
    }

    /// Draws the UI (and the FPS counter over it) into the window's back buffer.
    fn draw_gles(
        &mut self,
        size: (u32, u32),
        scale: f32,
        dpi: f64,
        cmds: &[platypus_ui::DrawCmd],
        fps_cmds: Option<&[platypus_ui::DrawCmd]>,
    ) -> Result<()> {
        let gles = self.gles.as_mut().context("no renderer")?;
        let target = gles.target(size, scale);
        gles.renderer.render(&target, super::CLEAR, cmds);
        if let Some(fps) = fps_cmds {
            let target = gles.target(size, dpi as f32);
            gles.renderer.render_overlay(&target, fps);
        }
        Ok(())
    }

    /// `--screenshot`: draws the UI once and writes it as a PNG.
    pub(crate) fn save_screenshot_gles(&mut self, path: &Path) -> Result<()> {
        let (dpi, size) = self.main_metrics().context("no window")?;
        let scale = super::ui_factor_for(dpi, size, self.ui_scale);
        self.load_gles_images();
        let cmds = self.gles_ui_commands(size, scale);
        let fps_cmds = self.gles_fps_commands(size, dpi);
        self.draw_gles(size, scale, dpi, &cmds, fps_cmds.as_deref())?;
        let gles = self.gles.as_ref().context("no renderer")?;
        let (w, h) = size;
        let pixels = gles.renderer.read_back(None, size);
        image::save_buffer(path, &pixels, w, h, image::ColorType::Rgba8)?;
        tracing::info!("saved screenshot to {}", path.display());
        Ok(())
    }

    /// Draws one frame of the screen UI and shows it.
    pub(crate) fn render_gles(&mut self, started: Instant) {
        let Some((dpi, size)) = self.main_metrics() else {
            return;
        };
        let scale = super::ui_factor_for(dpi, size, self.ui_scale);
        let Some(gles) = &self.gles else {
            return;
        };
        let dark = gles.window.theme() == Some(platypus_platform::Theme::Dark);
        self.ui.set_media_env(platypus_ui::media::MediaEnv {
            resolution: scale,
            dark,
            ..self.ui.media_env()
        });
        self.load_gles_images();
        let cmds = self.gles_ui_commands(size, scale);
        self.scene.scene.update_world();
        let fps_cmds = self.gles_fps_commands(size, dpi);
        let Some(gles) = &self.gles else {
            return;
        };
        // Nothing changed since the last frame: nothing to draw or swap.
        if gles.shown.as_ref().is_some_and(|(sz, sc, c, f)| {
            *sz == size && *sc == scale.to_bits() && *c == cmds && *f == fps_cmds
        }) {
            // The counter still counts the frame: it shows the loop's pace, not the redraws.
            self.tick_fps();
            return;
        }
        if let Err(e) = self.draw_gles(size, scale, dpi, &cmds, fps_cmds.as_deref()) {
            tracing::error!("GLES draw: {e}");
            return;
        }
        let Some(gles) = &mut self.gles else {
            return;
        };
        if let Err(e) = gles.context.swap_buffers() {
            tracing::error!("{e}");
        }
        gles.shown = Some((size, scale.to_bits(), cmds, fps_cmds));
        self.render_time += started.elapsed();
        gles.report.0 += 1;
        gles.report.1 += started.elapsed();
        let secs = gles.report.2.elapsed().as_secs_f64();
        if secs >= 1.0 {
            tracing::debug!(
                "GLES renderer: {:.1} fps, {:.2} ms per frame (draw + swap)",
                f64::from(gles.report.0) / secs,
                gles.report.1.as_secs_f64() * 1000.0 / f64::from(gles.report.0)
            );
            gles.report = (0, Duration::ZERO, Instant::now());
        }
        self.tick_fps();
    }
}
