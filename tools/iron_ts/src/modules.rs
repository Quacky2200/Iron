//! The module graph: several source files become one source text for the lowering.
//!
//! `bundle` starts at an entry file, follows relative imports (`./x`, `../y/z.ts`) and re-exports,
//! and writes the files, dependencies first, into one text:
//!
//! - imports of `@iron/*` and `react` are erased (their names come from the engine manifests and the
//!   ambient prelude, which is also what a program with no such import at all uses: the names are
//!   globals, see `types_dts`); an aliased name (`import { Color as C }`) is renamed to the real one;
//! - a bare import (`"foo"`, `"foo/sub"`) is a file of the game's `modules/` folder next to the entry
//!   (`src/modules/foo.ts`, `foo/index.ts`, `foo/sub.ts`), then handled like a relative import;
//! - imports of relative files are erased and their names renamed to the final names of the exporting
//!   file's declarations; `import type` is treated like `import` (types collide too);
//! - `export` is dropped from every file but the entry (the entry's exports stay the program's exports);
//! - top-level names that two files both declare are renamed (`name_2`, ...) in the later file;
//! - `import "./x.css"` is not read: it is listed in `Bundle::css` for the build tool.
//!
//! The text is edited, not re-printed: edits replace byte ranges, so everything else (comments, JSX)
//! stays as written. Newlines of erased statements are kept so a file's lines stay recognisable.

use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_ast_visit::{Visit, walk};
use oxc_parser::Parser;
use oxc_span::{GetSpan, SourceType};
use std::collections::{HashMap, HashSet};

/// The result of bundling.
#[derive(Debug, Default, Clone)]
pub struct Bundle {
    /// One TypeScript (TSX when any file is) source text.
    pub source: String,
    /// The files in output order (dependencies first, the entry last), normalised with `/`.
    pub files: Vec<String>,
    /// Stylesheets named by `import "./x.css"`, in import order, without duplicates.
    pub css: Vec<String>,
    /// `@iron/*` / `react` names the program imports (module, name), sorted and distinct.
    pub library_imports: Vec<(String, String)>,
    /// For each file: the 0-based line of `source` where its text starts, and its path.
    pub starts: Vec<(usize, String)>,
}

impl Bundle {
    /// The file and line (1-based) that line `line` (1-based) of `source` came from.
    pub fn locate(&self, line: usize) -> Option<(&str, usize)> {
        let at = line.checked_sub(1)?;
        let (start, file) = self.starts.iter().rev().find(|(s, _)| *s <= at)?;
        Some((file, at - start + 1))
    }

    /// `message` with every `line N` (a line of the bundled source) rewritten as `file:line`.
    pub fn locate_in(&self, message: &str) -> String {
        let mut out = String::new();
        let mut rest = message;
        while let Some(i) = rest.find("line ") {
            out.push_str(&rest[..i]);
            let after = &rest[i + 5..];
            let digits: String = after.chars().take_while(char::is_ascii_digit).collect();
            match digits.parse::<usize>().ok().and_then(|n| self.locate(n)) {
                Some((file, l)) => out.push_str(&format!("{file}:{l}")),
                None => out.push_str(&format!("line {digits}")),
            }
            rest = &after[digits.len()..];
        }
        out.push_str(rest);
        out
    }
}

/// Reads a file by its normalised path; `None` when it does not exist.
pub type ReadFn<'a> = &'a dyn Fn(&str) -> Option<String>;

/// Bundles `entry` and what it imports, reading files from disk.
pub fn bundle(entry: &std::path::Path) -> Result<Bundle, String> {
    let entry = entry.to_string_lossy().replace('\\', "/");
    bundle_with(&entry, &|p| std::fs::read_to_string(p).ok())
}

/// Bundles `entry` reading through `read` (a map of paths in tests).
pub fn bundle_with(entry: &str, read: ReadFn) -> Result<Bundle, String> {
    let entry = normalise(entry);
    let mut graph = Graph {
        modules_root: modules_root(&entry),
        ..Graph::default()
    };
    graph.load(&entry, read, true)?;
    graph.emit()
}

/// `a/./b/../c` -> `a/c`.
fn normalise(path: &str) -> String {
    let mut out: Vec<&str> = Vec::new();
    let unified = path.replace('\\', "/");
    for part in unified.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                if out.last().is_some_and(|l| *l != "..") {
                    out.pop();
                } else {
                    out.push("..");
                }
            }
            p => out.push(p),
        }
    }
    let joined = out.join("/");
    // Keep a leading slash (absolute path) or a drive-less relative path as it was.
    if path.starts_with('/') {
        format!("/{joined}")
    } else {
        joined
    }
}

/// The folder bare imports are looked up in: `modules/` next to the entry file.
fn modules_root(entry: &str) -> String {
    match entry.rsplit_once('/') {
        Some((dir, _)) if !dir.is_empty() => format!("{dir}/modules"),
        _ => "modules".to_string(),
    }
}

