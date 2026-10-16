//! Paint-only style values (no effect on layout) and their CSS parsers:
//! corner radii, box shadows, blur filters and backgrounds.

use crate::Rgba;
use crate::color::parse_angle_degrees;
use crate::draw::Radii;

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

/// One corner's radius: horizontal and vertical (equal for a circular corner).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CornerRadius {
    pub x: Len,
    pub y: Len,
}

impl CornerRadius {
    pub const ZERO: Self = Self::circular(Len::Px(0.0));

    pub const fn circular(r: Len) -> Self {
        Self { x: r, y: r }
    }

    /// One corner's longhand: `10px` or `10px 20px` (horizontal, vertical).
    pub fn parse(s: &str) -> Option<Self> {
        match parse_radius_list(s)?[..] {
            [r] => Some(Self::circular(r)),
            [x, y] => Some(Self { x, y }),
            _ => None,
        }
    }
}

fn parse_radius_list(s: &str) -> Option<Vec<Len>> {
    let values: Vec<Len> = s
        .split_whitespace()
        .map(Len::parse)
        .collect::<Option<_>>()?;
    let negative = values.iter().any(|v| match v {
        Len::Px(n) | Len::Percent(n) => *n < 0.0,
    });
    (!negative && !values.is_empty()).then_some(values)
}

/// `borderRadius`: 1–4 values in CSS order, optionally `/` and 1–4 vertical radii
/// for elliptical corners (`10px / 20px`).
pub fn parse_radii(s: &str) -> Option<Corners<CornerRadius>> {
    let four = |part: &str| -> Option<[Len; 4]> {
        Some(match parse_radius_list(part)?[..] {
            [a] => [a, a, a, a],
            [a, b] => [a, b, a, b],
            [a, b, c] => [a, b, c, b],
            [a, b, c, d] => [a, b, c, d],
            _ => return None,
        })
    };
    let (x, y) = match s.split_once('/') {
        Some((x, y)) => (four(x)?, four(y)?),
        None => {
            let x = four(s)?;
            (x, x)
        }
    };
    let corner = |i: usize| CornerRadius { x: x[i], y: y[i] };
    Some(Corners {
        top_left: corner(0),
        top_right: corner(1),
        bottom_right: corner(2),
        bottom_left: corner(3),
    })
}

/// Resolves radii for a `width` × `height` box (horizontal percentages of the
/// width, vertical of the height) and scales them all down, like CSS, so adjacent
/// corners never overlap.
pub fn resolve_radii(radii: &Corners<CornerRadius>, width: f32, height: f32) -> Radii {
    let corners = radii.to_array();
    let x = corners.map(|c| c.x.resolve(width).max(0.0));
    let y = corners.map(|c| c.y.resolve(height).max(0.0));
    let fit = |side: f32, a: f32, b: f32| if a + b > side { side / (a + b) } else { 1.0 };
    let f = fit(width, x[0], x[1])
        .min(fit(width, x[3], x[2]))
        .min(fit(height, y[0], y[3]))
        .min(fit(height, y[1], y[2]));
    Radii {
        x: x.map(|v| v * f),
        y: y.map(|v| v * f),
    }
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
        /// `repeating-linear-gradient`: the stops repeat beyond the first and last.
        repeating: bool,
    },
    /// Centred, sized to the farthest corner (the CSS default).
    Radial {
        shape: RadialShape,
        stops: Vec<ColorStop>,
        repeating: bool,
    },
    /// `conic-gradient`: colours sweep clockwise around `at`, starting `from` degrees
    /// (0 is up). Stop positions are fractions of a turn.
    Conic {
        from: f32,
        at: BackgroundPosition,
        stops: Vec<ColorStop>,
        repeating: bool,
    },
}

