//! Paint-only style values (no effect on layout) and their CSS parsers:
//! corner radii, box shadows, blur filters and backgrounds.

use crate::Rgba;
use crate::color::parse_angle_degrees;

/// A pixel length or a percentage (stored as a fraction, `50%` → `0.5`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Len {
    Px(f32),
    Percent(f32),
}

impl Len {
    /// Parses `12px`, `12` (only `0` is valid unitless CSS, but numbers are pixels
    /// everywhere else in Iron, so any number is accepted) or `50%`.
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim();
        if let Some(p) = s.strip_suffix('%') {
            return Some(Len::Percent(p.trim().parse::<f32>().ok()? / 100.0));
        }
        let n = s.strip_suffix("px").unwrap_or(s).trim();
        Some(Len::Px(n.parse().ok()?))
    }

    pub fn resolve(self, basis: f32) -> f32 {
        match self {
            Len::Px(px) => px,
            Len::Percent(f) => f * basis,
        }
    }
}

/// Per-corner values in CSS order: top-left, top-right, bottom-right, bottom-left.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Corners<T> {
    pub top_left: T,
    pub top_right: T,
    pub bottom_right: T,
    pub bottom_left: T,
}

impl<T: Copy> Corners<T> {
    pub const fn all(v: T) -> Self {
        Self {
            top_left: v,
            top_right: v,
            bottom_right: v,
            bottom_left: v,
        }
    }

    pub fn map<U>(self, f: impl Fn(T) -> U) -> Corners<U> {
        Corners {
            top_left: f(self.top_left),
            top_right: f(self.top_right),
            bottom_right: f(self.bottom_right),
            bottom_left: f(self.bottom_left),
        }
    }

    pub fn to_array(self) -> [T; 4] {
        [
            self.top_left,
            self.top_right,
            self.bottom_right,
            self.bottom_left,
        ]
    }
}

/// `borderRadius` with 1–4 values, CSS order. Only circular corners are supported
/// (no `a / b` elliptical radii); percentages resolve against the shorter side.
pub fn parse_radii(s: &str) -> Option<Corners<Len>> {
    let values: Vec<Len> = s
        .split_whitespace()
        .map(Len::parse)
        .collect::<Option<_>>()?;
    if values.iter().any(|v| match v {
        Len::Px(n) | Len::Percent(n) => *n < 0.0,
    }) {
        return None;
    }
    let (tl, tr, br, bl) = match values[..] {
        [a] => (a, a, a, a),
        [a, b] => (a, b, a, b),
        [a, b, c] => (a, b, c, b),
        [a, b, c, d] => (a, b, c, d),
        _ => return None,
    };
    Some(Corners {
        top_left: tl,
        top_right: tr,
        bottom_right: br,
        bottom_left: bl,
    })
}

/// Resolves radii for a `width` × `height` box and scales them down, like CSS,
/// so adjacent corners never overlap.
pub fn resolve_radii(radii: &Corners<Len>, width: f32, height: f32) -> [f32; 4] {
    let basis = width.min(height);
    let r = radii.map(|l| l.resolve(basis).max(0.0)).to_array();
    let [tl, tr, br, bl] = r;
    let fit = |side: f32, a: f32, b: f32| if a + b > side { side / (a + b) } else { 1.0 };
    let f = fit(width, tl, tr)
        .min(fit(width, bl, br))
        .min(fit(height, tl, bl))
        .min(fit(height, tr, br));
    r.map(|v| v * f)
}

/// One `boxShadow` (or, in M3, `textShadow`) entry.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shadow {
    pub offset_x: f32,
    pub offset_y: f32,
    pub blur: f32,
    pub spread: f32,
    pub color: Rgba,
    /// The colour is `currentColor` (given explicitly, or omitted as in CSS) and
    /// is resolved from the element's text colour when drawn.
    pub current_color: bool,
    pub inset: bool,
}

/// A comma-separated shadow list, e.g. `"0 4px 12px rgba(0,0,0,.4), inset 0 0 0 1px #fff"`.
/// `none` gives an empty list. A missing colour means `currentColor`, as in CSS.
pub fn parse_shadows(s: &str) -> Option<Vec<Shadow>> {
    if s.trim() == "none" {
        return Some(Vec::new());
    }
    split_top_level(s, ',')
        .into_iter()
        .map(parse_shadow)
        .collect()
}

