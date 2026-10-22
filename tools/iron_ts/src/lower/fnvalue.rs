//! Functions that are not declared in the program used as values: `const hex = Color.hex`,
//! `xs.map(Math.abs)`, `xs.map(Vector3.zero.add)` is not one of them. Each becomes a closure over a
//! small wrapper function that makes the call (`MakeClosure`, no environment).

use super::expr::mk;
use super::*;
use crate::manifest::ExternFn;

/// The `Math` functions and their number of parameters when used as values.
fn math_arity(f: &str) -> Option<usize> {
    Some(match f {
        "sqrt" | "abs" | "floor" | "sin" | "cos" | "sign" | "ceil" | "trunc" | "round"
        | "fround" | "clz32" | "tan" | "asin" | "acos" | "atan" | "exp" | "log" | "log2"
        | "log10" | "cbrt" | "sinh" | "cosh" | "tanh" | "asinh" | "acosh" | "atanh" | "expm1"
        | "log1p" => 1,
        "pow" | "hypot" | "atan2" | "imul" | "min" | "max" => 2,
        "random" => 0,
        _ => return None,
    })
}

impl<'a> Lowerer<'a> {
    /// A closure over a wrapper function of `ptys` parameters whose body returns what `build`
    /// makes of them. Wrappers are shared by `key`.
    fn wrapper_value(
        &mut self,
        key: &str,
        ptys: Vec<Ty>,
        build: impl FnOnce(&mut Self, Vec<Expr>) -> R<Expr>,
    ) -> R<Expr> {
        if let Some(&wid) = self.wrappers.get(key) {
            let (ptys, ret) = self.fsig[wid].clone();
            return Ok(mk(
                ExprKind::MakeClosure(wid, None),
                Ty::Fn(Box::new(FnTy {
                    params: ptys,
                    ret: ret.unwrap_or(Ty::Void),
                })),
            ));
        }
        let saved = std::mem::take(&mut self.cur);
        let env = self.new_local(Self::env_ty());
        let vars: Vec<VarId> = ptys.iter().map(|t| self.new_local(t.clone())).collect();
        let args: Vec<Expr> = vars.iter().map(|v| self.var_expr(*v)).collect();
        let built = build(self, args);
        let ctx = std::mem::replace(&mut self.cur, saved);
        let call = built?;
        let ret = call.ty.clone();
        let body = if ret == Ty::Void {
            vec![Stmt::Eval(call), Stmt::Return(None)]
        } else {
            vec![Stmt::Return(Some(call))]
        };
        let mut params = vec![env];
        params.extend(vars);
        let wid = self.m.funcs.len();
        self.m.funcs.push(Func {
            name: format!("{key}$value"),
            exported: false,
            params,
            ret: ret.clone(),
            locals: ctx.locals,
            body,
        });
        self.fsig.push((ptys.clone(), Some(ret.clone())));
        self.fstate.push(2);
        self.wrappers.insert(key.to_string(), wid);
        Ok(mk(
            ExprKind::MakeClosure(wid, None),
            Ty::Fn(Box::new(FnTy { params: ptys, ret })),
        ))
    }

    /// `Math.abs` as a value, or `None` for a name that is not a function of `Math`.
    pub(crate) fn math_value(&mut self, f: &str) -> R<Option<Expr>> {
        let Some(n) = math_arity(f) else {
            return Ok(None);
        };
        let name = f.to_string();
        let e = self.wrapper_value(&format!("Math.{f}"), vec![Ty::Num; n], move |_, args| {
            Self::math_op(&name, args).ok_or_else(|| format!("Math.{name} as a value"))
        })?;
        Ok(Some(e))
    }

    /// A static function of the engine API (`Color.hex`, `Mathf.clamp`) as a value. The closure
    /// takes the parameters the call cannot leave out; the others take their default.
    pub(crate) fn extern_value(&mut self, what: &str, f: ExternFn) -> R<Expr> {
        let given = f.optional_from.min(f.params.len());
        if f.rest {
            bail!("{what} has rest parameters and cannot be used as a value")
        }
        for i in given..f.params.len() {
            if f.defaults.get(i).copied().flatten().is_none() {
                bail!("{what} has optional parameters and cannot be used as a value")
            }
        }
        let ptys: Vec<Ty> = f.params[..given]
            .iter()
            .map(|(_, t)| self.ext_ir(t))
            .collect();
        let key = format!("ext:{}", f.symbol);
        self.wrapper_value(&key, ptys, move |this, mut args| {
            for i in given..f.params.len() {
                let d = f.defaults[i].expect("checked");
                args.push(match f.params[i].1 {
                    ExtTy::Bool => mk(ExprKind::Bool(d != 0.0), Ty::Bool),
                    _ => mk(ExprKind::Num(d), Ty::Num),
                });
            }
            Ok(this.emit_extern(&f, args))
        })
    }

    /// `Class.name` used as a value, where `Class` is not a local: a static method of a class of
    /// the program, of the engine API or of the facade.
    pub(crate) fn static_fn_value(&mut self, class: &str, name: &str) -> R<Option<Expr>> {
        if let Some(info) = self.classes.get(class)
            && let Some(f) = info.statics.get(name).cloned()
            && self.fids.contains_key(&f)
        {
            return self.function_value(&f).map(Some);
        }
        let facade = format!("__xt_{class}_{name}");
        if self.fids.contains_key(&facade) {
            return self.function_value(&facade).map(Some);
        }
        if let Some(f) = self
            .manifest
            .classes
            .get(class)
            .and_then(|k| k.statics.get(name))
            .cloned()
        {
            return self.extern_value(&format!("{class}.{name}"), f).map(Some);
        }
        if let Some(f) = self
            .manifest
            .namespaces
            .get(class)
            .and_then(|n| n.get(name))
            .cloned()
        {
            return self.extern_value(&format!("{class}.{name}"), f).map(Some);
        }
        Ok(None)
    }
}
