// SPDX-License-Identifier: Apache-2.0

//! Paths a program builds at run time: which asset families does its code name?
//!
//! A template literal or a `+` chain with a literal folder in front (`` `assets/sfx/${n}.wav` ``,
//! `"assets/" + name + ".png"`) can name many files. Each hole becomes `*` (one path part), so
//! the first is `assets/sfx/*.wav`. Whole literals are reported as they are, so a build can warn
//! about one that names no file. Plan: `docs/research/platypus-virtual-fs-plan.md` section 2.6 (c).

use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_ast_visit::{Visit, walk};
use oxc_parser::Parser;
use oxc_span::SourceType;

/// What a program says about asset paths. `line` is the 1-based line of the source text.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct AssetPaths {
    /// Dynamic paths as globs (`assets/emoji/*.svg`): a literal folder first, at least one hole,
    /// and a file name with a dot. Sorted, one row per glob and line.
    pub patterns: Vec<(String, usize)>,
    /// Whole string literals without whitespace (candidates; the caller keeps the asset-looking ones).
    pub literals: Vec<(String, usize)>,
}

enum Part {
    Lit(String),
    Hole,
}

struct Scan<'s> {
    source: &'s str,
    out: AssetPaths,
}

impl Scan<'_> {
    fn line(&self, offset: u32) -> usize {
        self.source.as_bytes()[..(offset as usize).min(self.source.len())]
            .iter()
            .filter(|b| **b == b'\n')
            .count()
            + 1
    }

    fn parts(&mut self, e: &Expression<'_>, parts: &mut Vec<Part>) {
        match e {
            Expression::StringLiteral(s) => parts.push(Part::Lit(s.value.to_string())),
            Expression::TemplateLiteral(t) => {
                for (i, q) in t.quasis.iter().enumerate() {
                    let text = q
                        .value
                        .cooked
                        .as_ref()
                        .map_or_else(|| q.value.raw.to_string(), |c| c.to_string());
                    if !text.is_empty() {
                        parts.push(Part::Lit(text));
                    }
                    if let Some(h) = t.expressions.get(i) {
                        self.hole(h, parts);
                    }
                }
            }
            Expression::ParenthesizedExpression(p) => self.parts(&p.expression, parts),
            Expression::BinaryExpression(b) if b.operator == BinaryOperator::Addition => {
                self.parts(&b.left, parts);
                self.parts(&b.right, parts);
            }
            other => self.hole(other, parts),
        }
    }

    /// A value only known at run time; looks inside it for patterns of its own.
    fn hole(&mut self, e: &Expression<'_>, parts: &mut Vec<Part>) {
        parts.push(Part::Hole);
        self.visit_expression(e);
    }
}

impl<'a> Visit<'a> for Scan<'_> {
    fn visit_template_literal(&mut self, it: &TemplateLiteral<'a>) {
        if it.expressions.is_empty() {
            let text = it
                .quasis
                .first()
                .and_then(|q| q.value.cooked.as_ref().map(|c| c.to_string()));
            if let Some(v) = text.filter(|v| !v.is_empty() && !v.chars().any(char::is_whitespace)) {
                let line = self.line(it.span.start);
                self.out.literals.push((v, line));
            }
            return;
        }
        let mut glob_parts: Vec<Part> = Vec::new();
        for (i, q) in it.quasis.iter().enumerate() {
            let text = q
                .value
                .cooked
                .as_ref()
                .map_or_else(|| q.value.raw.to_string(), |c| c.to_string());
            if !text.is_empty() {
                glob_parts.push(Part::Lit(text));
            }
            if let Some(h) = it.expressions.get(i) {
                glob_parts.push(Part::Hole);
                self.visit_expression(h);
            }
        }
        self.record_parts(glob_parts, it.span.start);
    }

    fn visit_binary_expression(&mut self, it: &BinaryExpression<'a>) {
        if it.operator != BinaryOperator::Addition {
            walk::walk_binary_expression(self, it);
            return;
        }
        let mut parts = Vec::new();
        self.parts(&it.left, &mut parts);
        self.parts(&it.right, &mut parts);
        self.record_parts(parts, it.span.start);
    }

    fn visit_string_literal(&mut self, it: &StringLiteral<'a>) {
        let v = it.value.as_str();
        if !v.is_empty() && !v.chars().any(char::is_whitespace) {
            let line = self.line(it.span.start);
            self.out.literals.push((v.to_string(), line));
        }
    }
}

