// SPDX-License-Identifier: Apache-2.0

//! The UI half of the generated declarations, written from the tables of `platypus_ui::tables` (style
//! properties, attributes, elements, events, React names) and from the `Ui` manifest: `interface
//! PlatypusStyle`, the event interfaces, `PlatypusProps`, `JSX`, the hooks, `createRoot` and the element
//! components. Nothing here is a declaration file written by hand; the TypeScript text a few names need
//! (the hook signatures, `ReactNode`) is Rust data next to the tables (decision 1 of
//! `docs/research/platypus-generated-dts-plan.md`).
//!
//! It also writes the compiled side of the same names (`prelude`): `StyleValue`, `StyleEntry`, the
//! `UiEvent` class whose fields read the event's slots, and the aliases `type PlatypusPointerEvent = UiEvent`.

use platypus_ui::tables::attrs::ATTRS;
use platypus_ui::tables::elements::ELEMENTS;
use platypus_ui::tables::events::{
    DATA_TRANSFER_TS, EVENT_FIELDS, EVENT_INTERFACES, EVENT_METHODS, EVENT_PROPS, EventField, Init,
};
use platypus_ui::tables::react::{CREATE_ROOT, HOOKS, Module, PLATYPUS_REACT_TYPES, REACT_TYPES};
use platypus_ui::tables::style::{NAMED_TYPES, STYLE_PROPS, ts_type};
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
    // A parameter's TypeScript type: its `ts` text (a literal union named in `aliases`), else
    // the manifest type's.
    let ts = |p: &serde_json::Value| {
        p["ts"].as_str().map_or_else(
            || crate::types_dts::manifest_ts_type(p["type"].as_str().unwrap_or("void")),
            str::to_owned,
        )
    };
    let mut head = String::new();
    for a in m["aliases"].as_array().into_iter().flatten() {
        head += &format!(
            "type {} = {};\n",
            a["name"].as_str().unwrap_or("_"),
            a["ts"].as_str().unwrap_or("never")
        );
    }
    let mut options_done: Vec<&str> = Vec::new();
    let mut s = String::from("interface PlatypusElement {\n");
    for f in class["functions"].as_array().into_iter().flatten() {
        let name = f["name"].as_str().unwrap_or("_");
        let returns = crate::types_dts::manifest_ts_type(f["returns"].as_str().unwrap_or("void"));
        let params: Vec<&serde_json::Value> = f["params"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|p| p["name"] != "this")
            .collect();
        // An options bag (`scrollIntoView({ block })`): its interface, the options form, and
        // unless `optionsOnly` the positional numbers (`scrollTo(left, top)`).
        if let Some(options) = f["optionsName"].as_str() {
            if !options_done.contains(&options) {
                options_done.push(options);
                head += &format!("interface {options} {{\n");
                for p in &params {
                    head += &format!("  {}?: {};\n", p["name"].as_str().unwrap_or("_"), ts(p));
                }
                head += "}\n";
            }
            if f["optionsOnly"].as_bool() != Some(true) {
                let numbers: Vec<String> = params
                    .iter()
                    .filter(|p| p["type"] == "f64")
                    .map(|p| format!("{}: {}", p["name"].as_str().unwrap_or("_"), ts(p)))
                    .collect();
                s += &format!("  {name}({}): {returns};\n", numbers.join(", "));
                s += &format!("  {name}(options: {options}): {returns};\n");
            } else {
                s += &format!("  {name}(options?: {options}): {returns};\n");
            }
            continue;
        }
        let params: Vec<String> = params
            .iter()
            .map(|p| format!("{}: {}", p["name"].as_str().unwrap_or("_"), ts(p)))
            .collect();
        s += &format!("  {name}({}): {returns};\n", params.join(", "));
    }
    s += "}\n";
    head + &s
}

fn style_interface() -> String {
    let mut s = String::from("interface PlatypusStyle {\n");
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
    s += DATA_TRANSFER_TS;
    s
}

