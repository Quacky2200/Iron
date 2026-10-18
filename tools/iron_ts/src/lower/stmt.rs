//! Statement lowering.

use super::expr::mk;
use super::*;

impl<'a> Lowerer<'a> {
    pub(crate) fn lower_stmts(&mut self, stmts: &'a [Statement<'a>], out: &mut Vec<Stmt>) -> R<()> {
        for s in stmts {
            self.lower_stmt(s, out)?;
        }
        Ok(())
    }

    fn scoped(&mut self, s: &'a Statement<'a>) -> R<Vec<Stmt>> {
        self.cur.scopes.push(HashMap::new());
        let mut out = Vec::new();
        let r = self.lower_stmt(s, &mut out);
        self.cur.scopes.pop();
        r?;
        Ok(out)
    }

    fn lower_stmt(&mut self, s: &'a Statement<'a>, out: &mut Vec<Stmt>) -> R<()> {
        match s {
            Statement::EmptyStatement(_) => {}
            Statement::BlockStatement(b) => {
                self.cur.scopes.push(HashMap::new());
                let r = self.lower_stmts(&b.body, out);
                self.cur.scopes.pop();
                r?;
            }
            Statement::ExpressionStatement(e) => self.lower_effect(&e.expression, out)?,
            Statement::VariableDeclaration(v) => self.lower_var_decl(v, out)?,
            Statement::IfStatement(i) => {
                let c = self.lower_expr(&i.test, None)?;
                let c = self.to_bool(c);
                let then = self.scoped(&i.consequent)?;
                let els = match &i.alternate {
                    Some(a) => self.scoped(a)?,
                    None => vec![],
                };
                out.push(Stmt::If(c, then, els));
            }
            Statement::WhileStatement(w) => {
                let label = self.new_label();
                let c = self.lower_expr(&w.test, None)?;
                let c = self.to_bool(c);
                let not = mk(ExprKind::Un(UnOp::Not, Box::new(c)), Ty::Bool);
                let entry = self.loop_entry(label, Some(label));
                self.cur.loops.push(entry);
                let body = self.scoped(&w.body);
                self.cur.loops.pop();
                let mut b = vec![Stmt::If(not, vec![Stmt::Break(label)], vec![])];
                b.extend(body?);
                out.push(Stmt::Loop(label, b));
            }
            Statement::ForOfStatement(f) => {
                let ForStatementLeft::VariableDeclaration(d) = &f.left else {
                    bail!("for-of over an existing variable")
                };
                let pat = &d.declarations[0].id;
                let arr = self.lower_expr(&f.right, None)?;
                let it = self.begin_iter(arr)?;
                self.cur.scopes.push(HashMap::new());
                // A captured loop variable gets a fresh cell every iteration.
                let mut bind = Vec::new();
                self.bind_param(pat, it.elem, &mut bind)?;
                let entry = self.loop_entry(it.label, Some(it.label));
                self.cur.loops.push(entry);
                let body = self.scoped(&f.body);
                self.cur.loops.pop();
                self.cur.scopes.pop();
                out.extend(it.pre);
                let mut b = it.head;
                b.extend(bind);
                b.extend(body?);
                out.push(Stmt::Loop(it.label, b));
            }
            Statement::ReturnStatement(r) => {
                let v = match &r.argument {
                    Some(e) => {
                        let exp = self.cur.ret.clone();
                        Some(self.lower_expr(e, exp.as_ref())?)
                    }
                    None => None,
                };
                self.emit_return(v, out)?;
            }
            Statement::BreakStatement(b) => {
                let l = self.jump_target(b.label.as_ref().map(|l| l.name.as_str()))?;
                out.push(Stmt::Break(l));
            }
            Statement::ContinueStatement(c) => {
                let name = c.label.as_ref().map(|l| l.name.as_str());
                let (l, is_loop) = self.continue_target(name)?;
                out.push(if is_loop {
                    Stmt::Continue(l)
                } else {
                    Stmt::Break(l)
                });
            }
            Statement::DoWhileStatement(d) => {
                // Loop { Block(cont) { body }; if !test break }: `continue` leaves the block and
                // reaches the test.
                let (top, cont) = (self.new_label(), self.new_label());
                let entry = self.loop_entry(top, Some(cont));
                self.cur.loops.push(entry);
                let body = self.scoped(&d.body);
                self.cur.loops.pop();
                let c = self.lower_expr(&d.test, None)?;
                let c = self.to_bool(c);
                let not = mk(ExprKind::Un(UnOp::Not, Box::new(c)), Ty::Bool);
                let b = vec![
                    Stmt::Block(cont, body?),
                    Stmt::If(not, vec![Stmt::Break(top)], vec![]),
                ];
                out.push(Stmt::Loop(top, b));
            }
            Statement::ForStatement(f) => self.lower_for(f, out)?,
            Statement::SwitchStatement(sw) => self.lower_switch(sw, out)?,
            Statement::LabeledStatement(l) => {
                let name = l.label.name.to_string();
                if matches!(
                    l.body,
                    Statement::ForStatement(_)
                        | Statement::WhileStatement(_)
                        | Statement::DoWhileStatement(_)
                        | Statement::ForOfStatement(_)
                ) {
                    self.cur.pending_label = Some(name);
                    self.lower_stmt(&l.body, out)?;
                } else {
                    // `name: { ... break name; ... }`
                    let b = self.new_label();
                    self.cur.loops.push(LoopLabels {
                        name: Some(name),
                        brk: b,
                        cont: None,
                        unlabeled_break: false,
                    });
                    let body = self.scoped(&l.body);
                    self.cur.loops.pop();
                    out.push(Stmt::Block(b, body?));
                }
            }
            other => bail!(
                "{}: unsupported statement",
                self.loc(oxc_span::GetSpan::span(other).start)
            ),
        }
        Ok(())
    }

    /// `switch`: find the first matching clause (or `default`), then run the clause bodies from
    /// there on, in source order, until a `break` leaves the block (that is JS's fall-through).
    ///
    /// ```text
    /// tmp = discriminant; idx = (index of default, or the clause count)
    /// Block sel { if tmp === c0 { idx = 0; break sel }  if tmp === c1 { idx = 1; break sel } ... }
    /// Block brk { if idx <= 0 { body0 }  if idx <= 1 { body1 } ... }
    /// ```
    fn lower_switch(&mut self, sw: &'a SwitchStatement<'a>, out: &mut Vec<Stmt>) -> R<()> {
        let d = self.lower_expr(&sw.discriminant, None)?;
        let tmp = self.new_local(d.ty.clone());
        out.push(Stmt::Assign(tmp, d.clone()));
        let n = sw.cases.len();
        let default = sw.cases.iter().position(|c| c.test.is_none()).unwrap_or(n);
        let idx = self.new_local(Ty::Num);
        out.push(Stmt::Assign(idx, super::expr::num(default as f64)));
        let (sel, brk) = (self.new_label(), self.new_label());
        let mut select = Vec::new();
        for (k, c) in sw.cases.iter().enumerate() {
            let Some(test) = &c.test else { continue };
            let want = d.ty.strip().clone();
            let t = self.lower_expr(test, Some(&want))?;
            let eq = self.lower_compare(CmpOp::Eq, self.var_expr(tmp), t, c.span.start)?;
            select.push(Stmt::If(
                eq,
                vec![
                    Stmt::Assign(idx, super::expr::num(k as f64)),
                    Stmt::Break(sel),
                ],
                vec![],
            ));
        }
        out.push(Stmt::Block(sel, select));
        self.cur.scopes.push(HashMap::new());
        self.cur.loops.push(LoopLabels {
            name: self.cur.pending_label.take(),
            brk,
            cont: None,
            unlabeled_break: true,
        });
        let mut bodies = Vec::new();
        let mut failed = None;
        for (k, c) in sw.cases.iter().enumerate() {
            let mut body = Vec::new();
            if let Err(e) = self.lower_stmts(&c.consequent, &mut body) {
                failed = Some(e);
                break;
            }
            let le = mk(
                ExprKind::Cmp(
                    CmpOp::Le,
                    Box::new(self.var_expr(idx)),
                    Box::new(super::expr::num(k as f64)),
                ),
                Ty::Bool,
            );
            bodies.push(Stmt::If(le, body, vec![]));
        }
        self.cur.loops.pop();
        self.cur.scopes.pop();
        if let Some(e) = failed {
            return Err(e);
        }
        out.push(Stmt::Block(brk, bodies));
        Ok(())
    }

    /// The entry for a loop being lowered; it takes a pending source label.
    pub(crate) fn loop_entry(&mut self, brk: LabelId, cont: Option<LabelId>) -> LoopLabels {
        LoopLabels {
            name: self.cur.pending_label.take(),
            brk,
            cont,
            unlabeled_break: true,
        }
    }

    /// Where `break` (or `break name`) goes.
    fn jump_target(&self, name: Option<&str>) -> R<LabelId> {
        for e in self.cur.loops.iter().rev() {
            match name {
                Some(n) if e.name.as_deref() == Some(n) => return Ok(e.brk),
                None if e.unlabeled_break => return Ok(e.brk),
                _ => {}
            }
        }
        match name {
            Some(n) => bail!("break to unknown label `{n}`"),
            None => bail!("break outside a loop or switch"),
        }
    }

    /// Where `continue` (or `continue name`) goes: the label, and whether it is a loop top
    /// (`Continue`) or the end of a body block (`Break`).
    fn continue_target(&self, name: Option<&str>) -> R<(LabelId, bool)> {
        for e in self.cur.loops.iter().rev() {
            let Some(c) = e.cont else { continue };
            if name.is_none() || e.name.as_deref() == name {
                return Ok((c, c == e.brk));
            }
        }
        match name {
            Some(n) => bail!("continue to `{n}`, which is not a loop"),
            None => bail!("continue outside a loop"),
        }
    }

    /// `for (init; test; update) body`:
    /// `init; Loop { if !test break; Block(cont) { body }; update }`, `continue` leaving the block.
    fn lower_for(&mut self, f: &'a ForStatement<'a>, out: &mut Vec<Stmt>) -> R<()> {
        let name = self.cur.pending_label.take();
        self.cur.scopes.push(HashMap::new());
        let r = self.lower_for_in_scope(f, name, out);
        self.cur.scopes.pop();
        r
    }

    fn lower_for_in_scope(
        &mut self,
        f: &'a ForStatement<'a>,
        name: Option<String>,
        out: &mut Vec<Stmt>,
    ) -> R<()> {
        match &f.init {
            Some(ForStatementInit::VariableDeclaration(v)) => self.lower_var_decl(v, out)?,
            Some(init) => {
                let e = init.as_expression().ok_or("for initialiser")?;
                self.lower_effect(e, out)?;
            }
            None => {}
        }
        // Captured `let` variables of the head are copied into a fresh cell every iteration.
        let scope = self.cur.scopes.len() - 1;
        let fresh: Vec<(VarId, Ty)> = self.cur.scopes[scope]
            .values()
            .filter_map(|b| match b {
                Binding::Boxed(cell, ty) => Some((*cell, ty.clone())),
                _ => None,
            })
            .collect();
        let (top, cont) = (self.new_label(), self.new_label());
        let mut b = Vec::new();
        if let Some(t) = &f.test {
            let c = self.lower_expr(t, None)?;
            let c = self.to_bool(c);
            let not = mk(ExprKind::Un(UnOp::Not, Box::new(c)), Ty::Bool);
            b.push(Stmt::If(not, vec![Stmt::Break(top)], vec![]));
        }
        self.cur.pending_label = name;
        let entry = self.loop_entry(top, Some(cont));
        self.cur.loops.push(entry);
        let body = self.scoped(&f.body);
        self.cur.loops.pop();
        b.push(Stmt::Block(cont, body?));
        for (cell, ty) in fresh {
            let cell_ty = self.cur.locals[cell as usize].clone();
            let old = self.var_expr(cell);
            let value = mk(ExprKind::CellGet(Box::new(old)), ty);
            b.push(Stmt::Assign(
                cell,
                mk(ExprKind::NewCell(Box::new(value)), cell_ty),
            ));
        }
        if let Some(u) = &f.update {
            self.lower_effect(u, &mut b)?;
        }
        out.push(Stmt::Loop(top, b));
        Ok(())
    }

    /// `const { a, b: [c] } = value;`
    fn lower_destructuring(&mut self, d: &'a VariableDeclarator<'a>, out: &mut Vec<Stmt>) -> R<()> {
        let Some(init) = &d.init else {
            bail!("{}: destructuring needs a value", self.loc(d.span.start))
        };
        let ann = match &d.type_annotation {
            Some(a) => Some(self.ty(&a.type_annotation, "pattern")?),
            None => None,
        };
        let mut v = self.lower_expr(init, ann.as_ref())?;
        if let Some(t) = &ann {
            v = self.coerce(v, t)?;
        }
        self.bind_pattern(&d.id, v, out)
    }

    /// Names a parameter (or loop variable) that is already in local `var`: a plain name, or a
    /// pattern taken apart in `out`.
    pub(crate) fn bind_param(
        &mut self,
        pat: &'a BindingPattern<'a>,
        var: VarId,
        out: &mut Vec<Stmt>,
    ) -> R<()> {
        match pat {
            BindingPattern::BindingIdentifier(b) => {
                self.bind_var(b.name.as_str(), var, out);
                Ok(())
            }
            p => {
                let v = self.var_expr(var);
                self.bind_pattern(p, v, out)
            }
        }
    }

    /// Binds the names of a pattern to the parts of `value`: `{ a, b: { c } }`, `[x, y]`, and
    /// `name = default` anywhere (the default replaces a null).
    pub(crate) fn bind_pattern(
        &mut self,
        pat: &'a BindingPattern<'a>,
        value: Expr,
        out: &mut Vec<Stmt>,
    ) -> R<()> {
        match pat {
            BindingPattern::BindingIdentifier(id) => {
                let var = self.new_local(value.ty.clone());
                out.push(Stmt::Assign(var, value));
                self.bind_var(id.name.as_str(), var, out);
            }
            BindingPattern::AssignmentPattern(a) => {
                if !value.ty.is_nullable() {
                    // Never null: the default cannot be used.
                    return self.bind_pattern(&a.left, value, out);
                }
                let base = value.ty.strip().clone();
                let tmp = self.new_local(value.ty.clone());
                out.push(Stmt::Assign(tmp, value));
                let t = self.var_expr(tmp);
                let d = self.lower_expr(&a.right, Some(&base))?;
                let d = self.coerce(d, &base)?;
                let have = mk(ExprKind::Cast(Box::new(t.clone())), base.clone());
                let picked = mk(
                    ExprKind::Cond(
                        Box::new(mk(ExprKind::IsNull(Box::new(t)), Ty::Bool)),
                        Box::new(d),
                        Box::new(have),
                    ),
                    base,
                );
                self.bind_pattern(&a.left, picked, out)?;
            }
            BindingPattern::ObjectPattern(o) => {
                if o.rest.is_some() {
                    bail!("a rest element in an object pattern is not supported")
                }
                let tmp = self.new_local(value.ty.clone());
                out.push(Stmt::Assign(tmp, value));
                for p in &o.properties {
                    if p.computed {
                        bail!("computed key in an object pattern")
                    }
                    let key = self.key_name(&p.key)?;
                    let part = self.member_of(self.var_expr(tmp), &key)?;
                    self.bind_pattern(&p.value, part, out)?;
                }
            }
            BindingPattern::ArrayPattern(a) => {
                if a.rest.is_some() {
                    bail!("a rest element in an array pattern is not supported")
                }
                let Ty::Arr(el) = value.ty.strip().clone() else {
                    bail!("cannot destructure {} as an array", value.ty)
                };
                let el = *el;
                let tmp = self.new_local(value.ty.clone());
                out.push(Stmt::Assign(tmp, value));
                for (i, p) in a.elements.iter().enumerate() {
                    let Some(p) = p else { continue };
                    // Past the end an element reads as null (a zero pointer, or the null NaN).
                    let read_ty = if el.is_ptr() || el == Ty::Num {
                        el.clone().nullable()
                    } else {
                        el.clone()
                    };
                    let read = mk(
                        ExprKind::Index(
                            Box::new(self.var_expr(tmp)),
                            Box::new(super::expr::num(i as f64)),
                        ),
                        read_ty.clone(),
                    );
                    let part = if matches!(p, BindingPattern::AssignmentPattern(_)) || read_ty == el
                    {
                        read
                    } else {
                        mk(ExprKind::Cast(Box::new(read)), el.clone())
                    };
                    self.bind_pattern(p, part, out)?;
                }
            }
        }
        Ok(())
    }

    fn lower_var_decl(&mut self, v: &'a VariableDeclaration<'a>, out: &mut Vec<Stmt>) -> R<()> {
        let v_kind = v.kind;
        for d in &v.declarations {
            let BindingPattern::BindingIdentifier(id) = &d.id else {
                self.lower_destructuring(d, out)?;
                continue;
            };
            let ann = match &d.type_annotation {
                Some(a) => Some(self.ty(&a.type_annotation, id.name.as_str())?),
                None => None,
            };
            // A closure that refers to its own name: the cell exists before the closure.
            let mut prebound = None;
            if self.captured.contains(id.name.as_str())
                && let Some(t @ Ty::Fn(_)) = &ann
            {
                let cell = self.new_local(Ty::Cell(Box::new(t.clone())));
                let cell_ty = self.cur.locals[cell as usize].clone();
                let zero = self.default_value(t);
                let new = Expr {
                    kind: ExprKind::NewCell(Box::new(zero)),
                    ty: cell_ty,
                };
                out.push(Stmt::Assign(cell, new));
                let scope = self.cur.scopes.len() - 1;
                self.cur.scopes[scope].insert(id.name.to_string(), Binding::Boxed(cell, t.clone()));
                prebound = Some(cell);
            }
            let init = match &d.init {
                Some(e) => {
                    let v = self.lower_expr(e, ann.as_ref())?;
                    Some(match &ann {
                        Some(t) => self.coerce(v, t)?,
                        // `let s = "x"` is a `string` (a `const` keeps its literal type).
                        None if v.ty == Ty::Lit && v_kind != VariableDeclarationKind::Const => {
                            self.widen_lit(v)
                        }
                        None => v,
                    })
                }
                None => None,
            };
            let ty = match (&ann, &init) {
                (Some(t), _) => t.clone(),
                (None, Some(v)) if v.ty != Ty::Null => v.ty.clone(),
                _ => bail!(
                    "{}: cannot infer the type of `{}`",
                    self.loc(d.span.start),
                    id.name
                ),
            };
            let v = match init {
                Some(v) => v,
                None => self.default_value(&ty),
            };
            if let Some(cell) = prebound {
                let c = self.var_expr(cell);
                out.push(Stmt::CellSet(c, v));
                continue;
            }
            let var = self.new_local(ty.clone());
            out.push(Stmt::Assign(var, v));
            self.bind_var(id.name.as_str(), var, out);
        }
        Ok(())
    }

    /// An expression statement: assignments and updates need no value.
    fn lower_effect(&mut self, e: &'a Expression<'a>, out: &mut Vec<Stmt>) -> R<()> {
        match e {
            Expression::AssignmentExpression(a) => {
                let (stmts, _) = self.lower_assign(a)?;
                out.extend(stmts);
            }
            Expression::UpdateExpression(u) => {
                let (stmts, _) = self.lower_update(u)?;
                out.extend(stmts);
            }
            Expression::ParenthesizedExpression(p) => self.lower_effect(&p.expression, out)?,
            _ => {
                let v = self.lower_expr(e, None)?;
                out.push(Stmt::Eval(v));
            }
        }
        Ok(())
    }
}
