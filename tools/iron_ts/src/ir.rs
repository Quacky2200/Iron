//! Iron IR: a small typed, structured IR between the TypeScript AST and Cranelift.
//!
//! Structured on purpose (blocks, loops, labelled break/continue instead of a CFG): lowering
//! from TypeScript stays simple, and `cranelift-frontend` builds the CFG in one pass. Every
//! expression carries its type, so codegen never guesses a machine type.
//!
//! Value representation (all object fields live in 8-byte slots, see `docs/research/iron-ts-spike.md`):
//! - `Num` f64, `Bool` i8, `Lit` (string-literal union member) i32, everything else a 64-bit pointer.
//! - `Nullable(ptr)` is the pointer with 0 as null; `Nullable(Num)` is an f64 with one reserved NaN
//!   payload as null (`NULL_NUM_BITS`), so arithmetic on a narrowed value needs no unboxing.

use std::fmt;

pub type StructId = usize;
pub type FuncId = usize;
pub type VarId = u32;
pub type GlobalId = usize;
pub type LabelId = u32;

/// The f64 bit pattern that means `null` / `undefined` in a `number | null` slot.
pub const NULL_NUM_BITS: u64 = 0x7FF8_DEAD_BEEF_0001;

#[derive(Clone, Debug, PartialEq)]
pub enum Ty {
    Void,
    Num,
    Bool,
    /// A member of a string-literal union, as a small integer (the id is global, see `Module::tags`).
    Lit,
    Str,
    /// The type of the `null` literal before it is coerced to a nullable type.
    Null,
    Obj(StructId),
    Arr(Box<Ty>),
    /// `Uint8Array`: the array header with one byte per element.
    Bytes,
    Fn(Box<FnTy>),
    Nullable(Box<Ty>),
    /// Reserved: any dynamic value (not produced by the spike's lowering yet).
    JsValue,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FnTy {
    pub params: Vec<Ty>,
    pub ret: Ty,
}

impl Ty {
    pub fn nullable(self) -> Ty {
        match self {
            Ty::Nullable(_) | Ty::Null => self,
            t => Ty::Nullable(Box::new(t)),
        }
    }
    /// Strips `Nullable` (same representation).
    pub fn strip(&self) -> &Ty {
        match self {
            Ty::Nullable(t) => t.strip(),
            t => t,
        }
    }
    pub fn is_nullable(&self) -> bool {
        matches!(self, Ty::Nullable(_) | Ty::Null)
    }
    pub fn is_ptr(&self) -> bool {
        matches!(
            self.strip(),
            Ty::Str | Ty::Obj(_) | Ty::Arr(_) | Ty::Bytes | Ty::Fn(_) | Ty::JsValue
        )
    }
    pub fn is_num(&self) -> bool {
        matches!(self.strip(), Ty::Num)
    }
}

#[derive(Clone, Debug)]
pub struct Field {
    pub name: String,
    pub ty: Ty,
}

#[derive(Clone, Debug)]
pub struct StructDef {
    pub name: String,
    pub fields: Vec<Field>,
}

impl StructDef {
    pub fn field(&self, name: &str) -> Option<usize> {
        self.fields.iter().position(|f| f.name == name)
    }
}

/// Byte offset of field `idx`: one 8-byte header word (the struct id + 1), then 8-byte slots.
pub fn field_offset(idx: usize) -> i32 {
    8 + 8 * idx as i32
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    /// JS `%` (fmod).
    Rem,
    /// Bit operations on numbers: operands go through ToInt32, results come back as f64.
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
    /// `>>>`: unsigned 32-bit result.
    UShr,
    /// `Math.imul`.
    Imul,
    Min,
    Max,
    Atan2,
    Hypot,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum UnOp {
    Neg,
    Not,
    Sqrt,
    Abs,
    Floor,
    Sin,
    Cos,
    /// `Math.sign`.
    Sign,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CmpOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

/// Calls into `iron_ts_rt` that are not inlined.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RtFn {
    Alloc,
    ArrNew,
    ArrPush,
    BytesNew,
    BytesFill,
}

#[derive(Clone, Debug)]
pub struct Expr {
    pub kind: ExprKind,
    pub ty: Ty,
}

#[derive(Clone, Debug)]
pub enum ExprKind {
    Num(f64),
    Bool(bool),
    /// A string-literal union member (index into `Module::tags`).
    Tag(u32),
    /// `null`: a zero pointer or the null NaN, depending on `ty`.
    Null,
    Var(VarId),
    Global(GlobalId),
    Un(UnOp, Box<Expr>),
    Bin(BinOp, Box<Expr>, Box<Expr>),
    Cmp(CmpOp, Box<Expr>, Box<Expr>),
    /// Short-circuit `a && b` / `a || b` on bools.
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
    /// `cond ? a : b`, evaluating one side only.
    Cond(Box<Expr>, Box<Expr>, Box<Expr>),
    /// True when a nullable value is null.
    IsNull(Box<Expr>),
    /// Reinterpret between representation-identical types (`T | null` <-> `T`, `Null` -> nullable).
    Cast(Box<Expr>),
    /// A number as a bool (`x !== 0 && !NaN`).
    Truthy(Box<Expr>),
    Field(Box<Expr>, StructId, usize),
    /// `arr[i]`: the element, or null / NaN-null when out of range. Bytes yield a Num (0 out of range).
    Index(Box<Expr>, Box<Expr>),
    Len(Box<Expr>),
    NewObj(StructId),
    Call(FuncId, Vec<Expr>),
    CallClosure(Box<Expr>, Vec<Expr>),
    Rt(RtFn, Vec<Expr>),
    /// Statements, then a value.
    Seq(Vec<Stmt>, Box<Expr>),
}

#[derive(Clone, Debug)]
pub enum Stmt {
    Assign(VarId, Expr),
    StoreField(Expr, StructId, usize, Expr),
    StoreIndex(Expr, Expr, Expr),
    StoreGlobal(GlobalId, Expr),
    If(Expr, Vec<Stmt>, Vec<Stmt>),
    /// `break(label)` leaves the block; falls out at its end.
    Block(LabelId, Vec<Stmt>),
    /// An infinite loop; `Continue(label)` jumps to its top, `Break(label)` leaves it.
    Loop(LabelId, Vec<Stmt>),
    Break(LabelId),
    Continue(LabelId),
    Return(Option<Expr>),
    Eval(Expr),
}

#[derive(Clone, Debug)]
pub struct Func {
    pub name: String,
    pub exported: bool,
    pub params: Vec<VarId>,
    pub ret: Ty,
    /// The type of every local, params first. `VarId` indexes it.
    pub locals: Vec<Ty>,
    pub body: Vec<Stmt>,
}

#[derive(Clone, Debug)]
pub struct Global {
    pub name: String,
    pub ty: Ty,
}

#[derive(Clone, Debug, Default)]
pub struct Module {
    pub structs: Vec<StructDef>,
    pub globals: Vec<Global>,
    pub funcs: Vec<Func>,
    /// String-literal union members; a `Tag` is an index here.
    pub tags: Vec<String>,
    /// Runs the top-level initialisers; exported as `ts_init`.
    pub init: Option<FuncId>,
    /// Put between `ts_` and an exported name, so several modules can link into one program.
    pub prefix: String,
}

// ---- A readable dump (for `iron_ts --dump-ir` and the doc) ----

impl fmt::Display for Ty {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Ty::Void => write!(f, "void"),
            Ty::Num => write!(f, "f64"),
            Ty::Bool => write!(f, "bool"),
            Ty::Lit => write!(f, "tag"),
            Ty::Str => write!(f, "str"),
            Ty::Null => write!(f, "null"),
            Ty::Obj(i) => write!(f, "obj#{i}"),
            Ty::Arr(t) => write!(f, "[{t}]"),
            Ty::Bytes => write!(f, "bytes"),
            Ty::Fn(t) => write!(f, "fn({} params)->{}", t.params.len(), t.ret),
            Ty::Nullable(t) => write!(f, "{t}?"),
            Ty::JsValue => write!(f, "js"),
        }
    }
}

