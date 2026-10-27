// SPDX-License-Identifier: Apache-2.0

//! Background images: loading `url(...)` files from the game's folder into
//! textures (cached by URL), and CSS `background-size` / `background-position`.
//! SVG images (and inline `<svg>` elements) are rasterized with resvg at the size
//! they're drawn, so they stay sharp.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use resvg::{tiny_skia, usvg};

use platypus_ui::{
    BackgroundPosition, BackgroundRepeat, BackgroundSize, BorderImageRepeat, Len, Rect, RepeatAxis,
};

use crate::Gpu;

/// The two ways images are sampled: filtered, and nearest-neighbour
/// (`image-rendering: pixelated`). Both repeat.
pub struct ImageSamplers {
    pub smooth: wgpu::Sampler,
    pub nearest: wgpu::Sampler,
}

pub struct LoadedImage {
    pub bind_group: wgpu::BindGroup,
    /// The same texture with the nearest-neighbour sampler.
    pub bind_group_nearest: wgpu::BindGroup,
    /// Natural size in pixels (treated as CSS pixels).
    pub size: (u32, u32),
}

/// Images by URL. Failed loads are remembered so they're reported once.
#[derive(Default)]
pub struct ImageCache {
    root: Option<PathBuf>,
    entries: HashMap<String, Option<LoadedImage>>,
    /// Parsed SVG documents, by URL (or by content for inline SVG).
    svgs: HashMap<String, Option<Arc<usvg::Tree>>>,
    /// usvg's options, with the bundled font for `<text>` (made on first use).
    svg_options: Option<usvg::Options<'static>>,
    /// Frames drawn, and when each `data:` SVG entry was last used: inline `<svg>`
    /// elements become a new data URL whenever they change (an animation makes one per
    /// frame), so unused ones are dropped.
    frame: u64,
    used: HashMap<String, u64>,
}

/// `data:` SVG entries unused for this many frames are dropped.
const SVG_DATA_TTL: u64 = 120;

/// Whether `url` names an SVG image.
pub fn is_svg(url: &str) -> bool {
    let url = url.trim();
    if url.starts_with("data:") {
        return url.starts_with("data:image/svg+xml");
    }
    let path = url.split(['?', '#']).next().unwrap_or(url);
    path.to_ascii_lowercase().ends_with(".svg")
}

/// Rounds a raster size up to a multiple of 8 (within 8..=4096), so small size
/// changes reuse the same raster.
fn bucket(px: f32) -> u32 {
    let px = px.ceil().clamp(1.0, 4096.0) as u32;
    px.div_ceil(8) * 8
}

impl ImageCache {
    pub fn set_root(&mut self, root: PathBuf) {
        self.root = Some(root);
        self.entries.clear();
        self.svgs.clear();
        self.used.clear();
    }

    /// Call once per frame: drops `data:` SVGs (and their rasters) that weren't drawn
    /// for a while.
    pub fn begin_frame(&mut self) {
        self.frame += 1;
        if !self.frame.is_multiple_of(30) {
            return;
        }
        let cutoff = self.frame.saturating_sub(SVG_DATA_TTL);
        let stale: Vec<String> = self
            .used
            .iter()
            .filter(|(_, at)| **at < cutoff)
            .map(|(k, _)| k.clone())
            .collect();
        for key in stale {
            self.used.remove(&key);
            self.entries.remove(&key);
            self.svgs.remove(&key);
        }
    }

    /// The cache key for `url`: `data:` SVGs (often long, and new whenever an inline
    /// `<svg>` changes) go by a hash, and are tracked for eviction.
    fn key_for(&mut self, url: &str) -> String {
        if url.starts_with("data:") && is_svg(url) {
            let key = format!("svg-data:{:016x}", hash(url));
            self.used.insert(key.clone(), self.frame);
            key
        } else {
            url.to_owned()
        }
    }

    /// An SVG drawn at `px` physical pixels: rasterized at about that size (cached)
    /// and keyed for [`Self::get`]. `source` is a URL, or SVG text for `inline`.
    /// Returns the key and the image's natural size.
    #[allow(clippy::too_many_arguments)]
    pub fn svg_at(
        &mut self,
        gpu: &Gpu,
        layout: &wgpu::BindGroupLayout,
        samplers: &ImageSamplers,
        source: &str,
        inline: bool,
        px: (f32, f32),
    ) -> Option<(String, (u32, u32))> {
        let id = if inline {
            format!("svg-inline:{:016x}", hash(source))
        } else {
            self.key_for(source)
        };
        let tree = self.svg_tree(&id, source, inline)?;
        let natural = svg_natural(&tree);
        let (w, h) = (bucket(px.0), bucket(px.1));
        let key = format!("{id}#{w}x{h}");
        if self.used.contains_key(&id) {
            self.used.insert(key.clone(), self.frame);
        }
        if !self.entries.contains_key(&key) {
            let loaded = rasterize(&tree, (w, h))
                .and_then(|pixels| upload(gpu, layout, samplers, &key, &pixels, (w, h), natural));
            self.entries.insert(key.clone(), loaded);
        }
        self.entries.get(&key)?.as_ref()?;
        Some((key, natural))
    }

