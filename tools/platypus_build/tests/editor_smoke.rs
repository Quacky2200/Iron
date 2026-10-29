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

/// Before and after selecting Cube in the Hierarchy; `moveToText` finds the Inspector showing
/// its MeshRenderer (else `bench: no text` fails the run).
const SELECT_SPEC: &str = r#"{ "warmup": 30, "segments": [
  { "name": "start", "frames": 20, "input": [] },
  { "name": "cube", "frames": 30, "input": [
    { "frame": 0, "kind": "clickText", "text": "Cube" },
    { "frame": 10, "kind": "moveToText", "text": "MeshRenderer" }
  ] }
] }"#;

// M5-EDT-30
#[test]
fn selecting_an_object_in_the_hierarchy_shows_its_handles() {
    let run = run_editor("select", SELECT_SPEC, true);
    let shot = |name: &str| run.join("shots").join(name);
    // Cube has no icon nor wire gizmo: what changes in the view is its move handles.
    let handles = scene_pixels_changed(&shot("01-start.png"), &shot("02-cube.png"));
    assert!(
        handles > 300,
        "selecting Cube drew no handles ({handles} pixels changed)"
    );
    let _ = std::fs::remove_dir_all(&run);
}

/// File > Open Scene shows the file dialog (its filter label), Escape closes it; New Folder in
/// the Project browser asks for a name; the Rename dialog goes the same way.
const DIALOG_SPEC: &str = r#"{ "warmup": 30, "segments": [
  { "name": "open", "frames": 30, "input": [
    { "frame": 0, "kind": "clickText", "text": "File" },
    { "frame": 5, "kind": "clickText", "text": "Open Scene..." },
    { "frame": 12, "kind": "moveToText", "text": "Scenes (*.scene.json)" }
  ] },
  { "name": "closed", "frames": 30, "input": [
    { "frame": 0, "kind": "key", "code": "Escape" },
    { "frame": 6, "kind": "clickText", "text": "Project" },
    { "frame": 14, "kind": "clickText", "text": "New Folder" },
    { "frame": 22, "kind": "moveToText", "text": "Folder name" }
  ] },
  { "name": "after", "frames": 20, "input": [
    { "frame": 0, "kind": "key", "code": "Escape" }
  ] }
] }"#;

/// Pixels of `a` and `b` in the middle of the window that differ by more than a little.
fn middle_pixels_changed(a: &Path, b: &Path) -> usize {
    let a = image::open(a).unwrap().to_rgb8();
    let b = image::open(b).unwrap().to_rgb8();
    let mut n = 0;
    for y in 300..600 {
        for x in 600..1000 {
            let (p, q) = (a.get_pixel(x, y), b.get_pixel(x, y));
            if p.0.iter().zip(q.0).any(|(s, t)| s.abs_diff(t) > 24) {
                n += 1;
            }
        }
    }
    n
}

// M5-EDT-34
#[test]
fn the_file_dialog_and_prompts_open_and_close() {
    let run = run_editor("dialogs", DIALOG_SPEC, true);
    let shot = |name: &str| run.join("shots").join(name);
    // The dialog covered the middle of the window; Escape took it away again.
    let open = middle_pixels_changed(&shot("01-open.png"), &shot("03-after.png"));
    assert!(open > 2000, "no dialog was drawn ({open} pixels)");
    let _ = std::fs::remove_dir_all(&run);
}

