//! `calc()` / `min()` / `max()` / `clamp()` that involve percentages, evaluated by
//! Taffy during layout once the percentage basis is known (`calc(100% - 20px)`).
//!
//! Relative units are already pixels by now ([`crate::units`] runs first, and math
//! without percentages never gets here). Expressions are parsed once into a small
//! tree and interned in a global table; a style carries the table index as Taffy's
//! opaque calc handle, and the layout tree's `resolve_calc_value` evaluates it.

use std::collections::HashMap;
use std::sync::{LazyLock, RwLock};

/// A parsed expression. Percentages are fractions (`50%` is `0.5`).
#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    Px(f32),
    Percent(f32),
    Number(f32),
    Add(Box<Expr>, Box<Expr>),
    Sub(Box<Expr>, Box<Expr>),
    Mul(Box<Expr>, Box<Expr>),
    Div(Box<Expr>, Box<Expr>),
    Min(Vec<Expr>),
    Max(Vec<Expr>),
    /// `clamp(min, preferred, max)`.
    Clamp(Box<Expr>, Box<Expr>, Box<Expr>),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Number,
    Length,
}

impl Expr {
    /// The value in pixels, with percentages taken of `basis`.
    pub fn eval(&self, basis: f32) -> f32 {
        match self {
            Expr::Px(v) | Expr::Number(v) => *v,
            Expr::Percent(f) => f * basis,
            Expr::Add(a, b) => a.eval(basis) + b.eval(basis),
            Expr::Sub(a, b) => a.eval(basis) - b.eval(basis),
            Expr::Mul(a, b) => a.eval(basis) * b.eval(basis),
            Expr::Div(a, b) => {
                let d = b.eval(basis);
                if d == 0.0 { 0.0 } else { a.eval(basis) / d }
            }
            Expr::Min(v) => v
                .iter()
                .map(|e| e.eval(basis))
                .fold(f32::INFINITY, f32::min),
            Expr::Max(v) => v
                .iter()
                .map(|e| e.eval(basis))
                .fold(f32::NEG_INFINITY, f32::max),
            Expr::Clamp(lo, x, hi) => x.eval(basis).min(hi.eval(basis)).max(lo.eval(basis)),
        }
    }

    /// Whether this is a valid length (lengths and percentages combine; numbers only
    /// scale them).
    fn kind(&self) -> Option<Kind> {
        Some(match self {
            Expr::Px(_) | Expr::Percent(_) => Kind::Length,
            Expr::Number(_) => Kind::Number,
            Expr::Add(a, b) | Expr::Sub(a, b) => {
                let k = a.kind()?;
                (k == b.kind()?).then_some(k)?
            }
            Expr::Mul(a, b) => match (a.kind()?, b.kind()?) {
                (Kind::Number, k) | (k, Kind::Number) => k,
                _ => return None,
            },
            Expr::Div(a, b) => match (a.kind()?, b.kind()?) {
                (k, Kind::Number) => k,
                _ => return None,
            },
            Expr::Min(v) | Expr::Max(v) => {
                let k = v.first()?.kind()?;
                v.iter().all(|e| e.kind() == Some(k)).then_some(k)?
            }
            Expr::Clamp(a, b, c) => {
                let k = a.kind()?;
                (b.kind()? == k && c.kind()? == k).then_some(k)?
            }
        })
    }
}

/// Parses a math function whose result is a length (`calc(100% - 40px)`).
pub fn parse_length(text: &str) -> Option<Expr> {
    let mut p = Parser {
        chars: text.chars().collect(),
        pos: 0,
    };
    let e = p.function()?;
    p.skip_ws();
    (p.pos == p.chars.len() && e.kind() == Some(Kind::Length)).then_some(e)
}

/// Whether `text` is a math function call (`calc(`, `min(`, `max(`, `clamp(`).
pub fn is_math(text: &str) -> bool {
    let t = text.trim_start().to_ascii_lowercase();
    ["calc(", "min(", "max(", "clamp("]
        .iter()
        .any(|f| t.starts_with(f))
}

struct Table {
    exprs: Vec<Expr>,
    by_text: HashMap<String, usize>,
}

static TABLE: LazyLock<RwLock<Table>> = LazyLock::new(|| {
    RwLock::new(Table {
        exprs: Vec::new(),
        by_text: HashMap::new(),
    })
});

/// Parses and interns a length expression, returning Taffy's opaque calc handle: the
/// table index, shifted past the 3 low bits Taffy uses as a tag. The same text
/// always gets the same handle, so equal styles stay equal.
pub fn handle(text: &str) -> Option<*const ()> {
    let key = text.trim().to_ascii_lowercase();
    if let Some(&i) = TABLE.read().ok()?.by_text.get(&key) {
        return Some(index_to_handle(i));
    }
    let expr = parse_length(&key)?;
    let mut table = TABLE.write().ok()?;
    // Another thread may have added it since the read.
    if let Some(&i) = table.by_text.get(&key) {
        return Some(index_to_handle(i));
    }
    let i = table.exprs.len();
    table.exprs.push(expr);
    table.by_text.insert(key, i);
    Some(index_to_handle(i))
}

fn index_to_handle(i: usize) -> *const () {
    std::ptr::without_provenance((i + 1) << 3)
}

/// Evaluates a handle from [`handle`] against a percentage basis (Taffy's
/// `resolve_calc_value`). Unknown handles evaluate to 0.
pub fn resolve(handle: *const (), basis: f32) -> f32 {
    let i = (handle.addr() >> 3).wrapping_sub(1);
    TABLE
        .read()
        .ok()
        .and_then(|t| t.exprs.get(i).map(|e| e.eval(basis)))
        .unwrap_or(0.0)
}

