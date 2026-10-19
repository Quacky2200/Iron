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

/// The `Transform` of a `GameObject`: the same object seen as its transform. Scripts get one with
/// `go.transform`, and it shares the object's id.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TransformRef(pub ObjectId);

impl Handle for TransformRef {
    fn to_raw(self) -> u64 {
        self.0.to_raw()
    }
    fn from_raw(raw: u64) -> Self {
        TransformRef(ObjectId::from_raw(raw))
    }
}

fn with<R>(f: impl FnOnce(&mut Scene) -> R) -> Option<R> {
    context::with(&SCENE, f)
}

fn world_position(object: ObjectId) -> Vec3 {
    with(|s| s.contains(object).then(|| s.world(object).position))
        .flatten()
        .unwrap_or_default()
}

fn none() -> ObjectId {
    // `from_raw(0)` wraps to a generation no slot has, so it misses everywhere.
    ObjectId::from_raw(0)
}

#[iron_api(
    namespace = "Scene",
    quickjs,
    handles(GameObject = ObjectId, Transform = TransformRef),
    values(
        Vector3 = Vec3 [x: f32, y: f32, z: f32],
        Quaternion = Quat [x: f32, y: f32, z: f32, w: f32]
    )
)]
pub mod scene_api {
    use super::{ObjectId, Quat, Transform, TransformRef, Vec3, none, with};

    // Math on the value classes, so scripts write `a.add(b)` and `v.magnitude` (compiled as direct
    // calls; a second crate that shares `Vector3` will move these to a math namespace).

    /// Component-wise sum.
    #[method(Vector3)]
    pub fn add(a: Vec3, b: Vec3) -> Vec3 {
        a + b
    }

    /// Component-wise difference.
    #[method(Vector3)]
    pub fn sub(a: Vec3, b: Vec3) -> Vec3 {
        a - b
    }

    /// Multiplies every component by a number.
    #[method(Vector3)]
    pub fn scale(a: Vec3, s: f32) -> Vec3 {
        a * s
    }

    #[method(Vector3)]
    pub fn dot(a: Vec3, b: Vec3) -> f32 {
        a.dot(b)
    }

    #[method(Vector3)]
    pub fn cross(a: Vec3, b: Vec3) -> Vec3 {
        a.cross(b)
    }

    #[method(Vector3)]
    pub fn distance(a: Vec3, b: Vec3) -> f32 {
        a.distance(b)
    }

    /// Linear interpolation: `a` at 0, `b` at 1.
    #[method(Vector3)]
    pub fn lerp(a: Vec3, b: Vec3, t: f32) -> Vec3 {
        a.lerp(b, t)
    }

    /// Its length.
    #[getter(Vector3)]
    pub fn magnitude(a: Vec3) -> f32 {
        a.length()
    }

    /// The same direction with length 1 (the zero vector stays zero).
    #[getter(Vector3)]
    pub fn normalized(a: Vec3) -> Vec3 {
        a.normalize_or_zero()
    }

    /// Combines two rotations: `b` is applied first, then `a` (`a * b`, like Unity).
    #[method(Quaternion)]
    pub fn mul(a: Quat, b: Quat) -> Quat {
        a * b
    }

    /// Rotates a vector.
    #[method(Quaternion)]
    pub fn rotate(q: Quat, v: Vec3) -> Vec3 {
        q * v
    }

    /// The opposite rotation.
    #[getter(Quaternion)]
    pub fn inverse(q: Quat) -> Quat {
        q.inverse()
    }

    // `GameObject`: creating, destroying, flags and the transform.
    impl ObjectId {
        /// Creates an enabled root object at the origin.
        pub fn create() -> ObjectId {
            with(|s| s.create("GameObject")).unwrap_or_else(none)
        }

        /// Destroys the object and everything under it.
        pub fn destroy(self) {
            with(|s| s.destroy(self));
        }

        /// Does the object exist (it may have been destroyed)?
        pub fn exists(self) -> bool {
            with(|s| s.contains(self)).unwrap_or(false)
        }

        /// The first object with this name, or no object.
        pub fn find(name: &str) -> ObjectId {
            with(|s| s.find(name)).flatten().unwrap_or_else(none)
        }

        /// Its name.
        #[property]
        pub fn name(self) -> String {
            with(|s| s.name(self).map(str::to_owned))
                .flatten()
                .unwrap_or_default()
        }

        pub fn set_name(self, name: &str) {
            with(|s| s.set_name(self, name));
        }

        /// Its transform.
        #[property]
        pub fn transform(self) -> TransformRef {
            TransformRef(self)
        }

        /// Is it switched on? (Its parents may still be off: see `activeInHierarchy`.)
        #[property]
        pub fn enabled(self) -> bool {
            with(|s| s.enabled(self)).unwrap_or(false)
        }

        pub fn set_enabled(self, enabled: bool) {
            with(|s| s.set_enabled(self, enabled));
        }

        /// Enabled, and so are all its parents.
        #[property]
        pub fn active_in_hierarchy(self) -> bool {
            with(|s| s.active(self)).unwrap_or(false)
        }

        /// Render group 0 to 31.
        #[property]
        pub fn layer(self) -> u32 {
            with(|s| s.layer(self) as u32).unwrap_or(0)
        }

        pub fn set_layer(self, layer: u32) {
            with(|s| s.set_layer(self, layer.min(31) as u8));
        }
    }

    // `Transform`: position, rotation and scale, and the hierarchy.
    impl TransformRef {
        /// The object this is the transform of.
        #[property]
        pub fn game_object(self) -> ObjectId {
            self.0
        }

        /// Position in the world.
        #[property]
        pub fn position(self) -> Vec3 {
            super::world_position(self.0)
        }

        /// Puts the object at a world position, keeping its world rotation.
        pub fn set_position(self, position: Vec3) {
            set_position(self.0, position);
        }

        /// Rotation in the world.
        #[property]
        pub fn rotation(self) -> Quat {
            rotation(self.0)
        }

        /// Rotates the object in the world, keeping its world position.
        pub fn set_rotation(self, rotation: Quat) {
            set_rotation(self.0, rotation);
        }

        /// Position relative to the parent.
        #[property]
        pub fn local_position(self) -> Vec3 {
            local_position(self.0)
        }

        pub fn set_local_position(self, position: Vec3) {
            set_local_position(self.0, position);
        }

        #[property]
        pub fn local_rotation(self) -> Quat {
            local_rotation(self.0)
        }

        pub fn set_local_rotation(self, rotation: Quat) {
            set_local_rotation(self.0, rotation);
        }

        #[property]
        pub fn local_scale(self) -> Vec3 {
            local_scale(self.0)
        }

        pub fn set_local_scale(self, scale: Vec3) {
            set_local_scale(self.0, scale);
        }

        /// The parent transform, or no object for a root.
        #[property]
        pub fn parent(self) -> TransformRef {
            TransformRef(parent(self.0))
        }

        /// Moves the object under a parent (none: to the root), optionally keeping its world pose.
        pub fn set_parent(self, parent: TransformRef, keep_world: bool) {
            set_parent(self.0, parent.0, keep_world);
        }
    }

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
