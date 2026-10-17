//! Physics for scripts: a rapier world driven by ops (DESIGN.md §8.1).
//!
//! Scripts never hold rapier handles. They pick numeric IDs for bodies and colliders
//! (like UI nodes) and queue `physics.*` ops; [`Physics::apply`] runs them, [`Physics::step`]
//! advances the world one fixed tick, and [`Physics::take_batch`] returns what scripts
//! need back (moved bodies, collision events, query results) as one batch per tick.

pub mod cloth;

use std::collections::HashMap;
use std::sync::Mutex;

use iron_core::Value;
use rapier3d::control::{CharacterAutostep, CharacterLength, KinematicCharacterController};
use rapier3d::prelude::*;

/// What a body or collider is called by scripts.
pub type ScriptId = u32;

/// A collision between two colliders, by script ID.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Contact {
    pub started: bool,
    pub a: ScriptId,
    pub b: ScriptId,
    /// How fast they were moving towards or past each other (m/s), going into the
    /// contact: for impact sounds and damage. Zero when it stops.
    pub speed: f32,
    /// One of them is a sensor (a trigger: reports overlaps, doesn't push).
    pub sensor: bool,
}

/// A body's pose and velocity after a step.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BodyState {
    pub id: ScriptId,
    pub position: [f32; 3],
    pub rotation: [f32; 4],
    pub velocity: [f32; 3],
    /// Radians per second about each world axis.
    pub angular_velocity: [f32; 3],
}

/// The answer to a `physics.raycast` query.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RayHit {
    pub query: u32,
    /// The collider hit, its distance along the ray and the surface normal; `None`
    /// when nothing was hit.
    pub hit: Option<(ScriptId, f32, [f32; 3])>,
}

/// Everything scripts get back from one tick.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Batch {
    pub bodies: Vec<BodyState>,
    pub contacts: Vec<Contact>,
    pub rays: Vec<RayHit>,
    /// Character controllers moved this tick, and whether they ended on the ground.
    pub characters: Vec<(ScriptId, bool)>,
}

impl Batch {
    pub fn is_empty(&self) -> bool {
        self.bodies.is_empty()
            && self.contacts.is_empty()
            && self.rays.is_empty()
            && self.characters.is_empty()
    }

    /// The batch as `__iron_physics(bodies, contacts, rays, characters)` arguments: flat
    /// number arrays, so a tick crosses the boundary as four arrays rather than objects.
    /// - bodies: `id, px, py, pz, qx, qy, qz, qw, vx, vy, vz, wx, wy, wz` each
    /// - contacts: `started (1/0), a, b, sensor (1/0), speed` each
    /// - rays: `query, collider (-1: miss), distance, nx, ny, nz` each
    /// - characters: `id, grounded (1/0)` each
    pub fn to_args(&self) -> Vec<Value> {
        let num = |v: f32| Value::Number(v as f64);
        let flag = |b: bool| Value::Number(if b { 1.0 } else { 0.0 });
        let mut bodies = Vec::with_capacity(self.bodies.len() * 14);
        for b in &self.bodies {
            bodies.push(b.id.into());
            bodies.extend(b.position.map(num));
            bodies.extend(b.rotation.map(num));
            bodies.extend(b.velocity.map(num));
            bodies.extend(b.angular_velocity.map(num));
        }
        let mut contacts = Vec::with_capacity(self.contacts.len() * 5);
        for c in &self.contacts {
            contacts.extend([
                flag(c.started),
                c.a.into(),
                c.b.into(),
                flag(c.sensor),
                num(c.speed),
            ]);
        }
        let mut rays = Vec::with_capacity(self.rays.len() * 6);
        for r in &self.rays {
            rays.push(r.query.into());
            match r.hit {
                Some((id, dist, n)) => {
                    rays.extend([id.into(), num(dist)]);
                    rays.extend(n.map(num));
                }
                None => rays.extend([-1.0, 0.0, 0.0, 0.0, 0.0].map(Value::Number)),
            }
        }
        let characters = self
            .characters
            .iter()
            .flat_map(|&(id, grounded)| [id.into(), flag(grounded)])
            .collect();
        vec![
            Value::Array(bodies),
            Value::Array(contacts),
            Value::Array(rays),
            Value::Array(characters),
        ]
    }
}

