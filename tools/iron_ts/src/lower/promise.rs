//! Promises in the lowerer: routing `await`, `.then/.catch/.finally`, `Promise.*` and the value an
//! async function returns to the per-type helpers `asyncs.rs` writes. Only here are the types
//! known: `p.then(v => v + 1)` is `__then$number$number` because the callback returns a number.

use super::classes::Target;
use super::expr::mk;
use super::*;

impl<'a> Lowerer<'a> {
    /// The key and value type when `t` is a promise class.
    pub(crate) fn promise_of(&self, t: &Ty) -> Option<(String, Ty)> {
        let Ty::Obj(sid) = t.strip() else {
            return None;
        };
        let st = &self.m.structs[*sid];
        let key = st.name.strip_prefix("Promise$")?;
        let ix = st.field("value")?;
        Some((key.to_string(), st.fields[ix].ty.clone()))
    }

    /// The key of the promise class whose value type is `t`.
    pub(crate) fn promise_key_for(&self, t: &Ty) -> Option<String> {
        let t = if *t == Ty::Void { &Ty::Num } else { t };
        self.m.structs.iter().find_map(|st| {
            let key = st.name.strip_prefix("Promise$")?;
            let ix = st.field("value")?;
            (st.fields[ix].ty == *t).then(|| key.to_string())
        })
    }

    /// A call of a declared helper with arguments that are already lowered.
    pub(crate) fn call_helper(&mut self, name: &str, args: Vec<Expr>) -> R<Expr> {
        let Some(&fid) = self.fids.get(name) else {
            bail!(
                "`{name}` does not exist: the program has no promise type for it (write `Promise<T>` for the type somewhere), or does not use the feature it belongs to"
            )
        };
        self.ensure_lowered(name)?;
        let (params, ret) = self.fsig[fid].clone();
        let Some(ret) = ret else {
            bail!("{name}: return type unknown")
        };
        let mut vals = Vec::new();
        for (a, p) in args.into_iter().zip(&params) {
            vals.push(self.coerce(a, p)?);
        }
        Ok(Self::target_call(&Target::Direct(fid), vals, ret))
    }

    /// A callback argument lowered with its return type inferred from the body.
    fn lower_callback(&mut self, e: &'a Expression<'a>, params: Vec<Ty>) -> R<Expr> {
        let exp = Ty::Fn(Box::new(FnTy {
            params,
            ret: Ty::Void,
        }));
        self.infer_ret = true;
        let r = self.lower_expr(e, Some(&exp));
        self.infer_ret = false;
        r
    }

    fn error_ty(&self) -> Ty {
        Ty::Obj(self.classes_sid("Error"))
    }

    /// `p.then(f)`, `p.catch(f)`, `p.finally(f)`.
    pub(crate) fn lower_promise_method(
        &mut self,
        recv: Expr,
        key: &str,
        value: Ty,
        m: &str,
        c: &'a CallExpression<'a>,
    ) -> R<Expr> {
        let at = self.loc(c.span.start);
        let args: Vec<&'a Expression<'a>> = c
            .arguments
            .iter()
            .filter_map(|a| a.as_expression())
            .collect();
        if m == "then" && args.len() == 2 {
            return self.lower_then2(recv, key, value, args[0], args[1], &at);
        }
        let [cb] = args.as_slice() else {
            bail!("{at}: Promise.{m} takes one callback (then takes two)")
        };
        match m {
            "then" => {
                let f = self.lower_callback(cb, vec![value])?;
                let Ty::Fn(sig) = f.ty.strip().clone() else {
                    bail!("{at}: then needs a function")
                };
                if sig.ret == Ty::Void {
                    return self.call_helper(&format!("__thenV${key}"), vec![recv, f]);
                }
                if let Some((rk, _)) = self.promise_of(&sig.ret) {
                    return self.call_helper(&format!("__thenP${key}${rk}"), vec![recv, f]);
                }
                let Some(rk) = self.promise_key_for(&sig.ret) else {
                    bail!(
                        "{at}: then returns {} but the program has no `Promise<{}>`; write that type somewhere",
                        sig.ret,
                        sig.ret
                    )
                };
                self.call_helper(&format!("__then${key}${rk}"), vec![recv, f])
            }
            "catch" => {
                let f = self.lower_callback(cb, vec![self.error_ty()])?;
                let Ty::Fn(sig) = f.ty.strip().clone() else {
                    bail!("{at}: catch needs a function")
                };
                let name = if sig.ret == Ty::Void {
                    "__catchV"
                } else if self.promise_of(&sig.ret).is_some() {
                    "__catchP"
                } else {
                    "__catch"
                };
                self.call_helper(&format!("{name}${key}"), vec![recv, f])
            }
            "finally" => {
                let f = self.lower_callback(cb, vec![])?;
                self.call_helper(&format!("__finally${key}"), vec![recv, f])
            }
            _ => unreachable!("routed by name"),
        }
    }

