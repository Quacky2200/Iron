//! Keeps GameObjects and their physics bodies together (`Rigidbody` / collider
//! components attach a body to their object with `physics.attach`): scripts moving an
//! object move its body before the step, and bodies the step moved move their objects.

use std::collections::HashMap;

use iron_core::Value;
use iron_core::glam::{Quat, Vec3};
use iron_physics::{Batch, Physics};
use iron_scene::ScriptScene;

/// Bodies attached to GameObjects, both ways.
#[derive(Default)]
pub struct Attachments {
    body_objects: HashMap<u32, u32>,
    object_bodies: HashMap<u32, u32>,
}

impl Attachments {
    /// Handles `physics.attach` and notes `physics.removeBody`; returns whether the op
    /// was `attach` (which the physics world doesn't see).
    pub fn op(&mut self, name: &str, args: &[Value]) -> bool {
        let id = |i: usize| args.get(i).and_then(Value::as_f64).map(|v| v as u32);
        match name {
            "attach" => {
                match (id(0), id(1)) {
                    (Some(body), Some(object)) => {
                        self.body_objects.insert(body, object);
                        self.object_bodies.insert(object, body);
                    }
                    _ => tracing::warn!("physics.attach needs a body and an object"),
                }
                true
            }
            "removeBody" => {
                if let Some(object) = id(0).and_then(|b| self.body_objects.remove(&b)) {
                    self.object_bodies.remove(&object);
                }
                false
            }
            _ => false,
        }
    }

    /// One physics step with the scene: bodies follow the objects scripts moved, the
    /// world steps, then objects follow the bodies that moved. Returns the step's batch
    /// (for `__iron_physics`).
    pub fn step(&self, physics: &mut Physics, scene: &mut ScriptScene) -> Batch {
        for object in scene.take_script_moved() {
            let (Some(&body), Some(id)) = (self.object_bodies.get(&object), scene.object(object))
            else {
                continue;
            };
            let (_, rotation, position) =
                scene.scene.world_matrix(id).to_scale_rotation_translation();
            physics.set_pose(body, position.to_array(), rotation.to_array());
        }
        physics.step();
        let batch = physics.take_batch();
        for moved in &batch.bodies {
            if let Some(&object) = self.body_objects.get(&moved.id) {
                scene.set_world_from_engine(
                    object,
                    Vec3::from_array(moved.position),
                    Quat::from_array(moved.rotation),
                );
            }
        }
        batch
    }
}

#[cfg(test)]
mod tests {
    use iron_core::ComponentId;
    use iron_scene::SceneEvent;
    use iron_script::{ComponentHost, QuickJsRuntime, ScriptRuntime};

    use super::*;

    const RIGIDBODIES: &str = include_str!("../../../tests/fixtures/rigidbodies.js");

    // M5-RB-01
    #[test]
    fn rigidbodies_move_their_game_objects_and_collide() {
        let mut rt = QuickJsRuntime::new();
        rt.load_bundle("rigidbodies.js", RIGIDBODIES).unwrap();
        let mut host = ComponentHost::new();
        let mut scene = ScriptScene::new();
        let mut physics = Physics::new();
        let mut attached = Attachments::default();
        let game = scene.create_engine_object("Drop");
        host.spawn_on(&mut rt, "Drop", Some(game), "Drop").unwrap();

        for _ in 0..150 {
            let active = |o: u32| scene.active(o);
            assert!(host.fixed_update(&mut rt, 1.0 / 60.0, &active).is_empty());
            assert!(host.update_active(&mut rt, 1.0 / 60.0, &active).is_empty());
            for op in rt.drain_ops() {
                if let Some(name) = op.name.strip_prefix("scene.") {
                    match scene.apply(name, &op.args).unwrap() {
                        Some(SceneEvent::ComponentAdded {
                            object,
                            component,
                            class,
                        }) => host.adopt(ComponentId(component), &class, Some(object)),
                        Some(SceneEvent::ComponentRemoved(id)) => {
                            host.remove(&mut rt, ComponentId(id))
                        }
                        Some(SceneEvent::Destroyed(objects)) => {
                            host.remove_on_objects(&mut rt, &objects)
                        }
                        None => {}
                    }
                } else if let Some(name) = op.name.strip_prefix("physics.") {
                    if !attached.op(name, &op.args) {
                        physics
                            .apply(name, &op.args)
                            .unwrap_or_else(|e| panic!("{name}: {e}"));
                    }
                } else if !op.name.starts_with("render.") {
                    panic!("unexpected op {}", op.name);
                }
            }
            let batch = attached.step(&mut physics, &mut scene);
            let moved = scene.take_batch();
            if !moved.is_empty() {
                rt.call_global("__iron_scene", &[Value::Array(moved)])
                    .unwrap();
            }
            if !batch.is_empty() {
                rt.call_global("__iron_physics", &batch.to_args()).unwrap();
            }
        }

        // The crate (a 1 m box dropped from 3 m onto a static floor whose top is at 0)
        // rests with its centre half a metre up, its GameObject with it, and its
        // components heard the trigger and the landing. The kinematic paddle, moved by its script, carried
        // its body along.
        let log = rt.eval("__rb_log()").unwrap();
        let Value::String(log) = log else {
            panic!("{log:?}")
        };
        // It falls through the trigger zone first, then lands.
        assert_eq!(log, "trigger zone, crate hit Floor");
        let crate_y = rt.eval("__crate_y()").unwrap().as_f64().unwrap();
        assert!((crate_y - 0.5).abs() < 0.03, "crate y = {crate_y}");
        let paddle = rt.eval("__paddle_body_x()").unwrap().as_f64().unwrap();
        assert!((paddle - 2.5).abs() < 1e-3, "paddle body x = {paddle}");

        // M5-RB-03: the character walked 2 m/s for 2.5 s along the floor, grounded, its
        // capsule's bottom on the floor (centre 1 m up).
        let Value::Array(w) = rt.eval("__walker()").unwrap() else {
            panic!()
        };
        let w: Vec<f64> = w.iter().map(|v| v.as_f64().unwrap()).collect();
        assert!(w[2] > 4.0 && w[2] < 5.2, "walked to z = {}", w[2]);
        assert!((w[1] - 1.0).abs() < 0.1, "walker y = {}", w[1]);
        assert_eq!(w[3], 1.0, "grounded");

        // M5-JNT-02: the rope holds the weight about 2 m below its anchor (6 m up, plus the
        // weight's radius of slack); the motor turned the wheel.
        let weight = rt.eval("__weight_y()").unwrap().as_f64().unwrap();
        assert!(weight > 3.5 && weight < 4.3, "weight y = {weight}");
        let angle = rt.eval("__wheel_angle()").unwrap().as_f64().unwrap();
        assert!(angle > 10.0, "wheel turned {angle} degrees");
    }
}
