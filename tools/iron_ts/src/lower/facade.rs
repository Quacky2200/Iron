//! The TypeScript `@iron/engine` shapes that the native API holds differently, expanded at
//! compile time over the engine API: object-valued namespace properties
//! (`RenderSettings.skybox = { .. }`) become per-field setters.

use super::expr::mk;
use super::*;

/// Namespace properties the TypeScript API holds as one object that the native API keeps as
/// per-field properties: (namespace, property) -> (the on/off property, the prefix of the field
/// properties, so `zenith` is `skyZenith`).
fn split_rule(ns: &str, prop: &str) -> Option<(&'static str, &'static str)> {
    match (ns, prop) {
        ("RenderSettings", "skybox") => Some(("skyboxEnabled", "sky")),
        ("RenderSettings", "bloom") => Some(("bloomEnabled", "bloom")),
        _ => None,
    }
}

const FACADE_TS: &str = include_str!("facade.ts");

/// `old` as a whole word (not part of a longer name).
fn replace_word(src: &str, old: &str, new: &str) -> String {
    let word = |c: char| c.is_alphanumeric() || c == '_' || c == '$';
    let mut out = String::with_capacity(src.len());
    let mut rest = src;
    while let Some(i) = rest.find(old) {
        let before = rest[..i].chars().next_back();
        let after = rest[i + old.len()..].chars().next();
        out.push_str(&rest[..i]);
        if before.is_some_and(word) || after.is_some_and(word) {
            out.push_str(old);
        } else {
            out.push_str(new);
        }
        rest = &rest[i + old.len()..];
    }
    out.push_str(rest);
    out
}

/// The program with the TypeScript names that are another name natively (`AnimatorController` is
/// `AnimationController`) renamed, and the facade chunks it needs appended.
pub fn extend_source(src: &str, manifest: &crate::manifest::Manifest) -> Option<String> {
    let mut out = src.to_string();
    if manifest.classes.contains_key("AnimationController") && src.contains("AnimatorController") {
        out = replace_word(&out, "AnimatorController", "AnimationController");
    }
    // The chunks the program names: only when the engine API has the classes they are written over.
    let mut extra = String::new();
    for chunk in FACADE_TS.split("//# when ").skip(1) {
        let (triggers, body) = chunk.split_once('\n').unwrap_or((chunk, ""));
        if triggers.split('|').any(|t| src.contains(t.trim_end())) {
            extra.push_str(body);
        }
    }
    if !extra.is_empty() {
        out.push('\n');
        out.push_str(&extra);
    }
    (out != src).then_some(out)
}

// The facade: members of the TypeScript `@iron/engine` that the native API has in another shape
// are written once in TypeScript (`facade.ts`, appended to a program that names them) as
// functions called `__x<kind>_<Class>_<member>`:
//
//   __xm_  method          `__xm_Cloth_foo(self: Cloth, a: number)`      for `cloth.foo(a)`
//   __xg_  property read   `__xg_Camera_rect(self: Camera): Rect`         for `cam.rect`
//   __xs_  property write  `__xs_Camera_rect(self: Camera, v: Rect)`      for `cam.rect = r`, `{ rect: r }`
//   __xt_  static call     `__xt_Sprite_grid(texture: Texture, ..)`        for `Sprite.grid(..)`
//   __xp_  static read     `__xp_Screen_windowSize(): Size`               for `Screen.windowSize`
//
// A method also has an object-argument variant `__xm_..$o` used when a call passes an object
// literal. These win over the native member of the same name.
impl<'a> Lowerer<'a> {
    fn x_name(kind: &str, class: &str, member: &str) -> String {
        format!("__x{kind}_{class}_{member}")
    }

    /// `recv.m(args)` through a facade method.
    pub(crate) fn ext_method_call(
        &mut self,
        class: &str,
        m: &str,
        recv: &Expr,
        c: &'a CallExpression<'a>,
    ) -> R<Option<Expr>> {
        let mut name = Self::x_name("m", class, m);
        let obj_variant = format!("{name}$o");
        if self.fids.contains_key(&obj_variant)
            && c.arguments
                .iter()
                .any(|a| matches!(a, Argument::ObjectExpression(_)))
        {
            name = obj_variant;
        }
        if !self.fids.contains_key(&name) {
            return Ok(None);
        }
        self.call_with_receiver(&name, recv.clone(), &c.arguments)
            .map(Some)
    }

