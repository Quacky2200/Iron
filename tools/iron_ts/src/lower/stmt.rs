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
            Statement::VariableDeclaration(v) => {
                for d in &v.declarations {
                    let BindingPattern::BindingIdentifier(id) = &d.id else {
                        bail!("{}: destructuring declaration", self.loc(d.span.start))
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
                        self.cur.scopes[scope]
                            .insert(id.name.to_string(), Binding::Boxed(cell, t.clone()));
                        prebound = Some(cell);
                    }
                    let init = match &d.init {
                        Some(e) => {
                            let v = self.lower_expr(e, ann.as_ref())?;
                            Some(match &ann {
                                Some(t) => self.coerce(v, t)?,
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
            }
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
                self.cur.loops.push(label);
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
                let BindingPattern::BindingIdentifier(id) = &d.declarations[0].id else {
                    bail!("destructured for-of variable")
                };
                let arr = self.lower_expr(&f.right, None)?;
                let it = self.begin_iter(arr)?;
                self.cur.scopes.push(HashMap::new());
                // A captured loop variable gets a fresh cell every iteration.
                let mut bind = Vec::new();
                self.bind_var(id.name.as_str(), it.elem, &mut bind);
                self.cur.loops.push(it.label);
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
            Statement::BreakStatement(_) => {
                let Some(&l) = self.cur.loops.last() else {
                    bail!("break outside a loop")
                };
                out.push(Stmt::Break(l));
            }
            Statement::ContinueStatement(_) => {
                let Some(&l) = self.cur.loops.last() else {
                    bail!("continue outside a loop")
                };
                out.push(Stmt::Continue(l));
            }
            other => bail!(
                "{}: unsupported statement",
                self.loc(oxc_span::GetSpan::span(other).start)
            ),
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
