// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

//! `bigint`: literals (`10n`), the operators, comparison (also against numbers), `BigInt(x)`,
//! `BigInt.asIntN/asUintN`, `toString(radix)` and `Number(big)`. A BigInt is a leaf GC object (see
//! `platypus_ts_rt/src/bigint.rs`); mixing it with a number in arithmetic is a compile error, where
//! JavaScript throws a `TypeError` at run time.

use super::expr::{mk, num};
use super::*;

impl<'a> Lowerer<'a> {
    fn is_big(e: &Expr) -> bool {
        e.ty.strip() == &Ty::BigInt
    }

    /// `123n`, `0xFFn`, `1_000n`.
    pub(crate) fn lower_bigint_literal(&mut self, b: &'a BigIntLiteral<'a>) -> R<Expr> {
        let raw = b
            .raw
            .as_ref()
            .map_or_else(|| b.value.to_string(), |r| r.to_string());
        let text = raw.trim_end_matches('n').to_string();
        let s = self.str_lit(&text);
        Ok(self.rt_str("ts_rt_bigint_literal_gc", vec![s], Ty::BigInt))
    }

    fn bigint_one(&mut self) -> Expr {
        let s = self.str_lit("1");
        self.rt_str("ts_rt_bigint_literal_gc", vec![s], Ty::BigInt)
    }

    /// `a op b` where either side is a BigInt; None when neither is.
    pub(crate) fn bigint_binary(&mut self, op: BinOp, l: &Expr, r: &Expr) -> R<Option<Expr>> {
        if !Self::is_big(l) && !Self::is_big(r) {
            return Ok(None);
        }
        if !(Self::is_big(l) && Self::is_big(r)) {
            bail!(
                "a bigint cannot be mixed with another type in arithmetic (JavaScript throws a TypeError); convert with BigInt(x) or Number(x)"
            )
        }
        let code = match op {
            BinOp::Add => 0.0,
            BinOp::Sub => 1.0,
            BinOp::Mul => 2.0,
            BinOp::Div => 3.0,
            BinOp::Rem => 4.0,
            BinOp::Pow => 5.0,
            BinOp::BitAnd => 6.0,
            BinOp::BitOr => 7.0,
            BinOp::BitXor => 8.0,
            BinOp::Shl => 9.0,
            BinOp::Shr => 10.0,
            BinOp::UShr => bail!("BigInts have no unsigned right shift, use >> instead"),
            _ => bail!("this operator is not defined for bigint"),
        };
        let call = self.rt_str(
            "ts_rt_bigint_op_gc",
            vec![num(code), l.clone(), r.clone()],
            Ty::BigInt,
        );
        Ok(Some(self.checked(call, Ty::BigInt)))
    }

    /// `-a` and `~a` on a BigInt.
    pub(crate) fn bigint_unary(&mut self, negate: bool, a: Expr) -> Expr {
        let flag = mk(ExprKind::Bool(negate), Ty::Bool);
        self.rt_str("ts_rt_bigint_unary_gc", vec![flag, a], Ty::BigInt)
    }

    /// `a++` / `--a` on a BigInt place: the new value.
    pub(crate) fn bigint_step(&mut self, old: Expr, up: bool) -> R<Expr> {
        let one = self.bigint_one();
        let op = if up { BinOp::Add } else { BinOp::Sub };
        Ok(self
            .bigint_binary(op, &old, &one)?
            .expect("both sides are BigInts"))
    }

    /// A comparison with a BigInt on either side (`1n < 2n`, `2n > 1`, `5n == 5`); None when neither is.
    pub(crate) fn bigint_compare(&mut self, op: CmpOp, l: &Expr, r: &Expr) -> R<Option<Expr>> {
        let (lb, rb) = (Self::is_big(l), Self::is_big(r));
        if !lb && !rb {
            return Ok(None);
        }
        let flip = |op: CmpOp| match op {
            CmpOp::Lt => CmpOp::Gt,
            CmpOp::Gt => CmpOp::Lt,
            CmpOp::Le => CmpOp::Ge,
            CmpOp::Ge => CmpOp::Le,
            other => other,
        };
        let (ordered, op) = match (lb, rb) {
            (true, true) => (
                self.rt_str("ts_rt_bigint_cmp", vec![l.clone(), r.clone()], Ty::Num),
                op,
            ),
            (true, false) if r.ty.strip() == &Ty::Num => (
                self.rt_str("ts_rt_bigint_cmp_f64", vec![l.clone(), r.clone()], Ty::Num),
                op,
            ),
            (false, true) if l.ty.strip() == &Ty::Num => (
                self.rt_str("ts_rt_bigint_cmp_f64", vec![r.clone(), l.clone()], Ty::Num),
                flip(op),
            ),
            _ => bail!("a bigint can be compared with a bigint or a number only"),
        };
        Ok(Some(mk(
            ExprKind::Cmp(op, Box::new(ordered), Box::new(num(0.0))),
            Ty::Bool,
        )))
    }

