// SPDX-License-Identifier: Apache-2.0

//! `Promise`, `async` and `await` by source rewriting plus classes written per value type, the way
//! generators are (`generators.rs`).
//!
//! * `Promise<T>` is the class `Promise$K` (`K` is the mangled text of `T`; `void` shares `number`'s
//!   class): state, value, reason and the reactions waiting for it. Its helpers are plain functions
//!   (`__fulfill$K`, `__then$K$V`, ...), written as TypeScript text in front of the program for
//!   every `K` the program mentions (plus `number`, `string`, `boolean`).
//! * `async function f(): Promise<T> { ... }` becomes `function f(): Promise<T> { return
//!   __async$K((__p) => { ... }); }`: the body runs as a coroutine (`coro.rs`) until its first
//!   `await`; `return x` becomes `__asyncRet$K(__p, x)`; `await e` becomes `__await(e)`, which the
//!   lowerer (`promise.rs`) points at `__await$K` once it knows the operand's type.
//! * `new Promise<T>(executor)` becomes `__newPromise$K(executor)`.
//! * `.then`, `.catch`, `.finally`, `Promise.resolve/reject/all/race` are routed by the lowerer
//!   (`promise.rs`), which knows the types.
//!
//! The microtask queue and the timers live in the runtime (`platypus_ts_rt/src/event.rs`).

use super::generators::{key_of, mangle};
use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_ast_visit::{Visit, walk};
use oxc_parser::Parser;
use oxc_span::{GetSpan, SourceType};
use oxc_syntax::scope::ScopeFlags;
use std::collections::BTreeMap;

pub(crate) struct Desugared {
    pub src: String,
    pub templates: String,
    /// The program has an `await` at the top level of a module: its top-level code is the body of
    /// an implicit async function (see `Lowerer::top_await`).
    pub top_await: bool,
}

/// The value types every program with promises gets a class for.
const BASE: [&str; 3] = ["number", "string", "boolean"];

/// One async function being walked: its promise key and whether its return type was written.
struct Entry {
    key: String,
    typed: bool,
    /// The written value type when it mentions a type parameter (`Promise<T>` in `f<T>`): its
    /// class is only known per instantiation, so the lowerer picks it (`__asyncG<T>`).
    generic: Option<String>,
}

struct Pass<'s> {
    src: &'s str,
    edits: Vec<(usize, usize, String)>,
    /// Value types seen: class key to type text.
    types: BTreeMap<String, String>,
    asyncs: Vec<Option<Entry>>,
    /// The type parameters of the functions and classes being walked, innermost last.
    tparams: Vec<Vec<String>>,
    /// How many functions the walk is inside (0: module level).
    fn_depth: u32,
    /// A promise type written with a type parameter exists: any class may be its value.
    generic_used: bool,
    /// Non-generic classes and interfaces declared, for those promises.
    decl_types: Vec<String>,
    async_from: Option<u32>,
    /// A `.then(f, g)` call exists.
    then2: bool,
    /// An `await` or `for await` outside every function.
    top_await: bool,
    error: Option<String>,
}

/// The span of `T` in a written `Promise<T>`.
fn promise_arg(t: &TSType<'_>) -> Option<oxc_span::Span> {
    let TSType::TSTypeReference(r) = t else {
        return None;
    };
    let TSTypeName::IdentifierReference(id) = &r.type_name else {
        return None;
    };
    (id.name == "Promise")
        .then(|| r.type_arguments.as_ref()?.params.first().map(|a| a.span()))
        .flatten()
}

fn type_text(x: &str) -> String {
    if matches!(x.trim(), "void" | "undefined") {
        "number".into()
    } else {
        x.trim().into()
    }
}

impl<'a> Pass<'_> {
    /// `for await (const x of xs) body` over an array or other iterable (promises or plain
    /// values): `for (const __faN of xs) { const x = __await(__faN); body }`.
    fn for_await(&mut self, it: &ForOfStatement<'a>) {
        if !matches!(self.asyncs.last(), Some(Some(_))) {
            if self.asyncs.is_empty() && self.fn_depth == 0 {
                self.top_await = true;
            } else {
                self.fail("`for await` is only supported inside an async function".into());
                return;
            }
        }
        let ForStatementLeft::VariableDeclaration(decl) = &it.left else {
            self.fail("`for await` needs a declaration: `for await (const x of xs)`".into());
            return;
        };
        let Some(d) = decl.declarations.first() else {
            self.fail("`for await` needs a declaration".into());
            return;
        };
        if d.type_annotation.is_some() {
            self.fail("`for await (const x: T of ..)`: remove the annotation".into());
            return;
        }
        let kind = decl.kind.as_str();
        let id_span = d.id.span();
        let pattern = self.text(id_span).to_string();
        let tmp = format!("__fa{}", it.span.start);
        let bind = format!(" {kind} {pattern} = __await({tmp});");
        self.edits.push((
            it.span.start as usize,
            decl.span.start as usize,
            "for (".into(),
        ));
        self.edits
            .push((id_span.start as usize, id_span.end as usize, tmp));
        match &it.body {
            Statement::BlockStatement(b) => {
                let at = b.span.start as usize + 1;
                self.edits.push((at, at, bind));
            }
            other => {
                let sp = other.span();
                self.edits
                    .push((sp.start as usize, sp.start as usize, format!("{{{bind} ")));
                self.edits
                    .push((sp.end as usize, sp.end as usize, " }".into()));
            }
        }
        self.visit_expression(&it.right);
        self.visit_statement(&it.body);
    }

