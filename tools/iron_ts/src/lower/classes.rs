//! Plain classes: fields, constructors, methods, accessors, statics, private members.
//!
//! A class is a struct (fields in declaration order). A method is an ordinary function whose
//! first parameter is `this`; a constructor is a function that allocates the object, stores the
//! parameter properties and field initialisers, runs the body and returns the object. Calls are
//! direct: with no inheritance yet the target of `a.m()` is always known. See
//! `docs/research/iron-ts-classes.md`.

use super::expr::mk;
use super::*;

/// What the lowering knows about one class.
#[derive(Clone, Default)]
pub(crate) struct ClassInfo {
    /// JavaScript method name -> the function that implements it.
    pub methods: HashMap<String, String>,
    pub getters: HashMap<String, String>,
    pub setters: HashMap<String, String>,
    pub statics: HashMap<String, String>,
    pub ctor: String,
}

/// The struct field that holds `#name`.
pub(crate) fn private_name(name: &str) -> String {
    format!("_p_{name}")
}

impl<'a> Lowerer<'a> {
    pub(crate) fn declare_class_name(&mut self, c: &'a Class<'a>) -> R<()> {
        let Some(id) = &c.id else {
            bail!("a class expression is not supported")
        };
        self.class_decls.insert(id.name.to_string(), c);
        Ok(())
    }

    fn member_key(&self, k: &PropertyKey<'a>) -> R<String> {
        match k {
            PropertyKey::PrivateIdentifier(p) => Ok(private_name(p.name.as_str())),
            other => self.key_name(other),
        }
    }

    /// A field's type: its annotation, or the type of a literal initialiser.
    fn field_ty(
        &mut self,
        ann: Option<&'a TSTypeAnnotation<'a>>,
        optional: bool,
        init: Option<&'a Expression<'a>>,
        what: &str,
    ) -> R<Ty> {
        let ty = match ann {
            Some(a) => self.ty(&a.type_annotation, what)?,
            None => match init {
                Some(Expression::NumericLiteral(_)) => Ty::Num,
                Some(Expression::BooleanLiteral(_)) => Ty::Bool,
                Some(Expression::UnaryExpression(u))
                    if u.operator == UnaryOperator::UnaryNegation
                        && matches!(u.argument, Expression::NumericLiteral(_)) =>
                {
                    Ty::Num
                }
                // `field = new Other()` is an `Other`.
                Some(Expression::NewExpression(n))
                    if matches!(&n.callee, Expression::Identifier(id)
                        if self.class_decls.contains_key(id.name.as_str())) =>
                {
                    let Expression::Identifier(id) = &n.callee else {
                        unreachable!("matched above")
                    };
                    self.named_type(id.name.as_str())?
                }
                _ => bail!(
                    "{what}: needs a type annotation (only a literal or `new Class()` is inferred)"
                ),
            },
        };
        Ok(if optional { ty.nullable() } else { ty })
    }

