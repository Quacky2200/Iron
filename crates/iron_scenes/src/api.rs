//! The script API of scenes: `SceneManager`, `Scene` and `Prefab`, declared once with
//! `#[iron_api]` for compiled scripts. `GameObject.instantiate(..)` and `dontDestroyOnLoad` are
//! lowered by the compiler onto `SceneManager.instantiate` / `dontDestroyOnLoad`.

use crate::manager::{self, PrefabRef, SceneRef};
use iron_api::callback::HandleArg;
use iron_api::{Callback, Handle, iron_api};
use iron_scene::ObjectId;

fn some_object(o: ObjectId) -> Option<ObjectId> {
    // A handle of 0 is "no object".
    (o.to_raw() != 0).then_some(o)
}

fn none() -> ObjectId {
    ObjectId::from_raw(0)
}

fn vec3(x: f64, y: f64, z: f64) -> Option<[f64; 3]> {
    (!x.is_nan() && !y.is_nan() && !z.is_nan()).then_some([x, y, z])
}

fn quat(x: f64, y: f64, z: f64, w: f64) -> Option<[f64; 4]> {
    (!x.is_nan() && !y.is_nan() && !z.is_nan() && !w.is_nan()).then_some([x, y, z, w])
}

/// The manifests of this crate's script API, for the compiler.
pub fn manifests() -> Vec<String> {
    vec![scene_manager_api::MANIFEST.to_json()]
}

#[iron_api(
    namespace = "SceneManager",
    quickjs,
    handles(Scene = SceneRef, Prefab = PrefabRef, GameObject = ObjectId)
)]
pub mod scene_manager_api {
    use super::*;

    /// Loaded scenes (not counting `dontDestroyOnLoad`).
    #[property]
    pub fn scene_count() -> f64 {
        manager::sync();
        manager::scene_count() as f64
    }

    /// The scene at an index (no scene past the end).
    pub fn get_scene_at(index: f64) -> SceneRef {
        manager::sync();
        SceneRef(manager::scene_at(index as usize))
    }

    /// The loaded scene of that name, or no scene.
    pub fn get_scene_by_name(name: &str) -> SceneRef {
        manager::sync();
        SceneRef(manager::scene_by_name(name))
    }

    /// The scene new objects go into.
    pub fn get_active_scene() -> SceneRef {
        manager::sync();
        SceneRef(manager::active())
    }

    /// Makes a loaded scene active (false if it is not loaded).
    pub fn set_active_scene(scene: SceneRef) -> bool {
        manager::set_active(scene.0)
    }

    /// A new empty scene, loaded alongside the others (no scene if the name is taken).
    pub fn create_scene(name: &str) -> SceneRef {
        manager::sync();
        SceneRef(manager::create_scene(name))
    }

    /// Loads saved scene data (JSON text). `"single"` (the default) unloads every other scene
    /// first and makes it active; `"additive"` adds it. A bad file gives no scene and a warning.
    #[optional(mode)]
    pub fn load_scene(data: &str, mode: &str) -> SceneRef {
        match manager::load_scene(data, mode == "additive") {
            Ok(h) => SceneRef(h),
            Err(e) => {
                crate::warn(e);
                SceneRef(0)
            }
        }
    }

    /// Asks the host to read a scene file from the game folder and load it; the number says which
    /// request finished.
    #[optional(mode)]
    pub fn load_scene_async(path: &str, mode: &str) -> f64 {
        manager::request_load(path, mode == "additive") as f64
    }

    /// Destroys a scene's objects and drops it. The last scene cannot be unloaded.
    pub fn unload_scene(scene: SceneRef) -> bool {
        manager::unload_scene(scene.0)
    }

    /// Moves a root object (with its children) to another loaded scene.
    pub fn move_game_object_to_scene(object: ObjectId, scene: SceneRef) {
        manager::move_to_scene(object, scene.0);
    }

    /// Moves every object of `source` into `destination` and unloads `source`.
    pub fn merge_scenes(source: SceneRef, destination: SceneRef) {
        manager::merge_scenes(source.0, destination.0);
    }

    /// Calls `f(scene, mode)` after a scene loads (`mode` is `"single"` or `"additive"`). Returns
    /// a token for `offSceneEvent` (the TypeScript returned an unsubscribe function).
    pub fn on_scene_loaded(f: Callback<(HandleArg<SceneRef>, String)>) -> u32 {
        manager::on_loaded(f)
    }

