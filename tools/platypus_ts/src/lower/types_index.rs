// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

//! Indexed access types as TypeScript has them: `T["k"]`, `T["a" | "b"]` and `T[keyof X]` (the
//! union of the properties' types), `T[number]` and `T[0]` on arrays and tuples, `typeof x["k"]`,
//! nested `A["b"]["c"]`, and `T[K]` in a generic function or alias whose `K` stands for literals
//! (a copy per set of them: `TypeEnv::keys`). The compiled `PlatypusStyle` is a list of entries;
//! its property types come from the prelude's `interface __PlatypusStyleProps` (`ui_dts.rs`).

use super::generics::{GenericFn, TKeys};
use super::*;
use oxc_ast_visit::{Visit, walk};

/// What an index type stands for.
enum Index {
    /// `number`: any element.
    Number,
    /// `0`, `1`: a tuple position.
    Position(usize),
    /// `string`: any entry of a dictionary.
    String,
    /// String keys: a literal, a union of them, `keyof X`, a key parameter.
    Keys(Vec<String>),
}

/// The type parameters (of `names`) a declaration indexes a type by: the `K` of `T[K]`.
struct IndexedBy<'n> {
    names: &'n [String],
    found: Vec<String>,
}

impl<'a> Visit<'a> for IndexedBy<'_> {
    fn visit_ts_indexed_access_type(&mut self, it: &TSIndexedAccessType<'a>) {
        let mut ix = &it.index_type;
        while let TSType::TSParenthesizedType(p) = ix {
            ix = &p.type_annotation;
        }
        if let TSType::TSTypeReference(r) = ix
            && r.type_arguments.is_none()
            && let TSTypeName::IdentifierReference(id) = &r.type_name
            && self.names.iter().any(|n| n == id.name.as_str())
            && !self.found.iter().any(|n| n == id.name.as_str())
        {
            self.found.push(id.name.to_string());
        }
        walk::walk_ts_indexed_access_type(self, it);
    }
}

/// Is `t` the bare type parameter `name`?
fn is_tparam(t: &TSType<'_>, name: &str) -> bool {
    match t {
        TSType::TSParenthesizedType(p) => is_tparam(&p.type_annotation, name),
        TSType::TSTypeReference(r) => {
            r.type_arguments.is_none()
                && matches!(&r.type_name, TSTypeName::IdentifierReference(id) if id.name == name)
        }
        _ => false,
    }
}

impl<'a> Lowerer<'a> {
    fn type_text(&self, t: &TSType<'_>) -> &str {
        let sp = oxc_span::GetSpan::span(t);
        self.src[sp.start as usize..sp.end as usize].trim()
    }

    /// `O[I]` (`t` is the whole type).
    pub(crate) fn indexed_access(
        &mut self,
        t: &'a TSType<'a>,
        i: &'a TSIndexedAccessType<'a>,
        hint: &str,
    ) -> R<Ty> {
        // `(typeof NAMES)[number]` where `NAMES = ["a", "b"] as const`: the union of them.
        if matches!(i.index_type, TSType::TSNumberKeyword(_))
            && let Some(ms) = self.lit_members(t)
        {
            for m in &ms {
                self.tag(m);
            }
            return Ok(Ty::Lit);
        }
        let at = self.loc(i.span.start);
        let what = self.type_text(&i.object_type).to_string();
        let index = self.index_of(&i.index_type, &at)?;
        let obj = self.ty(&i.object_type, hint)?;
        // `PlatypusStyle["position"]` (also through aliases and type parameters): the declared
        // property's type, as the compiled style is a list of entries.
        if let Index::Keys(keys) = &index
            && self.is_style_list(&obj)
            && let Some(props) = self.ifaces.get(crate::ui_dts::STYLE_PROPS_IFACE).copied()
        {
            let mut tys = Vec::new();
            for k in keys {
                let Some(ann) = Self::property_type(&props.body.body, k) else {
                    bail!("{at}: property \"{k}\" does not exist on type {what}")
                };
                // Every style property is optional: `| undefined`, as in `tsc`.
                tys.push(self.ty(ann, hint)?.nullable());
            }
            return self.union_nullable(tys, &at);
        }
        self.index_ty(&obj, index, &at, &what)
    }

