//! `Map`, `Set`, `WeakMap` and `WeakSet` (runtime: `iron_ts_rt/src/map.rs`). Keys are numbers,
//! strings (or string-literal unions, stored as strings), booleans, handles or objects (by
//! identity); a weak collection is a strong one (nothing can tell the difference except memory).
//!
//! Keys and values cross into the runtime as raw 64-bit words (`ExprKind::Bits` / `Unbits`).
//! Iteration (`for...of`, `forEach`) is live: it keeps a cursor, so entries deleted or added while
//! it runs behave as in JavaScript. `keys()` / `values()` outside a `for...of`, spreads and
//! `Array.from` take a snapshot array, which is what JavaScript does there too.

use super::expr::{Elem, mk, num};
use super::*;

/// The key type as stored: a string-literal union is a string.
pub(crate) fn map_key_ty(t: Ty) -> Ty {
    if t == Ty::Lit { Ty::Str } else { t }
}

fn key_kind(t: &Ty) -> R<f64> {
    Ok(match t.strip() {
        Ty::Num => 0.0,
        Ty::Str | Ty::Lit => 1.0,
        Ty::Bool | Ty::Handle(_) => 3.0,
        t if t.is_ptr() => 2.0,
        t => bail!("{t} cannot be a Map or Set key"),
    })
}

/// A loop over the entries of a collection with a cursor.
pub(crate) struct CollIter {
    pub pre: Vec<Stmt>,
    pub label: LabelId,
    pub head: Vec<Stmt>,
    pub map: VarId,
    pub key: VarId,
    pub val: VarId,
}

impl<'a> Lowerer<'a> {
    fn bits(&self, e: Expr) -> Expr {
        mk(ExprKind::Bits(Box::new(e)), Ty::Handle("bits".into()))
    }

    fn unbits(&self, e: Expr, ty: Ty) -> Expr {
        mk(ExprKind::Unbits(Box::new(e)), ty)
    }

    fn rt_map(&self, sym: &'static str, args: Vec<Expr>, ty: Ty) -> Expr {
        mk(ExprKind::Rt(RtFn::Str(sym), args), ty)
    }

    /// `(key type, value type)` of a map or set (the value type of a set is `boolean`, unused).
    fn parts(t: &Ty) -> Option<(Ty, Ty, bool)> {
        match t.strip() {
            Ty::Map(k, v) => Some(((**k).clone(), (**v).clone(), false)),
            Ty::Set(k) => Some(((**k).clone(), Ty::Bool, true)),
            _ => None,
        }
    }

    /// A key expression as the key type, then as a word.
    fn key_word(&mut self, key: Expr, k: &Ty) -> R<Expr> {
        let key = self.widen_lit(key);
        let key = self.coerce(key, k)?;
        Ok(self.bits(key))
    }

    fn val_word(&mut self, v: Expr, ty: &Ty) -> R<Expr> {
        let v = self.coerce(v, ty)?;
        Ok(self.bits(v))
    }

    /// A new collection of this type.
    pub(crate) fn new_collection(&mut self, ty: &Ty) -> R<Expr> {
        let (k, v, _) = Self::parts(ty).expect("a map or set type");
        let kk = key_kind(&k)?;
        let vp = if v.is_ptr() { 1.0 } else { 0.0 };
        Ok(self.rt_map("ts_rt_map_new_gc", vec![num(kk), num(vp)], ty.clone()))
    }

