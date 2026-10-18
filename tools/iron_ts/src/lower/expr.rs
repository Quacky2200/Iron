//! Expression lowering.

use super::*;

pub(crate) fn mk(kind: ExprKind, ty: Ty) -> Expr {
    Expr { kind, ty }
}

pub(crate) fn num(n: f64) -> Expr {
    mk(ExprKind::Num(n), Ty::Num)
}

/// JavaScript's ToInt32, with the same saturating conversion the generated code uses.
fn to_i32(x: f64) -> i32 {
    if x.is_finite() { (x as i64) as i32 } else { 0 }
}

/// A bit operation on two constants.
fn fold_bits(op: BinOp, a: f64, b: f64) -> f64 {
    let (x, y) = (to_i32(a), to_i32(b));
    let n = (y & 31) as u32;
    match op {
        BinOp::BitAnd => (x & y) as f64,
        BinOp::BitOr => (x | y) as f64,
        BinOp::BitXor => (x ^ y) as f64,
        BinOp::Shl => x.wrapping_shl(n) as f64,
        BinOp::Shr => (x >> n) as f64,
        _ => ((x as u32) >> n) as f64,
    }
}

/// One piece of an array being built: a value, or an array whose elements are spread in.
pub(crate) enum Elem {
    One(Expr),
    Spread(Expr),
}

/// A place that can be read and written (assignment targets, `++`, compound assignment).
pub(crate) enum Place {
    Var(VarId, Ty),
    /// A captured variable: the cell and the value's type.
    Cell(Expr, Ty),
    /// A module-level `let`/`var`.
    Global(GlobalId, Ty),
    Field(Expr, StructId, usize, Ty),
    Index(Expr, Expr, Ty),
}

impl<'a> Lowerer<'a> {
    pub(crate) fn loc(&self, offset: u32) -> String {
        format!("line {}", self.line(offset))
    }

    /// A value used as a number (`number | null` narrowed away).
    pub(crate) fn as_num(&self, e: Expr) -> R<Expr> {
        match e.ty.strip() {
            Ty::Num => self.coerce(e, &Ty::Num),
            t => bail!("expected a number, found {t}"),
        }
    }

    pub(crate) fn lower_expr(&mut self, e: &'a Expression<'a>, exp: Option<&Ty>) -> R<Expr> {
        match e {
            Expression::NumericLiteral(n) => Ok(num(n.value)),
            Expression::BooleanLiteral(b) => Ok(mk(ExprKind::Bool(b.value), Ty::Bool)),
            Expression::NullLiteral(_) => Ok(mk(ExprKind::Null, Ty::Null)),
            Expression::StringLiteral(s) => {
                let t = self.tag(s.value.as_str());
                Ok(mk(ExprKind::Tag(t), Ty::Lit))
            }
            Expression::Identifier(id) => self.lower_ident(id.name.as_str()),
            Expression::ParenthesizedExpression(p) => self.lower_expr(&p.expression, exp),
            Expression::TSAsExpression(a) => self.lower_expr(&a.expression, exp),
            Expression::TSSatisfiesExpression(s) => {
                // `{...} as const satisfies Record<string, V>`: every property is a V.
                let mut inner = &s.expression;
                while let Expression::TSAsExpression(a) = inner {
                    inner = &a.expression;
                }
                if let (TSType::TSTypeReference(r), Expression::ObjectExpression(o)) =
                    (&s.type_annotation, inner)
                    && let (TSTypeName::IdentifierReference(n), Some(args)) =
                        (&r.type_name, &r.type_arguments)
                    && n.name == "Record"
                    && args.params.len() == 2
                {
                    let v = self.ty(&args.params[1], "Record")?;
                    return self.lower_object(o, None, Some(&v));
                }
                self.lower_expr(&s.expression, exp)
            }
            Expression::TSNonNullExpression(n) => {
                let e = self.lower_expr(&n.expression, exp)?;
                let t = e.ty.strip().clone();
                Ok(mk(ExprKind::Cast(Box::new(e)), t))
            }
            Expression::ArrowFunctionExpression(a) => {
                self.lower_closure(super::closure::ClosureNode::Arrow(a), exp)
            }
            Expression::FunctionExpression(f) => {
                self.lower_closure(super::closure::ClosureNode::Func(f), exp)
            }
            Expression::StaticMemberExpression(m) => self.lower_static_member(m),
            Expression::ComputedMemberExpression(m) => self.lower_computed_member(m),
            Expression::ChainExpression(c) => self.lower_chain(c),
            Expression::CallExpression(c) => self.lower_call(c),
            Expression::NewExpression(n) => self.lower_new(n, exp),
            Expression::ObjectExpression(o) => self.lower_object(o, exp, None),
            Expression::ArrayExpression(a) => self.lower_array_lit(a, exp),
            Expression::UnaryExpression(u) => self.lower_unary(u),
            Expression::BinaryExpression(b) => self.lower_binary(b),
            Expression::LogicalExpression(l) => self.lower_logical(l, exp),
            Expression::ConditionalExpression(c) => {
                let test = self.lower_expr(&c.test, None)?;
                let test = self.to_bool(test);
                let a = self.lower_expr(&c.consequent, exp)?;
                let b = self.lower_expr(&c.alternate, exp)?;
                let ty = self.unify(&a.ty, &b.ty)?;
                let (a, b) = (self.coerce(a, &ty)?, self.coerce(b, &ty)?);
                Ok(mk(
                    ExprKind::Cond(Box::new(test), Box::new(a), Box::new(b)),
                    ty,
                ))
            }
            Expression::AssignmentExpression(a) => {
                let (mut stmts, value) = self.lower_assign(a)?;
                let ty = value.ty.clone();
                Ok(mk(
                    ExprKind::Seq(std::mem::take(&mut stmts), Box::new(value)),
                    ty,
                ))
            }
            Expression::UpdateExpression(u) => {
                let (stmts, value) = self.lower_update(u)?;
                let ty = value.ty.clone();
                Ok(mk(ExprKind::Seq(stmts, Box::new(value)), ty))
            }
            Expression::SequenceExpression(s) => {
                let mut stmts = Vec::new();
                let n = s.expressions.len();
                for e in &s.expressions[..n - 1] {
                    let x = self.lower_expr(e, None)?;
                    stmts.push(Stmt::Eval(x));
                }
                let last = self.lower_expr(&s.expressions[n - 1], exp)?;
                let ty = last.ty.clone();
                Ok(mk(ExprKind::Seq(stmts, Box::new(last)), ty))
            }
            _ => bail!(
                "{}: unsupported expression",
                self.loc(oxc_span::GetSpan::span(e).start)
            ),
        }
    }

