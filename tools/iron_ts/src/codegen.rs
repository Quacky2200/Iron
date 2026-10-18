//! Iron IR -> Cranelift -> an object file for the host target.
//!
//! Machine types: `Num` f64, `Bool` i8, `Lit` i32, pointers i64. Object and array slots are
//! 8 bytes wide whatever the value (small values are zero-extended on store).

use crate::ir::*;
use cranelift_codegen::ir::condcodes::{FloatCC, IntCC};
use cranelift_codegen::ir::{
    self, AbiParam, InstBuilder, MemFlagsData, Signature, TrapCode, Value, types,
};
use cranelift_codegen::isa::CallConv;
use cranelift_codegen::settings::{self, Configurable};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext, Variable};
use cranelift_module::{
    DataDescription, DataId, FuncId as CFuncId, Linkage, Module as _, default_libcall_names,
};
use cranelift_object::{ObjectBuilder, ObjectModule};
use std::collections::HashMap;

type R<T> = Result<T, String>;

pub fn clif_ty(t: &Ty) -> ir::Type {
    match t {
        Ty::Num => types::F64,
        Ty::Nullable(i) if **i == Ty::Num => types::F64,
        Ty::Bool => types::I8,
        Ty::Lit => types::I32,
        _ => types::I64,
    }
}

fn sig_of(call_conv: CallConv, params: &[Ty], ret: &Ty) -> Signature {
    let mut s = Signature::new(call_conv);
    for p in params {
        s.params.push(AbiParam::new(clif_ty(p)));
    }
    if *ret != Ty::Void {
        s.returns.push(AbiParam::new(clif_ty(ret)));
    }
    s
}

pub struct Compiled {
    pub object: Vec<u8>,
}

/// Compiles a module. `opt` is Cranelift's `opt_level`: "none", "speed" or "speed_and_size".
pub fn compile(m: &Module, opt: &str) -> R<Compiled> {
    let mut flags = settings::builder();
    flags.set("opt_level", opt).map_err(|e| e.to_string())?;
    flags.set("is_pic", "false").map_err(|e| e.to_string())?;
    flags
        .set("unwind_info", "false")
        .map_err(|e| e.to_string())?;
    let isa = cranelift_codegen::isa::lookup(target_lexicon::Triple::host())
        .map_err(|e| e.to_string())?
        .finish(settings::Flags::new(flags))
        .map_err(|e| e.to_string())?;
    let call_conv = isa.default_call_conv();
    let fcfg = isa.frontend_config();
    let mut module = ObjectModule::new(
        ObjectBuilder::new(isa, "iron_ts", default_libcall_names()).map_err(|e| e.to_string())?,
    );

    // Functions: declare all first so calls can be forward references.
    let mut cfuncs: Vec<CFuncId> = Vec::new();
    for f in &m.funcs {
        let ptys: Vec<Ty> = f
            .params
            .iter()
            .map(|p| f.locals[*p as usize].clone())
            .collect();
        let sig = sig_of(call_conv, &ptys, &f.ret);
        let (name, linkage) = if f.exported {
            (format!("ts_{}{}", m.prefix, f.name), Linkage::Export)
        } else {
            (format!("iron_ts_local_{}", f.name), Linkage::Local)
        };
        cfuncs.push(
            module
                .declare_function(&name, linkage, &sig)
                .map_err(|e| e.to_string())?,
        );
    }
    // Globals: one 8-byte slot each.
    let mut gdata: Vec<DataId> = Vec::new();
    for g in &m.globals {
        let id = module
            .declare_data(
                &format!("ts_g_{}{}", m.prefix, g.name),
                Linkage::Local,
                true,
                false,
            )
            .map_err(|e| e.to_string())?;
        let mut d = DataDescription::new();
        d.define_zeroinit(8);
        module.define_data(id, &d).map_err(|e| e.to_string())?;
        gdata.push(id);
    }

    let mut imports: HashMap<&'static str, CFuncId> = HashMap::new();
    let mut fbc = FunctionBuilderContext::new();
    for (fi, f) in m.funcs.iter().enumerate() {
        let mut ctx = module.make_context();
        let ptys: Vec<Ty> = f
            .params
            .iter()
            .map(|p| f.locals[*p as usize].clone())
            .collect();
        ctx.func.signature = sig_of(call_conv, &ptys, &f.ret);
        {
            let mut b = FunctionBuilder::new(&mut ctx.func, &mut fbc);
            let entry = b.create_block();
            b.append_block_params_for_function_params(entry);
            b.switch_to_block(entry);
            let mut vars = Vec::new();
            for (i, t) in f.locals.iter().enumerate() {
                let ct = clif_ty(t);
                let v = b.declare_var(ct);
                vars.push(v);
                if let Some(pi) = f.params.iter().position(|p| *p as usize == i) {
                    let pv = b.block_params(entry)[pi];
                    b.def_var(v, pv);
                } else {
                    let z = zero(&mut b, ct);
                    b.def_var(v, z);
                }
            }
            let (mf_t, mf_p) = (MemFlagsData::trusted(), MemFlagsData::new());
            let mut cg = Cg {
                b,
                mf_t,
                mf_p,
                module: &mut module,
                m,
                f,
                vars,
                cfuncs: &cfuncs,
                gdata: &gdata,
                imports: &mut imports,
                labels: HashMap::new(),
                call_conv,
            };
            cg.stmts(&f.body)?;
            cg.b.ins().trap(TrapCode::unwrap_user(1));
            cg.b.seal_all_blocks();
            cg.b.finalize(fcfg);
        }
        module
            .define_function(cfuncs[fi], &mut ctx)
            .map_err(|e| format!("{}: {e:?}", f.name))?;
        module.clear_context(&mut ctx);
    }
    let object = module.finish().emit().map_err(|e| e.to_string())?;
    Ok(Compiled { object })
}

