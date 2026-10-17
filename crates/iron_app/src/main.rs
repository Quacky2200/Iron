// A windowed program: no console window of its own (the log goes to a file, see
// `logging`); tests keep the console harness.
#![cfg_attr(not(test), windows_subsystem = "windows")]
//! `iron [game] [--screenshot out.png] [--frames N]`: opens a window, loads the
//! game's script bundle and runs it.
//!
//! `[game]` is either a game folder (loads `<game>/dist/game.js`, built with
//! `bun run build`) or a `.js` bundle directly; without one, the current folder is the game. `--screenshot` saves the UI after
//! `N` frames (default 60) and exits, which is handy for checking output.

mod cursor;
mod display;
mod game_input;
mod info;
mod lifecycle;
mod logging;
mod models;
mod panels;
mod particles;
mod physics_sync;
mod prefs;
mod reload;
mod ui_scale;
mod world_view;

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use iron_platform::{
    CursorIcon, CustomCursorRequest, ImeEvent, InputEvent, MouseButton, Platform, PlatformApp,
    WheelDelta, Window, WindowConfig,
};
use iron_render::wgpu;
use iron_render::{
    Color, Gpu, OffscreenTarget, RenderTarget, UiRenderer, WindowSurface, WorldRenderer,
};
use iron_script::{ComponentHost, QuickJsRuntime, ScriptRuntime};
use iron_ui::{Clipboard, Input, Interaction, UiEvent, UiTree, events};

const TICK: f64 = 1.0 / 60.0;
/// Stops a slow frame from triggering a long burst of catch-up ticks.
const MAX_TICKS_PER_FRAME: u32 = 5;
/// The longest frame `onUpdate` is told about (a hitch shouldn't teleport things).
const MAX_FRAME_DT: f64 = 0.1;
/// Pixels per wheel notch ("line"), like browsers.
const WHEEL_LINE: f32 = 40.0;
const CLEAR: Color = Color::rgb(0.05, 0.06, 0.09);

struct Args {
    game: PathBuf,
    screenshot: Option<PathBuf>,
    frames: u32,
}

fn parse_args() -> Result<Args> {
    parse_args_from(std::env::args_os().skip(1))
}

/// Parses the arguments after the program name. The game is the first argument that isn't
/// an option, or the current folder when there's none.
fn parse_args_from(args: impl IntoIterator<Item = std::ffi::OsString>) -> Result<Args> {
    let usage = "usage: iron [game folder | bundle.js] [--screenshot out.png] [--frames N]";
    let mut args = args.into_iter();
    let mut game = None;
    let mut parsed = Args {
        game: PathBuf::from("."),
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
            Some(s) if game.is_none() && !s.starts_with("--") => game = Some(PathBuf::from(s)),
            None if game.is_none() => game = Some(PathBuf::from(arg)),
            _ => bail!("unexpected argument {arg:?}\n{usage}"),
        }
    }
    if let Some(game) = game {
        parsed.game = game;
    }
    Ok(parsed)
}

fn main() -> Result<()> {
    logging::attach_console();
    let result = run();
    if let Err(e) = &result {
        // Into the log as well as stderr: a windowed program's stderr may go nowhere.
        tracing::error!("{e:#}");
    }
    result
}

