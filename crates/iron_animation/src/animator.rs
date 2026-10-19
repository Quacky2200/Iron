//! The `Animator` component (`js/engine/src/animation.ts`): runs an `AnimatorController`. Parameters
//! set from script move it between states, cross-fading; `play` and `cross_fade` jump directly.

use crate::clip::{ClipId, World};
use crate::controller::{ConditionMode, ControllerId, Motion, NodeId, Param, ParameterType};
use crate::curve::{WrapMode, wrap};
use crate::player::{FiredEvent, fire_events};
use crate::pose::{Pose, PropertySink};
use iron_math::jsnum;
use iron_scene::{ObjectId, Scene};

#[derive(Clone, Copy, Debug, PartialEq)]
struct Playing {
    node: NodeId,
    time: f64,
}

/// The state being faded to, and how far (0..1) over how long.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Next {
    node: NodeId,
    time: f64,
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
            self.current = Some(Playing { node, time: 0.0 });
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
        let length = self.length_of(world, node);
        self.current = Some(Playing {
            node,
            time: normalized_time * length,
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
        let length = self.length_of(world, node);
        self.next = Some(Next {
            node,
            time: normalized_time * length,
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
        self.info(world, p.map(|p| (p.node, p.time)))
    }

    pub fn next_info(&self, world: &World) -> StateInfo {
        self.info(world, self.next.map(|n| (n.node, n.time)))
    }

    fn info(&self, world: &World, p: Option<(NodeId, f64)>) -> StateInfo {
        let length = p.map_or(0.0, |(n, _)| self.length_of(world, n));
        StateInfo {
            name: p
                .and_then(|(n, _)| world.graphs.node(n))
                .map(|n| n.name.clone())
                .unwrap_or_default(),
            normalized_time: match p {
                Some((_, t)) if length > 0.0 => t / length,
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
        let length = self.length_of(world, from.node);
        let normalized = if length > 0.0 {
            from.time / length
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
            let start = t.offset.unwrap_or(0.0) * self.length_of(world, node);
            if duration <= 0.0 {
                self.current = Some(Playing { node, time: start });
            } else {
                self.next = Some(Next {
                    node,
                    time: start,
                    progress: 0.0,
                    duration,
                });
            }
            return;
        }
    }

    /// Advances one state by `dt` and adds its clips to the pose.
    #[allow(clippy::too_many_arguments)]
    fn run(
        &mut self,
        world: &World,
        scene: &Scene,
        sink: &mut dyn PropertySink,
        pose: &mut Pose,
        events: &mut Vec<FiredEvent>,
        playing: &mut (NodeId, f64),
        dt: f64,
        weight: f64,
    ) {
        let before = playing.1;
        playing.1 += dt * self.state_speed(world, playing.0);
        let length = self.length_of(world, playing.0);
        for c in self.clips(world, playing.0) {
            let clip_len = world.clip_length(c.clip);
            // Blended clips share the state normalized time.
            let t = if length > 0.0 {
                (playing.1 / length) * clip_len
            } else {
                0.0
            };
            let t0 = if length > 0.0 {
                (before / length) * clip_len
            } else {
                0.0
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
            pose.add_aligned(
                scene,
                world,
                sink,
                self.object,
                c.clip,
                wrap(t, clip_len, mode),
                c.weight * weight,
            );
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
        self.ensure(world);
        if self.current.is_none() {
            return;
        }
        self.check_transitions(world);
        let mut pose = Pose::new();
        let cur = self.current.expect("checked");
        let mut cur = (cur.node, cur.time);
        if let Some(mut next) = self.next {
            next.progress = jsnum::min(1.0, next.progress + dt / next.duration);
            let w = next.progress;
            let mut nx = (next.node, next.time);
            self.run(world, scene, sink, &mut pose, events, &mut cur, dt, 1.0 - w);
            self.run(world, scene, sink, &mut pose, events, &mut nx, dt, w);
            if w >= 1.0 {
                self.current = Some(Playing {
                    node: nx.0,
                    time: nx.1,
                });
                self.next = None;
            } else {
                self.current = Some(Playing {
                    node: cur.0,
                    time: cur.1,
                });
                self.next = Some(Next {
                    node: nx.0,
                    time: nx.1,
                    progress: w,
                    duration: next.duration,
                });
            }
        } else {
            self.run(world, scene, sink, &mut pose, events, &mut cur, dt, 1.0);
            self.current = Some(Playing {
                node: cur.0,
                time: cur.1,
            });
        }
        pose.apply(scene, sink);
    }
}