/// `url(...)` or a gradient: `linear-gradient(...)`, `radial-gradient(...)`,
/// `conic-gradient(...)` and their `repeating-` forms.
pub fn parse_background_image(s: &str) -> Option<Option<BackgroundImage>> {
    let s = s.trim();
    if s == "none" {
        return Some(None);
    }
    let (repeating, s) = match s.strip_prefix("repeating-") {
        Some(rest) if rest.contains("-gradient(") => (true, rest),
        _ => (false, s),
    };
    let call = |name: &str| s.strip_prefix(name)?.strip_suffix(')');
    if let Some(arg) = call("url(").filter(|_| !repeating) {
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
        } else if Rgba::parse(split_top_level(first, ' ').first()?).is_none() {
            let angle = parse_angle_degrees(first)?;
            parts.remove(0);
            GradientDirection::Angle(angle)
        } else {
            GradientDirection::Angle(180.0)
        };
        return Some(Some(BackgroundImage::Linear {
            direction,
            stops: parse_stops(&parts, Len::parse)?,
            repeating,
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
            stops: parse_stops(&parts, Len::parse)?,
            repeating,
        }));
    }
    if let Some(args) = call("conic-gradient(") {
        let mut parts = split_top_level(args, ',');
        let mut from = 0.0;
        let mut at = BackgroundPosition {
            x: Len::Percent(0.5),
            y: Len::Percent(0.5),
        };
        let first = parts.first()?.trim();
        if first.starts_with("from ") || first.starts_with("at ") {
            let (from_part, at_part) = match first.find("at ") {
                Some(i) => (&first[..i], Some(&first[i + 3..])),
                None => (first, None),
            };
            if let Some(angle) = from_part.trim().strip_prefix("from ") {
                from = parse_angle_degrees(angle)?;
            } else if !from_part.trim().is_empty() {
                return None;
            }
            if let Some(position) = at_part {
                at = parse_background_position(position)?;
            }
            parts.remove(0);
        }
        // Stop positions are angles or percentages of a turn.
        let turn = |t: &str| match Len::parse(t) {
            Some(Len::Percent(f)) => Some(Len::Percent(f)),
            Some(Len::Px(0.0)) => Some(Len::Percent(0.0)),
            _ => parse_angle_degrees(t)
                .filter(|_| t.ends_with(|c: char| c.is_ascii_alphabetic()))
                .map(|deg| Len::Percent(deg / 360.0)),
        };
        return Some(Some(BackgroundImage::Conic {
            from,
            at,
            stops: parse_stops(&parts, turn)?,
            repeating,
        }));
    }
    None
}

/// Colour stops: `color`, `color pos` or `color pos1 pos2` (two stops of one colour).
fn parse_stops(parts: &[&str], position: impl Fn(&str) -> Option<Len>) -> Option<Vec<ColorStop>> {
    let mut stops = Vec::new();
    for p in parts {
        let tokens = split_top_level(p, ' ');
        let (color, positions) = tokens.split_first()?;
        let color = Rgba::parse(color)?;
        match positions {
            [] => stops.push(ColorStop {
                color,
                position: None,
            }),
            [_] | [_, _] => {
                for pos in positions {
                    stops.push(ColorStop {
                        color,
                        position: Some(position(pos)?),
                    });
                }
            }
            _ => return None,
        }
    }
    (stops.len() >= 2).then_some(stops)
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
/// `object-fit`: how an image fills its element's content box.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ObjectFit {
    #[default]
    Fill,
    Contain,
    Cover,
    None,
    ScaleDown,
}

impl ObjectFit {
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s.trim() {
            "fill" => Self::Fill,
            "contain" => Self::Contain,
            "cover" => Self::Cover,
            "none" => Self::None,
            "scale-down" => Self::ScaleDown,
            _ => return None,
        })
    }

    /// The equivalent `background-size` for an image of `natural` size in `area`
    /// (`scale-down` is `none` or `contain`, whichever is smaller).
    pub fn background_size(self, natural: (f32, f32), area: (f32, f32)) -> BackgroundSize {
        let full = Some(Len::Percent(1.0));
        match self {
            Self::Fill => BackgroundSize::Explicit(full, full),
            Self::Contain => BackgroundSize::Contain,
            Self::Cover => BackgroundSize::Cover,
            Self::None => BackgroundSize::Explicit(None, None),
            Self::ScaleDown if natural.0 <= area.0 && natural.1 <= area.1 => {
                BackgroundSize::Explicit(None, None)
            }
            Self::ScaleDown => BackgroundSize::Contain,
        }
    }
}

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

/// How a background image repeats along one axis.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RepeatAxis {
    #[default]
    Repeat,
    NoRepeat,
    /// Repeat, scaled so a whole number of tiles fits.
    Round,
    /// Repeat whole tiles only, spreading the leftover space between them.
    Space,
}

