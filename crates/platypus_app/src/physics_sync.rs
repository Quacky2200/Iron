// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

//! Keeps GameObjects and their physics bodies together (`Rigidbody` / collider
//! components attach a body to their object with `physics.attach`): scripts moving an
//! object move its body before the step, and bodies the step moved move their objects.

use std::collections::HashMap;

use platypus_core::glam::{Quat, Vec3};
use platypus_physics::{Batch, Physics};
use platypus_scene::ScriptScene;

/// Bodies attached to GameObjects, both ways.
#[derive(Default)]
pub struct Attachments {
    body_objects: HashMap<u32, u32>,
    object_bodies: HashMap<u32, u32>,
}

impl Attachments {
    /// One physics step with the scene: bodies follow the objects scripts moved, the
    /// world steps, then objects follow the bodies that moved. Returns the step's batch
    /// (for `__platypus_physics`).
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