    /// A call of the facade function `name(self, ..)` with `recv` as `self`: the arguments fill
    /// the parameters after it (a missing one takes its default, or null for `x?: T`).
    fn call_with_receiver(&mut self, name: &str, recv: Expr, args: &'a [Argument<'a>]) -> R<Expr> {
        let fid = self.fids[name];
        self.ensure_lowered(name)?;
        let (ptys, ret) = self.fsig[fid].clone();
        let Some(ret) = ret else {
            bail!("{name}: return type unknown")
        };
        let fp: &'a FormalParameters<'a> = self.fsrc[name].params;
        let n_fixed = fp.items.len() - 1;
        if args.len() > n_fixed {
            bail!(
                "{name}: {} arguments, at most {n_fixed} expected",
                args.len()
            );
        }
        let t = self.new_local(recv.ty.clone());
        let mut pre = vec![Stmt::Assign(t, recv)];
        let mut vals = vec![self.var_expr(t)];
        let mut names: HashMap<String, Binding> = HashMap::new();
        for i in 0..n_fixed {
            let (p, pty) = (&fp.items[i + 1], ptys[i + 1].clone());
            let v = if let Some(a) = args.get(i) {
                let Some(e) = a.as_expression() else {
                    bail!("{name}: spread argument")
                };
                let v = self.lower_expr(e, Some(&pty))?;
                self.coerce(v, &pty)?
            } else if let Some(init) = &p.initializer {
                let saved = std::mem::replace(&mut self.cur.scopes, vec![names.clone()]);
                let r = self.lower_expr(init, Some(&pty));
                self.cur.scopes = saved;
                self.coerce(r?, &pty)?
            } else if p.optional {
                self.coerce(mk(ExprKind::Null, Ty::Null), &pty)?
            } else {
                bail!("{name}: {} arguments, {n_fixed} expected", args.len())
            };
            let tmp = self.new_local(pty);
            pre.push(Stmt::Assign(tmp, v));
            vals.push(self.var_expr(tmp));
            if let BindingPattern::BindingIdentifier(b) = &p.pattern {
                names.insert(b.name.to_string(), Binding::Direct(tmp));
            }
        }
        let call = Self::target_call(&classes::Target::Direct(fid), vals, ret.clone());
        Ok(mk(ExprKind::Seq(pre, Box::new(call)), ret))
    }

    /// `setPoints(this, points)` as the setter of `points` (`{ points: [..] }`, `line.positions = ..`):
    /// a method named `set<Prop>` taking the receiver and one value.
    pub(crate) fn set_method(&self, class: &str, prop: &str) -> Option<crate::manifest::ExternFn> {
        let mut cap = prop.to_string();
        cap[..1].make_ascii_uppercase();
        self.manifest
            .classes
            .get(class)?
            .methods
            .get(&format!("set{cap}"))
            .filter(|f| f.params.len() == 2)
            .cloned()
    }

    /// `obj.prop` through a facade getter.
    pub(crate) fn ext_getter(&mut self, class: &str, prop: &str, obj: &Expr) -> R<Option<Expr>> {
        let name = Self::x_name("g", class, prop);
        if !self.fids.contains_key(&name) {
            // `camera.transform`: a component's object's transform.
            if prop == "transform"
                && class != "GameObject"
                && let Some(go) = self
                    .manifest
                    .classes
                    .get(class)
                    .and_then(|k| k.getters.get("gameObject"))
                    .cloned()
                && let Some(tr) = self
                    .manifest
                    .classes
                    .get("GameObject")
                    .and_then(|k| k.getters.get("transform"))
                    .cloned()
            {
                let owner = self.emit_extern(&go, vec![obj.clone()]);
                return Ok(Some(self.emit_extern(&tr, vec![owner])));
            }
            return Ok(None);
        }
        self.call_helper(&name, vec![obj.clone()]).map(Some)
    }

    /// The facade function that sets `obj.prop`, and the type of the value it takes.
    pub(crate) fn ext_setter(&mut self, class: &str, prop: &str) -> R<Option<(FuncId, Ty)>> {
        let name = Self::x_name("s", class, prop);
        let Some(&fid) = self.fids.get(&name) else {
            return Ok(None);
        };
        self.ensure_lowered(&name)?;
        let ty = self.fsig[fid].0[1].clone();
        Ok(Some((fid, ty)))
    }

    /// `Class.m(args)` through a facade static function.
    pub(crate) fn ext_static_call(
        &mut self,
        class: &str,
        m: &str,
        c: &'a CallExpression<'a>,
    ) -> R<Option<Expr>> {
        if class == "GameObject" && m == "createPrimitive" {
            return self.lower_create_primitive(c);
        }
        if m.ends_with("$n") {
            return Ok(None);
        }
        let mut name = Self::x_name("t", class, m);
        let argc_variant = format!("{name}${}", c.arguments.len());
        if self.fids.contains_key(&argc_variant) {
            name = argc_variant;
        }
        let obj_variant = format!("{name}$o");
        if self.fids.contains_key(&obj_variant)
            && c.arguments
                .iter()
                .any(|a| matches!(a, Argument::ObjectExpression(_)))
        {
            name = obj_variant;
        }
        if !self.fids.contains_key(&name) {
            return Ok(None);
        }
        self.call_declared(&name, &c.arguments, None).map(Some)
    }

    /// `GameObject.createPrimitive("cube", { position, scale, .. })`: `Primitives.create(kind)`,
    /// then the options as for `new GameObject`.
    fn lower_create_primitive(&mut self, c: &'a CallExpression<'a>) -> R<Option<Expr>> {
        let at = self.loc(c.span.start);
        let Some(create) = self
            .manifest
            .namespaces
            .get("Primitives")
            .and_then(|n| n.get("create"))
            .cloned()
        else {
            return Ok(None);
        };
        let Some(kind) = c.arguments.first().and_then(|a| a.as_expression()) else {
            bail!("{at}: createPrimitive takes a kind")
        };
        let kind = self.lower_expr(kind, Some(&Ty::Str))?;
        let kind = self.coerce(kind, &Ty::Str)?;
        let ty = Ty::Handle("GameObject".to_string());
        let tmp = self.new_local(ty.clone());
        let none = mk(ExprKind::Null, Ty::Str.nullable());
        let made = self.emit_extern(&create, vec![kind, none]);
        let mut stmts = vec![Stmt::Assign(tmp, made)];
        match c.arguments.get(1).and_then(|a| a.as_expression()) {
            Some(Expression::ObjectExpression(o)) => {
                self.apply_options("GameObject", tmp, o, &mut stmts, &at)?
            }
            Some(_) => bail!("{at}: createPrimitive options are an object literal"),
            None => {}
        }
        let result = self.var_expr(tmp);
        Ok(Some(mk(ExprKind::Seq(stmts, Box::new(result)), ty)))
    }

    /// `Class.prop` through a facade static getter, or a static function of the engine API that
    /// takes nothing (`Camera.main`).
    pub(crate) fn ext_static_prop(&mut self, class: &str, prop: &str) -> R<Option<Expr>> {
        let name = Self::x_name("p", class, prop);
        if self.fids.contains_key(&name) {
            return self.call_helper(&name, vec![]).map(Some);
        }
        let native = self
            .manifest
            .classes
            .get(class)
            .and_then(|k| k.statics.get(prop))
            .filter(|f| f.params.is_empty() && f.ret != ExtTy::Void)
            .cloned();
        match native {
            Some(f) => {
                let ty = self.ext_ir(&f.ret);
                let mut e = self.emit_extern(&f, vec![]);
                // A handle that may be none (`Camera.main`).
                if matches!(ty, Ty::Handle(_)) {
                    e.ty = ty.nullable();
                }
                Ok(Some(e))
            }
            None => Ok(None),
        }
    }

    /// A `new GameObject(name, { .. })` option the plain setter lookup would not find: `parent`
    /// (`setParent(p, false)`), and `position` / `rotation` / `scale`, which are local to the
    /// parent (`localPosition`, ...). `Ok(false)` leaves the key to the default path.
    pub(crate) fn game_object_option(
        &mut self,
        tmp: VarId,
        key: &str,
        value: &'a Expression<'a>,
        stmts: &mut Vec<Stmt>,
        at: &str,
    ) -> R<bool> {
        let go = &self.manifest.classes["GameObject"];
        if key == "parent" {
            let Some(f) = go.methods.get("setParent").cloned() else {
                bail!("{at}: GameObject has no setParent")
            };
            let want = self.ext_ir(&f.params[1].1);
            let v = self.lower_expr(value, Some(&want))?;
            let v = self.coerce_ext(v, &want)?;
            let h = self.var_expr(tmp);
            let keep = mk(ExprKind::Bool(false), Ty::Bool);
            stmts.push(Stmt::Eval(self.emit_extern(&f, vec![h, v, keep])));
            return Ok(true);
        }
        let local = match key {
            "position" => "localPosition",
            "rotation" => "localRotation",
            "scale" => "localScale",
            "active" => {
                let Some(setter) = go.setters.get("enabled").cloned() else {
                    return Ok(false);
                };
                let v = self.lower_expr(value, Some(&Ty::Bool))?;
                let v = self.coerce(v, &Ty::Bool)?;
                let h = self.var_expr(tmp);
                stmts.push(Stmt::Eval(self.emit_extern(&setter, vec![h, v])));
                return Ok(true);
            }
            _ => return Ok(false),
        };
        let get = go.getters.get("transform").cloned();
        let set = self
            .manifest
            .classes
            .get("Transform")
            .and_then(|t| t.setters.get(local))
            .cloned();
        let (Some(get), Some(set)) = (get, set) else {
            return Ok(false);
        };
        let want = self.ext_ir(&set.params[1].1);
        let v = self.lower_expr(value, Some(&want))?;
        let v = self.coerce(v, &want)?;
        let h = self.var_expr(tmp);
        let tr = self.emit_extern(&get, vec![h]);
        stmts.push(Stmt::Eval(self.emit_extern(&set, vec![tr, v])));
        Ok(true)
    }

    /// `RenderSettings.skybox = { .. }` / `= null`: off, then on with the defaults, then each key's
    /// setter (a `null` key turns its `has<Key>` switch off, as `sunDirection: null` does).
    pub(crate) fn lower_split_assign(
        &mut self,
        a: &'a AssignmentExpression<'a>,
    ) -> R<Option<Vec<Stmt>>> {
        let Some(SimpleAssignmentTarget::StaticMemberExpression(m)) =
            a.left.as_simple_assignment_target()
        else {
            return Ok(None);
        };
        let Expression::Identifier(id) = &m.object else {
            return Ok(None);
        };
        let (ns, prop) = (id.name.as_str(), m.property.name.as_str());
        if a.operator != AssignmentOperator::Assign || self.lookup(ns).is_some() {
            return Ok(None);
        }
        let Some((enable, prefix)) = split_rule(ns, prop) else {
            return Ok(None);
        };
        let at = self.loc(a.span.start);
        let props = self.manifest.ns_props.get(ns).cloned().unwrap_or_default();
        let setter_of = |name: &str| props.get(name).and_then(|p| p.setter.clone());
        let Some(enable) = setter_of(enable) else {
            bail!("{at}: {ns}.{prop} has no switch in the engine API")
        };
        let mut stmts = Vec::new();
        let switch = |this: &mut Self, on: bool| {
            let v = mk(ExprKind::Bool(on), Ty::Bool);
            Stmt::Eval(this.emit_extern(&enable, vec![v]))
        };
        stmts.push(switch(self, false));
        match &a.right {
            Expression::NullLiteral(_) => Ok(Some(stmts)),
            Expression::ObjectExpression(o) => {
                stmts.push(switch(self, true));
                for p in &o.properties {
                    let ObjectPropertyKind::ObjectProperty(p) = p else {
                        bail!("{at}: spread in {ns}.{prop}")
                    };
                    let PropertyKey::StaticIdentifier(key) = &p.key else {
                        bail!("{at}: {ns}.{prop} keys are plain names")
                    };
                    let mut cap = key.name.to_string();
                    cap[..1].make_ascii_uppercase();
                    if matches!(&p.value, Expression::NullLiteral(_)) {
                        // `sunDirection: null` is `skyHasSun = false`.
                        let base = cap.strip_suffix("Direction").unwrap_or(&cap);
                        let Some(has) = setter_of(&format!("{prefix}Has{base}")) else {
                            bail!("{at}: {ns}.{prop}.{} cannot be null", key.name)
                        };
                        let off = mk(ExprKind::Bool(false), Ty::Bool);
                        stmts.push(Stmt::Eval(self.emit_extern(&has, vec![off])));
                        continue;
                    }
                    let Some(setter) = setter_of(&format!("{prefix}{cap}")) else {
                        bail!("{at}: {ns}.{prop} has no `{}`", key.name)
                    };
                    let want = self.ext_ir(&setter.params[0].1);
                    let v = self.lower_expr(&p.value, Some(&want))?;
                    let v = self.coerce_ext(v, &want)?;
                    stmts.push(Stmt::Eval(self.emit_extern(&setter, vec![v])));
                }
                Ok(Some(stmts))
            }
            _ => bail!("{at}: {ns}.{prop} takes an object literal or null"),
        }
    }
}
