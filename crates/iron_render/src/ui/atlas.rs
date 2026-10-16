//! Glyph atlas: glyphs rasterized with swash at physical pixel size into two
//! texture pages (an R8 coverage page and an RGBA page for colour emoji),
//! cached by font, glyph, size, subpixel offset and variation.

use std::collections::HashMap;

use etagere::{BucketedAtlasAllocator, size2};
use iron_ui::GlyphRun;
use swash::scale::image::Content;
use swash::scale::{Render, ScaleContext, Source, StrikeWith};
use swash::zeno::{Angle, Format, Transform, Vector};

use crate::Gpu;

/// Horizontal subpixel positions per pixel (glyphs are rasterized at 0, ¼, ½, ¾).
pub const SUBPIXEL_BINS: u8 = 4;

const MASK_SIZE: u32 = 2048;
const COLOR_SIZE: u32 = 1024;
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

struct Page {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    allocator: BucketedAtlasAllocator,
    bytes_per_pixel: u32,
}

impl Page {
    fn new(gpu: &Gpu, size: u32, format: wgpu::TextureFormat, label: &str) -> Self {
        let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width: size,
                height: size,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let view = texture.create_view(&Default::default());
        Self {
            texture,
            view,
            allocator: BucketedAtlasAllocator::new(size2(size as i32, size as i32)),
            bytes_per_pixel: if format == wgpu::TextureFormat::R8Unorm {
                1
            } else {
                4
            },
        }
    }

    /// Reserves space and uploads `data`; returns the image's top-left.
    fn insert(&mut self, gpu: &Gpu, width: u32, height: u32, data: &[u8]) -> Option<(u32, u32)> {
        let alloc = self.allocator.allocate(size2(
            width as i32 + 2 * PADDING,
            height as i32 + 2 * PADDING,
        ))?;
        let x = (alloc.rectangle.min.x + PADDING) as u32;
        let y = (alloc.rectangle.min.y + PADDING) as u32;
        gpu.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d { x, y, z: 0 },
                aspect: wgpu::TextureAspect::All,
            },
            data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * self.bytes_per_pixel),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        Some((x, y))
    }
}

pub struct GlyphAtlas {
    mask: Page,
    color: Page,
    cache: HashMap<GlyphKey, Option<GlyphEntry>>,
    scale: ScaleContext,
    /// Set when a page fills up; the atlas is cleared at the start of the next frame.
    full: bool,
}

impl GlyphAtlas {
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            mask: Page::new(
                gpu,
                MASK_SIZE,
                wgpu::TextureFormat::R8Unorm,
                "glyph coverage",
            ),
            color: Page::new(
                gpu,
                COLOR_SIZE,
                wgpu::TextureFormat::Rgba8UnormSrgb,
                "glyph colour",
            ),
            cache: HashMap::new(),
            scale: ScaleContext::new(),
            full: false,
        }
    }

    pub fn mask_view(&self) -> &wgpu::TextureView {
        &self.mask.view
    }

    pub fn color_view(&self) -> &wgpu::TextureView {
        &self.color.view
    }

    /// Call once per frame before rasterizing: clears the atlas if it filled up
    /// last frame (everything visible is simply rasterized again).
    pub fn begin_frame(&mut self) {
        if self.full {
            tracing::debug!("glyph atlas full; clearing");
            self.mask.allocator.clear();
            self.color.allocator.clear();
            self.cache.clear();
            self.full = false;
        }
    }

    /// The atlas entry for one glyph of `run` at `size_px` physical pixels with a
    /// subpixel offset of `subpixel / SUBPIXEL_BINS` pixels. `None` for glyphs
    /// with no image (spaces) or when the atlas is full.
    pub fn glyph(
        &mut self,
        gpu: &Gpu,
        run: &GlyphRun,
        glyph: u32,
        size_px: f32,
        subpixel: u8,
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
        };
        if let Some(entry) = self.cache.get(&key) {
            return *entry;
        }
        let entry = self.rasterize(gpu, run, glyph, size_px, subpixel);
        if entry.is_some() || !self.full {
            // Don't remember failures caused by a full atlas.
            self.cache.insert(key, entry);
        }
        entry
    }

    fn rasterize(
        &mut self,
        gpu: &Gpu,
        run: &GlyphRun,
        glyph: u32,
        size_px: f32,
        subpixel: u8,
    ) -> Option<GlyphEntry> {
        let font = swash::FontRef::from_index(run.font.data.data(), run.font.index as usize)?;
        let mut scaler = self
            .scale
            .builder(font)
            .size(size_px)
            .hint(false)
            .normalized_coords(run.normalized_coords.iter().copied())
            .build();
        let mut render = Render::new(&[
            Source::ColorOutline(0),
            Source::ColorBitmap(StrikeWith::BestFit),
            Source::Outline,
        ]);
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
        let page = if color {
            &mut self.color
        } else {
            &mut self.mask
        };
        let Some((x, y)) = page.insert(gpu, w, h, &image.data) else {
            self.full = true;
            return None;
        };
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