/// Collects rapier's collision events during a step, with the colliders' relative
/// speed (events fire before the solver, so it's the speed going into the contact).
#[derive(Default)]
struct Events(Mutex<Vec<(CollisionEvent, f32)>>);

impl EventHandler for Events {
    fn handle_collision_event(
        &self,
        bodies: &RigidBodySet,
        colliders: &ColliderSet,
        event: CollisionEvent,
        _pair: Option<&ContactPair>,
    ) {
        let velocity = |h: ColliderHandle| {
            colliders
                .get(h)
                .and_then(|c| bodies.get(c.parent()?))
                .map_or(Vector::ZERO, |b| b.linvel())
        };
        let speed = if event.started() {
            (velocity(event.collider1()) - velocity(event.collider2())).length()
        } else {
            0.0
        };
        if let Ok(mut events) = self.0.lock() {
            events.push((event, speed));
        }
    }

    fn handle_contact_force_event(
        &self,
        _dt: Real,
        _bodies: &RigidBodySet,
        _colliders: &ColliderSet,
        _pair: &ContactPair,
        _total_force_magnitude: Real,
    ) {
    }

    fn handle_soft_body_tear_event(&self, _soft_bodies: &SoftBodySet, _event: &SoftBodyTearEvent) {}
}

/// The physics world scripts drive.
pub struct Physics {
    world: PhysicsWorld,
    bodies: HashMap<ScriptId, RigidBodyHandle>,
    colliders: HashMap<ScriptId, ColliderHandle>,
    /// Bodies moved by a character controller: the controller and the move asked for
    /// this tick.
    characters: HashMap<ScriptId, (KinematicCharacterController, Option<Vector>)>,
    /// Joints by script id.
    joints: HashMap<ScriptId, ImpulseJointHandle>,
    /// A fixed body for joints to the world (`null` for the second body), made when
    /// first needed.
    world_anchor: Option<RigidBodyHandle>,
    /// Raycasts queued this tick, answered after the step.
    rays: Vec<(u32, Ray, f32)>,
    /// Colliders were added or removed since the last step: they aren't in the query
    /// structures until a step runs, so character moves wait a step (moving through a
    /// floor that was just added would be wrong).
    dirty: bool,
    batch: Batch,
}

impl Default for Physics {
    fn default() -> Self {
        Self::new()
    }
}

impl Physics {
    /// An empty world with Earth gravity (y up, metres).
    pub fn new() -> Self {
        let mut world = PhysicsWorld::new();
        world.gravity = Vector::new(0.0, -9.81, 0.0);
        Self {
            world,
            bodies: HashMap::new(),
            colliders: HashMap::new(),
            characters: HashMap::new(),
            rays: Vec::new(),
            joints: HashMap::new(),
            world_anchor: None,
            dirty: false,
            batch: Batch::default(),
        }
    }

    pub fn body_count(&self) -> usize {
        self.bodies.len()
    }

    pub fn collider_count(&self) -> usize {
        self.colliders.len()
    }

    /// A body's position, if it exists.
    pub fn position(&self, id: ScriptId) -> Option<[f32; 3]> {
        let body = self.world.bodies.get(*self.bodies.get(&id)?)?;
        Some(body.translation().to_array())
    }

