// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

//! The startup video-settings prompt: before a game's window opens, a small window asks the
//! player for the display mode, resolution, vsync and quality level, once. A game opts in with
//! `videoSettingsPrompt: true` in `project.json` (or `platypus-build package
//! --video-settings-prompt`, which writes it into the packaged copy); `--video-settings-prompt` /
//! `--no-video-settings-prompt` on the command line force it on or off (on also ignores the
//! "shown once" memory, so a player can reopen it).
//!
//! The dialog (`video_prompt.tsx`) runs in a child process of this same program
//! (`--video-settings-dialog <folder>`, as the crash dialog does) and only as UI: its bundle says
//! `"renderer": "ui"`, so it takes the light path (the GLES executor, wgpu with small GPU
//! allocations when it is missing) and has no world renderer. A child,
//! not a phase of this process, because winit makes one event loop per process, and so the
//! dialog's window and GPU context are gone before the game's are made. Its answer comes back as
//! its exit code and its own prefs file; the choice is written into the game's prefs under the
//! keys the engine already reads (`platypus.display`, `platypus.quality`), so the game starts
//! through its normal path, and a game's own settings menu still changes it later.

use std::path::{Path, PathBuf};
use std::process::Command;

use platypus_platform::display::{DisplaySettings, FullScreenMode};
use serde_json::{Value, json};

use super::prefs::Prefs;
use crate::info::Manifest;

/// Shows the prompt (even when it was answered before).
pub const PROMPT_FLAG: &str = "--video-settings-prompt";
/// Never shows it.
pub const NO_PROMPT_FLAG: &str = "--no-video-settings-prompt";
/// The dialog's own process (followed by its folder).
pub const DIALOG_FLAG: &str = "--video-settings-dialog";
/// Set in the game's prefs once the player pressed Play.
pub const CHOSEN_KEY: &str = "platypus.videoSettings.chosen";
/// The dialog's exit code for Play (0: Quit).
const EXIT_PLAY: i32 = 10;

const DIALOG_SOURCE: &str = include_str!("video_prompt.tsx");
/// The resolution list's helpers (tested with `bun test`), put in front of the dialog.
const SIZES_SOURCE: &str = include_str!("video_prompt_sizes.ts");
/// The dialog's window in logical pixels, fixed (not resizable, no maximize button): sized to
/// its content, shorter without the mode and resolution rows (a subtitle fits beside the icon).
const DIALOG_SIZE: (u32, u32) = (600, 500);

fn dialog_height(current: &Current) -> u32 {
    if current.resizable {
        DIALOG_SIZE.1
    } else {
        300
    }
}

/// Whether to show the prompt: the command line first (on: always, even after a choice or in a
/// screenshot run), then the project (with a packaged game's build argument already in it), off
/// by default; the project's opt-in skips it once the player chose and in runs without a player.
pub fn should_prompt(
    cli: Option<bool>,
    project: Option<bool>,
    chosen: bool,
    offline: bool,
) -> bool {
    match cli {
        Some(on) => on,
        None => project == Some(true) && !chosen && !offline,
    }
}

/// Whether the player already answered the prompt.
pub fn chosen(prefs: &Prefs) -> bool {
    prefs
        .get(CHOSEN_KEY)
        .is_some_and(|v| v.as_bool() == Some(true) || v.as_f64() == Some(1.0))
}

/// What the dialog starts with: the game's current (saved or default) choice.
#[derive(Clone, Debug, PartialEq)]
pub struct Current {
    pub title: String,
    /// The game's subtitle, empty when it has none.
    pub subtitle: String,
    /// False for a fixed-size game: no mode or resolution choice.
    pub resizable: bool,
    pub display: DisplaySettings,
    pub vsync: bool,
    pub level: String,
    /// The startup prompt (Play / Quit) rather than the one a running game opened
    /// (`Application.showVideoSettings`: Apply / Cancel).
    pub startup: bool,
}

