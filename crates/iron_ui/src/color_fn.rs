//! CSS Color 4/5 functions beyond `rgb()` / `hsl()`: `hwb()`, `lab()`, `lch()`,
//! `oklab()`, `oklch()`, `color(srgb | srgb-linear | display-p3 ...)` and
//! `color-mix()`, plus s&box's `darken()`, `lighten()` and `mix()` (which work like
//! their SCSS namesakes; `color-mix()` is the standard way to do the same).
//! Conversions follow the CSS Color 4 sample code; out-of-gamut results are clipped.

use crate::Rgba;
use crate::color::parse_angle_degrees;
use crate::paint::split_top_level;

/// Parses one of the functions above (`s` is lower-case). `None` if `s` isn't one of
/// them or is invalid.
pub(crate) fn parse(s: &str) -> Option<Rgba> {
    let (name, args) = s.strip_suffix(')')?.split_once('(')?;
    let args = args.trim();
    match name.trim() {
        "hwb" => hwb(args),
        "lab" => {
            let ([l, a, b], alpha) = channels(args)?;
            let lab = [pct_or(l, 100.0)?, pct_or(a, 125.0)?, pct_or(b, 125.0)?];
            Some(from_linear(lab_to_linear(lab), alpha))
        }
        "lch" => {
            let ([l, c, h], alpha) = channels(args)?;
            let lab = polar_to_rect(pct_or(l, 100.0)?, pct_or(c, 150.0)?, hue(h)?);
            Some(from_linear(lab_to_linear(lab), alpha))
        }
        "oklab" => {
            let ([l, a, b], alpha) = channels(args)?;
            let lab = [pct_or(l, 1.0)?, pct_or(a, 0.4)?, pct_or(b, 0.4)?];
            Some(from_linear(oklab_to_linear(lab), alpha))
        }
        "oklch" => {
            let ([l, c, h], alpha) = channels(args)?;
            let lab = polar_to_rect(pct_or(l, 1.0)?, pct_or(c, 0.4)?, hue(h)?);
            Some(from_linear(oklab_to_linear(lab), alpha))
        }
        "color" => color_fn(args),
        "color-mix" => color_mix(args),
        // s&box (SCSS-style) helpers.
        "darken" => shade(args, -1.0),
        "lighten" => shade(args, 1.0),
        "mix" => scss_mix(args),
        _ => None,
    }
}

/// Three channels and an alpha (`a b c / alpha`); `none` reads as 0.
fn channels(args: &str) -> Option<([&str; 3], Option<&str>)> {
    let (main, alpha) = match args.split_once('/') {
        Some((m, a)) => (m, Some(a.trim())),
        None => (args, None),
    };
    let parts: Vec<&str> = main.split_whitespace().collect();
    let [a, b, c] = parts[..] else {
        return None;
    };
    Some(([a, b, c], alpha))
}

/// A number, or a percentage of `full`.
fn pct_or(p: &str, full: f32) -> Option<f32> {
    if p == "none" {
        return Some(0.0);
    }
    match p.strip_suffix('%') {
        Some(n) => Some(n.parse::<f32>().ok()? / 100.0 * full),
        None => p.parse().ok(),
    }
}

fn hue(p: &str) -> Option<f32> {
    if p == "none" {
        return Some(0.0);
    }
    parse_angle_degrees(p)
}

fn alpha_of(alpha: Option<&str>) -> Option<f32> {
    match alpha {
        None => Some(1.0),
        Some(a) => Some(pct_or(a, 1.0)?.clamp(0.0, 1.0)),
    }
}

fn polar_to_rect(l: f32, c: f32, h_deg: f32) -> [f32; 3] {
    let h = h_deg.to_radians();
    [l, c.max(0.0) * h.cos(), c.max(0.0) * h.sin()]
}

fn rect_to_polar([l, a, b]: [f32; 3]) -> [f32; 3] {
    let c = (a * a + b * b).sqrt();
    let h = b.atan2(a).to_degrees().rem_euclid(360.0);
    [l, c, h]
}

