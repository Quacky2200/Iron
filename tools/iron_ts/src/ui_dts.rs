//! The UI half of the generated declarations, written from the tables of `iron_ui_tables` (style
//! properties, attributes, elements, events, React names) and from the `Ui` manifest: `interface
//! IronStyle`, the event interfaces, `IronProps`, `JSX`, the hooks, `createRoot` and the element
//! components. Nothing here is a declaration file written by hand; the TypeScript text a few names need
//! (the hook signatures, `ReactNode`) is Rust data next to the tables (decision 1 of
//! `docs/research/iron-generated-dts-plan.md`).
//!
//! It also writes the compiled side of the same names (`prelude`): `StyleValue`, `StyleEntry`, the
//! `UiEvent` class whose fields read the event's slots, and the aliases `type IronPointerEvent = UiEvent`.

use iron_ui_tables::attrs::ATTRS;
use iron_ui_tables::elements::ELEMENTS;
use iron_ui_tables::events::{
    EVENT_FIELDS, EVENT_INTERFACES, EVENT_METHODS, EVENT_PROPS, EventField, Init,
};
use iron_ui_tables::react::{CREATE_ROOT, HOOKS, IRON_REACT_TYPES, Module, REACT_TYPES};
use iron_ui_tables::style::{NAMED_TYPES, STYLE_PROPS, ts_type};
use std::sync::OnceLock;

/// `/** doc */` for one line, a block for several; empty for no doc.
fn doc_comment(doc: &str, indent: &str) -> String {
    if doc.is_empty() {
        return String::new();
    }
    if !doc.contains('\n') {
        return format!("{indent}/** {doc} */\n");
    }
    let mut s = format!("{indent}/**\n");
    for line in doc.lines() {
        s += &format!("{indent} * {line}\n");
    }
    s += &format!("{indent} */\n");
    s
}

/// `type Name = "a" | "b";`, broken into one `|` line per member when it does not fit 80 columns.
fn union_alias(name: &str, members: &[&str]) -> String {
    let parts: Vec<String> = members.iter().map(|m| format!("\"{m}\"")).collect();
    let one = format!("type {name} = {};\n", parts.join(" | "));
    if one.len() <= 81 {
        return one;
    }
    let mut s = format!("type {name} =\n");
    for p in &parts {
        s += &format!("  | {p}\n");
    }
    // The last member ends the statement.
    s.pop();
    s += ";\n";
    s
}

/// The members of the `Ui` manifest's `UiNode` class (`focus()`, `blur()`, `scrollTo(x, y)`): what a
/// `ref` receives.
fn element_interface() -> String {
    let m: serde_json::Value =
        serde_json::from_str(crate::lower::UI_MANIFEST).expect("the embedded Ui manifest");
    let class = m["classes"]
        .as_array()
        .and_then(|cs| cs.iter().find(|c| c["name"] == "UiNode"))
        .expect("the Ui manifest has UiNode");
    let mut s = String::from("interface IronElement {\n");
    for f in class["functions"].as_array().into_iter().flatten() {
        let params: Vec<String> = f["params"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|p| p["name"] != "this")
            .map(|p| {
                format!(
                    "{}: {}",
                    p["name"].as_str().unwrap_or("_"),
                    crate::types_dts::manifest_ts_type(p["type"].as_str().unwrap_or("void"))
                )
            })
            .collect();
        s += &format!(
            "  {}({}): {};\n",
            f["name"].as_str().unwrap_or("_"),
            params.join(", "),
            crate::types_dts::manifest_ts_type(f["returns"].as_str().unwrap_or("void"))
        );
    }
    s += "}\n";
    s
}

fn style_interface() -> String {
    let mut s = String::from("interface IronStyle {\n");
    for p in STYLE_PROPS {
        if !p.group.is_empty() {
            if !s.ends_with("{\n") {
                s += "\n";
            }
            s += &format!("  // {}\n", p.group);
        }
        s += &doc_comment(p.doc, "  ");
        s += &format!("  {}?: {};\n", p.name, ts_type(p.ty));
    }
    s += "}\n";
    s
}

fn event_interfaces() -> String {
    let field = |name: &str| -> &EventField {
        EVENT_FIELDS
            .iter()
            .find(|f| f.name == name)
            .unwrap_or_else(|| panic!("event field {name}"))
    };
    let mut s = String::new();
    for i in EVENT_INTERFACES {
        let ext = i.extends.map_or(String::new(), |e| format!(" extends {e}"));
        if i.fields.is_empty() && i.extends.is_some() {
            s += &format!("interface {}{ext} {{}}\n", i.name);
            continue;
        }
        s += &format!("interface {}{ext} {{\n", i.name);
        for n in i.fields {
            let f = field(n);
            s += &format!("  readonly {}: {};\n", f.name, f.ts);
        }
        if i.extends.is_none() {
            for (m, _) in EVENT_METHODS {
                s += &format!("  {m}(): void;\n");
            }
        }
        s += "}\n";
    }
    s
}

