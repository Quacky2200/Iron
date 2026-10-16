//! Text style properties: their specified values (what a node sets) and computed
//! values (after CSS inheritance down the tree).
//!
//! All of these properties are inherited, as in CSS (`textDecoration` isn't
//! strictly inherited in CSS, but it visually propagates to descendants, which is
//! the same thing for us).

use iron_core::Value;

use crate::Rgba;
use crate::paint::{Shadow, parse_shadows};

/// A font size or spacing: pixels, or relative to a font size (`em`, `%`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FontLen {
    Px(f32),
    /// Multiple of the reference font size (`1.5em`, `150%`).
    Em(f32),
}

impl FontLen {
    fn parse(v: &Value) -> Option<Self> {
        match v {
            Value::Number(n) => Some(FontLen::Px(*n as f32)),
            Value::String(s) => {
                let s = s.trim();
                if let Some(n) = s.strip_suffix("em") {
                    return Some(FontLen::Em(n.trim().parse().ok()?));
                }
                if let Some(n) = s.strip_suffix('%') {
                    return Some(FontLen::Em(n.trim().parse::<f32>().ok()? / 100.0));
                }
                let n = s.strip_suffix("px").unwrap_or(s).trim();
                Some(FontLen::Px(n.parse().ok()?))
            }
            _ => None,
        }
    }

    fn resolve(self, font_size: f32) -> f32 {
        match self {
            FontLen::Px(px) => px,
            FontLen::Em(em) => em * font_size,
        }
    }
}

/// A `font-weight` as set: a number, or relative to the parent's weight.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FontWeightSpec {
    Absolute(f32),
    Bolder,
    Lighter,
}

impl FontWeightSpec {
    /// Numbers 1–1000, CSS's `normal`/`bold`/`bolder`/`lighter`, and the named
    /// weights s&box and font tools use (`thin` ... `black`).
    fn parse(value: &Value) -> Option<Self> {
        let w = match value {
            Value::Number(n) => *n as f32,
            Value::String(s) => match s.trim() {
                "bolder" => return Some(Self::Bolder),
                "lighter" => return Some(Self::Lighter),
                "thin" | "hairline" => 100.0,
                "extralight" | "extra-light" | "ultralight" | "ultra-light" => 200.0,
                "light" => 300.0,
                "normal" | "regular" => 400.0,
                "medium" => 500.0,
                "semibold" | "semi-bold" | "demibold" | "demi-bold" => 600.0,
                "bold" => 700.0,
                "extrabold" | "extra-bold" | "ultrabold" | "ultra-bold" => 800.0,
                "black" | "heavy" => 900.0,
                other => other.parse().ok()?,
            },
            _ => return None,
        };
        (1.0..=1000.0).contains(&w).then_some(Self::Absolute(w))
    }

    /// The computed weight, for a parent of weight `parent` (CSS's table for
    /// `bolder` / `lighter`).
    fn resolve(self, parent: f32) -> f32 {
        match self {
            Self::Absolute(w) => w,
            Self::Bolder if parent < 350.0 => 400.0,
            Self::Bolder if parent < 550.0 => 700.0,
            Self::Bolder => parent.max(900.0),
            Self::Lighter if parent < 100.0 => parent,
            Self::Lighter if parent < 550.0 => 100.0,
            Self::Lighter if parent < 750.0 => 400.0,
            Self::Lighter => 700.0,
        }
    }
}

/// `font-size`: lengths and the CSS keywords (`medium` is 16px; `smaller` and
/// `larger` scale the parent's size by 1.2).
fn parse_font_size(value: &Value) -> Option<FontLen> {
    let keyword = match value.as_str().map(str::trim) {
        Some("xx-small") => Some(FontLen::Px(9.0)),
        Some("x-small") => Some(FontLen::Px(10.0)),
        Some("small") => Some(FontLen::Px(13.0)),
        Some("medium") => Some(FontLen::Px(16.0)),
        Some("large") => Some(FontLen::Px(18.0)),
        Some("x-large") => Some(FontLen::Px(24.0)),
        Some("xx-large") => Some(FontLen::Px(32.0)),
        Some("xxx-large") => Some(FontLen::Px(48.0)),
        Some("smaller") => Some(FontLen::Em(1.0 / 1.2)),
        Some("larger") => Some(FontLen::Em(1.2)),
        _ => None,
    };
    keyword
        .or_else(|| FontLen::parse(value))
        .filter(|l| match l {
            FontLen::Px(n) | FontLen::Em(n) => *n >= 0.0,
        })
}

/// `font-variant-numeric` as OpenType features (`tabular-nums` → `"tnum" 1`).
fn parse_variant_numeric(s: &str) -> Option<String> {
    if s.trim() == "normal" {
        return Some(String::new());
    }
    let tags: Vec<&str> = s
        .split_whitespace()
        .map(|w| {
            Some(match w {
                "lining-nums" => "lnum",
                "oldstyle-nums" => "onum",
                "proportional-nums" => "pnum",
                "tabular-nums" => "tnum",
                "diagonal-fractions" => "frac",
                "stacked-fractions" => "afrc",
                "ordinal" => "ordn",
                "slashed-zero" => "zero",
                _ => return None,
            })
        })
        .collect::<Option<_>>()?;
    Some(
        tags.iter()
            .map(|t| format!("\"{t}\" 1"))
            .collect::<Vec<_>>()
            .join(", "),
    )
}

