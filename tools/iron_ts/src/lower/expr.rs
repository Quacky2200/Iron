//! Expression lowering.

use super::*;
use crate::manifest::ExternFn;

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

/// `**` with JavaScript's special cases (the runtime's `ts_rt_pow` does the same).
fn js_pow(x: f64, y: f64) -> f64 {
    if y.is_nan() || (x.abs() == 1.0 && y.is_infinite()) {
        f64::NAN
    } else {
        x.powf(y)
    }
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
    /// `xs.length`: reading is `Len`, writing truncates or grows.
    ArrLen(Expr),
    /// A property with a getter and/or setter: the receiver, the two functions and the value type.
    Accessor(Expr, Option<classes::Target>, classes::Target, Ty),
    Field(Expr, StructId, usize, Ty),
    Index(Expr, Expr, Ty),
    /// A property of an engine handle class: the receiver, its getter (if any), its setter and
    /// the value type.
    Ext(Expr, Option<ExternFn>, ExternFn, Ty),
    /// A property of a namespace (no receiver): its getter (if any), its setter and the value type.
    NsProp(Option<ExternFn>, ExternFn, Ty),
}

impl<'a> Lowerer<'a> {
    pub(crate) fn loc(&self, offset: u32) -> String {
        format!("line {}", self.line(offset))
    }

    /// A value used as a number (`number | null` narrowed away).
    pub(crate) fn as_num(&self, e: Expr) -> R<Expr> {
        // A Date is its time value (`end - start`, `+date`).
        let e = self.date_value(e);
        match e.ty.strip() {
            Ty::Num => self.coerce(e, &Ty::Num),
            t => bail!("expected a number, found {t}"),
        }
    }

    /// Is this call `JSON.parse(..)`? (It needs the expected type.)
    pub(crate) fn is_json_parse(c: &CallExpression<'_>) -> bool {
        matches!(&c.callee, Expression::StaticMemberExpression(m)
            if m.property.name == "parse"
                && matches!(&m.object, Expression::Identifier(id) if id.name == "JSON"))
    }

