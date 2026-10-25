//! The main window drawn by the CPU, in a `ui_app_only` build (EXPERIMENTAL): the UI draw list
//! goes through `iron_render_cpu` (vello_cpu) and is shown with softbuffer, so no GPU API (and
//! no GPU driver) is loaded. There is no 3D world, world panels, secondary windows or FPS
//! overlay on this path.
//!
//! Without the feature `CpuGfx` can't exist and the accessors below only look at the GPU path.

use std::sync::Arc;

use iron_platform::Window;

use super::App;

#[cfg(feature = "ui_app_only")]
pub(crate) struct CpuGfx {
    window: Arc<Window>,
    presenter: iron_render_cpu::Presenter<Arc<Window>>,
    renderer: iron_render_cpu::CpuRenderer,
    /// Frames and render time since the last report.
    report: (u32, std::time::Duration, std::time::Instant),
    /// What the window shows now (size, scale, draw lists), so an unchanged frame isn't redrawn.
    shown: Option<Shown>,
}

#[cfg(feature = "ui_app_only")]
type Shown = (
    (u32, u32),
    u32,
    Vec<iron_ui::DrawCmd>,
    Option<Vec<iron_ui::DrawCmd>>,
);

#[cfg(not(feature = "ui_app_only"))]
pub(crate) enum CpuGfx {}

impl CpuGfx {
    fn window(&self) -> &Arc<Window> {
        #[cfg(feature = "ui_app_only")]
        return &self.window;
        #[cfg(not(feature = "ui_app_only"))]
        match *self {}
    }
}

impl App {
    /// The main OS window, whichever renderer draws it.
    pub(crate) fn main_window(&self) -> Option<Arc<Window>> {
        self.gfx
            .as_ref()
            .and_then(|g| g.window.clone())
            .or_else(|| self.cpu.as_ref().map(|c| c.window().clone()))
    }

    /// The main window's DPI scale and client size in physical pixels, once it exists.
    pub(crate) fn main_metrics(&self) -> Option<(f64, (u32, u32))> {
        if let Some(gfx) = &self.gfx {
            return Some((gfx.dpi(), gfx.inner_size()));
        }
        let window = self.cpu.as_ref()?.window();
        let size = window.inner_size();
        Some((window.scale_factor(), (size.width, size.height)))
    }
}

