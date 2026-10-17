//! Transitions and animations (M3d group 7): timing functions, interpolation of
//! computed CSS values, and the `transition*` / `animation*` properties.
//!
//! Values are interpolated as CSS text, token by token: numbers with the same unit
//! blend, numbers with different units blend through `calc()`, colours blend in
//! premultiplied sRGB, and everything else must match exactly. That covers lengths,
//! colours, transforms and filters with the same functions, shadows with the same
//! layers, and so on; `none` blends with a function list or a shadow as its neutral
//! form (zero translations, unit scales, transparent shadows). Anything else isn't
//! interpolable: transitions skip it and animations flip halfway, as in CSS.

use crate::Rgba;

/// An easing function (`transition-timing-function`, `animation-timing-function`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Timing {
    Linear,
    CubicBezier(f32, f32, f32, f32),
    /// `steps(n, position)`.
    Steps(u32, StepPosition),
}

/// Where a `steps()` jump happens.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepPosition {
    Start,
    End,
    None,
    Both,
}

impl Default for Timing {
    /// `ease`, the CSS initial value.
    fn default() -> Self {
        Self::EASE
    }
}

impl Timing {
    pub const EASE: Timing = Timing::CubicBezier(0.25, 0.1, 0.25, 1.0);

    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim();
        Some(match s {
            "linear" => Timing::Linear,
            "ease" => Timing::EASE,
            "ease-in" => Timing::CubicBezier(0.42, 0.0, 1.0, 1.0),
            "ease-out" => Timing::CubicBezier(0.0, 0.0, 0.58, 1.0),
            "ease-in-out" => Timing::CubicBezier(0.42, 0.0, 0.58, 1.0),
            "step-start" => Timing::Steps(1, StepPosition::Start),
            "step-end" => Timing::Steps(1, StepPosition::End),
            _ => {
                let (name, args) = s.strip_suffix(')')?.split_once('(')?;
                let args: Vec<&str> = args.split(',').map(str::trim).collect();
                match (name.trim(), &args[..]) {
                    ("cubic-bezier", [a, b, c, d]) => {
                        let n = |v: &str| v.parse::<f32>().ok().filter(|n| n.is_finite());
                        let (x1, y1, x2, y2) = (n(a)?, n(b)?, n(c)?, n(d)?);
                        // x must stay in [0, 1] so time keeps moving forward.
                        if !(0.0..=1.0).contains(&x1) || !(0.0..=1.0).contains(&x2) {
                            return None;
                        }
                        Timing::CubicBezier(x1, y1, x2, y2)
                    }
                    ("steps", [n, rest @ ..]) if rest.len() <= 1 => {
                        let n: u32 = n.parse().ok()?;
                        let position = match rest.first().copied() {
                            None | Some("end" | "jump-end") => StepPosition::End,
                            Some("start" | "jump-start") => StepPosition::Start,
                            Some("jump-none") => StepPosition::None,
                            Some("jump-both") => StepPosition::Both,
                            _ => return None,
                        };
                        let min = if position == StepPosition::None { 2 } else { 1 };
                        if n < min {
                            return None;
                        }
                        Timing::Steps(n, position)
                    }
                    _ => return None,
                }
            }
        })
    }

    /// The eased progress at input progress `t` (0..=1).
    pub fn eval(self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Timing::Linear => t,
            Timing::CubicBezier(x1, y1, x2, y2) => {
                let bezier = |a: f32, b: f32, s: f32| {
                    let u = 1.0 - s;
                    3.0 * u * u * s * a + 3.0 * u * s * s * b + s * s * s
                };
                // Solve x(s) = t: Newton's method, then bisection if it stalls.
                let mut s = t;
                for _ in 0..8 {
                    let x = bezier(x1, x2, s) - t;
                    if x.abs() < 1e-6 {
                        return bezier(y1, y2, s);
                    }
                    let u = 1.0 - s;
                    let dx = 3.0 * u * u * x1 + 6.0 * u * s * (x2 - x1) + 3.0 * s * s * (1.0 - x2);
                    if dx.abs() < 1e-6 {
                        break;
                    }
                    s = (s - x / dx).clamp(0.0, 1.0);
                }
                let (mut lo, mut hi) = (0.0_f32, 1.0_f32);
                s = t;
                for _ in 0..30 {
                    let x = bezier(x1, x2, s);
                    if (x - t).abs() < 1e-6 {
                        break;
                    }
                    if x < t {
                        lo = s;
                    } else {
                        hi = s;
                    }
                    s = (lo + hi) / 2.0;
                }
                bezier(y1, y2, s)
            }
            Timing::Steps(n, position) => {
                let n = n as f32;
                let mut step = (t * n).floor();
                if matches!(position, StepPosition::Start | StepPosition::Both) {
                    step += 1.0;
                }
                if t >= 1.0 && position != StepPosition::Start && position != StepPosition::Both {
                    step = n;
                }
                let jumps = match position {
                    StepPosition::Start | StepPosition::End => n,
                    StepPosition::None => n - 1.0,
                    StepPosition::Both => n + 1.0,
                };
                (step / jumps).clamp(0.0, 1.0)
            }
        }
    }
}

