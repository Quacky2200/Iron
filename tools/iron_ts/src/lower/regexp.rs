//! `RegExp`: literals, `new RegExp`, `test` and `exec`, and the match object. A regular expression
//! is an object of the prelude class `RegExp` (its `lastIndex` is a plain field); the engine and
//! the registry of compiled expressions are in the runtime (`iron_ts_rt/src/regexp.rs`). A match is
//! a `RegExpMatchArray` object: `m[i]`, `m.length`, destructuring, spreading and the array methods
//! see its `matches` array; `m.index`, `m.input` and `m.groups` are its fields.

use super::expr::{mk, num};
use super::*;

impl<'a> Lowerer<'a> {
    pub(crate) fn regexp_sid(&self) -> Option<StructId> {
        self.classes
            .contains_key("RegExp")
            .then(|| self.classes_sid("RegExp"))
    }

    pub(crate) fn match_sid(&self) -> Option<StructId> {
        self.classes
            .contains_key("RegExpMatchArray")
            .then(|| self.classes_sid("RegExpMatchArray"))
    }

    fn field_ix(&self, sid: StructId, name: &str) -> usize {
        self.m.structs[sid]
            .field(name)
            .unwrap_or_else(|| panic!("prelude class lost its field `{name}`"))
    }

    fn field_of(&self, obj: Expr, sid: StructId, name: &str) -> Expr {
        let i = self.field_ix(sid, name);
        let ty = self.m.structs[sid].fields[i].ty.clone();
        mk(ExprKind::Field(Box::new(obj), sid, i), ty)
    }

    /// A match object seen as its array of strings (`m[1]`, `m.length`, `for (x of m)`).
    pub(crate) fn match_view(&self, e: Expr) -> Expr {
        match (self.match_sid(), e.ty.strip()) {
            (Some(ms), Ty::Obj(s)) if *s == ms => self.field_of(e, ms, "matches"),
            _ => e,
        }
    }

    fn store_field(&self, obj: &Expr, sid: StructId, name: &str, v: Expr) -> Stmt {
        Stmt::StoreField(obj.clone(), sid, self.field_ix(sid, name), v)
    }

    /// A new `RegExp` object for `source` and `flags` (strings). A bad pattern or flags is a
    /// `SyntaxError` when it runs.
    pub(crate) fn make_regexp(&mut self, source: Expr, flags: Expr, at: &str) -> R<Expr> {
        let Some(sid) = self.regexp_sid() else {
            bail!("{at}: RegExp needs the RegExp classes (the program has no mention of RegExp)")
        };
        let (src, fl) = (self.new_local(Ty::Str), self.new_local(Ty::Str));
        let (obj, id, canon) = (
            self.new_local(Ty::Obj(sid)),
            self.new_local(Ty::Num),
            self.new_local(Ty::Str),
        );
        let (ov, srcv, flv, idv, canonv) = (
            self.var_expr(obj),
            self.var_expr(src),
            self.var_expr(fl),
            self.var_expr(id),
            self.var_expr(canon),
        );
        let compile = self.rt_str("ts_rt_regex_new", vec![srcv.clone(), flv], Ty::Num);
        let compile = self.checked(compile, Ty::Num);
        let canon_e = self.rt_str("ts_rt_regex_flags_gc", vec![idv.clone()], Ty::Str);
        let source_e = self.rt_str("ts_rt_regex_source_gc", vec![srcv.clone()], Ty::Str);
        let mut stmts = vec![
            Stmt::Assign(src, source),
            Stmt::Assign(fl, flags),
            Stmt::Assign(obj, mk(ExprKind::NewObj(sid), Ty::Obj(sid))),
            Stmt::Assign(id, compile),
            Stmt::Assign(canon, canon_e),
            self.store_field(&ov, sid, "source", source_e),
            self.store_field(&ov, sid, "flags", canonv.clone()),
            self.store_field(&ov, sid, "id", idv),
        ];
        for (name, letter) in [
            ("global", "g"),
            ("ignoreCase", "i"),
            ("multiline", "m"),
            ("dotAll", "s"),
            ("unicode", "u"),
            ("unicodeSets", "v"),
            ("sticky", "y"),
            ("hasIndices", "d"),
        ] {
            let lit = self.str_lit(letter);
            let has = self.rt_str(
                "ts_rt_str_includes",
                vec![canonv.clone(), lit, num(0.0)],
                Ty::Bool,
            );
            stmts.push(self.store_field(&ov, sid, name, has));
        }
        let ty = Ty::Obj(sid);
        Ok(mk(ExprKind::Seq(stmts, Box::new(ov)), ty))
    }

