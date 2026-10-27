// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

//! `Cloth` components: each simulates a copy of its object's mesh (`platypus_physics::cloth`)
//! on the fixed tick, holding pinned vertices where the object is and pushing vertices out
//! of its sphere and capsule colliders, then hands the moved vertices to the renderer.
//!
//! `cloth.cloth id object sourceMesh(name | id) targetMesh [stretch bend damping
//! friction iterations] [extra acceleration xyz] [random acceleration xyz] useGravity
//! [pin points xyz...] pinRadius [pin box min xyz max xyz]|null [colliders]` sets one up
//! (again: rebuilt); each collider is `["sphere", object, radius, [center]]` or
//! `["capsule", object, radius, height, [center]]` (a capsule stands along its object's
//! y). `cloth.remove id` drops it.

use std::collections::HashMap;

use platypus_core::Value;
use platypus_core::glam::{Mat4, Vec3};
use platypus_physics::cloth::{Cloth, ClothCollider, ClothSettings};
use platypus_scene::ScriptScene;

use crate::world_view::WorldViews;

#[derive(Clone, Copy, Debug)]
enum ColliderRef {
    Sphere {
        object: u32,
        radius: f32,
        center: Vec3,
    },
    Capsule {
        object: u32,
        radius: f32,
        height: f32,
        center: Vec3,
    },
}

struct Entry {
    object: u32,
    target: u32,
    cloth: Cloth,
    extra: Vec3,
    use_gravity: bool,
    colliders: Vec<ColliderRef>,
}

#[derive(Default)]
pub struct ClothSystem {
    cloths: HashMap<u32, Entry>,
}

fn nums(v: Option<&Value>) -> Vec<f32> {
    match v {
        Some(Value::Array(a)) => a
            .iter()
            .filter_map(|x| x.as_f64())
            .map(|x| x as f32)
            .collect(),
        _ => Vec::new(),
    }
}

fn vec3(v: Option<&Value>) -> Vec3 {
    match nums(v)[..] {
        [x, y, z, ..] => Vec3::new(x, y, z),
        _ => Vec3::ZERO,
    }
}

impl ClothSystem {
    pub fn is_empty(&self) -> bool {
        self.cloths.is_empty()
    }

    pub fn apply(
        &mut self,
        name: &str,
        args: &[Value],
        scene: &ScriptScene,
        views: &mut WorldViews,
    ) -> Result<(), String> {
        let id = |i: usize| -> Result<u32, String> {
            args.get(i)
                .and_then(Value::as_f64)
                .map(|v| v as u32)
                .ok_or_else(|| format!("argument {i} must be an id"))
        };
        match name {
            "remove" => {
                self.cloths.remove(&id(0)?);
                Ok(())
            }
            "cloth" => {
                let (cid, object, target) = (id(0)?, id(1)?, id(3)?);
                let source = views
                    .mesh_source(args.get(2))
                    .ok_or("the cloth's mesh isn't known yet")?;
                let s = nums(args.get(4));
                let get = |i: usize, d: f32| s.get(i).copied().unwrap_or(d);
                let pins: Vec<Vec3> = nums(args.get(8))
                    .as_chunks::<3>()
                    .0
                    .iter()
                    .map(|c| Vec3::from(*c))
                    .collect();
                let pin_radius = args.get(9).and_then(Value::as_f64).unwrap_or(0.05) as f32;
                let pin_box = match nums(args.get(10))[..] {
                    [a, b, c, d, e, f] => Some((Vec3::new(a, b, c), Vec3::new(d, e, f))),
                    _ => None,
                };
                let pinned = |p: Vec3| {
                    pins.iter().any(|q| q.distance(p) <= pin_radius)
                        || pin_box.is_some_and(|(lo, hi)| p.cmpge(lo).all() && p.cmple(hi).all())
                };
                let world = world_of(scene, object).unwrap_or(Mat4::IDENTITY);
                let vertices: Vec<Vec3> = source.positions.iter().map(|p| Vec3::from(*p)).collect();
                let mut cloth = Cloth::new(&vertices, &source.indices, world, pinned);
                cloth.settings = ClothSettings {
                    stretching_stiffness: get(0, 1.0),
                    bending_stiffness: get(1, 0.3),
                    damping: get(2, 0.1),
                    friction: get(3, 0.5),
                    iterations: get(4, 8.0).clamp(1.0, 64.0) as u32,
                    random_acceleration: vec3(args.get(6)),
                    ..Default::default()
                };
                let colliders = match args.get(11) {
                    Some(Value::Array(list)) => list.iter().filter_map(collider).collect(),
                    _ => Vec::new(),
                };
                views.set_cloth_mesh(target, source);
                self.cloths.insert(
                    cid,
                    Entry {
                        object,
                        target,
                        cloth,
                        extra: vec3(args.get(5)),
                        use_gravity: !matches!(args.get(7), Some(Value::Bool(false))),
                        colliders,
                    },
                );
                Ok(())
            }
            other => Err(format!("unknown cloth op `{other}`")),
        }
    }

