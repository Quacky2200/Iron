//! Accessors in object literals: `{ sfx: .., get scene() { return self.effects; } }`.
//!
//! The getter runs on every read, so it cannot be a value stored when the object is made (the
//! object it reads may not exist yet). A name that has a getter somewhere in the program gets a
//! hidden companion field `_p_get_<name>` (a nullable closure) in every plain struct that has a
//! field of that name; the literal stores its getter there, and a read of the field calls the
//! closure when there is one (`member_of`). A setter (`set v(n) { .. }`) is the same with
//! `_p_set_<name>`: a write of the field calls the closure when there is one (`Place::Thunked`).
//! Either may use `this`: it is the object being made (`lower_this_prop` in `expr.rs`).

use super::expr::mk;
use super::*;
use oxc_ast_visit::{Visit, walk};

#[derive(Default)]
struct Names {
    getters: HashSet<String>,
    setters: HashSet<String>,
}

impl<'a> Visit<'a> for Names {
    fn visit_object_property(&mut self, it: &ObjectProperty<'a>) {
        if !it.computed
            && let PropertyKey::StaticIdentifier(id) = &it.key
        {
            match it.kind {
                PropertyKind::Get => {
                    self.getters.insert(id.name.to_string());
                }
                PropertyKind::Set => {
                    self.setters.insert(id.name.to_string());
                }
                PropertyKind::Init => {}
            }
        }
        walk::walk_object_property(self, it);
    }
}

/// The hidden field that holds the getter of `name`.
pub(crate) fn thunk_name(name: &str) -> String {
    format!("_p_get_{name}")
}

/// The hidden field that holds the setter of `name`.
pub(crate) fn setter_name(name: &str) -> String {
    format!("_p_set_{name}")
}

impl<'a> Lowerer<'a> {
    /// The names that have a getter in an object literal of the program.
    pub(crate) fn scan_accessors(&mut self, program: &Program<'a>) {
        let mut v = Names::default();
        v.visit_program(program);
        self.accessor_names.extend(v.getters);
        self.setter_names.extend(v.setters);
    }

    /// `fields` with a hidden getter field after each field that some literal gives a getter.
    pub(crate) fn with_thunks(&self, fields: Vec<Field>) -> Vec<Field> {
        if self.accessor_names.is_empty() && self.setter_names.is_empty() {
            return fields;
        }
        let mut out = Vec::with_capacity(fields.len());
        for f in fields {
            let plain = !f.name.starts_with("_p_") && !matches!(f.ty, Ty::Fn(_));
            let hidden = self.accessor_names.contains(&f.name) && plain;
            let hidden_set = self.setter_names.contains(&f.name) && plain;
            let (name, ty) = (f.name.clone(), f.ty.clone());
            out.push(f);
            if hidden {
                out.push(Field {
                    name: thunk_name(&name),
                    ty: Ty::Fn(Box::new(FnTy {
                        params: vec![],
                        ret: ty.clone(),
                    }))
                    .nullable(),
                    optional: false,
                });
            }
            if hidden_set {
                out.push(Field {
                    name: setter_name(&name),
                    ty: Ty::Fn(Box::new(FnTy {
                        params: vec![ty],
                        ret: Ty::Void,
                    }))
                    .nullable(),
                    optional: false,
                });
            }
        }
        out
    }

    /// `obj.prop` where the struct has a getter closure for it: the closure's result when set.
    pub(crate) fn thunked_read(&mut self, obj: Expr, sid: StructId, i: usize, prop: &str) -> Expr {
        let ty = self.m.structs[sid].fields[i].ty.clone();
        let Some(t) = self.m.structs[sid].field(&thunk_name(prop)) else {
            return mk(ExprKind::Field(Box::new(obj), sid, i), ty);
        };
        let fty = self.m.structs[sid].fields[t].ty.clone();
        let tmp = self.new_local(obj.ty.clone());
        let o = self.var_expr(tmp);
        let thunk = mk(ExprKind::Field(Box::new(o.clone()), sid, t), fty.clone());
        let plain = mk(ExprKind::Field(Box::new(o), sid, i), ty.clone());
        let callee = mk(ExprKind::Cast(Box::new(thunk.clone())), fty.strip().clone());
        let call = mk(ExprKind::CallClosure(Box::new(callee), vec![]), ty.clone());
        let is_null = mk(ExprKind::IsNull(Box::new(thunk)), Ty::Bool);
        mk(
            ExprKind::Seq(
                vec![Stmt::Assign(tmp, obj)],
                Box::new(mk(
                    ExprKind::Cond(Box::new(is_null), Box::new(plain), Box::new(call)),
                    ty.clone(),
                )),
            ),
            ty,
        )
    }
}
