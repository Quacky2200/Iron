// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

//! `JSON.stringify` and `JSON.parse`. The program is typed, so the compiler describes the static
//! type of the value in a byte string (a *descriptor*, documented in `platypus_ts_rt/src/json.rs`) and
//! the runtime walks it. `JSON.parse` needs its type from the context (`const x: T = JSON.parse(s)`
//! or `JSON.parse(s) as T`).

use super::*;
use std::collections::HashMap as Map;

struct Builder {
    out: Vec<u8>,
    /// Engine handles are their ids (the records the engine API takes and gives), not `{}`.
    ids: bool,
    /// Struct bodies still to write: (struct id, class header word).
    queue: Vec<StructId>,
    /// Body offset of every struct whose body is written or queued, and the places that need it.
    body_at: Map<StructId, usize>,
    patches: Vec<(usize, StructId)>,
    /// Offsets of the struct header words (the module's base is added at code generation).
    headers: Vec<usize>,
    /// For `JSON.parse` (a union can be written but not read back).
    parse: bool,
}

impl Builder {
    fn u32(&mut self, v: u32) {
        self.out.extend_from_slice(&v.to_le_bytes());
    }
}

/// A key that JavaScript orders first (an array index): the canonical decimal form of a number
/// below 2^32 - 1.
fn array_index(name: &str) -> Option<u64> {
    if name.is_empty() || name.len() > 10 || !name.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    if name.len() > 1 && name.starts_with('0') {
        return None;
    }
    let n: u64 = name.parse().ok()?;
    (n < 4_294_967_295).then_some(n)
}

impl<'a> Lowerer<'a> {
    fn json_desc(&mut self, b: &mut Builder, ty: &Ty) -> R<()> {
        match ty {
            Ty::Num => b.out.push(0),
            Ty::Bool => b.out.push(1),
            Ty::Str => b.out.push(2),
            Ty::Lit => {
                b.out.push(3);
                let tags = self.m.tags.clone();
                b.u32(tags.len() as u32);
                for t in tags {
                    b.u32(t.len() as u32);
                    b.out.extend_from_slice(t.as_bytes());
                }
            }
            Ty::Arr(e) => {
                b.out.push(4);
                b.out.push(e.is_ptr() as u8);
                self.json_desc(b, e)?;
            }
            Ty::Nullable(inner) => {
                b.out.push(5);
                self.json_desc(b, inner)?;
            }
            Ty::Obj(sid)
                if self.m.structs[*sid].fields.iter().all(|f| f.ty == Ty::Num)
                    && self.value_structs.values().any(|v| v == sid) =>
            {
                // An engine value class stringifies as its numbers, `[x,y,z]`.
                b.out.push(8);
                b.out.push(self.m.structs[*sid].fields.len() as u8);
                b.headers.push(b.out.len());
                b.out.extend_from_slice(&(*sid as u64 + 1).to_le_bytes());
                for f in &self.m.structs[*sid].fields {
                    b.out.push(f.name.len() as u8);
                    b.out.extend_from_slice(f.name.as_bytes());
                }
            }
            Ty::Obj(sid) if self.unions.contains_key(sid) => {
                if b.parse {
                    bail!("JSON.parse into a union (a tagged `A | B`) is not supported")
                }
                // Stringified as the member its tag names.
                let members = self.unions[sid].clone();
                b.out.push(11);
                b.out.push(members.len() as u8);
                for m in &members {
                    self.json_desc(b, m)?;
                }
            }
            Ty::Obj(sid) => {
                // The class and every class that extends it: an object may be a subclass.
                let mut family: Vec<StructId> = vec![*sid];
                let mut others: Vec<StructId> = self
                    .class_of_sid
                    .keys()
                    .copied()
                    .filter(|s| s != sid && self.is_subclass(*s, *sid))
                    .collect();
                others.sort_unstable();
                family.extend(others);
                b.out.push(6);
                b.u32(family.len() as u32);
                for s in family {
                    let header = s as u64 + 1;
                    b.headers.push(b.out.len());
                    b.out.extend_from_slice(&header.to_le_bytes());
                    b.patches.push((b.out.len(), s));
                    b.u32(0);
                    if let std::collections::hash_map::Entry::Vacant(e) = b.body_at.entry(s) {
                        e.insert(usize::MAX);
                        b.queue.push(s);
                    }
                }
            }
            // A dictionary of an engine record (`Gamepad.buttons`): its entries in order.
            Ty::Map(k, v) if b.ids && matches!(k.strip(), Ty::Str) => {
                b.out.push(10);
                b.out.push(v.is_ptr() as u8);
                self.json_desc(b, v)?;
            }
            // Engine objects (GameObject, Camera, ...) stringify as `{}` like host objects in a
            // browser: their members are not own properties. They cannot be parsed.
            Ty::Handle(_) if b.ids => b.out.push(9),
            Ty::Map(..) | Ty::Set(_) | Ty::Handle(_) => b.out.push(7),
            t => bail!("JSON of {t} values is not supported"),
        }
        Ok(())
    }

