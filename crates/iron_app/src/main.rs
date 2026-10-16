//! `iron <game> [--screenshot out.png] [--frames N]`: opens a window, loads the
//! game's script bundle and runs it.
//!
//! `<game>` is either a game folder (loads `<game>/dist/game.js`, built with
//! `bun run build`) or a `.js` bundle directly. `--screenshot` saves the UI after
//! `N` frames (default 60) and exits, which is handy for checking output.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context, Result, bail};
use iron_platform::{InputEvent, Platform, PlatformApp, Window, WindowConfig};
use iron_render::wgpu;
use iron_render::{Color, Gpu, OffscreenTarget, RenderTarget, UiRenderer, WindowSurface};
use iron_script::{ComponentHost, QuickJsRuntime, ScriptRuntime};
use iron_ui::UiTree;

const TICK: f64 = 1.0 / 60.0;
/// Stops a slow frame from triggering a long burst of catch-up ticks.
const MAX_TICKS_PER_FRAME: u32 = 5;
const CLEAR: Color = Color::rgb(0.05, 0.06, 0.09);

struct Args {
    game: PathBuf,
    screenshot: Option<PathBuf>,
    frames: u32,
}

fn parse_args() -> Result<Args> {
    let usage = "usage: iron <game folder | bundle.js> [--screenshot out.png] [--frames N]";
    let mut args = std::env::args_os().skip(1);
    let mut parsed = Args {
        game: args.next().context(usage)?.into(),
        screenshot: None,
        frames: 60,
    };
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("--screenshot") => parsed.screenshot = Some(args.next().context(usage)?.into()),
            Some("--frames") => {
                parsed.frames = args
                    .next()
                    .and_then(|n| n.to_str()?.parse().ok())
                    .context("--frames needs a number")?
            }
            _ => bail!("unexpected argument {arg:?}\n{usage}"),
        }
    }
    Ok(parsed)
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,wgpu_core=warn,wgpu_hal=error".into()),
        )
        .init();

    let args = parse_args()?;
    let (bundle_path, asset_root) = resolve_bundle(&args.game)?;
    let code = std::fs::read_to_string(&bundle_path)
        .with_context(|| format!("reading {}", bundle_path.display()))?;
    let name = bundle_path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "game.js".into());

    let mut scripts = QuickJsRuntime::new();
    scripts
        .load_bundle(&name, &code)
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    // No scene system yet (M5): spawn one instance of every registered component.
    let mut components = ComponentHost::new();
    let classes =
        ComponentHost::registered_classes(&mut scripts).map_err(|e| anyhow::anyhow!("{e}"))?;
    for class in &classes {
        components
            .spawn(&mut scripts, class)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
    }
    tracing::info!(?classes, "loaded {}", bundle_path.display());

    let app = App {
        scripts,
        components,
        ui: UiTree::with_system_fonts(1.0, 1.0),
        asset_root,
        gfx: None,
        started: Instant::now(),
        last_frame: None,
        accumulator: 0.0,
        frames: 0,
        screenshot: args.screenshot.map(|path| (path, args.frames)),
        done: false,
    };
    Platform::new().run(
        WindowConfig {
            title: format!("Iron — {name}"),
            ..Default::default()
        },
        app,
    )?;
    Ok(())
}

/// The bundle to load and the folder its assets (`url(...)` images) live in.
fn resolve_bundle(path: &Path) -> Result<(PathBuf, PathBuf)> {
    if path.is_dir() {
        let bundle = path.join("dist").join("game.js");
        if !bundle.is_file() {
            bail!(
                "{} not found; build it first with `bun run build` in {}",
                bundle.display(),
                path.display()
            );
        }
        return Ok((bundle, path.to_path_buf()));
    }
    if path.is_file() {
        let root = path.parent().unwrap_or(Path::new(".")).to_path_buf();
        return Ok((path.to_path_buf(), root));
    }
    bail!("{} is not a game folder or bundle", path.display())
}

struct Gfx {
    gpu: Gpu,
    surface: WindowSurface,
    window: Arc<Window>,
    renderer: UiRenderer,
}

struct App {
    scripts: QuickJsRuntime,
    components: ComponentHost,
    ui: UiTree,
    asset_root: PathBuf,
    gfx: Option<Gfx>,
    started: Instant,
    last_frame: Option<Instant>,
    accumulator: f64,
    frames: u32,
    screenshot: Option<(PathBuf, u32)>,
    done: bool,
}