    fn text(&self, span: oxc_span::Span) -> &str {
        &self.src[span.start as usize..span.end as usize]
    }

    fn fail(&mut self, msg: String) {
        self.error.get_or_insert(msg);
    }

    /// Whether a written type mentions a type parameter in scope.
    fn is_generic(&self, x: &str) -> bool {
        x.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '$'))
            .any(|w| self.tparams.iter().flatten().any(|t| t == w))
    }

    fn push_tparams(&mut self, d: Option<&TSTypeParameterDeclaration<'_>>) {
        self.tparams.push(
            d.map(|d| d.params.iter().map(|p| p.name.name.to_string()).collect())
                .unwrap_or_default(),
        );
    }

    fn add_type(&mut self, x: &str) -> String {
        let key = key_of(x, None);
        if x.trim_start().starts_with("Promise<") {
            self.fail(format!(
                "`Promise<{x}>`: a promise of a promise is not supported (promises flatten: write the inner type)"
            ));
        }
        if self.is_generic(x) {
            self.generic_used = true;
            return key;
        }
        self.types
            .entry(key.clone())
            .or_insert_with(|| type_text(x));
        key
    }

    /// The key of `Promise<T>` in a written return type.
    fn promise_key(&mut self, t: &TSType<'_>) -> Option<String> {
        let TSType::TSTypeReference(r) = t else {
            return None;
        };
        let TSTypeName::IdentifierReference(id) = &r.type_name else {
            return None;
        };
        if id.name != "Promise" {
            return None;
        }
        let arg = r.type_arguments.as_ref()?.params.first()?;
        let x = self.text(arg.span()).to_string();
        Some(self.add_type(&x))
    }

    fn begin_async(
        &mut self,
        ret: Option<&TSTypeAnnotation<'_>>,
        generator: bool,
        at: u32,
    ) -> Option<Entry> {
        if generator {
            self.fail("async generators are not supported".into());
            return None;
        }
        let key = ret.and_then(|r| self.promise_key(&r.type_annotation));
        let generic = ret
            .and_then(|r| promise_arg(&r.type_annotation))
            .map(|sp| self.text(sp).to_string())
            .filter(|x| self.is_generic(x));
        if ret.is_some() && key.is_none() {
            self.fail(format!(
                "an async function must return `Promise<T>` (at offset {at})"
            ));
            return None;
        }
        Some(match key {
            Some(key) => Entry {
                key,
                typed: true,
                generic,
            },
            None => Entry {
                key: self.add_type("void"),
                typed: false,
                generic: None,
            },
        })
    }

    /// Removes the `async` keyword found between `from` and `to`.
    fn drop_async(&mut self, from: usize, to: usize) {
        match self.src[from..to.max(from)].find("async") {
            Some(i) => self.edits.push((from + i, from + i + 5, String::new())),
            None => self.fail("could not find the `async` keyword".into()),
        }
    }

    /// The call that starts the coroutine of an async function.
    fn async_head(e: &Entry) -> String {
        match &e.generic {
            Some(x) => format!("__asyncG<{x}>((__p: Promise<{x}>) => {{"),
            None => format!("__async${}((__p) => {{", e.key),
        }
    }

    /// What `return e` calls (the lowerer reads the promise's class from `__p`).
    fn ret_name(e: &Entry) -> String {
        match &e.generic {
            Some(_) => "__asyncRet$G".into(),
            None => format!("__asyncRet${}", e.key),
        }
    }

    /// Wraps a block body (`{ ... }`) in the coroutine closure.
    fn wrap_block(&mut self, body: oxc_span::Span, e: &Entry) {
        let (open, close) = (body.start as usize + 1, body.end as usize - 1);
        self.edits
            .push((open, open, format!(" return {}", Self::async_head(e))));
        self.edits.push((close, close, "}); ".into()));
    }

    fn visit_function_inner(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        if it.r#async {
            let entry = self.begin_async(it.return_type.as_deref(), it.generator, it.span.start);
            if let (Some(e), Some(body)) = (&entry, &it.body) {
                let from = self.async_from.take().unwrap_or(it.span.start) as usize;
                self.drop_async(from, it.params.span.start as usize);
                self.wrap_block(body.span, e);
            }
            self.asyncs.push(entry);
            walk::walk_function(self, it, flags);
            self.asyncs.pop();
        } else {
            self.async_from = None;
            let saved = std::mem::take(&mut self.asyncs);
            walk::walk_function(self, it, flags);
            self.asyncs = saved;
        }
    }

    fn visit_arrow_inner(&mut self, it: &ArrowFunctionExpression<'a>) {
        if it.r#async {
            let entry = self.begin_async(it.return_type.as_deref(), false, it.span.start);
            let mut closer = None;
            if let Some(e) = &entry {
                let params_end = it.params.span.end as usize;
                self.drop_async(
                    it.span.start as usize,
                    params_end.max(it.span.start as usize),
                );
                match &it.body {
                    ArrowFunctionBody::FunctionBody(b) => self.wrap_block(b.span, e),
                    other => {
                        let b = other.as_expression().map(|x| x.span());
                        let Some(b) = b else {
                            self.fail("unsupported async arrow body".into());
                            return;
                        };
                        self.edits.push((
                            b.start as usize,
                            b.start as usize,
                            format!("{} {}(__p, ", Self::async_head(e), Self::ret_name(e)),
                        ));
                        closer = Some(b.end as usize);
                    }
                }
            }
            self.asyncs.push(entry);
            walk::walk_arrow_function_expression(self, it);
            self.asyncs.pop();
            if let Some(end) = closer {
                self.edits.push((end, end, "); })".into()));
            }
        } else {
            let saved = std::mem::take(&mut self.asyncs);
            walk::walk_arrow_function_expression(self, it);
            self.asyncs = saved;
        }
    }
}

