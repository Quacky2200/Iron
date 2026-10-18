//! The scene's script API, declared once with `#[iron_api]` (see `docs/iron-api.md`): the
//! `Scene` namespace of `GameObject` handles and `Vector3` / `Quaternion` values. It sits next to
//! the op-based `ScriptScene`, which keeps working; this is what compiled scripts call directly.
//!
//! The functions work on the scene the host made current with [`enter`]. With none (or for a
//! stale id) they answer with a default: no object, the origin, the identity rotation.

use crate::{ObjectId, Scene, Transform};
use glam::{Quat, Vec3};
use iron_api::{Handle, context, iron_api};
use std::cell::Cell;

thread_local! {
    static SCENE: Cell<*mut Scene> = const { Cell::new(std::ptr::null_mut()) };
}

/// Makes `scene` the one the `Scene` API functions work on, on this thread, until the scope is
/// dropped. The host does this around running scripts.
pub fn enter(scene: &mut Scene) -> context::Scope<'_, Scene> {
    context::enter(&SCENE, scene)
}

/// A handle is the object's index and generation, offset so that **no live object is 0** (0 means
/// "no object" in a script's `GameObject | null`).
impl Handle for ObjectId {
    fn to_raw(self) -> u64 {
        ((self.generation as u64 + 1) << 32) | self.index as u64
    }
    fn from_raw(raw: u64) -> Self {
        ObjectId {
            index: raw as u32,
            generation: ((raw >> 32) as u32).wrapping_sub(1),
        }
    }
}

fn with<R>(f: impl FnOnce(&mut Scene) -> R) -> Option<R> {
    context::with(&SCENE, f)
}

fn none() -> ObjectId {
    // `from_raw(0)` wraps to a generation no slot has, so it misses everywhere.
    ObjectId::from_raw(0)
}

#[iron_api(
    namespace = "Scene",
    handles(GameObject = ObjectId),
    values(
        Vector3 = Vec3 [x: f32, y: f32, z: f32],
        Quaternion = Quat [x: f32, y: f32, z: f32, w: f32]
    )
)]
pub mod scene_api {
    use super::{ObjectId, Quat, Transform, Vec3, none, with};

    /// Creates an enabled root object at the origin.
    pub fn create() -> ObjectId {
        with(|s| s.create("GameObject")).unwrap_or_else(none)
    }

    /// Destroys an object and everything under it.
    pub fn destroy(object: ObjectId) {
        with(|s| s.destroy(object));
    }

    /// Does the object exist (it may have been destroyed)?
    pub fn exists(object: ObjectId) -> bool {
        with(|s| s.contains(object)).unwrap_or(false)
    }

    /// How many objects there are.
    pub fn count() -> u32 {
        with(|s| s.len() as u32).unwrap_or(0)
    }

    /// The parent, or no object for a root.
    pub fn parent(object: ObjectId) -> ObjectId {
        with(|s| s.parent(object)).flatten().unwrap_or_else(none)
    }

    /// Moves an object under a parent (no object: to the root), optionally keeping its world pose.
    pub fn set_parent(object: ObjectId, parent: ObjectId, keep_world: bool) {
        with(|s| {
            let p = s.contains(parent).then_some(parent);
            s.set_parent(object, p, keep_world)
        });
    }

    pub fn enabled(object: ObjectId) -> bool {
        with(|s| s.enabled(object)).unwrap_or(false)
    }

    pub fn set_enabled(object: ObjectId, enabled: bool) {
        with(|s| s.set_enabled(object, enabled));
    }

    /// Enabled, and so are all its parents.
    pub fn active(object: ObjectId) -> bool {
        with(|s| s.active(object)).unwrap_or(false)
    }

    pub fn layer(object: ObjectId) -> u32 {
        with(|s| s.layer(object) as u32).unwrap_or(0)
    }

    pub fn set_layer(object: ObjectId, layer: u32) {
        with(|s| s.set_layer(object, layer.min(31) as u8));
    }

    /// Position relative to the parent.
    pub fn local_position(object: ObjectId) -> Vec3 {
        with(|s| s.local(object).map(|t| t.position))
            .flatten()
            .unwrap_or_default()
    }

    pub fn set_local_position(object: ObjectId, position: Vec3) {
        with(|s| {
            if let Some(t) = s.local(object) {
                s.set_local(object, Transform { position, ..t });
            }
        });
    }

    pub fn local_rotation(object: ObjectId) -> Quat {
        with(|s| s.local(object).map(|t| t.rotation))
            .flatten()
            .unwrap_or(Quat::IDENTITY)
    }

    pub fn set_local_rotation(object: ObjectId, rotation: Quat) {
        with(|s| {
            if let Some(t) = s.local(object) {
                s.set_local(
                    object,
                    Transform {
                        rotation: rotation.normalize(),
                        ..t
                    },
                );
            }
        });
    }

    pub fn local_scale(object: ObjectId) -> Vec3 {
        with(|s| s.local(object).map(|t| t.scale))
            .flatten()
            .unwrap_or(Vec3::ONE)
    }

    pub fn set_local_scale(object: ObjectId, scale: Vec3) {
        with(|s| {
            if let Some(t) = s.local(object) {
                s.set_local(object, Transform { scale, ..t });
            }
        });
    }

    /// Position in the world.
    pub fn position(object: ObjectId) -> Vec3 {
        with(|s| s.contains(object).then(|| s.world(object).position))
            .flatten()
            .unwrap_or_default()
    }

    /// Puts the object at a world position, keeping its world rotation.
    pub fn set_position(object: ObjectId, position: Vec3) {
        with(|s| {
            if s.contains(object) {
                let rotation = s.world(object).rotation;
                s.set_world(object, position, rotation);
            }
        });
    }

    /// Rotation in the world.
    pub fn rotation(object: ObjectId) -> Quat {
        with(|s| s.contains(object).then(|| s.world(object).rotation))
            .flatten()
            .unwrap_or(Quat::IDENTITY)
    }

    /// Rotates the object in the world, keeping its world position.
    pub fn set_rotation(object: ObjectId, rotation: Quat) {
        with(|s| {
            if s.contains(object) {
                let position = s.world(object).position;
                s.set_world(object, position, rotation);
            }
        });
    }
}