    /// `p.then(f, g)`: both callbacks give the same kind of result.
    fn lower_then2(
        &mut self,
        recv: Expr,
        key: &str,
        value: Ty,
        f: &'a Expression<'a>,
        g: &'a Expression<'a>,
        at: &str,
    ) -> R<Expr> {
        let f = self.lower_callback(f, vec![value])?;
        let g = self.lower_callback(g, vec![self.error_ty()])?;
        let (Ty::Fn(fs), Ty::Fn(gs)) = (f.ty.strip().clone(), g.ty.strip().clone()) else {
            bail!("{at}: then needs functions")
        };
        if fs.ret == Ty::Void && gs.ret == Ty::Void {
            return self.call_helper(&format!("__then2V${key}"), vec![recv, f, g]);
        }
        if fs.ret == Ty::Void || gs.ret == Ty::Void {
            bail!(
                "{at}: the two callbacks of then(f, g) must both return a value or both return nothing"
            )
        }
        if let (Some((rk, _)), Some(_)) = (self.promise_of(&fs.ret), self.promise_of(&gs.ret)) {
            return self.call_helper(&format!("__then2P${key}${rk}"), vec![recv, f, g]);
        }
        let Some(rk) = self.promise_key_for(&fs.ret) else {
            bail!("{at}: no `Promise<{}>` in the program", fs.ret)
        };
        self.call_helper(&format!("__then2${key}${rk}"), vec![recv, f, g])
    }

    /// `Promise.resolve`, `Promise.reject`, `Promise.all`, `Promise.race`.
    pub(crate) fn lower_promise_static(&mut self, m: &str, c: &'a CallExpression<'a>) -> R<Expr> {
        let at = self.loc(c.span.start);
        let Some(arg) = c.arguments.first().and_then(|a| a.as_expression()) else {
            bail!("{at}: Promise.{m} needs an argument")
        };
        let v = self.lower_expr(arg, None)?;
        match m {
            "resolve" => {
                if self.promise_of(&v.ty).is_some() {
                    return Ok(v);
                }
                let Some(k) = self.promise_key_for(&v.ty) else {
                    bail!("{at}: no `Promise<{}>` in the program", v.ty)
                };
                self.call_helper(&format!("__resolved${k}"), vec![v])
            }
            "reject" => {
                let k = match c.type_arguments.as_ref().and_then(|a| a.params.first()) {
                    Some(t) => {
                        let t = self.ty(t, "gen")?;
                        self.promise_key_for(&t)
                            .ok_or_else(|| format!("{at}: no `Promise<{t}>` in the program"))?
                    }
                    None => "number".to_string(),
                };
                self.call_helper(&format!("__rejected${k}"), vec![v])
            }
            "all" | "race" | "any" | "allSettled" => {
                let Ty::Arr(el) = v.ty.strip().clone() else {
                    bail!("{at}: Promise.{m} takes an array of promises")
                };
                let Some((k, _)) = self.promise_of(&el) else {
                    bail!("{at}: Promise.{m} takes an array of promises")
                };
                self.call_helper(&format!("__{m}${k}"), vec![v])
            }
            _ => bail!("{at}: Promise.{m} is not supported"),
        }
    }

    /// `__await(e)`: wait for a promise (or give one tick to a plain value).
    pub(crate) fn lower_await(&mut self, c: &'a CallExpression<'a>) -> R<Expr> {
        let at = self.loc(c.span.start);
        let Some(arg) = c.arguments.first().and_then(|a| a.as_expression()) else {
            bail!("{at}: await needs an operand")
        };
        let v = self.lower_expr(arg, None)?;
        if let Some((k, _)) = self.promise_of(&v.ty) {
            return self.call_helper(&format!("__await${k}"), vec![v]);
        }
        let Some(k) = self.promise_key_for(&v.ty) else {
            bail!(
                "{at}: awaiting a {} needs a `Promise<{}>` somewhere in the program",
                v.ty,
                v.ty
            )
        };
        self.call_helper(&format!("__awaitV${k}"), vec![v])
    }

    /// `__asyncRet$K(__p, e)`: what `return e` does in an async function.
    pub(crate) fn lower_async_return(&mut self, key: &str, c: &'a CallExpression<'a>) -> R<Expr> {
        let at = self.loc(c.span.start);
        let args: Vec<&'a Expression<'a>> = c
            .arguments
            .iter()
            .filter_map(|a| a.as_expression())
            .collect();
        let [p, e] = args.as_slice() else {
            bail!("{at}: __asyncRet$K(promise, value)")
        };
        let p = self.lower_expr(p, None)?;
        let e = self.lower_expr(e, None)?;
        if e.ty == Ty::Void {
            let done = self.call_helper(&format!("__asyncRetN${key}"), vec![p])?;
            return Ok(mk(
                ExprKind::Seq(vec![Stmt::Eval(e)], Box::new(done)),
                Ty::Void,
            ));
        }
        if self.promise_of(&e.ty).is_some() {
            return self.call_helper(&format!("__asyncRetP${key}"), vec![p, e]);
        }
        self.call_helper(&format!("__asyncRetV${key}"), vec![p, e])
    }
}
