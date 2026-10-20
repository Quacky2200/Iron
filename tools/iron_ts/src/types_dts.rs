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
    f.params
        .iter()
        .skip(usize::from(skip_this))
        .map(|(n, t, o)| format!("{}{}: {t}", safe_ident(n), if *o { "?" } else { "" }))
        .collect::<Vec<_>>()
        .join(", ")
}

fn safe_ident(n: &str) -> String {
    const RESERVED: &[&str] = &[
        "default", "function", "class", "new", "delete", "in", "of", "var", "let", "const",
    ];
    if RESERVED.contains(&n) {
        format!("{n}_")
    } else {
        n.to_string()
    }
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
            let members = self.classes.entry(name).or_default();
            for f in &fs {
                match f.kind.as_str() {
                    "static" => push_unique(
                        members,
                        format!("static {}({}): {};", f.name, params_text(f, false), f.ret),
                    ),
                    "method" => push_unique(
                        members,
                        format!("{}({}): {};", f.name, params_text(f, true), f.ret),
                    ),
                    "get" => {
                        let writable = fs.iter().any(|g| g.kind == "set" && g.name == f.name);
                        let line = if writable {
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
                        let has_getter = fs.iter().any(|g| g.kind == "get" && g.name == f.name);
                        if !has_getter {
                            let ty = f
                                .params
                                .get(1)
                                .map(|p| p.1.clone())
                                .unwrap_or_else(|| "unknown".into());
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
                    _ => push_unique(
                        members,
                        format!("function {}({}): {};", f.name, params_text(f, false), f.ret),
                    ),
                }
            }
        }
        Ok(())
    }

    /// `prefix` is `declare ` at the top level and `export ` inside the module block.
    fn text(&self, prefix: &str, indent: &str) -> String {
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
            if let Some(fields) = self.values.get(name) {
                let ctor: Vec<String> = fields.iter().map(|f| format!("{f}: number")).collect();
                s += &format!("{indent}  constructor({});\n", ctor.join(", "));
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
                s += &format!("{indent}  {m}\n");
            }
            s += &format!("{indent}}}\n");
        }
        for (name, members) in &self.namespaces {
            // A namespace named like a class would clash with its statics: keep only what the class lacks.
            s += &format!("{indent}{prefix}namespace {name} {{\n");
            for m in members {
                s += &format!("{indent}  {m}\n");
            }
            s += &format!("{indent}}}\n");
        }
        s
    }
}

/// The declarations for every manifest: `@iron/engine` as a module, then `ambient` (the project's
/// extra ambient declarations: `react`, `@iron/react`, `JSX`), verbatim.
pub fn generate(manifests: &[String], ambient: &str) -> Result<String, String> {
    let mut d = Decls::default();
    for m in manifests {
        d.add(m)?;
    }
    let mut out = String::from(
        "// Generated by `iron-build types` from the engine manifests. Do not edit.\n\
         // Declares the module \"@iron/engine\" so a game type-checks with no node_modules.\n\n\
         declare module \"@iron/engine\" {\n",
    );
    out += &d.text("export ", "  ");
    out += "}\n";
    if !ambient.trim().is_empty() {
        out += "\n";
        out += ambient;
        if !ambient.ends_with('\n') {
            out += "\n";
        }
    }
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
    fn manifests_become_declarations_inside_the_engine_module() {
        let t = generate(&[DEMO.to_string()], "declare const extra: number;").unwrap();
        assert!(t.contains("declare module \"@iron/engine\" {"), "{t}");
        assert!(t.contains("export type Thing = number &"), "{t}");
        assert!(
            t.contains("export class V2 {\n    constructor(x: number, y: number);"),
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
        assert!(t.ends_with("declare const extra: number;\n"), "{t}");
    }
}