/// `font-feature-settings`: `normal`, or `"tag" [on | off | n]` items, normalised to
/// `"tag" n`.
fn parse_feature_settings(s: &str) -> Option<String> {
    if s.trim() == "normal" {
        return Some(String::new());
    }
    let items: Vec<String> = s
        .split(',')
        .map(|item| {
            let item = item.trim();
            let quote = item.chars().next().filter(|c| *c == '"' || *c == '\'')?;
            let rest = &item[1..];
            let end = rest.find(quote)?;
            let tag = &rest[..end];
            (tag.len() == 4 && tag.chars().all(|c| c.is_ascii_graphic())).then_some(())?;
            let value = match rest[end + 1..].trim() {
                "" | "on" => 1,
                "off" => 0,
                n => n.parse::<u16>().ok()?,
            };
            Some(format!("\"{tag}\" {value}"))
        })
        .collect::<Option<_>>()?;
    Some(items.join(", "))
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LineHeight {
    /// The font's own line spacing.
    Normal,
    /// Multiple of the element's font size (CSS unitless number).
    Factor(f32),
    /// Pixels, or relative to the font size when given as `em` / `%`.
    Len(FontLen),
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextAlign {
    #[default]
    Start,
    End,
    Left,
    Right,
    Center,
    Justify,
}

/// CSS `direction`: the base direction of text, and what `start` / `end` mean.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Direction {
    #[default]
    Ltr,
    Rtl,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WhiteSpace {
    /// Collapse spaces and wrap.
    #[default]
    Normal,
    /// Collapse spaces, never wrap.
    NoWrap,
    /// Keep spaces and newlines, never wrap.
    Pre,
    /// Keep spaces and newlines, wrap.
    PreWrap,
    /// Collapse spaces but keep newlines, wrap.
    PreLine,
    /// Like `pre-wrap`, but spaces at line ends take room and can wrap.
    BreakSpaces,
}

/// `word-break`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WordBreak {
    #[default]
    Normal,
    /// Lines may break between any two letters.
    BreakAll,
    /// No breaks inside CJK words either.
    KeepAll,
}

/// `overflow-wrap` (alias `word-wrap`): breaking words that don't fit a line.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OverflowWrap {
    #[default]
    Normal,
    /// Break anywhere if needed; also shrinks the min-content width.
    Anywhere,
    /// Break anywhere if needed.
    BreakWord,
}

/// `text-transform`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextTransform {
    #[default]
    None,
    Uppercase,
    Lowercase,
    /// The first letter of each word in upper case.
    Capitalize,
}

impl TextTransform {
    /// `text` transformed; `word_start` says whether the text begins a word and is
    /// updated for the next run (words continue across inline elements).
    pub fn apply(self, text: &str, word_start: &mut bool) -> String {
        let out = match self {
            Self::None => text.to_owned(),
            Self::Uppercase => text.to_uppercase(),
            Self::Lowercase => text.to_lowercase(),
            Self::Capitalize => {
                let mut out = String::with_capacity(text.len());
                let mut start = *word_start;
                for c in text.chars() {
                    if start && c.is_alphanumeric() {
                        out.extend(c.to_uppercase());
                    } else {
                        out.push(c);
                    }
                    start = !c.is_alphanumeric() && c != '\'' && c != '\u{2019}';
                }
                out
            }
        };
        if let Some(last) = text.chars().last() {
            *word_start = !last.is_alphanumeric();
        }
        out
    }
}

/// `text-overflow` (not inherited): what shows where clipped text is cut off.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextOverflow {
    #[default]
    Clip,
    Ellipsis,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Decoration {
    pub underline: bool,
    pub line_through: bool,
    pub overline: bool,
}

/// `text-decoration-style`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DecorationStyle {
    #[default]
    Solid,
    Double,
    Dotted,
    Dashed,
    Wavy,
}

impl DecorationStyle {
    fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "solid" => Self::Solid,
            "double" => Self::Double,
            "dotted" => Self::Dotted,
            "dashed" => Self::Dashed,
            "wavy" => Self::Wavy,
            _ => return None,
        })
    }
}

/// `text-decoration-thickness` / `text-underline-offset`: `auto` (`None`) or a length.
fn auto_len(s: &str) -> Option<Option<FontLen>> {
    match s {
        "auto" | "from-font" => Some(None),
        other => FontLen::parse(&Value::String(other.to_owned())).map(Some),
    }
}

