//! JSX and hooks for the native UI (`iron_ui_native`), as a source rewrite before parsing.
//!
//! - `<box style={{ width: 10 }} onClick={f}>kids</box>` becomes builder calls of the `Ui`
//!   namespace: `Ui.child(Ui.on(Ui.style(Ui.begin("box"), "width", 10), "click", __handler(f)), ...)`.
//!   The calls pick their version by argument type (a number, string, boolean, element, array of
//!   elements), see `Manifest::overloads`.
//! - `<Counter start={1} />` becomes `Ui.component(ID, (): UiElement => Counter({ start: 1 }))`; ID is a
//!   hash of the component's name, stable across builds (it says which state belongs to which component).
//! - `const [n, setN] = useState(0)` becomes `const __st = useState(0); const n = __st.value; const setN = __st.set;`
//!   (`useState` is the generic function of `ui.ts`).
//! - `useEffect(f, [a, b])` becomes `(Ui.depsBegin(), Ui.dep(a), Ui.dep(b), Ui.effect(...))`; `useMemo` and
//!   `useCallback` take their dependency list the same way. Without a list they run on every render.
//!
//! The rewrite runs in passes: each rewrites the outermost nodes and keeps the text of everything inside
//! them, so a nested element is handled by the next pass.

use super::R;
use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_ast_visit::{Visit, walk};
use oxc_parser::Parser;
use oxc_span::{GetSpan, SourceType};
use std::collections::{HashMap, HashSet};

/// The prelude: the event class, the handler wrapper and the generic hooks.
pub const UI_PRELUDE: &str = include_str!("ui.ts");

/// Does the program use JSX or the hooks?
pub fn wanted(src: &str) -> bool {
    src.contains("</")
        || src.contains("/>")
        || src.contains("<>")
        || [
            "useState",
            "useEffect",
            "useMemo",
            "useCallback",
            "useRef",
            "useReducer",
            "useContext",
            "createContext",
            "memo",
            "forwardRef",
        ]
        .iter()
        .any(|h| src.contains(&format!("{h}(")) || src.contains(&format!("{h}<")))
}

/// A stable component id from its name (FNV-1a, 31 bits).
fn component_id(name: &str) -> u32 {
    let mut h: u32 = 0x811c_9dc5;
    for b in name.bytes() {
        h ^= u32::from(b);
        h = h.wrapping_mul(0x0100_0193);
    }
    h & 0x7fff_ffff
}

/// A JavaScript string literal for `s`.
fn js_string(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// JSX text with React's whitespace rules: lines are trimmed, blank lines dropped, the rest joined by spaces.
fn jsx_text(raw: &str) -> String {
    let lines: Vec<&str> = raw.split('\n').collect();
    let last = lines.len() - 1;
    let mut parts = Vec::new();
    for (i, l) in lines.iter().enumerate() {
        let mut l = l.replace('\t', " ");
        if i != 0 {
            l = l.trim_start().to_string();
        }
        if i != last {
            l = l.trim_end().to_string();
        }
        if !l.is_empty() {
            parts.push(l);
        }
    }
    parts.join(" ")
}

struct Edit {
    start: usize,
    end: usize,
    text: String,
}

struct Finder<'s> {
    src: &'s str,
    edits: Vec<Edit>,
    error: Option<String>,
    /// `const Fast = memo(Slow)`: `Fast` -> `Slow`.
    memo: HashMap<String, String>,
    /// Components made by `forwardRef`: they take the ref as a second argument.
    forward: HashSet<String>,
}

