//! Generators by source rewriting. A generator body is a function that runs on its own native stack
//! (a coroutine, `iron_ts_rt/src/coro.rs`). Before parsing, every `function*` / `*method()` is
//! rewritten so the body becomes a closure handed to a per-type generator class:
//!
//! ```text
//! function* count(n: number): Generator<number> { for (...) yield i; }
//!   =>
//! function count(n: number): Generator<number> { return __genNew$number(() => { for (...) __yield$number(i); }); }
//! ```
//!
//! and the classes `Generator$K`, `Iterator$K`, `IteratorResult$K` and the helpers `__genNew$K`,
//! `__yield$K` are written as TypeScript text for each yield type `K` the program mentions, in front
//! of the program. They call a few intrinsics (`__coNew`, `__coResume`, `__coYield<T>`, `__coValue<T>`,
//! `__coError`, `__coFinish`, lowered in `strings.rs` -> `lower_coro_intrinsic`). Closures, captured
//! variables, `this`, `try`/`finally` and exceptions are the ordinary machinery. Limits: the return
//! type must be written (`Generator<T>`), `yield*` is a statement, and `return value` needs `Generator<T, T>`.
//! `throw(e)` and `return(v)` set an error at the suspended `yield` (`ts_rt_co_inject_set`); `return` uses
//! a marker class that `catch` clauses rewritten into the generator body rethrow.

use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_ast_visit::{Visit, walk};
use oxc_parser::Parser;
use oxc_span::{GetSpan, SourceType};
use oxc_syntax::scope::ScopeFlags;
use std::collections::{BTreeMap, HashSet};

/// The result of rewriting a program.
pub(crate) struct Desugared {
    pub src: String,
    /// TypeScript text for the generator classes, to go in front of the program.
    pub templates: String,
}

/// The type names the generator classes stand for.
pub(crate) const FAMILY: &[&str] = &[
    "Generator",
    "IterableIterator",
    "Iterable",
    "Iterator",
    "IteratorResult",
];

/// One yield type: the key in class names, the yield type's text and the sent type's text.
#[derive(Clone)]
struct Inst {
    key: String,
    x: String,
    n: Option<String>,
}

pub(crate) fn mangle(text: &str) -> String {
    text.chars()
        .filter(|c| !c.is_whitespace())
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect()
}

/// The class-name key for the type arguments of `Generator<X, R, N>` and friends.
pub(crate) fn key_of(x: &str, n: Option<&str>) -> String {
    let x = if matches!(x.trim(), "void" | "undefined") {
        "number"
    } else {
        x
    };
    match n {
        Some(n) if !matches!(n.trim(), "undefined" | "void" | "unknown" | "any") => {
            format!("{}${}", mangle(x), mangle(n))
        }
        _ => mangle(x),
    }
}

struct Pass<'s> {
    src: &'s str,
    edits: Vec<(usize, usize, String)>,
    insts: BTreeMap<String, Inst>,
    /// The key of each generator being walked, innermost last.
    gens: Vec<String>,
    /// Starts of `yield*` expression statements already rewritten whole.
    stmt_yields: HashSet<u32>,
    star_from: Option<u32>,
    error: Option<String>,
}

