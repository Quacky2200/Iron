// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

//! Textures for the world pass (Unity's `Texture2D` settings): sRGB pixels with a
//! filter mode, a wrap mode and optional mipmaps, made on the CPU.

/// How texels are blended when sampled (Unity's `FilterMode`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TextureFilter {
    /// Nearest texel: crisp pixel art.
    Point,
    /// Blends neighbouring texels; mip levels don't blend.
    #[default]
    Bilinear,
    /// Blends texels and mip levels.
    Trilinear,
}

/// What happens outside 0..1 (Unity's `TextureWrapMode`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TextureWrap {
    #[default]
    Repeat,
    Clamp,
    Mirror,
}

impl TextureFilter {
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "point" => Self::Point,
            "bilinear" => Self::Bilinear,
            "trilinear" => Self::Trilinear,
            _ => return None,
        })
    }
}

impl TextureWrap {
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "repeat" => Self::Repeat,
            "clamp" => Self::Clamp,
            "mirror" => Self::Mirror,
            _ => return None,
        })
    }
}

/// A texture to upload: straight-alpha RGBA8, top row first.
#[derive(Clone, Debug)]
pub struct TextureData {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    pub filter: TextureFilter,
    pub wrap: TextureWrap,
    /// Make smaller copies for distant, minified sampling.
    pub mipmaps: bool,
    /// Sharper at grazing angles (Unity's `anisoLevel`, 1 to 16; trilinear only),
    /// within what `QualitySettings.anisotropicFiltering` allows.
    pub anisotropy: u16,
    /// Colours in sRGB (photos, albedo). False for data such as normal maps, so the
    /// bytes are read (and mips averaged) as they are.
    pub srgb: bool,
}

impl TextureData {
    pub fn is_valid(&self) -> bool {
        self.width > 0
            && self.height > 0
            && self.rgba.len() == self.width as usize * self.height as usize * 4
    }
}

/// How a cooked texture's texels are stored.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TexelFormat {
    /// 4 bytes a texel.
    Rgba8,
    /// 8 bytes per 4 × 4 block: colour with on/off alpha (DXT1).
    Bc1,
    /// 16 bytes per 4 × 4 block: colour and smooth alpha (DXT5).
    Bc3,
}

impl TexelFormat {
    /// Bytes per 4 × 4 block (or per texel, for RGBA8).
    pub fn block_bytes(self) -> u32 {
        match self {
            Self::Rgba8 => 4,
            Self::Bc1 => 8,
            Self::Bc3 => 16,
        }
    }

    /// The bytes a `width` × `height` level takes.
    pub fn level_bytes(self, width: u32, height: u32) -> usize {
        match self {
            Self::Rgba8 => width as usize * height as usize * 4,
            _ => {
                width.div_ceil(4) as usize
                    * height.div_ceil(4) as usize
                    * self.block_bytes() as usize
            }
        }
    }
}

/// A texture prepared ahead of time (`platypus-game --cook`): every mip level made, possibly
/// block-compressed, ready to upload as it is.
#[derive(Clone, Debug, PartialEq)]
pub struct CookedTexture {
    pub width: u32,
    pub height: u32,
    pub format: TexelFormat,
    pub srgb: bool,
    /// Level 0 first, each half the size of the one before (at least 1).
    pub mips: Vec<Vec<u8>>,
    pub filter: TextureFilter,
    pub wrap: TextureWrap,
    pub anisotropy: u16,
}

impl CookedTexture {
    /// Whether each level holds the bytes its size needs (and BC sizes are whole blocks).
    pub fn is_valid(&self) -> bool {
        let block_ok = self.format == TexelFormat::Rgba8
            || (self.width.is_multiple_of(4) && self.height.is_multiple_of(4));
        block_ok
            && self.width > 0
            && self.height > 0
            && !self.mips.is_empty()
            && self.mips.iter().enumerate().all(|(i, m)| {
                let (w, h) = ((self.width >> i).max(1), (self.height >> i).max(1));
                m.len() == self.format.level_bytes(w, h)
            })
    }
}