    /// Runs one `physics.*` op (the name without the prefix). Returns an error message
    /// for a bad op; nothing is changed then.
    pub fn apply(&mut self, name: &str, args: &[Value]) -> Result<(), String> {
        let a = Args(args);
        self.dirty |= matches!(name, "collider" | "removeBody" | "removeCollider");
        match name {
            "gravity" => self.world.gravity = a.vec3(0)?,

            "timestep" => self.world.integration_parameters.dt = a.num(0)?.max(1e-4),
            "body" => self.add_body(&a)?,
            "collider" => self.add_collider(&a)?,
            "removeBody" => {
                let id = a.id(0)?;
                self.characters.remove(&id);
                if let Some(h) = self.bodies.remove(&id)
                    && let Some(body) = self.world.remove_body(h)
                {
                    // Its colliders went with it.
                    self.colliders.retain(|_, c| !body.colliders().contains(c));
                }
            }
            "removeCollider" => {
                if let Some(h) = self.colliders.remove(&a.id(0)?) {
                    self.world.remove_collider(h);
                }
            }
            "set" => self.set(&a)?,
            "colliderSet" => self.collider_set(&a)?,
            "joint" => self.add_joint(&a)?,
            "removeJoint" => {
                if let Some(h) = self.joints.remove(&a.id(0)?) {
                    // Gone already if one of its bodies was removed.
                    self.world.remove_impulse_joint(h);
                }
            }
            "force" => {
                let (body, v) = (self.body_mut(a.id(0)?)?, a.vec3(2)?);
                match a.str(1)? {
                    "force" => body.add_force(v, true),
                    "impulse" => body.apply_impulse(v, true),
                    "torque" => body.add_torque(v, true),
                    "torqueImpulse" => body.apply_torque_impulse(v, true),
                    // `force id impulseAt [impulse] [world point]`
                    "impulseAt" => body.apply_impulse_at_point(v, a.vec3(3)?, true),
                    other => return Err(format!("unknown force kind `{other}`")),
                }
            }
            "clearForces" => {
                let body = self.body_mut(a.id(0)?)?;
                body.reset_forces(true);
                body.reset_torques(true);
            }
            "character" => {
                let id = a.id(0)?;
                if !self.bodies.contains_key(&id) {
                    return Err(format!("no body {id}"));
                }
                let controller = KinematicCharacterController {
                    max_slope_climb_angle: a.opt_num(1).unwrap_or(45.0).to_radians(),
                    snap_to_ground: a.opt_num(2).map(CharacterLength::Absolute),
                    autostep: a.opt_num(3).map(|h| CharacterAutostep {
                        max_height: CharacterLength::Absolute(h),
                        min_width: CharacterLength::Absolute(0.2),
                        include_dynamic_bodies: false,
                    }),
                    ..Default::default()
                };
                self.characters.insert(id, (controller, None));
            }
            "move" => {
                let id = a.id(0)?;
                let delta = a.vec3(1)?;
                let Some((_, pending)) = self.characters.get_mut(&id) else {
                    return Err(format!("body {id} has no character controller"));
                };
                *pending = Some(pending.unwrap_or(Vector::ZERO) + delta);
            }
            "raycast" => {
                // `raycast query [origin] [direction] maxDistance`
                let dir = a.vec3(2)?;
                if dir.length_squared() == 0.0 {
                    return Err("raycast needs a direction".into());
                }
                let ray = Ray::new(a.vec3(1)?, dir.normalize());
                self.rays.push((a.id(0)?, ray, a.num(3)?.max(0.0)));
            }
            other => return Err(format!("unknown physics op `{other}`")),
        }
        Ok(())
    }

    /// `body id kind [px py pz] [qx qy qz qw] [vx vy vz] options`: options is
    /// `[gravityScale, linearDamping, angularDamping, ccd, lockRotations]`.
    fn add_body(&mut self, a: &Args) -> Result<(), String> {
        let id = a.id(0)?;
        let builder = match a.str(1)? {
            "dynamic" => RigidBodyBuilder::dynamic(),
            "fixed" => RigidBodyBuilder::fixed(),
            "kinematic" => RigidBodyBuilder::kinematic_position_based(),
            other => return Err(format!("unknown body kind `{other}`")),
        };
        let opts = a.list(5).unwrap_or(&[]);
        let opt = |i: usize| opts.get(i).and_then(Value::as_f64).map(|v| v as f32);
        let flag = |i: usize| matches!(opts.get(i), Some(Value::Bool(true)));
        let mut builder = builder
            .translation(a.vec3(2).unwrap_or(Vector::ZERO))
            .linvel(a.vec3(4).unwrap_or(Vector::ZERO))
            .gravity_scale(opt(0).unwrap_or(1.0))
            .linear_damping(opt(1).unwrap_or(0.0))
            .angular_damping(opt(2).unwrap_or(0.0))
            .ccd_enabled(flag(3));
        if flag(4) {
            builder = builder.lock_rotations();
        }
        let mut body = builder.build();
        if let Ok(q) = a.quat(3) {
            body.set_rotation(q, false);
        }
        if let Some(old) = self.bodies.remove(&id) {
            self.world.remove_body(old);
        }
        let handle = self.world.insert_body(body);
        self.bodies.insert(id, handle);
        Ok(())
    }