impl<'a> Visit<'a> for Pass<'_> {
    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        self.push_tparams(it.type_parameters.as_deref());
        self.fn_depth += 1;
        self.visit_function_inner(it, flags);
        self.fn_depth -= 1;
        self.tparams.pop();
    }

    fn visit_ts_interface_declaration(&mut self, it: &TSInterfaceDeclaration<'a>) {
        if it.type_parameters.is_none() {
            self.decl_types.push(it.id.name.to_string());
        }
        walk::walk_ts_interface_declaration(self, it);
    }

    fn visit_class(&mut self, it: &Class<'a>) {
        if let (Some(id), None) = (&it.id, &it.type_parameters) {
            self.decl_types.push(id.name.to_string());
        }
        self.push_tparams(it.type_parameters.as_deref());
        walk::walk_class(self, it);
        self.tparams.pop();
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        self.push_tparams(it.type_parameters.as_deref());
        self.fn_depth += 1;
        self.visit_arrow_inner(it);
        self.fn_depth -= 1;
        self.tparams.pop();
    }

    fn visit_method_definition(&mut self, it: &MethodDefinition<'a>) {
        if it.value.r#async {
            self.async_from = Some(it.span.start);
        }
        walk::walk_method_definition(self, it);
    }

    fn visit_object_property(&mut self, it: &ObjectProperty<'a>) {
        if let Expression::FunctionExpression(f) = &it.value
            && f.r#async
        {
            self.async_from = Some(it.span.start);
        }
        walk::walk_object_property(self, it);
    }

    fn visit_ts_type_reference(&mut self, it: &TSTypeReference<'a>) {
        if let TSTypeName::IdentifierReference(id) = &it.type_name
            && id.name == "Promise"
            && let Some(args) = &it.type_arguments
            && let Some(a) = args.params.first()
        {
            let x = self.text(a.span()).to_string();
            self.add_type(&x);
        }
        walk::walk_ts_type_reference(self, it);
    }

    fn visit_return_statement(&mut self, it: &ReturnStatement<'a>) {
        if let Some(Some(entry)) = self.asyncs.last()
            && let Some(arg) = &it.argument
        {
            if !entry.typed {
                self.fail(
                    "an async function that returns a value needs its type written, e.g. `Promise<number>`"
                        .into(),
                );
                return;
            }
            let ret = Self::ret_name(entry);
            self.edits.push((
                it.span.start as usize,
                arg.span().start as usize,
                format!("{{ {ret}(__p, "),
            ));
            walk::walk_return_statement(self, it);
            // After the operand's own edits, so a closing `)` of an `await` comes first.
            self.edits.push((
                arg.span().end as usize,
                arg.span().end as usize,
                "); return; }".into(),
            ));
            return;
        }
        walk::walk_return_statement(self, it);
    }

    fn visit_await_expression(&mut self, it: &AwaitExpression<'a>) {
        if !matches!(self.asyncs.last(), Some(Some(_))) {
            if self.asyncs.is_empty() && self.fn_depth == 0 {
                // Top-level `await`: the module's code runs as an implicit async function.
                self.top_await = true;
            } else {
                self.fail("`await` is only supported inside an async function".into());
                return;
            }
        }
        let a = it.argument.span();
        self.edits
            .push((it.span.start as usize, a.start as usize, "__await(".into()));
        self.edits
            .push((a.end as usize, a.end as usize, ")".into()));
        walk::walk_await_expression(self, it);
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        // `Promise.withResolvers<T>()`.
        if let Expression::StaticMemberExpression(m) = &it.callee
            && m.property.name == "withResolvers"
            && matches!(&m.object, Expression::Identifier(id) if id.name == "Promise")
        {
            match &it.type_arguments {
                Some(ta) if !ta.params.is_empty() => {
                    let x = self.text(ta.params[0].span()).to_string();
                    let key = self.add_type(&x);
                    let name = if self.is_generic(&x) {
                        self.fail(
                            "Promise.withResolvers<T> inside a generic function is not supported; use `new Promise<T>(..)`"
                                .into(),
                        );
                        return;
                    } else {
                        format!("__withResolvers${key}")
                    };
                    self.edits
                        .push((m.span.start as usize, ta.span.end as usize, name));
                }
                _ => self.fail(
                    "`Promise.withResolvers` needs its type written, e.g. `Promise.withResolvers<number>()`"
                        .into(),
                ),
            }
        }
        if let Expression::StaticMemberExpression(m) = &it.callee
            && m.property.name == "then"
            && it.arguments.len() == 2
        {
            self.then2 = true;
        }
        walk::walk_call_expression(self, it);
    }

    fn visit_for_of_statement(&mut self, it: &ForOfStatement<'a>) {
        if it.r#await {
            self.for_await(it);
            return;
        }
        walk::walk_for_of_statement(self, it);
    }

    fn visit_new_expression(&mut self, it: &NewExpression<'a>) {
        if let Expression::Identifier(id) = &it.callee
            && id.name == "Promise"
        {
            match &it.type_arguments {
                Some(ta) if !ta.params.is_empty() => {
                    let x = self.text(ta.params[0].span()).to_string();
                    let key = self.add_type(&x);
                    let name = if self.is_generic(&x) {
                        format!("__newPromiseG<{x}>")
                    } else {
                        format!("__newPromise${key}")
                    };
                    self.edits
                        .push((it.span.start as usize, ta.span.end as usize, name));
                }
                _ => self.fail(
                    "`new Promise` needs its type written, e.g. `new Promise<number>(...)`".into(),
                ),
            }
        }
        walk::walk_new_expression(self, it);
    }
}

