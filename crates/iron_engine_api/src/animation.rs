//! The script API of animation: `Keyframe`, `AnimationCurve` and `AnimationClip` over
//! `iron_animation`. The design and what differs from `js/engine/src/animation.ts` is
//! `docs/research/iron-native-animation.md`.
//!
//! Differences so far: a curve is built key by key (`fromPoints` waits for arrays), a clip's
//! target is a class name string (`"Light"`), an event parameter is a number, and `toJSON()` is
//! text (`toJson()`), with `AnimationClip.fromJson(text)`.

use iron_animation::{
    Binding, ClipEvent, ClipId, Curve, CurveId, Keyframe, NoComponents, Parameter, WrapMode,
    with_world,
};
use iron_api::{Handle, iron_api};
use iron_scene::ObjectId;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CurveRef(pub u64);

impl Handle for CurveRef {
    fn to_raw(self) -> u64 {
        self.0
    }
    fn from_raw(raw: u64) -> Self {
        CurveRef(raw)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClipRef(pub u64);

impl Handle for ClipRef {
    fn to_raw(self) -> u64 {
        self.0
    }
    fn from_raw(raw: u64) -> Self {
        ClipRef(raw)
    }
}

impl CurveRef {
    fn id(self) -> CurveId {
        CurveId(self.0 as u32)
    }
}

impl ClipRef {
    fn id(self) -> ClipId {
        ClipId(self.0 as u32)
    }
}

fn zero_if_nan(x: f64) -> f64 {
    if x.is_nan() { 0.0 } else { x }
}

fn new_curve(c: Curve) -> CurveRef {
    CurveRef(u64::from(with_world(|w| w.add_curve(c)).0))
}

#[iron_api(namespace = "Keyframe", quickjs, values(
    Keyframe = Keyframe [time: f64, value: f64, inTangent: f64, outTangent: f64]))]
pub mod keyframe_api {
    use super::{Keyframe, zero_if_nan};

    /// One key on a curve: tangents are slopes (value per second); an infinite tangent holds a
    /// value (a step).
    #[optional(in_tangent, out_tangent)]
    pub fn create(time: f64, value: f64, in_tangent: f64, out_tangent: f64) -> Keyframe {
        Keyframe {
            time,
            value,
            in_tangent: zero_if_nan(in_tangent),
            out_tangent: zero_if_nan(out_tangent),
        }
    }
}

#[iron_api(namespace = "AnimationCurve", handles(AnimationCurve = CurveRef), values(
    Keyframe = Keyframe [time: f64, value: f64, inTangent: f64, outTangent: f64]))]
pub mod animation_curve_api {
    use super::{Curve, CurveRef, Keyframe, WrapMode, new_curve, with_world, zero_if_nan};

    impl CurveRef {
        /// An empty curve (add keys with `addKey`).
        pub fn create() -> CurveRef {
            new_curve(Curve::default())
        }

        /// A straight line from (t0, v0) to (t1, v1).
        pub fn linear(t0: f64, v0: f64, t1: f64, v1: f64) -> CurveRef {
            new_curve(Curve::linear(t0, v0, t1, v1))
        }

        /// From (t0, v0) to (t1, v1), easing in and out (flat tangents).
        pub fn ease_in_out(t0: f64, v0: f64, t1: f64, v1: f64) -> CurveRef {
            new_curve(Curve::ease_in_out(t0, v0, t1, v1))
        }

        /// `value` from t0 to t1.
        pub fn constant(t0: f64, t1: f64, value: f64) -> CurveRef {
            new_curve(Curve::constant(t0, t1, value))
        }

        /// Adds a key (flat tangents), keeping them in time order; returns its index. For
        /// tangents add a `Keyframe` with `addKeyframe`.
        #[optional(value)]
        pub fn add_key(self, time: f64, value: f64) -> f64 {
            let key = Keyframe::new(time, zero_if_nan(value));
            with_world(|w| {
                w.curve_mut(self.id())
                    .map_or(-1.0, |c| c.add_key(key) as f64)
            })
        }