fn zero(b: &mut FunctionBuilder, t: ir::Type) -> Value {
    if t == types::F64 {
        b.ins().f64const(0.0)
    } else {
        b.ins().iconst(t, 0)
    }
}

struct Cg<'a, 'b> {
    b: FunctionBuilder<'b>,
    mf_t: MemFlagsData,
    mf_p: MemFlagsData,
    module: &'a mut ObjectModule,
    m: &'a Module,
    f: &'a Func,
    vars: Vec<Variable>,
    cfuncs: &'a [CFuncId],
    gdata: &'a [DataId],
    imports: &'a mut HashMap<&'static str, CFuncId>,
    /// label -> (break target, continue target)
    labels: HashMap<LabelId, (ir::Block, ir::Block)>,
    call_conv: CallConv,
}

impl Cg<'_, '_> {
    fn null_num(&mut self) -> Value {
        self.b.ins().f64const(f64::from_bits(NULL_NUM_BITS))
    }

    fn null_of(&mut self, t: &Ty) -> Value {
        if clif_ty(t) == types::F64 {
            self.null_num()
        } else {
            let ct = clif_ty(t);
            self.b.ins().iconst(ct, 0)
        }
    }

    /// Calls a runtime function by symbol name.
    fn rt(
        &mut self,
        name: &'static str,
        params: &[ir::Type],
        ret: Option<ir::Type>,
        args: &[Value],
    ) -> Option<Value> {
        let id = match self.imports.get(name) {
            Some(i) => *i,
            None => {
                let mut s = Signature::new(self.call_conv);
                for p in params {
                    s.params.push(AbiParam::new(*p));
                }
                if let Some(r) = ret {
                    s.returns.push(AbiParam::new(r));
                }
                let id = self
                    .module
                    .declare_function(name, Linkage::Import, &s)
                    .unwrap();
                self.imports.insert(name, id);
                id
            }
        };
        let fr = self.module.declare_func_in_func(id, self.b.func);
        let call = self.b.ins().call(fr, args);
        self.b.inst_results(call).first().copied()
    }

    fn cvt_i64(&mut self, v: Value) -> Value {
        self.b.ins().fcvt_to_sint_sat(types::I64, v)
    }

    /// JS ToInt32 for the values games use (|x| < 2^63).
    fn cvt_i32(&mut self, v: Value) -> Value {
        let w = self.cvt_i64(v);
        self.b.ins().ireduce(types::I32, w)
    }

