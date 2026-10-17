//! `iron <game> [--screenshot out.png] [--frames N]`: opens a window, loads the
//! game's script bundle and runs it.
//!
//! `<game>` is either a game folder (loads `<game>/dist/game.js`, built with
//! `bun run build`) or a `.js` bundle directly. `--screenshot` saves the UI after
//! `N` frames (default 60) and exits, which is handy for checking output.

mod cursor;
mod game_input;
mod ui_scale;

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context, Result, bail};
use iron_platform::{
    CursorIcon, CustomCursorRequest, ImeEvent, InputEvent, MouseButton, Platform, PlatformApp,
    WheelDelta, Window, WindowConfig,
};
use iron_render::wgpu;
use iron_render::{Color, Gpu, OffscreenTarget, RenderTarget, UiRenderer, WindowSurface};
use iron_script::{ComponentHost, QuickJsRuntime, ScriptRuntime};
use iron_ui::{Clipboard, Input, Interaction, UiEvent, UiTree, events};

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
    // Set by @iron/engine's `input` module.
    let game_input = scripts
        .eval("typeof __iron_input === 'function'")
        .is_ok_and(|v| v == iron_core::Value::Bool(true));
    let audio_events = scripts
        .eval("typeof __iron_audio === 'function'")
        .is_ok_and(|v| v == iron_core::Value::Bool(true));
    let physics_events = scripts
        .eval("typeof __iron_physics === 'function'")
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
        interaction: Interaction::new(),
        ui_events,
        game_input,
        physics: iron_physics::Physics::new(),
        audio: None,
        audio_events,
        physics_events,
        nav: game_input::GamepadNav::default(),
        cursor: ("default", Vec::new(), 1.0),
        ui_scale: ui_scale::UiScale::None,
        custom_cursor: None,
        cursors_loaded: HashSet::new(),
        cursors_failed: HashSet::new(),
        clipboard: None,
        ime_allowed: false,
        ime_area: None,
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
    interaction: Interaction,
    /// Whether the game handles UI events (`__iron_ui_event` exists).
    ui_events: bool,
    /// Whether the game reads input (`__iron_input` exists).
    game_input: bool,
    /// The physics world scripts drive with `physics.*` ops.
    physics: iron_physics::Physics,
    /// Whether the game takes physics results (`__iron_physics` exists).
    physics_events: bool,
    /// The sound mixer, opened by the first `audio.*` op.
    audio: Option<Box<dyn iron_audio::Mixer>>,
    /// Whether the game takes finished-sound events (`__iron_audio` exists).
    audio_events: bool,
    nav: game_input::GamepadNav,
    /// The CSS cursor currently shown: keyword, images, and the scale they were for.
    cursor: (&'static str, Vec<iron_ui::CursorImage>, f64),
    /// How screen UI is scaled to the window (`setUiScale`).
    ui_scale: ui_scale::UiScale,
    /// A custom cursor for the platform to show.
    custom_cursor: Option<CustomCursorRequest>,
    /// Cursor images (by `url@scale`) already sent to the platform, or unusable.
    cursors_loaded: HashSet<String>,
    cursors_failed: HashSet<String>,
    clipboard: Option<arboard::Clipboard>,
    /// Whether the window currently accepts IME input, and the last candidate area.
    ime_allowed: bool,
    ime_area: Option<iron_ui::Rect>,
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
            // What the components asked for applies before physics steps.
            self.apply_script_output();
            self.step_physics();
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

        self.apply_script_output();
        self.update_audio(elapsed);

        // The tree may have moved under a still pointer, or the focused element gone.
        self.handle_ui(Input::Refresh);
    }

    /// One physics tick, then its results to scripts (`__iron_physics`).
    fn step_physics(&mut self) {
        self.physics.step();
        let batch = self.physics.take_batch();
        if !self.physics_events || batch.is_empty() {
            return;
        }
        if let Err(e) = self.scripts.call_global("__iron_physics", &batch.to_args()) {
            tracing::error!(target: "script", "physics events: {e}");
        }
        // Collision handlers may have queued more ops; they apply next tick.
    }

    /// Lets audio progress and tells scripts which sounds finished (`__iron_audio`).
    fn update_audio(&mut self, elapsed: f64) {
        let Some(mixer) = self.audio.as_mut() else {
            return;
        };
        mixer.advance(elapsed);
        let ended = mixer.poll();
        if ended.is_empty() || !self.audio_events {
            return;
        }
        let ended = iron_core::Value::Array(ended.into_iter().map(Into::into).collect());
        if let Err(e) = self.scripts.call_global("__iron_audio", &[ended]) {
            tracing::error!(target: "script", "audio events: {e}");
        }
    }

    /// Runs one `audio.*` op, opening the mixer on first use (games without sound never
    /// open a device). Screenshot runs mix offline, silently.
    fn audio_op(&mut self, name: &str, args: &[iron_core::Value]) {
        let offline = self.screenshot.is_some();
        let mixer = self
            .audio
            .get_or_insert_with(|| iron_audio::open_mixer(offline));
        let result = if name == "load" {
            load_sound(mixer.as_mut(), &self.asset_root, args)
        } else {
            mixer.apply(name, args)
        };
        if let Err(e) = result {
            tracing::warn!("audio.{name}: {e}");
        }
    }

    /// Applies React commits, then the ops scripts queued, in order.
    fn apply_script_output(&mut self) {
        for commit in self.scripts.drain_commits() {
            for e in self.ui.apply_commit(&commit) {
                tracing::warn!(target: "ui", "{e}");
            }
        }
        for op in self.scripts.drain_ops() {
            match op.name.as_str() {
                "loadFont" => self.load_font_op(&op.args),
                "uiScale" => match ui_scale::UiScale::from_op(&op.args) {
                    Some(s) => self.ui_scale = s,
                    None => tracing::warn!("uiScale: invalid arguments {:?}", op.args),
                },
                "addStyleSheet" | "removeStyleSheet" => self.stylesheet_op(&op.name, &op.args),
                "focus" | "blur" | "setPointerCapture" | "releasePointerCapture" => {
                    let Some(id) = op.args.first().and_then(iron_core::Value::as_f64) else {
                        tracing::warn!("{} needs an element id", op.name);
                        continue;
                    };
                    let id = iron_core::NodeId(id as u32);
                    self.handle_ui(match op.name.as_str() {
                        "focus" => Input::Focus(id),
                        "blur" => Input::Blur(id),
                        "setPointerCapture" => Input::SetPointerCapture(id),
                        _ => Input::ReleasePointerCapture(id),
                    });
                }
                "scrollTo" | "scrollBy" | "scrollIntoView" => {
                    let num = |i: usize| op.args.get(i).and_then(iron_core::Value::as_f64);
                    let Some(id) = num(0).map(|n| iron_core::NodeId(n as u32)) else {
                        tracing::warn!("{} needs an element id", op.name);
                        continue;
                    };
                    let (a, b) = (num(1).unwrap_or(0.0) as f32, num(2).unwrap_or(0.0) as f32);
                    self.handle_ui(match op.name.as_str() {
                        "scrollTo" => Input::ScrollTo { id, x: a, y: b },
                        "scrollBy" => Input::ScrollBy { id, dx: a, dy: b },
                        _ => Input::ScrollIntoView(id),
                    });
                }
                name if name.starts_with("audio.") => {
                    self.audio_op(&name["audio.".len()..], &op.args)
                }
                name if name.starts_with("physics.") => {
                    if let Err(e) = self.physics.apply(&name["physics.".len()..], &op.args) {
                        tracing::warn!("{name}: {e}");
                    }
                }
                other => tracing::warn!("unknown script op `{other}`"),
            }
        }
    }

    /// Runs one UI input, sending its events to scripts, then updates the OS cursor.
    fn handle_ui(&mut self, input: Input) {
        let (scripts, ui_events) = (&mut self.scripts, self.ui_events);
        self.interaction.handle(&mut self.ui, input, &mut |event| {
            dispatch(scripts, ui_events, event)
        });
        match self.interaction.take_clipboard() {
            Some(Clipboard::Copy(text)) => {
                if let Some(Err(e)) = self.clipboard().map(|c| c.set_text(text)) {
                    tracing::warn!("copy failed: {e}");
                }
            }
            Some(Clipboard::Paste) => {
                if let Some(text) = self.clipboard().and_then(|c| c.get_text().ok()) {
                    self.handle_ui(Input::Paste(text));
                }
            }
            None => {}
        }
        self.update_ime();
        self.update_cursor();
    }

    /// Shows the hovered element's cursor: its first image that loads, else its keyword.
    fn update_cursor(&mut self) {
        let Some(gfx) = &self.gfx else {
            return;
        };
        let (keyword, images) = self.interaction.cursor_with_images(&self.ui);
        let scale = gfx.window.scale_factor();
        if (keyword, images, scale) == (self.cursor.0, &self.cursor.1[..], self.cursor.2) {
            return;
        }
        self.cursor = (keyword, images.to_vec(), scale);
        let custom = self.cursor.1.iter().find_map(|image| {
            let key = format!("{}@{scale}", image.url);
            if self.cursors_loaded.contains(&key) {
                return Some(CustomCursorRequest { key, image: None });
            }
            if self.cursors_failed.contains(&key) {
                return None;
            }
            let decoded = cursor::read(&self.asset_root, &image.url)
                .and_then(|bytes| cursor::decode(&bytes, image.hotspot, scale));
            match decoded {
                Some(rgba) => {
                    self.cursors_loaded.insert(key.clone());
                    Some(CustomCursorRequest {
                        key,
                        image: Some(rgba),
                    })
                }
                None => {
                    tracing::warn!(
                        "cursor image `{}` can't be used (missing, undecodable or over {}px)",
                        image.url.chars().take(80).collect::<String>(),
                        cursor::MAX_CURSOR_SIZE
                    );
                    self.cursors_failed.insert(key);
                    None
                }
            }
        });
        match custom {
            Some(request) => {
                gfx.window.set_cursor_visible(true);
                self.custom_cursor = Some(request);
            }
            None => {
                gfx.window.set_cursor_visible(keyword != "none");
                if let Ok(icon) = keyword.parse::<CursorIcon>() {
                    gfx.window.set_cursor(icon);
                }
            }
        }
    }

    /// Allows IME input while an editable field has focus, and keeps the candidate
    /// window next to its caret.
    fn update_ime(&mut self) {
        let Some(gfx) = &self.gfx else {
            return;
        };
        let area = self.interaction.ime_area(&self.ui);
        if area.is_some() != self.ime_allowed {
            self.ime_allowed = area.is_some();
            gfx.window.set_ime_allowed(self.ime_allowed);
            self.ime_area = None;
        }
        if let Some(a) = area
            && Some(a) != self.ime_area
        {
            self.ime_area = Some(a);
            // UI pixels → the window's logical pixels.
            let z = ui_factor(&gfx.window, self.ui_scale) / gfx.window.scale_factor() as f32;
            gfx.window.set_ime_cursor_area(
                iron_platform::dpi::LogicalPosition::new(a.x * z, a.y * z),
                iron_platform::dpi::LogicalSize::new(
                    (a.width * z).max(1.0),
                    (a.height * z).max(1.0),
                ),
            );
        }
    }

    /// Sends an input event to the game's `input` API (@iron/engine).
    fn send_game_input(&mut self, args: &[iron_core::Value]) {
        if !self.game_input {
            return;
        }
        if let Err(e) = self.scripts.call_global("__iron_input", args) {
            tracing::error!(target: "script", "input: {e}");
        }
    }

    /// The system clipboard, opened on first use (it can be unavailable, e.g. headless).
    fn clipboard(&mut self) -> Option<&mut arboard::Clipboard> {
        if self.clipboard.is_none() {
            match arboard::Clipboard::new() {
                Ok(c) => self.clipboard = Some(c),
                Err(e) => tracing::warn!("clipboard unavailable: {e}"),
            }
        }
        self.clipboard.as_mut()
    }

    /// Physical pixels per UI pixel: the window's DPI scale times the UI scale.
    fn scale(&self) -> f32 {
        self.gfx
            .as_ref()
            .map_or(1.0, |gfx| ui_factor(&gfx.window, self.ui_scale))
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
        load_images(&mut self.ui, &mut gfx.renderer, &gfx.gpu);
        let cmds = self.ui.draw_list();
        gfx.renderer.render(
            &gfx.gpu,
            &RenderTarget {
                view: &view,
                format: target.format(),
                size: (w, h),
                scale: ui_factor(&gfx.window, self.ui_scale),
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
    fn take_custom_cursor(&mut self) -> Option<CustomCursorRequest> {
        self.custom_cursor.take()
    }

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
        if let InputEvent::Gamepad { id, event } = &event {
            self.send_game_input(&game_input::script_args(*id, event));
            // The d-pad moves UI focus only once something has it.
            let nav = self.nav.ui_input(*id, event);
            if let Some(nav) = nav.filter(|_| self.interaction.focused().is_some()) {
                self.handle_ui(nav);
            }
            return;
        }
        let game_key: Option<Vec<iron_core::Value>> = match &event {
            InputEvent::KeyDown {
                code, key, repeat, ..
            } => Some(vec![
                "keydown".into(),
                code.as_str().into(),
                key.as_str().into(),
                (*repeat).into(),
            ]),
            InputEvent::KeyUp { code, key } => Some(vec![
                "keyup".into(),
                code.as_str().into(),
                key.as_str().into(),
            ]),
            InputEvent::Focused(false) => Some(vec!["blur".into()]),
            _ => None,
        };
        let scale = self.scale() as f64;
        let input = match event {
            InputEvent::Resized { width, height } => {
                if let Some(gfx) = &mut self.gfx {
                    gfx.surface.resize(&gfx.gpu, width, height);
                }
                return;
            }
            InputEvent::MouseMove { x, y } => Input::PointerMove {
                x: (x / scale) as f32,
                y: (y / scale) as f32,
            },
            InputEvent::MouseLeave => Input::PointerLeave,
            InputEvent::MouseDown { button } => Input::PointerDown {
                button: dom_button(button),
                time_ms: self.now_ms(),
            },
            InputEvent::MouseUp { button } => Input::PointerUp {
                button: dom_button(button),
            },
            InputEvent::MouseWheel(delta) => {
                let (dx, dy) = match delta {
                    WheelDelta::Lines { x, y } => (x * WHEEL_LINE, y * WHEEL_LINE),
                    WheelDelta::Pixels { x, y } => ((x / scale) as f32, (y / scale) as f32),
                };
                Input::Wheel { dx, dy }
            }
            InputEvent::KeyDown {
                code,
                key,
                repeat,
                text,
            } => Input::KeyDown {
                key,
                code,
                repeat,
                text,
            },
            InputEvent::KeyUp { code, key } => Input::KeyUp { key, code },
            InputEvent::Modifiers(m) => Input::Modifiers(events::Modifiers {
                shift: m.shift,
                ctrl: m.ctrl,
                alt: m.alt,
                meta: m.meta,
            }),
            // Releases outside an unfocused window may never arrive.
            InputEvent::Focused(false) => Input::WindowBlur,
            InputEvent::Ime(ImeEvent::Preedit(text, cursor)) => Input::ImePreedit { text, cursor },
            InputEvent::Ime(ImeEvent::Commit(text)) => Input::ImeCommit(text),
            InputEvent::Ime(ImeEvent::Disabled) => Input::ImePreedit {
                text: String::new(),
                cursor: None,
            },
            _ => return,
        };
        self.handle_ui(input);
        // Keys typed into a text field are the field's, not the game's.
        let typing = self
            .interaction
            .focused()
            .is_some_and(|id| self.ui.is_editor(id));
        if let Some(args) = game_key.filter(|_| !typing) {
            self.send_game_input(&args);
        }
    }

    fn frame(&mut self) {
        self.tick();
        // Caret blinking.
        self.ui.set_time(self.now_ms());
        // The caret may have moved with layout.
        self.update_ime();
        let Some(gfx) = &mut self.gfx else {
            return;
        };

        // The UI lays out in UI pixels: the window size divided by its DPI and UI scale.
        let scale = ui_factor(&gfx.window, self.ui_scale);
        let size = gfx.window.inner_size();
        self.ui
            .set_size(size.width as f32 / scale, size.height as f32 / scale);
        // `@media (resolution)` and `(prefers-color-scheme)` follow the window.
        let dark = gfx.window.theme() == Some(iron_platform::Theme::Dark);
        self.ui.set_media_env(iron_ui::media::MediaEnv {
            resolution: scale,
            dark,
            ..self.ui.media_env()
        });
        load_images(&mut self.ui, &mut gfx.renderer, &gfx.gpu);
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

/// Sends one UI event to scripts; returns whether its default action was prevented.
fn dispatch(scripts: &mut QuickJsRuntime, enabled: bool, event: &UiEvent) -> bool {
    if !enabled {
        return false;
    }
    match scripts.call_global("__iron_ui_event", &event.script_args()) {
        Ok(prevented) => prevented == iron_core::Value::Bool(true),
        Err(e) => {
            tracing::error!(target: "script", "{} handler: {e}", event.kind.name());
            false
        }
    }
}

/// Physical pixels per UI pixel for `window`: its DPI scale times `ui_scale`'s factor
/// for its logical size.
fn ui_factor(window: &Window, ui_scale: ui_scale::UiScale) -> f32 {
    let dpi = window.scale_factor() as f32;
    let size = window.inner_size();
    let (w, h) = (size.width as f32 / dpi, size.height as f32 / dpi);
    dpi * ui_scale.factor(w, h)
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
/// Loads images that `image` elements started using and tells the UI their natural
/// sizes, so they lay out before they're drawn.
fn load_images(ui: &mut UiTree, renderer: &mut UiRenderer, gpu: &Gpu) {
    for src in ui.take_image_requests() {
        let size = renderer.image_size(gpu, &src);
        ui.set_image_size(&src, size);
    }
}

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
/// `audio.load id src stream`: reads the file (or `data:` URL) from the game folder.
fn load_sound(
    mixer: &mut dyn iron_audio::Mixer,
    root: &Path,
    args: &[iron_core::Value],
) -> Result<(), String> {
    let (Some(id), Some(src)) = (
        args.first().and_then(iron_core::Value::as_f64),
        args.get(1).and_then(iron_core::Value::as_str),
    ) else {
        return Err("needs an id and a file".into());
    };
    let stream = matches!(args.get(2), Some(iron_core::Value::Bool(true)));
    let bytes = if src.trim_start().starts_with("data:") {
        iron_core::decode_data_url(src).ok_or_else(|| "invalid data URL".to_owned())?
    } else {
        let path = iron_core::resolve_asset(root, src)
            .ok_or_else(|| format!("`{src}` must be a relative path inside the game folder"))?;
        std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?
    };
    mixer
        .load(id as u32, bytes, stream)
        .map_err(|e| format!("`{src}`: {e}"))
}

fn add_stylesheet(ui: &mut UiTree, root: &Path, css: &str, base: &str, source: &str) -> u32 {
    // `@import`s load from the game folder (Bun inlines them when bundling; this is
    // for stylesheets added at runtime).
    let added = ui.add_stylesheet_with_imports(css, base, &mut |path| {
        let file = iron_core::resolve_asset(root, path)?;
        std::fs::read_to_string(file).ok()
    });
    for warning in &added.warnings {
        tracing::warn!(target: "css", "{source}: {warning}");
    }
    for face in &added.font_faces {
        load_font(ui, root, &face.family, &face.src, face.weight, face.italic);
    }
    added.id
}