        /// Adds a ready key; returns its index.
        pub fn add_keyframe(self, key: Keyframe) -> f64 {
            with_world(|w| {
                w.curve_mut(self.id())
                    .map_or(-1.0, |c| c.add_key(key) as f64)
            })
        }

        pub fn remove_key(self, index: f64) {
            with_world(|w| {
                if let Some(c) = w.curve_mut(self.id())
                    && index >= 0.0
                {
                    c.remove_key(index as usize);
                }
            });
        }

        /// The number of keys.
        #[property]
        pub fn length(self) -> f64 {
            with_world(|w| w.curve(self.id()).map_or(0.0, |c| c.keys.len() as f64))
        }

        /// The i-th key (a zero key past the end).
        pub fn key(self, index: f64) -> Keyframe {
            with_world(|w| {
                w.curve(self.id())
                    .and_then(|c| c.keys.get(index as usize).copied())
                    .unwrap_or(Keyframe::new(0.0, 0.0))
            })
        }

        /// The curve's value at `time`.
        pub fn evaluate(self, time: f64) -> f64 {
            with_world(|w| w.curve(self.id()).map_or(0.0, |c| c.evaluate(time)))
        }

        /// `once`, `loop`, `pingPong` or `clampForever` (the default): before the first key.
        #[property]
        pub fn pre_wrap_mode(self) -> String {
            with_world(|w| {
                w.curve(self.id())
                    .map_or(WrapMode::ClampForever, |c| c.pre_wrap)
                    .name()
                    .to_string()
            })
        }

        pub fn set_pre_wrap_mode(self, mode: &str) {
            if let Some(m) = WrapMode::parse(mode) {
                with_world(|w| {
                    if let Some(c) = w.curve_mut(self.id()) {
                        c.pre_wrap = m;
                    }
                });
            }
        }

        /// After the last key.
        #[property]
        pub fn post_wrap_mode(self) -> String {
            with_world(|w| {
                w.curve(self.id())
                    .map_or(WrapMode::ClampForever, |c| c.post_wrap)
                    .name()
                    .to_string()
            })
        }

        pub fn set_post_wrap_mode(self, mode: &str) {
            if let Some(m) = WrapMode::parse(mode) {
                with_world(|w| {
                    if let Some(c) = w.curve_mut(self.id()) {
                        c.post_wrap = m;
                    }
                });
            }
        }
    }
}

#[iron_api(namespace = "AnimationClip", handles(
    AnimationClip = ClipRef, AnimationCurve = CurveRef, GameObject = ObjectId))]
pub mod animation_clip_api {
    use super::{
        Binding, ClipEvent, ClipRef, NoComponents, ObjectId, Parameter, WrapMode, with_world,
    };

    impl ClipRef {
        /// A clip with a name (`"clip"` without one).
        #[optional(name)]
        pub fn create(name: &str) -> ClipRef {
            let n = if name.is_empty() { "clip" } else { name };
            ClipRef(u64::from(
                with_world(|w| w.add_clip(iron_animation::clip::Clip::new(n))).0,
            ))
        }

        #[property]
        pub fn name(self) -> String {
            with_world(|w| {
                w.clip(self.id())
                    .map(|c| c.name.clone())
                    .unwrap_or_default()
            })
        }

        pub fn set_name(self, name: &str) {
            with_world(|w| {
                if let Some(c) = w.clip_mut(self.id()) {
                    c.name = name.to_string();
                }
            });
        }

        /// What happens past the end when played (`once` stops at the end, holding it).
        #[property]
        pub fn wrap_mode(self) -> String {
            with_world(|w| {
                w.clip(self.id())
                    .map_or(WrapMode::Once, |c| c.wrap_mode)
                    .name()
                    .to_string()
            })
        }

        pub fn set_wrap_mode(self, mode: &str) {
            if let Some(m) = WrapMode::parse(mode) {
                with_world(|w| {
                    if let Some(c) = w.clip_mut(self.id()) {
                        c.wrap_mode = m;
                    }
                });
            }
        }