fn parse_shadow(s: &str) -> Option<Shadow> {
    let mut inset = false;
    let mut color = None;
    let mut lengths = Vec::new();
    for token in split_top_level(s, ' ') {
        if token == "inset" && !inset {
            inset = true;
        } else if let Some(Len::Px(px)) = Len::parse(token) {
            lengths.push(px);
        } else if color.is_none() && token.eq_ignore_ascii_case("currentcolor") {
            color = Some(None);
        } else if color.is_none() {
            color = Some(Some(Rgba::parse(token)?));
        } else {
            return None;
        }
    }
    let (offset_x, offset_y, blur, spread) = match lengths[..] {
        [x, y] => (x, y, 0.0, 0.0),
        [x, y, b] => (x, y, b, 0.0),
        [x, y, b, s] => (x, y, b, s),
        _ => return None,
    };
    if blur < 0.0 {
        return None;
    }
    Some(Shadow {
        offset_x,
        offset_y,
        blur,
        spread,
        color: color.flatten().unwrap_or(Rgba::new(0.0, 0.0, 0.0, 1.0)),
        current_color: !matches!(color, Some(Some(_))),
        inset,
    })
}

/// `filter` / `backdropFilter`: `blur(<px>)` or `none`. Returns the blur radius
/// (the Gaussian standard deviation, as in CSS); `0` means no filter.
pub fn parse_blur_filter(s: &str) -> Option<f32> {
    let s = s.trim();
    if s == "none" {
        return Some(0.0);
    }
    let arg = s.strip_prefix("blur(")?.strip_suffix(')')?;
    match Len::parse(arg)? {
        Len::Px(px) if px >= 0.0 => Some(px),
        _ => None,
    }
}

/// A colour stop in a gradient. `position` is `None` when the stop should be
/// spaced evenly between its neighbours.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColorStop {
    pub color: Rgba,
    pub position: Option<Len>,
}

/// Direction of a linear gradient.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GradientDirection {
    /// CSS angle: `0deg` points up, `90deg` right (`to right`), `180deg` down (the default).
    Angle(f32),
    /// `to top right` and friends: the angle depends on the box's aspect ratio,
    /// so it's resolved at draw time.
    Corner { right: bool, bottom: bool },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RadialShape {
    Circle,
    Ellipse,
}

#[derive(Clone, Debug, PartialEq)]
pub enum BackgroundImage {
    /// An image file, loaded through the asset system by the renderer.
    Url(String),
    Linear {
        direction: GradientDirection,
        stops: Vec<ColorStop>,
    },
    /// Centred, sized to the farthest corner (the CSS default).
    Radial {
        shape: RadialShape,
        stops: Vec<ColorStop>,
    },
}

/// `url(...)`, `linear-gradient(...)` or `radial-gradient(...)`.
pub fn parse_background_image(s: &str) -> Option<Option<BackgroundImage>> {
    let s = s.trim();
    if s == "none" {
        return Some(None);
    }
    let call = |name: &str| s.strip_prefix(name)?.strip_suffix(')');
    if let Some(arg) = call("url(") {
        let path = arg.trim().trim_matches(['"', '\'']);
        return (!path.is_empty()).then(|| Some(BackgroundImage::Url(path.to_owned())));
    }
    if let Some(args) = call("linear-gradient(") {
        let mut parts = split_top_level(args, ',');
        let first = *parts.first()?;
        let direction = if let Some(to) = first.strip_prefix("to ") {
            parts.remove(0);
            let words: Vec<&str> = to.split_whitespace().collect();
            let has = |w: &str| words.contains(&w);
            match (
                words.len(),
                has("left") || has("right"),
                has("top") || has("bottom"),
            ) {
                (1, true, false) => {
                    GradientDirection::Angle(if has("right") { 90.0 } else { 270.0 })
                }
                (1, false, true) => {
                    GradientDirection::Angle(if has("bottom") { 180.0 } else { 0.0 })
                }
                (2, true, true) => GradientDirection::Corner {
                    right: has("right"),
                    bottom: has("bottom"),
                },
                _ => return None,
            }
        } else if Rgba::parse(first.split_whitespace().next()?).is_none() {
            let angle = parse_angle_degrees(first)?;
            parts.remove(0);
            GradientDirection::Angle(angle)
        } else {
            GradientDirection::Angle(180.0)
        };
        return Some(Some(BackgroundImage::Linear {
            direction,
            stops: parse_stops(&parts)?,
        }));
    }
    if let Some(args) = call("radial-gradient(") {
        let mut parts = split_top_level(args, ',');
        let shape = match *parts.first()? {
            "circle" => Some(RadialShape::Circle),
            "ellipse" => Some(RadialShape::Ellipse),
            _ => None,
        };
        if shape.is_some() {
            parts.remove(0);
        }
        return Some(Some(BackgroundImage::Radial {
            shape: shape.unwrap_or(RadialShape::Ellipse),
            stops: parse_stops(&parts)?,
        }));
    }
    None
}

