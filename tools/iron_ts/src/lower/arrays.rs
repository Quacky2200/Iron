//! Array methods lowered to loops, with callbacks inlined at the call site.
//!
//! `xs.filter((x) => p(x))` becomes an index loop whose body is the callback's body; `return`
//! inside it assigns the result and leaves the callback's block. No closure object is made and
//! callbacks may use the surrounding function's locals directly.

use super::expr::{mk, num};
use super::*;

pub(crate) struct Iter {
    pub pre: Vec<Stmt>,
    pub label: LabelId,
    /// Loop-top statements: leave at the end, bind `elem` and `idx`, step.
    pub head: Vec<Stmt>,
    pub arr: VarId,
    pub elem: VarId,
    pub idx: VarId,
    pub el_ty: Ty,
}

impl<'a> Lowerer<'a> {
    pub(crate) fn begin_iter(&mut self, arr: Expr) -> R<Iter> {
        self.begin_iter_dir(arr, false)
    }

    /// Like `begin_iter`; `rev` walks from the last element to the first (`findLast`).
    pub(crate) fn begin_iter_dir(&mut self, arr: Expr, rev: bool) -> R<Iter> {
        let el_ty = match arr.ty.strip() {
            Ty::Arr(e) => (**e).clone(),
            Ty::Typed(_) => Ty::Num,
            t => bail!("cannot iterate {t}"),
        };
        let a = self.new_local(arr.ty.clone());
        let i = self.new_local(Ty::Num);
        let elem = self.new_local(el_ty.clone());
        let idx = self.new_local(Ty::Num);
        let label = self.new_label();
        let (va, vi) = (self.var_expr(a), self.var_expr(i));
        let len = mk(ExprKind::Len(Box::new(va.clone())), Ty::Num);
        let done = if rev {
            mk(
                ExprKind::Cmp(CmpOp::Lt, Box::new(vi.clone()), Box::new(num(0.0))),
                Ty::Bool,
            )
        } else {
            mk(
                ExprKind::Cmp(CmpOp::Ge, Box::new(vi.clone()), Box::new(len.clone())),
                Ty::Bool,
            )
        };
        let raw_ty = match &el_ty {
            e if !matches!(arr.ty.strip(), Ty::Typed(_)) && (e.is_ptr() || *e == Ty::Num) => {
                e.clone().nullable()
            }
            e => e.clone(),
        };
        let raw = mk(ExprKind::Index(Box::new(va), Box::new(vi.clone())), raw_ty);
        let get = if raw.ty == el_ty {
            raw
        } else {
            mk(ExprKind::Cast(Box::new(raw)), el_ty.clone())
        };
        let step = mk(
            ExprKind::Bin(
                if rev { BinOp::Sub } else { BinOp::Add },
                Box::new(vi.clone()),
                Box::new(num(1.0)),
            ),
            Ty::Num,
        );
        let head = vec![
            Stmt::If(done, vec![Stmt::Break(label)], vec![]),
            Stmt::Assign(elem, get),
            Stmt::Assign(idx, vi),
            Stmt::Assign(i, step),
        ];
        let first = if rev {
            mk(
                ExprKind::Bin(BinOp::Sub, Box::new(len), Box::new(num(1.0))),
                Ty::Num,
            )
        } else {
            num(0.0)
        };
        let pre = vec![Stmt::Assign(a, arr), Stmt::Assign(i, first)];
        Ok(Iter {
            pre,
            label,
            head,
            arr: a,
            elem,
            idx,
            el_ty,
        })
    }

    /// Emits a `return` for the innermost function or inlined callback.
    pub(crate) fn emit_return(&mut self, val: Option<Expr>, out: &mut Vec<Stmt>) -> R<()> {
        if self.fin_applies(&Exit::Return) {
            return self.leave_through_finally(Exit::Return, val, out);
        }
        if let Some(rt) = self.cur.rets.last().cloned() {
            match (rt.kind, val) {
                (RetKind::Pred, Some(v)) => {
                    let b = self.to_bool(v);
                    out.push(Stmt::Assign(rt.var.unwrap(), b));
                }
                (RetKind::Val, Some(v)) => {
                    let var = rt.var.unwrap();
                    if self.cur.locals[var as usize] == Ty::Void {
                        self.cur.locals[var as usize] = v.ty.clone();
                    }
                    let t = self.cur.locals[var as usize].clone();
                    let v = self.coerce(v, &t)?;
                    out.push(Stmt::Assign(var, v));
                }
                (_, Some(v)) => out.push(Stmt::Eval(v)),
                (_, None) => {}
            }
            out.push(Stmt::Break(rt.label));
            return Ok(());
        }
        match val {
            Some(v) => {
                let v = match self.cur.ret.clone() {
                    Some(t) => self.coerce(v, &t)?,
                    None => {
                        self.cur.ret = Some(v.ty.clone());
                        v
                    }
                };
                out.push(Stmt::Return(Some(v)));
            }
            None => {
                if self.cur.ret.is_none() {
                    self.cur.ret = Some(Ty::Void);
                }
                out.push(Stmt::Return(None));
            }
        }
        Ok(())
    }