/// The mip chain a texture's settings ask for (level 0 first; one level without mipmaps).
pub fn mip_chain(data: &TextureData) -> Vec<(Vec<u8>, u32, u32)> {
    let mut levels = vec![(data.rgba.clone(), data.width, data.height)];
    if data.mipmaps {
        while let Some((px, w, h)) = levels.last()
            && (*w > 1 || *h > 1)
        {
            let next = half(px, *w, *h, data.srgb);
            levels.push(next);
        }
    }
    levels
}

/// Unity's `QualitySettings.anisotropicFiltering`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Anisotropic {
    /// Never, whatever textures ask.
    Disable,
    /// As each texture's `anisotropy` asks.
    #[default]
    Enable,
    /// At least 9x on every trilinear texture (Unity's `ForceEnable`).
    ForceEnable,
}

impl Anisotropic {
    /// The samples a texture asking for `level` gets.
    pub fn level(self, level: u16) -> u16 {
        match self {
            Self::Disable => 1,
            Self::Enable => level.clamp(1, 16),
            Self::ForceEnable => level.clamp(9, 16),
        }
    }
}

/// What a sampler is made from: the texture's settings and the quality's.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct SamplerKey {
    pub filter: TextureFilter,
    pub wrap: TextureWrap,
    pub anisotropy: u16,
    /// Mips skipped at the top (Unity's `globalTextureMipmapLimit`).
    pub mip_limit: u32,
}

pub(crate) fn sampler(device: &wgpu::Device, key: SamplerKey) -> wgpu::Sampler {
    let SamplerKey {
        filter,
        wrap,
        anisotropy,
        mip_limit,
    } = key;
    let address = match wrap {
        TextureWrap::Repeat => wgpu::AddressMode::Repeat,
        TextureWrap::Clamp => wgpu::AddressMode::ClampToEdge,
        TextureWrap::Mirror => wgpu::AddressMode::MirrorRepeat,
    };
    let (texel, mip) = match filter {
        TextureFilter::Point => (wgpu::FilterMode::Nearest, wgpu::MipmapFilterMode::Nearest),
        TextureFilter::Bilinear => (wgpu::FilterMode::Linear, wgpu::MipmapFilterMode::Nearest),
        TextureFilter::Trilinear => (wgpu::FilterMode::Linear, wgpu::MipmapFilterMode::Linear),
    };
    device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("world texture"),
        address_mode_u: address,
        address_mode_v: address,
        address_mode_w: address,
        mag_filter: texel,
        min_filter: texel,
        mipmap_filter: mip,
        lod_min_clamp: mip_limit as f32,
        // Anisotropy needs every filter linear.
        anisotropy_clamp: if filter == TextureFilter::Trilinear {
            anisotropy.clamp(1, 16)
        } else {
            1
        },
        ..Default::default()
    })
}

/// sRGB byte to linear, and back: mip levels average light, not encoded values.
fn to_linear(c: u8) -> f32 {
    let c = f32::from(c) / 255.0;
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn to_srgb(l: f32) -> u8 {
    let l = l.clamp(0.0, 1.0);
    let c = if l <= 0.003_130_8 {
        l * 12.92
    } else {
        1.055 * l.powf(1.0 / 2.4) - 0.055
    };
    (c * 255.0 + 0.5) as u8
}

/// The next mip level: each texel averages up to 2×2 of `src` (`w` × `h`), weighted by
/// alpha so see-through texels don't darken their neighbours' colour. `srgb` false
/// averages the bytes as they are (data textures).
pub(crate) fn half(src: &[u8], w: u32, h: u32, srgb: bool) -> (Vec<u8>, u32, u32) {
    if !srgb {
        return half_linear(src, w, h);
    }
    let (nw, nh) = ((w / 2).max(1), (h / 2).max(1));
    let mut out = Vec::with_capacity(nw as usize * nh as usize * 4);
    for y in 0..nh {
        for x in 0..nw {
            let (mut rgb, mut a, mut n) = ([0.0f32; 3], 0.0f32, 0.0f32);
            for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                let (sx, sy) = ((x * 2 + dx).min(w - 1), (y * 2 + dy).min(h - 1));
                let i = (sy as usize * w as usize + sx as usize) * 4;
                let alpha = f32::from(src[i + 3]) / 255.0;
                for k in 0..3 {
                    rgb[k] += to_linear(src[i + k]) * alpha;
                }
                a += alpha;
                n += 1.0;
            }
            for c in rgb {
                out.push(if a > 0.0 { to_srgb(c / a) } else { 0 });
            }
            out.push((a / n * 255.0 + 0.5) as u8);
        }
    }
    (out, nw, nh)
}

