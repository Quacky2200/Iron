//! `iron <game>`: opens a window, loads the game's script bundle and runs it.
//!
//! `<game>` is either a game folder (loads `<game>/dist/game.js`, built with
//! `bun run build`) or a `.js` bundle directly.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context, Result, bail};
use iron_platform::{InputEvent, Platform, PlatformApp, Window, WindowConfig};
use iron_render::{Color, Gpu, WindowSurface};
use iron_script::{ComponentHost, QuickJsRuntime, ScriptRuntime};

const TICK: f64 = 1.0 / 60.0;
/// Stops a slow frame from triggering a long burst of catch-up ticks.
const MAX_TICKS_PER_FRAME: u32 = 5;
const CLEAR: Color = Color::rgb(0.05, 0.06, 0.09);

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,wgpu_core=warn,wgpu_hal=error".into()),
        )
        .init();

    let arg = std::env::args_os()
        .nth(1)
        .context("usage: iron <game folder | bundle.js>")?;
    let bundle_path = resolve_bundle(Path::new(&arg))?;
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
    if classes.is_empty() {
        tracing::warn!("the bundle registered no components");
    }
    for class in &classes {
        components
            .spawn(&mut scripts, class)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
    }
    tracing::info!(?classes, "loaded {}", bundle_path.display());

    let app = App {
        scripts,
        components,
        gpu: None,
        last_frame: None,
        accumulator: 0.0,
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

fn resolve_bundle(path: &Path) -> Result<PathBuf> {
    if path.is_dir() {
        let bundle = path.join("dist").join("game.js");
        if !bundle.is_file() {
            bail!(
                "{} not found; build it first with `bun run build` in {}",
                bundle.display(),
                path.display()
            );
        }
        return Ok(bundle);
    }
    if path.is_file() {
        return Ok(path.to_path_buf());
    }
    bail!("{} is not a game folder or bundle", path.display())
}

struct App {
    scripts: QuickJsRuntime,
    components: ComponentHost,
    gpu: Option<(Gpu, WindowSurface, Arc<Window>)>,
    last_frame: Option<Instant>,
    accumulator: f64,
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

        // Ops have no consumers until M2 (UI); drain so they don't pile up.
        let ops = self.scripts.drain_ops();
        if !ops.is_empty() {
            tracing::trace!(count = ops.len(), "dropped script ops");
        }
    }
}

impl PlatformApp for App {
    fn window_ready(&mut self, window: Arc<Window>) {
        let size = window.inner_size();
        match Gpu::with_surface(window.clone(), size.width, size.height) {
            Ok((gpu, surface)) => {
                tracing::info!(adapter = %gpu.adapter_name(), "renderer ready");
                self.gpu = Some((gpu, surface, window));
            }
            Err(e) => tracing::error!("renderer unavailable: {e}"),
        }
    }

    fn input(&mut self, event: InputEvent) {
        if let InputEvent::Resized { width, height } = event
            && let Some((gpu, surface, _)) = &mut self.gpu
        {
            surface.resize(gpu, width, height);
        }
    }

    fn frame(&mut self) {
        self.tick();
        if let Some((gpu, surface, _)) = &mut self.gpu {
            surface.present_clear(gpu, CLEAR);
        }
    }
}
