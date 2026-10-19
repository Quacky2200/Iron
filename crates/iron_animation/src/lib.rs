//! Animation without a script engine: curves, clips, blending and (as they arrive) the players
//! that advance them. The script API over it is `iron_engine_api::animation`; the design is
//! `docs/research/iron-native-animation.md`. Behaviour and numbers follow `js/engine/src/animation.ts`.

pub mod clip;
pub mod curve;
pub mod pose;

pub use clip::{Binding, ClipEvent, ClipId, CurveId, Parameter, World, with_world};
pub use curve::{Curve, Keyframe, PointMode, WrapMode, wrap};
pub use pose::{NoComponents, Pose, PropertySink, resolve, sample};
