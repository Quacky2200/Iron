// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

//! Objects and dictionaries as collections of entries: `Object.entries` and `Object.fromEntries` as
//! values, `key in obj` with a key known only at run time, `delete obj.key` and
//! `for (const k in obj)`. The keys of a struct are its fields, and an optional field (a
//! `Partial<Record<Tag, V>>`) is there only while it is not null; a `Record<string, V>` is a map.

use super::expr::{mk, num};
use super::*;

/// What a `delete` names.
enum DeleteKey<'a> {
    Name(String),
    Expr(&'a Expression<'a>),
}

impl<'a> Lowerer<'a> {
    /// Can field `i` of the struct be absent (an optional field that is nullable)?
    pub(crate) fn absent_able(&self, sid: StructId, i: usize) -> bool {
        let f = &self.m.structs[sid].fields[i];
        f.optional && f.ty.is_nullable()
    }

    /// `Object.entries(obj)` as an array of `[key, value]` pairs.
    pub(crate) fn entries_array(&mut self, obj: Expr, what: &str) -> R<Expr> {
        let (keys, vals, mut pre) = self.object_parts(obj, what, true)?;
        let vty = match vals.ty.strip() {
            Ty::Arr(e) => (**e).clone(),
            _ => Ty::Num,
        };
        let keys_v = self.new_local(keys.ty.clone());
        let vals_v = self.new_local(vals.ty.clone());
        pre.push(Stmt::Assign(keys_v, keys));
        pre.push(Stmt::Assign(vals_v, vals));
        let it = self.begin_iter(self.var_expr(keys_v))?;
        pre.extend(it.pre.clone());
        let raw_ty = if vty.is_ptr() || vty == Ty::Num {
            vty.clone().nullable()
        } else {
            vty.clone()
        };
        let raw = mk(
            ExprKind::Index(
                Box::new(self.var_expr(vals_v)),
                Box::new(self.var_expr(it.idx)),
            ),
            raw_ty.clone(),
        );
        let v = if raw_ty == vty {
            raw
        } else {
            mk(ExprKind::Cast(Box::new(raw)), vty.clone())
        };
        let pair = self.tuple_or_array(vec![self.var_expr(it.elem), v])?;
        let arr_ty = Ty::Arr(Box::new(pair.ty.clone()));
        let out = self.new_local(arr_ty.clone());
        pre.push(Stmt::Assign(
            out,
            mk(ExprKind::Rt(RtFn::ArrNew, vec![num(0.0)]), arr_ty.clone()),
        ));
        let mut body = it.head.clone();
        body.push(Stmt::Eval(mk(
            ExprKind::Rt(RtFn::ArrPush, vec![self.var_expr(out), pair]),
            Ty::Void,
        )));
        pre.push(Stmt::Loop(it.label, body));
        Ok(mk(ExprKind::Seq(pre, Box::new(self.var_expr(out))), arr_ty))
    }

    /// `Object.fromEntries(pairs)`: a dictionary of the `[key, value]` pairs.
    pub(crate) fn lower_from_entries(&mut self, c: &'a CallExpression<'a>) -> R<Expr> {
        let at = self.loc(c.span.start);
        let Some(arg) = c.arguments.first().and_then(|a| a.as_expression()) else {
            bail!("{at}: Object.fromEntries needs an argument")
        };
        let arr = self.lower_expr(arg, None)?;
        let Ty::Arr(pair) = arr.ty.strip().clone() else {
            bail!(
                "{at}: Object.fromEntries takes an array of [key, value] pairs, found {}",
                arr.ty
            )
        };
        // How to read the key and the value out of a pair.
        let (kty, vty) = match pair.strip().clone() {
            Ty::Obj(sid) if self.is_tuple(sid) && self.m.structs[sid].fields.len() >= 2 => (
                self.m.structs[sid].fields[0].ty.clone(),
                self.m.structs[sid].fields[1].ty.clone(),
            ),
            Ty::Arr(e) => ((*e).clone(), (*e).clone()),
            other => bail!("{at}: Object.fromEntries: a pair is {other}, not [key, value]"),
        };
        if !matches!(kty.strip(), Ty::Str | Ty::Lit) {
            bail!("{at}: Object.fromEntries: the keys must be strings, found {kty}")
        }
        let vty = vty.strip().clone();
        let map_ty = Ty::Map(Box::new(Ty::Str), Box::new(vty.clone()));
        let m = self.new_local(map_ty.clone());
        let it = self.begin_iter(arr)?;
        let mut stmts = it.pre.clone();
        stmts.push(Stmt::Assign(m, self.new_collection(&map_ty)?));
        let elem = self.var_expr(it.elem);
        let (k, v) = match pair.strip().clone() {
            Ty::Obj(sid) => {
                let fk = self.m.structs[sid].fields[0].ty.clone();
                let fv = self.m.structs[sid].fields[1].ty.clone();
                (
                    mk(ExprKind::Field(Box::new(elem.clone()), sid, 0), fk),
                    mk(ExprKind::Field(Box::new(elem), sid, 1), fv),
                )
            }
            _ => {
                let at_ty = if vty.is_ptr() || vty == Ty::Num {
                    vty.clone().nullable()
                } else {
                    vty.clone()
                };
                let read = |i: usize, this: &Self| {
                    let raw = mk(
                        ExprKind::Index(Box::new(elem.clone()), Box::new(num(i as f64))),
                        at_ty.clone(),
                    );
                    if at_ty == vty {
                        raw
                    } else {
                        let _ = this;
                        mk(ExprKind::Cast(Box::new(raw)), vty.clone())
                    }
                };
                (read(0, self), read(1, self))
            }
        };
        let k = self.coerce(k, &Ty::Str)?;
        let v = self.coerce(v, &vty)?;
        let set = self.map_set_value(self.var_expr(m), k, Some(v))?;
        let mut body = it.head.clone();
        body.push(Stmt::Eval(set));
        stmts.push(Stmt::Loop(it.label, body));
        Ok(mk(ExprKind::Seq(stmts, Box::new(self.var_expr(m))), map_ty))
    }