impl Pass<'_> {
    fn text(&self, span: oxc_span::Span) -> &str {
        &self.src[span.start as usize..span.end as usize]
    }

    fn inst_of(&mut self, args: &TSTypeParameterInstantiation<'_>) -> Option<Inst> {
        let x = self.text(args.params.first()?.span()).to_string();
        let n = args.params.get(2).map(|n| self.text(n.span()).to_string());
        // The return type must be the yield type or nothing: one `value` field holds both.
        if let Some(r) = args.params.get(1) {
            let r = self.text(r.span()).trim().to_string();
            if !matches!(r.as_str(), "void" | "undefined" | "any" | "unknown") && r != x.trim() {
                self.fail(format!(
                    "a generator's return type must be its yield type or void, not `{r}`"
                ));
            }
        }
        let key = key_of(&x, n.as_deref());
        let x = if matches!(x.trim(), "void" | "undefined") {
            "number".to_string()
        } else {
            x
        };
        let n = n.filter(|n| !matches!(n.trim(), "undefined" | "void" | "unknown" | "any"));
        let inst = Inst {
            key: key.clone(),
            x,
            n,
        };
        self.insts.entry(key).or_insert_with(|| inst.clone());
        Some(inst)
    }

    fn fail(&mut self, msg: String) {
        self.error.get_or_insert(msg);
    }

    fn begin(&mut self, f: &Function<'_>) {
        if f.r#async {
            self.fail(format!(
                "async generators (`async function*`, `AsyncGenerator`, `for await` over one) are not supported: use an async function that returns an array, or a sync generator (at offset {})",
                f.span.start
            ));
            return;
        }
        let inst = f
            .return_type
            .as_ref()
            .and_then(|r| match &r.type_annotation {
                TSType::TSTypeReference(t) => match &t.type_name {
                    TSTypeName::IdentifierReference(id) if FAMILY.contains(&id.name.as_str()) => {
                        t.type_arguments.as_ref().and_then(|a| self.inst_of(a))
                    }
                    _ => None,
                },
                _ => None,
            });
        let Some(inst) = inst else {
            self.fail(format!(
                "a generator needs its return type written, e.g. `Generator<number>` (at offset {})",
                f.span.start
            ));
            self.gens.push(String::new());
            return;
        };
        let from = self.star_from.take().unwrap_or(f.span.start) as usize;
        let to = f.params.span.start as usize;
        match self.src[from..to.max(from)].find('*') {
            Some(i) => self.edits.push((from + i, from + i + 1, String::new())),
            None => self.fail("could not find the `*` of a generator".into()),
        }
        if let Some(body) = &f.body {
            let (open, close) = (body.span.start as usize + 1, body.span.end as usize - 1);
            self.edits.push((
                open,
                open,
                format!(" return __genNew${}(() => {{", inst.key),
            ));
            self.edits.push((close, close, "}); ".to_string()));
        }
        self.gens.push(inst.key);
    }
}

impl<'a> Visit<'a> for Pass<'_> {
    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        if it.generator {
            self.begin(it);
            walk::walk_function(self, it, flags);
            self.gens.pop();
        } else {
            self.star_from = None;
            // A nested plain function has no `yield` of its own.
            let saved = std::mem::take(&mut self.gens);
            walk::walk_function(self, it, flags);
            self.gens = saved;
        }
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        let saved = std::mem::take(&mut self.gens);
        walk::walk_arrow_function_expression(self, it);
        self.gens = saved;
    }

    fn visit_method_definition(&mut self, it: &MethodDefinition<'a>) {
        if it.value.generator {
            self.star_from = Some(it.span.start);
        }
        walk::walk_method_definition(self, it);
    }

    fn visit_object_property(&mut self, it: &ObjectProperty<'a>) {
        if let Expression::FunctionExpression(f) = &it.value
            && f.generator
        {
            self.star_from = Some(it.span.start);
        }
        walk::walk_object_property(self, it);
    }

    fn visit_ts_type_reference(&mut self, it: &TSTypeReference<'a>) {
        if let TSTypeName::IdentifierReference(id) = &it.type_name
            && FAMILY.contains(&id.name.as_str())
            && let Some(args) = &it.type_arguments
        {
            self.inst_of(args);
        }
        walk::walk_ts_type_reference(self, it);
    }

    fn visit_return_statement(&mut self, it: &ReturnStatement<'a>) {
        if let (Some(key), Some(arg)) = (self.gens.last().cloned(), &it.argument) {
            let text = self.text(arg.span()).to_string();
            if text.contains("yield") {
                self.fail("`return yield ...` is not supported; use a variable".into());
                return;
            }
            self.edits.push((
                it.span.start as usize,
                it.span.end as usize,
                format!("{{ __genRet${key}({text}); return; }}"),
            ));
            return;
        }
        walk::walk_return_statement(self, it);
    }

    // `return()` is raised at the `yield` as a marker error so `finally` blocks run; a `catch`
    // in the generator must not swallow it.
    fn visit_catch_clause(&mut self, it: &CatchClause<'a>) {
        if !self.gens.is_empty() {
            let body = it.body.span.start as usize;
            match it.param.as_ref().map(|p| &p.pattern) {
                Some(BindingPattern::BindingIdentifier(id)) => {
                    let n = id.name.as_str();
                    self.edits.push((
                        body + 1,
                        body + 1,
                        format!(" if ({n} instanceof __GenReturn) {{ throw {n}; }}"),
                    ));
                }
                Some(_) => self.fail("a catch pattern in a generator must be a name".into()),
                None => {
                    self.edits.push((body, body, "(__c) ".to_string()));
                    self.edits.push((
                        body + 1,
                        body + 1,
                        " if (__c instanceof __GenReturn) { throw __c; }".to_string(),
                    ));
                }
            }
        }
        walk::walk_catch_clause(self, it);
    }

    fn visit_expression_statement(&mut self, it: &ExpressionStatement<'a>) {
        if let Expression::YieldExpression(y) = &it.expression
            && y.delegate
            && let Some(key) = self.gens.last().cloned()
        {
            // `yield* e;` becomes a loop that yields each element.
            let arg = y.argument.as_ref().map(|a| self.text(a.span()).to_string());
            match arg {
                Some(arg) => {
                    self.edits.push((
                        it.span.start as usize,
                        it.span.end as usize,
                        format!("for (const __y of ({arg})) {{ __yield${key}(__y); }}"),
                    ));
                    self.stmt_yields.insert(y.span.start);
                }
                None => self.fail("`yield*` needs an operand".into()),
            }
        }
        walk::walk_expression_statement(self, it);
    }

    fn visit_yield_expression(&mut self, it: &YieldExpression<'a>) {
        if self.stmt_yields.contains(&it.span.start) {
            return;
        }
        let Some(key) = self.gens.last().cloned() else {
            return;
        };
        if it.delegate {
            self.fail("`yield*` is supported as a statement (`yield* other();`) only".into());
            return;
        }
        let arg = match &it.argument {
            Some(a) => self.text(a.span()).to_string(),
            None => String::new(),
        };
        // A yield inside the operand of another is rewritten with the outer one; keep it simple.
        if arg.contains("yield") {
            self.fail(
                "a `yield` inside another `yield`'s operand is not supported; use a variable"
                    .into(),
            );
            return;
        }
        self.edits.push((
            it.span.start as usize,
            it.span.end as usize,
            format!("__yield${key}({arg})"),
        ));
    }
}

