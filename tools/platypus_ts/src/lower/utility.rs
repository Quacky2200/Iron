// SPDX-License-Identifier: Apache-2.0

//! `Partial<T>`, `Required<T>`, `Readonly<T>`, `Pick<T, K>` and `Omit<T, K>`: plain objects whose
//! fields come from `T` (an interface, an alias, a class, or an engine class: its settable
//! properties), so option bags written as `Partial<LineRenderer2D>` are ordinary structs.
//!
//! `Readonly<T>` is `T` itself (no copy: nothing here checks writes). The others make a new
//! struct (one per shape, like any object type); a value of `T` goes where such a struct is
//! expected by a field-by-field copy (`coerce`).

use super::*;
use std::collections::BTreeSet;

impl<'a> Lowerer<'a> {
    pub(crate) fn utility_type(
        &mut self,
        name: &str,
        args: &[&'a TSType<'a>],
        hint: &str,
    ) -> R<Ty> {
        let base = self.ty(args[0], hint)?;
        if name == "Readonly" {
            return Ok(base);
        }
        // A dictionary already has optional entries.
        if matches!(base.strip(), Ty::Map(..)) && matches!(name, "Partial" | "Required") {
            return Ok(base);
        }
        let keys = if matches!(name, "Pick" | "Omit") {
            let Some(arg) = args.get(1) else {
                bail!("{name}<T, K> needs its keys")
            };
            let Some(keys) = self.lit_members_opt(arg) else {
                bail!("{name}<T, K>: K must be a union of string literals")
            };
            Some(keys)
        } else {
            None
        };
        // A class declared later has no fields yet: fill the struct in once every class is laid out.
        let early = match base.strip() {
            // (an interface still being laid out, reached again through its own members, has none yet)
            Ty::Obj(sid) => {
                self.class_of_sid.contains_key(sid) || self.m.structs[*sid].fields.is_empty()
            }
            // The facade's property functions are declared after the program's own code.
            Ty::Handle(_) => true,
            _ => false,
        };
        if early && !self.classes_ready {
            // One struct per (utility, type, keys).
            let key = format!("{name}|{}|{keys:?}", base.strip());
            if let Some(id) = self.shapes.get(&key) {
                return Ok(Ty::Obj(*id));
            }
            let id = self.new_struct(&format!("{name}_{hint}"), vec![]);
            self.shapes.insert(key, id);
            self.pending_utility
                .push((id, name.to_string(), base.strip().clone(), keys));
            return Ok(Ty::Obj(id));
        }
        let fields = self.type_fields(base.strip(), name)?;
        let fields = Self::shape_utility(name, fields, keys)?;
        Ok(Ty::Obj(self.anon_struct(&format!("{name}_{hint}"), fields)))
    }

    /// Fills the structs of the `Partial<Class>` types met before the classes were laid out.
    pub(crate) fn finish_utility_types(&mut self) -> R<()> {
        for (id, name, base, keys) in std::mem::take(&mut self.pending_utility) {
            let fields = self.type_fields(&base, &name)?;
            self.m.structs[id].fields = Self::shape_utility(&name, fields, keys)?;
        }
        Ok(())
    }

    fn shape_utility(
        name: &str,
        mut fields: Vec<Field>,
        keys: Option<Vec<String>>,
    ) -> R<Vec<Field>> {
        match name {
            "Pick" | "Omit" => {
                let keys = keys.unwrap_or_default();
                for k in &keys {
                    if name == "Pick" && !fields.iter().any(|f| &f.name == k) {
                        bail!("Pick: `{k}` is not a property of the type")
                    }
                }
                fields.retain(|f| keys.contains(&f.name) == (name == "Pick"));
            }
            "Partial" => {
                for f in &mut fields {
                    f.ty = f.ty.clone().nullable();
                    f.optional = true;
                }
            }
            _ => {
                // Required
                for f in &mut fields {
                    if f.optional || f.ty.is_nullable() {
                        f.ty = f.ty.strip().clone();
                    }
                    f.optional = false;
                }
            }
        }
        Ok(fields)
    }

