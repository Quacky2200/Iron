//! Guard rails for "every declaration is generated from Rust" (docs/research/iron-generated-dts-plan.md):
//! a ratchet over the hand-written `.d.ts` files, a check that nothing is declared both by the manifests
//! and by a hand-written file, and a snapshot of the API surface `tsc` sees.

use oxc_allocator::Allocator;
use oxc_ast::ast::{Statement, TSNamespaceDeclarationBody};
use oxc_parser::Parser;
use oxc_span::{GetSpan, SourceType};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for e in rd.flatten() {
        let p = e.path();
        let name = e.file_name().to_string_lossy().to_string();
        if p.is_dir() {
            if matches!(
                name.as_str(),
                "target" | "node_modules" | "dist" | "types" | ".git" | ".claude"
            ) {
                continue;
            }
            walk(&p, out);
        } else {
            out.push(p);
        }
    }
}

fn rel(p: &Path) -> String {
    p.strip_prefix(repo())
        .unwrap_or(p)
        .to_string_lossy()
        .replace('\\', "/")
}

/// The hand-written declaration files still allowed, each with why and what removes it. Lower this
/// list, never raise it: a new `.d.ts` under `tools/` fails the test. The UI declarations
/// (`ambient.d.ts`) are generated from `iron_ui::tables` now (`tools/iron_ts/src/ui_dts.rs`).
const ALLOWED_D_TS: &[(&str, &str)] = &[(
    "tools/iron_ts/src/lower/engine_ts.d.ts",
    "TypeScript-side engine shapes (records, enums, promises, overloads) the manifests cannot carry yet:      plan phase 2 (F-REC, F-ENUM, F-UNION, F-PROMISE) moves them into #[iron_api]; phase 5 deletes the file",
)];

/// Hand-written TypeScript compiled into games, each with why it is not Rust yet. The facade may
/// only lose `//# when` chunks.
const MAX_FACADE_CHUNKS: usize = 23;
const ALLOWED_TS: &[(&str, &str)] = &[
    (
        "tools/iron_ts/src/lower/facade.ts",
        "engine adapters and logic that wait for native APIs (plan phases 2 and 3)",
    ),
    (
        "tools/iron_ts/src/lower/ui.ts",
        "the hook helpers (useState, useReducer, useRef, useMemo, useContext, ...): generic over the program's own types, so they are          monomorphised per use by the compiler and cannot be one native function; TODO: lower them in Rust (jsx.rs) as IR          over the native slots of iron_ts_ui once the compiler can emit generic calls into a native library",
    ),
    (
        "tools/iron_ts/src/lower/temporal.ts",
        "the Temporal runtime library (decision 2: moved to Rust long term, not in this pass)",
    ),
];

// M9-TS-3100
#[test]
fn no_new_hand_written_declaration_files_appear() {
    let mut files = Vec::new();
    walk(&repo().join("tools"), &mut files);
    let found: BTreeSet<String> = files
        .iter()
        .map(|p| rel(p))
        .filter(|r| r.ends_with(".d.ts"))
        .collect();
    let allowed: BTreeSet<String> = ALLOWED_D_TS.iter().map(|(p, _)| p.to_string()).collect();
    let new: Vec<_> = found.difference(&allowed).collect();
    assert!(
        new.is_empty(),
        "hand-written .d.ts files are not allowed (generate them from #[iron_api]): {new:?}"
    );
    let stale: Vec<_> = allowed.difference(&found).collect();
    assert!(
        stale.is_empty(),
        "these files are gone: remove them from ALLOWED_D_TS (the ratchet only goes down): {stale:?}"
    );
    // No Rust source may include a declaration file that is not on the list either.
    let mut included = BTreeSet::new();
    for p in &files {
        if p.extension().is_some_and(|e| e == "rs") {
            let Ok(t) = std::fs::read_to_string(p) else {
                continue;
            };
            for l in t.lines() {
                if l.contains("include_str!(") && l.contains(".d.ts\"") {
                    let name = l.split('"').nth(1).unwrap_or("");
                    included.insert(name.rsplit('/').next().unwrap_or("").to_string());
                }
            }
        }
    }
    let known: BTreeSet<String> = allowed
        .iter()
        .map(|p| p.rsplit('/').next().unwrap().to_string())
        .collect();
    assert!(
        included.is_subset(&known),
        "include_str! of an unlisted declaration file: {included:?}"
    );
    // The facade and the Python generator only shrink.
    let facade = std::fs::read_to_string(repo().join("tools/iron_ts/src/lower/facade.ts")).unwrap();
    let chunks = facade
        .lines()
        .filter(|l| l.starts_with("//# when "))
        .count();
    assert!(
        chunks <= MAX_FACADE_CHUNKS,
        "facade.ts has {chunks} chunks, at most {MAX_FACADE_CHUNKS}: lower the limit when you remove one"
    );
    assert_eq!(
        chunks, MAX_FACADE_CHUNKS,
        "facade.ts lost a chunk: lower MAX_FACADE_CHUNKS to {chunks}"
    );
    let ts: BTreeSet<String> = files
        .iter()
        .map(|p| rel(p))
        .filter(|r| {
            r.starts_with("tools/iron_ts/src/lower/") && r.ends_with(".ts") && !r.ends_with(".d.ts")
        })
        .collect();
    let allowed_ts: BTreeSet<String> = ALLOWED_TS.iter().map(|(p, _)| p.to_string()).collect();
    assert_eq!(
        ts, allowed_ts,
        "TypeScript compiled into games: list it in ALLOWED_TS with a reason and a TODO, or move it to Rust"
    );
    let py: BTreeSet<String> = files
        .iter()
        .map(|p| rel(p))
        .filter(|r| r.ends_with(".py") && r.starts_with("tools/iron_ts/"))
        .collect();
    assert!(py.is_empty(), "no Python step under tools/iron_ts: {py:?}");
}