/// The files a path `base` (no extension needed) can mean, in lookup order.
fn candidates_of(base: &str) -> Vec<String> {
    let stem = base
        .strip_suffix(".js")
        .or_else(|| base.strip_suffix(".jsx"))
        .unwrap_or(base)
        .to_string();
    let mut candidates = vec![base.to_string()];
    for ext in ["ts", "tsx"] {
        candidates.push(format!("{stem}.{ext}"));
        candidates.push(format!("{base}.{ext}"));
    }
    for ext in ["ts", "tsx"] {
        candidates.push(format!("{base}/index.{ext}"));
    }
    candidates
}

fn is_library(spec: &str) -> bool {
    spec.starts_with("@iron/") || spec == "react" || spec.starts_with("react/")
}

fn is_relative(spec: &str) -> bool {
    spec.starts_with("./") || spec.starts_with("../") || spec == "." || spec == ".."
}

#[derive(Debug, Clone)]
enum ImportKind {
    Named { imported: String, local: String },
    Default(String),
    Namespace(String),
}

#[derive(Debug, Clone)]
struct Import {
    spec: String,
    kinds: Vec<ImportKind>,
}

#[derive(Debug, Clone)]
enum ExportSrc {
    /// `export { local as exported }` / `export function local`.
    Local(String),
    /// `export { imported as exported } from "spec"`.
    From(String, String),
    /// `export * from "spec"`.
    Star(String),
}

#[derive(Debug, Clone)]
struct Export {
    exported: String,
    src: ExportSrc,
}

/// One identifier use or declaration (not a property name).
#[derive(Debug, Clone)]
struct Occ {
    start: u32,
    end: u32,
    name: String,
    /// `{ x }` in an object or a pattern: renaming writes `x: new`.
    shorthand: bool,
}

#[derive(Debug, Clone)]
struct Edit {
    start: u32,
    end: u32,
    text: String,
}

#[derive(Default)]
struct Module {
    path: String,
    src: String,
    imports: Vec<Import>,
    exports: Vec<Export>,
    top: Vec<String>,
    occs: Vec<Occ>,
    /// Edits that do not depend on renaming: erased statements, dropped `export`.
    edits: Vec<Edit>,
    /// Statements whose text is appended after the module (the entry's re-exports are rewritten).
    export_from_entry: Vec<(String, String)>,
    tsx: bool,
    /// Name given to an anonymous default expression.
    default_name: Option<String>,
}

#[derive(Default)]
struct Graph {
    mods: Vec<Module>,
    index: HashMap<String, usize>,
    /// Output order (indices into `mods`).
    order: Vec<usize>,
    css: Vec<String>,
    libs: Vec<(String, String)>,
    /// Where bare imports are looked up.
    modules_root: String,
}

/// What a name resolves to across files.
#[derive(Debug, Clone)]
enum Res {
    Local(usize, String),
    Lib(String),
}

impl Graph {
    fn resolve_path(&self, from: &str, spec: &str, read: ReadFn) -> Result<String, String> {
        let bare = !is_relative(spec);
        let base = if bare {
            normalise(&format!("{}/{spec}", self.modules_root))
        } else {
            let dir = from.rsplit_once('/').map(|(d, _)| d).unwrap_or("");
            if dir.is_empty() {
                normalise(spec)
            } else {
                normalise(&format!("{dir}/{spec}"))
            }
        };
        let candidates = candidates_of(&base);
        for c in &candidates {
            if read(c).is_some() {
                return Ok(c.clone());
            }
        }
        if bare {
            let mut tried: Vec<&str> = Vec::new();
            for c in candidates.iter().skip(1) {
                if !tried.contains(&c.as_str()) {
                    tried.push(c);
                }
            }
            return Err(format!(
                "{from}: cannot find module \"{spec}\": bare imports are looked up in {root}/ (tried {}).                  To use a TypeScript package, copy its .ts source into {root}/<name>/.",
                tried.join(", "),
                root = self.modules_root
            ));
        }
        Err(format!(
            "{from}: cannot find module \"{spec}\" (tried {})",
            candidates.join(", ")
        ))
    }

