//! Unions of different types (`number | [number, number]`, `string | Handler`): a struct with a
//! tag and one slot per member, made when a value goes where the union is expected and read back
//! (projected) where `typeof x === ".."` / `Array.isArray(x)` / a truthiness test narrowed it.
//!
//! The program already passes `tsc --strict`, so a use of a member is trusted to follow a test:
//! the lowering only has to know *which* member a path means there. `narrowed` holds the paths
//! (`o.scale`) the enclosing tests narrowed to a member; reading such a path projects the member.
//! `typeof u` and `Array.isArray(u)` on a union that is not narrowed read the tag.

use super::expr::{mk, num};
use super::*;

impl<'a> Lowerer<'a> {
    /// The members of a union struct type.
    pub(crate) fn union_members(&self, t: &Ty) -> Option<Vec<Ty>> {
        match t.strip() {
            Ty::Obj(sid) => self.unions.get(sid).cloned(),
            _ => None,
        }
    }

    /// The union of these (distinct, at least two) member types.
    pub(crate) fn make_union(&mut self, members: Vec<Ty>) -> Ty {
        let mut fields = vec![Field {
            name: "$tag".into(),
            ty: Ty::Num,
            optional: false,
        }];
        for (i, m) in members.iter().enumerate() {
            fields.push(Field {
                name: format!("${i}"),
                ty: m.clone(),
                optional: false,
            });
        }
        let sid = self.anon_struct("Union", fields);
        self.unions.insert(sid, members);
        Ty::Obj(sid)
    }

    /// The member a value of type `from` is, if any: the same type, a string literal for a
    /// `string` member, or a type assignable to the member.
    pub(crate) fn member_index(&self, members: &[Ty], from: &Ty) -> Option<usize> {
        let f = from.strip();
        members
            .iter()
            .position(|m| m.strip() == f)
            .or_else(|| {
                if *f == Ty::Lit {
                    members.iter().position(|m| *m.strip() == Ty::Str)
                } else {
                    None
                }
            })
            .or_else(|| members.iter().position(|m| self.assignable(f, m.strip())))
    }

    /// A value as member `idx` of a union (null stays null for a nullable member).
    pub(crate) fn wrap_in_union(&mut self, e: Expr, union: &Ty, idx: usize) -> R<Expr> {
        let Ty::Obj(sid) = union.strip().clone() else {
            bail!("not a union")
        };
        let members = self.unions[&sid].clone();
        let member = members[idx].clone();
        let nullable_in = e.ty.is_nullable() && !member.is_nullable();
        let v = self.new_local(e.ty.clone());
        let ve = self.var_expr(v);
        let value = if nullable_in {
            mk(ExprKind::Cast(Box::new(ve.clone())), member.clone())
        } else if e.ty.strip() == &Ty::Lit && member.strip() == &Ty::Str {
            mk(ExprKind::TagStr(Box::new(ve.clone())), Ty::Str)
        } else if e.ty != member {
            self.coerce(ve.clone(), &member)?
        } else {
            ve.clone()
        };
        let obj = self.new_local(Ty::Obj(sid));
        let oe = self.var_expr(obj);
        let made = vec![
            Stmt::Assign(obj, mk(ExprKind::NewObj(sid), Ty::Obj(sid))),
            Stmt::StoreField(oe.clone(), sid, 0, num(idx as f64)),
            Stmt::StoreField(oe.clone(), sid, 1 + idx, value),
        ];
        let result = mk(ExprKind::Seq(made, Box::new(oe)), Ty::Obj(sid));
        if nullable_in {
            // A null member makes a null union.
            let nty = Ty::Obj(sid).nullable();
            let null = mk(ExprKind::Null, nty.clone());
            let is_null = mk(ExprKind::IsNull(Box::new(ve)), Ty::Bool);
            let cond = mk(
                ExprKind::Cond(
                    Box::new(is_null),
                    Box::new(null),
                    Box::new(mk(ExprKind::Cast(Box::new(result)), nty.clone())),
                ),
                nty.clone(),
            );
            return Ok(mk(
                ExprKind::Seq(vec![Stmt::Assign(v, e)], Box::new(cond)),
                nty,
            ));
        }
        Ok(mk(
            ExprKind::Seq(vec![Stmt::Assign(v, e)], Box::new(result)),
            Ty::Obj(sid),
        ))
    }

