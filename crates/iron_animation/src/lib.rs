// SPDX-License-Identifier: Apache-2.0

//! Animation without a script engine: curves, clips, blending and the players that advance them.
//! The script API over it is `iron_engine_api::animation`; the design is
//! `docs/research/iron-native-animation.md`. Behaviour and numbers follow `js/engine/src/animation.ts`.

pub mod animator;
mod bind;
pub mod clip;
pub mod controller;
pub mod culling;
pub mod curve;
pub mod gltf;
#[cfg(test)]
mod legacy_pose;
mod parallel;
#[cfg(test)]
mod parallel_tests;
pub mod player;
pub mod pose;
#[cfg(test)]
mod pose_tests;
pub mod profile;
pub mod registry;

pub use animator::{AnimatorPlayer, StateInfo};
pub use clip::{Binding, ClipEvent, ClipId, CurveId, Parameter, World, with_world};
pub use controller::{
    AnimParameter, BlendChild, BlendId, BlendTree, Condition, ConditionId, ConditionMode,
    Controller, ControllerId, Graphs, Motion, Node, NodeId, Param, ParameterId, ParameterType,
    Transition, TransitionId,
};
pub use culling::{AllVisible, CullingMode, CullingType, Visibility, culled_roots};
pub use curve::{Curve, Keyframe, PointMode, WrapMode, wrap};
pub use parallel::{set_threads, threads};
pub use player::{AnimState, AnimationPlayer, FiredEvent, update, update_culled};
pub use pose::{NoComponents, Pose, PropertySink, resolve, sample};
pub use registry::{RegistrySink, register_setter, register_type};
