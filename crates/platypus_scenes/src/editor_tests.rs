// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

//! `EditorScene`: snapshots for undo, and scene files in the project.

use crate::editor_api::{editor_scene_api as api, scene_name_of};
use crate::manager;
use platypus_api::Handle;
use platypus_scene::ObjectId;
use platypus_scene::api::with_scene;
use serde_json::Value;

const SCENE: &str = r#"{"format":"platypus.scene","version":1,"name":"Room","objects":[
  {"id":1,"name":"A"},
  {"id":2,"name":"Lamp","position":[1,2,3],"children":[{"id":3,"name":"Bulb","components":[{"id":4,"type":"Camera","properties":{"fieldOfView":40}}]}]},
  {"id":5,"name":"C"}
]}"#;

fn names(list: &[ObjectId]) -> Vec<String> {
    with_scene(|s| {
        list.iter()
            .map(|o| s.name(*o).unwrap_or_default().to_owned())
            .collect()
    })
    .unwrap()
}

/// The editor's project is one for the process: these tests take turns.
static ONE: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn project(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "platypus-editor-scene-{name}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    platypus_engine_api::editor::enable(&dir, &dir);
    dir
}

// M5-UNDO-01
#[test]
fn a_snapshot_builds_the_object_again_in_its_place() {
    manager::reset();
    let mut scene = platypus_scene::Scene::new();
    let _scope = platypus_scene::api::enter(&mut scene);
    let _one = ONE.lock().unwrap_or_else(|e| e.into_inner());
    let dir = project("snap");
    let s = manager::load_scene(SCENE, false).unwrap();
    let lamp = manager::roots(s)[1];
    let text = api::snapshot(lamp);
    assert!(!text.is_empty());
    platypus_scene::api::scene_api::destroy(lamp);
    assert_eq!(names(&manager::roots(s)), ["A", "C"]);
    let back = api::restore(&text, ObjectId::from_raw(0), 1.0);
    assert_ne!(back.to_raw(), 0);
    assert_eq!(names(&manager::roots(s)), ["A", "Lamp", "C"]);
    let kids = with_scene(|sc| sc.children(back).to_vec()).unwrap();
    assert_eq!(names(&kids), ["Bulb"]);
    let saved: Value = serde_json::from_str(&manager::serialize(s).unwrap()).unwrap();
    assert_eq!(
        saved["objects"][1]["position"],
        serde_json::json!([1.0, 2.0, 3.0])
    );
    assert_eq!(
        saved["objects"][1]["children"][0]["components"][0]["type"],
        "Camera"
    );
    // Under a parent.
    let a = manager::roots(s)[0];
    api::restore(&text, a, 0.0);
    let under = with_scene(|sc| sc.children(a).to_vec()).unwrap();
    assert_eq!(names(&under), ["Lamp"]);
    assert_eq!(api::restore("{}", ObjectId::from_raw(0), 0.0).to_raw(), 0);
    platypus_engine_api::editor::reset();
    let _ = std::fs::remove_dir_all(&dir);
}

// M5-SCN-11
#[test]
fn scenes_save_to_project_files_and_open_again() {
    manager::reset();
    let mut scene = platypus_scene::Scene::new();
    let _scope = platypus_scene::api::enter(&mut scene);
    let _one = ONE.lock().unwrap_or_else(|e| e.into_inner());
    let dir = project("files");
    manager::load_scene(SCENE, false).unwrap();
    assert!(api::save_scene("scenes/level.scene.json"));
    assert_eq!(api::scene_path(), "scenes/level.scene.json");
    assert_eq!(api::scene_name(), "level");
    let text = std::fs::read_to_string(dir.join("scenes/level.scene.json")).unwrap();
    let data: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(data["name"], "level");
    assert_eq!(data["objects"].as_array().unwrap().len(), 3);

    assert!(api::new_scene("Untitled"));
    assert_eq!(api::scene_path(), "");
    assert_eq!(manager::roots(manager::active()).len(), 0);
    assert!(api::open_scene("scenes/level.scene.json"));
    assert_eq!(api::scene_path(), "scenes/level.scene.json");
    assert_eq!(
        names(&manager::roots(manager::active())),
        ["A", "Lamp", "C"]
    );
    assert!(!api::open_scene("scenes/missing.scene.json"));
    assert!(platypus_engine_api::editor::editor_api::last_error().contains("missing"));
    assert!(!api::save_scene("../outside.scene.json"));
    platypus_engine_api::editor::reset();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn scene_names_come_from_their_files() {
    assert_eq!(scene_name_of("scenes/level.scene.json"), "level");
    assert_eq!(scene_name_of("a.json"), "a");
    assert_eq!(scene_name_of("x"), "x");
}
