// SPDX-License-Identifier: Apache-2.0

//! The `Animator` component (`js/engine/src/animation.ts`): runs an `AnimatorController`. Parameters
//! set from script move it between states, cross-fading; `play` and `cross_fade` jump directly.

use crate::clip::{ClipId, World};
use crate::controller::{ConditionMode, ControllerId, Motion, NodeId, Param, ParameterType};
use crate::culling::CullingMode;
use crate::curve::{WrapMode, wrap};
use crate::player::{FiredEvent, fire_events};
use crate::pose::{Pose, PropertySink};
use platypus_math::jsnum;
use platypus_scene::{ObjectId, Scene};

#[derive(Clone, Copy, Debug, PartialEq)]
struct Playing {
    node: NodeId,
    /// How far through the state, over its length (past 1 after loops), as Unity keeps it: a
    /// blend tree's length changes with its weights, so seconds would jump about in its cycle.
    norm: f64,
}

/// The state being faded to, and how far (0..1) over how long.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Next {
    node: NodeId,
    norm: f64,
    progress: f64,
    duration: f64,
}

/// Where an animator is in a state (Unity's `AnimatorStateInfo`).
#[derive(Clone, Debug, PartialEq)]
pub struct StateInfo {
    pub name: String,
    /// Time over the state length (past 1 after loops).
    pub normalized_time: f64,
    /// The state length in seconds.
    pub length: f64,
}

#[derive(Clone, Debug)]
pub struct AnimatorPlayer {
    pub object: ObjectId,
    pub controller: Option<ControllerId>,
    /// Multiplies every state speed.
    pub speed: f64,
    /// What it does while not seen.
    pub culling_mode: CullingMode,
    /// Off: the frame's update skips it (Unity's `enabled = false`); `Animator.update(dt)` still
    /// runs it by hand.
    pub enabled: bool,
    values: Vec<(String, Param)>,
    current: Option<Playing>,
    next: Option<Next>,
}

/// A clip of the state being played with its weight.
struct Weighted {
    clip: ClipId,
    weight: f64,
    time_scale: f64,
}

impl AnimatorPlayer {
    pub fn new(object: ObjectId) -> AnimatorPlayer {
        AnimatorPlayer {
            object,
            controller: None,
            speed: 1.0,
            culling_mode: CullingMode::AlwaysAnimate,
            enabled: true,
            values: Vec::new(),
            current: None,
            next: None,
        }
    }

    fn set_value(&mut self, name: &str, v: Param) {
        match self.values.iter_mut().find(|e| e.0 == name) {
            Some(e) => e.1 = v,
            None => self.values.push((name.to_string(), v)),
        }
    }

    fn value(&self, name: &str) -> Option<Param> {
        self.values.iter().find(|e| e.0 == name).map(|e| e.1)
    }

    fn ensure(&mut self, world: &World) {
        let Some(cid) = self.controller else {
            return;
        };
        if self.current.is_some() {
            return;
        }
        let g = &world.graphs;
        let Some(c) = g.controller(cid) else {
            return;
        };
        for pid in &c.parameters {
            let Some(p) = g.parameter(*pid) else {
                continue;
            };
            if self.value(&p.name).is_none() {
                let default = p.default.unwrap_or(match p.ty {
                    ParameterType::Bool | ParameterType::Trigger => Param::Bool(false),
                    _ => Param::Num(0.0),
                });
                self.set_value(&p.name, default);
            }
        }
        let name = c.default_state.clone().or_else(|| {
            c.nodes
                .first()
                .and_then(|n| g.node(*n))
                .map(|n| n.name.clone())
        });
        if let Some(name) = name
            && let Some(node) = g.find_node(c, &name)
        {
            self.current = Some(Playing { node, norm: 0.0 });
        }
    }

    pub fn set_float(&mut self, name: &str, v: f64) {
        self.set_value(name, Param::Num(v));
    }

    pub fn get_float(&mut self, world: &World, name: &str) -> f64 {
        self.ensure(world);
        self.value(name).map_or(0.0, Param::number)
    }

    pub fn set_integer(&mut self, name: &str, v: f64) {
        self.set_value(name, Param::Num(v.trunc()));
    }

    pub fn get_integer(&mut self, world: &World, name: &str) -> f64 {
        self.get_float(world, name).trunc()
    }

    pub fn set_bool(&mut self, name: &str, v: bool) {
        self.set_value(name, Param::Bool(v));
    }

    pub fn get_bool(&mut self, world: &World, name: &str) -> bool {
        self.ensure(world);
        self.value(name) == Some(Param::Bool(true))
    }

    /// On until a transition uses it.
    pub fn set_trigger(&mut self, name: &str) {
        self.set_value(name, Param::Bool(true));
    }