    /// Parses `path` and its relative imports (depth first: dependencies are pushed to `order` first).
    fn load(&mut self, path: &str, read: ReadFn, entry: bool) -> Result<usize, String> {
        if let Some(&i) = self.index.get(path) {
            return Ok(i);
        }
        let src = read(path).ok_or_else(|| format!("cannot read {path}"))?;
        let mut m = parse_module(path, src, entry)?;
        let idx = self.mods.len();
        self.index.insert(path.to_string(), idx);
        let imports = m.imports.clone();
        let reexports: Vec<String> = m
            .exports
            .iter()
            .filter_map(|e| match &e.src {
                ExportSrc::From(s, _) | ExportSrc::Star(s) => Some(s.clone()),
                _ => None,
            })
            .collect();
        self.mods.push(Module::default());
        // Targets keyed by spec for emit-time resolution.
        for imp in &imports {
            if is_library(&imp.spec) {
                continue;
            }
            if is_relative(&imp.spec) && imp.spec.ends_with(".css") {
                let p = normalise(&format!(
                    "{}/{}",
                    path.rsplit_once('/').map(|x| x.0).unwrap_or(""),
                    imp.spec
                ));
                if !self.css.contains(&p) {
                    self.css.push(p);
                }
                continue;
            }
            let target = self.resolve_path(path, &imp.spec, read)?;
            self.load(&target, read, false)?;
        }
        for spec in reexports {
            let target = self.resolve_path(path, &spec, read)?;
            self.load(&target, read, false)?;
        }
        for imp in &imports {
            if is_library(&imp.spec) {
                for k in &imp.kinds {
                    if let ImportKind::Named { imported, .. } = k {
                        self.libs.push((imp.spec.clone(), imported.clone()));
                    }
                }
            }
        }
        m.path = path.to_string();
        self.mods[idx] = m;
        self.order.push(idx);
        Ok(idx)
    }

    fn target(&self, from: usize, spec: &str) -> Result<usize, String> {
        let p = &self.mods[from].path;
        let base = if is_relative(spec) {
            let dir = p.rsplit_once('/').map(|(d, _)| d).unwrap_or("");
            if dir.is_empty() {
                normalise(spec)
            } else {
                normalise(&format!("{dir}/{spec}"))
            }
        } else {
            normalise(&format!("{}/{spec}", self.modules_root))
        };
        for c in candidates_of(&base) {
            if let Some(&i) = self.index.get(&c) {
                return Ok(i);
            }
        }
        Err(format!("{p}: module \"{spec}\" was not loaded"))
    }

    /// The declaration that `import { name } from m` reaches.
    fn resolve_export(&self, m: usize, name: &str, depth: usize) -> Result<Res, String> {
        if depth > 32 {
            return Err(format!(
                "{}: re-export cycle on `{name}`",
                self.mods[m].path
            ));
        }
        let module = &self.mods[m];
        for e in &module.exports {
            if e.exported == name {
                return match &e.src {
                    ExportSrc::Local(l) => self.resolve_local(m, l, depth + 1),
                    ExportSrc::From(spec, imp) => {
                        self.resolve_export(self.target(m, spec)?, imp, depth + 1)
                    }
                    ExportSrc::Star(_) => unreachable!(),
                };
            }
        }
        for e in &module.exports {
            if let ExportSrc::Star(spec) = &e.src
                && let Ok(t) = self.target(m, spec)
                && let Ok(r) = self.resolve_export(t, name, depth + 1)
            {
                return Ok(r);
            }
        }
        Err(format!("{}: has no export `{name}`", module.path))
    }

    /// A name used inside module `m`: its declaration, following an import to the other file.
    fn resolve_local(&self, m: usize, local: &str, depth: usize) -> Result<Res, String> {
        if depth > 32 {
            return Err(format!("{}: import cycle on `{local}`", self.mods[m].path));
        }
        for imp in &self.mods[m].imports {
            for k in &imp.kinds {
                match k {
                    ImportKind::Named { imported, local: l } if l == local => {
                        return if is_library(&imp.spec) {
                            Ok(Res::Lib(imported.clone()))
                        } else {
                            self.resolve_export(self.target(m, &imp.spec)?, imported, depth + 1)
                        };
                    }
                    ImportKind::Default(l) if l == local => {
                        return if is_library(&imp.spec) {
                            Ok(Res::Lib(l.clone()))
                        } else {
                            self.resolve_export(self.target(m, &imp.spec)?, "default", depth + 1)
                        };
                    }
                    ImportKind::Namespace(l) if l == local => {
                        if is_library(&imp.spec) {
                            return Ok(Res::Lib(l.clone()));
                        }
                        return Err(format!(
                            "{}: `import * as {l}` of a project file is not supported; import the names",
                            self.mods[m].path
                        ));
                    }
                    _ => {}
                }
            }
        }
        Ok(Res::Local(m, local.to_string()))
    }

    fn all_export_names(&self, m: usize, seen: &mut HashSet<usize>) -> Vec<String> {
        if !seen.insert(m) {
            return vec![];
        }
        let mut out = vec![];
        for e in &self.mods[m].exports {
            match &e.src {
                ExportSrc::Star(spec) => {
                    if let Ok(t) = self.target(m, spec) {
                        out.extend(self.all_export_names(t, seen));
                    }
                }
                _ => out.push(e.exported.clone()),
            }
        }
        out
    }