/// Reads a top-level name out of a declaration line (`declare class X {`, `interface X {`,
/// `type X = ..`, `declare function f(..)`, `declare const c: T`, `declare namespace N {`).
fn declared_names(src: &str) -> BTreeSet<String> {
    let alloc = Allocator::default();
    let ret = Parser::new(&alloc, src, SourceType::d_ts()).parse();
    assert!(ret.diagnostics.is_empty(), "{:?}", ret.diagnostics);
    let mut names = BTreeSet::new();
    for s in &ret.program.body {
        let text = &src[s.span().start as usize..s.span().end as usize];
        let mut t = text.trim_start();
        for kw in ["export ", "declare ", "abstract "] {
            t = t.strip_prefix(kw).unwrap_or(t);
        }
        for kw in [
            "class ",
            "interface ",
            "namespace ",
            "function ",
            "const ",
            "let ",
            "var ",
            "type ",
            "enum ",
        ] {
            if let Some(r) = t.strip_prefix(kw) {
                let name: String = r
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '$')
                    .collect();
                if !name.is_empty() {
                    names.insert(name);
                }
                break;
            }
        }
    }
    names
}

/// Names that both a manifest and a hand-written declaration file declare today (an `interface`
/// of the supplement that adds members to a generated class, a `type` next to a handle). The
/// plan removes them phase by phase; a new one fails the test, and one that goes must be taken
/// off this list.
const KNOWN_OVERLAPS: &[&str] = &[
    "Camera",
    "Light",
    "Material",
    "Mesh",
    "Model",
    "ParticleSystem",
    "Scene",
    "SceneManager",
    "Sprite",
    "Texture",
    "Window",
    "AudioSource",
    "LODGroup",
    "MeshRenderer",
    "AnimationCurve",
    "AnimationController",
    "GameObject",
    "Prefab",
    "Random",
    "input",
];

// M9-TS-3101
#[test]
fn nothing_is_declared_by_both_the_manifests_and_a_hand_written_file() {
    let ms: Vec<String> = iron_build::manifests()
        .into_iter()
        .filter(|m| !m.contains("\"namespace\":\"Ui\""))
        .collect();
    let generated = iron_ts::types_dts::generate(&ms, "").unwrap();
    // The `declare module` re-export blocks are not declarations of their own.
    let gen_names = declared_names(&generated);
    let hand = declared_names(iron_ts::types_dts::ENGINE_TS_DTS);
    let overlap: BTreeSet<String> = gen_names.intersection(&hand).cloned().collect();
    let known: BTreeSet<String> = KNOWN_OVERLAPS.iter().map(|s| s.to_string()).collect();
    let new: Vec<_> = overlap.difference(&known).collect();
    assert!(
        new.is_empty(),
        "declared by a manifest and by a hand-written .d.ts: {new:?}"
    );
    let gone: Vec<_> = known.difference(&overlap).collect();
    assert!(
        gone.is_empty(),
        "no longer declared twice, remove from KNOWN_OVERLAPS: {gone:?}"
    );
}