/// Linear-light sRGB plus alpha to an [`Rgba`] (gamma-encoded, clipped). Returns
/// `None` alpha strings that don't parse via the caller's `?`.
fn from_linear(lin: [f32; 3], alpha: Option<&str>) -> Rgba {
    let [r, g, b] = lin.map(|v| gamma(v).clamp(0.0, 1.0));
    Rgba::new(r, g, b, alpha_of(alpha).unwrap_or(1.0))
}

fn gamma(v: f32) -> f32 {
    let a = v.abs();
    let e = if a <= 0.003_130_8 {
        12.92 * a
    } else {
        1.055 * a.powf(1.0 / 2.4) - 0.055
    };
    e.copysign(v)
}

fn linearize(v: f32) -> f32 {
    let a = v.abs();
    let l = if a <= 0.040_45 {
        a / 12.92
    } else {
        ((a + 0.055) / 1.055).powf(2.4)
    };
    l.copysign(v)
}

fn mul(m: [[f32; 3]; 3], v: [f32; 3]) -> [f32; 3] {
    m.map(|row| row[0] * v[0] + row[1] * v[1] + row[2] * v[2])
}

const XYZ65_TO_LINEAR_SRGB: [[f32; 3]; 3] = [
    [3.240_97, -1.537_383_2, -0.498_610_76],
    [-0.969_243_65, 1.875_967_5, 0.041_555_06],
    [0.055_630_08, -0.203_976_96, 1.056_971_5],
];
const LINEAR_SRGB_TO_XYZ65: [[f32; 3]; 3] = [
    [0.412_390_8, 0.357_584_33, 0.180_480_8],
    [0.212_639, 0.715_168_7, 0.072_192_32],
    [0.019_330_82, 0.119_194_78, 0.950_532_14],
];
const D50_TO_D65: [[f32; 3]; 3] = [
    [0.955_473_4, -0.023_098_455, 0.063_259_244],
    [-0.028_369_71, 1.009_995_4, 0.021_041_441],
    [0.012_314_015, -0.020_507_65, 1.330_365_9],
];
const D65_TO_D50: [[f32; 3]; 3] = [
    [1.047_929_8, 0.022_946_793, -0.050_192_23],
    [0.029_627_815, 0.990_434_5, -0.017_073_825],
    [-0.009_243_058, 0.015_055_145, 0.751_874_3],
];
const LINEAR_P3_TO_XYZ65: [[f32; 3]; 3] = [
    [0.486_570_95, 0.265_667_7, 0.198_217_29],
    [0.228_974_56, 0.691_738_5, 0.079_286_91],
    [0.0, 0.045_113_38, 1.043_944_4],
];
const D50_WHITE: [f32; 3] = [0.964_22, 1.0, 0.825_21];
const KAPPA: f32 = 24389.0 / 27.0;
const EPSILON: f32 = 216.0 / 24389.0;

fn lab_to_linear([l, a, b]: [f32; 3]) -> [f32; 3] {
    let fy = (l + 16.0) / 116.0;
    let fx = a / 500.0 + fy;
    let fz = fy - b / 200.0;
    let cube = |f: f32| {
        let f3 = f * f * f;
        if f3 > EPSILON {
            f3
        } else {
            (116.0 * f - 16.0) / KAPPA
        }
    };
    let y = if l > KAPPA * EPSILON {
        fy * fy * fy
    } else {
        l / KAPPA
    };
    let xyz50 = [cube(fx) * D50_WHITE[0], y, cube(fz) * D50_WHITE[2]];
    mul(XYZ65_TO_LINEAR_SRGB, mul(D50_TO_D65, xyz50))
}