    /// Inlines an arrow function as a block; `params` are the locals its parameters bind to.
    pub(crate) fn inline_cb(
        &mut self,
        cb: &'a Expression<'a>,
        params: &[VarId],
        kind: RetKind,
        res: Option<VarId>,
    ) -> R<Stmt> {
        let Expression::ArrowFunctionExpression(f) = cb else {
            return self.closure_callback(cb, params, kind, res);
        };
        self.cur.scopes.push(HashMap::new());
        let mut body = Vec::new();
        for (p, var) in f.params.items.iter().zip(params) {
            self.bind_param(&p.pattern, *var, &mut body)?;
        }
        let label = self.new_label();
        let saved_loops = std::mem::take(&mut self.cur.loops);
        // A `try ... finally` around the call is not left by a `break` inside the callback.
        let saved_fins = std::mem::take(&mut self.cur.fins);
        self.cur.rets.push(RetTarget {
            label,
            var: res,
            kind,
        });
        let r = match &f.body {
            ArrowFunctionBody::FunctionBody(b) => self.lower_stmts(&b.statements, &mut body),
            other => match other.as_expression() {
                Some(e) => self
                    .lower_expr(e, None)
                    .and_then(|v| self.emit_return(Some(v), &mut body)),
                None => Err("arrow body".to_string()),
            },
        };
        self.cur.rets.pop();
        self.cur.loops = saved_loops;
        self.cur.fins = saved_fins;
        self.cur.scopes.pop();
        r?;
        Ok(Stmt::Block(label, body))
    }

    /// A callback that is not an arrow function in the call (`xs.map(f)`): called as a closure.
    fn closure_callback(
        &mut self,
        cb: &'a Expression<'a>,
        params: &[VarId],
        kind: RetKind,
        res: Option<VarId>,
    ) -> R<Stmt> {
        let callee = self.lower_expr(cb, None)?;
        let Ty::Fn(sig) = callee.ty.strip().clone() else {
            bail!("an array method callback must be a function")
        };
        let mut args = Vec::new();
        for (p, t) in params.iter().zip(&sig.params) {
            let a = self.var_expr(*p);
            args.push(self.coerce(a, t)?);
        }
        let call = mk(
            ExprKind::CallClosure(Box::new(callee), args),
            sig.ret.clone(),
        );
        let label = self.new_label();
        self.cur.rets.push(RetTarget {
            label,
            var: res,
            kind,
        });
        let mut body = Vec::new();
        let r = if sig.ret == Ty::Void {
            body.push(Stmt::Eval(call));
            self.emit_return(None, &mut body)
        } else {
            self.emit_return(Some(call), &mut body)
        };
        self.cur.rets.pop();
        r?;
        Ok(Stmt::Block(label, body))
    }