    /// The raw 64-bit slot value for a typed value.
    fn slot_of(&mut self, v: Value, t: &Ty) -> Value {
        match clif_ty(t) {
            types::F64 => self.b.ins().bitcast(types::I64, self.mf_p, v),
            types::I8 | types::I32 => self.b.ins().uextend(types::I64, v),
            _ => v,
        }
    }

    fn dead_block(&mut self) {
        let d = self.b.create_block();
        self.b.switch_to_block(d);
    }

    fn stmts(&mut self, body: &[Stmt]) -> R<()> {
        for s in body {
            self.stmt(s)?;
        }
        Ok(())
    }

    fn expr_as(&mut self, e: &Expr, want: &Ty) -> R<Value> {
        if matches!(e.kind, ExprKind::Null) {
            return Ok(self.null_of(want));
        }
        self.expr(e)
    }

    fn stmt(&mut self, s: &Stmt) -> R<()> {
        match s {
            Stmt::Assign(v, e) => {
                let val = self.expr_as(e, &self.f.locals[*v as usize].clone())?;
                let var = self.vars[*v as usize];
                self.b.def_var(var, val);
            }
            Stmt::StoreField(o, _sid, i, v) => {
                let ptr = self.expr(o)?;
                let val = self.expr_as(v, &v.ty)?;
                let slot = self.slot_of(val, &v.ty);
                self.b.ins().store(self.mf_t, slot, ptr, field_offset(*i));
            }
            Stmt::StoreIndex(a, i, v) => {
                let arr = self.expr(a)?;
                let idx = self.expr(i)?;
                let val = self.expr(v)?;
                let idx = self.cvt_i64(idx);
                let len = self.b.ins().load(types::I64, self.mf_t, arr, 8);
                let ok = self.b.ins().icmp(IntCC::UnsignedLessThan, idx, len);
                let (st, join) = (self.b.create_block(), self.b.create_block());
                self.b.ins().brif(ok, st, &[], join, &[]);
                self.b.switch_to_block(st);
                let data = self.b.ins().load(types::I64, self.mf_t, arr, 24);
                if matches!(a.ty.strip(), Ty::Bytes) {
                    let n = self.cvt_i64(val);
                    let byte = self.b.ins().ireduce(types::I8, n);
                    let addr = self.b.ins().iadd(data, idx);
                    self.b.ins().store(self.mf_t, byte, addr, 0);
                } else {
                    let slot = self.slot_of(val, &v.ty);
                    let off = self.b.ins().imul_imm_s(idx, 8);
                    let addr = self.b.ins().iadd(data, off);
                    self.b.ins().store(self.mf_t, slot, addr, 0);
                }
                self.b.ins().jump(join, &[]);
                self.b.switch_to_block(join);
            }
            Stmt::StoreGlobal(g, e) => {
                let val = self.expr(e)?;
                let slot = self.slot_of(val, &e.ty);
                let gv = self
                    .module
                    .declare_data_in_func(self.gdata[*g], self.b.func);
                let addr = self.b.ins().symbol_value(types::I64, gv);
                self.b.ins().store(self.mf_t, slot, addr, 0);
            }
            Stmt::If(c, t, e) => {
                let cv = self.expr(c)?;
                let (tb, eb, join) = (
                    self.b.create_block(),
                    self.b.create_block(),
                    self.b.create_block(),
                );
                self.b.ins().brif(cv, tb, &[], eb, &[]);
                self.b.switch_to_block(tb);
                self.stmts(t)?;
                self.b.ins().jump(join, &[]);
                self.b.switch_to_block(eb);
                self.stmts(e)?;
                self.b.ins().jump(join, &[]);
                self.b.switch_to_block(join);
            }
            Stmt::Block(l, body) => {
                let after = self.b.create_block();
                self.labels.insert(*l, (after, after));
                self.stmts(body)?;
                self.b.ins().jump(after, &[]);
                self.b.switch_to_block(after);
            }
            Stmt::Loop(l, body) => {
                let (head, after) = (self.b.create_block(), self.b.create_block());
                self.labels.insert(*l, (after, head));
                self.b.ins().jump(head, &[]);
                self.b.switch_to_block(head);
                self.stmts(body)?;
                self.b.ins().jump(head, &[]);
                self.b.switch_to_block(after);
            }
            Stmt::Break(l) => {
                let t = self.labels[l].0;
                self.b.ins().jump(t, &[]);
                self.dead_block();
            }
            Stmt::Continue(l) => {
                let t = self.labels[l].1;
                self.b.ins().jump(t, &[]);
                self.dead_block();
            }
            Stmt::Return(v) => {
                match v {
                    Some(e) => {
                        let val = self.expr_as(e, &self.f.ret.clone())?;
                        self.b.ins().return_(&[val]);
                    }
                    None => {
                        self.b.ins().return_(&[]);
                    }
                }
                self.dead_block();
            }
            Stmt::Eval(e) => {
                self.expr(e)?;
            }
        }
        Ok(())
    }