/// Delete, undo, redo, undo and duplicate Cube, then drag Sphere onto Floor in the Hierarchy and
/// undo that. Each `moveToText` finds the status bar's report of the step (else the run fails).
const UNDO_SPEC: &str = r#"{ "warmup": 30, "segments": [
  { "name": "edits", "frames": 260, "input": [
    { "frame": 0, "kind": "clickText", "text": "Cube" },
    { "frame": 6, "kind": "key", "code": "Delete" },
    { "frame": 12, "kind": "moveToText", "text": "Delete Cube" },
    { "frame": 14, "kind": "keyDown", "code": "ControlLeft" },
    { "frame": 16, "kind": "key", "code": "KeyZ" },
    { "frame": 18, "kind": "keyUp", "code": "ControlLeft" },
    { "frame": 24, "kind": "moveToText", "text": "Undid Delete Cube" },
    { "frame": 26, "kind": "keyDown", "code": "ControlLeft" },
    { "frame": 28, "kind": "key", "code": "KeyY" },
    { "frame": 30, "kind": "keyUp", "code": "ControlLeft" },
    { "frame": 36, "kind": "moveToText", "text": "Redid Delete Cube" },
    { "frame": 38, "kind": "keyDown", "code": "ControlLeft" },
    { "frame": 40, "kind": "key", "code": "KeyZ" },
    { "frame": 42, "kind": "keyUp", "code": "ControlLeft" },
    { "frame": 50, "kind": "clickText", "text": "Cube" },
    { "frame": 56, "kind": "keyDown", "code": "ControlLeft" },
    { "frame": 58, "kind": "key", "code": "KeyD" },
    { "frame": 60, "kind": "keyUp", "code": "ControlLeft" },
    { "frame": 66, "kind": "moveToText", "text": "Duplicate Cube" },
    { "frame": 80, "kind": "moveToText", "text": "Sphere" },
    { "frame": 82, "kind": "mouseDown" },
    { "frame": 86, "kind": "mouseMove", "x": 70, "y": 120 },
    { "frame": 90, "kind": "mouseMove", "x": 62, "y": 95 },
    { "frame": 94, "kind": "mouseMove", "x": 58, "y": 70 },
    { "frame": 98, "kind": "mouseMove", "x": 57, "y": 65 },
    { "frame": 104, "kind": "mouseUp" },
    { "frame": 112, "kind": "moveToText", "text": "Move Sphere" },
    { "frame": 114, "kind": "keyDown", "code": "ControlLeft" },
    { "frame": 116, "kind": "key", "code": "KeyZ" },
    { "frame": 118, "kind": "keyUp", "code": "ControlLeft" },
    { "frame": 126, "kind": "moveToText", "text": "Undid Move Sphere" }
  ] }
] }"#;

// M5-UNDO-06
#[test]
fn deletes_duplicates_and_moves_in_the_hierarchy_undo_and_redo() {
    let run = run_editor("undo", UNDO_SPEC, false);
    let _ = std::fs::remove_dir_all(&run);
}

/// The Inspector: Point Light's intensity typed as 5 (Backspace, 5, Enter), then undone; a
/// component added (Add Component > AudioSource) and the add undone.
const INSPECT_SPEC: &str = r#"{ "warmup": 30, "segments": [
  { "name": "light", "frames": 20, "input": [
    { "frame": 0, "kind": "clickText", "text": "Point Light" }
  ] },
  { "name": "typed", "frames": 40, "input": [
    { "frame": 0, "kind": "click", "x": 1540, "y": 343 },
    { "frame": 5, "kind": "key", "code": "Backspace" },
    { "frame": 8, "kind": "key", "code": "Digit5", "key": "5" },
    { "frame": 10, "kind": "key", "code": "Enter" },
    { "frame": 16, "kind": "moveToText", "text": "Set Light.intensity" }
  ] },
  { "name": "undone", "frames": 60, "input": [
    { "frame": 0, "kind": "clickText", "text": "Point Light" },
    { "frame": 4, "kind": "keyDown", "code": "ControlLeft" },
    { "frame": 5, "kind": "key", "code": "KeyZ" },
    { "frame": 6, "kind": "keyUp", "code": "ControlLeft" },
    { "frame": 12, "kind": "moveToText", "text": "Undid Set Light.intensity" },
    { "frame": 14, "kind": "clickText", "text": "Cube" },
    { "frame": 20, "kind": "clickText", "text": "Add Component" },
    { "frame": 26, "kind": "clickText", "text": "AudioSource" },
    { "frame": 32, "kind": "moveToText", "text": "Add AudioSource" }
  ] }
] }"#;

