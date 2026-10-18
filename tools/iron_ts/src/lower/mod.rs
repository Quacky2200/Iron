//! TypeScript AST (oxc) -> Iron IR.
//!
//! Types come from the source's own annotations (see `types.rs`); expression types are inferred
//! bottom-up from them, with a contextual "expected" type pushed down for literals. The program
//! must already pass `tsc --strict`: the lowering trusts narrowing instead of re-proving it.

macro_rules! bail {
    ($($t:tt)*) => { return Err(format!($($t)*)) };
}

mod arrays;
mod closure;
mod expr;
mod scan;
mod stmt;
mod types;

use crate::ir::*;
use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_parser::Parser;
use oxc_span::SourceType;
use std::collections::{HashMap, HashSet};

pub type R<T> = Result<T, String>;

#[derive(Clone, Copy)]
pub(crate) enum FnBody<'a> {
    Block(&'a FunctionBody<'a>),
    Expr(&'a Expression<'a>),
}

pub(crate) struct FnSrc<'a> {
    pub params: &'a FormalParameters<'a>,
    pub ret: Option<&'a TSType<'a>>,
    pub body: FnBody<'a>,
    pub exported: bool,
}

/// What a name means inside a function.
#[derive(Clone)]
pub(crate) enum Binding {
    /// A local variable holding the value.
    Direct(VarId),
    /// A captured variable: a local holding the cell, with the value's type.
    Boxed(VarId, Ty),
}

#[derive(Clone)]
pub(crate) enum ConstInfo {
    /// A compile-time constant, inlined at every use.
    Inline(Expr),
    Global(GlobalId, Ty),
}

#[derive(Clone)]
pub(crate) enum RetKind {
    /// The callback's result is a truth value (filter, find, some, every).
    Pred,
    /// The callback's result is stored in the variable; its type is inferred if still `Void`.
    Val,
    /// Nothing is returned (forEach).
    Void,
}

#[derive(Clone)]
pub(crate) struct RetTarget {
    pub label: LabelId,
    pub var: Option<VarId>,
    pub kind: RetKind,
}

#[derive(Default)]
pub(crate) struct FnCtx {
    pub locals: Vec<Ty>,
    pub scopes: Vec<HashMap<String, Binding>>,
    /// Declared or inferred return type.
    pub ret: Option<Ty>,
    pub next_label: u32,
    pub loops: Vec<LabelId>,
    pub rets: Vec<RetTarget>,
    /// Open optional chains: (exit label, result variable).
    pub chain: Vec<(LabelId, VarId)>,
}

pub struct Lowerer<'a> {
    pub(crate) src: &'a str,
    pub(crate) m: Module,
    pub(crate) aliases: HashMap<String, &'a TSTypeAliasDeclaration<'a>>,
    pub(crate) ifaces: HashMap<String, &'a TSInterfaceDeclaration<'a>>,
    pub(crate) named: HashMap<String, Ty>,
    pub(crate) shapes: HashMap<String, StructId>,
    pub(crate) tag_ids: HashMap<String, u32>,
    pub(crate) fsrc: HashMap<String, FnSrc<'a>>,
    pub(crate) fids: HashMap<String, FuncId>,
    pub(crate) fsig: Vec<(Vec<Ty>, Option<Ty>)>,
    /// 0 declared, 1 being lowered, 2 done.
    pub(crate) fstate: Vec<u8>,
    pub(crate) consts: HashMap<String, ConstInfo>,
    pub(crate) cur: FnCtx,
    /// Names that closures in the function being lowered capture (`scan.rs`).
    pub(crate) captured: HashSet<String>,
    pub(crate) closure_free: HashMap<u32, Vec<String>>,
    pub(crate) lambdas: usize,
    /// Wrapper functions that let a top-level function be used as a value.
    pub(crate) wrappers: HashMap<String, FuncId>,
}

/// Parses and lowers one TypeScript module.
pub fn lower_source(src: &str) -> R<Module> {
    let alloc = Allocator::default();
    let ret = Parser::new(&alloc, src, SourceType::ts()).parse();
    if !ret.diagnostics.is_empty() {
        bail!("parse error: {}", ret.diagnostics[0]);
    }
    let program = ret.program;
    let mut l = Lowerer {
        src,
        m: Module::default(),
        aliases: HashMap::new(),
        ifaces: HashMap::new(),
        named: HashMap::new(),
        shapes: HashMap::new(),
        tag_ids: HashMap::new(),
        fsrc: HashMap::new(),
        fids: HashMap::new(),
        fsig: Vec::new(),
        fstate: Vec::new(),
        consts: HashMap::new(),
        cur: FnCtx::default(),
        captured: HashSet::new(),
        closure_free: HashMap::new(),
        lambdas: 0,
        wrappers: HashMap::new(),
    };
    l.lower_program(&program)?;
    Ok(l.m)
}

