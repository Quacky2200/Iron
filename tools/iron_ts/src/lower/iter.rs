//! Well-known symbols as member names and the iterator protocol for classes: `[Symbol.iterator]()`
//! is a method with the internal name `@@iterator`; `for...of`, spread and `Array.from` call it and
//! then `next()` until `done`. The result is an `IteratorResult<T>` object (`done`, `value`) of the
//! program's own types; built-in collections and arrays keep their dedicated loops.

use super::classes::Which;
use super::expr::mk;
use super::*;

/// The internal member name of a well-known symbol (`Symbol.iterator`).
pub(crate) fn well_known(e: &Expression<'_>) -> Option<&'static str> {
    let Expression::StaticMemberExpression(m) = e else {
        return None;
    };
    let Expression::Identifier(id) = &m.object else {
        return None;
    };
    if id.name != "Symbol" {
        return None;
    }
    Some(match m.property.name.as_str() {
        "iterator" => "@@iterator",
        "asyncIterator" => "@@asyncIterator",
        "toPrimitive" => "@@toPrimitive",
        "hasInstance" => "@@hasInstance",
        "toStringTag" => "@@toStringTag",
        _ => return None,
    })
}

impl<'a> Lowerer<'a> {
    /// A computed class member key: only well-known symbols are known at compile time.
    pub(crate) fn symbol_key(&self, k: &PropertyKey<'a>) -> R<String> {
        match k.as_expression().and_then(well_known) {
            Some(name) => Ok(name.to_string()),
            None => bail!(
                "a computed member name must be a well-known symbol such as [Symbol.iterator]"
            ),
        }
    }

    /// Does this type have an `[Symbol.iterator]()` method?
    pub(crate) fn is_iterable(&mut self, t: &Ty) -> bool {
        match t.strip() {
            Ty::Obj(sid) => self.target_for(*sid, Which::Method, "@@iterator").is_some(),
            _ => false,
        }
    }

    /// `recv.name()` for a method (or a closure-typed field) with no arguments.
    pub(crate) fn call_method0(&mut self, recv: Expr, name: &str) -> R<Expr> {
        let Ty::Obj(sid) = recv.ty.strip().clone() else {
            bail!("`{name}()` on {}", recv.ty)
        };
        if let Some((stat, target)) = self.target_for(sid, Which::Method, name) {
            return self.call_target(&stat, target, &[], Some(recv));
        }
        if let Some(ix) = self.m.structs[sid].field(name) {
            let fty = self.m.structs[sid].fields[ix].ty.clone();
            if let Ty::Fn(sig) = fty.strip() {
                let callee = mk(ExprKind::Field(Box::new(recv), sid, ix), fty.clone());
                return Ok(mk(
                    ExprKind::CallClosure(Box::new(callee), vec![]),
                    sig.ret.clone(),
                ));
            }
        }
        bail!("the object has no method `{name}`")
    }

    /// `for (const x of iterable)` where the iterable has `[Symbol.iterator]()`: an `@@iterator()`
    /// call, then `next()` until `done`.
    pub(crate) fn lower_for_of_iterable(
        &mut self,
        f: &'a ForOfStatement<'a>,
        pat: &'a BindingPattern<'a>,
        obj: Expr,
        out: &mut Vec<Stmt>,
    ) -> R<()> {
        let it = self.call_method0(obj, "@@iterator")?;
        let it_var = self.new_local(it.ty.clone());
        let label = self.new_label();
        let itv = self.var_expr(it_var);
        let res = self.call_method0(itv, "next")?;
        let Ty::Obj(rsid) = res.ty.strip().clone() else {
            bail!("next() must return an object with `done` and `value`")
        };
        let r_var = self.new_local(res.ty.clone());
        let rv = self.var_expr(r_var);
        let done = self.field_of(rv.clone(), rsid, "done");
        let value = self.field_of(rv, rsid, "value");
        let elem = self.new_local(value.ty.clone());
        let head = vec![
            Stmt::Assign(r_var, res),
            Stmt::If(done, vec![Stmt::Break(label)], vec![]),
            Stmt::Assign(elem, value),
        ];
        self.cur.scopes.push(HashMap::new());
        let mut bind = Vec::new();
        if let Err(e) = self.bind_param(pat, elem, &mut bind) {
            self.cur.scopes.pop();
            return Err(e);
        }
        let entry = self.loop_entry(label, Some(label));
        self.cur.loops.push(entry);
        let body = self.scoped(&f.body);
        self.cur.loops.pop();
        self.cur.scopes.pop();
        out.push(Stmt::Assign(it_var, it));
        let mut b = head;
        b.extend(bind);
        b.extend(body?);
        out.push(Stmt::Loop(label, b));
        Ok(())
    }

    /// The elements of an iterable (`[Symbol.iterator]()` then `next()`) collected into an array,
    /// for spread and `Array.from`.
    pub(crate) fn iterable_to_array(&mut self, obj: Expr) -> R<Expr> {
        let it = self.call_method0(obj, "@@iterator")?;
        let it_var = self.new_local(it.ty.clone());
        let label = self.new_label();
        let itv = self.var_expr(it_var);
        let res = self.call_method0(itv, "next")?;
        let Ty::Obj(rsid) = res.ty.strip().clone() else {
            bail!("next() must return an object with `done` and `value`")
        };
        let r_var = self.new_local(res.ty.clone());
        let rv = self.var_expr(r_var);
        let done = self.field_of(rv.clone(), rsid, "done");
        let value = self.field_of(rv, rsid, "value");
        let el_ty = value.ty.clone();
        let arr_ty = Ty::Arr(Box::new(el_ty));
        let out = self.new_local(arr_ty.clone());
        let ov = self.var_expr(out);
        let stmts = vec![
            Stmt::Assign(it_var, it),
            Stmt::Assign(
                out,
                mk(
                    ExprKind::Rt(RtFn::ArrNew, vec![super::expr::num(0.0)]),
                    arr_ty.clone(),
                ),
            ),
            Stmt::Loop(
                label,
                vec![
                    Stmt::Assign(r_var, res),
                    Stmt::If(done, vec![Stmt::Break(label)], vec![]),
                    Stmt::Eval(mk(
                        ExprKind::Rt(RtFn::ArrPush, vec![ov.clone(), value]),
                        Ty::Void,
                    )),
                ],
            ),
        ];
        Ok(mk(ExprKind::Seq(stmts, Box::new(ov)), arr_ty))
    }
}