/// The saved quality choice (`platypus.quality`, JSON text in prefs).
fn saved_quality(prefs: &Prefs) -> Option<Value> {
    prefs
        .get("platypus.quality")
        .and_then(Value::as_str)
        .and_then(|t| serde_json::from_str(t).ok())
}

impl Current {
    pub fn new(
        manifest: &Manifest,
        prefs: &Prefs,
        display: DisplaySettings,
        resizable: bool,
    ) -> Self {
        let saved = saved_quality(prefs);
        let level = saved
            .as_ref()
            .and_then(|q| q.get("level")?.as_str().map(str::to_owned))
            .or_else(|| manifest.quality.get("default")?.as_str().map(str::to_owned))
            .unwrap_or_else(|| "High".into());
        // A save's vsync counts only when it is the player's (see `apply_default_quality`).
        let vsync = saved
            .as_ref()
            .filter(|q| q.get(platypus_engine_api::screen::PLAYER_VSYNC) == Some(&json!(true)))
            .and_then(|q| q["current"]["vSyncCount"].as_f64())
            .is_none_or(|v| v >= 1.0);
        Self {
            title: manifest.window_title().to_owned(),
            subtitle: manifest.subtitle.clone(),
            resizable,
            display,
            vsync,
            level,
            startup: true,
        }
    }

    /// The constants the dialog's source starts with.
    fn constants(&self) -> String {
        let s = |v: &str| serde_json::to_string(v).unwrap_or_else(|_| "\"\"".into());
        let d = &self.display;
        let (w, h) = if d.mode == FullScreenMode::Windowed {
            (d.window_width, d.window_height)
        } else {
            (d.width, d.height)
        };
        format!(
            // Typed: a literal `false` would make `RESIZABLE ? <Row/> : null` only ever null.
            "const GAME: string = {};\nconst SUBTITLE: string = {};\nconst RESIZABLE: boolean = {};\nconst MODE: string = {};\nconst WIDTH: number = {w};\nconst HEIGHT: number = {h};\nconst REFRESH: number = {};\nconst VSYNC: boolean = {};\nconst LEVEL: string = {};\nconst STARTUP: boolean = {};\n",
            s(&self.title),
            s(&self.subtitle),
            self.resizable,
            s(d.mode.name()),
            d.refresh_rate,
            self.vsync,
            s(&self.level),
            self.startup,
        )
    }
}

/// What the player chose (the dialog's prefs file).
#[derive(Clone, Debug, PartialEq)]
pub struct Choice {
    pub mode: FullScreenMode,
    /// The size (window or exclusive mode); `None` keeps the current one (borderless).
    pub size: Option<(u32, u32, f32)>,
    pub vsync: bool,
    pub level: String,
}

impl Choice {
    /// From the dialog's PlayerPrefs (`mode`, `width`, `height`, `refreshRate`, `vsync`,
    /// `level`); `None` when they are missing.
    pub fn from_prefs(values: &serde_json::Map<String, Value>) -> Option<Choice> {
        let num = |k: &str| values.get(k).and_then(Value::as_f64);
        let mode = FullScreenMode::parse(values.get("mode")?.as_str()?)?;
        let size = match (num("width"), num("height")) {
            (Some(w), Some(h)) if w >= 1.0 && h >= 1.0 => Some((
                w as u32,
                h as u32,
                num("refreshRate").unwrap_or(0.0).max(0.0) as f32,
            )),
            _ => None,
        };
        Some(Choice {
            mode,
            size,
            vsync: num("vsync")? >= 1.0,
            level: values.get("level")?.as_str()?.to_owned(),
        })
    }

