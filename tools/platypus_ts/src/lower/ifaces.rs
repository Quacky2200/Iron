// SPDX-License-Identifier: Apache-2.0

//! Interfaces implemented by classes: `interface Shape { area(): number }`, `class Circle implements
//! Shape`, a `Shape[]` holding circles and squares, `shape.area()`.
//!
//! A *dynamic* interface is one that some class names in `implements` (and the interfaces it
//! extends). A value of its type is a pointer to any object that has the members: a class instance
//! or a plain object. Every object starts with the id of its struct, so a member access is a call
//! through a table indexed by that id (`CallVirtual`, the mechanism classes use for overridden
//! methods), one table per (interface, member, kind):
//!
//! - **call**: `shape.area(..)` of a method, the table holds the method itself (or, for an object
//!   whose member is a function field, a small wrapper that calls the closure);
//! - **get**: `shape.name`, or a method / optional method read as a value, the table holds a getter
//!   (a wrapper that reads the field, calls the accessor, binds the method or gives null);
//! - **set**: `shape.name = v`.
//!
//! Which structs go in the tables is only known once the whole program is lowered (a struct
//! counts when a value of it is assigned to the interface type, subclasses included): the tables
//! are filled at the end (`finish_ifaces`), repeating while that lowers more code. Classes are not
//! copied or wrapped, so `shape instanceof Circle`, identity and writes through the interface work
//! as in JavaScript. An interface struct itself is never built or read: a use that would
//! (destructuring, spread, `Object.keys`, JSON) is a compile error naming the interface.

use super::expr::{Place, mk};
use super::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum TableKind {
    Call,
    Get,
    Set,
}

pub(crate) struct IfaceTable {
    pub iface: StructId,
    pub member: String,
    pub kind: TableKind,
    /// Index into `Module::vtables`.
    pub vt: usize,
    /// Structs that have an entry.
    pub done: HashSet<StructId>,
    /// The member's type (for a call: the function type).
    pub ty: Ty,
}

impl<'a> Lowerer<'a> {
    // ---- which interfaces are dynamic ----

    /// The interfaces an interface extends, by name.
    pub(crate) fn iface_parents(&self, name: &str) -> Vec<String> {
        let Some(i) = self.ifaces.get(name) else {
            return vec![];
        };
        i.extends
            .iter()
            .filter_map(|h| match &h.type_name {
                TSTypeName::IdentifierReference(id) => Some(id.name.to_string()),
                _ => None,
            })
            .collect()
    }

    /// Marks the interfaces classes implement, and the ones those extend, as dynamic.
    pub(crate) fn mark_dynamic_ifaces(&mut self, classes: &[&'a Class<'a>]) {
        let mut stack: Vec<String> = Vec::new();
        for c in classes {
            for imp in &c.implements {
                if imp.type_arguments.is_some() {
                    continue;
                }
                if let TSTypeName::IdentifierReference(id) = &imp.expression
                    && self.ifaces.contains_key(id.name.as_str())
                {
                    stack.push(id.name.to_string());
                }
            }
        }
        while let Some(n) = stack.pop() {
            if self.dyn_ifaces.insert(n.clone()) {
                stack.extend(self.iface_parents(&n));
            }
        }
    }

    pub(crate) fn is_dyn(&self, sid: StructId) -> bool {
        self.iface_sids.contains_key(&sid)
    }

    /// Is interface `a` the interface `b` or one that extends it (through any number of steps)?
    fn iface_extends(&self, a: &str, b: &str) -> bool {
        if a == b {
            return true;
        }
        self.iface_parents(a)
            .iter()
            .any(|p| self.iface_extends(p, b))
    }

    // ---- assignability ----

    /// Can an object of struct `x` be used where the dynamic interface struct `y` is expected?
    /// Records it as an implementer when it can.
    pub(crate) fn iface_accepts(&self, x: StructId, y: StructId) -> bool {
        let Some(yname) = self.iface_sids.get(&y) else {
            return false;
        };
        if let Some(xname) = self.iface_sids.get(&x) {
            return self.iface_extends(xname, yname);
        }
        let ys = &self.m.structs[y];
        let xs = &self.m.structs[x];
        let ok = ys.fields.iter().all(|f| {
            f.optional
                || xs.field(&f.name).is_some()
                || self.resolve_in(x, Which::Method, &f.name).is_some()
                || self.resolve_in(x, Which::Getter, &f.name).is_some()
        });
        if ok {
            self.iface_impls.borrow_mut().insert((y, x));
        }
        ok
    }

