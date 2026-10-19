//! TypeScript AST (oxc) -> Iron IR.
//!
//! Types come from the source's own annotations (see `types.rs`); expression types are inferred
//! bottom-up from them, with a contextual "expected" type pushed down for literals. The program
//! must already pass `tsc --strict`: the lowering trusts narrowing instead of re-proving it.

macro_rules! bail {
    ($($t:tt)*) => { return Err(format!($($t)*)) };
}

mod arrays;
mod classes;
mod closure;
mod date;
mod enums;
mod expr;
mod intl;
mod json;
mod maps;
mod object;
mod prelude;
mod regexp;
mod scan;
mod stmt;
mod strings;
mod types;

use crate::ir::*;
use classes::Which;
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

/// What a declared function is, for the `this` parameter and the constructor's body.
#[derive(Clone, Copy)]
pub(crate) enum FnKind {
    Free,
    /// A method or accessor of the class with this struct: `this` is the first parameter.
    Method(StructId),
    /// A constructor body (`C$init`): `this` is the first parameter, the object already exists.
    /// The flag: a derived class with no constructor of its own.
    Ctor(StructId, bool),
}

pub(crate) struct FnSrc<'a> {
    pub params: &'a FormalParameters<'a>,
    pub ret: Option<&'a TSType<'a>>,
    pub body: FnBody<'a>,
    pub exported: bool,
    pub kind: FnKind,
    /// A return type that is not an annotation (a constructor returns its class).
    pub ret_ty: Option<Ty>,
}

/// What a name means inside a function.
#[derive(Clone)]
pub(crate) enum Binding {
    /// A local variable holding the value.
    Direct(VarId),
    /// A captured variable: a local holding the cell, with the value's type.
    Boxed(VarId, Ty),
}

/// A top-level declarator: (declarator, exported, declared with `const`).
enum TopItem<'a> {
    Var(&'a VariableDeclarator<'a>, bool, bool),
    Class(&'a Class<'a>),
}

