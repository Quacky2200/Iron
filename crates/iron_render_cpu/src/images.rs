//! `url(...)` images for the CPU renderer: rasters decoded with `image`, SVGs drawn with resvg at
//! the size they are shown at (cached by that size), both as premultiplied pixmaps.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use resvg::{tiny_skia, usvg};
use vello_cpu::peniko::ImageAlphaType;
use vello_cpu::{PixelMetadata, Pixmap};

struct Entry {
    /// The decoded raster, or the SVG at its natural size.
    pixmap: Arc<Pixmap>,
    /// Size in CSS pixels.
    natural: (u32, u32),
    svg: Option<Arc<usvg::Tree>>,
}

#[derive(Default)]
pub struct Images {
    root: Option<PathBuf>,
    entries: HashMap<String, Option<Entry>>,
    /// SVGs rasterized for a size: `url#WxH`.
    rasters: HashMap<String, Arc<Pixmap>>,
    svg_options: Option<usvg::Options<'static>>,
}

fn is_svg(url: &str) -> bool {
    let url = url.trim();
    if url.starts_with("data:") {
        return url.starts_with("data:image/svg+xml");
    }
    let path = url.split(['?', '#']).next().unwrap_or(url);
    path.to_ascii_lowercase().ends_with(".svg")
}

fn short(url: &str) -> &str {
    match url.char_indices().nth(60) {
        Some((i, _)) => &url[..i],
        None => url,
    }
}

fn to_pixmap(rgba: Vec<u8>, width: u32, height: u32) -> Option<Pixmap> {
    let (w, h) = (u16::try_from(width).ok()?, u16::try_from(height).ok()?);
    Some(Pixmap::from_parts(
        rgba,
        w,
        h,
        PixelMetadata {
            may_have_transparency: true,
            alpha_type: ImageAlphaType::Alpha,
        },
    ))
}

impl Images {
    pub fn set_root(&mut self, root: PathBuf) {
        self.root = Some(root);
        self.entries.clear();
        self.rasters.clear();
    }

    fn options(&mut self) -> &usvg::Options<'static> {
        self.svg_options.get_or_insert_with(|| {
            let mut options = usvg::Options::default();
            let db = Arc::make_mut(&mut options.fontdb);
            db.load_font_data(iron_ui::text::INTER.to_vec());
            db.load_font_data(iron_ui::text::INTER_ITALIC.to_vec());
            let family = db
                .faces()
                .next()
                .and_then(|f| f.families.first())
                .map_or_else(|| iron_ui::text::DEFAULT_FAMILY.to_owned(), |f| f.0.clone());
            db.set_sans_serif_family(family.clone());
            db.set_serif_family(family.clone());
            options.font_family = family;
            options
        })
    }

    fn bytes(&self, url: &str) -> Option<Vec<u8>> {
        if url.trim_start().starts_with("data:") {
            let bytes = iron_core::decode_data_url(url);
            if bytes.is_none() {
                tracing::warn!("image data URL couldn't be decoded");
            }
            return bytes;
        }
        let Some(root) = &self.root else {
            tracing::warn!("image `{url}` ignored: no asset folder set");
            return None;
        };
        iron_core::read_asset(root, url)
            .inspect_err(|e| tracing::warn!("image `{url}`: {e}"))
            .ok()
    }

    fn load(&mut self, url: &str) -> Option<Entry> {
        let bytes = self.bytes(url)?;
        if is_svg(url) {
            let tree = match usvg::Tree::from_data(&bytes, self.options()) {
                Ok(t) => Arc::new(t),
                Err(e) => {
                    tracing::warn!("SVG `{}` couldn't be parsed: {e}", short(url));
                    return None;
                }
            };
            let size = tree.size();
            let natural = (
                (size.width().ceil() as u32).max(1),
                (size.height().ceil() as u32).max(1),
            );
            let pixmap = Arc::new(rasterize(&tree, natural)?);
            return Some(Entry {
                pixmap,
                natural,
                svg: Some(tree),
            });
        }
        let image = match image::load_from_memory(&bytes) {
            Ok(i) => i.to_rgba8(),
            Err(e) => {
                tracing::warn!("image `{}` could not be loaded: {e}", short(url));
                return None;
            }
        };
        let (w, h) = image.dimensions();
        Some(Entry {
            pixmap: Arc::new(to_pixmap(image.into_raw(), w, h)?),
            natural: (w, h),
            svg: None,
        })
    }

    fn entry(&mut self, url: &str) -> Option<&Entry> {
        if !self.entries.contains_key(url) {
            let loaded = self.load(url);
            self.entries.insert(url.to_owned(), loaded);
        }
        self.entries.get(url)?.as_ref()
    }

    /// The image's natural size in CSS pixels.
    pub fn size(&mut self, url: &str) -> Option<(u32, u32)> {
        self.entry(url).map(|e| e.natural)
    }

    /// The pixels to draw `url` with at `px` physical pixels (an SVG is rasterized at about
    /// that size), and the image's natural size.
    pub fn get(&mut self, url: &str, px: (f32, f32)) -> Option<(Arc<Pixmap>, (u32, u32))> {
        let (pixmap, natural, svg) = {
            let e = self.entry(url)?;
            (e.pixmap.clone(), e.natural, e.svg.clone())
        };
        let Some(tree) = svg else {
            return Some((pixmap, natural));
        };
        // Rounded up to a multiple of 8, so small size changes reuse the raster.
        let bucket = |v: f32| (v.ceil().clamp(1.0, 4096.0) as u32).div_ceil(8) * 8;
        let size = (bucket(px.0), bucket(px.1));
        let key = format!("{url}#{}x{}", size.0, size.1);
        if let Some(r) = self.rasters.get(&key) {
            return Some((r.clone(), natural));
        }
        let raster = Arc::new(rasterize(&tree, size)?);
        self.rasters.insert(key, raster.clone());
        Some((raster, natural))
    }
}

fn rasterize(tree: &usvg::Tree, (w, h): (u32, u32)) -> Option<Pixmap> {
    let mut pixmap = tiny_skia::Pixmap::new(w, h)?;
    let size = tree.size();
    let transform = tiny_skia::Transform::from_scale(
        w as f32 / size.width().max(f32::EPSILON),
        h as f32 / size.height().max(f32::EPSILON),
    );
    resvg::render(tree, transform, &mut pixmap.as_mut());
    // tiny-skia's pixels are premultiplied RGBA8.
    let (w, h) = (pixmap.width(), pixmap.height());
    let (w, h) = (u16::try_from(w).ok()?, u16::try_from(h).ok()?);
    Some(Pixmap::from_parts(
        pixmap.take(),
        w,
        h,
        PixelMetadata {
            may_have_transparency: true,
            alpha_type: ImageAlphaType::AlphaPremultiplied,
        },
    ))
}