/// One `transition` entry.
#[derive(Clone, Debug, PartialEq)]
pub struct TransitionSpec {
    /// camelCase property name, or `all`.
    pub property: String,
    pub duration_ms: f64,
    pub delay_ms: f64,
    pub timing: Timing,
}

impl TransitionSpec {
    /// Whether this entry applies to a change of `prop` (camelCase). A shorthand and
    /// its longhands match each other (`background` ↔ `backgroundColor`).
    pub fn matches(&self, prop: &str) -> bool {
        self.property == "all" || related(&self.property, prop)
    }
}

/// Whether two camelCase property names are the same property or a shorthand and one
/// of its longhands.
pub fn related(a: &str, b: &str) -> bool {
    let longhand_of = |long: &str, short: &str| {
        long.strip_prefix(short)
            .is_some_and(|rest| rest.starts_with(|c: char| c.is_ascii_uppercase()))
    };
    a == b || longhand_of(a, b) || longhand_of(b, a)
}

/// The `transition*` longhands as set on an element (lists, combined by index).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TransitionLists {
    pub property: Vec<String>,
    pub duration: Vec<f64>,
    pub delay: Vec<f64>,
    pub timing: Vec<Timing>,
}

impl TransitionLists {
    /// The entries: one per property, the other lists repeating to match (CSS).
    pub fn specs(&self) -> Vec<TransitionSpec> {
        // `transition-property` defaults to `all`.
        let all = ["all".to_owned()];
        let properties = if self.property.is_empty() {
            &all[..]
        } else {
            &self.property[..]
        };
        let pick = |list: &[f64], i: usize| {
            if list.is_empty() {
                0.0
            } else {
                list[i % list.len()]
            }
        };
        properties
            .iter()
            .enumerate()
            .filter(|(_, p)| p.as_str() != "none")
            .map(|(i, p)| TransitionSpec {
                property: p.clone(),
                duration_ms: pick(&self.duration, i),
                delay_ms: pick(&self.delay, i),
                timing: if self.timing.is_empty() {
                    Timing::EASE
                } else {
                    self.timing[i % self.timing.len()]
                },
            })
            .filter(|s| s.duration_ms > 0.0)
            .collect()
    }

    pub fn is_empty(&self) -> bool {
        self.specs().is_empty()
    }
}

/// A CSS time (`0.3s`, `150ms`) in milliseconds.
pub fn parse_time(s: &str) -> Option<f64> {
    let s = s.trim();
    let (n, scale) = if let Some(n) = s.strip_suffix("ms") {
        (n, 1.0)
    } else if let Some(n) = s.strip_suffix('s') {
        (n, 1000.0)
    } else if s == "0" {
        ("0", 1.0)
    } else {
        return None;
    };
    let v: f64 = n.trim().parse().ok()?;
    v.is_finite().then_some(v * scale)
}

