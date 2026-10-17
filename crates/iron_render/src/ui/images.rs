//! Background images: loading `url(...)` files from the game's folder into
//! textures (cached by URL), and CSS `background-size` / `background-position`.

use std::collections::HashMap;
use std::path::PathBuf;

use iron_ui::{
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
}

impl ImageCache {
    pub fn set_root(&mut self, root: PathBuf) {
        self.root = Some(root);
        self.entries.clear();
    }

    pub fn get(
        &mut self,
        gpu: &Gpu,
        layout: &wgpu::BindGroupLayout,
        samplers: &ImageSamplers,
        url: &str,
    ) -> Option<&LoadedImage> {
        if !self.entries.contains_key(url) {
            let loaded = self.load(gpu, layout, samplers, url);
            self.entries.insert(url.to_owned(), loaded);
        }
        self.entries.get(url)?.as_ref()
    }

    fn load(
        &self,
        gpu: &Gpu,
        layout: &wgpu::BindGroupLayout,
        samplers: &ImageSamplers,
        url: &str,
    ) -> Option<LoadedImage> {
        let decoded = if url.trim_start().starts_with("data:") {
            // Inlined by the bundler (small images referenced from CSS).
            let Some(bytes) = iron_core::decode_data_url(url) else {
                tracing::warn!("background image data URL couldn't be decoded");
                return None;
            };
            image::load_from_memory(&bytes)
        } else {
            let Some(root) = &self.root else {
                tracing::warn!("background image `{url}` ignored: no asset folder set");
                return None;
            };
            let Some(path) = iron_core::resolve_asset(root, url) else {
                tracing::warn!(
                    "background image `{url}` rejected: it must be a relative path inside the game folder"
                );
                return None;
            };
            image::open(&path)
        };
        let image = match decoded {
            Ok(i) => i.to_rgba8(),
            Err(e) => {
                tracing::warn!("background image `{}` could not be loaded: {e}", short(url));
                return None;
            }
        };
        let (w, h) = image.dimensions();
        let size = wgpu::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        };
        let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some(url),
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
            &image,
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
                label: Some(url),
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
        let bind_group = bind(&samplers.smooth);
        let bind_group_nearest = bind(&samplers.nearest);
        tracing::debug!("loaded background image `{url}` ({w}x{h})");
        Some(LoadedImage {
            bind_group,
            bind_group_nearest,
            size: (w, h),
        })
    }
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
}