    /// `new Map(..)`, `new Set(..)`, `new WeakMap()`, `new WeakSet()`.
    pub(crate) fn lower_new_collection(
        &mut self,
        name: &str,
        n: &'a NewExpression<'a>,
        exp: Option<&Ty>,
    ) -> R<Expr> {
        let at = self.loc(n.span.start);
        let is_set = name.ends_with("Set");
        // The type: type arguments, else the context.
        let mut ty = None;
        if let Some(args) = &n.type_arguments {
            let ps: Vec<&'a TSType<'a>> = args.params.iter().collect();
            ty = Some(if is_set {
                let Some(k) = ps.first() else {
                    bail!("{at}: {name} needs a type argument")
                };
                Ty::Set(Box::new(map_key_ty(self.ty(k, name)?)))
            } else {
                let (Some(k), Some(v)) = (ps.first(), ps.get(1)) else {
                    bail!("{at}: {name} needs two type arguments")
                };
                Ty::Map(
                    Box::new(map_key_ty(self.ty(k, name)?)),
                    Box::new(self.ty(v, name)?),
                )
            });
        }
        if ty.is_none()
            && let Some(t) = exp
            && matches!(t.strip(), Ty::Map(..) | Ty::Set(_))
        {
            ty = Some(t.strip().clone());
        }
        let arg = n.arguments.first().and_then(|a| a.as_expression());
        // `new Set(items)`: the element type of the array.
        let mut source: Option<Expr> = None;
        if ty.is_none()
            && is_set
            && let Some(e) = arg
        {
            let a = self.lower_expr(e, None)?;
            if let Ty::Arr(el) = a.ty.strip().clone() {
                ty = Some(Ty::Set(Box::new(map_key_ty(*el))));
                source = Some(a);
            }
        }
        let Some(ty) = ty else {
            bail!(
                "{at}: `new {name}` needs a type: `new {name}<{}>()` or an annotation",
                if is_set { "string" } else { "string, number" }
            )
        };
        let (k, v, _) = Self::parts(&ty).expect("a map or set type");
        let m = self.new_local(ty.clone());
        let mut stmts = vec![Stmt::Assign(m, self.new_collection(&ty)?)];
        if let Some(e) = arg {
            if is_set {
                let a = match source {
                    Some(a) => a,
                    None => self.lower_expr(e, Some(&Ty::Arr(Box::new(k.clone()))))?,
                };
                if !matches!(a.ty.strip(), Ty::Arr(_)) {
                    bail!("{at}: new {name}(..) takes an array")
                }
                let it = self.begin_iter(a)?;
                stmts.extend(it.pre.clone());
                let mut body = it.head.clone();
                let key = self.var_expr(it.elem);
                let word = self.key_word(key, &k)?;
                let unit = self.bits(num(0.0));
                body.push(Stmt::Eval(self.rt_map(
                    "ts_rt_map_set",
                    vec![self.var_expr(m), word, unit],
                    Ty::Void,
                )));
                stmts.push(Stmt::Loop(it.label, body));
            } else {
                // `new Map([[k, v], ...])`: pairs written out.
                let Expression::ArrayExpression(list) = e else {
                    bail!("{at}: new {name}(..) takes an array literal of [key, value] pairs")
                };
                for item in &list.elements {
                    let Some(Expression::ArrayExpression(pair)) = item.as_expression() else {
                        bail!("{at}: new {name}(..) takes [key, value] pairs")
                    };
                    let (Some(ke), Some(ve)) = (
                        pair.elements.first().and_then(|x| x.as_expression()),
                        pair.elements.get(1).and_then(|x| x.as_expression()),
                    ) else {
                        bail!("{at}: new {name}(..) takes [key, value] pairs")
                    };
                    let kv = self.lower_expr(ke, Some(&k))?;
                    let kw = self.key_word(kv, &k)?;
                    let vv = self.lower_expr(ve, Some(&v))?;
                    let vw = self.val_word(vv, &v)?;
                    let kt = self.new_local(Ty::Handle("bits".into()));
                    let vt = self.new_local(Ty::Handle("bits".into()));
                    stmts.push(Stmt::Assign(kt, kw));
                    stmts.push(Stmt::Assign(vt, vw));
                    stmts.push(Stmt::Eval(self.rt_map(
                        "ts_rt_map_set",
                        vec![self.var_expr(m), self.var_expr(kt), self.var_expr(vt)],
                        Ty::Void,
                    )));
                }
            }
        }
        Ok(mk(ExprKind::Seq(stmts, Box::new(self.var_expr(m))), ty))
    }

