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
pub mod callback;
pub use callback::Callback;
#[cfg(feature = "quickjs")]
pub use rquickjs;
#[cfg(feature = "quickjs")]
pub mod qjs;

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
    /// A `string`. A script string comes in as a pointer to its object (read with
    /// [`script_str`]) and a result goes out through [`return_string`]. The Rust types are `&str`
    /// and `String`.
    Str,
    /// `number[]`, passed as the script array.
    F64s,
    /// `string[]`, passed as the script array of strings (read with [`script_strs`]). The Rust
    /// types are `&[String]` and `Vec<String>`.
    Strs,
    /// An array of a value class (its TypeScript name, and that name with `[]`).
    Values(&'static str, &'static str),
    /// A closure the engine keeps and calls later: the manifest signature (`fn(f64,string):void`)
    /// and the TypeScript type (`(a: number, b: string) => void`).
    Callback(&'static str, &'static str),
    /// An enum declared in `enums(..)`: scripts see one of its names (a string); across the C ABI
    /// it is the index of the name (`i32`, -1 for a name that is not one of them).
    Enum(&'static str),
    /// A record declared in `records(..)`: a plain object of its fields. Only results so far; it
    /// comes back through [`return_flat`].
    Record(&'static str),
    /// A `Vec` result of values or records: the manifest name of the element (`value:Vector2`)
    /// and the TypeScript type (`Vector2[]`). It comes back through [`return_flat`].
    List(&'static str, &'static str),
}

impl Ty {
    /// The TypeScript type.
    pub fn ts(&self) -> &'static str {
        match self {
            Ty::Void => "void",
            Ty::Bool => "boolean",
            Ty::F64 | Ty::F32 | Ty::I32 | Ty::U32 => "number",
            Ty::Handle(name) | Ty::Value(name) => name,
            Ty::Str => "string",
            Ty::F64s => "number[]",
            Ty::Strs => "string[]",
            Ty::Values(_, arr) => arr,
            Ty::Callback(_, ts) => ts,
            Ty::Enum(name) | Ty::Record(name) => name,
            Ty::List(_, arr) => arr,
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
            Ty::Str => "string".into(),
            Ty::F64s => "f64s".into(),
            Ty::Strs => "strings".into(),
            Ty::Values(n, _) => format!("values:{n}"),
            Ty::Callback(sig, _) => (*sig).into(),
            Ty::Enum(n) => format!("enum:{n}"),
            Ty::Record(n) => format!("record:{n}"),
            Ty::List(inner, _) => format!("list:{inner}"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Param {
    pub name: &'static str,
    pub ty: Ty,
    /// A trailing parameter a call may leave out (`#[optional(name)]`): the function sees NaN for
    /// a number, an empty string, `false` or a zero handle.
    pub optional: bool,
    /// What a left-out parameter is when `#[optional(name = value)]` says (a flag is 0 or 1).
    pub default: Option<f64>,
    /// The last parameter takes every remaining argument (`#[rest(points)]`): `...points: V[]`.
    pub rest: bool,
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

#[derive(Clone, Copy, Debug, PartialEq)]
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
    /// `#[options(after = p)]`: scripts may pass the optional parameters after parameter `p` (the
    /// index) as one trailing object (`new Spline(points, { closed: true })`).
    pub options_after: Option<usize>,
    /// `#[nullable]`: a handle result (or the value a setter takes) may be `null`.
    pub nullable: bool,
    /// `#[ts_type = ".."]`: the TypeScript type of a property, when it is narrower than the Rust
    /// type (a string-literal union for a `String`).
    pub ts_type: Option<&'static str>,
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
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClassDef {
    pub name: &'static str,
    pub doc: &'static str,
    pub functions: &'static [Function],
}

/// An enum scripts see as a string union (`"auto" | "bezier" | "linear"`) and that travels as the
/// index of the name. The namespace that declares it also exports `name(i)` and `parse(s)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EnumDef {
    pub name: &'static str,
    pub doc: &'static str,
    /// The names, in index order.
    pub values: &'static [&'static str],
}

/// One field of a record.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RecordField {
    pub name: &'static str,
    /// `F64`, `Bool`, a value class or an enum.
    pub ty: Ty,
    pub doc: &'static str,
}

/// A plain object scripts get as a result (`SplineHit { segment, u }`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RecordDef {
    pub name: &'static str,
    pub doc: &'static str,
    pub fields: &'static [RecordField],
}

/// The addresses of a namespace's `extern "C"` shims by symbol name (what `#[iron_api]` emits as
/// `iron_symbols`). A JIT resolves a compiled program's imports through [`symbol`].
#[derive(Clone, Copy)]
pub struct SymbolTable(pub fn() -> Vec<(&'static str, usize)>);

impl std::fmt::Debug for SymbolTable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SymbolTable")
    }
}

impl PartialEq for SymbolTable {
    fn eq(&self, other: &Self) -> bool {
        self.0 as usize == other.0 as usize
    }
}

type Registry = std::sync::Mutex<std::collections::HashMap<&'static str, usize>>;

fn symbol_registry() -> &'static Registry {
    static R: std::sync::OnceLock<Registry> = std::sync::OnceLock::new();
    R.get_or_init(Default::default)
}