fn collapse(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// One normalised line per top-level declaration, and per member of a class, interface or
/// namespace (sorted, comments and layout dropped).
fn surface(src: &str) -> String {
    let alloc = Allocator::default();
    let ret = Parser::new(&alloc, src, SourceType::d_ts()).parse();
    assert!(ret.diagnostics.is_empty(), "{:?}", ret.diagnostics);
    let text = |span: oxc_span::Span| collapse(&src[span.start as usize..span.end as usize]);
    let mut lines: Vec<String> = Vec::new();
    for s in &ret.program.body {
        match s {
            Statement::ClassDeclaration(c) => {
                let head = text(oxc_span::Span::new(c.span.start, c.body.span.start));
                let mut members: Vec<String> = c.body.body.iter().map(|m| text(m.span())).collect();
                members.sort();
                lines.push(head.clone());
                lines.extend(members.into_iter().map(|m| format!("{head} :: {m}")));
            }
            Statement::TSInterfaceDeclaration(i) => {
                let head = text(oxc_span::Span::new(i.span.start, i.body.span.start));
                let mut members: Vec<String> = i.body.body.iter().map(|m| text(m.span())).collect();
                members.sort();
                lines.push(head.clone());
                lines.extend(members.into_iter().map(|m| format!("{head} :: {m}")));
            }
            Statement::TSNamespaceDeclaration(m) => {
                let TSNamespaceDeclarationBody::TSModuleBlock(b) = &m.body else {
                    lines.push(text(m.span));
                    continue;
                };
                let head = text(oxc_span::Span::new(m.span.start, b.span.start));
                let mut members: Vec<String> = b.body.iter().map(|m| text(m.span())).collect();
                members.sort();
                lines.push(head.clone());
                lines.extend(members.into_iter().map(|m| format!("{head} :: {m}")));
            }
            Statement::TSExternalModuleDeclaration(m) => {
                let Some(b) = &m.body else {
                    lines.push(text(m.span));
                    continue;
                };
                let head = text(oxc_span::Span::new(m.span.start, b.span.start));
                let mut members: Vec<String> = b.body.iter().map(|m| text(m.span())).collect();
                members.sort();
                lines.push(head.clone());
                lines.extend(members.into_iter().map(|m| format!("{head} :: {m}")));
            }
            other => lines.push(text(other.span())),
        }
    }
    lines.sort();
    lines.dedup();
    let mut out = lines.join("\n");
    out.push('\n');
    out
}

fn snapshot_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/api_surface.txt")
}

// M9-TS-3102
#[test]
fn the_generated_api_surface_matches_the_blessed_snapshot() {
    let dts = iron_build::types_dts().unwrap();
    let now = surface(&dts);
    let path = snapshot_path();
    if std::env::var("IRON_BLESS").is_ok_and(|v| v == "1") {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, &now).unwrap();
        return;
    }
    let blessed = std::fs::read_to_string(&path)
        .unwrap_or_else(|_| panic!("{} is missing: run with IRON_BLESS=1", path.display()));
    if blessed != now {
        let a: BTreeSet<&str> = blessed.lines().collect();
        let b: BTreeSet<&str> = now.lines().collect();
        let removed: Vec<_> = a.difference(&b).take(40).collect();
        let added: Vec<_> = b.difference(&a).take(40).collect();
        panic!(
            "the API surface changed (review, then IRON_BLESS=1 to accept)\nremoved:\n{}\nadded:\n{}",
            removed
                .iter()
                .map(|s| format!("  - {s}"))
                .collect::<Vec<_>>()
                .join("\n"),
            added
                .iter()
                .map(|s| format!("  + {s}"))
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
}

// M9-TS-3164
//
// Declared types match what the QuickJS declarations say, so a game written for either type-checks
// as written: handle-valued properties and options may be `null`, a property that is one of a few
// names reads as that union (and still takes any string, as the engine ignores an unknown name),
// and a namespace property that is one of a few names is the union.
#[test]
fn declared_types_are_the_ones_games_are_written_against() {
    let dts = iron_build::types_dts().unwrap();
    for want in [
        "mainTexture?: Texture | null",
        "get type(): \"directional\" | \"point\" | \"spot\";",
        "set type(v: string);",
        "let softShadowQuality: \"low\" | \"medium\" | \"high\";",
    ] {
        assert!(dts.contains(want), "the declarations lack `{want}`");
    }
}
