//! Physics for compiled scripts: rigid bodies, colliders and ray queries over `iron_physics`,
//! declared with `#[iron_api]` (design: `docs/research/iron-native-physics.md`).
//!
//! Bodies and colliders are numbers chosen here; the world is one per thread. The host calls
//! [`step`] once per fixed tick: it advances the world, moves the GameObjects bodies are attached
//! to, and queues the contacts for [`take_contacts`] (the host turns them into
//! `onCollisionEnter` and friends). A ray answers at once.

use iron_api::{Handle, iron_api};
use iron_core::Value;
use iron_math::{Q, V3};
use iron_physics::{Contact, Physics};
use iron_scene::ObjectId;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

macro_rules! number_handle {
    ($($name:ident),*) => {$(
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub struct $name(pub u32);
        impl Handle for $name {
            fn to_raw(self) -> u64 {
                u64::from(self.0)
            }
            fn from_raw(raw: u64) -> Self {
                $name(raw as u32)
            }
        }
    )*};
}

// 0 is "none" in each (a script's `Body | null`).
number_handle!(BodyRef, ColliderRef, HitRef);

struct HitData {
    collider: u32,
    distance: f64,
    point: V3,
    normal: V3,
}

struct State {
    physics: Physics,
    next_body: u32,
    next_collider: u32,
    /// The GameObject each body moves.
    attached: HashMap<u32, ObjectId>,
    /// Bodies whose character controller ended its last move on the ground.
    grounded: HashSet<u32>,
    /// The colliders of each body (the body's removal drops them).
    owner: HashMap<u32, u32>,
    kinds: HashMap<u32, String>,
    /// Ray hits handed out since the last step.
    hits: Vec<HitData>,
    contacts: Vec<Contact>,
}

impl State {
    fn new() -> Self {
        State {
            physics: Physics::new(),
            next_body: 1,
            next_collider: 1,
            attached: HashMap::new(),
            grounded: HashSet::new(),
            owner: HashMap::new(),
            kinds: HashMap::new(),
            hits: Vec::new(),
            contacts: Vec::new(),
        }
    }
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::new());
}

fn st<R>(f: impl FnOnce(&mut State) -> R) -> R {
    STATE.with(|s| f(&mut s.borrow_mut()))
}

/// A new empty world (a new game, tests).
pub fn reset() {
    STATE.with(|s| *s.borrow_mut() = State::new());
}

/// Runs a `physics.*` op; a bad one is reported and ignored (a script cannot do anything with it).
fn op(name: &str, args: Vec<Value>) {
    if let Err(e) = st(|s| s.physics.apply(name, &args)) {
        eprintln!("physics.{name}: {e}");
    }
}

fn num(x: f64) -> Value {
    Value::Number(x)
}

fn v3(v: V3) -> Value {
    Value::Array(vec![num(v.x), num(v.y), num(v.z)])
}

fn quat(q: Q) -> Value {
    Value::Array(vec![num(q.x), num(q.y), num(q.z), num(q.w)])
}

fn from3(a: [f32; 3]) -> V3 {
    V3 {
        x: f64::from(a[0]),
        y: f64::from(a[1]),
        z: f64::from(a[2]),
    }
}

/// Advances the world one fixed tick, moves the objects bodies are attached to, and keeps the
/// contacts for [`take_contacts`].
pub fn step() {
    let (moves, grounded_now, contacts) = st(|s| {
        s.hits.clear();
        s.physics.step();
        let batch = s.physics.take_batch();
        let moves: Vec<(ObjectId, [f32; 3], [f32; 4])> = batch
            .bodies
            .iter()
            .filter_map(|b| Some((*s.attached.get(&b.id)?, b.position, b.rotation)))
            .collect();
        (moves, batch.characters, batch.contacts)
    });
    iron_scene::api::with_scene(|scene| {
        for (o, p, r) in moves {
            scene.set_world(
                o,
                glam::Vec3::from_array(p),
                glam::Quat::from_xyzw(r[0], r[1], r[2], r[3]),
            );
        }
    });
    st(|s| {
        for (id, g) in grounded_now {
            if g {
                s.grounded.insert(id);
            } else {
                s.grounded.remove(&id);
            }
        }
        s.contacts.extend(contacts);
    });
}

