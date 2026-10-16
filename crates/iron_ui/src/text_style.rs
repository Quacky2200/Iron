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
}

/// Text properties as set on one node. `None` means "inherit".
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TextProps {
    pub color: Option<Rgba>,
    pub font_family: Option<String>,
    pub font_size: Option<FontLen>,
    pub font_weight: Option<f32>,
    pub italic: Option<bool>,
    pub line_height: Option<LineHeight>,
    pub letter_spacing: Option<FontLen>,
    pub word_spacing: Option<FontLen>,
    pub text_align: Option<TextAlign>,
    pub direction: Option<Direction>,
    pub decoration: Option<Decoration>,
    pub white_space: Option<WhiteSpace>,
    pub text_shadow: Option<Vec<Shadow>>,
    pub text_transform: Option<TextTransform>,
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
    pub white_space: WhiteSpace,
    pub text_shadow: Vec<Shadow>,
    pub text_transform: TextTransform,
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
            white_space: WhiteSpace::Normal,
            text_shadow: Vec::new(),
            text_transform: TextTransform::None,
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
            font_weight: props.font_weight.unwrap_or(self.font_weight),
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
            white_space: props.white_space.unwrap_or(self.white_space),
            text_shadow: props
                .text_shadow
                .clone()
                .unwrap_or_else(|| self.text_shadow.clone()),
            text_transform: props.text_transform.unwrap_or(self.text_transform),
            text_overflow: props.text_overflow.unwrap_or_default(),
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
    "whiteSpace",
    "textShadow",
    "textTransform",
    "textOverflow",
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
            "textDecoration" | "textDecorationLine" => props.decoration = None,
            "whiteSpace" => props.white_space = None,
            "textShadow" => props.text_shadow = None,
            "textTransform" => props.text_transform = None,
            "textOverflow" => props.text_overflow = None,
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
        "fontSize" => {
            props.font_size = Some(parse!(FontLen::parse(value).filter(|l| match l {
                FontLen::Px(n) | FontLen::Em(n) => *n >= 0.0,
            })));
        }
        "fontWeight" => {
            let w = match (value, s) {
                (Value::Number(n), _) => Some(*n as f32),
                (_, Some("normal")) => Some(400.0),
                (_, Some("bold")) => Some(700.0),
                (_, Some(other)) => other.parse().ok(),
                _ => None,
            };
            props.font_weight = Some(parse!(w.filter(|w| (1.0..=1000.0).contains(w))));
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
            let mut d = Decoration::default();
            for word in parse!(s).split_whitespace() {
                match word {
                    "none" => {}
                    "underline" => d.underline = true,
                    "line-through" => d.line_through = true,
                    _ => return Some(Err(())),
                }
            }
            props.decoration = Some(d);
        }
        "whiteSpace" => {
            props.white_space = Some(parse!(match s {
                Some("normal") => Some(WhiteSpace::Normal),
                Some("nowrap") => Some(WhiteSpace::NoWrap),
                Some("pre") => Some(WhiteSpace::Pre),
                Some("pre-wrap") => Some(WhiteSpace::PreWrap),
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

// M3-TXT-04, M3-TXT-06
#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(p.font_weight, Some(700.0));
        assert_eq!(p.italic, Some(true));
        assert_eq!(p.line_height, Some(LineHeight::Factor(1.5)));
        assert_eq!(p.letter_spacing, Some(FontLen::Em(0.1)));
        assert_eq!(
            p.decoration,
            Some(Decoration {
                underline: true,
                line_through: true
            })
        );
        assert_eq!(p.text_shadow.as_ref().map(Vec::len), Some(2));

        set(&mut p, "fontWeight", 300.0.into());
        assert_eq!(p.font_weight, Some(300.0));
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
            ("fontWeight", "heavy".into()),
            ("fontWeight", 0.0.into()),
            ("fontStyle", "slanted".into()),
            ("textAlign", "middle".into()),
            ("textDecoration", "wavy".into()),
            ("whiteSpace", "pre-line".into()),
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
