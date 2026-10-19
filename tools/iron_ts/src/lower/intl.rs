//! `Intl`, step I1: `normalize`, `localeCompare`, `toLocaleUpperCase`/`toLocaleLowerCase` and
//! `Intl.Collator`. The work is in the runtime (`iron_ts_rt/src/intl.rs`, ICU4X). Locale arguments
//! are strings or array literals of strings; an options argument must be an object literal with
//! literal values (it is turned into a `key=value;...` string here, so the runtime never sees an
//! object). A collator is an object of the prelude class `Intl_Collator` holding an id.

use super::expr::mk;
use super::*;

impl<'a> Lowerer<'a> {
    fn is_undefined(e: &Expression<'_>) -> bool {
        matches!(e, Expression::Identifier(id) if id.name == "undefined")
    }

    /// The options object literal as `key=value;key=value`.
    pub(crate) fn options_spec(&self, e: Option<&'a Expression<'a>>, at: &str) -> R<String> {
        let Some(e) = e else {
            return Ok(String::new());
        };
        if Self::is_undefined(e) {
            return Ok(String::new());
        }
        let Expression::ObjectExpression(o) = e else {
            bail!(
                "{at}: Intl options must be an object literal (its values are read at compile time)"
            )
        };
        let mut out = String::new();
        for p in &o.properties {
            let ObjectPropertyKind::ObjectProperty(p) = p else {
                bail!("{at}: Intl options: spread is not supported")
            };
            let key = match &p.key {
                PropertyKey::StaticIdentifier(id) => id.name.to_string(),
                PropertyKey::StringLiteral(s) => s.value.to_string(),
                _ => bail!("{at}: Intl options: computed keys are not supported"),
            };
            // `"x" as const` and `("x")` are the literal too.
            let mut v = &p.value;
            loop {
                match v {
                    Expression::TSAsExpression(a) => v = &a.expression,
                    Expression::TSSatisfiesExpression(a) => v = &a.expression,
                    Expression::TSNonNullExpression(a) => v = &a.expression,
                    Expression::ParenthesizedExpression(a) => v = &a.expression,
                    _ => break,
                }
            }
            let value = match v {
                Expression::StringLiteral(s) => s.value.to_string(),
                Expression::BooleanLiteral(b) => b.value.to_string(),
                Expression::NumericLiteral(n) => n.value.to_string(),
                v if Self::is_undefined(v) => continue,
                _ => bail!("{at}: Intl option `{key}` must be a literal"),
            };
            out.push_str(&format!("{key}={value};"));
        }
        Ok(out)
    }

    /// A locales argument as the comma-separated string the runtime reads ("" for none).
    pub(crate) fn locales_arg(&mut self, e: Option<&'a Expression<'a>>) -> R<Expr> {
        let Some(e) = e else {
            return Ok(self.str_lit(""));
        };
        if Self::is_undefined(e) {
            return Ok(self.str_lit(""));
        }
        if let Expression::ArrayExpression(a) = e {
            let mut acc = self.str_lit("");
            for (i, el) in a.elements.iter().enumerate() {
                let Some(x) = el.as_expression() else {
                    bail!("a locale list cannot have holes or spreads")
                };
                let v = self.str_expr(x)?;
                if i > 0 {
                    let comma = self.str_lit(",");
                    acc = self.str_concat(acc, comma)?;
                }
                acc = self.str_concat(acc, v)?;
            }
            return Ok(acc);
        }
        self.str_expr(e)
    }

    pub(crate) fn str_expr(&mut self, e: &'a Expression<'a>) -> R<Expr> {
        let v = self.lower_expr(e, Some(&Ty::Str))?;
        let v = self.widen_lit(v);
        self.coerce(v, &Ty::Str)
    }

    /// `localeCompare`, `normalize`, `toLocaleUpperCase` and `toLocaleLowerCase`; None for any other
    /// method.
    pub(crate) fn lower_intl_str_method(
        &mut self,
        recv: Expr,
        m: &str,
        c: &'a CallExpression<'a>,
    ) -> R<Option<Expr>> {
        let at = self.loc(c.span.start);
        let arg = |i: usize| c.arguments.get(i).and_then(|a| a.as_expression());
        Ok(Some(match m {
            "localeCompare" => {
                let Some(that) = arg(0) else {
                    bail!("{at}: localeCompare needs an argument")
                };
                let that = self.str_expr(that)?;
                let loc = self.locales_arg(arg(1))?;
                let spec = self.options_spec(arg(2), &at)?;
                let opts = self.str_lit(&spec);
                let call = self.rt_str(
                    "ts_rt_str_locale_compare",
                    vec![recv, that, loc, opts],
                    Ty::Num,
                );
                self.checked(call, Ty::Num)
            }
            "normalize" => {
                let form = match arg(0) {
                    Some(e) if !Self::is_undefined(e) => self.str_expr(e)?,
                    _ => self.str_lit("NFC"),
                };
                let call = self.rt_str("ts_rt_str_normalize_gc", vec![recv, form], Ty::Str);
                self.checked(call, Ty::Str)
            }
            "toLocaleUpperCase" | "toLocaleLowerCase" => {
                let loc = self.locales_arg(arg(0))?;
                let up = mk(ExprKind::Bool(m == "toLocaleUpperCase"), Ty::Bool);
                let call = self.rt_str("ts_rt_str_locale_case_gc", vec![recv, loc, up], Ty::Str);
                self.checked(call, Ty::Str)
            }
            _ => return Ok(None),
        }))
    }

    /// `new Intl.Collator(locales?, options?)` and `new Intl.NumberFormat(locales?, options?)`: an
    /// object of the prelude class holding an id into the runtime's table.
    pub(crate) fn lower_new_intl(
        &mut self,
        kind: &str,
        args: &'a [Argument<'a>],
        at: u32,
    ) -> R<Expr> {
        let at = self.loc(at);
        let class = format!("Intl_{kind}");
        if !self.classes.contains_key(class.as_str()) {
            bail!("{at}: Intl.{kind} needs the Intl classes")
        }
        let sid = self.classes_sid(&class);
        let arg = |i: usize| args.get(i).and_then(|a| a.as_expression());
        let loc = self.locales_arg(arg(0))?;
        let spec = self.options_spec(arg(1), &at)?;
        let opts = self.str_lit(&spec);
        let sym = match kind {
            "Collator" => "ts_rt_collator_new",
            "DateTimeFormat" => "ts_rt_dtf_new",
            "PluralRules" => "ts_rt_plural_new",
            "ListFormat" => "ts_rt_list_new",
            "RelativeTimeFormat" => "ts_rt_rtf_new",
            _ => "ts_rt_number_format_new",
        };
        let made = self.rt_str(sym, vec![loc, opts], Ty::Num);
        let id = self.checked(made, Ty::Num);
        // The class constructor runs the field initializers (`compare` / `format` are closures over `this`).
        let made_obj = self.new_object(&class, &[], 0)?;
        let obj = self.new_local(Ty::Obj(sid));
        let ov = self.var_expr(obj);
        let stmts = vec![
            Stmt::Assign(obj, made_obj),
            self.store_field(&ov, sid, "id", id),
        ];
        Ok(mk(ExprKind::Seq(stmts, Box::new(ov)), Ty::Obj(sid)))
    }
}
