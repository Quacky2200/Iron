//! The draw list: GPU-free paint commands produced by [`crate::UiTree::draw_list`]
//! and consumed by the renderer (DESIGN.md §5.2 "Draw list shape").

use std::sync::Arc;

use crate::Rgba;
use crate::paint::{
    BackgroundImage, BackgroundPosition, BackgroundRepeat, BackgroundSize, BlendMode,
};

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

    /// Whether (x, y) is inside the rectangle with rounded corners `radii`
    /// (top-left, top-right, bottom-right, bottom-left), as used for hit testing.
    pub fn contains_rounded(self, radii: [f32; 4], x: f32, y: f32) -> bool {
        let (right, bottom) = (self.x + self.width, self.y + self.height);
        if x < self.x || y < self.y || x >= right || y >= bottom {
            return false;
        }
        // Corner centres, each inset by its radius.
        let corners = [
            (self.x + radii[0], self.y + radii[0], radii[0]),
            (right - radii[1], self.y + radii[1], radii[1]),
            (right - radii[2], bottom - radii[2], radii[2]),
            (self.x + radii[3], bottom - radii[3], radii[3]),
        ];
        corners.iter().enumerate().all(|(i, &(cx, cy, r))| {
            let outside_x = if i == 0 || i == 3 { x < cx } else { x > cx };
            let outside_y = if i < 2 { y < cy } else { y > cy };
            if r <= 0.0 || !outside_x || !outside_y {
                return true;
            }
            let (dx, dy) = (x - cx, y - cy);
            dx * dx + dy * dy <= r * r
        })
    }

    /// The overlap of two rectangles (zero-sized when they don't overlap).
    pub fn intersect(self, other: Rect) -> Rect {
        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        let right = (self.x + self.width).min(other.x + other.width);
        let bottom = (self.y + self.height).min(other.y + other.height);
        Rect::new(x, y, (right - x).max(0.0), (bottom - y).max(0.0))
    }

    pub fn is_empty(self) -> bool {
        self.width <= 0.0 || self.height <= 0.0
    }

    pub fn union(self, other: Rect) -> Rect {
        let x = self.x.min(other.x);
        let y = self.y.min(other.y);
        let right = (self.x + self.width).max(other.x + other.width);
        let bottom = (self.y + self.height).max(other.y + other.height);
        Rect::new(x, y, right - x, bottom - y)
    }
}

/// A clip region: a rectangle with elliptical corners (`radii_x` / `radii_y` per corner:
/// top-left, top-right, bottom-right, bottom-left). Rectangles, rounded boxes, circles
/// and ellipses are all this shape.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClipShape {
    pub rect: Rect,
    pub radii_x: [f32; 4],
    pub radii_y: [f32; 4],
}

impl ClipShape {
    pub fn rect(rect: Rect) -> Self {
        Self {
            rect,
            radii_x: [0.0; 4],
            radii_y: [0.0; 4],
        }
    }

    /// A box with circular corners.
    pub fn rounded(rect: Rect, radii: [f32; 4]) -> Self {
        Self {
            rect,
            radii_x: radii,
            radii_y: radii,
        }
    }

    /// Whether (x, y) is inside the shape (right and bottom edges excluded).
    pub fn contains(&self, x: f32, y: f32) -> bool {
        let r = self.rect;
        let (right, bottom) = (r.x + r.width, r.y + r.height);
        if x < r.x || y < r.y || x >= right || y >= bottom {
            return false;
        }
        let left = x < r.x + r.width / 2.0;
        let top = y < r.y + r.height / 2.0;
        let corner = match (top, left) {
            (true, true) => 0,
            (true, false) => 1,
            (false, false) => 2,
            (false, true) => 3,
        };
        let (rx, ry) = (self.radii_x[corner], self.radii_y[corner]);
        if rx <= 0.0 || ry <= 0.0 {
            return true;
        }
        let cx = if left { r.x + rx } else { right - rx };
        let cy = if top { r.y + ry } else { bottom - ry };
        let outside_x = if left { x < cx } else { x > cx };
        let outside_y = if top { y < cy } else { y > cy };
        if !outside_x || !outside_y {
            return true;
        }
        let (dx, dy) = ((x - cx) / rx, (y - cy) / ry);
        dx * dx + dy * dy <= 1.0
    }
}