    /// Member `idx` of a union value (nullable when the union is: null stays null).
    pub(crate) fn project_union(&mut self, e: Expr, idx: usize) -> Expr {
        let Ty::Obj(sid) = e.ty.strip().clone() else {
            return e;
        };
        let member = self.unions[&sid][idx].clone();
        let nullable = e.ty.is_nullable();
        let plain = Ty::Obj(sid);
        if !nullable {
            return mk(ExprKind::Field(Box::new(e), sid, 1 + idx), member);
        }
        let out = member.clone().nullable();
        let tmp = self.new_local(e.ty.clone());
        let t = self.var_expr(tmp);
        let is_null = mk(ExprKind::IsNull(Box::new(t.clone())), Ty::Bool);
        let read = mk(
            ExprKind::Field(
                Box::new(mk(ExprKind::Cast(Box::new(t)), plain)),
                sid,
                1 + idx,
            ),
            member,
        );
        let read = if read.ty.is_nullable() {
            read
        } else {
            mk(ExprKind::Cast(Box::new(read)), out.clone())
        };
        let none = mk(ExprKind::Null, out.clone());
        mk(
            ExprKind::Seq(
                vec![Stmt::Assign(tmp, e)],
                Box::new(mk(
                    ExprKind::Cond(Box::new(is_null), Box::new(none), Box::new(read)),
                    out.clone(),
                )),
            ),
            out,
        )
    }

    /// `typeof u` for a union value: the name of the member its tag says.
    pub(crate) fn union_typeof(&mut self, u: Expr) -> Expr {
        let members = self.union_members(&u.ty).unwrap_or_default();
        let Ty::Obj(sid) = u.ty.strip().clone() else {
            return u;
        };
        let tmp = self.new_local(u.ty.clone());
        let t = self.var_expr(tmp);
        let name = |this: &mut Self, m: &Ty| {
            let n = match m.strip() {
                Ty::Num => "number",
                Ty::Bool => "boolean",
                Ty::BigInt => "bigint",
                Ty::Str | Ty::Lit => "string",
                Ty::Fn(_) => "function",
                _ => "object",
            };
            let tag = this.tag(n);
            mk(ExprKind::Tag(tag), Ty::Lit)
        };
        let plain = mk(ExprKind::Cast(Box::new(t.clone())), Ty::Obj(sid));
        let tag_of = mk(ExprKind::Field(Box::new(plain), sid, 0), Ty::Num);
        let mut chain = {
            let last = members.last().cloned().unwrap_or(Ty::Num);
            name(self, &last)
        };
        for i in (0..members.len().saturating_sub(1)).rev() {
            let test = mk(
                ExprKind::Cmp(CmpOp::Eq, Box::new(tag_of.clone()), Box::new(num(i as f64))),
                Ty::Bool,
            );
            let this_name = name(self, &members[i]);
            chain = mk(
                ExprKind::Cond(Box::new(test), Box::new(this_name), Box::new(chain)),
                Ty::Lit,
            );
        }
        if u.ty.is_nullable() {
            let is_null = mk(ExprKind::IsNull(Box::new(t)), Ty::Bool);
            let undef = self.tag("undefined");
            chain = mk(
                ExprKind::Cond(
                    Box::new(is_null),
                    Box::new(mk(ExprKind::Tag(undef), Ty::Lit)),
                    Box::new(chain),
                ),
                Ty::Lit,
            );
        }
        mk(
            ExprKind::Seq(vec![Stmt::Assign(tmp, u)], Box::new(chain)),
            Ty::Lit,
        )
    }

