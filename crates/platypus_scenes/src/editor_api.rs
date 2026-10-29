// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

//! `EditorScene`: what the editor asks of scenes. Snapshots of an object and what is under it
//! (undo of deletes, copy and paste), and scene files in the project: new, open and save. Like
//! `Editor`, it works only in the editor (`platypus-app editor --project <folder>`); failures
//! warn and leave their reason in `Editor.lastError`. Files are project paths
//! (`platypus_engine_api::editor`'s rules).

use crate::data::{Saver, load_objects};
use crate::manager;
use platypus_api::{Handle, platypus_api};
use platypus_engine_api::editor::{files, project_op};
use platypus_scene::ObjectId;
use platypus_scene::api::with_scene;
use serde_json::{Value, json};

/// An object and what is under it as JSON text: components, children and references inside it
/// to each other; references outside and assets stay live (it is for this session only).
pub fn snapshot(object: ObjectId) -> Option<String> {
    crate::ensure_kinds();
    manager::sync();
    let mut saver = Saver::new(true);
    saver.number(&[object]);
    let root = saver.save_object(object)?;
    Some(json!({ "format": "platypus.snapshot", "root": root }).to_string())
}

/// Builds a [`snapshot`] again under `parent` (none: a root of the active scene) at sibling
/// `index` (past the end: last), with its local transform as it was.
pub fn restore(text: &str, parent: Option<ObjectId>, index: usize) -> Result<ObjectId, String> {
    crate::ensure_kinds();
    manager::sync();
    let data: Value = serde_json::from_str(text).map_err(|e| format!("not a snapshot: {e}"))?;
    let root = data
        .get("root")
        .filter(|_| data["format"] == "platypus.snapshot")
        .ok_or("not a snapshot")?;
    let tag = if parent.is_some() {
        0
    } else {
        manager::active()
    };
    let made = load_objects(std::slice::from_ref(root), parent, tag, None)
        .into_iter()
        .next()
        .ok_or("nothing was made")?;
    with_scene(|s| s.set_sibling_index(made, index));
    Ok(made)
}

/// The name a scene saved at `path` gets: the file's name without `.scene.json`.
pub fn scene_name_of(path: &str) -> String {
    let file = path.rsplit('/').next().unwrap_or(path);
    let lower = file.to_lowercase();
    for ext in [".scene.json", ".json"] {
        if lower.ends_with(ext) && lower.len() > ext.len() {
            return file[..file.len() - ext.len()].to_string();
        }
    }
    file.to_string()
}

#[platypus_api(namespace = "EditorScene", handles(GameObject = ObjectId))]
pub mod editor_scene_api {
    use super::*;

    fn none() -> ObjectId {
        ObjectId::from_raw(0)
    }

    /// An object and everything under it as text, for `restore` (undo, copy and paste). "" for an
    /// object that can't be copied on its own (a part of a model).
    pub fn snapshot(object: ObjectId) -> String {
        project_op("snapshot", |_| {
            super::snapshot(object).ok_or_else(|| "this object can't be copied on its own".into())
        })
    }

    /// Builds a snapshot again under `parent` (no object: at the root) at sibling `index`;
    /// returns the new object (no object when the text isn't a snapshot).
    pub fn restore(data: &str, parent: ObjectId, index: f64) -> ObjectId {
        project_op("restore", |_| {
            let parent = (parent.to_raw() != 0).then_some(parent);
            super::restore(data, parent, index.max(0.0) as usize).map(Some)
        })
        .unwrap_or_else(none)
    }

    /// Replaces the loaded scenes with an empty one named `name` (objects kept with
    /// `dontDestroyOnLoad`, the editor's own, stay). It has no file until saved.
    pub fn new_scene(name: &str) -> bool {
        project_op("newScene", |_| {
            let text =
                json!({ "format": "platypus.scene", "version": 1, "name": name, "objects": [] });
            manager::load_scene(&text.to_string(), false).map(|_| true)
        })
    }

    /// Opens a project scene file in place of the loaded scenes.
    pub fn open_scene(path: &str) -> bool {
        project_op("openScene", |root| {
            let text = files::read_text(root, path)?;
            manager::load_scene_file(&text, path).map(|_| true)
        })
    }

    /// Saves the active scene to a project file (its folders made, the old file replaced in one
    /// step); the scene takes the file's path and name.
    pub fn save_scene(path: &str) -> bool {
        project_op("saveScene", |root| {
            let scene = manager::active();
            manager::rename_scene(scene, &scene_name_of(path), Some(path));
            let data = manager::scene_value(scene).ok_or("no scene is loaded")?;
            let text = serde_json::to_string_pretty(&data).map_err(|e| e.to_string())?;
            files::write_text(root, path, &format!("{text}\n"))?;
            Ok(true)
        })
    }

    /// The active scene's file ("" for a scene that was never saved).
    #[property]
    pub fn scene_path() -> String {
        manager::sync();
        manager::info(manager::active())
            .and_then(|i| i.path)
            .unwrap_or_default()
    }

    /// The active scene's name.
    #[property]
    pub fn scene_name() -> String {
        manager::sync();
        manager::info(manager::active())
            .map(|i| i.name)
            .unwrap_or_default()
    }
}