impl<'s> Finder<'s> {
    fn slice(&self, sp: oxc_span::Span) -> &'s str {
        &self.src[sp.start as usize..sp.end as usize]
    }

    fn fail(&mut self, msg: String) {
        if self.error.is_none() {
            self.error = Some(msg);
        }
    }

    fn line(&self, offset: u32) -> usize {
        self.src[..offset as usize].matches('\n').count() + 1
    }

    /// The text of an attribute value.
    fn attr_value(&mut self, v: &Option<JSXAttributeValue<'_>>, at: u32) -> String {
        match v {
            None => "true".into(),
            Some(JSXAttributeValue::StringLiteral(s)) => js_string(s.value.as_str()),
            Some(JSXAttributeValue::ExpressionContainer(c)) => match &c.expression {
                JSXExpression::EmptyExpression(_) => {
                    self.fail(format!("line {}: an empty attribute value", self.line(at)));
                    "null".into()
                }
                e => self.slice(e.span()).to_string(),
            },
            Some(_) => {
                self.fail(format!(
                    "line {}: an element as an attribute value is not supported",
                    self.line(at)
                ));
                "null".into()
            }
        }
    }

    fn element_text(&mut self, e: &JSXElement<'_>) -> String {
        let open = &e.opening_element;
        let at = open.span.start;
        let (name, is_component) = match &open.name {
            JSXElementName::Identifier(i) => (i.name.to_string(), false),
            JSXElementName::IdentifierReference(i) => (i.name.to_string(), true),
            // `<Theme.Provider value={v}>`
            JSXElementName::MemberExpression(m) if m.property.name == "Provider" => {
                if let JSXMemberExpressionObject::IdentifierReference(ctx) = &m.object {
                    return self.provider_text(ctx.name.as_str(), e);
                }
                self.fail(format!(
                    "line {}: a provider is `Name.Provider`",
                    self.line(at)
                ));
                return String::new();
            }
            _ => {
                self.fail(format!(
                    "line {}: only plain tag names are supported",
                    self.line(at)
                ));
                return String::new();
            }
        };
        if is_component {
            return self.component_text(&name, e);
        }
        let mut out = format!("Ui.begin({})", js_string(&name));
        for item in &open.attributes {
            let JSXAttributeItem::Attribute(a) = item else {
                self.fail(format!(
                    "line {}: spread attributes are not supported",
                    self.line(at)
                ));
                return out;
            };
            let JSXAttributeName::Identifier(n) = &a.name else {
                self.fail(format!(
                    "line {}: namespaced attributes are not supported",
                    self.line(at)
                ));
                return out;
            };
            let n = n.name.as_str();
            match n {
                "key" => {
                    let v = self.attr_value(&a.value, at);
                    out = format!("Ui.key({out}, {v})");
                }
                "ref" => {
                    let v = self.attr_value(&a.value, at);
                    out = format!("Ui.ref({out}, {v})");
                }
                "style" => {
                    let Some(JSXAttributeValue::ExpressionContainer(c)) = &a.value else {
                        self.fail(format!(
                            "line {}: style must be {{{{ ... }}}}",
                            self.line(at)
                        ));
                        return out;
                    };
                    let Some(Expression::ObjectExpression(o)) = c.expression.as_expression() else {
                        self.fail(format!(
                            "line {}: style must be an object literal ({{{{ width: 10 }}}})",
                            self.line(at)
                        ));
                        return out;
                    };
                    for p in &o.properties {
                        let ObjectPropertyKind::ObjectProperty(p) = p else {
                            self.fail(format!("line {}: spread in style", self.line(at)));
                            return out;
                        };
                        let key = match &p.key {
                            PropertyKey::StaticIdentifier(k) => k.name.to_string(),
                            PropertyKey::StringLiteral(k) => k.value.to_string(),
                            _ => {
                                self.fail(format!(
                                    "line {}: style keys must be plain names",
                                    self.line(at)
                                ));
                                return out;
                            }
                        };
                        let v = self.slice(p.value.span()).to_string();
                        out = format!("Ui.style({out}, {}, {v})", js_string(&key));
                    }
                }
                n if n.len() > 2 && n.starts_with("on") && n.as_bytes()[2].is_ascii_uppercase() => {
                    let v = self.attr_value(&a.value, at);
                    let mut kind = n[2..].to_lowercase();
                    let mut capture = false;
                    if let Some(k) = kind.strip_suffix("capture") {
                        kind = k.to_string();
                        capture = true;
                    }
                    if kind == "change" {
                        kind = "input".into();
                    }
                    if capture {
                        kind.push_str(":capture");
                    }
                    out = format!("Ui.on({out}, {}, __handler({v}))", js_string(&kind));
                }
                n => {
                    let v = self.attr_value(&a.value, at);
                    out = format!("Ui.attr({out}, {}, {v})", js_string(n));
                }
            }
        }
        self.children_text(&mut out, &e.children, at);
        out
    }

    fn children_text(&mut self, out: &mut String, children: &[JSXChild<'_>], at: u32) {
        for c in children {
            match c {
                JSXChild::Text(t) => {
                    let s = jsx_text(t.value.as_str());
                    if !s.is_empty() {
                        *out = format!("Ui.child({out}, {})", js_string(&s));
                    }
                }
                JSXChild::ExpressionContainer(c) => match &c.expression {
                    JSXExpression::EmptyExpression(_) => {}
                    e => {
                        let t = self.slice(e.span()).to_string();
                        *out = format!("Ui.child({out}, {t})");
                    }
                },
                JSXChild::Element(el) => {
                    let t = self.slice(el.span).to_string();
                    *out = format!("Ui.child({out}, {t})");
                }
                JSXChild::Fragment(f) => {
                    let t = self.slice(f.span).to_string();
                    *out = format!("Ui.child({out}, {t})");
                }
                JSXChild::Spread(_) => self.fail(format!(
                    "line {}: spread children are not supported",
                    self.line(at)
                )),
            }
        }
    }

    fn component_text(&mut self, name: &str, e: &JSXElement<'_>) -> String {
        let open = &e.opening_element;
        let at = open.span.start;
        let target = self
            .memo
            .get(name)
            .cloned()
            .unwrap_or_else(|| name.to_string());
        let is_memo = target != name;
        let is_forward = self.forward.contains(&target);
        let mut props = Vec::new();
        let mut dep_values = Vec::new();
        let mut ref_value = None;
        let mut key = None;
        for item in &open.attributes {
            let JSXAttributeItem::Attribute(a) = item else {
                self.fail(format!(
                    "line {}: spread props are not supported",
                    self.line(at)
                ));
                return String::new();
            };
            let JSXAttributeName::Identifier(n) = &a.name else {
                self.fail(format!("line {}: namespaced props", self.line(at)));
                return String::new();
            };
            let v = self.attr_value(&a.value, at);
            if n.name == "key" {
                key = Some(v);
            } else if n.name == "ref" && is_forward {
                ref_value = Some(v);
            } else {
                dep_values.push(v.clone());
                props.push(format!("{}: {v}", n.name));
            }
        }
        let mut kids = Vec::new();
        for c in &e.children {
            match c {
                JSXChild::Element(el) => kids.push(self.slice(el.span).to_string()),
                JSXChild::Fragment(f) => kids.push(self.slice(f.span).to_string()),
                JSXChild::ExpressionContainer(c) => {
                    if let Some(x) = c.expression.as_expression() {
                        kids.push(self.slice(x.span()).to_string());
                    }
                }
                JSXChild::Text(t) if jsx_text(t.value.as_str()).is_empty() => {}
                _ => self.fail(format!(
                    "line {}: a component takes elements as children, not text",
                    self.line(at)
                )),
            }
        }
        if !kids.is_empty() {
            props.push(format!("children: [{}]", kids.join(", ")));
        }
        let call = if is_forward {
            format!(
                "{target}({{ {} }}, {})",
                props.join(", "),
                ref_value.as_deref().unwrap_or("null")
            )
        } else if props.is_empty() {
            format!("{target}()")
        } else {
            format!("{target}({{ {} }})", props.join(", "))
        };
        let mut out = format!(
            "Ui.component({}, (): UiElement => {call})",
            component_id(name)
        );
        if let Some(k) = key {
            out = format!("Ui.key({out}, {k})");
        }
        if is_memo {
            // The props are the component's dependencies: equal ones skip the render.
            let mut seq = vec!["Ui.depsBegin()".to_string()];
            seq.extend(dep_values.iter().map(|v| format!("Ui.dep({v})")));
            seq.push(out);
            out = format!("({})", seq.join(", "));
        }
        out
    }

    /// `<Ctx.Provider value={v} key="k">children</Ctx.Provider>`.
    fn provider_text(&mut self, ctx: &str, e: &JSXElement<'_>) -> String {
        let at = e.opening_element.span.start;
        let mut value = None;
        let mut key = None;
        for item in &e.opening_element.attributes {
            let JSXAttributeItem::Attribute(a) = item else {
                self.fail(format!("line {}: spread on a provider", self.line(at)));
                return String::new();
            };
            let JSXAttributeName::Identifier(n) = &a.name else {
                self.fail(format!("line {}: namespaced attribute", self.line(at)));
                return String::new();
            };
            let v = self.attr_value(&a.value, at);
            match n.name.as_str() {
                "value" => value = Some(v),
                "key" => key = Some(v),
                other => self.fail(format!(
                    "line {}: a provider takes `value` and `key`, not `{other}`",
                    self.line(at)
                )),
            }
        }
        let Some(value) = value else {
            self.fail(format!(
                "line {}: a provider needs a `value`",
                self.line(at)
            ));
            return String::new();
        };
        let mut out = format!("Ui.provider({ctx}.id, {value})");
        if let Some(k) = key {
            out = format!("Ui.key({out}, {k})");
        }
        self.children_text(&mut out, &e.children, at);
        out
    }

    fn fragment_text(&mut self, f: &JSXFragment<'_>) -> String {
        let mut out = String::from("Ui.fragment()");
        self.children_text(&mut out, &f.children, f.span.start);
        out
    }
}