/// Rewrites the async functions and promises of `src`; None when the program uses none.
pub(crate) fn desugar(src: &str) -> Result<Option<Desugared>, String> {
    if !src.contains("Promise") && !src.contains("async") && !src.contains("await") {
        return Ok(None);
    }
    let alloc = Allocator::default();
    let ret = Parser::new(&alloc, src, SourceType::ts()).parse();
    if !ret.diagnostics.is_empty() {
        return Ok(None);
    }
    let mut pass = Pass {
        src,
        edits: Vec::new(),
        types: BTreeMap::new(),
        asyncs: Vec::new(),
        tparams: Vec::new(),
        fn_depth: 0,
        generic_used: false,
        decl_types: Vec::new(),
        async_from: None,
        then2: false,
        top_await: false,
        error: None,
    };
    pass.visit_program(&ret.program);
    if let Some(e) = pass.error {
        return Err(e);
    }
    if pass.types.is_empty() && pass.edits.is_empty() {
        return Ok(None);
    }
    for b in BASE {
        pass.add_type(b);
    }
    // A generic function can make a promise of any type its callers use: give the program's own
    // classes and the plain arrays a promise class each.
    if pass.generic_used {
        let mut more: Vec<String> = BASE.iter().map(|b| format!("{b}[]")).collect();
        for d in std::mem::take(&mut pass.decl_types) {
            more.push(format!("{d}[]"));
            more.push(d);
        }
        for t in more {
            pass.add_type(&t);
        }
    }
    // The types the program wrote; `then` can return these. The arrays below are derived.
    let core: Vec<String> = pass.types.keys().cloned().collect();
    let base: Vec<String> = pass.types.values().cloned().collect();
    // `Promise.all` makes an array of each value, `Promise.allSettled` an array of results.
    let wants_all = src.contains("Promise.all(");
    let wants_settled = src.contains("Promise.allSettled");
    for x in &base {
        if wants_all {
            pass.add_type(&format!("{x}[]"));
        }
        if wants_settled {
            pass.add_type(&format!("PromiseSettledResult<{x}>[]"));
        }
    }
    let mut edits = pass.edits;
    edits.sort_by_key(|e| (e.0, e.1));
    let mut out = String::with_capacity(src.len() + 256);
    let mut at = 0;
    for (s, e, text) in &edits {
        if *s < at {
            return Err("overlapping async rewrites".into());
        }
        out.push_str(&src[at..*s]);
        out.push_str(text);
        at = *e;
    }
    out.push_str(&src[at..]);
    let wants = Wants {
        then: src.contains(".then("),
        catch: src.contains(".catch("),
        finally: src.contains(".finally("),
        all: wants_all,
        race: src.contains("Promise.race"),
        any: src.contains("Promise.any"),
        then2: pass.then2,
        resolvers: src.contains("withResolvers"),
    };
    let types: Vec<(String, String)> = pass.types.into_iter().collect();
    let mut templates = String::from(COMMON);
    if pass.top_await {
        templates += MODULES;
    }
    for (k, x) in &types {
        templates += &per_type(k, x, &wants);
    }
    if wants.then {
        for (k, x) in &types {
            templates += &then_void(k, x);
            if wants.then2 {
                templates += &fill(THEN2_VOID, k, x);
            }
            for (v, xv) in types.iter().filter(|(v, _)| core.contains(v)) {
                templates += &then_pair(k, x, v, xv);
                if wants.then2 {
                    templates += &then_pair2(k, x, v, xv);
                }
            }
        }
    }
    for (k, x) in types.iter().filter(|(k, _)| core.contains(k)) {
        if wants.all {
            let ka = key_of(&format!("{x}[]"), None);
            templates += &all_of(k, x, &ka);
        }
        if wants.any {
            templates += &fill(ANY, k, x);
        }
        if wants_settled {
            let ka = key_of(&format!("PromiseSettledResult<{x}>[]"), None);
            templates += &fill(SETTLED, k, x).replace("@A@", &ka);
        }
    }
    Ok(Some(Desugared {
        src: out,
        templates,
        top_await: pass.top_await,
    }))
}

