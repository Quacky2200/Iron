// SPDX-License-Identifier: Apache-2.0

//! Local generic arrows over the keys of a type, as a source rewrite before parsing:
//!
//! ```ts
//! const set = <K extends keyof Profile>(key: K, value: Profile[K]) => setForm((f) => ({ ...f, [key]: value }));
//! set("name", "Ada");
//! ```
//!
//! The key is a runtime value but the field it picks decides the types, so the arrow is written
//! once per key the program calls it with (`set$name`, ..), with `K` and `[key]` replaced by the
//! literal, and each call names its copy. A call whose first argument is not a string literal is
//! left alone (and fails later: the key has to be known when compiling).

use super::R;
use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_ast_visit::{Visit, walk};
use oxc_parser::Parser;
use oxc_span::SourceType;

struct Decl {
    name: String,
    /// The statement `const name = <K extends keyof T>(key: K, ..) => ..;`.
    stmt: (usize, usize),
    /// The arrow, from after its type parameters to its end.
    arrow: (usize, usize),
    k: String,
    key: String,
}

#[derive(Default)]
struct Finder {
    decls: Vec<Decl>,
    /// (callee span, name, literal)
    calls: Vec<((usize, usize), String, String)>,
}

impl<'a> Visit<'a> for Finder {
    fn visit_variable_declaration(&mut self, d: &VariableDeclaration<'a>) {
        if d.declarations.len() == 1
            && let decl = &d.declarations[0]
            && let BindingPattern::BindingIdentifier(id) = &decl.id
            && let Some(Expression::ArrowFunctionExpression(a)) = &decl.init
            && let Some(tp) = &a.type_parameters
            && tp.params.len() == 1
            && let Some(constraint) = &tp.params[0].constraint
            && matches!(constraint, TSType::TSTypeOperatorType(o) if o.operator == TSTypeOperatorOperator::Keyof)
            && let Some(first) = a.params.items.first()
            && let BindingPattern::BindingIdentifier(key) = &first.pattern
        {
            self.decls.push(Decl {
                name: id.name.to_string(),
                stmt: (d.span.start as usize, d.span.end as usize),
                arrow: (tp.span.end as usize, a.span.end as usize),
                k: tp.params[0].name.name.to_string(),
                key: key.name.to_string(),
            });
        }
        walk::walk_variable_declaration(self, d);
    }

    fn visit_call_expression(&mut self, c: &CallExpression<'a>) {
        if let Expression::Identifier(f) = &c.callee
            && let Some(Argument::StringLiteral(s)) = c.arguments.first()
        {
            self.calls.push((
                (f.span.start as usize, f.span.end as usize),
                f.name.to_string(),
                s.value.to_string(),
            ));
        }
        walk::walk_call_expression(self, c);
    }
}

/// `old` as a whole word.
fn replace_word(src: &str, old: &str, new: &str) -> String {
    let word = |c: char| c.is_alphanumeric() || c == '_' || c == '$';
    let mut out = String::with_capacity(src.len());
    let mut rest = src;
    while let Some(i) = rest.find(old) {
        let before = rest[..i].chars().next_back();
        let after = rest[i + old.len()..].chars().next();
        out.push_str(&rest[..i]);
        if before.is_some_and(word) || after.is_some_and(word) {
            out.push_str(old);
        } else {
            out.push_str(new);
        }
        rest = &rest[i + old.len()..];
    }
    out.push_str(rest);
    out
}

pub fn desugar(src: &str) -> R<Option<String>> {
    if !src.contains("keyof ") {
        return Ok(None);
    }
    let alloc = Allocator::default();
    let ret = Parser::new(&alloc, src, SourceType::tsx()).parse();
    if !ret.diagnostics.is_empty() {
        return Ok(None); // the real parse reports it
    }
    let mut f = Finder::default();
    f.visit_program(&ret.program);
    if f.decls.is_empty() {
        return Ok(None);
    }
    // (start, end, text)
    let mut edits: Vec<(usize, usize, String)> = Vec::new();
    for d in &f.decls {
        let mut lits: Vec<&str> = Vec::new();
        for (span, name, lit) in &f.calls {
            if *name == d.name {
                if !lits.contains(&lit.as_str()) {
                    lits.push(lit);
                }
                edits.push((span.0, span.1, format!("{}${}", d.name, lit)));
            }
        }
        let arrow = &src[d.arrow.0..d.arrow.1];
        let mut copies = Vec::new();
        for lit in &lits {
            let quoted = format!("{lit:?}");
            let body = replace_word(arrow, &d.k, &quoted);
            let body = body.replace(&format!("[{}]", d.key), &format!("[{quoted}]"));
            copies.push(format!("const {}${} = {body};", d.name, lit));
        }
        if copies.is_empty() {
            continue;
        }
        edits.push((d.stmt.0, d.stmt.1, copies.join(" ")));
    }
    if edits.is_empty() {
        return Ok(None);
    }
    edits.sort_by_key(|e| std::cmp::Reverse(e.0));
    let mut out = src.to_string();
    for (a, b, t) in edits {
        out.replace_range(a..b, &t);
    }
    Ok(Some(out))
}