impl<'a> Visit<'a> for Finder<'_> {
    fn visit_jsx_element(&mut self, e: &JSXElement<'a>) {
        let text = self.element_text(e);
        self.edits.push(Edit {
            start: e.span.start as usize,
            end: e.span.end as usize,
            text,
        });
    }

    fn visit_jsx_fragment(&mut self, f: &JSXFragment<'a>) {
        let text = self.fragment_text(f);
        self.edits.push(Edit {
            start: f.span.start as usize,
            end: f.span.end as usize,
            text,
        });
    }

    fn visit_variable_declaration(&mut self, d: &VariableDeclaration<'a>) {
        // `const [a, setA] = useState(init);`
        if d.declarations.len() == 1
            && let decl = &d.declarations[0]
            && let BindingPattern::ArrayPattern(arr) = &decl.id
            && let Some(Expression::CallExpression(call)) = &decl.init
            && let Expression::Identifier(hook) = &call.callee
            && matches!(hook.name.as_str(), "useState" | "useReducer")
        {
            let (first, second) = if hook.name == "useState" {
                ("value", "set")
            } else {
                ("state", "dispatch")
            };
            let names: Vec<Option<String>> = arr
                .elements
                .iter()
                .map(|el| match el {
                    Some(BindingPattern::BindingIdentifier(b)) => Some(b.name.to_string()),
                    _ => None,
                })
                .collect();
            if arr.rest.is_some() || names.len() > 2 {
                self.fail(format!(
                    "line {}: useState returns [value, setter]",
                    self.line(d.span.start)
                ));
                return;
            }
            let tmp = format!("__st{}", d.span.start);
            let mut text = format!("const {tmp} = {};", self.slice(call.span));
            if let Some(Some(v)) = names.first() {
                text += &format!(" const {v} = {tmp}.{first};");
            }
            if let Some(Some(s)) = names.get(1) {
                text += &format!(" const {s} = {tmp}.{second};");
            }
            // The declaration's span has no `;`; a trailing one in the source stays harmless.
            self.edits.push(Edit {
                start: d.span.start as usize,
                end: d.span.end as usize,
                text: text.trim_end_matches(';').to_string(),
            });
            return;
        }
        walk::walk_variable_declaration(self, d);
    }

    fn visit_call_expression(&mut self, c: &CallExpression<'a>) {
        if let Expression::Identifier(f) = &c.callee
            && matches!(
                f.name.as_str(),
                "useEffect" | "useMemo" | "useCallback" | "useImperativeHandle"
            )
            && (c.arguments.len() == 2 || f.name == "useEffect" || c.arguments.len() == 3)
            && let Some(first) = c.arguments.first().and_then(|a| a.as_expression())
        {
            // `useImperativeHandle(ref, create, deps)` has its list third.
            let dep_at = if f.name == "useImperativeHandle" {
                2
            } else {
                1
            };
            if f.name == "useImperativeHandle" && c.arguments.len() < 3 {
                return;
            }
            let deps = match c.arguments.get(dep_at).and_then(|a| a.as_expression()) {
                Some(Expression::ArrayExpression(arr)) => {
                    let mut seq = vec!["Ui.depsBegin()".to_string()];
                    for el in &arr.elements {
                        match el.as_expression() {
                            Some(x) => seq.push(format!("Ui.dep({})", self.slice(x.span()))),
                            None => {
                                self.fail(format!(
                                    "line {}: a dependency list holds expressions",
                                    self.line(c.span.start)
                                ));
                                return;
                            }
                        }
                    }
                    Some(seq)
                }
                None => None,
                Some(_) => {
                    self.fail(format!(
                        "line {}: the dependencies must be an array literal",
                        self.line(c.span.start)
                    ));
                    return;
                }
            };
            let f_text = self.slice(first.span()).to_string();
            let call = if f.name == "useEffect" {
                format!("Ui.effect((): void => {{ Ui.runEffect({f_text}); }})")
            } else if f.name == "useImperativeHandle" {
                let second = c.arguments.get(1).and_then(|a| a.as_expression());
                let create = second.map_or("", |x| self.slice(x.span()));
                format!("useImperativeHandle({f_text}, {create})")
            } else {
                format!("{}({f_text})", f.name)
            };
            let text = match deps {
                Some(mut seq) => {
                    seq.push(call);
                    format!("({})", seq.join(", "))
                }
                None if f.name == "useEffect" => call,
                None => return,
            };
            self.edits.push(Edit {
                start: c.span.start as usize,
                end: c.span.end as usize,
                text,
            });
            return;
        }
        walk::walk_call_expression(self, c);
    }
}