fn parse_stops(parts: &[&str]) -> Option<Vec<ColorStop>> {
    if parts.len() < 2 {
        return None;
    }
    parts
        .iter()
        .map(|p| {
            let tokens = split_top_level(p, ' ');
            let (color, position) = match tokens[..] {
                [c] => (c, None),
                [c, pos] => (c, Some(Len::parse(pos)?)),
                _ => return None,
            };
            Some(ColorStop {
                color: Rgba::parse(color)?,
                position,
            })
        })
        .collect()
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BackgroundSize {
    Cover,
    Contain,
    /// Width and height; `None` means `auto` (keep the image's aspect ratio).
    Explicit(Option<Len>, Option<Len>),
}

impl Default for BackgroundSize {
    fn default() -> Self {
        BackgroundSize::Explicit(None, None)
    }
}

pub fn parse_background_size(s: &str) -> Option<BackgroundSize> {
    let len_or_auto = |t: &str| match t {
        "auto" => Some(None),
        other => Len::parse(other).map(Some),
    };
    match s.split_whitespace().collect::<Vec<_>>()[..] {
        ["cover"] => Some(BackgroundSize::Cover),
        ["contain"] => Some(BackgroundSize::Contain),
        [w] => Some(BackgroundSize::Explicit(len_or_auto(w)?, None)),
        [w, h] => Some(BackgroundSize::Explicit(len_or_auto(w)?, len_or_auto(h)?)),
        _ => None,
    }
}

/// Where the background sits in the box: percentages align the image's point with
/// the box's point (CSS semantics), pixels offset from the top-left.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BackgroundPosition {
    pub x: Len,
    pub y: Len,
}

impl Default for BackgroundPosition {
    fn default() -> Self {
        Self {
            x: Len::Percent(0.0),
            y: Len::Percent(0.0),
        }
    }
}

/// `center`, `top`, `left top`, `right 20%`, `10px 50%`, ...
pub fn parse_background_position(s: &str) -> Option<BackgroundPosition> {
    let horizontal = |t: &str| match t {
        "left" => Some(Len::Percent(0.0)),
        "center" => Some(Len::Percent(0.5)),
        "right" => Some(Len::Percent(1.0)),
        _ => None,
    };
    let vertical = |t: &str| match t {
        "top" => Some(Len::Percent(0.0)),
        "center" => Some(Len::Percent(0.5)),
        "bottom" => Some(Len::Percent(1.0)),
        _ => None,
    };
    let tokens: Vec<&str> = s.split_whitespace().collect();
    let centre = Len::Percent(0.5);
    let (x, y) = match tokens[..] {
        [t] => match (horizontal(t), vertical(t)) {
            (Some(x), _) if t != "center" => (x, centre),
            (_, Some(y)) => (centre, y),
            _ => (Len::parse(t)?, centre),
        },
        [a, b] => {
            // Keywords may come in either order (`top left`); lengths are x then y.
            if let (Some(y), Some(x)) = (vertical(a), horizontal(b))
                && a != "center"
            {
                (x, y)
            } else {
                (
                    horizontal(a).or_else(|| Len::parse(a))?,
                    vertical(b).or_else(|| Len::parse(b))?,
                )
            }
        }
        _ => return None,
    };
    Some(BackgroundPosition { x, y })
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BackgroundRepeat {
    #[default]
    Repeat,
    NoRepeat,
    RepeatX,
    RepeatY,
}

pub fn parse_background_repeat(s: &str) -> Option<BackgroundRepeat> {
    Some(match s.trim() {
        "repeat" => BackgroundRepeat::Repeat,
        "no-repeat" => BackgroundRepeat::NoRepeat,
        "repeat-x" => BackgroundRepeat::RepeatX,
        "repeat-y" => BackgroundRepeat::RepeatY,
        _ => return None,
    })
}

/// Splits on `sep` outside parentheses, trimming parts and dropping empty ones
/// (so `' '` also collapses repeated spaces).
pub(crate) fn split_top_level(s: &str, sep: char) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth = 0usize;
    let mut start = 0;
    for (i, c) in s.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            c if c == sep && depth == 0 => {
                parts.push(&s[start..i]);
                start = i + c.len_utf8();
            }
            _ => {}
        }
    }
    parts.push(&s[start..]);
    parts
        .into_iter()
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .collect()
}