    /// `BigInt(x)`.
    pub(crate) fn lower_bigint_call(&mut self, c: &'a CallExpression<'a>) -> R<Expr> {
        let at = self.loc(c.span.start);
        let Some(e) = c.arguments.first().and_then(|a| a.as_expression()) else {
            bail!("{at}: BigInt() needs an argument")
        };
        let v = self.lower_expr(e, None)?;
        let call = match v.ty.strip().clone() {
            Ty::BigInt => return Ok(v),
            Ty::Num => self.rt_str("ts_rt_bigint_from_f64_gc", vec![v], Ty::BigInt),
            Ty::Bool => {
                let n = self.bool_as_num(v);
                self.rt_str("ts_rt_bigint_from_f64_gc", vec![n], Ty::BigInt)
            }
            Ty::Str | Ty::Lit => {
                let s = self.widen_lit(v);
                let s = self.coerce(s, &Ty::Str)?;
                self.rt_str("ts_rt_bigint_from_text_gc", vec![s], Ty::BigInt)
            }
            t => bail!(
                "{at}: BigInt({t}) is not supported (a number, a string, a boolean or a bigint)"
            ),
        };
        Ok(self.checked(call, Ty::BigInt))
    }

    /// `Number(big)`.
    pub(crate) fn bigint_to_number(&mut self, v: Expr) -> Expr {
        self.rt_str("ts_rt_bigint_to_f64", vec![v], Ty::Num)
    }

    /// `big.toString(radix)` as text.
    pub(crate) fn bigint_to_string(&mut self, v: Expr, radix: Expr) -> Expr {
        let call = self.rt_str("ts_rt_bigint_to_string_gc", vec![v, radix], Ty::Str);
        self.checked(call, Ty::Str)
    }

    /// `BigInt.asIntN(bits, big)` and `BigInt.asUintN`.
    pub(crate) fn lower_bigint_static(&mut self, m: &str, c: &'a CallExpression<'a>) -> R<Expr> {
        let at = self.loc(c.span.start);
        let args: Vec<&'a Expression<'a>> = c
            .arguments
            .iter()
            .filter_map(|a| a.as_expression())
            .collect();
        match (m, args.as_slice()) {
            ("asIntN" | "asUintN", [bits, big]) => {
                let bits = self.num_arg(bits)?;
                let big = self.lower_expr(big, None)?;
                if !Self::is_big(&big) {
                    bail!("{at}: BigInt.{m} needs a bigint")
                }
                let signed = mk(ExprKind::Bool(m == "asIntN"), Ty::Bool);
                let call = self.rt_str("ts_rt_bigint_as_n_gc", vec![signed, bits, big], Ty::BigInt);
                Ok(self.checked(call, Ty::BigInt))
            }
            _ => bail!("{at}: BigInt.{m} is not supported"),
        }
    }

    /// A method call on a `bigint`.
    pub(crate) fn lower_bigint_method(
        &mut self,
        recv: Expr,
        m: &str,
        c: &'a CallExpression<'a>,
    ) -> R<Expr> {
        let at = self.loc(c.span.start);
        match m {
            "valueOf" => Ok(recv),
            "toString" => {
                let radix = match c.arguments.first().and_then(|a| a.as_expression()) {
                    Some(e) => self.num_arg(e)?,
                    None => num(10.0),
                };
                Ok(self.bigint_to_string(recv, radix))
            }
            _ => bail!("{at}: the bigint method `{m}` is not supported yet"),
        }
    }
}