    /// All the structs that count as implementers of `iface`: the registered ones, those of the
    /// interfaces that extend it, and the subclasses of every class among them.
    fn implementers(&self, iface: StructId) -> Vec<StructId> {
        let name = self.iface_sids[&iface].clone();
        let mut direct: Vec<StructId> = Vec::new();
        for (i, s) in self.iface_impls.borrow().iter() {
            if let Some(iname) = self.iface_sids.get(i)
                && self.iface_extends(iname, &name)
            {
                direct.push(*s);
            }
        }
        let mut out: Vec<StructId> = Vec::new();
        for s in direct {
            out.push(s);
            if self.class_of_sid.contains_key(&s) {
                out.extend(
                    self.class_of_sid
                        .keys()
                        .copied()
                        .filter(|c| *c != s && self.is_subclass(*c, s)),
                );
            }
        }
        out.sort_unstable();
        out.dedup();
        // Abstract classes have no instances of their own.
        out.retain(|s| {
            self.class_of_sid
                .get(s)
                .is_none_or(|n| !self.classes[n].is_abstract)
        });
        out
    }

    /// A class that has a member the interface does not: `tsc` narrowed the value (`face instanceof
    /// FlipFace`), so use it as that class. The implementers decide.
    pub(crate) fn iface_downcast(
        &self,
        iface: StructId,
        what: impl Fn(&Self, StructId) -> bool,
    ) -> Option<StructId> {
        let cands: Vec<StructId> = self
            .implementers(iface)
            .into_iter()
            .filter(|s| self.class_of_sid.contains_key(s) && what(self, *s))
            .collect();
        let first = *cands.first()?;
        cands
            .iter()
            .all(|c| self.is_subclass(*c, first))
            .then_some(first)
    }

    // ---- member access ----

    fn iface_table(&mut self, iface: StructId, member: &str, kind: TableKind, ty: &Ty) -> usize {
        if let Some(t) = self
            .iface_tables
            .iter()
            .find(|t| t.iface == iface && t.member == member && t.kind == kind)
        {
            return t.vt;
        }
        self.m.vtables.push(VTable { entries: vec![] });
        let vt = self.m.vtables.len() - 1;
        self.iface_tables.push(IfaceTable {
            iface,
            member: member.to_string(),
            kind,
            vt,
            done: HashSet::new(),
            ty: ty.clone(),
        });
        vt
    }

    /// `v.name` on a value of a dynamic interface.
    pub(crate) fn iface_member(&mut self, obj: Expr, iface: StructId, name: &str) -> R<Expr> {
        let Some(i) = self.m.structs[iface].field(name) else {
            // A member only a class has (the value was narrowed with `instanceof`).
            let sub = self.iface_downcast(iface, |s, c| {
                s.has_field(c, name)
                    || s.resolve_in(c, Which::Getter, name).is_some()
                    || s.resolve_in(c, Which::Method, name).is_some()
            });
            if let Some(sub) = sub {
                return self.member_of(Self::downcast(obj, sub), name);
            }
            bail!(
                "interface {} has no member `{name}`",
                self.m.structs[iface].name
            )
        };
        let ty = self.m.structs[iface].fields[i].ty.clone();
        let vt = self.iface_table(iface, name, TableKind::Get, &ty);
        Ok(mk(ExprKind::CallVirtual(vt, vec![obj]), ty))
    }