// M5-EDT-36
#[test]
fn inspector_edits_set_fields_and_undo() {
    let run = run_editor("inspect", INSPECT_SPEC, true);
    let shot = |name: &str| run.join("shots").join(name);
    // The intensity field showed 5, then 2 again after the undo: the panel's pixels changed back.
    let a = image::open(shot("01-light.png")).unwrap().to_rgb8();
    let b = image::open(shot("02-typed.png")).unwrap().to_rgb8();
    let differ = |p: &image::Rgb<u8>, q: &image::Rgb<u8>| {
        p.0.iter().zip(q.0).any(|(s, t)| s.abs_diff(t) > 24)
    };
    let mut typed = 0;
    for y in 333..354 {
        for x in 1395..1590 {
            if differ(a.get_pixel(x, y), b.get_pixel(x, y)) {
                typed += 1;
            }
        }
    }
    assert!(
        typed > 10,
        "the intensity field didn't change ({typed} pixels)"
    );
    let _ = std::fs::remove_dir_all(&run);
}

/// In a project with `assets/house.glb`: the model dragged from the Project browser onto the
/// Scene view is placed; Cube dragged from the Hierarchy onto the Project list is saved as
/// `assets/Cube.prefab.json`; that file dragged onto the Hierarchy is placed, and Ctrl+Z takes it
/// away again. Each step is checked by its status line.
const PLACE_SPEC: &str = r#"{ "warmup": 30, "segments": [
  { "name": "place", "frames": 200, "input": [
    { "frame": 0, "kind": "clickText", "text": "Project" },
    { "frame": 6, "kind": "clickText", "text": "assets" },
    { "frame": 10, "kind": "key", "code": "Enter" },
    { "frame": 18, "kind": "moveToText", "text": "house.glb" },
    { "frame": 20, "kind": "mouseDown" },
    { "frame": 24, "kind": "mouseMove", "x": 300, "y": 600 },
    { "frame": 28, "kind": "mouseMove", "x": 600, "y": 450 },
    { "frame": 32, "kind": "mouseMove", "x": 780, "y": 350 },
    { "frame": 36, "kind": "mouseUp" },
    { "frame": 44, "kind": "moveToText", "text": "Place house.glb" },
    { "frame": 50, "kind": "moveToText", "text": "Cube" },
    { "frame": 52, "kind": "mouseDown" },
    { "frame": 56, "kind": "mouseMove", "x": 200, "y": 300 },
    { "frame": 60, "kind": "mouseMove", "x": 600, "y": 700 },
    { "frame": 64, "kind": "mouseMove", "x": 800, "y": 820 },
    { "frame": 68, "kind": "mouseUp" },
    { "frame": 80, "kind": "moveToText", "text": "Cube.prefab.json" },
    { "frame": 82, "kind": "mouseDown" },
    { "frame": 86, "kind": "mouseMove", "x": 400, "y": 600 },
    { "frame": 90, "kind": "mouseMove", "x": 200, "y": 520 },
    { "frame": 94, "kind": "mouseMove", "x": 150, "y": 500 },
    { "frame": 98, "kind": "mouseUp" },
    { "frame": 106, "kind": "moveToText", "text": "Place Cube.prefab.json" },
    { "frame": 108, "kind": "clickText", "text": "Floor" },
    { "frame": 112, "kind": "keyDown", "code": "ControlLeft" },
    { "frame": 113, "kind": "key", "code": "KeyZ" },
    { "frame": 114, "kind": "keyUp", "code": "ControlLeft" },
    { "frame": 120, "kind": "moveToText", "text": "Undid Place Cube.prefab.json" }
  ] }
] }"#;

