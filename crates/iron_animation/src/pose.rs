//! Blended values for one frame (`Pose` in `js/engine/src/animation.ts`): per object, transform
//! channels as weighted sums (quaternions sign-aligned, then normalised), and other properties as
//! weighted averages. `apply` writes each transform once.
//!
//! Curves reach their objects through bindings resolved once per animated root (`bind.rs`): a
//! bone is a slot in a dense array, a channel a number, so a frame does no string work.

use crate::bind::{CHANNELS, EULER, NO_CHANNEL, POS, ROT, SCALE};
use crate::clip::{ClipId, World};
use crate::curve::Spot;
use glam::{Quat, Vec3};
use iron_math::{Q, V3};
use iron_scene::{ObjectId, Scene, Transform};
use std::rc::Rc;

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

/// One bone's sums: values and total weights per channel.
#[derive(Clone, Copy)]
struct BonePose {
    v: [f64; CHANNELS],
    w: [f64; CHANNELS],
}

impl BonePose {
    const ZERO: BonePose = BonePose {
        v: [0.0; CHANNELS],
        w: [0.0; CHANNELS],
    };

    /// Any weight on channels `from..from + n`?
    fn has(&self, from: usize, n: usize) -> bool {
        self.w[from..from + n].iter().any(|w| *w != 0.0)
    }

    /// The weighted average of channels `from..`, or `current` where nothing was added.
    fn avg<const N: usize>(&self, from: usize, current: [f64; N]) -> [f64; N] {
        let mut out = current;
        for (i, o) in out.iter_mut().enumerate() {
            let w = self.w[from + i];
            if w != 0.0 {
                *o = self.v[from + i] / w;
            }
        }
        out
    }
}

struct PropPose {
    object: ObjectId,
    key: u32,
    type_name: Rc<str>,
    property: Rc<str>,
    value: f64,
    weight: f64,
}

/// One frame of blended values, for one animated root. `apply` empties it for the next frame,
/// keeping its buffers (the players share one: `World`'s scratch pose).
#[derive(Default)]
pub struct Pose {
    root: Option<ObjectId>,
    /// By the root's bone slot.
    bones: Vec<BonePose>,
    touched: Vec<bool>,
    /// Bones in the order first added to, with their objects.
    order: Vec<(u32, ObjectId)>,
    props: Vec<PropPose>,
    /// Scratch for one clip: each track's value and key search, the bones flipped.
    values: Vec<Option<f64>>,
    spots: Vec<Spot>,
    flips: Vec<u32>,
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

impl Pose {
    pub fn new() -> Pose {
        Pose::default()
    }

    fn bone(&mut self, slot: u32, go: ObjectId) -> &mut BonePose {
        let s = slot as usize;
        if s >= self.bones.len() {
            self.bones.resize(s + 1, BonePose::ZERO);
            self.touched.resize(s + 1, false);
        }
        if !self.touched[s] {
            self.touched[s] = true;
            self.order.push((slot, go));
        }
        &mut self.bones[s]
    }