/// The program with JSX and hooks rewritten, or `None` when it has none.
pub fn desugar(src: &str) -> R<Option<String>> {
    if !wanted(src) {
        return Ok(None);
    }
    let (mut cur, memo, forward) = prepass(src)?;
    let mut changed = cur != src;
    for _ in 0..64 {
        let alloc = Allocator::default();
        let ret = Parser::new(&alloc, &cur, SourceType::tsx()).parse();
        if !ret.diagnostics.is_empty() {
            // The ordinary parse reports it with its line.
            return Ok(if changed { Some(cur) } else { None });
        }
        let mut finder = Finder {
            src: &cur,
            edits: Vec::new(),
            error: None,
            memo: memo.clone(),
            forward: forward.clone(),
        };
        finder.visit_program(&ret.program);
        if let Some(e) = finder.error {
            return Err(e);
        }
        let mut edits = finder.edits;
        if edits.is_empty() {
            return Ok(if changed { Some(cur) } else { None });
        }
        edits.sort_by_key(|e| e.start);
        let mut out = String::with_capacity(cur.len());
        let mut at = 0;
        for e in &edits {
            out.push_str(&cur[at..e.start]);
            out.push_str(&e.text);
            at = e.end;
        }
        out.push_str(&cur[at..]);
        // Keep the line count: the replacement holds the original's newlines.
        let lost = cur.matches('\n').count() as isize - out.matches('\n').count() as isize;
        if lost > 0 {
            out.push_str(&"\n".repeat(lost as usize));
        }
        cur = out;
        changed = true;
    }
    Err("JSX nests too deeply".into())
}