#[derive(Clone)]
pub(crate) enum ConstInfo {
    /// A compile-time constant, inlined at every use.
    Inline(Expr),
    /// A module-level value in a data slot; the flag says it is a `let`/`var` (assignable).
    Global(GlobalId, Ty, bool),
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

/// What `break` and `continue` mean at one nesting level.
#[derive(Clone)]
pub(crate) struct LoopLabels {
    /// The source label (`outer: for ...`), if any.
    pub name: Option<String>,
    /// `break` target: leaves the loop, switch or labelled block.
    pub brk: LabelId,
    /// `continue` target; none for a switch or a labelled block.
    pub cont: Option<LabelId>,
    /// Does an unlabelled `break` stop here (loops and switches, not labelled blocks)?
    pub unlabeled_break: bool,
}

/// How control leaves a `try` ... `finally` early.
#[derive(Clone, PartialEq)]
pub(crate) enum Exit {
    Return,
    Break(LabelId),
    /// `continue`: the label, and whether it is a loop top (`Continue`) or a block end (`Break`).
    Continue(LabelId, bool),
}

/// An open `try ... finally`. A `return`, `break` or `continue` inside it records which exit it
/// wanted in `kind`, stores a return value in `ret_var`, and leaves through the label `done`; the
/// finalizer runs after that label, and then the recorded exit is taken (through the finalizers
/// of the enclosing `try`s too).
pub(crate) struct FinCtx {
    pub done: LabelId,
    pub kind: VarId,
    pub loops_depth: usize,
    pub rets_depth: usize,
    pub ret_var: Option<VarId>,
    pub exits: Vec<Exit>,
}

#[derive(Default)]
pub(crate) struct FnCtx {
    /// Open `try ... finally`s, innermost last.
    pub fins: Vec<FinCtx>,
    pub locals: Vec<Ty>,
    pub scopes: Vec<HashMap<String, Binding>>,
    /// Declared or inferred return type.
    pub ret: Option<Ty>,
    pub next_label: u32,
    /// Enclosing loops, switches and labelled statements, innermost last.
    pub loops: Vec<LoopLabels>,
    /// A label just read (`outer:`) that the loop lowered next takes as its name.
    pub pending_label: Option<String>,
    pub rets: Vec<RetTarget>,
    /// Open optional chains: (exit label, result variable).
    pub chain: Vec<(LabelId, VarId)>,
    /// Lowering a method or constructor of this class.
    pub cls: Option<(String, StructId)>,
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
    /// The engine API scripts may call (`Namespace.fn(..)`) and its handle types.
    pub(crate) manifest: crate::manifest::Manifest,
    /// The structs of the manifest's value classes, made when first named.
    pub(crate) value_structs: HashMap<String, StructId>,
    /// Class declarations by name, and what is known about each once its struct exists.
    pub(crate) class_decls: HashMap<String, &'a Class<'a>>,
    pub(crate) classes: HashMap<String, classes::ClassInfo>,
    pub(crate) class_of_sid: HashMap<StructId, String>,
    /// An empty parameter list and body for classes without a constructor.
    pub(crate) empty_fn: Option<(&'a FormalParameters<'a>, &'a FunctionBody<'a>)>,
    /// Enum members by enum name (`enums.rs`).
    pub(crate) enums: HashMap<String, HashMap<String, Expr>>,
    pub(crate) cur: FnCtx,
    /// Names that closures in the function being lowered capture (`scan.rs`).
    pub(crate) captured: HashSet<String>,
    pub(crate) closure_free: HashMap<u32, Vec<String>>,
    pub(crate) lambdas: usize,
    /// Wrapper functions that let a top-level function be used as a value.
    pub(crate) wrappers: HashMap<String, FuncId>,
    /// The capture groups of `const re = /literal/` names (for `replace` callbacks).
    pub(crate) regex_info: HashMap<String, Vec<Option<String>>>,
    /// Lines of prelude source in front of the program (skipped in messages).
    pub(crate) prelude_lines: usize,
}

/// Parses and lowers one TypeScript module.
pub fn lower_source(src: &str) -> R<Module> {
    lower_source_with(src, &[])
}

/// Like `lower_source`, with engine API manifests (JSON, one per namespace).
pub fn lower_source_with(src: &str, manifests: &[String]) -> R<Module> {
    let mut manifest = crate::manifest::Manifest::default();
    for m in manifests {
        manifest.add_json(m)?;
    }
    // Programs that use errors get the `Error` classes compiled in front of them.
    let mut head = String::new();
    if prelude::wanted(src)
        || prelude::wants_regexp(src)
        || prelude::wants_date(src)
        || prelude::wants_intl(src)
    {
        // Regular expressions throw `SyntaxError`s, so they bring the error classes along.
        head += prelude::ERRORS;
    }
    if prelude::wants_regexp(src) {
        head += prelude::REGEXP;
    }
    if prelude::wants_date(src) {
        head += prelude::DATE;
    }
    if src.contains("Intl.Collator") || src.contains("Intl.NumberFormat") {
        head += prelude::INTL;
    }
    let prelude_lines = head.matches('\n').count();
    let full = format!("{head}{src}");
    let src = full.as_str();
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
        manifest,
        value_structs: HashMap::new(),
        class_decls: HashMap::new(),
        classes: HashMap::new(),
        class_of_sid: HashMap::new(),
        empty_fn: None,
        enums: HashMap::new(),
        cur: FnCtx::default(),
        captured: HashSet::new(),
        closure_free: HashMap::new(),
        lambdas: 0,
        wrappers: HashMap::new(),
        regex_info: HashMap::new(),
        prelude_lines,
    };
    l.lower_program(&program)?;
    let mut m = l.m;
    crate::throws::analyze(&mut m);
    Ok(m)
}

