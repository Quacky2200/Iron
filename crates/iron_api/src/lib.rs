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
    /// A small value class (`Vector3`): scripts see an object with the fields declared in
    /// [`Namespace::values`]; across the C ABI it travels as one scalar per field (a result comes
    /// back through a trailing pointer).
    Value(&'static str),
}

impl Ty {
    /// The TypeScript type.
    pub fn ts(&self) -> &'static str {
        match self {
            Ty::Void => "void",
            Ty::Bool => "boolean",
            Ty::F64 | Ty::F32 | Ty::I32 | Ty::U32 => "number",
            Ty::Handle(name) | Ty::Value(name) => name,
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
            Ty::Value(n) => format!("value:{n}"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Param {
    pub name: &'static str,
    pub ty: Ty,
}

/// How a function is called from a script.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FnKind {
    /// `Namespace.f(..)` or, in a class, `Class.f(..)`.
    Static,
    /// `object.f(..)`: the first parameter of the C function is the receiver.
    Method,
    /// `object.name`: a property read.
    Get,
    /// `object.name = value`: a property write.
    Set,
}

impl FnKind {
    pub fn name(&self) -> &'static str {
        match self {
            FnKind::Static => "static",
            FnKind::Method => "method",
            FnKind::Get => "get",
            FnKind::Set => "set",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Function {
    pub kind: FnKind,
    /// The name scripts use (`deltaTime`).
    /// The name scripts use (`deltaTime`; for a property, the property).
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
/// One field of a value class.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ValueField {
    pub name: &'static str,
    /// `F32`, `F64`, `I32` or `U32`; every field of a value class has the same one.
    pub ty: Ty,
}

/// A value class: a few numbers that scripts handle as an object (`new Vector3(1, 2, 3)`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ValueDef {
    pub name: &'static str,
    pub fields: &'static [ValueField],
}

/// A handle class: `GameObject`, `Transform`. Its functions take the receiver as their first
/// parameter (`this`); scripts see methods, properties and statics.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClassDef {
    pub name: &'static str,
    pub functions: &'static [Function],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Namespace {
    pub name: &'static str,
    /// The handle types it mentions: `(TypeScript name)`.
    pub handles: &'static [&'static str],
    /// The value classes its functions take and return.
    pub values: &'static [ValueDef],
    /// The handle classes with methods, properties or statics.
    pub classes: &'static [ClassDef],
    pub functions: &'static [Function],
}

#[cfg(feature = "glam")]
mod glam_values {
    use super::ValueType;
    use glam::{Quat, Vec2, Vec3, Vec4};

    impl ValueType<2> for Vec2 {
        fn from_parts(p: [f64; 2]) -> Self {
            Vec2::new(p[0] as f32, p[1] as f32)
        }
        fn to_parts(self) -> [f64; 2] {
            [self.x as f64, self.y as f64]
        }
    }
    impl ValueType<3> for Vec3 {
        fn from_parts(p: [f64; 3]) -> Self {
            Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32)
        }
        fn to_parts(self) -> [f64; 3] {
            [self.x as f64, self.y as f64, self.z as f64]
        }
    }
    impl ValueType<4> for Vec4 {
        fn from_parts(p: [f64; 4]) -> Self {
            Vec4::new(p[0] as f32, p[1] as f32, p[2] as f32, p[3] as f32)
        }
        fn to_parts(self) -> [f64; 4] {
            [self.x as f64, self.y as f64, self.z as f64, self.w as f64]
        }
    }
    impl ValueType<4> for Quat {
        fn from_parts(p: [f64; 4]) -> Self {
            Quat::from_xyzw(p[0] as f32, p[1] as f32, p[2] as f32, p[3] as f32)
        }
        fn to_parts(self) -> [f64; 4] {
            [self.x as f64, self.y as f64, self.z as f64, self.w as f64]
        }
    }
}

