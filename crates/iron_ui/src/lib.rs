//! Iron's UI core: the node tree built from React commits, CSS-like styles,
//! layout (Taffy behind the [`LayoutEngine`] trait) and a GPU-free draw list.
//!
//! Pipeline (DESIGN.md §5.1): `Commit` ops → [`UiTree`] → layout → [`Quad`]s → renderer.

mod color;
mod draw;
mod layout;
pub mod opcodes;
mod style;
mod tree;

pub use color::Rgba;
pub use draw::{Quad, Rect};
pub use layout::{LayoutEngine, NodeLayout, TaffyLayout};
pub use style::{StyleError, Visual, apply_style};
pub use tree::{ROOT, UiError, UiTree};

/// Re-exported because [`LayoutEngine`] uses Taffy's `Style` as its
/// backend-neutral description of CSS layout properties.
pub use taffy::Style as LayoutStyle;
