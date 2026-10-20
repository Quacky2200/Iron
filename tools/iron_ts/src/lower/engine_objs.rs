//! Engine objects made the way gameplay code writes them: `new GameObject("Main Camera", {
//! position: new Vector3(0, 0, 10) })` and `go.addComponent(Camera, { orthographic: true })`.
//!
//! Both are expanded at compile time over the engine API: `GameObject.create()` then the setter
//! of `name` and of each option (an option the object has no property for is looked up on its
//! `transform`, so `position` and `rotation` work), and `Camera.attach(go)` then the setter of
//! each option. A key with no setter is an error that names it.

use super::expr::mk;
use super::*;
use crate::manifest::ExternFn;

impl<'a> Lowerer<'a> {
    /// Does `new Class(..)` mean "create an engine object" (the class has a static `create`)?
    pub(crate) fn is_creatable(&self, class: &str) -> bool {
        self.manifest
            .classes
            .get(class)
            .is_some_and(|k| k.statics.contains_key("create"))
    }

    /// `new GameObject(name?, options?)`.
    pub(crate) fn lower_new_handle(&mut self, class: &str, n: &'a NewExpression<'a>) -> R<Expr> {
        self.lower_new_handle_args(class, &n.arguments, n.span.start)
    }

    /// `Class.create({ .. })` on a class whose `create` takes nothing: the same as
    /// `new Class({ .. })`, the object's keys set as properties.
    pub(crate) fn is_create_with_options(&self, class: &str, c: &CallExpression<'_>) -> bool {
        self.manifest
            .classes
            .get(class)
            .and_then(|k| k.statics.get("create"))
            .is_some_and(|f| f.params.is_empty())
            && matches!(c.arguments.as_slice(), [Argument::ObjectExpression(_)])
    }

