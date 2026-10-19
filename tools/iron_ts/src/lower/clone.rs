//! `structuredClone(x)`: a deep copy of data, one lifted function per type (so a recursive type such
//! as a tree is a recursive function). Numbers, strings, booleans and bigints are shared (they are
//! immutable); arrays and objects are copied, typed arrays are copied, a class instance keeps its
//! class (JavaScript gives a plain object). Maps, sets and functions are not supported.
//! A cyclic structure does not end.

use super::expr::{mk, num};
use super::*;

impl<'a> Lowerer<'a> {
    pub(crate) fn lower_structured_clone(&mut self, c: &'a CallExpression<'a>) -> R<Expr> {
        let at = self.loc(c.span.start);
        let Some(arg) = c.arguments.first().and_then(|a| a.as_expression()) else {
            bail!("{at}: structuredClone needs a value")
        };
        let v = self.lower_expr(arg, None)?;
        self.clone_value(v)
            .map_err(|e| format!("{at}: structuredClone: {e}"))
    }

    /// A deep copy of the value of `v`.
    fn clone_value(&mut self, v: Expr) -> R<Expr> {
        let ty = v.ty.clone();
        if let Ty::Nullable(inner) = &ty {
            let plain = matches!(
                inner.strip(),
                Ty::Num | Ty::Bool | Ty::Str | Ty::Lit | Ty::BigInt | Ty::Handle(_) | Ty::Fn(_)
            );
            if plain {
                return Ok(v);
            }
            let fid = self.clone_fn(&ty)?;
            return Ok(mk(ExprKind::Call(fid, vec![v]), ty));
        }
        match ty.strip() {
            Ty::Num
            | Ty::Bool
            | Ty::Str
            | Ty::Lit
            | Ty::BigInt
            | Ty::Void
            | Ty::Null
            | Ty::Handle(_) => Ok(v),
            Ty::Typed(k) => {
                let kind = num(f64::from(k.code()));
                let all = num(f64::INFINITY);
                Ok(self.rt_str(
                    "ts_rt_typed_slice_gc",
                    vec![v, kind, num(0.0), all],
                    ty.clone(),
                ))
            }
            Ty::Fn(_) => bail!("a function cannot be cloned"),
            Ty::Map(..) | Ty::Set(_) => bail!("Map and Set are not supported yet"),
            Ty::Obj(_) | Ty::Arr(_) => {
                let fid = self.clone_fn(&ty)?;
                Ok(mk(ExprKind::Call(fid, vec![v]), ty))
            }
            t => bail!("a value of type {t} cannot be cloned"),
        }
    }

    /// The lifted function that copies a value of `ty`.
    fn clone_fn(&mut self, ty: &Ty) -> R<FuncId> {
        let key = format!("clone#{ty}");
        if let Some(&f) = self.wrappers.get(&key) {
            return Ok(f);
        }
        let id = self.m.funcs.len();
        self.m.funcs.push(Func {
            name: format!("clone{id}"),
            exported: false,
            params: vec![0],
            ret: ty.clone(),
            locals: vec![ty.clone()],
            body: vec![],
        });
        self.fsig.push((vec![ty.clone()], Some(ty.clone())));
        self.fstate.push(2);
        self.wrappers.insert(key, id);

        let x = mk(ExprKind::Var(0), ty.clone());
        let mut locals = vec![ty.clone()];
        let mut local = |t: &Ty| {
            locals.push(t.clone());
            (locals.len() - 1) as VarId
        };
        let body = match ty.clone() {
            Ty::Obj(sid) => {
                let out = local(ty);
                let ov = mk(ExprKind::Var(out), ty.clone());
                let mut body = vec![Stmt::Assign(
                    out,
                    self.rt_str("ts_rt_obj_copy_gc", vec![x.clone()], ty.clone()),
                )];
                let fields = self.m.structs[sid].fields.clone();
                for (i, f) in fields.iter().enumerate() {
                    let needs = !matches!(
                        f.ty.strip(),
                        Ty::Num
                            | Ty::Bool
                            | Ty::Str
                            | Ty::Lit
                            | Ty::BigInt
                            | Ty::Void
                            | Ty::Null
                            | Ty::Fn(_)
                            | Ty::Handle(_)
                    );
                    if !needs {
                        continue;
                    }
                    let read = mk(ExprKind::Field(Box::new(x.clone()), sid, i), f.ty.clone());
                    let copy = self.clone_value(read)?;
                    body.push(Stmt::StoreField(ov.clone(), sid, i, copy));
                }
                body.push(Stmt::Return(Some(ov)));
                body
            }
            Ty::Arr(el) => {
                let out = local(ty);
                let i = local(&Ty::Num);
                let (ov, iv) = (
                    mk(ExprKind::Var(out), ty.clone()),
                    mk(ExprKind::Var(i), Ty::Num),
                );
                let raw_ty = if el.is_ptr() || *el == Ty::Num {
                    (*el).clone().nullable()
                } else {
                    (*el).clone()
                };
                let raw = mk(
                    ExprKind::Index(Box::new(x.clone()), Box::new(iv.clone())),
                    raw_ty.clone(),
                );
                let item = if raw_ty == *el {
                    raw
                } else {
                    mk(ExprKind::Cast(Box::new(raw)), (*el).clone())
                };
                let copy = self.clone_value(item)?;
                let done = mk(
                    ExprKind::Cmp(
                        CmpOp::Ge,
                        Box::new(iv.clone()),
                        Box::new(mk(ExprKind::Len(Box::new(x.clone())), Ty::Num)),
                    ),
                    Ty::Bool,
                );
                vec![
                    Stmt::Assign(
                        out,
                        mk(ExprKind::Rt(RtFn::ArrNew, vec![num(0.0)]), ty.clone()),
                    ),
                    Stmt::Assign(i, num(0.0)),
                    Stmt::Loop(
                        0,
                        vec![
                            Stmt::If(done, vec![Stmt::Break(0)], vec![]),
                            Stmt::Eval(mk(
                                ExprKind::Rt(RtFn::ArrPush, vec![ov.clone(), copy]),
                                Ty::Void,
                            )),
                            Stmt::Assign(
                                i,
                                mk(
                                    ExprKind::Bin(BinOp::Add, Box::new(iv), Box::new(num(1.0))),
                                    Ty::Num,
                                ),
                            ),
                        ],
                    ),
                    Stmt::Return(Some(ov)),
                ]
            }
            Ty::Nullable(inner) => {
                let plain = mk(ExprKind::Cast(Box::new(x.clone())), (*inner).clone());
                let copy = self.clone_value(plain)?;
                let back = mk(ExprKind::Cast(Box::new(copy)), ty.clone());
                vec![
                    Stmt::If(
                        mk(ExprKind::IsNull(Box::new(x.clone())), Ty::Bool),
                        vec![Stmt::Return(Some(mk(ExprKind::Null, ty.clone())))],
                        vec![],
                    ),
                    Stmt::Return(Some(back)),
                ]
            }
            t => bail!("a value of type {t} cannot be cloned"),
        };
        let f = &mut self.m.funcs[id];
        f.locals = locals;
        f.body = body;
        Ok(id)
    }
}
