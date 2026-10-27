// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

//! Icon conversion: a PNG (or any image the `image` crate reads) becomes an `.ico` with the
//! sizes Windows asks for; an `.ico` is passed through. Done in-process, no external tools.

use image::imageops::FilterType;
use image::{DynamicImage, ImageFormat, RgbaImage};
use std::io::Cursor;

/// The sizes of a converted icon (Explorer's small, medium, large and jumbo views).
pub const SIZES: [u32; 4] = [16, 32, 48, 256];

/// `.ico` bytes for an icon file's contents: an `.ico` as it is, anything else converted to
/// [`SIZES`] (the image is centred on a transparent square first when it is not square;
/// sizes larger than the image are still produced by scaling up).
pub fn to_ico(bytes: &[u8]) -> Result<Vec<u8>, String> {
    if is_ico(bytes) {
        return Ok(bytes.to_vec());
    }
    let img = image::load_from_memory(bytes).map_err(|e| format!("icon: {e}"))?;
    let square = squared(&img);
    let mut pngs = Vec::new();
    for size in SIZES {
        let scaled = DynamicImage::ImageRgba8(square.clone())
            .resize_exact(size, size, FilterType::Lanczos3)
            .to_rgba8();
        let mut png = Vec::new();
        scaled
            .write_to(&mut Cursor::new(&mut png), ImageFormat::Png)
            .map_err(|e| format!("icon: {e}"))?;
        pngs.push((size, png));
    }
    Ok(write_ico(&pngs))
}

pub fn is_ico(bytes: &[u8]) -> bool {
    bytes.len() >= 22 && bytes[..4] == [0, 0, 1, 0] && u16::from_le_bytes([bytes[4], bytes[5]]) > 0
}

/// The (width, height) of every image in an `.ico` (256 is stored as 0).
pub fn ico_sizes(bytes: &[u8]) -> Vec<(u32, u32)> {
    if !is_ico(bytes) {
        return Vec::new();
    }
    let n = u16::from_le_bytes([bytes[4], bytes[5]]) as usize;
    (0..n)
        .filter_map(|i| bytes.get(6 + i * 16..6 + i * 16 + 2))
        .map(|e| {
            let d = |b: u8| if b == 0 { 256 } else { u32::from(b) };
            (d(e[0]), d(e[1]))
        })
        .collect()
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

/// An `.ico` file holding PNG-compressed images (Windows Vista and later read them at every size).
fn write_ico(images: &[(u32, Vec<u8>)]) -> Vec<u8> {
    let mut out = vec![0, 0, 1, 0];
    out.extend((images.len() as u16).to_le_bytes());
    let mut offset = 6 + 16 * images.len() as u32;
    for (size, png) in images {
        let dim = if *size >= 256 { 0 } else { *size as u8 };
        out.extend([dim, dim, 0, 0]);
        out.extend(1u16.to_le_bytes()); // planes
        out.extend(32u16.to_le_bytes()); // bits per pixel
        out.extend((png.len() as u32).to_le_bytes());
        out.extend(offset.to_le_bytes());
        offset += png.len() as u32;
    }
    for (_, png) in images {
        out.extend(png);
    }
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

    // M9-TS-2420
    #[test]
    fn a_png_becomes_an_ico_with_four_sizes() {
        let ico = to_ico(&png(64, 64)).unwrap();
        assert!(is_ico(&ico));
        assert_eq!(ico_sizes(&ico), [(16, 16), (32, 32), (48, 48), (256, 256)]);
        // The result decodes as an image.
        assert!(image::load_from_memory_with_format(&ico, ImageFormat::Ico).is_ok());
    }

    // M9-TS-2421
    #[test]
    fn a_wide_image_is_centred_on_a_square_and_an_ico_passes_through() {
        let ico = to_ico(&png(100, 40)).unwrap();
        assert_eq!(ico_sizes(&ico).len(), 4);
        assert_eq!(to_ico(&ico).unwrap(), ico);
        assert!(to_ico(b"not an image").is_err());
    }
}
