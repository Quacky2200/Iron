// SPDX-License-Identifier: Apache-2.0

//! Temporal (ISO calendar): the classes are TypeScript in `temporal.ts`, compiled in front of
//! programs that mention `Temporal.`. The source rewrite `Temporal.X` -> `Temporal_X` makes them
//! ordinary classes; this file routes what needs types: `Temporal.X.from(v)` and a string passed
//! where a Temporal object is expected both go through the parse functions.

use super::*;

/// The prelude text.
pub const TEMPORAL: &str = include_str!("temporal.ts");

/// `Temporal.PlainDate` -> `Temporal_PlainDate` (and the other names) in the program text.
pub fn rewrite(src: &str) -> Option<String> {
    if !src.contains("Temporal.") {
        return None;
    }
    // A string that says `Temporal.PlainDate` keeps saying it.
    let literals = super::literals::literal_spans(src);
    let b = src.as_bytes();
    let mut out = String::with_capacity(src.len());
    let mut i = 0;
    let mut any = false;
    while i < src.len() {
        if src[i..].starts_with("Temporal.")
            && !super::literals::in_spans(&literals, i)
            && (i == 0
                || !(b[i - 1].is_ascii_alphanumeric() || b[i - 1] == b'_' || b[i - 1] == b'.'))
            && b.get(i + 9).is_some_and(|c| c.is_ascii_uppercase())
        {
            out.push_str("Temporal_");
            any = true;
            i += 9;
        } else {
            let ch = src[i..].chars().next().expect("in range");
            out.push(ch);
            i += ch.len_utf8();
        }
    }
    any.then_some(out)
}

/// The parse function for a Temporal class name.
fn parser(class: &str) -> Option<&'static str> {
    Some(match class {
        "Temporal_Duration" => "__tpParseDuration",
        "Temporal_PlainDate" => "__tpParseDate",
        "Temporal_PlainTime" => "__tpParseTime",
        "Temporal_PlainDateTime" => "__tpParseDateTime",
        "Temporal_Instant" => "__tpParseInstant",
        "Temporal_ZonedDateTime" => "__tpParseZoned",
        "Temporal_PlainYearMonth" => "__tpParseYearMonth",
        "Temporal_PlainMonthDay" => "__tpParseMonthDay",
        _ => return None,
    })
}

/// The copy-and-validate function for a Temporal class name.
fn copier(class: &str) -> Option<&'static str> {
    Some(match class {
        "Temporal_Duration" => "__tpDurationFrom",
        "Temporal_PlainDate" => "__tpDateFrom",
        "Temporal_PlainTime" => "__tpTimeFrom",
        "Temporal_PlainDateTime" => "__tpDateTimeFrom",
        "Temporal_Instant" => "__tpInstantFrom",
        "Temporal_ZonedDateTime" => "__tpZonedFrom",
        "Temporal_PlainYearMonth" => "__tpYearMonthFrom",
        "Temporal_PlainMonthDay" => "__tpMonthDayFrom",
        _ => return None,
    })
}

impl<'a> Lowerer<'a> {
    /// A string argument where a Temporal object is expected is parsed (`date.equals("2024-01-01")`).
    pub(crate) fn temporal_arg(&mut self, v: Expr, want: &Ty) -> R<Expr> {
        let Ty::Obj(sid) = want.strip() else {
            return Ok(v);
        };
        if !matches!(v.ty.strip(), Ty::Str | Ty::Lit) {
            return Ok(v);
        }
        let name = self.m.structs[*sid].name.clone();
        // `round("minute")` is `round({ smallestUnit: "minute" })`.
        if name == "Temporal_RoundOptions" {
            let v = self.widen_lit(v);
            return self.call_helper("__tpRoundOpts", vec![v]);
        }
        let Some(f) = parser(&name) else {
            return Ok(v);
        };
        let v = self.widen_lit(v);
        self.call_helper(f, vec![v])
    }

    /// `Temporal.X.from(v)` for a string, an object literal or another Temporal object.
    pub(crate) fn lower_temporal_from(
        &mut self,
        class: &str,
        c: &'a CallExpression<'a>,
    ) -> R<Expr> {
        let at = self.loc(c.span.start);
        let Some(arg) = c.arguments.first().and_then(|a| a.as_expression()) else {
            bail!("{at}: {class}.from needs a value")
        };
        let ty = Ty::Obj(self.classes_sid(class));
        // A zoned date-time literal carries a time zone: it is built from its own shape.
        if class == "Temporal_ZonedDateTime" && matches!(arg, Expression::ObjectExpression(_)) {
            let like = self.named_type("Temporal_ZonedLike")?;
            let v = self.lower_expr(arg, Some(&like))?;
            return self.call_helper("__tpZonedFromLike", vec![v]);
        }
        let v = self.lower_expr(arg, Some(&ty))?;
        if matches!(v.ty.strip(), Ty::Str | Ty::Lit) {
            let v = self.widen_lit(v);
            let f = parser(class).ok_or_else(|| format!("{at}: no parser for {class}"))?;
            return self.call_helper(f, vec![v]);
        }
        let f = copier(class).ok_or_else(|| format!("{at}: no copier for {class}"))?;
        let v = self.coerce(v, &ty)?;
        self.call_helper(f, vec![v])
    }
}
