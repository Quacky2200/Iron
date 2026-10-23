// SPDX-License-Identifier: Apache-2.0

// A windowed program: no console window of its own (the log goes to a file, see
// `logging`); tests keep the console harness.
#![cfg_attr(not(test), windows_subsystem = "windows")]
//! `iron [game] [--screenshot out.png] [--frames N]`: opens a window, loads the
//! game's script bundle and runs it.
//!
//! `[game]` is either a game folder (loads `<game>/dist/game.irbundle`, built with
//! `iron-build build`; a QuickJS `dist/game.js` is deprecated and needs `--force`) or a bundle directly; without one, the current folder is the game. `--screenshot` saves the UI after
//! `N` frames (default 60) and exits, which is handy for checking output.

pub(crate) mod anim_cull;
pub(crate) mod asset_db;
mod asset_root;
mod bench;
pub(crate) mod bone_cache;
mod cloth_sync;
pub(crate) mod cook;
mod cursor;
mod display;
mod editor;
mod file_drop;
mod fps;
mod frame_target;
mod game_input;
// `pub(crate)`: `iron_editor` (bin/iron_editor.rs) includes this file as a module and
// re-exports these at its root, for the `crate::` paths between modules.
pub(crate) mod info;
mod lifecycle;
mod logging;
pub(crate) mod models;
mod native;
mod native_pump;
mod native_render;
mod panels;
pub(crate) mod particles;
mod payload;
mod physics_sync;
mod prefs;
mod project;
pub(crate) mod reload;
mod render_sync;
pub(crate) mod sync_profile;
mod ui_scale;
mod windows;
pub(crate) mod world_view;

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
use iron_render::{Color, Gpu, OffscreenTarget, RenderTarget, UiRenderer, WorldRenderer};
use iron_script::{ComponentHost, QuickJsRuntime, ScriptRuntime};
use iron_ui::{
    Clipboard, Input, Interaction, ScrollAlign, ScrollBehavior, UiEvent, UiTree, events,
};

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
    /// Cook the game's assets (`--cook`) and exit.
    cook: bool,
    /// `--fps` / `--no-fps`: the FPS counter on or off, over the defaults.
    fps: Option<bool>,
    /// `--data <dir>`: where a packaged game's assets are (over `$IRON_DATA` and `data/` beside
    /// the exe).
    data: Option<PathBuf>,
    /// The folder assets are read from when it is not the game folder (a packaged game).
    asset_root: Option<PathBuf>,
    /// `--force`: run a deprecated QuickJS game anyway.
    force: bool,
    /// Everything after `--`, for the game's scripts (`Application.args`).
    script_args: Vec<String>,
    /// `--bench`: a windowless timed run (see `bench.rs`).
    bench: Option<bench::BenchArgs>,
}

impl Args {
    /// No player: a screenshot or bench run.
    fn offline(&self) -> bool {
        self.screenshot.is_some() || self.bench.is_some()
    }
}

/// The deprecation note for the QuickJS backend (logged once per run).
const QUICKJS_DEPRECATED: &str = "the QuickJS backend is deprecated: build games with `iron-build build` (iron_ts, native, writes dist/game.irbundle); pass --force to run this QuickJS bundle anyway";

/// What to do with a bundle: native ones always run; QuickJS ones only with `--force` (the
/// editor, itself QuickJS-based, counts as forced).
#[derive(Debug, PartialEq, Eq)]
enum QuickJsGate {
    Native,
    Forced,
    Refused,
}

fn quickjs_gate(native: bool, force: bool, editor: bool) -> QuickJsGate {
    if native {
        QuickJsGate::Native
    } else if force || editor {
        QuickJsGate::Forced
    } else {
        QuickJsGate::Refused
    }
}

fn parse_args() -> Result<Args> {
    let mut args = parse_args_from(std::env::args_os().skip(1))?;
    // `iron_editor [project]`: the editor runs its own UI on the project folder.
    if editor::ENABLED {
        editor::set_project(&args.game);
        args.game = editor::ui_folder();
    } else if let Some(dir) = payload::embedded_game() {
        // A packaged single-file game runs its own code (extracted from the exe), whatever the
        // arguments say, and reads its assets from the data folder beside the exe.
        let exe = std::env::current_exe().ok();
        let env = std::env::var_os("IRON_DATA");
        args.asset_root = Some(
            asset_root::locate_data(&asset_root::DataSearch {
                flag: args.data.as_deref(),
                env: env.as_deref(),
                exe_dir: exe.as_deref().and_then(Path::parent),
                code_dir: &dir,
            })
            .map_err(anyhow::Error::msg)?,
        );
        args.game = dir;
    }
    Ok(args)
}