    pub(crate) fn lower_expr(&mut self, e: &'a Expression<'a>, exp: Option<&Ty>) -> R<Expr> {
        // `const order: SineType[] = xs.map(x => x.type)`: the callback's literals stay literals.
        if let (Some(Ty::Arr(el)), Expression::CallExpression(c)) = (exp, e)
            && **el == Ty::Lit
            && matches!(&c.callee, Expression::StaticMemberExpression(m) if m.property.name == "map")
        {
            self.map_keeps_lit = true;
        }
        match e {
            Expression::NumericLiteral(n) => Ok(num(n.value)),
            Expression::BigIntLiteral(b) => self.lower_bigint_literal(b),
            Expression::BooleanLiteral(b) => Ok(mk(ExprKind::Bool(b.value), Ty::Bool)),
            Expression::NullLiteral(_) => Ok(mk(ExprKind::Null, Ty::Null)),
            Expression::StringLiteral(s) => {
                // A literal with a lone surrogate cannot be a union member (tags are UTF-8 text).
                if s.lone_surrogates || matches!(exp.map(|t| t.strip()), Some(Ty::Str)) {
                    let bytes = strings::literal_bytes(s.value.as_str(), s.lone_surrogates);
                    return Ok(self.str_bytes(bytes));
                }
                let t = self.tag(s.value.as_str());
                Ok(mk(ExprKind::Tag(t), Ty::Lit))
            }
            Expression::TemplateLiteral(t) => self.lower_template(t),
            Expression::RegExpLiteral(r) => self.lower_regexp_literal(r),
            Expression::Identifier(id) => self.lower_ident(id.name.as_str()),
            Expression::ParenthesizedExpression(p) => self.lower_expr(&p.expression, exp),
            Expression::TSAsExpression(a) => {
                // `JSON.parse(text) as T`: the cast gives the type to build.
                let mut inner = &a.expression;
                while let Expression::ParenthesizedExpression(p) = inner {
                    inner = &p.expression;
                }
                if let Expression::CallExpression(c) = inner
                    && Self::is_json_parse(c)
                {
                    let ty = self.ty(&a.type_annotation, "JSON")?;
                    return self.lower_json_parse(c, Some(&ty));
                }
                let v = self.lower_expr(&a.expression, exp)?;
                // `x as Sub` for classes: the same pointer, seen as the subclass.
                if let Ok(Ty::Obj(t)) = self.ty(&a.type_annotation, "as")
                    && self.class_of_sid.contains_key(&t)
                    && matches!(v.ty.strip(), Ty::Obj(_))
                {
                    let ty = if v.ty.is_nullable() {
                        Ty::Obj(t).nullable()
                    } else {
                        Ty::Obj(t)
                    };
                    return Ok(mk(ExprKind::Cast(Box::new(v)), ty));
                }
                Ok(v)
            }
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
            Expression::ThisExpression(_) => match self.lookup("this") {
                Some(b) => Ok(self.read_binding(&b)),
                None => bail!(
                    "{}: `this` outside a method",
                    self.loc(oxc_span::GetSpan::span(e).start)
                ),
            },
            Expression::PrivateFieldExpression(p) => {
                let obj = self.lower_expr(&p.object, None)?;
                self.member_of(obj, &classes::private_name(p.field.name.as_str()))
            }
            Expression::StaticMemberExpression(m) => self.lower_static_member(m),
            Expression::ComputedMemberExpression(m) => self.lower_computed_member(m),
            Expression::ChainExpression(c) => self.lower_chain(c),
            Expression::CallExpression(c) if Self::is_json_parse(c) => {
                self.lower_json_parse(c, exp)
            }
            Expression::CallExpression(c) => self.lower_call(c),
            Expression::NewExpression(n) => self.lower_new(n, exp),
            Expression::ObjectExpression(o) => self.lower_object(o, exp, None),
            Expression::ArrayExpression(a) => self.lower_array_lit(a, exp),
            Expression::UnaryExpression(u) => self.lower_unary(u),
            Expression::BinaryExpression(b) => self.lower_binary(b),
            Expression::LogicalExpression(l) => self.lower_logical(l, exp),
            Expression::ConditionalExpression(c) => {
                let test = self.lower_cond(&c.test)?;
                let a = self.lower_expr(&c.consequent, exp)?;
                let b = self.lower_expr(&c.alternate, exp)?;
                // `on ? "pointer" : undefined` with no union expected is a `string | undefined`.
                let (a, b) = if (a.ty == Ty::Null || b.ty == Ty::Null)
                    && exp.is_none_or(|t| t.strip() != &Ty::Lit)
                {
                    (self.widen_lit(a), self.widen_lit(b))
                } else {
                    (a, b)
                };
                // `big ? 100 : "50%"` where a `number | string` is expected: both go to the union.
                let ty = match exp.map(|t| t.strip()) {
                    Some(sv) if self.is_style_value(sv) && a.ty != b.ty => {
                        if a.ty.is_nullable()
                            || b.ty.is_nullable()
                            || a.ty == Ty::Null
                            || b.ty == Ty::Null
                        {
                            sv.clone().nullable()
                        } else {
                            sv.clone()
                        }
                    }
                    _ => self.unify(&a.ty, &b.ty)?,
                };
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
        // `Time.deltaTime`: the getter of a namespace property.
        if let Expression::Identifier(id) = &m.object
            && self.lookup(id.name.as_str()).is_none()
            && let Some(g) = self
                .manifest
                .ns_props
                .get(id.name.as_str())
                .and_then(|p| p.get(prop))
                .and_then(|p| p.getter.clone())
        {
            return Ok(self.emit_extern(&g, vec![]));
        }
        if let Expression::Identifier(id) = &m.object
            && id.name == "Math"
            && self.lookup("Math").is_none()
        {
            return match prop {
                "PI" => Ok(num(std::f64::consts::PI)),
                "SQRT2" => Ok(num(std::f64::consts::SQRT_2)),
                "E" => Ok(num(std::f64::consts::E)),
                "LN2" => Ok(num(std::f64::consts::LN_2)),
                "LN10" => Ok(num(std::f64::consts::LN_10)),
                "LOG2E" => Ok(num(std::f64::consts::LOG2_E)),
                "LOG10E" => Ok(num(std::f64::consts::LOG10_E)),
                "SQRT1_2" => Ok(num(std::f64::consts::FRAC_1_SQRT_2)),
                _ => bail!("Math.{prop} is not a value"),
            };
        }
        if let Expression::Identifier(id) = &m.object
            && id.name == "Number"
            && self.lookup("Number").is_none()
        {
            return match prop {
                "EPSILON" => Ok(num(f64::EPSILON)),
                "MAX_SAFE_INTEGER" => Ok(num(9007199254740991.0)),
                "MIN_SAFE_INTEGER" => Ok(num(-9007199254740991.0)),
                "MAX_VALUE" => Ok(num(f64::MAX)),
                "MIN_VALUE" => Ok(num(f64::from_bits(1))),
                "POSITIVE_INFINITY" => Ok(num(f64::INFINITY)),
                "NEGATIVE_INFINITY" => Ok(num(f64::NEG_INFINITY)),
                "NaN" => Ok(num(f64::NAN)),
                _ => bail!("Number.{prop} is not a value"),
            };
        }
        // `GameObject.roots`: the objects at the world root.
        if let Expression::Identifier(id) = &m.object
            && id.name == "GameObject"
            && prop == "roots"
            && self.lookup("GameObject").is_none()
            && self.fids.contains_key("__go_roots")
        {
            return self.call_helper("__go_roots", vec![]);
        }
        if let Expression::Identifier(id) = &m.object
            && self.lookup(id.name.as_str()).is_none()
            && let Some(v) = self.enum_member(id.name.as_str(), prop)
        {
            return v;
        }
        if let Expression::Identifier(id) = &m.object
            && self.lookup(id.name.as_str()).is_none()
            && self.classes.contains_key(id.name.as_str())
        {
            // `static get current()`
            if let Some(f) = self.classes[id.name.as_str()]
                .statics
                .get(&format!("get {prop}"))
                .cloned()
            {
                return self.call_declared(&f, &[], None);
            }
            return match self.consts.get(&format!("{}.{prop}", id.name)).cloned() {
                Some(ConstInfo::Inline(e)) => Ok(e),
                Some(ConstInfo::Global(g, ty, false)) => Ok(mk(ExprKind::Global(g), ty)),
                Some(ConstInfo::Global(g, ty, true)) => Ok(mk(ExprKind::GlobalVar(g), ty)),
                None => bail!("class {} has no static value `{prop}`", id.name),
            };
        }
        let mut obj = self.lower_expr(&m.object, None)?;
        if m.optional {
            obj = self.chain_guard(obj)?;
        }
        self.member_of(obj, prop)
    }

    /// A method (or getter) a value class got from the engine API: `a.add(b)`, `v.magnitude`.
    pub(crate) fn value_member(&self, sid: StructId, name: &str, getter: bool) -> Option<ExternFn> {
        let class = self
            .value_structs
            .iter()
            .find_map(|(n, s)| (*s == sid).then_some(n))?;
        let k = self.manifest.classes.get(class)?;
        let table = if getter { &k.getters } else { &k.methods };
        table.get(name).cloned()
    }

    /// `obj.prop` on an already-lowered object.
    pub(crate) fn member_of(&mut self, obj: Expr, prop: &str) -> R<Expr> {
        // A string literal (or a constant of one) used as a `string`: `"abc".length`.
        let obj = self.widen_lit(obj);
        match obj.ty.strip().clone() {
            Ty::Obj(sid) => {
                let Some(i) = self.m.structs[sid].field(prop) else {
                    if Some(sid) == self.match_sid() && prop == "length" {
                        let view = self.match_view(obj);
                        return Ok(mk(ExprKind::Len(Box::new(view)), Ty::Num));
                    }
                    if let Some(g) = self.value_member(sid, prop, true) {
                        return Ok(self.emit_extern(&g, vec![obj]));
                    }
                    if let Some((stat, target)) = self.target_for(sid, Which::Getter, prop) {
                        return self.call_getter(&stat, &target, obj);
                    }
                    if self.resolve_in(sid, Which::Method, prop).is_some() {
                        return self.bound_method(obj, sid, prop);
                    }
                    // Only a subclass has it: tsc narrowed the object, so use it as that class.
                    let sub = self.downcast_for(sid, |s, c| {
                        s.has_field(c, prop)
                            || s.resolve_in(c, Which::Getter, prop).is_some()
                            || s.resolve_in(c, Which::Method, prop).is_some()
                    });
                    if let Some(sub) = sub {
                        return self.member_of(Self::downcast(obj, sub), prop);
                    }
                    bail!("{} has no field `{prop}`", self.m.structs[sid].name)
                };
                let ty = self.m.structs[sid].fields[i].ty.clone();
                Ok(mk(ExprKind::Field(Box::new(obj), sid, i), ty))
            }
            Ty::Arr(_) | Ty::Typed(_) if prop == "length" => {
                Ok(mk(ExprKind::Len(Box::new(obj)), Ty::Num))
            }
            Ty::Typed(_) if prop == "byteOffset" => {
                Ok(self.rt_str("ts_rt_typed_byte_offset", vec![obj], Ty::Num))
            }
            Ty::Typed(k) if prop == "buffer" => {
                let bytes = self.rt_str(
                    "ts_rt_typed_bytes_view_gc",
                    vec![obj, num(f64::from(k.code()))],
                    Ty::Typed(crate::ir::TypedKind::U8),
                );
                self.call_helper("__wrapBuffer", vec![bytes])
            }
            Ty::Typed(k) if prop == "byteLength" => {
                let len = mk(ExprKind::Len(Box::new(obj)), Ty::Num);
                Ok(mk(
                    ExprKind::Bin(BinOp::Mul, Box::new(len), Box::new(num(k.size() as f64))),
                    Ty::Num,
                ))
            }
            Ty::Str if prop == "length" => Ok(self.str_length(obj)),
            Ty::Map(..) | Ty::Set(_) if prop == "size" => Ok(self.collection_size(obj)),
            // `m.groups.year`: a regular expression's named groups are read by name.
            Ty::Map(k, v) if *k == Ty::Str && v.is_ptr() => self.map_prop(obj, prop),
            Ty::Handle(class) => {
                let getter = self
                    .manifest
                    .classes
                    .get(&class)
                    .and_then(|k| k.getters.get(prop))
                    .cloned();
                match getter {
                    Some(g) => Ok(self.emit_extern(&g, vec![obj])),
                    None if class == "GameObject" => self.game_object_member(obj, prop),
                    None => bail!("{class} has no property `{prop}`"),
                }
            }
            t => bail!("no property `{prop}` on {t}"),
        }
    }

    fn lower_computed_member(&mut self, m: &'a ComputedMemberExpression<'a>) -> R<Expr> {
        let mut arr = self.lower_expr(&m.object, None)?;
        // `names[kind]` where `names` is a `Record<Kind, V>` and `kind` a union member.
        if let Ty::Obj(sid) = arr.ty.strip().clone()
            && Some(sid) != self.match_sid()
            && !m.optional
        {
            let key = self.lower_expr(&m.expression, None)?;
            if key.ty == Ty::Lit && !self.m.structs[sid].fields.is_empty() {
                return Ok(self.record_index(arr, sid, key));
            }
            bail!("cannot index an object by {}", key.ty)
        }
        arr = self.match_view(arr);
        if m.optional {
            arr = self.chain_guard(arr)?;
        }
        let idx = self.lower_expr(&m.expression, Some(&Ty::Num))?;
        let idx = self.as_num(idx)?;
        let ty = match arr.ty.strip() {
            Ty::Typed(_) => Ty::Num,
            Ty::Arr(el) => match &**el {
                // Out of range reads null (a zero pointer, or the null NaN for numbers).
                e if e.is_ptr() || *e == Ty::Num => e.clone().nullable(),
                e => e.clone(),
            },
            t => bail!("cannot index {t}"),
        };
        Ok(mk(ExprKind::Index(Box::new(arr), Box::new(idx)), ty))
    }

    /// `rec[key]` for a record struct: the field whose name is the key's tag.
    fn record_index(&mut self, obj: Expr, sid: StructId, key: Expr) -> Expr {
        let fields = self.m.structs[sid].fields.clone();
        let ty = fields[0].ty.clone();
        let (to, tk) = (self.new_local(obj.ty.clone()), self.new_local(Ty::Lit));
        let (ov, kv) = (self.var_expr(to), self.var_expr(tk));
        let field = |i: usize| mk(ExprKind::Field(Box::new(ov.clone()), sid, i), ty.clone());
        let mut acc = field(fields.len() - 1);
        for i in (0..fields.len() - 1).rev() {
            let t = self.tag(&fields[i].name);
            let is = mk(
                ExprKind::Cmp(
                    CmpOp::Eq,
                    Box::new(kv.clone()),
                    Box::new(mk(ExprKind::Tag(t), Ty::Lit)),
                ),
                Ty::Bool,
            );
            acc = mk(
                ExprKind::Cond(Box::new(is), Box::new(field(i)), Box::new(acc)),
                ty.clone(),
            );
        }
        mk(
            ExprKind::Seq(
                vec![Stmt::Assign(to, obj), Stmt::Assign(tk, key)],
                Box::new(acc),
            ),
            ty,
        )
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
        // A union member that may be missing is a `string | undefined` (a tag has no null).
        let inner = self.widen_lit(inner?);
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
        if u.operator == UnaryOperator::Typeof {
            return self.lower_typeof(&u.argument);
        }
        if u.operator == UnaryOperator::LogicalNot {
            let b = self.lower_cond(&u.argument)?;
            return Ok(mk(ExprKind::Un(UnOp::Not, Box::new(b)), Ty::Bool));
        }
        let a = self.lower_expr(&u.argument, None)?;
        match u.operator {
            UnaryOperator::UnaryNegation if a.ty.strip() == &Ty::BigInt => {
                Ok(self.bigint_unary(true, a))
            }
            UnaryOperator::BitwiseNot if a.ty.strip() == &Ty::BigInt => {
                Ok(self.bigint_unary(false, a))
            }
            UnaryOperator::UnaryPlus if a.ty.strip() == &Ty::BigInt => {
                bail!(
                    "{}: unary + on a bigint (JavaScript throws a TypeError); use Number(x)",
                    self.loc(u.span.start)
                )
            }
            UnaryOperator::UnaryNegation => {
                let a = self.as_num(a)?;
                Ok(match a.kind {
                    ExprKind::Num(n) => num(-n),
                    _ => mk(ExprKind::Un(UnOp::Neg, Box::new(a)), Ty::Num),
                })
            }
            UnaryOperator::UnaryPlus => self.as_num(a),
            // ~x is ToInt32(x) ^ -1.
            UnaryOperator::BitwiseNot => self.arith(BinOp::BitXor, a, num(-1.0)),
            _ => bail!("{}: unsupported unary operator", self.loc(u.span.start)),
        }
    }

    /// `Namespace.fn(args)` for a function of the engine API: a direct call of its symbol, with
    /// the arguments checked against the manifest and numbers converted at the call.
    /// The struct of a manifest value class (`Vector3`): one `number` field per field, in order.
    pub(crate) fn value_struct(&mut self, name: &str) -> StructId {
        if let Some(s) = self.value_structs.get(name) {
            return *s;
        }
        let fields: Vec<Field> = self.manifest.values[name]
            .iter()
            .map(|(n, _)| Field {
                name: n.clone(),
                ty: Ty::Num,
                optional: false,
            })
            .collect();
        let sid = self.new_struct(name, fields);
        self.value_structs.insert(name.to_string(), sid);
        self.named.insert(name.to_string(), Ty::Obj(sid));
        sid
    }

    /// The IR type scripts see for a boundary type.
    pub(crate) fn ext_ir(&mut self, t: &ExtTy) -> Ty {
        match t {
            ExtTy::Value(n) => Ty::Obj(self.value_struct(n)),
            ExtTy::ValueArr(n) => Ty::Arr(Box::new(Ty::Obj(self.value_struct(n)))),
            other => other.to_ir(),
        }
    }

    /// `new Vector3(x, y, z)`: the fields in order.
    pub(crate) fn new_value(&mut self, name: &str, args: &'a [Argument<'a>], at: u32) -> R<Expr> {
        // A value class with a static `create` is built by it (`new DateTime(2024, 5, 3)`).
        if let Some(create) = self
            .manifest
            .namespaces
            .get(name)
            .and_then(|n| n.get("create"))
            .cloned()
        {
            return self.call_ext_args(create, None, &format!("new {name}"), at, args);
        }
        let sid = self.value_struct(name);
        let n = self.m.structs[sid].fields.len();
        if args.len() != n {
            bail!(
                "{}: new {name}: {} arguments, {n} expected",
                self.loc(at),
                args.len()
            )
        }
        let ty = Ty::Obj(sid);
        let tmp = self.new_local(ty.clone());
        let mut stmts = vec![Stmt::Assign(tmp, mk(ExprKind::NewObj(sid), ty.clone()))];
        for (i, a) in args.iter().enumerate() {
            let Some(e) = a.as_expression() else {
                bail!("new {name}: spread argument")
            };
            let v = self.lower_expr(e, Some(&Ty::Num))?;
            let v = self.as_num(v)?;
            stmts.push(Stmt::StoreField(self.var_expr(tmp), sid, i, v));
        }
        Ok(mk(ExprKind::Seq(stmts, Box::new(self.var_expr(tmp))), ty))
    }

    /// `Namespace.fn(args)` for a function of the engine API: a direct call of its symbol, with
    /// the arguments checked against the manifest and numbers converted at the call. A value
    /// class argument is passed as its fields; a value class result is written into a new object.
    fn lower_extern_call(&mut self, ns: &str, name: &str, c: &'a CallExpression<'a>) -> R<Expr> {
        if ns == "GameObject"
            && name == "findObjectOfType"
            && let Some(e) = self.lower_find_object_of_type(c)?
        {
            return Ok(e);
        }
        if ns == "Ui" && name == "load" {
            return self.lower_ui_load(c);
        }
        if ns == "Ui" && name == "runEffect" {
            return self.lower_ui_run_effect(c);
        }
        if ns == "Ui" && (name == "styleObj" || name == "attrObj") {
            return self.lower_ui_fields(name == "attrObj", c);
        }
        if ns == "Ui"
            && name == "child"
            && let Some(e) = self.lower_ui_child_opt(c)?
        {
            return Ok(e);
        }
        if ns == "Ui"
            && (name == "style" || name == "attr")
            && let Some(e) = self.lower_ui_set(name == "attr", c)?
        {
            return Ok(e);
        }
        if let Some(cands) = self
            .manifest
            .overloads
            .get(&(ns.to_string(), name.to_string()))
            .cloned()
        {
            return self.lower_overloaded(ns, name, &cands, c);
        }
        // A namespace named like a handle class (`AnimationCurve`) also holds the class statics.
        let found = self.manifest.namespaces[ns].get(name).cloned().or_else(|| {
            self.manifest
                .classes
                .get(ns)
                .and_then(|k| k.statics.get(name))
                .cloned()
        });
        let Some(f) = found else {
            bail!("{}: {ns} has no function `{name}`", self.loc(c.span.start))
        };
        self.call_ext_fn(f, None, &format!("{ns}.{name}"), c)
    }

    /// How an argument's type is passed to the boundary, for choosing among overloads.
    fn ext_class(&self, t: &Ty) -> Option<ExtTy> {
        Some(match t {
            Ty::Num => ExtTy::F64,
            Ty::Bool => ExtTy::Bool,
            Ty::Str | Ty::Lit => ExtTy::Str,
            Ty::Handle(n) => ExtTy::Handle(n.clone()),
            Ty::Arr(e) if matches!(&**e, Ty::Handle(_)) => {
                let Ty::Handle(n) = &**e else { unreachable!() };
                ExtTy::HandleArr(n.clone())
            }
            Ty::Arr(e) if **e == Ty::Num => ExtTy::F64Arr,
            Ty::Arr(e)
                if matches!(&**e, Ty::Obj(sid)
                    if self.manifest.values.contains_key(&self.m.structs[*sid].name)) =>
            {
                let Ty::Obj(sid) = &**e else { unreachable!() };
                ExtTy::ValueArr(self.m.structs[*sid].name.clone())
            }
            Ty::Nullable(inner) => match &**inner {
                Ty::Num => ExtTy::F64,
                Ty::Handle(n) => ExtTy::Handle(n.clone()),
                // A list that may be missing (`children?: ReactNode`): null is an empty list.
                Ty::Arr(e) if matches!(&**e, Ty::Handle(_)) => {
                    let Ty::Handle(n) = &**e else { unreachable!() };
                    ExtTy::HandleArr(n.clone())
                }
                other if other.is_ptr() => ExtTy::Ptr,
                _ => return None,
            },
            // A value class (`Vector3`) is passed as itself.
            Ty::Obj(sid)
                if self
                    .manifest
                    .values
                    .contains_key(&self.m.structs[*sid].name) =>
            {
                ExtTy::Value(self.m.structs[*sid].name.clone())
            }
            other if other.is_ptr() => ExtTy::Ptr,
            _ => return None,
        })
    }

    /// A boundary function with several versions: the one whose parameters take the argument types.
    fn lower_overloaded(
        &mut self,
        ns: &str,
        name: &str,
        cands: &[ExternFn],
        c: &'a CallExpression<'a>,
    ) -> R<Expr> {
        let mut vals = Vec::new();
        for a in &c.arguments {
            let Some(e) = a.as_expression() else {
                bail!("{ns}.{name}: spread argument")
            };
            let v = self.lower_expr(e, None)?;
            vals.push(self.widen_lit(v));
        }
        let at = self.loc(c.span.start);
        self.pick_overload(ns, name, cands, vals, &at)
    }

    /// The version of a boundary function whose parameters take the (lowered) argument types.
    fn pick_overload(
        &mut self,
        ns: &str,
        name: &str,
        cands: &[ExternFn],
        vals: Vec<Expr>,
        at: &str,
    ) -> R<Expr> {
        let classes: Vec<Option<ExtTy>> = vals.iter().map(|v| self.ext_class(&v.ty)).collect();
        for f in cands {
            if f.params.len() == vals.len()
                && f.params
                    .iter()
                    .zip(&classes)
                    .all(|((_, p), c)| c.as_ref() == Some(p))
            {
                return Ok(self.emit_extern(f, vals));
            }
        }
        // A string or array with no version of its own goes where any script pointer does.
        let loose: Vec<Option<ExtTy>> = classes
            .iter()
            .map(|c| match c {
                Some(ExtTy::Str | ExtTy::HandleArr(_) | ExtTy::F64Arr | ExtTy::ValueArr(_)) => {
                    Some(ExtTy::Ptr)
                }
                other => other.clone(),
            })
            .collect();
        for f in cands {
            if f.params.len() == vals.len()
                && f.params
                    .iter()
                    .zip(&loose)
                    .all(|((_, p), c)| c.as_ref() == Some(p))
            {
                return Ok(self.emit_extern(f, vals));
            }
        }
        let have: Vec<String> = vals.iter().map(|v| v.ty.to_string()).collect();
        bail!("{at}: {ns}.{name} has no version for ({})", have.join(", "))
    }

    /// `Ui.load<T>(slot)`: reads a hook slot as a `T` (a number, a boolean or a script pointer).
    fn lower_ui_load(&mut self, c: &'a CallExpression<'a>) -> R<Expr> {
        let at = self.loc(c.span.start);
        let (Some(targ), Some(arg)) = (
            c.type_arguments.as_ref().and_then(|t| t.params.first()),
            c.arguments.first().and_then(|a| a.as_expression()),
        ) else {
            bail!("{at}: Ui.load<T>(slot)")
        };
        let t = self.ty(targ, "Ui.load")?;
        let slot = self.lower_expr(arg, Some(&Ty::Num))?;
        let slot = self.coerce(slot, &Ty::Num)?;
        let (symbol, ret) = match t.strip() {
            Ty::Num => ("ts_ui_load_num", ExtTy::F64),
            Ty::Bool => ("ts_ui_load_bool", ExtTy::Bool),
            Ty::Handle(n) => ("ts_ui_load_num", ExtTy::Handle(n.clone())),
            other if other.is_ptr() => ("ts_ui_load_ptr", ExtTy::Ptr),
            other => bail!("{at}: Ui.load<{other}> is not supported"),
        };
        let f = ExternFn {
            optional_from: usize::MAX,
            defaults: Vec::new(),
            symbol: symbol.into(),
            params: vec![("slot".into(), ExtTy::F64)],
            ret,
        };
        let mut e = self.emit_extern(&f, vec![slot]);
        e.ty = t;
        Ok(e)
    }

    /// `Ui.styleObj(h, obj)` / `Ui.attrObj(h, obj)` (a dynamic `style={x}` or `{...x}` in JSX): the
    /// fields of a typed object become style properties or attributes. An optional field that is
    /// null is left out; a number, string or boolean goes through as itself; anything else (a
    /// function, a nested object) is an error that names the field.
    fn lower_ui_fields(&mut self, attr: bool, c: &'a CallExpression<'a>) -> R<Expr> {
        let at = self.loc(c.span.start);
        let what = if attr { "{...props}" } else { "style={...}" };
        let (Some(he), Some(oe)) = (
            c.arguments.first().and_then(|a| a.as_expression()),
            c.arguments.get(1).and_then(|a| a.as_expression()),
        ) else {
            bail!("{at}: Ui.styleObj(h, obj)")
        };
        let h = self.lower_expr(he, None)?;
        // `style={..}` is an `IronStyle` (a list of property entries) when the ambient types are there.
        let bag = if !attr && self.fids.contains_key("__apply_style") {
            Some(Ty::Arr(Box::new(self.named_type("StyleEntry")?)))
        } else {
            None
        };
        let o = self.lower_expr(oe, bag.as_ref())?;
        if let Some(bag) = &bag
            && o.ty.strip() == bag
        {
            let o = self.coerce(o, &bag.clone().nullable())?;
            return Ok(mk(
                ExprKind::Call(self.fids["__apply_style"], vec![h, o]),
                Ty::Handle("UiElement".into()),
            ));
        }
        let Ty::Obj(sid) = o.ty.strip().clone() else {
            bail!(
                "{at}: {what} needs an object with known fields, not {}",
                o.ty
            )
        };
        let th = self.new_local(h.ty.clone());
        let to = self.new_local(o.ty.clone());
        let mut stmts = vec![Stmt::Assign(th, h.clone()), Stmt::Assign(to, o)];
        let fields = self.m.structs[sid].fields.clone();
        for (i, f) in fields.iter().enumerate() {
            let field = mk(
                ExprKind::Field(Box::new(self.var_expr(to)), sid, i),
                f.ty.clone(),
            );
            let set = self.ui_set_stmts(th, &f.name, field, attr, &at, what)?;
            stmts.extend(set);
        }
        let result = self.var_expr(th);
        let ty = result.ty.clone();
        Ok(mk(ExprKind::Seq(stmts, Box::new(result)), ty))
    }

    /// The statements that set one style property or attribute `key` of the element in `th` to
    /// `field` (a pure expression): a number, a string, a boolean (attributes), a `number | string`
    /// (`StyleValue`), or any of those that may be null, which leaves the property out.
    pub(crate) fn ui_set_stmts(
        &mut self,
        th: VarId,
        key: &str,
        field: Expr,
        attr: bool,
        at: &str,
        what: &str,
    ) -> R<Vec<Stmt>> {
        let element = || ExtTy::Handle("UiElement".into());
        let ext = |symbol: &str, vt: ExtTy| ExternFn {
            optional_from: usize::MAX,
            defaults: Vec::new(),
            symbol: symbol.into(),
            params: vec![
                ("h".into(), element()),
                ("key".into(), ExtTy::Str),
                ("v".into(), vt),
            ],
            ret: element(),
        };
        let mut plain = mk(
            ExprKind::Cast(Box::new(field.clone())),
            field.ty.strip().clone(),
        );
        plain = self.widen_lit(plain);
        let mut out = Vec::new();
        if let Ty::Obj(sid) = plain.ty.clone()
            && self.is_style_value(&plain.ty)
        {
            // `n`, `s`, `isStr` are fields 0, 1, 2 of the prelude class.
            let tv = self.new_local(plain.ty.clone());
            out.push(Stmt::Assign(tv, plain));
            let get = |s: &Self, i: usize, ty: Ty| {
                mk(ExprKind::Field(Box::new(s.var_expr(tv)), sid, i), ty)
            };
            let (n, st, is_str) = (
                get(self, 0, Ty::Num),
                get(self, 1, Ty::Str),
                get(self, 2, Ty::Bool),
            );
            let (num_sym, str_sym) = if attr {
                ("ts_ui_attr_num", "ts_ui_attr_str")
            } else {
                ("ts_ui_style_num", "ts_ui_style_str")
            };
            let hv = self.var_expr(th);
            let k = self.str_lit(key);
            let cn = self.emit_extern(&ext(num_sym, ExtTy::F64), vec![hv, k, n]);
            let hv = self.var_expr(th);
            let k = self.str_lit(key);
            let cs = self.emit_extern(&ext(str_sym, ExtTy::Str), vec![hv, k, st]);
            out.push(Stmt::If(is_str, vec![Stmt::Eval(cs)], vec![Stmt::Eval(cn)]));
        } else {
            let (symbol, vt) = match (&plain.ty, attr) {
                (Ty::Num, false) => ("ts_ui_style_num", ExtTy::F64),
                (Ty::Str, false) => ("ts_ui_style_str", ExtTy::Str),
                (Ty::Num, true) => ("ts_ui_attr_num", ExtTy::F64),
                (Ty::Str, true) => ("ts_ui_attr_str", ExtTy::Str),
                (Ty::Bool, true) => ("ts_ui_attr_bool", ExtTy::Bool),
                (other, _) => {
                    bail!("{at}: {what}: `{key}` has type {other}, which cannot be set this way")
                }
            };
            let hv = self.var_expr(th);
            let k = self.str_lit(key);
            let call = self.emit_extern(&ext(symbol, vt), vec![hv, k, plain]);
            out.push(Stmt::Eval(call));
        }
        Ok(if field.ty.is_nullable() {
            vec![Stmt::If(
                mk(ExprKind::IsNull(Box::new(field)), Ty::Bool),
                vec![],
                out,
            )]
        } else {
            out
        })
    }

    /// `Ui.child(h, x)` where `x` may be undefined (`{error}` for `error?: string`): nothing is
    /// added then. `None` when `x` is not a nullable string or number.
    fn lower_ui_child_opt(&mut self, c: &'a CallExpression<'a>) -> R<Option<Expr>> {
        let at = self.loc(c.span.start);
        let [h, v] = c.arguments.as_slice() else {
            return Ok(None);
        };
        let (Some(h), Some(v)) = (h.as_expression(), v.as_expression()) else {
            return Ok(None);
        };
        let ve = self.lower_expr(v, None)?;
        if !matches!(&ve.ty, Ty::Nullable(inner) if matches!(&**inner, Ty::Str | Ty::Num | Ty::Lit))
        {
            // Not ours: lower again through the plain path with the value already lowered.
            let he = self.lower_expr(h, None)?;
            let cands = self
                .manifest
                .overloads
                .get(&("Ui".to_string(), "child".to_string()))
                .cloned()
                .unwrap_or_default();
            let vals = vec![he, self.widen_lit(ve)];
            return self
                .pick_overload("Ui", "child", &cands, vals, &at)
                .map(Some);
        }
        let he = self.lower_expr(h, None)?;
        let th = self.new_local(he.ty.clone());
        let tv = self.new_local(ve.ty.clone());
        let plain = mk(
            ExprKind::Cast(Box::new(self.var_expr(tv))),
            ve.ty.strip().clone(),
        );
        let plain = self.widen_lit(plain);
        let cands = self
            .manifest
            .overloads
            .get(&("Ui".to_string(), "child".to_string()))
            .cloned()
            .unwrap_or_default();
        let call =
            self.pick_overload("Ui", "child", &cands, vec![self.var_expr(th), plain], &at)?;
        let stmts = vec![
            Stmt::Assign(th, he),
            Stmt::Assign(tv, ve),
            Stmt::If(
                mk(ExprKind::IsNull(Box::new(self.var_expr(tv))), Ty::Bool),
                vec![],
                vec![Stmt::Eval(call)],
            ),
        ];
        let result = self.var_expr(th);
        let ty = result.ty.clone();
        Ok(Some(mk(ExprKind::Seq(stmts, Box::new(result)), ty)))
    }

    /// `Ui.style(h, key, v)` / `Ui.attr(h, key, v)` whose value has no version of its own: a
    /// `number | string`, or a value that may be undefined (`cursor: on ? "pointer" : undefined`).
    fn lower_ui_set(&mut self, attr: bool, c: &'a CallExpression<'a>) -> R<Option<Expr>> {
        let at = self.loc(c.span.start);
        let [h, k, v] = c.arguments.as_slice() else {
            return Ok(None);
        };
        let (Some(h), Some(k), Some(v)) = (h.as_expression(), k.as_expression(), v.as_expression())
        else {
            return Ok(None);
        };
        let Expression::StringLiteral(key) = k else {
            return Ok(None);
        };
        // Lower the value once: if it needs no special path, the plain call is built from it.
        let ve = self.lower_expr(v, None)?;
        let special = ve.ty.is_nullable() || self.is_style_value(ve.ty.strip());
        let he = self.lower_expr(h, None)?;
        let fname = if attr { "attr" } else { "style" };
        if !special {
            let cands = self
                .manifest
                .overloads
                .get(&("Ui".to_string(), fname.to_string()))
                .cloned()
                .unwrap_or_default();
            let vals = vec![he, self.str_lit(key.value.as_str()), self.widen_lit(ve)];
            return self.pick_overload("Ui", fname, &cands, vals, &at).map(Some);
        }
        let th = self.new_local(he.ty.clone());
        let tv = self.new_local(ve.ty.clone());
        let mut stmts = vec![Stmt::Assign(th, he), Stmt::Assign(tv, ve)];
        let value = self.var_expr(tv);
        let set = self.ui_set_stmts(th, key.value.as_str(), value, attr, &at, "Ui")?;
        stmts.extend(set);
        let result = self.var_expr(th);
        let ty = result.ty.clone();
        Ok(Some(mk(ExprKind::Seq(stmts, Box::new(result)), ty)))
    }

    /// `Ui.runEffect(f)`: calls an effect function; a function it returns is its cleanup.
    fn lower_ui_run_effect(&mut self, c: &'a CallExpression<'a>) -> R<Expr> {
        let at = self.loc(c.span.start);
        let Some(arg) = c.arguments.first().and_then(|a| a.as_expression()) else {
            bail!("{at}: Ui.runEffect(f)")
        };
        let saved = std::mem::replace(&mut self.infer_ret, true);
        let hint = Ty::Fn(Box::new(FnTy {
            params: vec![],
            ret: Ty::Void,
        }));
        let f = self.lower_expr(arg, Some(&hint));
        self.infer_ret = saved;
        let f = f?;
        let Ty::Fn(sig) = f.ty.strip().clone() else {
            bail!("{at}: an effect must be a function")
        };
        let tf = self.new_local(f.ty.clone());
        let mut stmts = vec![Stmt::Assign(tf, f)];
        let call = mk(
            ExprKind::CallClosure(Box::new(self.var_expr(tf)), vec![]),
            sig.ret.clone(),
        );
        match sig.ret.strip() {
            Ty::Void => stmts.push(Stmt::Eval(call)),
            Ty::Fn(_) => {
                let r = self.new_local(sig.ret.clone());
                stmts.push(Stmt::Assign(r, call));
                let set = ExternFn {
                    optional_from: usize::MAX,
                    defaults: Vec::new(),
                    symbol: "ts_ui_set_cleanup".into(),
                    params: vec![("f".into(), ExtTy::Ptr)],
                    ret: ExtTy::Void,
                };
                let r = self.var_expr(r);
                stmts.push(Stmt::Eval(self.emit_extern(&set, vec![r])));
            }
            other => bail!("{at}: an effect returns nothing or a cleanup function, not {other}"),
        }
        let unit = mk(ExprKind::Bool(false), Ty::Void);
        Ok(mk(ExprKind::Seq(stmts, Box::new(unit)), Ty::Void))
    }

    /// A call of a boundary function with the call's arguments, after an optional receiver
    /// (the first parameter of a class method).
    pub(crate) fn call_ext_fn(
        &mut self,
        f: ExternFn,
        recv: Option<Expr>,
        what: &str,
        c: &'a CallExpression<'a>,
    ) -> R<Expr> {
        self.call_ext_args(f, recv, what, c.span.start, &c.arguments)
    }

    /// `call_ext_fn` over an argument list (a call's, or a `new`'s).
    pub(crate) fn call_ext_args(
        &mut self,
        f: ExternFn,
        recv: Option<Expr>,
        what: &str,
        span_start: u32,
        arguments: &'a [Argument<'a>],
    ) -> R<Expr> {
        let skip = usize::from(recv.is_some());
        let at = self.loc(span_start);
        let params: Vec<(String, ExtTy)> = f.params[skip..].to_vec();
        let mut given: Vec<&'a Expression<'a>> = Vec::new();
        for a in arguments {
            let Some(e) = a.as_expression() else {
                bail!("{at}: {what}: spread argument")
            };
            given.push(e);
        }
        // A trailing `{ key: value }` fills the parameters after the positional ones by name
        // (`audio.load(path, { stream: true })`); a parameter left out is false, 0 or "".
        let mut options: Option<&'a ObjectExpression<'a>> = None;
        if let Some(Expression::ObjectExpression(o)) = given.last().copied()
            && given.len() <= params.len()
            // An object where a value class goes is the value (`{ x, y }`) unless its keys are
            // parameter names and not the value's fields.
            && match &params[given.len() - 1].1 {
                ExtTy::Value(vn) => !o.properties.iter().any(|p| {
                    matches!(p, ObjectPropertyKind::ObjectProperty(p)
                        if matches!(&p.key, PropertyKey::StaticIdentifier(k)
                            if self.manifest.values.get(vn.as_str())
                                .is_some_and(|f| f.iter().any(|(n, _)| n == k.name.as_str()))))
                }),
                _ => true,
            }
            && o.properties.iter().all(|p| {
                matches!(p, ObjectPropertyKind::ObjectProperty(p)
                    if matches!(&p.key, PropertyKey::StaticIdentifier(k)
                        if params[given.len() - 1..].iter().any(|(n, _)| n == k.name.as_str())))
            })
        {
            options = Some(o);
            given.pop();
        }
        let opt_from = f.optional_from.saturating_sub(skip);
        if given.len() > params.len()
            || (options.is_none() && given.len() < params.len() && given.len() < opt_from && {
                // A flag may be left out (false); a missing number or string is an error.
                !params[given.len()..]
                    .iter()
                    .all(|(_, t)| matches!(t, ExtTy::Bool))
            })
        {
            bail!(
                "{at}: {what}: {} arguments, {} expected",
                arguments.len(),
                params.len()
            )
        }
        let mut args: Vec<Expr> = recv.into_iter().collect();
        for (i, (pname, pt)) in params.iter().enumerate() {
            let expr: Option<&'a Expression<'a>> = if i < given.len() {
                Some(given[i])
            } else {
                options.and_then(|o| {
                    o.properties.iter().find_map(|p| match p {
                        ObjectPropertyKind::ObjectProperty(p)
                            if matches!(&p.key, PropertyKey::StaticIdentifier(k) if k.name == pname.as_str()) =>
                        {
                            Some(&p.value)
                        }
                        _ => None,
                    })
                })
            };
            let Some(e) = expr else {
                // Left out: the default the API names, else false, 0 or an empty string.
                if let Some(Some(d)) = f.defaults.get(skip + i) {
                    match pt {
                        ExtTy::Bool => {
                            args.push(mk(ExprKind::Bool(*d != 0.0), Ty::Bool));
                            continue;
                        }
                        ExtTy::F64 | ExtTy::F32 | ExtTy::I32 | ExtTy::U32 => {
                            args.push(num(*d));
                            continue;
                        }
                        _ => {}
                    }
                }
                args.push(match pt {
                    ExtTy::Bool => mk(ExprKind::Bool(false), Ty::Bool),
                    ExtTy::Str => self.str_lit(""),
                    // An optional number is NaN: the function tells "left out" from 0.
                    ExtTy::F64 | ExtTy::F32 if i >= opt_from => num(f64::NAN),
                    ExtTy::F64 | ExtTy::F32 | ExtTy::I32 | ExtTy::U32 => num(0.0),
                    ExtTy::Handle(_) if i >= opt_from => mk(ExprKind::Null, self.ext_ir(pt)),
                    // An optional value class has NaN in every field: the function tells "left
                    // out" from a value.
                    ExtTy::Value(vn) if i >= opt_from => {
                        let sid = self.value_struct(vn);
                        let nf = self.m.structs[sid].fields.len();
                        let ty = Ty::Obj(sid);
                        let tmp = self.new_local(ty.clone());
                        let mut stmts =
                            vec![Stmt::Assign(tmp, mk(ExprKind::NewObj(sid), ty.clone()))];
                        for k in 0..nf {
                            stmts.push(Stmt::StoreField(self.var_expr(tmp), sid, k, num(f64::NAN)));
                        }
                        mk(ExprKind::Seq(stmts, Box::new(self.var_expr(tmp))), ty)
                    }
                    // An optional array is empty.
                    ExtTy::F64Arr | ExtTy::ValueArr(_) | ExtTy::HandleArr(_) if i >= opt_from => {
                        let ty = self.ext_ir(pt);
                        mk(ExprKind::Rt(RtFn::ArrNew, vec![num(0.0)]), ty)
                    }
                    other => bail!("{at}: {what}: parameter `{pname}` ({other:?}) has no default"),
                });
                continue;
            };
            if *pt == ExtTy::Ptr {
                // Any script pointer goes through as it is.
                let v = self.lower_expr(e, None)?;
                if !v.ty.is_ptr() {
                    bail!("{at}: {what}: expected an object, found {}", v.ty)
                }
                args.push(v);
                continue;
            }
            let want = self.ext_ir(pt);
            let mut v = self.lower_expr(e, Some(&want))?;
            // A value class argument may be another class with the same fields (a `Vector2Int`
            // where a `Vector2` goes): the fields are read by position.
            if let (Ty::Obj(a), Ty::Obj(b)) = (v.ty.strip(), &want)
                && a != b
                && self.manifest.values.contains_key(&self.m.structs[*a].name)
                && self.m.structs[*a].fields.len() == self.m.structs[*b].fields.len()
            {
                v.ty = want.clone();
            }
            args.push(self.coerce_ext(v, &want)?);
        }
        Ok(self.emit_extern(&f, args))
    }