fn linear_to_lab(lin: [f32; 3]) -> [f32; 3] {
    let xyz50 = mul(D65_TO_D50, mul(LINEAR_SRGB_TO_XYZ65, lin));
    let f = |t: f32| {
        if t > EPSILON {
            t.cbrt()
        } else {
            (KAPPA * t + 16.0) / 116.0
        }
    };
    let [fx, fy, fz] = [
        f(xyz50[0] / D50_WHITE[0]),
        f(xyz50[1]),
        f(xyz50[2] / D50_WHITE[2]),
    ];
    [116.0 * fy - 16.0, 500.0 * (fx - fy), 200.0 * (fy - fz)]
}

fn oklab_to_linear([l, a, b]: [f32; 3]) -> [f32; 3] {
    let l_ = l + 0.396_337_78 * a + 0.215_803_76 * b;
    let m_ = l - 0.105_561_346 * a - 0.063_854_17 * b;
    let s_ = l - 0.089_484_18 * a - 1.291_485_5 * b;
    let (l3, m3, s3) = (l_ * l_ * l_, m_ * m_ * m_, s_ * s_ * s_);
    [
        4.076_741_7 * l3 - 3.307_711_6 * m3 + 0.230_969_94 * s3,
        -1.268_438 * l3 + 2.609_757_4 * m3 - 0.341_319_38 * s3,
        -0.004_196_086_3 * l3 - 0.703_418_6 * m3 + 1.707_614_7 * s3,
    ]
}

fn linear_to_oklab([r, g, b]: [f32; 3]) -> [f32; 3] {
    let l = (0.412_221_46 * r + 0.536_332_55 * g + 0.051_445_995 * b).cbrt();
    let m = (0.211_903_5 * r + 0.680_699_5 * g + 0.107_396_96 * b).cbrt();
    let s = (0.088_302_46 * r + 0.281_718_85 * g + 0.629_978_7 * b).cbrt();
    [
        0.210_454_26 * l + 0.793_617_8 * m - 0.004_072_047 * s,
        1.977_998_5 * l - 2.428_592_2 * m + 0.450_593_7 * s,
        0.025_904_037 * l + 0.782_771_77 * m - 0.808_675_77 * s,
    ]
}

fn hwb(args: &str) -> Option<Rgba> {
    let ([h, w, b], alpha) = match crate::color::split_color_args(args) {
        Some(parts) => parts,
        None => channels(args)?,
    };
    let (h, w, b) = (
        hue(h)?,
        pct_or(w, 1.0)? / percent_scale(w),
        pct_or(b, 1.0)? / percent_scale(b),
    );
    let a = alpha_of(alpha)?;
    if w + b >= 1.0 {
        let gray = w / (w + b);
        return Some(Rgba::new(gray, gray, gray, a));
    }
    let pure = Rgba::parse(&format!("hsl({h}deg 100% 50%)"))?;
    let f = |c: f32| c * (1.0 - w - b) + w;
    Some(Rgba::new(f(pure.r), f(pure.g), f(pure.b), a))
}

/// `hwb()` whiteness/blackness are percentages, or numbers out of 100.
fn percent_scale(p: &str) -> f32 {
    if p.ends_with('%') { 1.0 } else { 100.0 }
}

/// `color(srgb | srgb-linear | display-p3 r g b [/ a])`, channels 0..1 or percentages.
fn color_fn(args: &str) -> Option<Rgba> {
    let (space, rest) = args.split_once(char::is_whitespace)?;
    let ([r, g, b], alpha) = channels(rest.trim())?;
    let c = [pct_or(r, 1.0)?, pct_or(g, 1.0)?, pct_or(b, 1.0)?];
    let lin = match space {
        "srgb" => c.map(linearize),
        "srgb-linear" => c,
        "display-p3" => mul(
            XYZ65_TO_LINEAR_SRGB,
            mul(LINEAR_P3_TO_XYZ65, c.map(linearize)),
        ),
        _ => return None,
    };
    let _ = alpha_of(alpha)?;
    Some(from_linear(lin, alpha))
}