/// Text properties as set on one node. `None` means "inherit".
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TextProps {
    pub color: Option<Rgba>,
    pub font_family: Option<String>,
    pub font_size: Option<FontLen>,
    pub font_weight: Option<FontWeightSpec>,
    pub italic: Option<bool>,
    pub line_height: Option<LineHeight>,
    pub letter_spacing: Option<FontLen>,
    pub word_spacing: Option<FontLen>,
    pub text_align: Option<TextAlign>,
    pub direction: Option<Direction>,
    pub decoration: Option<Decoration>,
    pub decoration_style: Option<DecorationStyle>,
    /// `Some(None)` is `currentColor`.
    pub decoration_color: Option<Option<Rgba>>,
    /// `Some(None)` is `auto` (from the font).
    pub decoration_thickness: Option<Option<FontLen>>,
    pub underline_offset: Option<Option<FontLen>>,
    /// `text-decoration-skip-ink`: parsed, but underlines don't skip descenders yet.
    pub skip_ink: Option<bool>,
    pub white_space: Option<WhiteSpace>,
    pub text_shadow: Option<Vec<Shadow>>,
    pub text_transform: Option<TextTransform>,
    pub word_break: Option<WordBreak>,
    pub overflow_wrap: Option<OverflowWrap>,
    /// OpenType features from `fontVariantNumeric` and `fontFeatureSettings`, in
    /// CSS `font-feature-settings` form (`"tnum" 1, ...`); empty for none.
    pub font_variant_numeric: Option<String>,
    pub font_feature_settings: Option<String>,
    /// Not inherited: `None` is `clip`.
    pub text_overflow: Option<TextOverflow>,
}

/// Fully resolved text style for a node.
#[derive(Clone, Debug, PartialEq)]
pub struct ComputedText {
    pub color: Rgba,
    pub font_family: String,
    pub font_size: f32,
    pub font_weight: f32,
    pub italic: bool,
    pub line_height: LineHeight,
    pub letter_spacing: f32,
    pub word_spacing: f32,
    pub text_align: TextAlign,
    pub direction: Direction,
    pub decoration: Decoration,
    pub decoration_style: DecorationStyle,
    /// `None`: the text colour.
    pub decoration_color: Option<Rgba>,
    /// Px; `None` uses the font's metrics.
    pub decoration_thickness: Option<f32>,
    /// Px below the baseline; `None` uses the font's metrics.
    pub underline_offset: Option<f32>,
    pub skip_ink: bool,
    pub white_space: WhiteSpace,
    pub text_shadow: Vec<Shadow>,
    pub text_transform: TextTransform,
    pub word_break: WordBreak,
    pub overflow_wrap: OverflowWrap,
    pub font_variant_numeric: String,
    pub font_feature_settings: String,
    pub text_overflow: TextOverflow,
}

impl Default for ComputedText {
    /// CSS initial values (black text, as in browsers), with `sans-serif` mapped to
    /// the bundled font.
    fn default() -> Self {
        Self {
            color: Rgba::new(0.0, 0.0, 0.0, 1.0),
            font_family: "sans-serif".into(),
            font_size: 16.0,
            font_weight: 400.0,
            italic: false,
            line_height: LineHeight::Normal,
            letter_spacing: 0.0,
            word_spacing: 0.0,
            text_align: TextAlign::Start,
            direction: Direction::Ltr,
            decoration: Decoration::default(),
            decoration_style: DecorationStyle::Solid,
            decoration_color: None,
            decoration_thickness: None,
            underline_offset: None,
            skip_ink: true,
            white_space: WhiteSpace::Normal,
            text_shadow: Vec::new(),
            text_transform: TextTransform::None,
            word_break: WordBreak::Normal,
            overflow_wrap: OverflowWrap::Normal,
            font_variant_numeric: String::new(),
            font_feature_settings: String::new(),
            text_overflow: TextOverflow::Clip,
        }
    }
}

impl ComputedText {
    /// The computed style of a child that sets `props`, inheriting the rest from `self`.
    pub fn child(&self, props: &TextProps) -> ComputedText {
        // `em` font sizes are relative to the parent's font size; spacing in `em`
        // is relative to the element's own font size.
        let font_size = props
            .font_size
            .map_or(self.font_size, |l| l.resolve(self.font_size).max(0.0));
        ComputedText {
            color: props.color.unwrap_or(self.color),
            font_family: props
                .font_family
                .clone()
                .unwrap_or_else(|| self.font_family.clone()),
            font_size,
            font_weight: props
                .font_weight
                .map_or(self.font_weight, |w| w.resolve(self.font_weight)),
            italic: props.italic.unwrap_or(self.italic),
            // A length line height computes to pixels; factors stay factors so
            // children with other font sizes scale with them (CSS behaviour).
            line_height: match props.line_height {
                Some(LineHeight::Len(l)) => LineHeight::Len(FontLen::Px(l.resolve(font_size))),
                Some(other) => other,
                None => self.line_height,
            },
            letter_spacing: props
                .letter_spacing
                .map_or(self.letter_spacing, |l| l.resolve(font_size)),
            word_spacing: props
                .word_spacing
                .map_or(self.word_spacing, |l| l.resolve(font_size)),
            text_align: props.text_align.unwrap_or(self.text_align),
            direction: props.direction.unwrap_or(self.direction),
            decoration: props.decoration.unwrap_or(self.decoration),
            decoration_style: props.decoration_style.unwrap_or(self.decoration_style),
            decoration_color: props.decoration_color.unwrap_or(self.decoration_color),
            decoration_thickness: props
                .decoration_thickness
                .map_or(self.decoration_thickness, |t| {
                    t.map(|l| l.resolve(font_size).max(0.0))
                }),
            underline_offset: props
                .underline_offset
                .map_or(self.underline_offset, |o| o.map(|l| l.resolve(font_size))),
            skip_ink: props.skip_ink.unwrap_or(self.skip_ink),
            white_space: props.white_space.unwrap_or(self.white_space),
            text_shadow: props
                .text_shadow
                .clone()
                .unwrap_or_else(|| self.text_shadow.clone()),
            text_transform: props.text_transform.unwrap_or(self.text_transform),
            word_break: props.word_break.unwrap_or(self.word_break),
            overflow_wrap: props.overflow_wrap.unwrap_or(self.overflow_wrap),
            font_variant_numeric: props
                .font_variant_numeric
                .clone()
                .unwrap_or_else(|| self.font_variant_numeric.clone()),
            font_feature_settings: props
                .font_feature_settings
                .clone()
                .unwrap_or_else(|| self.font_feature_settings.clone()),
            text_overflow: props.text_overflow.unwrap_or_default(),
        }
    }