    /// `new Class(args)` over the arguments (also `Class.create({ .. })`).
    pub(crate) fn lower_new_handle_args(
        &mut self,
        class: &str,
        arguments: &'a [Argument<'a>],
        start: u32,
    ) -> R<Expr> {
        let at = self.loc(start);
        let k = self.manifest.classes[class].clone();
        let create = k.statics["create"].clone();
        let ty = Ty::Handle(class.to_string());
        // `create(a, b)`: the constructor arguments are its parameters (`new Rng(seed)`).
        if !create.params.is_empty() {
            // A trailing `{ key: value }` after the required parameters sets properties once the
            // object is made (`new AnimationTransition("A", "B", { duration: 0.2 })`), unless its
            // keys are parameters of `create` (then call_ext_args maps them by name).
            let args: &'a [Argument<'a>] = arguments;
            let required = create.optional_from.min(create.params.len());
            let options = match args.last() {
                Some(Argument::ObjectExpression(o))
                    if args.len() > required
                        && !o.properties.iter().all(|p| {
                            matches!(p, ObjectPropertyKind::ObjectProperty(p)
                                if matches!(&p.key, PropertyKey::StaticIdentifier(k)
                                    if create.params.iter().any(|(n, _)| n == k.name.as_str())))
                        }) =>
                {
                    Some(&**o)
                }
                _ => None,
            };
            let positional = if options.is_some() {
                &args[..args.len() - 1]
            } else {
                args
            };
            let mut e =
                self.call_ext_args(create, None, &format!("new {class}"), start, positional)?;
            e.ty = ty.clone();
            let Some(o) = options else {
                return Ok(e);
            };
            let tmp = self.new_local(ty.clone());
            let mut stmts = vec![Stmt::Assign(tmp, e)];
            self.apply_options(class, tmp, o, &mut stmts, &at)?;
            let result = self.var_expr(tmp);
            return Ok(mk(ExprKind::Seq(stmts, Box::new(result)), ty));
        }
        let tmp = self.new_local(ty.clone());
        let made = self.emit_extern(&create, vec![]);
        let mut stmts = vec![Stmt::Assign(tmp, made)];
        for (i, a) in arguments.iter().enumerate() {
            let Some(e) = a.as_expression() else {
                bail!("{at}: new {class}: spread argument")
            };
            match e {
                Expression::ObjectExpression(o) if i > 0 || arguments.len() == 1 => {
                    self.apply_options(class, tmp, o, &mut stmts, &at)?;
                }
                _ if i == 0 => {
                    let Some(setter) = k.setters.get("name").cloned() else {
                        bail!("{at}: {class} has no `name` to set from new {class}(name)")
                    };
                    let v = self.lower_expr(e, Some(&Ty::Str))?;
                    let v = self.coerce(v, &Ty::Str)?;
                    let h = self.var_expr(tmp);
                    stmts.push(Stmt::Eval(self.emit_extern(&setter, vec![h, v])));
                }
                _ => bail!("{at}: new {class}(name?, options?)"),
            }
        }
        let result = self.var_expr(tmp);
        Ok(mk(ExprKind::Seq(stmts, Box::new(result)), ty))
    }

    /// The object a component call is on: a `GameObject` handle, or a component (its `gameObject`).
    pub(crate) fn component_target(&self, recv: Expr) -> Option<Expr> {
        match recv.ty.strip() {
            Ty::Handle(_) => Some(recv),
            Ty::Obj(sid) => {
                let sid = *sid;
                let idx = self.m.structs[sid].field("gameObject")?;
                let ty = self.m.structs[sid].fields[idx].ty.strip().clone();
                Some(mk(ExprKind::Field(Box::new(recv), sid, idx), ty))
            }
            _ => None,
        }
    }

    /// `go.getComponent(Player)` / `go.addComponent(Player, { speed: 3 })` for a class of this
    /// program (registered with `register`). `None` when the argument is not such a class.
    pub(crate) fn lower_user_component_call(
        &mut self,
        go: &Expr,
        m: &str,
        c: &'a CallExpression<'a>,
    ) -> R<Option<Expr>> {
        let at = self.loc(c.span.start);
        let Some(Expression::Identifier(cls)) = c.arguments.first().and_then(|a| a.as_expression())
        else {
            return Ok(None);
        };
        let name = cls.name.to_string();
        if !self.class_decls.contains_key(&name) {
            // An engine component (`Camera`): its static `findOn(go)`.
            if m == "getComponent"
                && let Some(find) = self
                    .manifest
                    .classes
                    .get(&name)
                    .and_then(|k| k.statics.get("findOn"))
                    .cloned()
            {
                let mut e = self.emit_extern(&find, vec![go.clone()]);
                e.ty = Ty::Handle(name).nullable();
                return Ok(Some(e));
            }
            return Ok(None);
        }
        let Ty::Obj(sid) = self.named_type(&name)? else {
            return Ok(None);
        };
        let which = mk(ExprKind::Num(sid as f64), Ty::Num);
        if m == "getComponent" {
            let call = ExprKind::Rt(RtFn::Str("ts_rt_component_get"), vec![go.clone(), which]);
            return Ok(Some(mk(call, Ty::Obj(sid).nullable())));
        }
        if m == "getComponents" {
            let call = ExprKind::Rt(RtFn::Str("ts_rt_component_all_gc"), vec![go.clone(), which]);
            return Ok(Some(mk(call, Ty::Arr(Box::new(Ty::Obj(sid))))));
        }
        if m == "getComponentInChildren" {
            // A function of TypeScript that walks the children (`children_helpers`).
            let e = self.call_helper(&format!("__cic_{name}"), vec![go.clone()])?;
            return Ok(Some(e));
        }
        let ty = Ty::Obj(sid);
        let tmp = self.new_local(ty.clone());
        let made = ExprKind::Rt(RtFn::Str("ts_rt_component_add_gc"), vec![go.clone(), which]);
        let mut stmts = vec![Stmt::Assign(tmp, mk(made, ty.clone()))];
        if let Some(e) = c.arguments.get(1).and_then(|a| a.as_expression()) {
            let Expression::ObjectExpression(o) = e else {
                bail!("{at}: addComponent options are an object literal")
            };
            for p in &o.properties {
                let ObjectPropertyKind::ObjectProperty(p) = p else {
                    bail!("{at}: spread in options")
                };
                let PropertyKey::StaticIdentifier(key) = &p.key else {
                    bail!("{at}: option names are plain names")
                };
                let Some(idx) = self.m.structs[sid].field(key.name.as_str()) else {
                    bail!("{at}: {name} has no field `{}` to set", key.name)
                };
                let want = self.m.structs[sid].fields[idx].ty.clone();
                let v = self.lower_expr(&p.value, Some(&want))?;
                let v = self.coerce(v, &want)?;
                stmts.push(Stmt::StoreField(self.var_expr(tmp), sid, idx, v));
            }
        }
        let result = self.var_expr(tmp);
        Ok(Some(mk(ExprKind::Seq(stmts, Box::new(result)), ty)))
    }

    /// The `GameObject` properties scene.ts has that the engine API does not declare: `children`
    /// (a helper that walks `childAt`), `components` (every component of the program on the object)
    /// and `scene` (`SceneManager.sceneOf`).
    pub(crate) fn game_object_member(&mut self, obj: Expr, prop: &str) -> R<Expr> {
        match prop {
            "children" if self.fids.contains_key("__go_children") => {
                self.call_helper("__go_children", vec![obj])
            }
            "components" => {
                let Ty::Obj(sid) = self.named_type("Component")? else {
                    bail!("GameObject.components needs the program to use components")
                };
                let which = mk(ExprKind::Num(sid as f64), Ty::Num);
                let call = ExprKind::Rt(RtFn::Str("ts_rt_component_all_gc"), vec![obj, which]);
                Ok(mk(call, Ty::Arr(Box::new(Ty::Obj(sid)))))
            }
            "scene" => {
                let f = self
                    .manifest
                    .namespaces
                    .get("SceneManager")
                    .and_then(|n| n.get("sceneOf"))
                    .cloned();
                match f {
                    Some(f) => Ok(self.emit_extern(&f, vec![obj])),
                    None => bail!("GameObject.scene needs the scene manager API"),
                }
            }
            _ => bail!("GameObject has no property `{prop}`"),
        }
    }

    /// `go.removeComponent(c)` and `c.destroy()` for a component of this program: it is gone for
    /// `getComponent` at once and its `onDestroy` runs when the calling hook returns.
    pub(crate) fn lower_remove_component(&mut self, component: Expr) -> Expr {
        mk(
            ExprKind::Rt(RtFn::Str("ts_rt_component_remove"), vec![component]),
            Ty::Void,
        )
    }

    /// `GameObject.instantiate(original, { position, rotation, parent })`,
    /// `GameObject.dontDestroyOnLoad(go)` and `prefab.instantiate({ .. })`: calls of the scene
    /// manager's functions (the options become the numbers it takes; NaN is "left out").
    pub(crate) fn lower_scene_call(
        &mut self,
        method: &str,
        c: &'a CallExpression<'a>,
    ) -> R<Option<Expr>> {
        let at = self.loc(c.span.start);
        let Some(ns) = self.manifest.namespaces.get("SceneManager").cloned() else {
            return Ok(None);
        };
        if method == "dontDestroyOnLoad" {
            let f = ns["dontDestroyOnLoad"].clone();
            return Ok(Some(self.call_ext_args(
                f,
                None,
                "GameObject.dontDestroyOnLoad",
                c.span.start,
                &c.arguments,
            )?));
        }
        let Some(first) = c.arguments.first().and_then(|a| a.as_expression()) else {
            bail!("{at}: instantiate takes an object or a prefab")
        };
        let original = self.lower_expr(first, None)?;
        let (f, lead) = match original.ty.strip() {
            Ty::Handle(h) if h == "Prefab" => (
                self.manifest.classes["Prefab"].methods["instantiate"].clone(),
                vec![original],
            ),
            _ => (ns["instantiate"].clone(), vec![original]),
        };
        let options = match c.arguments.get(1).and_then(|a| a.as_expression()) {
            Some(Expression::ObjectExpression(o)) => Some(&**o),
            Some(_) => bail!("{at}: instantiate options are an object literal"),
            None => None,
        };
        self.finish_instantiate(f, lead, options, &at).map(Some)
    }

    /// The call of an `instantiate` function (`lead`: the original or the prefab) with the
    /// options object's `position`, `rotation` and `parent` as arguments.
    pub(crate) fn finish_instantiate(
        &mut self,
        f: ExternFn,
        lead: Vec<Expr>,
        options: Option<&'a ObjectExpression<'a>>,
        at: &str,
    ) -> R<Expr> {
        let nan = || mk(ExprKind::Num(f64::NAN), Ty::Num);
        let (mut position, mut rotation, mut parent) = (None, None, None);
        for p in options.map(|o| o.properties.iter()).into_iter().flatten() {
            let ObjectPropertyKind::ObjectProperty(p) = p else {
                bail!("{at}: spread in options")
            };
            let PropertyKey::StaticIdentifier(key) = &p.key else {
                bail!("{at}: option names are plain names")
            };
            match key.name.as_str() {
                "position" => {
                    let sid = self.value_struct("Vector3");
                    position = Some((self.lower_expr(&p.value, Some(&Ty::Obj(sid)))?, sid));
                }
                "rotation" => {
                    let sid = self.value_struct("Quaternion");
                    rotation = Some((self.lower_expr(&p.value, Some(&Ty::Obj(sid)))?, sid));
                }
                "parent" => parent = Some(self.lower_expr(&p.value, None)?),
                other => bail!("{at}: instantiate has no option `{other}`"),
            }
        }
        let mut args = lead;
        // A vector or quaternion is read into a temporary once, then its fields are the arguments.
        let mut pre = Vec::new();
        let mut parts =
            |this: &mut Self, v: Option<(Expr, usize)>, n: usize, args: &mut Vec<Expr>| match v {
                Some((e, sid)) => {
                    let tmp = this.new_local(Ty::Obj(sid));
                    pre.push(Stmt::Assign(tmp, e));
                    for i in 0..n {
                        args.push(mk(
                            ExprKind::Field(Box::new(this.var_expr(tmp)), sid, i),
                            Ty::Num,
                        ));
                    }
                }
                None => args.extend((0..n).map(|_| mk(ExprKind::Num(f64::NAN), Ty::Num))),
            };
        parts(self, position, 3, &mut args);
        parts(self, rotation, 4, &mut args);
        let handle_ty = Ty::Handle("GameObject".into());
        args.push(match parent {
            Some(p) => p,
            None => mk(ExprKind::Null, handle_ty),
        });
        let _ = nan;
        let call = self.emit_extern(&f, args);
        let ty = call.ty.clone();
        Ok(mk(ExprKind::Seq(pre, Box::new(call)), ty))
    }

    /// `GameObject.findObjectOfType(Player)`: the first component of that class on any object.
    pub(crate) fn lower_find_object_of_type(
        &mut self,
        c: &'a CallExpression<'a>,
    ) -> R<Option<Expr>> {
        let Some(Expression::Identifier(cls)) = c.arguments.first().and_then(|a| a.as_expression())
        else {
            return Ok(None);
        };
        let name = cls.name.to_string();
        if !self.class_decls.contains_key(&name) {
            return Ok(None);
        }
        let Ty::Obj(sid) = self.named_type(&name)? else {
            return Ok(None);
        };
        let which = mk(ExprKind::Num(sid as f64), Ty::Num);
        let call = ExprKind::Rt(RtFn::Str("ts_rt_component_find_any"), vec![which]);
        Ok(Some(mk(call, Ty::Obj(sid).nullable())))
    }

    /// `go.addComponent(Camera, { .. })` for an engine component class (one with a static `attach`).
    pub(crate) fn lower_add_component(&mut self, recv: Expr, c: &'a CallExpression<'a>) -> R<Expr> {
        if let Some(e) = self.lower_user_component_call(&recv, "addComponent", c)? {
            return Ok(e);
        }
        let at = self.loc(c.span.start);
        let Some(Expression::Identifier(cls)) = c.arguments.first().and_then(|a| a.as_expression())
        else {
            bail!("{at}: addComponent takes a component class")
        };
        let class = cls.name.to_string();
        let Some(attach) = self
            .manifest
            .classes
            .get(&class)
            .and_then(|k| k.statics.get("attach"))
            .cloned()
        else {
            bail!("{at}: `{class}` is not a component the engine API can add")
        };
        let ty = Ty::Handle(class.clone());
        let tmp = self.new_local(ty.clone());
        let made = self.emit_extern(&attach, vec![recv]);
        let mut stmts = vec![Stmt::Assign(tmp, made)];
        if let Some(e) = c.arguments.get(1).and_then(|a| a.as_expression()) {
            let Expression::ObjectExpression(o) = e else {
                bail!("{at}: addComponent options are an object literal")
            };
            self.apply_options(&class, tmp, o, &mut stmts, &at)?;
        }
        let result = self.var_expr(tmp);
        Ok(mk(ExprKind::Seq(stmts, Box::new(result)), ty))
    }

    /// The setters for `{ key: value, .. }` on the handle in `tmp`.
    fn apply_options(
        &mut self,
        class: &str,
        tmp: VarId,
        o: &'a ObjectExpression<'a>,
        stmts: &mut Vec<Stmt>,
        at: &str,
    ) -> R<()> {
        // `parent` first: `position`, `rotation` and `scale` of a GameObject are local to it.
        let mut props: Vec<&ObjectPropertyKind<'a>> = o.properties.iter().collect();
        props.sort_by_key(|p| {
            !matches!(p, ObjectPropertyKind::ObjectProperty(p)
                if matches!(&p.key, PropertyKey::StaticIdentifier(k) if k.name == "parent"))
        });
        for p in props {
            let ObjectPropertyKind::ObjectProperty(p) = p else {
                bail!("{at}: spread in options")
            };
            let PropertyKey::StaticIdentifier(key) = &p.key else {
                bail!("{at}: option names are plain names")
            };
            let key = key.name.as_str();
            if class == "GameObject" && self.game_object_option(tmp, key, &p.value, stmts, at)? {
                continue;
            }
            let own = self.manifest.classes[class].setters.get(key).cloned();
            if let Some(setter) = own {
                let want = self.ext_ir(&setter.params[1].1);
                let v = self.lower_expr(&p.value, Some(&want))?;
                let v = self.coerce_ext(v, &want)?;
                let h = self.var_expr(tmp);
                stmts.push(Stmt::Eval(self.emit_extern(&setter, vec![h, v])));
                continue;
            }
            // `position` of a GameObject is its transform's.
            let transform = self.manifest.classes[class]
                .getters
                .get("transform")
                .cloned();
            let tr_setter = self
                .manifest
                .classes
                .get("Transform")
                .and_then(|t| t.setters.get(key))
                .cloned();
            let (Some(get), Some(setter)) = (transform, tr_setter) else {
                bail!("{at}: {class} has no property `{key}` to set")
            };
            let want = self.ext_ir(&setter.params[1].1);
            let v = self.lower_expr(&p.value, Some(&want))?;
            let v = self.coerce(v, &want)?;
            let h = self.var_expr(tmp);
            let tr = self.emit_extern(&get, vec![h]);
            stmts.push(Stmt::Eval(self.emit_extern(&setter, vec![tr, v])));
        }
        Ok(())
    }
}