    fn lower_ident(&mut self, name: &str) -> R<Expr> {
        if let Some(b) = self.lookup(name) {
            return Ok(self.read_binding(&b));
        }
        match self.consts.get(name).cloned() {
            Some(ConstInfo::Inline(e)) => return Ok(e),
            Some(ConstInfo::Global(g, ty, false)) => return Ok(mk(ExprKind::Global(g), ty)),
            Some(ConstInfo::Global(g, ty, true)) => return Ok(mk(ExprKind::GlobalVar(g), ty)),
            None => {}
        }
        match name {
            "Infinity" => Ok(num(f64::INFINITY)),
            "NaN" => Ok(num(f64::NAN)),
            "undefined" => Ok(mk(ExprKind::Null, Ty::Null)),
            _ if self.fids.contains_key(name) => self.function_value(name),
            _ => bail!("unknown name `{name}`"),
        }
    }

    fn lower_static_member(&mut self, m: &'a StaticMemberExpression<'a>) -> R<Expr> {
        let prop = m.property.name.as_str();
        if let Expression::Identifier(id) = &m.object
            && id.name == "Math"
            && self.lookup("Math").is_none()
        {
            return match prop {
                "PI" => Ok(num(std::f64::consts::PI)),
                "SQRT2" => Ok(num(std::f64::consts::SQRT_2)),
                "E" => Ok(num(std::f64::consts::E)),
                _ => bail!("Math.{prop} is not a value"),
            };
        }
        if let Expression::Identifier(id) = &m.object
            && self.lookup(id.name.as_str()).is_none()
            && let Some(v) = self.enum_member(id.name.as_str(), prop)
        {
            return v;
        }
        let mut obj = self.lower_expr(&m.object, None)?;
        if m.optional {
            obj = self.chain_guard(obj)?;
        }
        self.member_of(obj, prop)
    }

    /// `obj.prop` on an already-lowered object.
    pub(crate) fn member_of(&mut self, obj: Expr, prop: &str) -> R<Expr> {
        match obj.ty.strip().clone() {
            Ty::Obj(sid) => {
                let Some(i) = self.m.structs[sid].field(prop) else {
                    bail!("{} has no field `{prop}`", self.m.structs[sid].name)
                };
                let ty = self.m.structs[sid].fields[i].ty.clone();
                Ok(mk(ExprKind::Field(Box::new(obj), sid, i), ty))
            }
            Ty::Arr(_) | Ty::Bytes if prop == "length" => {
                Ok(mk(ExprKind::Len(Box::new(obj)), Ty::Num))
            }
            t => bail!("no property `{prop}` on {t}"),
        }
    }

    fn lower_computed_member(&mut self, m: &'a ComputedMemberExpression<'a>) -> R<Expr> {
        let mut arr = self.lower_expr(&m.object, None)?;
        if m.optional {
            arr = self.chain_guard(arr)?;
        }
        let idx = self.lower_expr(&m.expression, Some(&Ty::Num))?;
        let idx = self.as_num(idx)?;
        let ty = match arr.ty.strip() {
            Ty::Bytes => Ty::Num,
            Ty::Arr(el) => match &**el {
                // Out of range reads null (a zero pointer, or the null NaN for numbers).
                e if e.is_ptr() || *e == Ty::Num => e.clone().nullable(),
                e => e.clone(),
            },
            t => bail!("cannot index {t}"),
        };
        Ok(mk(ExprKind::Index(Box::new(arr), Box::new(idx)), ty))
    }

    /// For `a?.b`: evaluates `a` once; if it is null the whole chain yields null.
    fn chain_guard(&mut self, obj: Expr) -> R<Expr> {
        let Some(&(label, res)) = self.cur.chain.last() else {
            bail!("optional access outside a chain")
        };
        let tmp = self.new_local(obj.ty.clone());
        let t = self.var_expr(tmp);
        let null = mk(ExprKind::Null, Ty::Null);
        let check = Stmt::If(
            mk(ExprKind::IsNull(Box::new(t.clone())), Ty::Bool),
            vec![Stmt::Assign(res, null), Stmt::Break(label)],
            vec![],
        );
        let ty = t.ty.clone();
        Ok(mk(
            ExprKind::Seq(vec![Stmt::Assign(tmp, obj), check], Box::new(t)),
            ty,
        ))
    }

