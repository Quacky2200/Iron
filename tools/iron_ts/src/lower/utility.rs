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
            Ty::Obj(sid) => self.class_of_sid.contains_key(sid),
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
