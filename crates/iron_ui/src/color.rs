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

    /// Parses `#rgb`, `#rgba`, `#rrggbb`, `#rrggbbaa`, `rgb()`/`rgba()` and
    /// `hsl()`/`hsla()` (comma or space separated, optional `/ alpha`), `hwb()`,
    /// `lab()`, `lch()`, `oklab()`, `oklch()`, `color()`, `color-mix()` (see
    /// [`crate::color_fn`]), `transparent` and the CSS named colours.
    pub fn parse(input: &str) -> Option<Self> {
        let s = input.trim().to_ascii_lowercase();
        if let Some(hex) = s.strip_prefix('#') {
            return parse_hex(hex);
        }
        let function_args = |names: [&str; 2]| {
            names
                .iter()
                .find_map(|n| s.strip_prefix(n))
                .and_then(|rest| rest.strip_suffix(')'))
        };
        if let Some(args) = function_args(["rgba(", "rgb("]) {
            return parse_rgb_args(args);
        }
        if let Some(args) = function_args(["hsla(", "hsl("]) {
            return parse_hsl_args(args);
        }
        if s.ends_with(')') {
            return crate::color_fn::parse(&s);
        }
        named(&s)
    }
}

/// Splits colour function arguments into 3 channels plus an optional alpha.
/// Accepts `a, b, c`, `a b c`, `a b c / alpha` and `a, b, c, alpha`.
pub(crate) fn split_color_args(args: &str) -> Option<([&str; 3], Option<&str>)> {
    let (channels, slash_alpha) = match args.split_once('/') {
        Some((c, a)) => (c, Some(a.trim())),
        None => (args, None),
    };
    let parts: Vec<&str> = channels
        .split([',', ' '])
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .collect();
    match (parts.as_slice(), slash_alpha) {
        ([a, b, c], alpha) => Some(([*a, *b, *c], alpha)),
        ([a, b, c, alpha], None) => Some(([*a, *b, *c], Some(*alpha))),
        _ => None,
    }
}

/// An alpha value: `0.5` or `50%`, clamped to `0..=1`.
fn parse_alpha(p: &str) -> Option<f32> {
    let v = match p.strip_suffix('%') {
        Some(pct) => pct.parse::<f32>().ok()? / 100.0,
        None => p.parse::<f32>().ok()?,
    };
    Some(v.clamp(0.0, 1.0))
}

fn parse_hsl_args(args: &str) -> Option<Rgba> {
    let ([h, s, l], alpha) = split_color_args(args)?;
    let hue = parse_angle_degrees(h)?;
    let pct = |p: &str| Some((p.strip_suffix('%')?.parse::<f32>().ok()? / 100.0).clamp(0.0, 1.0));
    let (s, l) = (pct(s)?, pct(l)?);
    let a = alpha.map_or(Some(1.0), parse_alpha)?;

    // CSS Color 4 `hsl()` to sRGB.
    let f = |n: f32| {
        let k = (n + hue / 30.0).rem_euclid(12.0);
        let amount = s * l.min(1.0 - l);
        l - amount * (k - 3.0).min(9.0 - k).clamp(-1.0, 1.0)
    };
    Some(Rgba::new(f(0.0), f(8.0), f(4.0), a))
}

/// An angle in degrees from `120`, `120deg`, `0.5turn`, `3.14rad` or `200grad`.
pub(crate) fn parse_angle_degrees(p: &str) -> Option<f32> {
    let p = p.trim();
    let (number, factor) = if let Some(n) = p.strip_suffix("deg") {
        (n, 1.0)
    } else if let Some(n) = p.strip_suffix("grad") {
        (n, 0.9)
    } else if let Some(n) = p.strip_suffix("rad") {
        (n, 180.0 / std::f32::consts::PI)
    } else if let Some(n) = p.strip_suffix("turn") {
        (n, 360.0)
    } else {
        (p, 1.0)
    };
    Some(number.trim().parse::<f32>().ok()? * factor)
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
    let (rgb, alpha) = split_color_args(args)?;
    let channel = |p: &str| -> Option<f32> {
        let v = match p.strip_suffix('%') {
            Some(pct) => pct.parse::<f32>().ok()? / 100.0,
            None => p.parse::<f32>().ok()? / 255.0,
        };
        Some(v.clamp(0.0, 1.0))
    };
    Some(Rgba::new(
        channel(rgb[0])?,
        channel(rgb[1])?,
        channel(rgb[2])?,
        alpha.map_or(Some(1.0), parse_alpha)?,
    ))
}