    fn lower_chain(&mut self, c: &'a ChainExpression<'a>) -> R<Expr> {
        let res = self.new_local(Ty::Void);
        let label = self.new_label();
        self.cur.chain.push((label, res));
        let inner = match &c.expression {
            ChainElement::StaticMemberExpression(m) => self.lower_static_member(m),
            ChainElement::ComputedMemberExpression(m) => self.lower_computed_member(m),
            ChainElement::CallExpression(c) => self.lower_call(c),
            ChainElement::TSNonNullExpression(n) => {
                let e = self.lower_expr(&n.expression, None)?;
                let t = e.ty.strip().clone();
                Ok(mk(ExprKind::Cast(Box::new(e)), t))
            }
            _ => bail!("unsupported optional chain"),
        };
        self.cur.chain.pop();
        let inner = inner?;
        let ty = inner.ty.clone().nullable();
        self.cur.locals[res as usize] = ty.clone();
        let v = self.coerce(inner, &ty)?;
        let block = Stmt::Block(label, vec![Stmt::Assign(res, v)]);
        Ok(mk(
            ExprKind::Seq(vec![block], Box::new(self.var_expr(res))),
            ty,
        ))
    }

    fn lower_unary(&mut self, u: &'a UnaryExpression<'a>) -> R<Expr> {
        let a = self.lower_expr(&u.argument, None)?;
        match u.operator {
            UnaryOperator::LogicalNot => {
                let b = self.to_bool(a);
                Ok(mk(ExprKind::Un(UnOp::Not, Box::new(b)), Ty::Bool))
            }
            UnaryOperator::UnaryNegation => {
                let a = self.as_num(a)?;
                Ok(match a.kind {
                    ExprKind::Num(n) => num(-n),
                    _ => mk(ExprKind::Un(UnOp::Neg, Box::new(a)), Ty::Num),
                })
            }
            UnaryOperator::UnaryPlus => self.as_num(a),
            _ => bail!("{}: unsupported unary operator", self.loc(u.span.start)),
        }
    }

    fn lower_binary(&mut self, b: &'a BinaryExpression<'a>) -> R<Expr> {
        use BinaryOperator as B;
        let l = self.lower_expr(&b.left, None)?;
        // A bare string literal compares against the other side's literal type.
        let r = self.lower_expr(&b.right, Some(l.ty.strip()))?;
        let cmp = |op| Some(op);
        let cop = match b.operator {
            B::StrictEquality | B::Equality => cmp(CmpOp::Eq),
            B::StrictInequality | B::Inequality => cmp(CmpOp::Ne),
            B::LessThan => cmp(CmpOp::Lt),
            B::LessEqualThan => cmp(CmpOp::Le),
            B::GreaterThan => cmp(CmpOp::Gt),
            B::GreaterEqualThan => cmp(CmpOp::Ge),
            _ => None,
        };
        if let Some(op) = cop {
            return self.lower_compare(op, l, r, b.span.start);
        }
        let op = match b.operator {
            B::Addition => BinOp::Add,
            B::Subtraction => BinOp::Sub,
            B::Multiplication => BinOp::Mul,
            B::Division => BinOp::Div,
            B::Remainder => BinOp::Rem,
            B::BitwiseAnd => BinOp::BitAnd,
            B::BitwiseOR => BinOp::BitOr,
            B::BitwiseXOR => BinOp::BitXor,
            B::ShiftLeft => BinOp::Shl,
            B::ShiftRight => BinOp::Shr,
            B::ShiftRightZeroFill => BinOp::UShr,
            _ => bail!("{}: unsupported operator", self.loc(b.span.start)),
        };
        self.arith(op, l, r)
    }

    pub(crate) fn arith(&mut self, op: BinOp, l: Expr, r: Expr) -> R<Expr> {
        let (l, r) = (self.as_num(l)?, self.as_num(r)?);
        // Constant folding, so `const UNITS = 10 / 729` is a constant rather than a global.
        if let (ExprKind::Num(a), ExprKind::Num(b)) = (&l.kind, &r.kind) {
            let v = match op {
                BinOp::Add => Some(a + b),
                BinOp::Sub => Some(a - b),
                BinOp::Mul => Some(a * b),
                BinOp::Div => Some(a / b),
                BinOp::Rem => Some(a % b),
                BinOp::BitAnd
                | BinOp::BitOr
                | BinOp::BitXor
                | BinOp::Shl
                | BinOp::Shr
                | BinOp::UShr => Some(fold_bits(op, *a, *b)),
                _ => None,
            };
            if let Some(v) = v {
                return Ok(num(v));
            }
        }
        Ok(mk(ExprKind::Bin(op, Box::new(l), Box::new(r)), Ty::Num))
    }