/// An engine object id. The Rust type that implements this is what API functions take and return;
/// scripts see the raw 64-bit value.
/// A Rust type that scripts see as a value class of `N` numbers (`glam::Vec3` as `Vector3`).
/// The parts travel as `f64`; the manifest says how wide each is on the wire.
pub trait ValueType<const N: usize>: Copy {
    fn from_parts(parts: [f64; N]) -> Self;
    fn to_parts(self) -> [f64; N];
}

pub trait Handle: Copy {
    fn to_raw(self) -> u64;
    fn from_raw(raw: u64) -> Self;
}

fn jsdoc(doc: &str, indent: &str) -> String {
    if doc.is_empty() {
        return String::new();
    }
    let mut s = format!("{indent}/**\n");
    for line in doc.lines() {
        s += &format!("{indent} * {}\n", line.trim());
    }
    s += &format!("{indent} */\n");
    s
}

/// `declare class` for a handle class: statics, methods and properties.
fn class_dts(c: &ClassDef) -> String {
    let mut s = format!("declare class {} {{\n", c.name);
    s += "  private constructor();\n";
    s += &format!("  private readonly __handle: \"{}\";\n", c.name);
    let user_params = |f: &Function, skip: usize| -> String {
        f.params
            .iter()
            .skip(skip)
            .map(|p| format!("{}: {}", p.name, p.ty.ts()))
            .collect::<Vec<_>>()
            .join(", ")
    };
    for f in c.functions {
        match f.kind {
            FnKind::Static => {
                s += &jsdoc(f.doc, "  ");
                s += &format!(
                    "  static {}({}): {};\n",
                    f.name,
                    user_params(f, 0),
                    f.ret.ts()
                );
            }
            FnKind::Method => {
                s += &jsdoc(f.doc, "  ");
                s += &format!("  {}({}): {};\n", f.name, user_params(f, 1), f.ret.ts());
            }
            FnKind::Get => {
                s += &jsdoc(f.doc, "  ");
                s += &format!("  get {}(): {};\n", f.name, f.ret.ts());
                // A setter right after its getter keeps them together in the declaration.
                if let Some(set) = c
                    .functions
                    .iter()
                    .find(|g| g.kind == FnKind::Set && g.name == f.name)
                {
                    s += &format!("  set {}(value: {});\n", f.name, set.params[1].ty.ts());
                }
            }
            FnKind::Set => {
                // Declared with its getter, unless there is none.
                let has_getter = c
                    .functions
                    .iter()
                    .any(|g| g.kind == FnKind::Get && g.name == f.name);
                if !has_getter {
                    s += &format!("  set {}(value: {});\n", f.name, f.params[1].ty.ts());
                }
            }
        }
    }
    s += "}\n";
    s
}

