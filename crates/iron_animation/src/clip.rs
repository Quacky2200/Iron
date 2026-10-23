//! Clips (`AnimationClip` in `js/engine/src/animation.ts`) and the arenas that hold curves and
//! clips. A clip binds curves (by id, shared: changing a curve later changes every clip that uses
//! it) to a property of an object under the animated one.

use crate::curve::{Curve, WrapMode, wrap};
use serde_json::{Value, json};
use std::cell::RefCell;

/// An index into the curve arena, plus one (0 is "none").
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CurveId(pub u32);

/// An index into the clip arena, plus one (0 is "none").
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ClipId(pub u32);

/// A method called on the animated object's components when a clip passes `time`.
#[derive(Clone, Debug, PartialEq)]
pub struct ClipEvent {
    pub time: f64,
    pub function_name: String,
    /// Passed to the method, if given.
    pub parameter: Option<Parameter>,
}

/// An event parameter: a number, a string or a boolean.
#[derive(Clone, Debug, PartialEq)]
pub enum Parameter {
    Num(f64),
    Str(String),
    Bool(bool),
}

/// What a curve drives.
#[derive(Clone, Debug, PartialEq)]
pub struct Binding {
    /// The animated object (""), or a descendant by child names ("Arm/Hand").
    pub path: String,
    /// `Transform`, `GameObject` (`active`) or a component class name.
    pub type_name: String,
    /// `localPosition.x`, `localRotation.w`, a number property, or one field of a value (`color.r`).
    pub property: String,
    pub curve: CurveId,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Clip {
    pub name: String,
    /// What happens past the end when played (`once` stops at the end, holding it).
    pub wrap_mode: WrapMode,
    pub frame_rate: f64,
    pub events: Vec<ClipEvent>,
    pub bindings: Vec<Binding>,
}

impl Clip {
    pub fn new(name: &str) -> Clip {
        Clip {
            name: name.to_string(),
            wrap_mode: WrapMode::Once,
            frame_rate: 60.0,
            events: Vec::new(),
            bindings: Vec::new(),
        }
    }
}

#[derive(Default)]
pub struct World {
    pub curves: Vec<Curve>,
    pub clips: Vec<Clip>,
    /// The `Animation` components, in the order they were made (the id is the index plus one).
    pub players: Vec<crate::player::AnimationPlayer>,
    /// The `Animator` components, in the order they were made.
    pub animators: Vec<crate::animator::AnimatorPlayer>,
    /// Controller pieces.
    pub graphs: crate::controller::Graphs,
    /// Counts changes to clips and curves: bindings made before one are made again.
    pub(crate) edits: u64,
    /// Clips' bindings per animated root.
    pub(crate) binds: RefCell<crate::bind::Binds>,
    /// The pose players fill and apply in turn, kept so its buffers are reused.
    pub(crate) scratch: RefCell<crate::pose::Pose>,
    /// Clip lengths by clip index, with the (edits, curve count) they were measured at.
    lengths: RefCell<Vec<((u64, usize), f64)>>,
}

thread_local! {
    static WORLD: RefCell<World> = RefCell::new(World::default());
}

/// Runs `f` on this thread's curves and clips.
pub fn with_world<R>(f: impl FnOnce(&mut World) -> R) -> R {
    WORLD.with(|w| f(&mut w.borrow_mut()))
}

/// Forgets every curve and clip (a test, or a new game).
pub fn reset() {
    with_world(|w| *w = World::default());
}

impl World {
    pub fn add_curve(&mut self, c: Curve) -> CurveId {
        self.curves.push(c);
        CurveId(self.curves.len() as u32)
    }

    pub fn add_clip(&mut self, c: Clip) -> ClipId {
        self.clips.push(c);
        ClipId(self.clips.len() as u32)
    }

    pub fn curve(&self, id: CurveId) -> Option<&Curve> {
        self.curves.get((id.0 as usize).wrapping_sub(1))
    }

    pub fn curve_mut(&mut self, id: CurveId) -> Option<&mut Curve> {
        self.edits += 1;
        self.curves.get_mut((id.0 as usize).wrapping_sub(1))
    }

    pub fn clip(&self, id: ClipId) -> Option<&Clip> {
        self.clips.get((id.0 as usize).wrapping_sub(1))
    }

    pub fn clip_mut(&mut self, id: ClipId) -> Option<&mut Clip> {
        self.edits += 1;
        self.clips.get_mut((id.0 as usize).wrapping_sub(1))
    }