fn props_interface() -> String {
    let mut s = String::from("interface IronProps {\n");
    let mut svg_comment = false;
    for a in ATTRS {
        if a.scope == iron_ui_tables::attrs::Scope::Svg && !svg_comment {
            svg_comment = true;
            s += "  // Inline SVG (`<svg>` and the shapes inside it): attributes as React names them.\n";
        }
        s += &doc_comment(a.doc, "  ");
        s += &format!("  {}?: {};\n", a.name, a.ts);
    }
    for e in EVENT_PROPS {
        s += &format!("  {}?: (e: {}) => void;\n", e.prop, e.interface);
    }
    s += "  [key: `data-${string}`]: string | number | boolean | undefined;\n";
    s += "}\n";
    s
}

fn jsx_namespace() -> String {
    let mut s = String::from(
        "declare namespace JSX {\n  type Element = ReactElementLike;\n  interface ElementChildrenAttribute {\n    children: {};\n  }\n  interface IntrinsicAttributes {\n    key?: string | number;\n  }\n  interface IntrinsicElements {\n",
    );
    for e in ELEMENTS {
        s += &format!("    {}: IronProps;\n", e.tag);
    }
    s += "    [tag: string]: IronProps;\n  }\n}\n";
    s
}

/// Names the engine used to declare in `ambient.d.ts` but are not UI: they wait for `#[iron_enum]` /
/// `#[iron_record]` on their Rust types (plan phase 2, `FullScreenMode` / `Gamepad` / `Collision`).
const ENGINE_AMBIENT: &str = "type FullScreenMode = \"ExclusiveFullScreen\" | \"FullScreenWindow\" | \"MaximizedWindow\" | \"Windowed\";
interface Gamepad {
  readonly id: number;
  readonly name: string;
  connected: boolean;
  readonly buttons: Record<string, number>;
  readonly axes: Record<string, number>;
}
interface Collision {
  readonly gameObject: GameObject;
  readonly collider: Collider;
  readonly relativeSpeed: number;
  readonly speed: number;
  readonly sensor: boolean;
}
type Collision2D = Collision;
";

/// The names of [`ENGINE_AMBIENT`], re-exported from `@iron/engine`.
pub const ENGINE_AMBIENT_NAMES: &[&str] =
    &["FullScreenMode", "Gamepad", "Collision", "Collision2D"];

/// React's own types and the compiler's console, as TypeScript text.
const REACT_TYPES_TS: &str = "type ReactNode = ReactElementLike | string | number | boolean | null | undefined | ReactNode[];
interface ReactElementLike {
  readonly __element: never;
}
interface Ref<T> {
  current: T;
}
";

const REACT_NAMESPACE: &str = "declare namespace React {
  type ReactNode = globalThis.ReactNode;
  type CSSProperties = IronStyle;
}
";

const CONSOLE: &str = "// The compiler lowers console.* calls itself (no DOM lib needed).
declare const console: {
  log(...args: unknown[]): void;
  info(...args: unknown[]): void;
  debug(...args: unknown[]): void;
  warn(...args: unknown[]): void;
  error(...args: unknown[]): void;
};
";

/// The ambient declarations: what `@iron/react`, `react` and a few engine names give, without
/// node_modules. `iron-build types` puts this after the engine globals in `types/iron.d.ts`.
pub fn declarations() -> String {
    let mut s = String::from(
        "// The ambient declarations of an Iron program: what `@iron/react` and `react` give, without node_modules.\n// Generated from the tables of iron_ui_tables (style, attributes, events, elements, React names).\n\n",
    );
    for (name, members) in NAMED_TYPES {
        s += &union_alias(name, members);
    }
    s += "type Length = number | string;\n\n";
    s += &style_interface();
    s += "\n";
    s += &event_interfaces();
    s += &element_interface();
    s += REACT_TYPES_TS;
    s += "\n";
    s += &props_interface();
    s += "\n";
    s += &jsx_namespace();
    s += "\n";
    s += REACT_NAMESPACE;
    s += "\n// The names of `react`, `@iron/react` and `@iron/engine`'s UI side are globals: a game needs no import.\n// `iron-build types` also declares the three modules as re-exports of these (see `types_dts.rs`).\n";
    for h in HOOKS {
        s += &format!("declare function {}{};\n", h.name, h.sig);
    }
    s += "\n";
    s += CONSOLE;
    s += "\n";
    s += &format!(
        "declare function {}{};\n",
        CREATE_ROOT.name, CREATE_ROOT.sig
    );
    for e in ELEMENTS {
        if let Some(c) = e.component {
            s += &format!("declare function {c}(props: IronProps): ReactElementLike;\n");
        }
    }
    s += "\n";
    s += ENGINE_AMBIENT;
    s
}