    /// The descriptor blob for a type, as an index into `Module::blobs`.
    pub(crate) fn json_blob(&mut self, ty: &Ty) -> R<u32> {
        self.json_blob_with(ty, false)
    }

    /// [`Self::json_blob`]; with `ids` an engine handle is its id.
    pub(crate) fn json_blob_with(&mut self, ty: &Ty, ids: bool) -> R<u32> {
        self.json_blob_for(ty, ids, false)
    }

    /// [`Self::json_blob_with`]; `parse` for a blob `JSON.parse` reads by.
    fn json_blob_for(&mut self, ty: &Ty, ids: bool, parse: bool) -> R<u32> {
        let mut b = Builder {
            ids,
            out: vec![0; 8],
            queue: Vec::new(),
            body_at: Map::new(),
            patches: Vec::new(),
            headers: Vec::new(),
            parse,
        };
        let root = b.out.len() as u32;
        b.out[4..8].copy_from_slice(&root.to_le_bytes());
        self.json_desc(&mut b, ty)?;
        while let Some(s) = b.queue.pop() {
            let at = b.out.len();
            b.body_at.insert(s, at);
            // An engine record (`Gamepad`) is its wire form wherever it is parsed or written: its
            // handles are ids and its dictionaries lists of pairs.
            let outer_ids = b.ids;
            if self.manifest.records.contains_key(&self.m.structs[s].name) {
                b.ids = true;
            }
            // Own, visible fields in JavaScript's key order: array indices ascending, then the rest.
            let mut fields: Vec<(String, Ty, usize, bool)> = self.m.structs[s]
                .fields
                .iter()
                .enumerate()
                .filter(|(_, f)| !f.name.starts_with("_p_"))
                .map(|(i, f)| (f.name.clone(), f.ty.clone(), i, f.optional))
                .collect();
            let mut indexed: Vec<_> = fields
                .iter()
                .filter(|f| array_index(&f.0).is_some())
                .cloned()
                .collect();
            if !indexed.is_empty() {
                indexed.sort_by_key(|f| array_index(&f.0));
                fields.retain(|f| array_index(&f.0).is_none());
                indexed.extend(fields);
                fields = indexed;
            }
            b.u32(fields.len() as u32);
            for (name, fty, slot, optional) in fields {
                b.u32(slot as u32);
                b.out.push(optional as u8);
                b.u32(name.len() as u32);
                b.out.extend_from_slice(name.as_bytes());
                self.json_desc(&mut b, &fty)?;
            }
            b.ids = outer_ids;
        }
        for (pos, s) in std::mem::take(&mut b.patches) {
            let at = b.body_at[&s] as u32;
            b.out[pos..pos + 4].copy_from_slice(&at.to_le_bytes());
        }
        let len = b.out.len() as u32;
        b.out[0..4].copy_from_slice(&len.to_le_bytes());
        self.m.blobs.push(Blob {
            bytes: b.out,
            headers: b.headers,
        });
        Ok(self.m.blobs.len() as u32 - 1)
    }

    fn blob_expr(&self, i: u32) -> Expr {
        super::expr::mk(ExprKind::Blob(i), Ty::Handle("blob".into()))
    }