    /// Calls `f(scene)` after a scene is unloaded.
    pub fn on_scene_unloaded(f: Callback<(HandleArg<SceneRef>,)>) -> u32 {
        manager::on_unloaded(f)
    }

    /// Calls `f(previous, next)` when the active scene changes.
    pub fn on_active_scene_changed(f: Callback<(HandleArg<SceneRef>, HandleArg<SceneRef>)>) -> u32 {
        manager::on_active_changed(f)
    }

    /// Stops a listener from one of the `onScene...` functions; false when the token is unknown.
    pub fn off_scene_event(token: u32) -> bool {
        manager::off(token)
    }

    /// Keeps the object (its root) when scenes unload.
    pub fn dont_destroy_on_load(object: ObjectId) {
        manager::dont_destroy_on_load(object);
    }

    /// The scene an object is in.
    pub fn scene_of(object: ObjectId) -> SceneRef {
        SceneRef(manager::scene_of(object))
    }

    /// Copies an object with its children and components. A position (world, or local under a
    /// parent) and rotation left out (NaN) are the original's; so is no parent.
    #[optional(px, py, pz, qx, qy, qz, qw, parent)]
    #[allow(clippy::too_many_arguments)]
    pub fn instantiate(
        original: ObjectId,
        px: f64,
        py: f64,
        pz: f64,
        qx: f64,
        qy: f64,
        qz: f64,
        qw: f64,
        parent: ObjectId,
    ) -> ObjectId {
        manager::instantiate(
            original,
            vec3(px, py, pz),
            quat(qx, qy, qz, qw),
            some_object(parent),
        )
        .unwrap_or_else(none)
    }

    impl SceneRef {
        #[property]
        pub fn name(self) -> String {
            manager::info(self.0).map(|i| i.name).unwrap_or_default()
        }

        /// Where it was loaded from (empty if it was not loaded from a file).
        #[property]
        pub fn path(self) -> String {
            manager::info(self.0)
                .and_then(|i| i.path)
                .unwrap_or_default()
        }

        #[property]
        pub fn handle(self) -> f64 {
            f64::from(self.0)
        }

        /// False once unloaded.
        #[property]
        pub fn is_loaded(self) -> bool {
            manager::is_loaded(self.0)
        }

        #[property]
        pub fn root_count(self) -> f64 {
            manager::roots(self.0).len() as f64
        }

        /// The i-th root object (no object past the end).
        pub fn root_at(self, index: f64) -> ObjectId {
            manager::roots(self.0)
                .get(index as usize)
                .copied()
                .unwrap_or_else(none)
        }

        /// Everything in it as scene data (JSON text).
        pub fn serialize(self) -> String {
            manager::serialize(self.0).unwrap_or_default()
        }
    }

    impl PrefabRef {
        /// Captures an object and everything under it as it is now.
        pub fn from_object(object: ObjectId) -> PrefabRef {
            PrefabRef(manager::prefab_from_object(object))
        }

        /// A prefab from saved data (`toJSON()`'s output); no prefab for bad data.
        pub fn from_data(data: &str) -> PrefabRef {
            match manager::prefab_from_data(data, None) {
                Ok(h) => PrefabRef(h),
                Err(e) => {
                    crate::warn(e);
                    PrefabRef(0)
                }
            }
        }

        #[property]
        pub fn name(self) -> String {
            manager::prefab_name(self.0)
        }

        /// As JSON text, for a `.prefab.json` file.
        #[js_name = "toJSON"]
        pub fn to_json(self) -> String {
            manager::prefab_json(self.0)
        }

        /// Places a copy (see `SceneManager.instantiate` for the left-out values).
        #[optional(px, py, pz, qx, qy, qz, qw, parent)]
        #[allow(clippy::too_many_arguments)]
        pub fn instantiate(
            self,
            px: f64,
            py: f64,
            pz: f64,
            qx: f64,
            qy: f64,
            qz: f64,
            qw: f64,
            parent: ObjectId,
        ) -> ObjectId {
            manager::instantiate_prefab(
                self.0,
                vec3(px, py, pz),
                quat(qx, qy, qz, qw),
                some_object(parent),
            )
            .unwrap_or_else(none)
        }
    }
}
