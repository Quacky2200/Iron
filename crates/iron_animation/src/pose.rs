//! Blended values for one frame (`Pose` in `js/engine/src/animation.ts`): per object, transform
//! channels as weighted sums (quaternions sign-aligned, then normalised), and other properties as
//! weighted averages. `apply` writes each transform once.

use crate::clip::{Binding, ClipId, World};
use glam::{Quat, Vec3};
use iron_math::{Q, V3};
use iron_scene::{ObjectId, Scene, Transform};

/// What the animation writes that is not a transform or an object's `active` flag: a number on a
/// component. The host (or the engine API) supplies it.
pub trait PropertySink {
    /// Does `object` have a component of this class (by name)?
    fn has_component(&mut self, object: ObjectId, type_name: &str) -> bool;
    /// Sets a number property (`intensity`) or one field of a value property (`color.r`).
    fn set_number(&mut self, object: ObjectId, type_name: &str, property: &str, value: f64);
}

/// A sink for programs with no animated components: nothing is found.
pub struct NoComponents;

impl PropertySink for NoComponents {
    fn has_component(&mut self, _: ObjectId, _: &str) -> bool {
        false
    }

    fn set_number(&mut self, _: ObjectId, _: &str, _: &str, _: f64) {}
}

#[derive(Default)]
struct TransformPose {
    p: [f64; 3],
    r: [f64; 4],
    s: [f64; 3],
    e: [f64; 3],
    /// Total weight per channel (`localPosition.x`), in the order first seen.
    w: Vec<(String, f64)>,
}

impl TransformPose {
    fn weight(&self, property: &str) -> f64 {
        self.w.iter().find(|e| e.0 == property).map_or(0.0, |e| e.1)
    }

    fn add_weight(&mut self, property: &str, weight: f64) {
        match self.w.iter_mut().find(|e| e.0 == property) {
            Some(e) => e.1 += weight,
            None => self.w.push((property.to_string(), weight)),
        }
    }
}

struct PropPose {
    object: ObjectId,
    type_name: String,
    property: String,
    value: f64,
    weight: f64,
}

/// One frame of blended values.
#[derive(Default)]
pub struct Pose {
    transforms: Vec<(ObjectId, TransformPose)>,
    props: Vec<PropPose>,
}

/// The object at a path ("" itself, "A/B" by child names) under `root`.
pub fn resolve(scene: &Scene, root: ObjectId, path: &str) -> Option<ObjectId> {
    if path.is_empty() {
        return Some(root);
    }
    let mut go = root;
    for name in path.split('/') {
        go = scene
            .children(go)
            .iter()
            .copied()
            .find(|c| scene.name(*c) == Some(name))?;
    }
    Some(go)
}

const AXES: &str = "xyzw";

impl Pose {
    pub fn new() -> Pose {
        Pose::default()
    }

    fn transform_of(&mut self, go: ObjectId) -> &mut TransformPose {
        match self.transforms.iter().position(|t| t.0 == go) {
            Some(i) => &mut self.transforms[i].1,
            None => {
                self.transforms.push((go, TransformPose::default()));
                &mut self.transforms.last_mut().expect("just pushed").1
            }
        }
    }

    /// Adds one binding at `time`, `sign` being -1 for a flipped quaternion channel.
    #[allow(clippy::too_many_arguments)]
    fn add_binding(
        &mut self,
        scene: &Scene,
        world: &World,
        sink: &mut dyn PropertySink,
        root: ObjectId,
        b: &Binding,
        time: f64,
        weight: f64,
        sign: f64,
    ) {
        let Some(go) = resolve(scene, root, &b.path) else {
            return;
        };
        let Some(curve) = world.curve(b.curve) else {
            return;
        };
        let value = curve.evaluate(time) * sign;
        if b.type_name == "Transform" {
            let t = self.transform_of(go);
            let Some((field, axis)) = b.property.split_once('.') else {
                return;
            };
            let Some(i) = AXES.find(axis).filter(|_| axis.len() == 1) else {
                return;
            };
            let slot = match (field, i) {
                ("localPosition", 0..=2) => &mut t.p[i],
                ("localRotation", 0..=3) => &mut t.r[i],
                ("localScale", 0..=2) => &mut t.s[i],
                ("localEulerAngles", 0..=2) => &mut t.e[i],
                _ => return,
            };
            *slot += value * weight;
            t.add_weight(&b.property, weight);
            return;
        }
        let present = if b.type_name == "GameObject" {
            true
        } else {
            sink.has_component(go, &b.type_name)
        };
        if !present {
            return;
        }
        match self
            .props
            .iter_mut()
            .find(|p| p.object == go && p.type_name == b.type_name && p.property == b.property)
        {
            Some(p) => {
                p.value += value * weight;
                p.weight += weight;
            }
            None => self.props.push(PropPose {
                object: go,
                type_name: b.type_name.clone(),
                property: b.property.clone(),
                value: value * weight,
                weight,
            }),
        }
    }

    /// Adds the clip at `time` with `weight`.
    #[allow(clippy::too_many_arguments)]
    pub fn add(
        &mut self,
        scene: &Scene,
        world: &World,
        sink: &mut dyn PropertySink,
        root: ObjectId,
        clip: ClipId,
        time: f64,
        weight: f64,
    ) {
        if weight <= 0.0 {
            return;
        }
        let Some(c) = world.clip(clip) else {
            return;
        };
        for b in &c.bindings {
            self.add_binding(scene, world, sink, root, b, time, weight, 1.0);
        }
    }

