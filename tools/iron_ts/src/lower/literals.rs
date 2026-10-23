// SPDX-License-Identifier: Apache-2.0

//! Where a program's text is code: the spans of its string literals, template text and comments, so
//! text-level passes (the window message glue, the native name aliases) leave what a string says
//! alone (a Markdown reader's docs mention `.request<..>(` and `AnimatorController`).

use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_ast_visit::Visit;
use oxc_parser::Parser;
use oxc_span::SourceType;

#[derive(Default)]
struct Spans {
    spans: Vec<(usize, usize)>,
}

impl<'a> Visit<'a> for Spans {
    fn visit_string_literal(&mut self, s: &StringLiteral<'a>) {
        self.spans
            .push((s.span.start as usize, s.span.end as usize));
    }

    fn visit_template_element(&mut self, t: &TemplateElement<'a>) {
        self.spans
            .push((t.span.start as usize, t.span.end as usize));
    }

    fn visit_jsx_text(&mut self, t: &JSXText<'a>) {
        self.spans
            .push((t.span.start as usize, t.span.end as usize));
    }
}

/// The byte spans of `src` that are not code (string literals with their quotes, template text,
/// JSX text, comments), sorted. Empty when `src` does not parse (then every byte counts as code).
pub(crate) fn literal_spans(src: &str) -> Vec<(usize, usize)> {
    let alloc = Allocator::default();
    let mut ret = Parser::new(&alloc, src, SourceType::tsx()).parse();
    if !ret.diagnostics.is_empty() {
        ret = Parser::new(&alloc, src, SourceType::ts()).parse();
        if !ret.diagnostics.is_empty() {
            return Vec::new();
        }
    }
    let mut v = Spans::default();
    v.visit_program(&ret.program);
    for c in &ret.program.comments {
        v.spans.push((c.span.start as usize, c.span.end as usize));
    }
    v.spans.sort_unstable();
    v.spans
}

/// `src` with every literal span (see [`literal_spans`]) blanked to one space: what text searches
/// for code should look at.
pub(crate) fn code_only(src: &str) -> String {
    let spans = literal_spans(src);
    if spans.is_empty() {
        return src.to_string();
    }
    let mut out = String::with_capacity(src.len());
    let mut at = 0;
    for (s, e) in spans {
        if s < at {
            continue;
        }
        out.push_str(&src[at..s]);
        out.push(' ');
        at = e;
    }
    out.push_str(&src[at..]);
    out
}

/// Whether byte `i` of the text is inside one of `spans` (sorted).
pub(crate) fn in_spans(spans: &[(usize, usize)], i: usize) -> bool {
    let k = spans.partition_point(|&(s, _)| s <= i);
    k > 0 && i < spans[k - 1].1
}

#[cfg(test)]
mod tests {
    use super::*;

    // M9-TS-3363
    #[test]
    fn strings_comments_and_template_text_are_not_code() {
        let src = "const a = \"x.request<{ n }>(1)\"; // .ask<Q>(\nconst b = `t ${a.length} .on(`; f.request<R>(1);";
        let code = code_only(src);
        assert!(!code.contains("x.request"), "{code}");
        assert!(!code.contains(".ask<"), "{code}");
        assert!(!code.contains(" .on("), "{code}");
        assert!(
            code.contains("a.length"),
            "template expressions stay: {code}"
        );
        assert!(code.contains("f.request<R>(1)"), "{code}");
        let spans = literal_spans(src);
        assert!(in_spans(&spans, src.find("x.request").unwrap()));
        assert!(!in_spans(&spans, src.find("f.request").unwrap()));
    }
}
