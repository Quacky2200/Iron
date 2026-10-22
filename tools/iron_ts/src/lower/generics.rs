//! Generics by monomorphisation: `function id<T>(x: T): T`, `class Box<T>`, `interface Pair<A, B>`,
//! `type Opt<T> = T | null`. Nothing is generic at run time. Each distinct list of type arguments
//! gets its own copy (`id$$number`, `Box$$string`), lowered with a type environment (`tenv`) that
//! maps the type parameters to their IR types, so `T` in any annotation inside means that type.
//!
//! Type arguments come from the call (`id<number>(1)`, `new Box<string>("a")`), else from the
//! arguments' types (`id(1)`, `new Box("a")`), else from a parameter default or the expected type.
//! Inference looks at the arguments' types by lowering them once for their type only, closure
//! arguments after the other arguments have fixed what their parameters are.
//! Not built (each is an error that says so): generic methods, generic functions used as values,
//! constraints are accepted and not checked.

use super::classes::Target;
use super::*;
use std::rc::Rc;

/// The type parameters in scope while lowering one specialisation.
pub(crate) type TEnv = Rc<HashMap<String, Ty>>;

/// A type parameter: its name and its default type, if any.
pub(crate) type TParams<'a> = Vec<(String, Option<&'a TSType<'a>>)>;

pub(crate) fn tparam_list<'a>(d: Option<&'a TSTypeParameterDeclaration<'a>>) -> TParams<'a> {
    d.map(|d| {
        d.params
            .iter()
            .map(|p| (p.name.name.to_string(), p.default.as_ref()))
            .collect()
    })
    .unwrap_or_default()
}

#[derive(Clone)]
pub(crate) struct GenericFn<'a> {
    pub tparams: TParams<'a>,
    pub params: &'a FormalParameters<'a>,
    pub ret: Option<&'a TSType<'a>>,
    pub body: FnBody<'a>,
}

#[derive(Clone)]
pub(crate) struct GenericClass<'a> {
    pub class: &'a Class<'a>,
    pub tparams: TParams<'a>,
}

impl<'a> Lowerer<'a> {
    /// A readable, unique text for a type, for naming specialisations.
    pub(crate) fn ty_key(&self, t: &Ty) -> String {
        match t {
            Ty::Num => "number".into(),
            Ty::Str => "string".into(),
            Ty::Bool => "boolean".into(),
            Ty::Lit => "literal".into(),
            Ty::BigInt => "bigint".into(),
            Ty::Void => "void".into(),
            Ty::Null => "null".into(),
            Ty::Obj(s) => format!("{}#{}", self.m.structs[*s].name, s),
            Ty::Arr(e) => format!("{}[]", self.ty_key(e)),
            Ty::Typed(k) => k.name().into(),
            Ty::Fn(f) => format!(
                "fn({})->{}",
                f.params
                    .iter()
                    .map(|p| self.ty_key(p))
                    .collect::<Vec<_>>()
                    .join(","),
                self.ty_key(&f.ret)
            ),
            Ty::Cell(e) => format!("cell({})", self.ty_key(e)),
            Ty::Nullable(e) => format!("{}?", self.ty_key(e)),
            Ty::Handle(n) => n.clone(),
            Ty::Map(k, v) => format!("Map({},{})", self.ty_key(k), self.ty_key(v)),
            Ty::Set(k) => format!("Set({})", self.ty_key(k)),
            Ty::JsValue => "js".into(),
        }
    }

    /// The name of `base` specialised to `targs`: the same list always gives the same name.
    pub(crate) fn inst_name(&mut self, base: &str, targs: &[Ty]) -> String {
        let keys: Vec<String> = targs.iter().map(|t| self.ty_key(t)).collect();
        let raw = format!("{base}<{}>", keys.join(","));
        if let Some(n) = self.inst_names.get(&raw) {
            return n.clone();
        }
        let clean: String = keys
            .join("$")
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '$' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        let mut name = format!("{base}$${clean}");
        while self.inst_names.values().any(|n| *n == name) {
            name.push('_');
        }
        self.inst_names.insert(raw, name.clone());
        name
    }

    fn make_env(tparams: &TParams<'a>, targs: &[Ty]) -> TEnv {
        Rc::new(
            tparams
                .iter()
                .map(|(n, _)| n.clone())
                .zip(targs.iter().cloned())
                .collect(),
        )
    }