    /// Writes the choice into the game's prefs, under the keys the engine reads at startup:
    /// `platypus.display` (not for a fixed-size game, whose window always has its own size) and
    /// `platypus.quality` (the level, with the player's vsync), and marks the prompt answered.
    pub fn apply(&self, prefs: &mut Prefs, display: DisplaySettings, resizable: bool) {
        if let Some(chosen) = self.display(display, resizable) {
            prefs.set("platypus.display", super::display::to_json(&chosen));
        }
        let quality = quality_pref(saved_quality(prefs).as_ref(), &self.level, self.vsync);
        prefs.set("platypus.quality", Value::String(quality.to_string()));
        prefs.set(CHOSEN_KEY, Value::Bool(true));
    }
}

impl Choice {
    /// The window's mode and size the choice asks for, over `current`; `None` for a fixed-size
    /// game (its window keeps its own size).
    pub fn display(&self, current: DisplaySettings, resizable: bool) -> Option<DisplaySettings> {
        if !resizable {
            return None;
        }
        let mut v = json!({ "mode": self.mode.name() });
        if let Some((w, h, hz)) = self.size {
            v["width"] = json!(w);
            v["height"] = json!(h);
            v["refreshRate"] = json!(hz);
        }
        Some(super::display::from_json(Some(&v), current))
    }
}

/// The saved quality choice after the prompt: the chosen level with the player's vsync. The
/// saved per-setting changes are kept when the level stays the same, dropped when it changes
/// (they were the old level's).
pub fn quality_pref(saved: Option<&Value>, level: &str, vsync: bool) -> Value {
    let same = saved.and_then(|s| s.get("level")?.as_str()) == Some(level);
    let mut current = saved
        .filter(|_| same)
        .and_then(|s| s.get("current").cloned())
        .filter(Value::is_object)
        .unwrap_or_else(|| json!({}));
    current["vSyncCount"] = json!(i32::from(vsync));
    json!({
        "level": level,
        "current": current,
        (platypus_engine_api::screen::PLAYER_VSYNC): true,
    })
}

/// How the dialog ended.
#[derive(Debug, PartialEq)]
pub enum Outcome {
    Play(Choice),
    Quit,
    /// A screenshot run: the dialog's picture is the run's output.
    Screenshot,
    /// It could not run (no GPU, a crash): the game starts as it is, nothing recorded.
    Failed,
}

/// The run's screenshot (path and frames), passed on to the dialog.
pub struct Screenshot<'a> {
    pub path: &'a Path,
    pub frames: u32,
}

/// Runs the dialog in a child process and waits for it (the startup prompt).
/// `icon`: the game's window icon as RGBA (its own, else Platypus's), for the dialog's window.
pub fn run(current: &Current, icon: &Icon, screenshot: Option<Screenshot<'_>>) -> Outcome {
    match Running::start(current, icon, screenshot) {
        Some(running) => running.wait(),
        None => Outcome::Failed,
    }
}

/// The dialog's process, while it is open.
pub struct Running {
    child: std::process::Child,
    dir: PathBuf,
    screenshot: bool,
}