    /// The truthiness of a union value: null is false, otherwise the member it holds is tested
    /// (a number is false at 0 and NaN, a string when empty, an object or array never).
    pub(crate) fn union_truthy(&mut self, u: Expr) -> Expr {
        let members = self.union_members(&u.ty).unwrap_or_default();
        let Ty::Obj(sid) = u.ty.strip().clone() else {
            return u;
        };
        let nullable = u.ty.is_nullable();
        let tmp = self.new_local(u.ty.clone());
        let t = self.var_expr(tmp);
        let plain = mk(ExprKind::Cast(Box::new(t.clone())), Ty::Obj(sid));
        let tag_of = mk(ExprKind::Field(Box::new(plain.clone()), sid, 0), Ty::Num);
        let member_truthy = |this: &mut Self, i: usize| {
            let m = members[i].clone();
            let read = mk(ExprKind::Field(Box::new(plain.clone()), sid, 1 + i), m);
            this.bool_of(read)
        };
        let mut chain = member_truthy(self, members.len() - 1);
        for i in (0..members.len() - 1).rev() {
            let test = mk(
                ExprKind::Cmp(CmpOp::Eq, Box::new(tag_of.clone()), Box::new(num(i as f64))),
                Ty::Bool,
            );
            let this_member = member_truthy(self, i);
            chain = mk(
                ExprKind::Cond(Box::new(test), Box::new(this_member), Box::new(chain)),
                Ty::Bool,
            );
        }
        if nullable {
            let not_null = mk(
                ExprKind::Un(
                    UnOp::Not,
                    Box::new(mk(ExprKind::IsNull(Box::new(t)), Ty::Bool)),
                ),
                Ty::Bool,
            );
            chain = mk(ExprKind::And(Box::new(not_null), Box::new(chain)), Ty::Bool);
        }
        mk(
            ExprKind::Seq(vec![Stmt::Assign(tmp, u)], Box::new(chain)),
            Ty::Bool,
        )
    }

    /// Is the union value one of the members `keep` accepts? (False for null.)
    pub(crate) fn union_is_member(&mut self, u: Expr, keep: impl Fn(&Ty) -> bool) -> Expr {
        let members = self.union_members(&u.ty).unwrap_or_default();
        let Ty::Obj(sid) = u.ty.strip().clone() else {
            return u;
        };
        let tmp = self.new_local(u.ty.clone());
        let t = self.var_expr(tmp);
        let plain = mk(ExprKind::Cast(Box::new(t.clone())), Ty::Obj(sid));
        let tag_of = mk(ExprKind::Field(Box::new(plain), sid, 0), Ty::Num);
        let mut any: Option<Expr> = None;
        for (i, m) in members.iter().enumerate() {
            if keep(m) {
                let is = mk(
                    ExprKind::Cmp(CmpOp::Eq, Box::new(tag_of.clone()), Box::new(num(i as f64))),
                    Ty::Bool,
                );
                any = Some(match any {
                    None => is,
                    Some(a) => mk(ExprKind::Or(Box::new(a), Box::new(is)), Ty::Bool),
                });
            }
        }
        let mut result = any.unwrap_or_else(|| mk(ExprKind::Bool(false), Ty::Bool));
        if u.ty.is_nullable() {
            let not_null = mk(
                ExprKind::Un(
                    UnOp::Not,
                    Box::new(mk(ExprKind::IsNull(Box::new(t)), Ty::Bool)),
                ),
                Ty::Bool,
            );
            result = mk(
                ExprKind::And(Box::new(not_null), Box::new(result)),
                Ty::Bool,
            );
        }
        mk(
            ExprKind::Seq(vec![Stmt::Assign(tmp, u)], Box::new(result)),
            Ty::Bool,
        )
    }