    /// Sets (or with `None` removes) the curve for one property.
    pub fn set_curve(
        &mut self,
        clip: ClipId,
        path: &str,
        type_name: &str,
        property: &str,
        curve: Option<CurveId>,
    ) {
        let Some(c) = self.clip_mut(clip) else {
            return;
        };
        if let Some(i) = c
            .bindings
            .iter()
            .position(|b| b.path == path && b.type_name == type_name && b.property == property)
        {
            c.bindings.remove(i);
        }
        if let Some(curve) = curve {
            c.bindings.push(Binding {
                path: path.to_string(),
                type_name: type_name.to_string(),
                property: property.to_string(),
                curve,
            });
        }
    }

    /// The last key's time over all curves (or the last event's). Kept until a clip or curve
    /// changes or a curve is added (players ask several times a frame).
    pub fn clip_length(&self, id: ClipId) -> f64 {
        let stamp = (self.edits, self.curves.len());
        let i = (id.0 as usize).wrapping_sub(1);
        if let Some(&(s, len)) = self.lengths.borrow().get(i)
            && s == stamp
        {
            return len;
        }
        let len = self.measure(id);
        let mut lengths = self.lengths.borrow_mut();
        if i < self.clips.len() {
            if lengths.len() <= i {
                lengths.resize(i + 1, ((u64::MAX, 0), 0.0));
            }
            lengths[i] = (stamp, len);
        }
        len
    }

    fn measure(&self, id: ClipId) -> f64 {
        let Some(c) = self.clip(id) else {
            return 0.0;
        };
        let mut end = 0.0;
        for b in &c.bindings {
            let last = self
                .curve(b.curve)
                .and_then(|cv| cv.keys.last())
                .map_or(0.0, |k| k.time);
            end = iron_math::jsnum::max(end, last);
        }
        for e in &c.events {
            end = iron_math::jsnum::max(end, e.time);
        }
        end
    }

    pub fn add_event(&mut self, clip: ClipId, event: ClipEvent) {
        if let Some(c) = self.clip_mut(clip) {
            c.events.push(event);
            c.events.sort_by(|a, b| {
                (a.time - b.time)
                    .partial_cmp(&0.0)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }
    }

    /// The clip's time `time` folded by its wrap mode (what `sampleAnimation` poses at).
    pub fn sample_time(&self, id: ClipId, time: f64) -> f64 {
        let mode = self.clip(id).map_or(WrapMode::Once, |c| c.wrap_mode);
        wrap(time, self.clip_length(id), mode)
    }

    /// As plain data (for files and scenes).
    pub fn clip_to_json(&self, id: ClipId) -> Value {
        let Some(c) = self.clip(id) else {
            return Value::Null;
        };
        json!({
            "name": c.name,
            "wrapMode": c.wrap_mode.name(),
            "frameRate": c.frame_rate,
            "curves": c.bindings.iter().map(|b| json!({
                "path": b.path, "type": b.type_name, "property": b.property,
                "curve": self.curve(b.curve).map(Curve::to_json).unwrap_or(Value::Null),
            })).collect::<Vec<_>>(),
            "events": c.events.iter().map(|e| {
                let mut o = json!({ "time": e.time, "functionName": e.function_name });
                match &e.parameter {
                    Some(Parameter::Num(n)) => o["parameter"] = json!(n),
                    Some(Parameter::Str(s)) => o["parameter"] = json!(s),
                    Some(Parameter::Bool(b)) => o["parameter"] = json!(b),
                    None => {}
                }
                o
            }).collect::<Vec<_>>(),
        })
    }

    pub fn clip_from_json(&mut self, data: &Value) -> ClipId {
        let mut clip = Clip::new(data["name"].as_str().unwrap_or("clip"));
        clip.wrap_mode = data["wrapMode"]
            .as_str()
            .and_then(WrapMode::parse)
            .unwrap_or(WrapMode::Once);
        clip.frame_rate = data["frameRate"].as_f64().unwrap_or(60.0);
        let id = self.add_clip(clip);
        for c in data["curves"].as_array().into_iter().flatten() {
            let curve = self.add_curve(Curve::from_json(&c["curve"]));
            self.set_curve(
                id,
                c["path"].as_str().unwrap_or(""),
                c["type"].as_str().unwrap_or(""),
                c["property"].as_str().unwrap_or(""),
                Some(curve),
            );
        }
        for e in data["events"].as_array().into_iter().flatten() {
            let parameter = match &e["parameter"] {
                Value::Number(n) => n.as_f64().map(Parameter::Num),
                Value::String(s) => Some(Parameter::Str(s.clone())),
                Value::Bool(b) => Some(Parameter::Bool(*b)),
                _ => None,
            };
            self.add_event(
                id,
                ClipEvent {
                    time: e["time"].as_f64().unwrap_or(0.0),
                    function_name: e["functionName"].as_str().unwrap_or("").to_string(),
                    parameter,
                },
            );
        }
        id
    }
}