/// `color-mix(in <space>, <color> [<p>%], <color> [<p>%])` with the spaces `srgb`,
/// `srgb-linear`, `lab`, `oklab`, `lch` and `oklch` (polar spaces mix hues the short
/// way round). Mixing is premultiplied, like CSS.
fn color_mix(args: &str) -> Option<Rgba> {
    let parts = split_top_level(args, ',');
    let [space, a, b] = parts[..] else {
        return None;
    };
    let space = space.strip_prefix("in")?.trim();
    let (ca, pa) = mix_arg(a)?;
    let (cb, pb) = mix_arg(b)?;
    let (pa, pb) = match (pa, pb) {
        (None, None) => (0.5, 0.5),
        (Some(p), None) => (p, 1.0 - p),
        (None, Some(p)) => (1.0 - p, p),
        (Some(p), Some(q)) => (p, q),
    };
    let sum = pa + pb;
    if sum <= 0.0 || pa < 0.0 || pb < 0.0 {
        return None;
    }
    let (wa, wb) = (pa / sum, pb / sum);
    // Weights adding up to less than 100% make the result partly transparent.
    let alpha_scale = sum.min(1.0);

    let to_space = |c: Rgba| -> Option<[f32; 3]> {
        let srgb = [c.r, c.g, c.b];
        let lin = srgb.map(linearize);
        Some(match space {
            "srgb" => srgb,
            "srgb-linear" => lin,
            "lab" => linear_to_lab(lin),
            "oklab" => linear_to_oklab(lin),
            "lch" => rect_to_polar(linear_to_lab(lin)),
            "oklch" => rect_to_polar(linear_to_oklab(lin)),
            _ => return None,
        })
    };
    let polar = matches!(space, "lch" | "oklch");
    let (xa, xb) = (to_space(ca)?, to_space(cb)?);
    let alpha = ca.a * wa + cb.a * wb;
    let mut mixed = [0.0; 3];
    for i in 0..3 {
        if polar && i == 2 {
            // Hue: the shorter way round, not premultiplied.
            let mut d = xb[2] - xa[2];
            if d > 180.0 {
                d -= 360.0;
            } else if d < -180.0 {
                d += 360.0;
            }
            mixed[2] = (xa[2] + d * wb).rem_euclid(360.0);
        } else {
            let premultiplied = xa[i] * ca.a * wa + xb[i] * cb.a * wb;
            mixed[i] = if alpha > 0.0 {
                premultiplied / alpha
            } else {
                0.0
            };
        }
    }
    let lin = match space {
        "srgb" => mixed.map(linearize),
        "srgb-linear" => mixed,
        "lab" => lab_to_linear(mixed),
        "oklab" => oklab_to_linear(mixed),
        "lch" => lab_to_linear(polar_to_rect(mixed[0], mixed[1], mixed[2])),
        _ => oklab_to_linear(polar_to_rect(mixed[0], mixed[1], mixed[2])),
    };
    let [r, g, b] = lin.map(|v| gamma(v).clamp(0.0, 1.0));
    Some(Rgba::new(r, g, b, (alpha * alpha_scale).clamp(0.0, 1.0)))
}

/// `darken(color, amount)` / `lighten(color, amount)`: moves HSL lightness down or up
/// by `amount` (`20%` or `0.2`), like SCSS.
fn shade(args: &str, sign: f32) -> Option<Rgba> {
    let parts = split_top_level(args, ',');
    let [color, amount] = parts[..] else {
        return None;
    };
    let c = Rgba::parse(color)?;
    let amount = pct_or(amount, 1.0)?;
    let (h, s, l) = to_hsl(c);
    let l = (l + sign * amount).clamp(0.0, 1.0);
    let rgb = Rgba::parse(&format!("hsl({h}deg {}% {}%)", s * 100.0, l * 100.0))?;
    Some(Rgba { a: c.a, ..rgb })
}