/// `const Fast = memo(Slow)` and `const Box = forwardRef((props, ref) => ...)` declarations: the
/// first is erased (the names are remembered), the second becomes a function taking the ref as its
/// second parameter. Returns the text and the two tables.
fn prepass(src: &str) -> R<(String, HashMap<String, String>, HashSet<String>)> {
    let mut memo = HashMap::new();
    let mut forward = HashSet::new();
    if !src.contains("memo(") && !src.contains("forwardRef(") {
        return Ok((src.to_string(), memo, forward));
    }
    let alloc = Allocator::default();
    let ret = Parser::new(&alloc, src, SourceType::tsx()).parse();
    if !ret.diagnostics.is_empty() {
        return Ok((src.to_string(), memo, forward));
    }
    let slice = |sp: oxc_span::Span| &src[sp.start as usize..sp.end as usize];
    let mut edits: Vec<Edit> = Vec::new();
    for st in &ret.program.body {
        let (decl, exported, span) = match st {
            Statement::VariableDeclaration(d) => (&**d, false, d.span),
            Statement::ExportDeclaration(x) => match &x.declaration {
                Declaration::VariableDeclaration(d) => (&**d, true, x.span),
                _ => continue,
            },
            _ => continue,
        };
        let [d] = decl.declarations.as_slice() else {
            continue;
        };
        let (BindingPattern::BindingIdentifier(id), Some(Expression::CallExpression(call))) =
            (&d.id, &d.init)
        else {
            continue;
        };
        let Expression::Identifier(f) = &call.callee else {
            continue;
        };
        let name = id.name.to_string();
        let arg = call.arguments.first().and_then(|a| a.as_expression());
        match (f.name.as_str(), arg) {
            ("memo", Some(Expression::Identifier(t))) => {
                memo.insert(name, t.name.to_string());
                let blank: String = slice(span)
                    .chars()
                    .map(|c| if c == '\n' { '\n' } else { ' ' })
                    .collect();
                edits.push(Edit {
                    start: span.start as usize,
                    end: span.end as usize,
                    text: blank,
                });
            }
            ("forwardRef", Some(arg)) => {
                let (params, ret_ty, body) = match arg {
                    Expression::ArrowFunctionExpression(a) => {
                        let body = match &a.body {
                            ArrowFunctionBody::FunctionBody(b) => slice(b.span).to_string(),
                            other => match other.as_expression() {
                                Some(e) => format!("{{ return {}; }}", slice(e.span())),
                                None => return Err("forwardRef: an empty arrow body".into()),
                            },
                        };
                        (&a.params, a.return_type.as_ref(), body)
                    }
                    Expression::FunctionExpression(f) => {
                        let Some(b) = &f.body else {
                            return Err("forwardRef: a function without a body".into());
                        };
                        (&f.params, f.return_type.as_ref(), slice(b.span).to_string())
                    }
                    _ => return Err("forwardRef takes a function".into()),
                };
                let plist: Vec<&str> = params.items.iter().map(|p| slice(p.span)).collect();
                if plist.len() != 2 {
                    return Err(format!(
                        "forwardRef({name}): the function takes (props, ref), not {} parameters",
                        plist.len()
                    ));
                }
                let ret = ret_ty.map_or("UiElement", |r| slice(r.type_annotation.span()));
                forward.insert(name.clone());
                edits.push(Edit {
                    start: span.start as usize,
                    end: span.end as usize,
                    text: format!(
                        "{}function {name}({}): {ret} {body}",
                        if exported { "export " } else { "" },
                        plist.join(", ")
                    ),
                });
            }
            _ => {}
        }
    }
    let mut out = String::with_capacity(src.len());
    let mut at = 0;
    for e in &edits {
        out.push_str(&src[at..e.start]);
        out.push_str(&e.text);
        at = e.end;
    }
    out.push_str(&src[at..]);
    Ok((out, memo, forward))
}