    pub(crate) fn lower_compare(&mut self, op: CmpOp, l: Expr, r: Expr, at: u32) -> R<Expr> {
        let eq = matches!(op, CmpOp::Eq | CmpOp::Ne);
        if eq && (l.ty == Ty::Null || r.ty == Ty::Null) {
            let other = if l.ty == Ty::Null { r } else { l };
            let is_null = if other.ty.is_nullable() {
                mk(ExprKind::IsNull(Box::new(other)), Ty::Bool)
            } else {
                mk(ExprKind::Bool(false), Ty::Bool)
            };
            return Ok(if op == CmpOp::Eq {
                is_null
            } else {
                mk(ExprKind::Un(UnOp::Not, Box::new(is_null)), Ty::Bool)
            });
        }
        let (a, b) = (l.ty.strip().clone(), r.ty.strip().clone());
        if a != b {
            bail!("{}: cannot compare {a} with {b}", self.loc(at));
        }
        if !eq && a != Ty::Num {
            bail!("{}: ordering comparison on {a}", self.loc(at));
        }
        Ok(mk(ExprKind::Cmp(op, Box::new(l), Box::new(r)), Ty::Bool))
    }

    fn lower_logical(&mut self, l: &'a LogicalExpression<'a>, exp: Option<&Ty>) -> R<Expr> {
        match l.operator {
            LogicalOperator::Coalesce => {
                let a = self.lower_expr(&l.left, None)?;
                let want = a.ty.strip().clone();
                let b = self.lower_expr(&l.right, Some(&want))?;
                let ty = if b.ty.is_nullable() {
                    want.clone().nullable()
                } else {
                    want.clone()
                };
                let tmp = self.new_local(a.ty.clone());
                let t = self.var_expr(tmp);
                let kept = self.coerce(t.clone(), &ty)?;
                let alt = self.coerce(b, &ty)?;
                let cond = mk(
                    ExprKind::Cond(
                        Box::new(mk(ExprKind::IsNull(Box::new(t)), Ty::Bool)),
                        Box::new(alt),
                        Box::new(kept),
                    ),
                    ty.clone(),
                );
                Ok(mk(
                    ExprKind::Seq(vec![Stmt::Assign(tmp, a)], Box::new(cond)),
                    ty,
                ))
            }
            op => {
                let _ = exp;
                let a = self.lower_expr(&l.left, None)?;
                let b = self.lower_expr(&l.right, None)?;
                let (a, b) = (self.to_bool(a), self.to_bool(b));
                let k = if op == LogicalOperator::And {
                    ExprKind::And(Box::new(a), Box::new(b))
                } else {
                    ExprKind::Or(Box::new(a), Box::new(b))
                };
                Ok(mk(k, Ty::Bool))
            }
        }
    }

    // ---- objects and arrays ----

    pub(crate) fn lower_object(
        &mut self,
        o: &'a ObjectExpression<'a>,
        exp: Option<&Ty>,
        record: Option<&Ty>,
    ) -> R<Expr> {
        let exp_sid = match exp.map(|t| t.strip()) {
            Some(Ty::Obj(s)) => Some(*s),
            _ => None,
        };
        // Lower every value first (with the field type as context when the struct is known).
        let mut spreads: Vec<Expr> = Vec::new();
        let mut props: Vec<(String, Expr)> = Vec::new();
        for p in &o.properties {
            match p {
                ObjectPropertyKind::SpreadProperty(s) => {
                    let e = self.lower_expr(&s.argument, exp)?;
                    spreads.push(e);
                }
                ObjectPropertyKind::ObjectProperty(p) => {
                    if p.computed || p.method {
                        bail!("{}: computed or method property", self.loc(p.span.start));
                    }
                    let name = self.key_name(&p.key)?;
                    let want = match exp_sid {
                        Some(s) => self.m.structs[s]
                            .field(&name)
                            .map(|i| self.m.structs[s].fields[i].ty.clone()),
                        None => record.cloned(),
                    };
                    let v = self.lower_expr(&p.value, want.as_ref())?;
                    props.push((name, v));
                }
            }
        }
        let sid = match exp_sid {
            Some(s) => s,
            None => {
                if let Some(first) = spreads.first() {
                    let Ty::Obj(s) = first.ty.strip() else {
                        bail!("spread of a non-object")
                    };
                    *s
                } else {
                    let fields = props
                        .iter()
                        .map(|(n, v)| Field {
                            name: n.clone(),
                            ty: record.cloned().unwrap_or_else(|| v.ty.clone()),
                        })
                        .collect();
                    self.anon_struct("Anon", fields)
                }
            }
        };
        let tmp = self.new_local(Ty::Obj(sid));
        let t = self.var_expr(tmp);
        let mut stmts = vec![Stmt::Assign(tmp, mk(ExprKind::NewObj(sid), Ty::Obj(sid)))];
        // JS order: spreads and properties in source order. Spreads first is equal here because the
        // source puts them before the overrides.
        for s in spreads {
            let src = self.new_local(s.ty.clone());
            stmts.push(Stmt::Assign(src, s));
            let n = self.m.structs[sid].fields.len();
            for i in 0..n {
                let fty = self.m.structs[sid].fields[i].ty.clone();
                let read = mk(ExprKind::Field(Box::new(self.var_expr(src)), sid, i), fty);
                stmts.push(Stmt::StoreField(t.clone(), sid, i, read));
            }
        }
        for (name, v) in props {
            let Some(i) = self.m.structs[sid].field(&name) else {
                bail!("{} has no field `{name}`", self.m.structs[sid].name)
            };
            let fty = self.m.structs[sid].fields[i].ty.clone();
            let v = self.coerce(v, &fty)?;
            stmts.push(Stmt::StoreField(t.clone(), sid, i, v));
        }
        // Fields that are nullable and not given start as null, not as a zero number.
        let given: Vec<String> = Vec::new();
        let _ = given;
        for i in 0..self.m.structs[sid].fields.len() {
            let fty = self.m.structs[sid].fields[i].ty.clone();
            if fty.is_nullable()
                && !stmts
                    .iter()
                    .any(|s| matches!(s, Stmt::StoreField(_, _, j, _) if *j == i))
            {
                stmts.insert(
                    1,
                    Stmt::StoreField(t.clone(), sid, i, mk(ExprKind::Null, fty)),
                );
            }
        }
        Ok(mk(ExprKind::Seq(stmts, Box::new(t)), Ty::Obj(sid)))
    }