    /// `Array.isArray`, `Array.from` and `Array.of`.
    pub(crate) fn lower_array_static(&mut self, f: &str, c: &'a CallExpression<'a>) -> R<Expr> {
        let arg = |i: usize| c.arguments.get(i).and_then(|a| a.as_expression());
        match f {
            "isArray" => {
                let e = arg(0).ok_or("Array.isArray needs an argument")?;
                let v = self.lower_expr(e, None)?;
                let is = matches!(v.ty.strip(), Ty::Arr(_) | Ty::Typed(_));
                Ok(mk(
                    ExprKind::Seq(
                        vec![Stmt::Eval(v)],
                        Box::new(mk(ExprKind::Bool(is), Ty::Bool)),
                    ),
                    Ty::Bool,
                ))
            }
            "of" => {
                let mut elems = Vec::new();
                let mut el = None;
                for a in &c.arguments {
                    let e = a.as_expression().ok_or("Array.of: spread")?;
                    let v = self.lower_expr(e, el.as_ref())?;
                    el.get_or_insert_with(|| v.ty.clone());
                    elems.push(super::expr::Elem::One(v));
                }
                self.build_array(elems, el.unwrap_or(Ty::Num))
            }
            "from" => {
                let a0 = arg(0).ok_or("Array.from needs an argument")?;
                // Array.from({ length: n }, (_, i) => value)
                if let Expression::ObjectExpression(o) = a0 {
                    let n_expr = o
                        .properties
                        .iter()
                        .find_map(|p| match p {
                            ObjectPropertyKind::ObjectProperty(p)
                                if matches!(&p.key, PropertyKey::StaticIdentifier(k) if k.name == "length") =>
                            {
                                Some(&p.value)
                            }
                            _ => None,
                        })
                        .ok_or("Array.from needs { length }")?;
                    let cb = arg(1).ok_or("Array.from({ length }) needs a function")?;
                    let n = self.lower_expr(n_expr, Some(&Ty::Num))?;
                    let n = self.as_num(n)?;
                    let (nv, iv, unused) = (
                        self.new_local(Ty::Num),
                        self.new_local(Ty::Num),
                        self.new_local(Ty::Num),
                    );
                    let r = self.new_local(Ty::Void);
                    let label = self.new_label();
                    let cbs = self.inline_cb(cb, &[unused, iv], RetKind::Val, Some(r))?;
                    let rty = self.cur.locals[r as usize].clone();
                    let ty = Ty::Arr(Box::new(rty));
                    let out = self.new_local(ty.clone());
                    let (nvv, ivv, ov) = (self.var_expr(nv), self.var_expr(iv), self.var_expr(out));
                    let done = mk(
                        ExprKind::Cmp(CmpOp::Ge, Box::new(ivv.clone()), Box::new(nvv.clone())),
                        Ty::Bool,
                    );
                    let next = mk(
                        ExprKind::Bin(BinOp::Add, Box::new(ivv), Box::new(num(1.0))),
                        Ty::Num,
                    );
                    let body = vec![
                        Stmt::If(done, vec![Stmt::Break(label)], vec![]),
                        cbs,
                        Stmt::Eval(mk(
                            ExprKind::Rt(RtFn::ArrPush, vec![ov.clone(), self.var_expr(r)]),
                            Ty::Void,
                        )),
                        Stmt::Assign(iv, next),
                    ];
                    let stmts = vec![
                        Stmt::Assign(nv, n),
                        Stmt::Assign(iv, num(0.0)),
                        Stmt::Assign(out, mk(ExprKind::Rt(RtFn::ArrNew, vec![nvv]), ty.clone())),
                        Stmt::Loop(label, body),
                    ];
                    return Ok(mk(ExprKind::Seq(stmts, Box::new(ov)), ty));
                }
                if c.arguments.len() > 1 {
                    bail!("Array.from(xs, fn): use xs.map(fn)")
                }
                let v = self.lower_expr(a0, None)?;
                // A set, or the keys or values of a map, as an array now.
                if let Some(snapshot) = self.collection_snapshot(v.clone()) {
                    return Ok(snapshot);
                }
                let v = if self.is_iterable(&v.ty) {
                    self.iterable_to_array(v)?
                } else {
                    v
                };
                if !matches!(v.ty.strip(), Ty::Arr(_)) {
                    bail!("Array.from of {} is not supported", v.ty)
                }
                let ty = v.ty.clone();
                Ok(mk(
                    ExprKind::Rt(RtFn::ArrSlice, vec![v, num(0.0), num(f64::INFINITY)]),
                    ty,
                ))
            }
            _ => bail!("Array.{f} is not supported"),
        }
    }

    /// An optional number argument of a call, or `default` when it is missing.
    fn num_arg_or(&mut self, c: &'a CallExpression<'a>, i: usize, default: f64) -> R<Expr> {
        match c.arguments.get(i).and_then(|a| a.as_expression()) {
            Some(e) => {
                let v = self.lower_expr(e, Some(&Ty::Num))?;
                self.as_num(v)
            }
            None => Ok(num(default)),
        }
    }

    /// The type of an element read that may be past the end: null for pointers and numbers.
    fn nullable_elem(&self, el: &Ty, method: &str) -> R<Ty> {
        if el.is_ptr() || *el == Ty::Num {
            Ok(el.clone().nullable())
        } else {
            bail!("`{method}` over {el} elements is not supported")
        }
    }

