//! `Object.keys / values / entries / assign / freeze / hasOwn`, the `in` operator, object spread and
//! rest. The program is typed and closed, so the keys of an object are the fields of its struct and
//! these are all resolved at compile time (`Object.keys(p)` is an array of constant strings).
//! Objects used as dictionaries with keys known only at run time need `Map`.

use super::expr::{Elem, mk, num};
use super::*;

impl<'a> Lowerer<'a> {
    /// The own, visible fields of a struct: name, type and index (private `#x` fields excluded).
    pub(crate) fn own_fields(&self, sid: StructId) -> Vec<(String, Ty, usize)> {
        self.m.structs[sid]
            .fields
            .iter()
            .enumerate()
            .filter(|(_, f)| !f.name.starts_with("_p_"))
            .map(|(i, f)| (f.name.clone(), f.ty.clone(), i))
            .collect()
    }

    fn object_arg(&mut self, c: &'a CallExpression<'a>, i: usize, what: &str) -> R<Expr> {
        let at = self.loc(c.span.start);
        let Some(e) = c.arguments.get(i).and_then(|a| a.as_expression()) else {
            bail!("{at}: {what}: missing argument")
        };
        let v = self.lower_expr(e, None)?;
        if !matches!(v.ty.strip(), Ty::Obj(_)) {
            bail!("{at}: {what} needs an object, found {}", v.ty)
        }
        Ok(v)
    }

    /// The keys of `obj` as an array of strings and its values as an array (their common type), both
    /// read from one evaluation of `obj`.
    pub(crate) fn object_parts(
        &mut self,
        obj: Expr,
        what: &str,
        need_values: bool,
    ) -> R<(Expr, Expr, Vec<Stmt>)> {
        let Ty::Obj(sid) = obj.ty.strip().clone() else {
            bail!("{what} needs an object, found {}", obj.ty)
        };
        let tmp = self.new_local(obj.ty.clone());
        let pre = vec![Stmt::Assign(tmp, obj)];
        let fields = self.own_fields(sid);
        let mut keys = Vec::new();
        for (name, _, _) in &fields {
            keys.push(Elem::One(self.str_lit(name)));
        }
        let keys = self.build_array(keys, Ty::Str)?;
        let vt_first = fields.first().map(|f| f.1.clone());
        let Some(vt) = vt_first.filter(|vt| fields.iter().all(|(_, t, _)| t == vt)) else {
            if need_values && !fields.is_empty() {
                bail!("{what}: the values have different types; read the fields you need")
            }
            let empty = mk(
                ExprKind::Rt(RtFn::ArrNew, vec![num(0.0)]),
                Ty::Arr(Box::new(Ty::Num)),
            );
            return Ok((keys, empty, pre));
        };
        let mut vals = Vec::new();
        for (_, ty, i) in &fields {
            let read = mk(
                ExprKind::Field(Box::new(self.var_expr(tmp)), sid, *i),
                ty.clone(),
            );
            vals.push(Elem::One(read));
        }
        let vals = self.build_array(vals, vt)?;
        Ok((keys, vals, pre))
    }