    /// `collider id body|null shape [dims] [px py pz] [qx qy qz qw] options`: options is
    /// `[friction, restitution, density, sensor, events]`. Shapes: `box` (half extents),
    /// `ball` (radius), `capsule` (half height, radius; along y), `cylinder` (half height,
    /// radius), `convex` (points, flat), `mesh` (points flat, then `indices` in the
    /// options' slot 5).
    fn add_collider(&mut self, a: &Args) -> Result<(), String> {
        let id = a.id(0)?;
        let parent = match a.0.get(1) {
            Some(Value::Null) | None => None,
            _ => Some(
                *self
                    .bodies
                    .get(&a.id(1)?)
                    .ok_or_else(|| format!("collider {id}: no body {}", a.id(1).unwrap_or(0)))?,
            ),
        };
        let dims: Vec<f32> = a
            .list(3)?
            .iter()
            .map(|v| v.as_f64().unwrap_or(0.0) as f32)
            .collect();
        let dim = |i: usize| dims.get(i).copied().unwrap_or(0.5).max(0.0);
        let points = || -> Vec<Vector> {
            dims.as_chunks::<3>()
                .0
                .iter()
                .map(|&p| Vector::from_array(p))
                .collect()
        };
        let opts = a.list(6).unwrap_or(&[]);
        let builder = match a.str(2)? {
            "box" => ColliderBuilder::cuboid(dim(0), dim(1), dim(2)),
            "ball" => ColliderBuilder::ball(dim(0)),
            "capsule" => ColliderBuilder::capsule_y(dim(0), dim(1)),
            "cylinder" => ColliderBuilder::cylinder(dim(0), dim(1)),
            "convex" => ColliderBuilder::convex_hull(&points())
                .ok_or_else(|| format!("collider {id}: the points have no convex hull"))?,
            "mesh" => {
                let indices: Vec<[u32; 3]> = opts
                    .get(5)
                    .and_then(|v| match v {
                        Value::Array(items) => Some(items),
                        _ => None,
                    })
                    .map(|items| {
                        let index = |v: &Value| v.as_f64().unwrap_or(0.0) as u32;
                        items
                            .as_chunks::<3>()
                            .0
                            .iter()
                            .map(|t| t.each_ref().map(index))
                            .collect()
                    })
                    .unwrap_or_default();
                let count = dims.len() as u32 / 3;
                if indices.is_empty() || indices.iter().flatten().any(|&i| i >= count) {
                    return Err(format!(
                        "collider {id}: mesh indices are missing or out of range"
                    ));
                }
                ColliderBuilder::trimesh(points(), indices)
                    .map_err(|e| format!("collider {id}: {e}"))?
            }
            other => return Err(format!("unknown collider shape `{other}`")),
        };
        let opt = |i: usize| opts.get(i).and_then(Value::as_f64).map(|v| v as f32);
        let flag = |i: usize| matches!(opts.get(i), Some(Value::Bool(true)));
        let mut builder = builder
            .position(Pose::from_parts(
                a.vec3(4).unwrap_or(Vector::ZERO),
                a.quat(5).unwrap_or(Rotation::IDENTITY),
            ))
            .friction(opt(0).unwrap_or(0.5))
            .restitution(opt(1).unwrap_or(0.0))
            .density(opt(2).unwrap_or(1.0))
            .sensor(flag(3))
            .user_data(id as u128);
        if flag(4) {
            builder = builder.active_events(ActiveEvents::COLLISION_EVENTS);
        }
        let collider = builder.build();
        if let Some(old) = self.colliders.remove(&id) {
            self.world.remove_collider(old);
        }
        let handle = self.world.insert_collider(collider, parent);
        self.colliders.insert(id, handle);
        Ok(())
    }