struct Wants {
    then: bool,
    catch: bool,
    finally: bool,
    all: bool,
    race: bool,
    any: bool,
    then2: bool,
    resolvers: bool,
}

const COMMON: &str = r#"function __asyncStep(co: Coroutine): void { __coResume(co); }
class __Rej {
  text: string;
  handled: boolean;
  constructor(text: string) {
    this.text = text;
    this.handled = false;
  }
}
let __rejs: __Rej[] = [];
let __rejScheduled = false;
function __rejTrack(e: Error): __Rej {
  const r = new __Rej(e.toString());
  __rejs.push(r);
  if (!__rejScheduled) {
    __rejScheduled = true;
    __afterMicrotasks(() => { __rejReport(); });
  }
  return r;
}
function __rejReport(): void {
  __rejScheduled = false;
  const list = __rejs;
  __rejs = [];
  for (const r of list) {
    if (!r.handled) console.error("Unhandled promise rejection: " + r.text + " (await it in a try/catch or add .catch)");
  }
}
"#;

/// The bookkeeping of the module closures (`lower_async_module`): the host starts the registered
/// classes when every module has finished, and says which module it waits for or which failed.
const MODULES: &str = r#"function __modWatch(name: string, p: Promise$number): void {
  __startup(0, name, "");
  __onSettled$number(p, () => {
    if (p.state === 2) __startup(2, name, (p.reason as Error).toString());
    else __startup(1, name, "");
  });
}
"#;

fn fill(t: &str, k: &str, x: &str) -> String {
    t.replace("@K@", k).replace("@X@", x)
}

fn per_type(k: &str, x: &str, w: &Wants) -> String {
    let mut s = fill(PER_TYPE, k, x);
    // `Promise<void>` shares `number`'s class, and its `resolve()` takes no value.
    s += &fill(if k == "number" { EXEC_OPTIONAL } else { EXEC }, k, x);
    if w.catch {
        s += &fill(CATCH, k, x);
    }
    if w.finally {
        s += &fill(FINALLY, k, x);
    }
    if w.race {
        s += &fill(RACE, k, x);
    }
    if w.resolvers {
        s += &fill(
            if k == "number" {
                RESOLVERS_OPTIONAL
            } else {
                RESOLVERS
            },
            k,
            x,
        );
    }
    s
}

fn then_void(k: &str, x: &str) -> String {
    fill(THEN_VOID, k, x)
}

fn then_pair(k: &str, x: &str, v: &str, xv: &str) -> String {
    fill(THEN_PAIR, k, x).replace("@V@", v).replace("@XV@", xv)
}

fn then_pair2(k: &str, x: &str, v: &str, xv: &str) -> String {
    fill(THEN2_PAIR, k, x).replace("@V@", v).replace("@XV@", xv)
}

fn all_of(k: &str, x: &str, ka: &str) -> String {
    fill(ALL, k, x).replace("@A@", ka)
}

/// The class key for a promise of `x` (for the lowerer).
pub(crate) fn key(x: &str) -> String {
    mangle(&type_text(x))
}