    /// One fixed step: every cloth on an active object moves, and its mesh updates.
    pub fn step(&mut self, dt: f32, gravity: Vec3, scene: &ScriptScene, views: &mut WorldViews) {
        for e in self.cloths.values_mut() {
            if !scene.active(e.object) {
                continue;
            }
            let Some(world) = world_of(scene, e.object) else {
                continue;
            };
            e.cloth.settings.acceleration =
                e.extra + if e.use_gravity { gravity } else { Vec3::ZERO };
            e.cloth.colliders = e
                .colliders
                .iter()
                .filter_map(|c| collider_in_world(scene, *c))
                .collect();
            e.cloth.step(dt, world);
            views.move_cloth_mesh(e.target, e.cloth.local_positions(world));
        }
    }

    /// Drops cloths on objects that are gone.
    pub fn retain(&mut self, alive: impl Fn(u32) -> bool) {
        self.cloths.retain(|_, e| alive(e.object));
    }
}

fn world_of(scene: &ScriptScene, object: u32) -> Option<Mat4> {
    scene
        .object(object)
        .map(|id| Mat4::from(scene.scene.world_matrix(id)))
}

fn collider(v: &Value) -> Option<ColliderRef> {
    let Value::Array(a) = v else { return None };
    let num = |i: usize| a.get(i).and_then(Value::as_f64).map(|x| x as f32);
    // Ids through `f64`: engine ids (2^30 up) do not fit in `f32`.
    let object = |i: usize| a.get(i).and_then(Value::as_f64).map(|x| x as u32);
    match a.first().and_then(Value::as_str)? {
        "sphere" => Some(ColliderRef::Sphere {
            object: object(1)?,
            radius: num(2)?,
            center: vec3(a.get(3)),
        }),
        "capsule" => Some(ColliderRef::Capsule {
            object: object(1)?,
            radius: num(2)?,
            height: num(3)?,
            center: vec3(a.get(4)),
        }),
        _ => None,
    }
}

/// A collider placed by its object's world matrix (scaled like the physics colliders).
fn collider_in_world(scene: &ScriptScene, c: ColliderRef) -> Option<ClothCollider> {
    match c {
        ColliderRef::Sphere {
            object,
            radius,
            center,
        } => {
            let m = world_of(scene, object)?;
            let (scale, _, _) = m.to_scale_rotation_translation();
            Some(ClothCollider::Sphere {
                center: m.transform_point3(center),
                radius: radius * scale.abs().max_element(),
            })
        }
        ColliderRef::Capsule {
            object,
            radius,
            height,
            center,
        } => {
            let m = world_of(scene, object)?;
            let (scale, _, _) = m.to_scale_rotation_translation();
            let r = radius * scale.x.abs().max(scale.z.abs());
            // The segment between the caps' centres, in local units.
            let half = (height * scale.y.abs() / 2.0 - r).max(0.0) / scale.y.abs().max(1e-6);
            Some(ClothCollider::Capsule {
                a: m.transform_point3(center - Vec3::Y * half),
                b: m.transform_point3(center + Vec3::Y * half),
                radius: r,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // M5-CLO-02
    #[test]
    fn a_cloth_on_a_quad_hangs_from_its_pinned_edge_and_follows_its_object() {
        let n = Value::from;
        let v3 = |x: f64, y: f64, z: f64| Value::Array(vec![n(x), n(y), n(z)]);
        let mut scene = ScriptScene::new();
        let mut views = WorldViews::default();
        let mut cloth = ClothSystem::default();
        scene
            .apply(
                "create",
                &[n(1.0), "flag".into(), Value::Null, v3(0.0, 2.0, 0.0)],
            )
            .unwrap();
        // The plane (10 m, 11 x 11 vertices in xz) pinned along its x = -5 edge; target
        // mesh 9.
        cloth
            .apply(
                "cloth",
                &[
                    n(5.0),
                    n(1.0),
                    "plane".into(),
                    n(9.0),
                    Value::Array(vec![n(1.0), n(0.2), n(0.1), n(0.5), n(10.0)]),
                    v3(0.0, 0.0, 0.0),
                    v3(0.0, 0.0, 0.0),
                    Value::Bool(true),
                    Value::Array(vec![]),
                    n(0.05),
                    Value::Array([-5.1, -1.0, -6.0, -4.9, 1.0, 6.0].map(n).to_vec()),
                    Value::Array(vec![]),
                ],
                &scene,
                &mut views,
            )
            .unwrap();
        assert!(matches!(views.mesh_uploads.last(), Some((9, Some(_)))));
        for _ in 0..120 {
            cloth.step(1.0 / 60.0, Vec3::new(0.0, -9.81, 0.0), &scene, &mut views);
        }
        let shape = views.mesh_data[&9].positions.clone();
        // The pinned edge stays; the far edge swings down under it.
        assert!((shape[0][0] + 5.0).abs() < 1e-4 && shape[0][1].abs() < 1e-4);
        assert!(shape[10][1] < -5.0, "far corner at {:?}", shape[10]);
        // Normals came back with the new shape; only the latest upload is queued.
        assert_eq!(views.mesh_data[&9].normals.len(), shape.len());
        assert_eq!(
            views.mesh_uploads.iter().filter(|(id, _)| *id == 9).count(),
            1
        );
        // Removed with its object.
        cloth.retain(|_| false);
        assert!(cloth.is_empty());
        assert!(cloth.apply("bend", &[], &scene, &mut views).is_err());
    }
}