    /// The parsed SVG for `id` (a URL, or `svg-inline:<hash>` for `text`).
    fn svg_tree(&mut self, id: &str, text: &str, inline: bool) -> Option<Arc<usvg::Tree>> {
        if let Some(tree) = self.svgs.get(id) {
            return tree.clone();
        }
        let options = self.svg_options.get_or_insert_with(|| {
            let mut options = usvg::Options::default();
            let db = Arc::make_mut(&mut options.fontdb);
            db.load_font_data(platypus_ui::text::INTER.to_vec());
            db.load_font_data(platypus_ui::text::INTER_ITALIC.to_vec());
            // The file's own family name ("Inter Variable"), whatever Platypus calls it.
            let family = db
                .faces()
                .next()
                .and_then(|f| f.families.first())
                .map_or_else(
                    || platypus_ui::text::DEFAULT_FAMILY.to_owned(),
                    |f| f.0.clone(),
                );
            db.set_sans_serif_family(family.clone());
            db.set_serif_family(family.clone());
            options.font_family = family;
            options
        });
        let data = if inline {
            Some(text.as_bytes().to_vec())
        } else if text.trim_start().starts_with("data:") {
            platypus_core::decode_data_url(text)
        } else {
            self.root.as_ref().and_then(|root| {
                platypus_core::read_asset(root, text)
                    .inspect_err(|e| tracing::warn!("{e}"))
                    .ok()
            })
        };
        let tree = match data.map(|d| usvg::Tree::from_data(&d, options)) {
            Some(Ok(tree)) => Some(Arc::new(tree)),
            Some(Err(e)) => {
                tracing::warn!("SVG `{}` couldn't be parsed: {e}", short(id));
                None
            }
            None => {
                tracing::warn!("SVG image `{}` couldn't be loaded", short(id));
                None
            }
        };
        self.svgs.insert(id.to_owned(), tree.clone());
        tree
    }

    pub fn get(
        &mut self,
        gpu: &Gpu,
        layout: &wgpu::BindGroupLayout,
        samplers: &ImageSamplers,
        url: &str,
    ) -> Option<&LoadedImage> {
        let key = self.key_for(url);
        if !self.entries.contains_key(&key) {
            let loaded = self.load(gpu, layout, samplers, url);
            self.entries.insert(key.clone(), loaded);
        }
        self.entries.get(&key)?.as_ref()
    }

    fn load(
        &mut self,
        gpu: &Gpu,
        layout: &wgpu::BindGroupLayout,
        samplers: &ImageSamplers,
        url: &str,
    ) -> Option<LoadedImage> {
        if is_svg(url) {
            // At its natural size; draws ask for a sharper copy with `svg_at`.
            let id = self.key_for(url);
            let tree = self.svg_tree(&id, url, false)?;
            let natural = svg_natural(&tree);
            let pixels = rasterize(&tree, natural)?;
            return upload(gpu, layout, samplers, url, &pixels, natural, natural);
        }
        let decoded = if url.trim_start().starts_with("data:") {
            // Inlined by the bundler (small images referenced from CSS).
            let Some(bytes) = platypus_core::decode_data_url(url) else {
                tracing::warn!("background image data URL couldn't be decoded");
                return None;
            };
            image::load_from_memory(&bytes)
        } else {
            let Some(root) = &self.root else {
                tracing::warn!("background image `{url}` ignored: no asset folder set");
                return None;
            };
            let bytes = match platypus_core::read_asset(root, url) {
                Ok(b) => b,
                Err(e) => {
                    tracing::warn!("background image `{url}`: {e}");
                    return None;
                }
            };
            image::load_from_memory(&bytes)
        };
        let image = match decoded {
            Ok(i) => i.to_rgba8(),
            Err(e) => {
                tracing::warn!("background image `{}` could not be loaded: {e}", short(url));
                return None;
            }
        };
        let (w, h) = image.dimensions();
        tracing::debug!("loaded background image `{}` ({w}x{h})", short(url));
        upload(gpu, layout, samplers, url, &image, (w, h), (w, h))
    }
}