    /// `Array.isArray(u)` for a union value.
    pub(crate) fn union_is_array(&mut self, u: Expr) -> Expr {
        let members = self.union_members(&u.ty).unwrap_or_default();
        let Ty::Obj(sid) = u.ty.strip().clone() else {
            return u;
        };
        let tmp = self.new_local(u.ty.clone());
        let t = self.var_expr(tmp);
        let plain = mk(ExprKind::Cast(Box::new(t.clone())), Ty::Obj(sid));
        let tag_of = mk(ExprKind::Field(Box::new(plain), sid, 0), Ty::Num);
        let mut any: Option<Expr> = None;
        for (i, m) in members.iter().enumerate() {
            if matches!(m.strip(), Ty::Arr(_) | Ty::Typed(_)) {
                let is = mk(
                    ExprKind::Cmp(CmpOp::Eq, Box::new(tag_of.clone()), Box::new(num(i as f64))),
                    Ty::Bool,
                );
                any = Some(match any {
                    None => is,
                    Some(a) => mk(ExprKind::Or(Box::new(a), Box::new(is)), Ty::Bool),
                });
            }
        }
        let mut result = any.unwrap_or_else(|| mk(ExprKind::Bool(false), Ty::Bool));
        if u.ty.is_nullable() {
            let not_null = mk(
                ExprKind::Un(
                    UnOp::Not,
                    Box::new(mk(ExprKind::IsNull(Box::new(t)), Ty::Bool)),
                ),
                Ty::Bool,
            );
            result = mk(
                ExprKind::And(Box::new(not_null), Box::new(result)),
                Ty::Bool,
            );
        }
        mk(
            ExprKind::Seq(vec![Stmt::Assign(tmp, u)], Box::new(result)),
            Ty::Bool,
        )
    }

    // ---- narrowing ----

    /// The text of a path (`x`, `this.o.scale`) a test can narrow.
    fn path_text(&self, e: &Expression<'_>) -> Option<String> {
        match e {
            Expression::Identifier(id) => Some(id.name.to_string()),
            Expression::ThisExpression(_) => Some("this".into()),
            Expression::ParenthesizedExpression(p) => self.path_text(&p.expression),
            Expression::TSNonNullExpression(n) => self.path_text(&n.expression),
            Expression::StaticMemberExpression(m) => Some(format!(
                "{}.{}",
                self.path_text(&m.object)?,
                m.property.name
            )),
            _ => None,
        }
    }

    /// Reading a path a test narrowed to a member of its union gives that member.
    pub(crate) fn narrow_value(&mut self, e: &'a Expression<'a>, v: Expr) -> Expr {
        if self.cur.narrowed.is_empty()
            || !self.unions.contains_key(match v.ty.strip() {
                Ty::Obj(s) => s,
                _ => return v,
            })
        {
            return v;
        }
        let Some(path) = self.path_text(e) else {
            return v;
        };
        // Every test around narrowed the path further: the members all of them allow.
        let mut allowed: Option<Vec<usize>> = None;
        for (p, idxs) in &self.cur.narrowed {
            if *p == path {
                allowed = Some(match allowed {
                    None => idxs.clone(),
                    Some(a) => a.into_iter().filter(|i| idxs.contains(i)).collect(),
                });
            }
        }
        match allowed {
            Some(a) if a.len() == 1 => self.project_union(v, a[0]),
            _ => v,
        }
    }

    /// Does a member of this type pass `typeof x === kind`?
    fn kind_matches(t: &Ty, kind: &str) -> bool {
        matches!(
            (kind, t.strip()),
            ("number", Ty::Num)
                | ("boolean", Ty::Bool)
                | ("bigint", Ty::BigInt)
                | ("string", Ty::Str | Ty::Lit)
                | ("function", Ty::Fn(_))
                | (
                    "object",
                    Ty::Arr(_) | Ty::Typed(_) | Ty::Obj(_) | Ty::Map(..) | Ty::Set(_)
                )
        )
    }

