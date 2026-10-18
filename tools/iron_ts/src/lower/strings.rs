//! Strings: literals, `+` and template literals, methods, `length`, and the conversions between
//! strings and numbers. Everything calls the runtime (`iron_ts_rt/src/string*.rs`), whose strings
//! behave exactly as JavaScript's (UTF-16 `length` and indices, code unit ordering). See
//! `docs/research/iron-ts-strings.md`.
//!
//! A string literal is a `Ty::Lit` tag when the context is a string-literal union and a `Ty::Str`
//! (a static string object) when the context is `string`. A tag used as a `string` is converted
//! (`ExprKind::TagStr`), so both kinds mix freely in `+`, comparisons and arguments.

use super::expr::{mk, num};
use super::*;

/// What a runtime string function takes.
#[derive(Clone, Copy)]
enum A {
    Str,
    Num,
}

/// What a runtime string function gives back.
#[derive(Clone, Copy)]
enum Ret {
    Num,
    Bool,
    Str,
    StrOrNull,
    StrArr,
}

/// An argument the call may leave out.
#[derive(Clone, Copy)]
enum Def {
    Num(f64),
    Str(&'static str),
}

struct StrFn {
    sym: &'static str,
    params: &'static [A],
    defaults: &'static [Def],
    ret: Ret,
}

const fn f(sym: &'static str, params: &'static [A], defaults: &'static [Def], ret: Ret) -> StrFn {
    StrFn {
        sym,
        params,
        defaults,
        ret,
    }
}

/// The methods of `string` that map onto one runtime function.
fn str_method(name: &str) -> Option<StrFn> {
    use A::{Num as N, Str as S};
    Some(match name {
        "charCodeAt" => f("ts_rt_str_char_code_at", &[N], &[Def::Num(0.0)], Ret::Num),
        "codePointAt" => f("ts_rt_str_code_point_at", &[N], &[Def::Num(0.0)], Ret::Num),
        "charAt" => f("ts_rt_str_char_at_gc", &[N], &[Def::Num(0.0)], Ret::Str),
        "at" => f("ts_rt_str_at_gc", &[N], &[Def::Num(0.0)], Ret::StrOrNull),
        "slice" => f(
            "ts_rt_str_slice_gc",
            &[N, N],
            &[Def::Num(0.0), Def::Num(f64::INFINITY)],
            Ret::Str,
        ),
        "substring" => f(
            "ts_rt_str_substring_gc",
            &[N, N],
            &[Def::Num(0.0), Def::Num(f64::INFINITY)],
            Ret::Str,
        ),
        "indexOf" => f(
            "ts_rt_str_index_of",
            &[S, N],
            &[Def::Num(0.0), Def::Num(0.0)],
            Ret::Num,
        ),
        "lastIndexOf" => f(
            "ts_rt_str_last_index_of",
            &[S, N],
            &[Def::Num(0.0), Def::Num(f64::NAN)],
            Ret::Num,
        ),
        "includes" => f(
            "ts_rt_str_includes",
            &[S, N],
            &[Def::Num(0.0), Def::Num(0.0)],
            Ret::Bool,
        ),
        "startsWith" => f(
            "ts_rt_str_starts_with",
            &[S, N],
            &[Def::Num(0.0), Def::Num(0.0)],
            Ret::Bool,
        ),
        "endsWith" => f(
            "ts_rt_str_ends_with",
            &[S, N],
            &[Def::Num(0.0), Def::Num(f64::NAN)],
            Ret::Bool,
        ),
        "trim" => f("ts_rt_str_trim_gc", &[], &[], Ret::Str),
        "trimStart" | "trimLeft" => f("ts_rt_str_trim_start_gc", &[], &[], Ret::Str),
        "trimEnd" | "trimRight" => f("ts_rt_str_trim_end_gc", &[], &[], Ret::Str),
        "repeat" => f("ts_rt_str_repeat_gc", &[N], &[Def::Num(0.0)], Ret::Str),
        "padStart" => f(
            "ts_rt_str_pad_start_gc",
            &[N, S],
            &[Def::Num(0.0), Def::Str(" ")],
            Ret::Str,
        ),
        "padEnd" => f(
            "ts_rt_str_pad_end_gc",
            &[N, S],
            &[Def::Num(0.0), Def::Str(" ")],
            Ret::Str,
        ),
        "toUpperCase" => f("ts_rt_str_to_upper_gc", &[], &[], Ret::Str),
        "toLowerCase" => f("ts_rt_str_to_lower_gc", &[], &[], Ret::Str),
        "split" => f("ts_rt_str_split_gc", &[S], &[], Ret::StrArr),
        _ => return None,
    })
}

impl<'a> Lowerer<'a> {
    /// A string literal as a `string`: a static object.
    pub(crate) fn str_lit(&mut self, text: &str) -> Expr {
        let i = match self.m.strs.iter().position(|s| s == text) {
            Some(i) => i,
            None => {
                self.m.strs.push(text.to_string());
                self.m.strs.len() - 1
            }
        };
        mk(ExprKind::Str(i as u32), Ty::Str)
    }