    fn lower_array_lit(&mut self, a: &'a ArrayExpression<'a>, exp: Option<&Ty>) -> R<Expr> {
        let mut el = match exp.map(|t| t.strip()) {
            Some(Ty::Arr(e)) => Some((**e).clone()),
            _ => None,
        };
        let mut elems = Vec::new();
        for item in &a.elements {
            if let ArrayExpressionElement::SpreadElement(sp) = item {
                let hint = el.clone().map(|e| Ty::Arr(Box::new(e)));
                let v = self.lower_expr(&sp.argument, hint.as_ref())?;
                let vt = match v.ty.strip() {
                    Ty::Arr(e) => (**e).clone(),
                    Ty::Bytes => Ty::Num,
                    t => bail!("{}: cannot spread {t}", self.loc(sp.span.start)),
                };
                if el.is_none() {
                    el = Some(vt);
                }
                elems.push(Elem::Spread(v));
                continue;
            }
            let Some(e) = item.as_expression() else {
                bail!("{}: hole in an array literal", self.loc(a.span.start))
            };
            let v = self.lower_expr(e, el.as_ref())?;
            if el.is_none() {
                el = Some(v.ty.clone());
            }
            elems.push(Elem::One(v));
        }
        self.build_array(elems, el.unwrap_or(Ty::Num))
    }

    /// A new array of `el` from values and spread arrays, in order:
    /// `t = new []; t.push(v); for (x of src) t.push(x); ...`.
    pub(crate) fn build_array(&mut self, elems: Vec<Elem>, el: Ty) -> R<Expr> {
        let ty = Ty::Arr(Box::new(el.clone()));
        let tmp = self.new_local(ty.clone());
        let t = self.var_expr(tmp);
        let n = elems.iter().filter(|e| matches!(e, Elem::One(_))).count();
        let mut stmts = vec![Stmt::Assign(
            tmp,
            mk(ExprKind::Rt(RtFn::ArrNew, vec![num(n as f64)]), ty.clone()),
        )];
        let push = |t: &Expr, v: Expr| {
            Stmt::Eval(mk(
                ExprKind::Rt(RtFn::ArrPush, vec![t.clone(), v]),
                Ty::Void,
            ))
        };
        for e in elems {
            match e {
                Elem::One(v) => {
                    let v = self.coerce(v, &el)?;
                    stmts.push(push(&t, v));
                }
                Elem::Spread(src) => {
                    let it = self.begin_iter(src)?;
                    stmts.extend(it.pre);
                    let x = self.var_expr(it.elem);
                    let x = self.coerce(x, &el)?;
                    let mut body = it.head;
                    body.push(push(&t, x));
                    stmts.push(Stmt::Loop(it.label, body));
                }
            }
        }
        Ok(mk(ExprKind::Seq(stmts, Box::new(t)), ty))
    }

    fn lower_new(&mut self, n: &'a NewExpression<'a>, exp: Option<&Ty>) -> R<Expr> {
        let Expression::Identifier(id) = &n.callee else {
            bail!("{}: unsupported `new`", self.loc(n.span.start))
        };
        match id.name.as_str() {
            "Uint8Array" => {
                let len = match n.arguments.first().and_then(|a| a.as_expression()) {
                    Some(e) => self.lower_expr(e, Some(&Ty::Num))?,
                    None => num(0.0),
                };
                let len = self.as_num(len)?;
                Ok(mk(ExprKind::Rt(RtFn::BytesNew, vec![len]), Ty::Bytes))
            }
            // A set is an array here: `new Set(xs)` shares `xs`, `has` is a linear search. The sets in
            // gameplay code are tiny; a hash set would need hashing of strings and objects.
            "Set" => match n.arguments.first().and_then(|a| a.as_expression()) {
                Some(e) => self.lower_expr(e, exp),
                None => {
                    let el = match exp.map(|t| t.strip()) {
                        Some(Ty::Arr(e)) => (**e).clone(),
                        _ => Ty::Num,
                    };
                    let ty = Ty::Arr(Box::new(el));
                    Ok(mk(ExprKind::Rt(RtFn::ArrNew, vec![num(0.0)]), ty))
                }
            },
            other => bail!("{}: `new {other}` is not supported", self.loc(n.span.start)),
        }
    }

    // ---- assignment ----