    /// `v.m(args)` on a value of a dynamic interface whose member `m` is a method. `None` when `m`
    /// is not one (a value read then called, or a class-only method of a narrowed value).
    pub(crate) fn iface_call(
        &mut self,
        recv: &Expr,
        iface: StructId,
        m: &str,
        c: &'a CallExpression<'a>,
    ) -> R<Option<Expr>> {
        let Some(i) = self.m.structs[iface].field(m) else {
            let sub =
                self.iface_downcast(iface, |s, c| s.resolve_in(c, Which::Method, m).is_some());
            return match sub {
                Some(sub) => self
                    .lower_method(Self::downcast(recv.clone(), sub), m, c)
                    .map(Some),
                None => Ok(None),
            };
        };
        let fty = self.m.structs[iface].fields[i].ty.clone();
        let Ty::Fn(sig) = &fty else {
            return Ok(None);
        };
        let sig = (**sig).clone();
        let args = self.lower_args(&c.arguments, &sig.params, m)?;
        let vt = self.iface_table(iface, m, TableKind::Call, &fty);
        let mut all = vec![recv.clone()];
        all.extend(args);
        Ok(Some(mk(ExprKind::CallVirtual(vt, all), sig.ret)))
    }

    /// `v.name = x` on a value of a dynamic interface: the place that reads and writes through tables.
    pub(crate) fn iface_place(&mut self, obj: Expr, iface: StructId, name: &str) -> R<Place> {
        let Some(i) = self.m.structs[iface].field(name) else {
            bail!(
                "interface {} has no member `{name}`",
                self.m.structs[iface].name
            )
        };
        let ty = self.m.structs[iface].fields[i].ty.clone();
        let get = self.iface_table(iface, name, TableKind::Get, &ty);
        let set = self.iface_table(iface, name, TableKind::Set, &ty);
        Ok(Place::Accessor(
            obj,
            Some(classes::Target::Virtual(get, 0)),
            classes::Target::Virtual(set, 0),
            ty,
        ))
    }

    // ---- filling the tables ----

    /// A function of `ptys` parameters (the receiver first) whose body `build` makes; its
    /// statements run, then the expression is returned.
    fn entry_func(
        &mut self,
        name: String,
        ptys: Vec<Ty>,
        build: impl FnOnce(&mut Self, Vec<Expr>) -> R<(Vec<Stmt>, Option<Expr>)>,
    ) -> R<FuncId> {
        let saved = std::mem::take(&mut self.cur);
        let vars: Vec<VarId> = ptys.iter().map(|t| self.new_local(t.clone())).collect();
        let args: Vec<Expr> = vars.iter().map(|v| self.var_expr(*v)).collect();
        let built = build(self, args);
        let ctx = std::mem::replace(&mut self.cur, saved);
        let (mut body, value) = built?;
        let ret = match value {
            Some(v) => {
                let ret = v.ty.clone();
                body.push(Stmt::Return(Some(v)));
                ret
            }
            None => {
                body.push(Stmt::Return(None));
                Ty::Void
            }
        };
        let id = self.m.funcs.len();
        self.m.funcs.push(Func {
            name,
            exported: false,
            params: vars,
            ret: ret.clone(),
            locals: ctx.locals,
            body,
        });
        self.fsig.push((ptys, Some(ret)));
        self.fstate.push(2);
        Ok(id)
    }

