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

    pub(crate) fn field_ix(&self, sid: StructId, name: &str) -> usize {
        self.m.structs[sid]
            .field(name)
            .unwrap_or_else(|| panic!("prelude class lost its field `{name}`"))
    }

    pub(crate) fn field_of(&self, obj: Expr, sid: StructId, name: &str) -> Expr {
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

    pub(crate) fn store_field(&self, obj: &Expr, sid: StructId, name: &str, v: Expr) -> Stmt {
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

// ---- the String methods that take a regular expression ----

impl<'a> Lowerer<'a> {
    /// The capture groups (and their names) of the regular expression an expression is, when the
    /// source says so: a literal, `new RegExp("literal")`, or a name declared from one.
    pub(crate) fn regex_groups_of(&self, e: &Expression<'_>) -> Option<Vec<Option<String>>> {
        match e {
            Expression::ParenthesizedExpression(p) => self.regex_groups_of(&p.expression),
            Expression::RegExpLiteral(r) => {
                let raw = r.raw.as_ref()?.as_str();
                let end = raw.rfind('/')?;
                Some(iron_ts_rt::regex_groups(&raw[1..end]))
            }
            Expression::NewExpression(n) if matches!(&n.callee, Expression::Identifier(id) if id.name == "RegExp") => {
                match n.arguments.first().and_then(|a| a.as_expression()) {
                    Some(Expression::StringLiteral(s)) => {
                        Some(iron_ts_rt::regex_groups(s.value.as_str()))
                    }
                    _ => None,
                }
            }
            Expression::Identifier(id) => self.regex_info.get(id.name.as_str()).cloned(),
            _ => None,
        }
    }

    /// Remembers the groups of a regex a name is declared from (for `replace` callbacks).
    pub(crate) fn note_regex(&mut self, name: &str, init: Option<&Expression<'_>>) {
        match init.and_then(|e| self.regex_groups_of(e)) {
            Some(g) => {
                self.regex_info.insert(name.to_string(), g);
            }
            None => {
                self.regex_info.remove(name);
            }
        }
    }

    /// The regular expression for a pattern argument: itself, or a string made into one (`escape`
    /// matches it literally, as `replace("a.b", ..)` does).
    fn regex_arg(
        &mut self,
        arg: Expr,
        is_re: bool,
        escape: bool,
        flags: &str,
        at: &str,
    ) -> R<Expr> {
        if is_re {
            return Ok(arg);
        }
        let arg = self.coerce(arg, &Ty::Str)?;
        let pattern = if escape {
            self.rt_str("ts_rt_regex_escape_gc", vec![arg], Ty::Str)
        } else {
            arg
        };
        let fl = self.str_lit(flags);
        self.make_regexp(pattern, fl, at)
    }

    fn throw_type_error(&mut self, message: &str) -> R<Stmt> {
        let Some(&fid) = self.fids.get("__rtThrow") else {
            bail!("the error classes are missing")
        };
        let msg = self.str_lit(message);
        Ok(Stmt::Eval(mk(
            ExprKind::Call(fid, vec![num(2.0), msg]),
            Ty::Void,
        )))
    }

    /// `text.match(re)`, `matchAll`, `search`, `replace`, `replaceAll` and `split(re)`. None when the
    /// call is an ordinary string call (`split` with a string).
    pub(crate) fn lower_regex_string_method(
        &mut self,
        text: Expr,
        m: &str,
        c: &'a CallExpression<'a>,
    ) -> R<Option<Expr>> {
        let Some(rsid) = self.regexp_sid() else {
            return Ok(None);
        };
        let at = self.loc(c.span.start);
        let Some(arg0e) = c.arguments.first().and_then(|a| a.as_expression()) else {
            return Ok(None);
        };
        let arg0 = self.lower_expr(arg0e, Some(&Ty::Str))?;
        let arg0 = self.widen_lit(arg0);
        let is_re = matches!(arg0.ty.strip(), Ty::Obj(s) if *s == rsid);
        if m == "split" && !is_re && c.arguments.len() < 2 {
            return Ok(None);
        }
        let groups_info = self.regex_groups_of(arg0e);
        let msid = self.match_sid().expect("the prelude defines both classes");
        let rty = Ty::Obj(rsid);
        let (t, r) = (self.new_local(Ty::Str), self.new_local(rty.clone()));
        let (tv, rv) = (self.var_expr(t), self.var_expr(r));
        let id = self.field_of(rv.clone(), rsid, "id");
        let global = self.field_of(rv.clone(), rsid, "global");
        let mut stmts = vec![Stmt::Assign(t, text)];
        let mty = Ty::Obj(msid);
        match m {
            "split" => {
                // A string separator with a limit goes the same way, matched literally.
                let re = self.regex_arg(arg0, is_re, true, "", &at)?;
                stmts.push(Stmt::Assign(r, re));
                let limit = match c.arguments.get(1).and_then(|a| a.as_expression()) {
                    Some(e) => {
                        let v = self.lower_expr(e, Some(&Ty::Num))?;
                        self.as_num(v)?
                    }
                    None => num(f64::NAN),
                };
                let ty = Ty::Arr(Box::new(Ty::Str));
                let call = self.rt_str("ts_rt_regex_split_gc", vec![id, tv, limit], ty.clone());
                Ok(Some(mk(ExprKind::Seq(stmts, Box::new(call)), ty)))
            }
            "search" => {
                let re = self.regex_arg(arg0, is_re, false, "", &at)?;
                stmts.push(Stmt::Assign(r, re));
                let call = self.rt_str("ts_rt_regex_search", vec![id, tv], Ty::Num);
                Ok(Some(mk(ExprKind::Seq(stmts, Box::new(call)), Ty::Num)))
            }
            "match" => {
                let re = self.regex_arg(arg0, is_re, false, "", &at)?;
                stmts.push(Stmt::Assign(r, re));
                let res = self.new_local(mty.clone().nullable());
                let arr_ty = Ty::Arr(Box::new(Ty::Str.nullable()));
                let all_ty = Ty::Arr(Box::new(Ty::Str));
                let a = self.new_local(all_ty.clone().nullable());
                let av = self.var_expr(a);
                let m_local = self.new_local(mty.clone());
                let mv = self.var_expr(m_local);
                // A global regex gives the text of every match; the object holds them as `matches`.
                let all = self.rt_str(
                    "ts_rt_regex_match_all_gc",
                    vec![id, tv.clone()],
                    all_ty.clone().nullable(),
                );
                let as_nullable = mk(ExprKind::Cast(Box::new(av.clone())), arr_ty);
                let build = vec![
                    Stmt::Assign(m_local, mk(ExprKind::NewObj(msid), mty.clone())),
                    self.store_field(&mv, msid, "matches", as_nullable),
                    self.store_field(&mv, msid, "index", num(0.0)),
                    self.store_field(&mv, msid, "input", tv.clone()),
                    self.store_field(
                        &mv,
                        msid,
                        "groups",
                        mk(
                            ExprKind::Null,
                            Ty::Map(Box::new(Ty::Str), Box::new(Ty::Str.nullable())).nullable(),
                        ),
                    ),
                    Stmt::Assign(
                        res,
                        mk(ExprKind::Cast(Box::new(mv)), mty.clone().nullable()),
                    ),
                ];
                let is_null = mk(ExprKind::IsNull(Box::new(av)), Ty::Bool);
                let global_branch = vec![
                    Stmt::Assign(a, all),
                    self.store_field(&rv, rsid, "lastIndex", num(0.0)),
                    Stmt::If(is_null, vec![], build),
                ];
                let exec = self.regex_run(rv.clone(), tv, false)?;
                stmts.push(Stmt::Assign(
                    res,
                    mk(ExprKind::Null, mty.clone().nullable()),
                ));
                stmts.push(Stmt::If(
                    global,
                    global_branch,
                    vec![Stmt::Assign(res, exec)],
                ));
                let resv = self.var_expr(res);
                Ok(Some(mk(
                    ExprKind::Seq(stmts, Box::new(resv)),
                    mty.nullable(),
                )))
            }
            "matchAll" => {
                let re = self.regex_arg(arg0, is_re, false, "g", &at)?;
                stmts.push(Stmt::Assign(r, re));
                let err = self.throw_type_error(
                    "String.prototype.matchAll argument must not be a non-global regular expression",
                )?;
                let not_global = mk(ExprKind::Un(UnOp::Not, Box::new(global)), Ty::Bool);
                stmts.push(Stmt::If(not_global, vec![err], vec![]));
                // Iterate a copy, so the original's lastIndex is not disturbed.
                let src = self.field_of(rv.clone(), rsid, "source");
                let fl = self.field_of(rv.clone(), rsid, "flags");
                let copy_re = self.make_regexp(src, fl, &at)?;
                let c2 = self.new_local(rty.clone());
                let c2v = self.var_expr(c2);
                stmts.push(Stmt::Assign(c2, copy_re));
                let li = self.field_of(rv.clone(), rsid, "lastIndex");
                stmts.push(self.store_field(&c2v, rsid, "lastIndex", li));
                let out_ty = Ty::Arr(Box::new(mty.clone()));
                let out = self.new_local(out_ty.clone());
                let outv = self.var_expr(out);
                stmts.push(Stmt::Assign(
                    out,
                    mk(ExprKind::Rt(RtFn::ArrNew, vec![num(0.0)]), out_ty.clone()),
                ));
                let label = self.new_label();
                let found = self.new_local(mty.clone().nullable());
                let foundv = self.var_expr(found);
                let run = self.regex_run(c2v.clone(), tv.clone(), false)?;
                let obj = mk(ExprKind::Cast(Box::new(foundv.clone())), mty.clone());
                let matches = self.field_of(obj.clone(), msid, "matches");
                let first = mk(
                    ExprKind::Cast(Box::new(mk(
                        ExprKind::Index(Box::new(matches), Box::new(num(0.0))),
                        Ty::Str.nullable(),
                    ))),
                    Ty::Str,
                );
                let empty = mk(
                    ExprKind::Cmp(
                        CmpOp::Eq,
                        Box::new(self.rt_str("ts_rt_str_length", vec![first], Ty::Num)),
                        Box::new(num(0.0)),
                    ),
                    Ty::Bool,
                );
                let c2id = self.field_of(c2v.clone(), rsid, "id");
                let c2li = self.field_of(c2v.clone(), rsid, "lastIndex");
                let next = self.rt_str("ts_rt_regex_advance", vec![c2id, tv, c2li], Ty::Num);
                let body = vec![
                    Stmt::Assign(found, run),
                    Stmt::If(
                        mk(ExprKind::IsNull(Box::new(foundv)), Ty::Bool),
                        vec![Stmt::Break(label)],
                        vec![],
                    ),
                    Stmt::Eval(mk(
                        ExprKind::Rt(RtFn::ArrPush, vec![outv.clone(), obj]),
                        Ty::Void,
                    )),
                    Stmt::If(
                        empty,
                        vec![self.store_field(&c2v, rsid, "lastIndex", next)],
                        vec![],
                    ),
                ];
                stmts.push(Stmt::Loop(label, body));
                Ok(Some(mk(ExprKind::Seq(stmts, Box::new(outv)), out_ty)))
            }
            "replace" | "replaceAll" => {
                let all = m == "replaceAll";
                let re = self.regex_arg(arg0, is_re, true, if all { "g" } else { "" }, &at)?;
                stmts.push(Stmt::Assign(r, re));
                if all && is_re {
                    let err = self.throw_type_error(
                        "String.prototype.replaceAll argument must not be a non-global regular expression",
                    )?;
                    let not_global =
                        mk(ExprKind::Un(UnOp::Not, Box::new(global.clone())), Ty::Bool);
                    stmts.push(Stmt::If(not_global, vec![err], vec![]));
                }
                let Some(repl_e) = c.arguments.get(1).and_then(|a| a.as_expression()) else {
                    bail!("{at}: {m}: missing replacement")
                };
                // A function written in the call is inlined; its parameter types come from here.
                let inline = matches!(
                    repl_e,
                    Expression::ArrowFunctionExpression(_) | Expression::FunctionExpression(_)
                );
                let repl = if inline {
                    None
                } else {
                    Some(self.lower_expr(repl_e, Some(&Ty::Str))?)
                };
                let wanted = match (&repl, repl_e) {
                    (_, Expression::ArrowFunctionExpression(f)) => Some(f.params.items.len()),
                    (_, Expression::FunctionExpression(f)) => Some(f.params.items.len()),
                    (Some(v), _) => match v.ty.strip() {
                        Ty::Fn(sig) => Some(sig.params.len()),
                        _ => None,
                    },
                    _ => None,
                };
                if let Some(wanted) = wanted {
                    // The pattern's groups are known from the source when it is a literal.
                    let groups = if is_re { groups_info } else { Some(Vec::new()) };
                    let out =
                        self.replace_with_function(rsid, r, t, repl_e, wanted, groups, &at)?;
                    stmts.extend(out.0);
                    return Ok(Some(mk(ExprKind::Seq(stmts, Box::new(out.1)), Ty::Str)));
                }
                let repl = repl.expect("not inlined");
                let repl = self.widen_lit(repl);
                let repl = self.coerce(repl, &Ty::Str)?;
                let call = self.rt_str(
                    "ts_rt_regex_replace_gc",
                    vec![id, tv, repl, global],
                    Ty::Str,
                );
                Ok(Some(mk(ExprKind::Seq(stmts, Box::new(call)), Ty::Str)))
            }
            _ => Ok(None),
        }
    }

    /// `text.replace(re, (match, p1, .., offset, string, groups) => ..)`: a loop that calls the
    /// callback for each match. Returns the statements and the result string.
    #[allow(clippy::too_many_arguments)]
    fn replace_with_function(
        &mut self,
        rsid: StructId,
        r: VarId,
        t: VarId,
        cb: &'a Expression<'a>,
        wanted: usize,
        groups: Option<Vec<Option<String>>>,
        at: &str,
    ) -> R<(Vec<Stmt>, Expr)> {
        let (rv, tv) = (self.var_expr(r), self.var_expr(t));
        let id = self.field_of(rv.clone(), rsid, "id");
        let global = self.field_of(rv.clone(), rsid, "global");
        let named = groups
            .as_ref()
            .is_some_and(|g| g.iter().any(|n| n.is_some()));
        if wanted > 1 && groups.is_none() {
            bail!(
                "{at}: a replacement function with more than the match as a parameter needs a regular expression written in the call or a `const` declared from one (its groups must be known)"
            )
        }
        let n = groups.as_ref().map_or(0, |g| g.len());
        let available = 1 + n + 2 + usize::from(named);
        if wanted > available {
            bail!(
                "{at}: the replacement function has {wanted} parameters; at most {available} are passed"
            )
        }
        let arr_ty = Ty::Arr(Box::new(Ty::Str.nullable()));
        let (a, idx, end) = (
            self.new_local(arr_ty.clone().nullable()),
            self.new_local(Ty::Num),
            self.new_local(Ty::Num),
        );
        let (out, last, pos) = (
            self.new_local(Ty::Str),
            self.new_local(Ty::Num),
            self.new_local(Ty::Num),
        );
        let label = self.new_label();
        let av = self.var_expr(a);
        let get = |this: &Self, i: usize, ty: Ty| {
            let el = mk(
                ExprKind::Index(
                    Box::new(mk(ExprKind::Cast(Box::new(av.clone())), arr_ty.clone())),
                    Box::new(num(i as f64)),
                ),
                Ty::Str.nullable(),
            );
            let _ = this;
            if ty == Ty::Str {
                mk(ExprKind::Cast(Box::new(el)), Ty::Str)
            } else {
                el
            }
        };
        // The parameters the callback takes, in order.
        let mut vars: Vec<VarId> = Vec::new();
        let mut binds: Vec<Stmt> = Vec::new();
        for i in 0..wanted {
            let (ty, value) = if i == 0 {
                (Ty::Str, get(self, 0, Ty::Str))
            } else if i <= n {
                (Ty::Str.nullable(), get(self, i, Ty::Str.nullable()))
            } else if i == n + 1 {
                (Ty::Num, self.var_expr(idx))
            } else if i == n + 2 {
                (Ty::Str, tv.clone())
            } else {
                let ty = Ty::Map(Box::new(Ty::Str), Box::new(Ty::Str.nullable())).nullable();
                let v = self.rt_str(
                    "ts_rt_regex_groups_gc",
                    vec![id.clone(), self.var_expr(a)],
                    ty.clone(),
                );
                (ty, v)
            };
            let v = self.new_local(ty);
            binds.push(Stmt::Assign(v, value));
            vars.push(v);
        }
        let res = self.new_local(Ty::Str);
        let cbs = self.inline_cb(cb, &vars, RetKind::Val, Some(res))?;
        let rty = self.cur.locals[res as usize].clone();
        let piece = self.stringify(self.var_expr(res))?;
        let _ = rty;
        let slice = |this: &Self, from: Expr, to: Expr| {
            this.rt_str("ts_rt_str_slice_gc", vec![tv.clone(), from, to], Ty::Str)
        };
        let exec = self.rt_str(
            "ts_rt_regex_exec_gc",
            vec![id.clone(), tv.clone(), self.var_expr(pos)],
            arr_ty.clone().nullable(),
        );
        let index = self.rt_str("ts_rt_regex_match_index", vec![], Ty::Num);
        let match_end = self.rt_str("ts_rt_regex_match_end", vec![], Ty::Num);
        let before = slice(self, self.var_expr(last), self.var_expr(idx));
        let joined = self.str_concat(self.var_expr(out), before)?;
        let with_piece = self.str_concat(self.var_expr(out), piece)?;
        let advance = self.rt_str(
            "ts_rt_regex_advance",
            vec![id.clone(), tv.clone(), self.var_expr(end)],
            Ty::Num,
        );
        let len = self.rt_str("ts_rt_str_length", vec![tv.clone()], Ty::Num);
        let lt = |a: Expr, b: Expr, op| mk(ExprKind::Cmp(op, Box::new(a), Box::new(b)), Ty::Bool);
        let empty_match = lt(self.var_expr(end), self.var_expr(idx), CmpOp::Eq);
        let mut body = vec![
            Stmt::Assign(a, exec),
            Stmt::If(
                mk(ExprKind::IsNull(Box::new(av.clone())), Ty::Bool),
                vec![Stmt::Break(label)],
                vec![],
            ),
            Stmt::Assign(idx, index),
            Stmt::Assign(end, match_end),
            Stmt::Assign(out, joined),
        ];
        body.extend(binds);
        body.push(cbs);
        body.push(Stmt::Assign(out, with_piece));
        body.push(Stmt::Assign(last, self.var_expr(end)));
        body.push(Stmt::If(
            mk(ExprKind::Un(UnOp::Not, Box::new(global.clone())), Ty::Bool),
            vec![Stmt::Break(label)],
            vec![],
        ));
        body.push(Stmt::Assign(pos, self.var_expr(end)));
        body.push(Stmt::If(
            empty_match,
            vec![Stmt::Assign(pos, advance)],
            vec![],
        ));
        body.push(Stmt::If(
            lt(self.var_expr(pos), len, CmpOp::Gt),
            vec![Stmt::Break(label)],
            vec![],
        ));
        let tail = slice(self, self.var_expr(last), num(f64::INFINITY));
        let result = self.str_concat(self.var_expr(out), tail)?;
        let reset = Stmt::If(
            global,
            vec![self.store_field(&rv, rsid, "lastIndex", num(0.0))],
            vec![],
        );
        let empty_str = self.str_lit("");
        let stmts = vec![
            Stmt::Assign(out, empty_str),
            Stmt::Assign(last, num(0.0)),
            Stmt::Assign(pos, num(0.0)),
            reset.clone(),
            Stmt::Loop(label, body),
            reset,
        ];
        Ok((stmts, result))
    }
}