/// `overflow` on one axis.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Overflow {
    #[default]
    Visible,
    Hidden,
    /// Like `hidden`, but can't be scrolled even by scripts.
    Clip,
    /// Scrollable, with a scrollbar always shown.
    Scroll,
    /// Scrollable, with a scrollbar when the content overflows.
    Auto,
}

impl Overflow {
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s.trim() {
            "visible" => Overflow::Visible,
            "hidden" => Overflow::Hidden,
            "clip" => Overflow::Clip,
            "scroll" => Overflow::Scroll,
            "auto" | "overlay" => Overflow::Auto,
            _ => return None,
        })
    }

    /// `overflow` with one or two values (x then y).
    pub fn parse_pair(s: &str) -> Option<[Self; 2]> {
        match s.split_whitespace().collect::<Vec<_>>()[..] {
            [a] => Overflow::parse(a).map(|o| [o, o]),
            [a, b] => Some([Overflow::parse(a)?, Overflow::parse(b)?]),
            _ => None,
        }
    }

    /// Computed values: when one axis scrolls or hides, `visible` on the other
    /// becomes `auto` and `clip` becomes `hidden` (CSS Overflow 3).
    pub fn computed(pair: [Self; 2]) -> [Self; 2] {
        let boxy = |o: Overflow| matches!(o, Overflow::Hidden | Overflow::Scroll | Overflow::Auto);
        if !boxy(pair[0]) && !boxy(pair[1]) {
            return pair;
        }
        pair.map(|o| match o {
            Overflow::Visible => Overflow::Auto,
            Overflow::Clip => Overflow::Hidden,
            o => o,
        })
    }

    /// Whether content is clipped on this axis.
    pub fn clips(self) -> bool {
        self != Overflow::Visible
    }

    /// Whether the user can scroll this axis.
    pub fn scrolls(self) -> bool {
        matches!(self, Overflow::Scroll | Overflow::Auto)
    }
}

/// The box a `clipPath` shape is measured against.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GeometryBox {
    #[default]
    Border,
    Padding,
    Content,
}

/// A circle or ellipse radius.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ShapeRadius {
    Len(Len),
    ClosestSide,
    FarthestSide,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ClipShapeSpec {
    /// `inset(top right bottom left round radii)`.
    Inset {
        top: Len,
        right: Len,
        bottom: Len,
        left: Len,
        radii: Corners<Len>,
    },
    /// `circle(r at x y)`.
    Circle { r: ShapeRadius, at: (Len, Len) },
    /// `ellipse(rx ry at x y)`.
    Ellipse {
        rx: ShapeRadius,
        ry: ShapeRadius,
        at: (Len, Len),
    },
    /// The reference box itself (`clipPath: padding-box`), with the element's corners.
    Box,
}

/// `clipPath`: a basic shape and/or a reference box. `polygon()`, `path()` and
/// `url()` aren't supported yet.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClipPath {
    pub shape: ClipShapeSpec,
    pub reference: GeometryBox,
}

