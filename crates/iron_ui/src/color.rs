/// An sRGB colour with straight (not premultiplied) alpha, each channel in `0.0..=1.0`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rgba {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Rgba {
    pub const TRANSPARENT: Rgba = Rgba::new(0.0, 0.0, 0.0, 0.0);

    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    fn from_u8(r: u8, g: u8, b: u8, a: f32) -> Self {
        Self::new(
            f32::from(r) / 255.0,
            f32::from(g) / 255.0,
            f32::from(b) / 255.0,
            a,
        )
    }

    pub fn with_alpha_scaled(self, factor: f32) -> Self {
        Self {
            a: self.a * factor,
            ..self
        }
    }

    /// Parses `#rgb`, `#rgba`, `#rrggbb`, `#rrggbbaa`, `rgb()`/`rgba()` (comma or
    /// space separated, optional `/ alpha`), `transparent` and a few named colours.
    pub fn parse(input: &str) -> Option<Self> {
        let s = input.trim().to_ascii_lowercase();
        if let Some(hex) = s.strip_prefix('#') {
            return parse_hex(hex);
        }
        if let Some(args) = s
            .strip_prefix("rgba(")
            .or_else(|| s.strip_prefix("rgb("))
            .and_then(|rest| rest.strip_suffix(')'))
        {
            return parse_rgb_args(args);
        }
        named(&s)
    }
}

fn parse_hex(hex: &str) -> Option<Rgba> {
    if !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let nibble = |i: usize| u8::from_str_radix(&hex[i..=i], 16).ok().map(|v| v * 17);
    let byte = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).ok();
    match hex.len() {
        3 => Some(Rgba::from_u8(nibble(0)?, nibble(1)?, nibble(2)?, 1.0)),
        4 => Some(Rgba::from_u8(
            nibble(0)?,
            nibble(1)?,
            nibble(2)?,
            f32::from(nibble(3)?) / 255.0,
        )),
        6 => Some(Rgba::from_u8(byte(0)?, byte(2)?, byte(4)?, 1.0)),
        8 => Some(Rgba::from_u8(
            byte(0)?,
            byte(2)?,
            byte(4)?,
            f32::from(byte(6)?) / 255.0,
        )),
        _ => None,
    }
}

fn parse_rgb_args(args: &str) -> Option<Rgba> {
    // `rgb(1, 2, 3)`, `rgb(1 2 3)`, `rgb(1 2 3 / 0.5)`, `rgba(1, 2, 3, 0.5)`.
    let (channels, slash_alpha) = match args.split_once('/') {
        Some((c, a)) => (c, Some(a)),
        None => (args, None),
    };
    let parts: Vec<&str> = channels
        .split([',', ' '])
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .collect();
    let channel = |p: &str| -> Option<f32> {
        let v = match p.strip_suffix('%') {
            Some(pct) => pct.parse::<f32>().ok()? / 100.0,
            None => p.parse::<f32>().ok()? / 255.0,
        };
        Some(v.clamp(0.0, 1.0))
    };
    let alpha = |p: &str| -> Option<f32> {
        let p = p.trim();
        let v = match p.strip_suffix('%') {
            Some(pct) => pct.parse::<f32>().ok()? / 100.0,
            None => p.parse::<f32>().ok()?,
        };
        Some(v.clamp(0.0, 1.0))
    };
    let (rgb, a) = match (parts.as_slice(), slash_alpha) {
        ([r, g, b], None) => ([*r, *g, *b], 1.0),
        ([r, g, b], Some(a)) => ([*r, *g, *b], alpha(a)?),
        ([r, g, b, a], None) => ([*r, *g, *b], alpha(a)?),
        _ => return None,
    };
    Some(Rgba::new(
        channel(rgb[0])?,
        channel(rgb[1])?,
        channel(rgb[2])?,
        a,
    ))
}

fn named(name: &str) -> Option<Rgba> {
    let (r, g, b) = match name {
        "transparent" => return Some(Rgba::TRANSPARENT),
        "black" => (0, 0, 0),
        "white" => (255, 255, 255),
        "red" => (255, 0, 0),
        "green" => (0, 128, 0),
        "lime" => (0, 255, 0),
        "blue" => (0, 0, 255),
        "yellow" => (255, 255, 0),
        "cyan" | "aqua" => (0, 255, 255),
        "magenta" | "fuchsia" => (255, 0, 255),
        "gray" | "grey" => (128, 128, 128),
        "orange" => (255, 165, 0),
        "purple" => (128, 0, 128),
        _ => return None,
    };
    Some(Rgba::from_u8(r, g, b, 1.0))
}

// M2-STY-02
#[cfg(test)]
mod tests {
    use super::*;

    fn rgba8(c: Rgba) -> [u8; 4] {
        [c.r, c.g, c.b, c.a].map(|v| (v * 255.0).round() as u8)
    }

    #[test]
    fn parses_hex_forms() {
        assert_eq!(rgba8(Rgba::parse("#f80").unwrap()), [255, 136, 0, 255]);
        assert_eq!(rgba8(Rgba::parse("#ff8800").unwrap()), [255, 136, 0, 255]);
        assert_eq!(rgba8(Rgba::parse("#ff880080").unwrap()), [255, 136, 0, 128]);
        assert_eq!(rgba8(Rgba::parse("#F808").unwrap()), [255, 136, 0, 136]);
    }

    #[test]
    fn parses_rgb_functions() {
        assert_eq!(
            rgba8(Rgba::parse("rgb(255 136 0)").unwrap()),
            [255, 136, 0, 255]
        );
        assert_eq!(
            rgba8(Rgba::parse("rgb(255, 136, 0)").unwrap()),
            [255, 136, 0, 255]
        );
        assert_eq!(
            rgba8(Rgba::parse("rgba(255,136,0,0.5)").unwrap()),
            [255, 136, 0, 128]
        );
        assert_eq!(
            rgba8(Rgba::parse("rgb(100% 0% 0% / 25%)").unwrap()),
            [255, 0, 0, 64]
        );
    }

    #[test]
    fn parses_names() {
        assert_eq!(Rgba::parse("transparent"), Some(Rgba::TRANSPARENT));
        assert_eq!(rgba8(Rgba::parse("White").unwrap()), [255, 255, 255, 255]);
    }

    #[test]
    fn rejects_bad_colours() {
        for bad in [
            "",
            "#12",
            "#gggggg",
            "rgb(1, 2)",
            "rgb(1 2 3 4 5)",
            "notacolour",
            "rgb(a b c)",
        ] {
            assert_eq!(Rgba::parse(bad), None, "{bad:?} should not parse");
        }
    }
}