    /// The paths a test narrows to some members when it comes out true (`positive`) or false.
    pub(crate) fn narrowings(
        &mut self,
        test: &'a Expression<'a>,
        positive: bool,
    ) -> Vec<(String, Vec<usize>)> {
        match test {
            Expression::ParenthesizedExpression(p) => self.narrowings(&p.expression, positive),
            Expression::UnaryExpression(u) if u.operator == UnaryOperator::LogicalNot => {
                self.narrowings(&u.argument, !positive)
            }
            Expression::LogicalExpression(l) => {
                let both = match l.operator {
                    LogicalOperator::And => positive,
                    LogicalOperator::Or => !positive,
                    LogicalOperator::Coalesce => return vec![],
                };
                if !both {
                    return vec![];
                }
                let mut out = self.narrowings(&l.left, positive);
                out.extend(self.narrowings(&l.right, positive));
                out
            }
            // `typeof P === "number"`
            Expression::BinaryExpression(b)
                if matches!(
                    b.operator,
                    BinaryOperator::StrictEquality
                        | BinaryOperator::StrictInequality
                        | BinaryOperator::Equality
                        | BinaryOperator::Inequality
                ) =>
            {
                let eq = matches!(
                    b.operator,
                    BinaryOperator::StrictEquality | BinaryOperator::Equality
                );
                let (typeof_side, lit_side) = match (&b.left, &b.right) {
                    (Expression::UnaryExpression(u), Expression::StringLiteral(s))
                        if u.operator == UnaryOperator::Typeof =>
                    {
                        (&u.argument, s.value.to_string())
                    }
                    (Expression::StringLiteral(s), Expression::UnaryExpression(u))
                        if u.operator == UnaryOperator::Typeof =>
                    {
                        (&u.argument, s.value.to_string())
                    }
                    _ => return vec![],
                };
                self.narrow_by(typeof_side, positive == eq, |t| {
                    Self::kind_matches(t, &lit_side)
                })
            }
            // `"name" in P`: the members that have the property (true), the others (false).
            Expression::BinaryExpression(b) if b.operator == BinaryOperator::In => {
                let Expression::StringLiteral(k) = &b.left else {
                    return vec![];
                };
                let name = k.value.to_string();
                // A member narrows on the property being there; an optional one may be missing.
                let mut has: Vec<StructId> = Vec::new();
                let mut must: Vec<StructId> = Vec::new();
                for (sid, st) in self.m.structs.iter().enumerate() {
                    if let Some(i) = st.field(&name) {
                        has.push(sid);
                        if !st.fields[i].optional {
                            must.push(sid);
                        }
                    }
                }
                let owners = if positive { has } else { must };
                self.narrow_by(&b.right, true, move |t| {
                    let is_owner = matches!(t.strip(), Ty::Obj(s) if owners.contains(s));
                    is_owner == positive
                })
            }
            // `P instanceof C`: the members that are `C` or a subclass (true), the others (false).
            Expression::BinaryExpression(b) if b.operator == BinaryOperator::Instanceof => {
                let Expression::Identifier(id) = &b.right else {
                    return vec![];
                };
                if !self.classes.contains_key(id.name.as_str()) {
                    return vec![];
                }
                let sid = self.classes_sid(id.name.as_str());
                let family: Vec<StructId> = self
                    .class_of_sid
                    .keys()
                    .copied()
                    .filter(|s| self.is_subclass(*s, sid))
                    .collect();
                self.narrow_by(
                    &b.left,
                    positive,
                    move |t| matches!(t.strip(), Ty::Obj(s) if family.contains(s)),
                )
            }
            // `P` as a test: when it is false the value is one of the members that can be falsy
            // (an object, an array or a function never is).
            Expression::Identifier(_)
            | Expression::StaticMemberExpression(_)
            | Expression::TSNonNullExpression(_) => {
                if positive {
                    return vec![];
                }
                self.narrow_by(test, true, |t| {
                    matches!(
                        t.strip(),
                        Ty::Num | Ty::Str | Ty::Lit | Ty::Bool | Ty::BigInt
                    )
                })
            }
            // `Array.isArray(P)`
            Expression::CallExpression(c)
                if matches!(&c.callee, Expression::StaticMemberExpression(m)
                    if m.property.name == "isArray"
                        && matches!(&m.object, Expression::Identifier(i) if i.name == "Array"))
                    && c.arguments.len() == 1 =>
            {
                let Some(arg) = c.arguments[0].as_expression() else {
                    return vec![];
                };
                self.narrow_by(arg, positive, |t| {
                    matches!(t.strip(), Ty::Arr(_) | Ty::Typed(_))
                })
            }
            _ => vec![],
        }
    }