    pub fn reset_trigger(&mut self, name: &str) {
        self.set_value(name, Param::Bool(false));
    }

    fn node_named(&self, world: &World, name: &str) -> Option<NodeId> {
        let g = &world.graphs;
        g.find_node(g.controller(self.controller?)?, name)
    }

    /// Jumps to a state (no fade), at a normalized time.
    pub fn play(&mut self, world: &World, name: &str, normalized_time: f64) {
        self.ensure(world);
        let Some(node) = self.node_named(world, name) else {
            return;
        };
        self.current = Some(Playing {
            node,
            norm: normalized_time,
        });
        self.next = None;
    }

    /// Fades to a state over `duration` seconds.
    pub fn cross_fade(&mut self, world: &World, name: &str, duration: f64, normalized_time: f64) {
        self.ensure(world);
        let node = self.node_named(world, name);
        let Some(node) = node.filter(|_| self.current.is_some()) else {
            if node.is_some() {
                self.play(world, name, normalized_time);
            }
            return;
        };
        self.next = Some(Next {
            node,
            norm: normalized_time,
            progress: 0.0,
            duration: jsnum::max(1e-4, duration),
        });
    }

    pub fn is_in_transition(&self) -> bool {
        self.next.is_some()
    }

    pub fn current_info(&mut self, world: &World) -> StateInfo {
        self.ensure(world);
        let p = self.current;
        self.info(world, p.map(|p| (p.node, p.norm)))
    }

    pub fn next_info(&self, world: &World) -> StateInfo {
        self.info(world, self.next.map(|n| (n.node, n.norm)))
    }

    fn info(&self, world: &World, p: Option<(NodeId, f64)>) -> StateInfo {
        let length = p.map_or(0.0, |(n, _)| self.length_of(world, n));
        StateInfo {
            name: p
                .and_then(|(n, _)| world.graphs.node(n))
                .map(|n| n.name.clone())
                .unwrap_or_default(),
            normalized_time: match p {
                Some((_, norm)) if length > 0.0 => norm,
                _ => 0.0,
            },
            length,
        }
    }