    fn emit(&self) -> Result<Bundle, String> {
        let entry = *self.order.last().ok_or("empty program")?;
        // Final names of the top-level declarations.
        let mut used: HashSet<String> = self.mods[entry].top.iter().cloned().collect();
        for (_, n) in &self.libs {
            used.insert(n.clone());
        }
        let mut renames: Vec<HashMap<String, String>> = vec![HashMap::new(); self.mods.len()];
        for &mi in &self.order {
            if mi == entry {
                continue;
            }
            for name in &self.mods[mi].top {
                if used.contains(name) {
                    let mut n = 2;
                    let fresh = loop {
                        let c = format!("{name}_{n}");
                        if !used.contains(&c) && !self.all_names_contain(&c) {
                            break c;
                        }
                        n += 1;
                    };
                    used.insert(fresh.clone());
                    renames[mi].insert(name.clone(), fresh);
                } else {
                    used.insert(name.clone());
                }
            }
        }
        let final_of = |r: &Res, renames: &Vec<HashMap<String, String>>| match r {
            Res::Local(m, n) => renames[*m].get(n).cloned().unwrap_or_else(|| n.clone()),
            Res::Lib(n) => n.clone(),
        };
        let mut out = String::new();
        let mut starts: Vec<(usize, String)> = Vec::new();
        let tsx = self.mods.iter().any(|m| m.tsx);
        let mut css = self.css.clone();
        css.dedup();
        for &mi in &self.order {
            let m = &self.mods[mi];
            // Local name -> final name, for every identifier of this file that must change.
            let mut map: HashMap<String, String> = renames[mi].clone();
            for imp in &m.imports {
                for k in &imp.kinds {
                    let local = match k {
                        ImportKind::Named { local, .. }
                        | ImportKind::Default(local)
                        | ImportKind::Namespace(local) => local,
                    };
                    let r = self.resolve_local(mi, local, 0)?;
                    let f = final_of(&r, &renames);
                    if &f != local {
                        map.insert(local.clone(), f);
                    }
                }
            }
            let mut edits = m.edits.clone();
            for o in &m.occs {
                if let Some(new) = map.get(&o.name) {
                    let text = if o.shorthand {
                        format!("{}: {new}", o.name)
                    } else {
                        new.clone()
                    };
                    edits.push(Edit {
                        start: o.start,
                        end: o.end,
                        text,
                    });
                }
            }
            edits.sort_by_key(|e| (e.start, e.end));
            let mut text = String::new();
            let mut at = 0usize;
            for e in &edits {
                let (s, en) = (e.start as usize, e.end as usize);
                if s < at {
                    return Err(format!(
                        "{}: overlapping edits at byte {s} (internal)",
                        m.path
                    ));
                }
                text.push_str(&m.src[at..s]);
                text.push_str(&e.text);
                at = en;
            }
            text.push_str(&m.src[at..]);
            // The entry re-exports other files' names: `export { final as name };`.
            for (spec, imported) in &m.export_from_entry {
                let t = self.target(mi, spec)?;
                let names: Vec<String> = if imported == "*" {
                    self.all_export_names(t, &mut HashSet::new())
                } else {
                    vec![imported.clone()]
                };
                for n in names {
                    let r = self.resolve_export(t, &n, 0)?;
                    text.push_str(&format!(
                        "\nexport {{ {} as {n} }};",
                        final_of(&r, &renames)
                    ));
                }
            }
            out.push_str(&format!("// ---- {}\n", m.path));
            starts.push((out.matches('\n').count(), m.path.clone()));
            out.push_str(&text);
            if !text.ends_with('\n') {
                out.push('\n');
            }
        }
        let mut libs = self.libs.clone();
        libs.sort();
        libs.dedup();
        let _ = tsx;
        Ok(Bundle {
            source: out,
            files: self
                .order
                .iter()
                .map(|&i| self.mods[i].path.clone())
                .collect(),
            css,
            library_imports: libs,
            starts,
        })
    }

    fn all_names_contain(&self, name: &str) -> bool {
        self.mods.iter().any(|m| m.top.iter().any(|t| t == name))
    }
}

