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

/// A texture to upload: straight-alpha sRGB RGBA8, top row first.
#[derive(Clone, Debug)]
pub struct TextureData {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    pub filter: TextureFilter,
    pub wrap: TextureWrap,
    /// Make smaller copies for distant, minified sampling.
    pub mipmaps: bool,
}

impl TextureData {
    pub fn is_valid(&self) -> bool {
        self.width > 0
            && self.height > 0
            && self.rgba.len() == self.width as usize * self.height as usize * 4
    }
}

pub(crate) fn sampler(
    device: &wgpu::Device,
    filter: TextureFilter,
    wrap: TextureWrap,
) -> wgpu::Sampler {
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
        ..Default::default()
    })
}

/// sRGB byte → linear, and back: mip levels average light, not encoded values.
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
/// alpha so see-through texels don't darken their neighbours' colour.
pub(crate) fn half(src: &[u8], w: u32, h: u32) -> (Vec<u8>, u32, u32) {
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
        let (m, w, h) = half(&src, 2, 2);
        assert_eq!((w, h), (1, 1));
        assert_eq!(&m[..], &[188, 188, 188, 255]);
        // A fully clear red texel adds no red.
        let src = [0, 0, 255, 255, 255, 0, 0, 0];
        let (m, _, _) = half(&src, 2, 1);
        assert_eq!(&m[..], &[0, 0, 255, 128]);
    }
}