impl ClipPath {
    /// Parses `inset(10px round 8px)`, `circle(40% at 50% 50%)`,
    /// `ellipse(closest-side farthest-side at left top)`, optionally with a box
    /// keyword (`border-box`, `padding-box`, `content-box`), or a box keyword alone.
    pub fn parse(s: &str) -> Option<Self> {
        let mut shape = None;
        let mut reference = None;
        let s = s.trim();
        let mut rest = s;
        while !rest.is_empty() {
            let end = match rest.find('(') {
                Some(open) if !rest[..open].contains(' ') => open + rest[open..].find(')')? + 1,
                _ => rest.find(' ').unwrap_or(rest.len()),
            };
            let token = rest[..end].trim();
            rest = rest[end..].trim_start();
            let geometry = match token {
                "border-box" | "margin-box" => Some(GeometryBox::Border),
                "padding-box" => Some(GeometryBox::Padding),
                "content-box" => Some(GeometryBox::Content),
                _ => None,
            };
            if let Some(g) = geometry {
                if reference.replace(g).is_some() {
                    return None;
                }
            } else if shape.replace(parse_shape(token)?).is_some() {
                return None;
            }
        }
        if shape.is_none() && reference.is_none() {
            return None;
        }
        Some(ClipPath {
            shape: shape.unwrap_or(ClipShapeSpec::Box),
            reference: reference.unwrap_or_default(),
        })
    }

    /// The clip for an element whose border box is `border`, given its border and
    /// padding widths (top, right, bottom, left) and resolved corner radii.
    pub fn resolve(
        &self,
        border: crate::Rect,
        borders: [f32; 4],
        padding: [f32; 4],
        radii: [f32; 4],
    ) -> crate::ClipShape {
        let inset = |r: crate::Rect, w: [f32; 4]| {
            crate::Rect::new(
                r.x + w[3],
                r.y + w[0],
                (r.width - w[1] - w[3]).max(0.0),
                (r.height - w[0] - w[2]).max(0.0),
            )
        };
        let reference = match self.reference {
            GeometryBox::Border => border,
            GeometryBox::Padding => inset(border, borders),
            GeometryBox::Content => inset(inset(border, borders), padding),
        };
        let (w, h) = (reference.width, reference.height);
        let center =
            |at: (Len, Len)| (reference.x + at.0.resolve(w), reference.y + at.1.resolve(h));
        match self.shape {
            ClipShapeSpec::Box => {
                let shrink = match self.reference {
                    GeometryBox::Border => [0.0; 4],
                    GeometryBox::Padding => borders,
                    GeometryBox::Content => [0, 1, 2, 3].map(|i| borders[i] + padding[i]),
                };
                inner_corners(reference, radii, shrink)
            }
            ClipShapeSpec::Inset {
                top,
                right,
                bottom,
                left,
                radii,
            } => {
                let rect = crate::Rect::new(
                    reference.x + left.resolve(w),
                    reference.y + top.resolve(h),
                    (w - left.resolve(w) - right.resolve(w)).max(0.0),
                    (h - top.resolve(h) - bottom.resolve(h)).max(0.0),
                );
                crate::ClipShape::rounded(rect, resolve_radii(&radii, rect.width, rect.height))
            }
            ClipShapeSpec::Circle { r, at } => {
                let (cx, cy) = center(at);
                let sides = [
                    cx - reference.x,
                    reference.x + w - cx,
                    cy - reference.y,
                    reference.y + h - cy,
                ];
                let r = match r {
                    // Percentages are of the box's normalized diagonal.
                    ShapeRadius::Len(l) => {
                        l.resolve((w * w + h * h).sqrt() / std::f32::consts::SQRT_2)
                    }
                    ShapeRadius::ClosestSide => sides.iter().fold(f32::MAX, |a, b| a.min(b.abs())),
                    ShapeRadius::FarthestSide => sides.iter().fold(0.0_f32, |a, b| a.max(b.abs())),
                };
                ellipse(cx, cy, r, r)
            }
            ClipShapeSpec::Ellipse { rx, ry, at } => {
                let (cx, cy) = center(at);
                let axis = |r: ShapeRadius, lo: f32, hi: f32, size: f32| match r {
                    ShapeRadius::Len(l) => l.resolve(size),
                    ShapeRadius::ClosestSide => lo.min(hi),
                    ShapeRadius::FarthestSide => lo.max(hi),
                };
                let rx = axis(
                    rx,
                    (cx - reference.x).abs(),
                    (reference.x + w - cx).abs(),
                    w,
                );
                let ry = axis(
                    ry,
                    (cy - reference.y).abs(),
                    (reference.y + h - cy).abs(),
                    h,
                );
                ellipse(cx, cy, rx, ry)
            }
        }
    }
}