    /// `/pattern/flags`.
    pub(crate) fn lower_regexp_literal(&mut self, r: &'a RegExpLiteral<'a>) -> R<Expr> {
        let at = self.loc(r.span.start);
        let Some(raw) = r.raw.as_ref().map(|s| s.as_str()) else {
            bail!("{at}: a regular expression literal without its source text")
        };
        let Some(end) = raw.rfind('/') else {
            bail!("{at}: malformed regular expression literal")
        };
        let (pattern, flags) = (&raw[1..end], &raw[end + 1..]);
        if let Err(e) = iron_ts_rt::regex_validate(pattern, flags) {
            bail!("{at}: {e}")
        }
        let (p, f) = (self.str_lit(pattern), self.str_lit(flags));
        self.make_regexp(p, f, &at)
    }

    /// `new RegExp(pattern, flags)` and `RegExp(pattern, flags)`.
    pub(crate) fn lower_new_regexp(&mut self, args: &'a [Argument<'a>], at: u32) -> R<Expr> {
        let at = self.loc(at);
        let Some(p) = args.first().and_then(|a| a.as_expression()) else {
            bail!("{at}: new RegExp needs a pattern")
        };
        let p = self.lower_expr(p, Some(&Ty::Str))?;
        let p = self.widen_lit(p);
        // `new RegExp(re)` copies a regular expression.
        if let (Some(sid), Ty::Obj(s)) = (self.regexp_sid(), p.ty.strip().clone())
            && s == sid
        {
            let tmp = self.new_local(p.ty.clone());
            let src = self.field_of(self.var_expr(tmp), sid, "source");
            let flags = match args.get(1).and_then(|a| a.as_expression()) {
                Some(f) => {
                    let f = self.lower_expr(f, Some(&Ty::Str))?;
                    self.widen_lit(f)
                }
                None => self.field_of(self.var_expr(tmp), sid, "flags"),
            };
            let made = self.make_regexp(src, flags, &at)?;
            let ty = made.ty.clone();
            return Ok(mk(
                ExprKind::Seq(vec![Stmt::Assign(tmp, p)], Box::new(made)),
                ty,
            ));
        }
        let p = self.coerce(p, &Ty::Str)?;
        let f = match args.get(1).and_then(|a| a.as_expression()) {
            Some(f) => {
                let f = self.lower_expr(f, Some(&Ty::Str))?;
                let f = self.widen_lit(f);
                self.coerce(f, &Ty::Str)?
            }
            None => self.str_lit(""),
        };
        self.make_regexp(p, f, &at)
    }

    /// `re.exec(text)` (the match object or null) or `re.test(text)` (a boolean).
    fn regex_run(&mut self, re: Expr, text: Expr, test: bool) -> R<Expr> {
        let rsid = self.regexp_sid().expect("checked by the caller");
        let msid = self.match_sid().expect("the prelude defines both classes");
        let arr_ty = Ty::Arr(Box::new(Ty::Str.nullable()));
        let mty = Ty::Obj(msid);
        let (r, t) = (self.new_local(Ty::Obj(rsid)), self.new_local(Ty::Str));
        let a = self.new_local(arr_ty.clone().nullable());
        let res = self.new_local(mty.clone().nullable());
        let m = self.new_local(mty.clone());
        let (rv, tv, av, mv) = (
            self.var_expr(r),
            self.var_expr(t),
            self.var_expr(a),
            self.var_expr(m),
        );
        let global = self.field_of(rv.clone(), rsid, "global");
        let sticky = self.field_of(rv.clone(), rsid, "sticky");
        let last_index = self.field_of(rv.clone(), rsid, "lastIndex");
        let id = self.field_of(rv.clone(), rsid, "id");
        let stateful = mk(ExprKind::Or(Box::new(global), Box::new(sticky)), Ty::Bool);
        let start = mk(
            ExprKind::Cond(
                Box::new(stateful.clone()),
                Box::new(last_index),
                Box::new(num(0.0)),
            ),
            Ty::Num,
        );
        let exec = self.rt_str(
            "ts_rt_regex_exec_gc",
            vec![id.clone(), tv.clone(), start],
            arr_ty.clone().nullable(),
        );
        let set_last = |this: &Self, v: Expr| {
            Stmt::If(
                stateful.clone(),
                vec![this.store_field(&rv, rsid, "lastIndex", v)],
                vec![],
            )
        };
        let is_null = mk(ExprKind::IsNull(Box::new(av.clone())), Ty::Bool);
        let mut stmts = vec![
            Stmt::Assign(r, re),
            Stmt::Assign(t, text),
            Stmt::Assign(a, exec),
        ];
        let end = self.rt_str("ts_rt_regex_match_end", vec![], Ty::Num);
        if test {
            stmts.push(Stmt::If(
                is_null.clone(),
                vec![set_last(self, num(0.0))],
                vec![set_last(self, end)],
            ));
            let found = mk(ExprKind::Un(UnOp::Not, Box::new(is_null)), Ty::Bool);
            return Ok(mk(ExprKind::Seq(stmts, Box::new(found)), Ty::Bool));
        }
        stmts.push(Stmt::Assign(
            res,
            mk(ExprKind::Null, mty.clone().nullable()),
        ));
        let nonnull = mk(ExprKind::Cast(Box::new(av.clone())), arr_ty.clone());
        let groups_ty = Ty::Map(Box::new(Ty::Str), Box::new(Ty::Str.nullable())).nullable();
        let groups = self.rt_str("ts_rt_regex_groups_gc", vec![id, av], groups_ty);
        let index = self.rt_str("ts_rt_regex_match_index", vec![], Ty::Num);
        let ok = vec![
            set_last(self, end),
            Stmt::Assign(m, mk(ExprKind::NewObj(msid), mty.clone())),
            self.store_field(&mv, msid, "matches", nonnull),
            self.store_field(&mv, msid, "index", index),
            self.store_field(&mv, msid, "input", tv),
            self.store_field(&mv, msid, "groups", groups),
            Stmt::Assign(
                res,
                mk(ExprKind::Cast(Box::new(mv)), mty.clone().nullable()),
            ),
        ];
        stmts.push(Stmt::If(is_null, vec![set_last(self, num(0.0))], ok));
        let resv = self.var_expr(res);
        Ok(mk(ExprKind::Seq(stmts, Box::new(resv)), mty.nullable()))
    }

    /// `String(re)`: `/source/flags`.
    pub(crate) fn regexp_to_string(&mut self, recv: Expr) -> R<Expr> {
        let sid = self.regexp_sid().expect("a RegExp receiver");
        let tmp = self.new_local(recv.ty.clone());
        let (slash, src, flags) = (
            self.str_lit("/"),
            self.field_of(self.var_expr(tmp), sid, "source"),
            self.field_of(self.var_expr(tmp), sid, "flags"),
        );
        let a = self.str_concat(slash.clone(), src)?;
        let b = self.str_concat(a, slash)?;
        let s = self.str_concat(b, flags)?;
        Ok(mk(
            ExprKind::Seq(vec![Stmt::Assign(tmp, recv)], Box::new(s)),
            Ty::Str,
        ))
    }

    /// A method call on a `RegExp` object.
    pub(crate) fn lower_regexp_method(
        &mut self,
        recv: Expr,
        m: &str,
        c: &'a CallExpression<'a>,
    ) -> R<Expr> {
        let at = self.loc(c.span.start);
        match m {
            "test" | "exec" => {
                let Some(e) = c.arguments.first().and_then(|a| a.as_expression()) else {
                    bail!("{at}: RegExp.{m}: missing argument")
                };
                let text = self.lower_expr(e, Some(&Ty::Str))?;
                let text = self.widen_lit(text);
                let text = self.coerce(text, &Ty::Str)?;
                self.regex_run(recv, text, m == "test")
            }
            "toString" => self.regexp_to_string(recv),
            _ => bail!("{at}: the RegExp method `{m}` is not supported yet"),
        }
    }
}