    pub(crate) fn place_of(
        &mut self,
        t: &'a SimpleAssignmentTarget<'a>,
        pre: &mut Vec<Stmt>,
    ) -> R<Place> {
        match t {
            SimpleAssignmentTarget::AssignmentTargetIdentifier(id) => {
                match self.lookup(id.name.as_str()) {
                    Some(Binding::Direct(v)) => {
                        Ok(Place::Var(v, self.cur.locals[v as usize].clone()))
                    }
                    Some(Binding::Boxed(cell, ty)) => Ok(Place::Cell(self.var_expr(cell), ty)),
                    None => match self.consts.get(id.name.as_str()) {
                        Some(ConstInfo::Global(g, ty, true)) => Ok(Place::Global(*g, ty.clone())),
                        Some(_) => bail!("assignment to constant `{}`", id.name),
                        None => bail!("assignment to unknown name `{}`", id.name),
                    },
                }
            }
            SimpleAssignmentTarget::StaticMemberExpression(m) => {
                let obj = self.lower_expr(&m.object, None)?;
                let f = self.member_of(obj, m.property.name.as_str())?;
                let ExprKind::Field(obj, sid, idx) = f.kind else {
                    bail!("not a field")
                };
                let tmp = self.new_local(obj.ty.clone());
                pre.push(Stmt::Assign(tmp, *obj));
                Ok(Place::Field(self.var_expr(tmp), sid, idx, f.ty))
            }
            SimpleAssignmentTarget::ComputedMemberExpression(m) => {
                let arr = self.lower_expr(&m.object, None)?;
                let idx = self.lower_expr(&m.expression, Some(&Ty::Num))?;
                let idx = self.as_num(idx)?;
                let ty = match arr.ty.strip() {
                    Ty::Bytes => Ty::Num,
                    Ty::Arr(e) => (**e).clone(),
                    t => bail!("cannot index {t}"),
                };
                let (ta, ti) = (self.new_local(arr.ty.clone()), self.new_local(Ty::Num));
                pre.push(Stmt::Assign(ta, arr));
                pre.push(Stmt::Assign(ti, idx));
                Ok(Place::Index(self.var_expr(ta), self.var_expr(ti), ty))
            }
            SimpleAssignmentTarget::TSNonNullExpression(n) => {
                // `a.b! = ..` is rare; lower the inner target as a member read.
                let _ = n;
                bail!("assignment through `!`")
            }
            _ => bail!("unsupported assignment target"),
        }
    }

    pub(crate) fn place_ty(p: &Place) -> &Ty {
        match p {
            Place::Var(_, t)
            | Place::Cell(_, t)
            | Place::Global(_, t)
            | Place::Field(_, _, _, t)
            | Place::Index(_, _, t) => t,
        }
    }

    pub(crate) fn place_read(&self, p: &Place) -> Expr {
        match p {
            Place::Var(v, _) => self.var_expr(*v),
            Place::Cell(c, t) => mk(ExprKind::CellGet(Box::new(c.clone())), t.clone()),
            Place::Global(g, t) => mk(ExprKind::GlobalVar(*g), t.clone()),
            Place::Field(o, s, i, t) => mk(ExprKind::Field(Box::new(o.clone()), *s, *i), t.clone()),
            Place::Index(a, i, t) => mk(
                ExprKind::Index(Box::new(a.clone()), Box::new(i.clone())),
                t.clone(),
            ),
        }
    }

    pub(crate) fn place_write(&self, p: &Place, v: Expr) -> Stmt {
        match p {
            Place::Var(var, _) => Stmt::Assign(*var, v),
            Place::Cell(cell, _) => Stmt::CellSet(cell.clone(), v),
            Place::Global(g, _) => Stmt::StoreGlobal(*g, v),
            Place::Field(o, s, i, _) => Stmt::StoreField(o.clone(), *s, *i, v),
            Place::Index(a, i, _) => Stmt::StoreIndex(a.clone(), i.clone(), v),
        }
    }

    fn assign_target(&mut self, t: &'a AssignmentTarget<'a>, pre: &mut Vec<Stmt>) -> R<Place> {
        match t.as_simple_assignment_target() {
            Some(s) => self.place_of(s, pre),
            None => bail!("destructuring assignment is not supported"),
        }
    }

    /// Statements for an assignment, and an expression for its value.
    pub(crate) fn lower_assign(&mut self, a: &'a AssignmentExpression<'a>) -> R<(Vec<Stmt>, Expr)> {
        use AssignmentOperator as A;
        let mut pre = Vec::new();
        let place = self.assign_target(&a.left, &mut pre)?;
        let pty = Self::place_ty(&place).clone();
        let new = if a.operator == A::Assign {
            let v = self.lower_expr(&a.right, Some(&pty))?;
            self.coerce(v, &pty)?
        } else {
            let op = match a.operator {
                A::Addition => BinOp::Add,
                A::Subtraction => BinOp::Sub,
                A::Multiplication => BinOp::Mul,
                A::Division => BinOp::Div,
                A::Remainder => BinOp::Rem,
                A::BitwiseAnd => BinOp::BitAnd,
                A::BitwiseOR => BinOp::BitOr,
                A::BitwiseXOR => BinOp::BitXor,
                A::ShiftLeft => BinOp::Shl,
                A::ShiftRight => BinOp::Shr,
                A::ShiftRightZeroFill => BinOp::UShr,
                _ => bail!(
                    "{}: unsupported compound assignment",
                    self.loc(a.span.start)
                ),
            };
            let r = self.lower_expr(&a.right, Some(&Ty::Num))?;
            let old = self.place_read(&place);
            let v = self.arith(op, old, r)?;
            self.coerce(v, &pty)?
        };
        pre.push(self.place_write(&place, new));
        let value = self.place_read(&place);
        Ok((pre, value))
    }