fn props_interface() -> String {
    let mut s = String::from("interface PlatypusProps {\n");
    let mut svg_comment = false;
    for a in ATTRS {
        if a.scope == platypus_ui::tables::attrs::Scope::Svg && !svg_comment {
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
        "declare namespace JSX {\n  type Element = ReactElementLike;\n  // A component may return any node (text, a list, null), as React's types allow (TS 5.1).\n  type ElementType = string | ((props: any) => ReactNode);\n  interface ElementChildrenAttribute {\n    children: {};\n  }\n  interface IntrinsicAttributes {\n    key?: string | number;\n  }\n  interface IntrinsicElements {\n",
    );
    for e in ELEMENTS {
        s += &format!("    {}: PlatypusProps;\n", e.tag);
    }
    s += "    [tag: string]: PlatypusProps;\n  }\n}\n";
    s
}

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
  type CSSProperties = PlatypusStyle;
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

/// The ambient declarations: what `@platypus/react`, `react` and a few engine names give, without
/// node_modules. `platypus-build types` puts this after the engine globals in `types/platypus.d.ts`.
pub fn declarations() -> String {
    let mut s = String::from(
        "// The ambient declarations of a Platypus program: what `@platypus/react` and `react` give, without node_modules.\n// Generated from the tables of platypus_ui::tables (style, attributes, events, elements, React names).\n\n",
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
    s += "\n// The names of `react`, `@platypus/react` and `@platypus/engine`'s UI side are globals: a game needs no import.\n// `platypus-build types` also declares the three modules as re-exports of these (see `types_dts.rs`).\n";
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
            s += &format!("declare function {c}(props: PlatypusProps): ReactElementLike;\n");
        }
    }
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

/// The names `@platypus/react` exports.
pub fn platypus_react_exports() -> Vec<String> {
    dedup(
        PLATYPUS_REACT_TYPES
            .iter()
            .map(|s| s.to_string())
            .chain(EVENT_INTERFACES.iter().map(|i| i.name.to_string()))
            .chain(["PlatypusDataTransfer", "PlatypusDroppedFile"].map(String::from))
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
    "// The ambient types of a Platypus program (generated by tools/platypus_ts/src/ui_dts.rs).
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
type PlatypusElement = UiNode;
";

const STYLE_ENTRIES: &str = "// A style is a list of property entries: `{ width: 10 }` typed `PlatypusStyle` builds the entries,
// `...other` concatenates, a later entry of a name wins when they are applied.
class StyleEntry {
  key: string = \"\";
  /** 0 number, 1 string, 2 nothing (the value was undefined). */
  kind: number = 2;
  n: number = 0;
  s: string = \"\";
}
type PlatypusStyle = StyleEntry[];
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
function __apply_style(h: UiElement, style: PlatypusStyle | undefined): UiElement {
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
        Init::Related => "Ui.evNum(9) === 0 ? null : new UiRelated()".into(),
        Init::Transfer => "__ui_transfer()".into(),
        Init::Dropped => "Ui.evKind() === \"dragend\" && Ui.evNum(4) === 1".into(),
    }
}

/// The compiled `dataTransfer` of a drag event (`PlatypusDataTransfer` of the tables): one object from
/// `dragstart` to `dragend` for an element drag, one per set of files dragged in from the system.
/// `Ui.evStr` holds the dragged files' paths.
const DATA_TRANSFER_PRELUDE: &str = "class UiRelated {
  id: number = Ui.evNum(9);
}

class UiDroppedFile {
  path: string;
  name: string;
  constructor(path: string) {
    this.path = path;
    let end = path.length;
    while (end > 0 && (path.charCodeAt(end - 1) === 47 || path.charCodeAt(end - 1) === 92)) end--;
    let cut = end;
    while (cut > 0 && path.charCodeAt(cut - 1) !== 47 && path.charCodeAt(cut - 1) !== 92) cut--;
    this.name = end === 0 ? path : path.slice(cut, end);
  }
}

class UiDataTransfer {
  dropEffect: string = \"none\";
  effectAllowed: string = \"uninitialized\";
  files: UiDroppedFile[] = [];
  keys: string[] = [];
  values: string[] = [];
  get types(): string[] {
    const r: string[] = [];
    for (let i = 0; i < this.keys.length; i++) r.push(this.keys[i]);
    if (this.files.length > 0) r.push(\"Files\");
    return r;
  }
  setData(format: string, data: string): void {
    const k = format.toLowerCase();
    const i = this.keys.indexOf(k);
    if (i >= 0) {
      this.values[i] = data;
    } else {
      this.keys.push(k);
      this.values.push(data);
    }
  }
  getData(format: string): string {
    const i = this.keys.indexOf(format.toLowerCase());
    return i >= 0 ? this.values[i] : \"\";
  }
  clearData(format?: string): void {
    if (format === undefined) {
      this.keys = [];
      this.values = [];
      return;
    }
    const i = this.keys.indexOf(format.toLowerCase());
    if (i >= 0) {
      this.keys.splice(i, 1);
      this.values.splice(i, 1);
    }
  }
  setDragImage(element: UiTarget | null, x: number = 0, y: number = 0): void {
    Ui.dragImage(element === null ? 0 : element.id, x, y);
  }
}

let __ui_drag: UiDataTransfer | null = null;
let __ui_file_drag: UiDataTransfer | null = null;
let __ui_file_key: string = \"\";
let __ui_no_drag: UiDataTransfer | null = null;

function __ui_transfer(): UiDataTransfer {
  const kind = Ui.evKind();
  if (kind.indexOf(\"drag\") !== 0 && kind !== \"drop\") {
    let none = __ui_no_drag;
    if (none === null) {
      none = new UiDataTransfer();
      __ui_no_drag = none;
    }
    return none;
  }
  const n = Ui.evStrCount();
  if (n > 0) {
    const paths: string[] = [];
    for (let i = 0; i < n; i++) paths.push(Ui.evStr(i));
    const key = paths.join(\"|\");
    let files = __ui_file_drag;
    if (files === null || __ui_file_key !== key) {
      files = new UiDataTransfer();
      for (let i = 0; i < n; i++) files.files.push(new UiDroppedFile(paths[i]));
      __ui_file_drag = files;
      __ui_file_key = key;
    }
    return files;
  }
  let t = __ui_drag;
  if (kind === \"dragstart\" || t === null) {
    t = new UiDataTransfer();
    __ui_drag = t;
  }
  if (kind === \"dragend\") {
    if (Ui.evNum(4) !== 1) t.dropEffect = \"none\";
    __ui_drag = null;
  }
  return t;
}

";

fn ui_event_class() -> String {
    let mut s = String::from(DATA_TRANSFER_PRELUDE);
    s += "class UiTarget {\n  id: number = Ui.evTarget();\n  type: string = Ui.evTargetKind();\n  value: string = Ui.evStr(0);\n}\n\nclass UiEvent {\n";
    for f in EVENT_FIELDS {
        let ty = match f.name {
            "target" => "UiTarget",
            "relatedTarget" => "UiRelated | null",
            "dataTransfer" => "UiDataTransfer",
            _ => f.ts,
        };
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
        s += "type PlatypusDataTransfer = UiDataTransfer;
type PlatypusDroppedFile = UiDroppedFile;
";
        s += "type FullScreenMode = \"ExclusiveFullScreen\" | \"FullScreenWindow\" | \"MaximizedWindow\" | \"Windowed\";\n";
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
                t.contains(&format!("    {}: PlatypusProps;", e.tag)),
                "{}",
                e.tag
            );
            if let Some(c) = e.component {
                assert!(
                    t.contains(&format!(
                        "declare function {c}(props: PlatypusProps): ReactElementLike;"
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
        assert!(t.contains("interface PlatypusElement {\n  focus(): void;\n  blur(): void;\n  scrollTo(left: number, top: number): void;\n  scrollTo(options: ScrollToOptions): void;\n  scrollBy(left: number, top: number): void;\n  scrollBy(options: ScrollToOptions): void;\n  scrollIntoView(options?: ScrollIntoViewOptions): void;\n}"), "{t}");
        // The DOM's scroll options (M3-EVT-08 / 09 in compiled games).
        assert!(t.contains("interface ScrollIntoViewOptions {\n  block?: ScrollLogicalPosition;\n  inline?: ScrollLogicalPosition;\n  behavior?: ScrollBehavior;\n}"), "{t}");
        assert!(
            t.contains("type ScrollBehavior = \"auto\" | \"instant\" | \"smooth\";"),
            "{t}"
        );
        // Docs travel with the properties.
        assert!(t.contains("/** `contents` drops the box: children join the parent's layout. */"));
    }

    // M9-TS-3334
    #[test]
    fn the_compiled_data_transfer_has_every_member_of_the_declared_interface() {
        let p = prelude();
        let class = &p[p.find("class UiDataTransfer").unwrap()..p.find("let __ui_drag").unwrap()];
        let decl = &DATA_TRANSFER_TS[DATA_TRANSFER_TS
            .find("interface PlatypusDataTransfer")
            .unwrap()..];
        let mut members = 0;
        for line in decl.lines().skip(1) {
            let t = line.trim_start();
            if t.starts_with("/**") || t.starts_with('}') || t.is_empty() {
                continue;
            }
            let name: String = t
                .trim_start_matches("readonly ")
                .chars()
                .take_while(|c| c.is_alphanumeric())
                .collect();
            let is_method = t.contains('(');
            assert!(
                class.contains(&format!("{}{name}", if is_method { "  " } else { "" }))
                    && (class.contains(&format!("  {name}("))
                        || class.contains(&format!("  {name}:"))
                        || class.contains(&format!("get {name}("))),
                "UiDataTransfer lacks `{name}`"
            );
            members += 1;
        }
        assert_eq!(members, 8);
        assert!(declarations().contains("interface PlatypusDataTransfer {"));
        assert!(p.contains("type PlatypusDataTransfer = UiDataTransfer;"));
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
        assert!(platypus_react_exports().contains(&"PlatypusScrollEvent".to_string()));
        assert!(platypus_react_exports().contains(&"Dialog".to_string()));
    }
}
