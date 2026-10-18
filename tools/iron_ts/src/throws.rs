//! Which functions can throw. Exceptions unwind by returning: after a call that can throw, the
//! generated code checks the pending-exception word and leaves for the handler. The program is
//! closed, so the set is computed once: a function can throw if it contains a `throw`, calls a
//! function that can, or (when anything reachable throws at all) makes a closure or virtual call,
//! whose targets are not known here. Functions outside the set pay nothing, and a function nothing
//! reaches (the prelude's helper when the program never needs it) counts for nothing.

use crate::frames::scan_stmt;
use crate::ir::{ExprKind, Module, Stmt};
use std::cell::{Cell, RefCell};

pub fn analyze(m: &mut Module) {
    let n = m.funcs.len();
    let mut direct = vec![false; n];
    let mut dynamic = vec![false; n];
    let mut callees: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut makes_virtual_call = vec![false; n];
    for (i, f) in m.funcs.iter().enumerate() {
        let calls = RefCell::new(Vec::new());
        let (throws, dynamic_call, virtual_call) =
            (Cell::new(false), Cell::new(false), Cell::new(false));
        for s in &f.body {
            scan_stmt(
                s,
                &|e| {
                    match &e.kind {
                        ExprKind::Call(fid, _) => calls.borrow_mut().push(*fid),
                        ExprKind::MakeClosure(fid, _) => calls.borrow_mut().push(*fid),
                        ExprKind::CallVirtual(..) => {
                            dynamic_call.set(true);
                            virtual_call.set(true);
                        }
                        ExprKind::CallClosure(..) => dynamic_call.set(true),
                        _ => {}
                    }
                    false
                },
                &|s| {
                    if matches!(s, Stmt::Throw(_)) {
                        throws.set(true);
                    }
                    false
                },
            );
        }
        direct[i] = throws.get();
        dynamic[i] = dynamic_call.get();
        makes_virtual_call[i] = virtual_call.get();
        callees[i] = calls.into_inner();
    }
    // What the program can reach: from the exported functions and `ts_init`, through calls,
    // closures, and (for a virtual call) every method in a method table.
    let mut reached = vec![false; n];
    let mut stack: Vec<usize> = (0..n)
        .filter(|i| m.funcs[*i].exported || m.init == Some(*i))
        .collect();
    let table_funcs: Vec<usize> = m
        .vtables
        .iter()
        .flat_map(|t| t.entries.iter().map(|(_, f)| *f))
        .collect();
    let mut tables_in = false;
    while let Some(i) = stack.pop() {
        if std::mem::replace(&mut reached[i], true) {
            continue;
        }
        stack.extend(callees[i].iter().copied());
        if makes_virtual_call[i] && !tables_in {
            tables_in = true;
            stack.extend(table_funcs.iter().copied());
        }
    }
    let any = (0..n).any(|i| reached[i] && direct[i]);
    let mut may: Vec<bool> = (0..n)
        .map(|i| reached[i] && (direct[i] || (any && dynamic[i])))
        .collect();
    loop {
        let mut changed = false;
        for i in 0..n {
            if reached[i] && !may[i] && callees[i].iter().any(|c| may[*c]) {
                may[i] = true;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    m.may_throw = may;
    m.any_throw = any;
}