pub fn dump(m: &Module) -> String {
    let mut s = String::new();
    for (i, st) in m.structs.iter().enumerate() {
        s += &format!("struct #{i} {} {{ ", st.name);
        for fl in &st.fields {
            s += &format!("{}: {}; ", fl.name, fl.ty);
        }
        s += "}\n";
    }
    for (i, g) in m.globals.iter().enumerate() {
        s += &format!("global #{i} {}: {}\n", g.name, g.ty);
    }
    for (i, func) in m.funcs.iter().enumerate() {
        s += &format!(
            "fn #{i} {}{}(",
            if func.exported { "export " } else { "" },
            func.name
        );
        for p in &func.params {
            s += &format!("v{p}: {}, ", func.locals[*p as usize]);
        }
        s += &format!(") -> {}\n", func.ret);
        dump_stmts(&mut s, &func.body, 1);
    }
    s
}

fn dump_stmts(s: &mut String, body: &[Stmt], depth: usize) {
    let pad = "  ".repeat(depth);
    for st in body {
        match st {
            Stmt::If(c, a, b) => {
                *s += &format!("{pad}if {} {{\n", ex(c));
                dump_stmts(s, a, depth + 1);
                if !b.is_empty() {
                    *s += &format!("{pad}}} else {{\n");
                    dump_stmts(s, b, depth + 1);
                }
                *s += &format!("{pad}}}\n");
            }
            Stmt::Block(l, b) => {
                *s += &format!("{pad}block L{l} {{\n");
                dump_stmts(s, b, depth + 1);
                *s += &format!("{pad}}}\n");
            }
            Stmt::Loop(l, b) => {
                *s += &format!("{pad}loop L{l} {{\n");
                dump_stmts(s, b, depth + 1);
                *s += &format!("{pad}}}\n");
            }
            Stmt::Assign(v, e) => *s += &format!("{pad}v{v} = {}\n", ex(e)),
            Stmt::StoreField(o, _, i, e) => *s += &format!("{pad}{}.#{i} = {}\n", ex(o), ex(e)),
            Stmt::StoreIndex(a, i, e) => *s += &format!("{pad}{}[{}] = {}\n", ex(a), ex(i), ex(e)),
            Stmt::StoreGlobal(g, e) => *s += &format!("{pad}g{g} = {}\n", ex(e)),
            Stmt::Break(l) => *s += &format!("{pad}break L{l}\n"),
            Stmt::Continue(l) => *s += &format!("{pad}continue L{l}\n"),
            Stmt::Return(e) => {
                *s += &format!("{pad}return {}\n", e.as_ref().map(ex).unwrap_or_default())
            }
            Stmt::Eval(e) => *s += &format!("{pad}{}\n", ex(e)),
        }
    }
}

