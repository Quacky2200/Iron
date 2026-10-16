//! Relative lengths and math functions, resolved to pixels during restyle (like
//! `var()`): `em`, `rem`, `vw`, `vh`, `vmin`, `vmax` (plus the `d`/`s`/`l` viewport
//! variants) and `calc()`, `min()`, `max()`, `clamp()`. The result is plain CSS text
//! (`2em` becomes `32px`), so every property parser keeps working unchanged.
//!
//! Math that mixes in percentages (`calc(100% - 20px)`) can't be resolved before
//! layout; it is left as written (and is rejected by properties that can't take it).

/// What relative units resolve against.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UnitContext {
    /// The element's font size (its parent's, for `font-size` itself).
    pub em: f32,
    /// The root element's font size.
    pub rem: f32,
    /// The viewport (window) size in logical pixels.
    pub viewport: (f32, f32),
}

/// Which kinds of relative values a declaration used, so the node can be restyled
/// when they change.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Uses {
    pub em: bool,
    pub rem: bool,
    pub viewport: bool,
}

impl Uses {
    pub fn any(self) -> bool {
        self.em || self.rem || self.viewport
    }

    pub fn union(self, o: Uses) -> Uses {
        Uses {
            em: self.em || o.em,
            rem: self.rem || o.rem,
            viewport: self.viewport || o.viewport,
        }
    }
}

const RELATIVE_UNITS: &[&str] = &[
    "em", "rem", "vw", "vh", "vmin", "vmax", "dvw", "dvh", "svw", "svh", "lvw", "lvh",
];
const MATH: &[&str] = &["calc", "min", "max", "clamp"];

/// Whether `prop: text` needs resolving at restyle. `fontSize` handles a plain `em`
/// itself (relative to the parent, as it inherits), so that case isn't deferred.
pub fn needs_resolution_for(prop: &str, text: &str) -> bool {
    if prop == "fontSize"
        && text
            .trim()
            .strip_suffix("em")
            .is_some_and(|n| n.trim().parse::<f32>().is_ok())
    {
        return false;
    }
    needs_resolution(text)
}

/// Whether `text` may contain something [`resolve`] changes (a cheap pre-check).
pub fn needs_resolution(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    MATH.iter().any(|f| lower.contains(&format!("{f}(")))
        || RELATIVE_UNITS.iter().any(|u| has_unit(&lower, u))
}

fn has_unit(lower: &str, unit: &str) -> bool {
    lower.match_indices(unit).any(|(i, _)| {
        let before = lower[..i].chars().next_back();
        let after = lower[i + unit.len()..].chars().next();
        before.is_some_and(|c| c.is_ascii_digit() || c == '.')
            && !after.is_some_and(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    })
}

/// Rewrites relative units and resolvable math in `text` to pixels. Returns `None`
/// for invalid math (the declaration is then ignored, like invalid CSS).
pub fn resolve(text: &str, ctx: &UnitContext) -> Option<(String, Uses)> {
    let mut out = String::with_capacity(text.len());
    let mut uses = Uses::default();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        // Quoted strings pass through untouched.
        if c == '"' || c == '\'' {
            let start = i;
            i += 1;
            while i < chars.len() && chars[i] != c {
                if chars[i] == '\\' {
                    i += 1;
                }
                i += 1;
            }
            i = (i + 1).min(chars.len());
            out.extend(&chars[start..i]);
            continue;
        }
        // Identifiers (and `#hex`) are copied whole, so digits inside them aren't
        // read as numbers; `url(...)` is copied whole too.
        if c.is_ascii_alphabetic() || c == '_' || c == '#' || (c == '-' && next_is_ident(&chars, i))
        {
            let start = i;
            i += 1;
            while i < chars.len()
                && (chars[i].is_ascii_alphanumeric() || chars[i] == '_' || chars[i] == '-')
            {
                i += 1;
            }
            let ident: String = chars[start..i].iter().collect::<String>();
            let lower = ident.to_ascii_lowercase();
            if i < chars.len() && chars[i] == '(' {
                let close = matching_paren(&chars, i)?;
                let inner: String = chars[i + 1..close].iter().collect();
                if lower == "url" {
                    out.push_str(&ident);
                    out.push('(');
                    out.push_str(&inner);
                    out.push(')');
                } else if MATH.contains(&lower.as_str()) {
                    let whole: String = chars[start..=close].iter().collect();
                    match eval_function(&whole, ctx, &mut uses) {
                        Ok(v) => out.push_str(&v.css()),
                        // Percentages: keep the function, with its units resolved.
                        Err(MathError::Percent) => {
                            let (inner, u) = resolve(&inner, ctx)?;
                            uses = uses.union(u);
                            out.push_str(&ident);
                            out.push('(');
                            out.push_str(&inner);
                            out.push(')');
                        }
                        Err(MathError::Invalid) => return None,
                    }
                } else {
                    // Other functions: resolve their arguments.
                    let (inner, u) = resolve(&inner, ctx)?;
                    uses = uses.union(u);
                    out.push_str(&ident);
                    out.push('(');
                    out.push_str(&inner);
                    out.push(')');
                }
                i = close + 1;
            } else {
                out.push_str(&ident);
            }
            continue;
        }
        if c.is_ascii_digit() || (c == '.' && chars.get(i + 1).is_some_and(char::is_ascii_digit)) {
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                i += 1;
            }
            // Exponent (`1e3`), but not a unit starting with e (`em`).
            if i + 1 < chars.len()
                && (chars[i] == 'e' || chars[i] == 'E')
                && (chars[i + 1].is_ascii_digit()
                    || ((chars[i + 1] == '-' || chars[i + 1] == '+')
                        && chars.get(i + 2).is_some_and(char::is_ascii_digit)))
            {
                i += 2;
                while i < chars.len() && chars[i].is_ascii_digit() {
                    i += 1;
                }
            }
            let number: String = chars[start..i].iter().collect();
            let unit_start = i;
            while i < chars.len() && chars[i].is_ascii_alphabetic() {
                i += 1;
            }
            let unit: String = chars[unit_start..i]
                .iter()
                .collect::<String>()
                .to_ascii_lowercase();
            match (number.parse::<f32>(), unit_px(&unit, ctx, &mut uses)) {
                (Ok(n), Some(scale)) => out.push_str(&px(n * scale)),
                _ => {
                    out.push_str(&number);
                    out.push_str(&chars[unit_start..i].iter().collect::<String>());
                }
            }
            continue;
        }
        out.push(c);
        i += 1;
    }
    Some((out, uses))
}