    fn rt_str(&self, sym: &'static str, args: Vec<Expr>, ty: Ty) -> Expr {
        mk(ExprKind::Rt(RtFn::Str(sym), args), ty)
    }

    /// A tag (string-literal union member) as a `string`.
    pub(crate) fn tag_to_str(&mut self, e: Expr) -> Expr {
        if let ExprKind::Tag(t) = e.kind {
            let text = self.m.tags[t as usize].clone();
            return self.str_lit(&text);
        }
        mk(ExprKind::TagStr(Box::new(e)), Ty::Str)
    }

    /// A value of a union of string literals widened to `string` (what `let` and array literals
    /// do with a literal in TypeScript).
    pub(crate) fn widen_lit(&mut self, e: Expr) -> Expr {
        if e.ty == Ty::Lit {
            self.tag_to_str(e)
        } else {
            e
        }
    }

    /// JavaScript's ToString for the values `+` and template literals join.
    pub(crate) fn stringify(&mut self, e: Expr) -> R<Expr> {
        match e.ty.clone() {
            Ty::Str => Ok(e),
            Ty::Lit => Ok(self.tag_to_str(e)),
            Ty::Num => Ok(self.rt_str("ts_rt_num_to_str_gc", vec![e], Ty::Str)),
            Ty::Bool => {
                let (t, f) = (self.str_lit("true"), self.str_lit("false"));
                Ok(mk(
                    ExprKind::Cond(Box::new(e), Box::new(t), Box::new(f)),
                    Ty::Str,
                ))
            }
            Ty::Null => Ok(self.str_lit("null")),
            Ty::Nullable(inner) => {
                let tmp = self.new_local(e.ty.clone());
                let v = self.var_expr(tmp);
                let is_null = mk(ExprKind::IsNull(Box::new(v.clone())), Ty::Bool);
                let plain = mk(ExprKind::Cast(Box::new(v)), *inner);
                let s = self.stringify(plain)?;
                let null = self.str_lit("null");
                let cond = mk(
                    ExprKind::Cond(Box::new(is_null), Box::new(null), Box::new(s)),
                    Ty::Str,
                );
                Ok(mk(
                    ExprKind::Seq(vec![Stmt::Assign(tmp, e)], Box::new(cond)),
                    Ty::Str,
                ))
            }
            t => bail!("cannot convert {t} to a string (objects have no toString yet)"),
        }
    }

    /// `a + b` as strings.
    pub(crate) fn str_concat(&mut self, a: Expr, b: Expr) -> R<Expr> {
        // Two constants join at compile time.
        if let (ExprKind::Tag(x), ExprKind::Tag(y)) = (&a.kind, &b.kind) {
            let text = format!("{}{}", self.m.tags[*x as usize], self.m.tags[*y as usize]);
            return Ok(self.str_lit(&text));
        }
        let (a, b) = (self.stringify(a)?, self.stringify(b)?);
        Ok(self.rt_str("ts_rt_str_concat_gc", vec![a, b], Ty::Str))
    }

    /// Is this a value `+` treats as a string operand?
    pub(crate) fn is_stringy(t: &Ty) -> bool {
        matches!(t.strip(), Ty::Str | Ty::Lit)
    }

