// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

//! JSX and hooks need no manifest from the caller (`M9-TS-1500..1519`): the compiler brings the
//! `Ui` namespace itself. These tests only compile; linking `ts_ui_*` is the host's business.

fn compile(src: &str, manifests: &[String]) -> Result<(), String> {
    platypus_ts::compile_with_manifests(src, "speed", "jx_", 100, manifests).map(|_| ())
}

fn ok(src: &str) {
    if let Err(e) = compile(src, &[]) {
        panic!("{e}");
    }
}

// M9-TS-1500
#[test]
fn a_jsx_component_compiles_with_no_manifest() {
    ok(
        "function App(): UiElement { return <box style={{ width: 10 }}>hi</box>; }
        export function root(): UiElement { return <App />; }",
    );
}

// M9-TS-1501
#[test]
fn a_jsx_component_compiles_with_another_manifest() {
    let manifests = vec![r#"{"namespace":"Other","functions":[{"name":"f","symbol":"ts_other_f","params":[],"returns":"void"}]}"#.to_string()];
    let src = "export function root(): UiElement { return <box>{1}</box>; }";
    if let Err(e) = compile(src, &manifests) {
        panic!("{e}");
    }
}

// M9-TS-1502
#[test]
fn a_hook_only_program_compiles() {
    ok(
        "export function counter(): number { const [n, setN] = useState(1); setN(n + 1); return n; }",
    );
}

// M9-TS-1503
#[test]
fn fragments_compile() {
    ok("export function root(): UiElement { return <><text>a</text><text>b</text></>; }");
}

// M9-TS-1504
#[test]
fn a_conditional_child_compiles() {
    ok(
        "export function root(on: boolean): UiElement { return <box>{on && <text>x</text>}</box>; }",
    );
}

// M9-TS-1505
#[test]
fn mapped_children_compile() {
    ok("export function root(xs: string[]): UiElement {
          return <box>{xs.map((s) => <text key={s}>{s}</text>)}</box>;
        }");
}

// M9-TS-1506
#[test]
fn spread_props_compile() {
    ok(
        "function Item(p: { a: number; b: number }): UiElement { return <box>{p.a + p.b}</box>; }
        export function root(): UiElement { const p = { a: 1, b: 2 }; return <Item {...p} />; }",
    );
}

// M9-TS-1507
#[test]
fn text_children_and_conditional_rendering_compile() {
    ok("export function root(n: number): UiElement {
          return <box>Count: {n} {n > 2 ? <text>big</text> : <text>small</text>}</box>;
        }");
}

// M9-TS-1508
#[test]
fn a_jsx_syntax_error_reports_its_line_from_the_tsx_parser() {
    let src = "export function root(): UiElement {\n  return (\n    <box>\n      <text>a</box>\n    </box>\n  );\n}\n";
    let e = compile(src, &[]).unwrap_err();
    assert!(e.contains("line 4") || e.contains("line 5"), "{e}");
    assert!(!e.contains("Expected '>' but found"), "{e}");
    assert!(
        e.contains("text") || e.contains("closing") || e.contains("Unexpected"),
        "{e}"
    );
}

// M9-TS-1509
#[test]
fn typescript_that_is_not_tsx_is_left_to_the_ordinary_parse() {
    // `<T>(..) =>` is not TSX and the string mentions `</`: the TSX parse is not the one that
    // speaks. (The compiler does not support generic arrows, but that is not a parse error.)
    let src = "export function f(x: number): number { const g = <T>(v: T): T => v; return g(x) + \"a</b\".length; }";
    if let Err(e) = compile(src, &[]) {
        assert!(!e.starts_with("parse error"), "{e}");
    }
}

// M9-TS-1510
#[test]
fn an_explicit_ui_manifest_is_still_accepted() {
    let src = "export function root(): UiElement { return <box />; }";
    let manifests = vec![platypus_ts::lower::UI_MANIFEST.to_string()];
    if let Err(e) = compile(src, &manifests) {
        panic!("{e}");
    }
}