    pub(crate) fn lower_method(
        &mut self,
        recv: Expr,
        m: &str,
        c: &'a CallExpression<'a>,
    ) -> R<Expr> {
        let rty = recv.ty.strip().clone();
        if let Ty::Obj(sid) = &rty {
            let sid = *sid;
            if matches!(m, "then" | "catch" | "finally")
                && let Some((key, value)) = self.promise_of(&rty)
            {
                return self.lower_promise_method(recv, &key, value, m, c);
            }
            if Some(sid) == self.regexp_sid() {
                return self.lower_regexp_method(recv, m, c);
            }
            if Some(sid) == self.date_sid() {
                return self.lower_date_method(recv, m, c);
            }
            // A match object is also an array of strings.
            if Some(sid) == self.match_sid() && !matches!(m, "toString") {
                let view = self.match_view(recv);
                return self.lower_method(view, m, c);
            }
            if let Some(f) = self.value_member(sid, m, false) {
                return self.call_ext_fn(f, Some(recv), m, c);
            }
            if let Some((stat, target)) = self.target_for(sid, Which::Method, m) {
                return self.call_target(&stat, target, &c.arguments, Some(recv));
            }
            // A method only a subclass has: tsc narrowed the receiver, so call it as that class.
            let sub = self.downcast_for(sid, |s, c| s.resolve_in(c, Which::Method, m).is_some());
            if let Some(sub) = sub {
                return self.lower_method(Self::downcast(recv, sub), m, c);
            }
        }
        if let Ty::Typed(kind) = &rty {
            return self.lower_typed_method(recv, *kind, m, c);
        }
        if rty == Ty::BigInt {
            return self.lower_bigint_method(recv, m, c);
        }
        if rty == Ty::Str {
            return self.lower_str_method(recv, m, c);
        }
        if matches!(rty, Ty::Map(..) | Ty::Set(_)) {
            return self.lower_collection_method(recv, m, c);
        }
        if rty == Ty::Num {
            return self.lower_num_method(recv, m, c);
        }
        if let Ty::Arr(el) = &rty
            && (**el == Ty::Str
                || **el == Ty::Str.nullable()
                || **el == Ty::Num
                || **el == Ty::Bool)
            && m == "join"
        {
            return self.lower_join(recv, c);
        }
        if let Ty::Handle(class) = &rty {
            let f = self
                .manifest
                .classes
                .get(class)
                .and_then(|k| k.methods.get(m))
                .cloned();
            let Some(f) = f else {
                bail!("{class} has no method `{m}`")
            };
            return self.call_ext_fn(f, Some(recv), &format!("{class}.{m}"), c);
        }
        let void = |stmts| {
            mk(
                ExprKind::Seq(stmts, Box::new(mk(ExprKind::Num(0.0), Ty::Void))),
                Ty::Void,
            )
        };
        let cb_arg = |i: usize| -> R<&'a Expression<'a>> {
            c.arguments
                .get(i)
                .and_then(|a| a.as_expression())
                .ok_or_else(|| format!("{m}: missing callback"))
        };
        match (&rty, m) {
            (Ty::Arr(el), "push") => {
                let a = self.new_local(recv.ty.clone());
                let mut stmts = vec![Stmt::Assign(a, recv)];
                for arg in &c.arguments {
                    let Some(e) = arg.as_expression() else {
                        bail!("push: spread")
                    };
                    let v = self.lower_expr(e, Some(el))?;
                    let v = self.coerce(v, el)?;
                    stmts.push(Stmt::Eval(mk(
                        ExprKind::Rt(RtFn::ArrPush, vec![self.var_expr(a), v]),
                        Ty::Void,
                    )));
                }
                Ok(void(stmts))
            }
            (
                Ty::Arr(_) | Ty::Typed(_),
                "forEach" | "filter" | "find" | "findLast" | "some" | "every" | "map",
            ) => {
                let cb = cb_arg(0)?;
                let it = self.begin_iter_dir(recv, m == "findLast")?;
                let el = it.el_ty.clone();
                let (ev, iv) = (it.elem, it.idx);
                let mut stmts = it.pre.clone();
                let mut body = it.head.clone();
                let label = it.label;
                let params = [ev, iv, it.arr];
                let result = match m {
                    "forEach" => {
                        body.push(self.inline_cb(cb, &params, RetKind::Void, None)?);
                        stmts.push(Stmt::Loop(label, body));
                        void(stmts)
                    }
                    "filter" | "find" | "findLast" | "some" | "every" => {
                        let r = self.new_local(Ty::Bool);
                        body.push(self.inline_cb(cb, &params, RetKind::Pred, Some(r))?);
                        let rv = self.var_expr(r);
                        let e = self.var_expr(ev);
                        match m {
                            "filter" => {
                                let ty = Ty::Arr(Box::new(el));
                                let out = self.new_local(ty.clone());
                                let ov = self.var_expr(out);
                                stmts.push(Stmt::Assign(
                                    out,
                                    mk(ExprKind::Rt(RtFn::ArrNew, vec![num(0.0)]), ty.clone()),
                                ));
                                body.push(Stmt::If(
                                    rv,
                                    vec![Stmt::Eval(mk(
                                        ExprKind::Rt(RtFn::ArrPush, vec![ov.clone(), e]),
                                        Ty::Void,
                                    ))],
                                    vec![],
                                ));
                                stmts.push(Stmt::Loop(label, body));
                                mk(ExprKind::Seq(stmts, Box::new(ov)), ty)
                            }
                            "find" | "findLast" => {
                                if !(el.is_ptr() || el == Ty::Num) {
                                    bail!("find over {el} elements");
                                }
                                let ty = el.clone().nullable();
                                let out = self.new_local(ty.clone());
                                stmts.push(Stmt::Assign(out, mk(ExprKind::Null, ty.clone())));
                                let hit = mk(ExprKind::Cast(Box::new(e)), ty.clone());
                                body.push(Stmt::If(
                                    rv,
                                    vec![Stmt::Assign(out, hit), Stmt::Break(label)],
                                    vec![],
                                ));
                                stmts.push(Stmt::Loop(label, body));
                                mk(ExprKind::Seq(stmts, Box::new(self.var_expr(out))), ty)
                            }
                            _ => {
                                let some = m == "some";
                                let out = self.new_local(Ty::Bool);
                                stmts.push(Stmt::Assign(out, mk(ExprKind::Bool(!some), Ty::Bool)));
                                let cond = if some {
                                    rv
                                } else {
                                    mk(ExprKind::Un(UnOp::Not, Box::new(rv)), Ty::Bool)
                                };
                                body.push(Stmt::If(
                                    cond,
                                    vec![
                                        Stmt::Assign(out, mk(ExprKind::Bool(some), Ty::Bool)),
                                        Stmt::Break(label),
                                    ],
                                    vec![],
                                ));
                                stmts.push(Stmt::Loop(label, body));
                                mk(ExprKind::Seq(stmts, Box::new(self.var_expr(out))), Ty::Bool)
                            }
                        }
                    }
                    _ => {
                        // map: the element type is whatever the callback returns.
                        let r = self.new_local(Ty::Void);
                        body.push(self.inline_cb(cb, &params, RetKind::Val, Some(r))?);
                        let rty = self.cur.locals[r as usize].clone();
                        // A callback that returns string literals (`v ? "1" : "0"`) makes strings.
                        let (rty, pushed) = if rty == Ty::Lit {
                            let v = self.var_expr(r);
                            (Ty::Str, self.widen_lit(v))
                        } else {
                            (rty, self.var_expr(r))
                        };
                        let ty = Ty::Arr(Box::new(rty));
                        let out = self.new_local(ty.clone());
                        let ov = self.var_expr(out);
                        stmts.push(Stmt::Assign(
                            out,
                            mk(ExprKind::Rt(RtFn::ArrNew, vec![num(0.0)]), ty.clone()),
                        ));
                        body.push(Stmt::Eval(mk(
                            ExprKind::Rt(RtFn::ArrPush, vec![ov.clone(), pushed]),
                            Ty::Void,
                        )));
                        stmts.push(Stmt::Loop(label, body));
                        mk(ExprKind::Seq(stmts, Box::new(ov)), ty)
                    }
                };
                Ok(result)
            }
            (Ty::Arr(el), "pop" | "shift") => {
                let rty = self.nullable_elem(el, m)?;
                let a = self.new_local(recv.ty.clone());
                let r = self.new_local(rty.clone());
                let av = self.var_expr(a);
                let len = mk(ExprKind::Len(Box::new(av.clone())), Ty::Num);
                let at = if m == "pop" {
                    mk(
                        ExprKind::Bin(BinOp::Sub, Box::new(len.clone()), Box::new(num(1.0))),
                        Ty::Num,
                    )
                } else {
                    num(0.0)
                };
                let get = mk(
                    ExprKind::Index(Box::new(av.clone()), Box::new(at.clone())),
                    rty.clone(),
                );
                let shrink = if m == "pop" {
                    ExprKind::Rt(RtFn::ArrSetLen, vec![av, at])
                } else {
                    ExprKind::Rt(RtFn::ArrDrop, vec![av, num(0.0), num(1.0)])
                };
                let nonempty = mk(
                    ExprKind::Cmp(CmpOp::Gt, Box::new(len), Box::new(num(0.0))),
                    Ty::Bool,
                );
                let stmts = vec![
                    Stmt::Assign(a, recv),
                    Stmt::Assign(r, get),
                    Stmt::If(nonempty, vec![Stmt::Eval(mk(shrink, Ty::Void))], vec![]),
                ];
                Ok(mk(ExprKind::Seq(stmts, Box::new(self.var_expr(r))), rty))
            }
            (Ty::Arr(el), "unshift") => {
                let a = self.new_local(recv.ty.clone());
                let mut stmts = vec![Stmt::Assign(a, recv)];
                for (k, arg) in c.arguments.iter().enumerate() {
                    let Some(e) = arg.as_expression() else {
                        bail!("unshift: spread")
                    };
                    let v = self.lower_expr(e, Some(el))?;
                    let v = self.coerce(v, el)?;
                    stmts.push(Stmt::Eval(mk(
                        ExprKind::Rt(RtFn::ArrInsert, vec![self.var_expr(a), num(k as f64), v]),
                        Ty::Void,
                    )));
                }
                let len = mk(ExprKind::Len(Box::new(self.var_expr(a))), Ty::Num);
                Ok(mk(ExprKind::Seq(stmts, Box::new(len)), Ty::Num))
            }
            (Ty::Arr(_), "slice") => {
                let start = self.num_arg_or(c, 0, 0.0)?;
                let end = self.num_arg_or(c, 1, f64::INFINITY)?;
                Ok(mk(
                    ExprKind::Rt(RtFn::ArrSlice, vec![recv.clone(), start, end]),
                    recv.ty.clone(),
                ))
            }
            (Ty::Arr(el), "splice") => {
                if c.arguments.is_empty() {
                    bail!("splice needs a start index")
                }
                let ty = recv.ty.clone();
                let start = self.num_arg_or(c, 0, 0.0)?;
                let count = self.num_arg_or(c, 1, f64::INFINITY)?;
                let mut elems = Vec::new();
                for arg in c.arguments.iter().skip(2) {
                    let Some(e) = arg.as_expression() else {
                        bail!("splice: spread")
                    };
                    let v = self.lower_expr(e, Some(el))?;
                    elems.push(super::expr::Elem::One(v));
                }
                let items = if elems.is_empty() {
                    mk(ExprKind::Null, ty.clone().nullable())
                } else {
                    self.build_array(elems, (**el).clone())?
                };
                Ok(mk(
                    ExprKind::Rt(RtFn::ArrSplice, vec![recv, start, count, items]),
                    ty,
                ))
            }
            (Ty::Arr(el), "concat") => {
                let ty = recv.ty.clone();
                let out = self.new_local(ty.clone());
                let ov = self.var_expr(out);
                let mut stmts = vec![Stmt::Assign(
                    out,
                    mk(ExprKind::Rt(RtFn::ArrNew, vec![num(0.0)]), ty.clone()),
                )];
                let first = self.new_local(ty.clone());
                stmts.push(Stmt::Assign(first, recv));
                stmts.push(Stmt::Eval(mk(
                    ExprKind::Rt(RtFn::ArrAppend, vec![ov.clone(), self.var_expr(first)]),
                    Ty::Void,
                )));
                for arg in &c.arguments {
                    let Some(e) = arg.as_expression() else {
                        bail!("concat: spread")
                    };
                    let v = self.lower_expr(e, Some(&ty))?;
                    if matches!(v.ty.strip(), Ty::Arr(_)) {
                        let t = self.new_local(v.ty.clone());
                        stmts.push(Stmt::Assign(t, v));
                        stmts.push(Stmt::Eval(mk(
                            ExprKind::Rt(RtFn::ArrAppend, vec![ov.clone(), self.var_expr(t)]),
                            Ty::Void,
                        )));
                    } else {
                        let v = self.coerce(v, el)?;
                        stmts.push(Stmt::Eval(mk(
                            ExprKind::Rt(RtFn::ArrPush, vec![ov.clone(), v]),
                            Ty::Void,
                        )));
                    }
                }
                Ok(mk(ExprKind::Seq(stmts, Box::new(ov)), ty))
            }
            (Ty::Arr(el), "reverse" | "sort" | "fill") => {
                let ty = recv.ty.clone();
                let a = self.new_local(ty.clone());
                let av = self.var_expr(a);
                let mut stmts = vec![Stmt::Assign(a, recv)];
                let call = match m {
                    "reverse" => ExprKind::Rt(RtFn::ArrReverse, vec![av.clone()]),
                    "sort" => {
                        if !(el.is_ptr() || **el == Ty::Num) {
                            bail!("sort over {el} elements")
                        }
                        match c.arguments.first().and_then(|a| a.as_expression()) {
                            // No comparator: by the string form of each element.
                            None => {
                                let kind = match **el {
                                    Ty::Num => 0.0,
                                    Ty::Str => 1.0,
                                    _ => bail!(
                                        "{}: sort() without a comparator over {el} elements is not supported; pass (a, b) => ...",
                                        self.loc(c.span.start)
                                    ),
                                };
                                ExprKind::Rt(
                                    RtFn::Str("ts_rt_arr_sort_default"),
                                    vec![av.clone(), num(kind)],
                                )
                            }
                            Some(cmp) => {
                                let fty = Ty::Fn(Box::new(FnTy {
                                    params: vec![(**el).clone(), (**el).clone()],
                                    ret: Ty::Num,
                                }));
                                let cmp = self.lower_expr(cmp, Some(&fty))?;
                                let cmp = self.coerce(cmp, &fty)?;
                                ExprKind::Rt(RtFn::ArrSort, vec![av.clone(), cmp])
                            }
                        }
                    }
                    _ => {
                        let v = c
                            .arguments
                            .first()
                            .and_then(|a| a.as_expression())
                            .ok_or("fill needs a value")?;
                        let v = self.lower_expr(v, Some(el))?;
                        let v = self.coerce(v, el)?;
                        let start = self.num_arg_or(c, 1, 0.0)?;
                        let end = self.num_arg_or(c, 2, f64::INFINITY)?;
                        ExprKind::Rt(RtFn::ArrFill, vec![av.clone(), v, start, end])
                    }
                };
                stmts.push(Stmt::Eval(mk(call, Ty::Void)));
                Ok(mk(ExprKind::Seq(stmts, Box::new(av)), ty))
            }
            // The copying forms: a copy, then the in-place method on it.
            (Ty::Arr(_), "toReversed" | "toSorted") => {
                let copy = mk(
                    ExprKind::Rt(
                        RtFn::ArrSlice,
                        vec![recv.clone(), num(0.0), num(f64::INFINITY)],
                    ),
                    recv.ty.clone(),
                );
                self.lower_method(copy, if m == "toSorted" { "sort" } else { "reverse" }, c)
            }
            // `xss.flat()`: one level, the arrays' elements in order.
            (Ty::Arr(el), "flat") => {
                let Ty::Arr(_) = el.strip() else {
                    bail!("flat: the elements are {el}, not arrays")
                };
                if let Some(d) = c.arguments.first() {
                    let ok = matches!(d.as_expression(), Some(Expression::NumericLiteral(n)) if n.value == 1.0);
                    if !ok {
                        bail!("flat: only the default depth (1) is supported")
                    }
                }
                let ty = (**el).clone();
                let it = self.begin_iter(recv)?;
                let out = self.new_local(ty.clone());
                let ov = self.var_expr(out);
                let mut stmts = it.pre.clone();
                stmts.push(Stmt::Assign(
                    out,
                    mk(ExprKind::Rt(RtFn::ArrNew, vec![num(0.0)]), ty.clone()),
                ));
                let mut body = it.head.clone();
                body.push(Stmt::Eval(mk(
                    ExprKind::Rt(RtFn::ArrAppend, vec![ov.clone(), self.var_expr(it.elem)]),
                    Ty::Void,
                )));
                stmts.push(Stmt::Loop(it.label, body));
                Ok(mk(ExprKind::Seq(stmts, Box::new(ov)), ty))
            }
            // `xs.flatMap(f)`: `f` returns an array for each element.
            (Ty::Arr(_), "flatMap") => {
                let cb = cb_arg(0)?;
                let it = self.begin_iter(recv)?;
                let r = self.new_local(Ty::Void);
                let params = [it.elem, it.idx, it.arr];
                let mut body = it.head.clone();
                body.push(self.inline_cb(cb, &params, RetKind::Val, Some(r))?);
                let ty = self.cur.locals[r as usize].clone();
                if !matches!(ty, Ty::Arr(_)) {
                    bail!("flatMap: the callback must return an array, found {ty}")
                }
                let out = self.new_local(ty.clone());
                let ov = self.var_expr(out);
                let mut stmts = it.pre.clone();
                stmts.push(Stmt::Assign(
                    out,
                    mk(ExprKind::Rt(RtFn::ArrNew, vec![num(0.0)]), ty.clone()),
                ));
                body.push(Stmt::Eval(mk(
                    ExprKind::Rt(RtFn::ArrAppend, vec![ov.clone(), self.var_expr(r)]),
                    Ty::Void,
                )));
                stmts.push(Stmt::Loop(it.label, body));
                Ok(mk(ExprKind::Seq(stmts, Box::new(ov)), ty))
            }
            (Ty::Arr(el), "at") => {
                let rty = self.nullable_elem(el, m)?;
                let i = self.num_arg_or(c, 0, 0.0)?;
                let a = self.new_local(recv.ty.clone());
                let iv = self.new_local(Ty::Num);
                let (av, ivv) = (self.var_expr(a), self.var_expr(iv));
                let neg = mk(
                    ExprKind::Cmp(CmpOp::Lt, Box::new(ivv.clone()), Box::new(num(0.0))),
                    Ty::Bool,
                );
                let wrapped = mk(
                    ExprKind::Bin(
                        BinOp::Add,
                        Box::new(mk(ExprKind::Len(Box::new(av.clone())), Ty::Num)),
                        Box::new(ivv.clone()),
                    ),
                    Ty::Num,
                );
                let at = mk(
                    ExprKind::Cond(Box::new(neg), Box::new(wrapped), Box::new(ivv)),
                    Ty::Num,
                );
                let get = mk(ExprKind::Index(Box::new(av), Box::new(at)), rty.clone());
                Ok(mk(
                    ExprKind::Seq(
                        vec![Stmt::Assign(a, recv), Stmt::Assign(iv, i)],
                        Box::new(get),
                    ),
                    rty,
                ))
            }
            (Ty::Arr(el), "indexOf" | "lastIndexOf") => {
                let Some(x) = c.arguments.first().and_then(|a| a.as_expression()) else {
                    bail!("{m}: missing argument")
                };
                let x = self.lower_expr(x, Some(el))?;
                let x = self.coerce(x, el)?;
                let xv = self.new_local(el.as_ref().clone());
                let it = self.begin_iter(recv)?;
                let out = self.new_local(Ty::Num);
                let mut stmts = it.pre.clone();
                stmts.push(Stmt::Assign(xv, x));
                stmts.push(Stmt::Assign(out, num(-1.0)));
                let mut body = it.head.clone();
                let eq = mk(
                    ExprKind::Cmp(
                        CmpOp::Eq,
                        Box::new(self.var_expr(it.elem)),
                        Box::new(self.var_expr(xv)),
                    ),
                    Ty::Bool,
                );
                let mut hit = vec![Stmt::Assign(out, self.var_expr(it.idx))];
                if m == "indexOf" {
                    hit.push(Stmt::Break(it.label));
                }
                body.push(Stmt::If(eq, hit, vec![]));
                stmts.push(Stmt::Loop(it.label, body));
                Ok(mk(
                    ExprKind::Seq(stmts, Box::new(self.var_expr(out))),
                    Ty::Num,
                ))
            }
            (Ty::Arr(_), "findIndex" | "findLastIndex") => {
                let cb = cb_arg(0)?;
                let it = self.begin_iter_dir(recv, m == "findLastIndex")?;
                let (ev, iv) = (it.elem, it.idx);
                let out = self.new_local(Ty::Num);
                let r = self.new_local(Ty::Bool);
                let mut stmts = it.pre.clone();
                stmts.push(Stmt::Assign(out, num(-1.0)));
                let mut body = it.head.clone();
                body.push(self.inline_cb(cb, &[ev, iv, it.arr], RetKind::Pred, Some(r))?);
                body.push(Stmt::If(
                    self.var_expr(r),
                    vec![Stmt::Assign(out, self.var_expr(iv)), Stmt::Break(it.label)],
                    vec![],
                ));
                stmts.push(Stmt::Loop(it.label, body));
                Ok(mk(
                    ExprKind::Seq(stmts, Box::new(self.var_expr(out))),
                    Ty::Num,
                ))
            }
            (Ty::Arr(_) | Ty::Typed(_), "reduce" | "reduceRight") => {
                let cb = cb_arg(0)?;
                let Some(init) = c.arguments.get(1).and_then(|a| a.as_expression()) else {
                    bail!("reduce needs an initial value")
                };
                let init = self.lower_expr(init, None)?;
                if init.ty == Ty::Null {
                    bail!("reduce: the initial value cannot be null")
                }
                let acc = self.new_local(init.ty.clone());
                let it = self.begin_iter_dir(recv, m == "reduceRight")?;
                let mut stmts = it.pre.clone();
                stmts.push(Stmt::Assign(acc, init));
                let mut body = it.head.clone();
                body.push(self.inline_cb(
                    cb,
                    &[acc, it.elem, it.idx, it.arr],
                    RetKind::Val,
                    Some(acc),
                )?);
                stmts.push(Stmt::Loop(it.label, body));
                let ty = self.cur.locals[acc as usize].clone();
                Ok(mk(ExprKind::Seq(stmts, Box::new(self.var_expr(acc))), ty))
            }
            (Ty::Arr(el), "includes" | "has") => {
                let Some(x) = c.arguments.first().and_then(|a| a.as_expression()) else {
                    bail!("{m}: missing argument")
                };
                let x = self.lower_expr(x, Some(el))?;
                let x = self.coerce(x, el)?;
                let xv = self.new_local(el.as_ref().clone());
                let it = self.begin_iter(recv)?;
                let out = self.new_local(Ty::Bool);
                let mut stmts = it.pre.clone();
                stmts.push(Stmt::Assign(xv, x));
                stmts.push(Stmt::Assign(out, mk(ExprKind::Bool(false), Ty::Bool)));
                let mut body = it.head.clone();
                let eq = mk(
                    ExprKind::Cmp(
                        CmpOp::Eq,
                        Box::new(self.var_expr(it.elem)),
                        Box::new(self.var_expr(xv)),
                    ),
                    Ty::Bool,
                );
                body.push(Stmt::If(
                    eq,
                    vec![
                        Stmt::Assign(out, mk(ExprKind::Bool(true), Ty::Bool)),
                        Stmt::Break(it.label),
                    ],
                    vec![],
                ));
                stmts.push(Stmt::Loop(it.label, body));
                Ok(mk(
                    ExprKind::Seq(stmts, Box::new(self.var_expr(out))),
                    Ty::Bool,
                ))
            }
            (t, m) => bail!(
                "{}: method `{m}` on {t} is not supported",
                self.loc(c.span.start)
            ),
        }
    }
}