/// Parses the arguments after the program name. The game is the first argument that isn't
/// an option, or the current folder when there's none.
fn parse_args_from(args: impl IntoIterator<Item = std::ffi::OsString>) -> Result<Args> {
    let usage = "usage: iron [game folder | bundle.js] [--screenshot out.png] [--frames N] [--cook] [--fps | --no-fps] [--data folder] [--force] [-- script args]
       iron [game] --bench [--bench-input spec.json] [--bench-out out.json] [--warmup N] [--frames N] [--fixed-dt seconds] [--size WxH]
--force runs a QuickJS (deprecated) bundle; games built with `iron-build build` are native; arguments after -- reach scripts as Application.args;
--bench runs without a window as fast as it can and writes timings as JSON (docs/iron-build.md, \"Benchmarks\")";
    let mut args = args.into_iter();
    let mut game = None;
    let mut parsed = Args {
        game: PathBuf::from("."),
        screenshot: None,
        frames: 60,
        cook: false,
        fps: None,
        data: None,
        asset_root: None,
        force: false,
        script_args: Vec::new(),
        bench: None,
    };
    let mut frames_given = false;
    while let Some(arg) = args.next() {
        if let Some(flag) = arg.to_str().filter(|s| bench::is_flag(s)) {
            let bench = parsed.bench.get_or_insert_with(bench::BenchArgs::default);
            bench.flag(flag, &mut args).context(usage)?;
            continue;
        }
        match arg.to_str() {
            Some("--data") => parsed.data = Some(args.next().context(usage)?.into()),
            Some("--screenshot") => parsed.screenshot = Some(args.next().context(usage)?.into()),
            Some("--cook") => parsed.cook = true,
            Some("--force") => parsed.force = true,
            Some("--fps") => parsed.fps = Some(true),
            Some("--no-fps") => parsed.fps = Some(false),
            Some("--") => {
                parsed.script_args = args.map(|a| a.to_string_lossy().into_owned()).collect();
                break;
            }
            Some("--frames") => {
                frames_given = true;
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
    // `--frames` is the measured frames of a bench run without an input file.
    if let Some(bench) = parsed.bench.as_mut()
        && frames_given
    {
        bench.frames = parsed.frames;
    }
    Ok(parsed)
}

/// The process's exit code, set by `Application.quit(code)`.
static EXIT_CODE: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(0);

pub(crate) fn main() -> Result<()> {
    bench::mark_start();
    logging::attach_console();
    // The editor's console shows the log from the start.
    logging::capture(editor::ENABLED);
    let result = run();
    if let Err(e) = &result {
        // Into the log as well as stderr: a windowed program's stderr may go nowhere.
        tracing::error!("{e:#}");
    }
    let code = EXIT_CODE.load(std::sync::atomic::Ordering::Relaxed);
    if result.is_ok() && code != 0 {
        std::process::exit(code);
    }
    result
}

/// Threads the animation update poses animators on: `IRON_ANIM_THREADS` (1: in turn), else
/// the machine's threads, up to 16 (more gain little: the writing stays in turn).
fn animation_threads() -> usize {
    std::env::var("IRON_ANIM_THREADS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or_else(|| std::thread::available_parallelism().map_or(1, |n| n.get().min(16)))
}

fn run() -> Result<()> {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            logging::init(None);
            return Err(e);
        }
    };
    // `--bench` reports where start-up goes (`startupParts`).
    if args.bench.is_some() {
        iron_ts::timing::enable();
    }
    iron_animation::set_threads(animation_threads());
    let mut lap = iron_ts::timing::Lap::start();
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
        .filter(|_| !args.offline());
    if let Some(log) = logging::init(logging::log_path(settings.as_deref())) {
        tracing::info!("log: {}", log.display());
    }
    if args.cook {
        // Cooking needs only the folder, not a build.
        let root = if args.game.is_dir() {
            args.game.clone()
        } else {
            args.game.parent().unwrap_or(Path::new(".")).to_path_buf()
        };
        let report = cook::cook_game(&root);
        tracing::info!(
            "cooked {} textures and {} models into {}",
            report.textures,
            report.models,
            root.join(".iron/cooked").display()
        );
        for f in &report.failed {
            tracing::warn!("not cooked: {f}");
        }
        return Ok(());
    }

    // The code folder (bundle, `package.json`, icon) and the assets' folder: the same for a game
    // folder, apart for a packaged exe (`--data` also sets it for a folder).
    let (bundle_path, code_root) = resolve_bundle(&args.game)?;
    match quickjs_gate(native::is_native(&bundle_path), args.force, editor::ENABLED) {
        QuickJsGate::Native => {}
        QuickJsGate::Forced => tracing::warn!("{QUICKJS_DEPRECATED}"),
        QuickJsGate::Refused => {
            tracing::warn!("{QUICKJS_DEPRECATED}");
            bail!(
                "{} is a QuickJS bundle (deprecated); rebuild with `iron-build build` or pass --force",
                bundle_path.display()
            );
        }
    }
    let asset_root = asset_root::AssetRoot::new(
        args.asset_root
            .clone()
            .or_else(|| args.data.clone())
            .unwrap_or_else(|| code_root.clone()),
    )
    .path()
    .to_path_buf();
    // Every asset read goes through this: the packs of a packaged game (all mounted now, so a
    // script can load any path), else the folder.
    let vfs = asset_root::mount(&asset_root).map_err(anyhow::Error::msg)?;
    iron_core::set_asset_fs(Some(std::sync::Arc::new(vfs.clone())));
    // Dev loop: `iron-build build` lists what a packaged build would hold; loading a file outside
    // it logs a warning (a packaged game would not find it).
    if let Ok(plan) = std::fs::read_to_string(code_root.join("dist/assets.plan")) {
        iron_core::set_pack_plan(plan.lines().filter(|l| !l.is_empty()).map(str::to_owned));
    }
    lap.mark("log, manifest, asset mount");
    // A compiled (native) game is JIT-compiled below; there is no JavaScript to read.
    let native_bundle = if native::is_native(&bundle_path) {
        Some(native::Bundle::read(&bundle_path)?)
    } else {
        None
    };
    lap.mark("bundle read");
    let code = if native_bundle.is_some() {
        String::new()
    } else {
        std::fs::read_to_string(&bundle_path)
            .with_context(|| format!("reading {}", bundle_path.display()))?
    };
    let name = bundle_path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "game.js".into());

    // The game's manifest and the machine, for `Application` and `SystemInfo`, before
    // the bundle's top level runs.
    let manifest = info::Manifest::load(&code_root);
    // `Window.main` starts with the manifest's title; the real size and scale come with the window.
    iron_engine_api::window::set_main(&manifest.title, (1280.0, 720.0), 1.0);
    iron_engine_api::window::set_supported(iron_platform::Os::current().supports_windows());
    let mut info = info::Info::new(&manifest);
    info.set_app("args", serde_json::json!(args.script_args));
    // The player's saved settings (not for screenshot runs, which stay the same every
    // time) and the window mode: the manifest's default, then the player's choice.
    let offline = args.offline();
    let prefs = if offline {
        prefs::Prefs::in_memory()
    } else {
        prefs::Prefs::load(&manifest)
    };
    // A game that keeps a fixed size (`iron.display.resizable: false`) always gets its own
    // settings: saved ones would tell its scripts about a window size it never has.
    let resizable = manifest.display["resizable"].as_bool().unwrap_or(true);
    let defaults = display::from_json(Some(&manifest.display), Default::default());
    let display_settings = if resizable {
        display::from_json(prefs.get("iron.display"), defaults)
    } else {
        defaults
    };
    if let Some(path) = prefs.path() {
        tracing::info!("player prefs: {}", path.display());
    }
    info.set("prefs", serde_json::Value::Object(prefs.all().clone()));
    // Assets' GUIDs and importer settings from their `.meta` files.
    info.set("assets", asset_db::scan_vfs(&vfs, &asset_root));
    info.set("assetIndex", asset_db::index(&vfs));
    native_pump::seed_asset_index(&info);
    info.set("display", display::state(None, &display_settings));
    lap.mark("prefs, asset index (.meta files)");
    let mut scripts = QuickJsRuntime::new();
    // The native API (`Window`, `Shapes`, `Spline`, ...) for bundled (JavaScript) games: the same
    // functions compiled games call. Compiled games have them natively.
    if !native::is_native(&bundle_path) {
        iron_native::install(&mut scripts).map_err(|e| anyhow::anyhow!("{e}"))?;
    }
    scripts
        .eval(&info.script())
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    // A dev build (`scripts/dev.ts`) comes as vendor.js plus the game's own code.
    let vendor = dev_vendor(&bundle_path, &code);
    if native_bundle.is_none() {
        load_bundles(&mut scripts, vendor.as_deref(), &name, &code)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
    }

    // One GameObject per starting component class (registered without
    // `spawn: false`), each with that component; a start scene (`iron.startScene`)
    // is loaded by the scripts themselves.
    let mut components = ComponentHost::new();
    let mut scene = iron_scene::ScriptScene::new();
    if native_bundle.is_none() {
        let classes =
            ComponentHost::registered_classes(&mut scripts).map_err(|e| anyhow::anyhow!("{e}"))?;
        spawn_starting_components(&mut scripts, &mut components, &mut scene, &classes)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        tracing::info!(?classes, "loaded {}", bundle_path.display());
    }
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

    lap.mark("QuickJS runtime (info script; JS games: their bundle)");
    // Bun writes CSS imported by the game next to the bundle (game.js -> game.css).
    let mut ui = UiTree::with_system_fonts(1.0, 1.0);
    lap.mark("UI tree and system fonts");
    let css_path = bundle_path.with_extension("css");
    let game_css = load_game_css(&mut ui, &asset_root, &code_root, &css_path);
    let game_css_source = game_css_source(&code_root, &css_path);
    lap.mark("game CSS");
    let native = match native_bundle {
        Some(bundle) => {
            native_render::configure_engine(&asset_root);
            native_pump::seed_prefs(&prefs);
            let (m, i) = (manifest.clone(), info.clone());
            let mut n = native::Native::load_with(
                bundle,
                &mut scene.scene,
                &mut ui,
                Box::new(move || native_pump::configure_engine(&m, &i)),
            )?;
            n.set_root(&asset_root);
            if let Some(start) = &manifest.start_scene {
                n.start_scene(&mut scene.scene, &mut ui, start);
            }
            Some(n)
        }
        None => None,
    };
    let mut lap = iron_ts::timing::Lap::start();

    // Rebuilds by `bun build --watch` reload the scripts (not for screenshot or bench runs).
    let watcher = if offline {
        None
    } else {
        reload::BundleWatcher::new(vec![
            bundle_path.clone(),
            css_path.clone(),
            bundle_path.with_file_name("vendor.js"),
            // `iron-build dev` writes this after the bundle (or with the error of a failed build).
            bundle_path.with_file_name(".iron-reload"),
        ])
        .inspect_err(|e| tracing::warn!("not watching for script changes: {e}"))
        .ok()
    };
    // Asset files that change (textures, models, sounds) load again in place. Packs are
    // read-only: only loose files are watched.
    let asset_watcher = if offline || vfs.root().is_none() {
        None
    } else {
        reload::AssetWatcher::new(&asset_root)
            .inspect_err(|e| tracing::warn!("not watching for asset changes: {e}"))
            .ok()
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
        native,
        scene,
        scene_events,
        world_view: world_view::WorldViews::with_physical_units(manifest.physical_units),
        panels: panels::Panels::default(),
        windows: windows::AppWindows::new(),
        domains: windows::Domains::default(),
        reload_claims_pending: false,
        window_commands: Vec::new(),
        windows_to_show: Vec::new(),
        main_focus_sent: None,
        pointer_on: Surface::Screen,
        keys_on: Surface::Screen,
        buttons_down: 0,
        pointer_px: None,
        sheet_sources: game_css_source.into_iter().collect(),
        font_sources: Vec::new(),
        screen: (0.0, 0.0, 0.0),
        render_stats: iron_render::WorldStats::default(),
        exposure_sent: HashMap::new(),
        ui,
        sheets: HashMap::new(),
        interaction: Interaction::new(),
        ui_events,
        game_input,
        physics: iron_physics::Physics::new(),
        attachments: physics_sync::Attachments::default(),
        cloth: cloth_sync::ClothSystem::default(),
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
        vfs,
        code_root: code_root.clone(),
        gfx: None,
        started: Instant::now(),
        clock: lifecycle::FixedClock::default(),
        lifecycle: lifecycle::Lifecycle::new(manifest.run_in_background, offline),
        physical_units: manifest.physical_units,
        pipeline_warned: false,
        frames: 0,
        fps_on: fps::enabled(
            args.fps.or((editor::ENABLED && !offline).then_some(false)),
            offline,
            std::env::var("IRON_FPS").ok().as_deref(),
            manifest.fps,
        ),
        fps: fps::FpsCounter::default(),
        fps_overlay: fps::FpsOverlay::new(),
        screenshot: args.screenshot.map(|path| (path, args.frames)),
        offline,
        fake_now: None,
        script_time: Duration::ZERO,
        render_time: Duration::ZERO,
        sync_time: Duration::ZERO,
        ops_time: Duration::ZERO,
        done: false,
        info,
        editor: editor::EditorHost::default(),
        file_drag: file_drop::FileDrag::default(),
    };
    let icon = manifest
        .icon
        .as_deref()
        .and_then(|path| info::load_icon(&code_root, path));
    // A window opens at the saved window size (a fixed one at its own); fullscreen modes are
    // applied once it exists.
    let (width, height) = (
        display_settings.window_width,
        display_settings.window_height,
    );
    lap.mark("app set-up");
    // A bench run makes no OS window at all: it draws offscreen, its own loop drives the frames.
    if let Some(bench) = args.bench {
        let size = bench.size.unwrap_or((width, height));
        return bench::run(app, &bench, size, &manifest.title);
    }
    // Screenshot runs make no OS windows for secondary windows: they draw offscreen.
    let virtual_windows = app.screenshot.is_some();
    Platform::new().run(
        WindowConfig {
            title: manifest.title.clone(),
            icon,
            width,
            height,
            virtual_windows,
            resizable,
        },
        app,
    )?;
    Ok(())
}

/// The bundle to load and the folder its assets (`url(...)` images) live in.
fn resolve_bundle(path: &Path) -> Result<(PathBuf, PathBuf)> {
    if path.is_dir() {
        // A game built for the native backend has an `.irbundle`.
        let native = path.join("dist").join("game.irbundle");
        if native.is_file() {
            return Ok((native, path.to_path_buf()));
        }
        let bundle = path.join("dist").join("game.js");
        if !bundle.is_file() {
            bail!(
                "{} not found; build it first with `iron-build build {}`",
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
    surface: frame_target::FrameTarget,
    /// The OS window; `None` for a windowless `--bench` run.
    window: Option<Arc<Window>>,
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
    /// The world renderer's last stats, as last told to scripts (`RenderStats`).
    render_stats: iron_render::WorldStats,
    /// Each camera's exposure as last sent to scripts (`Camera.exposure`).
    exposure_sent: HashMap<u32, (f32, f32)>,
    /// React UI on planes in the world (`WorldPanel`s), each with its own tree.
    panels: panels::Panels,
    /// Secondary windows (`Window.create`), each with its own tree, and which windows have focus.
    windows: windows::AppWindows,
    /// Isolated event domains (families of windows under a root other than the main window).
    domains: windows::Domains,
    /// A reload happened: after the new code's first frame, keyed windows nobody claimed close.
    reload_claims_pending: bool,
    /// Platform commands scripts' window requests became, and the windows drawn for the first time.
    window_commands: Vec<iron_platform::WindowCommand>,
    windows_to_show: Vec<iron_platform::WindowKey>,
    /// The game's focus as last given to the lifecycle (any of its windows focused).
    main_focus_sent: Option<bool>,
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
    /// A compiled game, when the folder has a native bundle (then `scripts` and
    /// `components` stay empty).
    native: Option<native::Native>,
    ui: UiTree,
    /// Where textures, models, sounds, fonts and scenes are read from.
    asset_root: PathBuf,
    /// The assets' file system (also installed for the loaders through `iron_core`).
    vfs: iron_pack::Vfs,
    /// Where the game's code, `package.json` and CSS are (the asset root, except in a packaged exe).
    code_root: PathBuf,
    gfx: Option<Gfx>,
    started: Instant,
    clock: lifecycle::FixedClock,
    /// Focus, and pausing in the background (`Application.runInBackground`).
    lifecycle: lifecycle::Lifecycle,
    frames: u32,
    screenshot: Option<(PathBuf, u32)>,
    /// A run without a player (`--screenshot`, `--bench`): no saved prefs, file watching, frame
    /// cap or pausing, and a silent mixer.
    offline: bool,
    /// The clock of a `--bench --fixed-dt` run (each frame exactly one step); `None`: real time.
    fake_now: Option<Instant>,
    /// Time spent in game code this frame (native or QuickJS hooks), and drawing it, for `--bench`.
    script_time: Duration,
    render_time: Duration,
    /// A compiled game's per-frame sync of its registries into the renderer
    /// (`sync_native_render`: animation, splines, cameras, `WorldViews` ops).
    sync_time: Duration,
    /// Applying what scripts queued (React commits, ops: for a QuickJS game the `render.*`,
    /// `scene.*`, ... ops), for `--bench`.
    ops_time: Duration,
    /// The FPS counter overlay (drawn last, at window resolution).
    fps_on: bool,
    fps: fps::FpsCounter,
    fps_overlay: fps::FpsOverlay,
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
    /// `Cloth` components' simulations.
    cloth: cloth_sync::ClothSystem,
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
    /// Real-world light units (`iron.lightUnits`), and whether the quality level has warned
    /// about leaving out light shafts the game asked for yet.
    physical_units: bool,
    pipeline_warned: bool,
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
    /// The editor's log feed and `editor.*` ops (refused outside `iron_editor`).
    editor: editor::EditorHost,
    /// Files dragged onto the window from the system.
    file_drag: file_drop::FileDrag,
    done: bool,
}

impl App {
    /// Sizes the UI to the window (in UI pixels: the window size divided by its DPI and UI
    /// scale) and tells scripts' `Screen` when that changes. Nothing before the window
    /// exists.
    fn sync_screen(&mut self) {
        let Some(gfx) = &self.gfx else {
            return;
        };
        let scale = ui_factor(gfx, self.ui_scale);
        let (width, height) = gfx.inner_size();
        self.ui
            .set_size(width as f32 / scale, height as f32 / scale);
        // Scripts' `Screen`: the size in UI pixels (what pointer events use) and the scale.
        let screen = (width as f32 / scale, height as f32 / scale, scale);
        if self.screen != screen {
            self.screen = screen;
            let args = [screen.0, screen.1, screen.2].map(|v| iron_core::Value::Number(v.into()));
            // Games without @iron/engine's Screen don't have the hook.
            let _ = self.scripts.call_global("__iron_screen", &args);
        }
    }

    /// Hot reload (DESIGN.md §6.1): loads the rebuilt bundle into a new runtime and
    /// swaps it in, or keeps the old one running if it fails. For now the world is
    /// wiped too: UI, script stylesheets, physics and audio start over, and components
    /// run `onStart` again (state survives once GameObjects exist, TODO.md M5).
    fn reload_scripts(&mut self) {
        let started = Instant::now();
        if self.native.is_some() {
            self.reload_native(started);
            return;
        }
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
        // The old code's window listeners go; windows with a key stay for the new code to claim.
        let unowned = iron_engine_api::window::reload_begin();
        let mut fresh = QuickJsRuntime::new();
        if let Err(e) = iron_native::install(&mut fresh) {
            tracing::error!("reload: {e}");
        }
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
                iron_engine_api::window::reload_abort();
                tracing::error!("reload failed, keeping the old code: {e}");
                return;
            }
        };

        self.vendor = vendor;
        // Everything the old scripts built. The new bundle's top level has already
        // queued its UI and ops; they apply after this.
        self.ui.clear_script_nodes();
        for w in self.windows.map.values_mut() {
            w.tree.clear_script_nodes();
        }
        self.reload_windows_closed(unowned.into_iter().map(|w| w.0).collect());
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
        self.world_view = world_view::WorldViews::with_physical_units(self.physical_units);
        // Tell the new scripts the screen size on the next frame.
        self.screen = (0.0, 0.0, 0.0);
        self.physics = iron_physics::Physics::new();
        self.attachments = physics_sync::Attachments::default();
        self.cloth = cloth_sync::ClothSystem::default();
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

    /// Hot reload of a compiled game: the bundle is compiled again; a failure keeps the old one.
    fn reload_native(&mut self, started: Instant) {
        // A failed build leaves the old bundle: say why and keep running it.
        if let Some(error) = native::failed_build(&self.bundle_path) {
            tracing::error!("build failed, keeping the old code: {error}");
            return;
        }
        let bundle = match native::Bundle::read(&self.bundle_path) {
            Ok(b) => b,
            Err(e) => {
                tracing::error!("reload: {e:#}");
                return;
            }
        };
        let Some(n) = self.native.as_mut() else {
            return;
        };
        let result = n.reload(bundle, &mut self.scene.scene, &mut self.ui);
        // The scene was replaced: the renderer's script ids pointed into the old one.
        self.scene.clear_ids();
        self.reset_native_render();
        self.close_reload_windows();
        self.native_after();
        match result {
            Ok(()) => tracing::info!(
                "reloaded {} in {:.1} ms",
                self.bundle_name,
                started.elapsed().as_secs_f64() * 1000.0
            ),
            Err(e) => tracing::error!("reload failed, keeping the old code: {e:#}"),
        }
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
        self.game_css = load_game_css(&mut self.ui, &self.asset_root, &self.code_root, &css_path);
        self.sheet_sources = game_css_source(&self.code_root, &css_path)
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
            // `Window.onDomainFocusChanged` of the main game.
            {
                let _scope = iron_scene::api::enter(&mut self.scene.scene);
                iron_engine_api::window::domain_focus_changed(
                    iron_engine_api::window::DomainId::MAIN,
                    focused,
                    paused,
                );
            }
            if let Some(n) = self.native.as_mut() {
                n.focus_changed(&mut self.scene.scene, &mut self.ui, focused, paused);
                self.native_after();
            }
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
        let now = self.now();
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
            // Script time leaves out applying what scripts asked for (`native_after`,
            // `apply_script_output`): that is engine work for both backends.
            if let Some(n) = self.native.as_mut() {
                let t = Instant::now();
                n.fixed_tick(&mut self.scene.scene, &mut self.ui, TICK);
                self.script_time += t.elapsed();
                self.native_after();
            }
            let t = Instant::now();
            let scene = &self.scene;
            self.components
                .fixed_update(&mut self.scripts, TICK, &|o| scene.active(o));
            self.script_time += t.elapsed();
            // What the components asked for applies before physics steps.
            self.apply_script_output();
            self.step_physics();
            if !self.cloth.is_empty() {
                let gravity = iron_core::glam::Vec3::from(self.physics.gravity());
                let scene = &self.scene;
                self.cloth.retain(|o| scene.object(o).is_some());
                self.cloth
                    .step(TICK as f32, gravity, &self.scene, &mut self.world_view);
            }
            self.send_scene_moves();
        }

        // Script time: timers and their microtasks, once per frame.
        let t = Instant::now();
        let now_ms = now.duration_since(self.started).as_secs_f64() * 1000.0;
        for e in self.scripts.tick(now_ms) {
            tracing::error!(target: "script", "uncaught: {e}");
        }
        self.script_time += t.elapsed();

        // Per-frame component updates (`onUpdate`, `onLateUpdate`).
        self.apply_script_output();
        if let Some(n) = self.native.as_mut() {
            let t = Instant::now();
            n.frame(
                &mut self.scene.scene,
                &mut self.ui,
                elapsed.min(MAX_FRAME_DT),
                now_ms,
            );
            self.script_time += t.elapsed();
            self.native_after();
        }
        let t = Instant::now();
        let scene = &self.scene;
        self.components
            .update_active(&mut self.scripts, elapsed.min(MAX_FRAME_DT), &|o| {
                scene.active(o)
            });
        self.script_time += t.elapsed();
        self.apply_script_output();
        let dt = self.components.frame_dt() as f32;
        // Auto exposure adapts by the frame's time.
        self.world_view.grading.delta_time = dt;
        self.world_view.advance_clouds(dt);
        let changed = self.world_view.update_particles(dt, &self.scene);
        if self.native.is_some() {
            native_render::report_particles(&changed, &mut self.scene.scene);
        }
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
        if let Some((key, _)) = self.windows.map.iter().find(|(_, w)| w.tree.contains(node)) {
            return Surface::Window(key.0);
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
                // (`pointer_on` is never a window: those route their own pointer.)
                Surface::Screen | Surface::Window(_) => {
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
        self.native_audio_commands();
        let Some(mixer) = self.audio.as_mut() else {
            return;
        };
        mixer.advance(elapsed);
        let ended = mixer.poll();
        if self.native.is_some() {
            if !ended.is_empty() {
                self.native_audio_ended(&ended);
            }
            return;
        }
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
        let offline = self.offline;
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
        let t = Instant::now();
        self.apply_script_output_now();
        self.ops_time += t.elapsed();
    }

    fn apply_script_output_now(&mut self) {
        for (target, commit) in self.scripts.drain_targeted_commits() {
            if target >= iron_core::WINDOW_TARGET {
                // A secondary window's tree; commits that come before the window opens wait.
                let key = target - iron_core::WINDOW_TARGET;
                match self.windows.map.get_mut(&iron_platform::WindowKey(key)) {
                    Some(w) => {
                        for e in w.tree.apply_commit(&commit) {
                            tracing::warn!(target: "ui", "{e}");
                        }
                    }
                    None => self
                        .windows
                        .pending_commits
                        .entry(key)
                        .or_default()
                        .push(commit),
                }
                continue;
            }
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
                    // Keys go to the window with OS focus; windows route their own.
                    if op.name == "focus" && !matches!(surface, Surface::Window(_)) {
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
                    // `scrollTo` keeps an axis it isn't given (NaN); the others default to 0.
                    let missing = if op.name == "scrollTo" { f64::NAN } else { 0.0 };
                    let (a, b) = (
                        num(1).unwrap_or(missing) as f32,
                        num(2).unwrap_or(missing) as f32,
                    );
                    // The last argument: `"auto"`, `"instant"` or `"smooth"`.
                    let behavior = op
                        .args
                        .get(3)
                        .and_then(iron_core::Value::as_str)
                        .and_then(ScrollBehavior::parse)
                        .unwrap_or_default();
                    let surface = self.surface_of(id);
                    self.handle_on(
                        surface,
                        match op.name.as_str() {
                            "scrollTo" => Input::ScrollTo {
                                id,
                                x: a,
                                y: b,
                                behavior,
                            },
                            "scrollBy" => Input::ScrollBy {
                                id,
                                dx: a,
                                dy: b,
                                behavior,
                            },
                            // `scrollIntoView id block inline behavior`; the DOM's defaults otherwise.
                            _ => {
                                let align = |i: usize, default| {
                                    op.args
                                        .get(i)
                                        .and_then(iron_core::Value::as_str)
                                        .and_then(ScrollAlign::parse)
                                        .unwrap_or(default)
                                };
                                Input::ScrollIntoView {
                                    id,
                                    block: align(1, ScrollAlign::Start),
                                    inline: align(2, ScrollAlign::Nearest),
                                    behavior,
                                }
                            }
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
                "render.text" => {
                    let (root, fonts, sheets) =
                        (&self.asset_root, &self.font_sources, &self.sheet_sources);
                    if let Err(e) = self
                        .panels
                        .apply_text(&op.args, || new_panel_tree(root, fonts, sheets))
                    {
                        tracing::warn!("render.text: {e}");
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
                    let text = iron_core::read_asset_string(&self.asset_root, path)
                        .map_err(|e| e.to_string());
                    texts.push(id);
                    texts.push(match text {
                        Ok(text) => iron_core::Value::String(text),
                        Err(e) => {
                            tracing::warn!("asset.readText `{path}`: {e}");
                            iron_core::Value::Null
                        }
                    });
                }
                "app.quit" => {
                    let code = op
                        .args
                        .first()
                        .and_then(iron_core::Value::as_f64)
                        .unwrap_or(0.0);
                    EXIT_CODE.store(code as i32, std::sync::atomic::Ordering::Relaxed);
                    self.done = true;
                }
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
                "dragImage" => self.drag_image_op(&op.args),
                name if name.starts_with("editor.") => {
                    self.editor
                        .op(&name["editor.".len()..], &op.args, &self.asset_root)
                }
                name if name.starts_with("cursor.") => {
                    self.cursor_op(&name["cursor.".len()..], &op.args)
                }
                name if name.starts_with("scene.") => {
                    self.scene_op(&name["scene.".len()..], &op.args)
                }
                name if name.starts_with("audio.") => {
                    self.audio_op(&name["audio.".len()..], &op.args)
                }
                name if name.starts_with("cloth.") => {
                    let name = &name["cloth.".len()..];
                    if let Err(e) =
                        self.cloth
                            .apply(name, &op.args, &self.scene, &mut self.world_view)
                    {
                        tracing::warn!("cloth.{name}: {e}");
                    }
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
        self.pump_windows();
    }

    /// Asset files changed on disk: textures, models and sounds from them load again
    /// under their ids, then scripts hear which files (`__iron_assets_changed`).
    fn reload_assets(&mut self, keys: &[String]) {
        // Files came or went: the index `Assets.exists` / `list` answer from (packs never change).
        if self.vfs.root().is_some() {
            self.info.set("assetIndex", asset_db::index(&self.vfs));
            native_pump::seed_asset_index(&self.info);
        }
        // New or changed `.meta` files: the database scripts read.
        if keys.iter().any(|k| k.ends_with(".meta")) {
            self.info
                .set("assets", asset_db::scan_vfs(&self.vfs, &self.asset_root));
            if let Err(e) = self.scripts.eval(&self.info.script()) {
                tracing::error!("asset database: {e}");
            }
        }
        let mut sizes = Vec::new();
        if self.native.is_some()
            && keys.iter().any(|k| k.ends_with(".meta"))
            && let Some(list) = self.info.0.get("assets").and_then(|a| a.as_array())
        {
            iron_engine_api::assets::set_assets(
                list.iter()
                    .filter_map(|a| {
                        Some(iron_engine_api::assets::AssetEntry {
                            path: a["path"].as_str()?.to_owned(),
                            guid: a["guid"].as_str()?.to_owned(),
                            importer_json: a["importer"].to_string(),
                        })
                    })
                    .collect(),
            );
        }
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
        self.native_assets_changed(keys, &sizes);
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
        // A compiled game runs its handlers inside the interaction layer's dispatch, so
        // `preventDefault()` reaches the default actions.
        let (native, scene) = (&mut self.native, &mut self.scene.scene);
        let clipboard = match surface {
            Surface::Screen => {
                self.interaction.handle(&mut self.ui, input, &mut |event| {
                    if let Some(n) = native.as_mut() {
                        return n.ui_event(scene, event);
                    }
                    dispatch(scripts, ui_events, event)
                });
                self.interaction.take_clipboard()
            }
            Surface::Panel(id) => {
                let Some(p) = self.panels.map.get_mut(&id) else {
                    return;
                };
                p.interaction.handle(&mut p.tree, input, &mut |event| {
                    if native.is_some() {
                        return native_render::panel_event(scene, id, event);
                    }
                    dispatch(scripts, ui_events, event)
                });
                p.interaction.take_clipboard()
            }
            Surface::Window(key) => {
                let Some(w) = self.windows.map.get_mut(&iron_platform::WindowKey(key)) else {
                    return;
                };
                let _current =
                    iron_engine_api::window::enter_current(iron_engine_api::window::WindowRef(key));
                w.interaction.handle(&mut w.tree, input, &mut |event| {
                    if native.is_some() {
                        return native_render::window_event(scene, key, event);
                    }
                    dispatch(scripts, ui_events, event)
                });
                w.interaction.take_clipboard()
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
        if let Some(n) = self.native.as_mut() {
            if let Surface::Panel(id) = surface
                && let Some(p) = self.panels.map.get_mut(&id)
            {
                native_render::panel_settle(id, &mut p.tree);
            }
            if let Surface::Window(key) = surface
                && let Some(w) = self.windows.map.get_mut(&iron_platform::WindowKey(key))
            {
                let _current =
                    iron_engine_api::window::enter_current(iron_engine_api::window::WindowRef(key));
                native_render::window_settle(key, &mut w.tree);
                n.settle(&mut self.scene.scene, &mut self.ui);
                self.native_after();
            } else {
                n.settle(&mut self.scene.scene, &mut self.ui);
                self.native_after();
            }
        }
        if let Surface::Window(key) = surface {
            self.update_window_cursor_and_ime(key);
            return;
        }
        self.update_ime();
        self.update_cursor();
    }

    /// A window's own cursor and IME state (the main window's are `update_cursor` / `update_ime`).
    fn update_window_cursor_and_ime(&mut self, key: u32) {
        let Some(w) = self.windows.map.get_mut(&iron_platform::WindowKey(key)) else {
            return;
        };
        let iron_platform::WindowHost::Os(window) = &w.host else {
            return;
        };
        let (keyword, _) = w.interaction.cursor_with_images(&w.tree);
        if keyword != w.cursor {
            w.cursor = keyword;
            window.set_cursor_visible(keyword != "none");
            if let Ok(icon) = keyword.parse::<CursorIcon>() {
                window.set_cursor(icon);
            }
        }
        let area = w.interaction.ime_area(&w.tree);
        if area.is_some() != w.ime_allowed {
            w.ime_allowed = area.is_some();
            window.set_ime_allowed(w.ime_allowed);
        }
        if let Some(a) = area {
            let z = 1.0 / window.scale_factor() as f32 * w.scale as f32;
            window.set_ime_cursor_area(
                iron_platform::dpi::LogicalPosition::new(a.x * z, a.y * z),
                iron_platform::dpi::LogicalSize::new(
                    (a.width * z).max(1.0),
                    (a.height * z).max(1.0),
                ),
            );
        }
    }

    /// After a call into a compiled game: its node commands, window, `Application`,
    /// `PlayerPrefs` and quality requests.
    fn native_after(&mut self) {
        self.apply_node_commands();
        self.native_requests();
        self.pump_windows();
    }

    /// Applies what compiled code asked of nodes it holds refs to (`focus()`, `blur()`,
    /// `scrollTo()`).
    fn apply_node_commands(&mut self) {
        use iron_ts_ui::NodeCommand;
        for c in iron_ts_ui::take_node_commands() {
            if let NodeCommand::DragImage(id, x, y) = c {
                // id 0: no drag image (the screen's interaction holds the drag).
                let node = (id > 0).then_some(iron_core::NodeId(id));
                let surface = node.map_or(Surface::Screen, |n| self.surface_of(n));
                let (x, y) = (x as f32, y as f32);
                self.handle_on(surface, Input::SetDragImage { id: node, x, y });
                continue;
            }
            let (id, input) = match c {
                NodeCommand::Focus(id) => (id, Input::Focus(iron_core::NodeId(id))),
                NodeCommand::Blur(id) => (id, Input::Blur(iron_core::NodeId(id))),
                NodeCommand::ScrollTo(id, x, y, behavior) => (
                    id,
                    Input::ScrollTo {
                        id: iron_core::NodeId(id),
                        x: x as f32,
                        y: y as f32,
                        behavior,
                    },
                ),
                NodeCommand::ScrollBy(id, dx, dy, behavior) => (
                    id,
                    Input::ScrollBy {
                        id: iron_core::NodeId(id),
                        dx: dx as f32,
                        dy: dy as f32,
                        behavior,
                    },
                ),
                NodeCommand::ScrollIntoView {
                    id,
                    block,
                    inline,
                    behavior,
                } => (
                    id,
                    Input::ScrollIntoView {
                        id: iron_core::NodeId(id),
                        block,
                        inline,
                        behavior,
                    },
                ),
                NodeCommand::DragImage(..) => unreachable!("handled above"),
            };
            let surface = self.surface_of(iron_core::NodeId(id));
            if matches!(c, NodeCommand::Focus(_)) && !matches!(surface, Surface::Window(_)) {
                self.keys_on = surface;
            }
            self.handle_on(surface, input);
        }
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
        let window = self.gfx.as_ref().and_then(|g| g.window.clone());
        self.display = match (&window, self.offline) {
            (Some(w), false) => iron_platform::display::apply(w, &wanted),
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
        let Some(window) = self.gfx.as_ref().and_then(|g| g.window.as_ref()) else {
            return;
        };
        use iron_platform::CursorGrabMode;
        let result = match self.script_cursor.lock {
            CursorLock::None => window.set_cursor_grab(CursorGrabMode::None),
            CursorLock::Confined => window.set_cursor_grab(CursorGrabMode::Confined),
            CursorLock::Locked => window
                .set_cursor_grab(CursorGrabMode::Locked)
                .or_else(|_| window.set_cursor_grab(CursorGrabMode::Confined)),
        };
        if let Err(e) = result {
            tracing::debug!("cursor lock: {e}");
        }
    }

    /// Shows the hovered element's cursor: its first image that loads, else its keyword.
    /// Where the UI asks for nothing special, scripts' `Cursor` image is the default.
    fn update_cursor(&mut self) {
        // No window, no cursor.
        let Some(window) = self.gfx.as_ref().and_then(|g| g.window.clone()) else {
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
        let scale = window.scale_factor();
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
                window.set_cursor_visible(shown);
                self.custom_cursor = Some(request);
            }
            None => {
                window.set_cursor_visible(shown && keyword != "none");
                if let Ok(icon) = keyword.parse::<CursorIcon>() {
                    window.set_cursor(icon);
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
        let Some(window) = &gfx.window else {
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
            Surface::Screen | Surface::Window(_) => {
                let area = self.interaction.ime_area(&self.ui);
                (area.is_some(), area)
            }
        };
        if allowed != self.ime_allowed {
            self.ime_allowed = allowed;
            window.set_ime_allowed(self.ime_allowed);
            self.ime_area = None;
        }
        if let Some(a) = area
            && Some(a) != self.ime_area
        {
            self.ime_area = Some(a);
            // UI pixels → the window's logical pixels.
            let z = ui_factor(gfx, self.ui_scale) / gfx.dpi() as f32;
            window.set_ime_cursor_area(
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
        if let Some(n) = self.native.as_mut() {
            n.game_input(&mut self.scene.scene, &mut self.ui, args);
            self.native_after();
            return;
        }
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
            .map_or(1.0, |gfx| ui_factor(gfx, self.ui_scale))
    }

    fn now_ms(&self) -> f64 {
        self.now().duration_since(self.started).as_secs_f64() * 1000.0
    }

    /// The game's clock: real time, or a `--fixed-dt` bench run's steps.
    fn now(&self) -> Instant {
        self.fake_now.unwrap_or_else(Instant::now)
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
        for w in self.windows.map.values_mut() {
            load_font(&mut w.tree, &self.asset_root, family, src, weight, italic);
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
        for w in self.windows.map.values_mut() {
            if let Some(id) = w.sheets.remove(&js_id) {
                w.tree.remove_stylesheet(id);
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
        for w in self.windows.map.values_mut() {
            let id = add_stylesheet(&mut w.tree, &self.asset_root, css, "", "addStyleSheet");
            w.sheets.insert(js_id, id);
        }
        self.sheet_sources
            .push((js_id, css.to_owned(), String::new(), "addStyleSheet".into()));
    }

    /// The renderer is ready: tells `SystemInfo` and `Window.main` about the GPU and the screen
    /// and makes the UI and world renderers. `window` is `None` for a windowless `--bench` run.
    fn install_gfx(
        &mut self,
        gpu: Gpu,
        surface: frame_target::FrameTarget,
        window: Option<Arc<Window>>,
    ) {
        tracing::info!(adapter = %gpu.adapter_name(), "renderer ready");
        let size = surface.size();
        let adapter = gpu.adapter.get_info();
        let monitor = window.as_ref().and_then(|w| w.current_monitor());
        let (sw, sh) = monitor.as_ref().map_or(size, |m| {
            let s = m.size();
            (s.width, s.height)
        });
        let hz = monitor
            .as_ref()
            .and_then(|m| m.refresh_rate_millihertz())
            .map_or(0.0, |mhz| f64::from(mhz) / 1000.0);
        let dpi = window.as_ref().map_or(1.0, |w| w.scale_factor());
        self.info.set_system(serde_json::json!({
            "graphicsDeviceName": adapter.name,
            "graphicsDeviceVendorID": adapter.vendor,
            "graphicsDeviceType": format!("{:?}", adapter.backend),
            "graphicsDeviceKind": format!("{:?}", adapter.device_type),
            "graphicsDriver": adapter.driver_info,
            "screenWidth": sw,
            "screenHeight": sh,
            "screenRefreshRate": hz,
            "dpi": dpi * 96.0,
        }));
        self.info
            .set("display", display::state(window.as_deref(), &self.display));
        if let Err(e) = self.scripts.eval(&self.info.script()) {
            tracing::error!("system info: {e}");
        }
        let mut renderer = UiRenderer::new(&gpu);
        renderer.set_asset_root(&self.asset_root);
        let mut world = WorldRenderer::new(&gpu);
        world.physical_units = self.physical_units;
        if self.physical_units {
            tracing::info!("real-world light units");
        }
        // `Window.main` learns the real size and scale (its title may be the game's own).
        let title = iron_engine_api::window::state(iron_engine_api::window::MAIN)
            .map_or_else(String::new, |s| s.title);
        iron_engine_api::window::set_main(&title, windows::to_ui(size, dpi), dpi);
        if let Some(w) = &window
            && title != w.title()
        {
            w.set_title(&title);
        }
        self.gfx = Some(Gfx {
            gpu,
            surface,
            window,
            renderer,
            world,
        });
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
        for id in self.world_view.exposure_resets.drain(..) {
            gfx.world.reset_exposure(id);
        }
        warn_pipeline(&mut self.pipeline_warned, &self.world_view);
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
        gfx.world.pipeline = q.pipeline;
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
        for (id, cooked) in self.world_view.cooked_uploads.drain(..) {
            // GPUs without BC get the blocks decoded.
            if !gfx.world.set_cooked_texture(&gfx.gpu, id, &cooked) {
                gfx.world
                    .set_texture(&gfx.gpu, id, &cook::uncook_texture(&cooked));
            }
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
        let scale = ui_factor(gfx, self.ui_scale);
        draw_frame(
            &gfx.gpu,
            &mut gfx.renderer,
            &mut gfx.world,
            &RenderTarget {
                view: &view,
                format: target.format(),
                size: (w, h),
                scale,
            },
            &views,
            &textures,
            &cmds,
        );
        if self.fps_on {
            let now = self.started.elapsed().as_secs_f64();
            let scale = gfx.dpi() as f32;
            let text = self.fps.text(now);
            let fps_cmds = self.fps_overlay.draw_list(&text, w, h, scale);
            gfx.renderer.render_overlay(
                &gfx.gpu,
                &RenderTarget {
                    view: &view,
                    format: target.format(),
                    size: (w, h),
                    scale,
                },
                &fps_cmds,
            );
        }
        let pixels = target.read_pixels(&gfx.gpu)?;
        iron_render::testing::write_png(path, w, h, &pixels)?;
        self.save_window_screenshots(path)
    }
}

impl PlatformApp for App {
    fn take_custom_cursor(&mut self) -> Option<CustomCursorRequest> {
        self.custom_cursor.take()
    }

    fn take_window_commands(&mut self) -> Vec<iron_platform::WindowCommand> {
        std::mem::take(&mut self.window_commands)
    }

    fn window_opened(&mut self, key: iron_platform::WindowKey, host: iron_platform::WindowHost) {
        self.window_open(key, host);
    }

    fn window_open_failed(&mut self, key: iron_platform::WindowKey, error: String) {
        self.in_game(|| {
            iron_engine_api::window::open_failed(iron_engine_api::window::WindowRef(key.0), &error)
        });
    }

    fn window_input(&mut self, key: iron_platform::WindowKey, event: InputEvent) {
        if key == iron_platform::WindowKey::MAIN {
            self.input(event);
        } else {
            self.window_input_for(key, event);
        }
    }

    fn close_requested(&mut self, key: iron_platform::WindowKey) -> bool {
        self.window_close_requested(key)
    }

    fn window_closed(&mut self, key: iron_platform::WindowKey) {
        self.window_gone(key);
    }

    fn events_done(&mut self) {
        self.window_events_done();
    }

    fn take_windows_to_show(&mut self) -> Vec<iron_platform::WindowKey> {
        std::mem::take(&mut self.windows_to_show)
    }

    fn window_ready(&mut self, window: Arc<Window>) {
        // The saved (or default) window mode, before the first frame shows. Screenshot
        // runs stay windowed.
        if !self.offline && self.display.mode != iron_platform::FullScreenMode::Windowed {
            self.display = iron_platform::display::apply(&window, &self.display);
        }
        let size = window.inner_size();
        match Gpu::with_surface(window.clone(), size.width, size.height) {
            Ok((gpu, surface)) => {
                self.install_gfx(
                    gpu,
                    frame_target::FrameTarget::Window(surface),
                    Some(window),
                );
            }
            Err(e) => tracing::error!("renderer unavailable: {e}"),
        }
    }

    fn input(&mut self, event: InputEvent) {
        if self.file_drag.event(&event) {
            return;
        }
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
            // The game's focus is any of its windows' (`window_events_done`).
            InputEvent::Focused(f) => {
                if f {
                    self.windows.focused.insert(iron_platform::WindowKey::MAIN);
                } else {
                    self.windows.focused.remove(&iron_platform::WindowKey::MAIN);
                }
                self.in_game(|| {
                    iron_engine_api::window::focus_changed(iron_engine_api::window::MAIN, f)
                });
            }
            InputEvent::Occluded(o) => self.lifecycle.set_occluded(o),
            InputEvent::Resized { width, height } => {
                self.lifecycle.set_size(width, height);
                if let Some(gfx) = &self.gfx {
                    let dpi = gfx.dpi();
                    let size = windows::to_ui((width, height), dpi);
                    self.in_game(|| {
                        iron_engine_api::window::resized(
                            iron_engine_api::window::MAIN,
                            size.0,
                            size.1,
                        )
                    });
                }
            }
            InputEvent::Moved { x, y } => {
                if let Some(gfx) = &self.gfx {
                    let dpi = gfx.dpi();
                    let (lx, ly) = (f64::from(x) / dpi, f64::from(y) / dpi);
                    self.in_game(|| {
                        iron_engine_api::window::moved(iron_engine_api::window::MAIN, lx, ly)
                    });
                }
            }
            InputEvent::ScaleChanged(s) => {
                self.in_game(|| {
                    iron_engine_api::window::scale_changed(iron_engine_api::window::MAIN, s)
                });
            }
            _ => {}
        }
        if matches!(event, InputEvent::Occluded(_) | InputEvent::Resized { .. }) {
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
            Surface::Screen | Surface::Window(_) => self
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
        // Size the UI before scripts run and their output applies: on the first frame the
        // tree is still 1x1, and their layout and scrolling (`scrollIntoView` on mount)
        // would happen at that size.
        self.sync_screen();
        // Domain turns: the main game's messages first, then its tick, then the isolated domains
        // (messages, timers, input), then the setup window of new windows ends.
        if !self.lifecycle.paused() {
            let now_ms = self.now_ms();
            self.in_game(|| {
                iron_engine_api::window::deliver(iron_engine_api::window::DomainId::MAIN, now_ms)
            });
        }
        self.tick();
        let now_ms = self.now_ms();
        self.pump_domains(now_ms);
        iron_engine_api::window::end_frame();
        self.close_unclaimed_windows();
        // New log lines and file results for the editor.
        self.editor.pump(&mut self.scripts);
        self.pump_file_drag();
        // Last-moment changes before drawing (`onPreRender`).
        if !self.lifecycle.paused() {
            if let Some(n) = self.native.as_mut() {
                let t = Instant::now();
                n.pre_render(&mut self.scene.scene, &mut self.ui);
                self.script_time += t.elapsed();
                self.native_after();
                self.native_audio_commands();
                let t = Instant::now();
                self.sync_native_render();
                self.sync_time += t.elapsed();
            }
            let t = Instant::now();
            let scene = &self.scene;
            self.components
                .pre_render(&mut self.scripts, &|o| scene.active(o));
            self.script_time += t.elapsed();
            self.apply_script_output();
        }
        // Caret blinking.
        self.ui.set_time(self.now_ms());
        // The caret may have moved with layout.
        self.update_ime();
        // Again, for a UI scale the scripts changed this frame.
        self.sync_screen();
        let render_started = Instant::now();
        let Some(gfx) = &mut self.gfx else {
            return;
        };
        let scale = ui_factor(gfx, self.ui_scale);
        // Scripts' `RenderStats`: what the last frame drew and culled, when it changes.
        let stats = gfx.world.stats();
        if self.render_stats != stats {
            self.render_stats = stats;
            let args = [
                stats.meshes,
                stats.culled_meshes,
                stats.quads,
                stats.culled_quads,
                stats.fog_culled,
                stats.lights,
                stats.culled_lights,
                stats.shadow_maps,
            ]
            .map(|v| iron_core::Value::Number(v as f64));
            let _ = self.scripts.call_global("__iron_render_stats", &args);
            // A compiled game reads `RenderStats` from the registry.
            iron_render_api::set_render_stats(iron_render_api::Stats {
                meshes: stats.meshes as f64,
                culled_meshes: stats.culled_meshes as f64,
                quads: stats.quads as f64,
                culled_quads: stats.culled_quads as f64,
                fog_culled: stats.fog_culled as f64,
                lights: stats.lights as f64,
                culled_lights: stats.culled_lights as f64,
                shadow_maps: stats.shadow_maps as f64,
                shadow_casters: stats.shadow_casters as f64,
            });
        }
        // Each camera's exposure (EV100 in use, and how bright it metered), when it changes.
        for id in self.world_view.camera_ids() {
            let Some(r) = gfx.world.exposure(id) else {
                continue;
            };
            let now = (r.ev, r.scene_ev);
            let moved = self.exposure_sent.get(&id).is_none_or(|&(ev, scene)| {
                (ev - now.0).abs() > 0.01
                    || (scene - now.1).abs() > 0.01
                    || scene.is_nan() != now.1.is_nan()
            });
            if moved {
                self.exposure_sent.insert(id, now);
                let scene = if now.1.is_nan() {
                    iron_core::Value::Null
                } else {
                    iron_core::Value::Number(now.1.into())
                };
                let args = [
                    iron_core::Value::Number(id.into()),
                    iron_core::Value::Number(now.0.into()),
                    scene,
                ];
                let _ = self.scripts.call_global("__iron_exposure", &args);
                // A compiled game's cameras (the id is the camera component's).
                iron_engine_api::camera::set_exposure(
                    iron_engine_api::camera::CameraRef(id),
                    f64::from(now.0),
                    f64::from(now.1),
                );
            }
        }
        // `@media (resolution)` and `(prefers-color-scheme)` follow the window.
        let dark = gfx.window.as_ref().and_then(|w| w.theme()) == Some(iron_platform::Theme::Dark);
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
        for id in self.world_view.exposure_resets.drain(..) {
            gfx.world.reset_exposure(id);
        }
        warn_pipeline(&mut self.pipeline_warned, &self.world_view);
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
        gfx.world.pipeline = q.pipeline;
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
        for (id, cooked) in self.world_view.cooked_uploads.drain(..) {
            // GPUs without BC get the blocks decoded.
            if !gfx.world.set_cooked_texture(&gfx.gpu, id, &cooked) {
                gfx.world
                    .set_texture(&gfx.gpu, id, &cook::uncook_texture(&cooked));
            }
        }
        let now = self
            .fake_now
            .unwrap_or_else(Instant::now)
            .duration_since(self.started)
            .as_secs_f64()
            * 1000.0;
        let gfx_window_scale = gfx.dpi() as f32;
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
        // The FPS counter: at window resolution and scale, over the world and the UI.
        let window_scale = gfx_window_scale;
        let fps_text = self
            .fps_on
            .then(|| self.fps.text(self.started.elapsed().as_secs_f64()));
        let fps_overlay = &mut self.fps_overlay;
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
            if let Some(text) = &fps_text {
                let fps_cmds = fps_overlay.draw_list(text, size.0, size.1, window_scale);
                renderer.render_overlay(
                    gpu,
                    &RenderTarget {
                        view,
                        format,
                        size,
                        scale: window_scale,
                    },
                    &fps_cmds,
                );
            }
        });
        self.world_view.probes_rendered();
        // What it saw of the renderers animation culling watches (nothing if no view drew).
        if let Some(gfx) = &mut self.gfx {
            let seen = gfx.world.take_seen();
            self.world_view.culling.note_seen(&seen);
        }
        self.draw_windows();
        self.render_time += render_started.elapsed();
        if self.fps_on {
            self.fps.frame(self.started.elapsed().as_secs_f64());
        }

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
        lifecycle::frame_cap_with(
            self.target_frame_rate,
            self.world_view.quality.vsync,
            self.offline,
            self.lifecycle.paused() && !self.paused(),
        )
    }

    /// Paused only when every event domain is (a live inspector window keeps the app running).
    fn paused(&self) -> bool {
        lifecycle::app_paused(self.lifecycle.paused(), self.domains.all_paused())
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
        // The program's closures go while the engine's per-thread state is still alive.
        native_render::clear_state();
        if let Some(n) = self.native.take() {
            n.shutdown();
        }
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
fn ui_factor(gfx: &Gfx, ui_scale: ui_scale::UiScale) -> f32 {
    let dpi = gfx.dpi() as f32;
    let (width, height) = gfx.inner_size();
    let (w, h) = (width as f32 / dpi, height as f32 / dpi);
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
    let data = iron_core::read_asset(root, src).map_err(|e| e.to_string());
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
    // A streamed sound is read in pieces (a pack's chunks, a file's windows), never whole.
    if stream
        && !src.trim_start().starts_with("data:")
        && let Some(fs) = iron_core::asset_fs()
    {
        let reader = fs.open_range(src).map_err(|e| e.to_string())?;
        return mixer
            .load_stream(id as u32, reader)
            .map_err(|e| format!("`{src}`: {e}"));
    }
    let bytes = iron_core::read_asset(root, src).map_err(|e| e.to_string())?;
    mixer
        .load(id as u32, bytes, stream)
        .map_err(|e| format!("`{src}`: {e}"))
}

/// A UI that takes input: the screen's, or a world panel's (by its id).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Surface {
    Screen,
    Panel(u32),
    /// A secondary window's UI (by window handle).
    Window(u32),
}

/// The key `game.css` has among the stylesheets every UI tree gets.
const GAME_CSS: u32 = u32::MAX;

/// `game.css` as a stylesheet source for panel trees.
fn game_css_source(code_root: &Path, css_path: &Path) -> Option<(u32, String, String, String)> {
    let css = std::fs::read_to_string(css_path).ok()?;
    let base = asset_root::css_base(code_root, css_path);
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

/// `root` is where its `url()`s and `@import`s resolve; the CSS file itself is in `code_root`.
fn load_game_css(ui: &mut UiTree, root: &Path, code_root: &Path, css_path: &Path) -> Option<u32> {
    let css = std::fs::read_to_string(css_path).ok()?;
    let base = asset_root::css_base(code_root, css_path);
    let id = add_stylesheet(ui, root, &css, &base, &css_path.display().to_string());
    tracing::info!("loaded stylesheet {}", css_path.display());
    Some(id)
}

fn add_stylesheet(ui: &mut UiTree, root: &Path, css: &str, base: &str, source: &str) -> u32 {
    // `@import`s load from the game folder (Bun inlines them when bundling; this is
    // for stylesheets added at runtime).
    let added = ui.add_stylesheet_with_imports(css, base, &mut |path| {
        iron_core::read_asset_string(root, path).ok()
    });
    for warning in &added.warnings {
        tracing::warn!(target: "css", "{source}: {warning}");
    }
    for face in &added.font_faces {
        load_font(ui, root, &face.family, &face.src, face.weight, face.italic);
    }
    added.id
}

/// Says once in the log when the game asks for light shafts its quality level leaves out
/// (`volumetricFog: false`, a phone's level, say), as Unity does.
fn warn_pipeline(warned: &mut bool, world: &world_view::WorldViews) {
    if !*warned && !world.quality.pipeline.volumetric_fog && world.uses_volumetric_fog() {
        *warned = true;
        tracing::warn!("RenderSettings.volumetricFog: this quality level has light shafts off");
    }
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

    // M9-APP-01
    #[test]
    fn force_flag_parses_and_defaults_off() {
        assert!(!parse(&[]).unwrap().force);
        assert!(parse(&["g", "--force"]).unwrap().force);
    }

    // M9-APP-02
    #[test]
    fn only_native_or_forced_or_editor_runs() {
        assert_eq!(quickjs_gate(true, false, false), QuickJsGate::Native);
        assert_eq!(quickjs_gate(false, false, false), QuickJsGate::Refused);
        assert_eq!(quickjs_gate(false, true, false), QuickJsGate::Forced);
        assert_eq!(quickjs_gate(false, false, true), QuickJsGate::Forced);
    }

    // M9-API-909
    #[test]
    fn fps_flags_parse() {
        assert_eq!(parse(&[]).unwrap().fps, None);
        assert_eq!(parse(&["--fps"]).unwrap().fps, Some(true));
        assert_eq!(parse(&["g", "--no-fps"]).unwrap().fps, Some(false));
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
        assert!(parse(&["--cook", "examples/world"]).unwrap().cook);
    }

    /// A mono 16-bit WAV of a sine, `n` samples at 8 kHz.
    fn wav(n: u32) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(b"RIFF");
        out.extend_from_slice(&(36 + n * 2).to_le_bytes());
        out.extend_from_slice(b"WAVEfmt ");
        out.extend_from_slice(&16u32.to_le_bytes());
        out.extend_from_slice(&[1, 0, 1, 0]);
        out.extend_from_slice(&8000u32.to_le_bytes());
        out.extend_from_slice(&16000u32.to_le_bytes());
        out.extend_from_slice(&[2, 0, 16, 0]);
        out.extend_from_slice(b"data");
        out.extend_from_slice(&(n * 2).to_le_bytes());
        for i in 0..n {
            let v = ((i as f32 * 0.3).sin() * 8000.0) as i16;
            out.extend_from_slice(&v.to_le_bytes());
        }
        out
    }

    // M9-API-1148
    #[test]
    fn sounds_load_from_packs_whole_and_streamed_and_a_miss_says_where_it_looked() {
        use iron_core::Value;
        use iron_pack::{EntrySpec, Pack, PackSource, PackWriter, Vfs, WriteOptions};
        let sound = wav(40_000);
        let mut w =
            PackWriter::new(std::io::Cursor::new(Vec::new()), WriteOptions::default()).unwrap();
        w.add(&EntrySpec::new("assets/sfx/beep.wav", &sound))
            .unwrap();
        let pack = Pack::from_bytes(w.finish().unwrap().0.into_inner()).unwrap();
        let mut vfs = Vfs::new();
        vfs.mount(std::sync::Arc::new(PackSource::new("assets", pack)));
        let root = Path::new("no-such-folder");
        let load = |mixer: &mut dyn iron_audio::Mixer, id: f64, src: &str, stream: bool| {
            let args = [
                Value::Number(id),
                Value::String(src.into()),
                Value::Bool(stream),
            ];
            iron_core::with_asset_fs(std::sync::Arc::new(vfs.clone()), || {
                load_sound(mixer, root, &args)
            })
        };
        let mut mixer = iron_audio::open_mixer(true);
        load(mixer.as_mut(), 1.0, "assets/sfx/beep.wav", false).unwrap();
        // Streamed: read through the pack's chunks (the file is compressed, 3 chunks of 256 KiB
        // would not be needed for 80 kB, but the reader works in windows all the same).
        load(mixer.as_mut(), 2.0, "assets/sfx/beep.wav", true).unwrap();
        let e = load(mixer.as_mut(), 3.0, "assets/sfx/none.wav", false).unwrap_err();
        assert_eq!(e, "asset \"assets/sfx/none.wav\" not found in assets.dat");
        let e = load(mixer.as_mut(), 4.0, "assets/sfx/none.wav", true).unwrap_err();
        assert!(e.contains("not found in assets.dat"), "{e}");
        mixer.advance(0.1);
    }

    // M5-APP-11
    #[test]
    fn arguments_after_a_double_dash_are_for_scripts() {
        let args = parse(&["examples/md-reader", "--", "--goto", "a.md#b", "x"]).unwrap();
        assert_eq!(args.game, PathBuf::from("examples/md-reader"));
        assert_eq!(args.script_args, ["--goto", "a.md#b", "x"]);
        // Iron's own options before `--` still count; nothing after it does.
        let args = parse(&["--frames", "3", "--", "--frames", "9"]).unwrap();
        assert_eq!((args.frames, args.script_args.len()), (3, 2));
        assert!(parse(&["examples/css"]).unwrap().script_args.is_empty());
    }
}
