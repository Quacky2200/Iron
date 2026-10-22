//! Annotation -> IR type resolution, struct layouts and coercions.

use super::*;

/// Can `a` stand for `g` because the caller's extra arguments are ignored, or because `g` returns
/// `void` and the result of `a` is dropped (TypeScript allows any return type there)? Both are
/// sound for the one scalar return register of the calling conventions the JIT targets: the
/// caller of a `void` function never reads it, and the collector finds roots in frames, not in
/// return registers.
fn fn_ignores_extra(a: &FnTy, g: &FnTy) -> bool {
    a.params.len() <= g.params.len()
        && (a.ret == g.ret || g.ret == Ty::Void)
        && a.params.iter().zip(&g.params).all(|(x, y)| x == y)
}

fn b_strip_owned(t: &Ty) -> Ty {
    t.strip().clone()
}

impl<'a> Lowerer<'a> {
    /// Is `t` the prelude's `StyleValue` (what `number | string` means)?
    pub(crate) fn is_style_value(&self, t: &Ty) -> bool {
        matches!(t, Ty::Obj(s) if self.m.structs[*s].name == "StyleValue")
            && self.fids.contains_key("__sv_num")
    }

    pub(crate) fn key_name(&self, k: &PropertyKey<'a>) -> R<String> {
        match k {
            PropertyKey::StaticIdentifier(i) => Ok(i.name.to_string()),
            PropertyKey::StringLiteral(s) => Ok(s.value.to_string()),
            _ => bail!("computed property key"),
        }
    }

    pub(crate) fn new_struct(&mut self, name: &str, fields: Vec<Field>) -> StructId {
        self.m.structs.push(StructDef {
            name: name.to_string(),
            fields,
        });
        self.m.structs.len() - 1
    }

    /// A struct for an inline `{ x: number; y: number }` type or a literal: one per shape.
    pub(crate) fn anon_struct(&mut self, name: &str, fields: Vec<Field>) -> StructId {
        let key = fields
            .iter()
            .map(|f| format!("{}:{}{}", f.name, f.ty, if f.optional { "?" } else { "" }))
            .collect::<Vec<_>>()
            .join(",");
        if let Some(id) = self.shapes.get(&key) {
            return *id;
        }
        let id = self.new_struct(name, fields);
        self.shapes.insert(key, id);
        id
    }

