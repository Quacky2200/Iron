//! Annotation -> IR type resolution, struct layouts and coercions.

use super::*;

impl<'a> Lowerer<'a> {
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
            TSType::TSUnionType(u) => self.union(&u.types, hint)?,
            TSType::TSTypeLiteral(l) => {
                let fields = self.members(&l.members, hint)?;
                Ty::Obj(self.anon_struct(hint, fields))
            }
            TSType::TSFunctionType(f) => {
                let mut params = Vec::new();
                for p in &f.params.items {
                    let Some(a) = &p.type_annotation else {
                        bail!("untyped callback parameter")
                    };
                    params.push(self.ty(&a.type_annotation, hint)?);
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
                let TSTypeName::IdentifierReference(id) = &r.type_name else {
                    bail!("qualified type name")
                };
                let args: Vec<&'a TSType<'a>> = r
                    .type_arguments
                    .as_ref()
                    .map(|a| a.params.iter().collect())
                    .unwrap_or_default();
                match id.name.as_str() {
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
            _ => bail!("unsupported type annotation"),
        })
    }

    fn members(&mut self, members: &'a [TSSignature<'a>], hint: &str) -> R<Vec<Field>> {
        let mut fields = Vec::new();
        for m in members {
            let TSSignature::TSPropertySignature(p) = m else {
                bail!("unsupported member in object type")
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

    /// An interface or alias by name, resolved once.
    pub(crate) fn named_type(&mut self, name: &str) -> R<Ty> {
        if let Some(t) = self.named.get(name) {
            return Ok(t.clone());
        }
        if self.manifest.values.contains_key(name) {
            return Ok(Ty::Obj(self.value_struct(name)));
        }
        if self.manifest.handles.contains(name) {
            return Ok(Ty::Handle(name.to_string()));
        }
        if self.class_decls.contains_key(name) {
            return self.class_type(name);
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
            (a, b) if a.strip() == b.strip() => Ok(Expr {
                kind: ExprKind::Cast(Box::new(e)),
                ty: to.clone(),
            }),
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