fn dedup(mut v: Vec<String>) -> Vec<String> {
    let mut seen = std::collections::BTreeSet::new();
    v.retain(|n| seen.insert(n.clone()));
    v
}

/// The names `react` exports (re-exported from the globals).
pub fn react_exports() -> Vec<String> {
    dedup(
        REACT_TYPES
            .iter()
            .map(|s| s.to_string())
            .chain(
                HOOKS
                    .iter()
                    .filter(|h| h.module == Module::React)
                    .map(|h| h.name.to_string()),
            )
            .collect(),
    )
}

/// The names `@iron/react` exports.
pub fn iron_react_exports() -> Vec<String> {
    dedup(
        IRON_REACT_TYPES
            .iter()
            .map(|s| s.to_string())
            .chain(EVENT_INTERFACES.iter().map(|i| i.name.to_string()))
            .chain([CREATE_ROOT.name.to_string()])
            .chain(
                ELEMENTS
                    .iter()
                    .filter_map(|e| e.component.map(String::from)),
            )
            .collect(),
    )
}

// ---- the compiled side ----

/// `number | string` is the class `StyleValue` (a number or a string converts where one is
/// expected); a style is a list of property entries (structs have at most 64 fields, and a style has
/// about 260 properties).
const STYLE_PRELUDE: &str =
    "// The ambient types of an Iron program (generated by tools/iron_ts/src/ui_dts.rs).
// `number | string` is the class `StyleValue`; a number or a string converts where one is expected.
class StyleValue {
  n: number = 0;
  s: string = \"\";
  isStr: boolean = false;
}
function __sv_num(n: number): StyleValue {
  const v = new StyleValue();
  v.n = n;
  return v;
}
function __sv_str(s: string): StyleValue {
  const v = new StyleValue();
  v.s = s;
  v.isStr = true;
  return v;
}
type ReactNode = UiElement[];
type IronElement = UiNode;
";

const STYLE_ENTRIES: &str = "// A style is a list of property entries: `{ width: 10 }` typed `IronStyle` builds the entries,
// `...other` concatenates, a later entry of a name wins when they are applied.
class StyleEntry {
  key: string = \"\";
  /** 0 number, 1 string, 2 nothing (the value was undefined). */
  kind: number = 2;
  n: number = 0;
  s: string = \"\";
}
type IronStyle = StyleEntry[];
function __se_num(key: string, n: number): StyleEntry {
  const e = new StyleEntry();
  e.key = key;
  e.kind = 0;
  e.n = n;
  return e;
}
function __se_str(key: string, s: string): StyleEntry {
  const e = new StyleEntry();
  e.key = key;
  e.kind = 1;
  e.s = s;
  return e;
}
function __se_sv(key: string, v: StyleValue): StyleEntry {
  return v.isStr ? __se_str(key, v.s) : __se_num(key, v.n);
}
function __se_onum(key: string, n: number | undefined): StyleEntry {
  return n === undefined ? new StyleEntry() : __se_num(key, n);
}
function __se_ostr(key: string, s: string | undefined): StyleEntry {
  return s === undefined ? new StyleEntry() : __se_str(key, s);
}
function __se_osv(key: string, v: StyleValue | undefined): StyleEntry {
  return v === undefined ? new StyleEntry() : __se_sv(key, v);
}
function __apply_style(h: UiElement, style: IronStyle | undefined): UiElement {
  if (style) {
    for (const e of style) {
      if (e.kind === 0) {
        Ui.style(h, e.key, e.n);
      } else if (e.kind === 1) {
        Ui.style(h, e.key, e.s);
      }
    }
  }
  return h;
}
";

/// The initialiser of a `UiEvent` field.
fn field_init(f: &EventField) -> String {
    match f.init {
        Init::Kind => "Ui.evKind()".into(),
        Init::Target => "new UiTarget()".into(),
        Init::Num(i) => format!("Ui.evNum({i})"),
        Init::Flag(i, mask) => format!("(Ui.evNum({i}) & {mask}) !== 0"),
        Init::One(i) => format!("Ui.evNum({i}) === 1"),
        Init::Str(i) => format!("Ui.evStr({i})"),
        Init::Const(c) => c.into(),
    }
}