    /// The type of `obj` indexed by `index` (`what` is the object type as written).
    fn index_ty(&mut self, obj: &Ty, index: Index, at: &str, what: &str) -> R<Ty> {
        Ok(match (index, obj.strip()) {
            (Index::Number | Index::Position(_), Ty::Arr(el)) => (**el).clone(),
            (Index::Number | Index::Position(_), Ty::Typed(_)) => Ty::Num,
            (Index::Number | Index::String, Ty::Map(_, v)) => (**v).clone(),
            (Index::Number, Ty::Obj(sid)) if self.is_tuple(*sid) => {
                let tys = self.m.structs[*sid]
                    .fields
                    .iter()
                    .map(|f| f.ty.clone())
                    .collect();
                self.union_nullable(tys, at)?
            }
            (Index::Position(n), Ty::Obj(sid)) if self.is_tuple(*sid) => {
                let fields = &self.m.structs[*sid].fields;
                match fields.get(n) {
                    Some(f) => f.ty.clone(),
                    None => bail!(
                        "{at}: tuple type {what} of length {} has no element at index {n}",
                        fields.len()
                    ),
                }
            }
            (Index::Keys(keys), Ty::Arr(_) | Ty::Str | Ty::Typed(_))
                if keys.iter().all(|k| k == "length") =>
            {
                Ty::Num
            }
            (Index::Keys(keys), Ty::Obj(sid))
                if self.is_tuple(*sid) && keys.iter().all(|k| k == "length") =>
            {
                Ty::Num
            }
            (Index::Keys(keys), Ty::Obj(sid)) => {
                let st = &self.m.structs[*sid];
                let mut idxs = Vec::new();
                for k in &keys {
                    let Some(f) = st.field(k) else {
                        bail!("{at}: property \"{k}\" does not exist on type {what}")
                    };
                    idxs.push(f);
                }
                // In field order, as `o[k]` reads them, so both make the same union.
                idxs.sort_unstable();
                idxs.dedup();
                let tys = idxs.iter().map(|f| st.fields[*f].ty.clone()).collect();
                self.union_nullable(tys, at)?
            }
            (Index::Keys(keys), Ty::Map(k, v)) if **k == Ty::Str && !keys.is_empty() => {
                (**v).clone()
            }
            (index, t) => {
                let by = match index {
                    Index::Number => "number".to_string(),
                    Index::Position(n) => n.to_string(),
                    Index::String => "string".to_string(),
                    Index::Keys(k) => k
                        .iter()
                        .map(|k| format!("\"{k}\""))
                        .collect::<Vec<_>>()
                        .join(" | "),
                };
                bail!("{at}: type {what} ({t}) cannot be indexed by {by}")
            }
        })
    }

    /// What an index type stands for, resolved through aliases, `keyof` and the type parameters of
    /// the specialisation being lowered.
    fn index_of(&mut self, t: &'a TSType<'a>, at: &str) -> R<Index> {
        match t {
            TSType::TSParenthesizedType(p) => return self.index_of(&p.type_annotation, at),
            TSType::TSNumberKeyword(_) => return Ok(Index::Number),
            TSType::TSStringKeyword(_) => return Ok(Index::String),
            TSType::TSLiteralType(l) => {
                if let TSLiteral::NumericLiteral(n) = &l.literal {
                    if n.value >= 0.0 && n.value.fract() == 0.0 {
                        return Ok(Index::Position(n.value as usize));
                    }
                    bail!(
                        "{at}: an index type must be a whole number, not {}",
                        n.value
                    )
                }
            }
            _ => {}
        }
        if let Some(keys) = self.lit_members(t).or_else(|| self.keys_of_keyof(t)) {
            return Ok(Index::Keys(keys));
        }
        let text = self.type_text(t).to_string();
        // A type parameter whose arguments did not say which literals it is.
        if let TSType::TSTypeReference(r) = t
            && r.type_arguments.is_none()
            && let TSTypeName::IdentifierReference(id) = &r.type_name
            && let Some(bound) = self.tenv.get(id.name.as_str())
        {
            match bound.strip() {
                Ty::Num => return Ok(Index::Number),
                Ty::Str | Ty::Lit => bail!(
                    "{at}: indexed access by `{text}`: which keys `{text}` is is not known in this call (its argument is a `string`); pass a string literal, a value of a string-literal union type, or write the type argument"
                ),
                b => bail!("{at}: indexed access by `{text}`, which is {b} here"),
            }
        }
        bail!(
            "{at}: indexed access by `{text}`: an index type must be a string literal, a union of them, `keyof X`, `number` or a tuple position"
        )
    }

    /// Is `t` the compiled `PlatypusStyle` (a list of `StyleEntry`)?
    fn is_style_list(&self, t: &Ty) -> bool {
        matches!(t.strip(), Ty::Arr(e) if matches!(e.strip(), Ty::Obj(s) if self.m.structs[*s].name == "StyleEntry"))
    }

