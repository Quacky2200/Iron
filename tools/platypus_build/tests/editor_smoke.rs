// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

//! The native editor (`editor/`) builds and runs: a windowless `platypus-app editor --bench` run
//! of a few hundred frames with clicks on the hierarchy, the dock's tabs and the console, which
//! must end cleanly (no crash, no panic). Both faults it was written after came from clicks: an
//! optional call of a missing row handler (`onSelect?.(i)`) and a setter called by a promise
//! after the Project browser closed. Needs the engine host (`PLATYPUS_HOST`, else the newest
//! `build/{release,debug}/platypus-app`, see `find_host`) and a GPU; no window opens. The run takes
//! about 4 s with a release host and 6 s with a debug one, after building the editor.

use platypus_build::{Options, build, package::find_host};
use std::path::Path;
use std::process::Command;

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
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let editor = repo.join("editor");
    build(
        &editor,
        &Options {
            no_tsc: true,
            ..Default::default()
        },
    )
    .unwrap_or_else(|e| panic!("building the editor: {e}"));
    let host = find_host(&editor, None).unwrap_or_else(|e| panic!("{e}"));
    let dir = std::env::temp_dir().join(format!("platypus-editor-smoke-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let spec = dir.join("input.json");
    std::fs::write(&spec, SPEC).unwrap();
    let log = dir.join("run.log");
    let status = Command::new(&host)
        .arg("editor")
        .arg("--project")
        .arg(repo.join("examples/arcade"))
        .args(["--bench", "--fixed-dt", "0.016", "--size", "1600x900"])
        .arg("--bench-input")
        .arg(&spec)
        .arg("--bench-out")
        .arg(dir.join("out.json"))
        .current_dir(&repo)
        .env("PLATYPUS_LOG_FILE", &log)
        .status()
        .unwrap();
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
    let _ = std::fs::remove_dir_all(&dir);
}
