// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

//! The data of an `AnimatorController` (`js/engine/src/animation.ts`, one layer): parameters, states
//! that play a clip or a 1D blend tree, and transitions on conditions or exit time. Every piece is
//! an object of its own in the arenas, so a script can keep a handle to it and change it after it
//! was added (the TypeScript objects were shared the same way).

use crate::clip::{ClipId, World};
use serde_json::{Value, json};

macro_rules! id {
    ($(#[$m:meta])* $name:ident) => {
        $(#[$m])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub struct $name(pub u32);
    };
}

id!(
    /// An index plus one into the controllers.
    ControllerId
);
id!(ParameterId);
id!(ConditionId);
id!(TransitionId);
id!(NodeId);
id!(BlendId);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParameterType {
    Float,
    Int,
    Bool,
    Trigger,
}

impl ParameterType {
    pub fn name(self) -> &'static str {
        match self {
            ParameterType::Float => "float",
            ParameterType::Int => "int",
            ParameterType::Bool => "bool",
            ParameterType::Trigger => "trigger",
        }
    }

    pub fn parse(s: &str) -> Option<ParameterType> {
        Some(match s {
            "float" => ParameterType::Float,
            "int" => ParameterType::Int,
            "bool" => ParameterType::Bool,
            "trigger" => ParameterType::Trigger,
            _ => return None,
        })
    }
}

/// The value of a parameter: a number, or a flag.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Param {
    Num(f64),
    Bool(bool),
}