    /// Adds the clip at `time` with `weight`, each bone's quaternion flipped to the side of what
    /// is already summed when `align`.
    #[allow(clippy::too_many_arguments)]
    fn add_clip(
        &mut self,
        scene: &Scene,
        world: &World,
        sink: &mut dyn PropertySink,
        root: ObjectId,
        clip: ClipId,
        time: f64,
        weight: f64,
        align: bool,
    ) {
        if weight <= 0.0 {
            return;
        }
        debug_assert!(
            self.root.is_none_or(|r| r == root),
            "a Pose is for one animated root"
        );
        self.root = Some(root);
        world.with_bound(scene, root, clip, |rb, bc| {
            // Each curve sampled once; curves with the same key times share the key search.
            let mut values = std::mem::take(&mut self.values);
            let mut spots = std::mem::take(&mut self.spots);
            let mut flips = std::mem::take(&mut self.flips);
            values.clear();
            spots.clear();
            flips.clear();
            for (i, t) in bc.tracks.iter().enumerate() {
                let value = world.curve(t.curve).map(|c| {
                    let lead = t.lead as usize;
                    let spot = if lead == i {
                        c.locate(time)
                    } else {
                        spots[lead]
                    };
                    spots.push(spot);
                    c.at(spot)
                });
                if value.is_none() {
                    spots.push(Spot::default());
                }
                values.push(value);
            }
            // Which bones' quaternions go to the other side.
            if align {
                for r in &bc.rotations {
                    let sum = match self.bones.get(r.slot as usize) {
                        Some(b)
                            if self.touched[r.slot as usize]
                                && !b.v[ROT..ROT + 4].iter().all(|v| *v == 0.0) =>
                        {
                            [b.v[ROT], b.v[ROT + 1], b.v[ROT + 2], b.v[ROT + 3]]
                        }
                        _ => continue,
                    };
                    let mut dot = 0.0;
                    for (i, axis) in r.axes.iter().enumerate() {
                        let v = axis.and_then(|t| values[t as usize]).unwrap_or(0.0);
                        dot += v * sum[i];
                    }
                    if dot < 0.0 {
                        flips.push(r.slot);
                    }
                }
            }
            for (t, value) in bc.tracks.iter().zip(&values) {
                let Some(v) = *value else {
                    continue;
                };
                let sign = if t.rotation && flips.contains(&t.slot) {
                    -1.0
                } else {
                    1.0
                };
                let value = v * sign;
                let b = self.bone(t.slot, rb.bones[t.slot as usize]);
                if t.channel != NO_CHANNEL {
                    let c = usize::from(t.channel);
                    b.v[c] += value * weight;
                    b.w[c] += weight;
                }
            }
            for p in &bc.props {
                let Some(curve) = world.curve(p.curve) else {
                    continue;
                };
                let value = curve.evaluate(time);
                let (type_name, property) = &rb.prop_keys[p.key as usize];
                if !p.game_object && !sink.has_component(p.object, type_name) {
                    continue;
                }
                match self
                    .props
                    .iter_mut()
                    .find(|q| q.object == p.object && q.key == p.key)
                {
                    Some(q) => {
                        q.value += value * weight;
                        q.weight += weight;
                    }
                    None => self.props.push(PropPose {
                        object: p.object,
                        key: p.key,
                        type_name: type_name.clone(),
                        property: property.clone(),
                        value: value * weight,
                        weight,
                    }),
                }
            }
            self.values = values;
            self.spots = spots;
            self.flips = flips;
        });
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
        self.add_clip(scene, world, sink, root, clip, time, weight, false);
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
        self.add_clip(scene, world, sink, root, clip, time, weight, true);
    }

    /// Writes the blended values and empties the pose (keeping its buffers).
    pub fn apply(&mut self, scene: &mut Scene, sink: &mut dyn PropertySink) {
        self.write(scene, sink);
        for &(slot, _) in &self.order {
            self.bones[slot as usize] = BonePose::ZERO;
            self.touched[slot as usize] = false;
        }
        self.order.clear();
        self.props.clear();
        self.root = None;
    }

    fn write(&self, scene: &mut Scene, sink: &mut dyn PropertySink) {
        for &(slot, go) in &self.order {
            let t = &self.bones[slot as usize];
            let Some(cur) = scene.local(go) else {
                continue;
            };
            let cp = [
                f64::from(cur.position.x),
                f64::from(cur.position.y),
                f64::from(cur.position.z),
            ];
            let position = if t.has(POS, 3) { t.avg(POS, cp) } else { cp };
            let cs = [
                f64::from(cur.scale.x),
                f64::from(cur.scale.y),
                f64::from(cur.scale.z),
            ];
            let scale = if t.has(SCALE, 3) {
                t.avg(SCALE, cs)
            } else {
                cs
            };
            let mut rotation = Q {
                x: f64::from(cur.rotation.x),
                y: f64::from(cur.rotation.y),
                z: f64::from(cur.rotation.z),
                w: f64::from(cur.rotation.w),
            };
            if t.has(ROT, 4) {
                let q = t.avg(ROT, [rotation.x, rotation.y, rotation.z, rotation.w]);
                rotation = Q {
                    x: q[0],
                    y: q[1],
                    z: q[2],
                    w: q[3],
                }
                .normalized();
            } else if t.has(EULER, 3) {
                let e0: V3 = rotation.euler_angles();
                let e = t.avg(EULER, [e0.x, e0.y, e0.z]);
                rotation = Q::euler(e[0], e[1], e[2]);
            }
            let lap = crate::profile::start();
            scene.set_local(
                go,
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
            crate::profile::stop("animation: set_local", lap);
        }
        for p in &self.props {
            if !scene.contains(p.object) {
                continue;
            }
            let value = p.value / p.weight;
            if &*p.type_name == "GameObject" {
                if &*p.property == "active" {
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