fn ex(e: &Expr) -> String {
    match &e.kind {
        ExprKind::Num(n) => format!("{n}"),
        ExprKind::Bool(b) => format!("{b}"),
        ExprKind::Tag(t) => format!("tag{t}"),
        ExprKind::Null => "null".into(),
        ExprKind::Var(v) => format!("v{v}"),
        ExprKind::Global(g) => format!("g{g}"),
        ExprKind::Un(o, a) => format!("{o:?}({})", ex(a)),
        ExprKind::Bin(o, a, b) => format!("{o:?}({}, {})", ex(a), ex(b)),
        ExprKind::Cmp(o, a, b) => format!("{o:?}({}, {})", ex(a), ex(b)),
        ExprKind::And(a, b) => format!("({} && {})", ex(a), ex(b)),
        ExprKind::Or(a, b) => format!("({} || {})", ex(a), ex(b)),
        ExprKind::Cond(c, a, b) => format!("({} ? {} : {})", ex(c), ex(a), ex(b)),
        ExprKind::IsNull(a) => format!("isnull({})", ex(a)),
        ExprKind::Cast(a) => format!("cast<{}>({})", e.ty, ex(a)),
        ExprKind::Truthy(a) => format!("truthy({})", ex(a)),
        ExprKind::Field(o, _, i) => format!("{}.#{i}", ex(o)),
        ExprKind::Index(a, i) => format!("{}[{}]", ex(a), ex(i)),
        ExprKind::Len(a) => format!("len({})", ex(a)),
        ExprKind::NewObj(s) => format!("new#{s}"),
        ExprKind::Call(f, a) => format!(
            "call#{f}({})",
            a.iter().map(ex).collect::<Vec<_>>().join(", ")
        ),
        ExprKind::CallClosure(c, a) => format!(
            "callc {}({})",
            ex(c),
            a.iter().map(ex).collect::<Vec<_>>().join(", ")
        ),
        ExprKind::Rt(r, a) => format!(
            "rt.{r:?}({})",
            a.iter().map(ex).collect::<Vec<_>>().join(", ")
        ),
        ExprKind::Seq(stmts, v) => {
            let mut s = String::new();
            dump_stmts(&mut s, stmts, 3);
            format!(
                "{{
{s}      => {}}}",
                ex(v)
            )
        }
    }
}