fn function_json(f: &Function) -> String {
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
    format!(
        "{{\"name\":{},\"symbol\":{},\"kind\":{},\"params\":[{}],\"returns\":{}}}",
        json_str(f.name),
        json_str(f.symbol),
        json_str(f.kind.name()),
        params.join(","),
        json_str(&f.ret.name())
    )
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
            if self.classes.iter().any(|c| c.name == *h) {
                continue;
            }
            s += &format!(
                "/** An engine object, by id. */\ndeclare type {h} = number & {{ readonly __handle: \"{h}\" }};\n"
            );
        }
        for c in self.classes {
            s += &class_dts(c);
        }
        for v in self.values {
            let ctor: Vec<String> = v
                .fields
                .iter()
                .map(|f| format!("{}: number", f.name))
                .collect();
            s += &format!("declare class {} {{\n", v.name);
            s += &format!("  constructor({});\n", ctor.join(", "));
            for f in v.fields {
                s += &format!("  {}: number;\n", f.name);
            }
            s += "}\n";
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
    ///
    /// Class functions list the receiver first, as a parameter named `this`.
    pub fn to_json(&self) -> String {
        let handles: Vec<String> = self.handles.iter().map(|h| json_str(h)).collect();
        let values: Vec<String> = self
            .values
            .iter()
            .map(|v| {
                let fields: Vec<String> = v
                    .fields
                    .iter()
                    .map(|f| {
                        format!(
                            "{{\"name\":{},\"type\":{}}}",
                            json_str(f.name),
                            json_str(&f.ty.name())
                        )
                    })
                    .collect();
                format!(
                    "{{\"name\":{},\"fields\":[{}]}}",
                    json_str(v.name),
                    fields.join(",")
                )
            })
            .collect();
        let functions: Vec<String> = self.functions.iter().map(function_json).collect();
        let classes: Vec<String> = self
            .classes
            .iter()
            .map(|c| {
                let fs: Vec<String> = c.functions.iter().map(function_json).collect();
                format!(
                    "{{\"name\":{},\"functions\":[{}]}}",
                    json_str(c.name),
                    fs.join(",")
                )
            })
            .collect();
        let mut s = format!(
            "{{\"namespace\":{},\"handles\":[{}],\"values\":[{}],\"classes\":[{}],\"functions\":[{}",
            json_str(self.name),
            handles.join(","),
            values.join(","),
            classes.join(","),
            functions.join(",")
        );
        s += "]}";
        s
    }
}

/// The engine's current world, for API functions that need it.
///
/// An `extern "C"` shim cannot take the world as an argument (scripts do not have one), so the
/// host makes it current on the thread that runs scripts and API functions reach it with
/// [`with`]. Declare the slot as a `thread_local!` of `Cell<*mut World>`:
///
/// ```ignore
/// thread_local! { static WORLD: Cell<*mut World> = const { Cell::new(std::ptr::null_mut()) }; }
/// let _scope = iron_api::context::enter(&WORLD, &mut world); // while scripts run
/// // inside an API function:
/// iron_api::context::with(&WORLD, |w| w.transform(id)).unwrap_or_default()
/// ```
pub mod context {
    use std::cell::Cell;
    use std::marker::PhantomData;
    use std::thread::LocalKey;

    /// Restores the previous current world when dropped.
    pub struct Scope<'a, T: 'static> {
        slot: &'static LocalKey<Cell<*mut T>>,
        previous: *mut T,
        _borrow: PhantomData<&'a mut T>,
    }

    /// Makes `value` the current world on this thread until the returned scope is dropped.
    pub fn enter<'a, T: 'static>(
        slot: &'static LocalKey<Cell<*mut T>>,
        value: &'a mut T,
    ) -> Scope<'a, T> {
        let previous = slot.with(|c| c.replace(value as *mut T));
        Scope {
            slot,
            previous,
            _borrow: PhantomData,
        }
    }

    impl<T: 'static> Drop for Scope<'_, T> {
        fn drop(&mut self) {
            self.slot.with(|c| c.set(self.previous));
        }
    }

    /// Runs `f` on the current world, or returns `None` when there is none. While `f` runs the
    /// world is not current, so an API function called from inside `f` gets `None` instead of a
    /// second `&mut` to the same value.
    pub fn with<T: 'static, R>(
        slot: &'static LocalKey<Cell<*mut T>>,
        f: impl FnOnce(&mut T) -> R,
    ) -> Option<R> {
        let ptr = slot.with(|c| c.replace(std::ptr::null_mut()));
        if ptr.is_null() {
            return None;
        }
        struct Restore<T: 'static>(&'static LocalKey<Cell<*mut T>>, *mut T);
        impl<T: 'static> Drop for Restore<T> {
            fn drop(&mut self) {
                self.0.with(|c| c.set(self.1));
            }
        }
        let _restore = Restore(slot, ptr);
        // SAFETY: `ptr` came from a live `&mut T` held by a `Scope`, which outlives this call; the
        // slot is null while `f` runs, so no other `&mut` to it can be made through the slot.
        Some(f(unsafe { &mut *ptr }))
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
