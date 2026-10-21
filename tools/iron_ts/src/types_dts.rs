//! `types/iron.d.ts` from the engine manifests: what `tsc` needs to type-check a game when there is no
//! `node_modules` (no `@iron/engine`, `react` or `@types/react` packages).
//!
//! The manifests (the JSON `iron_api` generates, `docs/iron-api.md`) list namespaces, handle and value
//! classes and their functions. This turns them into declarations, once as the module `@iron/engine`
//! (so the examples' `import { Color } from "@iron/engine"` type-checks as written). The doc comments
//! are not in the manifests, so the declarations have none. The ambient declarations (React-style
//! names, `JSX.IntrinsicElements`, `@iron/react`, `react`) are appended as they are.

use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

fn ts_type(t: &str) -> String {
    match t {
        "void" => "void".into(),
        "bool" => "boolean".into(),
        "f64" | "f32" | "i32" | "u32" => "number".into(),
        "string" => "string".into(),
        "f64s" => "number[]".into(),
        "ptr" => "unknown".into(),
        other => {
            if let Some(n) = other
                .strip_prefix("handle:")
                .or_else(|| other.strip_prefix("value:"))
            {
                n.to_string()
            } else if let Some(n) = other
                .strip_prefix("values:")
                .or_else(|| other.strip_prefix("handles:"))
            {
                format!("{n}[]")
            } else if let Some(inner) = other.strip_prefix("fn:") {
                format!("() => {}", ts_type(inner))
            } else if let Some(rest) = other.strip_prefix("fn(")
                && let Some((params, ret)) = rest.split_once("):")
            {
                let ps: Vec<String> = params
                    .split(',')
                    .filter(|p| !p.is_empty())
                    .enumerate()
                    .map(|(i, p)| format!("a{i}: {}", ts_type(p)))
                    .collect();
                format!("({}) => {}", ps.join(", "), ts_type(ret))
            } else {
                "unknown".into()
            }
        }
    }
}

struct Func {
    kind: String,
    name: String,
    params: Vec<(String, String, bool)>,
    ret: String,
}

fn funcs(v: &Value) -> Vec<Func> {
    let Some(arr) = v.as_array() else {
        return vec![];
    };
    arr.iter()
        .map(|f| Func {
            kind: f["kind"].as_str().unwrap_or("static").to_string(),
            name: f["name"].as_str().unwrap_or("").to_string(),
            params: f["params"]
                .as_array()
                .map(|ps| {
                    ps.iter()
                        .map(|p| {
                            (
                                p["name"].as_str().unwrap_or("p").to_string(),
                                ts_type(p["type"].as_str().unwrap_or("void")),
                                p["optional"].as_bool().unwrap_or(false),
                            )
                        })
                        .collect()
                })
                .unwrap_or_default(),
            ret: ts_type(f["returns"].as_str().unwrap_or("void")),
        })
        .collect()
}