        #[property]
        pub fn frame_rate(self) -> f64 {
            with_world(|w| w.clip(self.id()).map_or(60.0, |c| c.frame_rate))
        }

        pub fn set_frame_rate(self, rate: f64) {
            with_world(|w| {
                if let Some(c) = w.clip_mut(self.id()) {
                    c.frame_rate = rate;
                }
            });
        }

        /// The last key's time over all curves (or the last event's).
        #[property]
        pub fn length(self) -> f64 {
            with_world(|w| w.clip_length(self.id()))
        }

        /// Sets the curve of one property: `path` is "" for the animated object or "Arm/Hand" for
        /// a descendant by names; `type` is `Transform`, `GameObject` (`active`) or a component
        /// class name; Transform properties are `localPosition.x|y|z`, `localScale.x|y|z`,
        /// `localRotation.x|y|z|w` and `localEulerAngles.x|y|z`.
        pub fn set_curve(
            self,
            path: &str,
            type_name: &str,
            property: &str,
            curve: super::CurveRef,
        ) {
            with_world(|w| w.set_curve(self.id(), path, type_name, property, Some(curve.id())));
        }

        /// Removes the curve of one property.
        pub fn remove_curve(self, path: &str, type_name: &str, property: &str) {
            with_world(|w| w.set_curve(self.id(), path, type_name, property, None));
        }

        /// How many curves the clip has.
        #[property]
        pub fn curve_count(self) -> f64 {
            with_world(|w| w.clip(self.id()).map_or(0.0, |c| c.bindings.len() as f64))
        }

        /// The i-th curve's path, type and property, joined with `|` ("" past the end).
        pub fn curve_at(self, index: f64) -> String {
            with_world(|w| {
                w.clip(self.id())
                    .and_then(|c| c.bindings.get(index as usize))
                    .map(|b: &Binding| format!("{}|{}|{}", b.path, b.type_name, b.property))
                    .unwrap_or_default()
            })
        }

        /// The curve of one property (no curve, a zero handle, when there is none).
        pub fn curve(self, path: &str, type_name: &str, property: &str) -> super::CurveRef {
            with_world(|w| {
                w.clip(self.id())
                    .and_then(|c| {
                        c.bindings.iter().find(|b| {
                            b.path == path && b.type_name == type_name && b.property == property
                        })
                    })
                    .map_or(super::CurveRef(0), |b| {
                        super::CurveRef(u64::from(b.curve.0))
                    })
            })
        }

        /// Calls a method named `function_name` on the animated object's components when the
        /// clip passes `time`, with a number if one is given.
        #[optional(parameter)]
        pub fn add_event(self, time: f64, function_name: &str, parameter: f64) {
            let parameter = (!parameter.is_nan()).then_some(Parameter::Num(parameter));
            with_world(|w| {
                w.add_event(
                    self.id(),
                    ClipEvent {
                        time,
                        function_name: function_name.to_string(),
                        parameter,
                    },
                )
            });
        }

        #[property]
        pub fn event_count(self) -> f64 {
            with_world(|w| w.clip(self.id()).map_or(0.0, |c| c.events.len() as f64))
        }

        /// Poses `go` as the clip is at `time` (Unity's `SampleAnimation`).
        pub fn sample_animation(self, go: ObjectId, time: f64) {
            with_world(|w| {
                iron_scene::api::with_scene(|scene| {
                    iron_animation::sample(scene, w, &mut NoComponents, go, self.id(), time)
                })
            });
        }

        /// The clip as JSON text (for files and scenes).
        pub fn to_json(self) -> String {
            with_world(|w| w.clip_to_json(self.id()).to_string())
        }

        /// A clip from the JSON `toJson` wrote (an empty clip if the text is not JSON).
        pub fn from_json(text: &str) -> ClipRef {
            let data = serde_json::from_str(text).unwrap_or(serde_json::Value::Null);
            ClipRef(u64::from(with_world(|w| w.clip_from_json(&data)).0))
        }
    }
}