impl Param {
    pub fn number(self) -> f64 {
        match self {
            Param::Num(n) => n,
            Param::Bool(b) => f64::from(u8::from(b)),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AnimParameter {
    pub name: String,
    pub ty: ParameterType,
    pub default: Option<Param>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConditionMode {
    If,
    IfNot,
    Greater,
    Less,
    Equals,
    NotEqual,
}

impl ConditionMode {
    pub fn name(self) -> &'static str {
        match self {
            ConditionMode::If => "if",
            ConditionMode::IfNot => "ifNot",
            ConditionMode::Greater => "greater",
            ConditionMode::Less => "less",
            ConditionMode::Equals => "equals",
            ConditionMode::NotEqual => "notEqual",
        }
    }

    pub fn parse(s: &str) -> Option<ConditionMode> {
        Some(match s {
            "if" => ConditionMode::If,
            "ifNot" => ConditionMode::IfNot,
            "greater" => ConditionMode::Greater,
            "less" => ConditionMode::Less,
            "equals" => ConditionMode::Equals,
            "notEqual" => ConditionMode::NotEqual,
            _ => return None,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Condition {
    pub parameter: String,
    pub mode: ConditionMode,
    pub threshold: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Transition {
    /// A state name, or `Any` (from any state, except into itself).
    pub from: String,
    pub to: String,
    pub conditions: Vec<ConditionId>,
    /// Only once the source state normalized time passes `exit_time`.
    pub has_exit_time: bool,
    pub exit_time: Option<f64>,
    /// The cross-fade, in seconds.
    pub duration: Option<f64>,
    /// Where the destination starts, normalized.
    pub offset: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BlendChild {
    pub clip: ClipId,
    pub threshold: f64,
    pub time_scale: Option<f64>,
}

/// A 1D blend tree: clips placed along a parameter, the two around its value blended.
#[derive(Clone, Debug, PartialEq)]
pub struct BlendTree {
    pub parameter: String,
    pub children: Vec<BlendChild>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Motion {
    Clip(ClipId),
    Blend(BlendId),
}

/// A state of a controller (`AnimationNode`).
#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    pub name: String,
    pub motion: Motion,
    pub speed: Option<f64>,
    /// A float parameter multiplying `speed`.
    pub speed_parameter: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Controller {
    pub parameters: Vec<ParameterId>,
    pub nodes: Vec<NodeId>,
    pub transitions: Vec<TransitionId>,
    /// The first state name when not set.
    pub default_state: Option<String>,
}

/// The arenas of the controller pieces.
#[derive(Default)]
pub struct Graphs {
    pub controllers: Vec<Controller>,
    pub parameters: Vec<AnimParameter>,
    pub conditions: Vec<Condition>,
    pub transitions: Vec<Transition>,
    pub nodes: Vec<Node>,
    pub blends: Vec<BlendTree>,
}

macro_rules! arena {
    ($add:ident, $get:ident, $get_mut:ident, $field:ident, $id:ident, $ty:ty) => {
        pub fn $add(&mut self, v: $ty) -> $id {
            self.$field.push(v);
            $id(self.$field.len() as u32)
        }
        pub fn $get(&self, id: $id) -> Option<&$ty> {
            self.$field.get((id.0 as usize).wrapping_sub(1))
        }
        pub fn $get_mut(&mut self, id: $id) -> Option<&mut $ty> {
            self.$field.get_mut((id.0 as usize).wrapping_sub(1))
        }
    };
}

impl Graphs {
    arena!(
        add_controller,
        controller,
        controller_mut,
        controllers,
        ControllerId,
        Controller
    );
    arena!(
        add_parameter,
        parameter,
        parameter_mut,
        parameters,
        ParameterId,
        AnimParameter
    );
    arena!(
        add_condition,
        condition,
        condition_mut,
        conditions,
        ConditionId,
        Condition
    );
    arena!(
        add_transition,
        transition,
        transition_mut,
        transitions,
        TransitionId,
        Transition
    );
    arena!(add_node, node, node_mut, nodes, NodeId, Node);
    arena!(add_blend, blend, blend_mut, blends, BlendId, BlendTree);

    /// The node of a controller by name.
    pub fn find_node(&self, c: &Controller, name: &str) -> Option<NodeId> {
        c.nodes
            .iter()
            .copied()
            .find(|n| self.node(*n).is_some_and(|n| n.name == name))
    }
}

impl World {
    /// The controller as plain data (clips inline), for files and scenes.
    pub fn controller_to_json(&self, id: ControllerId) -> Value {
        let g = &self.graphs;
        let Some(c) = g.controller(id) else {
            return Value::Null;
        };
        let num = |p: Option<crate::controller::Param>| match p {
            Some(Param::Num(n)) => json!(n),
            Some(Param::Bool(b)) => json!(b),
            None => Value::Null,
        };
        json!({
            "parameters": c.parameters.iter().filter_map(|p| g.parameter(*p)).map(|p| {
                let mut o = json!({ "name": p.name, "type": p.ty.name() });
                if p.default.is_some() { o["defaultValue"] = num(p.default); }
                o
            }).collect::<Vec<_>>(),
            "states": c.nodes.iter().filter_map(|n| g.node(*n)).map(|n| {
                let motion = match n.motion {
                    Motion::Clip(clip) => json!({ "clip": self.clip_to_json(clip) }),
                    Motion::Blend(b) => {
                        let t = g.blend(b);
                        json!({
                            "blendParameter": t.map(|t| t.parameter.clone()).unwrap_or_default(),
                            "children": t.map(|t| t.children.iter().map(|c| {
                                let mut o = json!({ "motion": self.clip_to_json(c.clip), "threshold": c.threshold });
                                if let Some(ts) = c.time_scale { o["timeScale"] = json!(ts); }
                                o
                            }).collect::<Vec<_>>()).unwrap_or_default(),
                        })
                    }
                };
                let mut o = json!({ "name": n.name, "motion": motion });
                if let Some(s) = n.speed { o["speed"] = json!(s); }
                if let Some(s) = &n.speed_parameter { o["speedParameter"] = json!(s); }
                o
            }).collect::<Vec<_>>(),
            "transitions": c.transitions.iter().filter_map(|t| g.transition(*t)).map(|t| {
                let mut o = json!({ "from": t.from, "to": t.to });
                if !t.conditions.is_empty() {
                    o["conditions"] = json!(t.conditions.iter().filter_map(|c| g.condition(*c)).map(|c| {
                        let mut o = json!({ "parameter": c.parameter, "mode": c.mode.name() });
                        if let Some(th) = c.threshold { o["threshold"] = json!(th); }
                        o
                    }).collect::<Vec<_>>());
                }
                if t.has_exit_time { o["hasExitTime"] = json!(true); }
                if let Some(x) = t.exit_time { o["exitTime"] = json!(x); }
                if let Some(x) = t.duration { o["duration"] = json!(x); }
                if let Some(x) = t.offset { o["offset"] = json!(x); }
                o
            }).collect::<Vec<_>>(),
            "defaultState": c.default_state.as_ref().map_or(Value::Null, |s| json!(s)),
        })
    }

    pub fn controller_from_json(&mut self, data: &Value) -> ControllerId {
        let mut c = Controller::default();
        for p in data["parameters"].as_array().into_iter().flatten() {
            let ty = p["type"]
                .as_str()
                .and_then(ParameterType::parse)
                .unwrap_or(ParameterType::Float);
            let default = match &p["defaultValue"] {
                Value::Number(n) => n.as_f64().map(Param::Num),
                Value::Bool(b) => Some(Param::Bool(*b)),
                _ => None,
            };
            let id = self.graphs.add_parameter(AnimParameter {
                name: p["name"].as_str().unwrap_or("").to_string(),
                ty,
                default,
            });
            c.parameters.push(id);
        }
        for s in data["states"].as_array().into_iter().flatten() {
            let m = &s["motion"];
            let motion = if m["clip"].is_object() {
                Motion::Clip(self.clip_from_json(&m["clip"]))
            } else {
                let mut tree = BlendTree {
                    parameter: m["blendParameter"].as_str().unwrap_or("").to_string(),
                    children: Vec::new(),
                };
                for ch in m["children"].as_array().into_iter().flatten() {
                    tree.children.push(BlendChild {
                        clip: self.clip_from_json(&ch["motion"]),
                        threshold: ch["threshold"].as_f64().unwrap_or(0.0),
                        time_scale: ch["timeScale"].as_f64(),
                    });
                }
                Motion::Blend(self.graphs.add_blend(tree))
            };
            let id = self.graphs.add_node(Node {
                name: s["name"].as_str().unwrap_or("").to_string(),
                motion,
                speed: s["speed"].as_f64(),
                speed_parameter: s["speedParameter"].as_str().map(str::to_string),
            });
            c.nodes.push(id);
        }
        for t in data["transitions"].as_array().into_iter().flatten() {
            let mut tr = Transition {
                from: t["from"].as_str().unwrap_or("").to_string(),
                to: t["to"].as_str().unwrap_or("").to_string(),
                conditions: Vec::new(),
                has_exit_time: t["hasExitTime"].as_bool().unwrap_or(false),
                exit_time: t["exitTime"].as_f64(),
                duration: t["duration"].as_f64(),
                offset: t["offset"].as_f64(),
            };
            for cd in t["conditions"].as_array().into_iter().flatten() {
                let id = self.graphs.add_condition(Condition {
                    parameter: cd["parameter"].as_str().unwrap_or("").to_string(),
                    mode: cd["mode"]
                        .as_str()
                        .and_then(ConditionMode::parse)
                        .unwrap_or(ConditionMode::If),
                    threshold: cd["threshold"].as_f64(),
                });
                tr.conditions.push(id);
            }
            let id = self.graphs.add_transition(tr);
            c.transitions.push(id);
        }
        c.default_state = data["defaultState"].as_str().map(str::to_string);
        self.graphs.add_controller(c)
    }
}