    /// Fills `targs` up to the parameter count with defaults (resolved with the ones before).
    pub(crate) fn complete_targs(
        &mut self,
        what: &str,
        tparams: &TParams<'a>,
        mut targs: Vec<Ty>,
    ) -> R<Vec<Ty>> {
        if targs.len() > tparams.len() {
            bail!(
                "{what}: {} type arguments, at most {} expected",
                targs.len(),
                tparams.len()
            );
        }
        while targs.len() < tparams.len() {
            let (name, default) = &tparams[targs.len()];
            let Some(d) = default else {
                bail!("{what}: type argument `{name}` is missing")
            };
            let env = Self::make_env(&tparams[..targs.len()].to_vec(), &targs);
            let saved = std::mem::replace(&mut self.tenv, env);
            let t = self.ty(d, what);
            self.tenv = saved;
            targs.push(t?);
        }
        Ok(targs)
    }

    // ---- inference ----

    /// Matches a type annotation against the type of a value and records what each type parameter
    /// in it must be.
    pub(crate) fn unify_ann(
        &mut self,
        ann: &'a TSType<'a>,
        actual: &Ty,
        tps: &[String],
        out: &mut HashMap<String, Ty>,
        what: &str,
    ) -> R<()> {
        match ann {
            TSType::TSParenthesizedType(p) => {
                self.unify_ann(&p.type_annotation, actual, tps, out, what)
            }
            TSType::TSArrayType(a) => match actual.strip() {
                Ty::Arr(e) => self.unify_ann(&a.element_type, e, tps, out, what),
                _ => Ok(()),
            },
            TSType::TSUnionType(u) => {
                let rest: Vec<&'a TSType<'a>> = u
                    .types
                    .iter()
                    .filter(|t| {
                        !matches!(t, TSType::TSNullKeyword(_) | TSType::TSUndefinedKeyword(_))
                    })
                    .collect();
                if rest.len() == 1 {
                    self.unify_ann(rest[0], actual.strip(), tps, out, what)?;
                }
                Ok(())
            }
            TSType::TSFunctionType(f) => {
                let Ty::Fn(sig) = actual.strip() else {
                    return Ok(());
                };
                for (p, t) in f.params.items.iter().zip(&sig.params) {
                    if let Some(a) = &p.type_annotation {
                        self.unify_ann(&a.type_annotation, t, tps, out, what)?;
                    }
                }
                // A closure that returns nothing fixes `T = void` (`under(root, () => this.build())`).
                if matches!(sig.ret, Ty::Void)
                    && let TSType::TSTypeReference(r) = &f.return_type.type_annotation
                    && let TSTypeName::IdentifierReference(id) = &r.type_name
                    && r.type_arguments.is_none()
                    && tps.iter().any(|t| t == id.name.as_str())
                {
                    out.entry(id.name.to_string()).or_insert(Ty::Void);
                    return Ok(());
                }
                self.unify_ann(&f.return_type.type_annotation, &sig.ret, tps, out, what)
            }
            TSType::TSTypeReference(r) => {
                let TSTypeName::IdentifierReference(id) = &r.type_name else {
                    return Ok(());
                };
                let n = id.name.as_str();
                let args: Vec<&'a TSType<'a>> = r
                    .type_arguments
                    .as_ref()
                    .map(|a| a.params.iter().collect())
                    .unwrap_or_default();
                if args.is_empty() && tps.iter().any(|t| t == n) {
                    let t = match actual {
                        Ty::Null | Ty::Void => return Ok(()),
                        t => t.clone(),
                    };
                    match out.get(n) {
                        None => {
                            out.insert(n.to_string(), t);
                        }
                        Some(old) if *old == t => {}
                        // A union member and a string: the string (a union is a string).
                        Some(old)
                            if matches!(
                                (old.strip(), t.strip()),
                                (Ty::Lit, Ty::Str) | (Ty::Str, Ty::Lit)
                            ) =>
                        {
                            out.insert(n.to_string(), Ty::Str);
                        }
                        // `null` seen first, or a nullable and a plain value: the nullable wins.
                        Some(old) if old.strip() == t.strip() => {
                            out.insert(n.to_string(), t.nullable());
                        }
                        Some(old) => bail!(
                            "{what}: conflicting types for `{n}`: {} and {}",
                            self.ty_key(old),
                            self.ty_key(&t)
                        ),
                    }
                    return Ok(());
                }
                match (n, actual.strip()) {
                    ("Array" | "ReadonlyArray", Ty::Arr(e)) if !args.is_empty() => {
                        self.unify_ann(args[0], e, tps, out, what)
                    }
                    ("Map" | "ReadonlyMap" | "WeakMap", Ty::Map(k, v)) if args.len() == 2 => {
                        self.unify_ann(args[0], k, tps, out, what)?;
                        self.unify_ann(args[1], v, tps, out, what)
                    }
                    ("Set" | "ReadonlySet" | "WeakSet", Ty::Set(k)) if args.len() == 1 => {
                        self.unify_ann(args[0], k, tps, out, what)
                    }
                    // `Promise<T>` against a promise class: T is its value type.
                    ("Promise", Ty::Obj(_)) if args.len() == 1 => {
                        match self.promise_of(actual.strip()) {
                            Some((_, value)) => self.unify_ann(args[0], &value, tps, out, what),
                            None => Ok(()),
                        }
                    }
                    (_, Ty::Obj(sid)) => {
                        // A specialised class, interface or alias: match the arguments in order.
                        let Some((base, targs)) = self.targs_of_struct.get(sid).cloned() else {
                            return Ok(());
                        };
                        if base == n {
                            for (a, t) in args.iter().zip(&targs) {
                                self.unify_ann(a, t, tps, out, what)?;
                            }
                        }
                        Ok(())
                    }
                    _ => Ok(()),
                }
            }
            _ => Ok(()),
        }
    }

