// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

//! Icon conversion: a PNG (or any image the `image` crate reads) becomes an `.ico` with the
//! sizes Windows asks for; an `.ico` is passed through. Done in-process, no external tools. A
//! game without an icon gets the Platypus icon ([`default_ico`]); the `.ico` container itself is
//! `platypus_core::icon`'s.

use image::imageops::FilterType;
use image::{DynamicImage, ImageFormat, RgbaImage};
use platypus_core::icon::{DEFAULT_PNG, ICO_SIZES, write_ico};
pub use platypus_core::icon::{ico_sizes, is_ico};
use std::io::Cursor;

/// The sizes of a converted icon (Explorer's views and the taskbar at 100-200%).
pub const SIZES: [u32; 7] = ICO_SIZES;

/// `.ico` bytes for an icon file's contents: an `.ico` as it is, anything else converted to
/// [`SIZES`] (the image is centred on a transparent square first when it is not square;
/// sizes larger than the image are still produced by scaling up).
pub fn to_ico(bytes: &[u8]) -> Result<Vec<u8>, String> {
    if is_ico(bytes) {
        return Ok(bytes.to_vec());
    }
    let img = image::load_from_memory(bytes).map_err(|e| format!("icon: {e}"))?;
    let square = squared(&img);
    // A large source is brought to 256 once and the small sizes made from that: the same
    // picture as scaling each from the original, a fraction of the work.
    let base = if square.width() > 256 {
        scaled(&square, 256)
    } else {
        square
    };
    let mut pngs = Vec::new();
    for size in SIZES {
        let img = if size == base.width() {
            base.clone()
        } else {
            scaled(&base, size)
        };
        let mut png = Vec::new();
        img.write_to(&mut Cursor::new(&mut png), ImageFormat::Png)
            .map_err(|e| format!("icon: {e}"))?;
        pngs.push((size, png));
    }
    Ok(write_ico(&pngs))
}

/// The Platypus icon as an `.ico`, for games that have none.
pub fn default_ico() -> Vec<u8> {
    to_ico(DEFAULT_PNG).expect("the Platypus icon converts")
}

fn scaled(img: &RgbaImage, side: u32) -> RgbaImage {
    DynamicImage::ImageRgba8(img.clone())
        .resize_exact(side, side, FilterType::Lanczos3)
        .to_rgba8()
}

fn squared(img: &DynamicImage) -> RgbaImage {
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    if w == h {
        return rgba;
    }
    let side = w.max(h);
    let mut out = RgbaImage::new(side, side);
    image::imageops::overlay(
        &mut out,
        &rgba,
        i64::from((side - w) / 2),
        i64::from((side - h) / 2),
    );
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png(w: u32, h: u32) -> Vec<u8> {
        let img = RgbaImage::from_pixel(w, h, image::Rgba([200, 30, 30, 255]));
        let mut out = Vec::new();
        img.write_to(&mut Cursor::new(&mut out), ImageFormat::Png)
            .unwrap();
        out
    }

    fn all_sizes() -> Vec<(u32, u32)> {
        SIZES.iter().map(|&s| (s, s)).collect()
    }

    // M9-TS-2420
    #[test]
    fn a_png_becomes_an_ico_with_seven_sizes() {
        let ico = to_ico(&png(64, 64)).unwrap();
        assert!(is_ico(&ico));
        assert_eq!(ico_sizes(&ico), all_sizes());
        // The result decodes as an image.
        assert!(image::load_from_memory_with_format(&ico, ImageFormat::Ico).is_ok());
    }

    // M9-TS-2421
    #[test]
    fn a_wide_image_is_centred_on_a_square_and_an_ico_passes_through() {
        let ico = to_ico(&png(100, 40)).unwrap();
        assert_eq!(ico_sizes(&ico).len(), SIZES.len());
        assert_eq!(to_ico(&ico).unwrap(), ico);
        assert!(to_ico(b"not an image").is_err());
    }

    // M9-ICO-05
    #[test]
    fn the_default_icon_decodes_and_converts_to_every_size() {
        let img = image::load_from_memory_with_format(DEFAULT_PNG, ImageFormat::Png).unwrap();
        assert_eq!((img.width(), img.height()), (1254, 1254));
        let ico = default_ico();
        assert_eq!(
            ico_sizes(&ico),
            [16, 24, 32, 48, 64, 128, 256].map(|s| (s, s))
        );
        let decoded = image::load_from_memory_with_format(&ico, ImageFormat::Ico).unwrap();
        assert_eq!(decoded.width(), 256);
    }
}