    /// The struct of class `name`, built from its declared fields on first use.
    pub(crate) fn class_type(&mut self, name: &str) -> R<Ty> {
        let c = self.class_decls[name];
        if c.heritage.is_some() {
            bail!("class {name}: `extends` is not supported yet (docs/research/iron-ts-classes.md)")
        }
        if c.r#abstract || c.type_parameters.is_some() || !c.decorators.is_empty() {
            bail!("class {name}: abstract, generic and decorated classes are not supported yet")
        }
        let sid = self.new_struct(name, vec![]);
        self.named.insert(name.to_string(), Ty::Obj(sid));
        self.class_of_sid.insert(sid, name.to_string());
        self.classes.insert(name.to_string(), ClassInfo::default());
        let mut fields: Vec<Field> = Vec::new();
        for el in &c.body.body {
            match el {
                ClassElement::PropertyDefinition(p) => {
                    if !p.decorators.is_empty() {
                        bail!("class {name}: decorators are not supported yet")
                    }
                    if p.r#static {
                        continue;
                    }
                    if p.computed {
                        bail!("class {name}: computed property name")
                    }
                    let key = self.member_key(&p.key)?;
                    let what = format!("{name}.{key}");
                    let ty = self.field_ty(
                        p.type_annotation.as_deref(),
                        p.optional,
                        p.value.as_ref(),
                        &what,
                    )?;
                    fields.push(Field { name: key, ty });
                }
                ClassElement::MethodDefinition(m)
                    if m.kind == MethodDefinitionKind::Constructor =>
                {
                    for p in &m.value.params.items {
                        if p.accessibility.is_none() && !p.readonly {
                            continue;
                        }
                        let BindingPattern::BindingIdentifier(b) = &p.pattern else {
                            bail!("class {name}: a parameter property must be a plain name")
                        };
                        let what = format!("{name}.{}", b.name);
                        let ty = self.field_ty(
                            p.type_annotation.as_deref(),
                            p.optional,
                            p.initializer.as_deref(),
                            &what,
                        )?;
                        fields.push(Field {
                            name: b.name.to_string(),
                            ty,
                        });
                    }
                }
                ClassElement::StaticBlock(_) | ClassElement::AccessorProperty(_) => {
                    bail!("class {name}: static blocks and `accessor` fields are not supported")
                }
                _ => {}
            }
        }
        self.m.structs[sid].fields = fields;
        Ok(Ty::Obj(sid))
    }

    /// An empty parameter list and body, for a class without a constructor.
    fn empty_function(&mut self) -> (&'a FormalParameters<'a>, &'a FunctionBody<'a>) {
        if let Some(e) = self.empty_fn {
            return e;
        }
        // Parsed once from a tiny source and leaked, so the references live as long as any
        // program does.
        let alloc: &'static Allocator = Box::leak(Box::new(Allocator::default()));
        let ret = Parser::new(alloc, "(function () {})", SourceType::ts()).parse();
        let program: &'static Program<'static> = Box::leak(Box::new(ret.program));
        let Statement::ExpressionStatement(es) = &program.body[0] else {
            unreachable!("the source is one expression statement")
        };
        let mut e = &es.expression;
        while let Expression::ParenthesizedExpression(p) = e {
            e = &p.expression;
        }
        let Expression::FunctionExpression(f) = e else {
            unreachable!("the source is a function expression")
        };
        let r = (&*f.params, &**f.body.as_ref().expect("has a body"));
        // SAFETY: the data is leaked ('static) and immutable; shortening its lifetime is sound.
        let r: (&'a FormalParameters<'a>, &'a FunctionBody<'a>) = unsafe { std::mem::transmute(r) };
        self.empty_fn = Some(r);
        r
    }

    /// Declares the constructor, methods and accessors of a class (in the main collection pass).
    pub(crate) fn collect_class(&mut self, c: &'a Class<'a>) -> R<()> {
        let name = c.id.as_ref().expect("checked").name.to_string();
        let Ty::Obj(sid) = self.named_type(&name)? else {
            unreachable!("a class is a struct")
        };
        let mut info = self.classes[&name].clone();
        let mut has_ctor = false;
        for el in &c.body.body {
            let ClassElement::MethodDefinition(m) = el else {
                continue;
            };
            if !m.decorators.is_empty() {
                bail!("class {name}: decorators are not supported yet")
            }
            if m.computed {
                bail!("class {name}: computed method name")
            }
            let key = self.member_key(&m.key)?;
            let f: &'a Function<'a> = &m.value;
            // An overload signature has no body.
            let Some(body) = &f.body else { continue };
            let ret = f.return_type.as_ref().map(|r| &r.type_annotation);
            let mut src = FnSrc {
                params: &f.params,
                ret,
                body: FnBody::Block(body),
                exported: false,
                kind: FnKind::Method(sid),
                ret_ty: None,
            };
            match (m.kind, m.r#static) {
                (MethodDefinitionKind::Constructor, _) => {
                    let fname = format!("{name}$new");
                    src.ret = None;
                    src.ret_ty = Some(Ty::Obj(sid));
                    src.kind = FnKind::Ctor(sid);
                    self.declare_fn(&fname, src)?;
                    info.ctor = fname;
                    has_ctor = true;
                }
                (MethodDefinitionKind::Method, false) => {
                    let fname = format!("{name}.{key}");
                    self.declare_fn(&fname, src)?;
                    info.methods.insert(key, fname);
                }
                (MethodDefinitionKind::Method, true) => {
                    let fname = format!("{name}.static {key}");
                    src.kind = FnKind::Free;
                    self.declare_fn(&fname, src)?;
                    info.statics.insert(key, fname);
                }
                (MethodDefinitionKind::Get, false) => {
                    let fname = format!("{name}.get {key}");
                    self.declare_fn(&fname, src)?;
                    info.getters.insert(key, fname);
                }
                (MethodDefinitionKind::Set, false) => {
                    let fname = format!("{name}.set {key}");
                    self.declare_fn(&fname, src)?;
                    info.setters.insert(key, fname);
                }
                _ => bail!("class {name}: static accessors are not supported"),
            }
        }
        if !has_ctor {
            let (params, body) = self.empty_function();
            let fname = format!("{name}$new");
            let src = FnSrc {
                params,
                ret: None,
                body: FnBody::Block(body),
                exported: false,
                kind: FnKind::Ctor(sid),
                ret_ty: Some(Ty::Obj(sid)),
            };
            self.declare_fn(&fname, src)?;
            info.ctor = fname;
        }
        self.classes.insert(name, info);
        Ok(())
    }

    /// Static fields, in source order, into the module's init function.
    pub(crate) fn lower_class_statics(
        &mut self,
        c: &'a Class<'a>,
        init_body: &mut Vec<Stmt>,
    ) -> R<()> {
        let name = c.id.as_ref().expect("checked").name.to_string();
        for el in &c.body.body {
            let ClassElement::PropertyDefinition(p) = el else {
                continue;
            };
            if !p.r#static {
                continue;
            }
            let key = self.member_key(&p.key)?;
            let what = format!("{name}.{key}");
            let ann = match &p.type_annotation {
                Some(a) => Some(self.ty(&a.type_annotation, &what)?),
                None => None,
            };
            let mut e = match &p.value {
                Some(v) => {
                    let e = self.lower_expr(v, ann.as_ref())?;
                    match &ann {
                        Some(t) => self.coerce(e, t)?,
                        None => e,
                    }
                }
                None => match &ann {
                    Some(t) => self.default_value(t),
                    None => bail!("{what}: needs a type or a value"),
                },
            };
            if p.optional {
                let nullable = e.ty.clone().nullable();
                e = self.coerce(e, &nullable)?;
            }
            let full = format!("{name}.{key}");
            if p.readonly
                && matches!(
                    e.kind,
                    ExprKind::Num(_) | ExprKind::Bool(_) | ExprKind::Tag(_)
                )
            {
                self.consts.insert(full, ConstInfo::Inline(e));
                continue;
            }
            if matches!(e.ty, Ty::Null) {
                bail!("{what}: a static initialised with null needs a type annotation")
            }
            let g = self.m.globals.len();
            self.m.globals.push(Global {
                name: full.clone(),
                ty: e.ty.clone(),
            });
            self.consts
                .insert(full, ConstInfo::Global(g, e.ty.clone(), !p.readonly));
            init_body.push(Stmt::StoreGlobal(g, e));
        }
        Ok(())
    }

    /// The start of a constructor body: allocate `this`, null the nullable fields, store the
    /// parameter properties, then run the field initialisers in source order.
    pub(crate) fn ctor_prologue(
        &mut self,
        class_name: &str,
        sid: StructId,
        params: &'a FormalParameters<'a>,
        pro: &mut Vec<Stmt>,
    ) -> R<()> {
        let this_ty = Ty::Obj(sid);
        let this = self.new_local(this_ty.clone());
        pro.push(Stmt::Assign(
            this,
            mk(ExprKind::NewObj(sid), this_ty.clone()),
        ));
        for i in 0..self.m.structs[sid].fields.len() {
            let fty = self.m.structs[sid].fields[i].ty.clone();
            if fty.is_nullable() {
                let t = self.var_expr(this);
                pro.push(Stmt::StoreField(t, sid, i, mk(ExprKind::Null, fty)));
            }
        }
        self.bind_var("this", this, pro);
        let this_e = |s: &mut Self| {
            let b = s.lookup("this").expect("bound above");
            s.read_binding(&b)
        };
        for p in &params.items {
            if p.accessibility.is_none() && !p.readonly {
                continue;
            }
            let BindingPattern::BindingIdentifier(b) = &p.pattern else {
                continue;
            };
            let idx = self.m.structs[sid]
                .field(b.name.as_str())
                .expect("declared as a field");
            let value = {
                let bind = self.lookup(b.name.as_str()).expect("a bound parameter");
                self.read_binding(&bind)
            };
            let fty = self.m.structs[sid].fields[idx].ty.clone();
            let value = self.coerce(value, &fty)?;
            let t = this_e(self);
            pro.push(Stmt::StoreField(t, sid, idx, value));
        }
        let c = self.class_decls[class_name];
        for el in &c.body.body {
            let ClassElement::PropertyDefinition(p) = el else {
                continue;
            };
            let (false, Some(init)) = (p.r#static, &p.value) else {
                continue;
            };
            let key = self.member_key(&p.key)?;
            let idx = self.m.structs[sid]
                .field(&key)
                .expect("declared as a field");
            let fty = self.m.structs[sid].fields[idx].ty.clone();
            let v = self.lower_expr(init, Some(&fty))?;
            let v = self.coerce(v, &fty)?;
            let t = this_e(self);
            pro.push(Stmt::StoreField(t, sid, idx, v));
        }
        Ok(())
    }

    /// The method `m` of the class a value belongs to, if there is one.
    pub(crate) fn class_method(&self, sid: StructId, m: &str) -> Option<String> {
        let cn = self.class_of_sid.get(&sid)?;
        self.classes.get(cn)?.methods.get(m).cloned()
    }

    pub(crate) fn class_accessor(&self, sid: StructId, m: &str, set: bool) -> Option<String> {
        let cn = self.class_of_sid.get(&sid)?;
        let info = self.classes.get(cn)?;
        if set { &info.setters } else { &info.getters }
            .get(m)
            .cloned()
    }

    /// Calls the declared function `name` (a function, method, static method or constructor).
    /// A receiver, if any, becomes the first argument and is evaluated before the others.
    pub(crate) fn call_declared(
        &mut self,
        name: &str,
        args: &'a [Argument<'a>],
        recv: Option<Expr>,
    ) -> R<Expr> {
        let fid = self.fids[name];
        self.ensure_lowered(name)?;
        let (params, ret) = self.fsig[fid].clone();
        let Some(ret) = ret else {
            bail!("{name}: return type unknown")
        };
        let skip = recv.is_some() as usize;
        let (mut pre, mut vals) = self.lower_fn_args(name, args, &params[skip..])?;
        if let Some(r) = recv {
            let r = if pre.is_empty() {
                r
            } else {
                // The receiver is evaluated before any argument.
                let t = self.new_local(r.ty.clone());
                pre.insert(0, Stmt::Assign(t, r));
                self.var_expr(t)
            };
            vals.insert(0, r);
        }
        let call = mk(ExprKind::Call(fid, vals), ret.clone());
        Ok(if pre.is_empty() {
            call
        } else {
            mk(ExprKind::Seq(pre, Box::new(call)), ret)
        })
    }

    /// A call of a getter (`obj.x`).
    pub(crate) fn call_getter(&mut self, fname: &str, obj: Expr) -> R<Expr> {
        let fid = self.fids[fname];
        self.ensure_lowered(fname)?;
        let Some(ret) = self.fsig[fid].1.clone() else {
            bail!("{fname}: return type unknown")
        };
        Ok(mk(ExprKind::Call(fid, vec![obj]), ret))
    }

    /// The type a property assignment through a setter takes.
    pub(crate) fn setter_value_ty(&mut self, fname: &str) -> R<Ty> {
        let fid = self.fids[fname];
        Ok(self.fsig[fid].0[1].clone())
    }
}