const PER_TYPE: &str = r#"class Promise$@K@ {
  state: number;
  value: @X@;
  reason: Error | null;
  rej: __Rej | null;
  locked: boolean;
  reactions: (() => void)[];
  constructor() {
    this.state = 0;
    this.value = __coZero<@X@>();
    this.reason = null;
    this.rej = null;
    this.locked = false;
    this.reactions = [];
  }
}
function __fulfill$@K@(p: Promise$@K@, v: @X@): void {
  if (p.state !== 0) return;
  p.state = 1;
  p.value = v;
  p.locked = true;
  const rs = p.reactions;
  p.reactions = [];
  for (const r of rs) __queueMicrotask(r);
}
function __reject$@K@(p: Promise$@K@, e: Error): void {
  if (p.state !== 0) return;
  p.state = 2;
  p.reason = e;
  p.locked = true;
  const rs = p.reactions;
  p.reactions = [];
  if (rs.length === 0) p.rej = __rejTrack(e);
  for (const r of rs) __queueMicrotask(r);
}
function __resolve$@K@(p: Promise$@K@, v: @X@): void {
  if (p.locked) return;
  __fulfill$@K@(p, v);
}
function __rejectIf$@K@(p: Promise$@K@, e: Error): void {
  if (p.locked) return;
  __reject$@K@(p, e);
}
function __onSettled$@K@(p: Promise$@K@, cb: () => void): void {
  if (p.state === 0) {
    p.reactions.push(cb);
  } else {
    if (p.rej !== null) p.rej.handled = true;
    __queueMicrotask(cb);
  }
}
function __adoptSettled$@K@(p: Promise$@K@, q: Promise$@K@): void {
  if (q.state === 1) __fulfill$@K@(p, q.value);
  else __reject$@K@(p, q.reason as Error);
}
function __adopt$@K@(p: Promise$@K@, q: Promise$@K@): void {
  if (p.locked) return;
  p.locked = true;
  __queueMicrotask(() => {
    __onSettled$@K@(q, () => { __adoptSettled$@K@(p, q); });
  });
}
function __async$@K@(body: (p: Promise$@K@) => void): Promise$@K@ {
  const p = new Promise$@K@();
  const co = __coNew(() => {
    try {
      body(p);
    } catch (e) {
      __rejectIf$@K@(p, e as Error);
    }
    if (!p.locked) __fulfill$@K@(p, __coZero<@X@>());
  });
  __asyncStep(co);
  return p;
}
function __asyncRetV$@K@(p: Promise$@K@, v: @X@): void { __resolve$@K@(p, v); }
function __asyncRetP$@K@(p: Promise$@K@, q: Promise$@K@): void { __adopt$@K@(p, q); }
function __asyncRetN$@K@(p: Promise$@K@): void { __resolve$@K@(p, __coZero<@X@>()); }
function __await$@K@(q: Promise$@K@): @X@ {
  const co = __coCurrent();
  __onSettled$@K@(q, () => { __asyncStep(co); });
  __coYield(0);
  if (q.state === 2) throw q.reason as Error;
  return q.value;
}
function __awaitV$@K@(v: @X@): @X@ {
  const co = __coCurrent();
  __queueMicrotask(() => { __asyncStep(co); });
  __coYield(0);
  return v;
}
function __resolved$@K@(v: @X@): Promise$@K@ {
  const p = new Promise$@K@();
  __fulfill$@K@(p, v);
  return p;
}
function __rejected$@K@(e: Error): Promise$@K@ {
  const p = new Promise$@K@();
  __reject$@K@(p, e);
  return p;
}
"#;

const CATCH: &str = r#"function __catch$@K@(p: Promise$@K@, f: (e: Error) => @X@): Promise$@K@ {
  const r = new Promise$@K@();
  __onSettled$@K@(p, () => {
    if (p.state === 1) { __fulfill$@K@(r, p.value); return; }
    try { __fulfill$@K@(r, f(p.reason as Error)); } catch (e) { __reject$@K@(r, e as Error); }
  });
  return r;
}
function __catchV$@K@(p: Promise$@K@, f: (e: Error) => void): Promise$@K@ {
  const r = new Promise$@K@();
  __onSettled$@K@(p, () => {
    if (p.state === 1) { __fulfill$@K@(r, p.value); return; }
    try { f(p.reason as Error); __fulfill$@K@(r, __coZero<@X@>()); } catch (e) { __reject$@K@(r, e as Error); }
  });
  return r;
}
function __catchP$@K@(p: Promise$@K@, f: (e: Error) => Promise$@K@): Promise$@K@ {
  const r = new Promise$@K@();
  __onSettled$@K@(p, () => {
    if (p.state === 1) { __fulfill$@K@(r, p.value); return; }
    try { __adopt$@K@(r, f(p.reason as Error)); } catch (e) { __reject$@K@(r, e as Error); }
  });
  return r;
}
"#;