fn ui_event_class() -> String {
    let mut s = String::from(
        "class UiTarget {\n  id: number = Ui.evTarget();\n  type: string = Ui.evTargetKind();\n  value: string = Ui.evStr(0);\n}\n\nclass UiEvent {\n",
    );
    for f in EVENT_FIELDS {
        let ty = if f.name == "target" { "UiTarget" } else { f.ts };
        s += &format!("  {}: {} = {};\n", f.name, ty, field_init(f));
    }
    for (m, call) in EVENT_METHODS {
        s += &format!("  {m}(): void {{\n    {call};\n  }}\n");
    }
    s += "}\n";
    s
}

/// The compiled-side ambient prelude: the style classes, the aliases of the event interfaces and
/// `UiEvent`. Added to every program that uses JSX or the hooks, after the hook helpers of `ui.ts`.
pub fn prelude() -> &'static str {
    static TEXT: OnceLock<String> = OnceLock::new();
    TEXT.get_or_init(|| {
        let mut s = String::from(STYLE_PRELUDE);
        for i in EVENT_INTERFACES {
            s += &format!("type {} = UiEvent;\n", i.name);
        }
        s += "type FullScreenMode = \"ExclusiveFullScreen\" | \"FullScreenWindow\" | \"MaximizedWindow\" | \"Windowed\";\n";
        s += "interface Gamepad {\n  id: number;\n  name: string;\n  connected: boolean;\n  buttons: Record<string, number>;\n  axes: Record<string, number>;\n}\n";
        s += STYLE_ENTRIES;
        s += &ui_event_class();
        s
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxc_allocator::Allocator;
    use oxc_parser::Parser;
    use oxc_span::SourceType;

    // M9-TS-3309
    #[test]
    fn the_declarations_parse_and_list_every_table_entry() {
        let t = declarations();
        let alloc = Allocator::default();
        let ret = Parser::new(&alloc, &t, SourceType::d_ts()).parse();
        assert!(ret.diagnostics.is_empty(), "{:?}", ret.diagnostics);
        for p in STYLE_PROPS {
            assert!(
                t.contains(&format!("  {}?: {};", p.name, ts_type(p.ty))),
                "{}",
                p.name
            );
        }
        for a in ATTRS {
            assert!(
                t.contains(&format!("  {}?: {};", a.name, a.ts)),
                "{}",
                a.name
            );
        }
        for e in EVENT_PROPS {
            assert!(
                t.contains(&format!("  {}?: (e: {}) => void;", e.prop, e.interface)),
                "{}",
                e.prop
            );
        }
        for e in ELEMENTS {
            assert!(
                t.contains(&format!("    {}: IronProps;", e.tag)),
                "{}",
                e.tag
            );
            if let Some(c) = e.component {
                assert!(
                    t.contains(&format!(
                        "declare function {c}(props: IronProps): ReactElementLike;"
                    )),
                    "{c}"
                );
            }
        }
        for h in HOOKS {
            assert!(t.contains(&format!("declare function {}{};", h.name, h.sig)));
        }
        // Long unions break one member per line like the formatter did; short ones stay on a line.
        assert!(t.contains(
            "type Overflow = \"visible\" | \"hidden\" | \"clip\" | \"scroll\" | \"auto\";"
        ));
        assert!(t.contains("type BorderStyle =\n  | \"none\"\n"));
        // `ref` receives what the Ui manifest's `UiNode` offers.
        assert!(t.contains("interface IronElement {\n  focus(): void;\n  blur(): void;\n  scrollTo(x: number, y: number): void;\n}"), "{t}");
        // Docs travel with the properties.
        assert!(t.contains("/** `contents` drops the box: children join the parent's layout. */"));
    }

    // M9-TS-3310
    #[test]
    fn the_prelude_has_a_field_per_event_field_and_an_alias_per_interface() {
        let p = prelude();
        for f in EVENT_FIELDS {
            assert!(p.contains(&format!("  {}: ", f.name)), "{}", f.name);
        }
        for i in EVENT_INTERFACES {
            assert!(
                p.contains(&format!("type {} = UiEvent;", i.name)),
                "{}",
                i.name
            );
        }
        assert!(
            p.contains("  shiftKey: boolean = (Ui.evNum(7) & 1) !== 0;"),
            "{p}"
        );
        assert!(p.contains("  repeat: boolean = Ui.evNum(8) === 1;"), "{p}");
        assert!(p.contains("  key: string = Ui.evStr(0);"), "{p}");
        assert!(
            p.contains("  stopPropagation(): void {\n    Ui.evStop();\n  }"),
            "{p}"
        );
        assert!(p.contains("class StyleValue {") && p.contains("class StyleEntry {"));
        // The module lists come from the same tables.
        assert!(react_exports().contains(&"useReducer".to_string()));
        assert!(iron_react_exports().contains(&"IronScrollEvent".to_string()));
        assert!(iron_react_exports().contains(&"Dialog".to_string()));
    }
}