    /// Type arguments for a call: written ones, else inferred from the arguments, else defaults,
    /// else from the type the result is expected to have.
    pub(crate) fn infer_targs(
        &mut self,
        what: &str,
        g: &GenericFn<'a>,
        explicit: Option<&'a TSTypeParameterInstantiation<'a>>,
        args: &'a [Argument<'a>],
        expected: Option<&Ty>,
        base: Option<&TEnv>,
    ) -> R<Vec<Ty>> {
        self.infer_targs_skip(what, g, explicit, args, expected, base, 0)
    }

    /// Like [`Self::infer_targs`] for a call whose first `skip` parameters are filled by something
    /// other than `args` (a facade method's receiver).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn infer_targs_skip(
        &mut self,
        what: &str,
        g: &GenericFn<'a>,
        explicit: Option<&'a TSTypeParameterInstantiation<'a>>,
        args: &'a [Argument<'a>],
        expected: Option<&Ty>,
        base: Option<&TEnv>,
        skip: usize,
    ) -> R<Vec<Ty>> {
        let (tparams, params, ret) = (&g.tparams, g.params, g.ret);
        let names: Vec<String> = tparams.iter().map(|(n, _)| n.clone()).collect();
        let mut found: HashMap<String, Ty> = HashMap::new();
        if let Some(e) = explicit {
            let mut written = Vec::new();
            for t in &e.params {
                written.push(self.ty(t, what)?);
            }
            if written.len() == names.len() {
                return Ok(written);
            }
            for (n, t) in names.iter().zip(written) {
                found.insert(n.clone(), t);
            }
        }
        let n_fixed = params.items.len();
        // Annotation for argument `i`: a fixed parameter's, or the rest parameter's element.
        let ann_for = |i: usize| -> Option<&'a TSType<'a>> {
            let i = i + skip;
            if i < n_fixed {
                params.items[i]
                    .type_annotation
                    .as_ref()
                    .map(|a| &a.type_annotation)
            } else {
                match params.rest.as_ref()?.type_annotation.as_ref() {
                    Some(a) => match &a.type_annotation {
                        TSType::TSArrayType(arr) => Some(&arr.element_type),
                        t => Some(t),
                    },
                    None => None,
                }
            }
        };
        let is_closure = |e: &Expression<'a>| {
            matches!(
                e,
                Expression::ArrowFunctionExpression(_) | Expression::FunctionExpression(_)
            )
        };
        // Pass 1: arguments that are not closures.
        for (i, a) in args.iter().enumerate() {
            let (Some(e), Some(ann)) = (a.as_expression(), ann_for(i)) else {
                continue;
            };
            if is_closure(e) {
                continue;
            }
            let mark = self.cur.locals.len();
            let v = self.lower_expr(e, None);
            self.cur.locals.truncate(mark);
            let mut v = v?;
            // A string written in the call is a `string`; a union-typed value keeps its union.
            if Self::is_fresh_literal(e) {
                v = self.widen_lit(v);
            }
            self.unify_ann(ann, &v.ty, &names, &mut found, what)?;
        }
        // Pass 2: closures, whose parameter types the first pass may have fixed. Their result
        // type is read from the body.
        for (i, a) in args.iter().enumerate() {
            let (Some(e), Some(ann)) = (a.as_expression(), ann_for(i)) else {
                continue;
            };
            if !is_closure(e) {
                continue;
            }
            let mut all: HashMap<String, Ty> = base.map(|b| (**b).clone()).unwrap_or_default();
            all.extend(found.clone());
            let partial: TEnv = Rc::new(all);
            let saved = std::mem::replace(&mut self.tenv, partial);
            let want = self.closure_param_types(ann, what);
            self.tenv = saved;
            if want.is_none() {
                self.unify_closure_annotations(e, ann, &names, &mut found, what)?;
                let mut all: HashMap<String, Ty> = base.map(|b| (**b).clone()).unwrap_or_default();
                all.extend(found.clone());
                let partial: TEnv = Rc::new(all);
                let saved = std::mem::replace(&mut self.tenv, partial);
                let again = self.closure_param_types(ann, what);
                self.tenv = saved;
                let Some(params_ty) = again else { continue };
                let expect = Ty::Fn(Box::new(FnTy {
                    params: params_ty,
                    ret: Ty::Void,
                }));
                let mark = self.cur.locals.len();
                let saved_infer = std::mem::replace(&mut self.infer_ret, true);
                let v = self.lower_expr(e, Some(&expect));
                self.infer_ret = saved_infer;
                self.cur.locals.truncate(mark);
                let v = v?;
                self.unify_ann(ann, &v.ty, &names, &mut found, what)?;
                continue;
            }
            let Some(params_ty) = want else { continue };
            let expect = Ty::Fn(Box::new(FnTy {
                params: params_ty,
                ret: Ty::Void,
            }));
            let mark = self.cur.locals.len();
            let saved_infer = std::mem::replace(&mut self.infer_ret, true);
            let v = self.lower_expr(e, Some(&expect));
            self.infer_ret = saved_infer;
            self.cur.locals.truncate(mark);
            let v = v?;
            self.unify_ann(ann, &v.ty, &names, &mut found, what)?;
        }
        // The expected result type.
        if let (Some(r), Some(exp)) = (ret, expected) {
            let mut extra = HashMap::new();
            self.unify_ann(r, exp, &names, &mut extra, what)?;
            for (k, v) in extra {
                found.entry(k).or_insert(v);
            }
        }
        let mut out = Vec::new();
        for (i, (name, default)) in tparams.iter().enumerate() {
            if let Some(t) = found.get(name) {
                out.push(t.clone());
            } else if let Some(d) = default {
                let env = Self::make_env(&tparams[..i].to_vec(), &out);
                let saved = std::mem::replace(&mut self.tenv, env);
                let t = self.ty(d, what);
                self.tenv = saved;
                out.push(t?);
            } else {
                bail!(
                    "{what}: cannot infer the type argument `{name}`; write it: {what}<...>(...)"
                );
            }
        }
        Ok(out)
    }

    /// A closure whose parameters carry annotations (`(from, msg: Select) => ..`) against the
    /// function type the callee declares: each annotated parameter fixes the type parameters its
    /// declared counterpart names.
    fn unify_closure_annotations(
        &mut self,
        closure: &'a Expression<'a>,
        ann: &'a TSType<'a>,
        tps: &[String],
        out: &mut HashMap<String, Ty>,
        what: &str,
    ) -> R<()> {
        let mut ann = ann;
        while let TSType::TSParenthesizedType(p) = ann {
            ann = &p.type_annotation;
        }
        let TSType::TSFunctionType(f) = ann else {
            return Ok(());
        };
        let params: &'a FormalParameters<'a> = match closure {
            Expression::ArrowFunctionExpression(a) => &a.params,
            Expression::FunctionExpression(a) => &a.params,
            _ => return Ok(()),
        };
        for (mine, declared) in params.items.iter().zip(&f.params.items) {
            let (Some(m), Some(d)) = (&mine.type_annotation, &declared.type_annotation) else {
                continue;
            };
            let Ok(actual) = self.ty(&m.type_annotation, what) else {
                continue;
            };
            self.unify_ann(&d.type_annotation, &actual, tps, out, what)?;
        }
        Ok(())
    }

    /// The parameter types of a function-typed annotation, if every one can be resolved now.
    fn closure_param_types(&mut self, ann: &'a TSType<'a>, what: &str) -> Option<Vec<Ty>> {
        let mut ann = ann;
        while let TSType::TSParenthesizedType(p) = ann {
            ann = &p.type_annotation;
        }
        let TSType::TSFunctionType(f) = ann else {
            return None;
        };
        let mut out = Vec::new();
        for p in &f.params.items {
            let a = p.type_annotation.as_ref()?;
            let t = self.ty(&a.type_annotation, what).ok()?;
            out.push(if p.optional { t.nullable() } else { t });
        }
        Some(out)
    }

    // ---- functions ----

    /// Declares (once) the copy of generic function `base` for `targs`; returns its name.
    pub(crate) fn instantiate_fn(&mut self, base: &str, targs: Vec<Ty>) -> R<String> {
        let name = self.inst_name(base, &targs);
        if self.fids.contains_key(&name) {
            return Ok(name);
        }
        let g = self.generic_fns[base].clone();
        let src = FnSrc {
            params: g.params,
            ret: g.ret,
            body: g.body,
            exported: false,
            kind: FnKind::Free,
            ret_ty: None,
            tenv: Some(Self::make_env(&g.tparams, &targs)),
        };
        self.declare_fn(&name, src)?;
        Ok(name)
    }

    /// A call `f(..)` / `f<T>(..)` of a generic function.
    pub(crate) fn lower_generic_call(
        &mut self,
        base: &str,
        c: &'a CallExpression<'a>,
        expected: Option<&Ty>,
    ) -> R<Expr> {
        let g = self.generic_fns[base].clone();
        let targs = self.infer_targs(
            base,
            &g,
            c.type_arguments.as_deref(),
            &c.arguments,
            expected,
            None,
        )?;
        let inst = self.instantiate_fn(base, targs)?;
        self.call_declared(&inst, &c.arguments, None)
    }

    // ---- classes ----

    /// Makes the copy of generic class `base` for `targs` (struct now; methods and layout in
    /// `flush_pending_classes`); returns its name.
    pub(crate) fn instantiate_class(&mut self, base: &str, targs: Vec<Ty>) -> R<String> {
        let name = self.inst_name(base, &targs);
        if self.class_decls.contains_key(&name) {
            return Ok(name);
        }
        let g = self.generic_classes[base].clone();
        self.class_decls.insert(name.clone(), g.class);
        self.class_tenv
            .insert(name.clone(), Self::make_env(&g.tparams, &targs));
        let ty = self.class_type(&name)?;
        if let Ty::Obj(sid) = ty {
            self.targs_of_struct.insert(sid, (base.to_string(), targs));
        }
        self.pending_collect.push(name.clone());
        if self.classes_ready || self.eager_classes {
            self.flush_pending_classes()?;
        }
        Ok(name)
    }

    /// Declares the methods of every class instantiated so far, and, once the program's own
    /// classes are laid out, lays these out too.
    pub(crate) fn flush_pending_classes(&mut self) -> R<()> {
        let mut done = Vec::new();
        while let Some(name) = self.pending_collect.first().cloned() {
            self.pending_collect.remove(0);
            let c = self.class_decls[&name];
            self.collect_class_as(c, &name)?;
            done.push(name);
        }
        if (self.classes_ready || self.eager_classes) && !done.is_empty() {
            self.finalize_some(&done)?;
        }
        Ok(())
    }

    /// The type arguments of `new Box(..)` / `new Box<T>(..)`.
    pub(crate) fn new_targs(
        &mut self,
        base: &str,
        n: &'a NewExpression<'a>,
        expected: Option<&Ty>,
    ) -> R<Vec<Ty>> {
        let g = self.generic_classes[base].clone();
        // The constructor's parameters, or none.
        let mut params: &'a FormalParameters<'a> = self.empty_function().0;
        for el in &g.class.body.body {
            if let ClassElement::MethodDefinition(m) = el
                && m.kind == MethodDefinitionKind::Constructor
            {
                params = &m.value.params;
            }
        }
        let ctor = GenericFn {
            tparams: g.tparams.clone(),
            params,
            ret: None,
            body: FnBody::Block(self.empty_function().1),
        };
        // From the expected type: `const b: Box<number> = new Box()`.
        let mut seed: Vec<Ty> = Vec::new();
        if let Some(Ty::Obj(sid)) = expected.map(|t| t.strip())
            && let Some((b, targs)) = self.targs_of_struct.get(sid)
            && b == base
        {
            seed = targs.clone();
        }
        let written = n.type_arguments.as_deref();
        if written.is_none() && !seed.is_empty() && seed.len() == g.tparams.len() {
            // Inference from the arguments wins when it succeeds.
            return Ok(self
                .infer_targs(base, &ctor, None, &n.arguments, None, None)
                .unwrap_or(seed));
        }
        self.infer_targs(base, &ctor, written, &n.arguments, None, None)
    }

    // ---- generic methods ----

    /// The generic method `m` of the class of `sid` or a base class, with the class that has it.
    pub(crate) fn find_generic_method(
        &self,
        sid: StructId,
        m: &str,
    ) -> Option<(String, GenericFn<'a>)> {
        let mut cur = self.class_of_sid.get(&sid)?.clone();
        loop {
            if let Some(g) = self.generic_methods.get(&(cur.clone(), m.to_string())) {
                return Some((cur, g.clone()));
            }
            cur = self.classes.get(&cur)?.parent.clone()?;
        }
    }

    /// `recv.m(..)` of a generic method: a copy per list of type arguments, called directly (a
    /// generic method is never virtual).
    pub(crate) fn call_generic_method(
        &mut self,
        recv: Expr,
        owner: &str,
        m: &str,
        g: GenericFn<'a>,
        c: &'a CallExpression<'a>,
    ) -> R<Expr> {
        let class_env = self.class_tenv.get(owner).cloned().unwrap_or_default();
        let what = format!("{owner}.{m}");
        let targs = self.infer_targs(
            &what,
            &g,
            c.type_arguments.as_deref(),
            &c.arguments,
            None,
            Some(&class_env),
        )?;
        let name = self.inst_name(&what, &targs);
        if !self.fids.contains_key(&name) {
            let mut env: HashMap<String, Ty> = (*class_env).clone();
            for ((n, _), t) in g.tparams.iter().zip(&targs) {
                env.insert(n.clone(), t.clone());
            }
            let sid = self.classes_sid(owner);
            let src = FnSrc {
                params: g.params,
                ret: g.ret,
                body: g.body,
                exported: false,
                kind: FnKind::Method(sid),
                ret_ty: None,
                tenv: Some(Rc::new(env)),
            };
            self.declare_fn(&name, src)?;
        }
        let fid = self.fids[&name];
        self.call_target(&name, Target::Direct(fid), &c.arguments, Some(recv))
    }

    /// Fills the tables of `x instanceof Box`: every struct of a specialisation of `Box` or of a
    /// subclass of one.
    pub(crate) fn fixup_generic_itables(&mut self) {
        for (table, base) in std::mem::take(&mut self.generic_itables) {
            let mut members: Vec<StructId> = self
                .class_of_sid
                .keys()
                .copied()
                .filter(|s| {
                    let mut cur = self.class_of_sid.get(s);
                    while let Some(n) = cur {
                        let sid = self.classes_sid(n);
                        if self
                            .targs_of_struct
                            .get(&sid)
                            .is_some_and(|(b, _)| *b == base)
                        {
                            return true;
                        }
                        cur = self.classes.get(n).and_then(|i| i.parent.as_ref());
                    }
                    false
                })
                .collect();
            members.sort_unstable();
            self.m.itables[table] = members;
        }
    }
}