/// The contacts since the last call, by collider number.
pub fn take_contacts() -> Vec<Contact> {
    st(|s| std::mem::take(&mut s.contacts))
}

/// The GameObject a collider's body moves, if it is attached to one.
pub fn object_of_collider(collider: u32) -> Option<ObjectId> {
    st(|s| {
        let body = s.physics.collider_body(collider)?;
        s.attached.get(&body).copied()
    })
}

fn add_collider(body: u32, shape: &str, dims: Vec<f64>, indices: Option<Vec<f64>>) -> ColliderRef {
    let id = st(|s| {
        let id = s.next_collider;
        s.next_collider += 1;
        id
    });
    let opts = vec![
        num(0.5),
        num(0.0),
        num(1.0),
        Value::Bool(false),
        Value::Bool(true),
        indices
            .map(|i| Value::Array(i.into_iter().map(num).collect()))
            .unwrap_or(Value::Null),
    ];
    op(
        "collider",
        vec![
            num(f64::from(id)),
            if body == 0 {
                Value::Null
            } else {
                num(f64::from(body))
            },
            Value::String(shape.to_owned()),
            Value::Array(dims.into_iter().map(num).collect()),
            Value::Array(vec![num(0.0); 3]),
            Value::Array(vec![num(0.0), num(0.0), num(0.0), num(1.0)]),
            Value::Array(opts),
        ],
    );
    st(|s| s.owner.insert(id, body));
    ColliderRef(id)
}