    pub(crate) fn lower_update(&mut self, u: &'a UpdateExpression<'a>) -> R<(Vec<Stmt>, Expr)> {
        let mut pre = Vec::new();
        let place = self.place_of(&u.argument, &mut pre)?;
        let pty = Self::place_ty(&place).clone();
        let old = self.place_read(&place);
        let delta = if u.operator == UpdateOperator::Increment {
            1.0
        } else {
            -1.0
        };
        let new = self.arith(BinOp::Add, old.clone(), num(delta))?;
        let new = self.coerce(new, &pty)?;
        let value = if u.prefix {
            pre.push(self.place_write(&place, new));
            self.place_read(&place)
        } else {
            let tmp = self.new_local(Ty::Num);
            pre.push(Stmt::Assign(tmp, old));
            pre.push(self.place_write(&place, new));
            self.var_expr(tmp)
        };
        Ok((pre, value))
    }

    // ---- calls ----

    pub(crate) fn lower_args(
        &mut self,
        args: &'a [Argument<'a>],
        params: &[Ty],
        what: &str,
    ) -> R<Vec<Expr>> {
        if args.len() != params.len() {
            bail!(
                "{what}: {} arguments, {} expected",
                args.len(),
                params.len()
            );
        }
        let mut out = Vec::new();
        for (a, p) in args.iter().zip(params) {
            let Some(e) = a.as_expression() else {
                bail!("{what}: spread argument")
            };
            let v = self.lower_expr(e, Some(p))?;
            out.push(self.coerce(v, p)?);
        }
        Ok(out)
    }

    /// Arguments for a call of a declared function. A missing argument takes the parameter's
    /// default (lowered here, in a scope that has only the earlier parameters) or null for `x?: T`;
    /// the arguments after the fixed ones become the rest parameter's array.
    ///
    /// Defaults are filled in by the caller, which is why such a function must be called directly:
    /// as a value (`xs.map(f)`) every parameter has to be passed.
    pub(crate) fn lower_fn_args(
        &mut self,
        name: &str,
        args: &'a [Argument<'a>],
        ptys: &[Ty],
    ) -> R<(Vec<Stmt>, Vec<Expr>)> {
        let fp: &'a FormalParameters<'a> = self.fsrc[name].params;
        let n_fixed = fp.items.len();
        let rest_ty = fp.rest.as_ref().map(|_| ptys[n_fixed].clone());
        if rest_ty.is_none() && args.len() > n_fixed {
            bail!(
                "{name}: {} arguments, at most {n_fixed} expected",
                args.len()
            );
        }
        let given = args.len().min(n_fixed);
        let mut vals: Vec<Expr> = Vec::new();
        for (a, p) in args[..given].iter().zip(ptys) {
            let Some(e) = a.as_expression() else {
                bail!("{name}: a spread argument can only feed a rest parameter")
            };
            let v = self.lower_expr(e, Some(p))?;
            vals.push(self.coerce(v, p)?);
        }
        let mut pre = Vec::new();
        if given < n_fixed {
            // The defaults may use earlier parameters: give those names to temporaries.
            let mut names: HashMap<String, Binding> = HashMap::new();
            for i in 0..given {
                let tmp = self.new_local(ptys[i].clone());
                pre.push(Stmt::Assign(tmp, vals[i].clone()));
                vals[i] = self.var_expr(tmp);
                if let BindingPattern::BindingIdentifier(b) = &fp.items[i].pattern {
                    names.insert(b.name.to_string(), Binding::Direct(tmp));
                }
            }
            for (p, pty) in fp.items.iter().zip(ptys).skip(given) {
                let v = if let Some(init) = &p.initializer {
                    let saved = std::mem::replace(&mut self.cur.scopes, vec![names.clone()]);
                    let r = self.lower_expr(init, Some(pty));
                    self.cur.scopes = saved;
                    self.coerce(r?, pty)?
                } else if p.optional {
                    self.coerce(mk(ExprKind::Null, Ty::Null), pty)?
                } else {
                    bail!("{name}: {} arguments, {n_fixed} expected", args.len())
                };
                let tmp = self.new_local(pty.clone());
                pre.push(Stmt::Assign(tmp, v));
                vals.push(self.var_expr(tmp));
                if let BindingPattern::BindingIdentifier(b) = &p.pattern {
                    names.insert(b.name.to_string(), Binding::Direct(tmp));
                }
            }
        }
        if let Some(rt) = rest_ty {
            let Ty::Arr(el) = &rt else {
                bail!("{name}: a rest parameter must be an array")
            };
            let el = (**el).clone();
            let mut elems = Vec::new();
            for a in &args[given..] {
                if let Argument::SpreadElement(sp) = a {
                    let v = self.lower_expr(&sp.argument, Some(&rt))?;
                    elems.push(Elem::Spread(v));
                } else if let Some(e) = a.as_expression() {
                    let v = self.lower_expr(e, Some(&el))?;
                    elems.push(Elem::One(self.coerce(v, &el)?));
                }
            }
            vals.push(self.build_array(elems, el)?);
        }
        Ok((pre, vals))
    }

    fn lower_call(&mut self, c: &'a CallExpression<'a>) -> R<Expr> {
        match &c.callee {
            Expression::Identifier(id) => {
                let name = id.name.as_str();
                if self.lookup(name).is_some()
                    || matches!(self.consts.get(name), Some(ConstInfo::Global(..)))
                {
                    let callee = self.lower_ident(name)?;
                    let Ty::Fn(sig) = callee.ty.clone() else {
                        bail!("`{name}` is not callable")
                    };
                    let args = self.lower_args(&c.arguments, &sig.params, name)?;
                    return Ok(mk(
                        ExprKind::CallClosure(Box::new(callee), args),
                        sig.ret.clone(),
                    ));
                }
                let Some(&fid) = self.fids.get(name) else {
                    bail!(
                        "{}: call to unknown function `{name}`",
                        self.loc(c.span.start)
                    )
                };
                self.ensure_lowered(name)?;
                let (params, ret) = self.fsig[fid].clone();
                let Some(ret) = ret else {
                    bail!("{name}: return type unknown")
                };
                let (pre, args) = self.lower_fn_args(name, &c.arguments, &params)?;
                let call = mk(ExprKind::Call(fid, args), ret.clone());
                Ok(if pre.is_empty() {
                    call
                } else {
                    mk(ExprKind::Seq(pre, Box::new(call)), ret)
                })
            }
            Expression::StaticMemberExpression(m) => {
                let method = m.property.name.as_str();
                if let Expression::Identifier(id) = &m.object
                    && id.name == "Math"
                    && self.lookup("Math").is_none()
                {
                    return self.lower_math(method, c);
                }
                let mut recv = self.lower_expr(&m.object, None)?;
                if m.optional {
                    recv = self.chain_guard(recv)?;
                }
                // A function-typed field (`handlers.onHit(x)`) is called, not looked up as a method.
                if let Ty::Obj(sid) = recv.ty.strip()
                    && let Some(i) = self.m.structs[*sid].field(method)
                    && matches!(self.m.structs[*sid].fields[i].ty.strip(), Ty::Fn(_))
                {
                    let callee = self.member_of(recv, method)?;
                    return self.call_closure_value(callee, c);
                }
                self.lower_method(recv, method, c)
            }
            // Anything else that is a function value: `f()()`, `(() => 1)()`, `fs[0](x)`.
            callee => {
                let callee = self.lower_expr(callee, None)?;
                self.call_closure_value(callee, c)
            }
        }
    }

    /// Calls a function-typed value with the call's arguments.
    pub(crate) fn call_closure_value(
        &mut self,
        callee: Expr,
        c: &'a CallExpression<'a>,
    ) -> R<Expr> {
        let Ty::Fn(sig) = callee.ty.strip().clone() else {
            bail!(
                "{}: call of a value that is not a function",
                self.loc(c.span.start)
            )
        };
        let args = self.lower_args(&c.arguments, &sig.params, "closure")?;
        Ok(mk(
            ExprKind::CallClosure(Box::new(callee), args),
            sig.ret.clone(),
        ))
    }

    fn lower_math(&mut self, f: &str, c: &'a CallExpression<'a>) -> R<Expr> {
        let mut args = Vec::new();
        for a in &c.arguments {
            let Some(e) = a.as_expression() else {
                bail!("Math.{f}: spread")
            };
            let v = self.lower_expr(e, Some(&Ty::Num))?;
            args.push(self.as_num(v)?);
        }
        let un = |op, a: Expr| mk(ExprKind::Un(op, Box::new(a)), Ty::Num);
        let bin = |op, a: Expr, b: Expr| mk(ExprKind::Bin(op, Box::new(a), Box::new(b)), Ty::Num);
        let n = args.len();
        let mut it = args.into_iter();
        Ok(match (f, n) {
            ("sqrt", 1) => un(UnOp::Sqrt, it.next().unwrap()),
            ("abs", 1) => un(UnOp::Abs, it.next().unwrap()),
            ("floor", 1) => un(UnOp::Floor, it.next().unwrap()),
            ("sin", 1) => un(UnOp::Sin, it.next().unwrap()),
            ("cos", 1) => un(UnOp::Cos, it.next().unwrap()),
            ("sign", 1) => un(UnOp::Sign, it.next().unwrap()),
            ("hypot", 2) => bin(BinOp::Hypot, it.next().unwrap(), it.next().unwrap()),
            ("atan2", 2) => bin(BinOp::Atan2, it.next().unwrap(), it.next().unwrap()),
            ("imul", 2) => bin(BinOp::Imul, it.next().unwrap(), it.next().unwrap()),
            ("min" | "max", n) if n >= 2 => {
                let op = if f == "min" { BinOp::Min } else { BinOp::Max };
                let mut acc = it.next().unwrap();
                for x in it {
                    acc = bin(op, acc, x);
                }
                acc
            }
            _ => bail!(
                "{}: Math.{f} with {n} arguments is not supported",
                self.loc(c.span.start)
            ),
        })
    }
}