    /// `key in obj` with a key known only at run time: a dictionary has it, or a struct has a
    /// field of that name (an optional one while it is set).
    pub(crate) fn lower_in_dynamic(
        &mut self,
        key: &'a Expression<'a>,
        obj: Expr,
        at: u32,
    ) -> R<Expr> {
        if let Ty::Map(k, _) = obj.ty.strip().clone() {
            let key = self.lower_expr(key, Some(&k))?;
            return self.map_has(obj, key);
        }
        let Ty::Obj(sid) = obj.ty.strip().clone() else {
            bail!("{}: `in` needs an object, found {}", self.loc(at), obj.ty)
        };
        let k = self.lower_expr(key, None)?;
        let k = self.coerce(k, &Ty::Str)?;
        let kv = self.new_local(Ty::Str);
        let ov = self.new_local(obj.ty.clone());
        let mut result = mk(ExprKind::Bool(false), Ty::Bool);
        for (name, _, i) in self.own_fields(sid).into_iter().rev() {
            let present = if self.absent_able(sid, i) {
                let fty = self.m.structs[sid].fields[i].ty.clone();
                let read = mk(ExprKind::Field(Box::new(self.var_expr(ov)), sid, i), fty);
                mk(
                    ExprKind::Un(
                        UnOp::Not,
                        Box::new(mk(ExprKind::IsNull(Box::new(read)), Ty::Bool)),
                    ),
                    Ty::Bool,
                )
            } else {
                mk(ExprKind::Bool(true), Ty::Bool)
            };
            let (kvar, lit) = (self.var_expr(kv), self.str_lit(&name));
            let is = self.lower_compare(CmpOp::Eq, kvar, lit, at)?;
            result = mk(
                ExprKind::Cond(Box::new(is), Box::new(present), Box::new(result)),
                Ty::Bool,
            );
        }
        Ok(mk(
            ExprKind::Seq(
                vec![Stmt::Assign(kv, k), Stmt::Assign(ov, obj)],
                Box::new(result),
            ),
            Ty::Bool,
        ))
    }