    /// `set id property values`: `position` [x y z], `rotation` [x y z w], `velocity`,
    /// `angularVelocity`, `target` (a kinematic body's next position), `targetRotation`,
    /// `enabled` (bool), `kind` (`dynamic` / `fixed` / `kinematic`), `mass` (kg, shared
    /// by its colliders), `gravityScale`, `linearDamping`, `angularDamping`, `ccd`
    /// (bool), `constraints` ([x y z] translations then [x y z] rotations allowed, 6
    /// bools), `sleep`, `wake`.
    fn set(&mut self, a: &Args) -> Result<(), String> {
        let id = a.id(0)?;
        let flag = |i: usize| matches!(a.0.get(i), Some(Value::Bool(true)));
        if a.str(1)? == "mass" {
            return self.set_mass(id, a.num(2)?.max(0.0));
        }
        let body = self.body_mut(id)?;
        match a.str(1)? {
            "position" => body.set_translation(a.vec3(2)?, true),
            "rotation" => body.set_rotation(a.quat(2)?, true),
            "velocity" => body.set_linvel(a.vec3(2)?, true),
            "angularVelocity" => body.set_angvel(a.vec3(2)?, true),
            "target" => body.set_next_kinematic_translation(a.vec3(2)?),
            "targetRotation" => body.set_next_kinematic_rotation(a.quat(2)?),
            "enabled" => body.set_enabled(flag(2)),
            "kind" => body.set_body_type(
                match a.str(2)? {
                    "dynamic" => RigidBodyType::Dynamic,
                    "fixed" => RigidBodyType::Fixed,
                    "kinematic" => RigidBodyType::KinematicPositionBased,
                    other => return Err(format!("unknown body kind `{other}`")),
                },
                true,
            ),
            "gravityScale" => body.set_gravity_scale(a.num(2)?, true),
            "linearDamping" => body.set_linear_damping(a.num(2)?.max(0.0)),
            "angularDamping" => body.set_angular_damping(a.num(2)?.max(0.0)),
            "ccd" => body.enable_ccd(flag(2)),
            "constraints" => {
                let c = a.list(2)?;
                let allow = |i: usize| !matches!(c.get(i), Some(Value::Bool(false)));
                body.set_enabled_translations(allow(0), allow(1), allow(2), true);
                body.set_enabled_rotations(allow(3), allow(4), allow(5), true);
            }
            "sleep" => body.sleep(),
            "wake" => body.wake_up(true),
            other => return Err(format!("unknown body property `{other}`")),
        }
        Ok(())
    }

    /// Gives a body `mass` kg, split evenly over its colliders (their shapes still set
    /// how it turns); a body without colliders gets it as extra mass.
    fn set_mass(&mut self, id: ScriptId, mass: f32) -> Result<(), String> {
        let handle = *self
            .bodies
            .get(&id)
            .ok_or_else(|| format!("no body {id}"))?;
        let colliders: Vec<ColliderHandle> = self.world.bodies[handle].colliders().to_vec();
        if colliders.is_empty() {
            self.world.bodies[handle].set_additional_mass(mass, true);
            return Ok(());
        }
        let share = mass / colliders.len() as f32;
        for c in colliders {
            if let Some(collider) = self.world.colliders.get_mut(c) {
                collider.set_mass(share);
            }
        }
        self.world.bodies[handle].wake_up(true);
        Ok(())
    }