/// A comma list, each item parsed by `f`.
pub fn parse_list<T>(s: &str, f: impl Fn(&str) -> Option<T>) -> Option<Vec<T>> {
    crate::paint::split_top_level(s, ',')
        .into_iter()
        .map(|p| f(p.trim()))
        .collect()
}

/// `transition-property`: `none`, `all` or property names (kebab-case in CSS).
pub fn parse_property(s: &str) -> Option<String> {
    let s = s.trim();
    let ok = !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    ok.then(|| crate::css::camel_case(s))
}

/// Parses the `transition` shorthand into its longhand lists.
pub fn parse_transition(s: &str) -> Option<TransitionLists> {
    let mut out = TransitionLists::default();
    for item in crate::paint::split_top_level(s, ',') {
        let (mut property, mut times, mut timing) = (None, Vec::new(), None);
        for part in split_space(item) {
            if let Some(t) = parse_time(part) {
                times.push(t);
            } else if let Some(t) = Timing::parse(part).filter(|_| timing.is_none()) {
                timing = Some(t);
            } else if part == "normal" || part == "allow-discrete" {
                // `transition-behavior`: discrete properties never transition here.
            } else if property.is_none() {
                property = Some(parse_property(part)?);
            } else {
                return None;
            }
        }
        if times.len() > 2 {
            return None;
        }
        out.property.push(property.unwrap_or_else(|| "all".into()));
        out.duration.push(times.first().copied().unwrap_or(0.0));
        out.delay.push(times.get(1).copied().unwrap_or(0.0));
        out.timing.push(timing.unwrap_or(Timing::EASE));
    }
    Some(out)
}

/// Splits on top-level spaces (keeping `cubic-bezier(0, 0, 1, 1)` whole).
pub fn split_space(s: &str) -> Vec<&str> {
    crate::paint::split_top_level(s.trim(), ' ')
        .into_iter()
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .collect()
}

// --- Running state -----------------------------------------------------------------

/// What runs on one element: its transitions, and the values they start from.
#[derive(Clone, Debug, Default)]
pub(crate) struct Motion {
    /// Each property's winning computed value at the last restyle (before any
    /// transition is applied); `None` until the element was styled once.
    pub values: Option<std::collections::HashMap<String, String>>,
    pub transitions: Vec<Transition>,
}

/// A running transition of one property.
#[derive(Clone, Debug)]
pub(crate) struct Transition {
    pub prop: String,
    pub from: String,
    pub to: String,
    /// When it starts moving (after its delay), in engine milliseconds.
    pub start_ms: f64,
    pub duration_ms: f64,
    pub timing: Timing,
    /// `transitionstart` was sent.
    pub started: bool,
}

impl Transition {
    /// The value at `now`, and whether the transition has finished.
    pub fn value_at(&self, now: f64) -> (String, bool) {
        let t = ((now - self.start_ms) / self.duration_ms).max(0.0);
        if t >= 1.0 {
            return (self.to.clone(), true);
        }
        let eased = f64::from(self.timing.eval(t as f32));
        let value = interpolate(&self.from, &self.to, eased).unwrap_or_else(|| self.to.clone());
        (value, false)
    }
}

/// A transition or animation event for scripts.
#[derive(Clone, Debug, PartialEq)]
pub struct MotionEvent {
    pub kind: crate::events::EventKind,
    pub target: iron_core::NodeId,
    /// The property (`transition*`, kebab-case like the DOM's `propertyName`) or the
    /// animation name.
    pub name: String,
    /// Seconds the transition or animation has run (`elapsedTime`).
    pub elapsed: f64,
}

