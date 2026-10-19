//! The scene's script API, declared once with `#[iron_api]` (see `docs/iron-api.md`): the
//! `Scene` namespace of `GameObject` handles and `Vector3` / `Quaternion` values. It sits next to
//! the op-based `ScriptScene`, which keeps working; this is what compiled scripts call directly.
//!
//! The functions work on the scene the host made current with [`enter`]. With none (or for a
//! stale id) they answer with a default: no object, the origin, the identity rotation.

use crate::{ObjectId, Scene, Transform};
use glam::{Quat, Vec3};
use iron_api::{Handle, context, iron_api};
use iron_math::{Q, V3};
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

/// Runs `f` on the scene the host made current with [`enter`] (`None` when there is none): for
/// engine systems (animation) that work on the scene next to the script API.
pub fn with_scene<R>(f: impl FnOnce(&mut Scene) -> R) -> Option<R> {
    with(f)
}

fn world_position(object: ObjectId) -> Vec3 {
    with(|s| s.contains(object).then(|| s.world(object).position))
        .flatten()
        .unwrap_or_default()
}

/// The script's doubles as the scene's floats, and back.
pub fn vec3(v: V3) -> Vec3 {
    Vec3::new(v.x as f32, v.y as f32, v.z as f32)
}

pub fn v3_of(v: Vec3) -> V3 {
    V3 {
        x: f64::from(v.x),
        y: f64::from(v.y),
        z: f64::from(v.z),
    }
}

pub fn quat(q: Q) -> Quat {
    Quat::from_xyzw(q.x as f32, q.y as f32, q.z as f32, q.w as f32)
}

pub fn q_of(q: Quat) -> Q {
    Q {
        x: f64::from(q.x),
        y: f64::from(q.y),
        z: f64::from(q.z),
        w: f64::from(q.w),
    }
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
        Vector3 = V3 [x: f64, y: f64, z: f64],
        Quaternion = Q [x: f64, y: f64, z: f64, w: f64]
    )
)]
pub mod scene_api {
    use super::{ObjectId, Q, Transform, TransformRef, V3, none, q_of, quat, v3_of, vec3, with};
    use glam::{Quat, Vec3};

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
        pub fn position(self) -> V3 {
            v3_of(super::world_position(self.0))
        }

        /// Puts the object at a world position, keeping its world rotation.
        pub fn set_position(self, position: V3) {
            set_position(self.0, position);
        }

        /// Rotation in the world.
        #[property]
        pub fn rotation(self) -> Q {
            rotation(self.0)
        }

        /// Rotates the object in the world, keeping its world position.
        pub fn set_rotation(self, rotation: Q) {
            set_rotation(self.0, rotation);
        }

        /// Position relative to the parent.
        #[property]
        pub fn local_position(self) -> V3 {
            local_position(self.0)
        }

        pub fn set_local_position(self, position: V3) {
            set_local_position(self.0, position);
        }

        #[property]
        pub fn local_rotation(self) -> Q {
            local_rotation(self.0)
        }

        pub fn set_local_rotation(self, rotation: Q) {
            set_local_rotation(self.0, rotation);
        }

        #[property]
        pub fn local_scale(self) -> V3 {
            local_scale(self.0)
        }

        pub fn set_local_scale(self, scale: V3) {
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
    pub fn local_position(object: ObjectId) -> V3 {
        v3_of(
            with(|s| s.local(object).map(|t| t.position))
                .flatten()
                .unwrap_or_default(),
        )
    }

    pub fn set_local_position(object: ObjectId, position: V3) {
        let position = vec3(position);
        with(|s| {
            if let Some(t) = s.local(object) {
                s.set_local(object, Transform { position, ..t });
            }
        });
    }

    pub fn local_rotation(object: ObjectId) -> Q {
        q_of(
            with(|s| s.local(object).map(|t| t.rotation))
                .flatten()
                .unwrap_or(Quat::IDENTITY),
        )
    }

    pub fn set_local_rotation(object: ObjectId, rotation: Q) {
        let rotation = quat(rotation);
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

    pub fn local_scale(object: ObjectId) -> V3 {
        v3_of(
            with(|s| s.local(object).map(|t| t.scale))
                .flatten()
                .unwrap_or(Vec3::ONE),
        )
    }

    pub fn set_local_scale(object: ObjectId, scale: V3) {
        let scale = vec3(scale);
        with(|s| {
            if let Some(t) = s.local(object) {
                s.set_local(object, Transform { scale, ..t });
            }
        });
    }

    /// Position in the world.
    pub fn position(object: ObjectId) -> V3 {
        v3_of(
            with(|s| s.contains(object).then(|| s.world(object).position))
                .flatten()
                .unwrap_or_default(),
        )
    }

    /// Puts the object at a world position, keeping its world rotation.
    pub fn set_position(object: ObjectId, position: V3) {
        let position = vec3(position);
        with(|s| {
            if s.contains(object) {
                let rotation = s.world(object).rotation;
                s.set_world(object, position, rotation);
            }
        });
    }

    /// Rotation in the world.
    pub fn rotation(object: ObjectId) -> Q {
        q_of(
            with(|s| s.contains(object).then(|| s.world(object).rotation))
                .flatten()
                .unwrap_or(Quat::IDENTITY),
        )
    }

    /// Rotates the object in the world, keeping its world position.
    pub fn set_rotation(object: ObjectId, rotation: Q) {
        let rotation = quat(rotation);
        with(|s| {
            if s.contains(object) {
                let position = s.world(object).position;
                s.set_world(object, position, rotation);
            }
        });
    }
}
