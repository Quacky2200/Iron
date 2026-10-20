//! Classes: fields, constructors, methods, accessors, statics, private members, inheritance.
//!
//! A class is a struct whose fields start with its base class's fields (the prefix layout), so a
//! pointer to a subclass object is also a valid pointer to a base object. A method is an ordinary
//! function whose first parameter is `this`. A constructor is the function `C$init(this, ...)`:
//! it runs the base class's `$init` (`super(...)`), stores the parameter properties and field
//! initialisers, then the body. `new C(a)` allocates the object and calls `C$init`.
//!
//! The compiler sees the whole program, so `a.m()` is a direct call whenever one implementation
//! of `m` is reachable from the static type of `a` (that class and its subclasses). When several
//! are, the call goes through a table indexed by the object's struct id (`CallVirtual`).
//! See `docs/research/iron-ts-classes.md`.

use super::expr::mk;
use super::*;

/// What the lowering knows about one class.
#[derive(Clone, Default)]
pub(crate) struct ClassInfo {
    pub parent: Option<String>,
    pub is_abstract: bool,
    /// Fields declared by this class (the struct also has the parent's, first).
    pub own_fields: Vec<Field>,
    /// JavaScript method name -> the function that implements it.
    pub methods: HashMap<String, String>,
    pub getters: HashMap<String, String>,
    pub setters: HashMap<String, String>,
    pub statics: HashMap<String, String>,
    /// The `$init` function.
    pub ctor: String,
    /// Fields marked `@property` here (the table of a class also has its base classes').
    pub property_fields: Vec<String>,
    pub collected: bool,
}

/// Which table of members a lookup is about.
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Which {
    Method,
    Getter,
    Setter,
}