    /// Like `add`, but a clip's quaternion is flipped to the side of what is already summed
    /// (`q` and `-q` are the same turn), so blends do not cancel.
    #[allow(clippy::too_many_arguments)]
    pub fn add_aligned(
        &mut self,
        scene: &Scene,
        world: &World,
        sink: &mut dyn PropertySink,
        root: ObjectId,
        clip: ClipId,
        time: f64,
        weight: f64,
    ) {
        if weight <= 0.0 {
            return;
        }
        let Some(c) = world.clip(clip) else {
            return;
        };
        let mut flips: Vec<(ObjectId, f64)> = Vec::new();
        for b in &c.bindings {
            if b.type_name != "Transform" || !b.property.starts_with("localRotation.") {
                continue;
            }
            let Some(go) = resolve(scene, root, &b.path) else {
                continue;
            };
            if flips.iter().any(|f| f.0 == go) {
                continue;
            }
            let sum = match self.transforms.iter().find(|t| t.0 == go) {
                Some((_, t)) if !t.r.iter().all(|v| *v == 0.0) => t.r,
                _ => continue,
            };
            let mut dot = 0.0;
            for (i, a) in AXES.chars().enumerate() {
                let prop = format!("localRotation.{a}");
                let v = c
                    .bindings
                    .iter()
                    .find(|o| o.path == b.path && o.type_name == "Transform" && o.property == prop)
                    .and_then(|o| world.curve(o.curve))
                    .map_or(0.0, |cv| cv.evaluate(time));
                dot += v * sum[i];
            }
            flips.push((go, if dot < 0.0 { -1.0 } else { 1.0 }));
        }
        if flips.iter().all(|f| f.1 > 0.0) {
            self.add(scene, world, sink, root, clip, time, weight);
            return;
        }
        for b in &c.bindings {
            let flip = if b.type_name == "Transform" && b.property.starts_with("localRotation.") {
                resolve(scene, root, &b.path)
                    .and_then(|go| flips.iter().find(|f| f.0 == go))
                    .map_or(1.0, |f| f.1)
            } else {
                1.0
            };
            self.add_binding(scene, world, sink, root, b, time, weight, flip);
        }
    }

    /// Writes the blended values.
    pub fn apply(self, scene: &mut Scene, sink: &mut dyn PropertySink) {
        for (go, t) in &self.transforms {
            let Some(cur) = scene.local(*go) else {
                continue;
            };
            let has = |f: &str, n: usize| {
                (0..n).any(|i| t.weight(&format!("{f}.{}", AXES.as_bytes()[i] as char)) != 0.0)
            };
            let avg = |f: &str, list: &[f64], current: &[f64]| -> Vec<f64> {
                current
                    .iter()
                    .enumerate()
                    .map(|(i, c)| {
                        let w = t.weight(&format!("{f}.{}", AXES.as_bytes()[i] as char));
                        if w != 0.0 { list[i] / w } else { *c }
                    })
                    .collect()
            };
            let cp = V3 {
                x: f64::from(cur.position.x),
                y: f64::from(cur.position.y),
                z: f64::from(cur.position.z),
            };
            let position = if has("localPosition", 3) {
                avg("localPosition", &t.p, &[cp.x, cp.y, cp.z])
            } else {
                vec![cp.x, cp.y, cp.z]
            };
            let cs = [
                f64::from(cur.scale.x),
                f64::from(cur.scale.y),
                f64::from(cur.scale.z),
            ];
            let scale = if has("localScale", 3) {
                avg("localScale", &t.s, &cs)
            } else {
                cs.to_vec()
            };
            let mut rotation = Q {
                x: f64::from(cur.rotation.x),
                y: f64::from(cur.rotation.y),
                z: f64::from(cur.rotation.z),
                w: f64::from(cur.rotation.w),
            };
            if has("localRotation", 4) {
                let q = avg(
                    "localRotation",
                    &t.r,
                    &[rotation.x, rotation.y, rotation.z, rotation.w],
                );
                rotation = Q {
                    x: q[0],
                    y: q[1],
                    z: q[2],
                    w: q[3],
                }
                .normalized();
            } else if has("localEulerAngles", 3) {
                let e0 = rotation.euler_angles();
                let e = avg("localEulerAngles", &t.e, &[e0.x, e0.y, e0.z]);
                rotation = Q::euler(e[0], e[1], e[2]);
            }
            scene.set_local(
                *go,
                Transform {
                    position: Vec3::new(position[0] as f32, position[1] as f32, position[2] as f32),
                    rotation: Quat::from_xyzw(
                        rotation.x as f32,
                        rotation.y as f32,
                        rotation.z as f32,
                        rotation.w as f32,
                    ),
                    scale: Vec3::new(scale[0] as f32, scale[1] as f32, scale[2] as f32),
                },
            );
        }
        for p in &self.props {
            if !scene.contains(p.object) {
                continue;
            }
            let value = p.value / p.weight;
            if p.type_name == "GameObject" {
                if p.property == "active" {
                    scene.set_enabled(p.object, value > 0.5);
                }
                continue;
            }
            sink.set_number(p.object, &p.type_name, &p.property, value);
        }
    }
}

/// Poses `go` as the clip is at `time` (Unity's `SampleAnimation`).
pub fn sample(
    scene: &mut Scene,
    world: &World,
    sink: &mut dyn PropertySink,
    go: ObjectId,
    clip: ClipId,
    time: f64,
) {
    let t = world.sample_time(clip, time);
    let mut pose = Pose::new();
    pose.add(scene, world, sink, go, clip, t, 1.0);
    pose.apply(scene, sink);
}