/// `background-repeat`, per axis.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BackgroundRepeat {
    pub x: RepeatAxis,
    pub y: RepeatAxis,
}

impl BackgroundRepeat {
    pub const REPEAT: Self = Self {
        x: RepeatAxis::Repeat,
        y: RepeatAxis::Repeat,
    };
    pub const NO_REPEAT: Self = Self {
        x: RepeatAxis::NoRepeat,
        y: RepeatAxis::NoRepeat,
    };
}

/// One layer's `background-repeat`: `repeat-x`, `repeat-y`, or one or two of
/// `repeat`, `no-repeat`, `round`, `space`.
pub fn parse_background_repeat(s: &str) -> Option<BackgroundRepeat> {
    let axis = |t: &str| {
        Some(match t {
            "repeat" => RepeatAxis::Repeat,
            "no-repeat" => RepeatAxis::NoRepeat,
            "round" => RepeatAxis::Round,
            "space" => RepeatAxis::Space,
            _ => return None,
        })
    };
    let words: Vec<&str> = s.split_whitespace().collect();
    let (x, y) = match words[..] {
        ["repeat-x"] => (RepeatAxis::Repeat, RepeatAxis::NoRepeat),
        ["repeat-y"] => (RepeatAxis::NoRepeat, RepeatAxis::Repeat),
        [both] => (axis(both)?, axis(both)?),
        [x, y] => (axis(x)?, axis(y)?),
        _ => return None,
    };
    Some(BackgroundRepeat { x, y })
}

/// `background-clip`: the area a background layer paints.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BackgroundClip {
    #[default]
    BorderBox,
    PaddingBox,
    ContentBox,
    /// Only inside the glyphs of the element's text (gradient text).
    Text,
}

impl BackgroundClip {
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s.trim() {
            "border-box" => Self::BorderBox,
            "padding-box" => Self::PaddingBox,
            "content-box" => Self::ContentBox,
            "text" => Self::Text,
            _ => return None,
        })
    }
}

/// `border-style` for one side. Iron's default is `solid` (a width alone draws a
/// border, as in s&box); CSS's is `none`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BorderStyle {
    None,
    Hidden,
    #[default]
    Solid,
    Dashed,
    Dotted,
    Double,
    Groove,
    Ridge,
    Inset,
    Outset,
}

impl BorderStyle {
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s.trim() {
            "none" => Self::None,
            "hidden" => Self::Hidden,
            "solid" => Self::Solid,
            "dashed" => Self::Dashed,
            "dotted" => Self::Dotted,
            "double" => Self::Double,
            "groove" => Self::Groove,
            "ridge" => Self::Ridge,
            "inset" => Self::Inset,
            "outset" => Self::Outset,
            _ => return None,
        })
    }

    /// Whether the side is drawn at all (`none` and `hidden` aren't, and take no space).
    pub fn is_visible(self) -> bool {
        !matches!(self, Self::None | Self::Hidden)
    }

    /// The style's number in the quad shader.
    pub fn index(self) -> u32 {
        self as u32
    }
}

/// `mix-blend-mode` / `background-blend-mode`: how colours combine with what's
/// beneath them (the W3C compositing modes, blended in sRGB).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BlendMode {
    #[default]
    Normal,
    Multiply,
    Screen,
    Overlay,
    Darken,
    Lighten,
    ColorDodge,
    ColorBurn,
    HardLight,
    SoftLight,
    Difference,
    Exclusion,
    Hue,
    Saturation,
    Color,
    Luminosity,
}

impl BlendMode {
    const NAMES: [&str; 16] = [
        "normal",
        "multiply",
        "screen",
        "overlay",
        "darken",
        "lighten",
        "color-dodge",
        "color-burn",
        "hard-light",
        "soft-light",
        "difference",
        "exclusion",
        "hue",
        "saturation",
        "color",
        "luminosity",
    ];
    const ALL: [BlendMode; 16] = [
        Self::Normal,
        Self::Multiply,
        Self::Screen,
        Self::Overlay,
        Self::Darken,
        Self::Lighten,
        Self::ColorDodge,
        Self::ColorBurn,
        Self::HardLight,
        Self::SoftLight,
        Self::Difference,
        Self::Exclusion,
        Self::Hue,
        Self::Saturation,
        Self::Color,
        Self::Luminosity,
    ];