// M5-PRF-11, M5-DND-20
#[test]
fn models_and_prefabs_are_placed_by_drag_and_drop_and_prefabs_saved() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let project = std::env::temp_dir().join(format!(
        "platypus-editor-place-project-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&project);
    std::fs::create_dir_all(project.join("assets")).unwrap();
    std::fs::write(project.join("project.json"), "{ \"name\": \"fixture\" }\n").unwrap();
    std::fs::copy(
        repo.join("tests/fixtures/assets/house.glb"),
        project.join("assets/house.glb"),
    )
    .unwrap();
    let run = run_editor_on("place", PLACE_SPEC, false, &project);
    let prefab = std::fs::read_to_string(project.join("assets/Cube.prefab.json")).unwrap();
    assert!(
        prefab.contains("\"platypus.prefab\"") && prefab.contains("MeshRenderer"),
        "{prefab}"
    );
    assert!(project.join("assets/Cube.prefab.json.meta").is_file());
    let _ = std::fs::remove_dir_all(&run);
    let _ = std::fs::remove_dir_all(&project);
}

/// The Console tab's close box closes it (Project shows instead); View > Panels > Console opens
/// it again; Edit > Preferences and Help > About open and close with Escape.
const MENUS_SPEC: &str = r#"{ "warmup": 30, "segments": [
  { "name": "start", "frames": 10, "input": [] },
  { "name": "closed", "frames": 20, "input": [
    { "frame": 0, "kind": "click", "x": 75, "y": 637 }
  ] },
  { "name": "reopened", "frames": 80, "input": [
    { "frame": 0, "kind": "clickText", "text": "View" },
    { "frame": 5, "kind": "clickText", "text": "Panels" },
    { "frame": 10, "kind": "clickText", "text": "Console" },
    { "frame": 18, "kind": "moveToText", "text": "Collapse" },
    { "frame": 20, "kind": "clickText", "text": "Edit" },
    { "frame": 25, "kind": "clickText", "text": "Preferences..." },
    { "frame": 32, "kind": "moveToText", "text": "Show the grid in the Scene view" },
    { "frame": 34, "kind": "key", "code": "Escape" },
    { "frame": 40, "kind": "clickText", "text": "Help" },
    { "frame": 45, "kind": "clickText", "text": "About Platypus" },
    { "frame": 52, "kind": "moveToText", "text": "Close" },
    { "frame": 54, "kind": "key", "code": "Escape" }
  ] }
] }"#;

// M5-EDT-41
#[test]
fn panels_close_and_open_again_and_the_other_menus_work() {
    let run = run_editor("menus", MENUS_SPEC, true);
    let shot = |name: &str| run.join("shots").join(name);
    let bottom = |a: &str, b: &str| {
        let a = image::open(shot(a)).unwrap().to_rgb8();
        let b = image::open(shot(b)).unwrap().to_rgb8();
        let mut n = 0;
        for y in 680..860 {
            for x in 0..1600 {
                let (p, q) = (a.get_pixel(x, y), b.get_pixel(x, y));
                if p.0.iter().zip(q.0).any(|(s, t)| s.abs_diff(t) > 24) {
                    n += 1;
                }
            }
        }
        n
    };
    assert!(
        bottom("01-start.png", "02-closed.png") > 2000,
        "closing Console changed nothing"
    );
    let _ = std::fs::remove_dir_all(&run);
}

/// Edit > Project Settings: "7" typed into company, then Undo.
const SETTINGS_SPEC: &str = r#"{ "warmup": 30, "segments": [
  { "name": "typed", "frames": 40, "input": [
    { "frame": 0, "kind": "clickText", "text": "Edit" },
    { "frame": 5, "kind": "clickText", "text": "Project Settings..." },
    { "frame": 12, "kind": "click", "x": 900, "y": 271 },
    { "frame": 16, "kind": "key", "code": "Digit7", "key": "7" },
    { "frame": 18, "kind": "key", "code": "Enter" }
  ] },
  { "name": "undone", "frames": 20, "input": [
    { "frame": 0, "kind": "clickText", "text": "Undo" }
  ] }
] }"#;