fn next_is_ident(chars: &[char], i: usize) -> bool {
    chars
        .get(i + 1)
        .is_some_and(|c| c.is_ascii_alphabetic() || *c == '_' || *c == '-')
}

fn matching_paren(chars: &[char], open: usize) -> Option<usize> {
    let mut depth = 0;
    for (j, c) in chars.iter().enumerate().skip(open) {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(j);
                }
            }
            _ => {}
        }
    }
    None
}

/// Pixels per `unit` for relative units; `None` for anything else (left as written).
fn unit_px(unit: &str, ctx: &UnitContext, uses: &mut Uses) -> Option<f32> {
    let (vw, vh) = (ctx.viewport.0 / 100.0, ctx.viewport.1 / 100.0);
    let scale = match unit {
        "em" => {
            uses.em = true;
            ctx.em
        }
        "rem" => {
            uses.rem = true;
            ctx.rem
        }
        "vw" | "dvw" | "svw" | "lvw" => vw,
        "vh" | "dvh" | "svh" | "lvh" => vh,
        "vmin" => vw.min(vh),
        "vmax" => vw.max(vh),
        _ => return None,
    };
    if unit.ends_with('w') || unit.ends_with('h') || unit.starts_with('v') {
        uses.viewport = true;
    }
    Some(scale)
}

fn px(v: f32) -> String {
    let v = (v * 1000.0).round() / 1000.0;
    format!("{v}px")
}

// ---- Math ------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq)]
enum MathValue {
    Number(f32),
    Px(f32),
}