    /// `` `a${x}b` ``.
    pub(crate) fn lower_template(&mut self, t: &'a TemplateLiteral<'a>) -> R<Expr> {
        let mut acc: Option<Expr> = None;
        let push = |this: &mut Self, part: Expr, acc: &mut Option<Expr>| -> R<()> {
            *acc = Some(match acc.take() {
                None => this.stringify(part)?,
                Some(a) => this.str_concat(a, part)?,
            });
            Ok(())
        };
        for (i, q) in t.quasis.iter().enumerate() {
            let text = q
                .value
                .cooked
                .as_ref()
                .map(|c| c.as_str())
                .unwrap_or_else(|| q.value.raw.as_str());
            if !text.is_empty() {
                let lit = self.str_lit(text);
                push(self, lit, &mut acc)?;
            }
            if let Some(x) = t.expressions.get(i) {
                let v = self.lower_expr(x, None)?;
                push(self, v, &mut acc)?;
            }
        }
        match acc {
            Some(a) => Ok(a),
            None => Ok(self.str_lit("")),
        }
    }

    fn str_arg(&mut self, e: &'a Expression<'a>) -> R<Expr> {
        let v = self.lower_expr(e, Some(&Ty::Str))?;
        let v = self.widen_lit(v);
        self.coerce(v, &Ty::Str)
    }

    fn num_arg(&mut self, e: &'a Expression<'a>) -> R<Expr> {
        let v = self.lower_expr(e, Some(&Ty::Num))?;
        self.as_num(v)
    }

    /// A method call on a `string` receiver.
    pub(crate) fn lower_str_method(
        &mut self,
        recv: Expr,
        m: &str,
        c: &'a CallExpression<'a>,
    ) -> R<Expr> {
        let at = self.loc(c.span.start);
        match m {
            "toString" | "valueOf" | "toLocaleString" => return Ok(recv),
            "concat" => {
                let mut acc = recv;
                for a in &c.arguments {
                    let Some(e) = a.as_expression() else {
                        bail!("{at}: concat: spread argument")
                    };
                    let v = self.lower_expr(e, None)?;
                    acc = self.str_concat(acc, v)?;
                }
                return Ok(acc);
            }
            _ => {}
        }
        let Some(sf) = str_method(m) else {
            bail!(
                "{at}: the string method `{m}` is not supported yet (localeCompare, normalize and the RegExp methods need their own work)"
            )
        };
        if c.arguments.len() > sf.params.len() {
            bail!(
                "{at}: {m}: {} arguments, at most {} expected",
                c.arguments.len(),
                sf.params.len()
            )
        }
        let mut args = vec![recv];
        for (i, kind) in sf.params.iter().enumerate() {
            match c.arguments.get(i) {
                Some(a) => {
                    let Some(e) = a.as_expression() else {
                        bail!("{at}: {m}: spread argument")
                    };
                    args.push(match kind {
                        A::Str => self.str_arg(e)?,
                        A::Num => self.num_arg(e)?,
                    });
                }
                None => match sf.defaults.get(i) {
                    Some(Def::Num(n)) => args.push(num(*n)),
                    Some(Def::Str(s)) => args.push(self.str_lit(s)),
                    None => bail!("{at}: {m}: missing argument"),
                },
            }
        }
        let ty = match sf.ret {
            Ret::Num => Ty::Num,
            Ret::Bool => Ty::Bool,
            Ret::Str => Ty::Str,
            Ret::StrOrNull => Ty::Str.nullable(),
            Ret::StrArr => Ty::Arr(Box::new(Ty::Str)),
        };
        Ok(self.rt_str(sf.sym, args, ty))
    }

    /// `x.toString()`, `x.toString(radix)` and `x.toFixed(digits)` on a number.
    pub(crate) fn lower_num_method(
        &mut self,
        recv: Expr,
        m: &str,
        c: &'a CallExpression<'a>,
    ) -> R<Expr> {
        let at = self.loc(c.span.start);
        let arg = |this: &mut Self, i: usize| -> R<Option<Expr>> {
            match c.arguments.get(i).and_then(|a| a.as_expression()) {
                Some(e) => Ok(Some(this.num_arg(e)?)),
                None => Ok(None),
            }
        };
        match m {
            "toString" => match arg(self, 0)? {
                None => Ok(self.rt_str("ts_rt_num_to_str_gc", vec![recv], Ty::Str)),
                Some(r) => Ok(self.rt_str("ts_rt_num_to_radix_gc", vec![recv, r], Ty::Str)),
            },
            "toFixed" => {
                let d = arg(self, 0)?.unwrap_or_else(|| num(0.0));
                Ok(self.rt_str("ts_rt_num_to_fixed_gc", vec![recv, d], Ty::Str))
            }
            "valueOf" => Ok(recv),
            _ => bail!("{at}: the number method `{m}` is not supported yet"),
        }
    }