    fn load_slot(&mut self, ptr: Value, off: i32, t: &Ty) -> Value {
        self.b.ins().load(clif_ty(t), self.mf_t, ptr, off)
    }

    fn expr(&mut self, e: &Expr) -> R<Value> {
        let b_f64 = |s: &mut Self, v: f64| s.b.ins().f64const(v);
        Ok(match &e.kind {
            ExprKind::Num(n) => b_f64(self, *n),
            ExprKind::Bool(v) => self.b.ins().iconst(types::I8, *v as i64),
            ExprKind::Tag(t) => self.b.ins().iconst(types::I32, *t as i64),
            ExprKind::Null => self.null_of(&e.ty),
            ExprKind::Var(v) => self.b.use_var(self.vars[*v as usize]),
            ExprKind::Global(g) => {
                let gv = self
                    .module
                    .declare_data_in_func(self.gdata[*g], self.b.func);
                let addr = self.b.ins().symbol_value(types::I64, gv);
                self.load_slot(addr, 0, &e.ty)
            }
            ExprKind::Un(op, a) => {
                let v = self.expr(a)?;
                match op {
                    UnOp::Neg => self.b.ins().fneg(v),
                    UnOp::Not => self.b.ins().bxor_imm_s(v, 1),
                    UnOp::Sqrt => self.b.ins().sqrt(v),
                    UnOp::Abs => self.b.ins().fabs(v),
                    UnOp::Floor => self.b.ins().floor(v),
                    UnOp::Sin => self
                        .rt("ts_rt_sin", &[types::F64], Some(types::F64), &[v])
                        .unwrap(),
                    UnOp::Cos => self
                        .rt("ts_rt_cos", &[types::F64], Some(types::F64), &[v])
                        .unwrap(),
                    UnOp::Sign => {
                        let z = b_f64(self, 0.0);
                        let (one, neg) = (b_f64(self, 1.0), b_f64(self, -1.0));
                        let gt = self.b.ins().fcmp(FloatCC::GreaterThan, v, z);
                        let lt = self.b.ins().fcmp(FloatCC::LessThan, v, z);
                        let r = self.b.ins().select(lt, neg, v);
                        self.b.ins().select(gt, one, r)
                    }
                }
            }
            ExprKind::Bin(op, a, c) => {
                let x = self.expr(a)?;
                let y = self.expr(c)?;
                match op {
                    BinOp::Add => self.b.ins().fadd(x, y),
                    BinOp::Sub => self.b.ins().fsub(x, y),
                    BinOp::Mul => self.b.ins().fmul(x, y),
                    BinOp::Div => self.b.ins().fdiv(x, y),
                    BinOp::Rem => self.rem(x, y),
                    BinOp::Min => self.b.ins().fmin(x, y),
                    BinOp::Max => self.b.ins().fmax(x, y),
                    BinOp::Atan2 => self
                        .rt(
                            "ts_rt_atan2",
                            &[types::F64, types::F64],
                            Some(types::F64),
                            &[x, y],
                        )
                        .unwrap(),
                    // sqrt(x*x + y*y): within an ulp of libm's hypot, without the call.
                    BinOp::Hypot => {
                        let (xx, yy) = (self.b.ins().fmul(x, x), self.b.ins().fmul(y, y));
                        let s = self.b.ins().fadd(xx, yy);
                        self.b.ins().sqrt(s)
                    }
                    BinOp::UShr => {
                        let (xi, yi) = (self.cvt_i32(x), self.cvt_i32(y));
                        let r = self.b.ins().ushr(xi, yi);
                        self.b.ins().fcvt_from_uint(types::F64, r)
                    }
                    BinOp::BitAnd
                    | BinOp::BitOr
                    | BinOp::BitXor
                    | BinOp::Shl
                    | BinOp::Shr
                    | BinOp::Imul => {
                        let (xi, yi) = (self.cvt_i32(x), self.cvt_i32(y));
                        let r = match op {
                            BinOp::BitAnd => self.b.ins().band(xi, yi),
                            BinOp::BitOr => self.b.ins().bor(xi, yi),
                            BinOp::BitXor => self.b.ins().bxor(xi, yi),
                            BinOp::Shl => self.b.ins().ishl(xi, yi),
                            BinOp::Shr => self.b.ins().sshr(xi, yi),
                            _ => self.b.ins().imul(xi, yi),
                        };
                        self.b.ins().fcvt_from_sint(types::F64, r)
                    }
                }
            }
            ExprKind::Cmp(op, a, c) => {
                let x = self.expr(a)?;
                let y = self.expr(c)?;
                if matches!(a.ty.strip(), Ty::Num) {
                    let cc = match op {
                        CmpOp::Eq => FloatCC::Equal,
                        CmpOp::Ne => FloatCC::NotEqual,
                        CmpOp::Lt => FloatCC::LessThan,
                        CmpOp::Le => FloatCC::LessThanOrEqual,
                        CmpOp::Gt => FloatCC::GreaterThan,
                        CmpOp::Ge => FloatCC::GreaterThanOrEqual,
                    };
                    self.b.ins().fcmp(cc, x, y)
                } else {
                    let cc = if *op == CmpOp::Eq {
                        IntCC::Equal
                    } else {
                        IntCC::NotEqual
                    };
                    self.b.ins().icmp(cc, x, y)
                }
            }
            ExprKind::And(a, c) | ExprKind::Or(a, c) => {
                let is_and = matches!(e.kind, ExprKind::And(..));
                let x = self.expr(a)?;
                let (rhs, join) = (self.b.create_block(), self.b.create_block());
                self.b.append_block_param(join, types::I8);
                if is_and {
                    self.b
                        .ins()
                        .brif(x, rhs, &[], join, &[ir::BlockArg::Value(x)]);
                } else {
                    self.b
                        .ins()
                        .brif(x, join, &[ir::BlockArg::Value(x)], rhs, &[]);
                }
                self.b.switch_to_block(rhs);
                let y = self.expr(c)?;
                self.b.ins().jump(join, &[ir::BlockArg::Value(y)]);
                self.b.switch_to_block(join);
                self.b.block_params(join)[0]
            }
            ExprKind::Cond(c, a, d) => {
                let cv = self.expr(c)?;
                let ct = clif_ty(&e.ty);
                let (tb, eb, join) = (
                    self.b.create_block(),
                    self.b.create_block(),
                    self.b.create_block(),
                );
                self.b.append_block_param(join, ct);
                self.b.ins().brif(cv, tb, &[], eb, &[]);
                self.b.switch_to_block(tb);
                let x = self.expr_as(a, &e.ty)?;
                self.b.ins().jump(join, &[ir::BlockArg::Value(x)]);
                self.b.switch_to_block(eb);
                let y = self.expr_as(d, &e.ty)?;
                self.b.ins().jump(join, &[ir::BlockArg::Value(y)]);
                self.b.switch_to_block(join);
                self.b.block_params(join)[0]
            }
            ExprKind::IsNull(a) => {
                let v = self.expr(a)?;
                if clif_ty(&a.ty) == types::F64 {
                    let bits = self.b.ins().bitcast(types::I64, self.mf_p, v);
                    self.b
                        .ins()
                        .icmp_imm_s(IntCC::Equal, bits, NULL_NUM_BITS as i64)
                } else {
                    self.b.ins().icmp_imm_s(IntCC::Equal, v, 0)
                }
            }
            ExprKind::Cast(a) => self.expr_as(a, &e.ty)?,
            ExprKind::Truthy(a) => {
                let v = self.expr(a)?;
                let z = b_f64(self, 0.0);
                self.b.ins().fcmp(FloatCC::OrderedNotEqual, v, z)
            }
            ExprKind::Field(o, _sid, i) => {
                let ptr = self.expr(o)?;
                self.load_slot(ptr, field_offset(*i), &e.ty)
            }
            ExprKind::Index(a, i) => {
                let arr = self.expr(a)?;
                let idx = self.expr(i)?;
                let idx = self.cvt_i64(idx);
                let len = self.b.ins().load(types::I64, self.mf_t, arr, 8);
                let ok = self.b.ins().icmp(IntCC::UnsignedLessThan, idx, len);
                let ct = clif_ty(&e.ty);
                let (inb, oob, join) = (
                    self.b.create_block(),
                    self.b.create_block(),
                    self.b.create_block(),
                );
                self.b.append_block_param(join, ct);
                self.b.ins().brif(ok, inb, &[], oob, &[]);
                self.b.switch_to_block(inb);
                let data = self.b.ins().load(types::I64, self.mf_t, arr, 24);
                let v = if matches!(a.ty.strip(), Ty::Bytes) {
                    let addr = self.b.ins().iadd(data, idx);
                    let byte = self.b.ins().uload8(types::I64, self.mf_t, addr, 0);
                    self.b.ins().fcvt_from_sint(types::F64, byte)
                } else {
                    let off = self.b.ins().imul_imm_s(idx, 8);
                    let addr = self.b.ins().iadd(data, off);
                    self.load_slot(addr, 0, &e.ty)
                };
                self.b.ins().jump(join, &[ir::BlockArg::Value(v)]);
                self.b.switch_to_block(oob);
                let d = if matches!(a.ty.strip(), Ty::Bytes) {
                    b_f64(self, 0.0)
                } else {
                    self.null_of(&e.ty)
                };
                self.b.ins().jump(join, &[ir::BlockArg::Value(d)]);
                self.b.switch_to_block(join);
                self.b.block_params(join)[0]
            }
            ExprKind::Len(a) => {
                let arr = self.expr(a)?;
                let len = self.b.ins().load(types::I64, self.mf_t, arr, 8);
                self.b.ins().fcvt_from_sint(types::F64, len)
            }
            ExprKind::NewObj(sid) => {
                let n = self.m.structs[*sid].fields.len();
                let size = self.b.ins().iconst(types::I64, 8 + 8 * n as i64);
                let p = self
                    .rt("ts_rt_alloc", &[types::I64], Some(types::I64), &[size])
                    .unwrap();
                let h = self.b.ins().iconst(types::I64, *sid as i64 + 1);
                self.b.ins().store(self.mf_t, h, p, 0);
                p
            }
            ExprKind::Call(fid, args) => {
                let mut vals = Vec::new();
                let ptys: Vec<Ty> = self.m.funcs[*fid]
                    .params
                    .iter()
                    .map(|p| self.m.funcs[*fid].locals[*p as usize].clone())
                    .collect();
                for (a, t) in args.iter().zip(&ptys) {
                    vals.push(self.expr_as(a, t)?);
                }
                let fr = self
                    .module
                    .declare_func_in_func(self.cfuncs[*fid], self.b.func);
                let call = self.b.ins().call(fr, &vals);
                match self.b.inst_results(call).first() {
                    Some(v) => *v,
                    None => self.b.ins().iconst(types::I64, 0),
                }
            }
            ExprKind::CallClosure(c, args) => {
                let clo = self.expr(c)?;
                let Ty::Fn(sig) = c.ty.strip() else {
                    return Err("call of a non-function".into());
                };
                let fp = self.b.ins().load(types::I64, self.mf_t, clo, 8);
                let env = self.b.ins().load(types::I64, self.mf_t, clo, 16);
                let mut vals = vec![env];
                for (a, t) in args.iter().zip(&sig.params) {
                    vals.push(self.expr_as(a, t)?);
                }
                let mut s = Signature::new(self.call_conv);
                s.params.push(AbiParam::new(types::I64));
                for p in &sig.params {
                    s.params.push(AbiParam::new(clif_ty(p)));
                }
                if sig.ret != Ty::Void {
                    s.returns.push(AbiParam::new(clif_ty(&sig.ret)));
                }
                let sr = self.b.import_signature(s);
                let call = self.b.ins().call_indirect(sr, fp, &vals);
                match self.b.inst_results(call).first() {
                    Some(v) => *v,
                    None => self.b.ins().iconst(types::I64, 0),
                }
            }
            ExprKind::Rt(r, args) => self.rt_call(*r, args)?,
            ExprKind::Seq(stmts, v) => {
                self.stmts(stmts)?;
                if v.ty == Ty::Void {
                    self.b.ins().iconst(types::I64, 0)
                } else {
                    self.expr_as(v, &e.ty)?
                }
            }
        })
    }

