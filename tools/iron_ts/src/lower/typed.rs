// SPDX-License-Identifier: Apache-2.0

//! Typed arrays (`Int8Array` .. `Float64Array`, `Uint8ClampedArray`): `new`, `from`, the in-place
//! methods (`fill`, `set`, `reverse`, `sort`), `slice`, and every read-only array method through a
//! `number[]` copy. Element reads and writes are in the code generator (`typed_load`/`typed_store`),
//! the runtime is `iron_ts_rt/src/typed.rs`. `subarray`, `new X(buffer, offset, length)` and `.buffer`
//! make views that share memory; `ArrayBuffer` and `DataView` are prelude classes.

use super::expr::{mk, num};
use super::*;
use crate::ir::TypedKind;

impl<'a> Lowerer<'a> {
    pub(crate) fn kind_num(kind: TypedKind) -> Expr {
        num(f64::from(kind.code()))
    }

    /// A typed array made from `src`: a length, a `number[]` or another typed array.
    pub(crate) fn typed_from(&mut self, kind: TypedKind, src: Expr, at: &str) -> R<Expr> {
        let ty = Ty::Typed(kind);
        Ok(match src.ty.strip().clone() {
            Ty::Num => self.rt_str("ts_rt_typed_new_gc", vec![Self::kind_num(kind), src], ty),
            Ty::Arr(el) if *el == Ty::Num => self.rt_str(
                "ts_rt_typed_from_array_gc",
                vec![Self::kind_num(kind), src],
                ty,
            ),
            Ty::Typed(k2) => self.rt_str(
                "ts_rt_typed_from_typed_gc",
                vec![Self::kind_num(kind), Self::kind_num(k2), src],
                ty,
            ),
            t => bail!(
                "{at}: new {}({t}) is not supported (a length, a number[] or a typed array)",
                kind.name()
            ),
        })
    }

    /// `new Float32Array(length | numbers | typed)`.
    pub(crate) fn lower_new_typed(&mut self, kind: TypedKind, n: &'a NewExpression<'a>) -> R<Expr> {
        let at = self.loc(n.span.start);
        let args: Vec<&'a Expression<'a>> = n
            .arguments
            .iter()
            .filter_map(|a| a.as_expression())
            .collect();
        match args.as_slice() {
            [] => self.typed_from(kind, num(0.0), &at),
            [first, rest @ ..] if rest.len() <= 2 => {
                let v = self.lower_expr(first, None)?;
                let is_buffer = matches!(v.ty.strip(), Ty::Obj(sid) if self.m.structs[*sid].name == "ArrayBuffer");
                if !is_buffer {
                    if !rest.is_empty() {
                        bail!(
                            "{at}: new {}(x, offset, length) needs an ArrayBuffer",
                            kind.name()
                        )
                    }
                    return self.typed_from(kind, v, &at);
                }
                let Ty::Obj(sid) = v.ty.strip().clone() else {
                    unreachable!("checked")
                };
                let bytes = self.field_of(v, sid, "bytes");
                let off = match rest.first() {
                    Some(e) => self.num_arg(e)?,
                    None => num(0.0),
                };
                let len = match rest.get(1) {
                    Some(e) => self.num_arg(e)?,
                    None => num(f64::NAN),
                };
                let ty = Ty::Typed(kind);
                let call = self.rt_str(
                    "ts_rt_typed_view_gc",
                    vec![Self::kind_num(kind), bytes, off, len],
                    ty.clone(),
                );
                Ok(self.checked(call, ty))
            }
            _ => bail!("{at}: new {}(buffer, offset, length)", kind.name()),
        }
    }

    /// `Float32Array.from(x)`, `.BYTES_PER_ELEMENT`.
    pub(crate) fn lower_typed_static(
        &mut self,
        kind: TypedKind,
        m: &str,
        c: &'a CallExpression<'a>,
    ) -> R<Expr> {
        let at = self.loc(c.span.start);
        match m {
            "from" => {
                let Some(e) = c.arguments.first().and_then(|a| a.as_expression()) else {
                    bail!("{at}: {}.from needs an argument", kind.name())
                };
                let v = self.lower_expr(e, None)?;
                self.typed_from(kind, v, &at)
            }
            _ => bail!("{at}: {}.{m} is not supported", kind.name()),
        }
    }

