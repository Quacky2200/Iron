//! Background images: loading `url(...)` files from the game's folder into
//! textures (cached by URL), and CSS `background-size` / `background-position`.

use std::collections::HashMap;
use std::path::PathBuf;

use iron_ui::{BackgroundPosition, BackgroundSize, Len, Rect};

use crate::Gpu;

pub struct LoadedImage {
    pub bind_group: wgpu::BindGroup,
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
        sampler: &wgpu::Sampler,
        url: &str,
    ) -> Option<&LoadedImage> {
        if !self.entries.contains_key(url) {
            let loaded = self.load(gpu, layout, sampler, url);
            self.entries.insert(url.to_owned(), loaded);
        }
        self.entries.get(url)?.as_ref()
    }

    fn load(
        &self,
        gpu: &Gpu,
        layout: &wgpu::BindGroupLayout,
        sampler: &wgpu::Sampler,
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
        let bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
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
        });
        tracing::debug!("loaded background image `{url}` ({w}x{h})");
        Some(LoadedImage {
            bind_group,
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
}