struct Parser {
    chars: Vec<char>,
    pos: usize,
}

impl Parser {
    fn skip_ws(&mut self) {
        while self.chars.get(self.pos).is_some_and(|c| c.is_whitespace()) {
            self.pos += 1;
        }
    }

    fn peek(&mut self) -> Option<char> {
        self.skip_ws();
        self.chars.get(self.pos).copied()
    }

    fn eat(&mut self, c: char) -> Option<()> {
        (self.peek()? == c).then(|| self.pos += 1)
    }

    fn sum(&mut self) -> Option<Expr> {
        let mut acc = self.product()?;
        while let Some(op @ ('+' | '-')) = self.peek() {
            self.pos += 1;
            let rhs = self.product()?;
            acc = if op == '+' {
                Expr::Add(Box::new(acc), Box::new(rhs))
            } else {
                Expr::Sub(Box::new(acc), Box::new(rhs))
            };
        }
        Some(acc)
    }

    fn product(&mut self) -> Option<Expr> {
        let mut acc = self.unary()?;
        while let Some(op @ ('*' | '/')) = self.peek() {
            self.pos += 1;
            let rhs = self.unary()?;
            acc = if op == '*' {
                Expr::Mul(Box::new(acc), Box::new(rhs))
            } else {
                Expr::Div(Box::new(acc), Box::new(rhs))
            };
        }
        Some(acc)
    }

    fn unary(&mut self) -> Option<Expr> {
        match self.peek()? {
            '(' => {
                self.pos += 1;
                let e = self.sum()?;
                self.eat(')')?;
                Some(e)
            }
            '-' if !self
                .chars
                .get(self.pos + 1)
                .is_some_and(|c| c.is_ascii_digit() || *c == '.') =>
            {
                self.pos += 1;
                Some(Expr::Mul(
                    Box::new(Expr::Number(-1.0)),
                    Box::new(self.unary()?),
                ))
            }
            c if c.is_ascii_alphabetic() => self.function(),
            _ => self.number(),
        }
    }

    fn function(&mut self) -> Option<Expr> {
        self.skip_ws();
        let start = self.pos;
        while self
            .chars
            .get(self.pos)
            .is_some_and(|c| c.is_ascii_alphabetic())
        {
            self.pos += 1;
        }
        let name: String = self.chars[start..self.pos].iter().collect();
        if self.chars.get(self.pos) != Some(&'(') {
            return None;
        }
        self.pos += 1;
        let mut args = vec![self.sum()?];
        while self.peek() == Some(',') {
            self.pos += 1;
            args.push(self.sum()?);
        }
        self.eat(')')?;
        match (name.as_str(), args.len()) {
            ("calc", 1) => args.pop(),
            ("min", _) => Some(Expr::Min(args)),
            ("max", _) => Some(Expr::Max(args)),
            ("clamp", 3) => {
                let hi = args.pop()?;
                let x = args.pop()?;
                let lo = args.pop()?;
                Some(Expr::Clamp(Box::new(lo), Box::new(x), Box::new(hi)))
            }
            _ => None,
        }
    }

    fn number(&mut self) -> Option<Expr> {
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
        let n: f32 = self.chars[start..self.pos]
            .iter()
            .collect::<String>()
            .parse()
            .ok()?;
        let unit_start = self.pos;
        while self
            .chars
            .get(self.pos)
            .is_some_and(|c| c.is_ascii_alphabetic() || *c == '%')
        {
            self.pos += 1;
        }
        match self.chars[unit_start..self.pos]
            .iter()
            .collect::<String>()
            .as_str()
        {
            "" => Some(Expr::Number(n)),
            "px" => Some(Expr::Px(n)),
            "%" => Some(Expr::Percent(n / 100.0)),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eval(s: &str, basis: f32) -> Option<f32> {
        parse_length(s).map(|e| e.eval(basis))
    }

    #[test]
    fn expressions_evaluate_against_the_basis() {
        assert_eq!(eval("calc(100% - 20px)", 300.0), Some(280.0));
        assert_eq!(eval("calc(50% + 10px * 2)", 100.0), Some(70.0));
        assert_eq!(eval("calc((100% - 30px) / 3)", 330.0), Some(100.0));
        assert_eq!(eval("min(100%, 600px)", 800.0), Some(600.0));
        assert_eq!(eval("max(50%, 120px)", 200.0), Some(120.0));
        assert_eq!(eval("clamp(100px, 50%, 400px)", 1000.0), Some(400.0));
        assert_eq!(eval("calc(-1 * 10%)", 200.0), Some(-20.0));
        assert_eq!(eval("calc(min(10%, 5px) + max(1px, 2%))", 100.0), Some(7.0));
    }

    #[test]
    fn type_errors_and_numbers_are_rejected() {
        for bad in [
            "calc(10% + 2)",
            "calc(10% * 10px)",
            "calc(2 / 10%)",
            "calc(1 + 2)",
            "min(10%, 2)",
            "clamp(1%, 2%)",
            "calc(1em)",
        ] {
            assert_eq!(parse_length(bad), None, "{bad}");
        }
    }

    #[test]
    fn handles_are_interned_and_resolve() {
        let a = handle("calc(100% - 20px)").unwrap();
        let b = handle("CALC(100% - 20px)").unwrap();
        assert_eq!(a, b, "same text, same handle");
        assert_eq!(a.addr() & 0b111, 0, "Taffy's tag bits stay clear");
        assert_eq!(resolve(a, 200.0), 180.0);
        assert_ne!(handle("calc(100% - 21px)").unwrap(), a);
        assert_eq!(resolve(std::ptr::without_provenance(0xFFFF_0000), 1.0), 0.0);
    }
}