fn flat(points: &[V3]) -> Vec<f64> {
    points.iter().flat_map(|p| [p.x, p.y, p.z]).collect()
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[iron_api(
    namespace = "Physics",
    handles(Body = BodyRef, Collider = ColliderRef, RaycastHit = HitRef, GameObject = ObjectId),
    values(Vector3 = V3 [x: f64, y: f64, z: f64], Quaternion = Q [x: f64, y: f64, z: f64, w: f64])
)]
pub mod physics_api {
    use super::*;

    /// Gravity in m/s² (y up; default 0, -9.81, 0).
    #[property]
    pub fn gravity() -> V3 {
        from3(st(|s| s.physics.gravity()))
    }

    pub fn set_gravity(g: V3) {
        op("gravity", vec![v3(g)]);
    }

    /// A body: `"dynamic"` (simulated), `"fixed"` (never moves) or `"kinematic"` (moved by script).
    pub fn create_body(kind: &str) -> BodyRef {
        let id = st(|s| {
            let id = s.next_body;
            s.next_body += 1;
            s.kinds.insert(id, kind.to_owned());
            id
        });
        op(
            "body",
            vec![
                num(f64::from(id)),
                Value::String(kind.to_owned()),
                Value::Array(vec![num(0.0); 3]),
                Value::Array(vec![num(0.0), num(0.0), num(0.0), num(1.0)]),
                Value::Array(vec![num(0.0); 3]),
                Value::Array(vec![
                    num(1.0),
                    num(0.0),
                    num(0.0),
                    Value::Bool(false),
                    Value::Bool(false),
                ]),
            ],
        );
        BodyRef(id)
    }

    /// A box collider on the world itself (floors, walls): half extents.
    pub fn add_box(half: V3) -> ColliderRef {
        add_collider(0, "box", vec![half.x, half.y, half.z], None)
    }

    pub fn add_ball(radius: f64) -> ColliderRef {
        add_collider(0, "ball", vec![radius], None)
    }

    /// Static level geometry from triangles: points and the indices of each triangle.
    pub fn add_mesh(points: &[V3], indices: &[f64]) -> ColliderRef {
        add_collider(0, "mesh", flat(points), Some(indices.to_vec()))
    }

    /// Casts a ray at once and answers the first collider hit within `maxDistance`, or no hit.
    #[optional(max_distance = 1000)]
    pub fn raycast(origin: V3, direction: V3, max_distance: f64) -> HitRef {
        let hit = st(|s| {
            s.physics.raycast_now(
                [origin.x as f32, origin.y as f32, origin.z as f32],
                [direction.x as f32, direction.y as f32, direction.z as f32],
                max_distance as f32,
            )
        });
        let Some((collider, distance, normal)) = hit else {
            return HitRef(0);
        };
        let len =
            (direction.x * direction.x + direction.y * direction.y + direction.z * direction.z)
                .sqrt();
        let d = f64::from(distance);
        let point = V3 {
            x: origin.x + direction.x / len * d,
            y: origin.y + direction.y / len * d,
            z: origin.z + direction.z / len * d,
        };
        st(|s| {
            s.hits.push(HitData {
                collider,
                distance: d,
                point,
                normal: from3(normal),
            });
            HitRef(s.hits.len() as u32)
        })
    }

    /// Advances the world one fixed tick (the host does this once per tick).
    pub fn step() {
        super::step();
    }

    impl BodyRef {
        /// Where it was after the last tick (or where it was last set).
        #[property]
        pub fn position(self) -> V3 {
            from3(st(|s| s.physics.position(self.0)).unwrap_or_default())
        }

        pub fn set_position(self, p: V3) {
            op(
                "set",
                vec![
                    num(f64::from(self.0)),
                    Value::String("position".into()),
                    v3(p),
                ],
            );
        }

        #[property]
        pub fn rotation(self) -> Q {
            let r = st(|s| s.physics.pose(self.0)).map_or([0.0, 0.0, 0.0, 1.0], |p| p.1);
            Q {
                x: f64::from(r[0]),
                y: f64::from(r[1]),
                z: f64::from(r[2]),
                w: f64::from(r[3]),
            }
        }

        pub fn set_rotation(self, q: Q) {
            op(
                "set",
                vec![
                    num(f64::from(self.0)),
                    Value::String("rotation".into()),
                    quat(q),
                ],
            );
        }

        #[property]
        pub fn velocity(self) -> V3 {
            from3(st(|s| s.physics.velocity(self.0)).unwrap_or_default())
        }

        pub fn set_velocity(self, v: V3) {
            op(
                "set",
                vec![
                    num(f64::from(self.0)),
                    Value::String("velocity".into()),
                    v3(v),
                ],
            );
        }

        /// Radians per second about each world axis.
        #[property]
        pub fn angular_velocity(self) -> V3 {
            from3(st(|s| s.physics.angular_velocity(self.0)).unwrap_or_default())
        }

        pub fn set_angular_velocity(self, v: V3) {
            op(
                "set",
                vec![
                    num(f64::from(self.0)),
                    Value::String("angularVelocity".into()),
                    v3(v),
                ],
            );
        }

        /// A character controller's ground contact after its last `move`.
        #[property]
        pub fn grounded(self) -> bool {
            st(|s| s.grounded.contains(&self.0))
        }

        /// A kinematic body's position at the end of the next tick (it pushes things on the way).
        pub fn move_to(self, p: V3) {
            op(
                "set",
                vec![
                    num(f64::from(self.0)),
                    Value::String("target".into()),
                    v3(p),
                ],
            );
        }

        /// An instant change in momentum (N·s): jumps, hits, explosions.
        pub fn apply_impulse(self, impulse: V3) {
            op(
                "force",
                vec![
                    num(f64::from(self.0)),
                    Value::String("impulse".into()),
                    v3(impulse),
                ],
            );
        }

        pub fn apply_torque_impulse(self, impulse: V3) {
            op(
                "force",
                vec![
                    num(f64::from(self.0)),
                    Value::String("torqueImpulse".into()),
                    v3(impulse),
                ],
            );
        }

        /// A force (N) applied every tick until `clearForces()`.
        pub fn add_force(self, force: V3) {
            op(
                "force",
                vec![
                    num(f64::from(self.0)),
                    Value::String("force".into()),
                    v3(force),
                ],
            );
        }

        pub fn add_torque(self, torque: V3) {
            op(
                "force",
                vec![
                    num(f64::from(self.0)),
                    Value::String("torque".into()),
                    v3(torque),
                ],
            );
        }

        pub fn clear_forces(self) {
            op("clearForces", vec![num(f64::from(self.0))]);
        }

        /// An instant impulse at a world point (it also spins the body).
        pub fn apply_impulse_at_point(self, impulse: V3, point: V3) {
            op(
                "force",
                vec![
                    num(f64::from(self.0)),
                    Value::String("impulseAt".into()),
                    v3(impulse),
                    v3(point),
                ],
            );
        }

        /// Total mass in kg (shared by its colliders).
        pub fn set_mass(self, kg: f64) {
            op(
                "set",
                vec![
                    num(f64::from(self.0)),
                    Value::String("mass".into()),
                    num(kg),
                ],
            );
        }

        pub fn set_gravity_scale(self, scale: f64) {
            op(
                "set",
                vec![
                    num(f64::from(self.0)),
                    Value::String("gravityScale".into()),
                    num(scale),
                ],
            );
        }

        pub fn set_damping(self, linear: f64, angular: f64) {
            op(
                "set",
                vec![
                    num(f64::from(self.0)),
                    Value::String("linearDamping".into()),
                    num(linear),
                ],
            );
            op(
                "set",
                vec![
                    num(f64::from(self.0)),
                    Value::String("angularDamping".into()),
                    num(angular),
                ],
            );
        }

        pub fn set_ccd(self, on: bool) {
            op(
                "set",
                vec![
                    num(f64::from(self.0)),
                    Value::String("ccd".into()),
                    Value::Bool(on),
                ],
            );
        }

        /// Keeps the body upright (no rotation about any axis).
        pub fn lock_rotation(self, locked: bool) {
            op(
                "set",
                vec![
                    num(f64::from(self.0)),
                    Value::String("constraints".into()),
                    Value::Array(vec![
                        Value::Bool(true),
                        Value::Bool(true),
                        Value::Bool(true),
                        Value::Bool(!locked),
                        Value::Bool(!locked),
                        Value::Bool(!locked),
                    ]),
                ],
            );
        }

        /// Switches between simulated (`"dynamic"`), immovable (`"fixed"`) and script-moved
        /// (`"kinematic"`).
        pub fn set_kind(self, kind: &str) {
            st(|s| s.kinds.insert(self.0, kind.to_owned()));
            op(
                "set",
                vec![
                    num(f64::from(self.0)),
                    Value::String("kind".into()),
                    Value::String(kind.to_owned()),
                ],
            );
        }

        #[property]
        pub fn kind(self) -> String {
            st(|s| s.kinds.get(&self.0).cloned().unwrap_or_default())
        }

        /// Disabled bodies are left out of the simulation (and keep their state).
        pub fn set_enabled(self, enabled: bool) {
            op(
                "set",
                vec![
                    num(f64::from(self.0)),
                    Value::String("enabled".into()),
                    Value::Bool(enabled),
                ],
            );
        }

        pub fn sleep(self) {
            op(
                "set",
                vec![num(f64::from(self.0)), Value::String("sleep".into())],
            );
        }

        pub fn wake_up(self) {
            op(
                "set",
                vec![num(f64::from(self.0)), Value::String("wake".into())],
            );
        }

        /// Makes the body move a GameObject each tick.
        pub fn attach_to(self, object: ObjectId) {
            st(|s| s.attached.insert(self.0, object));
        }

        /// A box collider (half extents).
        pub fn add_box(self, half: V3) -> ColliderRef {
            add_collider(self.0, "box", vec![half.x, half.y, half.z], None)
        }

        pub fn add_ball(self, radius: f64) -> ColliderRef {
            add_collider(self.0, "ball", vec![radius], None)
        }

        /// Along y.
        pub fn add_capsule(self, half_height: f64, radius: f64) -> ColliderRef {
            add_collider(self.0, "capsule", vec![half_height, radius], None)
        }

        pub fn add_cylinder(self, half_height: f64, radius: f64) -> ColliderRef {
            add_collider(self.0, "cylinder", vec![half_height, radius], None)
        }

        /// The convex hull of these points.
        pub fn add_convex(self, points: &[V3]) -> ColliderRef {
            add_collider(self.0, "convex", flat(points), None)
        }

        /// Makes the body a character: `move` then slides it along walls, climbs slopes and
        /// steps. Use a kinematic body with a capsule collider.
        #[optional(max_slope = 45, snap_to_ground, step_height)]
        pub fn make_character(self, max_slope: f64, snap_to_ground: f64, step_height: f64) {
            let opt = |x: f64| if x.is_nan() { Value::Null } else { num(x) };
            op(
                "character",
                vec![
                    num(f64::from(self.0)),
                    num(max_slope),
                    opt(snap_to_ground),
                    opt(step_height),
                ],
            );
        }

        /// Moves a character by `delta` metres this tick (moves in the same tick add up).
        pub fn move_by(self, delta: V3) {
            op("move", vec![num(f64::from(self.0)), v3(delta)]);
        }

        /// Removes the body and its colliders.
        pub fn remove(self) {
            st(|s| {
                s.attached.remove(&self.0);
                s.kinds.remove(&self.0);
                s.owner.retain(|_, b| *b != self.0);
            });
            op("removeBody", vec![num(f64::from(self.0))]);
        }
    }

    impl ColliderRef {
        pub fn set_friction(self, friction: f64) {
            op(
                "colliderSet",
                vec![
                    num(f64::from(self.0)),
                    Value::String("friction".into()),
                    num(friction),
                ],
            );
        }

        /// Bounciness, 0 to 1.
        pub fn set_restitution(self, restitution: f64) {
            op(
                "colliderSet",
                vec![
                    num(f64::from(self.0)),
                    Value::String("restitution".into()),
                    num(restitution),
                ],
            );
        }

        /// A trigger: reports overlaps but does not push anything.
        pub fn set_sensor(self, sensor: bool) {
            op(
                "colliderSet",
                vec![
                    num(f64::from(self.0)),
                    Value::String("sensor".into()),
                    Value::Bool(sensor),
                ],
            );
        }

        /// The body it is on (no body for a world collider).
        #[property]
        pub fn body(self) -> BodyRef {
            BodyRef(st(|s| s.owner.get(&self.0).copied()).unwrap_or(0))
        }

        pub fn remove(self) {
            st(|s| s.owner.remove(&self.0));
            op("removeCollider", vec![num(f64::from(self.0))]);
        }
    }

    impl HitRef {
        /// The collider that was hit.
        #[property]
        pub fn collider(self) -> ColliderRef {
            ColliderRef(
                st(|s| {
                    s.hits
                        .get((self.0 as usize).wrapping_sub(1))
                        .map(|h| h.collider)
                })
                .unwrap_or(0),
            )
        }

        /// Metres along the ray.
        #[property]
        pub fn distance(self) -> f64 {
            st(|s| {
                s.hits
                    .get((self.0 as usize).wrapping_sub(1))
                    .map(|h| h.distance)
            })
            .unwrap_or(0.0)
        }

        #[property]
        pub fn point(self) -> V3 {
            st(|s| {
                s.hits
                    .get((self.0 as usize).wrapping_sub(1))
                    .map(|h| h.point)
            })
            .unwrap_or_default()
        }

        #[property]
        pub fn normal(self) -> V3 {
            st(|s| {
                s.hits
                    .get((self.0 as usize).wrapping_sub(1))
                    .map(|h| h.normal)
            })
            .unwrap_or_default()
        }
    }
}

/// The manifests of this crate's script API.
pub fn manifests() -> Vec<String> {
    vec![physics_api::MANIFEST.to_json()]
}