    /// The data properties of an object type.
    fn type_fields(&mut self, t: &Ty, what: &str) -> R<Vec<Field>> {
        match t {
            Ty::Obj(sid) => Ok(self.m.structs[*sid]
                .fields
                .iter()
                .filter(|f| !f.name.starts_with("_p_"))
                .cloned()
                .collect()),
            Ty::Handle(class) => self.handle_props(class),
            other => bail!("{what}<T>: T must be an object type, not {other}"),
        }
    }

    /// The settable properties of an engine class as TypeScript sees them: the manifest's setters,
    /// `setX(v)` methods, and the facade's `__xs_Class_x` functions.
    pub(crate) fn handle_props(&mut self, class: &str) -> R<Vec<Field>> {
        let mut names: BTreeSet<String> = BTreeSet::new();
        if let Some(k) = self.manifest.classes.get(class) {
            names.extend(k.setters.keys().cloned());
            for m in k.methods.keys() {
                if let Some(rest) = m.strip_prefix("set")
                    && rest.chars().next().is_some_and(|c| c.is_ascii_uppercase())
                    && self.set_method(class, &lower_first(rest)).is_some()
                {
                    names.insert(lower_first(rest));
                }
            }
        }
        let prefix = format!("__xs_{class}_");
        for n in self.fids.keys() {
            if let Some(p) = n.strip_prefix(&prefix) {
                names.insert(p.to_string());
            }
        }
        let mut out = Vec::new();
        for n in names {
            let ty = if let Some(&fid) = self.fids.get(&format!("{prefix}{n}")) {
                self.fsig[fid].0[1].clone()
            } else if let Some(s) = self.manifest.classes[class].setters.get(&n).cloned() {
                self.ext_ir(&s.params[1].1)
            } else if let Some(s) = self.set_method(class, &n) {
                self.ext_ir(&s.params[1].1)
            } else {
                continue;
            };
            out.push(Field {
                name: n,
                ty,
                optional: false,
            });
        }
        if out.is_empty() {
            bail!("`{class}` has no settable properties to build an object type from")
        }
        Ok(out)
    }
}