/// `mix(color1, color2 [, weight])`: `weight` (default 50%) of `color1`, the rest
/// `color2`, mixed in sRGB like SCSS (`color-mix(in srgb, color1 weight, color2)`).
fn scss_mix(args: &str) -> Option<Rgba> {
    let parts = split_top_level(args, ',');
    let (a, b, weight) = match parts[..] {
        [a, b] => (a, b, 0.5),
        [a, b, w] => (a, b, pct_or(w, 1.0)?),
        _ => return None,
    };
    if !(0.0..=1.0).contains(&weight) {
        return None;
    }
    color_mix(&format!("in srgb, {a} {}%, {b}", weight * 100.0))
}

/// sRGB to HSL: hue in degrees, saturation and lightness in 0..1.
fn to_hsl(c: Rgba) -> (f32, f32, f32) {
    let max = c.r.max(c.g).max(c.b);
    let min = c.r.min(c.g).min(c.b);
    let l = (max + min) / 2.0;
    let d = max - min;
    if d == 0.0 {
        return (0.0, 0.0, l);
    }
    let s = d / (1.0 - (2.0 * l - 1.0).abs());
    let h = if max == c.r {
        60.0 * ((c.g - c.b) / d).rem_euclid(6.0)
    } else if max == c.g {
        60.0 * ((c.b - c.r) / d + 2.0)
    } else {
        60.0 * ((c.r - c.g) / d + 4.0)
    };
    (h, s, l)
}