    /// All OpenType features, in `font-feature-settings` form (settings win over
    /// `font-variant-numeric` for the same tag, as the later entry).
    pub fn font_features(&self) -> String {
        match (
            self.font_variant_numeric.is_empty(),
            self.font_feature_settings.is_empty(),
        ) {
            (true, _) => self.font_feature_settings.clone(),
            (false, true) => self.font_variant_numeric.clone(),
            (false, false) => format!(
                "{}, {}",
                self.font_variant_numeric, self.font_feature_settings
            ),
        }
    }

    /// Line height in pixels, or `None` for the font's normal line spacing.
    pub fn line_height_px(&self) -> Option<f32> {
        match self.line_height {
            LineHeight::Normal => None,
            LineHeight::Factor(f) => Some(f * self.font_size),
            LineHeight::Len(l) => Some(l.resolve(self.font_size)),
        }
    }
}

/// Property names handled here; all are inherited except `textOverflow`.
pub const TEXT_PROPS: &[&str] = &[
    "color",
    "fontFamily",
    "fontSize",
    "fontWeight",
    "fontStyle",
    "lineHeight",
    "letterSpacing",
    "wordSpacing",
    "textAlign",
    "direction",
    "textDecoration",
    "textDecorationLine",
    "textDecorationStyle",
    "textDecorationColor",
    "textDecorationThickness",
    "textUnderlineOffset",
    "textDecorationSkipInk",
    "whiteSpace",
    "textShadow",
    "textTransform",
    "textOverflow",
    "wordBreak",
    "overflowWrap",
    "wordWrap",
    "font",
    "fontVariantNumeric",
    "fontFeatureSettings",
];

pub fn is_text_prop(prop: &str) -> bool {
    TEXT_PROPS.contains(&prop)
}