impl<'a> Lowerer<'a> {
    pub(crate) fn line(&self, offset: u32) -> usize {
        self.src[..(offset as usize).min(self.src.len())]
            .matches('\n')
            .count()
            + 1
    }

    pub(crate) fn tag(&mut self, name: &str) -> u32 {
        if let Some(t) = self.tag_ids.get(name) {
            return *t;
        }
        let id = self.m.tags.len() as u32;
        self.m.tags.push(name.to_string());
        self.tag_ids.insert(name.to_string(), id);
        id
    }

    fn lower_program(&mut self, program: &'a Program<'a>) -> R<()> {
        // 1. Types and function signatures, in any order.
        let mut vars: Vec<(&'a VariableDeclarator<'a>, bool)> = Vec::new();
        for stmt in &program.body {
            match stmt {
                Statement::ExportDeclaration(e) => self.collect(&e.declaration, true, &mut vars)?,
                Statement::VariableDeclaration(d) => self.collect_vars(d, false, &mut vars)?,
                Statement::FunctionDeclaration(f) => self.collect_fn(f, false)?,
                Statement::TSInterfaceDeclaration(d) => {
                    self.ifaces.insert(d.id.name.to_string(), d);
                }
                Statement::TSTypeAliasDeclaration(d) => {
                    self.aliases.insert(d.id.name.to_string(), d);
                }
                _ => {}
            }
        }
        // 2. Top-level values, in source order, into the init function.
        let init_id = self.m.funcs.len();
        self.m.funcs.push(Func {
            name: "init".into(),
            exported: true,
            params: vec![],
            ret: Ty::Void,
            locals: vec![],
            body: vec![],
        });
        self.fsig.push((vec![], Some(Ty::Void)));
        self.fstate.push(2);
        self.cur = FnCtx::default();
        self.cur.scopes.push(HashMap::new());
        let mut init_body = Vec::new();
        for (decl, _exported) in vars {
            self.lower_top_var(decl, &mut init_body)?;
        }
        init_body.push(Stmt::Return(None));
        let ctx = std::mem::take(&mut self.cur);
        self.m.funcs[init_id].locals = ctx.locals;
        self.m.funcs[init_id].body = init_body;
        self.m.init = Some(init_id);
        // 3. Every function body.
        let mut names: Vec<String> = self.fids.keys().cloned().collect();
        names.sort_by_key(|n| self.fids[n]);
        for n in names {
            self.ensure_lowered(&n)?;
        }
        Ok(())
    }

    fn collect(
        &mut self,
        d: &'a Declaration<'a>,
        exported: bool,
        vars: &mut Vec<(&'a VariableDeclarator<'a>, bool)>,
    ) -> R<()> {
        match d {
            Declaration::VariableDeclaration(v) => self.collect_vars(v, exported, vars)?,
            Declaration::FunctionDeclaration(f) => self.collect_fn(f, exported)?,
            Declaration::TSInterfaceDeclaration(d) => {
                self.ifaces.insert(d.id.name.to_string(), d);
            }
            Declaration::TSTypeAliasDeclaration(d) => {
                self.aliases.insert(d.id.name.to_string(), d);
            }
            _ => {}
        }
        Ok(())
    }

    fn collect_fn(&mut self, f: &'a Function<'a>, exported: bool) -> R<()> {
        let Some(id) = &f.id else {
            bail!("anonymous function")
        };
        let Some(body) = &f.body else { return Ok(()) };
        let src = FnSrc {
            params: &f.params,
            ret: f.return_type.as_ref().map(|r| &r.type_annotation),
            body: FnBody::Block(body),
            exported,
        };
        self.declare_fn(id.name.as_str(), src)
    }