/// An SVG's natural size in CSS pixels (its `width`/`height`, else its `viewBox`).
fn svg_natural(tree: &usvg::Tree) -> (u32, u32) {
    let size = tree.size();
    (
        (size.width().ceil() as u32).max(1),
        (size.height().ceil() as u32).max(1),
    )
}

/// Renders `tree` stretched to `w` × `h` pixels, as straight-alpha RGBA.
fn rasterize(tree: &usvg::Tree, (w, h): (u32, u32)) -> Option<Vec<u8>> {
    let mut pixmap = tiny_skia::Pixmap::new(w, h)?;
    let size = tree.size();
    let transform =
        tiny_skia::Transform::from_scale(w as f32 / size.width(), h as f32 / size.height());
    resvg::render(tree, transform, &mut pixmap.as_mut());
    // tiny-skia works premultiplied; textures here hold straight alpha.
    let mut pixels = pixmap.take();
    for p in pixels.as_chunks_mut::<4>().0 {
        let a = p[3];
        if a > 0 && a < 255 {
            for c in &mut p[..3] {
                *c = ((u32::from(*c) * 255 + u32::from(a) / 2) / u32::from(a)).min(255) as u8;
            }
        }
    }
    Some(pixels)
}

/// Uploads straight-alpha RGBA `pixels` (`w` × `h`) as a texture with both samplers.
/// `natural` is the size layout uses (an SVG's raster can be bigger).
fn upload(
    gpu: &Gpu,
    layout: &wgpu::BindGroupLayout,
    samplers: &ImageSamplers,
    url: &str,
    pixels: &[u8],
    (w, h): (u32, u32),
    natural: (u32, u32),
) -> Option<LoadedImage> {
    let label = short(url);
    let size = wgpu::Extent3d {
        width: w,
        height: h,
        depth_or_array_layers: 1,
    };
    let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    gpu.queue.write_texture(
        texture.as_image_copy(),
        pixels,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4 * w),
            rows_per_image: Some(h),
        },
        size,
    );
    let view = texture.create_view(&Default::default());
    let bind = |sampler: &wgpu::Sampler| {
        gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(label),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        })
    };
    Some(LoadedImage {
        bind_group: bind(&samplers.smooth),
        bind_group_nearest: bind(&samplers.nearest),
        size: natural,
    })
}