/// Adds symbols (name, address) to the process-wide table.
pub fn register_symbols(symbols: impl IntoIterator<Item = (&'static str, usize)>) {
    let mut r = symbol_registry().lock().unwrap_or_else(|e| e.into_inner());
    r.extend(symbols);
}

/// The address of an engine shim or runtime function, if its namespace was registered. Reading a
/// namespace's manifest (`Namespace::to_json`, so every crate's `manifests()`) registers it.
pub fn symbol(name: &str) -> Option<usize> {
    if name == "iron_api_result" {
        return Some(iron_api_result as *const () as usize);
    }
    if name == "iron_api_result_str" {
        return Some(iron_api_result_str as *const () as usize);
    }
    let r = symbol_registry().lock().unwrap_or_else(|e| e.into_inner());
    r.get(name).copied()
}

/// How many symbols are registered.
pub fn symbol_count() -> usize {
    symbol_registry()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .len()
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Namespace {
    /// The shims' addresses.
    pub symbols: SymbolTable,
    pub name: &'static str,
    /// The doc comment of the module.
    pub doc: &'static str,
    /// Value parameters are declared as `VectorNLike` (a plain `{ x, y }` object does): `like`.
    pub like: bool,
    /// The handle types it mentions: `(TypeScript name)`.
    pub handles: &'static [&'static str],
    pub enums: &'static [EnumDef],
    pub records: &'static [RecordDef],
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

/// The header of a script string object (`iron_ts_rt::Str`): the bytes follow it. The layout is
/// part of the contract with the compiled code and is checked by a test in `iron_ts_tests`.
#[repr(C)]
pub struct ScriptStr {
    pub header: u64,
    pub byte_len: i64,
    pub utf16_len: i64,
    pub flags: u64,
    cursor_unit: i64,
    cursor_byte: i64,
}

const SCRIPT_STR_WELL_FORMED: u64 = 2;

/// The numbers of a script `number[]`, in place. A null array is empty.
///
/// # Safety
/// `p` must be null or a live script array of numbers that stays alive and unchanged while the
/// slice is used.
pub unsafe fn script_f64s<'a>(p: *const u8) -> &'a [f64] {
    if p.is_null() {
        return &[];
    }
    // SAFETY: by contract: header, length, capacity, then the data pointer of 8-byte slots.
    unsafe {
        let a = &*(p as *const ScriptArray);
        if a.len <= 0 || a.data.is_null() {
            return &[];
        }
        std::slice::from_raw_parts(a.data as *const f64, a.len as usize)
    }
}

/// The strings of a script `string[]`, copied. A null array is empty.
///
/// # Safety
/// `p` must be null or a live script array of strings (or nulls) that stays alive and unchanged
/// while this runs.
pub unsafe fn script_strs(p: *const u8) -> Vec<String> {
    if p.is_null() {
        return Vec::new();
    }
    // SAFETY: by contract: header, length, capacity, then the data pointer of 8-byte slots
    // holding string pointers.
    unsafe {
        let a = &*(p as *const ScriptArray);
        if a.len <= 0 || a.data.is_null() {
            return Vec::new();
        }
        (0..a.len as usize)
            .map(|i| script_str(*(a.data as *const *const u8).add(i)).into_owned())
            .collect()
    }
}

/// The elements of a script array of value-class objects (`Vector3[]`), read into `T`s.
///
/// # Safety
/// `p` must be null or a live script array whose elements are live objects of the value class
/// (a header word and `N` number fields).
pub unsafe fn script_values<const N: usize, T: ValueType<N>>(p: *const u8) -> Vec<T> {
    if p.is_null() {
        return Vec::new();
    }
    // SAFETY: by contract.
    unsafe {
        let a = &*(p as *const ScriptArray);
        if a.len <= 0 || a.data.is_null() {
            return Vec::new();
        }
        (0..a.len as usize)
            .map(|i| {
                let obj = *(a.data as *const *const f64).add(i);
                let mut parts = [0.0f64; N];
                for (k, part) in parts.iter_mut().enumerate() {
                    *part = if obj.is_null() { 0.0 } else { *obj.add(1 + k) };
                }
                T::from_parts(parts)
            })
            .collect()
    }
}

/// The layout of a script array (`iron_ts_rt::Array`).
#[repr(C)]
struct ScriptArray {
    header: u64,
    len: i64,
    cap: i64,
    data: *mut u8,
}

/// A script string as Rust text, borrowed when it has no lone surrogates (the normal case, no
/// copy) and otherwise copied with U+FFFD for each lone surrogate. A null pointer is "".
///
/// # Safety
/// `p` must be null or point to a live script string that stays alive and unchanged while the
/// result is used (the collector does not move objects, and the caller roots the string).
pub unsafe fn script_str<'a>(p: *const u8) -> std::borrow::Cow<'a, str> {
    if p.is_null() {
        return std::borrow::Cow::Borrowed("");
    }
    // SAFETY: by contract `p` is a script string: a header and `byte_len` bytes after it.
    let (flags, bytes) = unsafe {
        let h = &*(p as *const ScriptStr);
        (
            h.flags,
            std::slice::from_raw_parts(
                p.add(std::mem::size_of::<ScriptStr>()),
                h.byte_len as usize,
            ),
        )
    };
    if flags & SCRIPT_STR_WELL_FORMED != 0 {
        // SAFETY: a well formed script string is valid UTF-8.
        return std::borrow::Cow::Borrowed(unsafe { std::str::from_utf8_unchecked(bytes) });
    }
    // A lone surrogate is `ED A0..BF xx`: one U+FFFD each.
    let mut out = String::with_capacity(bytes.len());
    let mut start = 0;
    let mut i = 0;
    while i + 2 < bytes.len() {
        if bytes[i] == 0xED && bytes[i + 1] >= 0xA0 {
            // SAFETY: the stretch before a lone surrogate is valid UTF-8.
            out.push_str(unsafe { std::str::from_utf8_unchecked(&bytes[start..i]) });
            out.push('\u{FFFD}');
            i += 3;
            start = i;
        } else {
            i += 1;
        }
    }
    // SAFETY: the rest is valid UTF-8.
    out.push_str(unsafe { std::str::from_utf8_unchecked(&bytes[start..]) });
    std::borrow::Cow::Owned(out)
}

thread_local! {
    static RETURNED: std::cell::RefCell<String> = const { std::cell::RefCell::new(String::new()) };
}

/// Hands a string result to compiled code: the text is kept here (per thread) and its bytes and
/// length are returned; the caller copies them into a script string straight away, before it makes
/// another engine call.
///
/// # Safety
/// `out_len` must be valid for a write.
pub unsafe fn return_string(s: impl Into<String>, out_len: *mut i64) -> *const u8 {
    RETURNED.with(|r| {
        let mut r = r.borrow_mut();
        *r = s.into();
        // SAFETY: by contract.
        unsafe { *out_len = r.len() as i64 };
        r.as_ptr()
    })
}

thread_local! {
    static FLAT: std::cell::RefCell<Vec<f64>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// Hands a list or record result to compiled code: the numbers (a value is its fields, a boolean
/// 0 or 1, an enum its index, element after element) are kept here, per thread, and the compiled
/// call reads them one by one with `iron_api_result(i)` straight after the call, before it makes
/// another engine call.
pub fn return_flat(flat: Vec<f64>) {
    FLAT.with(|f| *f.borrow_mut() = flat);
}

thread_local! {
    static STRINGS: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// Hands a `Vec<String>` result to compiled code: the strings are kept here (per thread) and the
/// compiled call reads them one by one with `iron_api_result_str(i, ..)` straight after the call,
/// before it makes another engine call.
pub fn return_strings(strings: Vec<String>) {
    STRINGS.with(|s| *s.borrow_mut() = strings);
}

/// The `i`-th string of the last [`return_strings`] (empty past the end): its bytes, with the
/// length written to `out_len`. The caller copies them into a script string at once.
///
/// # Safety
/// `out_len` must be valid for a write.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn iron_api_result_str(i: u32, out_len: *mut i64) -> *const u8 {
    STRINGS.with(|s| {
        let s = s.borrow();
        let text = s.get(i as usize).map_or("", String::as_str);
        // SAFETY: by contract.
        unsafe { *out_len = text.len() as i64 };
        text.as_ptr()
    })
}

/// The `i`-th number of the last [`return_flat`] (0 past the end).
#[unsafe(no_mangle)]
pub extern "C" fn iron_api_result(i: u32) -> f64 {
    FLAT.with(|f| f.borrow().get(i as usize).copied().unwrap_or(0.0))
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
    s += &class_members(c);
    s += "}\n";
    s
}

/// A class function with no receiver (`this` is the first parameter of an instance member).
fn is_static_member(f: &Function) -> bool {
    f.params.first().is_none_or(|p| p.name != "this")
}

/// The statics, methods and accessors of a class (a handle class or a value class).
fn class_members(c: &ClassDef) -> String {
    let mut s = String::new();
    let user_params = |f: &Function, skip: usize| -> String {
        f.params
            .iter()
            .skip(skip)
            .map(|p| {
                format!(
                    "{}{}: {}",
                    p.name,
                    if p.optional { "?" } else { "" },
                    p.ty.ts()
                )
            })
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
                let st = is_static_member(f);
                // A setter right after its getter keeps them together in the declaration.
                let set = c.functions.iter().find(|g| {
                    g.kind == FnKind::Set && g.name == f.name && is_static_member(g) == st
                });
                if st {
                    match set {
                        None => {
                            s += &format!(
                                "  static readonly {}: {};
",
                                f.name,
                                f.ret.ts()
                            )
                        }
                        Some(set) => {
                            s += &format!(
                                "  static get {}(): {};
",
                                f.name,
                                f.ret.ts()
                            );
                            s += &format!(
                                "  static set {}(value: {});
",
                                f.name,
                                set.params[0].ty.ts()
                            );
                        }
                    }
                } else {
                    s += &format!(
                        "  get {}(): {};
",
                        f.name,
                        f.ret.ts()
                    );
                    if let Some(set) = set {
                        s += &format!(
                            "  set {}(value: {});
",
                            f.name,
                            set.params[1].ty.ts()
                        );
                    }
                }
            }
            FnKind::Set => {
                // Declared with its getter, unless there is none.
                let st = is_static_member(f);
                let has_getter = c.functions.iter().any(|g| {
                    g.kind == FnKind::Get && g.name == f.name && is_static_member(g) == st
                });
                if !has_getter {
                    if st {
                        s += &format!(
                            "  static set {}(value: {});
",
                            f.name,
                            f.params[0].ty.ts()
                        );
                    } else {
                        s += &format!(
                            "  set {}(value: {});
",
                            f.name,
                            f.params[1].ty.ts()
                        );
                    }
                }
            }
        }
    }
    s
}

fn function_json(f: &Function) -> String {
    let params: Vec<String> = f
        .params
        .iter()
        .map(|p| {
            format!(
                "{{\"name\":{},\"type\":{}{}{}{}}}",
                json_str(p.name),
                json_str(&p.ty.name()),
                if p.optional { ",\"optional\":true" } else { "" },
                p.default
                    .map_or(String::new(), |d| format!(",\"default\":{d}")),
                if p.rest { ",\"rest\":true" } else { "" }
            )
        })
        .collect();
    format!(
        "{{\"name\":{},\"symbol\":{},\"kind\":{},\"params\":[{}],\"returns\":{}{}{}{}{}}}",
        json_str(f.name),
        json_str(f.symbol),
        json_str(f.kind.name()),
        params.join(","),
        json_str(&f.ret.name()),
        if f.doc.is_empty() {
            String::new()
        } else {
            format!(",\"doc\":{}", json_str(f.doc))
        },
        f.options_after
            .map_or(String::new(), |a| format!(",\"optionsAfter\":{a}")),
        if f.nullable { ",\"nullable\":true" } else { "" },
        f.ts_type
            .map_or(String::new(), |t| format!(",\"tsType\":{}", json_str(t)))
    )
}

/// `,"doc":"..."` for a non-empty doc comment, else nothing.
fn doc_json(doc: &str) -> String {
    if doc.is_empty() {
        String::new()
    } else {
        format!(",\"doc\":{}", json_str(doc))
    }
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
            // A value class's methods are declared inside the value class below.
            if !self.values.iter().any(|v| v.name == c.name) {
                s += &class_dts(c);
            }
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
            if let Some(c) = self.classes.iter().find(|c| c.name == v.name) {
                s += &class_members(c);
            }
            s += "}\n";
        }
        s += &format!("declare namespace {} {{\n", self.name);
        for f in self.functions {
            // A namespace property: a getter (and maybe a setter) without a receiver.
            if f.kind == FnKind::Get {
                s += &jsdoc(f.doc, "  ");
                let writable = self
                    .functions
                    .iter()
                    .any(|g| g.kind == FnKind::Set && g.name == f.name);
                s += &format!(
                    "  {} {}: {};\n",
                    if writable { "let" } else { "const" },
                    f.name,
                    f.ret.ts()
                );
                continue;
            }
            if f.kind == FnKind::Set {
                continue;
            }
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
                .map(|p| {
                    format!(
                        "{}{}: {}",
                        p.name,
                        if p.optional { "?" } else { "" },
                        p.ty.ts()
                    )
                })
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
        register_symbols((self.symbols.0)());
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
                    "{{\"name\":{}{},\"functions\":[{}]}}",
                    json_str(c.name),
                    doc_json(c.doc),
                    fs.join(",")
                )
            })
            .collect();
        let enums: Vec<String> = self
            .enums
            .iter()
            .map(|e| {
                let values: Vec<String> = e.values.iter().map(|v| json_str(v)).collect();
                format!(
                    "{{\"name\":{}{},\"values\":[{}],\"nameSymbol\":{},\"parseSymbol\":{}}}",
                    json_str(e.name),
                    doc_json(e.doc),
                    values.join(","),
                    json_str(&format!("iron_{}_name", e.name)),
                    json_str(&format!("iron_{}_parse", e.name))
                )
            })
            .collect();
        let records: Vec<String> = self
            .records
            .iter()
            .map(|r| {
                let fields: Vec<String> = r
                    .fields
                    .iter()
                    .map(|f| {
                        format!(
                            "{{\"name\":{},\"type\":{}{}}}",
                            json_str(f.name),
                            json_str(&f.ty.name()),
                            doc_json(f.doc)
                        )
                    })
                    .collect();
                format!(
                    "{{\"name\":{}{},\"fields\":[{}]}}",
                    json_str(r.name),
                    doc_json(r.doc),
                    fields.join(",")
                )
            })
            .collect();
        let mut s = format!(
            "{{\"namespace\":{}{}{},\"handles\":[{}],\"values\":[{}],\"enums\":[{}],\"records\":[{}],\"classes\":[{}],\"functions\":[{}",
            json_str(self.name),
            doc_json(self.doc),
            if self.like { ",\"like\":true" } else { "" },
            handles.join(","),
            values.join(","),
            enums.join(","),
            records.join(","),
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

#[cfg(test)]
mod array_tests {
    use super::*;

    // M9-API-48
    #[test]
    fn a_script_number_array_is_read_in_place() {
        let mut data = [1.5f64, 2.5, 3.5];
        let a = ScriptArray {
            header: 0,
            len: 3,
            cap: 3,
            data: data.as_mut_ptr() as *mut u8,
        };
        let s = unsafe { script_f64s(&a as *const ScriptArray as *const u8) };
        assert_eq!(s, [1.5, 2.5, 3.5]);
        assert!(unsafe { script_f64s(std::ptr::null()) }.is_empty());
        let empty = ScriptArray {
            header: 0,
            len: 0,
            cap: 0,
            data: std::ptr::null_mut(),
        };
        assert!(unsafe { script_f64s(&empty as *const ScriptArray as *const u8) }.is_empty());
    }
}