    /// `joint id kind bodyA bodyB|null [anchorA] [anchorB] [params]`: joins two bodies at
    /// anchors in their own spaces (`null`: to the world, anchor B in world space).
    /// Kinds and params: `fixed`; `ball` (turns freely about the anchor); `hinge` [axis x
    /// y z, use limits (0/1), min, max (degrees), use motor (0/1), target speed (degrees
    /// per second), motor strength]; `spring` [rest length, stiffness, damping]; `rope`
    /// [max length]. Joined bodies don't collide with each other.
    fn add_joint(&mut self, a: &Args) -> Result<(), String> {
        let id = a.id(0)?;
        let body1 = *self
            .bodies
            .get(&a.id(2)?)
            .ok_or_else(|| format!("joint {id}: no body {}", a.id(2).unwrap_or(0)))?;
        let body2 = match a.0.get(3) {
            Some(Value::Null) | None => self.world_anchor(),
            _ => *self
                .bodies
                .get(&a.id(3)?)
                .ok_or_else(|| format!("joint {id}: no body {}", a.id(3).unwrap_or(0)))?,
        };
        let anchor1 = a.vec3(4).unwrap_or(Vector::ZERO);
        let anchor2 = a.vec3(5).unwrap_or(Vector::ZERO);
        let p = a.list(6).unwrap_or(&[]);
        let n = |i: usize, d: f32| p.get(i).and_then(Value::as_f64).map_or(d, |v| v as f32);
        let joint: GenericJoint = match a.str(1)? {
            "fixed" => FixedJointBuilder::new()
                .local_anchor1(anchor1)
                .local_anchor2(anchor2)
                .contacts_enabled(false)
                .into(),
            "ball" => SphericalJointBuilder::new()
                .local_anchor1(anchor1)
                .local_anchor2(anchor2)
                .contacts_enabled(false)
                .into(),
            "hinge" => {
                let axis = Vector::new(n(0, 0.0), n(1, 1.0), n(2, 0.0)).normalize_or(Vector::Y);
                let mut b = RevoluteJointBuilder::new(axis)
                    .local_anchor1(anchor1)
                    .local_anchor2(anchor2)
                    .contacts_enabled(false);
                if n(3, 0.0) > 0.5 {
                    b = b.limits([n(4, -90.0).to_radians(), n(5, 90.0).to_radians()]);
                }
                if n(6, 0.0) > 0.5 {
                    b = b.motor_velocity(n(7, 0.0).to_radians(), n(8, 1.0));
                }
                b.into()
            }
            "spring" => SpringJointBuilder::new(n(0, 1.0).max(0.0), n(1, 10.0), n(2, 0.5))
                .local_anchor1(anchor1)
                .local_anchor2(anchor2)
                .contacts_enabled(false)
                .into(),
            "rope" => RopeJointBuilder::new(n(0, 1.0).max(0.0))
                .local_anchor1(anchor1)
                .local_anchor2(anchor2)
                .contacts_enabled(false)
                .into(),
            other => return Err(format!("unknown joint kind `{other}`")),
        };
        if let Some(old) = self.joints.remove(&id) {
            self.world.remove_impulse_joint(old);
        }
        let handle = self.world.insert_impulse_joint(body1, body2, joint);
        self.joints.insert(id, handle);
        Ok(())
    }

    /// The fixed body joints to the world hang from.
    fn world_anchor(&mut self) -> RigidBodyHandle {
        if let Some(h) = self.world_anchor {
            return h;
        }
        let h = self.world.insert_body(RigidBodyBuilder::fixed());
        self.world_anchor = Some(h);
        h
    }

    pub fn joint_count(&self) -> usize {
        self.joints.len()
    }

    /// `colliderSet id property value`: `friction`, `restitution`, `sensor` (bool),
    /// `offset` ([x y z] [x y z w] from its body).
    fn collider_set(&mut self, a: &Args) -> Result<(), String> {
        let id = a.id(0)?;
        let collider = self
            .colliders
            .get(&id)
            .and_then(|h| self.world.colliders.get_mut(*h))
            .ok_or_else(|| format!("no collider {id}"))?;
        match a.str(1)? {
            "friction" => collider.set_friction(a.num(2)?.max(0.0)),
            "restitution" => collider.set_restitution(a.num(2)?.clamp(0.0, 1.0)),
            "sensor" => collider.set_sensor(matches!(a.0.get(2), Some(Value::Bool(true)))),
            "offset" => collider.set_position_wrt_parent(Pose::from_parts(
                a.vec3(2)?,
                a.quat(3).unwrap_or(Rotation::IDENTITY),
            )),
            other => return Err(format!("unknown collider property `{other}`")),
        }
        Ok(())
    }

