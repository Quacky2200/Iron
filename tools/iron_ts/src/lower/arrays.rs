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
        let el_ty = match arr.ty.strip() {
            Ty::Arr(e) => (**e).clone(),
            Ty::Bytes => Ty::Num,
            t => bail!("cannot iterate {t}"),
        };
        let a = self.new_local(arr.ty.clone());
        let i = self.new_local(Ty::Num);
        let elem = self.new_local(el_ty.clone());
        let idx = self.new_local(Ty::Num);
        let label = self.new_label();
        let (va, vi) = (self.var_expr(a), self.var_expr(i));
        let len = mk(ExprKind::Len(Box::new(va.clone())), Ty::Num);
        let done = mk(
            ExprKind::Cmp(CmpOp::Ge, Box::new(vi.clone()), Box::new(len)),
            Ty::Bool,
        );
        let raw_ty = match &el_ty {
            e if arr.ty.strip() != &Ty::Bytes && (e.is_ptr() || *e == Ty::Num) => {
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
            ExprKind::Bin(BinOp::Add, Box::new(vi.clone()), Box::new(num(1.0))),
            Ty::Num,
        );
        let head = vec![
            Stmt::If(done, vec![Stmt::Break(label)], vec![]),
            Stmt::Assign(elem, get),
            Stmt::Assign(idx, vi),
            Stmt::Assign(i, step),
        ];
        let pre = vec![Stmt::Assign(a, arr), Stmt::Assign(i, num(0.0))];
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
    fn inline_cb(
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
            let BindingPattern::BindingIdentifier(b) = &p.pattern else {
                bail!("destructured callback parameter")
            };
            self.bind_var(b.name.as_str(), *var, &mut body);
        }
        let label = self.new_label();
        let saved_loops = std::mem::take(&mut self.cur.loops);
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

    pub(crate) fn lower_method(
        &mut self,
        recv: Expr,
        m: &str,
        c: &'a CallExpression<'a>,
    ) -> R<Expr> {
        let rty = recv.ty.strip().clone();
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
            (Ty::Arr(_) | Ty::Bytes, "forEach" | "filter" | "find" | "some" | "every" | "map") => {
                let cb = cb_arg(0)?;
                let it = self.begin_iter(recv)?;
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
                    "filter" | "find" | "some" | "every" => {
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
                            "find" => {
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
                        let ty = Ty::Arr(Box::new(rty));
                        let out = self.new_local(ty.clone());
                        let ov = self.var_expr(out);
                        stmts.push(Stmt::Assign(
                            out,
                            mk(ExprKind::Rt(RtFn::ArrNew, vec![num(0.0)]), ty.clone()),
                        ));
                        body.push(Stmt::Eval(mk(
                            ExprKind::Rt(RtFn::ArrPush, vec![ov.clone(), self.var_expr(r)]),
                            Ty::Void,
                        )));
                        stmts.push(Stmt::Loop(label, body));
                        mk(ExprKind::Seq(stmts, Box::new(ov)), ty)
                    }
                };
                Ok(result)
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
            (Ty::Bytes, "fill") => {
                let mut args = vec![recv];
                for (i, a) in c.arguments.iter().enumerate().take(3) {
                    let Some(e) = a.as_expression() else {
                        bail!("fill: spread")
                    };
                    let v = self.lower_expr(e, Some(&Ty::Num))?;
                    let v = self.as_num(v)?;
                    let _ = i;
                    args.push(v);
                }
                if args.len() != 4 {
                    bail!("Uint8Array.fill needs (value, start, end)");
                }
                Ok(mk(ExprKind::Rt(RtFn::BytesFill, args), Ty::Void))
            }
            (t, m) => bail!(
                "{}: method `{m}` on {t} is not supported",
                self.loc(c.span.start)
            ),
        }
    }
}
