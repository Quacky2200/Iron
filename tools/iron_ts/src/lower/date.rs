//! `Date`: an object of the prelude class `Date` whose one field `t` is the time value (milliseconds
//! since the epoch, NaN when invalid). The calendar, local time and parsing are in the runtime
//! (`iron_ts_rt/src/date.rs`); this file lowers the constructor, the statics and the methods.
//! `toString`, `toTimeString` and the `toLocale*` family need time zone names and locale data, which
//! come with `Intl`; until then they are compile errors.

use super::expr::{mk, num};
use super::*;

/// The getters: name -> (field, local).
const GETTERS: [(&str, usize, bool); 16] = [
    ("getFullYear", 0, true),
    ("getMonth", 1, true),
    ("getDate", 2, true),
    ("getHours", 3, true),
    ("getMinutes", 4, true),
    ("getSeconds", 5, true),
    ("getMilliseconds", 6, true),
    ("getDay", 7, true),
    ("getUTCFullYear", 0, false),
    ("getUTCMonth", 1, false),
    ("getUTCDate", 2, false),
    ("getUTCHours", 3, false),
    ("getUTCMinutes", 4, false),
    ("getUTCSeconds", 5, false),
    ("getUTCMilliseconds", 6, false),
    ("getUTCDay", 7, false),
];

/// The setters: name -> (first field, how many arguments at most, local).
const SETTERS: [(&str, usize, usize, bool); 14] = [
    ("setFullYear", 0, 3, true),
    ("setMonth", 1, 2, true),
    ("setDate", 2, 1, true),
    ("setHours", 3, 4, true),
    ("setMinutes", 4, 3, true),
    ("setSeconds", 5, 2, true),
    ("setMilliseconds", 6, 1, true),
    ("setUTCFullYear", 0, 3, false),
    ("setUTCMonth", 1, 2, false),
    ("setUTCDate", 2, 1, false),
    ("setUTCHours", 3, 4, false),
    ("setUTCMinutes", 4, 3, false),
    ("setUTCSeconds", 5, 2, false),
    ("setUTCMilliseconds", 6, 1, false),
];

impl<'a> Lowerer<'a> {
    pub(crate) fn date_sid(&self) -> Option<StructId> {
        self.classes
            .contains_key("Date")
            .then(|| self.classes_sid("Date"))
    }

    /// The time value of a `Date` expression (what `valueOf` gives); other values pass through.
    pub(crate) fn date_value(&self, e: Expr) -> Expr {
        match (self.date_sid(), e.ty.strip()) {
            (Some(ds), Ty::Obj(s)) if *s == ds => self.field_of(e, ds, "t"),
            _ => e,
        }
    }

    pub(crate) fn is_date(&self, e: &Expr) -> bool {
        matches!((self.date_sid(), e.ty.strip()), (Some(ds), Ty::Obj(s)) if *s == ds)
    }

    fn new_date(&mut self, t: Expr) -> Expr {
        let sid = self.date_sid().expect("the Date class");
        let obj = self.new_local(Ty::Obj(sid));
        let ov = self.var_expr(obj);
        let stmts = vec![
            Stmt::Assign(obj, mk(ExprKind::NewObj(sid), Ty::Obj(sid))),
            self.store_field(&ov, sid, "t", t),
        ];
        mk(ExprKind::Seq(stmts, Box::new(ov)), Ty::Obj(sid))
    }