/// Applies a text property. Returns `None` if `prop` isn't a text property,
/// `Some(Err(()))` for an invalid value (nothing changes) and `Some(Ok(()))` otherwise.
/// `Value::Null` resets the property to "inherit".
pub fn apply(props: &mut TextProps, prop: &str, value: &Value) -> Option<Result<(), ()>> {
    if !is_text_prop(prop) {
        return None;
    }
    if *value == Value::Null {
        match prop {
            "color" => props.color = None,
            "fontFamily" => props.font_family = None,
            "fontSize" => props.font_size = None,
            "fontWeight" => props.font_weight = None,
            "fontStyle" => props.italic = None,
            "lineHeight" => props.line_height = None,
            "letterSpacing" => props.letter_spacing = None,
            "wordSpacing" => props.word_spacing = None,
            "textAlign" => props.text_align = None,
            "direction" => props.direction = None,
            "textDecorationLine" => props.decoration = None,
            "textDecoration" => {
                props.decoration = None;
                props.decoration_style = None;
                props.decoration_color = None;
                props.decoration_thickness = None;
            }
            "textDecorationStyle" => props.decoration_style = None,
            "textDecorationColor" => props.decoration_color = None,
            "textDecorationThickness" => props.decoration_thickness = None,
            "textUnderlineOffset" => props.underline_offset = None,
            "textDecorationSkipInk" => props.skip_ink = None,
            "whiteSpace" => props.white_space = None,
            "textShadow" => props.text_shadow = None,
            "textTransform" => props.text_transform = None,
            "textOverflow" => props.text_overflow = None,
            "wordBreak" => props.word_break = None,
            "overflowWrap" | "wordWrap" => props.overflow_wrap = None,
            "font" => {
                props.font_size = None;
                props.font_weight = None;
                props.italic = None;
                props.line_height = None;
                props.font_family = None;
            }
            "fontVariantNumeric" => props.font_variant_numeric = None,
            "fontFeatureSettings" => props.font_feature_settings = None,
            _ => unreachable!("listed in TEXT_PROPS"),
        }
        return Some(Ok(()));
    }
    let s = value.as_str().map(str::trim);
    // Each arm parses its value or returns `Some(Err(()))`, then assigns.
    macro_rules! parse {
        ($e:expr) => {
            match $e {
                Some(v) => v,
                None => return Some(Err(())),
            }
        };
    }
    match prop {
        "color" => props.color = Some(parse!(s.and_then(Rgba::parse))),
        "fontFamily" => {
            props.font_family = Some(parse!(s.filter(|f| !f.is_empty())).to_owned());
        }
        "fontSize" => props.font_size = Some(parse!(parse_font_size(value))),
        "fontWeight" => props.font_weight = Some(parse!(FontWeightSpec::parse(value))),
        "font" => {
            let font = parse!(s.and_then(parse_font_shorthand));
            props.italic = Some(font.italic);
            props.font_weight = Some(font.weight);
            props.font_size = Some(font.size);
            props.line_height = Some(font.line_height);
            props.font_family = Some(font.family);
        }
        "fontVariantNumeric" => {
            props.font_variant_numeric = Some(parse!(s.and_then(parse_variant_numeric)));
        }
        "fontFeatureSettings" => {
            props.font_feature_settings = Some(parse!(s.and_then(parse_feature_settings)));
        }
        "fontStyle" => {
            props.italic = Some(parse!(match s {
                Some("normal") => Some(false),
                Some("italic" | "oblique") => Some(true),
                _ => None,
            }));
        }
        "lineHeight" => {
            let lh = match (value, s) {
                (Value::Number(n), _) => Some(LineHeight::Factor(*n as f32)),
                (_, Some("normal")) => Some(LineHeight::Normal),
                (_, Some(t)) => match t.parse::<f32>() {
                    Ok(n) => Some(LineHeight::Factor(n)),
                    Err(_) => FontLen::parse(value).map(LineHeight::Len),
                },
                _ => None,
            };
            props.line_height = Some(parse!(lh));
        }
        "letterSpacing" | "wordSpacing" => {
            let len = parse!(match s {
                Some("normal") => Some(FontLen::Px(0.0)),
                _ => FontLen::parse(value),
            });
            if prop == "letterSpacing" {
                props.letter_spacing = Some(len);
            } else {
                props.word_spacing = Some(len);
            }
        }
        "textAlign" => {
            props.text_align = Some(parse!(match s {
                Some("start") => Some(TextAlign::Start),
                Some("end") => Some(TextAlign::End),
                Some("left") => Some(TextAlign::Left),
                Some("right") => Some(TextAlign::Right),
                Some("center") => Some(TextAlign::Center),
                Some("justify") => Some(TextAlign::Justify),
                _ => None,
            }));
        }
        "direction" => {
            props.direction = Some(parse!(match s {
                Some("ltr") => Some(Direction::Ltr),
                Some("rtl") => Some(Direction::Rtl),
                _ => None,
            }));
        }
        "textDecoration" | "textDecorationLine" => {
            let shorthand = prop == "textDecoration";
            let mut d = Decoration::default();
            let (mut style, mut color, mut thickness) = (None, None, None);
            for word in crate::paint::split_top_level(parse!(s), ' ') {
                match word {
                    "none" => {}
                    "underline" => d.underline = true,
                    "line-through" => d.line_through = true,
                    "overline" => d.overline = true,
                    w if shorthand && style.is_none() && DecorationStyle::parse(w).is_some() => {
                        style = DecorationStyle::parse(w);
                    }
                    w if shorthand && thickness.is_none() && auto_len(w).is_some() => {
                        thickness = auto_len(w);
                    }
                    w if shorthand && color.is_none() && w.eq_ignore_ascii_case("currentcolor") => {
                        color = Some(None);
                    }
                    w if shorthand && color.is_none() => color = Some(Some(parse!(Rgba::parse(w)))),
                    _ => return Some(Err(())),
                }
            }
            props.decoration = Some(d);
            if shorthand {
                // The shorthand resets what it leaves out.
                props.decoration_style = Some(style.unwrap_or_default());
                props.decoration_color = Some(color.flatten());
                props.decoration_thickness = Some(thickness.flatten());
            }
        }
        "textDecorationStyle" => {
            props.decoration_style = Some(parse!(s.and_then(DecorationStyle::parse)));
        }
        "textDecorationColor" => {
            props.decoration_color = Some(parse!(match s {
                Some(c) if c.eq_ignore_ascii_case("currentcolor") => Some(None),
                Some(c) => Rgba::parse(c).map(Some),
                None => None,
            }));
        }
        "textDecorationThickness" | "textUnderlineOffset" => {
            let len = match value {
                Value::Number(n) => Some(Some(FontLen::Px(*n as f32))),
                _ => s.and_then(auto_len),
            };
            if prop == "textDecorationThickness" {
                props.decoration_thickness = Some(parse!(len));
            } else {
                props.underline_offset = Some(parse!(len));
            }
        }
        "textDecorationSkipInk" => {
            props.skip_ink = Some(parse!(match s {
                Some("auto" | "all") => Some(true),
                Some("none") => Some(false),
                _ => None,
            }));
        }
        "whiteSpace" => {
            props.white_space = Some(parse!(match s {
                Some("normal") => Some(WhiteSpace::Normal),
                Some("nowrap") => Some(WhiteSpace::NoWrap),
                Some("pre") => Some(WhiteSpace::Pre),
                Some("pre-wrap") => Some(WhiteSpace::PreWrap),
                Some("pre-line") => Some(WhiteSpace::PreLine),
                Some("break-spaces") => Some(WhiteSpace::BreakSpaces),
                _ => None,
            }));
        }
        "textTransform" => {
            props.text_transform = Some(parse!(match s {
                Some("none") => Some(TextTransform::None),
                Some("uppercase") => Some(TextTransform::Uppercase),
                Some("lowercase") => Some(TextTransform::Lowercase),
                Some("capitalize") => Some(TextTransform::Capitalize),
                _ => None,
            }));
        }
        "wordBreak" => {
            let (word_break, wrap) = parse!(match s {
                Some("normal") => Some((WordBreak::Normal, None)),
                Some("break-all") => Some((WordBreak::BreakAll, None)),
                Some("keep-all") => Some((WordBreak::KeepAll, None)),
                // Deprecated, but common: `normal` plus `overflow-wrap: anywhere`.
                Some("break-word") => Some((WordBreak::Normal, Some(OverflowWrap::Anywhere))),
                _ => None,
            });
            props.word_break = Some(word_break);
            if wrap.is_some() {
                props.overflow_wrap = wrap;
            }
        }
        "overflowWrap" | "wordWrap" => {
            props.overflow_wrap = Some(parse!(match s {
                Some("normal") => Some(OverflowWrap::Normal),
                Some("anywhere") => Some(OverflowWrap::Anywhere),
                Some("break-word") => Some(OverflowWrap::BreakWord),
                _ => None,
            }));
        }
        "textOverflow" => {
            props.text_overflow = Some(parse!(match s {
                Some("clip") => Some(TextOverflow::Clip),
                Some("ellipsis") => Some(TextOverflow::Ellipsis),
                _ => None,
            }));
        }
        // CSS text shadows have no spread and can't be inset.
        "textShadow" => {
            props.text_shadow =
                Some(parse!(s.and_then(parse_shadows).filter(|list| {
                    list.iter().all(|sh| !sh.inset && sh.spread == 0.0)
                })));
        }
        _ => unreachable!("listed in TEXT_PROPS"),
    }
    Some(Ok(()))
}