fn parse_module(path: &str, src: String, entry: bool) -> Result<Module, String> {
    let alloc = Allocator::default();
    let tsx = path.ends_with(".tsx") || path.ends_with(".jsx");
    let st = if tsx {
        SourceType::tsx()
    } else {
        SourceType::ts()
    };
    let ret = Parser::new(&alloc, &src, st).parse();
    if let Some(d) = ret.diagnostics.first() {
        let line = d.labels.first().map(|l| {
            src[..(l.offset() as usize).min(src.len())]
                .matches('\n')
                .count()
                + 1
        });
        return Err(match line {
            Some(l) => format!("{path}: parse error (line {l}): {d}"),
            None => format!("{path}: parse error: {d}"),
        });
    }
    let mut m = Module {
        path: path.to_string(),
        tsx,
        ..Module::default()
    };
    let erase = |s: u32, e: u32, src: &str| Edit {
        start: s,
        end: e,
        text: "\n".repeat(src[s as usize..e as usize].matches('\n').count()),
    };
    for stmt in &ret.program.body {
        match stmt {
            Statement::ImportDeclaration(d) => {
                let mut kinds = Vec::new();
                if let Some(specs) = &d.specifiers {
                    for s in specs {
                        match s {
                            ImportDeclarationSpecifier::ImportSpecifier(s) => {
                                kinds.push(ImportKind::Named {
                                    imported: s.imported.name().to_string(),
                                    local: s.local.name.to_string(),
                                })
                            }
                            ImportDeclarationSpecifier::ImportDefaultSpecifier(s) => {
                                kinds.push(ImportKind::Default(s.local.name.to_string()))
                            }
                            ImportDeclarationSpecifier::ImportNamespaceSpecifier(s) => {
                                kinds.push(ImportKind::Namespace(s.local.name.to_string()))
                            }
                        }
                    }
                }
                m.imports.push(Import {
                    spec: d.source.value.to_string(),
                    kinds,
                });
                m.edits.push(erase(d.span.start, d.span.end, &src));
            }
            Statement::ExportDeclaration(d) => {
                for n in declared_names(&d.declaration) {
                    m.exports.push(Export {
                        exported: n.clone(),
                        src: ExportSrc::Local(n),
                    });
                }
                if !entry {
                    m.edits.push(Edit {
                        start: d.span.start,
                        end: d.declaration.span().start,
                        text: String::new(),
                    });
                }
            }
            Statement::ExportNamedDeclaration(d) => {
                for s in &d.specifiers {
                    m.exports.push(Export {
                        exported: s.exported.name().to_string(),
                        src: ExportSrc::Local(s.local.name().to_string()),
                    });
                }
                // The entry's `export { a as b }` stays (its locals are renamed with the rest).
                if !entry {
                    m.edits.push(erase(d.span.start, d.span.end, &src));
                }
            }
            Statement::ExportFromDeclaration(d) => {
                let spec = d.source.value.to_string();
                for s in &d.specifiers {
                    let local = s.local.name().to_string();
                    if entry {
                        m.export_from_entry
                            .push((spec.clone(), s.exported.name().to_string()));
                        // The entry exports the name under its own: `export { final as exported }`.
                    }
                    m.exports.push(Export {
                        exported: s.exported.name().to_string(),
                        src: ExportSrc::From(spec.clone(), local),
                    });
                }
                m.edits.push(erase(d.span.start, d.span.end, &src));
            }
            Statement::ExportAllDeclaration(d) => {
                let spec = d.source.value.to_string();
                if d.exported.is_some() {
                    return Err(format!("{path}: `export * as ns` is not supported"));
                }
                if entry {
                    m.export_from_entry.push((spec.clone(), "*".into()));
                }
                m.exports.push(Export {
                    exported: "*".into(),
                    src: ExportSrc::Star(spec),
                });
                m.edits.push(erase(d.span.start, d.span.end, &src));
            }
            Statement::ExportDefaultDeclaration(d) => {
                let default_name = format!(
                    "__default_{}",
                    path.replace(|c: char| !c.is_alphanumeric(), "_")
                );
                match &d.declaration {
                    ExportDefaultDeclarationKind::FunctionDeclaration(f) => {
                        let Some(id) = &f.id else {
                            return Err(format!(
                                "{path}: an anonymous `export default function` is not supported; name it"
                            ));
                        };
                        m.exports.push(Export {
                            exported: "default".into(),
                            src: ExportSrc::Local(id.name.to_string()),
                        });
                        if !entry {
                            m.edits.push(Edit {
                                start: d.span.start,
                                end: f.span.start,
                                text: String::new(),
                            });
                        }
                    }
                    ExportDefaultDeclarationKind::ClassDeclaration(c) => {
                        let Some(id) = &c.id else {
                            return Err(format!(
                                "{path}: an anonymous `export default class` is not supported; name it"
                            ));
                        };
                        m.exports.push(Export {
                            exported: "default".into(),
                            src: ExportSrc::Local(id.name.to_string()),
                        });
                        if !entry {
                            m.edits.push(Edit {
                                start: d.span.start,
                                end: c.span.start,
                                text: String::new(),
                            });
                        }
                    }
                    ExportDefaultDeclarationKind::TSInterfaceDeclaration(i) => {
                        m.exports.push(Export {
                            exported: "default".into(),
                            src: ExportSrc::Local(i.id.name.to_string()),
                        });
                        if !entry {
                            m.edits.push(Edit {
                                start: d.span.start,
                                end: i.span.start,
                                text: String::new(),
                            });
                        }
                    }
                    other => {
                        if !entry {
                            // An expression: `export default expr;` -> `const __default_x = expr;`.
                            m.top.push(default_name.clone());
                            m.exports.push(Export {
                                exported: "default".into(),
                                src: ExportSrc::Local(default_name.clone()),
                            });
                            m.edits.push(Edit {
                                start: d.span.start,
                                end: other.span().start,
                                text: format!("const {default_name} = "),
                            });
                            m.default_name = Some(default_name);
                        }
                    }
                }
            }
            _ => {}
        }
    }
    // Top-level declared names.
    for stmt in &ret.program.body {
        let decl: Option<&Declaration> = match stmt {
            Statement::ExportDeclaration(d) => Some(&d.declaration),
            Statement::VariableDeclaration(d) => {
                m.top.extend(var_names(d));
                None
            }
            Statement::FunctionDeclaration(f) => {
                m.top.extend(f.id.as_ref().map(|i| i.name.to_string()));
                None
            }
            Statement::ClassDeclaration(c) => {
                m.top.extend(c.id.as_ref().map(|i| i.name.to_string()));
                None
            }
            Statement::TSTypeAliasDeclaration(t) => {
                m.top.push(t.id.name.to_string());
                None
            }
            Statement::TSInterfaceDeclaration(t) => {
                m.top.push(t.id.name.to_string());
                None
            }
            Statement::TSEnumDeclaration(t) => {
                m.top.push(t.id.name.to_string());
                None
            }
            Statement::ExportDefaultDeclaration(d) => {
                match &d.declaration {
                    ExportDefaultDeclarationKind::FunctionDeclaration(f) => {
                        m.top.extend(f.id.as_ref().map(|i| i.name.to_string()))
                    }
                    ExportDefaultDeclarationKind::ClassDeclaration(c) => {
                        m.top.extend(c.id.as_ref().map(|i| i.name.to_string()))
                    }
                    ExportDefaultDeclarationKind::TSInterfaceDeclaration(i) => {
                        m.top.push(i.id.name.to_string())
                    }
                    _ => {}
                }
                None
            }
            _ => None,
        };
        if let Some(d) = decl {
            m.top.extend(declared_names(d));
        }
    }
    // Identifier uses and declarations.
    let mut v = Occs { occs: Vec::new() };
    for stmt in &ret.program.body {
        match stmt {
            Statement::ImportDeclaration(_) => {}
            Statement::ExportDeclaration(d) => v.visit_declaration(&d.declaration),
            Statement::ExportNamedDeclaration(d) => {
                if entry {
                    for s in &d.specifiers {
                        if let ModuleExportName::IdentifierReference(r) = &s.local {
                            v.occs.push(Occ {
                                start: r.span.start,
                                end: r.span.end,
                                name: r.name.to_string(),
                                shorthand: false,
                            });
                        }
                    }
                }
            }
            Statement::ExportFromDeclaration(_) => {}
            Statement::ExportAllDeclaration(_) => {}
            other => v.visit_statement(other),
        }
    }
    m.occs = v.occs;
    m.src = src;
    Ok(m)
}

