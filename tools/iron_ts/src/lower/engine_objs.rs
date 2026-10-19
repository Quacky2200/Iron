//! Engine objects made the way gameplay code writes them: `new GameObject("Main Camera", {
//! position: new Vector3(0, 0, 10) })` and `go.addComponent(Camera, { orthographic: true })`.
//!
//! Both are expanded at compile time over the engine API: `GameObject.create()` then the setter
//! of `name` and of each option (an option the object has no property for is looked up on its
//! `transform`, so `position` and `rotation` work), and `Camera.attach(go)` then the setter of
//! each option. A key with no setter is an error that names it.

use super::expr::mk;
use super::*;

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
        let at = self.loc(n.span.start);
        let k = self.manifest.classes[class].clone();
        let create = k.statics["create"].clone();
        let ty = Ty::Handle(class.to_string());
        let tmp = self.new_local(ty.clone());
        let made = self.emit_extern(&create, vec![]);
        let mut stmts = vec![Stmt::Assign(tmp, made)];
        for (i, a) in n.arguments.iter().enumerate() {
            let Some(e) = a.as_expression() else {
                bail!("{at}: new {class}: spread argument")
            };
            match e {
                Expression::ObjectExpression(o) if i > 0 || n.arguments.len() == 1 => {
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

    /// `go.addComponent(Camera, { .. })` for an engine component class (one with a static `attach`).
    pub(crate) fn lower_add_component(&mut self, recv: Expr, c: &'a CallExpression<'a>) -> R<Expr> {
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
        for p in &o.properties {
            let ObjectPropertyKind::ObjectProperty(p) = p else {
                bail!("{at}: spread in options")
            };
            let PropertyKey::StaticIdentifier(key) = &p.key else {
                bail!("{at}: option names are plain names")
            };
            let key = key.name.as_str();
            let own = self.manifest.classes[class].setters.get(key).cloned();
            if let Some(setter) = own {
                let want = self.ext_ir(&setter.params[1].1);
                let v = self.lower_expr(&p.value, Some(&want))?;
                let v = self.coerce(v, &want)?;
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
