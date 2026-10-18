//! Which variables do closures capture?
//!
//! Before a function is lowered, its body is scanned for closures created in script: arrow and
//! function expressions that are not the callback of an array method (those are inlined, see
//! `arrays.rs`, and see their surroundings directly). The free names of each such closure are
//! variables of an enclosing function; every local with one of those names is stored in a heap
//! cell (`Binding::Boxed`) so the closure and the function share it.
//!
//! The scan works on names, not resolved scopes, so it can only over-approximate: a local that
//! happens to share a name with a captured one is boxed too, which costs a little speed and is
//! never wrong.

use oxc_ast::ast::*;
use oxc_ast_visit::{Visit, walk};
use oxc_syntax::scope::ScopeFlags;
use std::collections::{HashMap, HashSet};

/// Array methods whose arrow-function argument is inlined into a loop.
pub(crate) const INLINED_CALLBACKS: &[&str] =
    &["forEach", "filter", "find", "some", "every", "map"];

#[derive(Default)]
struct Frame {
    refs: HashSet<String>,
    decls: HashSet<String>,
}

#[derive(Default)]
pub(crate) struct Scan {
    pub captured: HashSet<String>,
    /// Free names of each closure, by the start offset of its source.
    pub free: HashMap<u32, Vec<String>>,
    frames: Vec<Frame>,
    inline: HashSet<u32>,
}

impl Scan {
    pub fn of_statements(stmts: &[Statement<'_>]) -> Scan {
        let mut s = Scan::default();
        s.frames.push(Frame::default());
        for st in stmts {
            s.visit_statement(st);
        }
        s
    }

    pub fn of_expression(e: &Expression<'_>) -> Scan {
        let mut s = Scan::default();
        s.frames.push(Frame::default());
        s.visit_expression(e);
        s
    }

    fn closure(&mut self, start: u32, walk_body: impl FnOnce(&mut Self)) {
        if self.inline.contains(&start) {
            walk_body(self);
            return;
        }
        self.frames.push(Frame::default());
        walk_body(self);
        let f = self.frames.pop().unwrap();
        let mut free: Vec<String> = f.refs.difference(&f.decls).cloned().collect();
        free.sort();
        self.captured.extend(free.iter().cloned());
        // What a nested closure needs from outside, the closure around it needs too.
        self.frames
            .last_mut()
            .unwrap()
            .refs
            .extend(free.iter().cloned());
        self.free.insert(start, free);
    }
}

impl<'a> Visit<'a> for Scan {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        self.frames
            .last_mut()
            .unwrap()
            .refs
            .insert(it.name.to_string());
    }

    fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        self.frames
            .last_mut()
            .unwrap()
            .decls
            .insert(it.name.to_string());
    }

    /// `this` inside an arrow function is captured like a variable.
    fn visit_this_expression(&mut self, _it: &ThisExpression) {
        self.frames
            .last_mut()
            .unwrap()
            .refs
            .insert("this".to_string());
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if let Expression::StaticMemberExpression(m) = &it.callee
            && INLINED_CALLBACKS.contains(&m.property.name.as_str())
            && let Some(Argument::ArrowFunctionExpression(a)) = it.arguments.first()
        {
            self.inline.insert(a.span.start);
        }
        walk::walk_call_expression(self, it);
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        self.closure(it.span.start, |s| {
            walk::walk_arrow_function_expression(s, it)
        });
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        self.closure(it.span.start, |s| walk::walk_function(s, it, flags));
    }
}