fn template(i: &Inst) -> String {
    let k = &i.key;
    let x = &i.x;
    let (sent_param, resume_args, yield_ret, yield_body) = match &i.n {
        Some(n) => (
            format!("sent?: {n}"),
            "this.co, sent".to_string(),
            n.clone(),
            format!(
                "const sent = __coYield<{n}>(v); const inj = __coInject(); if (inj !== null) throw inj as Error; return sent;"
            ),
        ),
        None => (
            String::new(),
            "this.co".to_string(),
            "void".to_string(),
            "__coYield(v); const inj = __coInject(); if (inj !== null) throw inj as Error;"
                .to_string(),
        ),
    };
    format!(
        "interface IteratorResult${k} {{ done: boolean; value: {x}; }}
interface Iterator${k} {{ next: () => IteratorResult${k}; }}
class __GenReturn${k} extends __GenReturn {{
  value: {x};
  constructor(v: {x}) {{ super(\"return\"); this.value = v; }}
}}
class Generator${k} {{
  co: Coroutine;
  constructor(co: Coroutine) {{ this.co = co; }}
  next({sent_param}): IteratorResult${k} {{
    const finished = __coResume({resume_args});
    const err = __coError();
    if (err !== null) {{
      if (err instanceof __GenReturn${k}) {{
        const r: IteratorResult${k} = {{ done: true, value: (err as __GenReturn${k}).value }};
        return r;
      }}
      throw err as Error;
    }}
    const r: IteratorResult${k} = {{ done: finished, value: __coValue<{x}>(this.co) }};
    return r;
  }}
  return(value?: {x}): IteratorResult${k} {{
    let v = __coZero<{x}>();
    if (value !== undefined) v = value;
    if (!__coStarted(this.co) || __coDone(this.co)) {{
      __coFinish(this.co);
      const r: IteratorResult${k} = {{ done: true, value: v }};
      return r;
    }}
    __coInjectSet(this.co, new __GenReturn${k}(v));
    return this.next();
  }}
  throw(e: Error): IteratorResult${k} {{
    if (!__coStarted(this.co) || __coDone(this.co)) {{
      __coFinish(this.co);
      throw e;
    }}
    __coInjectSet(this.co, e);
    return this.next();
  }}
  [Symbol.iterator](): Generator${k} {{ return this; }}
}}
function __genNew${k}(body: () => void): Generator${k} {{ return new Generator${k}(__coNew(body)); }}
function __yield${k}(v: {x}): {yield_ret} {{ {yield_body} }}
function __genRet${k}(v: {x}): void {{ __coRet<{x}>(v); }}
"
    )
}

/// Rewrites the generators of `src`; None when the program has no generator or iterator type.
pub(crate) fn desugar(src: &str) -> Result<Option<Desugared>, String> {
    if !src.contains("function*")
        && !src.contains("function *")
        && !src.contains('*')
        && !FAMILY.iter().any(|f| src.contains(&format!("{f}<")))
    {
        return Ok(None);
    }
    let alloc = Allocator::default();
    let ret = Parser::new(&alloc, src, SourceType::ts()).parse();
    if !ret.diagnostics.is_empty() {
        // The normal pipeline reports parse errors.
        return Ok(None);
    }
    let mut pass = Pass {
        src,
        edits: Vec::new(),
        insts: BTreeMap::new(),
        gens: Vec::new(),
        stmt_yields: HashSet::new(),
        star_from: None,
        error: None,
    };
    pass.visit_program(&ret.program);
    if let Some(e) = pass.error {
        return Err(e);
    }
    if pass.insts.is_empty() && pass.edits.is_empty() {
        return Ok(None);
    }
    let mut edits = pass.edits;
    edits.sort_by_key(|e| (e.0, e.1));
    let mut out = String::with_capacity(src.len() + 256);
    let mut at = 0;
    for (s, e, text) in &edits {
        if *s < at {
            return Err("overlapping generator rewrites".into());
        }
        out.push_str(&src[at..*s]);
        out.push_str(text);
        at = *e;
    }
    out.push_str(&src[at..]);
    let templates: String = pass.insts.values().map(template).collect();
    Ok(Some(Desugared {
        src: out,
        templates,
    }))
}

// ---- the intrinsics the generator classes call ----

use super::expr::mk;
use super::{Expr, ExprKind, Lowerer, R, Stmt, Ty};

impl<'a> Lowerer<'a> {
    /// A value as the runtime's raw word, and whether that word is a pointer.
    fn coro_slot(&mut self, v: Expr) -> (Expr, Expr) {
        let v = if v.ty.strip() == &Ty::Bool {
            self.bool_as_num(v)
        } else {
            v
        };
        let is_ptr = v.ty.strip().is_ptr();
        let slot = mk(ExprKind::Bits(Box::new(v)), Ty::Handle("bits".into()));
        (slot, mk(ExprKind::Bool(is_ptr), Ty::Bool))
    }

    /// `__coNew`, `__coResume`, `__coYield`, `__coValue`, `__coError`, `__coFinish`.
    pub(crate) fn lower_coro_intrinsic(
        &mut self,
        name: &str,
        c: &'a CallExpression<'a>,
    ) -> R<Option<Expr>> {
        let at = self.loc(c.span.start);
        let args: Vec<&'a Expression<'a>> = c
            .arguments
            .iter()
            .filter_map(|a| a.as_expression())
            .collect();
        // Only programs with coroutines have the class.
        let co_ty = match self.named.get("Coroutine") {
            Some(Ty::Obj(s)) => Ty::Obj(*s),
            _ => Ty::Void,
        };
        let type_arg = match c.type_arguments.as_ref().and_then(|a| a.params.first()) {
            Some(t) => Some(self.ty(t, "gen")?),
            None => None,
        };
        let bits = Ty::Handle("bits".into());
        Ok(Some(match name {
            "__coNew" => {
                let Some(body) = args.first() else {
                    bail!("{at}: __coNew needs the body")
                };
                let body = self.lower_expr(body, None)?;
                self.rt_str("ts_rt_co_new_gc", vec![body], co_ty)
            }
            "__coResume" => {
                let Some(co) = args.first() else {
                    bail!("{at}: __coResume needs the coroutine")
                };
                let co = self.lower_expr(co, None)?;
                let (slot, flag) = match args.get(1) {
                    Some(s) => {
                        let v = self.lower_expr(s, None)?;
                        self.coro_slot(v)
                    }
                    None => (
                        mk(ExprKind::Bits(Box::new(super::expr::num(0.0))), bits),
                        mk(ExprKind::Bool(false), Ty::Bool),
                    ),
                };
                self.rt_str("ts_rt_co_resume", vec![co, slot, flag], Ty::Bool)
            }
            "__coYield" => {
                let Some(v) = args.first() else {
                    bail!("{at}: __coYield needs a value")
                };
                let v = self.lower_expr(v, None)?;
                let (slot, flag) = self.coro_slot(v);
                let call = self.rt_str("ts_rt_co_yield", vec![slot, flag], bits);
                match type_arg {
                    Some(t) => mk(ExprKind::Unbits(Box::new(call)), t),
                    None => mk(
                        ExprKind::Seq(vec![Stmt::Eval(call)], Box::new(super::expr::num(0.0))),
                        Ty::Void,
                    ),
                }
            }
            "__coValue" => {
                let (Some(co), Some(t)) = (args.first(), type_arg) else {
                    bail!("{at}: __coValue<T>(co)")
                };
                let co = self.lower_expr(co, None)?;
                let raw = self.rt_str("ts_rt_co_value", vec![co], bits);
                mk(ExprKind::Unbits(Box::new(raw)), t)
            }
            "__coError" => {
                let err = Ty::Obj(self.classes_sid("Error")).nullable();
                self.rt_str("ts_rt_exc_take", vec![], err)
            }
            "__coInject" => {
                let err = Ty::Obj(self.classes_sid("Error")).nullable();
                self.rt_str("ts_rt_co_inject_take", vec![], err)
            }
            "__coInjectSet" => {
                let (Some(co), Some(e)) = (args.first(), args.get(1)) else {
                    bail!("{at}: __coInjectSet(co, error)")
                };
                let co = self.lower_expr(co, None)?;
                let e = self.lower_expr(e, None)?;
                self.rt_str("ts_rt_co_inject_set", vec![co, e], Ty::Void)
            }
            "__coRet" => {
                let Some(v) = args.first() else {
                    bail!("{at}: __coRet needs a value")
                };
                let v = self.lower_expr(v, None)?;
                let (slot, flag) = self.coro_slot(v);
                self.rt_str("ts_rt_co_set_return", vec![slot, flag], Ty::Void)
            }
            "__coStarted" | "__coDone" => {
                let Some(co) = args.first() else {
                    bail!("{at}: {name} needs the coroutine")
                };
                let co = self.lower_expr(co, None)?;
                let f = if name == "__coStarted" {
                    "ts_rt_co_started"
                } else {
                    "ts_rt_co_done"
                };
                self.rt_str(f, vec![co], Ty::Bool)
            }
            "__coZero" => {
                let Some(t) = type_arg else {
                    bail!("{at}: __coZero<T>()")
                };
                mk(
                    ExprKind::Unbits(Box::new(mk(
                        ExprKind::Bits(Box::new(super::expr::num(0.0))),
                        bits,
                    ))),
                    t,
                )
            }
            "__dvGet" | "__dvSet" => {
                let n = if name == "__dvGet" { 4 } else { 5 };
                if args.len() != n {
                    bail!("{at}: {name} takes {n} arguments")
                }
                let mut v = Vec::new();
                for a in &args {
                    v.push(self.lower_expr(a, None)?);
                }
                if name == "__dvGet" {
                    let call = self.rt_str("ts_rt_dv_get", v, Ty::Num);
                    self.checked(call, Ty::Num)
                } else {
                    let call = self.rt_str("ts_rt_dv_set", v, Ty::Void);
                    self.checked(call, Ty::Void)
                }
            }
            "__coCurrent" => self.rt_str("ts_rt_co_current", vec![], co_ty),
            "__queueMicrotask" => {
                let Some(cb) = args.first() else {
                    bail!("{at}: __queueMicrotask needs a callback")
                };
                let cb = self.lower_expr(cb, None)?;
                self.rt_str("ts_rt_microtask_push", vec![cb], Ty::Void)
            }
            "__afterMicrotasks" => {
                let Some(cb) = args.first() else {
                    bail!("{at}: __afterMicrotasks needs a callback")
                };
                let cb = self.lower_expr(cb, None)?;
                self.rt_str("ts_rt_after_microtasks", vec![cb], Ty::Void)
            }
            "__timerAdd" => {
                let (Some(cb), Some(ms), Some(rep)) = (args.first(), args.get(1), args.get(2))
                else {
                    bail!("{at}: __timerAdd(callback, ms, repeat)")
                };
                let cb = self.lower_expr(cb, None)?;
                let ms = self.lower_expr(ms, None)?;
                let rep = self.lower_expr(rep, None)?;
                self.rt_str("ts_rt_timer_add", vec![cb, ms, rep], Ty::Num)
            }
            "__timerClear" => {
                let Some(id) = args.first() else {
                    bail!("{at}: __timerClear needs an id")
                };
                let id = self.lower_expr(id, None)?;
                self.rt_str("ts_rt_timer_clear", vec![id], Ty::Void)
            }
            "__coFinish" => {
                let Some(co) = args.first() else {
                    bail!("{at}: __coFinish needs the coroutine")
                };
                let co = self.lower_expr(co, None)?;
                self.rt_str("ts_rt_co_finish", vec![co], Ty::Void)
            }
            _ => return Ok(None),
        }))
    }
}