fn ellipse(cx: f32, cy: f32, rx: f32, ry: f32) -> crate::ClipShape {
    let (rx, ry) = (rx.max(0.0), ry.max(0.0));
    crate::ClipShape {
        rect: crate::Rect::new(cx - rx, cy - ry, 2.0 * rx, 2.0 * ry),
        radii_x: [rx; 4],
        radii_y: [ry; 4],
    }
}

/// `rect` (already shrunk by `shrink`: top, right, bottom, left) with the outer
/// `radii` reduced by the shrink on each side, as CSS does for inner edges: the
/// horizontal radius loses the left/right width, the vertical one the top/bottom.
pub fn inner_corners(rect: crate::Rect, radii: [f32; 4], shrink: [f32; 4]) -> crate::ClipShape {
    let [t, r, b, l] = shrink;
    let x = [l, r, r, l];
    let y = [t, t, b, b];
    crate::ClipShape {
        rect,
        radii_x: [0, 1, 2, 3].map(|i| (radii[i] - x[i]).max(0.0)),
        radii_y: [0, 1, 2, 3].map(|i| (radii[i] - y[i]).max(0.0)),
    }
}

fn parse_shape(token: &str) -> Option<ClipShapeSpec> {
    let (name, args) = token.strip_suffix(')')?.split_once('(')?;
    let args = args.trim();
    match name.trim() {
        "inset" => {
            let (lengths, round) = match args.split_once(" round ") {
                Some((l, r)) => (l, Some(r)),
                None => (args, None),
            };
            let v: Vec<Len> = lengths
                .split_whitespace()
                .map(Len::parse)
                .collect::<Option<_>>()?;
            let (top, right, bottom, left) = match v[..] {
                [a] => (a, a, a, a),
                [a, b] => (a, b, a, b),
                [a, b, c] => (a, b, c, b),
                [a, b, c, d] => (a, b, c, d),
                _ => return None,
            };
            let radii = match round {
                Some(r) => parse_radii(r)?,
                None => Corners::all(Len::Px(0.0)),
            };
            Some(ClipShapeSpec::Inset {
                top,
                right,
                bottom,
                left,
                radii,
            })
        }
        "circle" | "ellipse" => {
            let tokens: Vec<&str> = args.split_whitespace().collect();
            let (radii, at) = match tokens.iter().position(|t| *t == "at") {
                Some(i) => (&tokens[..i], Some(tokens[i + 1..].join(" "))),
                None => (&tokens[..], None),
            };
            let at = match at {
                Some(a) => {
                    let p = parse_background_position(&a)?;
                    (p.x, p.y)
                }
                None => (Len::Percent(0.5), Len::Percent(0.5)),
            };
            let radius = |t: &str| match t {
                "closest-side" => Some(ShapeRadius::ClosestSide),
                "farthest-side" => Some(ShapeRadius::FarthestSide),
                t => Len::parse(t).map(ShapeRadius::Len),
            };
            let r: Vec<ShapeRadius> = radii.iter().map(|t| radius(t)).collect::<Option<_>>()?;
            if name.trim() == "circle" {
                let r = match r[..] {
                    [] => ShapeRadius::ClosestSide,
                    [r] => r,
                    _ => return None,
                };
                Some(ClipShapeSpec::Circle { r, at })
            } else {
                let (rx, ry) = match r[..] {
                    [] => (ShapeRadius::ClosestSide, ShapeRadius::ClosestSide),
                    [rx, ry] => (rx, ry),
                    _ => return None,
                };
                Some(ClipShapeSpec::Ellipse { rx, ry, at })
            }
        }
        _ => None,
    }
}