fn lower_first(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_lowercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

impl<'a> Lowerer<'a> {
    /// The signature of the function a `typeof f` / `typeof f<string>` (an instantiation
    /// expression) names: a top-level function, a generic one with its type arguments written, or
    /// a constant closure. A return type that is inferred comes from lowering the body now.
    fn fn_of_query(&mut self, q: &'a TSTypeQuery<'a>, hint: &str) -> R<FnTy> {
        let TSTypeQueryExprName::IdentifierReference(id) = &q.expr_name else {
            bail!("typeof of a qualified name")
        };
        let name = id.name.as_str();
        let fname = if self.generic_fns.contains_key(name) {
            let Some(targs) = &q.type_arguments else {
                bail!(
                    "typeof {name}: a generic function needs its type arguments (typeof {name}<T>)"
                )
            };
            let mut tys = Vec::new();
            for t in &targs.params {
                tys.push(self.ty(t, hint)?);
            }
            if tys.len() != self.generic_fns[name].tparams.len() {
                bail!("typeof {name}<..>: write every type argument")
            }
            let eager = std::mem::replace(&mut self.eager_classes, !self.classes_ready);
            let r = self.instantiate_fn(name, tys);
            self.eager_classes = eager;
            r?
        } else if self.fids.contains_key(name) {
            name.to_string()
        } else if let Some(f) = self.top_fn_decls.get(name).copied() {
            // Not collected yet (a type met before the function): its written signature.
            let mut params = Vec::new();
            for p in &f.params.items {
                let Some(a) = &p.type_annotation else {
                    bail!("typeof {name}: its parameters need types")
                };
                let t = self.ty(&a.type_annotation, hint)?;
                params.push(if p.optional { t.nullable() } else { t });
            }
            let Some(r) = &f.return_type else {
                bail!("typeof {name}: write its return type")
            };
            let ret = self.ty(&r.type_annotation, hint)?;
            return Ok(FnTy { params, ret });
        } else {
            bail!("typeof {name}: only a top-level function here")
        };
        let fid = self.fids[&fname];
        if self.fsig[fid].1.is_none() {
            let eager = std::mem::replace(&mut self.eager_classes, !self.classes_ready);
            let r = self.ensure_lowered(&fname);
            self.eager_classes = eager;
            r?;
        }
        let (params, ret) = self.fsig[fid].clone();
        let Some(ret) = ret else {
            bail!("typeof {name}: its return type is not known yet; write it")
        };
        Ok(FnTy { params, ret })
    }

    /// The function type `ReturnType<F>` / `Parameters<F>` look into: `typeof f` or a function type.
    fn fn_type_of(&mut self, t: &'a TSType<'a>, what: &str, hint: &str) -> R<FnTy> {
        match t {
            TSType::TSParenthesizedType(p) => self.fn_type_of(&p.type_annotation, what, hint),
            TSType::TSTypeQuery(q) => self.fn_of_query(q, hint),
            other => match self.ty(other, hint)? {
                Ty::Fn(f) => Ok(*f),
                t => bail!("{what}<F>: F must be a function type, not {t}"),
            },
        }
    }

    /// `ReturnType<F>`, `Parameters<F>`, `Awaited<T>`, `NonNullable<T>`, `Exclude<T, U>`,
    /// `Extract<T, U>`; `None` for any other name.
    pub(crate) fn alias_utility(
        &mut self,
        name: &str,
        args: &[&'a TSType<'a>],
        hint: &str,
    ) -> R<Option<Ty>> {
        let arity = match name {
            "ReturnType" | "Parameters" | "Awaited" | "NonNullable" => 1,
            "Exclude" | "Extract" => 2,
            _ => return Ok(None),
        };
        if args.len() != arity {
            return Ok(None);
        }
        Ok(Some(match name {
            "ReturnType" => self.fn_type_of(args[0], name, hint)?.ret,
            "Parameters" => {
                let f = self.fn_type_of(args[0], name, hint)?;
                self.tuple_ty(f.params)
            }
            "Awaited" => {
                let t = self.ty(args[0], hint)?;
                match self.promise_of(&t) {
                    Some((_, value)) => value,
                    None => t,
                }
            }
            "NonNullable" => self.ty(args[0], hint)?.strip().clone(),
            _ => {
                // `Exclude<"a" | "b", "a">`: the members left (or kept); a union with `null`:
                // that part left out (or kept).
                let keep = name == "Extract";
                if let (Some(all), Some(some)) =
                    (self.lit_members(args[0]), self.lit_members(args[1]))
                {
                    let members: Vec<String> = all
                        .into_iter()
                        .filter(|m| some.contains(m) == keep)
                        .collect();
                    for m in &members {
                        self.tag(m);
                    }
                    return Ok(Some(Ty::Lit));
                }
                let t = self.ty(args[0], hint)?;
                let u = self.ty(args[1], hint)?;
                if matches!(u, Ty::Null | Ty::Void) {
                    if keep { u } else { t.strip().clone() }
                } else if t.strip() == u.strip() {
                    if keep { t } else { Ty::Void }
                } else if keep {
                    bail!("Extract<T, U>: only unions of string literals are supported")
                } else {
                    t
                }
            }
        }))
    }
}

impl<'a> Lowerer<'a> {
    /// Notes every top-level function declaration, and registers the generic ones (a type may name
    /// `typeof f<T>` before `f` is collected).
    pub(crate) fn register_top_functions(&mut self, program: &'a Program<'a>) {
        for stmt in &program.body {
            let f = match stmt {
                Statement::FunctionDeclaration(f) => Some(&**f),
                Statement::ExportDeclaration(x) => match &x.declaration {
                    Declaration::FunctionDeclaration(f) => Some(&**f),
                    _ => None,
                },
                _ => None,
            };
            let Some(f) = f else { continue };
            let (Some(id), Some(body)) = (&f.id, &f.body) else {
                continue;
            };
            self.top_fn_decls.insert(id.name.to_string(), f);
            if f.type_parameters.is_some() {
                self.generic_fns.insert(
                    id.name.to_string(),
                    generics::GenericFn {
                        tparams: generics::tparam_list(f.type_parameters.as_deref()),
                        params: &f.params,
                        ret: f.return_type.as_ref().map(|r| &r.type_annotation),
                        body: FnBody::Block(body),
                    },
                );
            }
        }
    }
}