    /// The entry of `table` for the struct `s`.
    fn iface_entry(&mut self, table: usize, s: StructId) -> R<FuncId> {
        let (iface, member, kind, ty) = {
            let t = &self.iface_tables[table];
            (t.iface, t.member.clone(), t.kind, t.ty.clone())
        };
        let ifname = self.m.structs[iface].name.clone();
        let sname = self.m.structs[s].name.clone();
        let label = format!("{ifname}.{kind:?}.{member}${sname}");
        let class = self.class_of_sid.contains_key(&s);
        let this_ty = Ty::Obj(s);
        let field = self.m.structs[s].field(&member);
        match kind {
            TableKind::Call => {
                if class && field.is_none() {
                    let Some(name) = self.resolve_in(s, Which::Method, &member) else {
                        bail!("{sname} does not implement {ifname}.{member}")
                    };
                    self.ensure_lowered(&name)?;
                    let fid = self.fids[&name];
                    let fp: &'a FormalParameters<'a> = self.fsrc[&name].params;
                    let Ty::Fn(sig) = ty.clone() else {
                        unreachable!("a call table is for a function member")
                    };
                    // The class method fills in its own defaults at its call sites: a call through
                    // the interface has to do it for the arguments left out (null).
                    if !fp.items.iter().any(|p| p.initializer.is_some()) {
                        return Ok(fid);
                    }
                    let own = self.fsig[fid].0.clone();
                    let mut ptys = vec![this_ty.clone()];
                    ptys.extend(sig.params.iter().cloned());
                    return self.entry_func(label, ptys, move |this, args| {
                        let mut names: HashMap<String, Binding> = HashMap::new();
                        let mut stmts = Vec::new();
                        let mut vals = vec![args[0].clone()];
                        for (i, p) in fp.items.iter().enumerate() {
                            let want = own[i + 1].clone();
                            let given = args[i + 1].clone();
                            let v = match &p.initializer {
                                Some(init) if given.ty.is_nullable() => {
                                    let saved = std::mem::replace(
                                        &mut this.cur.scopes,
                                        vec![names.clone()],
                                    );
                                    let d = this.lower_expr(init, Some(&want));
                                    this.cur.scopes = saved;
                                    let d = this.coerce(d?, &want)?;
                                    let have =
                                        mk(ExprKind::Cast(Box::new(given.clone())), want.clone());
                                    mk(
                                        ExprKind::Cond(
                                            Box::new(mk(
                                                ExprKind::IsNull(Box::new(given)),
                                                Ty::Bool,
                                            )),
                                            Box::new(d),
                                            Box::new(have),
                                        ),
                                        want.clone(),
                                    )
                                }
                                _ => this.coerce(given, &want)?,
                            };
                            let tmp = this.new_local(want.clone());
                            stmts.push(Stmt::Assign(tmp, v));
                            vals.push(this.var_expr(tmp));
                            if let BindingPattern::BindingIdentifier(b) = &p.pattern {
                                names.insert(b.name.to_string(), Binding::Direct(tmp));
                            }
                        }
                        let call = mk(ExprKind::Call(fid, vals), sig.ret.clone());
                        if sig.ret == Ty::Void {
                            stmts.push(Stmt::Eval(call));
                            Ok((stmts, None))
                        } else {
                            Ok((stmts, Some(call)))
                        }
                    });
                }
                let Some(fi) = field else {
                    bail!("{sname} does not implement {ifname}.{member}")
                };
                let Ty::Fn(sig) = ty.clone() else {
                    unreachable!("a call table is for a function member")
                };
                let mut ptys = vec![this_ty.clone()];
                ptys.extend(sig.params.iter().cloned());
                let fty = self.m.structs[s].fields[fi].ty.clone();
                self.entry_func(label, ptys, move |this, args| {
                    let f = mk(
                        ExprKind::Field(Box::new(args[0].clone()), s, fi),
                        fty.clone(),
                    );
                    let f = if f.ty.is_nullable() {
                        let plain = f.ty.strip().clone();
                        mk(ExprKind::Cast(Box::new(f)), plain)
                    } else {
                        f
                    };
                    let _ = this;
                    let call = mk(
                        ExprKind::CallClosure(Box::new(f), args[1..].to_vec()),
                        sig.ret.clone(),
                    );
                    Ok(if sig.ret == Ty::Void {
                        (vec![Stmt::Eval(call)], None)
                    } else {
                        (vec![], Some(call))
                    })
                })
            }
            TableKind::Get => {
                // An accessor of the class is its own getter.
                if let Some(name) = self.resolve_in(s, Which::Getter, &member) {
                    self.ensure_lowered(&name)?;
                    return Ok(self.fids[&name]);
                }
                let is_method =
                    field.is_none() && self.resolve_in(s, Which::Method, &member).is_some();
                self.entry_func(label, vec![this_ty.clone()], move |this, args| {
                    let this_e = args[0].clone();
                    let value = if let Some(fi) = field {
                        let fty = this.m.structs[s].fields[fi].ty.clone();
                        mk(ExprKind::Field(Box::new(this_e), s, fi), fty)
                    } else if is_method {
                        this.bound_method(this_e, s, &member)?
                    } else if ty.is_nullable() {
                        mk(ExprKind::Null, ty.clone())
                    } else {
                        bail!("{sname} does not implement {ifname}.{member}")
                    };
                    let value = this.coerce(value, &ty)?;
                    Ok((vec![], Some(value)))
                })
            }
            TableKind::Set => {
                if let Some(name) = self.resolve_in(s, Which::Setter, &member) {
                    self.ensure_lowered(&name)?;
                    return Ok(self.fids[&name]);
                }
                let Some(fi) = field else {
                    bail!("{sname} cannot be written through {ifname}.{member}")
                };
                self.entry_func(label, vec![this_ty, ty.clone()], move |this, args| {
                    let fty = this.m.structs[s].fields[fi].ty.clone();
                    let v = this.coerce(args[1].clone(), &fty)?;
                    Ok((vec![Stmt::StoreField(args[0].clone(), s, fi, v)], None))
                })
            }
        }
    }