    /// `new Date()`, `new Date(ms)`, `new Date(string)`, `new Date(date)` and
    /// `new Date(year, month, ...)`.
    pub(crate) fn lower_new_date(&mut self, args: &'a [Argument<'a>], at: u32) -> R<Expr> {
        let at = self.loc(at);
        let exprs: Vec<&'a Expression<'a>> =
            args.iter().filter_map(|a| a.as_expression()).collect();
        if exprs.len() != args.len() || exprs.len() > 7 {
            bail!("{at}: new Date takes at most seven plain arguments")
        }
        let t = match exprs.as_slice() {
            [] => self.rt_str("ts_rt_date_now", vec![], Ty::Num),
            [one] => {
                let v = self.lower_expr(one, None)?;
                let v = self.date_value(v);
                match v.ty.strip() {
                    Ty::Num => self.rt_str("ts_rt_date_clip", vec![v], Ty::Num),
                    Ty::Str | Ty::Lit => {
                        let s = self.widen_lit(v);
                        let s = self.coerce(s, &Ty::Str)?;
                        self.rt_str("ts_rt_date_parse", vec![s], Ty::Num)
                    }
                    t => {
                        bail!("{at}: new Date({t}) is not supported (a number, a string or a Date)")
                    }
                }
            }
            many => {
                let mut parts = vec![
                    num(0.0),
                    num(0.0),
                    num(1.0),
                    num(0.0),
                    num(0.0),
                    num(0.0),
                    num(0.0),
                ];
                for (i, e) in many.iter().enumerate() {
                    parts[i] = self.num_arg(e)?;
                }
                parts.push(mk(ExprKind::Bool(true), Ty::Bool));
                self.rt_str("ts_rt_date_from_parts", parts, Ty::Num)
            }
        };
        Ok(self.new_date(t))
    }

    /// `Date.now()`, `Date.UTC(..)` and `Date.parse(s)`.
    pub(crate) fn lower_date_static(&mut self, m: &str, c: &'a CallExpression<'a>) -> R<Expr> {
        let at = self.loc(c.span.start);
        let args: Vec<&'a Expression<'a>> = c
            .arguments
            .iter()
            .filter_map(|a| a.as_expression())
            .collect();
        match m {
            "now" => Ok(self.rt_str("ts_rt_date_now", vec![], Ty::Num)),
            "parse" => {
                let [s] = args.as_slice() else {
                    bail!("{at}: Date.parse takes one argument")
                };
                let v = self.lower_expr(s, Some(&Ty::Str))?;
                let v = self.widen_lit(v);
                let v = self.coerce(v, &Ty::Str)?;
                Ok(self.rt_str("ts_rt_date_parse", vec![v], Ty::Num))
            }
            "UTC" => {
                if args.is_empty() || args.len() > 7 {
                    bail!("{at}: Date.UTC takes one to seven arguments")
                }
                let mut parts = vec![
                    num(f64::NAN),
                    num(0.0),
                    num(1.0),
                    num(0.0),
                    num(0.0),
                    num(0.0),
                    num(0.0),
                ];
                for (i, e) in args.iter().enumerate() {
                    parts[i] = self.num_arg(e)?;
                }
                parts.push(mk(ExprKind::Bool(false), Ty::Bool));
                Ok(self.rt_str("ts_rt_date_from_parts", parts, Ty::Num))
            }
            _ => bail!("{at}: Date.{m} is not supported"),
        }
    }

    /// A method call on a `Date` object.
    pub(crate) fn lower_date_method(
        &mut self,
        recv: Expr,
        m: &str,
        c: &'a CallExpression<'a>,
    ) -> R<Expr> {
        let at = self.loc(c.span.start);
        let sid = self.date_sid().expect("the Date class");
        let args: Vec<&'a Expression<'a>> = c
            .arguments
            .iter()
            .filter_map(|a| a.as_expression())
            .collect();
        let this = self.new_local(Ty::Obj(sid));
        let (tv, thisv) = (self.var_expr(this), self.var_expr(this));
        let t = self.field_of(tv, sid, "t");
        let with_this = |stmts: Vec<Stmt>, mut pre: Vec<Stmt>, value: Expr, ty: Ty| {
            pre.extend(stmts);
            mk(ExprKind::Seq(pre, Box::new(value)), ty)
        };
        let bind = vec![Stmt::Assign(this, recv)];
        if let Some((_, f, local)) = GETTERS.iter().find(|g| g.0 == m) {
            let call = self.rt_str(
                "ts_rt_date_field",
                vec![t, num(*f as f64), mk(ExprKind::Bool(*local), Ty::Bool)],
                Ty::Num,
            );
            return Ok(with_this(vec![], bind, call, Ty::Num));
        }
        if let Some((_, first, max, local)) = SETTERS.iter().find(|g| g.0 == m) {
            if args.is_empty() || args.len() > *max {
                bail!("{at}: {m} takes one to {max} arguments")
            }
            let mut call_args = vec![
                t,
                num(*first as f64),
                num(args.len() as f64),
                mk(ExprKind::Bool(*local), Ty::Bool),
            ];
            for i in 0..4 {
                call_args.push(match args.get(i) {
                    Some(e) => self.num_arg(e)?,
                    None => num(0.0),
                });
            }
            let call = self.rt_str("ts_rt_date_set", call_args, Ty::Num);
            let store = self.store_field(&thisv, sid, "t", call);
            let value = self.field_of(thisv, sid, "t");
            return Ok(with_this(vec![store], bind, value, Ty::Num));
        }
        match m {
            "getTime" | "valueOf" => Ok(with_this(vec![], bind, t, Ty::Num)),
            "getTimezoneOffset" => {
                let call = self.rt_str("ts_rt_date_offset", vec![t], Ty::Num);
                Ok(with_this(vec![], bind, call, Ty::Num))
            }
            "setTime" => {
                let [e] = args.as_slice() else {
                    bail!("{at}: setTime takes one argument")
                };
                let v = self.num_arg(e)?;
                let clipped = self.rt_str("ts_rt_date_clip", vec![v], Ty::Num);
                let store = self.store_field(&thisv, sid, "t", clipped);
                let value = self.field_of(thisv, sid, "t");
                Ok(with_this(vec![store], bind, value, Ty::Num))
            }
            "toISOString" => {
                let call = self.rt_str("ts_rt_date_iso_gc", vec![t], Ty::Str);
                let call = self.checked(call, Ty::Str);
                Ok(with_this(vec![], bind, call, Ty::Str))
            }
            "toJSON" => {
                // null for an invalid date, else the ISO string.
                let ty = Ty::Str.nullable();
                let res = self.new_local(ty.clone());
                let iso = self.rt_str("ts_rt_date_iso_gc", vec![t.clone()], Ty::Str);
                let invalid = mk(
                    ExprKind::Cmp(CmpOp::Ne, Box::new(t.clone()), Box::new(t)),
                    Ty::Bool,
                );
                let stmts = vec![Stmt::If(
                    invalid,
                    vec![Stmt::Assign(res, mk(ExprKind::Null, ty.clone()))],
                    vec![Stmt::Assign(res, iso)],
                )];
                let value = self.var_expr(res);
                Ok(with_this(stmts, bind, value, ty))
            }
            "toUTCString" | "toGMTString" => {
                let call = self.rt_str("ts_rt_date_utc_string_gc", vec![t], Ty::Str);
                Ok(with_this(vec![], bind, call, Ty::Str))
            }
            "toDateString" => {
                let call = self.rt_str("ts_rt_date_date_string_gc", vec![t], Ty::Str);
                Ok(with_this(vec![], bind, call, Ty::Str))
            }
            "toString" | "toTimeString" | "toLocaleString" | "toLocaleDateString"
            | "toLocaleTimeString" => {
                bail!(
                    "{at}: Date.{m} needs time zone names and locale data, which come with Intl (not built yet); use toISOString, toUTCString or toDateString"
                )
            }
            _ => bail!("{at}: the Date method `{m}` is not supported"),
        }
    }
}
