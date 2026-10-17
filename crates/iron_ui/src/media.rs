//! `@media` queries (M3d group 13): parsing query lists and matching them against the
//! window and device, re-evaluated when those change.

/// What media queries are matched against.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MediaEnv {
    /// The viewport in CSS (UI) pixels.
    pub width: f32,
    pub height: f32,
    /// Device pixels per CSS pixel (the window's DPI scale times the UI scale).
    pub resolution: f32,
    pub dark: bool,
    pub reduced_motion: bool,
    /// The primary pointing device: a mouse is `fine` and can hover; a gamepad or
    /// touch screen would be `coarse` / `none`.
    pub pointer: Pointer,
    pub hover: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pointer {
    None,
    Coarse,
    Fine,
}

impl Default for MediaEnv {
    fn default() -> Self {
        Self {
            width: 0.0,
            height: 0.0,
            resolution: 1.0,
            dark: false,
            reduced_motion: false,
            pointer: Pointer::Fine,
            hover: true,
        }
    }
}

/// A comma-separated query list: matches when any query does.
#[derive(Clone, Debug, PartialEq)]
pub struct MediaList(pub Vec<MediaQuery>);

#[derive(Clone, Debug, PartialEq)]
pub struct MediaQuery {
    pub not: bool,
    /// `screen` and `all` match; `print` (and unknown types) don't.
    pub media_type: Option<String>,
    pub condition: Option<Condition>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Condition {
    And(Vec<Condition>),
    Or(Vec<Condition>),
    Not(Box<Condition>),
    Feature(Feature),
    /// Something this parser doesn't know (an unknown feature): never matches.
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Cmp {
    Lt,
    Le,
    Eq,
    Ge,
    Gt,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Feature {
    /// A numeric feature compared with values (in px, dppx or a ratio), e.g.
    /// `(min-width: 600px)` is `Range("width", [(Ge, 600)])`.
    Range(String, Vec<(Cmp, f32)>),
    /// A keyword feature (`orientation: landscape`), or just a name for a boolean test
    /// (`(hover)`).
    Keyword(String, Option<String>),
}

impl MediaList {
    pub fn parse(text: &str) -> Option<Self> {
        let text = text.trim();
        if text.is_empty() {
            return Some(MediaList(vec![MediaQuery {
                not: false,
                media_type: None,
                condition: None,
            }]));
        }
        crate::paint::split_top_level(text, ',')
            .into_iter()
            .map(|q| MediaQuery::parse(q.trim()))
            .collect::<Option<Vec<_>>>()
            .map(MediaList)
    }

    pub fn matches(&self, env: &MediaEnv) -> bool {
        self.0.iter().any(|q| q.matches(env))
    }
}

impl MediaQuery {
    fn parse(text: &str) -> Option<Self> {
        let lower = text.to_ascii_lowercase();
        let mut rest = lower.trim();
        // `[not | only] type [and condition]`, or just a condition.
        if !rest.starts_with('(') && !rest.starts_with("not (") && !rest.starts_with("not(") {
            let mut not = false;
            if let Some(r) = rest.strip_prefix("not ") {
                not = true;
                rest = r.trim_start();
            } else if let Some(r) = rest.strip_prefix("only ") {
                rest = r.trim_start();
            }
            let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
            let media_type = rest[..end].to_owned();
            if media_type.is_empty()
                || !media_type
                    .chars()
                    .all(|c| c.is_ascii_alphabetic() || c == '-')
            {
                return None;
            }
            let after = rest[end..].trim_start();
            let condition = if after.is_empty() {
                None
            } else {
                let cond = after.strip_prefix("and")?;
                Some(parse_condition(cond.trim())?)
            };
            return Some(MediaQuery {
                not,
                media_type: Some(media_type),
                condition,
            });
        }
        Some(MediaQuery {
            not: false,
            media_type: None,
            condition: Some(parse_condition(rest)?),
        })
    }

    fn matches(&self, env: &MediaEnv) -> bool {
        let type_ok = matches!(self.media_type.as_deref(), None | Some("all" | "screen"));
        let result = type_ok && self.condition.as_ref().is_none_or(|c| c.matches(env));
        result != self.not
    }
}

/// `(a) and (b)`, `(a) or (b)`, `not (a)`, nested parentheses.
fn parse_condition(text: &str) -> Option<Condition> {
    let text = text.trim();
    if let Some(inner) = text.strip_prefix("not") {
        let inner = inner.trim_start();
        if inner.starts_with('(') {
            return Some(Condition::Not(Box::new(parse_in_parens(inner)?)));
        }
    }
    // Split on top-level ` and ` / ` or ` between parenthesized parts.
    let mut parts = Vec::new();
    let mut joiner: Option<&str> = None;
    let mut rest = text;
    loop {
        rest = rest.trim_start();
        let close = matching_paren(rest)?;
        parts.push(parse_in_parens(&rest[..=close])?);
        rest = rest[close + 1..].trim_start();
        if rest.is_empty() {
            break;
        }
        let word = if rest.starts_with("and") {
            "and"
        } else if rest.starts_with("or") {
            "or"
        } else {
            return None;
        };
        // Mixing `and` and `or` without parentheses is invalid CSS.
        if joiner.is_some_and(|j| j != word) {
            return None;
        }
        joiner = Some(word);
        rest = &rest[word.len()..];
    }
    Some(match (parts.len(), joiner) {
        (1, _) => parts.pop()?,
        (_, Some("or")) => Condition::Or(parts),
        _ => Condition::And(parts),
    })
}

/// The index of the `)` closing the `(` that `text` starts with.
fn matching_paren(text: &str) -> Option<usize> {
    if !text.starts_with('(') {
        return None;
    }
    let mut depth = 0;
    for (i, c) in text.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

/// `( feature )` or `( condition )`.
fn parse_in_parens(text: &str) -> Option<Condition> {
    let inner = text.trim().strip_prefix('(')?.strip_suffix(')')?.trim();
    if inner.starts_with('(') || inner.starts_with("not") {
        return parse_condition(inner);
    }
    Some(parse_feature(inner))
}

fn parse_feature(text: &str) -> Condition {
    let feature = |f| Condition::Feature(f);
    // `name: value`
    if let Some((name, value)) = text.split_once(':') {
        let (name, value) = (name.trim(), value.trim());
        let (cmp, base) = if let Some(b) = name.strip_prefix("min-") {
            (Cmp::Ge, b)
        } else if let Some(b) = name.strip_prefix("max-") {
            (Cmp::Le, b)
        } else {
            (Cmp::Eq, name)
        };
        if is_range_feature(base) {
            return match value_of(base, value) {
                Some(v) => feature(Feature::Range(base.to_owned(), vec![(cmp, v)])),
                None => Condition::Unknown,
            };
        }
        if cmp != Cmp::Eq || !is_keyword_feature(name) {
            return Condition::Unknown;
        }
        return feature(Feature::Keyword(name.to_owned(), Some(value.to_owned())));
    }
    // Range syntax: `width >= 600px`, `400px <= width < 800px`.
    let tokens = range_tokens(text);
    match tokens.as_slice() {
        [a, op, b] => {
            let (Some(op), a, b) = (cmp_of(op), a.as_str(), b.as_str()) else {
                return Condition::Unknown;
            };
            if is_range_feature(a) {
                value_of(a, b).map_or(Condition::Unknown, |v| {
                    feature(Feature::Range(a.to_owned(), vec![(op, v)]))
                })
            } else if is_range_feature(b) {
                value_of(b, a).map_or(Condition::Unknown, |v| {
                    feature(Feature::Range(b.to_owned(), vec![(flip(op), v)]))
                })
            } else {
                Condition::Unknown
            }
        }
        [a, op1, name, op2, b] if is_range_feature(name) => {
            let (Some(op1), Some(op2)) = (cmp_of(op1), cmp_of(op2)) else {
                return Condition::Unknown;
            };
            match (value_of(name, a), value_of(name, b)) {
                (Some(lo), Some(hi)) => feature(Feature::Range(
                    name.clone(),
                    vec![(flip(op1), lo), (op2, hi)],
                )),
                _ => Condition::Unknown,
            }
        }
        [name] if is_range_feature(name) || is_keyword_feature(name) => {
            feature(Feature::Keyword(name.clone(), None))
        }
        _ => Condition::Unknown,
    }
}

fn range_tokens(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if matches!(c, '<' | '>' | '=') {
            if !cur.trim().is_empty() {
                out.push(cur.trim().to_owned());
            }
            cur.clear();
            let mut op = c.to_string();
            if c != '=' && chars.peek() == Some(&'=') {
                op.push(chars.next().unwrap_or('='));
            }
            out.push(op);
        } else {
            cur.push(c);
        }
    }
    if !cur.trim().is_empty() {
        out.push(cur.trim().to_owned());
    }
    out
}

fn cmp_of(op: &str) -> Option<Cmp> {
    Some(match op {
        "<" => Cmp::Lt,
        "<=" => Cmp::Le,
        "=" => Cmp::Eq,
        ">=" => Cmp::Ge,
        ">" => Cmp::Gt,
        _ => return None,
    })
}

/// `600px < width` is `width > 600px`.
fn flip(c: Cmp) -> Cmp {
    match c {
        Cmp::Lt => Cmp::Gt,
        Cmp::Le => Cmp::Ge,
        Cmp::Gt => Cmp::Lt,
        Cmp::Ge => Cmp::Le,
        Cmp::Eq => Cmp::Eq,
    }
}

fn is_range_feature(name: &str) -> bool {
    matches!(name, "width" | "height" | "aspect-ratio" | "resolution")
}

fn is_keyword_feature(name: &str) -> bool {
    matches!(
        name,
        "orientation"
            | "prefers-color-scheme"
            | "prefers-reduced-motion"
            | "pointer"
            | "any-pointer"
            | "hover"
            | "any-hover"
    )
}

/// A value for a range feature: lengths in px (`em`/`rem` are 16px), resolutions in
/// dppx (`dpi` / 96, `x`), ratios as `w/h` or a number.
fn value_of(feature: &str, value: &str) -> Option<f32> {
    let v = value.trim();
    let num = |s: &str| s.trim().parse::<f32>().ok().filter(|n| n.is_finite());
    match feature {
        "width" | "height" => {
            if let Some(n) = v.strip_suffix("px") {
                num(n)
            } else if let Some(n) = v.strip_suffix("rem").or_else(|| v.strip_suffix("em")) {
                num(n).map(|n| n * 16.0)
            } else {
                num(v).filter(|n| *n == 0.0)
            }
        }
        "resolution" => {
            if let Some(n) = v.strip_suffix("dppx").or_else(|| v.strip_suffix('x')) {
                num(n)
            } else if let Some(n) = v.strip_suffix("dpi") {
                num(n).map(|n| n / 96.0)
            } else if let Some(n) = v.strip_suffix("dpcm") {
                num(n).map(|n| n * 2.54 / 96.0)
            } else {
                None
            }
        }
        "aspect-ratio" => match v.split_once('/') {
            Some((w, h)) => Some(num(w)? / num(h).filter(|h| *h > 0.0)?),
            None => num(v),
        },
        _ => None,
    }
}

impl Condition {
    pub fn matches(&self, env: &MediaEnv) -> bool {
        match self {
            Condition::And(c) => c.iter().all(|c| c.matches(env)),
            Condition::Or(c) => c.iter().any(|c| c.matches(env)),
            Condition::Not(c) => !c.matches(env),
            Condition::Unknown => false,
            Condition::Feature(Feature::Range(name, tests)) => {
                let actual = match name.as_str() {
                    "width" => env.width,
                    "height" => env.height,
                    "resolution" => env.resolution,
                    _ => env.width / env.height.max(1e-6),
                };
                tests.iter().all(|(cmp, v)| {
                    let eps = 1e-4;
                    match cmp {
                        Cmp::Lt => actual < *v - eps,
                        Cmp::Le => actual <= *v + eps,
                        Cmp::Eq => (actual - v).abs() <= eps,
                        Cmp::Ge => actual >= *v - eps,
                        Cmp::Gt => actual > *v + eps,
                    }
                })
            }
            Condition::Feature(Feature::Keyword(name, value)) => {
                let pointer = match env.pointer {
                    Pointer::None => "none",
                    Pointer::Coarse => "coarse",
                    Pointer::Fine => "fine",
                };
                let actual = match name.as_str() {
                    "orientation" if env.height >= env.width => "portrait",
                    "orientation" => "landscape",
                    "prefers-color-scheme" if env.dark => "dark",
                    "prefers-color-scheme" => "light",
                    "prefers-reduced-motion" if env.reduced_motion => "reduce",
                    "prefers-reduced-motion" => "no-preference",
                    "pointer" | "any-pointer" => pointer,
                    "hover" | "any-hover" if env.hover => "hover",
                    "hover" | "any-hover" => "none",
                    // Boolean range tests: `(width)` is true unless it's 0.
                    "width" => return env.width > 0.0,
                    "height" => return env.height > 0.0,
                    _ => return true,
                };
                match value {
                    Some(v) => v == actual,
                    // Boolean context: true unless the value is `none` / `no-preference`.
                    None => !matches!(actual, "none" | "no-preference"),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(width: f32, height: f32) -> MediaEnv {
        MediaEnv {
            width,
            height,
            ..MediaEnv::default()
        }
    }

    fn m(q: &str, e: &MediaEnv) -> bool {
        MediaList::parse(q)
            .unwrap_or_else(|| panic!("{q}"))
            .matches(e)
    }

    // M3-MQ-01
    #[test]
    fn sizes_ranges_and_logic() {
        let e = env(800.0, 600.0);
        assert!(m("(min-width: 600px)", &e));
        assert!(!m("(max-width: 600px)", &e));
        assert!(m("(width >= 800px)", &e));
        assert!(!m("(width > 800px)", &e));
        assert!(m("(400px <= width < 900px)", &e));
        assert!(m("(600px < width)", &e));
        assert!(m("(min-width: 40em)", &e), "em is 16px");
        assert!(m("screen and (orientation: landscape)", &e));
        assert!(!m("print", &e));
        assert!(m("not print", &e));
        assert!(m("only screen and (min-height: 500px)", &e));
        assert!(
            m("(max-width: 100px), (min-height: 500px)", &e),
            "a list is an or"
        );
        assert!(m("((min-width: 900px) or (height: 600px)) and (hover)", &e));
        assert!(!m(
            "not ((min-width: 600px) and (orientation: landscape))",
            &e
        ));
        assert!(m("(aspect-ratio: 4/3)", &e));
        assert!(m("(min-aspect-ratio: 1)", &e));
        assert!(m("all", &e) && m("", &e));
    }

    // M3-MQ-01
    #[test]
    fn device_features() {
        let mut e = env(800.0, 600.0);
        e.resolution = 2.0;
        assert!(m("(min-resolution: 2dppx)", &e));
        assert!(m("(resolution >= 192dpi)", &e));
        assert!(!m("(prefers-color-scheme: dark)", &e));
        e.dark = true;
        assert!(m("(prefers-color-scheme: dark)", &e));
        assert!(m("(pointer: fine) and (hover: hover)", &e));
        assert!(!m("(prefers-reduced-motion)", &e), "no-preference is false");
        e.pointer = Pointer::Coarse;
        e.hover = false;
        assert!(m("(any-pointer: coarse)", &e) && !m("(hover)", &e));
    }

    // M3-MQ-01
    #[test]
    fn unknown_and_bad_queries() {
        let e = env(800.0, 600.0);
        assert!(!m("(color-gamut: p3)", &e), "unknown features never match");
        assert!(m("not (color-gamut: p3)", &e));
        assert!(
            MediaList::parse("(a) and (b) or (c)").is_none(),
            "and/or need parentheses"
        );
        assert!(MediaList::parse("screen (min-width: 1px)").is_none());
        assert!(MediaList::parse("(min-width: 1px").is_none());
    }
}