    pub fn parse(s: &str) -> Option<Self> {
        let i = Self::NAMES.iter().position(|n| *n == s.trim())?;
        Some(Self::ALL[i])
    }

    /// The mode's number in the composite shader (`normal` is 0).
    pub fn index(self) -> u32 {
        Self::ALL.iter().position(|m| *m == self).unwrap_or(0) as u32
    }
}

/// A comma-separated list of background layer values (`a, b, c`), top layer first.
pub fn parse_layers<T>(s: &str, item: impl Fn(&str) -> Option<T>) -> Option<Vec<T>> {
    split_top_level(s, ',').into_iter().map(item).collect()
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
        radii: Corners<CornerRadius>,
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
        radii: Radii,
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
pub fn inner_corners(rect: crate::Rect, radii: Radii, shrink: [f32; 4]) -> crate::ClipShape {
    crate::ClipShape::rounded(rect, radii.inner(shrink))
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
                None => Corners::all(CornerRadius::ZERO),
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
        let x = |s: &str| parse_radii(s).unwrap().to_array().map(|c| c.x);
        let px = |v: [f32; 4]| v.map(Len::Px);
        assert_eq!(x("1px 2px 3px 4px"), px([1.0, 2.0, 3.0, 4.0]));
        assert_eq!(x("1px 2px"), px([1.0, 2.0, 1.0, 2.0]));
        assert_eq!(x("1px 2px 3px"), px([1.0, 2.0, 3.0, 2.0]));
        assert_eq!(parse_radii("-1px"), None);
        assert_eq!(parse_radii("1px 2px 3px 4px 5px"), None);
    }

    // M3-BRD-02
    #[test]
    fn elliptical_radii_parse_after_a_slash() {
        let r = parse_radii("10px 20px / 5px").unwrap().to_array();
        assert_eq!(r.map(|c| c.x), [10.0, 20.0, 10.0, 20.0].map(Len::Px));
        assert_eq!(r.map(|c| c.y), [Len::Px(5.0); 4]);
        assert_eq!(
            CornerRadius::parse("10px 50%"),
            Some(CornerRadius {
                x: Len::Px(10.0),
                y: Len::Percent(0.5)
            })
        );
        for bad in ["10px /", "/ 5px", "1px / 2px / 3px", "1px 2px 3px"] {
            assert!(parse_radii(bad).is_none() || bad == "1px 2px 3px", "{bad}");
        }
        assert_eq!(CornerRadius::parse("1px 2px 3px"), None);
    }

    // M2-VIS-03, M3-BRD-02
    #[test]
    fn radii_resolve_and_scale_like_css() {
        let circle = parse_radii("50%").unwrap();
        assert_eq!(
            resolve_radii(&circle, 40.0, 40.0),
            Radii::circular([20.0; 4])
        );
        // On a wide box, 50% is an ellipse: half the width across, half the height down.
        let ellipse = resolve_radii(&circle, 100.0, 40.0);
        assert_eq!((ellipse.x, ellipse.y), ([50.0; 4], [20.0; 4]));
        // 30px corners on a 40px-tall box overlap vertically: scaled by 40/60.
        let big = parse_radii("30px").unwrap();
        let r = resolve_radii(&big, 100.0, 40.0);
        assert!(
            r.x.iter().chain(&r.y).all(|v| (v - 20.0).abs() < 1e-4),
            "{r:?}"
        );
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

    // M3-BG-03
    #[test]
    fn blend_modes_parse_to_shader_indices() {
        assert_eq!(BlendMode::parse("normal").map(BlendMode::index), Some(0));
        assert_eq!(BlendMode::parse("multiply"), Some(BlendMode::Multiply));
        assert_eq!(
            BlendMode::parse(" color-dodge ").map(BlendMode::index),
            Some(6)
        );
        assert_eq!(
            BlendMode::parse("luminosity").map(BlendMode::index),
            Some(15)
        );
        assert_eq!(BlendMode::parse("plus-darker"), None);
    }

    // M3-BG-01
    #[test]
    fn parses_conic_and_repeating_gradients() {
        let Some(BackgroundImage::Conic {
            from,
            at,
            stops,
            repeating,
        }) = parse_background_image(
            "conic-gradient(from 0.25turn at 10px 20%, red, blue 90deg, lime 50% 75%)",
        )
        .unwrap()
        else {
            panic!("expected a conic gradient");
        };
        assert_eq!(from, 90.0);
        assert_eq!((at.x, at.y), (Len::Px(10.0), Len::Percent(0.2)));
        assert!(!repeating);
        let positions: Vec<_> = stops.iter().map(|s| s.position).collect();
        assert_eq!(
            positions,
            vec![
                None,
                Some(Len::Percent(0.25)),
                Some(Len::Percent(0.5)),
                Some(Len::Percent(0.75))
            ],
            "angles are fractions of a turn; two positions make two stops"
        );

        let Some(BackgroundImage::Conic { at, .. }) =
            parse_background_image("conic-gradient(red, blue)").unwrap()
        else {
            panic!()
        };
        assert_eq!(
            (at.x, at.y),
            (Len::Percent(0.5), Len::Percent(0.5)),
            "centred"
        );
        assert!(matches!(
            parse_background_image("repeating-linear-gradient(red 0 10px, blue 10px 20px)"),
            Some(Some(BackgroundImage::Linear { repeating: true, ref stops, .. })) if stops.len() == 4
        ));
        assert!(matches!(
            parse_background_image("repeating-radial-gradient(circle, red, blue 10%)"),
            Some(Some(BackgroundImage::Radial {
                repeating: true,
                ..
            }))
        ));
        assert!(matches!(
            parse_background_image("repeating-conic-gradient(red 0 10deg, blue 10deg 20deg)"),
            Some(Some(BackgroundImage::Conic {
                repeating: true,
                ..
            }))
        ));
        for bad in [
            "conic-gradient(from 10px, red, blue)",
            "conic-gradient(red 10px, blue)",
            "conic-gradient(red)",
            "repeating-url(a.png)",
            "linear-gradient(red 1px 2px 3px, blue)",
        ] {
            assert_eq!(parse_background_image(bad), None, "{bad}");
        }
    }

    // M2-VIS-06
    #[test]
    fn parses_gradients_and_urls() {
        assert_eq!(
            parse_background_image("url('ui/panel.png')").unwrap(),
            Some(BackgroundImage::Url("ui/panel.png".into()))
        );
        let Some(BackgroundImage::Linear {
            direction, stops, ..
        }) = parse_background_image("linear-gradient(to right, red, rgba(0, 0, 255, 0.5) 75%)")
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

        let Some(BackgroundImage::Radial { shape, stops, .. }) =
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
            "conic-gradient(red)",
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
            Some(BackgroundRepeat::NO_REPEAT)
        );
        assert_eq!(
            parse_background_repeat("repeat-x"),
            Some(BackgroundRepeat {
                x: RepeatAxis::Repeat,
                y: RepeatAxis::NoRepeat
            })
        );
        assert_eq!(
            parse_background_repeat("round space"),
            Some(BackgroundRepeat {
                x: RepeatAxis::Round,
                y: RepeatAxis::Space
            })
        );
        assert_eq!(parse_background_repeat("sideways"), None);
        assert_eq!(parse_background_repeat("repeat-x repeat"), None);
    }

    // M3d step 1: angle units and modern colour functions in gradients.
    #[test]
    fn gradients_take_any_angle_unit_and_colour_function() {
        for (angle, deg) in [
            ("0.25turn", 90.0),
            ("100grad", 90.0),
            ("3.14159265rad", 180.0),
        ] {
            let Some(BackgroundImage::Linear { direction, .. }) =
                parse_background_image(&format!("linear-gradient({angle}, red, blue)")).unwrap()
            else {
                panic!("{angle}");
            };
            let GradientDirection::Angle(a) = direction else {
                panic!("{angle}");
            };
            assert!((a - deg).abs() < 1e-3, "{angle}: {a}");
        }
        let Some(BackgroundImage::Linear { stops, .. }) = parse_background_image(
            "linear-gradient(oklch(0.7 0.1 200) 10%, color-mix(in srgb, red 25%, blue))",
        )
        .unwrap() else {
            panic!("expected a linear gradient");
        };
        assert_eq!(stops.len(), 2);
        assert_eq!(stops[0].position, Some(Len::Percent(0.1)));
    }
}