fn named(name: &str) -> Option<Rgba> {
    if name == "transparent" {
        return Some(Rgba::TRANSPARENT);
    }
    let index = NAMED_COLORS
        .binary_search_by(|(n, _)| (*n).cmp(name))
        .ok()?;
    let rgb = NAMED_COLORS[index].1;
    Some(Rgba::from_u8(
        (rgb >> 16) as u8,
        (rgb >> 8) as u8,
        rgb as u8,
        1.0,
    ))
}

/// The CSS Color 4 named colours, sorted by name for binary search.
const NAMED_COLORS: &[(&str, u32)] = &[
    ("aliceblue", 0xf0f8ff),
    ("antiquewhite", 0xfaebd7),
    ("aqua", 0x00ffff),
    ("aquamarine", 0x7fffd4),
    ("azure", 0xf0ffff),
    ("beige", 0xf5f5dc),
    ("bisque", 0xffe4c4),
    ("black", 0x000000),
    ("blanchedalmond", 0xffebcd),
    ("blue", 0x0000ff),
    ("blueviolet", 0x8a2be2),
    ("brown", 0xa52a2a),
    ("burlywood", 0xdeb887),
    ("cadetblue", 0x5f9ea0),
    ("chartreuse", 0x7fff00),
    ("chocolate", 0xd2691e),
    ("coral", 0xff7f50),
    ("cornflowerblue", 0x6495ed),
    ("cornsilk", 0xfff8dc),
    ("crimson", 0xdc143c),
    ("cyan", 0x00ffff),
    ("darkblue", 0x00008b),
    ("darkcyan", 0x008b8b),
    ("darkgoldenrod", 0xb8860b),
    ("darkgray", 0xa9a9a9),
    ("darkgreen", 0x006400),
    ("darkgrey", 0xa9a9a9),
    ("darkkhaki", 0xbdb76b),
    ("darkmagenta", 0x8b008b),
    ("darkolivegreen", 0x556b2f),
    ("darkorange", 0xff8c00),
    ("darkorchid", 0x9932cc),
    ("darkred", 0x8b0000),
    ("darksalmon", 0xe9967a),
    ("darkseagreen", 0x8fbc8f),
    ("darkslateblue", 0x483d8b),
    ("darkslategray", 0x2f4f4f),
    ("darkslategrey", 0x2f4f4f),
    ("darkturquoise", 0x00ced1),
    ("darkviolet", 0x9400d3),
    ("deeppink", 0xff1493),
    ("deepskyblue", 0x00bfff),
    ("dimgray", 0x696969),
    ("dimgrey", 0x696969),
    ("dodgerblue", 0x1e90ff),
    ("firebrick", 0xb22222),
    ("floralwhite", 0xfffaf0),
    ("forestgreen", 0x228b22),
    ("fuchsia", 0xff00ff),
    ("gainsboro", 0xdcdcdc),
    ("ghostwhite", 0xf8f8ff),
    ("gold", 0xffd700),
    ("goldenrod", 0xdaa520),
    ("gray", 0x808080),
    ("green", 0x008000),
    ("greenyellow", 0xadff2f),
    ("grey", 0x808080),
    ("honeydew", 0xf0fff0),
    ("hotpink", 0xff69b4),
    ("indianred", 0xcd5c5c),
    ("indigo", 0x4b0082),
    ("ivory", 0xfffff0),
    ("khaki", 0xf0e68c),
    ("lavender", 0xe6e6fa),
    ("lavenderblush", 0xfff0f5),
    ("lawngreen", 0x7cfc00),
    ("lemonchiffon", 0xfffacd),
    ("lightblue", 0xadd8e6),
    ("lightcoral", 0xf08080),
    ("lightcyan", 0xe0ffff),
    ("lightgoldenrodyellow", 0xfafad2),
    ("lightgray", 0xd3d3d3),
    ("lightgreen", 0x90ee90),
    ("lightgrey", 0xd3d3d3),
    ("lightpink", 0xffb6c1),
    ("lightsalmon", 0xffa07a),
    ("lightseagreen", 0x20b2aa),
    ("lightskyblue", 0x87cefa),
    ("lightslategray", 0x778899),
    ("lightslategrey", 0x778899),
    ("lightsteelblue", 0xb0c4de),
    ("lightyellow", 0xffffe0),
    ("lime", 0x00ff00),
    ("limegreen", 0x32cd32),
    ("linen", 0xfaf0e6),
    ("magenta", 0xff00ff),
    ("maroon", 0x800000),
    ("mediumaquamarine", 0x66cdaa),
    ("mediumblue", 0x0000cd),
    ("mediumorchid", 0xba55d3),
    ("mediumpurple", 0x9370db),
    ("mediumseagreen", 0x3cb371),
    ("mediumslateblue", 0x7b68ee),
    ("mediumspringgreen", 0x00fa9a),
    ("mediumturquoise", 0x48d1cc),
    ("mediumvioletred", 0xc71585),
    ("midnightblue", 0x191970),
    ("mintcream", 0xf5fffa),
    ("mistyrose", 0xffe4e1),
    ("moccasin", 0xffe4b5),
    ("navajowhite", 0xffdead),
    ("navy", 0x000080),
    ("oldlace", 0xfdf5e6),
    ("olive", 0x808000),
    ("olivedrab", 0x6b8e23),
    ("orange", 0xffa500),
    ("orangered", 0xff4500),
    ("orchid", 0xda70d6),
    ("palegoldenrod", 0xeee8aa),
    ("palegreen", 0x98fb98),
    ("paleturquoise", 0xafeeee),
    ("palevioletred", 0xdb7093),
    ("papayawhip", 0xffefd5),
    ("peachpuff", 0xffdab9),
    ("peru", 0xcd853f),
    ("pink", 0xffc0cb),
    ("plum", 0xdda0dd),
    ("powderblue", 0xb0e0e6),
    ("purple", 0x800080),
    ("rebeccapurple", 0x663399),
    ("red", 0xff0000),
    ("rosybrown", 0xbc8f8f),
    ("royalblue", 0x4169e1),
    ("saddlebrown", 0x8b4513),
    ("salmon", 0xfa8072),
    ("sandybrown", 0xf4a460),
    ("seagreen", 0x2e8b57),
    ("seashell", 0xfff5ee),
    ("sienna", 0xa0522d),
    ("silver", 0xc0c0c0),
    ("skyblue", 0x87ceeb),
    ("slateblue", 0x6a5acd),
    ("slategray", 0x708090),
    ("slategrey", 0x708090),
    ("snow", 0xfffafa),
    ("springgreen", 0x00ff7f),
    ("steelblue", 0x4682b4),
    ("tan", 0xd2b48c),
    ("teal", 0x008080),
    ("thistle", 0xd8bfd8),
    ("tomato", 0xff6347),
    ("turquoise", 0x40e0d0),
    ("violet", 0xee82ee),
    ("wheat", 0xf5deb3),
    ("white", 0xffffff),
    ("whitesmoke", 0xf5f5f5),
    ("yellow", 0xffff00),
    ("yellowgreen", 0x9acd32),
];

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

    // M2-VIS-01
    #[test]
    fn parses_hsl() {
        assert_eq!(
            rgba8(Rgba::parse("hsl(120 100% 25%)").unwrap()),
            [0, 128, 0, 255]
        );
        assert_eq!(
            rgba8(Rgba::parse("hsl(0, 100%, 50%)").unwrap()),
            [255, 0, 0, 255]
        );
        assert_eq!(
            rgba8(Rgba::parse("hsl(240deg 100% 50%)").unwrap()),
            [0, 0, 255, 255]
        );
        assert_eq!(
            rgba8(Rgba::parse("hsl(0.5turn 100% 50% / 50%)").unwrap()),
            [0, 255, 255, 128]
        );
        assert_eq!(
            rgba8(Rgba::parse("hsla(60, 100%, 50%, 0.25)").unwrap()),
            [255, 255, 0, 64]
        );
        assert_eq!(
            rgba8(Rgba::parse("hsl(0 0% 100%)").unwrap()),
            [255, 255, 255, 255]
        );
        assert_eq!(
            Rgba::parse("hsl(120 100 50)"),
            None,
            "saturation/lightness need %"
        );
    }

    // M2-VIS-01
    #[test]
    fn parses_every_named_colour() {
        assert!(
            NAMED_COLORS.windows(2).all(|w| w[0].0 < w[1].0),
            "NAMED_COLORS must stay sorted for binary search"
        );
        assert_eq!(NAMED_COLORS.len(), 148);
        assert_eq!(
            rgba8(Rgba::parse("rebeccapurple").unwrap()),
            [0x66, 0x33, 0x99, 255]
        );
        assert_eq!(
            rgba8(Rgba::parse("LightGoldenRodYellow").unwrap()),
            [0xfa, 0xfa, 0xd2, 255]
        );
        assert_eq!(
            rgba8(Rgba::parse("aliceblue").unwrap()),
            [0xf0, 0xf8, 0xff, 255]
        );
        assert_eq!(
            rgba8(Rgba::parse("yellowgreen").unwrap()),
            [0x9a, 0xcd, 0x32, 255]
        );
    }
}