impl Scan<'_> {
    fn record_parts(&mut self, parts: Vec<Part>, offset: u32) {
        if !parts.iter().any(|p| matches!(p, Part::Hole))
            || !parts.iter().any(|p| matches!(p, Part::Lit(_)))
        {
            return;
        }
        let mut glob = String::new();
        for p in &parts {
            match p {
                Part::Lit(s) => glob.push_str(s),
                Part::Hole if !glob.ends_with('*') => glob.push('*'),
                Part::Hole => {}
            }
        }
        let first = glob.split('/').next().unwrap_or("");
        let last = glob.rsplit('/').next().unwrap_or("");
        let ok = glob.contains('/')
            && !first.is_empty()
            && !first.contains(['*', ':', ' '])
            && !glob.contains("://")
            && !glob.chars().any(char::is_whitespace)
            && last.contains('.')
            && !last.ends_with('.');
        if ok {
            let line = self.line(offset);
            self.out.patterns.push((glob, line));
        }
    }
}

/// Scans a bundled TypeScript (or TSX) `source`. A source that does not parse gives nothing.
pub fn asset_paths(source: &str) -> AssetPaths {
    for st in [SourceType::tsx(), SourceType::ts()] {
        let alloc = Allocator::default();
        let ret = Parser::new(&alloc, source, st).parse();
        if ret.diagnostics.is_empty() {
            return run(source, &ret.program);
        }
    }
    AssetPaths::default()
}

fn run(source: &str, program: &Program<'_>) -> AssetPaths {
    let mut scan = Scan {
        source,
        out: AssetPaths::default(),
    };
    scan.visit_program(program);
    let mut out = scan.out;
    out.patterns.sort();
    out.patterns.dedup();
    out.literals.sort();
    out.literals.dedup();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn globs(src: &str) -> Vec<String> {
        asset_paths(src).patterns.into_iter().map(|p| p.0).collect()
    }

    // M9-TS-2551
    #[test]
    fn templates_and_plus_chains_become_globs_one_part_per_hole() {
        let g = globs(
            "const a = `assets/sfx/${n}.${ext(n)}`;\n\
             const b = \"assets/\" + name + \".png\";\n\
             const c = `assets/emoji/${code}.svg`;\n\
             const d = `assets/${theme}/tex/${`${a}${b}`}.png`;",
        );
        assert_eq!(
            g,
            [
                "assets/*.png",
                "assets/*/tex/*.png",
                "assets/emoji/*.svg",
                "assets/sfx/*.*"
            ]
        );
    }

    // M9-TS-2551
    #[test]
    fn patterns_without_a_folder_or_a_file_name_are_refused() {
        let g = globs(
            "const a = `${dir}/x.png`;\nconst b = `assets/${n}`;\nconst c = `${n}.png`;\n\
             const d = \"v\" + 1;\nconst e = `https://x.com/${n}.png`;\nconst f = `hello ${n}.png`;",
        );
        assert!(g.is_empty(), "{g:?}");
    }

    // M9-TS-2551
    #[test]
    fn whole_literals_and_lines_are_reported() {
        let r = asset_paths(
            "const a = 'assets/a.png';\n\nconst b = `assets/b.wav`;\nconst t = `assets/${x}.svg`;",
        );
        let lits: Vec<_> = r.literals.iter().map(|l| (l.0.as_str(), l.1)).collect();
        assert_eq!(lits, [("assets/a.png", 1), ("assets/b.wav", 3)]);
        assert_eq!(r.patterns, [("assets/*.svg".to_string(), 4)]);
    }
}
