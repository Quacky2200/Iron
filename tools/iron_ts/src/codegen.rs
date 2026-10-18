//! Iron IR -> Cranelift -> an object file for the host target.
//!
//! Machine types: `Num` f64, `Bool` i8, `Lit` i32, pointers i64. Object and array slots are
//! 8 bytes wide whatever the value (small values are zero-extended on store).

use crate::frames;
use crate::ir::*;
use cranelift_codegen::ir::condcodes::{FloatCC, IntCC};
use cranelift_codegen::ir::{
    self, AbiParam, InstBuilder, MemFlagsData, Signature, StackSlot, StackSlotData, StackSlotKind,
    TrapCode, Value, types,
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

/// The machine type of a value at the engine boundary.
fn ext_clif(t: &ExtTy) -> ir::Type {
    match t {
        ExtTy::F64 => types::F64,
        ExtTy::F32 => types::F32,
        ExtTy::I32 | ExtTy::U32 => types::I32,
        ExtTy::Bool => types::I8,
        ExtTy::Handle(_) | ExtTy::Value(_) | ExtTy::Void => types::I64,
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
    // Engine functions the module calls: imports, with the C ABI of the manifest.
    let mut ext_funcs: Vec<CFuncId> = Vec::new();
    for d in &m.externs {
        let mut sig = Signature::new(call_conv);
        for p in &d.params {
            sig.params.push(AbiParam::new(ext_clif(p)));
        }
        if d.out.is_some() {
            sig.params.push(AbiParam::new(types::I64));
        }
        if d.ret != ExtTy::Void {
            sig.returns.push(AbiParam::new(ext_clif(&d.ret)));
        }
        ext_funcs.push(
            module
                .declare_function(&d.symbol, Linkage::Import, &sig)
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
        // The collector reads pointer slots as `*mut u8`: they must be aligned.
        d.set_align(8);
        module.define_data(id, &d).map_err(|e| e.to_string())?;
        gdata.push(id);
    }

    // Method tables: one 8-byte slot per struct id, holding the function address (or 0).
    let mut vtdata: Vec<DataId> = Vec::new();
    for (i, vt) in m.vtables.iter().enumerate() {
        let id = module
            .declare_data(
                &format!("ts_vt_{}{}", m.prefix, i),
                Linkage::Local,
                false,
                false,
            )
            .map_err(|e| e.to_string())?;
        let mut d = DataDescription::new();
        d.define(vec![0u8; m.structs.len() * 8].into_boxed_slice());
        d.set_align(8);
        for (sid, fid) in &vt.entries {
            let fr = module.declare_func_in_data(cfuncs[*fid], &mut d);
            d.write_function_addr((*sid * 8) as u32, fr);
        }
        module.define_data(id, &d).map_err(|e| e.to_string())?;
        vtdata.push(id);
    }
    // `instanceof` tables: one byte per struct id.
    let mut itdata: Vec<DataId> = Vec::new();
    for (i, members) in m.itables.iter().enumerate() {
        let id = module
            .declare_data(
                &format!("ts_it_{}{}", m.prefix, i),
                Linkage::Local,
                false,
                false,
            )
            .map_err(|e| e.to_string())?;
        let mut bytes = vec![0u8; m.structs.len()];
        for s in members {
            bytes[*s] = 1;
        }
        let mut d = DataDescription::new();
        d.define(bytes.into_boxed_slice());
        module.define_data(id, &d).map_err(|e| e.to_string())?;
        itdata.push(id);
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
            let plan = frames::plan(f);
            let frame_slot = (plan.total() > 0).then(|| {
                b.create_sized_stack_slot(StackSlotData::new(
                    StackSlotKind::ExplicitSlot,
                    16 + 8 * plan.total(),
                    4,
                ))
            });
            let mut cg = Cg {
                frame_slot,
                ptr_slot: plan.ptr_slot.clone(),
                next_spill: plan.n_locals,
                n_slots: plan.total(),
                b,
                mf_t,
                mf_p,
                module: &mut module,
                m,
                f,
                vars,
                cfuncs: &cfuncs,
                ext_funcs: &ext_funcs,
                gdata: &gdata,
                vtdata: &vtdata,
                itdata: &itdata,
                imports: &mut imports,
                labels: HashMap::new(),
                call_conv,
            };
            cg.prologue(m.init == Some(fi))?;
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
    /// The shadow-stack frame (see `frames.rs`): pointer locals are mirrored here.
    frame_slot: Option<StackSlot>,
    ptr_slot: Vec<Option<u32>>,
    next_spill: u32,
    n_slots: u32,
    mf_t: MemFlagsData,
    mf_p: MemFlagsData,
    module: &'a mut ObjectModule,
    m: &'a Module,
    f: &'a Func,
    vars: Vec<Variable>,
    cfuncs: &'a [CFuncId],
    ext_funcs: &'a [CFuncId],
    gdata: &'a [DataId],
    vtdata: &'a [DataId],
    itdata: &'a [DataId],
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

    /// Stores a pointer in frame slot `idx` (a GC root).
    fn root_store(&mut self, idx: u32, v: Value) {
        let slot = self.frame_slot.expect("frame");
        let base = self.b.ins().stack_addr(types::I64, slot, 0);
        self.b.ins().store(self.mf_t, v, base, 16 + 8 * idx as i32);
    }

    /// Links this function's frame onto the shadow stack and roots its pointer parameters. The
    /// init function also tells the collector which struct slots are pointers and which globals
    /// are roots.
    fn prologue(&mut self, is_init: bool) -> R<()> {
        if let Some(slot) = self.frame_slot {
            let base = self.b.ins().stack_addr(types::I64, slot, 0);
            let n = self.b.ins().iconst(types::I64, self.n_slots as i64);
            self.rt(
                "ts_rt_frame_push",
                &[types::I64, types::I64],
                None,
                &[base, n],
            );
            for p in self.f.params.clone() {
                if let Some(i) = self.ptr_slot[p as usize] {
                    let v = self.b.use_var(self.vars[p as usize]);
                    self.root_store(i, v);
                }
            }
        }
        if is_init {
            for (sid, st) in self.m.structs.iter().enumerate() {
                if st.fields.len() > 64 {
                    return Err(format!("struct {} has more than 64 fields", st.name));
                }
                let mut mask = 0u64;
                for (i, f) in st.fields.iter().enumerate() {
                    if f.ty.is_ptr() {
                        mask |= 1 << i;
                    }
                }
                let h = self
                    .b
                    .ins()
                    .iconst(types::I64, self.m.struct_base as i64 + sid as i64 + 1);
                let n = self.b.ins().iconst(types::I32, st.fields.len() as i64);
                let mk = self.b.ins().iconst(types::I64, mask as i64);
                self.rt(
                    "ts_rt_register_struct",
                    &[types::I64, types::I32, types::I64],
                    None,
                    &[h, n, mk],
                );
            }
            for (g, gl) in self.m.globals.iter().enumerate() {
                if gl.ty.is_ptr() {
                    let gv = self.module.declare_data_in_func(self.gdata[g], self.b.func);
                    let addr = self.b.ins().symbol_value(types::I64, gv);
                    self.rt("ts_rt_add_global_root", &[types::I64], None, &[addr]);
                }
            }
        }
        Ok(())
    }

    /// Evaluates operands in order, spilling pointer values that a later operand's allocation
    /// could otherwise leave unrooted.
    fn eval_args(&mut self, args: &[(&Expr, Ty)]) -> R<Vec<Value>> {
        let refs: Vec<&Expr> = args.iter().map(|(e, _)| *e).collect();
        let flags = frames::spill_flags(&refs);
        let mut vals = Vec::new();
        for ((e, t), spill) in args.iter().zip(flags) {
            let v = self.expr_as(e, t)?;
            if spill {
                let idx = self.next_spill;
                self.next_spill += 1;
                if idx >= self.n_slots {
                    // The plan in frames.rs and this walk disagree; writing on would corrupt the stack.
                    return Err(format!(
                        "{}: frame plan too small ({} slots)",
                        self.f.name, self.n_slots
                    ));
                }
                self.root_store(idx, v);
            }
            vals.push(v);
        }
        Ok(vals)
    }

    fn pop_frame(&mut self) {
        if self.frame_slot.is_some() {
            self.rt("ts_rt_frame_pop", &[], None, &[]);
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
                if let Some(i) = self.ptr_slot[*v as usize] {
                    self.root_store(i, val);
                }
            }
            Stmt::StoreField(o, _sid, i, v) => {
                let vals = self.eval_args(&[(o, o.ty.clone()), (v, v.ty.clone())])?;
                let (ptr, val) = (vals[0], vals[1]);
                let slot = self.slot_of(val, &v.ty);
                self.b.ins().store(self.mf_t, slot, ptr, field_offset(*i));
            }
            Stmt::CellSet(c, v) => {
                let vals = self.eval_args(&[(c, c.ty.clone()), (v, v.ty.clone())])?;
                let slot = self.slot_of(vals[1], &v.ty);
                self.b.ins().store(self.mf_t, slot, vals[0], 8);
            }
            Stmt::StoreIndex(a, i, v) => {
                let vals =
                    self.eval_args(&[(a, a.ty.clone()), (i, i.ty.clone()), (v, v.ty.clone())])?;
                let (arr, idx, val) = (vals[0], vals[1], vals[2]);
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
                        self.pop_frame();
                        self.b.ins().return_(&[val]);
                    }
                    None => {
                        self.pop_frame();
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
            ExprKind::Global(g) | ExprKind::GlobalVar(g) => {
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
                    UnOp::Ceil => self.b.ins().ceil(v),
                    UnOp::Trunc => self.b.ins().trunc(v),
                    // floor(x), plus one when the fraction is at least a half; copysign keeps -0.
                    UnOp::Round => {
                        let r = self.b.ins().floor(v);
                        let frac = self.b.ins().fsub(v, r);
                        let (half, one) = (b_f64(self, 0.5), b_f64(self, 1.0));
                        let up = self.b.ins().fcmp(FloatCC::GreaterThanOrEqual, frac, half);
                        let r1 = self.b.ins().fadd(r, one);
                        let pick = self.b.ins().select(up, r1, r);
                        self.b.ins().fcopysign(pick, v)
                    }
                    UnOp::Fround => {
                        let s = self.b.ins().fdemote(types::F32, v);
                        self.b.ins().fpromote(types::F64, s)
                    }
                    UnOp::Clz32 => {
                        let i = self.cvt_i32(v);
                        let c = self.b.ins().clz(i);
                        self.b.ins().fcvt_from_uint(types::F64, c)
                    }
                    UnOp::Libm(name) => self
                        .rt(name, &[types::F64], Some(types::F64), &[v])
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
                    BinOp::Pow => self
                        .rt(
                            "ts_rt_pow",
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
                // Pointer operands wait for each other like arguments do (an address compared
                // after being freed and reused would be wrong).
                let vals = self.eval_args(&[(a, a.ty.clone()), (c, c.ty.clone())])?;
                let (x, y) = (vals[0], vals[1]);
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
                let vals = self.eval_args(&[(a, a.ty.clone()), (i, i.ty.clone())])?;
                let (arr, idx) = (vals[0], vals[1]);
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
                let h = self
                    .b
                    .ins()
                    .iconst(types::I64, self.m.struct_base as i64 + *sid as i64 + 1);
                self.rt(
                    "ts_rt_obj_new_gc",
                    &[types::I64, types::I64],
                    Some(types::I64),
                    &[h, size],
                )
                .unwrap()
            }
            ExprKind::CallExtern(i, args) => {
                let decl = &self.m.externs[*i];
                let list: Vec<(&Expr, Ty)> = args
                    .iter()
                    .zip(&decl.params)
                    .map(|(a, p)| (a, p.to_ir()))
                    .collect();
                let vals = self.eval_args(&list)?;
                // Scripts hold every number as an f64; the engine takes its own width.
                let mut conv = Vec::new();
                for (v, p) in vals.iter().zip(&decl.params) {
                    conv.push(match p {
                        ExtTy::F32 => self.b.ins().fdemote(types::F32, *v),
                        ExtTy::I32 => self.b.ins().fcvt_to_sint_sat(types::I32, *v),
                        ExtTy::U32 => self.b.ins().fcvt_to_uint_sat(types::I32, *v),
                        _ => *v,
                    });
                }
                let fr = self
                    .module
                    .declare_func_in_func(self.ext_funcs[*i], self.b.func);
                let call = self.b.ins().call(fr, &conv);
                let ret = decl.ret.clone();
                match (self.b.inst_results(call).first().copied(), ret) {
                    (Some(r), ExtTy::F32) => self.b.ins().fpromote(types::F64, r),
                    (Some(r), ExtTy::I32) => self.b.ins().fcvt_from_sint(types::F64, r),
                    (Some(r), ExtTy::U32) => self.b.ins().fcvt_from_uint(types::F64, r),
                    (Some(r), _) => r,
                    (None, _) => self.b.ins().iconst(types::I64, 0),
                }
            }
            ExprKind::CallExternInto(i, args, dest) => {
                let decl = &self.m.externs[*i];
                let (part, n) = decl.out.clone().expect("a value result has an out pointer");
                let list: Vec<(&Expr, Ty)> = args
                    .iter()
                    .zip(&decl.params)
                    .map(|(a, p)| (a, p.to_ir()))
                    .collect();
                let vals = self.eval_args(&list)?;
                let mut conv = Vec::new();
                for (v, p) in vals.iter().zip(&decl.params) {
                    conv.push(match p {
                        ExtTy::F32 => self.b.ins().fdemote(types::F32, *v),
                        ExtTy::I32 => self.b.ins().fcvt_to_sint_sat(types::I32, *v),
                        ExtTy::U32 => self.b.ins().fcvt_to_uint_sat(types::I32, *v),
                        _ => *v,
                    });
                }
                // Room for the result's scalars on the stack; the engine writes them there.
                let width = match ext_clif(&part) {
                    types::F64 => 8,
                    _ => 4,
                };
                let slot = self.b.create_sized_stack_slot(StackSlotData::new(
                    StackSlotKind::ExplicitSlot,
                    (width * n) as u32,
                    3,
                ));
                let addr = self.b.ins().stack_addr(types::I64, slot, 0);
                conv.push(addr);
                let fr = self
                    .module
                    .declare_func_in_func(self.ext_funcs[*i], self.b.func);
                self.b.ins().call(fr, &conv);
                // Copy the scalars into the fields of the destination object, as f64.
                let obj = self.expr(dest)?;
                for k in 0..n {
                    let raw =
                        self.b
                            .ins()
                            .load(ext_clif(&part), self.mf_t, addr, (width * k) as i32);
                    let num = match part {
                        ExtTy::F32 => self.b.ins().fpromote(types::F64, raw),
                        ExtTy::I32 => self.b.ins().fcvt_from_sint(types::F64, raw),
                        ExtTy::U32 => self.b.ins().fcvt_from_uint(types::F64, raw),
                        _ => raw,
                    };
                    self.b.ins().store(self.mf_t, num, obj, field_offset(k));
                }
                self.b.ins().iconst(types::I64, 0)
            }
            ExprKind::CallVirtual(table, args) => {
                // The signature comes from any entry; they agree up to the receiver's class.
                let fid = self.m.vtables[*table].entries[0].1;
                let ptys: Vec<Ty> = self.m.funcs[fid]
                    .params
                    .iter()
                    .map(|p| self.m.funcs[fid].locals[*p as usize].clone())
                    .collect();
                let list: Vec<(&Expr, Ty)> = args.iter().zip(ptys.clone()).collect();
                let vals = self.eval_args(&list)?;
                // The object's header holds its struct id: index the table with it.
                let header = self.b.ins().load(types::I64, self.mf_t, vals[0], 0);
                let kind = self.b.ins().band_imm_s(header, 0xFFFF_FFFF);
                let idx = self
                    .b
                    .ins()
                    .iadd_imm_s(kind, -(self.m.struct_base as i64 + 1));
                let off = self.b.ins().imul_imm_s(idx, 8);
                let gv = self
                    .module
                    .declare_data_in_func(self.vtdata[*table], self.b.func);
                let base = self.b.ins().symbol_value(types::I64, gv);
                let slot = self.b.ins().iadd(base, off);
                let target = self.b.ins().load(types::I64, self.mf_t, slot, 0);
                let sig = sig_of(self.call_conv, &ptys, &self.m.funcs[fid].ret);
                let sr = self.b.import_signature(sig);
                let call = self.b.ins().call_indirect(sr, target, &vals);
                match self.b.inst_results(call).first() {
                    Some(v) => *v,
                    None => self.b.ins().iconst(types::I64, 0),
                }
            }
            ExprKind::InstanceOf(a, table) => {
                let v = self.expr(a)?;
                let (yes, no, join) = (
                    self.b.create_block(),
                    self.b.create_block(),
                    self.b.create_block(),
                );
                self.b.append_block_param(join, types::I8);
                let isnull = self.b.ins().icmp_imm_s(IntCC::Equal, v, 0);
                self.b.ins().brif(isnull, no, &[], yes, &[]);
                self.b.switch_to_block(yes);
                let header = self.b.ins().load(types::I64, self.mf_t, v, 0);
                let kind = self.b.ins().band_imm_s(header, 0xFFFF_FFFF);
                let idx = self
                    .b
                    .ins()
                    .iadd_imm_s(kind, -(self.m.struct_base as i64 + 1));
                let gv = self
                    .module
                    .declare_data_in_func(self.itdata[*table], self.b.func);
                let base = self.b.ins().symbol_value(types::I64, gv);
                let addr = self.b.ins().iadd(base, idx);
                let hit = self.b.ins().load(types::I8, self.mf_t, addr, 0);
                self.b.ins().jump(join, &[ir::BlockArg::Value(hit)]);
                self.b.switch_to_block(no);
                let zero = self.b.ins().iconst(types::I8, 0);
                self.b.ins().jump(join, &[ir::BlockArg::Value(zero)]);
                self.b.switch_to_block(join);
                self.b.block_params(join)[0]
            }
            ExprKind::Call(fid, args) => {
                let ptys: Vec<Ty> = self.m.funcs[*fid]
                    .params
                    .iter()
                    .map(|p| self.m.funcs[*fid].locals[*p as usize].clone())
                    .collect();
                let list: Vec<(&Expr, Ty)> = args.iter().zip(ptys).collect();
                let vals = self.eval_args(&list)?;
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
                let Ty::Fn(sig) = c.ty.strip() else {
                    return Err("call of a non-function".into());
                };
                let mut list: Vec<(&Expr, Ty)> = vec![(&**c, c.ty.clone())];
                list.extend(args.iter().zip(sig.params.iter().cloned()));
                let evaled = self.eval_args(&list)?;
                let clo = evaled[0];
                let fp = self.b.ins().load(types::I64, self.mf_t, clo, 8);
                let env = self.b.ins().load(types::I64, self.mf_t, clo, 16);
                let mut vals = vec![env];
                vals.extend(&evaled[1..]);
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
            ExprKind::NewCell(v) => {
                let val = self.expr(v)?;
                let slot = self.slot_of(val, &v.ty);
                let is_ptr = self.b.ins().iconst(types::I64, v.ty.is_ptr() as i64);
                self.rt(
                    "ts_rt_cell_new_gc",
                    &[types::I64, types::I64],
                    Some(types::I64),
                    &[slot, is_ptr],
                )
                .unwrap()
            }
            ExprKind::CellGet(c) => {
                let ptr = self.expr(c)?;
                self.load_slot(ptr, 8, &e.ty)
            }
            ExprKind::MakeClosure(fid, env) => {
                let env = match env {
                    Some(env) => self.expr(env)?,
                    None => self.b.ins().iconst(types::I64, 0),
                };
                let fr = self
                    .module
                    .declare_func_in_func(self.cfuncs[*fid], self.b.func);
                let addr = self.b.ins().func_addr(types::I64, fr);
                self.rt(
                    "ts_rt_closure_new_gc",
                    &[types::I64, types::I64],
                    Some(types::I64),
                    &[addr, env],
                )
                .unwrap()
            }
            ExprKind::Rt(r, args) => self.rt_call(*r, args, &e.ty)?,
            ExprKind::Seq(stmts, v) => {
                self.stmts(stmts)?;
                if v.ty == Ty::Void {
                    // A void call still has to run (`Seq(temporaries, call)`).
                    self.expr(v)?;
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

    fn rt_call(&mut self, r: RtFn, args: &[Expr], ty: &Ty) -> R<Value> {
        let list: Vec<(&Expr, Ty)> = args.iter().map(|a| (a, a.ty.clone())).collect();
        let vals = self.eval_args(&list)?;
        let dummy = |s: &mut Self| s.b.ins().iconst(types::I64, 0);
        Ok(match r {
            RtFn::Alloc => return Err("RtFn::Alloc is unused".into()),
            RtFn::ArrNew => {
                let cap = self.cvt_i64(vals[0]);
                let ptrs = matches!(ty.strip(), Ty::Arr(el) if el.is_ptr()) as i64;
                let ptrs = self.b.ins().iconst(types::I64, ptrs);
                self.rt(
                    "ts_rt_arr_new_gc",
                    &[types::I64, types::I64],
                    Some(types::I64),
                    &[cap, ptrs],
                )
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
                self.rt("ts_rt_bytes_new_gc", &[types::I64], Some(types::I64), &[n])
                    .unwrap()
            }
            RtFn::ArrSetLen => {
                self.rt(
                    "ts_rt_arr_set_len",
                    &[types::I64, types::F64],
                    None,
                    &[vals[0], vals[1]],
                );
                dummy(self)
            }
            RtFn::ArrInsert => {
                let v = self.slot_of(vals[2], &args[2].ty);
                self.rt(
                    "ts_rt_arr_insert",
                    &[types::I64, types::F64, types::I64],
                    None,
                    &[vals[0], vals[1], v],
                );
                dummy(self)
            }
            RtFn::ArrDrop => {
                self.rt(
                    "ts_rt_arr_drop",
                    &[types::I64, types::F64, types::F64],
                    None,
                    &[vals[0], vals[1], vals[2]],
                );
                dummy(self)
            }
            RtFn::ArrSplice => self
                .rt(
                    "ts_rt_arr_splice",
                    &[types::I64, types::F64, types::F64, types::I64],
                    Some(types::I64),
                    &[vals[0], vals[1], vals[2], vals[3]],
                )
                .unwrap(),
            RtFn::ArrSlice => self
                .rt(
                    "ts_rt_arr_slice",
                    &[types::I64, types::F64, types::F64],
                    Some(types::I64),
                    &[vals[0], vals[1], vals[2]],
                )
                .unwrap(),
            RtFn::ArrAppend => {
                self.rt(
                    "ts_rt_arr_append",
                    &[types::I64, types::I64],
                    None,
                    &[vals[0], vals[1]],
                );
                dummy(self)
            }
            RtFn::ArrReverse => {
                self.rt("ts_rt_arr_reverse", &[types::I64], None, &[vals[0]]);
                dummy(self)
            }
            RtFn::ArrSort => {
                let ptrs = matches!(args[0].ty.strip(), Ty::Arr(el) if el.is_ptr()) as i64;
                let mode = self.b.ins().iconst(types::I64, ptrs);
                self.rt(
                    "ts_rt_arr_sort",
                    &[types::I64, types::I64, types::I64],
                    None,
                    &[vals[0], vals[1], mode],
                );
                dummy(self)
            }
            RtFn::ArrFill => {
                let v = self.slot_of(vals[1], &args[1].ty);
                self.rt(
                    "ts_rt_arr_fill",
                    &[types::I64, types::I64, types::F64, types::F64],
                    None,
                    &[vals[0], v, vals[2], vals[3]],
                );
                dummy(self)
            }
            RtFn::Random => self.rt("ts_rt_random", &[], Some(types::F64), &[]).unwrap(),
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