fn run() -> Result<()> {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            logging::init(None);
            return Err(e);
        }
    };
    // The log file sits with the game's settings (not for screenshot runs, which leave
    // the player's files alone; `IRON_LOG_FILE` still works there).
    let manifest_dir = match resolve_bundle(&args.game) {
        Ok((_, root)) => Some(root),
        Err(_) => None,
    };
    let settings = manifest_dir
        .as_deref()
        .map(info::Manifest::load)
        .and_then(|m| prefs::settings_dir(&m))
        .filter(|_| args.screenshot.is_none());
    if let Some(log) = logging::init(logging::log_path(settings.as_deref())) {
        tracing::info!("log: {}", log.display());
    }

    let (bundle_path, asset_root) = resolve_bundle(&args.game)?;
    let code = std::fs::read_to_string(&bundle_path)
        .with_context(|| format!("reading {}", bundle_path.display()))?;
    let name = bundle_path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "game.js".into());

    // The game's manifest and the machine, for `Application` and `SystemInfo`, before
    // the bundle's top level runs.
    let manifest = info::Manifest::load(&asset_root);
    let mut info = info::Info::new(&manifest);
    // The player's saved settings (not for screenshot runs, which stay the same every
    // time) and the window mode: the manifest's default, then the player's choice.
    let prefs = match args.screenshot {
        Some(_) => prefs::Prefs::in_memory(),
        None => prefs::Prefs::load(&manifest),
    };
    let display_settings = display::from_json(
        prefs.get("iron.display"),
        display::from_json(Some(&manifest.display), Default::default()),
    );
    if let Some(path) = prefs.path() {
        tracing::info!("player prefs: {}", path.display());
    }
    info.set("prefs", serde_json::Value::Object(prefs.all().clone()));
    info.set("display", display::state(None, &display_settings));
    let mut scripts = QuickJsRuntime::new();
    scripts
        .eval(&info.script())
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    // A dev build (`scripts/dev.ts`) comes as vendor.js plus the game's own code.
    let vendor = dev_vendor(&bundle_path, &code);
    load_bundles(&mut scripts, vendor.as_deref(), &name, &code)
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    // One GameObject per starting component class (registered without
    // `spawn: false`), each with that component; a start scene (`iron.startScene`)
    // is loaded by the scripts themselves.
    let mut components = ComponentHost::new();
    let mut scene = iron_scene::ScriptScene::new();
    let classes =
        ComponentHost::registered_classes(&mut scripts).map_err(|e| anyhow::anyhow!("{e}"))?;
    spawn_starting_components(&mut scripts, &mut components, &mut scene, &classes)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
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
    let scene_events = scripts
        .eval("typeof __iron_scene === 'function'")
        .is_ok_and(|v| v == iron_core::Value::Bool(true));
    let physics_events = scripts
        .eval("typeof __iron_physics === 'function'")
        .is_ok_and(|v| v == iron_core::Value::Bool(true));

    // Bun writes CSS imported by the game next to the bundle (game.js -> game.css).
    let mut ui = UiTree::with_system_fonts(1.0, 1.0);
    let css_path = bundle_path.with_extension("css");
    let game_css = load_game_css(&mut ui, &asset_root, &css_path);
    let game_css_source = game_css_source(&asset_root, &css_path);

    // Rebuilds by `bun build --watch` reload the scripts (not for screenshot runs).
    let watcher = match args.screenshot {
        Some(_) => None,
        None => reload::BundleWatcher::new(vec![
            bundle_path.clone(),
            css_path.clone(),
            bundle_path.with_file_name("vendor.js"),
        ])
        .inspect_err(|e| tracing::warn!("not watching for script changes: {e}"))
        .ok(),
    };
    // Asset files that change (textures, models, sounds) load again in place.
    let asset_watcher = match args.screenshot {
        Some(_) => None,
        None => reload::AssetWatcher::new(&asset_root)
            .inspect_err(|e| tracing::warn!("not watching for asset changes: {e}"))
            .ok(),
    };

    let app = App {
        prefs,
        display: display_settings,
        target_frame_rate: -1,
        vendor,
        bundle_path,
        bundle_name: name.clone(),
        game_css,
        watcher,
        asset_watcher,
        sound_files: HashMap::new(),
        scripts,
        components,
        scene,
        scene_events,
        world_view: world_view::WorldViews::default(),
        panels: panels::Panels::default(),
        pointer_on: Surface::Screen,
        keys_on: Surface::Screen,
        buttons_down: 0,
        pointer_px: None,
        sheet_sources: game_css_source.into_iter().collect(),
        font_sources: Vec::new(),
        screen: (0.0, 0.0, 0.0),
        ui,
        sheets: HashMap::new(),
        interaction: Interaction::new(),
        ui_events,
        game_input,
        physics: iron_physics::Physics::new(),
        attachments: physics_sync::Attachments::default(),
        audio: None,
        audio_events,
        physics_events,
        nav: game_input::GamepadNav::default(),
        cursor: ("default", Vec::new(), 1.0),
        ui_scale: ui_scale::UiScale::None,
        custom_cursor: None,
        script_cursor: ScriptCursor::default(),
        focused: true,
        cursors_loaded: HashSet::new(),
        cursors_failed: HashSet::new(),
        clipboard: None,
        ime_allowed: false,
        ime_area: None,
        asset_root,
        gfx: None,
        started: Instant::now(),
        clock: lifecycle::FixedClock::default(),
        lifecycle: lifecycle::Lifecycle::new(manifest.run_in_background, args.screenshot.is_some()),
        frames: 0,
        screenshot: args.screenshot.map(|path| (path, args.frames)),
        done: false,
        info,
    };
    let icon = manifest
        .icon
        .as_deref()
        .and_then(|path| info::load_icon(&app.asset_root, path));
    // A window opens at the saved window size; fullscreen modes are applied once it exists.
    let (width, height) = (
        display_settings.window_width,
        display_settings.window_height,
    );
    Platform::new().run(
        WindowConfig {
            title: manifest.title.clone(),
            icon,
            width,
            height,
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
    world: WorldRenderer,
}

/// Draws one frame into `target`: the 3D world when a camera is active, with the
/// screen UI over it; otherwise the UI over the plain background.
fn draw_frame(
    gpu: &Gpu,
    renderer: &mut UiRenderer,
    world: &mut WorldRenderer,
    target: &RenderTarget<'_>,
    views: &[iron_render::WorldView],
    textures: &[&wgpu::TextureView],
    cmds: &[iron_ui::DrawCmd],
) {
    if views.is_empty() {
        renderer.render(gpu, target, CLEAR, cmds);
    } else if iron_render::needs_backdrop(cmds) {
        // Backdrop blurs and blend modes read the world: draw it where the UI can
        // sample it, then the UI over a copy of it into the target.
        let backdrop = renderer.backdrop_target(gpu, target.size, target.format);
        let world_target = RenderTarget {
            view: &backdrop,
            format: target.format,
            size: target.size,
            scale: target.scale,
        };
        world.render_views(gpu, &world_target, views, textures);
        renderer.render_over(gpu, target, &backdrop, cmds);
    } else {
        world.render_views(gpu, target, views, textures);
        renderer.render_overlay(gpu, target, cmds);
    }
}

struct App {
    /// The player's saved settings (`PlayerPrefs`), written on `prefs.save` and on exit.
    prefs: prefs::Prefs,
    /// The window's mode and size, as last applied.
    display: iron_platform::DisplaySettings,
    /// `Application.targetFrameRate` (-1: no cap); ignored while vsync is on.
    target_frame_rate: i32,
    /// A dev build's vendor bundle (React, the engine), kept while only the game's code
    /// changes: then reloads happen in place, with Fast Refresh.
    vendor: Option<String>,
    /// GameObjects scripts made (and the engine made for them).
    scene: iron_scene::ScriptScene,
    /// Whether the game takes engine moves (`__iron_scene` exists).
    scene_events: bool,
    /// The screen size in UI pixels and the UI scale, as last told to scripts.
    screen: (f32, f32, f32),
    /// React UI on planes in the world (`WorldPanel`s), each with its own tree.
    panels: panels::Panels,
    /// The surface under the pointer (or holding it while a button is down), and the one
    /// keys go to (the last one clicked).
    pointer_on: Surface,
    keys_on: Surface,
    buttons_down: u32,
    /// The pointer in window pixels, while it's over the window.
    pointer_px: Option<(f32, f32)>,
    /// Stylesheets every UI tree gets: (key, css, base, source name). Keys are script
    /// sheet ids, or `GAME_CSS`.
    sheet_sources: Vec<(u32, String, String, String)>,
    /// Fonts scripts loaded (family, src, weight, italic), replayed into new panel trees.
    font_sources: Vec<(String, String, Option<f32>, Option<bool>)>,
    /// Cameras and planes scripts put in the world.
    world_view: world_view::WorldViews,
    bundle_path: PathBuf,
    bundle_name: String,
    /// The tree's id for the game's bundled stylesheet (`game.css`), if it has one.
    game_css: Option<u32>,
    watcher: Option<reload::BundleWatcher>,
    asset_watcher: Option<reload::AssetWatcher>,
    /// Sounds loaded from files (by asset key): their `audio.load` arguments, to load
    /// again when the file changes.
    sound_files: HashMap<String, Vec<Vec<iron_core::Value>>>,
    scripts: QuickJsRuntime,
    components: ComponentHost,
    ui: UiTree,
    asset_root: PathBuf,
    gfx: Option<Gfx>,
    started: Instant,
    clock: lifecycle::FixedClock,
    /// Focus, and pausing in the background (`Application.runInBackground`).
    lifecycle: lifecycle::Lifecycle,
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
    /// Bodies attached to GameObjects (`physics.attach`).
    attachments: physics_sync::Attachments,
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
    /// The game's manifest and the machine, as scripts see them (`__iron_info`).
    info: info::Info,
    /// Scripts' `Cursor`: shown or not, how it's held, and their default image.
    script_cursor: ScriptCursor,
    /// Whether the window has focus (raw mouse motion only counts then).
    focused: bool,
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
    /// Hot reload (DESIGN.md §6.1): loads the rebuilt bundle into a new runtime and
    /// swaps it in, or keeps the old one running if it fails. For now the world is
    /// wiped too: UI, script stylesheets, physics and audio start over, and components
    /// run `onStart` again (state survives once GameObjects exist, TODO.md M5).
    fn reload_scripts(&mut self) {
        let started = Instant::now();
        let code = match std::fs::read_to_string(&self.bundle_path) {
            Ok(code) => code,
            Err(e) => {
                tracing::error!("reload: reading {}: {e}", self.bundle_path.display());
                return;
            }
        };
        let vendor = dev_vendor(&self.bundle_path, &code);
        if vendor.is_some() && vendor == self.vendor {
            self.reload_in_place(&code, started);
            return;
        }
        // The old code saves its scenes (objects, components, their fields) as JSON;
        // the new code rebuilds them instead of starting over.
        // (Bundles without @iron/engine's scenes have nothing to save.)
        let can_save = self
            .scripts
            .eval("typeof __iron_save_state === 'function'")
            .is_ok_and(|v| v == iron_core::Value::Bool(true));
        let saved = match can_save.then(|| self.scripts.call_global("__iron_save_state", &[])) {
            Some(Ok(iron_core::Value::String(json))) => Some(json),
            Some(Err(e)) => {
                tracing::warn!("reload: saving state failed, starting over: {e}");
                None
            }
            _ => None,
        };
        let mut fresh = QuickJsRuntime::new();
        if let Err(e) = fresh.eval(&self.info.script()) {
            tracing::error!("reload: {e}");
        }
        if saved.is_some() {
            // Keeps the start scene from loading on top of the restored ones.
            let _ = fresh.eval("globalThis.__iron_reloading = true");
        }
        let mut scene = iron_scene::ScriptScene::new();
        let mut restored = false;
        let classes = match iron_script::reload(
            &mut self.scripts,
            fresh,
            &mut self.components,
            |rt| load_bundles(rt, vendor.as_deref(), &self.bundle_name, &code),
            |rt, host, classes| {
                if let Some(json) = saved {
                    let _ = rt.eval("globalThis.__iron_reloading = false");
                    restored = rt
                        .call_global("__iron_restore_state", &[iron_core::Value::String(json)])?
                        == iron_core::Value::Bool(true);
                }
                if restored {
                    // The restored objects and components arrive as ops.
                    Ok(())
                } else {
                    spawn_starting_components(rt, host, &mut scene, classes)
                }
            },
        ) {
            Ok(classes) => classes,
            Err(e) => {
                tracing::error!("reload failed, keeping the old code: {e}");
                return;
            }
        };

        self.vendor = vendor;
        // Everything the old scripts built. The new bundle's top level has already
        // queued its UI and ops; they apply after this.
        self.ui.clear_script_nodes();
        for (_, sheet) in self.sheets.drain() {
            self.ui.remove_stylesheet(sheet);
        }
        if let Some(sheet) = self.game_css.take() {
            self.ui.remove_stylesheet(sheet);
        }
        self.reload_game_css();
        self.font_sources.clear();
        self.panels = panels::Panels::default();
        (self.pointer_on, self.keys_on) = (Surface::Screen, Surface::Screen);
        self.ui_scale = ui_scale::UiScale::None;
        self.scene = scene;
        self.world_view = world_view::WorldViews::default();
        // Tell the new scripts the screen size on the next frame.
        self.screen = (0.0, 0.0, 0.0);
        self.physics = iron_physics::Physics::new();
        self.attachments = physics_sync::Attachments::default();
        // Dropping the mixer stops every sound.
        self.audio = None;
        self.handle_ui(Input::Refresh);

        let has = |scripts: &mut QuickJsRuntime, f: &str| {
            scripts
                .eval(&format!("typeof {f} === 'function'"))
                .is_ok_and(|v| v == iron_core::Value::Bool(true))
        };
        self.ui_events = has(&mut self.scripts, "__iron_ui_event");
        self.game_input = has(&mut self.scripts, "__iron_input");
        self.physics_events = has(&mut self.scripts, "__iron_physics");
        self.audio_events = has(&mut self.scripts, "__iron_audio");
        self.scene_events = has(&mut self.scripts, "__iron_scene");
        self.lifecycle.scripts_restarted();
        self.sync_lifecycle();
        tracing::info!(
            ?classes,
            restored,
            "reloaded {} in {:.1} ms",
            self.bundle_name,
            started.elapsed().as_secs_f64() * 1000.0
        );
    }

    /// A dev build whose vendor bundle didn't change: the game's new code runs in the
    /// same runtime. React (Fast Refresh) keeps its state, components move onto their new
    /// classes, and the world, physics and sounds stay as they are.
    fn reload_in_place(&mut self, code: &str, started: Instant) {
        if let Err(e) = iron_script::load_game(&mut self.scripts, &self.bundle_name, code) {
            // A syntax error stops it before anything ran; a throwing top level may have
            // run partly.
            tracing::error!("reload failed: {e}");
            return;
        }
        // New classes may have other lifecycle methods.
        self.components.refresh_hooks();
        if let Some(sheet) = self.game_css.take() {
            self.ui.remove_stylesheet(sheet);
        }
        self.reload_game_css();
        self.handle_ui(Input::Refresh);
        tracing::info!(
            "reloaded {} in place in {:.1} ms",
            self.bundle_name,
            started.elapsed().as_secs_f64() * 1000.0
        );
    }

    /// The CSS Bun wrote next to the bundle, applied again.
    fn reload_game_css(&mut self) {
        let css_path = self.bundle_path.with_extension("css");
        self.game_css = load_game_css(&mut self.ui, &self.asset_root, &css_path);
        self.sheet_sources = game_css_source(&self.asset_root, &css_path)
            .into_iter()
            .collect();
    }

    /// Starts or ends a pause in the background and tells scripts about focus and
    /// pausing (`__iron_focus(focused, paused)`).
    fn sync_lifecycle(&mut self) {
        let change = self.lifecycle.update(Instant::now());
        if change.paused
            && let Some(audio) = self.audio.as_mut()
        {
            audio.set_muted(true);
        }
        if let Some(gap) = change.resumed {
            // Script time (timers, caret, UI time) skips the pause, and fixed ticks
            // start over instead of catching up.
            self.started += gap;
            self.clock.restart();
            if let Some(audio) = self.audio.as_mut() {
                audio.set_muted(false);
            }
        }
        if let Some((focused, paused)) = change.report {
            // Games without @iron/engine's Application don't have the hook.
            let has = self
                .scripts
                .eval("typeof __iron_focus === 'function'")
                .is_ok_and(|v| v == iron_core::Value::Bool(true));
            if has
                && let Err(e) = self
                    .scripts
                    .call_global("__iron_focus", &[focused.into(), paused.into()])
            {
                tracing::error!(target: "script", "__iron_focus: {e}");
            }
            // What the hooks asked for (a pause menu, paused sounds) applies now.
            self.apply_script_output();
        }
    }

    fn tick(&mut self) {
        let now = Instant::now();
        if self.watcher.as_mut().is_some_and(|w| w.poll(now)) {
            self.reload_scripts();
        }
        let changed = self
            .asset_watcher
            .as_mut()
            .map(|w| w.poll(now))
            .unwrap_or_default();
        if !changed.is_empty() {
            self.reload_assets(&changed);
        }
        // Paused in the background: the game's time stands still.
        if self.lifecycle.paused() {
            return;
        }
        let (elapsed, ticks) = self.clock.advance(now, TICK, MAX_TICKS_PER_FRAME);
        for _ in 0..ticks {
            let scene = &self.scene;
            self.components
                .fixed_update(&mut self.scripts, TICK, &|o| scene.active(o));
            // What the components asked for applies before physics steps.
            self.apply_script_output();
            self.step_physics();
            self.send_scene_moves();
        }

        // Script time: timers and their microtasks, once per frame.
        let now_ms = now.duration_since(self.started).as_secs_f64() * 1000.0;
        for e in self.scripts.tick(now_ms) {
            tracing::error!(target: "script", "uncaught: {e}");
        }

        // Per-frame component updates (`onUpdate`, `onLateUpdate`).
        self.apply_script_output();
        let scene = &self.scene;
        self.components
            .update_active(&mut self.scripts, elapsed.min(MAX_FRAME_DT), &|o| {
                scene.active(o)
            });
        self.apply_script_output();
        let dt = self.components.frame_dt() as f32;
        let changed = self.world_view.update_particles(dt, &self.scene);
        // `__iron_particles([id, count, alive, ...])` for systems that changed.
        if !changed.is_empty()
            && let Err(e) = self.scripts.call_global(
                "__iron_particles",
                &[iron_core::Value::Array(
                    changed.into_iter().map(iron_core::Value::Number).collect(),
                )],
            )
        {
            tracing::debug!("__iron_particles: {e}");
        }
        self.update_audio(elapsed);

        // The trees may have moved under a still pointer, or the focused element gone.
        self.handle_ui(Input::Refresh);
        let ids: Vec<u32> = self.panels.map.keys().copied().collect();
        for id in ids {
            self.handle_on(Surface::Panel(id), Input::Refresh);
        }
    }

    /// Objects the engine moved this tick, to scripts (`__iron_scene`).
    fn send_scene_moves(&mut self) {
        let moved = self.scene.take_batch();
        if moved.is_empty() || !self.scene_events {
            return;
        }
        if let Err(e) = self
            .scripts
            .call_global("__iron_scene", &[iron_core::Value::Array(moved)])
        {
            tracing::error!(target: "script", "scene moves: {e}");
        }
    }

    /// Runs one `scene.*` op and what it asks of components.
    fn scene_op(&mut self, name: &str, args: &[iron_core::Value]) {
        use iron_scene::SceneEvent;
        match self.scene.apply(name, args) {
            Ok(None) => {}
            Ok(Some(SceneEvent::ComponentAdded {
                object,
                component,
                class,
            })) => self
                .components
                .adopt(iron_core::ComponentId(component), &class, Some(object)),
            Ok(Some(SceneEvent::ComponentRemoved(id))) => self
                .components
                .remove(&mut self.scripts, iron_core::ComponentId(id)),
            Ok(Some(SceneEvent::Destroyed(objects))) => {
                self.components
                    .remove_on_objects(&mut self.scripts, &objects);
                self.world_view.forget_objects(&objects);
                self.panels.forget_objects(&objects);
            }
            Err(e) => tracing::warn!("scene.{name}: {e}"),
        }
    }

    /// One physics tick, then its results to scripts (`__iron_physics`).
    fn step_physics(&mut self) {
        let batch = self.attachments.step(&mut self.physics, &mut self.scene);
        if !self.physics_events || batch.is_empty() {
            return;
        }
        if let Err(e) = self.scripts.call_global("__iron_physics", &batch.to_args()) {
            tracing::error!(target: "script", "physics events: {e}");
        }
        // Collision handlers may have queued more ops; they apply next tick.
    }

    /// The screen and every world panel.
    fn surfaces(&self) -> Vec<Surface> {
        std::iter::once(Surface::Screen)
            .chain(self.panels.map.keys().map(|id| Surface::Panel(*id)))
            .collect()
    }

    /// The UI tree holding a node (ops like `focus` name nodes, not trees).
    fn surface_of(&self, node: iron_core::NodeId) -> Surface {
        if self.ui.contains(node) {
            return Surface::Screen;
        }
        self.panels
            .map
            .iter()
            .find(|(_, p)| p.tree.contains(node))
            .map_or(Surface::Screen, |(id, _)| Surface::Panel(*id))
    }

    /// The ray from the camera under a window pixel.
    fn camera_ray(
        &self,
        px: f32,
        py: f32,
    ) -> Option<(iron_core::glam::Vec3, iron_core::glam::Vec3)> {
        let size = self.gfx.as_ref()?.surface.size();
        // Cameras drawing into textures aren't on the screen.
        let mut views = self.world_view.views(&self.scene, size, &[]);
        views.retain(|v| v.target_texture.is_none());
        panels::camera_ray(&views, size, px, py)
    }

    /// What the pointer at a window pixel is over: screen UI wherever the screen has an
    /// element (layout boxes covering the screen should use `pointerEvents: none`),
    /// otherwise the nearest world panel the camera ray hits, otherwise the screen.
    fn surface_at(&mut self, px: f32, py: f32) -> (Surface, f32, f32) {
        let scale = self.scale();
        let (ux, uy) = (px / scale, py / scale);
        let on_screen = self.ui.hit_test(ux, uy).is_some_and(|n| n != iron_ui::ROOT);
        if !on_screen
            && !self.panels.map.is_empty()
            && let Some((origin, dir)) = self.camera_ray(px, py)
            && let Some(hit) = self.panels.hit(&self.scene, origin, dir)
        {
            return (Surface::Panel(hit.id), hit.x, hit.y);
        }
        (Surface::Screen, ux, uy)
    }

    /// Sends the pointer's position to the surface it's over (or the one holding it while
    /// a button is down), leaving the previous one.
    fn route_pointer(&mut self) {
        let Some((px, py)) = self.pointer_px else {
            return;
        };
        let (surface, x, y) = if self.buttons_down > 0 {
            // A drag stays with where it started, even off the panel's edge.
            match self.pointer_on {
                Surface::Panel(id) => {
                    let point = self
                        .camera_ray(px, py)
                        .and_then(|(o, d)| self.panels.project(&self.scene, id, o, d));
                    let (x, y) = point.unwrap_or((-1e6, -1e6));
                    (Surface::Panel(id), x, y)
                }
                Surface::Screen => {
                    let scale = self.scale();
                    (Surface::Screen, px / scale, py / scale)
                }
            }
        } else {
            self.surface_at(px, py)
        };
        if surface != self.pointer_on {
            self.handle_on(self.pointer_on, Input::PointerLeave);
            self.pointer_on = surface;
        }
        self.handle_on(surface, Input::PointerMove { x, y });
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
        let paused = self.lifecycle.paused();
        let mixer = self
            .audio
            .get_or_insert_with(|| iron_audio::open_mixer(offline));
        // A mixer opened during a pause starts silent.
        mixer.set_muted(paused);
        if name == "load"
            && let Some(src) = args.get(1).and_then(iron_core::Value::as_str)
            && !src.trim_start().starts_with("data:")
        {
            let list = self.sound_files.entry(reload::asset_key(src)).or_default();
            let id = args.first();
            list.retain(|a| a.first() != id);
            list.push(args.to_vec());
        }
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
        for (target, commit) in self.scripts.drain_targeted_commits() {
            let tree = if target == 0 {
                &mut self.ui
            } else {
                let (root, fonts, sheets) =
                    (&self.asset_root, &self.font_sources, &self.sheet_sources);
                &mut self
                    .panels
                    .get_or_create(target, || new_panel_tree(root, fonts, sheets))
                    .tree
            };
            for e in tree.apply_commit(&commit) {
                tracing::warn!(target: "ui", "{e}");
            }
        }
        // Textures and models loaded by these ops, for `__iron_textures` /
        // `__iron_models` after them.
        let mut loaded: Vec<(u32, u32, u32)> = Vec::new();
        let mut models: Vec<(u32, Option<serde_json::Value>)> = Vec::new();
        // Text files read for scripts (`asset.readText id path`), for `__iron_asset_text`.
        let mut texts: Vec<iron_core::Value> = Vec::new();
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
                    let surface = self.surface_of(id);
                    if op.name == "focus" {
                        self.keys_on = surface;
                    }
                    self.handle_on(
                        surface,
                        match op.name.as_str() {
                            "focus" => Input::Focus(id),
                            "blur" => Input::Blur(id),
                            "setPointerCapture" => Input::SetPointerCapture(id),
                            _ => Input::ReleasePointerCapture(id),
                        },
                    );
                }
                "scrollTo" | "scrollBy" | "scrollIntoView" => {
                    let num = |i: usize| op.args.get(i).and_then(iron_core::Value::as_f64);
                    let Some(id) = num(0).map(|n| iron_core::NodeId(n as u32)) else {
                        tracing::warn!("{} needs an element id", op.name);
                        continue;
                    };
                    let (a, b) = (num(1).unwrap_or(0.0) as f32, num(2).unwrap_or(0.0) as f32);
                    let surface = self.surface_of(id);
                    self.handle_on(
                        surface,
                        match op.name.as_str() {
                            "scrollTo" => Input::ScrollTo { id, x: a, y: b },
                            "scrollBy" => Input::ScrollBy { id, dx: a, dy: b },
                            _ => Input::ScrollIntoView(id),
                        },
                    );
                }
                "render.model" => {
                    let id = op
                        .args
                        .first()
                        .and_then(iron_core::Value::as_f64)
                        .unwrap_or(0.0) as u32;
                    match self.world_view.model_op(&self.asset_root, &op.args) {
                        Ok(description) => models.push((id, Some(description))),
                        Err(e) => {
                            tracing::warn!("render.model: {e}");
                            models.push((id, None));
                        }
                    }
                }
                "render.texture" | "render.textureRemove" => {
                    let name = &op.name["render.".len()..];
                    match self.world_view.texture_op(name, &self.asset_root, &op.args) {
                        Ok(Some((id, w, h))) => loaded.push((id, w, h)),
                        Ok(None) => {}
                        Err(e) => {
                            tracing::warn!("render.texture: {e}");
                            // Scripts hear about failures too (a 0 × 0 size).
                            if let Some(id) = op.args.first().and_then(iron_core::Value::as_f64) {
                                loaded.push((id as u32, 0, 0));
                                // Sprites of it show the missing checker.
                                self.world_view.mark_missing(id as u32);
                            }
                        }
                    }
                }
                "render.panel" => {
                    let (root, fonts, sheets) =
                        (&self.asset_root, &self.font_sources, &self.sheet_sources);
                    if let Err(e) = self
                        .panels
                        .apply(&op.args, || new_panel_tree(root, fonts, sheets))
                    {
                        tracing::warn!("render.panel: {e}");
                    }
                }
                name if name.starts_with("render.") => {
                    if name == "render.remove"
                        && let Some(id) = op.args.first().and_then(iron_core::Value::as_f64)
                    {
                        self.panels.remove(id as u32);
                    }
                    if let Err(e) = self.world_view.apply(&name["render.".len()..], &op.args) {
                        tracing::warn!("{name}: {e}");
                    }
                }
                "asset.readText" => {
                    let id = op.args.first().cloned().unwrap_or(iron_core::Value::Null);
                    let path = op
                        .args
                        .get(1)
                        .and_then(iron_core::Value::as_str)
                        .unwrap_or("");
                    let text = iron_core::resolve_asset(&self.asset_root, path)
                        .ok_or_else(|| "outside the game folder".to_string())
                        .and_then(|file| std::fs::read_to_string(file).map_err(|e| e.to_string()));
                    texts.push(id);
                    texts.push(match text {
                        Ok(text) => iron_core::Value::String(text),
                        Err(e) => {
                            tracing::warn!("asset.readText `{path}`: {e}");
                            iron_core::Value::Null
                        }
                    });
                }
                "app.quit" => self.done = true,
                "app.targetFrameRate" => {
                    let rate = op.args.first().and_then(iron_core::Value::as_f64);
                    self.target_frame_rate = rate.map_or(-1, |r| r.round() as i32);
                    self.info
                        .set_app("targetFrameRate", self.target_frame_rate.into());
                }
                "app.runInBackground" => {
                    let on = op.args.first() == Some(&iron_core::Value::Bool(true));
                    self.lifecycle.run_in_background = on;
                    self.info.set_app("runInBackground", on.into());
                    self.sync_lifecycle();
                }
                name if name.starts_with("prefs.") => {
                    self.prefs_op(&name["prefs.".len()..], &op.args)
                }
                "screen.set" => self.screen_op(&op.args),
                name if name.starts_with("cursor.") => {
                    self.cursor_op(&name["cursor.".len()..], &op.args)
                }
                name if name.starts_with("scene.") => {
                    self.scene_op(&name["scene.".len()..], &op.args)
                }
                name if name.starts_with("audio.") => {
                    self.audio_op(&name["audio.".len()..], &op.args)
                }
                name if name.starts_with("physics.") => {
                    let name = &name["physics.".len()..];
                    // Mesh colliders made from render meshes get the mesh's data here.
                    let mut args = std::borrow::Cow::Borrowed(&op.args);
                    if name == "collider"
                        && op.args.get(2).and_then(iron_core::Value::as_str) == Some("renderMesh")
                        && let Err(e) = self.world_view.resolve_mesh_collider(args.to_mut())
                    {
                        tracing::warn!("physics.collider: {e}");
                        continue;
                    }
                    // `attach` ties a body to its GameObject (Rigidbody, colliders).
                    if !self.attachments.op(name, &args)
                        && let Err(e) = self.physics.apply(name, &args)
                    {
                        tracing::warn!("physics.{name}: {e}");
                    }
                }
                other => tracing::warn!("unknown script op `{other}`"),
            }
        }
        // `__iron_asset_text([id, text | null, ...])`: files scripts asked for.
        if !texts.is_empty()
            && let Err(e) = self
                .scripts
                .call_global("__iron_asset_text", &[iron_core::Value::Array(texts)])
        {
            tracing::error!("script error in __iron_asset_text: {e}");
        }
        // `__iron_models([id, json | null, ...])`: what each model holds, for
        // `Model.instantiate` (null: it failed).
        if !models.is_empty() {
            let list = models
                .into_iter()
                .flat_map(|(id, d)| {
                    [
                        iron_core::Value::from(id),
                        d.map_or(iron_core::Value::Null, |d| {
                            iron_core::Value::String(d.to_string())
                        }),
                    ]
                })
                .collect();
            if let Err(e) = self
                .scripts
                .call_global("__iron_models", &[iron_core::Value::Array(list)])
            {
                tracing::error!("script error in __iron_models: {e}");
            }
        }
        self.report_textures(loaded);
    }

    /// Asset files changed on disk: textures, models and sounds from them load again
    /// under their ids, then scripts hear which files (`__iron_assets_changed`).
    fn reload_assets(&mut self, keys: &[String]) {
        let mut sizes = Vec::new();
        for key in keys {
            let (s, models) = self.world_view.reload_asset(&self.asset_root, key);
            if !s.is_empty() || !models.is_empty() {
                tracing::info!("reloaded {key}");
            }
            sizes.extend(s);
            for args in self.sound_files.get(key).cloned().unwrap_or_default() {
                if let Some(mixer) = self.audio.as_mut()
                    && let Err(e) = load_sound(mixer.as_mut(), &self.asset_root, &args)
                {
                    tracing::warn!("reloading sound `{key}`: {e}");
                }
            }
        }
        self.report_textures(sizes);
        if self
            .scripts
            .eval("typeof __iron_assets_changed === 'function'")
            .is_ok_and(|v| v == iron_core::Value::Bool(true))
        {
            let list = keys
                .iter()
                .map(|k| iron_core::Value::String(k.clone()))
                .collect();
            if let Err(e) = self
                .scripts
                .call_global("__iron_assets_changed", &[iron_core::Value::Array(list)])
            {
                tracing::error!(target: "script", "__iron_assets_changed: {e}");
            }
            self.apply_script_output();
        }
    }

    /// Tells scripts textures' sizes: `__iron_textures([[id, width, height], ...])` (0 × 0
    /// failed). Ops it queues run with the next batch.
    fn report_textures(&mut self, loaded: Vec<(u32, u32, u32)>) {
        if !loaded.is_empty()
            && self
                .scripts
                .eval("typeof __iron_textures === 'function'")
                .is_ok_and(|v| v == iron_core::Value::Bool(true))
        {
            let list = loaded
                .into_iter()
                .map(|(id, w, h)| {
                    iron_core::Value::Array(vec![
                        iron_core::Value::from(f64::from(id)),
                        iron_core::Value::from(f64::from(w)),
                        iron_core::Value::from(f64::from(h)),
                    ])
                })
                .collect();
            if let Err(e) = self
                .scripts
                .call_global("__iron_textures", &[iron_core::Value::Array(list)])
            {
                tracing::error!("script error in __iron_textures: {e}");
            }
        }
    }

    /// Runs one UI input, sending its events to scripts, then updates the OS cursor.
    fn handle_ui(&mut self, input: Input) {
        self.handle_on(Surface::Screen, input);
    }

    /// Runs one UI input on the screen's UI or a world panel's, sending its events to
    /// scripts, then updates the IME and the OS cursor.
    fn handle_on(&mut self, surface: Surface, input: Input) {
        let (scripts, ui_events) = (&mut self.scripts, self.ui_events);
        let clipboard = match surface {
            Surface::Screen => {
                self.interaction.handle(&mut self.ui, input, &mut |event| {
                    dispatch(scripts, ui_events, event)
                });
                self.interaction.take_clipboard()
            }
            Surface::Panel(id) => {
                let Some(p) = self.panels.map.get_mut(&id) else {
                    return;
                };
                p.interaction.handle(&mut p.tree, input, &mut |event| {
                    dispatch(scripts, ui_events, event)
                });
                p.interaction.take_clipboard()
            }
        };
        match clipboard {
            Some(Clipboard::Copy(text)) => {
                if let Some(Err(e)) = self.clipboard().map(|c| c.set_text(text)) {
                    tracing::warn!("copy failed: {e}");
                }
            }
            Some(Clipboard::Paste) => {
                if let Some(text) = self.clipboard().and_then(|c| c.get_text().ok()) {
                    self.handle_on(surface, Input::Paste(text));
                }
            }
            None => {}
        }
        self.update_ime();
        self.update_cursor();
    }

    /// Runs a `cursor.*` op: `visible bool`, `lock none|locked|confined`, `image
    /// src|null [hotspot x y]`.
    /// `prefs.set key value`, `prefs.delete key`, `prefs.deleteAll`, `prefs.save`.
    fn prefs_op(&mut self, name: &str, args: &[iron_core::Value]) {
        let key = args.first().and_then(iron_core::Value::as_str);
        match (name, key) {
            ("set", Some(key)) => {
                let value = args.get(1).map_or(serde_json::Value::Null, prefs::json_of);
                self.prefs.set(key, value);
            }
            ("delete", Some(key)) => self.prefs.delete(key),
            ("deleteAll", _) => self.prefs.delete_all(),
            ("save", _) => self.prefs.save(),
            _ => tracing::warn!("prefs.{name}: bad arguments {args:?}"),
        }
    }

    /// `screen.set mode width height refreshRate`: changes the window's mode and size,
    /// remembers it for next time and tells scripts what took effect. A size of 0 is the
    /// mode's default: the monitor's resolution (exclusive) or the last window size.
    fn screen_op(&mut self, args: &[iron_core::Value]) {
        let num = |i: usize| args.get(i).and_then(iron_core::Value::as_f64);
        let wanted = iron_platform::DisplaySettings {
            mode: args
                .first()
                .and_then(iron_core::Value::as_str)
                .and_then(iron_platform::FullScreenMode::parse)
                .unwrap_or(self.display.mode),
            width: num(1).map_or(self.display.width, |w| w.max(0.0) as u32),
            height: num(2).map_or(self.display.height, |h| h.max(0.0) as u32),
            refresh_rate: num(3).map_or(self.display.refresh_rate, |r| r.max(0.0) as f32),
            ..self.display
        };
        self.apply_display(wanted);
        self.prefs
            .set("iron.display", display::to_json(&self.display));
    }

    /// Puts the window in a mode (screenshot runs leave it alone) and updates
    /// `__iron_info.display`.
    fn apply_display(&mut self, wanted: iron_platform::DisplaySettings) {
        let window = self.gfx.as_ref().map(|g| g.window.clone());
        self.display = match (&window, &self.screenshot) {
            (Some(w), None) => iron_platform::display::apply(w, &wanted),
            _ => wanted,
        };
        self.info
            .set("display", display::state(window.as_deref(), &self.display));
        if let Err(e) = self.scripts.eval(&self.info.script()) {
            tracing::error!("display info: {e}");
        }
    }

    fn cursor_op(&mut self, name: &str, args: &[iron_core::Value]) {
        match (name, args.first()) {
            ("visible", Some(iron_core::Value::Bool(v))) => self.script_cursor.visible = *v,
            ("lock", Some(iron_core::Value::String(mode))) => {
                self.script_cursor.lock = match mode.as_str() {
                    "locked" => CursorLock::Locked,
                    "confined" => CursorLock::Confined,
                    _ => CursorLock::None,
                };
                self.apply_cursor_lock();
            }
            ("image", Some(iron_core::Value::String(src))) => {
                let hotspot = match args.get(1) {
                    Some(iron_core::Value::Array(h)) => {
                        let n =
                            |i: usize| h.get(i).and_then(iron_core::Value::as_f64).unwrap_or(0.0);
                        Some((n(0) as f32, n(1) as f32))
                    }
                    _ => None,
                };
                self.script_cursor.image = Some(iron_ui::CursorImage {
                    url: src.clone(),
                    hotspot,
                });
            }
            ("image", _) => self.script_cursor.image = None,
            _ => {
                tracing::warn!("cursor.{name}: bad arguments {args:?}");
                return;
            }
        }
        // Show the change even if what's hovered didn't change.
        self.cursor.0 = "";
        self.update_cursor();
    }

    /// Holds the cursor as scripts asked. Locking isn't available everywhere (Windows
    /// can only confine): it falls back to confining, with the cursor hidden.
    fn apply_cursor_lock(&mut self) {
        let Some(gfx) = &self.gfx else {
            return;
        };
        use iron_platform::CursorGrabMode;
        let result = match self.script_cursor.lock {
            CursorLock::None => gfx.window.set_cursor_grab(CursorGrabMode::None),
            CursorLock::Confined => gfx.window.set_cursor_grab(CursorGrabMode::Confined),
            CursorLock::Locked => gfx
                .window
                .set_cursor_grab(CursorGrabMode::Locked)
                .or_else(|_| gfx.window.set_cursor_grab(CursorGrabMode::Confined)),
        };
        if let Err(e) = result {
            tracing::debug!("cursor lock: {e}");
        }
    }

    /// Shows the hovered element's cursor: its first image that loads, else its keyword.
    /// Where the UI asks for nothing special, scripts' `Cursor` image is the default.
    fn update_cursor(&mut self) {
        let Some(gfx) = &self.gfx else {
            return;
        };
        let (keyword, ui_images) = match self.pointer_on {
            Surface::Panel(id) if self.panels.map.contains_key(&id) => {
                let p = &self.panels.map[&id];
                p.interaction.cursor_with_images(&p.tree)
            }
            _ => self.interaction.cursor_with_images(&self.ui),
        };
        let script_image;
        let images = match &self.script_cursor.image {
            Some(image) if ui_images.is_empty() && keyword == "default" => {
                script_image = [image.clone()];
                &script_image[..]
            }
            _ => ui_images,
        };
        let scale = gfx.window.scale_factor();
        if (keyword, images, scale) == (self.cursor.0, &self.cursor.1[..], self.cursor.2) {
            return;
        }
        self.cursor = (keyword, images.to_vec(), scale);
        // Hidden by scripts, or locked (which hides it, as in Unity).
        let shown = self.script_cursor.visible && self.script_cursor.lock != CursorLock::Locked;
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
                gfx.window.set_cursor_visible(shown);
                self.custom_cursor = Some(request);
            }
            None => {
                gfx.window.set_cursor_visible(shown && keyword != "none");
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
        // A field on a world panel takes IME text too; its candidate window stays where
        // the platform puts it.
        let (allowed, area) = match self.keys_on {
            Surface::Panel(id) => (
                self.panels
                    .map
                    .get(&id)
                    .is_some_and(|p| p.interaction.ime_area(&p.tree).is_some()),
                None,
            ),
            Surface::Screen => {
                let area = self.interaction.ime_area(&self.ui);
                (area.is_some(), area)
            }
        };
        if allowed != self.ime_allowed {
            self.ime_allowed = allowed;
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
        for p in self.panels.map.values_mut() {
            load_font(&mut p.tree, &self.asset_root, family, src, weight, italic);
        }
        self.font_sources
            .push((family.to_owned(), src.to_owned(), weight, italic));
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
        // Every UI tree (the screen's and each panel's) has the same stylesheets.
        self.sheet_sources.retain(|(key, ..)| *key != js_id);
        for p in self.panels.map.values_mut() {
            if let Some(id) = p.sheets.remove(&js_id) {
                p.tree.remove_stylesheet(id);
            }
        }
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
        for p in self.panels.map.values_mut() {
            let id = add_stylesheet(&mut p.tree, &self.asset_root, css, "", "addStyleSheet");
            p.sheets.insert(js_id, id);
        }
        self.sheet_sources
            .push((js_id, css.to_owned(), String::new(), "addStyleSheet".into()));
    }

    fn save_screenshot(&mut self, path: &Path) -> Result<()> {
        let gfx = self.gfx.as_mut().context("no renderer")?;
        let (w, h) = gfx.surface.size();
        let target =
            OffscreenTarget::with_format(&gfx.gpu, w, h, wgpu::TextureFormat::Rgba8UnormSrgb);
        let view = target.view().context("window has no size")?;
        load_images(&mut self.ui, &mut gfx.renderer, &gfx.gpu);
        let cmds = self.ui.draw_list();
        self.scene.scene.update_world();
        for (id, data) in self.world_view.mesh_uploads.drain(..) {
            match data {
                Some(data) => {
                    gfx.world.set_mesh(&gfx.gpu, id, &data);
                }
                None => gfx.world.remove_mesh(id),
            }
        }
        gfx.world.bloom = self.world_view.bloom;
        gfx.world.grading = self.world_view.grading;
        (
            gfx.world.shadow_distance,
            gfx.world.shadow_resolution,
            gfx.world.shadow_cascades,
        ) = self.world_view.shadow_settings;
        gfx.world.set_sky(self.world_view.sky);
        let q = self.world_view.quality;
        gfx.surface.set_vsync(&gfx.gpu, q.vsync);
        gfx.world.msaa = q.msaa;
        gfx.world.render_scale = q.render_scale;
        gfx.world.shadow_quality = q.shadows;
        gfx.world.soft_shadow_radius = q.soft_radius;
        gfx.world.anisotropic = q.anisotropic;
        gfx.world.texture_mip_limit = q.texture_mip_limit;
        (gfx.world.environment_intensity, gfx.world.ambient_from_sky) = self.world_view.environment;
        for (id, data) in self.world_view.texture_uploads.drain(..) {
            match data {
                Some(data) => {
                    gfx.world.set_texture(&gfx.gpu, id, &data);
                }
                None => gfx.world.remove_texture(id),
            }
        }
        for (id, size) in self.world_view.render_texture_uploads.drain(..) {
            gfx.world.set_render_texture(&gfx.gpu, id, size);
        }
        let now = self.started.elapsed().as_secs_f64() * 1000.0;
        let (quads, textures) = self.panels.render(
            &gfx.gpu,
            &mut gfx.renderer,
            &self.scene,
            now,
            |tree, renderer| load_images(tree, renderer, &gfx.gpu),
        );
        let views = self.world_view.views(&self.scene, (w, h), &quads);
        let textures: Vec<&wgpu::TextureView> = textures.iter().collect();
        draw_frame(
            &gfx.gpu,
            &mut gfx.renderer,
            &mut gfx.world,
            &RenderTarget {
                view: &view,
                format: target.format(),
                size: (w, h),
                scale: ui_factor(&gfx.window, self.ui_scale),
            },
            &views,
            &textures,
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
        // The saved (or default) window mode, before the first frame shows. Screenshot
        // runs stay windowed.
        if self.screenshot.is_none() && self.display.mode != iron_platform::FullScreenMode::Windowed
        {
            self.display = iron_platform::display::apply(&window, &self.display);
        }
        let size = window.inner_size();
        match Gpu::with_surface(window.clone(), size.width, size.height) {
            Ok((gpu, surface)) => {
                tracing::info!(adapter = %gpu.adapter_name(), "renderer ready");
                // `SystemInfo` learns the GPU and the screen.
                let adapter = gpu.adapter.get_info();
                let monitor = window.current_monitor();
                let (sw, sh) = monitor.as_ref().map_or((size.width, size.height), |m| {
                    let s = m.size();
                    (s.width, s.height)
                });
                let hz = monitor
                    .as_ref()
                    .and_then(|m| m.refresh_rate_millihertz())
                    .map_or(0.0, |mhz| f64::from(mhz) / 1000.0);
                self.info.set_system(serde_json::json!({
                    "graphicsDeviceName": adapter.name,
                    "graphicsDeviceVendorID": adapter.vendor,
                    "graphicsDeviceType": format!("{:?}", adapter.backend),
                    "graphicsDeviceKind": format!("{:?}", adapter.device_type),
                    "graphicsDriver": adapter.driver_info,
                    "screenWidth": sw,
                    "screenHeight": sh,
                    "screenRefreshRate": hz,
                    "dpi": window.scale_factor() * 96.0,
                }));
                self.info
                    .set("display", display::state(Some(&window), &self.display));
                if let Err(e) = self.scripts.eval(&self.info.script()) {
                    tracing::error!("system info: {e}");
                }
                let mut renderer = UiRenderer::new(&gpu);
                renderer.set_asset_root(&self.asset_root);
                let world = WorldRenderer::new(&gpu);
                self.gfx = Some(Gfx {
                    gpu,
                    surface,
                    window,
                    renderer,
                    world,
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
        // The mouse for gameplay code (`input.mousePosition`, buttons, look, wheel), in UI
        // pixels, whatever the UI does with it.
        let ui = self.scale() as f64;
        let game_mouse: Option<Vec<iron_core::Value>> = match &event {
            InputEvent::MouseMove { x, y } => {
                Some(vec!["mousemove".into(), (x / ui).into(), (y / ui).into()])
            }
            InputEvent::MouseDown { button } | InputEvent::MouseUp { button } => {
                let code = match button {
                    MouseButton::Left => "MouseLeft".to_owned(),
                    MouseButton::Right => "MouseRight".to_owned(),
                    MouseButton::Middle => "MouseMiddle".to_owned(),
                    MouseButton::Other(n) => format!("Mouse{n}"),
                };
                let down = matches!(event, InputEvent::MouseDown { .. });
                Some(if down {
                    vec!["keydown".into(), code.into(), "".into(), false.into()]
                } else {
                    vec!["keyup".into(), code.into(), "".into()]
                })
            }
            InputEvent::MouseWheel(delta) => {
                let (dx, dy) = match delta {
                    WheelDelta::Lines { x, y } => (f64::from(*x), f64::from(*y)),
                    // About a line per 40 pixels, like browsers.
                    WheelDelta::Pixels { x, y } => (x / 40.0, y / 40.0),
                };
                Some(vec!["wheel".into(), dx.into(), dy.into()])
            }
            InputEvent::MouseMotion { dx, dy } if self.focused => {
                Some(vec!["mousedelta".into(), (*dx).into(), (*dy).into()])
            }
            _ => None,
        };
        if let Some(args) = game_mouse {
            self.send_game_input(&args);
        }
        match event {
            InputEvent::Focused(f) => self.lifecycle.set_focused(f),
            InputEvent::Occluded(o) => self.lifecycle.set_occluded(o),
            InputEvent::Resized { width, height } => self.lifecycle.set_size(width, height),
            _ => {}
        }
        if matches!(
            event,
            InputEvent::Focused(_) | InputEvent::Occluded(_) | InputEvent::Resized { .. }
        ) {
            self.sync_lifecycle();
        }
        if let InputEvent::Focused(focused) = event {
            self.focused = focused;
            // Grabs are let go while the window is in the background.
            if focused {
                self.apply_cursor_lock();
            }
        }
        if let InputEvent::MouseMotion { .. } = event {
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
            InputEvent::MouseMove { x, y } => {
                self.pointer_px = Some((x as f32, y as f32));
                self.route_pointer();
                return;
            }
            InputEvent::MouseLeave => {
                self.pointer_px = None;
                self.handle_on(self.pointer_on, Input::PointerLeave);
                return;
            }
            InputEvent::MouseDown { button } => {
                self.buttons_down += 1;
                // Keys go to whatever was clicked last.
                self.keys_on = self.pointer_on;
                let time_ms = self.now_ms();
                self.handle_on(
                    self.pointer_on,
                    Input::PointerDown {
                        button: dom_button(button),
                        time_ms,
                    },
                );
                return;
            }
            InputEvent::MouseUp { button } => {
                self.handle_on(
                    self.pointer_on,
                    Input::PointerUp {
                        button: dom_button(button),
                    },
                );
                self.buttons_down = self.buttons_down.saturating_sub(1);
                // The pointer may be over something else now.
                if self.buttons_down == 0 {
                    self.route_pointer();
                }
                return;
            }
            InputEvent::MouseWheel(delta) => {
                let (dx, dy) = match delta {
                    WheelDelta::Lines { x, y } => (x * WHEEL_LINE, y * WHEEL_LINE),
                    WheelDelta::Pixels { x, y } => ((x / scale) as f32, (y / scale) as f32),
                };
                self.handle_on(self.pointer_on, Input::Wheel { dx, dy });
                return;
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
            InputEvent::Modifiers(m) => {
                let modifiers = events::Modifiers {
                    shift: m.shift,
                    ctrl: m.ctrl,
                    alt: m.alt,
                    meta: m.meta,
                };
                for surface in self.surfaces() {
                    self.handle_on(surface, Input::Modifiers(modifiers));
                }
                return;
            }
            // Releases outside an unfocused window may never arrive.
            InputEvent::Focused(false) => {
                self.buttons_down = 0;
                for surface in self.surfaces() {
                    self.handle_on(surface, Input::WindowBlur);
                }
                if let Some(args) = game_key {
                    self.send_game_input(&args);
                }
                return;
            }
            InputEvent::Ime(ImeEvent::Preedit(text, cursor)) => Input::ImePreedit { text, cursor },
            InputEvent::Ime(ImeEvent::Commit(text)) => Input::ImeCommit(text),
            InputEvent::Ime(ImeEvent::Disabled) => Input::ImePreedit {
                text: String::new(),
                cursor: None,
            },
            _ => return,
        };
        // Keys and IME text go to the surface clicked last.
        self.handle_on(self.keys_on, input);
        // Keys typed into a text field are the field's, not the game's.
        let typing = match self.keys_on {
            Surface::Screen => self
                .interaction
                .focused()
                .is_some_and(|id| self.ui.is_editor(id)),
            Surface::Panel(id) => self
                .panels
                .map
                .get(&id)
                .is_some_and(|p| p.interaction.focused().is_some_and(|f| p.tree.is_editor(f))),
        };
        if let Some(args) = game_key.filter(|_| !typing) {
            self.send_game_input(&args);
        }
    }

    fn frame(&mut self) {
        self.tick();
        // Last-moment changes before drawing (`onPreRender`).
        if !self.lifecycle.paused() {
            let scene = &self.scene;
            self.components
                .pre_render(&mut self.scripts, &|o| scene.active(o));
            self.apply_script_output();
        }
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
        // Scripts' `Screen`: the size in UI pixels (what pointer events use) and the scale.
        let screen = (size.width as f32 / scale, size.height as f32 / scale, scale);
        if self.screen != screen {
            self.screen = screen;
            let args = [screen.0, screen.1, screen.2].map(|v| iron_core::Value::Number(v.into()));
            // Games without @iron/engine's Screen don't have the hook.
            let _ = self.scripts.call_global("__iron_screen", &args);
        }
        // `@media (resolution)` and `(prefers-color-scheme)` follow the window.
        let dark = gfx.window.theme() == Some(iron_platform::Theme::Dark);
        self.ui.set_media_env(iron_ui::media::MediaEnv {
            resolution: scale,
            dark,
            ..self.ui.media_env()
        });
        load_images(&mut self.ui, &mut gfx.renderer, &gfx.gpu);
        let cmds = self.ui.draw_list();
        self.scene.scene.update_world();
        for (id, data) in self.world_view.mesh_uploads.drain(..) {
            match data {
                Some(data) => {
                    gfx.world.set_mesh(&gfx.gpu, id, &data);
                }
                None => gfx.world.remove_mesh(id),
            }
        }
        gfx.world.bloom = self.world_view.bloom;
        gfx.world.grading = self.world_view.grading;
        (
            gfx.world.shadow_distance,
            gfx.world.shadow_resolution,
            gfx.world.shadow_cascades,
        ) = self.world_view.shadow_settings;
        gfx.world.set_sky(self.world_view.sky);
        let q = self.world_view.quality;
        gfx.surface.set_vsync(&gfx.gpu, q.vsync);
        gfx.world.msaa = q.msaa;
        gfx.world.render_scale = q.render_scale;
        gfx.world.shadow_quality = q.shadows;
        gfx.world.soft_shadow_radius = q.soft_radius;
        gfx.world.anisotropic = q.anisotropic;
        gfx.world.texture_mip_limit = q.texture_mip_limit;
        (gfx.world.environment_intensity, gfx.world.ambient_from_sky) = self.world_view.environment;
        for (id, data) in self.world_view.texture_uploads.drain(..) {
            match data {
                Some(data) => {
                    gfx.world.set_texture(&gfx.gpu, id, &data);
                }
                None => gfx.world.remove_texture(id),
            }
        }
        for (id, size) in self.world_view.render_texture_uploads.drain(..) {
            gfx.world.set_render_texture(&gfx.gpu, id, size);
        }
        let now = self.started.elapsed().as_secs_f64() * 1000.0;
        let (scene, world_view) = (&self.scene, &self.world_view);
        let Gfx {
            gpu,
            surface,
            renderer,
            world,
            ..
        } = gfx;
        // World panels draw into their textures first.
        let (quads, textures) = self
            .panels
            .render(gpu, renderer, scene, now, |tree, renderer| {
                load_images(tree, renderer, gpu)
            });
        let textures: Vec<&wgpu::TextureView> = textures.iter().collect();
        surface.present_with(gpu, |view, format, size| {
            let target = RenderTarget {
                view,
                format,
                size,
                scale,
            };
            draw_frame(
                gpu,
                renderer,
                world,
                &target,
                &world_view.views(scene, size, &quads),
                &textures,
                &cmds,
            );
        });
        self.world_view.probes_rendered();

        self.frames += 1;
        // `IRON_RELOAD_AT_FRAME=N`: a screenshot run reloads its scripts after frame N,
        // to check hot reload without a window.
        if self.screenshot.is_some()
            && std::env::var("IRON_RELOAD_AT_FRAME")
                .ok()
                .and_then(|n| n.parse::<u32>().ok())
                == Some(self.frames)
        {
            self.reload_scripts();
        }
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

    fn frame_interval(&self) -> Option<Duration> {
        lifecycle::frame_cap(
            self.target_frame_rate,
            self.world_view.quality.vsync,
            self.screenshot.is_some(),
        )
    }

    fn paused(&self) -> bool {
        self.lifecycle.paused()
    }

    fn idle(&mut self) {
        // Hot reload still picks up rebuilds; they show once the game resumes.
        if self
            .watcher
            .as_mut()
            .is_some_and(|w| w.poll(Instant::now()))
        {
            self.reload_scripts();
        }
    }
}

impl Drop for App {
    /// The player's settings are written when the game closes, like Unity's.
    fn drop(&mut self) {
        self.prefs.save();
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

/// How scripts hold the cursor (Unity's `CursorLockMode`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum CursorLock {
    #[default]
    None,
    /// Hidden and held in place; raw motion still arrives (mouse look).
    Locked,
    /// Kept inside the window.
    Confined,
}

/// Scripts' `Cursor` settings.
#[derive(Clone, Debug)]
struct ScriptCursor {
    visible: bool,
    lock: CursorLock,
    /// Shown where the UI asks for no particular cursor.
    image: Option<iron_ui::CursorImage>,
}

impl Default for ScriptCursor {
    fn default() -> Self {
        Self {
            visible: true,
            lock: CursorLock::None,
            image: None,
        }
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

/// A UI that takes input: the screen's, or a world panel's (by its id).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Surface {
    Screen,
    Panel(u32),
}

/// The key `game.css` has among the stylesheets every UI tree gets.
const GAME_CSS: u32 = u32::MAX;

/// `game.css` as a stylesheet source for panel trees.
fn game_css_source(root: &Path, css_path: &Path) -> Option<(u32, String, String, String)> {
    let css = std::fs::read_to_string(css_path).ok()?;
    let base = css_path
        .parent()
        .and_then(|dir| dir.strip_prefix(root).ok())
        .map(|rel| rel.to_string_lossy().replace('\\', "/"))
        .unwrap_or_default();
    Some((GAME_CSS, css, base, css_path.display().to_string()))
}

/// A UI tree for a new world panel, with the fonts and stylesheets the screen has.
fn new_panel_tree(
    root: &Path,
    fonts: &[(String, String, Option<f32>, Option<bool>)],
    sheets: &[(u32, String, String, String)],
) -> panels::NewTree {
    let mut tree = UiTree::with_system_fonts(1.0, 1.0);
    for (family, src, weight, italic) in fonts {
        load_font(&mut tree, root, family, src, *weight, *italic);
    }
    let ids = sheets
        .iter()
        .map(|(key, css, base, source)| (*key, add_stylesheet(&mut tree, root, css, base, source)))
        .collect();
    (tree, ids)
}

/// The starting components: one GameObject per registered class (not `spawn: false`),
/// named after it, holding that component.
fn spawn_starting_components(
    rt: &mut QuickJsRuntime,
    host: &mut ComponentHost,
    scene: &mut iron_scene::ScriptScene,
    classes: &[String],
) -> Result<(), iron_script::ScriptError> {
    for class in classes {
        let object = scene.create_engine_object(class);
        host.spawn_on(rt, class, Some(object), class)?;
    }
    Ok(())
}

/// Loads the stylesheet Bun writes next to the bundle (game.js -> game.css), if any.
/// A dev build's vendor bundle: `game.js` starts with `// @iron-dev` and `vendor.js`
/// sits next to it (`None` for a normal, self-contained build).
fn dev_vendor(bundle_path: &Path, code: &str) -> Option<String> {
    if !code.starts_with("// @iron-dev") {
        return None;
    }
    let path = bundle_path.with_file_name("vendor.js");
    std::fs::read_to_string(&path)
        .inspect_err(|e| tracing::error!("dev build without {}: {e}", path.display()))
        .ok()
}

/// Loads a game: its one bundle, or a dev build's vendor bundle and then its code.
fn load_bundles(
    rt: &mut QuickJsRuntime,
    vendor: Option<&str>,
    name: &str,
    code: &str,
) -> Result<(), iron_script::ScriptError> {
    match vendor {
        Some(vendor) => {
            rt.load_bundle("vendor.js", vendor)?;
            iron_script::load_game(rt, name, code)
        }
        None => rt.load_bundle(name, code),
    }
}

fn load_game_css(ui: &mut UiTree, root: &Path, css_path: &Path) -> Option<u32> {
    let css = std::fs::read_to_string(css_path).ok()?;
    let base = css_path
        .parent()
        .and_then(|dir| dir.strip_prefix(root).ok())
        .map(|rel| rel.to_string_lossy().replace('\\', "/"))
        .unwrap_or_default();
    let id = add_stylesheet(ui, root, &css, &base, &css_path.display().to_string());
    tracing::info!("loaded stylesheet {}", css_path.display());
    Some(id)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<Args> {
        parse_args_from(args.iter().map(std::ffi::OsString::from))
    }

    // M5-APP-03
    #[test]
    fn game_folder_defaults_to_the_current_folder() {
        let args = parse(&[]).unwrap();
        assert_eq!(args.game, PathBuf::from("."));
        let args = parse(&["--screenshot", "out.png", "--frames", "5"]).unwrap();
        assert_eq!(args.game, PathBuf::from("."));
        assert_eq!(args.screenshot, Some(PathBuf::from("out.png")));
        assert_eq!(args.frames, 5);
    }

    // M5-APP-03
    #[test]
    fn game_folder_can_come_before_or_after_options() {
        assert_eq!(
            parse(&["examples/css"]).unwrap().game,
            PathBuf::from("examples/css")
        );
        let args = parse(&["--frames", "3", "examples/css"]).unwrap();
        assert_eq!(args.game, PathBuf::from("examples/css"));
        assert!(parse(&["a", "b"]).is_err());
        assert!(parse(&["--nope"]).is_err());
    }
}