#[cfg(feature = "ui_app_only")]
impl App {
    /// Starts drawing `window` with the CPU renderer.
    pub(crate) fn install_cpu(&mut self, window: Arc<Window>) {
        use iron_script::ScriptRuntime as _;
        let presenter = match iron_render_cpu::Presenter::new(window.clone()) {
            Ok(p) => p,
            Err(e) => {
                tracing::error!("CPU renderer unavailable: {e}");
                return;
            }
        };
        tracing::info!("renderer ready (CPU, vello_cpu)");
        let size = window.inner_size();
        let dpi = window.scale_factor();
        self.info.set_system(serde_json::json!({
            "graphicsDeviceName": "CPU (vello_cpu)",
            "graphicsDeviceType": "Cpu",
            "graphicsDeviceKind": "Cpu",
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
        let title = iron_engine_api::window::state(iron_engine_api::window::MAIN)
            .map_or_else(String::new, |s| s.title);
        iron_engine_api::window::set_main(
            &title,
            super::windows::to_ui((size.width, size.height), dpi),
            dpi,
        );
        if title != window.title() {
            window.set_title(&title);
        }
        let mut renderer = iron_render_cpu::CpuRenderer::new();
        renderer.set_asset_root(&self.asset_root);
        self.cpu = Some(CpuGfx {
            window,
            presenter,
            renderer,
            report: (0, std::time::Duration::ZERO, std::time::Instant::now()),
            shown: None,
        });
    }

    /// `--screenshot`: draws the UI once and writes it as a PNG.
    pub(crate) fn save_screenshot_cpu(&mut self, path: &std::path::Path) -> anyhow::Result<()> {
        use anyhow::Context as _;
        let (dpi, size) = self.main_metrics().context("no window")?;
        let scale = super::ui_factor_for(dpi, size, self.ui_scale);
        self.load_cpu_images();
        let cmds = self.ui.draw_list();
        let fps_cmds = self.fps_commands(size, dpi);
        let cpu = self.cpu.as_mut().context("no renderer")?;
        let clear = iron_ui::Rgba::new(0.05, 0.06, 0.09, 1.0);
        let mut layers: Vec<(f32, &[iron_ui::DrawCmd])> = vec![(scale, &cmds)];
        if let Some(fps) = &fps_cmds {
            layers.push((dpi as f32, fps));
        }
        let pixmap = cpu.renderer.render(size, clear, &layers);
        let rgba: Vec<u8> = pixmap
            .data()
            .iter()
            .flat_map(|p| [p.r, p.g, p.b, p.a])
            .collect();
        image::save_buffer(
            path,
            &rgba,
            u32::from(pixmap.width()),
            u32::from(pixmap.height()),
            image::ColorType::Rgba8,
        )?;
        tracing::info!("saved screenshot to {}", path.display());
        Ok(())
    }

    /// Tells the UI the size of every image it asked for (layout needs it).
    fn load_cpu_images(&mut self) {
        let Some(cpu) = &mut self.cpu else {
            return;
        };
        for src in self.ui.take_image_requests() {
            let size = cpu.renderer.image_size(&src);
            self.ui.set_image_size(&src, size);
        }
    }

    /// The FPS counter's commands: at window resolution and scale, over the UI.
    fn fps_commands(&mut self, size: (u32, u32), dpi: f64) -> Option<Vec<iron_ui::DrawCmd>> {
        self.fps_on.then(|| {
            let text = self.fps.text(self.started.elapsed().as_secs_f64());
            self.fps_overlay
                .draw_list(&text, size.0, size.1, dpi as f32)
        })
    }

    /// Draws one frame of the screen UI.
    pub(crate) fn render_cpu(&mut self, started: std::time::Instant) {
        let Some((dpi, size)) = self.main_metrics() else {
            return;
        };
        let scale = super::ui_factor_for(dpi, size, self.ui_scale);
        let Some(cpu) = &mut self.cpu else {
            return;
        };
        let dark = cpu.window.theme() == Some(iron_platform::Theme::Dark);
        self.ui.set_media_env(iron_ui::media::MediaEnv {
            resolution: scale,
            dark,
            ..self.ui.media_env()
        });
        self.load_cpu_images();
        let cmds = self.ui.draw_list();
        self.scene.scene.update_world();
        let clear = iron_ui::Rgba::new(0.05, 0.06, 0.09, 1.0);
        let fps_cmds = self.fps_commands(size, dpi);
        let Some(cpu) = &mut self.cpu else {
            return;
        };
        // Nothing changed since the last frame: nothing to draw or present.
        if cpu.shown.as_ref().is_some_and(|(sz, sc, c, f)| {
            *sz == size && *sc == scale.to_bits() && *c == cmds && *f == fps_cmds
        }) {
            // The counter still counts the frame: it shows the loop's pace, not the redraws.
            if self.fps_on {
                self.fps.frame(self.started.elapsed().as_secs_f64());
            }
            return;
        }
        let mut layers: Vec<(f32, &[iron_ui::DrawCmd])> = vec![(scale, &cmds)];
        if let Some(fps) = &fps_cmds {
            layers.push((dpi as f32, fps));
        }
        let pixmap = cpu.renderer.render(size, clear, &layers);
        if let Err(e) = cpu.presenter.present(pixmap) {
            tracing::error!("CPU present: {e}");
        }
        cpu.shown = Some((size, scale.to_bits(), cmds, fps_cmds));
        self.render_time += started.elapsed();
        if let Some(cpu) = &mut self.cpu {
            cpu.report.0 += 1;
            cpu.report.1 += started.elapsed();
            let secs = cpu.report.2.elapsed().as_secs_f64();
            if secs >= 1.0 {
                tracing::info!(
                    "CPU renderer: {:.1} fps, {:.2} ms per frame (draw + present)",
                    f64::from(cpu.report.0) / secs,
                    cpu.report.1.as_secs_f64() * 1000.0 / f64::from(cpu.report.0)
                );
                cpu.report = (0, std::time::Duration::ZERO, std::time::Instant::now());
            }
        }
        if self.fps_on {
            self.fps.frame(self.started.elapsed().as_secs_f64());
        }
    }
}