    fn collect_vars(
        &mut self,
        v: &'a VariableDeclaration<'a>,
        exported: bool,
        vars: &mut Vec<(&'a VariableDeclarator<'a>, bool)>,
    ) -> R<()> {
        for d in &v.declarations {
            // `const f = (..) => ..` is a function declaration in disguise.
            if let (
                BindingPattern::BindingIdentifier(id),
                Some(Expression::ArrowFunctionExpression(a)),
                None,
            ) = (&d.id, &d.init, &d.type_annotation)
            {
                let body = match &a.body {
                    ArrowFunctionBody::FunctionBody(b) => FnBody::Block(b),
                    other => FnBody::Expr(other.as_expression().ok_or("arrow body")?),
                };
                let src = FnSrc {
                    params: &a.params,
                    ret: a.return_type.as_ref().map(|r| &r.type_annotation),
                    body,
                    exported,
                };
                self.declare_fn(id.name.as_str(), src)?;
                continue;
            }
            vars.push((d, exported));
        }
        Ok(())
    }

    fn declare_fn(&mut self, name: &str, src: FnSrc<'a>) -> R<()> {
        let mut ptys = Vec::new();
        for p in &src.params.items {
            let Some(ann) = &p.type_annotation else {
                bail!("{name}: parameter without a type")
            };
            ptys.push(self.ty(&ann.type_annotation, name)?);
        }
        let ret = match src.ret {
            Some(r) => Some(self.ty(r, name)?),
            None => None,
        };
        let id = self.m.funcs.len();
        self.m.funcs.push(Func {
            name: name.to_string(),
            exported: src.exported,
            params: vec![],
            ret: Ty::Void,
            locals: vec![],
            body: vec![],
        });
        self.fsig.push((ptys, ret));
        self.fstate.push(0);
        self.fids.insert(name.to_string(), id);
        self.fsrc.insert(name.to_string(), src);
        Ok(())
    }

    /// Lowers a function body now if it has not been yet (call sites need an inferred return type).
    pub(crate) fn ensure_lowered(&mut self, name: &str) -> R<()> {
        let id = self.fids[name];
        match self.fstate[id] {
            2 => return Ok(()),
            1 => {
                if self.fsig[id].1.is_none() {
                    bail!("{name}: recursive function needs a return type annotation");
                }
                return Ok(());
            }
            _ => {}
        }
        self.fstate[id] = 1;
        let saved = std::mem::take(&mut self.cur);
        // Closures inside this function decide which of its variables live in cells.
        let scan = match self.fsrc[name].body {
            FnBody::Block(b) => scan::Scan::of_statements(&b.statements),
            FnBody::Expr(e) => scan::Scan::of_expression(e),
        };
        let saved_captured = std::mem::replace(&mut self.captured, scan.captured);
        self.closure_free.extend(scan.free);
        let r = self.lower_fn(name, id);
        self.captured = saved_captured;
        self.cur = saved;
        r?;
        self.fstate[id] = 2;
        Ok(())
    }

    fn lower_fn(&mut self, name: &str, id: FuncId) -> R<()> {
        let (params, body) = {
            let s = &self.fsrc[name];
            (s.params, s.body)
        };
        let exp_ret = self.fsig[id].1.clone();
        self.cur = FnCtx {
            ret: exp_ret,
            ..FnCtx::default()
        };
        self.cur.scopes.push(HashMap::new());
        let ptys = self.fsig[id].0.clone();
        let mut pvars = Vec::new();
        let mut pro = Vec::new();
        for (p, ty) in params.items.iter().zip(ptys) {
            let BindingPattern::BindingIdentifier(b) = &p.pattern else {
                bail!("{name}: destructured parameter")
            };
            let v = self.new_local(ty);
            self.bind_var(b.name.as_str(), v, &mut pro);
            pvars.push(v);
        }
        let (stmts, ret) = self.lower_body_into(body, pro)?;
        self.fsig[id].1 = Some(ret.clone());
        let ctx = std::mem::take(&mut self.cur);
        let f = &mut self.m.funcs[id];
        f.params = pvars;
        f.ret = ret;
        f.locals = ctx.locals;
        f.body = stmts;
        Ok(())
    }

    // ---- locals and scopes ----

    pub(crate) fn new_local(&mut self, ty: Ty) -> VarId {
        self.cur.locals.push(ty);
        (self.cur.locals.len() - 1) as VarId
    }

    pub(crate) fn new_label(&mut self) -> LabelId {
        self.cur.next_label += 1;
        self.cur.next_label
    }

    pub(crate) fn lookup(&self, name: &str) -> Option<Binding> {
        self.cur
            .scopes
            .iter()
            .rev()
            .find_map(|s| s.get(name).cloned())
    }