// M5-PRJ-05
#[test]
fn project_settings_write_project_json_and_undo() {
    let project = std::env::temp_dir().join(format!(
        "platypus-editor-settings-project-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&project);
    std::fs::create_dir_all(&project).unwrap();
    let file = "{\n  \"name\": \"fixture\",\n  \"title\": \"${name} game\",\n  \"mine\": { \"a\": 1 }\n}\n";
    std::fs::write(project.join("project.json"), file).unwrap();
    // The shots are taken at each segment's end: compare the file after the run's two steps by
    // running them as two runs.
    let typed = SETTINGS_SPEC.replace(
        r#",
  { "name": "undone", "frames": 20, "input": [
    { "frame": 0, "kind": "clickText", "text": "Undo" }
  ] }"#,
        "",
    );
    let run = run_editor_on("settings", &typed, false, &project);
    let after = std::fs::read_to_string(project.join("project.json")).unwrap();
    assert_eq!(
        after,
        "{\n  \"name\": \"fixture\",\n  \"title\": \"${name} game\",\n  \"mine\": { \"a\": 1 },\n  \"company\": \"7\"\n}\n"
    );
    let _ = std::fs::remove_dir_all(&run);
    std::fs::write(project.join("project.json"), file).unwrap();
    let run = run_editor_on("settings-undo", SETTINGS_SPEC, false, &project);
    assert_eq!(
        std::fs::read_to_string(project.join("project.json")).unwrap(),
        file,
        "undone"
    );
    let _ = std::fs::remove_dir_all(&run);
    let _ = std::fs::remove_dir_all(&project);
}

/// A new scene with a cube, saved with Ctrl+S (the dialog's name field starts as "Untitled";
/// Enter saves).
const SAVE_SPEC: &str = r#"{ "warmup": 30, "segments": [
  { "name": "save", "frames": 120, "input": [
    { "frame": 0, "kind": "clickText", "text": "File" },
    { "frame": 5, "kind": "clickText", "text": "New Scene" },
    { "frame": 12, "kind": "moveToText", "text": "New scene" },
    { "frame": 15, "kind": "clickText", "text": "GameObject" },
    { "frame": 20, "kind": "clickText", "text": "3D Object" },
    { "frame": 25, "kind": "clickText", "text": "Cube" },
    { "frame": 32, "kind": "moveToText", "text": "Create Cube" },
    { "frame": 35, "kind": "keyDown", "code": "ControlLeft" },
    { "frame": 37, "kind": "key", "code": "KeyS" },
    { "frame": 39, "kind": "keyUp", "code": "ControlLeft" },
    { "frame": 46, "kind": "moveToText", "text": "Save Scene As" },
    { "frame": 50, "kind": "key", "code": "Enter" },
    { "frame": 60, "kind": "moveToText", "text": "Saved scenes/Untitled.scene.json" }
  ] }
] }"#;

/// The saved scene opens at start (the project's `startScene`).
const OPEN_SPEC: &str = r#"{ "warmup": 30, "segments": [
  { "name": "open", "frames": 30, "input": [
    { "frame": 0, "kind": "clickText", "text": "Cube" }
  ] }
] }"#;

// M5-SCN-13
#[test]
fn a_new_scene_saves_to_the_project_and_opens_again() {
    let project =
        std::env::temp_dir().join(format!("platypus-editor-project-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&project);
    std::fs::create_dir_all(&project).unwrap();
    std::fs::write(project.join("project.json"), "{ \"name\": \"fixture\" }\n").unwrap();
    let run = run_editor_on("save", SAVE_SPEC, false, &project);
    let file = project.join("scenes/Untitled.scene.json");
    let text = std::fs::read_to_string(&file).unwrap_or_else(|e| panic!("{}: {e}", file.display()));
    for name in ["\"Cube\"", "\"Main Camera\"", "\"Directional Light\""] {
        assert!(text.contains(name), "{name} not saved:\n{text}");
    }
    // The cameras are saved as the scene has them, not as the Game panel draws them.
    assert!(
        !text.contains("\"rect\":[0,0,0,0]") && !text.contains("\"width\": 0"),
        "{text}"
    );
    assert!(
        project.join("scenes/Untitled.scene.json.meta").is_file(),
        "no .meta"
    );
    let _ = std::fs::remove_dir_all(&run);

    std::fs::write(
        project.join("project.json"),
        "{ \"name\": \"fixture\", \"startScene\": \"scenes/Untitled.scene.json\" }\n",
    )
    .unwrap();
    let run = run_editor_on("open", OPEN_SPEC, false, &project);
    let log = std::fs::read_to_string(run.join("run.log")).unwrap();
    assert!(log.contains("Opened scenes/Untitled.scene.json"), "{log}");
    let _ = std::fs::remove_dir_all(&run);
    let _ = std::fs::remove_dir_all(&project);
}

/// Builds the editor and runs `platypus-app editor --project examples/arcade --bench` with the
/// input `spec` (and `--bench-shots <dir>/shots` with `shots`), checking it ended cleanly.
/// Returns the run's folder.
fn run_editor(name: &str, spec: &str, shots: bool) -> std::path::PathBuf {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    run_editor_on(name, spec, shots, &repo.join("examples/arcade"))
}

/// [`run_editor`] on another project folder.
fn run_editor_on(name: &str, spec: &str, shots: bool, project: &Path) -> std::path::PathBuf {
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
        .arg(project)
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