    /// JS `%`. Whole numbers with a positive divisor (loop indexes, wrap-around) take an
    /// integer `srem`; anything else calls libm's fmod. (A negative dividend that leaves 0 gives
    /// +0 instead of JS's -0.)
    fn rem(&mut self, x: Value, y: Value) -> Value {
        let (xi, yi) = (self.cvt_i64(x), self.cvt_i64(y));
        let (xb, yb) = (
            self.b.ins().fcvt_from_sint(types::F64, xi),
            self.b.ins().fcvt_from_sint(types::F64, yi),
        );
        let (xe, ye) = (
            self.b.ins().fcmp(FloatCC::Equal, xb, x),
            self.b.ins().fcmp(FloatCC::Equal, yb, y),
        );
        let pos = self.b.ins().icmp_imm_s(IntCC::SignedGreaterThan, yi, 0);
        let both = self.b.ins().band(xe, ye);
        let ok = self.b.ins().band(both, pos);
        let (fast, slow, join) = (
            self.b.create_block(),
            self.b.create_block(),
            self.b.create_block(),
        );
        self.b.append_block_param(join, types::F64);
        self.b.ins().brif(ok, fast, &[], slow, &[]);
        self.b.switch_to_block(fast);
        let r = self.b.ins().srem(xi, yi);
        let rf = self.b.ins().fcvt_from_sint(types::F64, r);
        self.b.ins().jump(join, &[ir::BlockArg::Value(rf)]);
        self.b.switch_to_block(slow);
        let m = self
            .rt(
                "ts_rt_fmod",
                &[types::F64, types::F64],
                Some(types::F64),
                &[x, y],
            )
            .unwrap();
        self.b.ins().jump(join, &[ir::BlockArg::Value(m)]);
        self.b.switch_to_block(join);
        self.b.block_params(join)[0]
    }

