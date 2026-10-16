//! `iron <game> [--screenshot out.png] [--frames N]`: opens a window, loads the
//! game's script bundle and runs it.
//!
//! `<game>` is either a game folder (loads `<game>/dist/game.js`, built with
//! `bun run build`) or a `.js` bundle directly. `--screenshot` saves the UI after
//! `N` frames (default 60) and exits, which is handy for checking output.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context, Result, bail};
use iron_platform::{
    CursorIcon, InputEvent, MouseButton, Platform, PlatformApp, WheelDelta, Window, WindowConfig,
};
use iron_render::wgpu;
use iron_render::{Color, Gpu, OffscreenTarget, RenderTarget, UiRenderer, WindowSurface};
use iron_script::{ComponentHost, QuickJsRuntime, ScriptRuntime};
use iron_ui::{Pointer, UiEvent, UiTree, events};

const TICK: f64 = 1.0 / 60.0;
/// Stops a slow frame from triggering a long burst of catch-up ticks.
const MAX_TICKS_PER_FRAME: u32 = 5;
/// Pixels per wheel notch ("line"), like browsers.
const WHEEL_LINE: f32 = 40.0;
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
    // Set by @iron/react; games without React UI get no UI events.
    let ui_events = scripts
        .eval("typeof __iron_ui_event === 'function'")
        .is_ok_and(|v| v == iron_core::Value::Bool(true));

    // Bun writes CSS imported by the game next to the bundle (game.js -> game.css).
    let mut ui = UiTree::with_system_fonts(1.0, 1.0);
    let css_path = bundle_path.with_extension("css");
    if let Ok(css) = std::fs::read_to_string(&css_path) {
        let base = css_path
            .parent()
            .and_then(|dir| dir.strip_prefix(&asset_root).ok())
            .map(|rel| rel.to_string_lossy().replace('\\', "/"))
            .unwrap_or_default();
        add_stylesheet(
            &mut ui,
            &asset_root,
            &css,
            &base,
            &css_path.display().to_string(),
        );
        tracing::info!("loaded stylesheet {}", css_path.display());
    }

    let app = App {
        scripts,
        components,
        ui,
        sheets: HashMap::new(),
        pointer: Pointer::new(),
        ui_events,
        cursor: "default",
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
    /// Stylesheets added by scripts: script id -> tree id.
    sheets: HashMap<u32, u32>,
    pointer: Pointer,
    /// Whether the game handles UI events (`__iron_ui_event` exists).
    ui_events: bool,
    /// The CSS cursor currently shown.
    cursor: &'static str,
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
                "loadFont" => self.load_font_op(&op.args),
                "addStyleSheet" | "removeStyleSheet" => self.stylesheet_op(&op.name, &op.args),
                other => tracing::warn!("unknown script op `{other}`"),
            }
        }

        // The tree may have moved under a still pointer.
        let events = self.pointer.refresh(&mut self.ui);
        self.dispatch_ui(events);
    }

    /// Sends UI events to scripts, then updates the OS cursor.
    fn dispatch_ui(&mut self, events: Vec<UiEvent>) {
        if self.ui_events {
            for event in &events {
                // The returned "default prevented" flag is used by focus and scrolling (later in M3c).
                if let Err(e) = self
                    .scripts
                    .call_global("__iron_ui_event", &event.script_args())
                {
                    tracing::error!(target: "script", "{} handler: {e}", event.kind.name());
                }
            }
        }
        let cursor = self.pointer.cursor(&self.ui);
        if cursor != self.cursor
            && let Some(gfx) = &self.gfx
        {
            self.cursor = cursor;
            gfx.window.set_cursor_visible(cursor != "none");
            if let Ok(icon) = cursor.parse::<CursorIcon>() {
                gfx.window.set_cursor(icon);
            }
        }
    }

    /// Converts a window position (physical pixels) to the UI's logical pixels.
    fn scale(&self) -> f32 {
        self.gfx
            .as_ref()
            .map_or(1.0, |gfx| gfx.window.scale_factor() as f32)
    }

    fn now_ms(&self) -> f64 {
        self.started.elapsed().as_secs_f64() * 1000.0
    }

    /// `loadFont(family, src, weight, style)` from `@iron/engine`.
    fn load_font_op(&mut self, args: &[iron_core::Value]) {
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
        load_font(&mut self.ui, &self.asset_root, family, src, weight, italic);
    }

    /// `addStyleSheet(id, css)` / `removeStyleSheet(id)` from `@iron/engine`.
    fn stylesheet_op(&mut self, name: &str, args: &[iron_core::Value]) {
        let Some(js_id) = args
            .first()
            .and_then(iron_core::Value::as_f64)
            .map(|n| n as u32)
        else {
            tracing::warn!("{name} needs an id: {args:?}");
            return;
        };
        if name == "removeStyleSheet" {
            if let Some(id) = self.sheets.remove(&js_id) {
                self.ui.remove_stylesheet(id);
            }
            return;
        }
        let css = args.get(1).and_then(iron_core::Value::as_str).unwrap_or("");
        let id = add_stylesheet(&mut self.ui, &self.asset_root, css, "", "addStyleSheet");
        if let Some(old) = self.sheets.insert(js_id, id) {
            self.ui.remove_stylesheet(old);
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
        let scale = self.scale() as f64;
        let now = self.now_ms();
        let ui = &mut self.ui;
        let events = match event {
            InputEvent::Resized { width, height } => {
                if let Some(gfx) = &mut self.gfx {
                    gfx.surface.resize(&gfx.gpu, width, height);
                }
                return;
            }
            InputEvent::MouseMove { x, y } => {
                self.pointer
                    .move_to(ui, (x / scale) as f32, (y / scale) as f32)
            }
            InputEvent::MouseLeave => self.pointer.leave(ui),
            InputEvent::MouseDown { button } => self.pointer.down(ui, dom_button(button), now),
            InputEvent::MouseUp { button } => self.pointer.up(ui, dom_button(button)),
            InputEvent::MouseWheel(delta) => {
                let (dx, dy) = match delta {
                    WheelDelta::Lines { x, y } => (x * WHEEL_LINE, y * WHEEL_LINE),
                    WheelDelta::Pixels { x, y } => ((x / scale) as f32, (y / scale) as f32),
                };
                self.pointer.wheel(ui, dx, dy)
            }
            InputEvent::Modifiers(m) => {
                self.pointer.set_modifiers(events::Modifiers {
                    shift: m.shift,
                    ctrl: m.ctrl,
                    alt: m.alt,
                    meta: m.meta,
                });
                return;
            }
            // Releases outside an unfocused window may never arrive.
            InputEvent::Focused(false) => {
                self.pointer.cancel(ui);
                return;
            }
            _ => return,
        };
        self.dispatch_ui(events);
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

/// The DOM's button number: 0 primary, 1 middle, 2 secondary, 3 back, 4 forward.
fn dom_button(button: MouseButton) -> u8 {
    match button {
        MouseButton::Left => 0,
        MouseButton::Middle => 1,
        MouseButton::Right => 2,
        MouseButton::Other(n) => n.min(u8::MAX as u16) as u8,
    }
}

/// Loads a font file (or `data:` URL) from the game folder and registers it.
fn load_font(
    ui: &mut UiTree,
    root: &Path,
    family: &str,
    src: &str,
    weight: Option<f32>,
    italic: Option<bool>,
) {
    let data = if src.trim_start().starts_with("data:") {
        iron_core::decode_data_url(src).ok_or_else(|| "invalid data URL".to_owned())
    } else {
        match iron_core::resolve_asset(root, src) {
            Some(path) => std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display())),
            None => Err(format!(
                "`{src}` must be a relative path inside the game folder"
            )),
        }
    };
    match data.and_then(|d| ui.register_font(d, Some(family), weight, italic)) {
        Ok(_) => tracing::info!("loaded font `{family}`"),
        Err(e) => tracing::warn!("font `{family}` failed to load: {e}"),
    }
}

/// Adds a stylesheet, logging its warnings and loading its `@font-face` fonts.
/// `base` is the stylesheet's folder inside the game folder (for its `url()`s).
fn add_stylesheet(ui: &mut UiTree, root: &Path, css: &str, base: &str, source: &str) -> u32 {
    let added = ui.add_stylesheet_with_base(css, base);
    for warning in &added.warnings {
        tracing::warn!(target: "css", "{source}: {warning}");
    }
    for face in &added.font_faces {
        load_font(ui, root, &face.family, &face.src, face.weight, face.italic);
    }
    added.id
}
