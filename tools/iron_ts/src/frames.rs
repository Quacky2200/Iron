//! Planning the shadow-stack frame of a function: which locals are mirrored, and how many
//! extra slots the code generator needs to keep pointer temporaries alive across allocations.
//!
//! The rules here and the code generator's `eval_args` must agree exactly: both use
//! `spill_flags`, so the slot count computed here is the number the generator consumes.

use crate::ir::*;

pub struct FramePlan {
    /// Frame slot of each local (only pointer-typed locals have one).
    pub ptr_slot: Vec<Option<u32>>,
    /// Slots for pointer locals; spill temporaries come after them.
    pub n_locals: u32,
    pub n_spills: u32,
}

impl FramePlan {
    pub fn total(&self) -> u32 {
        self.n_locals + self.n_spills
    }
}

pub fn plan(f: &Func) -> FramePlan {
    let mut ptr_slot = Vec::new();
    let mut n = 0;
    for t in &f.locals {
        if t.is_ptr() {
            ptr_slot.push(Some(n));
            n += 1;
        } else {
            ptr_slot.push(None);
        }
    }
    let mut spills = 0;
    count_stmts(&f.body, &mut spills);
    FramePlan {
        ptr_slot,
        n_locals: n,
        n_spills: spills,
    }
}

/// A pointer that stays reachable through a root while the other operands are evaluated.
fn is_stable(e: &Expr) -> bool {
    match &e.kind {
        ExprKind::Var(_) | ExprKind::Global(_) | ExprKind::Null => true,
        ExprKind::Cast(a) => is_stable(a),
        // Reachable from a stable object; only a store during the other operands could drop it.
        ExprKind::Field(o, ..) => is_stable(o),
        _ => false,
    }
}

/// Which operands (evaluated in order) must be spilled to the frame because a later operand
/// can allocate and so collect.
pub fn spill_flags(args: &[&Expr]) -> Vec<bool> {
    (0..args.len())
        .map(|i| {
            args[i].ty.is_ptr() && !is_stable(args[i]) && args[i + 1..].iter().any(|a| may_alloc(a))
        })
        .collect()
}

fn n_spills(args: &[&Expr]) -> u32 {
    spill_flags(args).iter().filter(|b| **b).count() as u32
}

/// Can evaluating this expression run a collection?
pub fn may_alloc(e: &Expr) -> bool {
    match &e.kind {
        ExprKind::NewObj(_) | ExprKind::Call(..) | ExprKind::CallClosure(..) => true,
        ExprKind::Rt(RtFn::ArrNew | RtFn::BytesNew, _) => true,
        ExprKind::Seq(stmts, v) => stmts.iter().any(stmt_may_alloc) || may_alloc(v),
        _ => children(e).iter().any(|c| may_alloc(c)),
    }
}

fn stmt_may_alloc(s: &Stmt) -> bool {
    match s {
        Stmt::Assign(_, e) | Stmt::StoreGlobal(_, e) | Stmt::Eval(e) => may_alloc(e),
        Stmt::Return(e) => e.as_ref().is_some_and(may_alloc),
        Stmt::StoreField(o, _, _, v) => may_alloc(o) || may_alloc(v),
        Stmt::StoreIndex(a, i, v) => may_alloc(a) || may_alloc(i) || may_alloc(v),
        Stmt::If(c, a, b) => {
            may_alloc(c) || a.iter().any(stmt_may_alloc) || b.iter().any(stmt_may_alloc)
        }
        Stmt::Block(_, b) | Stmt::Loop(_, b) => b.iter().any(stmt_may_alloc),
        Stmt::Break(_) | Stmt::Continue(_) => false,
    }
}

/// Direct sub-expressions (not the statements inside a `Seq`).
fn children(e: &Expr) -> Vec<&Expr> {
    match &e.kind {
        ExprKind::Un(_, a)
        | ExprKind::IsNull(a)
        | ExprKind::Cast(a)
        | ExprKind::Truthy(a)
        | ExprKind::Field(a, ..)
        | ExprKind::Len(a) => vec![a],
        ExprKind::Bin(_, a, b)
        | ExprKind::Cmp(_, a, b)
        | ExprKind::And(a, b)
        | ExprKind::Or(a, b)
        | ExprKind::Index(a, b) => vec![a, b],
        ExprKind::Cond(c, a, b) => vec![c, a, b],
        ExprKind::Call(_, args) | ExprKind::Rt(_, args) => args.iter().collect(),
        ExprKind::CallClosure(c, args) => std::iter::once(&**c).chain(args.iter()).collect(),
        ExprKind::Seq(_, v) => vec![v],
        _ => vec![],
    }
}

fn count_expr(e: &Expr, n: &mut u32) {
    match &e.kind {
        ExprKind::Call(_, args) | ExprKind::Rt(_, args) => {
            *n += n_spills(&args.iter().collect::<Vec<_>>());
        }
        ExprKind::CallClosure(c, args) => {
            let v: Vec<&Expr> = std::iter::once(&**c).chain(args.iter()).collect();
            *n += n_spills(&v);
        }
        ExprKind::Index(a, i) => *n += n_spills(&[a, i]),
        ExprKind::Seq(stmts, _) => count_stmts(stmts, n),
        _ => {}
    }
    for c in children(e) {
        count_expr(c, n);
    }
}

fn count_stmts(stmts: &[Stmt], n: &mut u32) {
    for s in stmts {
        match s {
            Stmt::Assign(_, e) | Stmt::StoreGlobal(_, e) | Stmt::Eval(e) => count_expr(e, n),
            Stmt::Return(e) => {
                if let Some(e) = e {
                    count_expr(e, n)
                }
            }
            Stmt::StoreField(o, _, _, v) => {
                *n += n_spills(&[o, v]);
                count_expr(o, n);
                count_expr(v, n);
            }
            Stmt::StoreIndex(a, i, v) => {
                *n += n_spills(&[a, i, v]);
                count_expr(a, n);
                count_expr(i, n);
                count_expr(v, n);
            }
            Stmt::If(c, a, b) => {
                count_expr(c, n);
                count_stmts(a, n);
                count_stmts(b, n);
            }
            Stmt::Block(_, b) | Stmt::Loop(_, b) => count_stmts(b, n),
            Stmt::Break(_) | Stmt::Continue(_) => {}
        }
    }
}
