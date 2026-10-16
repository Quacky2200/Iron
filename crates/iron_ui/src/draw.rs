use crate::Rgba;

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
}

/// One box to paint: background, border and rounded corners. Colours already
/// include the node's (inherited) opacity.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Quad {
    pub rect: Rect,
    pub background: Rgba,
    pub border_color: Rgba,
    /// Border widths: top, right, bottom, left.
    pub border: [f32; 4],
    pub radius: f32,
}

impl Quad {
    /// Whether painting this quad would change any pixels.
    pub fn is_visible(&self) -> bool {
        let has_border = self.border_color.a > 0.0 && self.border.iter().any(|w| *w > 0.0);
        self.rect.width > 0.0 && self.rect.height > 0.0 && (self.background.a > 0.0 || has_border)
    }
}