    fn rt_call(&mut self, r: RtFn, args: &[Expr]) -> R<Value> {
        let mut vals = Vec::new();
        for a in args {
            vals.push(self.expr(a)?);
        }
        let dummy = |s: &mut Self| s.b.ins().iconst(types::I64, 0);
        Ok(match r {
            RtFn::Alloc => self
                .rt("ts_rt_alloc", &[types::I64], Some(types::I64), &vals)
                .unwrap(),
            RtFn::ArrNew => {
                let cap = self.cvt_i64(vals[0]);
                self.rt("ts_rt_arr_new", &[types::I64], Some(types::I64), &[cap])
                    .unwrap()
            }
            RtFn::ArrPush => {
                let slot = self.slot_of(vals[1], &args[1].ty);
                self.rt(
                    "ts_rt_arr_push",
                    &[types::I64, types::I64],
                    None,
                    &[vals[0], slot],
                );
                dummy(self)
            }
            RtFn::BytesNew => {
                let n = self.cvt_i64(vals[0]);
                self.rt("ts_rt_bytes_new", &[types::I64], Some(types::I64), &[n])
                    .unwrap()
            }
            RtFn::BytesFill => {
                let (v, s, e) = (
                    self.cvt_i64(vals[1]),
                    self.cvt_i64(vals[2]),
                    self.cvt_i64(vals[3]),
                );
                self.rt(
                    "ts_rt_bytes_fill",
                    &[types::I64, types::I64, types::I64, types::I64],
                    None,
                    &[vals[0], v, s, e],
                );
                dummy(self)
            }
        })
    }
}