    /// `typed.xyz(...)`.
    pub(crate) fn lower_typed_method(
        &mut self,
        recv: Expr,
        kind: TypedKind,
        m: &str,
        c: &'a CallExpression<'a>,
    ) -> R<Expr> {
        let at = self.loc(c.span.start);
        let ty = Ty::Typed(kind);
        let args: Vec<&'a Expression<'a>> = c
            .arguments
            .iter()
            .filter_map(|a| a.as_expression())
            .collect();
        let this = self.new_local(ty.clone());
        let thisv = self.var_expr(this);
        let bind = Stmt::Assign(this, recv);
        let optional = |this: &mut Self, i: usize, default: f64| -> R<Expr> {
            match args.get(i) {
                Some(e) => this.num_arg(e),
                None => Ok(num(default)),
            }
        };
        match m {
            "fill" => {
                let Some(v) = args.first() else {
                    bail!("{at}: fill needs a value")
                };
                let v = self.num_arg(v)?;
                let (s, e) = (optional(self, 1, 0.0)?, optional(self, 2, f64::INFINITY)?);
                let call = self.rt_str(
                    "ts_rt_typed_fill",
                    vec![thisv.clone(), Self::kind_num(kind), v, s, e],
                    Ty::Void,
                );
                Ok(mk(
                    ExprKind::Seq(vec![bind, Stmt::Eval(call)], Box::new(thisv)),
                    ty,
                ))
            }
            "set" => {
                let Some(src) = args.first() else {
                    bail!("{at}: set needs a source")
                };
                let src = self.lower_expr(src, None)?;
                let src_kind = match src.ty.strip() {
                    Ty::Arr(el) if **el == Ty::Num => num(255.0),
                    Ty::Typed(k2) => Self::kind_num(*k2),
                    t => bail!("{at}: set({t}) is not supported (a number[] or a typed array)"),
                };
                let off = optional(self, 1, 0.0)?;
                let call = self.rt_str(
                    "ts_rt_typed_set",
                    vec![thisv.clone(), Self::kind_num(kind), src, src_kind, off],
                    Ty::Void,
                );
                let call = self.checked(call, Ty::Void);
                Ok(mk(
                    ExprKind::Seq(vec![bind, Stmt::Eval(call)], Box::new(num(0.0))),
                    Ty::Void,
                ))
            }
            "slice" => {
                let (s, e) = (optional(self, 0, 0.0)?, optional(self, 1, f64::INFINITY)?);
                let call = self.rt_str(
                    "ts_rt_typed_slice_gc",
                    vec![thisv, Self::kind_num(kind), s, e],
                    ty.clone(),
                );
                Ok(mk(ExprKind::Seq(vec![bind], Box::new(call)), ty))
            }
            "subarray" => {
                let (s, e) = (optional(self, 0, 0.0)?, optional(self, 1, f64::INFINITY)?);
                let call = self.rt_str(
                    "ts_rt_typed_subarray_gc",
                    vec![thisv, Self::kind_num(kind), s, e],
                    ty.clone(),
                );
                Ok(mk(ExprKind::Seq(vec![bind], Box::new(call)), ty))
            }
            "reverse" | "sort" | "toSorted" | "toReversed" if args.is_empty() => {
                let copy = matches!(m, "toSorted" | "toReversed");
                let target = if copy {
                    self.rt_str(
                        "ts_rt_typed_slice_gc",
                        vec![
                            thisv.clone(),
                            Self::kind_num(kind),
                            num(0.0),
                            num(f64::INFINITY),
                        ],
                        ty.clone(),
                    )
                } else {
                    thisv.clone()
                };
                let t = self.new_local(ty.clone());
                let tv = self.var_expr(t);
                let call = self.rt_str(
                    "ts_rt_typed_reorder",
                    vec![
                        tv.clone(),
                        Self::kind_num(kind),
                        mk(
                            ExprKind::Bool(m.ends_with("ort") || m == "toSorted"),
                            Ty::Bool,
                        ),
                    ],
                    Ty::Void,
                );
                Ok(mk(
                    ExprKind::Seq(
                        vec![bind, Stmt::Assign(t, target), Stmt::Eval(call)],
                        Box::new(tv),
                    ),
                    ty,
                ))
            }
            "sort" | "toSorted" => {
                // With a comparator: sort a number[] copy, then write it back.
                let arr = self.rt_str(
                    "ts_rt_typed_to_array_gc",
                    vec![Self::kind_num(kind), thisv.clone()],
                    Ty::Arr(Box::new(Ty::Num)),
                );
                let sorted = self.lower_method(arr, "sort", c)?;
                if m == "toSorted" {
                    let r = self.typed_from(kind, sorted, &at)?;
                    return Ok(mk(ExprKind::Seq(vec![bind], Box::new(r)), ty));
                }
                let set = self.rt_str(
                    "ts_rt_typed_set",
                    vec![
                        thisv.clone(),
                        Self::kind_num(kind),
                        sorted,
                        num(255.0),
                        num(0.0),
                    ],
                    Ty::Void,
                );
                Ok(mk(
                    ExprKind::Seq(vec![bind, Stmt::Eval(set)], Box::new(thisv)),
                    ty,
                ))
            }
            "toString" => {
                let arr = self.rt_str(
                    "ts_rt_typed_to_array_gc",
                    vec![Self::kind_num(kind), thisv],
                    Ty::Arr(Box::new(Ty::Num)),
                );
                let comma = self.str_lit(",");
                let joined = self.rt_str("ts_rt_str_join_nums_gc", vec![arr, comma], Ty::Str);
                Ok(mk(ExprKind::Seq(vec![bind], Box::new(joined)), Ty::Str))
            }
            // Everything else reads or builds from a number[] copy; map and filter come back typed.
            _ => {
                let arr = self.rt_str(
                    "ts_rt_typed_to_array_gc",
                    vec![Self::kind_num(kind), thisv],
                    Ty::Arr(Box::new(Ty::Num)),
                );
                let r = self.lower_method(arr, m, c)?;
                let r = if matches!(m, "map" | "filter") {
                    self.typed_from(kind, r, &at)?
                } else {
                    r
                };
                Ok(mk(ExprKind::Seq(vec![bind], Box::new(r.clone())), r.ty))
            }
        }
    }
}