const FINALLY: &str = r#"function __finally$@K@(p: Promise$@K@, f: () => void): Promise$@K@ {
  const r = new Promise$@K@();
  __onSettled$@K@(p, () => {
    try { f(); } catch (e) { __reject$@K@(r, e as Error); return; }
    // The specification settles the result two jobs later.
    __queueMicrotask(() => { __queueMicrotask(() => { __adoptSettled$@K@(r, p); }); });
  });
  return r;
}
"#;

const RACE: &str = r#"function __raceOne$@K@(r: Promise$@K@, p: Promise$@K@): void {
  __onSettled$@K@(p, () => { __adoptSettled$@K@(r, p); });
}
function __race$@K@(ps: Promise$@K@[]): Promise$@K@ {
  const r = new Promise$@K@();
  for (let i = 0; i < ps.length; i++) __raceOne$@K@(r, ps[i]);
  return r;
}
"#;

const THEN_VOID: &str = r#"function __thenV$@K@(p: Promise$@K@, f: (value: @X@) => void): Promise$number {
  const r = new Promise$number();
  __onSettled$@K@(p, () => {
    if (p.state === 1) {
      try { f(p.value); __fulfill$number(r, 0); } catch (e) { __reject$number(r, e as Error); }
    } else {
      __reject$number(r, p.reason as Error);
    }
  });
  return r;
}
"#;

const THEN_PAIR: &str = r#"function __then$@K@$@V@(p: Promise$@K@, f: (value: @X@) => @XV@): Promise$@V@ {
  const r = new Promise$@V@();
  __onSettled$@K@(p, () => {
    if (p.state === 1) {
      try { __fulfill$@V@(r, f(p.value)); } catch (e) { __reject$@V@(r, e as Error); }
    } else {
      __reject$@V@(r, p.reason as Error);
    }
  });
  return r;
}
function __thenP$@K@$@V@(p: Promise$@K@, f: (value: @X@) => Promise$@V@): Promise$@V@ {
  const r = new Promise$@V@();
  __onSettled$@K@(p, () => {
    if (p.state === 1) {
      try { __adopt$@V@(r, f(p.value)); } catch (e) { __reject$@V@(r, e as Error); }
    } else {
      __reject$@V@(r, p.reason as Error);
    }
  });
  return r;
}
"#;

const ALL: &str = r#"function __allOne$@K@(r: Promise$@A@, p: Promise$@K@, i: number, out: @X@[], left: number[]): void {
  __onSettled$@K@(p, () => {
    if (p.state === 1) {
      out[i] = p.value;
      left[0] -= 1;
      if (left[0] === 0) __fulfill$@A@(r, out);
    } else {
      __reject$@A@(r, p.reason as Error);
    }
  });
}
function __all$@K@(ps: Promise$@K@[]): Promise$@A@ {
  const r = new Promise$@A@();
  const out: @X@[] = [];
  for (let i = 0; i < ps.length; i++) out.push(__coZero<@X@>());
  if (ps.length === 0) {
    __fulfill$@A@(r, out);
    return r;
  }
  const left: number[] = [ps.length];
  for (let i = 0; i < ps.length; i++) __allOne$@K@(r, ps[i], i, out, left);
  return r;
}
"#;

const EXEC: &str = r#"function __newPromise$@K@(exec: (resolve: (value: @X@) => void, reject: (reason: Error) => void) => void): Promise$@K@ {
  const p = new Promise$@K@();
  try {
    exec((v: @X@) => { __resolve$@K@(p, v); }, (e: Error) => { __rejectIf$@K@(p, e); });
  } catch (e) {
    __rejectIf$@K@(p, e as Error);
  }
  return p;
}
"#;

const RESOLVERS: &str = r#"class Resolvers$@K@ {
  promise: Promise$@K@;
  resolve: (value: @X@) => void;
  reject: (reason: Error) => void;
  constructor(promise: Promise$@K@, resolve: (value: @X@) => void, reject: (reason: Error) => void) {
    this.promise = promise;
    this.resolve = resolve;
    this.reject = reject;
  }
}
function __withResolvers$@K@(): Resolvers$@K@ {
  const p = new Promise$@K@();
  return new Resolvers$@K@(p, (v: @X@) => { __resolve$@K@(p, v); }, (e: Error) => { __rejectIf$@K@(p, e); });
}
"#;

const RESOLVERS_OPTIONAL: &str = r#"class Resolvers$@K@ {
  promise: Promise$@K@;
  resolve: (value?: @X@) => void;
  reject: (reason: Error) => void;
  constructor(promise: Promise$@K@, resolve: (value?: @X@) => void, reject: (reason: Error) => void) {
    this.promise = promise;
    this.resolve = resolve;
    this.reject = reject;
  }
}
function __withResolvers$@K@(): Resolvers$@K@ {
  const p = new Promise$@K@();
  return new Resolvers$@K@(p, (v?: @X@) => {
    let vv = __coZero<@X@>();
    if (v !== undefined) vv = v;
    __resolve$@K@(p, vv);
  }, (e: Error) => { __rejectIf$@K@(p, e); });
}
"#;