struct FontShorthand {
    italic: bool,
    weight: FontWeightSpec,
    size: FontLen,
    line_height: LineHeight,
    family: String,
}

/// `font: [style] [variant] [weight] size[/line-height] family`; what's left out
/// goes back to its initial value, as in CSS.
fn parse_font_shorthand(s: &str) -> Option<FontShorthand> {
    let mut font = FontShorthand {
        italic: false,
        weight: FontWeightSpec::Absolute(400.0),
        size: FontLen::Px(16.0),
        line_height: LineHeight::Normal,
        family: String::new(),
    };
    let mut rest = s.trim();
    loop {
        let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
        let token = &rest[..end];
        rest = rest[end..].trim_start();
        let (size, line_height) = match token.split_once('/') {
            Some((size, lh)) => (size, Some(lh)),
            None => (token, None),
        };
        // A bare number is a weight; the size needs a unit (or a keyword).
        let is_number = size.parse::<f32>().is_ok();
        if let Some(size) = parse_font_size(&Value::String(size.to_owned())).filter(|_| !is_number)
        {
            font.size = size;
            if let Some(lh) = line_height {
                font.line_height = match lh.parse::<f32>() {
                    Ok(n) => LineHeight::Factor(n),
                    Err(_) if lh == "normal" => LineHeight::Normal,
                    Err(_) => LineHeight::Len(FontLen::parse(&Value::String(lh.to_owned()))?),
                };
            }
            font.family = rest.trim().to_owned();
            return (!font.family.is_empty()).then_some(font);
        }
        match token {
            "normal" | "small-caps" => {}
            "italic" | "oblique" => font.italic = true,
            t => font.weight = FontWeightSpec::parse(&Value::String(t.to_owned()))?,
        }
        if rest.is_empty() {
            return None;
        }
    }
}

// M3-TXT-04, M3-TXT-06
#[cfg(test)]
mod tests {
    use super::*;

    // M3-TXT-13
    #[test]
    fn decoration_shorthand_and_longhands() {
        let mut p = TextProps::default();
        set(
            &mut p,
            "textDecoration",
            "underline overline wavy red 2px".into(),
        );
        assert_eq!(
            p.decoration,
            Some(Decoration {
                underline: true,
                line_through: false,
                overline: true
            })
        );
        assert_eq!(p.decoration_style, Some(DecorationStyle::Wavy));
        assert_eq!(
            p.decoration_color,
            Some(Some(Rgba::new(1.0, 0.0, 0.0, 1.0)))
        );
        assert_eq!(p.decoration_thickness, Some(Some(FontLen::Px(2.0))));
        // The shorthand resets what it leaves out; the line longhand doesn't.
        set(&mut p, "textDecorationLine", "line-through".into());
        assert_eq!(p.decoration_style, Some(DecorationStyle::Wavy));
        set(&mut p, "textDecoration", "underline".into());
        assert_eq!(p.decoration_style, Some(DecorationStyle::Solid));
        assert_eq!(p.decoration_color, Some(None), "currentColor");
        set(&mut p, "textDecorationThickness", "0.1em".into());
        set(&mut p, "textUnderlineOffset", 3.0.into());
        set(&mut p, "textDecorationSkipInk", "none".into());
        let c = ComputedText::default().child(&p);
        assert_eq!(c.decoration_thickness, Some(1.6));
        assert_eq!(c.underline_offset, Some(3.0));
        assert!(!c.skip_ink);
        for (prop, bad) in [
            ("textDecoration", "underline wavy dotted"),
            ("textDecorationStyle", "zigzag"),
            ("textDecorationThickness", "thick"),
            ("textDecorationSkipInk", "maybe"),
        ] {
            assert_eq!(
                apply(&mut p, prop, &bad.into()),
                Some(Err(())),
                "{prop}: {bad}"
            );
        }
    }