    /// `groups.name` on a `Map<string, V>`: the entry for that name, or null.
    pub(crate) fn map_prop(&mut self, map: Expr, prop: &str) -> R<Expr> {
        let (k, v, _) = Self::parts(&map.ty).expect("a map");
        let t = self.new_local(map.ty.clone());
        let tv = self.var_expr(t);
        let key = self.str_lit(prop);
        let kw = self.key_word(key, &k)?;
        let idx = self.new_local(Ty::Num);
        let iv = self.var_expr(idx);
        let found = self.rt_map("ts_rt_map_find", vec![tv.clone(), kw], Ty::Num);
        let raw = self.rt_map(
            "ts_rt_map_value_at",
            vec![tv, iv.clone()],
            Ty::Handle("bits".into()),
        );
        let rty = v.clone().nullable();
        let hit = mk(
            ExprKind::Cast(Box::new(self.unbits(raw, v.strip().clone()))),
            rty.clone(),
        );
        let miss = mk(ExprKind::Null, rty.clone());
        let missing = mk(
            ExprKind::Cmp(CmpOp::Lt, Box::new(iv), Box::new(num(0.0))),
            Ty::Bool,
        );
        Ok(mk(
            ExprKind::Seq(
                vec![Stmt::Assign(t, map), Stmt::Assign(idx, found)],
                Box::new(mk(
                    ExprKind::Cond(Box::new(missing), Box::new(miss), Box::new(hit)),
                    rty.clone(),
                )),
            ),
            rty,
        ))
    }

    /// `map.size` / `set.size`.
    pub(crate) fn collection_size(&self, c: Expr) -> Expr {
        self.rt_map("ts_rt_map_size", vec![c], Ty::Num)
    }

