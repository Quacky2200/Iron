//! `JSON.stringify` and `JSON.parse`. The program is typed, so the compiler describes the static
//! type of the value in a byte string (a *descriptor*, documented in `iron_ts_rt/src/json.rs`) and
//! the runtime walks it. `JSON.parse` needs its type from the context (`const x: T = JSON.parse(s)`
//! or `JSON.parse(s) as T`).

use super::*;
use std::collections::HashMap as Map;

struct Builder {
    out: Vec<u8>,
    /// Struct bodies still to write: (struct id, class header word).
    queue: Vec<StructId>,
    /// Body offset of every struct whose body is written or queued, and the places that need it.
    body_at: Map<StructId, usize>,
    patches: Vec<(usize, StructId)>,
    /// Offsets of the struct header words (the module's base is added at code generation).
    headers: Vec<usize>,
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
            // Engine objects (GameObject, Camera, ...) stringify as `{}` like host objects in a
            // browser: their members are not own properties. They cannot be parsed.
            Ty::Map(..) | Ty::Set(_) | Ty::Handle(_) => b.out.push(7),
            t => bail!("JSON of {t} values is not supported"),
        }
        Ok(())
    }

    /// The descriptor blob for a type, as an index into `Module::blobs`.
    pub(crate) fn json_blob(&mut self, ty: &Ty) -> R<u32> {
        let mut b = Builder {
            out: vec![0; 8],
            queue: Vec::new(),
            body_at: Map::new(),
            patches: Vec::new(),
            headers: Vec::new(),
        };
        let root = b.out.len() as u32;
        b.out[4..8].copy_from_slice(&root.to_le_bytes());
        self.json_desc(&mut b, ty)?;
        while let Some(s) = b.queue.pop() {
            let at = b.out.len();
            b.body_at.insert(s, at);
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
        use super::expr::mk;
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
        let blob = self.json_blob(&ty)?;
        let raw = mk(
            ExprKind::Rt(
                RtFn::Str("ts_rt_json_parse_gc"),
                vec![self.blob_expr(blob), text],
            ),
            Ty::Handle("bits".into()),
        );
        let value = mk(ExprKind::Unbits(Box::new(raw)), ty.clone());
        Ok(self.checked(value, ty))
    }
}