impl Running {
    /// Starts the dialog's process; `None` when it can't (logged).
    pub fn start(
        current: &Current,
        icon: &Icon,
        screenshot: Option<Screenshot<'_>>,
    ) -> Option<Running> {
        let exe = std::env::current_exe().ok()?;
        let dir =
            std::env::temp_dir().join(format!("platypus-video-settings-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        if let Err(e) = write_dialog_game(&dir, current, icon) {
            tracing::warn!("video settings dialog: {e}");
            return None;
        }
        let mut cmd = Command::new(exe);
        cmd.arg(DIALOG_FLAG)
            .arg(&dir)
            // A crash in the dialog must not open the crash dialog; its prefs and log stay with it.
            .env(super::crash_dialog::CHILD_ENV, "1")
            .env("PLATYPUS_PREFS_DIR", &dir)
            .env("PLATYPUS_LOG_FILE", dir.join("dialog.log"));
        if let Some(s) = &screenshot {
            cmd.arg("--screenshot")
                .arg(s.path)
                .args(["--frames", &s.frames.to_string()]);
        }
        match cmd.spawn() {
            Ok(child) => Some(Running {
                child,
                dir,
                screenshot: screenshot.is_some(),
            }),
            Err(e) => {
                tracing::warn!("video settings dialog: {e}");
                let _ = std::fs::remove_dir_all(&dir);
                None
            }
        }
    }

    /// Waits for the player's answer.
    pub fn wait(mut self) -> Outcome {
        let code = self.child.wait().ok().and_then(|s| s.code());
        self.finish(code)
    }

    /// The answer, once the dialog has closed (a running game asks every frame).
    pub fn poll(&mut self) -> Option<Outcome> {
        match self.child.try_wait() {
            Ok(None) => None,
            Ok(Some(status)) => Some(self.finish(status.code())),
            Err(_) => Some(self.finish(None)),
        }
    }

    fn finish(&mut self, code: Option<i32>) -> Outcome {
        let outcome = match code {
            _ if self.screenshot => Outcome::Screenshot,
            Some(0) => Outcome::Quit,
            Some(EXIT_PLAY) => Choice::from_prefs(Prefs::open(self.dir.join("prefs.json")).all())
                .map_or(Outcome::Failed, Outcome::Play),
            _ => Outcome::Failed,
        };
        if outcome == Outcome::Failed {
            let log = std::fs::read_to_string(self.dir.join("dialog.log")).unwrap_or_default();
            tracing::warn!("the video settings dialog did not run (exit {code:?})\n{log}");
        }
        let _ = std::fs::remove_dir_all(&self.dir);
        outcome
    }
}

/// An icon as RGBA pixels, width and height (`info::load_icon`, `info::default_icon`).
pub type Icon = (Vec<u8>, u32, u32);

/// The dialog's folder: `project.json` (a fixed-size UI app named after the game, with its icon,
/// so the window and taskbar show the game), `icon.png` and
/// `build/game.pbundle`, its source the dialog's TSX after the constants.
fn write_dialog_game(dir: &Path, current: &Current, icon: &Icon) -> std::io::Result<()> {
    std::fs::create_dir_all(dir.join("build"))?;
    let (rgba, w, h) = icon;
    image::save_buffer(dir.join("icon.png"), rgba, *w, *h, image::ColorType::Rgba8)
        .map_err(std::io::Error::other)?;
    let project = json!({
        "name": "video-settings",
        "title": current.title,
        "company": "Platypus",
        "identifier": "dev.platypus.video-settings",
        "icon": "icon.png",
        "display": { "mode": "Windowed", "width": DIALOG_SIZE.0, "height": dialog_height(current), "resizable": false },
        "fps": false,
    });
    let bundle = json!({
        "format": 1,
        "title": current.title,
        "company": "Platypus",
        "identifier": "dev.platypus.video-settings",
        "entry": "video_prompt.tsx",
        "source": format!("{}{SIZES_SOURCE}{DIALOG_SOURCE}", current.constants()),
        "files": [],
        "css": null,
        "assets": [],
        "patterns": [],
        // Only UI: the light renderer (its tier, as `platypus-build` would find it).
        "renderer": "ui",
    });
    std::fs::write(dir.join("project.json"), project.to_string())?;
    std::fs::write(dir.join("build").join("game.pbundle"), bundle.to_string())
}

/// `Application.showVideoSettings()`: the same dialog over a running game. The game pauses while
/// it is open (the dialog is a process of its own, polled each frame and while idle); Apply puts
/// the choice into effect through the paths scripts use (the window's mode and size, the quality
/// level and vsync) and saves it; Cancel changes nothing.
impl super::App {
    pub(super) fn open_video_settings(&mut self) {
        if self.offline {
            tracing::warn!("Application.showVideoSettings: no dialog in screenshot or bench runs");
            return;
        }
        if self.video_dialog.is_some() {
            return;
        }
        let resizable = self.manifest.display["resizable"].as_bool().unwrap_or(true);
        let live = platypus_engine_api::screen::quality();
        let current = Current {
            level: live.name.clone(),
            vsync: live.v_sync_count > 0,
            startup: false,
            ..Current::new(&self.manifest, &self.prefs, self.display, resizable)
        };
        let icon = self.manifest.window_icon(&self.code_root);
        match Running::start(&current, &icon, None) {
            Some(running) => {
                self.video_dialog = Some(running);
                self.lifecycle.set_modal(true);
                self.sync_lifecycle();
            }
            None => tracing::warn!("Application.showVideoSettings: the dialog could not start"),
        }
    }