    /// Fills the tables with the structs registered so far. True if it added anything (lowering
    /// what it needed may have registered more, or declared functions to lower).
    pub(crate) fn finish_ifaces(&mut self) -> R<bool> {
        let mut any = false;
        loop {
            let mut progress = false;
            let mut t = 0;
            while t < self.iface_tables.len() {
                let impls = self.implementers(self.iface_tables[t].iface);
                for s in impls {
                    if self.iface_tables[t].done.contains(&s) {
                        continue;
                    }
                    self.iface_tables[t].done.insert(s);
                    let fid = self.iface_entry(t, s)?;
                    let vt = self.iface_tables[t].vt;
                    self.m.vtables[vt].entries.push((s, fid));
                    progress = true;
                }
                t += 1;
            }
            if !progress {
                break;
            }
            any = true;
        }
        Ok(any)
    }

    /// Tables of interfaces nothing implements get one entry that does nothing, so the code that
    /// calls through them is well formed (it never runs).
    pub(crate) fn fill_empty_tables(&mut self) -> R<()> {
        for t in 0..self.iface_tables.len() {
            let vt = self.iface_tables[t].vt;
            if !self.m.vtables[vt].entries.is_empty() {
                continue;
            }
            let (iface, kind, ty) = {
                let t = &self.iface_tables[t];
                (t.iface, t.kind, t.ty.clone())
            };
            let (mut ptys, ret) = match (kind, &ty) {
                (TableKind::Call, Ty::Fn(sig)) => (sig.params.clone(), sig.ret.clone()),
                (TableKind::Get, ty) => (vec![], ty.clone()),
                (_, ty) => (vec![ty.clone()], Ty::Void),
            };
            ptys.insert(0, Ty::Obj(iface));
            let name = format!("{}.unused", self.m.structs[iface].name);
            let fid = self.entry_func(name, ptys, move |this, _| {
                Ok(if ret == Ty::Void {
                    (vec![], None)
                } else {
                    (vec![], Some(this.default_value(&ret)))
                })
            })?;
            self.m.vtables[vt].entries.push((iface, fid));
        }
        Ok(())
    }

    /// An interface struct is only a type: reading, writing or building one would be wrong.
    pub(crate) fn check_iface_structs(&self) -> R<()> {
        if self.iface_sids.is_empty() {
            return Ok(());
        }
        let bad = std::cell::RefCell::new(None::<(StructId, String)>);
        for f in &self.m.funcs {
            let check = |e: &Expr| {
                if let ExprKind::Field(_, s, _) | ExprKind::NewObj(s) = &e.kind
                    && self.iface_sids.contains_key(s)
                {
                    bad.borrow_mut().get_or_insert((*s, f.name.clone()));
                }
                false
            };
            let check_stmt = |s: &Stmt| {
                if let Stmt::StoreField(_, sid, _, _) = s
                    && self.iface_sids.contains_key(sid)
                {
                    bad.borrow_mut().get_or_insert((*sid, f.name.clone()));
                }
                false
            };
            for s in &f.body {
                crate::frames::scan_stmt(s, &check, &check_stmt);
            }
        }
        if let Some((s, func)) = bad.into_inner() {
            bail!(
                "{func}: interface {} is implemented by classes, so its values are accessed through \
                 its members: destructuring, spread, Object.keys, JSON and the like are not supported on it",
                self.m.structs[s].name
            )
        }
        Ok(())
    }
}
