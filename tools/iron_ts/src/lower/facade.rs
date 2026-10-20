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

impl<'a> Lowerer<'a> {
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