    /// The prelude's interface of the style properties, when `name` is the prelude's `PlatypusStyle`.
    pub(crate) fn style_props_iface(&self, name: &str) -> Option<&'a TSInterfaceDeclaration<'a>> {
        if name != "PlatypusStyle" {
            return None;
        }
        let alias = self.aliases.get(name)?;
        if !self.in_prelude(alias.span.start) {
            return None;
        }
        self.ifaces.get(crate::ui_dts::STYLE_PROPS_IFACE).copied()
    }

    /// The declared type of property `key` among `members`.
    fn property_type(members: &'a [TSSignature<'a>], key: &str) -> Option<&'a TSType<'a>> {
        members.iter().find_map(|m| match m {
            TSSignature::TSPropertySignature(p)
                if p.key.static_name().is_some_and(|n| n == key) =>
            {
                p.type_annotation.as_ref().map(|a| &a.type_annotation)
            }
            _ => None,
        })
    }

    /// The members of an object type as written: a type literal, an interface without `extends`,
    /// an alias of one, or `PlatypusStyle`.
    fn object_members(&self, t: &'a TSType<'a>) -> Option<&'a [TSSignature<'a>]> {
        match t {
            TSType::TSParenthesizedType(p) => self.object_members(&p.type_annotation),
            TSType::TSTypeLiteral(l) => Some(&l.members),
            TSType::TSTypeReference(r) if r.type_arguments.is_none() => {
                let TSTypeName::IdentifierReference(id) = &r.type_name else {
                    return None;
                };
                let name = id.name.as_str();
                if let Some(i) = self.style_props_iface(name) {
                    return Some(&i.body.body);
                }
                if let Some(i) = self.ifaces.get(name) {
                    return i.extends.is_empty().then_some(&i.body.body[..]);
                }
                let a = self.aliases.get(name)?;
                if a.type_parameters.is_some() {
                    return None;
                }
                self.object_members(&a.type_annotation)
            }
            _ => None,
        }
    }

    /// The string literals `O[K]` is when the properties `K` names are typed as literal unions
    /// (for `lit_members`).
    pub(crate) fn indexed_lit_members(
        &self,
        i: &'a TSIndexedAccessType<'a>,
    ) -> Option<Vec<String>> {
        let keys = self.lit_members(&i.index_type)?;
        let members = self.object_members(&i.object_type)?;
        let mut out: Vec<String> = Vec::new();
        for k in &keys {
            for m in self.lit_members_opt(Self::property_type(members, k)?)? {
                if !out.contains(&m) {
                    out.push(m);
                }
            }
        }
        Some(out)
    }

    /// The property names of a struct type (`keyof T` for a type parameter bound to it).
    pub(crate) fn field_names(&self, t: &Ty) -> Option<Vec<String>> {
        let Ty::Obj(sid) = t.strip() else {
            return None;
        };
        Some(
            self.m.structs[*sid]
                .fields
                .iter()
                .filter(|f| !f.name.starts_with("_p_") && !f.name.starts_with('$'))
                .map(|f| f.name.clone())
                .collect(),
        )
    }

    /// The union of property types, some of which may be `undefined`: the union of the rest, or
    /// null.
    pub(crate) fn union_nullable(&mut self, tys: Vec<Ty>, at: &str) -> R<Ty> {
        let nullable = tys
            .iter()
            .any(|t| t.is_nullable() || matches!(t, Ty::Null | Ty::Void));
        let rest: Vec<Ty> = tys
            .iter()
            .filter(|t| !matches!(t, Ty::Null | Ty::Void))
            .map(|t| t.strip().clone())
            .collect();
        if rest.is_empty() {
            if tys.is_empty() {
                bail!("{at}: indexed access by no keys")
            }
            return Ok(Ty::Null);
        }
        let u = self.union_of(rest)?;
        Ok(if nullable { u.nullable() } else { u })
    }

    // ---- typeof ----

    /// `typeof x`: a local's type, or a top-level constant's from its value.
    pub(crate) fn typeof_ident(&mut self, name: &str) -> R<Ty> {
        match self.lookup(name) {
            Some(Binding::Direct(v)) => return Ok(self.cur.locals[v as usize].clone()),
            Some(Binding::Boxed(_, ty)) => return Ok(ty),
            None => {}
        }
        let Some(init) = self.const_inits.get(name).copied() else {
            bail!("typeof {name}: only a top-level constant with a value")
        };
        match self.guess_ty(init, 0)? {
            Some(t) => Ok(t),
            None => bail!(
                "typeof {name}: the type of its value is not known without lowering it; write the type"
            ),
        }
    }

    /// `typeof a.b.c`: the type of the property reached.
    pub(crate) fn typeof_path(&mut self, n: &'a TSQualifiedName<'a>, at: u32) -> R<Ty> {
        let base = match &n.left {
            TSTypeName::IdentifierReference(id) => self.typeof_ident(id.name.as_str())?,
            TSTypeName::QualifiedName(q) => self.typeof_path(q, at)?,
            TSTypeName::ThisExpression(_) => {
                bail!("{}: `typeof this.x` is not supported", self.loc(at))
            }
        };
        let sp = oxc_span::GetSpan::span(&n.left);
        let left = &self.src[sp.start as usize..sp.end as usize];
        let key = n.right.name.as_str();
        match base.strip() {
            Ty::Obj(sid) => {
                let st = &self.m.structs[*sid];
                match st.field(key) {
                    Some(f) => Ok(st.fields[f].ty.clone()),
                    None => bail!(
                        "{}: property \"{key}\" does not exist on typeof {left}",
                        self.loc(at)
                    ),
                }
            }
            Ty::Arr(_) | Ty::Str if key == "length" => Ok(Ty::Num),
            t => bail!(
                "{}: typeof {left}.{key}: {left} is {t}, not an object",
                self.loc(at)
            ),
        }
    }

    // ---- key parameters ----

    /// The literals each key parameter of a generic function call stands for (`K` of
    /// `get<T, K extends keyof T>(o: T, k: K): T[K]` called as `get(p, "x")`), from a written type
    /// argument or the arguments of the parameters typed `K`. Those parameters become `Ty::Lit`.
    pub(crate) fn key_args(
        &mut self,
        g: &GenericFn<'a>,
        explicit: Option<&'a TSTypeParameterInstantiation<'a>>,
        args: &'a [Argument<'a>],
        targs: &mut [Ty],
    ) -> TKeys {
        let names: Vec<String> = g.tparams.iter().map(|(n, _)| n.clone()).collect();
        let mut v = IndexedBy {
            names: &names,
            found: Vec::new(),
        };
        v.visit_formal_parameters(g.params);
        if let Some(r) = g.ret {
            v.visit_ts_type(r);
        }
        match g.body {
            FnBody::Block(b) => v.visit_function_body(b),
            FnBody::Expr(e) => v.visit_expression(e),
        }
        let mut out = TKeys::new();
        for k in v.found {
            let pos = names
                .iter()
                .position(|n| *n == k)
                .expect("a type parameter");
            let lits = match explicit.and_then(|e| e.params.get(pos)) {
                Some(t) => self.lit_members(t),
                None => self.arg_lits(g, &k, args),
            };
            if let Some(mut lits) = lits {
                lits.sort();
                lits.dedup();
                for m in &lits {
                    self.tag(m);
                }
                targs[pos] = Ty::Lit;
                out.push((k, lits));
            }
        }
        out
    }

    /// The literals the arguments of the parameters typed `k` are, if every one is known.
    fn arg_lits(
        &mut self,
        g: &GenericFn<'a>,
        k: &str,
        args: &'a [Argument<'a>],
    ) -> Option<Vec<String>> {
        let mut all: Option<Vec<String>> = None;
        for (p, a) in g.params.items.iter().zip(args) {
            let Some(ann) = &p.type_annotation else {
                continue;
            };
            if !is_tparam(&ann.type_annotation, k) {
                continue;
            }
            let lits = self.expr_lits(a.as_expression()?)?;
            all.get_or_insert_with(Vec::new).extend(lits);
        }
        all
    }

    /// The literals of the key parameters of a generic alias or interface used with arguments
    /// (`Get<P, "x">`); those arguments become `Ty::Lit`.
    pub(crate) fn alias_key_args(
        &mut self,
        base: &str,
        args: &[&'a TSType<'a>],
        targs: &mut [Ty],
    ) -> TKeys {
        let iface = self.ifaces.get(base).copied();
        let alias = self.aliases.get(base).copied();
        let params = match (iface, alias) {
            (Some(i), _) => i.type_parameters.as_deref(),
            (None, Some(a)) => a.type_parameters.as_deref(),
            _ => return TKeys::new(),
        };
        let names: Vec<String> = generics::tparam_list(params)
            .into_iter()
            .map(|(n, _)| n)
            .collect();
        let mut v = IndexedBy {
            names: &names,
            found: Vec::new(),
        };
        match (iface, alias) {
            (Some(i), _) => v.visit_ts_interface_body(&i.body),
            (None, Some(a)) => v.visit_ts_type(&a.type_annotation),
            _ => {}
        }
        let mut out = TKeys::new();
        for k in v.found {
            let pos = names
                .iter()
                .position(|n| *n == k)
                .expect("a type parameter");
            let Some(mut lits) = args.get(pos).and_then(|a| self.lit_members(a)) else {
                continue;
            };
            lits.sort();
            lits.dedup();
            if let Some(t) = targs.get_mut(pos) {
                *t = Ty::Lit;
            }
            out.push((k, lits));
        }
        out
    }
}