    /// Whether the dialog closed; then its answer is applied and the game resumes.
    pub(super) fn poll_video_settings(&mut self) {
        let Some(outcome) = self.video_dialog.as_mut().and_then(Running::poll) else {
            return;
        };
        self.video_dialog = None;
        self.lifecycle.set_modal(false);
        if let Outcome::Play(choice) = outcome {
            self.apply_video_choice(&choice);
        }
        self.sync_lifecycle();
        // The dialog had focus: give it back to the game's window.
        if let Some(w) = self.main_window() {
            w.focus_window();
        }
    }

    fn apply_video_choice(&mut self, choice: &Choice) {
        let resizable = self.manifest.display["resizable"].as_bool().unwrap_or(true);
        if let Some(wanted) = choice.display(self.display, resizable) {
            self.apply_display(wanted);
            self.prefs
                .set("platypus.display", super::display::to_json(&self.display));
        }
        let (level, vsync) = (choice.level.clone(), choice.vsync);
        // As a script would: `setQualityLevel` then `vSyncCount` (saved with `playerVSync`).
        self.in_game(|| {
            use platypus_engine_api::screen::quality_api;
            quality_api::set_quality_level_by_name(&level);
            quality_api::set_v_sync_count(i32::from(vsync));
            platypus_engine_api::app::video_settings_changed();
        });
        self.prefs.set(CHOSEN_KEY, Value::Bool(true));
        self.prefs.save();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // M9-VSP-01
    #[test]
    fn the_command_line_wins_then_the_project_and_the_prompt_is_off_by_default() {
        // Off unless asked for.
        assert!(!should_prompt(None, None, false, false));
        assert!(!should_prompt(None, Some(false), false, false));
        assert!(should_prompt(None, Some(true), false, false));
        // The command line over the project, both ways.
        assert!(!should_prompt(Some(false), Some(true), false, false));
        assert!(should_prompt(Some(true), Some(false), false, false));
        assert!(should_prompt(Some(true), None, false, false));
    }

    // M9-VSP-02
    #[test]
    fn the_prompt_is_shown_once_and_never_without_a_player_unless_forced() {
        assert!(
            !should_prompt(None, Some(true), true, false),
            "answered before"
        );
        assert!(
            !should_prompt(None, Some(true), false, true),
            "a screenshot or bench run"
        );
        // Forced: reopens it after a choice, and renders it in a screenshot run.
        assert!(should_prompt(Some(true), Some(true), true, false));
        assert!(should_prompt(Some(true), None, false, true));
        let mut p = Prefs::in_memory();
        assert!(!chosen(&p));
        p.set(CHOSEN_KEY, Value::Bool(true));
        assert!(chosen(&p));
        // As the engine's PlayerPrefs give a bool back (a number).
        p.set(CHOSEN_KEY, json!(1.0));
        assert!(chosen(&p));
    }

    fn values(v: Value) -> serde_json::Map<String, Value> {
        v.as_object().unwrap().clone()
    }

    // M9-VSP-03
    #[test]
    fn the_choice_goes_into_the_engines_display_and_quality_prefs() {
        let choice = Choice::from_prefs(&values(json!({
            "mode": "ExclusiveFullScreen", "width": 1920.0, "height": 1080.0, "refreshRate": 144.0,
            "vsync": 0.0, "level": "Medium",
        })))
        .unwrap();
        assert_eq!(choice.size, Some((1920, 1080, 144.0)));
        let mut p = Prefs::in_memory();
        // A saved choice of another level: its per-setting changes don't carry over.
        p.set(
            "platypus.quality",
            json!(r#"{"level":"High","current":{"renderScale":0.5}}"#),
        );
        choice.apply(&mut p, DisplaySettings::default(), true);
        let d =
            super::super::display::from_json(p.get("platypus.display"), DisplaySettings::default());
        assert_eq!(
            (d.mode, d.width, d.height, d.refresh_rate),
            (FullScreenMode::ExclusiveFullScreen, 1920, 1080, 144.0)
        );
        let q: Value =
            serde_json::from_str(p.get("platypus.quality").unwrap().as_str().unwrap()).unwrap();
        assert_eq!(
            q,
            json!({ "level": "Medium", "current": { "vSyncCount": 0 }, "playerVSync": true })
        );
        assert!(chosen(&p));
        // The same level keeps what was changed on it.
        let q = quality_pref(
            Some(&json!({ "level": "Low", "current": { "renderScale": 0.5, "vSyncCount": 0 } })),
            "Low",
            true,
        );
        assert_eq!(q["current"], json!({ "renderScale": 0.5, "vSyncCount": 1 }));
        // Windowed: the window's size; borderless keeps the saved one.
        let windowed = Choice::from_prefs(&values(json!({
            "mode": "Windowed", "width": 1600.0, "height": 900.0, "vsync": 1.0, "level": "High",
        })))
        .unwrap();
        windowed.apply(&mut p, DisplaySettings::default(), true);
        let d =
            super::super::display::from_json(p.get("platypus.display"), DisplaySettings::default());
        assert_eq!(
            (d.mode, d.window_width, d.window_height),
            (FullScreenMode::Windowed, 1600, 900)
        );
        let borderless = Choice::from_prefs(&values(
            json!({ "mode": "FullScreenWindow", "vsync": 1.0, "level": "High" }),
        ))
        .unwrap();
        assert_eq!(borderless.size, None);
        // Missing answers are no choice.
        assert_eq!(
            Choice::from_prefs(&values(json!({ "mode": "Windowed" }))),
            None
        );
    }

    // M9-VSP-04
    #[test]
    fn a_fixed_size_game_gets_no_mode_or_resolution_choice() {
        let manifest = Manifest::from_json(
            &json!({ "platypus": { "title": "Fox", "quality": { "default": "Low" },
                                   "display": { "resizable": false } } }),
            "fox",
        );
        let p = Prefs::in_memory();
        let current = Current::new(&manifest, &p, DisplaySettings::default(), false);
        let constants = current.constants();
        assert!(constants.contains("const RESIZABLE: boolean = false;"));
        assert!(constants.contains("const LEVEL: string = \"Low\";"));
        assert!(
            constants.contains("const VSYNC: boolean = true;"),
            "vsync starts on"
        );
        // The dialog hides both rows without RESIZABLE, and its choice leaves the display alone.
        assert!(DIALOG_SOURCE.contains(r#"display: RESIZABLE ? "flex" : "none""#));
        let mut p = Prefs::in_memory();
        let choice = Choice::from_prefs(&values(json!({
            "mode": "Windowed", "width": 640.0, "height": 480.0, "vsync": 1.0, "level": "Low",
        })))
        .unwrap();
        choice.apply(&mut p, DisplaySettings::default(), false);
        assert!(p.get("platypus.display").is_none());
        assert!(p.get("platypus.quality").is_some());
    }

    // M9-VSP-05
    #[test]
    fn the_dialog_starts_from_the_saved_choice() {
        let manifest = Manifest::from_json(&json!({ "platypus": { "title": "Fox" } }), "fox");
        let mut p = Prefs::in_memory();
        p.set(
            "platypus.quality",
            json!(r#"{"level":"Ultra","current":{"vSyncCount":0},"playerVSync":true}"#),
        );
        let c = Current::new(&manifest, &p, DisplaySettings::default(), true);
        assert_eq!((c.level.as_str(), c.vsync), ("Ultra", false));
        // An older save's vsync (not the player's) does not count: on.
        p.set(
            "platypus.quality",
            json!(r#"{"level":"Ultra","current":{"vSyncCount":0}}"#),
        );
        assert!(Current::new(&manifest, &p, DisplaySettings::default(), true).vsync);
    }

    // M9-VSP-12
    #[test]
    fn the_dialog_window_is_named_after_the_game_and_has_its_icon() {
        let manifest = Manifest::from_json(&json!({ "platypus": { "title": "Fox Run" } }), "fox");
        let current = Current::new(
            &manifest,
            &Prefs::in_memory(),
            DisplaySettings::default(),
            true,
        );
        let dir = std::env::temp_dir().join(format!("platypus-vsp-folder-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        write_dialog_game(&dir, &current, &(vec![0, 255, 0, 255], 1, 1)).unwrap();
        let dialog = Manifest::load(&dir);
        assert_eq!(dialog.window_title(), "Fox Run");
        assert_eq!(dialog.window_icon(&dir), (vec![0, 255, 0, 255], 1, 1));
        // A fixed size (no resize edges, no maximize button: the `resizable: false` path).
        assert_eq!(dialog.display["resizable"], false);
        assert_eq!(
            (
                dialog.display["width"].as_u64(),
                dialog.display["height"].as_u64()
            ),
            (Some(600), Some(500))
        );
        let bundle = std::fs::read_to_string(dir.join("build/game.pbundle")).unwrap();
        assert!(
            bundle.contains("const GAME: string = \\\"Fox Run\\\";"),
            "{bundle}"
        );
        assert!(!bundle.contains("const GAME: string = \\\"Platypus"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    // M9-VSP-16
    #[test]
    fn a_choice_applied_in_a_running_game_maps_to_the_window_settings() {
        let current = DisplaySettings {
            mode: FullScreenMode::Windowed,
            window_width: 1280,
            window_height: 720,
            ..DisplaySettings::default()
        };
        let exclusive = Choice {
            mode: FullScreenMode::ExclusiveFullScreen,
            size: Some((1920, 1080, 144.0)),
            vsync: false,
            level: "Low".into(),
        };
        let d = exclusive.display(current, true).unwrap();
        assert_eq!(
            (d.mode, d.width, d.height, d.refresh_rate),
            (FullScreenMode::ExclusiveFullScreen, 1920, 1080, 144.0)
        );
        // The window size is remembered for going back to a window.
        assert_eq!((d.window_width, d.window_height), (1280, 720));
        let borderless = Choice {
            mode: FullScreenMode::FullScreenWindow,
            size: None,
            ..exclusive.clone()
        };
        assert_eq!(
            borderless.display(current, true).unwrap().mode,
            FullScreenMode::FullScreenWindow
        );
        // A fixed-size game's window is left alone.
        assert_eq!(exclusive.display(current, false), None);
        // The dialog a running game opens says Apply / Cancel, the startup one Play / Quit.
        let manifest = Manifest::from_json(&json!({ "platypus": { "title": "Fox" } }), "fox");
        let c = Current::new(&manifest, &Prefs::in_memory(), current, true);
        assert!(c.constants().contains("const STARTUP: boolean = true;"));
        let live = Current {
            startup: false,
            ..c
        };
        assert!(live.constants().contains("const STARTUP: boolean = false;"));
        assert!(DIALOG_SOURCE.contains(r#"STARTUP ? "Quit" : "Cancel""#));
        assert!(DIALOG_SOURCE.contains(r#"STARTUP ? "Play" : "Apply""#));
    }
}