    /// Moves a body to where a script put its GameObject: a kinematic body travels there
    /// over the next step (pushing things on the way); others teleport (keeping their
    /// velocity, like setting a Unity transform).
    /// The world's gravity (cloth uses it too).
    pub fn gravity(&self) -> [f32; 3] {
        let g = self.world.gravity;
        [g.x, g.y, g.z]
    }

    /// Arrays rather than vectors: callers may use another glam version than rapier's.
    pub fn set_pose(&mut self, id: ScriptId, position: [f32; 3], rotation: [f32; 4]) {
        let (position, rotation) = (Vector::from_array(position), Rotation::from_array(rotation));
        let Ok(body) = self.body_mut(id) else { return };
        if body.is_kinematic() {
            body.set_next_kinematic_translation(position);
            body.set_next_kinematic_rotation(rotation);
        } else {
            body.set_translation(position, true);
            body.set_rotation(rotation, true);
        }
    }

    /// A body's position and rotation, if it exists.
    pub fn pose(&self, id: ScriptId) -> Option<([f32; 3], [f32; 4])> {
        let body = self.world.bodies.get(*self.bodies.get(&id)?)?;
        Some((body.translation().to_array(), body.rotation().to_array()))
    }

    fn body_mut(&mut self, id: ScriptId) -> Result<&mut RigidBody, String> {
        self.bodies
            .get(&id)
            .and_then(|h| self.world.bodies.get_mut(*h))
            .ok_or_else(|| format!("no body {id}"))
    }

    /// Advances the world by one timestep (`physics.timestep`, 1/60 s by default):
    /// moves character controllers, steps, then answers raycasts and records what moved.
    pub fn step(&mut self) {
        self.move_characters();
        let events = Events::default();
        self.world.step_with_events(&(), &events);

        let script_id = |h: ColliderHandle| self.world.colliders.get(h).map(|c| c.user_data as u32);
        for (event, speed) in events.0.into_inner().unwrap_or_default() {
            let (Some(a), Some(b)) = (script_id(event.collider1()), script_id(event.collider2()))
            else {
                continue;
            };
            self.batch.contacts.push(Contact {
                started: event.started(),
                a,
                b,
                sensor: event.sensor(),
                speed,
            });
        }

        let ids: HashMap<RigidBodyHandle, ScriptId> =
            self.bodies.iter().map(|(&id, &h)| (h, id)).collect();
        for (handle, body) in self.world.active_bodies() {
            if !body.is_dynamic() && !body.is_kinematic() {
                continue;
            }
            let Some(&id) = ids.get(&handle) else {
                continue;
            };
            self.batch.bodies.push(BodyState {
                id,
                position: body.translation().to_array(),
                rotation: body.rotation().to_array(),
                velocity: body.linvel().to_array(),
                angular_velocity: body.angvel().to_array(),
            });
        }

        for (query, ray, max) in std::mem::take(&mut self.rays) {
            let hit = self
                .world
                .cast_ray_and_get_normal(&ray, max, true, QueryFilter::default())
                .and_then(|(h, hit)| {
                    Some((script_id(h)?, hit.time_of_impact, hit.normal.to_array()))
                });
            self.batch.rays.push(RayHit { query, hit });
        }
    }

