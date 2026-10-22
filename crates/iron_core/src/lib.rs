//! Shared types for every Iron crate: typed IDs, script values and the op buffer
//! that carries commands from scripts to the engine.

pub use glam;

/// The UI target id of a window's tree: `WINDOW_TARGET | handle` in `__iron_commit(ops, target)`
/// (0 is the screen, smaller ids are world panels).
pub const WINDOW_TARGET: u32 = 0x4000_0000;

mod assets;
pub mod glob;
mod ids;
mod ops;
pub mod payload;
pub mod project;
mod value;

pub use assets::{decode_data_url, plan_warning, resolve_asset, set_pack_plan};
pub use ids::{ComponentId, EntityId, NodeId};
pub use ops::{Commit, Op, OpBuffer};
pub use value::Value;