    /// Names the local `var` (which already holds the value). A variable that closures capture
    /// moves into a new cell first, in `out`.
    pub(crate) fn bind_var(&mut self, name: &str, var: VarId, out: &mut Vec<Stmt>) {
        let scope = self.cur.scopes.len() - 1;
        if self.captured.contains(name) {
            let ty = self.cur.locals[var as usize].clone();
            let cell = self.new_local(Ty::Cell(Box::new(ty.clone())));
            let cell_ty = self.cur.locals[cell as usize].clone();
            let init = Expr {
                kind: ExprKind::NewCell(Box::new(self.var_expr(var))),
                ty: cell_ty,
            };
            out.push(Stmt::Assign(cell, init));
            self.cur.scopes[scope].insert(name.to_string(), Binding::Boxed(cell, ty));
        } else {
            self.cur.scopes[scope].insert(name.to_string(), Binding::Direct(var));
        }
    }

    /// The current value of a name.
    pub(crate) fn read_binding(&self, b: &Binding) -> Expr {
        match b {
            Binding::Direct(v) => self.var_expr(*v),
            Binding::Boxed(cell, ty) => Expr {
                kind: ExprKind::CellGet(Box::new(self.var_expr(*cell))),
                ty: ty.clone(),
            },
        }
    }

    /// Lowers a function body in the current context (parameters already bound; `pro` holds the
    /// statements that set them up). Returns the statements and the return type.
    pub(crate) fn lower_body_into(
        &mut self,
        body: FnBody<'a>,
        pro: Vec<Stmt>,
    ) -> R<(Vec<Stmt>, Ty)> {
        let mut stmts = pro;
        match body {
            FnBody::Block(b) => self.lower_stmts(&b.statements, &mut stmts)?,
            FnBody::Expr(e) => {
                let exp = self.cur.ret.clone();
                let ex = self.lower_expr(e, exp.as_ref())?;
                if exp == Some(Ty::Void) {
                    stmts.push(Stmt::Eval(ex));
                } else {
                    let ex = match exp {
                        Some(t) => self.coerce(ex, &t)?,
                        None => ex,
                    };
                    self.cur.ret = Some(ex.ty.clone());
                    stmts.push(Stmt::Return(Some(ex)));
                }
            }
        }
        let ret = self.cur.ret.clone().unwrap_or(Ty::Void);
        if ret == Ty::Void {
            stmts.push(Stmt::Return(None));
        } else {
            // Falling off the end of a non-void function: unreachable under tsc, keep codegen total.
            let z = self.default_value(&ret);
            stmts.push(Stmt::Return(Some(z)));
        }
        Ok((stmts, ret))
    }

    pub(crate) fn var_expr(&self, v: VarId) -> Expr {
        Expr {
            kind: ExprKind::Var(v),
            ty: self.cur.locals[v as usize].clone(),
        }
    }

    pub(crate) fn default_value(&self, ty: &Ty) -> Expr {
        let kind = match ty {
            Ty::Num => ExprKind::Num(0.0),
            Ty::Bool => ExprKind::Bool(false),
            Ty::Lit => ExprKind::Tag(0),
            _ => ExprKind::Null,
        };
        Expr {
            kind,
            ty: ty.clone(),
        }
    }

    fn lower_top_var(&mut self, d: &'a VariableDeclarator<'a>, init_body: &mut Vec<Stmt>) -> R<()> {
        let BindingPattern::BindingIdentifier(id) = &d.id else {
            bail!("destructured top-level const")
        };
        let name = id.name.to_string();
        let Some(init) = &d.init else {
            bail!("{name}: top-level binding without a value")
        };
        let exp = match &d.type_annotation {
            Some(a) => Some(self.ty(&a.type_annotation, &name)?),
            None => None,
        };
        let mut e = self.lower_expr(init, exp.as_ref())?;
        if let Some(t) = &exp {
            e = self.coerce(e, t)?;
        }
        if matches!(
            e.kind,
            ExprKind::Num(_) | ExprKind::Bool(_) | ExprKind::Tag(_)
        ) {
            self.consts.insert(name, ConstInfo::Inline(e));
            return Ok(());
        }
        let g = self.m.globals.len();
        self.m.globals.push(Global {
            name: name.clone(),
            ty: e.ty.clone(),
        });
        self.consts.insert(name, ConstInfo::Global(g, e.ty.clone()));
        init_body.push(Stmt::StoreGlobal(g, e));
        Ok(())
    }
}