    /// The clips of a state with their weights (a blend tree has its two neighbours).
    fn clips(&self, world: &World, node: NodeId) -> Vec<Weighted> {
        let g = &world.graphs;
        let Some(n) = g.node(node) else {
            return Vec::new();
        };
        match n.motion {
            Motion::Clip(clip) => vec![Weighted {
                clip,
                weight: 1.0,
                time_scale: 1.0,
            }],
            Motion::Blend(b) => {
                let Some(tree) = g.blend(b) else {
                    return Vec::new();
                };
                let mut children = tree.children.clone();
                children.sort_by(|a, b| {
                    (a.threshold - b.threshold)
                        .partial_cmp(&0.0)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
                let (Some(first), Some(last)) = (children.first(), children.last()) else {
                    return Vec::new();
                };
                let v = self.value(&tree.parameter).map_or(0.0, Param::number);
                let w = |c: &crate::controller::BlendChild, weight: f64| Weighted {
                    clip: c.clip,
                    weight,
                    time_scale: c.time_scale.unwrap_or(1.0),
                };
                if v <= first.threshold {
                    return vec![w(first, 1.0)];
                }
                if v >= last.threshold {
                    return vec![w(last, 1.0)];
                }
                let i = children
                    .iter()
                    .position(|c| c.threshold > v)
                    .unwrap_or(children.len() - 1);
                let (a, b) = (&children[i - 1], &children[i]);
                let t = (v - a.threshold) / (b.threshold - a.threshold);
                vec![w(a, 1.0 - t), w(b, t)]
            }
        }
    }

    /// A state length: its clip, or the weighted lengths of its blend.
    fn length_of(&self, world: &World, node: NodeId) -> f64 {
        self.clips(world, node).iter().fold(0.0, |l, c| {
            l + c.weight * world.clip_length(c.clip) / c.time_scale
        })
    }

    fn state_speed(&mut self, world: &World, node: NodeId) -> f64 {
        let g = &world.graphs;
        let (speed, param) = g
            .node(node)
            .map_or((None, None), |n| (n.speed, n.speed_parameter.clone()));
        let factor = match param.filter(|p| !p.is_empty()) {
            Some(p) => self.get_float(world, &p),
            None => 1.0,
        };
        speed.unwrap_or(1.0) * factor * self.speed
    }

    fn passes(&self, world: &World, cond: crate::controller::ConditionId) -> bool {
        let Some(c) = world.graphs.condition(cond) else {
            return false;
        };
        let v = self.value(&c.parameter);
        let n = v.map_or(0.0, Param::number);
        let t = c.threshold.unwrap_or(0.0);
        match c.mode {
            ConditionMode::If => v == Some(Param::Bool(true)),
            ConditionMode::IfNot => v != Some(Param::Bool(true)),
            ConditionMode::Greater => n > t,
            ConditionMode::Less => n < t,
            ConditionMode::Equals => n == t,
            ConditionMode::NotEqual => n != t,
        }
    }

    /// Starts the first transition out of the current state whose conditions hold.
    fn check_transitions(&mut self, world: &World) {
        let (Some(cid), Some(from), None) = (self.controller, self.current, self.next) else {
            return;
        };
        let g = &world.graphs;
        let Some(c) = g.controller(cid) else {
            return;
        };
        let from_name = g
            .node(from.node)
            .map(|n| n.name.clone())
            .unwrap_or_default();
        let normalized = if self.length_of(world, from.node) > 0.0 {
            from.norm
        } else {
            1.0
        };
        for tid in &c.transitions {
            let Some(t) = g.transition(*tid) else {
                continue;
            };
            if t.from != from_name && !(t.from == "Any" && t.to != from_name) {
                continue;
            }
            if t.has_exit_time && normalized < t.exit_time.unwrap_or(1.0) {
                continue;
            }
            if !t.has_exit_time && t.conditions.is_empty() {
                continue;
            }
            if !t.conditions.iter().all(|cd| self.passes(world, *cd)) {
                continue;
            }
            // Triggers are used up.
            for cd in &t.conditions {
                let Some(cond) = g.condition(*cd) else {
                    continue;
                };
                let is_trigger = c
                    .parameters
                    .iter()
                    .filter_map(|p| g.parameter(*p))
                    .find(|p| p.name == cond.parameter)
                    .is_some_and(|p| p.ty == ParameterType::Trigger);
                if is_trigger {
                    self.set_value(&cond.parameter, Param::Bool(false));
                }
            }
            let Some(node) = g.find_node(c, &t.to) else {
                continue;
            };
            let duration = t.duration.unwrap_or(0.25);
            let start = t.offset.unwrap_or(0.0);
            if duration <= 0.0 {
                self.current = Some(Playing { node, norm: start });
            } else {
                self.next = Some(Next {
                    node,
                    norm: start,
                    progress: 0.0,
                    duration,
                });
            }
            return;
        }
    }

    /// Advances one state by `dt` and adds its clips to the pose (none: culled, only advanced).
    #[allow(clippy::too_many_arguments)]
    fn run(
        &mut self,
        world: &World,
        scene: &Scene,
        sink: &mut dyn PropertySink,
        mut pose: Option<&mut Pose>,
        events: &mut Vec<FiredEvent>,
        playing: &mut (NodeId, f64),
        dt: f64,
        weight: f64,
    ) {
        let before = playing.1;
        let length = self.length_of(world, playing.0);
        if length > 0.0 {
            playing.1 += dt * self.state_speed(world, playing.0) / length;
        }
        for c in self.clips(world, playing.0) {
            let clip_len = world.clip_length(c.clip);
            // Blended clips share the state's normalized time, so their steps stay in phase.
            let (t, t0) = if length > 0.0 {
                (playing.1 * clip_len, before * clip_len)
            } else {
                (0.0, 0.0)
            };
            let wm = world.clip(c.clip).map_or(WrapMode::Once, |k| k.wrap_mode);
            let mode = if wm == WrapMode::Once {
                WrapMode::ClampForever
            } else {
                wm
            };
            if c.weight * weight >= 0.5 {
                fire_events(world, self.object, c.clip, t0, t, clip_len, mode, events);
            }
            let Some(pose) = pose.as_deref_mut() else {
                continue;
            };
            let lap = crate::profile::start();
            pose.add_aligned(
                scene,
                world,
                sink,
                self.object,
                c.clip,
                wrap(t, clip_len, mode),
                c.weight * weight,
            );
            crate::profile::stop("animation: pose add", lap);
        }
    }

    /// Advances by `dt` and poses the object.
    pub fn animate(
        &mut self,
        world: &World,
        scene: &mut Scene,
        sink: &mut dyn PropertySink,
        dt: f64,
        events: &mut Vec<FiredEvent>,
    ) {
        self.animate_seen(world, scene, sink, dt, events, true);
    }

    /// [`Self::animate`] for an object that was (or was not) seen last frame: unseen, its
    /// `culling_mode` decides whether it advances without posing or stops altogether.
    pub fn animate_seen(
        &mut self,
        world: &World,
        scene: &mut Scene,
        sink: &mut dyn PropertySink,
        dt: f64,
        events: &mut Vec<FiredEvent>,
        seen: bool,
    ) {
        let mut pose = std::mem::take(&mut *world.scratch.lock().expect("not poisoned"));
        if self.step(world, scene, sink, dt, events, seen, &mut pose) {
            let lap = crate::profile::start();
            pose.apply(scene, sink);
            crate::profile::stop("animation: pose apply", lap);
        }
        *world.scratch.lock().expect("not poisoned") = pose;
    }

    /// Advances by `dt` and adds the pose to `pose` (empty), reading the scene only: true when
    /// it is to be written ([`Self::animate_seen`] writes it at once; the parallel update later,
    /// in order).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn step(
        &mut self,
        world: &World,
        scene: &Scene,
        sink: &mut dyn PropertySink,
        dt: f64,
        events: &mut Vec<FiredEvent>,
        seen: bool,
        pose: &mut Pose,
    ) -> bool {
        let write = seen || self.culling_mode == CullingMode::AlwaysAnimate;
        if !write && self.culling_mode == CullingMode::CullCompletely {
            return false;
        }
        self.ensure(world);
        if self.current.is_none() {
            return false;
        }
        self.check_transitions(world);
        let cur = self.current.expect("checked");
        let mut cur = (cur.node, cur.norm);
        let mut target = write.then_some(pose);
        if let Some(mut next) = self.next {
            next.progress = jsnum::min(1.0, next.progress + dt / next.duration);
            let w = next.progress;
            let mut nx = (next.node, next.norm);
            let p = target.as_deref_mut();
            self.run(world, scene, sink, p, events, &mut cur, dt, 1.0 - w);
            let p = target.as_deref_mut();
            self.run(world, scene, sink, p, events, &mut nx, dt, w);
            if w >= 1.0 {
                self.current = Some(Playing {
                    node: nx.0,
                    norm: nx.1,
                });
                self.next = None;
            } else {
                self.current = Some(Playing {
                    node: cur.0,
                    norm: cur.1,
                });
                self.next = Some(Next {
                    node: nx.0,
                    norm: nx.1,
                    progress: w,
                    duration: next.duration,
                });
            }
        } else {
            self.run(world, scene, sink, target, events, &mut cur, dt, 1.0);
            self.current = Some(Playing {
                node: cur.0,
                norm: cur.1,
            });
        }
        write
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clip::Clip;
    use crate::controller::{BlendChild, BlendTree, Controller, Node};
    use crate::curve::Curve;
    use crate::pose::NoComponents;

    /// A looping clip moving x from 0 to 1 over `length` seconds.
    fn slide(w: &mut World, length: f64) -> ClipId {
        let mut c = Clip::new("slide");
        c.wrap_mode = WrapMode::Loop;
        let id = w.add_clip(c);
        let curve = w.add_curve(Curve::linear(0.0, 0.0, length, 1.0));
        w.set_curve(id, "", "Transform", "localPosition.x", Some(curve));
        id
    }

    // M5-ANI-07
    #[test]
    fn a_blend_tree_keeps_its_phase_when_its_weights_change() {
        let mut w = World::default();
        let (short, long) = (slide(&mut w, 1.0), slide(&mut w, 2.0));
        let blend = w.graphs.add_blend(BlendTree {
            parameter: "speed".into(),
            children: vec![
                BlendChild {
                    clip: short,
                    threshold: 0.0,
                    time_scale: None,
                },
                BlendChild {
                    clip: long,
                    threshold: 1.0,
                    time_scale: None,
                },
            ],
        });
        let node = w.graphs.add_node(Node {
            name: "Move".into(),
            motion: Motion::Blend(blend),
            speed: None,
            speed_parameter: None,
        });
        let controller = w.graphs.add_controller(Controller {
            nodes: vec![node],
            ..Default::default()
        });
        let mut scene = Scene::new();
        let go = scene.create("Runner");
        let mut a = AnimatorPlayer::new(go);
        a.controller = Some(controller);
        let mut events = Vec::new();
        let x = |scene: &Scene| f64::from(scene.local(go).unwrap().position.x);
        // A quarter of the way through the short clip.
        a.animate(&w, &mut scene, &mut NoComponents, 0.25, &mut events);
        assert!((x(&scene) - 0.25).abs() < 1e-6);
        // Switched to the long clip: still a quarter of the way through (not 0.125).
        a.set_float("speed", 1.0);
        a.animate(&w, &mut scene, &mut NoComponents, 0.0, &mut events);
        assert!((x(&scene) - 0.25).abs() < 1e-6, "{}", x(&scene));
        assert!((a.current_info(&w).normalized_time - 0.25).abs() < 1e-9);
        // Then on at the blend's pace: half a second of a 2 s cycle is another quarter.
        a.animate(&w, &mut scene, &mut NoComponents, 0.5, &mut events);
        assert!((x(&scene) - 0.5).abs() < 1e-6, "{}", x(&scene));
    }
}
