// SPDX-License-Identifier: Apache-2.0

//! The bound `Pose` against the string-keyed one it replaced (`legacy_pose.rs`): random hierarchies,
//! clips, weights and times, with the hierarchy edited between frames, must give the same
//! transforms, `active` flags and component writes, bit for bit.

use crate::clip::{Clip, ClipId, World};
use crate::curve::{Curve, Keyframe};
use crate::legacy_pose::{Ctx, LegacyPose};
use crate::pose::{Pose, PropertySink};
use platypus_scene::{ObjectId, Scene, Transform};

/// xorshift64*: the same numbers on every run.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }

    fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }

    fn range(&mut self, a: f64, b: f64) -> f64 {
        a + (b - a) * self.unit()
    }
}

/// Lights on even-numbered objects; every write logged.
#[derive(Default)]
struct Sink(Vec<(ObjectId, String, String, u64)>);

impl PropertySink for Sink {
    fn has_component(&mut self, object: ObjectId, type_name: &str) -> bool {
        type_name == "Light" && object.index.is_multiple_of(2)
    }

    fn set_number(&mut self, object: ObjectId, type_name: &str, property: &str, value: f64) {
        self.0
            .push((object, type_name.into(), property.into(), value.to_bits()));
    }
}

const NAMES: [&str; 5] = ["Hips", "Spine", "Arm", "Hand", "Arm"];
const PROPS: [&str; 18] = [
    "localPosition.x",
    "localPosition.y",
    "localPosition.z",
    "localRotation.x",
    "localRotation.y",
    "localRotation.z",
    "localRotation.w",
    "localRotation.w",
    "localScale.x",
    "localScale.z",
    "localEulerAngles.y",
    "localEulerAngles.x",
    "localPosition.w",
    "localRotation.q",
    "localRotation.",
    "position.x",
    "localScale",
    "localRotation.xy",
];

/// Every object's path from `root`, by names.
fn paths(scene: &Scene, root: ObjectId) -> Vec<String> {
    let mut out = vec![String::new()];
    let mut stack = vec![(root, String::new())];
    while let Some((go, path)) = stack.pop() {
        for c in scene.children(go) {
            let p = if path.is_empty() {
                scene.name(*c).unwrap_or("").to_string()
            } else {
                format!("{path}/{}", scene.name(*c).unwrap_or(""))
            };
            out.push(p.clone());
            stack.push((*c, p));
        }
    }
    out
}

/// Keys at random times, or at `times` (as a glTF channel's x, y, z, w curves share them).
fn random_curve(rng: &mut Rng, times: Option<&[f64]>) -> Curve {
    let own: Vec<f64> = {
        let mut t = rng.range(-0.5, 0.5);
        (0..rng.below(5))
            .map(|_| {
                t += rng.range(0.0, 0.7);
                t
            })
            .collect()
    };
    let keys = times
        .unwrap_or(&own)
        .iter()
        .map(|&t| {
            let tangent = |rng: &mut Rng| {
                if rng.below(8) == 0 {
                    f64::INFINITY
                } else {
                    rng.range(-2.0, 2.0)
                }
            };
            Keyframe {
                time: t,
                value: rng.range(-1.0, 1.0),
                in_tangent: tangent(rng),
                out_tangent: tangent(rng),
            }
        })
        .collect();
    Curve::new(keys)
}