/// A background image or gradient layer. Images are sized and positioned against
/// the quad's rect and painted only inside `area` (the `background-clip` box,
/// rounded by `radii`). Gradients fill `area`.
#[derive(Clone, Debug, PartialEq)]
pub struct ImageFill {
    pub image: Arc<BackgroundImage>,
    pub size: BackgroundSize,
    pub position: BackgroundPosition,
    pub repeat: BackgroundRepeat,
    pub area: Rect,
    pub radii: [f32; 4],
}

/// One box: background colour, image/gradient layers, border and rounded corners.
#[derive(Clone, Debug, PartialEq)]
pub struct Quad {
    pub rect: Rect,
    pub background: Rgba,
    /// Background layers in paint order (bottom first), over the colour, under the border.
    pub images: Vec<ImageFill>,
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
            && (self.background.a > 0.0 || !self.images.is_empty() || has_border)
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
    /// blurred by `blur` and composited with `opacity` and `blend` at the matching
    /// `PopLayer`.
    PushLayer {
        bounds: Rect,
        blur: f32,
        opacity: f32,
        blend: BlendMode,
    },
    PopLayer,
    /// Following commands are clipped to `shape` (intersected with any clip already
    /// active) until the matching `PopClip`.
    PushClip(ClipShape),
    PopClip,
    /// Shaped, positioned text and its decorations.
    Text(TextCmd),
}

/// One run of glyphs in a single font, size and colour.
#[derive(Clone, Debug)]
pub struct GlyphRun {
    pub font: parley::FontData,
    /// Font size in logical pixels.
    pub font_size: f32,
    /// Variable-font coordinates (e.g. weight), normalized as in the font.
    pub normalized_coords: Vec<i16>,
    /// Synthetic bold when the family has no bold face.
    pub embolden: bool,
    /// Synthetic italic angle in degrees when the family has no italic face.
    pub skew: Option<f32>,
    pub color: Rgba,
    pub glyphs: Vec<PositionedGlyph>,
}

impl PartialEq for GlyphRun {
    fn eq(&self, other: &Self) -> bool {
        self.font.data.id() == other.font.data.id()
            && self.font.index == other.font.index
            && self.font_size == other.font_size
            && self.normalized_coords == other.normalized_coords
            && self.embolden == other.embolden
            && self.skew == other.skew
            && self.color == other.color
            && self.glyphs == other.glyphs
    }
}

/// A glyph at its baseline origin, in logical pixels (window space).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PositionedGlyph {
    pub id: u32,
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TextCmd {
    /// Area covered by the glyphs (approximate: line boxes).
    pub bounds: Rect,
    pub runs: Vec<GlyphRun>,
    /// Underlines and line-throughs as filled rectangles.
    pub decorations: Vec<(Rect, Rgba)>,
    /// `background-clip: text`: a gradient painted inside the glyphs, laid out over
    /// the element's box (the rect), under the text colour.
    pub fill: Option<(Arc<BackgroundImage>, Rect)>,
}

impl TextCmd {
    /// A copy moved by (dx, dy) with every colour replaced by `color`
    /// (used for text shadows).
    pub fn shadow(&self, dx: f32, dy: f32, color: Rgba) -> TextCmd {
        TextCmd {
            bounds: self.bounds.offset(dx, dy),
            runs: self
                .runs
                .iter()
                .map(|r| GlyphRun {
                    color,
                    glyphs: r
                        .glyphs
                        .iter()
                        .map(|g| PositionedGlyph {
                            id: g.id,
                            x: g.x + dx,
                            y: g.y + dy,
                        })
                        .collect(),
                    ..r.clone()
                })
                .collect(),
            decorations: self
                .decorations
                .iter()
                .map(|(r, _)| (r.offset(dx, dy), color))
                .collect(),
            fill: None,
        }
    }
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
            DrawCmd::Text(t) => Some(t.bounds),
            DrawCmd::PopLayer | DrawCmd::PushClip(_) | DrawCmd::PopClip => None,
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
            DrawCmd::Text(t) => {
                for run in &mut t.runs {
                    run.color = run.color.with_alpha_scaled(factor);
                }
                for (_, color) in &mut t.decorations {
                    *color = color.with_alpha_scaled(factor);
                }
            }
            DrawCmd::BackdropBlur { .. }
            | DrawCmd::PushLayer { .. }
            | DrawCmd::PopLayer
            | DrawCmd::PushClip(_)
            | DrawCmd::PopClip => {}
        }
    }
}
