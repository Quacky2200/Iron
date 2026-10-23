// SPDX-License-Identifier: Apache-2.0

//! The string-keyed `Pose` from before bindings were cached (`bind.rs`): every frame it walked each
//! curve's path and compared property names. Kept for tests only, as the oracle the bound `Pose`
//! must match bit for bit.

use crate::clip::{Binding, ClipId, World};
use crate::pose::{PropertySink, resolve};
use glam::{Quat, Vec3};
use iron_math::{Q, V3};
use iron_scene::{ObjectId, Scene, Transform};

#[derive(Default)]
struct TransformPose {
    p: [f64; 3],
    r: [f64; 4],
    s: [f64; 3],
    e: [f64; 3],
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

#[derive(Default)]
pub(crate) struct LegacyPose {
    transforms: Vec<(ObjectId, TransformPose)>,
    props: Vec<PropPose>,
}

const AXES: &str = "xyzw";

/// What a pose call needs besides the clip.
pub(crate) struct Ctx<'a> {
    pub scene: &'a Scene,
    pub world: &'a World,
    pub root: ObjectId,
}

impl LegacyPose {
    fn transform_of(&mut self, go: ObjectId) -> &mut TransformPose {
        match self.transforms.iter().position(|t| t.0 == go) {
            Some(i) => &mut self.transforms[i].1,
            None => {
                self.transforms.push((go, TransformPose::default()));
                &mut self.transforms.last_mut().expect("just pushed").1
            }
        }
    }

    fn add_binding(
        &mut self,
        cx: &Ctx,
        sink: &mut dyn PropertySink,
        b: &Binding,
        time: f64,
        weight: f64,
        sign: f64,
    ) {
        let Some(go) = resolve(cx.scene, cx.root, &b.path) else {
            return;
        };
        let Some(curve) = cx.world.curve(b.curve) else {
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

    pub(crate) fn add(
        &mut self,
        cx: &Ctx,
        sink: &mut dyn PropertySink,
        clip: ClipId,
        time: f64,
        weight: f64,
    ) {
        if weight <= 0.0 {
            return;
        }
        let Some(c) = cx.world.clip(clip) else {
            return;
        };
        for b in &c.bindings {
            self.add_binding(cx, sink, b, time, weight, 1.0);
        }
    }

    pub(crate) fn add_aligned(
        &mut self,
        cx: &Ctx,
        sink: &mut dyn PropertySink,
        clip: ClipId,
        time: f64,
        weight: f64,
    ) {
        if weight <= 0.0 {
            return;
        }
        let Some(c) = cx.world.clip(clip) else {
            return;
        };
        let mut flips: Vec<(ObjectId, f64)> = Vec::new();
        for b in &c.bindings {
            if b.type_name != "Transform" || !b.property.starts_with("localRotation.") {
                continue;
            }
            let Some(go) = resolve(cx.scene, cx.root, &b.path) else {
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
                    .and_then(|o| cx.world.curve(o.curve))
                    .map_or(0.0, |cv| cv.evaluate(time));
                dot += v * sum[i];
            }
            flips.push((go, if dot < 0.0 { -1.0 } else { 1.0 }));
        }
        if flips.iter().all(|f| f.1 > 0.0) {
            self.add(cx, sink, clip, time, weight);
            return;
        }
        for b in &c.bindings {
            let flip = if b.type_name == "Transform" && b.property.starts_with("localRotation.") {
                resolve(cx.scene, cx.root, &b.path)
                    .and_then(|go| flips.iter().find(|f| f.0 == go))
                    .map_or(1.0, |f| f.1)
            } else {
                1.0
            };
            self.add_binding(cx, sink, b, time, weight, flip);
        }
    }

    pub(crate) fn apply(self, scene: &mut Scene, sink: &mut dyn PropertySink) {
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