fn random_clip(rng: &mut Rng, world: &mut World, paths: &[String]) -> ClipId {
    let id = world.add_clip(Clip::new("random"));
    let shared: Vec<f64> = (0..rng.below(6)).map(|i| i as f64 * 0.3).collect();
    for _ in 0..rng.below(40) {
        let path = if rng.below(10) == 0 {
            "Nowhere/Arm".to_string()
        } else {
            paths[rng.below(paths.len())].clone()
        };
        let (type_name, property) = match rng.below(12) {
            0 => ("GameObject", "active"),
            1 => ("Light", "intensity"),
            2 => ("Light", "color.g"),
            3 => ("Camera", "fieldOfView"),
            _ => ("Transform", PROPS[rng.below(PROPS.len())]),
        };
        // Sometimes a curve that does not exist (yet).
        let curve = if rng.below(15) == 0 {
            crate::clip::CurveId(9_999_999)
        } else {
            let times = (rng.below(2) == 0).then_some(&shared[..]);
            let mut c = random_curve(rng, times);
            if rng.below(10) == 0 {
                c.post_wrap = crate::WrapMode::Loop;
            }
            world.add_curve(c)
        };
        // Pushed directly, so duplicate bindings for one property happen too.
        world
            .clip_mut(id)
            .expect("clip")
            .bindings
            .push(crate::Binding {
                path,
                type_name: type_name.into(),
                property: property.into(),
                curve,
            });
    }
    id
}

/// A random hierarchy under a root; the same calls on two scenes give the same ids.
fn build(scene: &mut Scene, seed: u64) -> ObjectId {
    let mut rng = Rng(seed);
    let root = scene.create("Root");
    let mut all = vec![root];
    for _ in 0..rng.below(12) + 1 {
        let parent = all[rng.below(all.len())];
        let go = scene.create(NAMES[rng.below(NAMES.len())]);
        scene.set_parent(go, Some(parent), false);
        let t = Transform {
            position: glam::Vec3::new(rng.unit() as f32, 0.5, -(rng.unit() as f32)),
            rotation: glam::Quat::from_rotation_y(rng.range(-3.0, 3.0) as f32),
            scale: glam::Vec3::splat(1.0 + rng.unit() as f32),
        };
        scene.set_local(go, t);
        all.push(go);
    }
    root
}

/// An edit between frames: a rename, a move, a new child or a destroyed object.
fn edit(scene: &mut Scene, root: ObjectId, rng: &mut Rng) {
    let all: Vec<ObjectId> = scene
        .walk()
        .filter(|o| scene.is_descendant(*o, root))
        .collect();
    let go = all[rng.below(all.len())];
    match rng.below(4) {
        0 => scene.set_name(go, NAMES[rng.below(NAMES.len())]),
        1 => {
            let to = all[rng.below(all.len())];
            scene.set_parent(go, Some(to), rng.below(2) == 0);
        }
        2 => {
            let c = scene.create(NAMES[rng.below(NAMES.len())]);
            scene.set_parent(c, Some(go), false);
        }
        _ if go != root => {
            scene.destroy(go);
        }
        _ => {}
    }
}

fn snapshot(scene: &Scene) -> Vec<(ObjectId, [u32; 10], bool)> {
    scene
        .walk()
        .map(|o| {
            let t = scene.local(o).expect("live");
            let b = |x: f32| x.to_bits();
            (
                o,
                [
                    b(t.position.x),
                    b(t.position.y),
                    b(t.position.z),
                    b(t.rotation.x),
                    b(t.rotation.y),
                    b(t.rotation.z),
                    b(t.rotation.w),
                    b(t.scale.x),
                    b(t.scale.y),
                    b(t.scale.z),
                ],
                scene.enabled(o),
            )
        })
        .collect()
}