    /// `coerce` for an argument of the engine API: a string where a handle is expected becomes
    /// the handle's `fromString` (`mesh: "cube"` is `Mesh.fromString("cube")`).
    pub(crate) fn coerce_ext(&mut self, v: Expr, want: &Ty) -> R<Expr> {
        if let Ty::Handle(class) = want.strip()
            && matches!(v.ty.strip(), Ty::Str | Ty::Lit)
            && let Some(from) = self
                .manifest
                .classes
                .get(class.as_str())
                .and_then(|k| k.statics.get("fromString"))
                .cloned()
        {
            let s = self.coerce(v, &Ty::Str)?;
            return Ok(self.emit_extern(&from, vec![s]));
        }
        self.coerce(v, want)
    }

    /// The call of a boundary function with arguments already lowered to the script types. A
    /// value class argument is passed as its fields; a value class result is written into a
    /// new object.
    pub(crate) fn emit_extern(&mut self, f: &ExternFn, args_in: Vec<Expr>) -> Expr {
        let any_value = f.params.iter().any(|(_, t)| matches!(t, ExtTy::Value(_)));
        let mut pre: Vec<Stmt> = Vec::new();
        let mut args: Vec<Expr> = Vec::new();
        let mut flat: Vec<ExtTy> = Vec::new();
        for (v, (_, pt)) in args_in.into_iter().zip(&f.params) {
            // With a value argument every argument goes into a temporary first, so they are
            // still evaluated in order.
            let v = if any_value {
                let t = self.new_local(v.ty.clone());
                pre.push(Stmt::Assign(t, v));
                self.var_expr(t)
            } else {
                v
            };
            match pt {
                ExtTy::Value(vn) => {
                    let sid = self.value_struct(vn);
                    for (i, (_, ft)) in self.manifest.values[vn].clone().iter().enumerate() {
                        args.push(mk(ExprKind::Field(Box::new(v.clone()), sid, i), Ty::Num));
                        flat.push(ft.clone());
                    }
                }
                other => {
                    args.push(v);
                    flat.push(other.clone());
                }
            }
        }
        let (ret_ext, out) = match &f.ret {
            ExtTy::Value(vn) => {
                let fields = &self.manifest.values[vn];
                (ExtTy::Void, Some((fields[0].1.clone(), fields.len())))
            }
            other => (other.clone(), None),
        };
        let decl = ExternDecl {
            symbol: f.symbol.clone(),
            params: flat,
            ret: ret_ext,
            out,
        };
        let idx = match self.m.externs.iter().position(|d| d.symbol == decl.symbol) {
            Some(i) => i,
            None => {
                self.m.externs.push(decl);
                self.m.externs.len() - 1
            }
        };
        if let ExtTy::Value(vn) = &f.ret {
            let sid = self.value_struct(vn);
            let ty = Ty::Obj(sid);
            let dest = self.new_local(ty.clone());
            pre.push(Stmt::Assign(dest, mk(ExprKind::NewObj(sid), ty.clone())));
            pre.push(Stmt::Eval(mk(
                ExprKind::CallExternInto(idx, args, Box::new(self.var_expr(dest))),
                Ty::Void,
            )));
            return mk(ExprKind::Seq(pre, Box::new(self.var_expr(dest))), ty);
        }
        let ret_ty = self.ext_ir(&f.ret);
        let call = mk(ExprKind::CallExtern(idx, args), ret_ty.clone());
        if pre.is_empty() {
            call
        } else {
            mk(ExprKind::Seq(pre, Box::new(call)), ret_ty)
        }
    }