fn half_linear(src: &[u8], w: u32, h: u32) -> (Vec<u8>, u32, u32) {
    let (nw, nh) = ((w / 2).max(1), (h / 2).max(1));
    let mut out = Vec::with_capacity(nw as usize * nh as usize * 4);
    for y in 0..nh {
        for x in 0..nw {
            let mut sum = [0u32; 4];
            for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                let (sx, sy) = ((x * 2 + dx).min(w - 1), (y * 2 + dy).min(h - 1));
                let i = (sy as usize * w as usize + sx as usize) * 4;
                for k in 0..4 {
                    sum[k] += u32::from(src[i + k]);
                }
            }
            out.extend(sum.map(|s| ((s + 2) / 4) as u8));
        }
    }
    (out, nw, nh)
}

/// The missing-texture checker: 4 × 4 squares of magenta and black, crisp and repeating.
pub fn missing() -> TextureData {
    let n = 8u32;
    let mut rgba = Vec::with_capacity((n * n * 4) as usize);
    for y in 0..n {
        for x in 0..n {
            let on = (x / 2 + y / 2) % 2 == 0;
            rgba.extend(if on {
                [255, 0, 255, 255]
            } else {
                [0, 0, 0, 255]
            });
        }
    }
    TextureData {
        width: n,
        height: n,
        rgba,
        filter: TextureFilter::Point,
        wrap: TextureWrap::Repeat,
        mipmaps: false,
        anisotropy: 1,
        srgb: true,
    }
}

/// A white spot fading smoothly to clear at its edge, `n` × `n`.
pub fn soft_spot(n: u32) -> TextureData {
    let mut rgba = Vec::with_capacity((n * n * 4) as usize);
    for y in 0..n {
        for x in 0..n {
            let at = |i: u32| (i as f32 + 0.5) / n as f32 * 2.0 - 1.0;
            let d = at(x).hypot(at(y));
            let a = (1.0 - d).clamp(0.0, 1.0);
            // Smoothstep-squared: a bright middle and a long, gentle tail.
            let a = a * a * (3.0 - 2.0 * a);
            rgba.extend([255, 255, 255, (a * 255.0 + 0.5) as u8]);
        }
    }
    TextureData {
        width: n,
        height: n,
        rgba,
        filter: TextureFilter::Trilinear,
        wrap: TextureWrap::Clamp,
        mipmaps: true,
        anisotropy: 1,
        srgb: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mip_levels_average_in_linear_light_and_ignore_clear_texels() {
        // Black and white average to linear 0.5 (sRGB 188), not sRGB 128.
        let src = [
            0, 0, 0, 255, 255, 255, 255, 255, 0, 0, 0, 255, 255, 255, 255, 255,
        ];
        let (m, w, h) = half(&src, 2, 2, true);
        assert_eq!((w, h), (1, 1));
        assert_eq!(&m[..], &[188, 188, 188, 255]);
        // A fully clear red texel adds no red.
        let src = [0, 0, 255, 255, 255, 0, 0, 0];
        let (m, _, _) = half(&src, 2, 1, true);
        assert_eq!(&m[..], &[0, 0, 255, 128]);
        // Data textures (normal maps) average their bytes.
        let (m, _, _) = half(&src, 2, 1, false);
        assert_eq!(&m[..], &[128, 0, 128, 128]);
    }
}