// M5-ANI-08
#[test]
fn bound_poses_match_the_string_keyed_ones_bit_for_bit() {
    // One pose for every frame and hierarchy: `apply` empties it.
    let mut new = Pose::new();
    for seed in 1..=300u64 {
        let mut rng = Rng(seed.wrapping_mul(0x9e37_79b9_7f4a_7c15));
        let (mut old_scene, mut new_scene) = (Scene::new(), Scene::new());
        let root = build(&mut old_scene, seed);
        assert_eq!(build(&mut new_scene, seed), root);
        let mut world = World::default();
        let p = paths(&old_scene, root);
        let clips: Vec<ClipId> = (0..rng.below(4) + 1)
            .map(|_| random_clip(&mut rng, &mut world, &p))
            .collect();
        let (mut old_sink, mut new_sink) = (Sink::default(), Sink::default());
        for frame in 0..6 {
            if frame > 0 && rng.below(2) == 0 {
                let mut a = Rng(rng.next());
                let mut b = Rng(a.0);
                edit(&mut old_scene, root, &mut a);
                edit(&mut new_scene, root, &mut b);
            }
            let mut old = LegacyPose::default();
            for _ in 0..rng.below(4) + 1 {
                let clip = clips[rng.below(clips.len())];
                let time = rng.range(-1.0, 3.0);
                let weight = match rng.below(6) {
                    0 => 0.0,
                    1 => -0.5,
                    _ => rng.range(0.0, 1.0),
                };
                let aligned = rng.below(3) != 0;
                let cx = Ctx {
                    scene: &old_scene,
                    world: &world,
                    root,
                };
                if aligned {
                    old.add_aligned(&cx, &mut old_sink, clip, time, weight);
                    new.add_aligned(&new_scene, &world, &mut new_sink, root, clip, time, weight);
                } else {
                    old.add(&cx, &mut old_sink, clip, time, weight);
                    new.add(&new_scene, &world, &mut new_sink, root, clip, time, weight);
                }
            }
            old.apply(&mut old_scene, &mut old_sink);
            new.apply(&mut new_scene, &mut new_sink);
            assert_eq!(
                snapshot(&old_scene),
                snapshot(&new_scene),
                "seed {seed} frame {frame}"
            );
            assert_eq!(old_sink.0, new_sink.0, "seed {seed} frame {frame}");
            assert_eq!(old_scene.take_moved(), new_scene.take_moved());
        }
    }
}

// M5-ANI-08
#[test]
fn a_rename_under_the_root_rebinds_its_curves() {
    let mut scene = Scene::new();
    let root = scene.create("Root");
    let arm = scene.create("Arm");
    scene.set_parent(arm, Some(root), false);
    let mut world = World::default();
    let clip = world.add_clip(Clip::new("wave"));
    let curve = world.add_curve(Curve::constant(0.0, 1.0, 2.0));
    world.set_curve(clip, "Arm", "Transform", "localPosition.x", Some(curve));
    let x = |s: &Scene, o| s.local(o).unwrap().position.x;
    crate::pose::sample(
        &mut scene,
        &world,
        &mut crate::NoComponents,
        root,
        clip,
        0.5,
    );
    assert_eq!(x(&scene, arm), 2.0);
    // Renamed: the path no longer reaches it, and another object takes the name.
    scene.set_name(arm, "Leg");
    let other = scene.create("Arm");
    scene.set_parent(other, Some(root), false);
    crate::pose::sample(
        &mut scene,
        &world,
        &mut crate::NoComponents,
        root,
        clip,
        0.5,
    );
    assert_eq!(x(&scene, other), 2.0);
    // A curve edited after binding is seen.
    world.curve_mut(curve).unwrap().keys[0].value = 3.0;
    world.curve_mut(curve).unwrap().keys[1].value = 3.0;
    crate::pose::sample(
        &mut scene,
        &world,
        &mut crate::NoComponents,
        root,
        clip,
        0.5,
    );
    assert_eq!(x(&scene, other), 3.0);
    // The kept length follows edits, and a curve added for a binding that named it early.
    assert_eq!(world.clip_length(clip), 1.0);
    world
        .curve_mut(curve)
        .unwrap()
        .add_key(Keyframe::new(2.0, 0.0));
    assert_eq!(world.clip_length(clip), 2.0);
    let next = crate::clip::CurveId(world.curves.len() as u32 + 1);
    world.clip_mut(clip).unwrap().bindings.push(crate::Binding {
        path: String::new(),
        type_name: "Transform".into(),
        property: "localScale.y".into(),
        curve: next,
    });
    assert_eq!(world.clip_length(clip), 2.0);
    world.add_curve(Curve::constant(0.0, 4.0, 1.0));
    assert_eq!(world.clip_length(clip), 4.0);
}