    /// A cursor loop over a collection, for `forEach` and `for...of`.
    pub(crate) fn begin_coll_iter(&mut self, c: Expr) -> R<CollIter> {
        let Some((k, v, is_set)) = Self::parts(&c.ty) else {
            bail!("cannot iterate {}", c.ty)
        };
        let map = self.new_local(c.ty.clone());
        let (seq, hint, idx) = (
            self.new_local(Ty::Num),
            self.new_local(Ty::Num),
            self.new_local(Ty::Num),
        );
        let (key, val) = (self.new_local(k.clone()), self.new_local(v.clone()));
        let label = self.new_label();
        let m = || self.var_expr(map);
        let next = self.rt_map(
            "ts_rt_map_next",
            vec![m(), self.var_expr(seq), self.var_expr(hint)],
            Ty::Num,
        );
        let done = mk(
            ExprKind::Cmp(CmpOp::Lt, Box::new(self.var_expr(idx)), Box::new(num(0.0))),
            Ty::Bool,
        );
        let at = |this: &Self, sym: &'static str, ty: Ty| {
            this.rt_map(sym, vec![m(), this.var_expr(idx)], ty)
        };
        let key_w = at(self, "ts_rt_map_key_at", Ty::Handle("bits".into()));
        let mut head = vec![
            Stmt::Assign(idx, next),
            Stmt::If(done, vec![Stmt::Break(label)], vec![]),
            Stmt::Assign(key, self.unbits(key_w, k)),
        ];
        if !is_set {
            let val_w = at(self, "ts_rt_map_value_at", Ty::Handle("bits".into()));
            head.push(Stmt::Assign(val, self.unbits(val_w, v)));
        }
        head.push(Stmt::Assign(seq, at(self, "ts_rt_map_seq_at", Ty::Num)));
        head.push(Stmt::Assign(hint, self.var_expr(idx)));
        let pre = vec![
            Stmt::Assign(map, c),
            Stmt::Assign(seq, num(0.0)),
            Stmt::Assign(hint, num(-1.0)),
        ];
        Ok(CollIter {
            pre,
            label,
            head,
            map,
            key,
            val,
        })
    }

    /// A method call on a `Map` or a `Set`.
    pub(crate) fn lower_collection_method(
        &mut self,
        recv: Expr,
        m: &str,
        c: &'a CallExpression<'a>,
    ) -> R<Expr> {
        let at = self.loc(c.span.start);
        let (k, v, is_set) = Self::parts(&recv.ty).expect("a map or set");
        let ty = recv.ty.strip().clone();
        let arg = |this: &mut Self, i: usize, want: &Ty| -> R<Expr> {
            match c.arguments.get(i).and_then(|a| a.as_expression()) {
                Some(e) => this.lower_expr(e, Some(want)),
                None => bail!("{at}: {m}: missing argument"),
            }
        };
        let t = self.new_local(ty.clone());
        let tv = self.var_expr(t);
        match (m, is_set) {
            ("get", false) => {
                if !(v.is_ptr() || v == Ty::Num) {
                    bail!(
                        "{at}: Map.get over {v} values is not supported (the result could not be told from a missing key); use has() first"
                    )
                }
                let key = arg(self, 0, &k)?;
                let kw = self.key_word(key, &k)?;
                let idx = self.new_local(Ty::Num);
                let iv = self.var_expr(idx);
                let found = self.rt_map("ts_rt_map_find", vec![tv.clone(), kw], Ty::Num);
                let raw = self.rt_map(
                    "ts_rt_map_value_at",
                    vec![tv.clone(), iv.clone()],
                    Ty::Handle("bits".into()),
                );
                let rty = v.clone().nullable();
                let hit = mk(
                    ExprKind::Cast(Box::new(self.unbits(raw, v.clone()))),
                    rty.clone(),
                );
                let miss = mk(ExprKind::Null, rty.clone());
                let missing = mk(
                    ExprKind::Cmp(CmpOp::Lt, Box::new(iv), Box::new(num(0.0))),
                    Ty::Bool,
                );
                Ok(mk(
                    ExprKind::Seq(
                        vec![Stmt::Assign(t, recv), Stmt::Assign(idx, found)],
                        Box::new(mk(
                            ExprKind::Cond(Box::new(missing), Box::new(miss), Box::new(hit)),
                            rty.clone(),
                        )),
                    ),
                    rty,
                ))
            }
            ("set", false) | ("add", true) => {
                let key = arg(self, 0, &k)?;
                let kw = self.key_word(key, &k)?;
                let vw = if is_set {
                    self.bits(num(0.0))
                } else {
                    let vv = arg(self, 1, &v)?;
                    self.val_word(vv, &v)?
                };
                let (kt, vt) = (
                    self.new_local(Ty::Handle("bits".into())),
                    self.new_local(Ty::Handle("bits".into())),
                );
                Ok(mk(
                    ExprKind::Seq(
                        vec![
                            Stmt::Assign(t, recv),
                            Stmt::Assign(kt, kw),
                            Stmt::Assign(vt, vw),
                            Stmt::Eval(self.rt_map(
                                "ts_rt_map_set",
                                vec![tv.clone(), self.var_expr(kt), self.var_expr(vt)],
                                Ty::Void,
                            )),
                        ],
                        Box::new(tv),
                    ),
                    ty,
                ))
            }
            ("has", _) | ("delete", _) => {
                let key = arg(self, 0, &k)?;
                let kw = self.key_word(key, &k)?;
                if m == "delete" {
                    return Ok(mk(
                        ExprKind::Seq(
                            vec![Stmt::Assign(t, recv)],
                            Box::new(self.rt_map("ts_rt_map_delete", vec![tv, kw], Ty::Bool)),
                        ),
                        Ty::Bool,
                    ));
                }
                let found = self.rt_map("ts_rt_map_find", vec![tv, kw], Ty::Num);
                let has = mk(
                    ExprKind::Cmp(CmpOp::Ge, Box::new(found), Box::new(num(0.0))),
                    Ty::Bool,
                );
                Ok(mk(
                    ExprKind::Seq(vec![Stmt::Assign(t, recv)], Box::new(has)),
                    Ty::Bool,
                ))
            }
            ("clear", _) => Ok(mk(
                ExprKind::Seq(
                    vec![
                        Stmt::Assign(t, recv),
                        Stmt::Eval(self.rt_map("ts_rt_map_clear", vec![tv], Ty::Void)),
                    ],
                    Box::new(mk(ExprKind::Num(0.0), Ty::Void)),
                ),
                Ty::Void,
            )),
            ("forEach", _) => {
                let Some(cb) = c.arguments.first().and_then(|a| a.as_expression()) else {
                    bail!("{at}: forEach needs a callback")
                };
                let it = self.begin_coll_iter(recv)?;
                let mut body = it.head.clone();
                // (value, key, collection); for a set the value is the key.
                let params = if is_set {
                    [it.key, it.key, it.map]
                } else {
                    [it.val, it.key, it.map]
                };
                body.push(self.inline_cb(cb, &params, RetKind::Void, None)?);
                let mut stmts = it.pre.clone();
                stmts.push(Stmt::Loop(it.label, body));
                Ok(mk(
                    ExprKind::Seq(stmts, Box::new(mk(ExprKind::Num(0.0), Ty::Void))),
                    Ty::Void,
                ))
            }
            ("keys", _) | ("values", true) => {
                Ok(self.rt_map("ts_rt_map_keys_gc", vec![recv], Ty::Arr(Box::new(k))))
            }
            ("values", false) => {
                Ok(self.rt_map("ts_rt_map_values_gc", vec![recv], Ty::Arr(Box::new(v))))
            }
            ("entries", _) => {
                bail!("{at}: entries() is supported as `for (const [k, v] of x.entries())`")
            }
            _ => bail!("{at}: the method `{m}` of a Map or Set is not supported yet"),
        }
    }

    /// `[...set]`, `Array.from(set)`, `Array.from(map.values())`: the keys now, as an array.
    pub(crate) fn collection_snapshot(&self, c: Expr) -> Option<Expr> {
        match Self::parts(&c.ty) {
            Some((k, _, true)) => {
                Some(self.rt_map("ts_rt_map_keys_gc", vec![c], Ty::Arr(Box::new(k))))
            }
            _ => None,
        }
    }

    /// `for (const x of set)`, `for (const [k, v] of map)`, and `.entries()` / `.keys()` /
    /// `.values()` of either: a live cursor loop. `mode` is "", "entries", "keys" or "values".
    pub(crate) fn lower_for_of_collection(
        &mut self,
        f: &'a ForOfStatement<'a>,
        pat: &'a BindingPattern<'a>,
        coll: Expr,
        mode: &str,
        out: &mut Vec<Stmt>,
    ) -> R<()> {
        let (_, _, is_set) = Self::parts(&coll.ty).expect("a map or set");
        let it = self.begin_coll_iter(coll)?;
        self.cur.scopes.push(HashMap::new());
        let mut bind = Vec::new();
        let r = (|| -> R<()> {
            // A map yields [key, value]; a set yields the key; keys() the key; values() the value.
            let pair = !is_set && matches!(mode, "" | "entries");
            if pair {
                let BindingPattern::ArrayPattern(ap) = pat else {
                    bail!(
                        "{}: iterating a Map gives [key, value]: `for (const [k, v] of map)`",
                        self.loc(f.span.start)
                    )
                };
                if ap.elements.len() > 2 || ap.rest.is_some() {
                    bail!("a Map entry pattern has two parts: [key, value]")
                }
                if let Some(Some(p)) = ap.elements.first() {
                    self.bind_param(p, it.key, &mut bind)?;
                }
                if let Some(Some(p)) = ap.elements.get(1) {
                    self.bind_param(p, it.val, &mut bind)?;
                }
            } else if is_set && mode == "entries" {
                bail!("a Set's entries() are [value, value] pairs; iterate the set itself")
            } else {
                let var = if !is_set && mode == "values" {
                    it.val
                } else {
                    it.key
                };
                self.bind_param(pat, var, &mut bind)?;
            }
            Ok(())
        })();
        if let Err(e) = r {
            self.cur.scopes.pop();
            return Err(e);
        }
        let entry = self.loop_entry(it.label, Some(it.label));
        self.cur.loops.push(entry);
        let body = self.scoped(&f.body);
        self.cur.loops.pop();
        self.cur.scopes.pop();
        out.extend(it.pre);
        let mut b = it.head;
        b.extend(bind);
        b.extend(body?);
        out.push(Stmt::Loop(it.label, b));
        Ok(())
    }
}

#[allow(dead_code)]
fn _unused(_: Elem) {}