    // M3-TXT-12
    #[test]
    fn font_shorthand_and_keywords() {
        let mut p = TextProps::default();
        set(
            &mut p,
            "font",
            "italic bold 20px/1.5 Inter, sans-serif".into(),
        );
        assert_eq!(p.italic, Some(true));
        assert_eq!(p.font_weight, Some(FontWeightSpec::Absolute(700.0)));
        assert_eq!(p.font_size, Some(FontLen::Px(20.0)));
        assert_eq!(p.line_height, Some(LineHeight::Factor(1.5)));
        assert_eq!(p.font_family.as_deref(), Some("Inter, sans-serif"));
        // Left-out parts reset; a bare number is a weight.
        set(&mut p, "font", "600 large serif".into());
        assert_eq!(p.italic, Some(false));
        assert_eq!(p.font_weight, Some(FontWeightSpec::Absolute(600.0)));
        assert_eq!(p.font_size, Some(FontLen::Px(18.0)));
        assert_eq!(p.line_height, Some(LineHeight::Normal));
        for bad in ["12px", "bold", "wobbly 12px serif", "12px/tall serif"] {
            assert_eq!(apply(&mut p, "font", &bad.into()), Some(Err(())), "{bad}");
        }

        let computed = |props: &[(&str, &str)]| {
            let mut c = ComputedText::default();
            for (prop, value) in props {
                let mut p = TextProps::default();
                set(&mut p, prop, (*value).into());
                c = c.child(&p);
            }
            c
        };
        let c = computed(&[("fontSize", "smaller")]);
        assert!((c.font_size - 16.0 / 1.2).abs() < 1e-4);
        assert_eq!(computed(&[("fontSize", "xx-large")]).font_size, 32.0);
        assert_eq!(computed(&[("fontWeight", "semibold")]).font_weight, 600.0);
        assert_eq!(
            computed(&[("fontWeight", "bolder"), ("fontWeight", "bolder")]).font_weight,
            900.0,
            "400 → 700 → 900"
        );
        assert_eq!(
            computed(&[("fontWeight", "black"), ("fontWeight", "lighter")]).font_weight,
            700.0
        );
    }

    // M3-TXT-12
    #[test]
    fn font_features_from_variants_and_settings() {
        let mut p = TextProps::default();
        set(
            &mut p,
            "fontVariantNumeric",
            "tabular-nums slashed-zero".into(),
        );
        set(&mut p, "fontFeatureSettings", "'liga' off, \"ss01\"".into());
        let c = ComputedText::default().child(&p);
        assert_eq!(
            c.font_features(),
            r#""tnum" 1, "zero" 1, "liga" 0, "ss01" 1"#
        );
        for (prop, bad) in [
            ("fontVariantNumeric", "tabular"),
            ("fontFeatureSettings", "liga"),
            ("fontFeatureSettings", "'toolong' 1"),
        ] {
            assert_eq!(
                apply(&mut p, prop, &bad.into()),
                Some(Err(())),
                "{prop}: {bad}"
            );
        }
        set(&mut p, "fontVariantNumeric", "normal".into());
        set(&mut p, "fontFeatureSettings", "normal".into());
        assert_eq!(ComputedText::default().child(&p).font_features(), "");
    }

    // M3-TXT-10
    #[test]
    fn text_transform_applies_across_runs() {
        let mut start = true;
        let caps = TextTransform::Capitalize;
        assert_eq!(
            caps.apply("hello wORLD it's", &mut start),
            "Hello WORLD It's"
        );
        // A run continuing a word isn't capitalized again.
        let mut start = true;
        assert_eq!(caps.apply("foo", &mut start), "Foo");
        assert_eq!(caps.apply("bar baz", &mut start), "bar Baz");
        let mut start = true;
        assert_eq!(
            TextTransform::Uppercase.apply("straße", &mut start),
            "STRASSE"
        );
        let mut p = TextProps::default();
        assert_eq!(
            apply(&mut p, "textTransform", &"shout".into()),
            Some(Err(()))
        );
        assert_eq!(
            apply(&mut p, "textOverflow", &"ellipsis".into()),
            Some(Ok(()))
        );
        let parent = ComputedText::default().child(&p);
        assert_eq!(parent.text_overflow, TextOverflow::Ellipsis);
        assert_eq!(
            parent.child(&TextProps::default()).text_overflow,
            TextOverflow::Clip,
            "not inherited"
        );
    }

    fn set(props: &mut TextProps, prop: &str, value: Value) {
        apply(props, prop, &value)
            .expect("text prop")
            .unwrap_or_else(|_| panic!("{prop}: {value:?} should parse"));
    }