    /// The JSON text of a value of a static type (what `JSON.stringify(v)` makes).
    pub(crate) fn json_text_of(&mut self, v: Expr) -> R<Expr> {
        use super::expr::{mk, num};
        let v = self.widen_lit(v);
        let blob = self.json_blob_with(&v.ty.clone(), true)?;
        // The value goes into a local first: the runtime allocates the text while it walks the
        // value, and only a local is a GC root.
        let held = self.new_local(v.ty.clone());
        let bits = mk(
            ExprKind::Bits(Box::new(self.var_expr(held))),
            Ty::Handle("bits".into()),
        );
        let text = self.rt_str(
            "ts_rt_json_stringify_gc",
            vec![
                self.blob_expr(blob),
                bits,
                num(f64::NAN),
                mk(ExprKind::Null, Ty::Str),
            ],
            Ty::Str,
        );
        Ok(mk(
            ExprKind::Seq(vec![Stmt::Assign(held, v)], Box::new(text)),
            Ty::Str,
        ))
    }

    /// An argument that is an `Option<Record>` of the engine API as JSON text, when it is written as
    /// an object literal, `null`, or a conditional of those: the keys written are the keys sent
    /// (each value stringified by its own static type, `null` kept), so `{ sunDirection: null }`
    /// differs from `{}`, which a struct with one `null` state for both cannot tell. `None` for any
    /// other expression (a variable, a spread: the record's struct is stringified instead).
    pub(crate) fn lower_record_literal(
        &mut self,
        e: &'a Expression<'a>,
        rn: &str,
    ) -> R<Option<Expr>> {
        use super::expr::mk;
        match e {
            Expression::ParenthesizedExpression(p) => self.lower_record_literal(&p.expression, rn),
            Expression::NullLiteral(_) => Ok(Some(self.str_lit("null"))),
            Expression::ConditionalExpression(c) => {
                let test = self.lower_expr(&c.test, None)?;
                let test = self.bool_of(test);
                let branch = |this: &mut Self, b: &'a Expression<'a>| -> R<Expr> {
                    if let Some(t) = this.lower_record_literal(b, rn)? {
                        return Ok(t);
                    }
                    let want = Ty::Obj(this.record_struct(rn)).nullable();
                    let v = this.lower_expr(b, Some(&want))?;
                    let v = this.coerce(v, &want)?;
                    this.json_text_of(v)
                };
                let yes = branch(self, &c.consequent)?;
                let no = branch(self, &c.alternate)?;
                Ok(Some(mk(
                    ExprKind::Cond(Box::new(test), Box::new(yes), Box::new(no)),
                    Ty::Str,
                )))
            }
            Expression::ObjectExpression(o) => {
                let fields = self.manifest.records[rn].clone();
                let mut text = self.str_lit("{");
                let mut first = true;
                for p in &o.properties {
                    let ObjectPropertyKind::ObjectProperty(p) = p else {
                        return Ok(None);
                    };
                    let PropertyKey::StaticIdentifier(key) = &p.key else {
                        return Ok(None);
                    };
                    let name = key.name.as_str();
                    let Some((_, f)) = fields.iter().find(|(n, _)| n == name) else {
                        bail!(
                            "{}: `{name}` is not a key of {rn}",
                            self.loc(key.span.start)
                        )
                    };
                    let want = self.rec_field_ty(f).nullable();
                    let v = self.lower_expr(&p.value, Some(&want))?;
                    let v = self.coerce_ext(v, &want)?;
                    let piece = self.json_text_of(v)?;
                    let head =
                        self.str_lit(&format!("{}\"{name}\":", if first { "" } else { "," }));
                    first = false;
                    text = self.str_concat(text, head)?;
                    text = self.str_concat(text, piece)?;
                }
                let close = self.str_lit("}");
                Ok(Some(self.str_concat(text, close)?))
            }
            _ => Ok(None),
        }
    }

    /// The value of type `ty` that JSON text holds (what `JSON.parse(text)` makes there).
    pub(crate) fn json_value_of(&mut self, text: Expr, ty: Ty) -> R<Expr> {
        self.json_value_with(text, ty, false)
    }