    /// `delete obj.key` / `delete obj[key]`: a dictionary loses the entry, an optional field of a
    /// struct becomes undefined; the result is `true`.
    pub(crate) fn lower_delete(&mut self, arg: &'a Expression<'a>, at: u32) -> R<Expr> {
        let mut arg = arg;
        while let Expression::ParenthesizedExpression(p) = arg {
            arg = &p.expression;
        }
        let (obj_e, key) = match arg {
            Expression::StaticMemberExpression(m) => {
                (&m.object, DeleteKey::Name(m.property.name.to_string()))
            }
            Expression::ComputedMemberExpression(m) => (&m.object, DeleteKey::Expr(&m.expression)),
            _ => bail!("{}: `delete` needs a property", self.loc(at)),
        };
        let obj = self.lower_expr(obj_e, None)?;
        match obj.ty.strip().clone() {
            Ty::Map(k, _) => {
                let key = match key {
                    DeleteKey::Name(n) => self.str_lit(&n),
                    DeleteKey::Expr(e) => self.lower_expr(e, Some(&k))?,
                };
                self.map_delete(obj, key)
            }
            Ty::Obj(sid) => {
                let tmp = self.new_local(obj.ty.clone());
                let mut stmts = vec![Stmt::Assign(tmp, obj)];
                let clear = |this: &mut Self, i: usize| {
                    let fty = this.m.structs[sid].fields[i].ty.clone();
                    Stmt::StoreField(this.var_expr(tmp), sid, i, mk(ExprKind::Null, fty))
                };
                match key {
                    DeleteKey::Name(n) => {
                        let Some(i) = self.m.structs[sid].field(&n) else {
                            bail!("{}: `delete {n}`: no such field", self.loc(at))
                        };
                        if !self.absent_able(sid, i) {
                            bail!(
                                "{}: `delete {n}`: only an optional property can be deleted (a required one is always there)",
                                self.loc(at)
                            )
                        }
                        stmts.push(clear(self, i));
                    }
                    DeleteKey::Expr(e) => {
                        let k = self.lower_expr(e, None)?;
                        let k = self.coerce(k, &Ty::Str)?;
                        let kv = self.new_local(Ty::Str);
                        stmts.push(Stmt::Assign(kv, k));
                        let mut any = false;
                        for (name, _, i) in self.own_fields(sid) {
                            if !self.absent_able(sid, i) {
                                continue;
                            }
                            any = true;
                            let (kvar, lit) = (self.var_expr(kv), self.str_lit(&name));
                            let is = self.lower_compare(CmpOp::Eq, kvar, lit, at)?;
                            let c = clear(self, i);
                            stmts.push(Stmt::If(is, vec![c], vec![]));
                        }
                        if !any {
                            bail!(
                                "{}: `delete` over an object with no optional properties",
                                self.loc(at)
                            )
                        }
                    }
                }
                Ok(mk(
                    ExprKind::Seq(stmts, Box::new(mk(ExprKind::Bool(true), Ty::Bool))),
                    Ty::Bool,
                ))
            }
            t => bail!("{}: `delete` on {t}", self.loc(at)),
        }
    }

    /// `for (const k in obj)`: the keys of an object or dictionary.
    pub(crate) fn lower_for_in(&mut self, f: &'a ForInStatement<'a>, out: &mut Vec<Stmt>) -> R<()> {
        let ForStatementLeft::VariableDeclaration(d) = &f.left else {
            bail!(
                "{}: `for (const k in obj)` is the supported form",
                self.loc(f.span.start)
            )
        };
        let pat = &d.declarations[0].id;
        let obj = self.lower_expr(&f.right, None)?;
        if !matches!(
            obj.ty.strip(),
            Ty::Obj(_) | Ty::Map(..) | Ty::Arr(_) | Ty::Typed(_)
        ) {
            bail!(
                "{}: `for ... in` over {}: only objects, dictionaries and arrays",
                self.loc(f.span.start),
                obj.ty
            )
        }
        // An array gives its indices, as strings (as JavaScript does).
        let over_array = matches!(obj.ty.strip(), Ty::Arr(_) | Ty::Typed(_));
        let (it, pre) = if over_array {
            (self.begin_iter(obj)?, Vec::new())
        } else {
            let (keys, _vals, pre) = self.object_parts(obj, "for-in", false)?;
            (self.begin_iter(keys)?, pre)
        };
        let mut index_key = Vec::new();
        let key_var = if over_array {
            let k = self.new_local(Ty::Str);
            let at = self.var_expr(it.idx);
            let text = self.rt_str("ts_rt_num_to_str_gc", vec![at], Ty::Str);
            index_key.push(Stmt::Assign(k, text));
            k
        } else {
            it.elem
        };
        self.cur.scopes.push(HashMap::new());
        let mut bind = Vec::new();
        let r = self.bind_param(pat, key_var, &mut bind);
        if let Err(e) = r {
            self.cur.scopes.pop();
            return Err(e);
        }
        let entry = self.loop_entry(it.label, Some(it.label));
        self.cur.loops.push(entry);
        let body = self.scoped(&f.body);
        self.cur.loops.pop();
        self.cur.scopes.pop();
        out.extend(pre);
        out.extend(it.pre);
        let mut b = it.head;
        b.extend(index_key);
        b.extend(bind);
        b.extend(body?);
        out.push(Stmt::Loop(it.label, b));
        Ok(())
    }
}