const EXEC_OPTIONAL: &str = r#"function __newPromise$@K@(exec: (resolve: (value?: @X@) => void, reject: (reason: Error) => void) => void): Promise$@K@ {
  const p = new Promise$@K@();
  try {
    exec((v?: @X@) => {
      let vv = __coZero<@X@>();
      if (v !== undefined) vv = v;
      __resolve$@K@(p, vv);
    }, (e: Error) => { __rejectIf$@K@(p, e); });
  } catch (e) {
    __rejectIf$@K@(p, e as Error);
  }
  return p;
}
"#;

const THEN2_VOID: &str = r#"function __then2V$@K@(p: Promise$@K@, f: (value: @X@) => void, g: (e: Error) => void): Promise$number {
  const r = new Promise$number();
  __onSettled$@K@(p, () => {
    try {
      if (p.state === 1) f(p.value);
      else g(p.reason as Error);
      __fulfill$number(r, 0);
    } catch (e) {
      __reject$number(r, e as Error);
    }
  });
  return r;
}
"#;

const THEN2_PAIR: &str = r#"function __then2$@K@$@V@(p: Promise$@K@, f: (value: @X@) => @XV@, g: (e: Error) => @XV@): Promise$@V@ {
  const r = new Promise$@V@();
  __onSettled$@K@(p, () => {
    try {
      if (p.state === 1) __fulfill$@V@(r, f(p.value));
      else __fulfill$@V@(r, g(p.reason as Error));
    } catch (e) {
      __reject$@V@(r, e as Error);
    }
  });
  return r;
}
function __then2P$@K@$@V@(p: Promise$@K@, f: (value: @X@) => Promise$@V@, g: (e: Error) => Promise$@V@): Promise$@V@ {
  const r = new Promise$@V@();
  __onSettled$@K@(p, () => {
    try {
      if (p.state === 1) __adopt$@V@(r, f(p.value));
      else __adopt$@V@(r, g(p.reason as Error));
    } catch (e) {
      __reject$@V@(r, e as Error);
    }
  });
  return r;
}
"#;

const ANY: &str = r#"function __anyOne$@K@(r: Promise$@K@, p: Promise$@K@, i: number, errors: Error[], left: number[]): void {
  __onSettled$@K@(p, () => {
    if (p.state === 1) {
      __fulfill$@K@(r, p.value);
    } else {
      errors[i] = p.reason as Error;
      left[0] -= 1;
      if (left[0] === 0) __reject$@K@(r, new AggregateError(errors, "All promises were rejected"));
    }
  });
}
function __any$@K@(ps: Promise$@K@[]): Promise$@K@ {
  const r = new Promise$@K@();
  const errors: Error[] = [];
  for (let i = 0; i < ps.length; i++) errors.push(new Error(""));
  if (ps.length === 0) {
    __reject$@K@(r, new AggregateError(errors, "All promises were rejected"));
    return r;
  }
  const left: number[] = [ps.length];
  for (let i = 0; i < ps.length; i++) __anyOne$@K@(r, ps[i], i, errors, left);
  return r;
}
"#;

const SETTLED: &str = r#"class PromiseSettledResult$@K@ {
  status: string;
  value: @X@;
  reason: Error | null;
  constructor(status: string, value: @X@, reason: Error | null) {
    this.status = status;
    this.value = value;
    this.reason = reason;
  }
}
function __settledOne$@K@(r: Promise$@A@, p: Promise$@K@, i: number, out: PromiseSettledResult$@K@[], left: number[]): void {
  __onSettled$@K@(p, () => {
    if (p.state === 1) out[i] = new PromiseSettledResult$@K@("fulfilled", p.value, null);
    else out[i] = new PromiseSettledResult$@K@("rejected", __coZero<@X@>(), p.reason);
    left[0] -= 1;
    if (left[0] === 0) __fulfill$@A@(r, out);
  });
}
function __allSettled$@K@(ps: Promise$@K@[]): Promise$@A@ {
  const r = new Promise$@A@();
  const out: PromiseSettledResult$@K@[] = [];
  for (let i = 0; i < ps.length; i++) out.push(new PromiseSettledResult$@K@("fulfilled", __coZero<@X@>(), null));
  if (ps.length === 0) {
    __fulfill$@A@(r, out);
    return r;
  }
  const left: number[] = [ps.length];
  for (let i = 0; i < ps.length; i++) __settledOne$@K@(r, ps[i], i, out, left);
  return r;
}
"#;