/// camelCase → kebab-case (`backgroundColor` → `background-color`).
pub fn kebab_case(prop: &str) -> String {
    let mut out = String::with_capacity(prop.len() + 4);
    for c in prop.chars() {
        if c.is_ascii_uppercase() {
            out.push('-');
            out.push(c.to_ascii_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

// --- Interpolation ------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
enum Token {
    Number(f64, String),
    Color(Rgba),
    Text(String),
}

const COLOR_FUNCTIONS: &[&str] = &[
    "rgb",
    "rgba",
    "hsl",
    "hsla",
    "hwb",
    "lab",
    "lch",
    "oklab",
    "oklch",
    "color",
    "color-mix",
];

/// Splits a value into numbers (with units), colours and other text.
fn tokenize(s: &str) -> Vec<Token> {
    let b = s.as_bytes();
    let mut out = Vec::new();
    let mut text = String::new();
    let mut i = 0;
    let flush = |text: &mut String, out: &mut Vec<Token>| {
        if !text.is_empty() {
            out.push(Token::Text(std::mem::take(text)));
        }
    };
    while i < b.len() {
        let c = b[i] as char;
        let prev_word = i > 0 && {
            let p = b[i - 1] as char;
            p.is_ascii_alphanumeric()
                || p == '_'
                || p == '-' && i > 1 && (b[i - 2] as char).is_ascii_alphabetic()
        };
        let starts_number = !prev_word
            && (c.is_ascii_digit()
                || (c == '.' && b.get(i + 1).is_some_and(u8::is_ascii_digit))
                || ((c == '-' || c == '+')
                    && b.get(i + 1).is_some_and(|n| {
                        n.is_ascii_digit()
                            || (*n == b'.' && b.get(i + 2).is_some_and(u8::is_ascii_digit))
                    })));
        if starts_number {
            flush(&mut text, &mut out);
            let start = i;
            i += 1;
            while i < b.len() && (b[i].is_ascii_digit() || b[i] == b'.') {
                i += 1;
            }
            // Exponent (`1e-3`).
            if i + 1 < b.len()
                && (b[i] == b'e' || b[i] == b'E')
                && (b[i + 1].is_ascii_digit()
                    || ((b[i + 1] == b'-' || b[i + 1] == b'+')
                        && b.get(i + 2).is_some_and(u8::is_ascii_digit)))
            {
                i += 2;
                while i < b.len() && b[i].is_ascii_digit() {
                    i += 1;
                }
            }
            let n: f64 = s[start..i].parse().unwrap_or(0.0);
            let unit_start = i;
            while i < b.len() && (b[i].is_ascii_alphabetic() || b[i] == b'%') {
                i += 1;
            }
            out.push(Token::Number(n, s[unit_start..i].to_owned()));
            continue;
        }
        if c == '#' && !prev_word {
            let start = i;
            i += 1;
            while i < b.len() && b[i].is_ascii_hexdigit() {
                i += 1;
            }
            match Rgba::parse(&s[start..i]) {
                Some(color) => {
                    flush(&mut text, &mut out);
                    out.push(Token::Color(color));
                }
                None => text.push_str(&s[start..i]),
            }
            continue;
        }
        if c.is_ascii_alphabetic() && !prev_word {
            let start = i;
            while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'-' || b[i] == b'_') {
                i += 1;
            }
            let word = &s[start..i];
            if b.get(i) == Some(&b'(')
                && COLOR_FUNCTIONS.contains(&word.to_ascii_lowercase().as_str())
            {
                // The whole function, to its matching parenthesis.
                let mut depth = 0;
                let mut j = i;
                while j < b.len() {
                    match b[j] {
                        b'(' => depth += 1,
                        b')' => {
                            depth -= 1;
                            if depth == 0 {
                                j += 1;
                                break;
                            }
                        }
                        _ => {}
                    }
                    j += 1;
                }
                if let Some(color) = Rgba::parse(&s[start..j]) {
                    flush(&mut text, &mut out);
                    out.push(Token::Color(color));
                    i = j;
                    continue;
                }
                text.push_str(word);
                continue;
            }
            match Rgba::parse(word).filter(|_| b.get(i) != Some(&b'(')) {
                Some(color) if !word.eq_ignore_ascii_case("currentcolor") => {
                    flush(&mut text, &mut out);
                    out.push(Token::Color(color));
                }
                _ => text.push_str(word),
            }
            continue;
        }
        text.push(c);
        i += 1;
    }
    flush(&mut text, &mut out);
    // Collapse runs of white space so `a  b` matches `a b`.
    out.into_iter()
        .map(|t| match t {
            Token::Text(s) => {
                let mut collapsed = String::with_capacity(s.len());
                for ch in s.chars() {
                    if !ch.is_whitespace() {
                        collapsed.push(ch);
                    } else if !collapsed.ends_with(' ') {
                        collapsed.push(' ');
                    }
                }
                Token::Text(collapsed)
            }
            t => t,
        })
        .collect()
}

fn fmt_number(n: f64) -> String {
    let r = (n * 10_000.0).round() / 10_000.0;
    if r == 0.0 { "0".into() } else { format!("{r}") }
}

fn mix_colors(a: Rgba, b: Rgba, t: f64) -> Rgba {
    // Premultiplied, so a fade to `transparent` doesn't darken.
    let t = t as f32;
    let alpha = a.a + (b.a - a.a) * t;
    let pm = |ca: f32, cb: f32| ca * a.a + (cb * b.a - ca * a.a) * t;
    let un = |v: f32| {
        if alpha > 0.0 {
            (v / alpha).clamp(0.0, 1.0)
        } else {
            0.0
        }
    };
    Rgba {
        r: un(pm(a.r, b.r)),
        g: un(pm(a.g, b.g)),
        b: un(pm(a.b, b.b)),
        a: alpha.clamp(0.0, 1.0),
    }
}

fn fmt_color(c: Rgba) -> String {
    format!(
        "rgba({}, {}, {}, {})",
        fmt_number(f64::from(c.r) * 255.0),
        fmt_number(f64::from(c.g) * 255.0),
        fmt_number(f64::from(c.b) * 255.0),
        fmt_number(f64::from(c.a))
    )
}

/// Interpolates CSS values `from` → `to` at `t` (0..=1, may overshoot for bouncy
/// easing). `None` if they can't be blended.
pub fn interpolate(from: &str, to: &str, t: f64) -> Option<String> {
    if from.trim() == to.trim() {
        return Some(to.trim().to_owned());
    }
    let (from, to) = match (from.trim(), to.trim()) {
        ("none", other) => (neutral(other)?, other.to_owned()),
        (other, "none") => (other.to_owned(), neutral(other)?),
        (a, b) => (a.to_owned(), b.to_owned()),
    };
    let (a, b) = (tokenize(&from), tokenize(&to));
    if a.len() != b.len() {
        return None;
    }
    let mut out = String::new();
    for (x, y) in a.iter().zip(&b) {
        match (x, y) {
            (Token::Number(m, u), Token::Number(n, v)) => {
                let blend = m + (n - m) * t;
                if u == v || (u.is_empty() && *m == 0.0) || (v.is_empty() && *n == 0.0) {
                    let unit = if u.is_empty() { v } else { u };
                    out.push_str(&fmt_number(blend));
                    out.push_str(unit);
                } else if u.is_empty() || v.is_empty() {
                    return None;
                } else {
                    // Mixed units (`10px` → `50%`): let `calc()` resolve them.
                    out.push_str(&format!(
                        "calc({}{u} * {} + {}{v} * {})",
                        fmt_number(*m),
                        fmt_number(1.0 - t),
                        fmt_number(*n),
                        fmt_number(t)
                    ));
                }
            }
            (Token::Color(c), Token::Color(d)) => out.push_str(&fmt_color(mix_colors(*c, *d, t))),
            (Token::Text(p), Token::Text(q)) if p == q => out.push_str(p),
            _ => return None,
        }
    }
    Some(out)
}

/// `none` as the neutral form of `other`: zero translations, rotations, skews and
/// offsets; unit scales and filter amounts; identity matrices; transparent colours.
fn neutral(other: &str) -> Option<String> {
    let tokens = tokenize(other);
    if !tokens.iter().any(|t| matches!(t, Token::Number(..))) {
        return None;
    }
    let mut out = String::new();
    let mut function = String::new();
    let mut arg = 0;
    for t in &tokens {
        match t {
            Token::Text(s) => {
                // Track the innermost function name and argument index.
                if let Some(open) = s.rfind('(') {
                    function = s[..open]
                        .rsplit(|c: char| !(c.is_ascii_alphanumeric() || c == '-'))
                        .next()
                        .unwrap_or("")
                        .to_owned();
                    arg = 0;
                }
                if s.contains(',') {
                    arg += s.matches(',').count();
                }
                // `) scale(` closes one function and opens the next.
                if s.rfind(')') > s.rfind('(') {
                    function.clear();
                }
                out.push_str(s);
            }
            Token::Number(_, unit) => {
                let one = matches!(
                    function.as_str(),
                    "scale"
                        | "scaleX"
                        | "scaleY"
                        | "scaleZ"
                        | "scale3d"
                        | "brightness"
                        | "contrast"
                        | "saturate"
                        | "opacity"
                ) || (function == "matrix" && matches!(arg, 0 | 3))
                    || (function == "matrix3d" && matches!(arg, 0 | 5 | 10 | 15));
                out.push_str(if one { "1" } else { "0" });
                out.push_str(unit);
            }
            Token::Color(c) => out.push_str(&fmt_color(Rgba { a: 0.0, ..*c })),
        }
    }
    Some(out)
}

/// The value a property has when no declaration sets it, for transitions to and from
/// it. `None` where that isn't a fixed value (`auto`, inherited, `currentColor`).
pub fn initial_value(prop: &str) -> Option<&'static str> {
    Some(match prop {
        "opacity" => "1",
        "transform" | "translate" | "rotate" | "scale" | "filter" | "backdropFilter"
        | "boxShadow" | "textShadow" => "none",
        "backgroundColor" | "outlineColor" => "transparent",
        p if p.starts_with("margin") || p.starts_with("padding") => "0px",
        p if p.starts_with("border") && (p.ends_with("Width") || p.ends_with("Radius")) => "0px",
        "borderRadius" | "borderWidth" | "outlineWidth" | "outlineOffset" | "letterSpacing"
        | "wordSpacing" | "gap" | "rowGap" | "columnGap" | "flexGrow" => "0",
        "flexShrink" => "1",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // M3-ANI-01
    #[test]
    fn timing_functions_follow_css() {
        assert_eq!(Timing::Linear.eval(0.3), 0.3);
        let ease = Timing::parse("ease").unwrap();
        assert!(
            (ease.eval(0.5) - 0.8024).abs() < 0.002,
            "{}",
            ease.eval(0.5)
        );
        assert_eq!(ease.eval(0.0), 0.0);
        assert!((ease.eval(1.0) - 1.0).abs() < 1e-4);
        let ease_in = Timing::parse("cubic-bezier(0.42, 0, 1, 1)").unwrap();
        assert!(ease_in.eval(0.25) < 0.1);
        assert!(
            Timing::parse("cubic-bezier(2, 0, 1, 1)").is_none(),
            "x outside 0..1"
        );
        let steps = Timing::parse("steps(4)").unwrap();
        assert_eq!(
            (steps.eval(0.1), steps.eval(0.3), steps.eval(1.0)),
            (0.0, 0.25, 1.0)
        );
        let start = Timing::parse("steps(4, jump-start)").unwrap();
        assert_eq!(start.eval(0.1), 0.25);
        let none = Timing::parse("steps(3, jump-none)").unwrap();
        assert_eq!(
            (none.eval(0.0), none.eval(0.5), none.eval(1.0)),
            (0.0, 0.5, 1.0)
        );
        assert_eq!(
            Timing::parse("step-end"),
            Some(Timing::Steps(1, StepPosition::End))
        );
        assert!(Timing::parse("bouncy").is_none());
    }

    // M3-ANI-01
    #[test]
    fn values_interpolate_token_by_token() {
        assert_eq!(interpolate("0px", "100px", 0.25).unwrap(), "25px");
        assert_eq!(interpolate("1", "0", 0.5).unwrap(), "0.5");
        assert_eq!(
            interpolate("10px 20px", "20px 40px", 0.5).unwrap(),
            "15px 30px"
        );
        assert_eq!(
            interpolate(
                "translate(0px, 10px) rotate(0deg)",
                "translate(100px, 0px) rotate(90deg)",
                0.5
            )
            .unwrap(),
            "translate(50px, 5px) rotate(45deg)"
        );
        assert_eq!(
            interpolate("red", "#0000ff", 0.5).unwrap(),
            "rgba(127.5, 0, 127.5, 1)"
        );
        // Fading to transparent keeps the colour (premultiplied).
        assert_eq!(
            interpolate("rgb(255, 0, 0)", "transparent", 0.5).unwrap(),
            "rgba(255, 0, 0, 0.5)"
        );
        assert_eq!(
            interpolate("10px", "50%", 0.5).unwrap(),
            "calc(10px * 0.5 + 50% * 0.5)"
        );
        assert_eq!(interpolate("0", "10px", 0.5).unwrap(), "5px");
        assert!(interpolate("auto", "10px", 0.5).is_none());
        assert!(interpolate("solid", "dashed", 0.5).is_none());
        assert!(interpolate("translate(1px)", "scale(2)", 0.5).is_none());
        assert!(interpolate("1px 2px", "1px", 0.5).is_none());
        // Identifiers with digits aren't numbers.
        assert_eq!(
            interpolate(
                "translate3d(0px, 0px, 0px)",
                "translate3d(10px, 0px, 0px)",
                0.5
            )
            .unwrap(),
            "translate3d(5px, 0px, 0px)"
        );
    }

    // M3-ANI-01
    #[test]
    fn none_blends_as_the_neutral_value() {
        assert_eq!(
            interpolate("none", "translateX(100px) scale(2)", 0.5).unwrap(),
            "translateX(50px) scale(1.5)"
        );
        assert_eq!(
            interpolate("0px 4px 8px red", "none", 0.5).unwrap(),
            "0px 2px 4px rgba(255, 0, 0, 0.5)"
        );
        assert_eq!(interpolate("none", "blur(10px)", 0.5).unwrap(), "blur(5px)");
        assert_eq!(
            interpolate("none", "brightness(3)", 0.5).unwrap(),
            "brightness(2)"
        );
        assert!(interpolate("none", "block", 0.5).is_none());
    }

    // M3-ANI-01
    #[test]
    fn transition_shorthand_and_matching() {
        let t = parse_transition("opacity 0.3s ease-in, transform 200ms 1s, color 1s").unwrap();
        let specs = t.specs();
        assert_eq!(specs.len(), 3);
        assert_eq!(specs[0].property, "opacity");
        assert_eq!(specs[0].duration_ms, 300.0);
        assert_eq!(specs[0].timing, Timing::parse("ease-in").unwrap());
        assert_eq!((specs[1].duration_ms, specs[1].delay_ms), (200.0, 1000.0));
        let t = parse_transition("background-color .5s").unwrap();
        assert_eq!(t.specs()[0].property, "backgroundColor");
        assert!(t.specs()[0].matches("background"));
        assert!(t.specs()[0].matches("backgroundColor"));
        assert!(!t.specs()[0].matches("color"));
        let all = parse_transition("1s linear").unwrap();
        assert!(all.specs()[0].matches("width"));
        assert!(parse_transition("opacity 1s 2s 3s").is_none());
        assert!(
            parse_transition("opacity 0s").unwrap().is_empty(),
            "no duration"
        );
        assert_eq!(parse_time("250ms"), Some(250.0));
        assert_eq!(parse_time("0"), Some(0.0));
        assert_eq!(parse_time("1"), None);
    }
}