fn var_names(d: &VariableDeclaration) -> Vec<String> {
    let mut out = Vec::new();
    for decl in &d.declarations {
        for id in decl.id.get_binding_identifiers() {
            out.push(id.name.to_string());
        }
    }
    out
}

fn declared_names(d: &Declaration) -> Vec<String> {
    match d {
        Declaration::VariableDeclaration(v) => var_names(v),
        Declaration::FunctionDeclaration(f) => f.id.iter().map(|i| i.name.to_string()).collect(),
        Declaration::ClassDeclaration(c) => c.id.iter().map(|i| i.name.to_string()).collect(),
        Declaration::TSTypeAliasDeclaration(t) => vec![t.id.name.to_string()],
        Declaration::TSInterfaceDeclaration(t) => vec![t.id.name.to_string()],
        Declaration::TSEnumDeclaration(t) => vec![t.id.name.to_string()],
        _ => vec![],
    }
}

struct Occs {
    occs: Vec<Occ>,
}

impl<'a> Visit<'a> for Occs {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        self.occs.push(Occ {
            start: it.span.start,
            end: it.span.end,
            name: it.name.to_string(),
            shorthand: false,
        });
    }

    fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        self.occs.push(Occ {
            start: it.span.start,
            end: it.span.end,
            name: it.name.to_string(),
            shorthand: false,
        });
    }

    fn visit_object_property(&mut self, it: &ObjectProperty<'a>) {
        if it.shorthand {
            if let Expression::Identifier(id) = &it.value {
                self.occs.push(Occ {
                    start: id.span.start,
                    end: id.span.end,
                    name: id.name.to_string(),
                    shorthand: true,
                });
            }
            return;
        }
        walk::walk_object_property(self, it);
    }

    fn visit_binding_property(&mut self, it: &BindingProperty<'a>) {
        if it.shorthand {
            // `{ x }` or `{ x = 1 }`: the binding is the identifier (left of a default).
            let id = match &it.value {
                BindingPattern::BindingIdentifier(id) => Some(id),
                BindingPattern::AssignmentPattern(a) => match &a.left {
                    BindingPattern::BindingIdentifier(id) => Some(id),
                    _ => None,
                },
                _ => None,
            };
            if let Some(id) = id {
                self.occs.push(Occ {
                    start: id.span.start,
                    end: id.span.end,
                    name: id.name.to_string(),
                    shorthand: true,
                });
                if let BindingPattern::AssignmentPattern(a) = &it.value {
                    self.visit_expression(&a.right);
                }
                return;
            }
        }
        walk::walk_binding_property(self, it);
    }

    fn visit_assignment_target_property_identifier(
        &mut self,
        it: &AssignmentTargetPropertyIdentifier<'a>,
    ) {
        let id = &it.binding;
        self.occs.push(Occ {
            start: id.span.start,
            end: id.span.end,
            name: id.name.to_string(),
            shorthand: true,
        });
        if let Some(init) = &it.init {
            self.visit_expression(init);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(entry: &str, files: &[(&str, &str)]) -> Result<Bundle, String> {
        let map: HashMap<String, String> = files
            .iter()
            .map(|(p, s)| (p.to_string(), s.to_string()))
            .collect();
        bundle_with(entry, &|p| map.get(p).cloned())
    }

    // M9-TS-1800
    #[test]
    fn a_relative_import_pulls_the_file_in_before_the_importer() {
        let b = run(
            "src/main.ts",
            &[
                (
                    "src/main.ts",
                    "import { f } from \"./a.ts\";\nexport function g(): number { return f(); }",
                ),
                ("src/a.ts", "export function f(): number { return 1; }"),
            ],
        )
        .unwrap();
        assert_eq!(b.files, ["src/a.ts", "src/main.ts"]);
        assert!(b.source.find("function f").unwrap() < b.source.find("function g").unwrap());
        assert!(!b.source.contains("import"), "{}", b.source);
        assert!(b.source.contains("export function g"));
        assert!(!b.source.contains("export function f"));
        // Lines of the bundle map back to their files.
        let line_of = |needle: &str| {
            b.source[..b.source.find(needle).unwrap()]
                .matches('\n')
                .count()
                + 1
        };
        assert_eq!(b.locate(line_of("function f")), Some(("src/a.ts", 1)));
        assert_eq!(b.locate(line_of("function g")), Some(("src/main.ts", 2)));
        assert_eq!(b.locate_in("x line 2: bad"), "x src/a.ts:1: bad");
    }

    // M9-TS-1801
    #[test]
    fn library_imports_are_erased_and_aliases_renamed() {
        let b = run(
            "main.ts",
            &[(
                "main.ts",
                "import { Color as C, type Sound } from \"@iron/engine\";\nimport { useState } from \"react\";\nexport const c = C.white;",
            )],
        )
        .unwrap();
        assert!(!b.source.contains("import"), "{}", b.source);
        assert!(b.source.contains("Color.white"), "{}", b.source);
        assert!(
            b.library_imports
                .contains(&("@iron/engine".into(), "Color".into()))
        );
    }

    // M9-TS-2312
    #[test]
    fn a_program_with_no_library_imports_bundles_the_same_way() {
        let b = run(
            "main.ts",
            &[
                ("main.ts", "import { twice } from \"./util\";\nexport const c = Color.white;\nexport const n = twice(2);"),
                ("util.ts", "export function twice(n: number): number { return n * 2; }"),
            ],
        )
        .unwrap();
        assert!(b.library_imports.is_empty());
        assert!(b.source.contains("Color.white"), "{}", b.source);
        assert!(b.source.contains("function twice"), "{}", b.source);
    }

    // M9-TS-1802
    #[test]
    fn colliding_top_level_names_are_renamed_in_the_dependency() {
        let b = run(
            "main.ts",
            &[
                ("main.ts", "import { v } from \"./a\";\nconst helper = 1;\nexport function f(): number { return helper + v(); }"),
                ("a.ts", "const helper = 2;\nexport function v(): number { return helper; }"),
            ],
        )
        .unwrap();
        assert!(b.source.contains("const helper_2 = 2"), "{}", b.source);
        assert!(b.source.contains("return helper_2;"), "{}", b.source);
        assert!(b.source.contains("return helper + v()"), "{}", b.source);
    }

    // M9-TS-1803
    #[test]
    fn imports_through_a_re_export_and_an_alias_reach_the_declaration() {
        let b = run(
            "main.ts",
            &[
                (
                    "main.ts",
                    "import { y as z } from \"./index\";\nexport const k = z + 1;",
                ),
                (
                    "index.ts",
                    "export { x as y } from \"./impl\";\nexport * from \"./more\";",
                ),
                ("impl.ts", "export const x = 1;"),
                ("more.ts", "export const m = 2;"),
            ],
        )
        .unwrap();
        assert!(b.source.contains("export const k = x + 1"), "{}", b.source);
        assert_eq!(b.files.len(), 4);
    }

    // M9-TS-1804
    #[test]
    fn shorthand_properties_keep_their_name_when_the_value_is_renamed() {
        let b = run(
            "main.ts",
            &[
                ("main.ts", "import { n as count } from \"./a\";\nexport const o = { count };\nconst n = 5;"),
                ("a.ts", "export const n = 1;"),
            ],
        )
        .unwrap();
        assert!(
            b.source.contains("{ count: n_2 }") || b.source.contains("{ count: n }"),
            "{}",
            b.source
        );
    }

    // M9-TS-1805
    #[test]
    fn css_imports_are_listed_not_read_and_missing_files_are_errors() {
        let b = run(
            "src/main.tsx",
            &[(
                "src/main.tsx",
                "import \"./ui.css\";\nimport \"./ui.css\";\nexport const a = 1;",
            )],
        )
        .unwrap();
        assert_eq!(b.css, ["src/ui.css"]);
        assert!(!b.source.contains("import"));
        let e = run("main.ts", &[("main.ts", "import { q } from \"./nope\";")]).unwrap_err();
        assert!(e.contains("cannot find module"), "{e}");
        let e = run(
            "main.ts",
            &[
                ("main.ts", "import { q } from \"./a\";"),
                ("a.ts", "export const z = 1;"),
            ],
        )
        .unwrap_err();
        assert!(e.contains("no export `q`"), "{e}");
    }

    // M9-TS-1806
    #[test]
    fn export_default_expressions_get_a_name() {
        let b = run(
            "main.ts",
            &[
                (
                    "main.ts",
                    "import cfg from \"./cfg\";\nexport const a = cfg.n;",
                ),
                ("cfg.ts", "export default { n: 3 };"),
            ],
        )
        .unwrap();
        assert!(
            b.source.contains("const __default_cfg_ts = { n: 3 }"),
            "{}",
            b.source
        );
        assert!(b.source.contains("= __default_cfg_ts.n"), "{}", b.source);
    }

    // M9-TS-2870
    #[test]
    fn a_bare_import_is_a_file_an_index_or_a_subpath_of_src_modules() {
        let b = run(
            "src/main.ts",
            &[
                (
                    "src/main.ts",
                    "import { lerp } from \"mathx\";
import { Stack } from \"ds\";
import { fmt } from \"ds/text/fmt\";
export const v = lerp(0, 1, 0.5) + fmt() + new Stack().n;",
                ),
                ("src/modules/mathx.ts", "export function lerp(a: number, b: number, t: number): number { return a + (b - a) * t; }"),
                ("src/modules/ds/index.ts", "export class Stack { n = 1; }"),
                ("src/modules/ds/text/fmt.tsx", "export function fmt(): number { return 2; }"),
            ],
        )
        .unwrap();
        assert_eq!(
            b.files,
            [
                "src/modules/mathx.ts",
                "src/modules/ds/index.ts",
                "src/modules/ds/text/fmt.tsx",
                "src/main.ts"
            ]
        );
        assert!(!b.source.contains("import"), "{}", b.source);
    }

    // M9-TS-2871
    #[test]
    fn a_missing_bare_module_names_where_it_looked() {
        let e = run(
            "src/main.ts",
            &[("src/main.ts", "import { q } from \"foo\";")],
        )
        .unwrap_err();
        assert!(
            e.contains("cannot find module \"foo\": bare imports are looked up in src/modules/ (tried src/modules/foo.ts, src/modules/foo.tsx, src/modules/foo/index.ts"),
            "{e}"
        );
        assert!(
            e.contains("copy its .ts source into src/modules/<name>/"),
            "{e}"
        );
    }

    // M9-TS-2872
    #[test]
    fn libraries_win_over_modules_and_stay_erased() {
        let b = run(
            "src/main.ts",
            &[
                (
                    "src/main.ts",
                    "import { useState } from \"react\";
import { Color } from \"@iron/engine\";
export const c = Color.white;",
                ),
                (
                    "src/modules/react.ts",
                    "throw new Error(\"must not load\");",
                ),
            ],
        )
        .unwrap();
        assert_eq!(b.files, ["src/main.ts"]);
        assert!(
            b.library_imports
                .contains(&("react".into(), "useState".into()))
        );
    }

    // M9-TS-2873
    #[test]
    fn modules_import_each_other_relatively_and_by_name_and_load_once() {
        let b = run(
            "src/main.ts",
            &[
                (
                    "src/main.ts",
                    "import { a } from \"one\";
import { b } from \"two\";
import { base } from \"shared\";
export const v = a() + b() + base();",
                ),
                (
                    "src/modules/one.ts",
                    "import { base } from \"./shared\";
export function a(): number { return base(); }",
                ),
                (
                    "src/modules/two.ts",
                    "import { base } from \"shared\";
export function b(): number { return base(); }",
                ),
                (
                    "src/modules/shared.ts",
                    "export function base(): number { return 1; }",
                ),
            ],
        )
        .unwrap();
        assert_eq!(b.source.matches("function base").count(), 1, "{}", b.source);
        assert_eq!(
            b.files.iter().filter(|f| f.ends_with("shared.ts")).count(),
            1
        );
    }
}