fn params_text(f: &Func, skip_this: bool) -> String {
    // A required parameter cannot follow an optional one: the rest after the first become optional.
    let mut seen_optional = false;
    f.params
        .iter()
        .skip(usize::from(skip_this))
        .map(|(n, t, o)| {
            seen_optional |= *o;
            format!(
                "{}{}: {t}",
                safe_ident(n),
                if seen_optional { "?" } else { "" }
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// The "options object" form of a call: the compiler fills the parameters after the leading
/// required ones (numbers, strings, objects; flags may be left out) from a trailing `{ key: value }`
/// named like them (`audio.load(path, { stream: true })`). `None` when every parameter is leading.
fn options_params(f: &Func, skip_this: bool) -> Option<String> {
    let ps: Vec<_> = f.params.iter().skip(usize::from(skip_this)).collect();
    let k = ps
        .iter()
        .take_while(|(_, t, o)| !o && t != "boolean")
        .count();
    if k == ps.len() {
        return None;
    }
    let mut parts: Vec<String> = ps[..k]
        .iter()
        .map(|(n, t, _)| format!("{}: {t}", safe_ident(n)))
        .collect();
    let keys: Vec<String> = ps[k..]
        .iter()
        .map(|(n, t, _)| format!("{n}?: {t}"))
        .collect();
    parts.push(format!("options?: {{ {} }}", keys.join("; ")));
    Some(parts.join(", "))
}

const RESERVED: &[&str] = &[
    "default", "function", "class", "new", "delete", "in", "of", "var", "let", "const",
];

fn is_reserved(n: &str) -> bool {
    RESERVED.contains(&n)
}

fn safe_ident(n: &str) -> String {
    if is_reserved(n) {
        format!("{n}_")
    } else {
        n.to_string()
    }
}

/// What the hand-written supplement (`engine_ts.d.ts`) asks of the generator: the TypeScript-side
/// shapes the manifests lack. Its members replace the manifest's members of the same name (an
/// `interface N { .. }` block for instance members, a `declare namespace N { .. }` block for
/// statics), and `//@ctor N(params)` lines are the constructors of the class `N`.
#[derive(Default)]
struct Supplement {
    ctors: BTreeMap<String, Vec<String>>,
    hide_inst: BTreeMap<String, BTreeSet<String>>,
    hide_static: BTreeMap<String, BTreeSet<String>>,
    /// Top-level names it declares (the module `@iron/engine` re-exports them too).
    names: BTreeSet<String>,
}

fn ident_at(s: &str) -> &str {
    let end = s
        .find(|c: char| !(c.is_alphanumeric() || c == '_' || c == '$'))
        .unwrap_or(s.len());
    &s[..end]
}

/// The name a member line declares (`static foo(..)`, `readonly x: T`, `function f(..)`, `const c: T`).
fn member_name(line: &str) -> &str {
    let mut s = line.trim_start();
    loop {
        let mut stripped = false;
        for kw in [
            "static ",
            "readonly ",
            "function ",
            "const ",
            "let ",
            "var ",
            "get ",
            "set ",
            "export ",
            "declare ",
        ] {
            if let Some(r) = s.strip_prefix(kw) {
                s = r;
                stripped = true;
            }
        }
        if !stripped {
            break;
        }
    }
    ident_at(s)
}

fn parse_supplement(text: &str) -> Supplement {
    let mut sup = Supplement::default();
    // (is_namespace, name) of the block being read.
    let mut block: Option<(bool, String)> = None;
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("//@ctor ") {
            let name = ident_at(rest).to_string();
            let params = rest[name.len()..].trim();
            sup.ctors
                .entry(name)
                .or_default()
                .push(format!("constructor{params};"));
            continue;
        }
        if line.starts_with("//") || line.trim().is_empty() {
            continue;
        }
        if !line.starts_with(' ') && !line.starts_with('}') {
            let mut s = line;
            for kw in ["export ", "declare ", "abstract "] {
                s = s.strip_prefix(kw).unwrap_or(s);
            }
            s = s.strip_prefix("export ").unwrap_or(s);
            for (kw, ns) in [
                ("interface ", false),
                ("namespace ", true),
                ("class ", false),
                ("function ", false),
                ("const ", false),
                ("let ", false),
                ("var ", false),
                ("type ", false),
                ("enum ", false),
            ] {
                if let Some(r) = s.strip_prefix(kw) {
                    let name = ident_at(r).to_string();
                    sup.names.insert(name.clone());
                    block = if line.trim_end().ends_with('{') && (ns || kw == "interface ") {
                        Some((ns, name))
                    } else {
                        None
                    };
                    break;
                }
            }
            continue;
        }
        if line.starts_with('}') {
            block = None;
            continue;
        }
        if line.starts_with("  ") && !line.starts_with("   ") {
            let n = member_name(line);
            if n.is_empty() {
                continue;
            }
            match &block {
                Some((true, b)) => {
                    sup.hide_static
                        .entry(b.clone())
                        .or_default()
                        .insert(n.to_string());
                }
                Some((false, b)) => {
                    sup.hide_inst
                        .entry(b.clone())
                        .or_default()
                        .insert(n.to_string());
                }
                None => {}
            }
        }
    }
    sup
}

#[derive(Default)]
struct Decls {
    handles: BTreeSet<String>,
    /// Value class -> its field names.
    values: BTreeMap<String, Vec<String>>,
    /// Class name -> member lines (deduplicated, in order).
    classes: BTreeMap<String, Vec<String>>,
    /// Namespace -> member lines.
    namespaces: BTreeMap<String, Vec<String>>,
    /// Class -> the parameters of its static `create` (`new Class(..)` calls it).
    creates: BTreeMap<String, Vec<(String, String, bool)>>,
    /// Class -> its writable properties (the keys of `new Class({ .. })`).
    props: BTreeMap<String, Vec<(String, String)>>,
}

fn push_unique(list: &mut Vec<String>, line: String) {
    if !list.contains(&line) {
        list.push(line);
    }
}

impl Decls {
    fn add(&mut self, json: &str) -> Result<(), String> {
        let v: Value = serde_json::from_str(json).map_err(|e| format!("manifest json: {e}"))?;
        for h in v["handles"].as_array().into_iter().flatten() {
            if let Some(h) = h.as_str() {
                self.handles.insert(h.to_string());
            }
        }
        for val in v["values"].as_array().into_iter().flatten() {
            let name = val["name"].as_str().unwrap_or("").to_string();
            let fields = val["fields"]
                .as_array()
                .map(|fs| {
                    fs.iter()
                        .map(|f| f["name"].as_str().unwrap_or("").to_string())
                        .collect()
                })
                .unwrap_or_default();
            self.values.insert(name, fields);
        }
        for c in v["classes"].as_array().into_iter().flatten() {
            let name = c["name"].as_str().unwrap_or("").to_string();
            let fs = funcs(&c["functions"]);
            let members = self.classes.entry(name.clone()).or_default();
            let props = self.props.entry(name.clone()).or_default();
            for f in &fs {
                match f.kind.as_str() {
                    "static" => {
                        push_unique(
                            members,
                            format!("static {}({}): {};", f.name, params_text(f, false), f.ret),
                        );
                        if let Some(o) = options_params(f, false) {
                            push_unique(members, format!("static {}({o}): {};", f.name, f.ret));
                        }
                        if f.name == "create" {
                            self.creates.insert(name.clone(), f.params.clone());
                        }
                    }
                    "method" => {
                        push_unique(
                            members,
                            format!("{}({}): {};", f.name, params_text(f, true), f.ret),
                        );
                        if let Some(o) = options_params(f, true) {
                            push_unique(members, format!("{}({o}): {};", f.name, f.ret));
                        }
                    }
                    "get" if f.params.is_empty() => {
                        // A static property of the class (no receiver).
                        let writable = fs
                            .iter()
                            .any(|g| g.kind == "set" && g.name == f.name && g.params.len() == 1);
                        let line = if writable {
                            format!("static {}: {};", f.name, f.ret)
                        } else {
                            format!("static readonly {}: {};", f.name, f.ret)
                        };
                        push_unique(members, line);
                    }
                    "set" if f.params.len() == 1 => {
                        let has_getter = fs
                            .iter()
                            .any(|g| g.kind == "get" && g.name == f.name && g.params.is_empty());
                        if !has_getter {
                            push_unique(members, format!("static {}: {};", f.name, f.params[0].1));
                        }
                    }
                    "get" => {
                        let writable = fs
                            .iter()
                            .any(|g| g.kind == "set" && g.name == f.name && g.params.len() > 1);
                        let line = if writable {
                            props.push((f.name.clone(), f.ret.clone()));
                            format!("{}: {};", f.name, f.ret)
                        } else {
                            format!("readonly {}: {};", f.name, f.ret)
                        };
                        // A property declared once; the same name as a method elsewhere is skipped.
                        if !members.iter().any(|m| {
                            m.trim_start_matches("readonly ")
                                .starts_with(&format!("{}:", f.name))
                        }) {
                            members.push(line);
                        }
                    }
                    "set" => {
                        let has_getter = fs
                            .iter()
                            .any(|g| g.kind == "get" && g.name == f.name && !g.params.is_empty());
                        if !has_getter {
                            let ty = f
                                .params
                                .get(1)
                                .map(|p| p.1.clone())
                                .unwrap_or_else(|| "unknown".into());
                            props.push((f.name.clone(), ty.clone()));
                            push_unique(members, format!("{}: {ty};", f.name));
                        }
                    }
                    _ => {}
                }
            }
        }
        let ns_name = v["namespace"].as_str().unwrap_or("").to_string();
        let fs = funcs(&v["functions"]);
        if !fs.is_empty() {
            let members = self.namespaces.entry(ns_name).or_default();
            for f in &fs {
                // `function in(..)` is not a declaration TypeScript accepts (the class statics keep it).
                if is_reserved(&f.name) {
                    continue;
                }
                match f.kind.as_str() {
                    "get" => {
                        let writable = fs.iter().any(|g| g.kind == "set" && g.name == f.name);
                        let kw = if writable { "let" } else { "const" };
                        let line = format!("{kw} {}: {};", f.name, f.ret);
                        if !members.iter().any(|m| {
                            m.starts_with(&format!("let {}:", f.name))
                                || m.starts_with(&format!("const {}:", f.name))
                        }) {
                            members.push(line);
                        }
                    }
                    "set" => {}
                    _ => {
                        push_unique(
                            members,
                            format!("function {}({}): {};", f.name, params_text(f, false), f.ret),
                        );
                        if let Some(o) = options_params(f, false) {
                            push_unique(members, format!("function {}({o}): {};", f.name, f.ret));
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// `{ prop?: T; .. }` over the writable properties of a class: the keys of `new Class({ .. })`.
    fn options_type(&self, name: &str, sup: &Supplement) -> String {
        let hidden = sup.hide_inst.get(name);
        let props: Vec<String> = self
            .props
            .get(name)
            .into_iter()
            .flatten()
            .filter(|(n, _)| !hidden.is_some_and(|h| h.contains(n)))
            .map(|(n, t)| format!("{n}?: {t}"))
            .collect();
        format!("{{ {} }}", props.join("; "))
    }

    /// The constructor lines of a class: the supplement's, else the value class's fields, else
    /// the handle class's `create` (and the options form the compiler also accepts).
    fn ctor_lines(&self, name: &str, sup: &Supplement) -> Vec<String> {
        if let Some(c) = sup.ctors.get(name) {
            let opts = self.options_type(name, sup);
            return c.iter().map(|l| l.replace("$OPTIONS", &opts)).collect();
        }
        if let Some(fields) = self.values.get(name) {
            let ctor: Vec<String> = fields.iter().map(|f| format!("{f}: number")).collect();
            return vec![format!("constructor({});", ctor.join(", "))];
        }
        let Some(create) = self.creates.get(name) else {
            return vec![];
        };
        let opts = self.options_type(name, sup);
        let mut seen_optional = false;
        let pos: Vec<String> = create
            .iter()
            .map(|(n, t, o)| {
                seen_optional |= *o;
                format!(
                    "{}{}: {t}",
                    safe_ident(n),
                    if seen_optional { "?" } else { "" }
                )
            })
            .collect();
        let mut out = vec![format!("constructor({});", pos.join(", "))];
        let named = self.props.get(name).is_some_and(|p| !p.is_empty());
        if named {
            let mut with = pos.clone();
            let q = if seen_optional { "?" } else { "" };
            with.push(format!("options{q}: {opts}"));
            out.push(format!("constructor({});", with.join(", ")));
            if create.is_empty() && self.props[name].iter().any(|(n, _)| n == "name") {
                out.push(format!("constructor(name: string, options?: {opts});"));
            }
        }
        out
    }

    /// `prefix` is `declare ` at the top level and `export ` inside the module block.
    fn text(&self, prefix: &str, indent: &str, sup: &Supplement) -> String {
        let mut s = String::new();
        for h in &self.handles {
            if self.classes.contains_key(h) || self.values.contains_key(h) {
                continue;
            }
            s +=
                &format!("{indent}{prefix}type {h} = number & {{ readonly __handle: \"{h}\" }};\n");
        }
        let mut names: BTreeSet<&String> = self.classes.keys().collect();
        names.extend(self.values.keys());
        for name in names {
            s += &format!("{indent}{prefix}class {name} {{\n");
            for c in self.ctor_lines(name, sup) {
                s += &format!("{indent}  {c}\n");
            }
            if let Some(fields) = self.values.get(name) {
                for f in fields {
                    s += &format!("{indent}  {f}: number;\n");
                }
            }
            for m in self.classes.get(name).into_iter().flatten() {
                // A value field and a property of the same name: the field wins.
                if let Some(fields) = self.values.get(name)
                    && fields.iter().any(|f| {
                        m.trim_start_matches("readonly ")
                            .starts_with(&format!("{f}:"))
                    })
                {
                    continue;
                }
                // The supplement's member of the same name replaces it.
                let hidden = if m.starts_with("static ") {
                    sup.hide_static.get(name)
                } else {
                    sup.hide_inst.get(name)
                };
                if hidden.is_some_and(|h| h.contains(member_name(m))) {
                    continue;
                }
                s += &format!("{indent}  {m}\n");
            }
            // `Class.create({ .. })` sets the keys as properties, like `new Class({ .. })`.
            if self.creates.get(name).is_some_and(|c| c.is_empty())
                && self.props.get(name).is_some_and(|p| !p.is_empty())
                && !sup
                    .hide_static
                    .get(name)
                    .is_some_and(|h| h.contains("create"))
            {
                s += &format!(
                    "{indent}  static create(options: {}): {name};\n",
                    self.options_type(name, sup)
                );
            }
            // A component handle (it has a `gameObject`) also has what every component has.
            let members = self.classes.get(name);
            if members.is_some_and(|m| m.iter().any(|l| l == "readonly gameObject: GameObject;")) {
                let hidden = sup.hide_inst.get(name);
                let has = |n: &str| {
                    members.is_some_and(|m| m.iter().any(|l| member_name(l) == n))
                        || hidden.is_some_and(|h| h.contains(n))
                };
                if !has("transform") {
                    s += &format!("{indent}  readonly transform: Transform;\n");
                }
                if !has("getComponent") {
                    s += &format!(
                        "{indent}  getComponent<T>(cls: {{ prototype: T }}): T | undefined;\n"
                    );
                }
            }
            s += &format!("{indent}}}\n");
        }
        for (name, members) in &self.namespaces {
            s += &format!("{indent}{prefix}namespace {name} {{\n");
            for m in members {
                if sup
                    .hide_static
                    .get(name)
                    .is_some_and(|h| h.contains(member_name(m)))
                {
                    continue;
                }
                s += &format!("{indent}  {m}\n");
            }
            s += &format!("{indent}}}\n");
        }
        s
    }
}

/// Names the ambient declarations (`ambient.d.ts`) declare globally, per module they are also
/// re-exported from (`import { useState } from "react"` still type-checks).
const REACT_NAMES: &[&str] = &[
    "ReactNode",
    "useState",
    "useEffect",
    "useLayoutEffect",
    "useRef",
    "useMemo",
    "useCallback",
    "useReducer",
    "createContext",
    "useContext",
    "memo",
    "forwardRef",
];
const IRON_REACT_NAMES: &[&str] = &[
    "IronStyle",
    "IronElement",
    "IronEvent",
    "IronKeyboardEvent",
    "IronPointerEvent",
    "IronWheelEvent",
    "IronInputEvent",
    "IronFocusEvent",
    "IronScrollEvent",
    "createRoot",
    "Text",
    "Span",
    "Input",
    "TextArea",
    "Image",
    "List",
    "Dialog",
];
const ENGINE_AMBIENT_NAMES: &[&str] = &["FullScreenMode", "Gamepad", "Collision", "Collision2D"];

fn reexports(module: &str, names: &[String]) -> String {
    let mut s = format!("declare module \"{module}\" {{\n");
    for n in names {
        s += &format!("  export import {n} = globalThis.{n};\n");
    }
    s += "}\n";
    s
}

/// The ambient declarations (`ambient.d.ts`, generated by `scripts/gen_ambient.py`) and the
/// hand-written TypeScript-side engine shapes (`engine_ts.d.ts`) the compiler ships.
pub const AMBIENT_DTS: &str = include_str!("lower/ambient.d.ts");
pub const ENGINE_TS_DTS: &str = include_str!("lower/engine_ts.d.ts");

/// `generate` with the shipped ambient and supplement declarations.
pub fn generate_default(manifests: &[String]) -> Result<String, String> {
    generate(manifests, AMBIENT_DTS, ENGINE_TS_DTS)
}

/// The declarations for every manifest as globals (every class, namespace, value class and handle
/// type is a top-level `declare`), then `ambient` (the project's ambient declarations: React-style
/// names, `JSX`, hooks, UI components) verbatim, then the modules `@iron/engine`, `@iron/react` and
/// `react` as re-exports of those globals. A game needs no import; the old imports still work.
pub fn generate(manifests: &[String], ambient: &str, supplement: &str) -> Result<String, String> {
    let sup = parse_supplement(supplement);
    let mut d = Decls::default();
    for m in manifests {
        d.add(m)?;
    }
    let mut out = String::from(
        "// Generated by `iron-build types` from the engine manifests. Do not edit.\n\
         // Every engine and UI name is a global, so a game type-checks with no import and no node_modules;\n\
         // `import { Vector3 } from \"@iron/engine\"` (and `react`, `@iron/react`) work too.\n\n",
    );
    out += &d.text("declare ", "", &sup);
    if !ambient.trim().is_empty() {
        out += "\n";
        out += ambient;
        if !ambient.ends_with('\n') {
            out += "\n";
        }
    }
    if !supplement.trim().is_empty() {
        out += "\n";
        out += supplement;
        if !supplement.ends_with('\n') {
            out += "\n";
        }
    }
    let mut engine: BTreeSet<String> = d.handles.iter().cloned().collect();
    engine.extend(d.classes.keys().cloned());
    engine.extend(d.values.keys().cloned());
    engine.extend(d.namespaces.keys().filter(|n| !n.is_empty()).cloned());
    engine.extend(ENGINE_AMBIENT_NAMES.iter().map(|s| s.to_string()));
    engine.extend(sup.names.iter().cloned());
    let strs = |l: &[&str]| l.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    out += "\n";
    out += &reexports("@iron/engine", &engine.into_iter().collect::<Vec<_>>());
    out += "\n";
    out += &reexports("@iron/react", &strs(IRON_REACT_NAMES));
    out += "\n";
    out += &reexports("react", &strs(REACT_NAMES));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const DEMO: &str = r#"{"namespace":"Time","handles":["Thing"],"values":[{"name":"V2","fields":[{"name":"x","type":"f64"},{"name":"y","type":"f64"}]}],
      "classes":[{"name":"Obj","functions":[
        {"name":"make","symbol":"s1","kind":"static","params":[{"name":"n","type":"string"}],"returns":"handle:Obj"},
        {"name":"move","symbol":"s2","kind":"method","params":[{"name":"this","type":"handle:Obj"},{"name":"to","type":"value:V2","optional":true}],"returns":"void"},
        {"name":"hp","symbol":"s3","kind":"get","params":[{"name":"this","type":"handle:Obj"}],"returns":"f64"},
        {"name":"hp","symbol":"s4","kind":"set","params":[{"name":"this","type":"handle:Obj"},{"name":"v","type":"f64"}],"returns":"void"},
        {"name":"id","symbol":"s5","kind":"get","params":[{"name":"this","type":"handle:Obj"}],"returns":"u32"}]}],
      "functions":[{"name":"deltaTime","symbol":"t1","kind":"get","params":[],"returns":"f64"},
        {"name":"on","symbol":"t2","kind":"static","params":[{"name":"cb","type":"fn(f64,string):void"}],"returns":"u32"}]}"#;

    // M9-TS-1810
    #[test]
    fn manifests_become_global_declarations_that_the_engine_module_re_exports() {
        let t = generate(&[DEMO.to_string()], "declare const extra: number;", "").unwrap();
        assert!(t.contains("declare module \"@iron/engine\" {"), "{t}");
        assert!(t.contains("  export import V2 = globalThis.V2;"), "{t}");
        assert!(t.contains("\ndeclare type Thing = number &"), "{t}");
        assert!(
            t.contains("\ndeclare class V2 {\n  constructor(x: number, y: number);"),
            "{t}"
        );
        assert!(t.contains("static make(n: string): Obj;"), "{t}");
        assert!(t.contains("move(to?: V2): void;"), "{t}");
        assert!(
            t.contains("hp: number;") && t.contains("readonly id: number;"),
            "{t}"
        );
        assert!(t.contains("const deltaTime: number;"), "{t}");
        assert!(
            t.contains("function on(cb: (a0: number, a1: string) => void): number;"),
            "{t}"
        );
        assert!(t.contains("\ndeclare const extra: number;\n"), "{t}");
        assert!(t.contains("export import Obj = globalThis.Obj;"), "{t}");
        // The same input gives the same text.
        assert_eq!(
            t,
            generate(&[DEMO.to_string()], "declare const extra: number;", "").unwrap()
        );
    }

    const CREATABLE: &str = r#"{"namespace":"Audio","handles":[],"values":[],
      "classes":[{"name":"Thing","functions":[
        {"name":"create","symbol":"c1","kind":"static","params":[],"returns":"handle:Thing"},
        {"name":"name","symbol":"c2","kind":"get","params":[{"name":"this","type":"handle:Thing"}],"returns":"string"},
        {"name":"name","symbol":"c3","kind":"set","params":[{"name":"this","type":"handle:Thing"},{"name":"v","type":"string"}],"returns":"void"},
        {"name":"size","symbol":"c4","kind":"get","params":[{"name":"this","type":"handle:Thing"}],"returns":"f64"},
        {"name":"size","symbol":"c5","kind":"set","params":[{"name":"this","type":"handle:Thing"},{"name":"v","type":"f64"}],"returns":"void"},
        {"name":"gameObject","symbol":"c6","kind":"get","params":[{"name":"this","type":"handle:Thing"}],"returns":"handle:GameObject"}]}],
      "functions":[
        {"name":"load","symbol":"f1","kind":"static","params":[{"name":"path","type":"string"},{"name":"stream","type":"bool"}],"returns":"handle:Thing"},
        {"name":"mix","symbol":"f2","kind":"static","params":[{"name":"a","type":"f64","optional":true},{"name":"b","type":"f64"}],"returns":"void"},
        {"name":"in","symbol":"f3","kind":"static","params":[],"returns":"void"}]}"#;

    // M9-TS-2310
    #[test]
    fn a_call_gets_an_options_object_form_and_a_creatable_class_constructors() {
        let t = generate(&[CREATABLE.to_string()], "", "").unwrap();
        // Leading required parameters stay positional; the rest are keys.
        assert!(
            t.contains("function load(path: string, options?: { stream?: boolean }): Thing;"),
            "{t}"
        );
        // A required parameter after an optional one becomes optional.
        assert!(
            t.contains("function mix(a?: number, b?: number): void;"),
            "{t}"
        );
        assert!(
            t.contains("function mix(options?: { a?: number; b?: number }): void;"),
            "{t}"
        );
        // `function in(..)` is not valid TypeScript: left out.
        assert!(!t.contains("function in("), "{t}");
        assert!(t.contains("  constructor();\n"), "{t}");
        assert!(
            t.contains("  constructor(options: { name?: string; size?: number });"),
            "{t}"
        );
        assert!(
            t.contains("  constructor(name: string, options?: { name?: string; size?: number });"),
            "{t}"
        );
        assert!(
            t.contains("  static create(options: { name?: string; size?: number }): Thing;"),
            "{t}"
        );
        // A component handle (it has a `gameObject`) gets a transform and getComponent.
        assert!(t.contains("  readonly transform: Transform;"), "{t}");
        assert!(
            t.contains("getComponent<T>(cls: { prototype: T }): T | undefined;"),
            "{t}"
        );
    }

    // M9-TS-2311
    #[test]
    fn the_supplement_replaces_members_of_the_same_name_and_adds_constructors() {
        let sup = "//@ctor Thing(label?: string, options?: $OPTIONS)\n\
                   interface Thing {\n  size: string;\n}\n\
                   declare namespace Thing {\n  function create(x: number): Thing;\n}\n\
                   declare namespace Audio {\n  function load(path: string): Thing;\n}\n\
                   declare class Extra {}\n";
        let t = generate(&[CREATABLE.to_string()], "", sup).unwrap();
        // The native `size: number` is gone (the interface declares it), the native create too.
        assert!(!t.contains("  size: number;"), "{t}");
        assert!(!t.contains("static create(options"), "{t}");
        assert!(!t.contains("function load(path: string, stream"), "{t}");
        assert!(
            t.contains("  constructor(label?: string, options?: { name?: string });"),
            "{t}"
        );
        assert!(!t.contains("  constructor();\n"), "{t}");
        assert!(t.contains("interface Thing {\n  size: string;\n}"), "{t}");
        // Names the supplement declares are exported by the module too.
        assert!(
            t.contains("  export import Extra = globalThis.Extra;"),
            "{t}"
        );
    }
}