    /// `x instanceof C` for a class `C` of this program: a header test against the table of `C`
    /// and its subclasses.
    fn lower_instanceof(&mut self, b: &'a BinaryExpression<'a>) -> R<Expr> {
        let Expression::Identifier(id) = &b.right else {
            bail!(
                "{}: `instanceof` needs a class name",
                self.loc(b.span.start)
            )
        };
        if self.generic_classes.contains_key(id.name.as_str()) {
            // `x instanceof Box`: any specialisation of Box, now or made later (fixed up at the end).
            let l = self.lower_expr(&b.left, None)?;
            if !matches!(l.ty.strip(), Ty::Obj(_)) {
                bail!("{}: `instanceof` on {}", self.loc(b.span.start), l.ty)
            }
            self.m.itables.push(Vec::new());
            let table = self.m.itables.len() - 1;
            self.generic_itables.push((table, id.name.to_string()));
            return Ok(mk(ExprKind::InstanceOf(Box::new(l), table), Ty::Bool));
        }
        if !self.classes.contains_key(id.name.as_str()) {
            bail!(
                "{}: `instanceof {}` is only supported for classes of this program",
                self.loc(b.span.start),
                id.name
            )
        }
        let sid = self.classes_sid(id.name.as_str());
        let l = self.lower_expr(&b.left, None)?;
        if !matches!(l.ty.strip(), Ty::Obj(_)) {
            bail!("{}: `instanceof` on {}", self.loc(b.span.start), l.ty)
        }
        let mut members: Vec<StructId> = self
            .class_of_sid
            .keys()
            .copied()
            .filter(|s| self.is_subclass(*s, sid))
            .collect();
        members.sort_unstable();
        let table = match self.m.itables.iter().position(|t| *t == members) {
            Some(i) => i,
            None => {
                self.m.itables.push(members);
                self.m.itables.len() - 1
            }
        };
        Ok(mk(ExprKind::InstanceOf(Box::new(l), table), Ty::Bool))
    }