/// One `color-mix()` argument: a colour and an optional percentage after it.
fn mix_arg(arg: &str) -> Option<(Rgba, Option<f32>)> {
    let arg = arg.trim();
    let tokens = split_top_level(arg, ' ');
    if let Some(last) = tokens.last()
        && let Some(p) = last.strip_suffix('%')
        && tokens.len() > 1
    {
        let color = arg[..arg.len() - last.len()].trim();
        return Some((Rgba::parse(color)?, Some(p.parse::<f32>().ok()? / 100.0)));
    }
    Some((Rgba::parse(arg)?, None))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rgb8(s: &str) -> [u8; 4] {
        let c = Rgba::parse(s).unwrap_or_else(|| panic!("{s} should parse"));
        [c.r, c.g, c.b, c.a].map(|v| (v * 255.0).round() as u8)
    }

    fn close(a: [u8; 4], b: [u8; 4]) -> bool {
        a.iter().zip(b).all(|(x, y)| x.abs_diff(y) <= 1)
    }

    #[test]
    fn hwb_matches_css() {
        assert_eq!(rgb8("hwb(0 0% 0%)"), [255, 0, 0, 255]);
        assert_eq!(rgb8("hwb(120 20% 40%)"), [51, 153, 51, 255]);
        assert_eq!(
            rgb8("hwb(0 60% 60%)"),
            [128, 128, 128, 255],
            "w + b > 1: grey"
        );
        assert_eq!(rgb8("hwb(240 0% 0% / 50%)"), [0, 0, 255, 128]);
    }

    #[test]
    fn lab_and_lch_match_css() {
        // CSS red in Lab (D50 white point, as in the CSS Color 4 spec).
        assert!(
            close(rgb8("lab(54.29 80.82 69.88)"), [255, 0, 0, 255]),
            "{:?}",
            rgb8("lab(54.29 80.82 69.88)")
        );
        assert!(close(rgb8("lab(100 0 0)"), [255, 255, 255, 255]));
        assert!(
            close(rgb8("lch(54.29 106.84 40.85)"), [255, 0, 0, 255]),
            "{:?}",
            rgb8("lch(54.29 106.84 40.85)")
        );
        assert_eq!(rgb8("lab(0% 0 0)"), [0, 0, 0, 255]);
    }

    #[test]
    fn oklab_and_oklch_match_css() {
        assert!(
            close(rgb8("oklab(0.628 0.2249 0.1258)"), [255, 0, 0, 255]),
            "{:?}",
            rgb8("oklab(0.628 0.2249 0.1258)")
        );
        assert!(
            close(rgb8("oklch(62.8% 0.2577 29.23)"), [255, 0, 0, 255]),
            "{:?}",
            rgb8("oklch(62.8% 0.2577 29.23)")
        );
        assert!(close(rgb8("oklch(1 0 none)"), [255, 255, 255, 255]));
        assert_eq!(rgb8("oklch(0.7 0.1 200 / 0.25)")[3], 64);
    }

    #[test]
    fn color_function_spaces() {
        assert_eq!(rgb8("color(srgb 1 0.5 0)"), [255, 128, 0, 255]);
        assert!(close(
            rgb8("color(srgb-linear 0.2140 0.2140 0.2140)"),
            [128, 128, 128, 255]
        ));
        assert!(close(rgb8("color(display-p3 1 1 1)"), [255, 255, 255, 255]));
        assert_eq!(
            rgb8("color(display-p3 1 0 0)")[0],
            255,
            "p3 red clips to sRGB red"
        );
    }

    #[test]
    fn color_mix_in_srgb_and_oklab() {
        assert_eq!(rgb8("color-mix(in srgb, red, blue)"), [128, 0, 128, 255]);
        assert_eq!(rgb8("color-mix(in srgb, red 25%, blue)"), [64, 0, 191, 255]);
        assert_eq!(
            rgb8("color-mix(in srgb, white 50%, black 50%)"),
            [128, 128, 128, 255]
        );
        // Percentages under 100% scale the alpha (s&box-style darken: mix with black).
        assert_eq!(rgb8("color-mix(in srgb, red 30%, blue 20%)")[3], 128);
        assert!(
            close(rgb8("color-mix(in oklab, white, black)"), [99, 99, 99, 255]),
            "{:?}",
            rgb8("color-mix(in oklab, white, black)")
        );
        assert_eq!(
            rgb8("color-mix(in srgb, rgb(0 0 255) 50%, transparent)"),
            [0, 0, 255, 128]
        );
        // Nested and with hsl() arguments.
        assert_eq!(
            rgb8("color-mix(in srgb, color-mix(in srgb, red, blue), hsl(0 0% 100%) 0%)"),
            [128, 0, 128, 255]
        );
    }

    #[test]
    fn color_mix_in_oklch_takes_the_short_hue_path() {
        // Red (hue ~29) and magenta (~328): the short way passes through pink, not green.
        let c = Rgba::parse("color-mix(in oklch, red, magenta)").unwrap();
        assert!(c.r > 0.8 && c.b > 0.3 && c.g < 0.3, "{c:?}");
    }

    #[test]
    fn bad_functions_are_rejected() {
        for bad in [
            "lab(1 2)",
            "oklch(a b c)",
            "color(rec2020 1 0 0)",
            "color-mix(in hsl, red, blue)",
            "color-mix(red, blue)",
            "color-mix(in srgb, red)",
            "color-mix(in srgb, red -10%, blue)",
            "hwb(0 0%)",
        ] {
            assert_eq!(Rgba::parse(bad), None, "{bad}");
        }
    }

    #[test]
    fn sbox_darken_lighten_and_mix() {
        assert_eq!(rgb8("darken(#ff0000, 20%)"), [153, 0, 0, 255]);
        assert_eq!(rgb8("lighten(#ff0000, 0.2)"), [255, 102, 102, 255]);
        assert_eq!(rgb8("darken(white, 100%)"), [0, 0, 0, 255]);
        assert_eq!(
            rgb8("lighten(rgba(0, 0, 255, 0.5), 10%)")[3],
            128,
            "alpha is kept"
        );
        assert_eq!(rgb8("mix(red, blue)"), [128, 0, 128, 255]);
        assert_eq!(
            rgb8("mix(red, blue, 25%)"),
            [64, 0, 191, 255],
            "25% of the first colour"
        );
        assert_eq!(
            rgb8("darken(mix(white, black), 10%)"),
            [102, 102, 102, 255],
            "nested"
        );
        for bad in [
            "darken(red)",
            "mix(red)",
            "mix(red, blue, 150%)",
            "lighten(nope, 10%)",
        ] {
            assert_eq!(Rgba::parse(bad), None, "{bad}");
        }
    }
}