    /// `Object.f(..)`.
    pub(crate) fn lower_object_static(
        &mut self,
        method: &str,
        c: &'a CallExpression<'a>,
    ) -> R<Expr> {
        let at = self.loc(c.span.start);
        match method {
            "keys" | "values" => {
                let o = self.object_arg(c, 0, &format!("Object.{method}"))?;
                let (keys, vals, mut pre) =
                    self.object_parts(o, &format!("Object.{method}"), method == "values")?;
                let result = if method == "keys" { keys } else { vals };
                let ty = result.ty.clone();
                Ok(mk(
                    ExprKind::Seq(std::mem::take(&mut pre), Box::new(result)),
                    ty,
                ))
            }
            "entries" => bail!(
                "{at}: Object.entries is supported as `for (const [k, v] of Object.entries(o))`"
            ),
            "freeze" => {
                // Nothing rejects a write to a frozen object (see docs/differences.md).
                let o = self.object_arg(c, 0, "Object.freeze")?;
                Ok(o)
            }
            "hasOwn" => {
                let o = self.object_arg(c, 0, "Object.hasOwn")?;
                let Some(Expression::StringLiteral(key)) =
                    c.arguments.get(1).and_then(|a| a.as_expression())
                else {
                    bail!(
                        "{at}: Object.hasOwn needs a string literal key (the keys are fixed at compile time)"
                    )
                };
                let Ty::Obj(sid) = o.ty.strip().clone() else {
                    unreachable!("checked")
                };
                let has = self
                    .own_fields(sid)
                    .iter()
                    .any(|(n, _, _)| n == key.value.as_str());
                Ok(mk(
                    ExprKind::Seq(
                        vec![Stmt::Eval(o)],
                        Box::new(mk(ExprKind::Bool(has), Ty::Bool)),
                    ),
                    Ty::Bool,
                ))
            }
            "assign" => {
                let target = self.object_arg(c, 0, "Object.assign")?;
                let Ty::Obj(tsid) = target.ty.strip().clone() else {
                    unreachable!("checked")
                };
                let t = self.new_local(target.ty.clone());
                let mut stmts = vec![Stmt::Assign(t, target.clone())];
                for i in 1..c.arguments.len() {
                    let src = self.object_arg(c, i, "Object.assign")?;
                    let Ty::Obj(ssid) = src.ty.strip().clone() else {
                        unreachable!("checked")
                    };
                    let s = self.new_local(src.ty.clone());
                    stmts.push(Stmt::Assign(s, src));
                    for (name, ty, fi) in self.own_fields(ssid) {
                        let Some(j) = self.m.structs[tsid].field(&name) else {
                            bail!(
                                "{at}: Object.assign: {} has no field `{name}` (objects cannot gain fields)",
                                self.m.structs[tsid].name
                            )
                        };
                        let want = self.m.structs[tsid].fields[j].ty.clone();
                        let read = mk(ExprKind::Field(Box::new(self.var_expr(s)), ssid, fi), ty);
                        let v = self.coerce(read, &want)?;
                        stmts.push(Stmt::StoreField(self.var_expr(t), tsid, j, v));
                    }
                }
                let ty = target.ty.clone();
                Ok(mk(ExprKind::Seq(stmts, Box::new(self.var_expr(t))), ty))
            }
            m => bail!("{at}: Object.{m} is not supported yet"),
        }
    }

    /// `"key" in obj` for a string literal key: decided by the object's type.
    pub(crate) fn lower_in(&mut self, key: &'a Expression<'a>, obj: Expr, at: u32) -> R<Expr> {
        let Expression::StringLiteral(k) = key else {
            bail!(
                "{}: `in` needs a string literal key (the keys are fixed at compile time)",
                self.loc(at)
            )
        };
        let Ty::Obj(sid) = obj.ty.strip().clone() else {
            bail!("{}: `in` needs an object, found {}", self.loc(at), obj.ty)
        };
        let name = k.value.as_str();
        let found = self.own_fields(sid).iter().any(|(n, _, _)| n == name)
            || self.resolve_in(sid, classes::Which::Method, name).is_some()
            || self.resolve_in(sid, classes::Which::Getter, name).is_some()
            // Every object has these through its prototype.
            || matches!(
                name,
                "toString" | "valueOf" | "constructor" | "hasOwnProperty" | "toLocaleString"
            );
        Ok(mk(
            ExprKind::Seq(
                vec![Stmt::Eval(obj)],
                Box::new(mk(ExprKind::Bool(found), Ty::Bool)),
            ),
            Ty::Bool,
        ))
    }

    /// `{ a, ...rest } = obj`: a new object with the fields of `obj` that were not named.
    pub(crate) fn object_rest(&mut self, src: &Expr, taken: &[String]) -> R<Expr> {
        let Ty::Obj(sid) = src.ty.strip().clone() else {
            bail!("a rest element needs an object, found {}", src.ty)
        };
        let fields: Vec<(String, Ty, usize)> = self
            .own_fields(sid)
            .into_iter()
            .filter(|(n, _, _)| !taken.contains(n))
            .collect();
        let rsid = self.anon_struct(
            "Anon",
            fields
                .iter()
                .map(|(n, t, _)| Field {
                    name: n.clone(),
                    ty: t.clone(),
                })
                .collect(),
        );
        let rty = Ty::Obj(rsid);
        let r = self.new_local(rty.clone());
        let mut stmts = vec![Stmt::Assign(r, mk(ExprKind::NewObj(rsid), rty.clone()))];
        for (k, (_, ty, i)) in fields.iter().enumerate() {
            let read = mk(ExprKind::Field(Box::new(src.clone()), sid, *i), ty.clone());
            stmts.push(Stmt::StoreField(self.var_expr(r), rsid, k, read));
        }
        Ok(mk(ExprKind::Seq(stmts, Box::new(self.var_expr(r))), rty))
    }
}