impl MathValue {
    fn css(self) -> String {
        match self {
            MathValue::Number(n) => format!("{}", (n * 1000.0).round() / 1000.0),
            MathValue::Px(v) => px(v),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum MathError {
    /// A percentage: only layout can resolve it.
    Percent,
    Invalid,
}

fn eval_function(text: &str, ctx: &UnitContext, uses: &mut Uses) -> Result<MathValue, MathError> {
    let mut p = Parser {
        chars: text.chars().collect(),
        pos: 0,
        ctx,
        uses,
    };
    let v = p.term()?;
    p.skip_ws();
    if p.pos != p.chars.len() {
        return Err(MathError::Invalid);
    }
    Ok(v)
}

struct Parser<'a> {
    chars: Vec<char>,
    pos: usize,
    ctx: &'a UnitContext,
    uses: &'a mut Uses,
}

impl Parser<'_> {
    fn skip_ws(&mut self) {
        while self.chars.get(self.pos).is_some_and(|c| c.is_whitespace()) {
            self.pos += 1;
        }
    }

    fn peek(&mut self) -> Option<char> {
        self.skip_ws();
        self.chars.get(self.pos).copied()
    }

    fn sum(&mut self) -> Result<MathValue, MathError> {
        let mut acc = self.product()?;
        while let Some(op @ ('+' | '-')) = self.peek() {
            self.pos += 1;
            let rhs = self.product()?;
            acc = match (acc, rhs) {
                (MathValue::Px(a), MathValue::Px(b)) => {
                    MathValue::Px(if op == '+' { a + b } else { a - b })
                }
                (MathValue::Number(a), MathValue::Number(b)) => {
                    MathValue::Number(if op == '+' { a + b } else { a - b })
                }
                // `1px + 2` mixes types: invalid CSS.
                _ => return Err(MathError::Invalid),
            };
        }
        Ok(acc)
    }

    fn product(&mut self) -> Result<MathValue, MathError> {
        let mut acc = self.unary()?;
        while let Some(op @ ('*' | '/')) = self.peek() {
            self.pos += 1;
            let rhs = self.unary()?;
            acc = match (op, acc, rhs) {
                ('*', MathValue::Number(a), MathValue::Number(b)) => MathValue::Number(a * b),
                ('*', MathValue::Number(n), MathValue::Px(v))
                | ('*', MathValue::Px(v), MathValue::Number(n)) => MathValue::Px(v * n),
                ('/', MathValue::Px(v), MathValue::Number(n)) if n != 0.0 => MathValue::Px(v / n),
                ('/', MathValue::Number(a), MathValue::Number(b)) if b != 0.0 => {
                    MathValue::Number(a / b)
                }
                _ => return Err(MathError::Invalid),
            };
        }
        Ok(acc)
    }

    fn unary(&mut self) -> Result<MathValue, MathError> {
        match self.peek() {
            Some('(') => {
                self.pos += 1;
                let v = self.sum()?;
                if self.peek() != Some(')') {
                    return Err(MathError::Invalid);
                }
                self.pos += 1;
                Ok(v)
            }
            Some('-')
                if !self
                    .chars
                    .get(self.pos + 1)
                    .is_some_and(|c| c.is_ascii_digit() || *c == '.') =>
            {
                self.pos += 1;
                Ok(match self.unary()? {
                    MathValue::Number(n) => MathValue::Number(-n),
                    MathValue::Px(v) => MathValue::Px(-v),
                })
            }
            Some(c) if c.is_ascii_alphabetic() => self.term(),
            Some(_) => self.number(),
            None => Err(MathError::Invalid),
        }
    }

    /// A math function: `calc(...)`, `min(...)`, `max(...)`, `clamp(...)`.
    fn term(&mut self) -> Result<MathValue, MathError> {
        self.skip_ws();
        let start = self.pos;
        while self
            .chars
            .get(self.pos)
            .is_some_and(|c| c.is_ascii_alphabetic())
        {
            self.pos += 1;
        }
        let name: String = self.chars[start..self.pos]
            .iter()
            .collect::<String>()
            .to_ascii_lowercase();
        if self.chars.get(self.pos) != Some(&'(') {
            return Err(MathError::Invalid);
        }
        self.pos += 1;
        let mut args = vec![self.sum()?];
        while self.peek() == Some(',') {
            self.pos += 1;
            args.push(self.sum()?);
        }
        if self.peek() != Some(')') {
            return Err(MathError::Invalid);
        }
        self.pos += 1;
        let same_kind = args.windows(2).all(|w| {
            matches!(
                (w[0], w[1]),
                (MathValue::Px(_), MathValue::Px(_)) | (MathValue::Number(_), MathValue::Number(_))
            )
        });
        if !same_kind {
            return Err(MathError::Invalid);
        }
        let value = |v: MathValue| match v {
            MathValue::Number(n) | MathValue::Px(n) => n,
        };
        let wrap = |n: f32| match args[0] {
            MathValue::Number(_) => MathValue::Number(n),
            MathValue::Px(_) => MathValue::Px(n),
        };
        match (name.as_str(), args.len()) {
            ("calc", 1) => Ok(args[0]),
            ("min", _) => Ok(wrap(
                args.iter().map(|a| value(*a)).fold(f32::INFINITY, f32::min),
            )),
            ("max", _) => Ok(wrap(
                args.iter()
                    .map(|a| value(*a))
                    .fold(f32::NEG_INFINITY, f32::max),
            )),
            // clamp(min, preferred, max): max wins over min, like CSS.
            ("clamp", 3) => Ok(wrap(value(args[1]).min(value(args[2])).max(value(args[0])))),
            _ => Err(MathError::Invalid),
        }
    }

    fn number(&mut self) -> Result<MathValue, MathError> {
        self.skip_ws();
        let start = self.pos;
        if matches!(self.chars.get(self.pos), Some('-' | '+')) {
            self.pos += 1;
        }
        while self
            .chars
            .get(self.pos)
            .is_some_and(|c| c.is_ascii_digit() || *c == '.')
        {
            self.pos += 1;
        }
        let number: f32 = self.chars[start..self.pos]
            .iter()
            .collect::<String>()
            .parse()
            .map_err(|_| MathError::Invalid)?;
        let unit_start = self.pos;
        while self
            .chars
            .get(self.pos)
            .is_some_and(|c| c.is_ascii_alphabetic() || *c == '%')
        {
            self.pos += 1;
        }
        let unit: String = self.chars[unit_start..self.pos]
            .iter()
            .collect::<String>()
            .to_ascii_lowercase();
        match unit.as_str() {
            "" => Ok(MathValue::Number(number)),
            "px" => Ok(MathValue::Px(number)),
            "%" => Err(MathError::Percent),
            u => match unit_px(u, self.ctx, self.uses) {
                Some(scale) => Ok(MathValue::Px(number * scale)),
                None => Err(MathError::Invalid),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CTX: UnitContext = UnitContext {
        em: 20.0,
        rem: 16.0,
        viewport: (1000.0, 500.0),
    };

    fn r(s: &str) -> Option<String> {
        resolve(s, &CTX).map(|(s, _)| s)
    }

    #[test]
    fn relative_units_become_pixels() {
        assert_eq!(r("2em").unwrap(), "40px");
        assert_eq!(r("1.5rem").unwrap(), "24px");
        assert_eq!(r("10vw").unwrap(), "100px");
        assert_eq!(r("10vh").unwrap(), "50px");
        assert_eq!(r("10vmin 10vmax").unwrap(), "50px 100px");
        assert_eq!(r("100dvh").unwrap(), "500px");
        assert_eq!(
            r("0 0.5em 1em rgba(0,0,0,.5)").unwrap(),
            "0 10px 20px rgba(0,0,0,.5)"
        );
    }

    #[test]
    fn other_values_are_left_alone() {
        for s in [
            "12px",
            "50%",
            "auto",
            "#1e3a5f",
            "url(img/2em.png)",
            "\"5em\"",
            "repeat(2, 1fr)",
            "h1",
            "1e3px",
        ] {
            assert_eq!(r(s).unwrap(), s, "{s}");
        }
        assert!(!needs_resolution("translate3d(1px, 2px)"));
        assert!(!needs_resolution("#1e3a5f"));
        assert!(needs_resolution("2em"));
        assert!(needs_resolution("calc(1px)"));
    }

    #[test]
    fn math_functions_evaluate() {
        assert_eq!(r("calc(10px + 2em)").unwrap(), "50px");
        assert_eq!(r("calc((1rem + 4px) * 2)").unwrap(), "40px");
        assert_eq!(r("calc(100px / 4 - 5px)").unwrap(), "20px");
        assert_eq!(r("min(10vw, 300px)").unwrap(), "100px");
        assert_eq!(r("max(1em, 30px, 2rem)").unwrap(), "32px");
        assert_eq!(r("clamp(10px, 5vw, 40px)").unwrap(), "40px");
        assert_eq!(
            r("clamp(60px, 5vw, 40px)").unwrap(),
            "60px",
            "min wins over max"
        );
        assert_eq!(r("calc(min(1em, 10px) + max(1px, 2px))").unwrap(), "12px");
        assert_eq!(
            r("calc(1.5 * 2)").unwrap(),
            "3",
            "unitless math stays a number"
        );
        assert_eq!(r("0 calc(2px * -1)").unwrap(), "0 -2px");
    }

    #[test]
    fn percentages_in_math_are_kept_for_layout() {
        assert_eq!(r("calc(100% - 2em)").unwrap(), "calc(100% - 40px)");
    }

    #[test]
    fn invalid_math_is_rejected() {
        for s in [
            "calc(1px + 2)",
            "calc(2px * 3px)",
            "calc(1px / 0)",
            "clamp(1px, 2px)",
            "calc(1px +)",
            "calc(1foo)",
        ] {
            assert_eq!(r(s), None, "{s}");
        }
    }

    #[test]
    fn uses_are_reported() {
        let (_, u) = resolve("calc(1em + 1rem)", &CTX).unwrap();
        assert!(u.em && u.rem && !u.viewport);
        let (_, u) = resolve("10vw", &CTX).unwrap();
        assert!(u.viewport && !u.em);
        let (_, u) = resolve("10px", &CTX).unwrap();
        assert!(!u.any());
    }
}