    #[test]
    fn parses_text_properties() {
        let mut p = TextProps::default();
        set(&mut p, "color", "#ff0000".into());
        set(&mut p, "fontFamily", "Charm, serif".into());
        set(&mut p, "fontSize", "20px".into());
        set(&mut p, "fontWeight", "bold".into());
        set(&mut p, "fontStyle", "italic".into());
        set(&mut p, "lineHeight", 1.5.into());
        set(&mut p, "letterSpacing", "0.1em".into());
        set(&mut p, "textAlign", "center".into());
        set(&mut p, "textDecoration", "underline line-through".into());
        set(&mut p, "whiteSpace", "pre-wrap".into());
        set(
            &mut p,
            "textShadow",
            "1px 2px 3px black, 0 0 4px red".into(),
        );
        assert_eq!(p.font_size, Some(FontLen::Px(20.0)));
        assert_eq!(p.font_weight, Some(FontWeightSpec::Absolute(700.0)));
        assert_eq!(p.italic, Some(true));
        assert_eq!(p.line_height, Some(LineHeight::Factor(1.5)));
        assert_eq!(p.letter_spacing, Some(FontLen::Em(0.1)));
        assert_eq!(
            p.decoration,
            Some(Decoration {
                underline: true,
                line_through: true,
                overline: false
            })
        );
        assert_eq!(p.text_shadow.as_ref().map(Vec::len), Some(2));

        set(&mut p, "fontWeight", 300.0.into());
        assert_eq!(p.font_weight, Some(FontWeightSpec::Absolute(300.0)));
        set(&mut p, "lineHeight", "24px".into());
        assert_eq!(p.line_height, Some(LineHeight::Len(FontLen::Px(24.0))));
        set(&mut p, "lineHeight", "normal".into());
        assert_eq!(p.line_height, Some(LineHeight::Normal));
    }

    #[test]
    fn rejects_bad_values_without_changing_anything() {
        let mut p = TextProps::default();
        set(&mut p, "fontSize", 12.0.into());
        for (prop, value) in [
            ("fontSize", Value::from("-2px")),
            ("fontWeight", "weighty".into()),
            ("fontWeight", 0.0.into()),
            ("fontStyle", "slanted".into()),
            ("textAlign", "middle".into()),
            ("textDecoration", "squiggly".into()),
            ("whiteSpace", "pre-lines".into()),
            ("direction", "sideways".into()),
            ("textShadow", "inset 0 0 2px red".into()),
            ("textShadow", "0 0 2px 3px red".into()),
            ("color", "nope".into()),
        ] {
            assert_eq!(
                apply(&mut p, prop, &value),
                Some(Err(())),
                "{prop}: {value:?}"
            );
        }
        assert_eq!(p.font_size, Some(FontLen::Px(12.0)));
        assert_eq!(
            apply(&mut p, "width", &10.0.into()),
            None,
            "not a text prop"
        );
    }

    // M3-TXT-09
    #[test]
    fn direction_parses_and_inherits() {
        let mut parent_props = TextProps::default();
        set(&mut parent_props, "direction", "rtl".into());
        let parent = ComputedText::default().child(&parent_props);
        assert_eq!(parent.direction, Direction::Rtl);
        assert_eq!(
            parent.child(&TextProps::default()).direction,
            Direction::Rtl,
            "direction inherits"
        );
        let mut child_props = TextProps::default();
        set(&mut child_props, "direction", "ltr".into());
        assert_eq!(parent.child(&child_props).direction, Direction::Ltr);
        set(&mut child_props, "direction", Value::Null);
        assert_eq!(child_props.direction, None);
        assert_eq!(ComputedText::default().direction, Direction::Ltr);
    }

    #[test]
    fn inheritance_and_relative_units() {
        let root = ComputedText::default();
        let mut parent_props = TextProps::default();
        set(&mut parent_props, "fontSize", 20.0.into());
        set(&mut parent_props, "color", "red".into());
        set(&mut parent_props, "lineHeight", 1.5.into());
        let parent = root.child(&parent_props);

        let mut child_props = TextProps::default();
        set(&mut child_props, "fontSize", "1.5em".into()); // 30px, relative to the parent
        set(&mut child_props, "letterSpacing", "0.1em".into()); // 3px, relative to itself
        let child = parent.child(&child_props);

        assert_eq!(child.font_size, 30.0);
        assert!((child.letter_spacing - 3.0).abs() < 1e-5);
        assert_eq!(
            child.color,
            Rgba::new(1.0, 0.0, 0.0, 1.0),
            "colour inherits"
        );
        // A factor line height scales with the child's own font size.
        assert_eq!(child.line_height_px(), Some(45.0));
        // A length line height is fixed at the element that sets it.
        let mut fixed = TextProps::default();
        set(&mut fixed, "lineHeight", "2em".into());
        let fixed = parent.child(&fixed); // 2em of 20px = 40px
        let grandchild = fixed.child(&child_props);
        assert_eq!(grandchild.line_height_px(), Some(40.0));

        // Resetting to null inherits again.
        apply(&mut child_props, "fontSize", &Value::Null);
        assert_eq!(parent.child(&child_props).font_size, 20.0);
    }
}
