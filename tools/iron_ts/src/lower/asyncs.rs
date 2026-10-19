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
//! The microtask queue and the timers live in the runtime (`iron_ts_rt/src/event.rs`).

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
}

/// The value types every program with promises gets a class for.
const BASE: [&str; 3] = ["number", "string", "boolean"];

/// One async function being walked: its promise key and whether its return type was written.
struct Entry {
    key: String,
    typed: bool,
}

struct Pass<'s> {
    src: &'s str,
    edits: Vec<(usize, usize, String)>,
    /// Value types seen: class key to type text.
    types: BTreeMap<String, String>,
    asyncs: Vec<Option<Entry>>,
    async_from: Option<u32>,
    error: Option<String>,
}

fn type_text(x: &str) -> String {
    if matches!(x.trim(), "void" | "undefined") {
        "number".into()
    } else {
        x.trim().into()
    }
}

impl Pass<'_> {
    fn text(&self, span: oxc_span::Span) -> &str {
        &self.src[span.start as usize..span.end as usize]
    }

    fn fail(&mut self, msg: String) {
        self.error.get_or_insert(msg);
    }

    fn add_type(&mut self, x: &str) -> String {
        let key = key_of(x, None);
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
        if ret.is_some() && key.is_none() {
            self.fail(format!(
                "an async function must return `Promise<T>` (at offset {at})"
            ));
            return None;
        }
        Some(match key {
            Some(key) => Entry { key, typed: true },
            None => Entry {
                key: self.add_type("void"),
                typed: false,
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

    /// Wraps a block body (`{ ... }`) in the coroutine closure.
    fn wrap_block(&mut self, body: oxc_span::Span, key: &str) {
        let (open, close) = (body.start as usize + 1, body.end as usize - 1);
        self.edits
            .push((open, open, format!(" return __async${key}((__p) => {{")));
        self.edits.push((close, close, "}); ".into()));
    }
}

impl<'a> Visit<'a> for Pass<'_> {
    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        if it.r#async {
            let entry = self.begin_async(it.return_type.as_deref(), it.generator, it.span.start);
            if let (Some(e), Some(body)) = (&entry, &it.body) {
                let from = self.async_from.take().unwrap_or(it.span.start) as usize;
                self.drop_async(from, it.params.span.start as usize);
                self.wrap_block(body.span, &e.key);
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

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
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
                    ArrowFunctionBody::FunctionBody(b) => self.wrap_block(b.span, &e.key),
                    other => {
                        let b = other.as_expression().map(|x| x.span());
                        let Some(b) = b else {
                            self.fail("unsupported async arrow body".into());
                            return;
                        };
                        self.edits.push((
                            b.start as usize,
                            b.start as usize,
                            format!("__async${}((__p) => {{ __asyncRet${}(__p, ", e.key, e.key),
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
            let key = entry.key.clone();
            self.edits.push((
                it.span.start as usize,
                arg.span().start as usize,
                format!("{{ __asyncRet${key}(__p, "),
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
            self.fail("`await` is only supported inside an async function".into());
            return;
        }
        let a = it.argument.span();
        self.edits
            .push((it.span.start as usize, a.start as usize, "__await(".into()));
        self.edits
            .push((a.end as usize, a.end as usize, ")".into()));
        walk::walk_await_expression(self, it);
    }

    fn visit_for_of_statement(&mut self, it: &ForOfStatement<'a>) {
        if it.r#await {
            self.fail("`for await` is not supported".into());
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
                    self.edits.push((
                        it.span.start as usize,
                        ta.span.end as usize,
                        format!("__newPromise${key}"),
                    ));
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
        async_from: None,
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
    // `Promise.all` makes an array of each value.
    let wants_all = src.contains("Promise.all");
    if wants_all {
        let base: Vec<String> = pass.types.values().cloned().collect();
        for x in base {
            if !x.ends_with("[]") {
                pass.add_type(&format!("{x}[]"));
            }
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
    };
    let types: Vec<(String, String)> = pass.types.into_iter().collect();
    let mut templates = String::from(COMMON);
    for (k, x) in &types {
        templates += &per_type(k, x, &wants);
    }
    if wants.then {
        for (k, x) in &types {
            templates += &then_void(k, x);
            for (v, xv) in &types {
                templates += &then_pair(k, x, v, xv);
            }
        }
    }
    if wants.all {
        for (k, x) in &types {
            if !x.ends_with("[]") {
                let ka = key_of(&format!("{x}[]"), None);
                templates += &all_of(k, x, &ka);
            }
        }
    }
    Ok(Some(Desugared {
        src: out,
        templates,
    }))
}

struct Wants {
    then: bool,
    catch: bool,
    finally: bool,
    all: bool,
    race: bool,
}

const COMMON: &str = "function __asyncStep(co: Coroutine): void { __coResume(co); }\n";

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
    s
}

fn then_void(k: &str, x: &str) -> String {
    fill(THEN_VOID, k, x)
}

fn then_pair(k: &str, x: &str, v: &str, xv: &str) -> String {
    fill(THEN_PAIR, k, x).replace("@V@", v).replace("@XV@", xv)
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
  locked: boolean;
  reactions: (() => void)[];
  constructor() {
    this.state = 0;
    this.value = __coZero<@X@>();
    this.reason = null;
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
  if (p.state === 0) p.reactions.push(cb);
  else __queueMicrotask(cb);
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