    fn lower_binary(&mut self, b: &'a BinaryExpression<'a>) -> R<Expr> {
        use BinaryOperator as B;
        if b.operator == B::Instanceof {
            return self.lower_instanceof(b);
        }
        if b.operator == B::In {
            let obj = self.lower_expr(&b.right, None)?;
            return self.lower_in(&b.left, obj, b.span.start);
        }
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
            // `<` and friends compare time values; `===` compares the objects.
            let (l, r) = if matches!(op, CmpOp::Eq | CmpOp::Ne) {
                (l, r)
            } else {
                (self.date_value(l), self.date_value(r))
            };
            return self.lower_compare(op, l, r, b.span.start);
        }
        if b.operator == B::Addition && (Self::is_stringy(&l.ty) || Self::is_stringy(&r.ty)) {
            return self.str_concat(l, r);
        }
        // A date added to anything is the concatenation of its string.
        if b.operator == B::Addition && (self.is_date(&l) || self.is_date(&r)) {
            return self.str_concat(l, r);
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
            B::Exponential => BinOp::Pow,
            _ => bail!("{}: unsupported operator", self.loc(b.span.start)),
        };
        self.arith(op, l, r)
    }

    pub(crate) fn arith(&mut self, op: BinOp, l: Expr, r: Expr) -> R<Expr> {
        if let Some(big) = self.bigint_binary(op, &l, &r)? {
            return Ok(big);
        }
        let (l, r) = (self.as_num(l)?, self.as_num(r)?);
        // Constant folding, so `const UNITS = 10 / 729` is a constant rather than a global.
        if let (ExprKind::Num(a), ExprKind::Num(b)) = (&l.kind, &r.kind) {
            let v = match op {
                BinOp::Add => Some(a + b),
                BinOp::Sub => Some(a - b),
                BinOp::Mul => Some(a * b),
                BinOp::Div => Some(a / b),
                BinOp::Rem => Some(a % b),
                BinOp::Pow => Some(js_pow(*a, *b)),
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
        if let Some(big) = self.bigint_compare(op, &l, &r)? {
            return Ok(big);
        }
        let eq = matches!(op, CmpOp::Eq | CmpOp::Ne);
        if eq && (l.ty == Ty::Null || r.ty == Ty::Null) {
            let mut other = if l.ty == Ty::Null { r } else { l };
            // An engine handle is 0 for "no object" (`go.parent`, `GameObject.find`): the manifest
            // types them as handles, so `=== null` tests the 0.
            if matches!(other.ty, Ty::Handle(_)) {
                other.ty = other.ty.nullable();
            }
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
        // A union member against a `string` is compared as a string.
        let (lt, rt) = (l.ty.strip(), r.ty.strip());
        let (l, r) = if (lt == &Ty::Lit && rt == &Ty::Str)
            || (lt == &Ty::Str && rt == &Ty::Lit)
            || (!eq && lt == &Ty::Lit && rt == &Ty::Lit)
        {
            let (l, r) = (self.widen_lit(l), self.widen_lit(r));
            (l, r)
        } else {
            (l, r)
        };
        let (a, b) = (l.ty.strip().clone(), r.ty.strip().clone());
        if !eq && a == Ty::Str && b == Ty::Str {
            return Ok(self.str_order(op, l, r));
        }
        if a != b && !self.assignable(&a, &b) && !self.assignable(&b, &a) {
            bail!("{}: cannot compare {a} with {b}", self.loc(at));
        }
        if !eq && a != Ty::Num {
            bail!("{}: ordering comparison on {a}", self.loc(at));
        }
        Ok(mk(ExprKind::Cmp(op, Box::new(l), Box::new(r)), Ty::Bool))
    }

    /// An expression used as a condition (`if`, loops, `?:`, `!`): its truthiness. `&&` and `||`
    /// stay boolean here, whatever their operands, which is what a test wants.
    pub(crate) fn lower_cond(&mut self, e: &'a Expression<'a>) -> R<Expr> {
        match e {
            Expression::ParenthesizedExpression(p) => self.lower_cond(&p.expression),
            Expression::LogicalExpression(l) if l.operator != LogicalOperator::Coalesce => {
                let a = self.lower_cond(&l.left)?;
                let b = self.lower_cond(&l.right)?;
                let k = if l.operator == LogicalOperator::And {
                    ExprKind::And(Box::new(a), Box::new(b))
                } else {
                    ExprKind::Or(Box::new(a), Box::new(b))
                };
                Ok(mk(k, Ty::Bool))
            }
            Expression::UnaryExpression(u) if u.operator == UnaryOperator::LogicalNot => {
                let b = self.lower_cond(&u.argument)?;
                Ok(mk(ExprKind::Un(UnOp::Not, Box::new(b)), Ty::Bool))
            }
            _ => {
                let v = self.lower_expr(e, None)?;
                Ok(self.to_bool(v))
            }
        }
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
                // `x || 5`, `a && b` on values: the operand, as JavaScript gives it, when both
                // sides have one type. Mixed sides (`obj && obj.x > 0`) stay boolean, which is
                // what every condition wants.
                let (ak, bk) = (a.ty.strip().clone(), b.ty.strip().clone());
                let value_typed = ak != Ty::Bool
                    && (ak == bk
                        || (ak == Ty::Lit && bk == Ty::Str)
                        || (ak == Ty::Str && bk == Ty::Lit));
                if value_typed {
                    // A string literal beside a string is a string.
                    let (a, b) = (
                        if ak == Ty::Lit { self.widen_lit(a) } else { a },
                        if bk == Ty::Lit { self.widen_lit(b) } else { b },
                    );
                    let or = op == LogicalOperator::Or;
                    let mut ty = self.unify(&a.ty, &b.ty)?;
                    // `x || 5` with a nullable x is never null: a null x is falsy.
                    if or && !b.ty.is_nullable() {
                        ty = ty.strip().clone();
                    }
                    let tmp = self.new_local(a.ty.clone());
                    let t = self.var_expr(tmp);
                    let test = self.to_bool(t.clone());
                    let kept = if a.ty.is_nullable() && !ty.is_nullable() {
                        mk(ExprKind::Cast(Box::new(t)), ty.clone())
                    } else {
                        self.coerce(t, &ty)?
                    };
                    let other = self.coerce(b, &ty)?;
                    let (yes, no) = if or { (kept, other) } else { (other, kept) };
                    let cond = mk(
                        ExprKind::Cond(Box::new(test), Box::new(yes), Box::new(no)),
                        ty.clone(),
                    );
                    return Ok(mk(
                        ExprKind::Seq(vec![Stmt::Assign(tmp, a)], Box::new(cond)),
                        ty,
                    ));
                }
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
        // `{ [Weather.Rain]: "Rain" }` where a `Record<Weather, string>` (an array) is expected.
        if let Some(Ty::Arr(el)) = exp.map(|t| t.strip())
            && o.properties
                .iter()
                .all(|p| matches!(p, ObjectPropertyKind::ObjectProperty(p) if p.computed))
            && !o.properties.is_empty()
        {
            let el = (**el).clone();
            return self.lower_enum_record(o, el);
        }
        // `{ width: 10, ...other }` where an `IronStyle` (a list of `StyleEntry`) is expected.
        if let Some(Ty::Arr(el)) = exp.map(|t| t.strip())
            && let Ty::Obj(esid) = el.strip()
            && self.m.structs[*esid].name == "StyleEntry"
            && self.fids.contains_key("__se_num")
        {
            return self.lower_style_bag(o, exp.expect("checked").strip().clone());
        }
        // Lower every value first (with the field type as context when the struct is known), in
        // source order: a later spread or property wins.
        enum Item {
            Spread(Expr),
            Prop(String, Expr),
        }
        let mut items: Vec<Item> = Vec::new();
        for p in &o.properties {
            match p {
                ObjectPropertyKind::SpreadProperty(s) => {
                    let e = self.lower_expr(&s.argument, exp)?;
                    if !matches!(e.ty.strip(), Ty::Obj(_)) {
                        bail!(
                            "{}: spread of {}, not an object",
                            self.loc(s.span.start),
                            e.ty
                        )
                    }
                    items.push(Item::Spread(e));
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
                    let mut v = self.lower_expr(&p.value, want.as_ref())?;
                    // A property of a fresh object is a `string`, not the literal it was written as.
                    if exp_sid.is_none() && record.is_none() {
                        v = self.widen_lit(v);
                    }
                    items.push(Item::Prop(name, v));
                }
            }
        }
        let sid = match exp_sid {
            Some(s) => s,
            None => {
                // The fields in order of first appearance; a later value takes the type.
                let mut fields: Vec<Field> = Vec::new();
                let mut upsert = |name: String, ty: Ty, optional: bool| match fields
                    .iter_mut()
                    .find(|f| f.name == name)
                {
                    Some(f) => {
                        f.ty = ty;
                        f.optional = optional;
                    }
                    None => fields.push(Field { name, ty, optional }),
                };
                let mut only_spread = None;
                for it in &items {
                    match it {
                        Item::Spread(e) => {
                            let Ty::Obj(s) = e.ty.strip() else {
                                unreachable!("checked")
                            };
                            only_spread = Some(*s);
                            for f in &self.m.structs[*s].fields {
                                upsert(f.name.clone(), f.ty.clone(), f.optional);
                            }
                        }
                        Item::Prop(n, v) => upsert(
                            n.clone(),
                            record.cloned().unwrap_or_else(|| v.ty.clone()),
                            false,
                        ),
                    }
                }
                let single_spread = items.len() == 1 && matches!(items[0], Item::Spread(_));
                // `{ ...base, name }`: the type of `base` when it already has every property.
                let covers = only_spread.filter(|s| {
                    !self.class_of_sid.contains_key(s)
                        && items.iter().all(|it| match it {
                            Item::Spread(e) => matches!(e.ty.strip(), Ty::Obj(t) if t == s),
                            Item::Prop(n, v) => self.m.structs[*s].field(n).is_some_and(|i| {
                                self.m.structs[*s].fields[i].ty.strip() == v.ty.strip()
                            }),
                        })
                });
                match only_spread {
                    // `{ ...a }` is a copy with the type of `a`.
                    Some(s) if single_spread => s,
                    Some(_) if covers.is_some() => covers.expect("checked"),
                    _ => self.anon_struct("Anon", fields),
                }
            }
        };
        let tmp = self.new_local(Ty::Obj(sid));
        let t = self.var_expr(tmp);
        let mut stmts = vec![Stmt::Assign(tmp, mk(ExprKind::NewObj(sid), Ty::Obj(sid)))];
        for it in items {
            match it {
                Item::Spread(s) => {
                    let Ty::Obj(ssid) = s.ty.strip().clone() else {
                        unreachable!("checked")
                    };
                    let src = self.new_local(s.ty.clone());
                    stmts.push(Stmt::Assign(src, s));
                    for fi in 0..self.m.structs[ssid].fields.len() {
                        let (name, fty) = {
                            let f = &self.m.structs[ssid].fields[fi];
                            (f.name.clone(), f.ty.clone())
                        };
                        let Some(i) = self.m.structs[sid].field(&name) else {
                            continue;
                        };
                        let want = self.m.structs[sid].fields[i].ty.clone();
                        let read = mk(ExprKind::Field(Box::new(self.var_expr(src)), ssid, fi), fty);
                        let read = self.coerce(read, &want)?;
                        stmts.push(Stmt::StoreField(t.clone(), sid, i, read));
                    }
                }
                Item::Prop(name, v) => {
                    let Some(i) = self.m.structs[sid].field(&name) else {
                        bail!("{} has no field `{name}`", self.m.structs[sid].name)
                    };
                    let fty = self.m.structs[sid].fields[i].ty.clone();
                    let v = self.coerce(v, &fty)?;
                    stmts.push(Stmt::StoreField(t.clone(), sid, i, v));
                }
            }
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

    /// `{ [Weather.Rain]: v, ... }` as an array indexed by the enum values; keys nobody gave hold the
    /// element type's zero.
    fn lower_enum_record(&mut self, o: &'a ObjectExpression<'a>, el: Ty) -> R<Expr> {
        let mut slots: Vec<Option<Expr>> = Vec::new();
        for p in &o.properties {
            let ObjectPropertyKind::ObjectProperty(p) = p else {
                unreachable!("checked")
            };
            let Some(k) = p.key.as_expression() else {
                bail!("{}: a computed key", self.loc(p.span.start))
            };
            let key = self.lower_expr(k, None)?;
            let ExprKind::Num(n) = key.kind else {
                bail!(
                    "{}: a key of a record over an enum must be an enum member",
                    self.loc(p.span.start)
                )
            };
            let i = n as usize;
            let v = self.lower_expr(&p.value, Some(&el))?;
            let v = self.coerce(v, &el)?;
            if slots.len() <= i {
                slots.resize(i + 1, None);
            }
            slots[i] = Some(v);
        }
        let elems = slots
            .into_iter()
            .map(|s| Elem::One(s.unwrap_or_else(|| self.default_value(&el))))
            .collect();
        self.build_array(elems, el)
    }

    /// An object literal as an `IronStyle`: a `StyleEntry` per property, spreads concatenated, so
    /// a later entry of the same name wins when they are applied. A value that may be undefined
    /// makes an entry that applies nothing.
    fn lower_style_bag(&mut self, o: &'a ObjectExpression<'a>, aty: Ty) -> R<Expr> {
        let Ty::Arr(ety) = aty.clone() else {
            unreachable!("checked")
        };
        let mut elems = Vec::new();
        for p in &o.properties {
            match p {
                ObjectPropertyKind::SpreadProperty(s) => {
                    let e = self.lower_expr(&s.argument, Some(&aty))?;
                    let e = if e.ty.is_nullable() {
                        // Spreading `undefined` adds nothing.
                        let tmp = self.new_local(e.ty.clone());
                        let empty = self.build_array(vec![], (*ety).clone())?;
                        let isnull = mk(ExprKind::IsNull(Box::new(self.var_expr(tmp))), Ty::Bool);
                        let plain = mk(ExprKind::Cast(Box::new(self.var_expr(tmp))), aty.clone());
                        mk(
                            ExprKind::Seq(
                                vec![Stmt::Assign(tmp, e)],
                                Box::new(mk(
                                    ExprKind::Cond(
                                        Box::new(isnull),
                                        Box::new(empty),
                                        Box::new(plain),
                                    ),
                                    aty.clone(),
                                )),
                            ),
                            aty.clone(),
                        )
                    } else {
                        e
                    };
                    elems.push(Elem::Spread(self.coerce(e, &aty)?));
                }
                ObjectPropertyKind::ObjectProperty(p) => {
                    if p.computed || p.method {
                        bail!("{}: computed or method property", self.loc(p.span.start));
                    }
                    let name = self.key_name(&p.key)?;
                    let v = self.lower_expr(&p.value, None)?;
                    let v = self.widen_lit(v);
                    let (f, v) = match v.ty.clone() {
                        Ty::Null => continue,
                        Ty::Num => ("__se_num", v),
                        Ty::Str => ("__se_str", v),
                        Ty::Nullable(inner) if *inner == Ty::Num => ("__se_onum", v),
                        Ty::Nullable(inner) if *inner == Ty::Str => ("__se_ostr", v),
                        t if self.is_style_value(t.strip()) => (
                            if t.is_nullable() {
                                "__se_osv"
                            } else {
                                "__se_sv"
                            },
                            v,
                        ),
                        t => bail!(
                            "{}: style property `{name}` has type {t}; a style takes numbers and strings",
                            self.loc(p.span.start)
                        ),
                    };
                    let key = self.str_lit(&name);
                    elems.push(Elem::One(mk(
                        ExprKind::Call(self.fids[f], vec![key, v]),
                        (*ety).clone(),
                    )));
                }
            }
        }
        self.build_array(elems, (*ety).clone())
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
                let mut v = self.lower_expr(&sp.argument, hint.as_ref())?;
                v = self.match_view(v);
                // `[...set]`: the set's elements now.
                if let Some(snapshot) = self.collection_snapshot(v.clone()) {
                    v = snapshot;
                }
                // `[...iterable]`: the elements of its iterator.
                if self.is_iterable(&v.ty) {
                    v = self.iterable_to_array(v)?;
                }
                let vt = match v.ty.strip() {
                    Ty::Arr(e) => (**e).clone(),
                    Ty::Typed(_) => Ty::Num,
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
            // `["a", "b"]` is a `string[]` unless the context says otherwise.
            let v = if el.is_none() { self.widen_lit(v) } else { v };
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
        if let Expression::StaticMemberExpression(m) = &n.callee
            && matches!(&m.object, Expression::Identifier(o) if o.name == "Intl")
            && matches!(
                m.property.name.as_str(),
                "Collator"
                    | "NumberFormat"
                    | "DateTimeFormat"
                    | "PluralRules"
                    | "ListFormat"
                    | "RelativeTimeFormat"
                    | "Segmenter"
                    | "Locale"
                    | "DisplayNames"
            )
        {
            return self.lower_new_intl(m.property.name.as_str(), &n.arguments, n.span.start);
        }
        let Expression::Identifier(id) = &n.callee else {
            bail!("{}: unsupported `new`", self.loc(n.span.start))
        };
        match id.name.as_str() {
            // `new Array(n)`: n zeroed slots (a hole reads as undefined in JS; here it is 0 or null).
            "Array" => {
                let el = match exp.map(|t| t.strip()) {
                    Some(Ty::Arr(e)) => (**e).clone(),
                    _ => Ty::Num,
                };
                let ty = Ty::Arr(Box::new(el));
                let len = match n.arguments.first().and_then(|a| a.as_expression()) {
                    Some(e) => {
                        let v = self.lower_expr(e, Some(&Ty::Num))?;
                        self.as_num(v)?
                    }
                    None => num(0.0),
                };
                if n.arguments.len() > 1 {
                    bail!(
                        "{}: `new Array(a, b, ...)`: use an array literal",
                        self.loc(n.span.start)
                    )
                }
                let (nv, tmp) = (self.new_local(Ty::Num), self.new_local(ty.clone()));
                let stmts = vec![
                    Stmt::Assign(nv, len),
                    Stmt::Assign(
                        tmp,
                        mk(
                            ExprKind::Rt(RtFn::ArrNew, vec![self.var_expr(nv)]),
                            ty.clone(),
                        ),
                    ),
                    Stmt::Eval(mk(
                        ExprKind::Rt(RtFn::ArrSetLen, vec![self.var_expr(tmp), self.var_expr(nv)]),
                        Ty::Void,
                    )),
                ];
                Ok(mk(ExprKind::Seq(stmts, Box::new(self.var_expr(tmp))), ty))
            }
            name if crate::ir::TypedKind::from_name(name).is_some() => {
                let kind = crate::ir::TypedKind::from_name(name).expect("checked");
                self.lower_new_typed(kind, n)
            }
            "Map" | "Set" | "WeakMap" | "WeakSet" => self.lower_new_collection(&id.name, n, exp),
            "WeakRef"
                if self.lookup("WeakRef").is_none() && !self.classes.contains_key("WeakRef") =>
            {
                let at = self.loc(n.span.start);
                let Some(arg) = n.arguments.first().and_then(|a| a.as_expression()) else {
                    bail!("{at}: new WeakRef needs an object")
                };
                let v = self.lower_expr(arg, None)?;
                let Ty::Obj(sid) = v.ty.strip().clone() else {
                    bail!("{at}: a WeakRef holds an object")
                };
                let name = self.m.structs[sid].name.clone();
                self.call_helper(&format!("__newWeakRef${name}"), vec![v])
            }
            "RegExp" => self.lower_new_regexp(&n.arguments, n.span.start),
            "Date" if self.date_sid().is_some() => self.lower_new_date(&n.arguments, n.span.start),
            other if self.manifest.values.contains_key(other) => {
                self.new_value(other, &n.arguments, n.span.start)
            }
            other if self.is_creatable(other) => self.lower_new_handle(other, n),
            other if self.classes.contains_key(other) => {
                self.new_object(other, &n.arguments, n.span.start)
            }
            other if self.generic_classes.contains_key(other) => {
                let targs = self.new_targs(other, n, exp)?;
                let tparams = self.generic_classes[other].tparams.clone();
                let targs = self.complete_targs(other, &tparams, targs)?;
                let inst = self.instantiate_class(other, targs)?;
                self.new_object(&inst, &n.arguments, n.span.start)
            }
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
                // `Gizmos.color = ...`: the setter of a namespace property.
                if let Expression::Identifier(id) = &m.object
                    && self.lookup(id.name.as_str()).is_none()
                    && let Some(p) = self
                        .manifest
                        .ns_props
                        .get(id.name.as_str())
                        .and_then(|p| p.get(m.property.name.as_str()))
                        .cloned()
                {
                    let Some(setter) = p.setter else {
                        bail!("{}.{} is read only", id.name, m.property.name)
                    };
                    let ty = self.ext_ir(&setter.params[0].1);
                    return Ok(Place::NsProp(p.getter, setter, ty));
                }
                // `Class.count = ...`: a static field.
                if let Expression::Identifier(id) = &m.object
                    && self.lookup(id.name.as_str()).is_none()
                    && self.classes.contains_key(id.name.as_str())
                {
                    let key = format!("{}.{}", id.name, m.property.name);
                    if self.classes[id.name.as_str()]
                        .statics
                        .contains_key(&format!("set {}", m.property.name))
                    {
                        bail!("assignment to the static accessor `{key}` is not supported yet");
                    }
                    return match self.consts.get(&key) {
                        Some(ConstInfo::Global(g, ty, true)) => Ok(Place::Global(*g, ty.clone())),
                        Some(_) => bail!("assignment to the readonly static `{key}`"),
                        None => bail!("no static value `{key}`"),
                    };
                }
                let obj = self.lower_expr(&m.object, None)?;
                if m.property.name == "length" && matches!(obj.ty.strip(), Ty::Arr(_)) {
                    let tmp = self.new_local(obj.ty.clone());
                    pre.push(Stmt::Assign(tmp, obj));
                    return Ok(Place::ArrLen(self.var_expr(tmp)));
                }
                self.field_place(obj, m.property.name.as_str(), pre)
            }
            SimpleAssignmentTarget::PrivateFieldExpression(p) => {
                let obj = self.lower_expr(&p.object, None)?;
                self.field_place(obj, &classes::private_name(p.field.name.as_str()), pre)
            }
            SimpleAssignmentTarget::ComputedMemberExpression(m) => {
                let arr = self.lower_expr(&m.object, None)?;
                let idx = self.lower_expr(&m.expression, Some(&Ty::Num))?;
                let idx = self.as_num(idx)?;
                let ty = match arr.ty.strip() {
                    Ty::Typed(_) => Ty::Num,
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

    /// `obj.prop` as an assignment target: a field, or a property with a setter.
    fn field_place(&mut self, obj: Expr, prop: &str, pre: &mut Vec<Stmt>) -> R<Place> {
        if let Ty::Handle(class) = obj.ty.strip().clone() {
            let k = self.manifest.classes.get(&class);
            let getter = k.and_then(|k| k.getters.get(prop)).cloned();
            let setter = k.and_then(|k| k.setters.get(prop)).cloned();
            let Some(setter) = setter else {
                bail!("{class}.{prop} cannot be assigned")
            };
            let ty = self.ext_ir(&setter.params[1].1);
            let tmp = self.new_local(obj.ty.clone());
            pre.push(Stmt::Assign(tmp, obj));
            return Ok(Place::Ext(self.var_expr(tmp), getter, setter, ty));
        }
        if let Ty::Obj(sid) = obj.ty.strip().clone()
            && !self.has_field(sid, prop)
        {
            if let Some((_, setter)) = self.target_for(sid, Which::Setter, prop) {
                let getter = self.target_for(sid, Which::Getter, prop).map(|g| g.1);
                let ty = self.setter_value_ty(&setter);
                let tmp = self.new_local(obj.ty.clone());
                pre.push(Stmt::Assign(tmp, obj));
                return Ok(Place::Accessor(self.var_expr(tmp), getter, setter, ty));
            }
            // A field or setter only a subclass has: tsc narrowed the object, so use it as that class.
            let sub = self.downcast_for(sid, |s, c| {
                s.has_field(c, prop) || s.resolve_in(c, Which::Setter, prop).is_some()
            });
            if let Some(sub) = sub {
                return self.field_place(Self::downcast(obj, sub), prop, pre);
            }
        }
        let f = self.member_of(obj, prop)?;
        let ExprKind::Field(obj, sid, idx) = f.kind else {
            bail!("`{prop}` is not a field")
        };
        let tmp = self.new_local(obj.ty.clone());
        pre.push(Stmt::Assign(tmp, *obj));
        Ok(Place::Field(self.var_expr(tmp), sid, idx, f.ty))
    }

    pub(crate) fn place_ty(p: &Place) -> &Ty {
        static NUM: Ty = Ty::Num;
        match p {
            Place::ArrLen(_) => &NUM,
            Place::Accessor(_, _, _, t) | Place::Ext(_, _, _, t) | Place::NsProp(_, _, t) => t,
            Place::Var(_, t)
            | Place::Cell(_, t)
            | Place::Global(_, t)
            | Place::Field(_, _, _, t)
            | Place::Index(_, _, t) => t,
        }
    }

    pub(crate) fn place_read(&mut self, p: &Place) -> Expr {
        match p {
            Place::Var(v, _) => self.var_expr(*v),
            Place::Cell(c, t) => mk(ExprKind::CellGet(Box::new(c.clone())), t.clone()),
            Place::Global(g, t) => mk(ExprKind::GlobalVar(*g), t.clone()),
            Place::ArrLen(a) => mk(ExprKind::Len(Box::new(a.clone())), Ty::Num),
            Place::Accessor(o, g, _, t) => match g {
                Some(g) => Self::target_call(g, vec![o.clone()], t.clone()),
                None => mk(ExprKind::Null, t.clone()),
            },
            Place::Ext(o, g, _, t) => match g {
                Some(g) => self.emit_extern(g, vec![o.clone()]),
                None => mk(ExprKind::Null, t.clone()),
            },
            Place::NsProp(g, _, t) => match g {
                Some(g) => self.emit_extern(g, vec![]),
                None => mk(ExprKind::Null, t.clone()),
            },
            Place::Field(o, s, i, t) => mk(ExprKind::Field(Box::new(o.clone()), *s, *i), t.clone()),
            Place::Index(a, i, t) => mk(
                ExprKind::Index(Box::new(a.clone()), Box::new(i.clone())),
                t.clone(),
            ),
        }
    }

    pub(crate) fn place_write(&mut self, p: &Place, v: Expr) -> Stmt {
        match p {
            Place::Var(var, _) => Stmt::Assign(*var, v),
            Place::Cell(cell, _) => Stmt::CellSet(cell.clone(), v),
            Place::Global(g, _) => Stmt::StoreGlobal(*g, v),
            Place::Accessor(o, _, setter, _) => {
                Stmt::Eval(Self::target_call(setter, vec![o.clone(), v], Ty::Void))
            }
            Place::Ext(o, _, setter, _) => Stmt::Eval(self.emit_extern(setter, vec![o.clone(), v])),
            Place::NsProp(_, setter, _) => Stmt::Eval(self.emit_extern(setter, vec![v])),
            Place::ArrLen(a) => Stmt::Eval(mk(
                ExprKind::Rt(RtFn::ArrSetLen, vec![a.clone(), v]),
                Ty::Void,
            )),
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
        if matches!(a.operator, A::LogicalAnd | A::LogicalOr | A::LogicalNullish) {
            // `p ||= v` is `if (!p) p = v`: the target is evaluated once and v only when needed.
            let old = self.place_read(&place);
            let cond = match a.operator {
                A::LogicalAnd => self.to_bool(old),
                A::LogicalOr => {
                    let t = self.to_bool(old);
                    mk(ExprKind::Un(UnOp::Not, Box::new(t)), Ty::Bool)
                }
                _ if old.ty.is_nullable() => mk(ExprKind::IsNull(Box::new(old)), Ty::Bool),
                _ => mk(ExprKind::Bool(false), Ty::Bool),
            };
            let v = self.lower_expr(&a.right, Some(&pty))?;
            let v = self.coerce(v, &pty)?;
            pre.push(Stmt::If(cond, vec![self.place_write(&place, v)], vec![]));
            let value = self.place_read(&place);
            return Ok((pre, value));
        }
        let is_accessor = matches!(
            place,
            Place::Accessor(..) | Place::Ext(..) | Place::NsProp(..)
        );
        let new = if a.operator == A::Assign {
            let v = self.lower_expr(&a.right, Some(&pty))?;
            self.coerce(v, &pty)?
        } else if a.operator == A::Addition && Self::is_stringy(&pty) {
            let r = self.lower_expr(&a.right, None)?;
            let old = self.place_read(&place);
            let v = self.str_concat(old, r)?;
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
                A::Exponential => BinOp::Pow,
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
        if is_accessor {
            // The value of `o.x = v` is `v`, whatever the getter says (there may be none).
            let t = self.new_local(pty.clone());
            pre.push(Stmt::Assign(t, new));
            let tv = self.var_expr(t);
            pre.push(self.place_write(&place, tv.clone()));
            return Ok((pre, tv));
        }
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
        let new = if pty.strip() == &Ty::BigInt {
            self.bigint_step(old.clone(), delta > 0.0)?
        } else {
            self.arith(BinOp::Add, old.clone(), num(delta))?
        };
        let new = self.coerce(new, &pty)?;
        let value = if u.prefix {
            pre.push(self.place_write(&place, new));
            self.place_read(&place)
        } else {
            let tmp = self.new_local(if pty.strip() == &Ty::BigInt {
                Ty::BigInt
            } else {
                Ty::Num
            });
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
        // A parameter that may be null (`x?: T`) can be left out.
        let optional_tail =
            params.len() > args.len() && params[args.len()..].iter().all(|p| p.is_nullable());
        if args.len() > params.len() || (args.len() < params.len() && !optional_tail) {
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
        for p in &params[args.len()..] {
            out.push(mk(ExprKind::Null, p.clone()));
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
            let v = self.temporal_arg(v, p)?;
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
                } else if let Ty::Obj(sid) = pty.strip()
                    && !self.class_of_sid.contains_key(sid)
                    && self.m.structs[*sid]
                        .fields
                        .iter()
                        .all(|f| f.ty.is_nullable())
                {
                    // `<Btn />` for `props: { onPress?: () => void }`: an object with nothing set.
                    let sid = *sid;
                    let tmp = self.new_local(pty.clone());
                    let t = self.var_expr(tmp);
                    let mut stmts = vec![Stmt::Assign(tmp, mk(ExprKind::NewObj(sid), pty.clone()))];
                    for i in 0..self.m.structs[sid].fields.len() {
                        let fty = self.m.structs[sid].fields[i].ty.clone();
                        stmts.push(Stmt::StoreField(t.clone(), sid, i, mk(ExprKind::Null, fty)));
                    }
                    mk(ExprKind::Seq(stmts, Box::new(t)), pty.clone())
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
            // `obj[Symbol.iterator]()`.
            Expression::ComputedMemberExpression(m)
                if iter::well_known(&m.expression).is_some() =>
            {
                let name = iter::well_known(&m.expression).expect("checked");
                let recv = self.lower_expr(&m.object, None)?;
                self.call_method0(recv, name)
            }
            Expression::Identifier(id) => {
                let mut name = id.name.as_str();
                // `Symbol("x")` is the prelude's `__Symbol`.
                if name == "Symbol"
                    && self.lookup(name).is_none()
                    && self.fids.contains_key("__Symbol")
                {
                    name = "__Symbol";
                }
                if name == "__await" {
                    return self.lower_await(c);
                }
                if let Some(key) = name.strip_prefix("__asyncRet$") {
                    return self.lower_async_return(key, c);
                }
                if self.lookup(name).is_some()
                    || matches!(self.consts.get(name), Some(ConstInfo::Global(..)))
                {
                    let mut callee = self.lower_ident(name)?;
                    // A function that may be missing, tested by the caller (`if (f) f()`).
                    if callee.ty.is_nullable() && matches!(callee.ty.strip(), Ty::Fn(_)) {
                        let plain = callee.ty.strip().clone();
                        callee = mk(ExprKind::Cast(Box::new(callee)), plain);
                    }
                    let Ty::Fn(sig) = callee.ty.clone() else {
                        bail!("`{name}` is not callable")
                    };
                    let args = self.lower_args(&c.arguments, &sig.params, name)?;
                    return Ok(mk(
                        ExprKind::CallClosure(Box::new(callee), args),
                        sig.ret.clone(),
                    ));
                }
                if self.lookup(name).is_none() && self.generic_fns.contains_key(name) {
                    return self.lower_generic_call(name, c, None);
                }
                if !self.fids.contains_key(name)
                    && let Some(e) = self.lower_string_global(name, c)?
                {
                    return Ok(e);
                }
                let Some(&fid) = self.fids.get(name) else {
                    bail!(
                        "{}: call to unknown function `{name}`",
                        self.loc(c.span.start)
                    )
                };
                let _ = fid;
                self.call_declared(name, &c.arguments, None)
            }
            Expression::Super(_) => self.lower_super_call(c),
            Expression::PrivateFieldExpression(p) => {
                let recv = self.lower_expr(&p.object, None)?;
                self.lower_method(recv, &classes::private_name(p.field.name.as_str()), c)
            }
            Expression::StaticMemberExpression(m) => {
                let method = m.property.name.as_str();
                if let Expression::Identifier(id) = &m.object
                    && id.name == "BigInt"
                    && self.lookup("BigInt").is_none()
                {
                    return self.lower_bigint_static(method, c);
                }
                if let Expression::Identifier(id) = &m.object
                    && id.name.starts_with("Temporal_")
                    && method == "from"
                    && self.classes.contains_key(id.name.as_str())
                {
                    return self.lower_temporal_from(id.name.as_str(), c);
                }
                if let Expression::Identifier(id) = &m.object
                    && id.name == "Intl"
                    && method == "getCanonicalLocales"
                    && self.fids.contains_key("__canonLocales1")
                {
                    let Some(arg) = c.arguments.first().and_then(|a| a.as_expression()) else {
                        bail!("Intl.getCanonicalLocales needs locales")
                    };
                    let v = self.lower_expr(arg, None)?;
                    let name = if matches!(v.ty.strip(), Ty::Arr(_)) {
                        "__canonLocalesN"
                    } else {
                        "__canonLocales1"
                    };
                    let v = self.widen_lit(v);
                    return self.call_helper(name, vec![v]);
                }
                if let Expression::Identifier(id) = &m.object
                    && id.name == "Promise"
                    && self.lookup("Promise").is_none()
                {
                    return self.lower_promise_static(method, c);
                }
                if let Expression::Identifier(id) = &m.object
                    && id.name == "Symbol"
                    && self.lookup("Symbol").is_none()
                    && matches!(method, "for" | "keyFor")
                    && self.fids.contains_key("__symbolFor")
                {
                    let f = if method == "for" {
                        "__symbolFor"
                    } else {
                        "__symbolKeyFor"
                    };
                    return self.call_declared(f, &c.arguments, None);
                }
                if let Expression::Identifier(id) = &m.object
                    && id.name == "Math"
                    && self.lookup("Math").is_none()
                {
                    return self.lower_math(method, c);
                }
                if let Expression::Identifier(id) = &m.object
                    && let Some(kind) = crate::ir::TypedKind::from_name(id.name.as_str())
                    && self.lookup(id.name.as_str()).is_none()
                {
                    return self.lower_typed_static(kind, method, c);
                }
                if let Expression::Identifier(id) = &m.object
                    && id.name == "Date"
                    && self.lookup("Date").is_none()
                    && self.date_sid().is_some()
                {
                    return self.lower_date_static(method, c);
                }
                if let Expression::Identifier(id) = &m.object
                    && id.name == "String"
                    && self.lookup("String").is_none()
                {
                    return self.lower_string_static(method, c);
                }
                if let Expression::Identifier(id) = &m.object
                    && id.name == "Number"
                    && self.lookup("Number").is_none()
                {
                    return self.lower_number_static(method, c);
                }
                if let Expression::Identifier(id) = &m.object
                    && id.name == "JSON"
                    && self.lookup("JSON").is_none()
                {
                    return match method {
                        "stringify" => self.lower_json_stringify(c),
                        "parse" => self.lower_json_parse(c, None),
                        m => bail!("{}: JSON.{m} is not supported", self.loc(c.span.start)),
                    };
                }
                if let Expression::Identifier(id) = &m.object
                    && id.name == "Object"
                    && self.lookup("Object").is_none()
                {
                    return self.lower_object_static(method, c);
                }
                if let Expression::Identifier(id) = &m.object
                    && id.name == "console"
                    && self.lookup("console").is_none()
                {
                    return self.lower_console(method, c);
                }
                if let Expression::Identifier(id) = &m.object
                    && id.name == "Array"
                    && self.lookup("Array").is_none()
                {
                    return self.lower_array_static(method, c);
                }
                if matches!(m.object, Expression::Super(_)) {
                    return self.lower_super_method(method, c);
                }
                if let Expression::Identifier(id) = &m.object
                    && id.name == "GameObject"
                    && method == "findObjectOfType"
                    && self.lookup("GameObject").is_none()
                    && let Some(e) = self.lower_find_object_of_type(c)?
                {
                    return Ok(e);
                }
                if let Expression::Identifier(id) = &m.object
                    && id.name == "GameObject"
                    && matches!(method, "instantiate" | "dontDestroyOnLoad")
                    && self.lookup("GameObject").is_none()
                    && let Some(e) = self.lower_scene_call(method, c)?
                {
                    return Ok(e);
                }
                if let Expression::Identifier(id) = &m.object
                    && method == "create"
                    && self.lookup(id.name.as_str()).is_none()
                    && self.is_create_with_options(id.name.as_str(), c)
                {
                    return self.lower_new_handle_args(
                        id.name.as_str(),
                        &c.arguments,
                        c.span.start,
                    );
                }
                if let Expression::Identifier(id) = &m.object
                    && self.lookup(id.name.as_str()).is_none()
                    && self.manifest.namespaces.contains_key(id.name.as_str())
                {
                    return self.lower_extern_call(id.name.as_str(), method, c);
                }
                if let Expression::Identifier(id) = &m.object
                    && self.lookup(id.name.as_str()).is_none()
                    && let Some(f) = self
                        .manifest
                        .classes
                        .get(id.name.as_str())
                        .and_then(|k| k.statics.get(method))
                        .cloned()
                {
                    if method == "create" && self.is_create_with_options(id.name.as_str(), c) {
                        return self.lower_new_handle_args(
                            id.name.as_str(),
                            &c.arguments,
                            c.span.start,
                        );
                    }
                    return self.call_ext_fn(f, None, &format!("{}.{method}", id.name), c);
                }
                if let Expression::Identifier(id) = &m.object
                    && self.lookup(id.name.as_str()).is_none()
                    && let Some(info) = self.classes.get(id.name.as_str())
                {
                    let Some(f) = info.statics.get(method).cloned() else {
                        bail!("class {} has no static method `{method}`", id.name)
                    };
                    return self.call_declared(&f, &c.arguments, None);
                }
                let mut recv = self.lower_expr(&m.object, None)?;
                if m.optional {
                    recv = self.chain_guard(recv)?;
                }
                // A literal (or a constant of one) as the receiver of a string method.
                let recv = self.widen_lit(recv);
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
            ("ceil", 1) => un(UnOp::Ceil, it.next().unwrap()),
            ("trunc", 1) => un(UnOp::Trunc, it.next().unwrap()),
            ("round", 1) => un(UnOp::Round, it.next().unwrap()),
            ("fround", 1) => un(UnOp::Fround, it.next().unwrap()),
            ("clz32", 1) => un(UnOp::Clz32, it.next().unwrap()),
            ("tan", 1) => un(UnOp::Libm("ts_rt_tan"), it.next().unwrap()),
            ("asin", 1) => un(UnOp::Libm("ts_rt_asin"), it.next().unwrap()),
            ("acos", 1) => un(UnOp::Libm("ts_rt_acos"), it.next().unwrap()),
            ("atan", 1) => un(UnOp::Libm("ts_rt_atan"), it.next().unwrap()),
            ("exp", 1) => un(UnOp::Libm("ts_rt_exp"), it.next().unwrap()),
            ("log", 1) => un(UnOp::Libm("ts_rt_log"), it.next().unwrap()),
            ("log2", 1) => un(UnOp::Libm("ts_rt_log2"), it.next().unwrap()),
            ("log10", 1) => un(UnOp::Libm("ts_rt_log10"), it.next().unwrap()),
            ("cbrt", 1) => un(UnOp::Libm("ts_rt_cbrt"), it.next().unwrap()),
            ("sinh", 1) => un(UnOp::Libm("ts_rt_sinh"), it.next().unwrap()),
            ("cosh", 1) => un(UnOp::Libm("ts_rt_cosh"), it.next().unwrap()),
            ("tanh", 1) => un(UnOp::Libm("ts_rt_tanh"), it.next().unwrap()),
            ("asinh", 1) => un(UnOp::Libm("ts_rt_asinh"), it.next().unwrap()),
            ("acosh", 1) => un(UnOp::Libm("ts_rt_acosh"), it.next().unwrap()),
            ("atanh", 1) => un(UnOp::Libm("ts_rt_atanh"), it.next().unwrap()),
            ("expm1", 1) => un(UnOp::Libm("ts_rt_expm1"), it.next().unwrap()),
            ("log1p", 1) => un(UnOp::Libm("ts_rt_log1p"), it.next().unwrap()),
            ("pow", 2) => bin(BinOp::Pow, it.next().unwrap(), it.next().unwrap()),
            ("random", 0) => mk(ExprKind::Rt(RtFn::Random, vec![]), Ty::Num),
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