/// Legacy `clip: rect(top, right, bottom, left)` (commas optional, `auto` for an
/// edge of the border box). Offsets are from the border box's top-left corner.
pub fn parse_clip_rect(s: &str) -> Option<[Option<f32>; 4]> {
    let inner = s.trim().strip_prefix("rect(")?.strip_suffix(')')?;
    let parts: Vec<&str> = inner
        .split(|c: char| c == ',' || c.is_whitespace())
        .filter(|p| !p.is_empty())
        .collect();
    let [t, r, b, l] = parts[..] else {
        return None;
    };
    let edge = |p: &str| -> Option<Option<f32>> {
        if p == "auto" {
            return Some(None);
        }
        match Len::parse(p)? {
            Len::Px(px) => Some(Some(px)),
            Len::Percent(_) => None,
        }
    };
    Some([edge(t)?, edge(r)?, edge(b)?, edge(l)?])
}

#[cfg(test)]
mod tests {
    use super::*;

    // M2-VIS-03
    #[test]
    fn radii_follow_css_corner_order() {
        let r = parse_radii("1px 2px 3px 4px").unwrap();
        assert_eq!(
            r.to_array(),
            [Len::Px(1.0), Len::Px(2.0), Len::Px(3.0), Len::Px(4.0)]
        );
        let r = parse_radii("1px 2px").unwrap();
        assert_eq!(
            r.to_array(),
            [Len::Px(1.0), Len::Px(2.0), Len::Px(1.0), Len::Px(2.0)]
        );
        let r = parse_radii("1px 2px 3px").unwrap();
        assert_eq!(
            r.to_array(),
            [Len::Px(1.0), Len::Px(2.0), Len::Px(3.0), Len::Px(2.0)]
        );
        assert_eq!(parse_radii("-1px"), None);
        assert_eq!(parse_radii("1px 2px 3px 4px 5px"), None);
    }

    // M2-VIS-03
    #[test]
    fn radii_resolve_and_scale_like_css() {
        let circle = parse_radii("50%").unwrap();
        assert_eq!(resolve_radii(&circle, 40.0, 40.0), [20.0; 4]);
        // 30px corners on a 40px-tall box overlap vertically: scaled by 40/60.
        let big = parse_radii("30px").unwrap();
        let r = resolve_radii(&big, 100.0, 40.0);
        assert!(r.iter().all(|v| (v - 20.0).abs() < 1e-4), "{r:?}");
    }

    // M2-VIS-04
    #[test]
    fn parses_shadow_lists() {
        let s = parse_shadows("0 4px 12px rgba(0,0,0,.4), inset 0 0 0 1px #fff").unwrap();
        assert_eq!(s.len(), 2);
        assert_eq!(
            (s[0].offset_x, s[0].offset_y, s[0].blur, s[0].spread),
            (0.0, 4.0, 12.0, 0.0)
        );
        assert!(!s[0].inset);
        assert!((s[0].color.a - 0.4).abs() < 1e-6);
        assert!(s[1].inset);
        assert_eq!(s[1].spread, 1.0);
        assert_eq!(s[1].color, Rgba::new(1.0, 1.0, 1.0, 1.0));

        let s = parse_shadows("red 2px 2px").unwrap();
        assert_eq!(s[0].color, Rgba::new(1.0, 0.0, 0.0, 1.0));
        assert_eq!(parse_shadows("none").unwrap(), []);
        for bad in [
            "2px",
            "1px 2px 3px 4px 5px",
            "1px 2px -3px",
            "1px 2px red blue",
            "1px 2px, ",
        ] {
            if bad == "1px 2px, " {
                // A trailing comma just drops the empty entry.
                assert_eq!(parse_shadows(bad).unwrap().len(), 1);
                continue;
            }
            assert_eq!(parse_shadows(bad), None, "{bad:?}");
        }
    }

