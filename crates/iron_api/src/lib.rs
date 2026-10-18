//! The engine's script API, declared once in Rust.
//!
//! A module marked `#[iron_api(namespace = "Time", quickjs)]` holds ordinary `pub fn`s. The macro
//! keeps them as they are and adds, inside the module:
//!
//! - `MANIFEST`: a [`Namespace`] describing every function (name, parameters, result, docs);
//! - one `extern "C"` shim per function (`iron_Time_deltaTime`), the symbol compiled scripts link to;
//! - with `quickjs`, `register_quickjs(ctx)`: the same functions as typed QuickJS globals.
//!
//! From the manifest, [`Namespace::to_dts`] writes the declaration file TypeScript sees and
//! [`Namespace::to_json`] the table the `iron_ts` compiler reads. One source, so they cannot drift.
//! Design: `docs/research/iron-ts-engine-boundary.md`. This is the first slice: free functions over
//! numbers, booleans and engine handles; value classes (`Vector3`), strings and objects come next.

pub use iron_api_macros::iron_api;
#[cfg(feature = "quickjs")]
pub use rquickjs;

/// A type that can cross the boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ty {
    Void,
    Bool,
    /// `number` in TypeScript, `f64` in Rust.
    F64,
    /// `number` in TypeScript, `f32` in Rust: narrowed when it crosses.
    F32,
    I32,
    U32,
    /// An engine object named by a 64-bit id (a generational index): `GameObject`, `Rigidbody`...
    /// The name is the TypeScript type.
    Handle(&'static str),
}

impl Ty {
    /// The TypeScript type.
    pub fn ts(&self) -> &'static str {
        match self {
            Ty::Void => "void",
            Ty::Bool => "boolean",
            Ty::F64 | Ty::F32 | Ty::I32 | Ty::U32 => "number",
            Ty::Handle(name) => name,
        }
    }

    /// The name the manifest JSON uses.
    pub fn name(&self) -> String {
        match self {
            Ty::Void => "void".into(),
            Ty::Bool => "bool".into(),
            Ty::F64 => "f64".into(),
            Ty::F32 => "f32".into(),
            Ty::I32 => "i32".into(),
            Ty::U32 => "u32".into(),
            Ty::Handle(n) => format!("handle:{n}"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Param {
    pub name: &'static str,
    pub ty: Ty,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Function {
    /// The name scripts use (`deltaTime`).
    pub name: &'static str,
    /// The Rust function it calls (`delta_time`).
    pub rust_name: &'static str,
    /// The `extern "C"` symbol (`iron_Time_deltaTime`).
    pub symbol: &'static str,
    pub doc: &'static str,
    pub params: &'static [Param],
    pub ret: Ty,
}

/// A group of functions under one name (`Time`, `Physics`): a global object in QuickJS, a `declare
/// namespace` in TypeScript.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Namespace {
    pub name: &'static str,
    /// The handle types it mentions: `(TypeScript name)`.
    pub handles: &'static [&'static str],
    pub functions: &'static [Function],
}

/// An engine object id. The Rust type that implements this is what API functions take and return;
/// scripts see the raw 64-bit value.
pub trait Handle: Copy {
    fn to_raw(self) -> u64;
    fn from_raw(raw: u64) -> Self;
}

fn json_str(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out += "\\\"",
            '\\' => out += "\\\\",
            '\n' => out += "\\n",
            c if (c as u32) < 0x20 => out += &format!("\\u{:04x}", c as u32),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

impl Namespace {
    /// The TypeScript declaration of this namespace, with the doc comments.
    pub fn to_dts(&self) -> String {
        let mut s = String::new();
        for h in self.handles {
            s += &format!(
                "/** An engine object, by id. */\ndeclare type {h} = number & {{ readonly __handle: \"{h}\" }};\n"
            );
        }
        s += &format!("declare namespace {} {{\n", self.name);
        for f in self.functions {
            if !f.doc.is_empty() {
                s += "  /**\n";
                for line in f.doc.lines() {
                    s += &format!("   * {}\n", line.trim());
                }
                s += "   */\n";
            }
            let params: Vec<String> = f
                .params
                .iter()
                .map(|p| format!("{}: {}", p.name, p.ty.ts()))
                .collect();
            s += &format!(
                "  function {}({}): {};\n",
                f.name,
                params.join(", "),
                f.ret.ts()
            );
        }
        s += "}\n";
        s
    }

    /// The manifest as JSON: what the compiler reads to call each function directly.
    pub fn to_json(&self) -> String {
        let mut s = format!("{{\"namespace\":{},\"functions\":[", json_str(self.name));
        for (i, f) in self.functions.iter().enumerate() {
            if i > 0 {
                s.push(',');
            }
            let params: Vec<String> = f
                .params
                .iter()
                .map(|p| {
                    format!(
                        "{{\"name\":{},\"type\":{}}}",
                        json_str(p.name),
                        json_str(&p.ty.name())
                    )
                })
                .collect();
            s += &format!(
                "{{\"name\":{},\"symbol\":{},\"params\":[{}],\"returns\":{}}}",
                json_str(f.name),
                json_str(f.symbol),
                params.join(","),
                json_str(&f.ret.name())
            );
        }
        s += "]}";
        s
    }
}

/// `snake_case` to `camelCase`, the casing scripts use.
pub fn camel(s: &str) -> String {
    let mut out = String::new();
    let mut up = false;
    for c in s.chars() {
        if c == '_' {
            up = true;
        } else if up {
            out.extend(c.to_uppercase());
            up = false;
        } else {
            out.push(c);
        }
    }
    out
}
