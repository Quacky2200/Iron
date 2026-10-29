// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

//! The native editor (`editor/`) builds and runs: a windowless `platypus-app editor --bench` run
//! of a few hundred frames with clicks on the hierarchy, the dock's tabs and the console, which
//! must end cleanly (no crash, no panic). Both faults it was written after came from clicks: an
//! optional call of a missing row handler (`onSelect?.(i)`) and a setter called by a promise
//! after the Project browser closed. Needs the engine host (`PLATYPUS_HOST`, else the newest
//! `build/{release,debug}/platypus-app`, see `find_host`) and a GPU; no window opens. The run takes
//! about 4 s with a release host and 6 s with a debug one, after building the editor. Shorter
//! runs compare `--bench-shots` pictures of the Scene view.

use platypus_build::{Options, build, package::find_host};
use std::path::Path;
use std::process::Command;
use std::sync::Mutex;

/// Clicks spaced so each one's work (and the Project browser's listing) lands before the next.
const SPEC: &str = r#"{ "warmup": 30, "segments": [
  { "name": "clicks", "frames": 400, "input": [
    { "frame": 0, "kind": "clickText", "text": "Cube" },
    { "frame": 20, "kind": "clickText", "text": "Sphere" },
    { "frame": 40, "kind": "clickText", "text": "Project" },
    { "frame": 60, "kind": "clickText", "text": "Console" },
    { "frame": 80, "kind": "clickText", "text": "Clear" },
    { "frame": 100, "kind": "clickText", "text": "Game" },
    { "frame": 120, "kind": "clickText", "text": "Scene" },
    { "frame": 140, "kind": "clickText", "text": "Floor" },
    { "frame": 160, "kind": "clickText", "text": "Project" },
    { "frame": 170, "kind": "clickText", "text": "Console" },
    { "frame": 180, "kind": "clickText", "text": "Collapse" },
    { "frame": 200, "kind": "mouseMove", "x": 700, "y": 340 },
    { "frame": 210, "kind": "click", "x": 700, "y": 340 }
  ] }
] }"#;

// M5-EDT-28
#[test]
fn the_editor_runs_and_takes_clicks_without_crashing() {
    let run = run_editor("smoke", SPEC, false);
    let _ = std::fs::remove_dir_all(&run);
}

/// Pictures of the Scene view (`--bench-shots`: the last frame of each segment), before and
/// after going to the Game tab and back.
const TABS_SPEC: &str = r#"{ "warmup": 30, "segments": [
  { "name": "start", "frames": 20, "input": [] },
  { "name": "game", "frames": 30, "input": [ { "frame": 0, "kind": "clickText", "text": "Game" } ] },
  { "name": "scene", "frames": 30, "input": [ { "frame": 0, "kind": "clickText", "text": "Scene" } ] }
] }"#;

/// Pixels of `a` and `b` inside the Scene view (default layout at 1600x900, clear of the
/// toolbar, the axis gizmo and the panel edges) that differ by more than a little.
fn scene_pixels_changed(a: &Path, b: &Path) -> usize {
    let a = image::open(a).unwrap().to_rgb8();
    let b = image::open(b).unwrap().to_rgb8();
    let mut n = 0;
    for y in 120..600 {
        for x in 400..1150 {
            let (p, q) = (a.get_pixel(x, y), b.get_pixel(x, y));
            if p.0.iter().zip(q.0).any(|(s, t)| s.abs_diff(t) > 24) {
                n += 1;
            }
        }
    }
    n
}

// M5-EDT-29
#[test]
fn the_scene_view_keeps_its_camera_after_the_game_tab() {
    let run = run_editor("tabs", TABS_SPEC, true);
    let shot = |name: &str| run.join("shots").join(name);
    // Back on the Scene tab, the view is its own camera's again (grid and icons), not the
    // game camera's.
    let back = scene_pixels_changed(&shot("01-start.png"), &shot("03-scene.png"));
    assert!(
        back < 200,
        "the Scene view changed after Scene > Game > Scene ({back} pixels)"
    );
    let _ = std::fs::remove_dir_all(&run);
}

/// Builds the editor and runs `platypus-app editor --project examples/arcade --bench` with the
/// input `spec` (and `--bench-shots <dir>/shots` with `shots`), checking it ended cleanly.
/// Returns the run's folder.
fn run_editor(name: &str, spec: &str, shots: bool) -> std::path::PathBuf {
    // The tests run in parallel; one at a time builds `editor/build`.
    static BUILD: Mutex<()> = Mutex::new(());
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let editor = repo.join("editor");
    let host = {
        let _one = BUILD.lock().unwrap_or_else(|e| e.into_inner());
        build(
            &editor,
            &Options {
                no_tsc: true,
                ..Default::default()
            },
        )
        .unwrap_or_else(|e| panic!("building the editor: {e}"));
        find_host(&editor, None).unwrap_or_else(|e| panic!("{e}"))
    };
    let dir = std::env::temp_dir().join(format!("platypus-editor-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let spec_path = dir.join("input.json");
    std::fs::write(&spec_path, spec).unwrap();
    let log = dir.join("run.log");
    let mut cmd = Command::new(&host);
    cmd.arg("editor")
        .arg("--project")
        .arg(repo.join("examples/arcade"))
        .args(["--bench", "--fixed-dt", "0.016", "--size", "1600x900"])
        .arg("--bench-input")
        .arg(&spec_path)
        .arg("--bench-out")
        .arg(dir.join("out.json"))
        .current_dir(&repo)
        .env("PLATYPUS_LOG_FILE", &log);
    if shots {
        cmd.arg("--bench-shots").arg(dir.join("shots"));
    }
    let status = cmd.status().unwrap();
    let log_text = std::fs::read_to_string(&log).unwrap_or_default();
    assert!(
        status.success(),
        "{} ended with {status}:\n{log_text}",
        host.display()
    );
    // `bench: no text ..`: a click that found nothing to press tested nothing.
    for bad in ["crash", "panic", "ERROR", "bench: no text"] {
        assert!(!log_text.contains(bad), "`{bad}` in the log:\n{log_text}");
    }
    assert!(log_text.contains("Platypus editor started"), "{log_text}");
    dir
}