    // M2-VIS-05
    #[test]
    fn parses_blur_filters() {
        assert_eq!(parse_blur_filter("blur(4px)"), Some(4.0));
        assert_eq!(parse_blur_filter("blur(0)"), Some(0.0));
        assert_eq!(parse_blur_filter("none"), Some(0.0));
        assert_eq!(parse_blur_filter("brightness(2)"), None);
        assert_eq!(parse_blur_filter("blur(-1px)"), None);
        assert_eq!(parse_blur_filter("blur(10%)"), None);
    }

    // M2-VIS-06
    #[test]
    fn parses_gradients_and_urls() {
        assert_eq!(
            parse_background_image("url('ui/panel.png')").unwrap(),
            Some(BackgroundImage::Url("ui/panel.png".into()))
        );
        let Some(BackgroundImage::Linear { direction, stops }) =
            parse_background_image("linear-gradient(to right, red, rgba(0, 0, 255, 0.5) 75%)")
                .unwrap()
        else {
            panic!("expected a linear gradient");
        };
        assert_eq!(direction, GradientDirection::Angle(90.0));
        assert_eq!(stops.len(), 2);
        assert_eq!(stops[1].position, Some(Len::Percent(0.75)));

        let Some(BackgroundImage::Linear { direction, .. }) =
            parse_background_image("linear-gradient(45deg, red, blue)").unwrap()
        else {
            panic!()
        };
        assert_eq!(direction, GradientDirection::Angle(45.0));
        let Some(BackgroundImage::Linear { direction, .. }) =
            parse_background_image("linear-gradient(red, blue)").unwrap()
        else {
            panic!()
        };
        assert_eq!(
            direction,
            GradientDirection::Angle(180.0),
            "default is to bottom"
        );
        let Some(BackgroundImage::Linear { direction, .. }) =
            parse_background_image("linear-gradient(to top left, red, blue)").unwrap()
        else {
            panic!()
        };
        assert_eq!(
            direction,
            GradientDirection::Corner {
                right: false,
                bottom: false
            }
        );

        let Some(BackgroundImage::Radial { shape, stops }) =
            parse_background_image("radial-gradient(circle, white 0%, black 100%)").unwrap()
        else {
            panic!()
        };
        assert_eq!(shape, RadialShape::Circle);
        assert_eq!(stops.len(), 2);

        assert_eq!(parse_background_image("none"), Some(None));
        for bad in [
            "linear-gradient(red)",
            "url()",
            "conic-gradient(red, blue)",
            "linear-gradient(to middle, red, blue)",
        ] {
            assert_eq!(parse_background_image(bad), None, "{bad:?}");
        }
    }

    // M2-VIS-06
    #[test]
    fn parses_background_size_position_repeat() {
        assert_eq!(parse_background_size("cover"), Some(BackgroundSize::Cover));
        assert_eq!(
            parse_background_size("50% auto"),
            Some(BackgroundSize::Explicit(Some(Len::Percent(0.5)), None))
        );
        assert_eq!(
            parse_background_size("32px"),
            Some(BackgroundSize::Explicit(Some(Len::Px(32.0)), None))
        );

        let pos = |s| parse_background_position(s).unwrap();
        assert_eq!(
            pos("center"),
            BackgroundPosition {
                x: Len::Percent(0.5),
                y: Len::Percent(0.5)
            }
        );
        assert_eq!(
            pos("top"),
            BackgroundPosition {
                x: Len::Percent(0.5),
                y: Len::Percent(0.0)
            }
        );
        assert_eq!(
            pos("right"),
            BackgroundPosition {
                x: Len::Percent(1.0),
                y: Len::Percent(0.5)
            }
        );
        assert_eq!(
            pos("top left"),
            BackgroundPosition {
                x: Len::Percent(0.0),
                y: Len::Percent(0.0)
            }
        );
        assert_eq!(
            pos("left bottom"),
            BackgroundPosition {
                x: Len::Percent(0.0),
                y: Len::Percent(1.0)
            }
        );
        assert_eq!(
            pos("10px 25%"),
            BackgroundPosition {
                x: Len::Px(10.0),
                y: Len::Percent(0.25)
            }
        );
        assert_eq!(parse_background_position("middle"), None);

        assert_eq!(
            parse_background_repeat("no-repeat"),
            Some(BackgroundRepeat::NoRepeat)
        );
        assert_eq!(parse_background_repeat("space"), None);
    }
}