    /// The path `p`, narrowed to the members of its union that (do not) satisfy `keep`.
    fn narrow_by(
        &mut self,
        p: &'a Expression<'a>,
        want: bool,
        keep: impl Fn(&Ty) -> bool,
    ) -> Vec<(String, Vec<usize>)> {
        let Some(path) = self.path_text(p) else {
            return vec![];
        };
        let mark = self.cur.locals.len();
        let saved = std::mem::take(&mut self.cur.narrowed);
        let v = self.lower_expr(p, None);
        self.cur.narrowed = saved;
        self.cur.locals.truncate(mark);
        let Ok(v) = v else { return vec![] };
        let Some(members) = self.union_members(&v.ty) else {
            return vec![];
        };
        let idxs: Vec<usize> = members
            .iter()
            .enumerate()
            .filter(|(_, m)| keep(m) == want)
            .map(|(i, _)| i)
            .collect();
        if idxs.is_empty() || idxs.len() == members.len() {
            vec![]
        } else {
            vec![(path, idxs)]
        }
    }

    /// Lowers `f` with these narrowings in force.
    pub(crate) fn with_narrowed<T>(
        &mut self,
        n: Vec<(String, Vec<usize>)>,
        f: impl FnOnce(&mut Self) -> R<T>,
    ) -> R<T> {
        let mark = self.cur.narrowed.len();
        self.cur.narrowed.extend(n);
        let r = f(self);
        self.cur.narrowed.truncate(mark);
        r
    }

    /// The expected type to push into a literal that goes where a union is expected: the one
    /// member an array (or object) literal can be.
    pub(crate) fn union_hint(&self, e: &Expression<'_>, exp: Option<&Ty>) -> Option<Ty> {
        let members = self.union_members(exp?)?;
        let pick = |f: &dyn Fn(&Ty) -> bool| {
            let mut it = members.iter().filter(|m| f(m));
            match (it.next(), it.next()) {
                (Some(m), None) => Some(m.clone()),
                _ => None,
            }
        };
        match e {
            Expression::ArrayExpression(_) => pick(&|m| matches!(m.strip(), Ty::Arr(_))),
            Expression::ObjectExpression(o) => {
                let objects = members
                    .iter()
                    .filter(|m| matches!(m.strip(), Ty::Obj(_)))
                    .count();
                if objects <= 1 {
                    return pick(&|m| matches!(m.strip(), Ty::Obj(_)));
                }
                // Several object types: the one whose properties are the literal's (every key
                // is a field, every field that is not optional is given).
                let mut keys: Vec<String> = Vec::new();
                for p in &o.properties {
                    let ObjectPropertyKind::ObjectProperty(p) = p else {
                        return None;
                    };
                    match &p.key {
                        PropertyKey::StaticIdentifier(k) if !p.computed => {
                            keys.push(k.name.to_string())
                        }
                        PropertyKey::StringLiteral(k) => keys.push(k.value.to_string()),
                        _ => return None,
                    }
                }
                pick(&|m| {
                    let Ty::Obj(sid) = m.strip() else {
                        return false;
                    };
                    let st = &self.m.structs[*sid];
                    keys.iter().all(|k| st.field(k).is_some())
                        && st.fields.iter().all(|f| {
                            f.optional || f.name.starts_with("_p_") || keys.contains(&f.name)
                        })
                })
            }
            _ => None,
        }
    }
}