impl<'a> Lowerer<'a> {
    pub(crate) fn line(&self, offset: u32) -> usize {
        (self.src[..(offset as usize).min(self.src.len())]
            .matches('\n')
            .count()
            + 1)
        .saturating_sub(self.prelude_lines)
        .max(1)
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
        // 0. Enums, interfaces, aliases and classes first: their names are types that signatures
        // below mention.
        for stmt in &program.body {
            let decl = match stmt {
                Statement::ExportDeclaration(x) => Some(&x.declaration),
                _ => None,
            };
            match (stmt, decl) {
                (Statement::TSEnumDeclaration(e), _) => self.declare_enum(e)?,
                (Statement::ClassDeclaration(c), _) => self.declare_class_name(c)?,
                (_, Some(Declaration::TSEnumDeclaration(e))) => self.declare_enum(e)?,
                (_, Some(Declaration::ClassDeclaration(c))) => self.declare_class_name(c)?,
                _ => {}
            }
        }
        let mut vars: Vec<TopItem<'a>> = Vec::new();
        for stmt in &program.body {
            match stmt {
                Statement::ExportDeclaration(e) => self.collect(&e.declaration, true, &mut vars)?,
                Statement::VariableDeclaration(d) => self.collect_vars(d, false, &mut vars)?,
                Statement::FunctionDeclaration(f) => self.collect_fn(f, false)?,
                Statement::ClassDeclaration(c) => {
                    self.collect_class(c)?;
                    vars.push(TopItem::Class(c));
                }
                Statement::TSInterfaceDeclaration(d) => {
                    self.ifaces.insert(d.id.name.to_string(), d);
                }
                Statement::TSTypeAliasDeclaration(d) => {
                    self.aliases.insert(d.id.name.to_string(), d);
                }
                _ => {}
            }
        }
        self.finalize_classes()?;
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
        for item in vars {
            match item {
                TopItem::Var(decl, _exported, is_const) => {
                    self.lower_top_var(decl, is_const, &mut init_body)?
                }
                TopItem::Class(c) => self.lower_class_statics(c, &mut init_body)?,
            }
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
        vars: &mut Vec<TopItem<'a>>,
    ) -> R<()> {
        match d {
            Declaration::VariableDeclaration(v) => self.collect_vars(v, exported, vars)?,
            Declaration::FunctionDeclaration(f) => self.collect_fn(f, exported)?,
            Declaration::ClassDeclaration(c) => {
                self.collect_class(c)?;
                vars.push(TopItem::Class(c));
            }
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
            kind: FnKind::Free,
            ret_ty: None,
        };
        self.declare_fn(id.name.as_str(), src)
    }

    fn collect_vars(
        &mut self,
        v: &'a VariableDeclaration<'a>,
        exported: bool,
        vars: &mut Vec<TopItem<'a>>,
    ) -> R<()> {
        let is_const = v.kind == VariableDeclarationKind::Const;
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
                    kind: FnKind::Free,
                    ret_ty: None,
                };
                self.declare_fn(id.name.as_str(), src)?;
                continue;
            }
            vars.push(TopItem::Var(d, exported, is_const));
        }
        Ok(())
    }

    pub(crate) fn declare_fn(&mut self, name: &str, src: FnSrc<'a>) -> R<()> {
        let mut ptys = Vec::new();
        if let FnKind::Method(sid) | FnKind::Ctor(sid, _) = src.kind {
            ptys.push(Ty::Obj(sid));
        }
        for p in &src.params.items {
            let Some(ann) = &p.type_annotation else {
                bail!("{name}: parameter without a type")
            };
            let ty = self.ty(&ann.type_annotation, name)?;
            // `x?: T` without a default is `T | undefined` inside.
            ptys.push(if p.optional && p.initializer.is_none() {
                ty.nullable()
            } else {
                ty
            });
        }
        if let Some(r) = &src.params.rest {
            let Some(ann) = &r.type_annotation else {
                bail!("{name}: rest parameter without a type")
            };
            let ty = self.ty(&ann.type_annotation, name)?;
            if !matches!(ty, Ty::Arr(_)) {
                bail!("{name}: a rest parameter must be an array type")
            }
            ptys.push(ty);
        }
        let ret = match (&src.ret_ty, src.ret) {
            (Some(t), _) => Some(t.clone()),
            (None, Some(r)) => Some(self.ty(r, name)?),
            (None, None) => None,
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
        let mut scan = match self.fsrc[name].body {
            FnBody::Block(b) => scan::Scan::of_statements(&b.statements),
            FnBody::Expr(e) => scan::Scan::of_expression(e),
        };
        // A constructor also runs the field initialisers (`handler = () => this.n`).
        if let FnKind::Ctor(sid, _) = self.fsrc[name].kind {
            let class = self.class_decls[&self.class_of_sid[&sid]];
            for el in &class.body.body {
                if let ClassElement::PropertyDefinition(p) = el
                    && !p.r#static
                    && let Some(v) = &p.value
                {
                    let extra = scan::Scan::of_expression(v);
                    scan.captured.extend(extra.captured);
                    scan.free.extend(extra.free);
                }
            }
        }
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
        let (params, body, kind) = {
            let s = &self.fsrc[name];
            (s.params, s.body, s.kind)
        };
        let exp_ret = self.fsig[id].1.clone();
        self.cur = FnCtx {
            ret: exp_ret,
            cls: match kind {
                FnKind::Method(s) | FnKind::Ctor(s, _) => Some((self.class_of_sid[&s].clone(), s)),
                FnKind::Free => None,
            },
            ..FnCtx::default()
        };
        self.cur.scopes.push(HashMap::new());
        let ptys = self.fsig[id].0.clone();
        let mut pvars = Vec::new();
        let mut pro = Vec::new();
        let mut ptys = ptys.into_iter();
        if matches!(kind, FnKind::Method(_) | FnKind::Ctor(..)) {
            let ty = ptys.next().expect("a method has a this type");
            let v = self.new_local(ty);
            self.bind_var("this", v, &mut pro);
            pvars.push(v);
        }
        for (p, ty) in params.items.iter().zip(ptys.by_ref()) {
            let v = self.new_local(ty);
            self.bind_param(&p.pattern, v, &mut pro)?;
            pvars.push(v);
        }
        if let (Some(r), Some(ty)) = (&params.rest, ptys.next()) {
            let v = self.new_local(ty);
            self.bind_param(&r.rest.argument, v, &mut pro)?;
            pvars.push(v);
        }
        if let FnKind::Ctor(sid, implicit) = kind {
            let class = self.class_of_sid[&sid].clone();
            self.ctor_prologue(&class, implicit, &pvars, &mut pro)?;
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

    fn lower_top_var(
        &mut self,
        d: &'a VariableDeclarator<'a>,
        is_const: bool,
        init_body: &mut Vec<Stmt>,
    ) -> R<()> {
        let BindingPattern::BindingIdentifier(id) = &d.id else {
            bail!("destructured top-level binding")
        };
        let name = id.name.to_string();
        let exp = match &d.type_annotation {
            Some(a) => Some(self.ty(&a.type_annotation, &name)?),
            None => None,
        };
        let e = match (&d.init, &exp) {
            (Some(init), _) => {
                let mut e = self.lower_expr(init, exp.as_ref())?;
                if let Some(t) = &exp {
                    e = self.coerce(e, t)?;
                } else if !is_const {
                    // `let s = "x"` is a `string`.
                    e = self.widen_lit(e);
                }
                e
            }
            // `let x: T;` starts as null / 0, like a field nobody set.
            (None, Some(t)) if !is_const => self.default_value(t),
            _ => bail!("{name}: top-level binding without a value"),
        };
        // Only a `const` may be folded into its uses; a `let` can change.
        if is_const
            && matches!(
                e.kind,
                ExprKind::Num(_) | ExprKind::Bool(_) | ExprKind::Tag(_)
            )
        {
            self.consts.insert(name, ConstInfo::Inline(e));
            return Ok(());
        }
        if matches!(e.ty, Ty::Null) {
            bail!("{name}: a variable initialised with null needs a type annotation")
        }
        let g = self.m.globals.len();
        self.m.globals.push(Global {
            name: name.clone(),
            ty: e.ty.clone(),
        });
        self.consts
            .insert(name, ConstInfo::Global(g, e.ty.clone(), !is_const));
        init_body.push(Stmt::StoreGlobal(g, e));
        Ok(())
    }
}
