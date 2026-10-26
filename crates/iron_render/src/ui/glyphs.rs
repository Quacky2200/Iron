//! The GPU-free half of the glyph atlas: glyphs rasterized with swash at physical pixel size,
//! packed into two pages (an R8 coverage page and an RGBA page for colour emoji) and cached by
//! font, glyph, size, subpixel offset and variation. Placing a glyph queues its pixels as a
//! [`GlyphUpload`]; whichever renderer owns the page textures writes them (`atlas.rs` for wgpu).

use std::collections::HashMap;

use etagere::{BucketedAtlasAllocator, size2};
use iron_ui::GlyphRun;
use swash::scale::image::Content;
use swash::scale::{Render, ScaleContext, Source, StrikeWith};
use swash::zeno::{Angle, Format, Stroke, Transform, Vector};

/// Horizontal subpixel positions per pixel (glyphs are rasterized at 0, ¼, ½, ¾).
pub const SUBPIXEL_BINS: u8 = 4;

/// Side of the coverage page, in pixels.
pub const MASK_SIZE: u32 = 2048;
/// Side of the colour page, in pixels.
pub const COLOR_SIZE: u32 = 1024;
/// Empty space around each glyph so filtering never reads a neighbour.
const PADDING: i32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct GlyphKey {
    font: u64,
    index: u32,
    glyph: u32,
    /// Size in physical pixels × 4.
    size: u32,
    subpixel: u8,
    coords: u64,
    embolden: bool,
    skew: u32,
    /// Outline stroke width in physical pixels × 4; 0 for the filled glyph.
    stroke: u32,
}

/// Where a rasterized glyph lives in the atlas and how to place it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GlyphEntry {
    /// In the colour page (emoji) rather than the coverage page.
    pub color: bool,
    /// Top-left of the glyph image in its page, in pixels.
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    /// Offset from the pen position to the image's left edge, in pixels.
    pub left: i32,
    /// Distance from the baseline up to the image's top edge, in pixels.
    pub top: i32,
}

/// Pixels of a newly placed glyph, to write into its page at (`x`, `y`): one byte per pixel in
/// the coverage page, four (RGBA) in the colour page.
#[derive(Clone, Debug, PartialEq)]
pub struct GlyphUpload {
    pub color: bool,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

pub struct GlyphCache {
    mask: BucketedAtlasAllocator,
    color: BucketedAtlasAllocator,
    cache: HashMap<GlyphKey, Option<GlyphEntry>>,
    scale: ScaleContext,
    /// Set when a page fills up; the atlas is cleared at the start of the next frame.
    full: bool,
    uploads: Vec<GlyphUpload>,
}

impl Default for GlyphCache {
    fn default() -> Self {
        Self::new()
    }
}

impl GlyphCache {
    pub fn new() -> Self {
        Self {
            mask: BucketedAtlasAllocator::new(size2(MASK_SIZE as i32, MASK_SIZE as i32)),
            color: BucketedAtlasAllocator::new(size2(COLOR_SIZE as i32, COLOR_SIZE as i32)),
            cache: HashMap::new(),
            scale: ScaleContext::new(),
            full: false,
            uploads: Vec::new(),
        }
    }

    /// Call once per frame before rasterizing: clears the atlas if it filled up
    /// last frame (everything visible is simply rasterized again).
    pub fn begin_frame(&mut self) {
        if self.full {
            tracing::debug!("glyph atlas full; clearing");
            self.mask.clear();
            self.color.clear();
            self.cache.clear();
            self.uploads.clear();
            self.full = false;
        }
    }

    /// The pixels placed since the last call, for the page textures.
    pub fn take_uploads(&mut self) -> Vec<GlyphUpload> {
        std::mem::take(&mut self.uploads)
    }

    /// The atlas entry for one glyph of `run` at `size_px` physical pixels with a
    /// subpixel offset of `subpixel / SUBPIXEL_BINS` pixels. `None` for glyphs
    /// with no image (spaces) or when the atlas is full.
    pub fn glyph(
        &mut self,
        run: &GlyphRun,
        glyph: u32,
        size_px: f32,
        subpixel: u8,
        stroke_px: f32,
    ) -> Option<GlyphEntry> {
        let key = GlyphKey {
            font: run.font.data.id(),
            index: run.font.index,
            glyph,
            size: (size_px * 4.0).round() as u32,
            subpixel,
            coords: hash_coords(&run.normalized_coords),
            embolden: run.embolden,
            skew: run.skew.unwrap_or(0.0).to_bits(),
            stroke: (stroke_px * 4.0).round() as u32,
        };
        if let Some(entry) = self.cache.get(&key) {
            return *entry;
        }
        let entry = self.rasterize(run, glyph, size_px, subpixel, stroke_px);
        if entry.is_some() || !self.full {
            // Don't remember failures caused by a full atlas.
            self.cache.insert(key, entry);
        }
        entry
    }

    fn rasterize(
        &mut self,
        run: &GlyphRun,
        glyph: u32,
        size_px: f32,
        subpixel: u8,
        stroke_px: f32,
    ) -> Option<GlyphEntry> {
        let font = swash::FontRef::from_index(run.font.data.data(), run.font.index as usize)?;
        let mut scaler = self
            .scale
            .builder(font)
            .size(size_px)
            .hint(false)
            .normalized_coords(run.normalized_coords.iter().copied())
            .build();
        // Strokes outline the glyph's shape; colour glyphs (emoji) aren't stroked.
        let sources: &[Source] = if stroke_px > 0.0 {
            &[Source::Outline]
        } else {
            &[
                Source::ColorOutline(0),
                Source::ColorBitmap(StrikeWith::BestFit),
                Source::Outline,
            ]
        };
        let mut render = Render::new(sources);
        if stroke_px > 0.0 {
            render.style(Stroke::new(stroke_px));
        }
        render.format(Format::Alpha).offset(Vector::new(
            f32::from(subpixel) / f32::from(SUBPIXEL_BINS),
            0.0,
        ));
        if run.embolden {
            render.embolden(size_px / 40.0);
        }
        if let Some(degrees) = run.skew {
            render.transform(Some(Transform::skew(
                Angle::from_degrees(degrees),
                Angle::ZERO,
            )));
        }
        let image = render.render(&mut scaler, glyph.try_into().ok()?)?;
        let (w, h) = (image.placement.width, image.placement.height);
        if w == 0 || h == 0 {
            return None;
        }
        let color = image.content == Content::Color;
        let allocator = if color {
            &mut self.color
        } else {
            &mut self.mask
        };
        let Some(alloc) = allocator.allocate(size2(w as i32 + 2 * PADDING, h as i32 + 2 * PADDING))
        else {
            self.full = true;
            return None;
        };
        let x = (alloc.rectangle.min.x + PADDING) as u32;
        let y = (alloc.rectangle.min.y + PADDING) as u32;
        self.uploads.push(GlyphUpload {
            color,
            x,
            y,
            width: w,
            height: h,
            data: image.data,
        });
        Some(GlyphEntry {
            color,
            x,
            y,
            width: w,
            height: h,
            left: image.placement.left,
            top: image.placement.top,
        })
    }
}

fn hash_coords(coords: &[i16]) -> u64 {
    // FNV-1a: tiny and good enough for a handful of variation coordinates.
    coords.iter().fold(0xcbf2_9ce4_8422_2325, |h, c| {
        (h ^ u64::from(*c as u16)).wrapping_mul(0x0100_0000_01b3)
    })
}
