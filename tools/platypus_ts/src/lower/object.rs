// SPDX-License-Identifier: Apache-2.0

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
        if !matches!(v.ty.strip(), Ty::Obj(_) | Ty::Map(..)) {
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
        // A dictionary: its keys and values now.
        if let Ty::Map(k, v) = obj.ty.strip().clone() {
            let tmp = self.new_local(obj.ty.clone());
            let pre = vec![Stmt::Assign(tmp, obj)];
            let keys = self.rt_map("ts_rt_map_keys_gc", vec![self.var_expr(tmp)], Ty::Arr(k));
            let vals = self.rt_map("ts_rt_map_values_gc", vec![self.var_expr(tmp)], Ty::Arr(v));
            return Ok((keys, vals, pre));
        }
        let Ty::Obj(sid) = obj.ty.strip().clone() else {
            bail!("{what} needs an object, found {}", obj.ty)
        };
        let tmp = self.new_local(obj.ty.clone());
        let mut pre = vec![Stmt::Assign(tmp, obj)];
        let fields = self.own_fields(sid);
        // An optional property is there only while it is set: the arrays are filled at run time.
        if fields.iter().any(|(_, _, i)| self.absent_able(sid, *i)) {
            let common = fields
                .first()
                .map(|f| f.1.strip().clone())
                .filter(|vt| fields.iter().all(|(_, t, _)| t.strip() == vt));
            if common.is_none() && need_values && !fields.is_empty() {
                bail!("{what}: the values have different types; read the fields you need")
            }
            let keys_ty = Ty::Arr(Box::new(Ty::Str));
            let vals_ty = Ty::Arr(Box::new(common.clone().unwrap_or(Ty::Num)));
            let (keys_v, vals_v) = (
                self.new_local(keys_ty.clone()),
                self.new_local(vals_ty.clone()),
            );
            pre.push(Stmt::Assign(
                keys_v,
                mk(ExprKind::Rt(RtFn::ArrNew, vec![num(0.0)]), keys_ty.clone()),
            ));
            pre.push(Stmt::Assign(
                vals_v,
                mk(ExprKind::Rt(RtFn::ArrNew, vec![num(0.0)]), vals_ty.clone()),
            ));
            for (name, ty, i) in &fields {
                let read = mk(
                    ExprKind::Field(Box::new(self.var_expr(tmp)), sid, *i),
                    ty.clone(),
                );
                let mut push = vec![Stmt::Eval(mk(
                    ExprKind::Rt(
                        RtFn::ArrPush,
                        vec![self.var_expr(keys_v), self.str_lit(name)],
                    ),
                    Ty::Void,
                ))];
                if let Some(vt) = &common {
                    let v = if ty.is_nullable() && !vt.is_nullable() {
                        mk(ExprKind::Cast(Box::new(read.clone())), vt.clone())
                    } else {
                        read.clone()
                    };
                    push.push(Stmt::Eval(mk(
                        ExprKind::Rt(RtFn::ArrPush, vec![self.var_expr(vals_v), v]),
                        Ty::Void,
                    )));
                }
                if self.absent_able(sid, *i) {
                    let absent = mk(ExprKind::IsNull(Box::new(read)), Ty::Bool);
                    let present = mk(ExprKind::Un(UnOp::Not, Box::new(absent)), Ty::Bool);
                    pre.push(Stmt::If(present, push, vec![]));
                } else {
                    pre.extend(push);
                }
            }
            return Ok((self.var_expr(keys_v), self.var_expr(vals_v), pre));
        }
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
            "entries" => {
                let o = self.object_arg(c, 0, "Object.entries")?;
                self.entries_array(o, "Object.entries")
            }
            "fromEntries" => self.lower_from_entries(c),
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
                // Into a dictionary: the properties of an object and the entries of another
                // dictionary are set one by one, later sources winning.
                if let Ty::Map(_, vty) = target.ty.strip().clone() {
                    return self.assign_into_map(c, target, &vty, &at);
                }
                let Ty::Obj(tsid) = target.ty.strip().clone() else {
                    unreachable!("checked")
                };
                let t = self.new_local(target.ty.clone());
                let mut stmts = vec![Stmt::Assign(t, target.clone())];
                for i in 1..c.arguments.len() {
                    let src = self.object_arg(c, i, "Object.assign")?;
                    match src.ty.strip().clone() {
                        Ty::Obj(ssid) => {
                            let s = self.new_local(src.ty.clone());
                            stmts.push(Stmt::Assign(s, src));
                            for (name, ty, fi) in self.own_fields(ssid) {
                                let Some(j) = self.m.structs[tsid].field(&name) else {
                                    // JavaScript adds the property; a struct has a fixed layout.
                                    let msg = format!(
                                        "Object.assign: `{name}` is not a field of {}; it was not copied (objects have fixed fields)",
                                        self.m.structs[tsid].name
                                    );
                                    let msg = self.str_lit(&msg);
                                    let warn = self.warn_once(msg);
                                    stmts.push(warn);
                                    continue;
                                };
                                let want = self.m.structs[tsid].fields[j].ty.clone();
                                let read =
                                    mk(ExprKind::Field(Box::new(self.var_expr(s)), ssid, fi), ty);
                                let v = self.coerce(read, &want)?;
                                stmts.push(Stmt::StoreField(self.var_expr(t), tsid, j, v));
                            }
                        }
                        // A dictionary: its keys are known only at run time.
                        Ty::Map(_, vty) => {
                            let tv = self.var_expr(t);
                            self.assign_map_into_struct(&mut stmts, tv, tsid, src, &vty)?;
                        }
                        other => bail!("{at}: Object.assign needs objects, found {other}"),
                    }
                }
                let ty = target.ty.clone();
                Ok(mk(ExprKind::Seq(stmts, Box::new(self.var_expr(t))), ty))
            }
            m => bail!("{at}: Object.{m} is not supported yet"),
        }
    }

    /// `Object.assign(dict, a, b, ..)` where `dict` is a `Record<string, V>`.
    fn assign_into_map(
        &mut self,
        c: &'a CallExpression<'a>,
        target: Expr,
        vty: &Ty,
        at: &str,
    ) -> R<Expr> {
        let t = self.new_local(target.ty.clone());
        let mut stmts = vec![Stmt::Assign(t, target.clone())];
        for i in 1..c.arguments.len() {
            let src = self.object_arg(c, i, "Object.assign")?;
            self.assign_source_into_map(&mut stmts, t, src, vty, at)?;
        }
        let ty = target.ty.clone();
        Ok(mk(ExprKind::Seq(stmts, Box::new(self.var_expr(t))), ty))
    }

    /// The properties of an object, or the entries of another dictionary, set into the
    /// dictionary in `t` (later sources win). Also `{ ...a }` into a `Record<string, V>`.
    pub(crate) fn assign_source_into_map(
        &mut self,
        stmts: &mut Vec<Stmt>,
        t: VarId,
        src: Expr,
        vty: &Ty,
        at: &str,
    ) -> R<()> {
        match src.ty.strip().clone() {
            Ty::Obj(ssid) => {
                let s = self.new_local(src.ty.clone());
                stmts.push(Stmt::Assign(s, src));
                for (name, ty, fi) in self.own_fields(ssid) {
                    let read = mk(
                        ExprKind::Field(Box::new(self.var_expr(s)), ssid, fi),
                        ty.clone(),
                    );
                    let v = self.coerce(read.clone(), vty)?;
                    let key = self.str_lit(&name);
                    let set = self.map_set_value(self.var_expr(t), key, Some(v))?;
                    // An optional property that is not there is not copied.
                    if self.absent_able(ssid, fi) {
                        let absent = mk(ExprKind::IsNull(Box::new(read)), Ty::Bool);
                        stmts.push(Stmt::If(absent, vec![], vec![Stmt::Eval(set)]));
                    } else {
                        stmts.push(Stmt::Eval(set));
                    }
                }
            }
            Ty::Map(..) => {
                let (keys, vals, pre) = self.object_parts(src, "Object.assign", true)?;
                if !matches!(vals.ty.strip(), Ty::Arr(e) if e.strip() == vty.strip()) {
                    bail!("{at}: Object.assign: the dictionaries hold different value types")
                }
                stmts.extend(pre);
                let values = self.new_local(vals.ty.clone());
                stmts.push(Stmt::Assign(values, vals));
                let it = self.begin_iter(keys)?;
                stmts.extend(it.pre);
                let entry = self.dict_entry(values, it.idx, vty);
                let set =
                    self.map_set_value(self.var_expr(t), self.var_expr(it.elem), Some(entry))?;
                let mut body = it.head;
                body.push(Stmt::Eval(set));
                stmts.push(Stmt::Loop(it.label, body));
            }
            other => bail!("{at}: Object.assign needs objects, found {other}"),
        }
        Ok(())
    }

    /// `values[idx]` of the values array of a dictionary, as its value type.
    fn dict_entry(&mut self, values: VarId, idx: VarId, vty: &Ty) -> Expr {
        let raw_ty = if vty.is_ptr() || *vty == Ty::Num {
            vty.clone().nullable()
        } else {
            vty.clone()
        };
        let raw = mk(
            ExprKind::Index(
                Box::new(self.var_expr(values)),
                Box::new(self.var_expr(idx)),
            ),
            raw_ty.clone(),
        );
        if raw_ty == *vty {
            raw
        } else {
            mk(ExprKind::Cast(Box::new(raw)), vty.clone())
        }
    }

    /// The entries of a dictionary copied into a struct by key, at run time: an entry whose key
    /// is a field of the struct is stored (a value of the wrong type is a `TypeError` naming the
    /// key and the struct); one that is no field cannot be added to a fixed layout, so a dev
    /// warning names it (once per call site). `t` is the struct (a variable).
    pub(crate) fn assign_map_into_struct(
        &mut self,
        stmts: &mut Vec<Stmt>,
        t: Expr,
        tsid: StructId,
        src: Expr,
        vty: &Ty,
    ) -> R<()> {
        let (keys, vals, pre) = self.object_parts(src, "Object.assign", true)?;
        stmts.extend(pre);
        let values = self.new_local(vals.ty.clone());
        stmts.push(Stmt::Assign(values, vals));
        let it = self.begin_iter(keys)?;
        stmts.extend(it.pre.clone());
        let entry = self.dict_entry(values, it.idx, vty);
        let key = self.var_expr(it.elem);
        let sname = self.m.structs[tsid].name.clone();
        // No field of that name: the warning.
        let head = self.str_lit("Object.assign: `");
        let tail = self.str_lit(&format!(
            "` is not a field of {sname}; it was not copied (objects have fixed fields)"
        ));
        let msg = self.str_concat(head, key.clone())?;
        let msg = self.str_concat(msg, tail)?;
        let mut chain = vec![self.warn_once(msg)];
        for (name, fty, j) in self.own_fields(tsid).into_iter().rev() {
            let then = match self.coerce(entry.clone(), &fty) {
                Ok(v) => vec![Stmt::StoreField(t.clone(), tsid, j, v)],
                Err(_) => match self.fids.get("__rtThrow").copied() {
                    Some(fid) => {
                        let msg = self.str_lit(&format!(
                            "Object.assign: the value of `{name}` is {vty}, but the field `{name}` of {sname} is {fty}"
                        ));
                        vec![Stmt::Eval(mk(
                            ExprKind::Call(fid, vec![num(2.0), msg]),
                            Ty::Void,
                        ))]
                    }
                    None => vec![],
                },
            };
            let lit = self.str_lit(&name);
            let is = self.lower_compare(CmpOp::Eq, key.clone(), lit, 0)?;
            chain = vec![Stmt::If(is, then, chain)];
        }
        let mut body = it.head;
        body.extend(chain);
        stmts.push(Stmt::Loop(it.label, body));
        Ok(())
    }

    /// A dev warning (`console.warn`) the first time this statement runs.
    pub(crate) fn warn_once(&mut self, msg: Expr) -> Stmt {
        let g = self.m.globals.len();
        self.m.globals.push(Global {
            name: format!("__warned{g}"),
            ty: Ty::Bool,
        });
        let flag = mk(ExprKind::GlobalVar(g), Ty::Bool);
        let log = self.rt_str("ts_rt_console", vec![num(3.0), msg], Ty::Void);
        Stmt::If(
            flag,
            vec![],
            vec![
                Stmt::StoreGlobal(g, mk(ExprKind::Bool(true), Ty::Bool)),
                Stmt::Eval(log),
            ],
        )
    }

    /// `"key" in obj` for a string literal key: decided by the object's type.
    pub(crate) fn lower_in(&mut self, key: &'a Expression<'a>, obj: Expr, at: u32) -> R<Expr> {
        let Expression::StringLiteral(k) = key else {
            return self.lower_in_dynamic(key, obj, at);
        };
        if let Ty::Map(..) = obj.ty.strip() {
            return self.lower_in_dynamic(key, obj, at);
        }
        let Ty::Obj(sid) = obj.ty.strip().clone() else {
            bail!("{}: `in` needs an object, found {}", self.loc(at), obj.ty)
        };
        let name = k.value.as_str();
        // A union of object types: the tag says whether the member has the property.
        if self.union_members(&obj.ty).is_some() {
            let owners: Vec<StructId> = self
                .m
                .structs
                .iter()
                .enumerate()
                .filter(|(_, st)| st.field(name).is_some())
                .map(|(i, _)| i)
                .collect();
            return Ok(self.union_is_member(
                obj,
                |m| matches!(m.strip(), Ty::Obj(s) if owners.contains(s)),
            ));
        }
        // An optional property is there while it is set.
        if let Some(i) = self.m.structs[sid].field(name)
            && self.absent_able(sid, i)
        {
            let fty = self.m.structs[sid].fields[i].ty.clone();
            let tmp = self.new_local(obj.ty.clone());
            let read = mk(ExprKind::Field(Box::new(self.var_expr(tmp)), sid, i), fty);
            let present = mk(
                ExprKind::Un(
                    UnOp::Not,
                    Box::new(mk(ExprKind::IsNull(Box::new(read)), Ty::Bool)),
                ),
                Ty::Bool,
            );
            return Ok(mk(
                ExprKind::Seq(vec![Stmt::Assign(tmp, obj)], Box::new(present)),
                Ty::Bool,
            ));
        }
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
        let rest_fields: Vec<Field> = fields
            .iter()
            .map(|(n, t, i)| Field {
                name: n.clone(),
                ty: t.clone(),
                optional: self.m.structs[sid].fields[*i].optional,
            })
            .collect();
        let rsid = self.anon_struct("Anon", rest_fields);
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
