//! Shared types for every Iron crate: typed IDs, script values and the op buffer
//! that carries commands from scripts to the engine.

pub use glam;

mod ids;
mod ops;
mod value;

pub use ids::{ComponentId, EntityId, NodeId};
pub use ops::{Op, OpBuffer};
pub use value::Value;