    /// `parts.join(sep)` for an array of strings.
    pub(crate) fn lower_join(&mut self, recv: Expr, c: &'a CallExpression<'a>) -> R<Expr> {
        let sep = match c.arguments.first().and_then(|a| a.as_expression()) {
            Some(e) => self.str_arg(e)?,
            None => self.str_lit(","),
        };
        Ok(self.rt_str("ts_rt_str_join_gc", vec![recv, sep], Ty::Str))
    }

    /// `String(x)`, `Number(x)`, `parseInt(s, radix)`, `parseFloat(s)`.
    pub(crate) fn lower_string_global(
        &mut self,
        name: &str,
        c: &'a CallExpression<'a>,
    ) -> R<Option<Expr>> {
        let at = self.loc(c.span.start);
        let first = |this: &mut Self| -> R<Expr> {
            match c.arguments.first().and_then(|a| a.as_expression()) {
                Some(e) => this.lower_expr(e, None),
                None => bail!("{at}: {name}: missing argument"),
            }
        };
        Ok(Some(match name {
            "String" => {
                if c.arguments.is_empty() {
                    return Ok(Some(self.str_lit("")));
                }
                let v = first(self)?;
                self.stringify(v)?
            }
            "Number" => {
                if c.arguments.is_empty() {
                    return Ok(Some(num(0.0)));
                }
                let v = first(self)?;
                match v.ty.strip() {
                    Ty::Num => v,
                    Ty::Bool => self.bool_as_num(v),
                    Ty::Str | Ty::Lit => {
                        let s = self.widen_lit(v);
                        self.rt_str("ts_rt_str_to_number", vec![s], Ty::Num)
                    }
                    t => bail!("{at}: Number({t}) is not supported"),
                }
            }
            "parseFloat" => {
                let v = first(self)?;
                let s = self.widen_lit(v);
                let s = self.coerce(s, &Ty::Str)?;
                self.rt_str("ts_rt_str_parse_float", vec![s], Ty::Num)
            }
            "parseInt" => {
                let v = first(self)?;
                let s = self.widen_lit(v);
                let s = self.coerce(s, &Ty::Str)?;
                let radix = match c.arguments.get(1).and_then(|a| a.as_expression()) {
                    Some(e) => self.num_arg(e)?,
                    None => num(f64::NAN),
                };
                self.rt_str("ts_rt_str_parse_int", vec![s, radix], Ty::Num)
            }
            _ => return Ok(None),
        }))
    }

    /// A bool as 0 or 1 (kept as a conditional so `Number(true)` is exact).
    fn bool_as_num(&self, b: Expr) -> Expr {
        mk(
            ExprKind::Cond(Box::new(b), Box::new(num(1.0)), Box::new(num(0.0))),
            Ty::Num,
        )
    }

    /// `String.fromCharCode(a, b, ...)` and `String.fromCodePoint(a, b, ...)`.
    pub(crate) fn lower_string_static(
        &mut self,
        method: &str,
        c: &'a CallExpression<'a>,
    ) -> R<Expr> {
        let sym = match method {
            "fromCharCode" => "ts_rt_str_from_char_code_gc",
            "fromCodePoint" => "ts_rt_str_from_code_point_gc",
            m => bail!("{}: String.{m} is not supported", self.loc(c.span.start)),
        };
        let mut acc = self.str_lit("");
        for a in &c.arguments {
            let Some(e) = a.as_expression() else {
                bail!("String.{method}: spread argument")
            };
            let n = self.num_arg(e)?;
            let one = self.rt_str(sym, vec![n], Ty::Str);
            acc = self.str_concat(acc, one)?;
        }
        Ok(acc)
    }

    /// An ordering comparison of two strings: `compare(a, b) op 0`.
    pub(crate) fn str_order(&self, op: CmpOp, a: Expr, b: Expr) -> Expr {
        let cmp = self.rt_str("ts_rt_str_compare", vec![a, b], Ty::Num);
        mk(
            ExprKind::Cmp(op, Box::new(cmp), Box::new(num(0.0))),
            Ty::Bool,
        )
    }

    /// `s.length`.
    pub(crate) fn str_length(&self, s: Expr) -> Expr {
        self.rt_str("ts_rt_str_length", vec![s], Ty::Num)
    }
}
