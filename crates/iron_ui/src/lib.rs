//! Iron's UI core: the node tree built from React commits, CSS-like styles,
//! layout (Taffy behind the [`LayoutEngine`] trait) and a GPU-free draw list.
//!
//! Pipeline (DESIGN.md §5.1): `Commit` ops → [`UiTree`] → layout → [`DrawCmd`]s → renderer.

mod color;
mod draw;
mod layout;
pub mod opcodes;
mod paint;
mod style;
pub mod text;
pub mod text_style;
mod tree;

pub use color::Rgba;
pub use draw::{DrawCmd, ImageFill, Quad, Rect, ShadowShape};
pub use layout::{LayoutEngine, NodeLayout, TaffyLayout};
pub use paint::{
    BackgroundImage, BackgroundPosition, BackgroundRepeat, BackgroundSize, ColorStop, Corners,
    GradientDirection, Len, RadialShape, Shadow,
};
pub use style::{StyleError, Visual, apply_style};
pub use tree::{ROOT, TEXT_NODE, UiError, UiTree};

/// Re-exported because [`LayoutEngine`] uses Taffy's `Style` as its
/// backend-neutral description of CSS layout properties.
pub use taffy::Style as LayoutStyle;