    /// Applies each character controller's pending move as its body's next kinematic
    /// position, sliding along and stepping over what it hits.
    fn move_characters(&mut self) {
        // New colliders aren't in the query structures until this step runs: their moves
        // stay pending until the next one. (Running collision detection here instead
        // trips rapier's island bookkeeping for bodies that just changed type.)
        if std::mem::take(&mut self.dirty) {
            return;
        }
        let dt = self.world.integration_parameters.dt;
        let mut moves = Vec::new();
        for (&id, (controller, pending)) in &mut self.characters {
            let Some(delta) = pending.take() else {
                continue;
            };
            let Some(body) = self.bodies.get(&id).and_then(|h| self.world.bodies.get(*h)) else {
                continue;
            };
            // The body's first collider is the character's shape.
            let Some(collider) = body
                .colliders()
                .first()
                .and_then(|c| self.world.colliders.get(*c))
            else {
                continue;
            };
            let handle = self.bodies[&id];
            let filter = QueryFilter::default()
                .exclude_rigid_body(handle)
                .exclude_sensors();
            let queries = self.world.query_pipeline_with_filter(filter);
            let movement = controller.move_shape(
                dt,
                &queries,
                collider.shape(),
                collider.position(),
                delta,
                |_| {},
            );
            moves.push((
                id,
                handle,
                body.translation() + movement.translation,
                movement.grounded,
            ));
        }
        for (id, handle, target, grounded) in moves {
            if let Some(body) = self.world.bodies.get_mut(handle) {
                if body.is_kinematic() {
                    body.set_next_kinematic_translation(target);
                } else {
                    body.set_translation(target, true);
                }
            }
            self.batch.characters.push((id, grounded));
        }
    }

    /// Takes what scripts need from the ticks since the last call.
    pub fn take_batch(&mut self) -> Batch {
        std::mem::take(&mut self.batch)
    }
}

/// Reads op arguments with errors that say what was wrong.
struct Args<'a>(&'a [Value]);

impl Args<'_> {
    fn num(&self, i: usize) -> Result<f32, String> {
        self.0
            .get(i)
            .and_then(Value::as_f64)
            .map(|v| v as f32)
            .filter(|v| v.is_finite())
            .ok_or_else(|| format!("argument {i} must be a number"))
    }

    fn opt_num(&self, i: usize) -> Option<f32> {
        self.num(i).ok()
    }

    /// Ids go through `f64`: `f32` can't hold integers past 2^24 (script component
    /// and engine object ids are bigger).
    fn id(&self, i: usize) -> Result<ScriptId, String> {
        self.0
            .get(i)
            .and_then(Value::as_f64)
            .filter(|v| v.is_finite() && *v >= 0.0 && *v <= u32::MAX as f64)
            .map(|v| v as ScriptId)
            .ok_or_else(|| format!("argument {i} must be an id"))
    }

    fn str(&self, i: usize) -> Result<&str, String> {
        self.0
            .get(i)
            .and_then(Value::as_str)
            .ok_or_else(|| format!("argument {i} must be a string"))
    }

    fn list(&self, i: usize) -> Result<&[Value], String> {
        match self.0.get(i) {
            Some(Value::Array(items)) => Ok(items),
            _ => Err(format!("argument {i} must be an array")),
        }
    }

    /// `N` numbers: an array, or the arguments from `i` on.
    fn floats<const N: usize>(&self, i: usize) -> Result<[f32; N], String> {
        let from: &[Value] = match self.0.get(i) {
            Some(Value::Array(items)) => items,
            _ => self.0.get(i..).unwrap_or(&[]),
        };
        let mut out = [0.0; N];
        for (k, slot) in out.iter_mut().enumerate() {
            *slot = from
                .get(k)
                .and_then(Value::as_f64)
                .map(|v| v as f32)
                .filter(|v| v.is_finite())
                .ok_or_else(|| format!("argument {i} must hold {N} numbers"))?;
        }
        Ok(out)
    }

    fn vec3(&self, i: usize) -> Result<Vector, String> {
        self.floats::<3>(i).map(Vector::from_array)
    }

    fn quat(&self, i: usize) -> Result<Rotation, String> {
        let [x, y, z, w] = self.floats::<4>(i)?;
        let q = Rotation::from_xyzw(x, y, z, w);
        if q.length_squared() < 1e-12 {
            return Err(format!("argument {i} is not a rotation"));
        }
        Ok(q.normalize())
    }
}
