//! Shared types for every Iron crate: typed IDs, script values and the op buffer
//! that carries commands from scripts to the engine.

pub use glam;

mod assets;
mod ids;
mod ops;
pub mod payload;
pub mod project;
mod value;

pub use assets::{decode_data_url, resolve_asset};
pub use ids::{ComponentId, EntityId, NodeId};
pub use ops::{Commit, Op, OpBuffer};
pub use value::Value;