    /// [`Self::json_value_of`]; with `ids` an engine handle is read from its id.
    pub(crate) fn json_value_with(&mut self, text: Expr, ty: Ty, ids: bool) -> R<Expr> {
        use super::expr::mk;
        let blob = self.json_blob_for(&ty, ids, true)?;
        // The text goes into a local first: the parser allocates while it reads it.
        let held = self.new_local(Ty::Str);
        let raw = mk(
            ExprKind::Rt(
                RtFn::Str("ts_rt_json_parse_gc"),
                vec![self.blob_expr(blob), self.var_expr(held)],
            ),
            Ty::Handle("bits".into()),
        );
        let value = mk(ExprKind::Unbits(Box::new(raw)), ty.clone());
        let parsed = self.checked(value, ty.clone());
        Ok(mk(
            ExprKind::Seq(vec![Stmt::Assign(held, text)], Box::new(parsed)),
            ty,
        ))
    }

    /// `JSON.stringify(value)` and `JSON.stringify(value, null, space)`.
    pub(crate) fn lower_json_stringify(&mut self, c: &'a CallExpression<'a>) -> R<Expr> {
        use super::expr::{mk, num};
        let at = self.loc(c.span.start);
        let Some(e) = c.arguments.first().and_then(|a| a.as_expression()) else {
            bail!("{at}: JSON.stringify needs a value")
        };
        let v = self.lower_expr(e, None)?;
        let v = self.widen_lit(v);
        if let Some(r) = c.arguments.get(1).and_then(|a| a.as_expression())
            && !matches!(r, Expression::NullLiteral(_))
            && !matches!(r, Expression::Identifier(id) if id.name == "undefined")
        {
            bail!("{at}: JSON.stringify with a replacer is not supported yet")
        }
        if v.ty == Ty::Null {
            bail!("{at}: JSON.stringify(null): give the value a type")
        }
        let blob = self.json_blob(&v.ty.clone())?;
        let (num_space, str_space) = match c.arguments.get(2).and_then(|a| a.as_expression()) {
            None => (num(f64::NAN), mk(ExprKind::Null, Ty::Str)),
            Some(s) => {
                let sv = self.lower_expr(s, Some(&Ty::Num))?;
                match sv.ty.strip() {
                    Ty::Num => (sv, mk(ExprKind::Null, Ty::Str)),
                    Ty::Str | Ty::Lit => {
                        let sv = self.widen_lit(sv);
                        (num(f64::NAN), sv)
                    }
                    t => {
                        bail!("{at}: JSON.stringify space must be a number or a string, found {t}")
                    }
                }
            }
        };
        let bits = mk(ExprKind::Bits(Box::new(v)), Ty::Handle("bits".into()));
        Ok(self.rt_str(
            "ts_rt_json_stringify_gc",
            vec![self.blob_expr(blob), bits, num_space, str_space],
            Ty::Str,
        ))
    }

    /// `JSON.parse(text)` as `ty`, from the context.
    pub(crate) fn lower_json_parse(
        &mut self,
        c: &'a CallExpression<'a>,
        exp: Option<&Ty>,
    ) -> R<Expr> {
        let at = self.loc(c.span.start);
        let Some(e) = c.arguments.first().and_then(|a| a.as_expression()) else {
            bail!("{at}: JSON.parse needs a string")
        };
        if c.arguments.len() > 1 {
            bail!("{at}: JSON.parse with a reviver is not supported yet")
        }
        let Some(ty) = exp.map(|t| t.strip().clone()).filter(|t| *t != Ty::Void) else {
            bail!(
                "{at}: JSON.parse needs a type: `const x: T = JSON.parse(s)` or `JSON.parse(s) as T`"
            )
        };
        let text = self.lower_expr(e, Some(&Ty::Str))?;
        let text = self.widen_lit(text);
        let text = self.coerce(text, &Ty::Str)?;
        // The text goes through a local (a GC root): the parser allocates while it reads it.
        self.json_value_of(text, ty)
    }
}