/// A stable hash of inline SVG text (FNV-1a), for cache keys.
fn hash(s: &str) -> u64 {
    s.bytes().fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// Where one tile of an image of `natural` size goes in `area`, following CSS
/// `background-size` and `background-position`.
pub fn tile_rect(
    area: Rect,
    natural: (f32, f32),
    size: BackgroundSize,
    position: BackgroundPosition,
) -> Rect {
    let (iw, ih) = (natural.0.max(1.0), natural.1.max(1.0));
    let (w, h) = (area.width, area.height);
    let (tw, th) = match size {
        BackgroundSize::Cover => {
            let s = (w / iw).max(h / ih);
            (iw * s, ih * s)
        }
        BackgroundSize::Contain => {
            let s = (w / iw).min(h / ih);
            (iw * s, ih * s)
        }
        BackgroundSize::Explicit(sw, sh) => match (sw, sh) {
            (None, None) => (iw, ih),
            (Some(sw), None) => {
                let tw = sw.resolve(w);
                (tw, tw * ih / iw)
            }
            (None, Some(sh)) => {
                let th = sh.resolve(h);
                (th * iw / ih, th)
            }
            (Some(sw), Some(sh)) => (sw.resolve(w), sh.resolve(h)),
        },
    };
    // Percentages line up the same point of the image and the box.
    let offset = |len: Len, space: f32| match len {
        Len::Percent(f) => space * f,
        Len::Px(px) => px,
    };
    Rect::new(
        area.x + offset(position.x, w - tw),
        area.y + offset(position.y, h - th),
        tw.max(0.0),
        th.max(0.0),
    )
}

/// Applies `background-repeat` to one placed tile in `area`: `round` rescales the
/// tile so whole tiles fit, `space` keeps its size and spreads the leftover space
/// as gaps. Returns the tile, the gap per axis and whether each axis repeats.
pub fn repeat_tiles(
    tile: Rect,
    area: Rect,
    repeat: BackgroundRepeat,
) -> (Rect, [f32; 2], [bool; 2]) {
    let axis = |pos: f32, len: f32, start: f32, space: f32, mode: RepeatAxis| match mode {
        RepeatAxis::Repeat => (pos, len, 0.0, true),
        RepeatAxis::NoRepeat => (pos, len, 0.0, false),
        RepeatAxis::Round if len > 0.0 => {
            let n = (space / len).round().max(1.0);
            (start, space / n, 0.0, true)
        }
        RepeatAxis::Space if len > 0.0 && (space / len).floor() >= 2.0 => {
            let n = (space / len).floor();
            (start, len, (space - n * len) / (n - 1.0), true)
        }
        // Too small to round, or room for one tile only: placed once.
        RepeatAxis::Round | RepeatAxis::Space => (pos, len, 0.0, false),
    };
    let (x, w, gx, rx) = axis(tile.x, tile.width, area.x, area.width, repeat.x);
    let (y, h, gy, ry) = axis(tile.y, tile.height, area.y, area.height, repeat.y);
    (Rect::new(x, y, w, h), [gx, gy], [rx, ry])
}

/// Lays tiles of size `tile` along one axis of a border image piece starting at
/// `start` with length `len`: (tile start, tile size, gap, repeats), or `None`
/// when `space` can't fit a single tile (CSS then draws nothing).
pub fn border_image_axis(
    start: f32,
    len: f32,
    tile: f32,
    mode: BorderImageRepeat,
) -> Option<(f32, f32, f32, bool)> {
    if tile <= 0.0 {
        return Some((start, len, 0.0, false));
    }
    Some(match mode {
        BorderImageRepeat::Stretch => (start, len, 0.0, false),
        // Centred, so the cut-off tiles at both ends match.
        BorderImageRepeat::Repeat => (start + (len - tile) / 2.0, tile, 0.0, true),
        BorderImageRepeat::Round => {
            let n = (len / tile).round().max(1.0);
            (start, len / n, 0.0, true)
        }
        BorderImageRepeat::Space => {
            let n = (len / tile).floor();
            if n < 1.0 {
                return None;
            }
            // The leftover space goes around the tiles, ends included.
            let gap = (len - n * tile) / (n + 1.0);
            (start + gap, tile, gap, true)
        }
    })
}

/// A URL shortened for log messages (data URLs can be huge).
fn short(url: &str) -> &str {
    match url.char_indices().nth(60) {
        Some((i, _)) => &url[..i],
        None => url,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn background_size_and_position_follow_css() {
        let area = Rect::new(10.0, 20.0, 100.0, 50.0);
        let natural = (20.0, 10.0);
        let top_left = BackgroundPosition::default();
        let centre = BackgroundPosition {
            x: Len::Percent(0.5),
            y: Len::Percent(0.5),
        };

        // auto: natural size at the top-left.
        let r = tile_rect(area, natural, BackgroundSize::default(), top_left);
        assert_eq!(r, Rect::new(10.0, 20.0, 20.0, 10.0));
        // cover fills the box, keeping the aspect ratio (2:1 image, 2:1 box).
        assert_eq!(
            tile_rect(area, natural, BackgroundSize::Cover, top_left),
            Rect::new(10.0, 20.0, 100.0, 50.0)
        );
        // contain on a tall box: limited by width, centred vertically.
        let tall = Rect::new(0.0, 0.0, 40.0, 100.0);
        assert_eq!(
            tile_rect(tall, natural, BackgroundSize::Contain, centre),
            Rect::new(0.0, 40.0, 40.0, 20.0)
        );
        // cover on a tall box: limited by height, centred horizontally.
        assert_eq!(
            tile_rect(tall, natural, BackgroundSize::Cover, centre),
            Rect::new(-80.0, 0.0, 200.0, 100.0)
        );
        // One explicit length keeps the aspect ratio; percentages use the box.
        assert_eq!(
            tile_rect(
                area,
                natural,
                BackgroundSize::Explicit(Some(Len::Px(40.0)), None),
                top_left
            ),
            Rect::new(10.0, 20.0, 40.0, 20.0)
        );
        assert_eq!(
            tile_rect(
                area,
                natural,
                BackgroundSize::Explicit(Some(Len::Percent(0.5)), Some(Len::Px(5.0))),
                centre
            ),
            Rect::new(35.0, 42.5, 50.0, 5.0)
        );
        // Pixel positions offset from the top-left.
        let px = BackgroundPosition {
            x: Len::Px(7.0),
            y: Len::Px(3.0),
        };
        assert_eq!(
            tile_rect(area, natural, BackgroundSize::default(), px),
            Rect::new(17.0, 23.0, 20.0, 10.0)
        );
    }

    // M3-BRD-04
    #[test]
    fn border_image_tiles_along_an_axis() {
        use BorderImageRepeat::*;
        assert_eq!(
            border_image_axis(10.0, 100.0, 30.0, Stretch),
            Some((10.0, 100.0, 0.0, false))
        );
        assert_eq!(
            border_image_axis(10.0, 100.0, 30.0, Repeat),
            Some((45.0, 30.0, 0.0, true)),
            "centred"
        );
        let (start, size, gap, rep) = border_image_axis(10.0, 100.0, 30.0, Round).unwrap();
        assert!((size - 100.0 / 3.0).abs() < 1e-4 && start == 10.0 && gap == 0.0 && rep);
        assert_eq!(
            border_image_axis(0.0, 100.0, 30.0, Space),
            Some((2.5, 30.0, 2.5, true))
        );
        assert_eq!(
            border_image_axis(0.0, 20.0, 30.0, Space),
            None,
            "no room for one"
        );
    }

    // M3-BG-02
    #[test]
    fn round_and_space_fit_whole_tiles() {
        let area = Rect::new(0.0, 0.0, 100.0, 50.0);
        let tile = Rect::new(7.0, 3.0, 30.0, 22.0);
        let repeat = |x, y| BackgroundRepeat { x, y };
        // 100 / 30 rounds to 3 tiles of 33.3; 50 / 22 rounds to 2 tiles of 25.
        let (t, gap, rep) = repeat_tiles(tile, area, repeat(RepeatAxis::Round, RepeatAxis::Round));
        assert!((t.width - 100.0 / 3.0).abs() < 1e-4 && t.height == 25.0);
        assert_eq!((t.x, t.y, gap, rep), (0.0, 0.0, [0.0, 0.0], [true, true]));
        // Space: 3 tiles of 30 with 5px gaps; only 2 of 22 fit in 50, one 6px gap.
        let (t, gap, rep) = repeat_tiles(tile, area, repeat(RepeatAxis::Space, RepeatAxis::Space));
        assert_eq!((t.x, t.width, t.y, t.height), (0.0, 30.0, 0.0, 22.0));
        assert_eq!((gap, rep), ([5.0, 6.0], [true, true]));
        // A tile too big to space twice is placed once, where `position` put it.
        let big = Rect::new(7.0, 3.0, 60.0, 30.0);
        let (t, _, rep) = repeat_tiles(big, area, repeat(RepeatAxis::Space, RepeatAxis::NoRepeat));
        assert_eq!((t.x, rep), (7.0, [false, false]));
    }

    struct MapFs(HashMap<String, Vec<u8>>);

    impl platypus_core::AssetFs for MapFs {
        fn read(&self, path: &str) -> std::io::Result<Vec<u8>> {
            self.0.get(path).cloned().ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("asset \"{path}\" not found in assets.dat"),
                )
            })
        }
        fn open_range(&self, _: &str) -> std::io::Result<Arc<dyn platypus_core::RangeRead>> {
            Err(std::io::ErrorKind::Unsupported.into())
        }
        fn exists(&self, path: &str) -> bool {
            self.0.contains_key(path)
        }
        fn list(&self) -> Vec<String> {
            self.0.keys().cloned().collect()
        }
        fn pack_of(&self, _: &str) -> String {
            "assets".into()
        }
    }

    // M9-API-1149
    #[test]
    fn svg_images_load_from_the_asset_file_system_not_the_folder() {
        let svg = br#"<svg xmlns="http://www.w3.org/2000/svg" width="8" height="6"><rect width="8" height="6"/></svg>"#;
        let fs = MapFs(HashMap::from([("assets/a.svg".to_owned(), svg.to_vec())]));
        let mut cache = ImageCache::default();
        cache.set_root(PathBuf::from("no-such-folder"));
        platypus_core::with_asset_fs(Arc::new(fs), || {
            let tree = cache
                .svg_tree("assets/a.svg", "assets/a.svg", false)
                .unwrap();
            assert_eq!(svg_natural(&tree), (8, 6));
            assert!(
                cache
                    .svg_tree("assets/b.svg", "assets/b.svg", false)
                    .is_none()
            );
        });
        // Without a file system the (missing) folder is asked, as before.
        let mut cache = ImageCache::default();
        cache.set_root(PathBuf::from("no-such-folder"));
        assert!(
            cache
                .svg_tree("assets/a.svg", "assets/a.svg", false)
                .is_none()
        );
    }
}
