//! Animation without a script engine: curves, clips, blending and the players that advance them.
//! The script API over it is `iron_engine_api::animation`; the design is
//! `docs/research/iron-native-animation.md`. Behaviour and numbers follow `js/engine/src/animation.ts`.

pub mod animator;
pub mod clip;
pub mod controller;
pub mod curve;
pub mod gltf;
pub mod player;
pub mod pose;
pub mod registry;

pub use animator::{AnimatorPlayer, StateInfo};
pub use clip::{Binding, ClipEvent, ClipId, CurveId, Parameter, World, with_world};
pub use controller::{
    AnimParameter, BlendChild, BlendId, BlendTree, Condition, ConditionId, ConditionMode,
    Controller, ControllerId, Graphs, Motion, Node, NodeId, Param, ParameterId, ParameterType,
    Transition, TransitionId,
};
pub use curve::{Curve, Keyframe, PointMode, WrapMode, wrap};
pub use player::{AnimState, AnimationPlayer, FiredEvent, update};
pub use pose::{NoComponents, Pose, PropertySink, resolve, sample};
pub use registry::{RegistrySink, register_setter, register_type};