/// How a method, getter or setter is called.
#[derive(Clone)]
pub(crate) enum Target {
    Direct(FuncId),
    /// Through the table with this id; the function is the statically resolved one (for the signature).
    Virtual(usize, FuncId),
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
        if c.type_parameters.is_some() {
            self.generic_classes.insert(
                id.name.to_string(),
                generics::GenericClass {
                    class: c,
                    tparams: generics::tparam_list(c.type_parameters.as_deref()),
                },
            );
            return Ok(());
        }
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
    pub(crate) fn field_ty(
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
                // `name = "x"` and `` name = `x${y}` `` are strings.
                Some(Expression::StringLiteral(_) | Expression::TemplateLiteral(_)) => Ty::Str,
                Some(Expression::UnaryExpression(u))
                    if u.operator == UnaryOperator::UnaryNegation
                        && matches!(u.argument, Expression::NumericLiteral(_)) =>
                {
                    Ty::Num
                }
                // `field = new Map<K, V>()`, `new Set<K>()`.
                Some(Expression::NewExpression(n))
                    if matches!(&n.callee, Expression::Identifier(id)
                        if matches!(id.name.as_str(), "Map" | "Set" | "WeakMap" | "WeakSet"))
                        && n.type_arguments.is_some() =>
                {
                    let Expression::Identifier(id) = &n.callee else {
                        unreachable!("matched above")
                    };
                    let e = self.lower_new_collection(&id.name, n, None)?;
                    e.ty.clone()
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
                Some(e) => match self.guess_ty(e, 0)? {
                    Some(t) => t,
                    None => bail!(
                        "{what}: needs a type annotation (only a literal, arithmetic, a top-level constant, an object literal of those, or `new Class()` is inferred)"
                    ),
                },
                None => bail!("{what}: needs a type or a value"),
            },
        };
        Ok(if optional { ty.nullable() } else { ty })
    }

    /// The type of a field initialiser, found from the syntax alone (the class is declared before
    /// any code is lowered): literals, arithmetic, a top-level `const` without an annotation,
    /// `a.b` of an object literal, and object literals of those.
    fn guess_ty(&mut self, e: &'a Expression<'a>, depth: u8) -> R<Option<Ty>> {
        if depth > 8 {
            return Ok(None);
        }
        Ok(match e {
            Expression::NumericLiteral(_) => Some(Ty::Num),
            Expression::BooleanLiteral(_) => Some(Ty::Bool),
            Expression::StringLiteral(_) | Expression::TemplateLiteral(_) => Some(Ty::Str),
            Expression::ParenthesizedExpression(p) => self.guess_ty(&p.expression, depth + 1)?,
            Expression::UnaryExpression(u) => match u.operator {
                UnaryOperator::UnaryNegation
                | UnaryOperator::UnaryPlus
                | UnaryOperator::BitwiseNot => Some(Ty::Num),
                UnaryOperator::LogicalNot => Some(Ty::Bool),
                _ => None,
            },
            Expression::BinaryExpression(b) => match b.operator {
                BinaryOperator::Subtraction
                | BinaryOperator::Multiplication
                | BinaryOperator::Division
                | BinaryOperator::Remainder
                | BinaryOperator::Exponential
                | BinaryOperator::ShiftLeft
                | BinaryOperator::ShiftRight
                | BinaryOperator::ShiftRightZeroFill
                | BinaryOperator::BitwiseAnd
                | BinaryOperator::BitwiseOR
                | BinaryOperator::BitwiseXOR => Some(Ty::Num),
                BinaryOperator::Addition => {
                    let l = self.guess_ty(&b.left, depth + 1)?;
                    let r = self.guess_ty(&b.right, depth + 1)?;
                    match (l, r) {
                        (Some(Ty::Str), _) | (_, Some(Ty::Str)) => Some(Ty::Str),
                        (Some(Ty::Num), Some(Ty::Num)) => Some(Ty::Num),
                        _ => None,
                    }
                }
                BinaryOperator::Equality
                | BinaryOperator::Inequality
                | BinaryOperator::StrictEquality
                | BinaryOperator::StrictInequality
                | BinaryOperator::LessThan
                | BinaryOperator::LessEqualThan
                | BinaryOperator::GreaterThan
                | BinaryOperator::GreaterEqualThan => Some(Ty::Bool),
                _ => None,
            },
            Expression::Identifier(id) => match self.const_inits.get(id.name.as_str()).copied() {
                Some(init) => self.guess_ty(init, depth + 1)?,
                None => None,
            },
            // `new Tween<number>(..)`: a generic class with its type arguments written.
            Expression::NewExpression(n)
                if n.type_arguments.is_some()
                    && matches!(&n.callee, Expression::Identifier(id)
                        if self.generic_classes.contains_key(id.name.as_str())) =>
            {
                let Expression::Identifier(id) = &n.callee else {
                    unreachable!("matched above")
                };
                let mut targs = Vec::new();
                for t in &n.type_arguments.as_ref().expect("checked").params {
                    targs.push(self.ty(t, id.name.as_str())?);
                }
                Some(self.generic_type(id.name.as_str(), targs)?)
            }
            // `new Vector3(..)` of the engine API.
            Expression::NewExpression(n)
                if matches!(&n.callee, Expression::Identifier(id)
                    if self.manifest.values.contains_key(id.name.as_str())) =>
            {
                let Expression::Identifier(id) = &n.callee else {
                    unreachable!("matched above")
                };
                Some(self.named_type(id.name.as_str())?)
            }
            // `Color.hex("..")`: a function of an engine namespace.
            Expression::CallExpression(c)
                if matches!(&c.callee, Expression::StaticMemberExpression(m)
                    if matches!(&m.object, Expression::Identifier(id)
                        if self.manifest.namespaces.contains_key(id.name.as_str()))) =>
            {
                let Expression::StaticMemberExpression(m) = &c.callee else {
                    unreachable!("matched above")
                };
                let Expression::Identifier(id) = &m.object else {
                    unreachable!("matched above")
                };
                let ret = self.manifest.namespaces[id.name.as_str()]
                    .get(m.property.name.as_str())
                    .map(|f| f.ret.clone());
                ret.map(|r| self.ext_ir(&r))
            }
            Expression::StaticMemberExpression(m)
                if matches!(&m.object, Expression::Identifier(id)
                    if self.manifest.ns_props.get(id.name.as_str())
                        .is_some_and(|p| p.contains_key(m.property.name.as_str()))) =>
            {
                // `Color.white`: a property of an engine namespace.
                let Expression::Identifier(id) = &m.object else {
                    unreachable!("matched above")
                };
                let ret = self.manifest.ns_props[id.name.as_str()][m.property.name.as_str()]
                    .getter
                    .as_ref()
                    .map(|g| g.ret.clone());
                ret.map(|r| self.ext_ir(&r))
            }
            Expression::StaticMemberExpression(m) => {
                // `CENTER.x` where `CENTER = { x: .., y: .. }`.
                let mut obj = &m.object;
                while let Expression::ParenthesizedExpression(p) = obj {
                    obj = &p.expression;
                }
                let base = match obj {
                    Expression::Identifier(id) => self
                        .const_inits
                        .get(id.name.as_str())
                        .copied()
                        .unwrap_or(obj),
                    other => other,
                };
                if let Expression::ObjectExpression(o) = base {
                    let want = m.property.name.as_str();
                    let mut found = None;
                    for p in &o.properties {
                        if let ObjectPropertyKind::ObjectProperty(p) = p
                            && let PropertyKey::StaticIdentifier(k) = &p.key
                            && k.name == want
                        {
                            found = Some(&p.value);
                        }
                    }
                    match found {
                        Some(v) => self.guess_ty(v, depth + 1)?,
                        None => None,
                    }
                } else {
                    None
                }
            }
            Expression::ObjectExpression(o) => {
                let mut fields = Vec::new();
                for p in &o.properties {
                    let ObjectPropertyKind::ObjectProperty(p) = p else {
                        return Ok(None);
                    };
                    let PropertyKey::StaticIdentifier(k) = &p.key else {
                        return Ok(None);
                    };
                    let Some(ty) = self.guess_ty(&p.value, depth + 1)? else {
                        return Ok(None);
                    };
                    fields.push(Field {
                        name: k.name.to_string(),
                        ty,
                        optional: false,
                    });
                }
                Some(Ty::Obj(self.anon_struct("object", fields)))
            }
            _ => None,
        })
    }

    /// The struct of class `name`, created on first use. Its fields are filled in by
    /// `finalize_classes`, once every class is known (a base class may mention its subclasses).
    pub(crate) fn class_type(&mut self, name: &str) -> R<Ty> {
        let env = self.class_tenv.get(name).cloned().unwrap_or_default();
        let saved = std::mem::replace(&mut self.tenv, env);
        let r = self.class_type_in_env(name);
        self.tenv = saved;
        r
    }

    fn class_type_in_env(&mut self, name: &str) -> R<Ty> {
        let c = self.class_decls[name];
        if !c.decorators.is_empty() {
            bail!("class {name}: decorated classes are not supported yet")
        }
        let sid = self.new_struct(name, vec![]);
        self.named.insert(name.to_string(), Ty::Obj(sid));
        self.class_of_sid.insert(sid, name.to_string());
        let mut info = ClassInfo {
            is_abstract: c.r#abstract,
            ..ClassInfo::default()
        };
        if let Some(h) = &c.heritage {
            let Expression::Identifier(pid) = &h.expression else {
                bail!("class {name}: `extends` needs a class name")
            };
            let mut parent = pid.name.to_string();
            if self.generic_classes.contains_key(&parent) {
                // `extends Base<number>`: the base class specialised.
                let mut targs = Vec::new();
                if let Some(ta) = &h.type_arguments {
                    for t in &ta.params {
                        targs.push(self.ty(t, name)?);
                    }
                }
                let tparams = self.generic_classes[&parent].tparams.clone();
                let targs = self.complete_targs(&parent, &tparams, targs)?;
                parent = self.instantiate_class(&parent, targs)?;
            } else if h.type_arguments.is_some() {
                bail!("class {name}: `{parent}` is not a generic class")
            }
            if !self.class_decls.contains_key(&parent) {
                bail!(
                    "class {name} extends `{parent}`, which is not a class of this program \
                     (engine classes wait for the engine boundary work)"
                )
            }
            info.parent = Some(parent);
        }
        self.classes.insert(name.to_string(), info.clone());
        if let Some(p) = &info.parent {
            self.named_type(p)?;
        }
        let mut fields: Vec<Field> = Vec::new();
        for el in &c.body.body {
            match el {
                ClassElement::PropertyDefinition(p) => {
                    let marked = self.is_property_decorated(&p.decorators, name)?;
                    if p.r#static {
                        continue;
                    }
                    if p.computed {
                        bail!("class {name}: computed property name")
                    }
                    let key = self.member_key(&p.key)?;
                    if marked {
                        info.property_fields.push(key.clone());
                    }
                    let what = format!("{name}.{key}");
                    let ty = self.field_ty(
                        p.type_annotation.as_deref(),
                        p.optional,
                        p.value.as_ref(),
                        &what,
                    )?;
                    fields.push(Field {
                        name: key,
                        ty,
                        optional: p.optional,
                    });
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
                            optional: p.optional,
                        });
                    }
                }
                ClassElement::StaticBlock(_) | ClassElement::AccessorProperty(_) => {
                    bail!("class {name}: static blocks and `accessor` fields are not supported")
                }
                _ => {}
            }
        }
        info.own_fields = fields;
        self.classes.insert(name.to_string(), info);
        Ok(Ty::Obj(sid))
    }

    /// Are these decorators exactly `@property` (or `@property()`)? Any other is an error: a
    /// decorator is compile-time metadata here, not code that runs.
    fn is_property_decorated(&self, decorators: &[Decorator<'a>], class: &str) -> R<bool> {
        let mut marked = false;
        for d in decorators {
            let name = match &d.expression {
                Expression::Identifier(id) => Some(id.name.as_str()),
                Expression::CallExpression(c) => match &c.callee {
                    Expression::Identifier(id) => Some(id.name.as_str()),
                    _ => None,
                },
                _ => None,
            };
            match name {
                Some("property") => marked = true,
                _ => bail!("class {class}: only `@property` decorators are supported"),
            }
        }
        Ok(marked)
    }

    /// Gives every class struct its fields: the base class's first, then its own. A field a
    /// subclass declares again (`x = 5` over the base's `x`) is the same slot.
    pub(crate) fn finalize_classes(&mut self) -> R<()> {
        let names: Vec<String> = self.classes.keys().cloned().collect();
        self.finalize_some(&names)
    }

    /// `finalize_classes` for some classes (specialisations made after the first pass).
    pub(crate) fn finalize_some(&mut self, names: &[String]) -> R<()> {
        let mut done: HashSet<String> = HashSet::new();
        for n in names {
            self.layout_class(n, &mut done, 0)?;
        }
        // The `@property` tables, in struct order.
        let mut by_sid: Vec<(StructId, String)> = names
            .iter()
            .map(|n| (self.classes_sid(n), n.clone()))
            .collect();
        by_sid.sort();
        for (sid, n) in by_sid {
            let mut chain = Vec::new();
            let mut bases = Vec::new();
            let mut cur = Some(n);
            while let Some(c) = cur {
                let info = &self.classes[&c];
                chain.push(info.property_fields.clone());
                bases.push(self.classes_sid(&c));
                cur = info.parent.clone();
            }
            // The class and its ancestors, for `getComponent(Base)` to find subclasses.
            self.m.class_bases.push((sid, bases));
            let mut entries: Vec<PropEntry> = Vec::new();
            for names in chain.iter().rev() {
                for f in names {
                    let index = self.m.structs[sid].field(f).expect("a declared field");
                    if entries.iter().all(|e| e.name != *f) {
                        entries.push(PropEntry {
                            name: f.clone(),
                            index,
                            ty: self.m.structs[sid].fields[index].ty.clone(),
                        });
                    }
                }
            }
            if !entries.is_empty() {
                self.m.props.push(PropTable {
                    class: sid,
                    entries,
                });
            }
        }
        Ok(())
    }

    fn layout_class(&mut self, name: &str, done: &mut HashSet<String>, depth: usize) -> R<()> {
        if done.contains(name) {
            return Ok(());
        }
        if depth > self.classes.len() {
            bail!("class {name}: a class cannot extend itself")
        }
        let info = self.classes[name].clone();
        let mut fields: Vec<Field> = Vec::new();
        if let Some(p) = &info.parent {
            self.layout_class(p, done, depth + 1)?;
            let psid = self.classes_sid(p);
            fields = self.m.structs[psid].fields.clone();
        }
        for f in info.own_fields {
            match fields.iter().position(|g| g.name == f.name) {
                Some(i) if fields[i].ty != f.ty => {
                    bail!(
                        "class {name}: field `{}` changes its type from the base class",
                        f.name
                    )
                }
                Some(_) => {}
                None => fields.push(f),
            }
        }
        let sid = self.classes_sid(name);
        self.m.structs[sid].fields = fields;
        done.insert(name.to_string());
        Ok(())
    }

    pub(crate) fn classes_sid(&self, name: &str) -> StructId {
        match self.named.get(name) {
            Some(Ty::Obj(s)) => *s,
            _ => unreachable!("class {name} has a struct"),
        }
    }

    /// An empty parameter list and body, for a class without a constructor.
    pub(crate) fn empty_function(&mut self) -> (&'a FormalParameters<'a>, &'a FunctionBody<'a>) {
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
        self.collect_class_as(c, &name)
    }

    /// `collect_class` for a class under `name` (a specialisation has its own name).
    pub(crate) fn collect_class_as(&mut self, c: &'a Class<'a>, name: &str) -> R<()> {
        let env = self.class_tenv.get(name).cloned();
        let name = name.to_string();
        let Ty::Obj(sid) = self.named_type(&name)? else {
            unreachable!("a class is a struct")
        };
        let mut info = self.classes[&name].clone();
        if let Some(p) = &info.parent
            && !self.classes[p].collected
        {
            bail!("class {name}: declare the base class `{p}` before its subclasses")
        }
        let mut has_ctor = false;
        for el in &c.body.body {
            let ClassElement::MethodDefinition(m) = el else {
                continue;
            };
            if !m.decorators.is_empty() {
                bail!("class {name}: decorators are not supported yet")
            }
            let key = if m.computed {
                self.symbol_key(&m.key)?
            } else {
                self.member_key(&m.key)?
            };
            let f: &'a Function<'a> = &m.value;
            if f.type_parameters.is_some() {
                if m.kind != MethodDefinitionKind::Method || m.r#static || f.body.is_none() {
                    bail!("class {name}: only plain instance methods can be generic")
                }
                self.generic_methods.insert(
                    (name.clone(), key),
                    generics::GenericFn {
                        tparams: generics::tparam_list(f.type_parameters.as_deref()),
                        params: &f.params,
                        ret: f.return_type.as_ref().map(|r| &r.type_annotation),
                        body: FnBody::Block(f.body.as_ref().expect("checked")),
                    },
                );
                continue;
            }
            let is_abstract_method = f.body.is_none() && c.r#abstract && !m.r#static;
            // An overload signature has no body; an abstract method becomes an empty stub that is
            // never called (every instance is of a subclass that overrides it).
            let (body, stub) = match &f.body {
                Some(b) => (&**b, false),
                None if is_abstract_method => (self.empty_function().1, true),
                None => continue,
            };
            let _ = stub;
            let ret = f.return_type.as_ref().map(|r| &r.type_annotation);
            let mut src = FnSrc {
                params: &f.params,
                ret,
                body: FnBody::Block(body),
                exported: false,
                kind: FnKind::Method(sid),
                ret_ty: None,
                tenv: env.clone(),
            };
            match (m.kind, m.r#static) {
                (MethodDefinitionKind::Constructor, _) => {
                    let fname = format!("{name}$init");
                    src.ret = None;
                    src.ret_ty = Some(Ty::Void);
                    src.kind = FnKind::Ctor(sid, false);
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
                (MethodDefinitionKind::Get | MethodDefinitionKind::Set, true) => {
                    // `static get current()`: a free function read as `Class.current`.
                    let getter = m.kind == MethodDefinitionKind::Get;
                    let fname =
                        format!("{name}.static {} {key}", if getter { "get" } else { "set" });
                    src.kind = FnKind::Free;
                    self.declare_fn(&fname, src)?;
                    info.statics.insert(
                        format!("{} {key}", if getter { "get" } else { "set" }),
                        fname,
                    );
                }
            }
        }
        if !has_ctor {
            let fname = format!("{name}$init");
            let (params, body) = match &info.parent {
                // A derived class without a constructor takes the base class's parameters and
                // passes them on.
                Some(p) => {
                    let pctor = self.classes[p].ctor.clone();
                    (self.fsrc[&pctor].params, self.empty_function().1)
                }
                None => self.empty_function(),
            };
            let src = FnSrc {
                params,
                ret: None,
                body: FnBody::Block(body),
                exported: false,
                kind: FnKind::Ctor(sid, info.parent.is_some()),
                ret_ty: Some(Ty::Void),
                tenv: env.clone(),
            };
            self.declare_fn(&fname, src)?;
            info.ctor = fname;
        }
        info.collected = true;
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

    /// What a constructor does right after the base class's constructor has returned (or first,
    /// for a class with no base): store the parameter properties, then run the field
    /// initialisers in source order.
    pub(crate) fn post_super(&mut self, out: &mut Vec<Stmt>) -> R<()> {
        let (class, sid) = self.cur.cls.clone().expect("inside a constructor");
        let params = self.fsrc[&self.classes[&class].ctor].params;
        let this_e = |s: &mut Self| {
            let b = s.lookup("this").expect("bound as the first parameter");
            s.read_binding(&b)
        };
        for p in &params.items {
            if p.accessibility.is_none() && !p.readonly {
                continue;
            }
            let BindingPattern::BindingIdentifier(b) = &p.pattern else {
                continue;
            };
            let Some(idx) = self.m.structs[sid].field(b.name.as_str()) else {
                continue;
            };
            let value = {
                let bind = self.lookup(b.name.as_str()).expect("a bound parameter");
                self.read_binding(&bind)
            };
            let fty = self.m.structs[sid].fields[idx].ty.clone();
            let value = self.coerce(value, &fty)?;
            let t = this_e(self);
            out.push(Stmt::StoreField(t, sid, idx, value));
        }
        let c = self.class_decls[&class];
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
            out.push(Stmt::StoreField(t, sid, idx, v));
        }
        Ok(())
    }

    /// The start of a constructor function: a class without a base initialises its fields; a
    /// derived class without its own constructor calls the base constructor with its parameters
    /// first (an explicit constructor does it where `super(...)` is written).
    pub(crate) fn ctor_prologue(
        &mut self,
        class: &str,
        implicit_super: bool,
        pvars: &[VarId],
        pro: &mut Vec<Stmt>,
    ) -> R<()> {
        let parent = self.classes[class].parent.clone();
        match parent {
            None => self.post_super(pro),
            Some(p) if implicit_super => {
                let pinit = self.fids[&self.classes[&p].ctor];
                let this = {
                    let b = self.lookup("this").expect("bound as the first parameter");
                    self.read_binding(&b)
                };
                let mut args = vec![this];
                // Parameters 1.. are the constructor's own (parameter 0 is `this`).
                for v in &pvars[1..] {
                    args.push(self.var_expr(*v));
                }
                pro.push(Stmt::Eval(mk(ExprKind::Call(pinit, args), Ty::Void)));
                self.post_super(pro)
            }
            Some(_) => Ok(()),
        }
    }

    /// `super(args)`: run the base constructor on `this`, then this class's own initialisation.
    pub(crate) fn lower_super_call(&mut self, c: &'a CallExpression<'a>) -> R<Expr> {
        let Some((class, _)) = self.cur.cls.clone() else {
            bail!("`super(...)` outside a constructor")
        };
        let Some(parent) = self.classes[&class].parent.clone() else {
            bail!("class {class} has no base class to call")
        };
        let pinit = self.classes[&parent].ctor.clone();
        let this = {
            let b = self.lookup("this").expect("bound as the first parameter");
            self.read_binding(&b)
        };
        let call = self.call_declared(&pinit, &c.arguments, Some(this))?;
        let mut stmts = vec![Stmt::Eval(call)];
        self.post_super(&mut stmts)?;
        Ok(mk(
            ExprKind::Seq(stmts, Box::new(mk(ExprKind::Num(0.0), Ty::Void))),
            Ty::Void,
        ))
    }

    /// `super.m(args)`: the base class's implementation, called directly on `this`.
    pub(crate) fn lower_super_method(&mut self, m: &str, c: &'a CallExpression<'a>) -> R<Expr> {
        let Some((class, _)) = self.cur.cls.clone() else {
            bail!("`super.{m}` outside a method")
        };
        let Some(parent) = self.classes[&class].parent.clone() else {
            bail!("class {class} has no base class")
        };
        let psid = self.classes_sid(&parent);
        let Some(f) = self.resolve_in(psid, Which::Method, m) else {
            bail!("the base class {parent} has no method `{m}`")
        };
        let this = {
            let b = self.lookup("this").expect("bound as the first parameter");
            self.read_binding(&b)
        };
        self.call_declared(&f, &c.arguments, Some(this))
    }

    /// `new C(args)`: allocate, null the nullable fields, run `C$init`.
    pub(crate) fn new_object(&mut self, class: &str, args: &'a [Argument<'a>], at: u32) -> R<Expr> {
        if self.classes[class].is_abstract {
            bail!(
                "{}: cannot create an instance of the abstract class {class}",
                self.loc(at)
            )
        }
        let sid = self.classes_sid(class);
        let ty = Ty::Obj(sid);
        let tmp = self.new_local(ty.clone());
        let mut stmts = vec![Stmt::Assign(tmp, mk(ExprKind::NewObj(sid), ty.clone()))];
        for i in 0..self.m.structs[sid].fields.len() {
            let fty = self.m.structs[sid].fields[i].ty.clone();
            if fty.is_nullable() {
                let t = self.var_expr(tmp);
                stmts.push(Stmt::StoreField(t, sid, i, mk(ExprKind::Null, fty)));
            }
        }
        let init = self.classes[class].ctor.clone();
        let this = self.var_expr(tmp);
        let call = self.call_declared(&init, args, Some(this))?;
        stmts.push(Stmt::Eval(call));
        Ok(mk(ExprKind::Seq(stmts, Box::new(self.var_expr(tmp))), ty))
    }

    // ---- the class hierarchy ----

    fn member_map(info: &ClassInfo, which: Which) -> &HashMap<String, String> {
        match which {
            Which::Method => &info.methods,
            Which::Getter => &info.getters,
            Which::Setter => &info.setters,
        }
    }

    /// The implementation of a member that a class has: its own, or the nearest base class's.
    pub(crate) fn resolve_in(&self, sid: StructId, which: Which, name: &str) -> Option<String> {
        let mut cur = self.class_of_sid.get(&sid)?;
        loop {
            let info = self.classes.get(cur)?;
            if let Some(f) = Self::member_map(info, which).get(name) {
                return Some(f.clone());
            }
            cur = info.parent.as_ref()?;
        }
    }

    /// Is `sub` the class `base` or one of its subclasses?
    pub(crate) fn is_subclass(&self, sub: StructId, base: StructId) -> bool {
        let mut cur = self.class_of_sid.get(&sub);
        while let Some(n) = cur {
            if self.classes_sid(n) == base {
                return true;
            }
            cur = self.classes.get(n).and_then(|i| i.parent.as_ref());
        }
        false
    }

    /// The nearest class that both of two class types derive from.
    pub(crate) fn common_base(&self, a: &Ty, b: &Ty) -> Option<StructId> {
        let (Ty::Obj(x), Ty::Obj(y)) = (a, b) else {
            return None;
        };
        let mut cur = self.class_of_sid.get(x);
        while let Some(n) = cur {
            let sid = self.classes_sid(n);
            if self.is_subclass(*y, sid) {
                return Some(sid);
            }
            cur = self.classes.get(n).and_then(|i| i.parent.as_ref());
        }
        None
    }

    /// The classes below `sid` (not `sid` itself), in the order they were created.
    fn strict_subclasses(&self, sid: StructId) -> Vec<StructId> {
        let mut v: Vec<StructId> = self
            .class_of_sid
            .keys()
            .copied()
            .filter(|s| *s != sid && self.is_subclass(*s, sid))
            .collect();
        v.sort_unstable();
        v
    }

    /// How a member is called on an object of static class `sid`: the statically resolved
    /// function (for the signature) and the target. One reachable implementation is a direct call.
    pub(crate) fn target_for(
        &mut self,
        sid: StructId,
        which: Which,
        name: &str,
    ) -> Option<(String, Target)> {
        let stat = self.resolve_in(sid, which, name)?;
        let mut entries: Vec<(StructId, FuncId)> = Vec::new();
        let mut distinct: Vec<FuncId> = Vec::new();
        let mut family = self.strict_subclasses(sid);
        family.insert(0, sid);
        for c in family {
            let cname = &self.class_of_sid[&c];
            if self.classes[cname].is_abstract {
                continue;
            }
            if let Some(f) = self.resolve_in(c, which, name) {
                let fid = self.fids[&f];
                entries.push((c, fid));
                if !distinct.contains(&fid) {
                    distinct.push(fid);
                }
            }
        }
        let stat_id = self.fids[&stat];
        if distinct.len() <= 1 {
            let f = distinct.first().copied().unwrap_or(stat_id);
            return Some((stat, Target::Direct(f)));
        }
        let table = match self.m.vtables.iter().position(|t| t.entries == entries) {
            Some(i) => i,
            None => {
                self.m.vtables.push(VTable { entries });
                self.m.vtables.len() - 1
            }
        };
        Some((stat, Target::Virtual(table, stat_id)))
    }

    pub(crate) fn target_fid(t: &Target) -> FuncId {
        match t {
            Target::Direct(f) | Target::Virtual(_, f) => *f,
        }
    }

    /// The call of a target; the receiver is the first argument.
    pub(crate) fn target_call(t: &Target, args: Vec<Expr>, ret: Ty) -> Expr {
        match t {
            Target::Direct(f) => mk(ExprKind::Call(*f, args), ret),
            Target::Virtual(table, _) => mk(ExprKind::CallVirtual(*table, args), ret),
        }
    }

    /// For a member the static class does not have: the one subclass chain that does (a downcast
    /// the program's own narrowing, already checked by tsc, makes valid).
    pub(crate) fn downcast_for(
        &self,
        sid: StructId,
        what: impl Fn(&Self, StructId) -> bool,
    ) -> Option<StructId> {
        let subs: Vec<StructId> = self
            .strict_subclasses(sid)
            .into_iter()
            .filter(|s| what(self, *s))
            .collect();
        // The topmost class that has it; unrelated candidates are ambiguous.
        let top = *subs.first()?;
        subs.iter()
            .all(|s| self.is_subclass(*s, top))
            .then_some(top)
    }

    pub(crate) fn has_field(&self, sid: StructId, prop: &str) -> bool {
        self.m.structs[sid].field(prop).is_some()
    }

    /// A pointer to a subclass object used as the subclass (same address, narrower type).
    pub(crate) fn downcast(obj: Expr, to: StructId) -> Expr {
        mk(ExprKind::Cast(Box::new(obj)), Ty::Obj(to))
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
        self.call_target(name, Target::Direct(fid), args, recv)
    }

    /// Like `call_declared`, through a target; `name` is the function whose signature and
    /// parameter defaults apply.
    pub(crate) fn call_target(
        &mut self,
        name: &str,
        target: Target,
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
        let call = Self::target_call(&target, vals, ret.clone());
        Ok(if pre.is_empty() {
            call
        } else {
            mk(ExprKind::Seq(pre, Box::new(call)), ret)
        })
    }

    /// `obj.m` used as a value: a closure that remembers `obj` and calls the method on it (so,
    /// unlike JavaScript, a detached method keeps its `this`). The environment is a one-element
    /// array holding the receiver; the wrapper function reads it back and calls the target.
    pub(crate) fn bound_method(&mut self, recv: Expr, sid: StructId, name: &str) -> R<Expr> {
        let Some((stat, target)) = self.target_for(sid, Which::Method, name) else {
            bail!("{} has no method `{name}`", self.m.structs[sid].name)
        };
        self.ensure_lowered(&stat)?;
        let sfid = self.fids[&stat];
        let (all_ptys, ret) = self.fsig[sfid].clone();
        let ret = ret.ok_or_else(|| format!("{stat}: return type unknown"))?;
        let ptys: Vec<Ty> = all_ptys[1..].to_vec();
        let key = match &target {
            Target::Direct(f) => format!("{stat}$bound#d{f}"),
            Target::Virtual(t, _) => format!("{stat}$bound#v{t}"),
        };
        let env_ty = Ty::Arr(Box::new(Ty::Cell(Box::new(Ty::Void))));
        let wid = match self.wrappers.get(&key) {
            Some(w) => *w,
            None => {
                let mut locals = vec![env_ty.clone()];
                locals.extend(ptys.iter().cloned());
                let params: Vec<VarId> = (0..locals.len() as u32).collect();
                let slot0 = mk(
                    ExprKind::Index(
                        Box::new(mk(ExprKind::Var(0), env_ty.clone())),
                        Box::new(mk(ExprKind::Num(0.0), Ty::Num)),
                    ),
                    Ty::Cell(Box::new(Ty::Void)).nullable(),
                );
                let this = mk(ExprKind::Cast(Box::new(slot0)), Ty::Obj(sid));
                let mut args = vec![this];
                for (i, t) in locals.iter().enumerate().skip(1) {
                    args.push(mk(ExprKind::Var(i as u32), t.clone()));
                }
                let call = Self::target_call(&target, args, ret.clone());
                let body = if ret == Ty::Void {
                    vec![Stmt::Eval(call), Stmt::Return(None)]
                } else {
                    vec![Stmt::Return(Some(call))]
                };
                let wid = self.m.funcs.len();
                self.m.funcs.push(Func {
                    name: key.clone(),
                    exported: false,
                    params,
                    ret: ret.clone(),
                    locals,
                    body,
                });
                self.fsig.push((ptys.clone(), Some(ret.clone())));
                self.fstate.push(2);
                self.wrappers.insert(key, wid);
                wid
            }
        };
        let env = self.new_local(env_ty.clone());
        let stmts = vec![
            Stmt::Assign(
                env,
                mk(
                    ExprKind::Rt(RtFn::ArrNew, vec![mk(ExprKind::Num(1.0), Ty::Num)]),
                    env_ty.clone(),
                ),
            ),
            Stmt::Eval(mk(
                ExprKind::Rt(RtFn::ArrPush, vec![self.var_expr(env), recv]),
                Ty::Void,
            )),
        ];
        let env_expr = mk(ExprKind::Seq(stmts, Box::new(self.var_expr(env))), env_ty);
        Ok(mk(
            ExprKind::MakeClosure(wid, Some(Box::new(env_expr))),
            Ty::Fn(Box::new(FnTy { params: ptys, ret })),
        ))
    }

    /// A call of a getter (`obj.x`).
    pub(crate) fn call_getter(&mut self, stat: &str, target: &Target, obj: Expr) -> R<Expr> {
        let fid = self.fids[stat];
        self.ensure_lowered(stat)?;
        let Some(ret) = self.fsig[fid].1.clone() else {
            bail!("{stat}: return type unknown")
        };
        Ok(Self::target_call(target, vec![obj], ret))
    }

    /// The type a property assignment through a setter takes.
    pub(crate) fn setter_value_ty(&mut self, t: &Target) -> Ty {
        self.fsig[Self::target_fid(t)].0[1].clone()
    }
}
