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
//! (`--video-settings-dialog <folder>`, as the crash dialog does) and only as UI: no world
//! renderer, the GLES executor (wgpu with small GPU allocations when it is missing). A child,
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
/// The dialog's window in logical pixels (shorter without the mode and resolution rows).
const DIALOG_SIZE: (u32, u32) = (600, 500);

fn dialog_height(resizable: bool) -> u32 {
    if resizable { DIALOG_SIZE.1 } else { 300 }
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
    /// False for a fixed-size game: no mode or resolution choice.
    pub resizable: bool,
    pub display: DisplaySettings,
    pub vsync: bool,
    pub level: String,
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
            title: manifest.title.clone(),
            resizable,
            display,
            vsync,
            level,
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
            "const GAME = {};\nconst RESIZABLE = {};\nconst MODE = {};\nconst WIDTH = {w};\nconst HEIGHT = {h};\nconst REFRESH = {};\nconst VSYNC = {};\nconst LEVEL = {};\n",
            s(&self.title),
            self.resizable,
            s(d.mode.name()),
            d.refresh_rate,
            self.vsync,
            s(&self.level),
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
        if resizable {
            let mut v = json!({ "mode": self.mode.name() });
            if let Some((w, h, hz)) = self.size {
                v["width"] = json!(w);
                v["height"] = json!(h);
                v["refreshRate"] = json!(hz);
            }
            let chosen = super::display::from_json(Some(&v), display);
            prefs.set("platypus.display", super::display::to_json(&chosen));
        }
        let quality = quality_pref(saved_quality(prefs).as_ref(), &self.level, self.vsync);
        prefs.set("platypus.quality", Value::String(quality.to_string()));
        prefs.set(CHOSEN_KEY, Value::Bool(true));
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

/// Runs the dialog in a child process and waits for it.
pub fn run(current: &Current, icon: Option<&Path>, screenshot: Option<Screenshot<'_>>) -> Outcome {
    let Ok(exe) = std::env::current_exe() else {
        return Outcome::Failed;
    };
    let dir = std::env::temp_dir().join(format!("platypus-video-settings-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    if let Err(e) = write_dialog_game(&dir, current, icon) {
        tracing::warn!("video settings prompt: {e}");
        return Outcome::Failed;
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
    let code = cmd.status().ok().and_then(|s| s.code());
    let outcome = match code {
        _ if screenshot.is_some() => Outcome::Screenshot,
        Some(0) => Outcome::Quit,
        Some(EXIT_PLAY) => Choice::from_prefs(Prefs::open(dir.join("prefs.json")).all())
            .map_or(Outcome::Failed, Outcome::Play),
        _ => Outcome::Failed,
    };
    if outcome == Outcome::Failed {
        let log = std::fs::read_to_string(dir.join("dialog.log")).unwrap_or_default();
        tracing::warn!(
            "the video settings prompt did not run (exit {code:?}); starting the game\n{log}"
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
    outcome
}

/// The dialog's folder: `project.json` (a fixed-size UI app with the game's title and icon) and
/// `build/game.pbundle`, its source the dialog's TSX after the constants.
fn write_dialog_game(dir: &Path, current: &Current, icon: Option<&Path>) -> std::io::Result<()> {
    std::fs::create_dir_all(dir.join("build"))?;
    let icon_name = icon.and_then(|p| {
        let name = format!(
            "icon.{}",
            p.extension().and_then(|e| e.to_str()).unwrap_or("png")
        );
        std::fs::copy(p, dir.join(&name)).ok().map(|_| name)
    });
    let title = format!("{} - Video settings", current.title);
    let mut project = json!({
        "name": "video-settings",
        "type": "app",
        "title": title,
        "company": "Platypus",
        "identifier": "dev.platypus.video-settings",
        "display": { "mode": "Windowed", "width": DIALOG_SIZE.0, "height": dialog_height(current.resizable), "resizable": false },
        "fps": false,
    });
    if let Some(name) = icon_name {
        project["icon"] = json!(name);
    }
    let bundle = json!({
        "format": 1,
        "title": title,
        "company": "Platypus",
        "identifier": "dev.platypus.video-settings",
        "entry": "video_prompt.tsx",
        "source": format!("{}{SIZES_SOURCE}{DIALOG_SOURCE}", current.constants()),
        "files": [],
        "css": null,
        "assets": [],
        "patterns": [],
    });
    std::fs::write(dir.join("project.json"), project.to_string())?;
    std::fs::write(dir.join("build").join("game.pbundle"), bundle.to_string())
}

/// The game's icon file, when it has one.
pub fn icon_path(code_root: &Path, manifest: &Manifest) -> Option<PathBuf> {
    manifest
        .icon
        .as_deref()
        .and_then(|i| platypus_core::resolve_asset(code_root, i))
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
        assert!(constants.contains("const RESIZABLE = false;"));
        assert!(constants.contains("const LEVEL = \"Low\";"));
        assert!(constants.contains("const VSYNC = true;"), "vsync starts on");
        // The dialog hides both rows without RESIZABLE, and its choice leaves the display alone.
        assert!(DIALOG_SOURCE.contains("{RESIZABLE ? ("));
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
}