    pub(crate) fn ty(&mut self, t: &'a TSType<'a>, hint: &str) -> R<Ty> {
        Ok(match t {
            TSType::TSNumberKeyword(_) => Ty::Num,
            TSType::TSBigIntKeyword(_) => Ty::BigInt,
            TSType::TSSymbolKeyword(_) => self.named_type("Symbol_")?,
            TSType::TSBooleanKeyword(_) => Ty::Bool,
            TSType::TSStringKeyword(_) => Ty::Str,
            TSType::TSVoidKeyword(_) | TSType::TSUndefinedKeyword(_) => Ty::Void,
            TSType::TSNullKeyword(_) => Ty::Null,
            TSType::TSParenthesizedType(p) => self.ty(&p.type_annotation, hint)?,
            TSType::TSTypeOperatorType(o) => self.ty(&o.type_annotation, hint)?,
            TSType::TSArrayType(a) => Ty::Arr(Box::new(self.ty(&a.element_type, hint)?)),
            TSType::TSLiteralType(l) => match &l.literal {
                TSLiteral::StringLiteral(s) => {
                    self.tag(s.value.as_str());
                    Ty::Lit
                }
                TSLiteral::NumericLiteral(_) => Ty::Num,
                TSLiteral::BooleanLiteral(_) => Ty::Bool,
                _ => bail!("unsupported literal type"),
            },
            // `[number, number]`: a tuple of one element type is an array of it.
            TSType::TSTupleType(t) => {
                let mut tys = Vec::new();
                for el in &t.element_types {
                    let Some(et) = el.as_ts_type() else {
                        bail!(
                            "{}: optional and rest tuple elements are not supported",
                            self.loc(t.span.start)
                        )
                    };
                    tys.push(self.ty(et, hint)?);
                }
                match tys.first() {
                    Some(first) if tys.iter().all(|t| t == first) => {
                        Ty::Arr(Box::new(first.clone()))
                    }
                    // Mixed element types: an object with fields `0`, `1`, ..
                    _ => {
                        let fields = tys
                            .iter()
                            .enumerate()
                            .map(|(i, t)| Field {
                                name: i.to_string(),
                                ty: t.clone(),
                                optional: false,
                            })
                            .collect();
                        Ty::Obj(self.anon_struct("Tuple", fields))
                    }
                }
            }
            TSType::TSUnionType(u) => self.union(&u.types, hint)?,
            TSType::TSTypeLiteral(l) => {
                let fields = self.members(&l.members, hint)?;
                Ty::Obj(self.anon_struct(hint, fields))
            }
            // `[t: number, run: () => void]`: the labels are for readers.
            TSType::TSNamedTupleMember(m) => match m.element_type.as_ts_type() {
                Some(et) if !m.optional => self.ty(et, hint)?,
                _ => bail!(
                    "{}: optional tuple elements are not supported",
                    self.loc(m.span.start)
                ),
            },
            TSType::TSFunctionType(f) => {
                let mut params = Vec::new();
                for p in &f.params.items {
                    let Some(a) = &p.type_annotation else {
                        bail!("untyped callback parameter")
                    };
                    let ty = self.ty(&a.type_annotation, hint)?;
                    // `(x?: T) => ..`: a call may leave it out, so it is null then.
                    params.push(if p.optional { ty.nullable() } else { ty });
                }
                let ret = self.ty(&f.return_type.type_annotation, hint)?;
                Ty::Fn(Box::new(FnTy { params, ret }))
            }
            TSType::TSIndexedAccessType(i) => {
                let obj = self.ty(&i.object_type, hint)?;
                let TSType::TSLiteralType(l) = &i.index_type else {
                    bail!("indexed access by a non-literal")
                };
                let TSLiteral::StringLiteral(s) = &l.literal else {
                    bail!("indexed access by a non-string")
                };
                let Ty::Obj(sid) = obj.strip() else {
                    bail!("indexed access on a non-object")
                };
                let st = &self.m.structs[*sid];
                let Some(i) = st.field(s.value.as_str()) else {
                    bail!("no field {}", s.value)
                };
                st.fields[i].ty.clone()
            }
            TSType::TSTypeReference(r) => {
                // `Intl.Collator` is the prelude class `Intl_Collator`.
                if let TSTypeName::QualifiedName(q) = &r.type_name
                    && matches!(&q.left, TSTypeName::IdentifierReference(l) if l.name == "Intl")
                {
                    return self.named_type(&format!("Intl_{}", q.right.name));
                }
                // `React.ReactNode` is `ReactNode`.
                if let TSTypeName::QualifiedName(q) = &r.type_name
                    && matches!(&q.left, TSTypeName::IdentifierReference(l) if l.name == "React")
                {
                    return self.named_type(q.right.name.as_str());
                }
                let TSTypeName::IdentifierReference(id) = &r.type_name else {
                    bail!("qualified type name")
                };
                let args: Vec<&'a TSType<'a>> = r
                    .type_arguments
                    .as_ref()
                    .map(|a| a.params.iter().collect())
                    .unwrap_or_default();
                // A type parameter of the specialisation being lowered.
                if args.is_empty()
                    && let Some(t) = self.tenv.get(id.name.as_str())
                {
                    return Ok(t.clone());
                }
                if !args.is_empty()
                    && (self.generic_classes.contains_key(id.name.as_str())
                        || self
                            .ifaces
                            .get(id.name.as_str())
                            .is_some_and(|i| i.type_parameters.is_some())
                        || self
                            .aliases
                            .get(id.name.as_str())
                            .is_some_and(|a| a.type_parameters.is_some()))
                {
                    let mut targs = Vec::new();
                    for a in &args {
                        targs.push(self.ty(a, hint)?);
                    }
                    return self.generic_type(id.name.as_str(), targs);
                }
                match id.name.as_str() {
                    "WeakRef" if !args.is_empty() => {
                        let sp = oxc_span::GetSpan::span(args[0]);
                        let name = self.src[sp.start as usize..sp.end as usize].trim();
                        self.named_type(&format!("WeakRef${name}"))?
                    }
                    "PromiseSettledResult" if !args.is_empty() => {
                        let sp = oxc_span::GetSpan::span(args[0]);
                        let key = asyncs::key(&self.src[sp.start as usize..sp.end as usize]);
                        self.named_type(&format!("PromiseSettledResult${key}"))?
                    }
                    "Promise" if !args.is_empty() => {
                        let sp = oxc_span::GetSpan::span(args[0]);
                        let text = &self.src[sp.start as usize..sp.end as usize];
                        let key = if self.mentions_tparam(text) {
                            // `Promise<T>` inside `f<T>`: the class of this specialisation's T.
                            let at = self.loc(sp.start);
                            self.generic_promise_key(args[0], &at)?
                        } else {
                            asyncs::key(text)
                        };
                        self.named_type(&format!("Promise${key}"))?
                    }
                    // `Generator<T>`, `Iterator<T>`, `IteratorResult<T>`: the classes written per type.
                    n if generators::FAMILY.contains(&n) && !args.is_empty() => {
                        let text = |t: &TSType<'_>| {
                            let sp = oxc_span::GetSpan::span(t);
                            self.src[sp.start as usize..sp.end as usize].to_string()
                        };
                        let key = generators::key_of(
                            &text(args[0]),
                            args.get(2).map(|a| text(a)).as_deref(),
                        );
                        let class = match n {
                            "Iterator" => format!("Iterator${key}"),
                            "IteratorResult" => format!("IteratorResult${key}"),
                            _ => format!("Generator${key}"),
                        };
                        self.named_type(&class)?
                    }
                    n if crate::ir::TypedKind::from_name(n).is_some() => {
                        Ty::Typed(crate::ir::TypedKind::from_name(n).expect("checked"))
                    }
                    // `Record<"a" | "b", V>`: a struct with one field per member.
                    "Record" if args.len() == 2 => {
                        // `Record<Weather, V>` over a numeric enum: an array indexed by the value.
                        if let TSType::TSTypeReference(k) = args[0]
                            && let TSTypeName::IdentifierReference(kid) = &k.type_name
                            && self.enums.contains_key(kid.name.as_str())
                        {
                            return Ok(Ty::Arr(Box::new(self.ty(args[1], hint)?)));
                        }
                        // `Record<string, unknown>` (a bag of style properties) is an `IronStyle`.
                        if matches!(args[0], TSType::TSStringKeyword(_))
                            && matches!(args[1], TSType::TSUnknownKeyword(_))
                            && self.class_decls.contains_key("StyleEntry")
                        {
                            return self.named_type("IronStyle");
                        }
                        let Some(keys) = self.lit_members(args[0]) else {
                            bail!(
                                "Record needs a union of string literals as its keys; \
                                 use Map<string, V> for other keys"
                            )
                        };
                        let vt = self.ty(args[1], hint)?;
                        let fields = keys
                            .into_iter()
                            .map(|name| Field {
                                name,
                                ty: vt.clone(),
                                optional: false,
                            })
                            .collect();
                        Ty::Obj(self.anon_struct("Record", fields))
                    }
                    "Map" | "WeakMap" | "ReadonlyMap" => {
                        let (Some(k), Some(v)) = (args.first(), args.get(1)) else {
                            bail!("{} needs two type arguments", id.name)
                        };
                        Ty::Map(
                            Box::new(maps::map_key_ty(self.ty(k, hint)?)),
                            Box::new(self.ty(v, hint)?),
                        )
                    }
                    "Set" | "WeakSet" | "ReadonlySet" => {
                        let Some(k) = args.first() else {
                            bail!("{} needs a type argument", id.name)
                        };
                        Ty::Set(Box::new(maps::map_key_ty(self.ty(k, hint)?)))
                    }
                    "Array" | "ReadonlyArray" => {
                        let Some(e) = args.first() else {
                            bail!("generic collection without a type")
                        };
                        Ty::Arr(Box::new(self.ty(e, hint)?))
                    }
                    n => self.named_type(n)?,
                }
            }
            other => {
                let kind: String = format!("{other:?}").chars().take(40).collect();
                bail!(
                    "unsupported type annotation ({kind}, byte {})",
                    oxc_span::GetSpan::span(other).start
                )
            }
        })
    }

    pub(crate) fn members(&mut self, members: &'a [TSSignature<'a>], hint: &str) -> R<Vec<Field>> {
        let mut fields = Vec::new();
        for m in members {
            // `run(a: T): R;` is the property `run: (a: T) => R`.
            if let TSSignature::TSMethodSignature(ms) = m {
                let name = self.key_name(&ms.key)?;
                let mut params = Vec::new();
                for p in &ms.params.items {
                    let Some(a) = &p.type_annotation else {
                        bail!("untyped method parameter")
                    };
                    let ty = self.ty(&a.type_annotation, &format!("{hint}_{name}"))?;
                    params.push(if p.optional { ty.nullable() } else { ty });
                }
                let ret = match &ms.return_type {
                    Some(r) => self.ty(&r.type_annotation, &format!("{hint}_{name}"))?,
                    None => Ty::Void,
                };
                let mut ty = Ty::Fn(Box::new(FnTy { params, ret }));
                if ms.optional {
                    ty = ty.nullable();
                }
                fields.push(Field {
                    name,
                    ty,
                    optional: ms.optional,
                });
                continue;
            }
            let TSSignature::TSPropertySignature(p) = m else {
                bail!(
                    "{}: unsupported member in object type (index and call signatures are not supported)",
                    self.loc(oxc_span::GetSpan::span(m).start)
                )
            };
            let name = self.key_name(&p.key)?;
            let Some(a) = &p.type_annotation else {
                bail!("property {name} without a type")
            };
            let mut ty = self.ty(&a.type_annotation, &format!("{hint}_{name}"))?;
            if p.optional {
                ty = ty.nullable();
            }
            fields.push(Field {
                name,
                ty,
                optional: p.optional,
            });
        }
        Ok(fields)
    }

    /// The members of a union of string literals (through aliases), or `None` for any other type.
    fn lit_members(&self, t: &'a TSType<'a>) -> Option<Vec<String>> {
        match t {
            TSType::TSParenthesizedType(p) => self.lit_members(&p.type_annotation),
            TSType::TSLiteralType(l) => match &l.literal {
                TSLiteral::StringLiteral(s) => Some(vec![s.value.to_string()]),
                _ => None,
            },
            TSType::TSUnionType(u) => {
                let mut out = Vec::new();
                for m in &u.types {
                    out.extend(self.lit_members(m)?);
                }
                Some(out)
            }
            TSType::TSTypeReference(r) => {
                let TSTypeName::IdentifierReference(id) = &r.type_name else {
                    return None;
                };
                let a = self.aliases.get(id.name.as_str())?;
                self.lit_members(&a.type_annotation)
            }
            _ => None,
        }
    }

    /// `Box<number>`, `Pair<A, B>`, `Opt<T>`: the specialisation of a generic class, interface or
    /// alias, made once per list of type arguments.
    pub(crate) fn generic_type(&mut self, base: &str, targs: Vec<Ty>) -> R<Ty> {
        if self.generic_classes.contains_key(base) {
            let tparams = self.generic_classes[base].tparams.clone();
            let targs = self.complete_targs(base, &tparams, targs)?;
            let name = self.instantiate_class(base, targs)?;
            return self.named_type(&name);
        }
        let (tparams, is_iface) = match self.ifaces.get(base) {
            Some(i) => (generics::tparam_list(i.type_parameters.as_deref()), true),
            None => (
                generics::tparam_list(self.aliases[base].type_parameters.as_deref()),
                false,
            ),
        };
        let targs = self.complete_targs(base, &tparams, targs)?;
        let name = self.inst_name(base, &targs);
        if let Some(t) = self.named.get(&name) {
            return Ok(t.clone());
        }
        let env: generics::TEnv = std::rc::Rc::new(
            tparams
                .iter()
                .map(|(n, _)| n.clone())
                .zip(targs.iter().cloned())
                .collect(),
        );
        let saved = std::mem::replace(&mut self.tenv, env);
        let r = if is_iface {
            let i = self.ifaces[base];
            if !i.extends.is_empty() {
                self.tenv = saved;
                bail!("interface {base}: extends is not supported");
            }
            let id = self.new_struct(&name, vec![]);
            self.named.insert(name.clone(), Ty::Obj(id));
            self.targs_of_struct.insert(id, (base.to_string(), targs));
            self.members(&i.body.body, &name).map(|fields| {
                self.m.structs[id].fields = fields;
                Ty::Obj(id)
            })
        } else {
            let a = self.aliases[base];
            self.ty(&a.type_annotation, &name)
        };
        self.tenv = saved;
        let t = r?;
        self.named.insert(name, t.clone());
        Ok(t)
    }

    /// An interface or alias by name, resolved once.
    /// Whether a written type names a type parameter of the specialisation being lowered.
    pub(crate) fn mentions_tparam(&self, text: &str) -> bool {
        !self.tenv.is_empty()
            && text
                .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '$'))
                .any(|w| self.tenv.contains_key(w))
    }

    pub(crate) fn named_type(&mut self, name: &str) -> R<Ty> {
        if let Some(t) = self.named.get(name) {
            return Ok(t.clone());
        }
        // The program's own declarations shadow the engine's names (`class Scene extends Component`).
        if self.class_decls.contains_key(name) {
            return self.class_type(name);
        }
        let own = self.ifaces.contains_key(name) || self.aliases.contains_key(name);
        if !own && self.manifest.values.contains_key(name) {
            return Ok(Ty::Obj(self.value_struct(name)));
        }
        if !own && self.manifest.handles.contains(name) {
            return Ok(Ty::Handle(name.to_string()));
        }
        if let Some(i) = self.ifaces.get(name).copied() {
            if !i.extends.is_empty() {
                bail!("interface {name}: extends is not supported");
            }
            let id = self.new_struct(name, vec![]);
            self.named.insert(name.to_string(), Ty::Obj(id));
            let fields = self.members(&i.body.body, name)?;
            self.m.structs[id].fields = fields;
            return Ok(Ty::Obj(id));
        }
        if let Some(a) = self.aliases.get(name).copied() {
            let t = self.ty(&a.type_annotation, name)?;
            self.named.insert(name.to_string(), t.clone());
            return Ok(t);
        }
        bail!("unknown type {name}")
    }

    fn union(&mut self, types: &'a [TSType<'a>], hint: &str) -> R<Ty> {
        let mut nullable = false;
        let mut rest: Vec<&'a TSType<'a>> = Vec::new();
        for t in types {
            match t {
                TSType::TSNullKeyword(_) | TSType::TSUndefinedKeyword(_) => nullable = true,
                t => rest.push(t),
            }
        }
        let ty = if rest.iter().all(|t| matches!(t, TSType::TSTypeLiteral(_))) && rest.len() > 1 {
            // A discriminated union of object types: one struct holding every variant's fields.
            let mut fields: Vec<Field> = Vec::new();
            for t in &rest {
                let TSType::TSTypeLiteral(l) = t else {
                    unreachable!()
                };
                for f in self.members(&l.members, hint)? {
                    match fields.iter().position(|o| o.name == f.name) {
                        None => fields.push(f),
                        Some(i) if fields[i].ty == f.ty => {}
                        Some(i) => bail!(
                            "union {hint}: field {} has two different types ({} and {})",
                            f.name,
                            fields[i].ty,
                            f.ty
                        ),
                    }
                }
            }
            Ty::Obj(self.anon_struct(hint, fields))
        } else {
            let mut tys = Vec::new();
            for t in &rest {
                tys.push(self.ty(t, hint)?);
            }
            if tys.is_empty() {
                Ty::Null
            } else if tys.iter().all(|t| *t == tys[0]) {
                tys.swap_remove(0)
            } else if self.class_decls.contains_key("StyleValue")
                && tys.contains(&Ty::Num)
                && tys.iter().all(|t| matches!(t, Ty::Num | Ty::Str | Ty::Lit))
            {
                // `number | string`: the prelude's `StyleValue` (CSS lengths and keywords).
                self.named_type("StyleValue")?
            } else {
                bail!(
                    "unsupported union of {:?}",
                    tys.iter().map(|t| t.to_string()).collect::<Vec<_>>()
                )
            }
        };
        Ok(if nullable { ty.nullable() } else { ty })
    }

    // ---- unify and coerce ----

    /// Can a value of type `from` be used as `to` without changing the pointer? A subclass for
    /// its base class, and arrays of those.
    pub(crate) fn assignable(&self, from: &Ty, to: &Ty) -> bool {
        match (from.strip(), to.strip()) {
            (a, b) if a == b => true,
            (Ty::Obj(x), Ty::Obj(y)) => self.is_subclass(*x, *y),
            (Ty::Arr(x), Ty::Arr(y)) => self.assignable(x, y),
            _ => false,
        }
    }

    pub(crate) fn unify(&self, a: &Ty, b: &Ty) -> R<Ty> {
        if a == b {
            return Ok(a.clone());
        }
        Ok(match (a, b) {
            (Ty::Null, t) | (t, Ty::Null) => t.clone().nullable(),
            (Ty::Lit, Ty::Str) | (Ty::Str, Ty::Lit) => Ty::Str,
            _ if a.strip() == b.strip() => a.strip().clone().nullable(),
            // Two classes of one family: the base class.
            _ if self.assignable(a, b) => b.strip().clone().nullable(),
            _ if self.assignable(b, a) => a.strip().clone().nullable(),
            // Two classes with a common base class: that base class.
            (x, y) if self.common_base(x.strip(), y.strip()).is_some() => {
                let base = self.common_base(x.strip(), y.strip()).expect("checked");
                let t = Ty::Obj(base);
                if a.is_nullable() || b.is_nullable() {
                    t.nullable()
                } else {
                    t
                }
            }
            _ => bail!("cannot unify {a} and {b}"),
        })
    }

    pub(crate) fn coerce(&self, e: Expr, to: &Ty) -> R<Expr> {
        if e.ty == *to {
            return Ok(e);
        }
        match (&e.ty, to) {
            (Ty::Null, t) if t.is_nullable() => Ok(Expr {
                kind: ExprKind::Null,
                ty: to.clone(),
            }),
            // `sprite = null`: an engine handle that is "none" (tsc has checked the declared type).
            (Ty::Null, Ty::Handle(_)) => Ok(Expr {
                kind: ExprKind::Null,
                ty: to.clone(),
            }),
            (a, b) if a.strip() == b.strip() => Ok(Expr {
                kind: ExprKind::Cast(Box::new(e)),
                ty: to.clone(),
            }),
            // A function that takes fewer parameters where more are given (`onClick={() => ..}` for
            // `(e) => void`): the extra arguments are ignored, as in JavaScript. The caller cleans
            // the argument registers, so the same closure works unchanged.
            (a, b)
                if matches!((a.strip(), b.strip()), (Ty::Fn(x), Ty::Fn(g)) if fn_ignores_extra(x, g))
                    && (b.is_nullable() || !a.is_nullable()) =>
            {
                Ok(Expr {
                    kind: ExprKind::Cast(Box::new(e)),
                    ty: to.clone(),
                })
            }
            // A component that may return nothing (`return null`) where an element is expected: a
            // null handle renders nothing.
            (Ty::Nullable(inner), Ty::Handle(n))
                if n == "UiElement" && matches!(&**inner, Ty::Handle(m) if m == n) =>
            {
                Ok(Expr {
                    kind: ExprKind::Cast(Box::new(e)),
                    ty: to.clone(),
                })
            }
            // A number or a string where a `number | string` (`StyleValue`) goes.
            (Ty::Num | Ty::Str | Ty::Lit, b) if self.is_style_value(b.strip()) => {
                let (f, e) = match &e.ty {
                    Ty::Num => ("__sv_num", e),
                    Ty::Str => ("__sv_str", e),
                    _ => (
                        "__sv_str",
                        Expr {
                            kind: ExprKind::TagStr(Box::new(e)),
                            ty: Ty::Str,
                        },
                    ),
                };
                let call = Expr {
                    kind: ExprKind::Call(self.fids[f], vec![e]),
                    ty: b_strip_owned(to),
                };
                if to.is_nullable() {
                    Ok(Expr {
                        kind: ExprKind::Cast(Box::new(call)),
                        ty: to.clone(),
                    })
                } else {
                    Ok(call)
                }
            }
            // A subclass object where its base class is expected: the same pointer.
            (a, b) if self.assignable(a.strip(), b.strip()) => Ok(Expr {
                kind: ExprKind::Cast(Box::new(e)),
                ty: to.clone(),
            }),
            // A union member where a `string` is expected.
            (Ty::Lit, t) if t.strip() == &Ty::Str => {
                let s = Expr {
                    kind: ExprKind::TagStr(Box::new(e)),
                    ty: Ty::Str,
                };
                if t.is_nullable() {
                    Ok(Expr {
                        kind: ExprKind::Cast(Box::new(s)),
                        ty: to.clone(),
                    })
                } else {
                    Ok(s)
                }
            }
            (a, Ty::Void) => bail!("cannot use {a} as void"),
            (a, b) => bail!("type mismatch: {a} where {b} is expected"),
        }
    }

    /// Truthiness as a `Bool`.
    pub(crate) fn to_bool(&self, e: Expr) -> Expr {
        let ex = |k| Expr {
            kind: k,
            ty: Ty::Bool,
        };
        match &e.ty {
            Ty::Bool => e,
            Ty::Nullable(inner) if matches!(**inner, Ty::Num) => {
                let null = ex(ExprKind::IsNull(Box::new(e.clone())));
                ex(ExprKind::And(
                    Box::new(ex(ExprKind::Un(UnOp::Not, Box::new(null)))),
                    Box::new(ex(ExprKind::Truthy(Box::new(e)))),
                ))
            }
            // `boolean | undefined` is true only when it is `true`.
            Ty::Nullable(inner) if matches!(**inner, Ty::Bool) => ex(ExprKind::Cast(Box::new(e))),
            // A nullable string or bigint is true when it is there and not empty / zero.
            Ty::Nullable(inner) if matches!(**inner, Ty::Str | Ty::BigInt) => {
                let null = ex(ExprKind::IsNull(Box::new(e.clone())));
                let inner_ty = (**inner).clone();
                let plain = Expr {
                    kind: ExprKind::Cast(Box::new(e)),
                    ty: inner_ty,
                };
                ex(ExprKind::And(
                    Box::new(ex(ExprKind::Un(UnOp::Not, Box::new(null)))),
                    Box::new(self.to_bool(plain)),
                ))
            }
            Ty::Num => ex(ExprKind::Truthy(Box::new(e))),
            // A bigint is false only at zero.
            Ty::BigInt => ex(ExprKind::Cmp(
                CmpOp::Ne,
                Box::new(Expr {
                    kind: ExprKind::Rt(RtFn::Str("ts_rt_bigint_sign"), vec![e]),
                    ty: Ty::Num,
                }),
                Box::new(Expr {
                    kind: ExprKind::Num(0.0),
                    ty: Ty::Num,
                }),
            )),
            // A union member is true unless its text is empty.
            Ty::Lit => match e.kind {
                ExprKind::Tag(t) => ex(ExprKind::Bool(!self.m.tags[t as usize].is_empty())),
                _ => self.to_bool(Expr {
                    kind: ExprKind::TagStr(Box::new(e)),
                    ty: Ty::Str,
                }),
            },
            // A string is true when it is not empty.
            Ty::Str => ex(ExprKind::Cmp(
                CmpOp::Ne,
                Box::new(Expr {
                    kind: ExprKind::Rt(RtFn::Str("ts_rt_str_length"), vec![e]),
                    ty: Ty::Num,
                }),
                Box::new(Expr {
                    kind: ExprKind::Num(0.0),
                    ty: Ty::Num,
                }),
            )),
            // A handle is an id; zero is no object.
            Ty::Handle(_) => {
                let zero = Expr {
                    kind: ExprKind::Null,
                    ty: e.ty.clone(),
                };
                ex(ExprKind::Cmp(CmpOp::Ne, Box::new(e), Box::new(zero)))
            }
            t if t.is_nullable() => {
                let null = ex(ExprKind::IsNull(Box::new(e)));
                ex(ExprKind::Un(UnOp::Not, Box::new(null)))
            }
            _ => ex(ExprKind::Bool(true)),
        }
    }
}
