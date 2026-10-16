//! The draw list: GPU-free paint commands produced by [`crate::UiTree::draw_list`]
//! and consumed by the renderer (DESIGN.md §5.2 "Draw list shape").

use std::sync::Arc;

use crate::Rgba;
use crate::paint::{BackgroundImage, BackgroundPosition, BackgroundRepeat, BackgroundSize};

/// An axis-aligned rectangle in logical pixels (window space).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn expand(self, by: f32) -> Rect {
        Rect::new(
            self.x - by,
            self.y - by,
            self.width + 2.0 * by,
            self.height + 2.0 * by,
        )
    }

    pub fn offset(self, dx: f32, dy: f32) -> Rect {
        Rect::new(self.x + dx, self.y + dy, self.width, self.height)
    }

    pub fn union(self, other: Rect) -> Rect {
        let x = self.x.min(other.x);
        let y = self.y.min(other.y);
        let right = (self.x + self.width).max(other.x + other.width);
        let bottom = (self.y + self.height).max(other.y + other.height);
        Rect::new(x, y, right - x, bottom - y)
    }
}

/// A background image or gradient with its sizing rules; the renderer resolves
/// it against the quad's rect.
#[derive(Clone, Debug, PartialEq)]
pub struct ImageFill {
    pub image: Arc<BackgroundImage>,
    pub size: BackgroundSize,
    pub position: BackgroundPosition,
    pub repeat: BackgroundRepeat,
}

/// One box: background colour, optional image/gradient, border and rounded corners.
#[derive(Clone, Debug, PartialEq)]
pub struct Quad {
    pub rect: Rect,
    pub background: Rgba,
    pub image: Option<ImageFill>,
    pub border_color: Rgba,
    /// Border widths: top, right, bottom, left.
    pub border: [f32; 4],
    /// Resolved corner radii: top-left, top-right, bottom-right, bottom-left.
    pub radii: [f32; 4],
}

impl Quad {
    /// Whether painting this quad would change any pixels.
    pub fn is_visible(&self) -> bool {
        let has_border = self.border_color.a > 0.0 && self.border.iter().any(|w| *w > 0.0);
        self.rect.width > 0.0
            && self.rect.height > 0.0
            && (self.background.a > 0.0 || self.image.is_some() || has_border)
    }
}

/// A shadow cast by a box, already resolved to pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShadowShape {
    /// The box casting the shadow (before offset and spread).
    pub rect: Rect,
    pub radii: [f32; 4],
    pub offset_x: f32,
    pub offset_y: f32,
    /// CSS blur radius (the Gaussian's standard deviation is half of this).
    pub blur: f32,
    pub spread: f32,
    pub color: Rgba,
}

#[derive(Clone, Debug, PartialEq)]
pub enum DrawCmd {
    /// Blur what has already been drawn behind `rect`, clipped to its rounded shape.
    BackdropBlur {
        rect: Rect,
        radii: [f32; 4],
        blur: f32,
    },
    /// An outer `boxShadow`, drawn before its box.
    BoxShadow(ShadowShape),
    Quad(Quad),
    /// An `inset` shadow, drawn over the background inside the border.
    InsetShadow {
        shadow: ShadowShape,
        border: [f32; 4],
    },
    /// Following commands draw into an offscreen layer covering `bounds`, which is
    /// blurred by `blur` and composited with `opacity` at the matching `PopLayer`.
    PushLayer {
        bounds: Rect,
        blur: f32,
        opacity: f32,
    },
    PopLayer,
}

impl DrawCmd {
    /// The area this command can change, if it paints anything itself.
    pub fn bounds(&self) -> Option<Rect> {
        match self {
            DrawCmd::BackdropBlur { rect, .. } => Some(*rect),
            DrawCmd::BoxShadow(s) => Some(
                s.rect
                    .offset(s.offset_x, s.offset_y)
                    .expand(s.spread.max(0.0) + s.blur),
            ),
            DrawCmd::Quad(q) => Some(q.rect),
            DrawCmd::InsetShadow { shadow, .. } => Some(shadow.rect),
            DrawCmd::PushLayer { bounds, .. } => Some(*bounds),
            DrawCmd::PopLayer => None,
        }
    }

    /// Scales the alpha of every colour this command paints.
    pub(crate) fn scale_alpha(&mut self, factor: f32) {
        match self {
            DrawCmd::BoxShadow(s) | DrawCmd::InsetShadow { shadow: s, .. } => {
                s.color = s.color.with_alpha_scaled(factor)
            }
            DrawCmd::Quad(q) => {
                q.background = q.background.with_alpha_scaled(factor);
                q.border_color = q.border_color.with_alpha_scaled(factor);
            }
            DrawCmd::BackdropBlur { .. } | DrawCmd::PushLayer { .. } | DrawCmd::PopLayer => {}
        }
    }
}