impl App {
    fn tick(&mut self) {
        let now = Instant::now();
        let elapsed = self
            .last_frame
            .map(|t| now.duration_since(t).as_secs_f64())
            .unwrap_or(TICK);
        self.last_frame = Some(now);

        self.accumulator += elapsed;
        let mut ticks = 0;
        while self.accumulator >= TICK && ticks < MAX_TICKS_PER_FRAME {
            self.components.update(&mut self.scripts, TICK);
            self.accumulator -= TICK;
            ticks += 1;
        }
        if ticks == MAX_TICKS_PER_FRAME {
            self.accumulator = 0.0;
        }

        // Script time: timers and their microtasks, once per frame.
        let now_ms = now.duration_since(self.started).as_secs_f64() * 1000.0;
        for e in self.scripts.tick(now_ms) {
            tracing::error!(target: "script", "uncaught: {e}");
        }

        // React commits update the UI tree; generic ops have no consumer yet.
        for commit in self.scripts.drain_commits() {
            for e in self.ui.apply_commit(&commit) {
                tracing::warn!(target: "ui", "{e}");
            }
        }
        for op in self.scripts.drain_ops() {
            match op.name.as_str() {
                "loadFont" => self.load_font(&op.args),
                other => tracing::warn!("unknown script op `{other}`"),
            }
        }
    }

    /// `loadFont(family, src, weight, style)` from `@iron/engine`.
    fn load_font(&mut self, args: &[iron_core::Value]) {
        use iron_core::Value;
        let (Some(family), Some(src)) = (
            args.first().and_then(Value::as_str),
            args.get(1).and_then(Value::as_str),
        ) else {
            tracing::warn!("loadFont needs a family and a src: {args:?}");
            return;
        };
        let weight = match args.get(2) {
            Some(Value::Number(n)) => Some(*n as f32),
            Some(Value::String(s)) if s == "bold" => Some(700.0),
            Some(Value::String(s)) if s == "normal" => Some(400.0),
            _ => None,
        };
        let italic = args.get(3).and_then(Value::as_str).map(|s| s == "italic");
        let Some(path) = iron_core::resolve_asset(&self.asset_root, src) else {
            tracing::warn!(
                "font `{src}` rejected: it must be a relative path inside the game folder"
            );
            return;
        };
        let result = std::fs::read(&path)
            .map_err(|e| e.to_string())
            .and_then(|data| self.ui.register_font(data, Some(family), weight, italic));
        match result {
            Ok(_) => tracing::info!("loaded font `{family}` from {}", path.display()),
            Err(e) => tracing::warn!(
                "font `{family}` from {} failed to load: {e}",
                path.display()
            ),
        }
    }

    fn save_screenshot(&mut self, path: &Path) -> Result<()> {
        let gfx = self.gfx.as_mut().context("no renderer")?;
        let (w, h) = gfx.surface.size();
        let target =
            OffscreenTarget::with_format(&gfx.gpu, w, h, wgpu::TextureFormat::Rgba8UnormSrgb);
        let view = target.view().context("window has no size")?;
        let cmds = self.ui.draw_list();
        gfx.renderer.render(
            &gfx.gpu,
            &RenderTarget {
                view: &view,
                format: target.format(),
                size: (w, h),
                scale: gfx.window.scale_factor() as f32,
            },
            CLEAR,
            &cmds,
        );
        let pixels = target.read_pixels(&gfx.gpu)?;
        iron_render::testing::write_png(path, w, h, &pixels)?;
        Ok(())
    }
}

impl PlatformApp for App {
    fn window_ready(&mut self, window: Arc<Window>) {
        let size = window.inner_size();
        match Gpu::with_surface(window.clone(), size.width, size.height) {
            Ok((gpu, surface)) => {
                tracing::info!(adapter = %gpu.adapter_name(), "renderer ready");
                let mut renderer = UiRenderer::new(&gpu);
                renderer.set_asset_root(&self.asset_root);
                self.gfx = Some(Gfx {
                    gpu,
                    surface,
                    window,
                    renderer,
                });
            }
            Err(e) => tracing::error!("renderer unavailable: {e}"),
        }
    }

    fn input(&mut self, event: InputEvent) {
        if let InputEvent::Resized { width, height } = event
            && let Some(gfx) = &mut self.gfx
        {
            gfx.surface.resize(&gfx.gpu, width, height);
        }
    }

    fn frame(&mut self) {
        self.tick();
        let Some(gfx) = &mut self.gfx else {
            return;
        };

        // The UI lays out in logical pixels: the window size divided by its scale.
        let scale = gfx.window.scale_factor() as f32;
        let size = gfx.window.inner_size();
        self.ui
            .set_size(size.width as f32 / scale, size.height as f32 / scale);
        let cmds = self.ui.draw_list();
        let Gfx {
            gpu,
            surface,
            renderer,
            ..
        } = gfx;
        surface.present_with(gpu, |view, format, size| {
            renderer.render(
                gpu,
                &RenderTarget {
                    view,
                    format,
                    size,
                    scale,
                },
                CLEAR,
                &cmds,
            );
        });

        self.frames += 1;
        if let Some((path, frames)) = self.screenshot.clone()
            && self.frames >= frames
        {
            match self.save_screenshot(&path) {
                Ok(()) => tracing::info!("saved screenshot to {}", path.display()),
                Err(e) => tracing::error!("screenshot failed: {e}"),
            }
            self.done = true;
        }
    }

    fn should_exit(&self) -> bool {
        self.done
    }
}
