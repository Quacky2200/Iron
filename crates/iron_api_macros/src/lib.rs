//! `#[iron_api]`: see the `iron_api` crate.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{
    Expr, FnArg, Ident, Item, ItemMod, Lit, Meta, Pat, ReturnType, Token, Type, Visibility,
    bracketed,
    parse::{Parse, ParseStream, Parser},
    punctuated::Punctuated,
    spanned::Spanned,
};

/// Marks a module whose `pub fn`s are the script API of one namespace.
///
/// ```ignore
/// #[iron_api(namespace = "Time", quickjs, handles(GameObject = ObjectId),
///            values(Vector3 = Vec3 [x: f32, y: f32, z: f32]))]
/// pub mod time_api {
///     /// Seconds since the last frame.
///     pub fn delta_time() -> f64 { ... }
/// }
/// ```
///
/// Arguments: `namespace = "Name"` (required); `quickjs` (also generate `register_quickjs`);
/// `handles(TsName = RustType, ...)` (types that implement `iron_api::Handle`);
/// `values(TsName = RustType [field: scalar, ...], ...)` (types that implement
/// `iron_api::ValueType<N>`; scripts see a class with those fields).
#[proc_macro_attribute]
pub fn iron_api(attr: TokenStream, item: TokenStream) -> TokenStream {
    match expand(attr.into(), item.into()) {
        Ok(t) => t.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

/// `Vector3 = Vec3 [x: f32, y: f32, z: f32]`
struct ValueDecl {
    ts: String,
    rust: Type,
    fields: Vec<(String, String)>,
}

impl Parse for ValueDecl {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let ts: Ident = input.parse()?;
        input.parse::<Token![=]>()?;
        let rust: Type = input.parse()?;
        let content;
        bracketed!(content in input);
        let mut fields = Vec::new();
        while !content.is_empty() {
            let name: Ident = content.parse()?;
            content.parse::<Token![:]>()?;
            let ty: Ident = content.parse()?;
            if !matches!(ty.to_string().as_str(), "f32" | "f64" | "i32" | "u32") {
                return Err(syn::Error::new(
                    ty.span(),
                    "a value field is f32, f64, i32 or u32",
                ));
            }
            fields.push((name.to_string(), ty.to_string()));
            if content.peek(Token![,]) {
                content.parse::<Token![,]>()?;
            }
        }
        if fields.is_empty() {
            return Err(syn::Error::new(ts.span(), "a value type needs fields"));
        }
        if fields.iter().any(|f| f.1 != fields[0].1) {
            return Err(syn::Error::new(
                ts.span(),
                "all fields of a value type must have the same type",
            ));
        }
        Ok(ValueDecl {
            ts: ts.to_string(),
            rust,
            fields,
        })
    }
}

/// `TangentMode = Tangent [Auto, Bezier, Linear]`: scripts see the variants as strings
/// (`"auto"`), the C ABI the index. A variant may name its string (`Windowed = "Windowed"`).
/// `extern TangentMode = ..` uses an enum another namespace declares: no helper functions or
/// symbols are made here (the module imports the declaring one's helpers).
struct EnumDecl {
    ts: String,
    rust: Type,
    /// The Rust variant names.
    variants: Vec<Ident>,
    /// The script string of a variant, when it is not the variant name with a lower-case first
    /// letter.
    explicit: Vec<Option<String>>,
    /// The `///` comment in front of the declaration.
    doc: String,
    /// Declared by another namespace's `enums(..)`.
    external: bool,
}

impl EnumDecl {
    /// The strings scripts see: the variant names with a lower-case first letter.
    fn names(&self) -> Vec<String> {
        self.variants
            .iter()
            .zip(&self.explicit)
            .map(|(v, e)| {
                if let Some(e) = e {
                    return e.clone();
                }
                let v = v.to_string();
                let mut c = v.chars();
                c.next()
                    .map(|f| f.to_lowercase().collect::<String>() + c.as_str())
                    .unwrap_or_default()
            })
            .collect()
    }
}

impl Parse for EnumDecl {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let doc = doc_of(&input.call(syn::Attribute::parse_outer)?);
        let external = input.peek(Token![extern]);
        if external {
            input.parse::<Token![extern]>()?;
        }
        let ts: Ident = input.parse()?;
        input.parse::<Token![=]>()?;
        let rust: Type = input.parse()?;
        let content;
        bracketed!(content in input);
        let mut variants = Vec::new();
        let mut explicit = Vec::new();
        while !content.is_empty() {
            variants.push(content.parse::<Ident>()?);
            if content.peek(Token![=]) {
                content.parse::<Token![=]>()?;
                explicit.push(Some(content.parse::<syn::LitStr>()?.value()));
            } else {
                explicit.push(None);
            }
            if content.peek(Token![,]) {
                content.parse::<Token![,]>()?;
            }
        }
        if variants.is_empty() {
            return Err(syn::Error::new(ts.span(), "an enum needs variants"));
        }
        Ok(EnumDecl {
            ts: ts.to_string(),
            rust,
            variants,
            explicit,
            doc,
            external,
        })
    }
}

/// `SplineHit = SplineHit [segment: f64, u: f64]`: a Rust struct with those public fields that
/// scripts get as a plain object. A field is `f64`, `bool`, `string`, a declared value class
/// (`<Value>Like` in a declaration that takes a plain object) or enum, `pairs` (a list of number
/// pairs), or an `Option<..>` of those (the key may be left out). `name: T as "key"` names the
/// script key.
struct RecordDecl {
    ts: String,
    rust: Type,
    /// (Rust field name, type text, doc, script key when it is not the camelCase of the name).
    fields: Vec<(Ident, String, String, Option<String>)>,
    /// The `///` comment in front of the declaration.
    doc: String,
}

impl Parse for RecordDecl {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let doc = doc_of(&input.call(syn::Attribute::parse_outer)?);
        let ts: Ident = input.parse()?;
        input.parse::<Token![=]>()?;
        let rust: Type = input.parse()?;
        let content;
        bracketed!(content in input);
        let mut fields = Vec::new();
        while !content.is_empty() {
            let fdoc = doc_of(&content.call(syn::Attribute::parse_outer)?);
            let name: Ident = content.parse()?;
            content.parse::<Token![:]>()?;
            let ty: Type = content.parse()?;
            let key = if content.peek(Token![as]) {
                content.parse::<Token![as]>()?;
                Some(content.parse::<syn::LitStr>()?.value())
            } else {
                None
            };
            let ty_text: String = quote!(#ty).to_string().split_whitespace().collect();
            fields.push((name, ty_text, fdoc, key));
            if content.peek(Token![,]) {
                content.parse::<Token![,]>()?;
            }
        }
        if fields.is_empty() {
            return Err(syn::Error::new(ts.span(), "a record needs fields"));
        }
        Ok(RecordDecl {
            ts: ts.to_string(),
            rust,
            fields,
            doc,
        })
    }
}

struct Args {
    namespace: String,
    quickjs: bool,
    /// Value parameters are declared as `<Value>Like` in the TypeScript.
    like: bool,
    /// (TypeScript name, Rust type name).
    handles: Vec<(String, String)>,
    values: Vec<ValueDecl>,
    enums: Vec<EnumDecl>,
    records: Vec<RecordDecl>,
    /// `aliases(AnimatorController = AnimationController)`: (alias, target).
    aliases: Vec<(String, String)>,
}

fn rust_name_of(ty: &Type) -> Option<String> {
    match ty {
        Type::Path(p) => p.path.segments.last().map(|s| s.ident.to_string()),
        _ => None,
    }
}

fn parse_args(attr: TokenStream2) -> syn::Result<Args> {
    let metas = Punctuated::<Meta, Token![,]>::parse_terminated.parse2(attr)?;
    let mut args = Args {
        namespace: String::new(),
        quickjs: false,
        like: false,
        handles: Vec::new(),
        values: Vec::new(),
        enums: Vec::new(),
        records: Vec::new(),
        aliases: Vec::new(),
    };
    for m in metas {
        match m {
            Meta::NameValue(nv) if nv.path.is_ident("namespace") => match &nv.value {
                Expr::Lit(l) => match &l.lit {
                    Lit::Str(s) => args.namespace = s.value(),
                    _ => return Err(syn::Error::new(nv.value.span(), "a string is expected")),
                },
                _ => return Err(syn::Error::new(nv.value.span(), "a string is expected")),
            },
            Meta::Path(p) if p.is_ident("quickjs") => args.quickjs = true,
            Meta::Path(p) if p.is_ident("like") => args.like = true,
            Meta::List(l) if l.path.is_ident("enums") => {
                let decls =
                    Punctuated::<EnumDecl, Token![,]>::parse_terminated.parse2(l.tokens.clone())?;
                args.enums.extend(decls);
            }
            Meta::List(l) if l.path.is_ident("records") => {
                let decls = Punctuated::<RecordDecl, Token![,]>::parse_terminated
                    .parse2(l.tokens.clone())?;
                args.records.extend(decls);
            }
            Meta::List(l) if l.path.is_ident("aliases") => {
                let pairs = Punctuated::<syn::MetaNameValue, Token![,]>::parse_terminated
                    .parse2(l.tokens.clone())?;
                for nv in pairs {
                    let alias = nv
                        .path
                        .get_ident()
                        .ok_or_else(|| syn::Error::new(nv.path.span(), "an alias name"))?
                        .to_string();
                    let Expr::Path(p) = &nv.value else {
                        return Err(syn::Error::new(nv.value.span(), "the aliased name"));
                    };
                    let target = p
                        .path
                        .get_ident()
                        .ok_or_else(|| syn::Error::new(p.span(), "the aliased name"))?
                        .to_string();
                    args.aliases.push((alias, target));
                }
            }
            Meta::List(l) if l.path.is_ident("handles") => {
                let pairs = Punctuated::<syn::MetaNameValue, Token![,]>::parse_terminated
                    .parse2(l.tokens.clone())?;
                for nv in pairs {
                    let ts = nv
                        .path
                        .get_ident()
                        .ok_or_else(|| syn::Error::new(nv.path.span(), "a type name"))?
                        .to_string();
                    let Expr::Path(p) = &nv.value else {
                        return Err(syn::Error::new(nv.value.span(), "a Rust type name"));
                    };
                    let rust = p
                        .path
                        .segments
                        .last()
                        .ok_or_else(|| syn::Error::new(p.span(), "a Rust type name"))?
                        .ident
                        .to_string();
                    args.handles.push((ts, rust));
                }
            }
            Meta::List(l) if l.path.is_ident("values") => {
                let decls = Punctuated::<ValueDecl, Token![,]>::parse_terminated
                    .parse2(l.tokens.clone())?;
                args.values.extend(decls);
            }
            other => {
                return Err(syn::Error::new(
                    other.span(),
                    "expected `namespace = \"..\"`, `quickjs`, `like`, `handles(..)`, `values(..)`, `enums(..)`, `records(..)` or `aliases(..)`",
                ));
            }
        }
    }
    if args.namespace.is_empty() {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            "`namespace = \"..\"` is required",
        ));
    }
    Ok(args)
}

/// What a parameter or result is.
#[derive(Clone)]
enum Kind {
    Unit,
    Bool,
    F64,
    F32,
    I32,
    U32,
    /// (TypeScript name, the Rust type).
    Handle(String, Box<Type>),
    /// Index into `Args::values`.
    Value(usize),
    /// `&str` (a parameter) or `String` (a result); the flag is true for `String`.
    Str(bool),
    /// `&[f64]`: a script `number[]`, read in place.
    F64s,
    /// `&[String]` (false) or `Vec<String>` (true): a script `string[]`, copied.
    Strs(bool),
    /// `&[V]` for a declared value type (index into `Args::values`): a script `Vector3[]`.
    Values(usize),
    /// `&[H]` for a declared handle type (TypeScript name, the Rust type): a script `GameObject[]`.
    Handles(String, Box<Type>),
    /// `Callback<(A, B)>`: a script closure the engine keeps. The elements are scalar kinds,
    /// strings, or `HandleArg<T>` for a declared handle `T`.
    Callback(Vec<Kind>),
    /// A declared enum (index into `Args::enums`): a script string, the index on the C ABI.
    Enum(usize),
    /// A declared record (index into `Args::records`), a result.
    Record(usize),
    /// `Vec<V>` of a value type, an enum or a record, or `Vec<String>`: a result.
    List(Box<Kind>),
    /// `Option<Enum>`: a name, or `None` for a left-out or unknown one (a parameter).
    OptEnum(usize),
    /// `Option<f64>`, `Option<String>` or `Option<V>` of a value class: `null` for `None`.
    Opt(Box<Kind>),
    /// `Option<R>` of a declared record (index into `Args::records`): a parameter or a result
    /// that travels as JSON text (`null` for `None`).
    OptRecord(usize),
}

fn classify(ty: &Type, args: &Args) -> syn::Result<Kind> {
    if let Type::Tuple(t) = ty
        && t.elems.is_empty()
    {
        return Ok(Kind::Unit);
    }
    if let Type::Reference(r) = ty
        && r.mutability.is_none()
        && rust_name_of(&r.elem).as_deref() == Some("str")
    {
        return Ok(Kind::Str(false));
    }
    if let Type::Reference(r) = ty
        && r.mutability.is_none()
        && let Type::Slice(s) = &*r.elem
    {
        let elem = rust_name_of(&s.elem);
        if elem.as_deref() == Some("f64") {
            return Ok(Kind::F64s);
        }
        if elem.as_deref() == Some("String") {
            return Ok(Kind::Strs(false));
        }
        if let Some(i) = args
            .values
            .iter()
            .position(|v| rust_name_of(&v.rust).as_deref() == elem.as_deref())
        {
            return Ok(Kind::Values(i));
        }
        if let Some((ts, _)) = args
            .handles
            .iter()
            .find(|(_, rust)| Some(rust.as_str()) == elem.as_deref())
        {
            return Ok(Kind::Handles(ts.clone(), Box::new((*s.elem).clone())));
        }
        return Err(syn::Error::new(
            ty.span(),
            "a slice parameter is `&[f64]`, `&[String]` or `&[V]` for a type declared in `values(..)` or `handles(..)`",
        ));
    }
    if let Type::Path(p) = ty
        && let Some(seg) = p.path.segments.last()
        && seg.ident == "Option"
        && let syn::PathArguments::AngleBracketed(ab) = &seg.arguments
        && let Some(syn::GenericArgument::Type(inner)) = ab.args.first()
    {
        return match classify(inner, args)? {
            Kind::Enum(i) => Ok(Kind::OptEnum(i)),
            k @ (Kind::F64 | Kind::Str(true) | Kind::Value(_)) => Ok(Kind::Opt(Box::new(k))),
            Kind::Record(i) => Ok(Kind::OptRecord(i)),
            _ => Err(syn::Error::new(
                inner.span(),
                "an `Option` holds an enum, `f64`, `String` or a type declared in `values(..)` or `records(..)`",
            )),
        };
    }
    if let Type::Path(p) = ty
        && let Some(seg) = p.path.segments.last()
        && seg.ident == "Vec"
        && let syn::PathArguments::AngleBracketed(ab) = &seg.arguments
        && let Some(syn::GenericArgument::Type(inner)) = ab.args.first()
    {
        let k = classify(inner, args)?;
        if !matches!(
            k,
            Kind::Value(_)
                | Kind::Record(_)
                | Kind::Str(true)
                | Kind::Enum(_)
                | Kind::Handle(..)
                | Kind::F64
        ) {
            return Err(syn::Error::new(
                inner.span(),
                "a `Vec` result holds `f64`, a `String` or a type declared in `values(..)`, `handles(..)`, `enums(..)` or `records(..)`",
            ));
        }
        return Ok(Kind::List(Box::new(k)));
    }
    if let Type::Path(p) = ty
        && let Some(seg) = p.path.segments.last()
        && seg.ident == "Callback"
        && let syn::PathArguments::AngleBracketed(ab) = &seg.arguments
        && let Some(syn::GenericArgument::Type(inner)) = ab.args.first()
    {
        let Type::Tuple(t) = inner else {
            return Err(syn::Error::new(
                inner.span(),
                "a `Callback` takes a tuple of arguments: `Callback<(f64, String)>`",
            ));
        };
        let mut kinds = Vec::new();
        for e in &t.elems {
            let k = if let Type::Path(ep) = e
                && let Some(es) = ep.path.segments.last()
                && es.ident == "HandleArg"
                && let syn::PathArguments::AngleBracketed(eab) = &es.arguments
                && let Some(syn::GenericArgument::Type(h)) = eab.args.first()
            {
                classify(h, args)?
            } else {
                classify(e, args)?
            };
            if !matches!(
                k,
                Kind::Bool | Kind::F64 | Kind::I32 | Kind::U32 | Kind::Str(true) | Kind::Handle(..)
            ) {
                return Err(syn::Error::new(
                    e.span(),
                    "a callback argument is f64, i32, u32, bool, String or HandleArg<T>",
                ));
            }
            kinds.push(k);
        }
        return Ok(Kind::Callback(kinds));
    }
    let Some(name) = rust_name_of(ty) else {
        return Err(syn::Error::new(
            ty.span(),
            "unsupported type in an iron_api function",
        ));
    };
    Ok(match name.as_str() {
        "bool" => Kind::Bool,
        "f64" => Kind::F64,
        "f32" => Kind::F32,
        "i32" => Kind::I32,
        "u32" => Kind::U32,
        "String" => Kind::Str(true),
        other => {
            if let Some((ts, _)) = args.handles.iter().find(|(_, rust)| rust == other) {
                Kind::Handle(ts.clone(), Box::new(ty.clone()))
            } else if let Some(i) = args
                .values
                .iter()
                .position(|v| rust_name_of(&v.rust).as_deref() == Some(other))
            {
                Kind::Value(i)
            } else if let Some(i) = args
                .enums
                .iter()
                .position(|v| rust_name_of(&v.rust).as_deref() == Some(other))
            {
                Kind::Enum(i)
            } else if let Some(i) = args
                .records
                .iter()
                .position(|v| rust_name_of(&v.rust).as_deref() == Some(other))
            {
                Kind::Record(i)
            } else {
                return Err(syn::Error::new(
                    ty.span(),
                    format!(
                        "`{other}` cannot cross the script boundary yet: use f64, f32, i32, u32, bool, () \
                         or a type declared in `handles(..)` or `values(..)`"
                    ),
                ));
            }
        }
    })
}

fn scalar_ty(name: &str) -> TokenStream2 {
    match name {
        "f32" => quote!(f32),
        "f64" => quote!(f64),
        "i32" => quote!(i32),
        _ => quote!(u32),
    }
}

fn scalar_manifest(name: &str) -> TokenStream2 {
    match name {
        "f32" => quote!(::iron_api::Ty::F32),
        "f64" => quote!(::iron_api::Ty::F64),
        "i32" => quote!(::iron_api::Ty::I32),
        _ => quote!(::iron_api::Ty::U32),
    }
}

/// The type of a property for pairing a getter with its setter: a list result and the array
/// parameter that takes it are the same type.
fn pair_ty(k: &Kind, args: &Args) -> String {
    let elem = |e: &Kind| manifest_ty(e, args).to_string();
    match k {
        Kind::List(inner) => format!("[{}]", elem(inner)),
        Kind::F64s => format!("[{}]", elem(&Kind::F64)),
        Kind::Strs(_) => format!("[{}]", elem(&Kind::Str(true))),
        Kind::Values(i) => format!("[{}]", elem(&Kind::Value(*i))),
        Kind::Handles(ts, rust) => format!("[{}]", elem(&Kind::Handle(ts.clone(), rust.clone()))),
        other => elem(other),
    }
}

/// The manifest `Ty` as tokens.
fn manifest_ty(k: &Kind, args: &Args) -> TokenStream2 {
    match k {
        Kind::Unit => quote!(::iron_api::Ty::Void),
        Kind::Bool => quote!(::iron_api::Ty::Bool),
        Kind::F64 => quote!(::iron_api::Ty::F64),
        Kind::F32 => quote!(::iron_api::Ty::F32),
        Kind::I32 => quote!(::iron_api::Ty::I32),
        Kind::U32 => quote!(::iron_api::Ty::U32),
        Kind::Str(_) => quote!(::iron_api::Ty::Str),
        Kind::Handle(ts, _) => quote!(::iron_api::Ty::Handle(#ts)),
        Kind::Value(i) => {
            let ts = &args.values[*i].ts;
            quote!(::iron_api::Ty::Value(#ts))
        }
        Kind::F64s => quote!(::iron_api::Ty::F64s),
        Kind::Strs(_) => quote!(::iron_api::Ty::Strs),
        Kind::Callback(ks) => {
            let names: Vec<String> = ks
                .iter()
                .map(|k| match k {
                    Kind::Bool => "bool".to_string(),
                    Kind::Str(_) => "string".to_string(),
                    Kind::Handle(ts, _) => format!("handle:{ts}"),
                    _ => "f64".to_string(),
                })
                .collect();
            let sig = format!("fn({}):void", names.join(","));
            let params: Vec<String> = ks
                .iter()
                .enumerate()
                .map(|(i, k)| {
                    let t = match k {
                        Kind::Bool => "boolean".to_string(),
                        Kind::Str(_) => "string".to_string(),
                        Kind::Handle(ts, _) => ts.clone(),
                        _ => "number".to_string(),
                    };
                    format!("{}: {t}", (b'a' + i as u8) as char)
                })
                .collect();
            let ts = format!("({}) => void", params.join(", "));
            quote!(::iron_api::Ty::Callback(#sig, #ts))
        }
        Kind::Values(i) => {
            let ts = &args.values[*i].ts;
            let arr = format!("{ts}[]");
            quote!(::iron_api::Ty::Values(#ts, #arr))
        }
        Kind::Handles(ts, _) => {
            let arr = format!("{ts}[]");
            quote!(::iron_api::Ty::Handles(#ts, #arr))
        }
        Kind::Enum(i) | Kind::OptEnum(i) => {
            let ts = &args.enums[*i].ts;
            quote!(::iron_api::Ty::Enum(#ts))
        }
        Kind::OptRecord(i) => {
            let name = format!("record:{}", args.records[*i].ts);
            let ts = format!("{} | null", args.records[*i].ts);
            quote!(::iron_api::Ty::Opt(#name, #ts))
        }
        Kind::Opt(inner) => {
            let (name, ts) = match &**inner {
                Kind::F64 => ("f64".to_string(), "number | null".to_string()),
                Kind::Str(_) => ("string".to_string(), "string | null".to_string()),
                Kind::Value(i) => (
                    format!("value:{}", args.values[*i].ts),
                    format!("{} | null", args.values[*i].ts),
                ),
                _ => unreachable!("classify only builds Opt of f64, strings and values"),
            };
            quote!(::iron_api::Ty::Opt(#name, #ts))
        }
        Kind::Record(i) => {
            let ts = &args.records[*i].ts;
            quote!(::iron_api::Ty::Record(#ts))
        }
        Kind::List(inner) => {
            let (name, ts) = match &**inner {
                Kind::Value(i) => (
                    format!("value:{}", args.values[*i].ts),
                    args.values[*i].ts.clone(),
                ),
                Kind::Record(i) => (
                    format!("record:{}", args.records[*i].ts),
                    args.records[*i].ts.clone(),
                ),
                Kind::Str(_) => ("string".to_string(), "string".to_string()),
                Kind::Enum(i) => (
                    format!("enum:{}", args.enums[*i].ts),
                    args.enums[*i].ts.clone(),
                ),
                Kind::Handle(ts, _) => (format!("handle:{ts}"), ts.clone()),
                Kind::F64 => ("f64".to_string(), "number".to_string()),
                _ => {
                    unreachable!("classify only builds lists of strings, values, enums and records")
                }
            };
            let arr = format!("{ts}[]");
            quote!(::iron_api::Ty::List(#name, #arr))
        }
    }
}

/// The C ABI type of a value (a value type is passed as its fields, see `shim_param`).
fn abi_ty(k: &Kind) -> TokenStream2 {
    match k {
        Kind::Unit | Kind::Value(_) | Kind::Record(_) => quote!(()),
        Kind::Callback(_) => quote!(*const u8),
        Kind::Enum(_) | Kind::OptEnum(_) => quote!(i32),
        Kind::OptRecord(_) => quote!(*const u8),
        Kind::List(_) => quote!(u32),
        Kind::Opt(inner) => match &**inner {
            Kind::F64 => quote!(f64),
            Kind::Str(_) => quote!(*const u8),
            _ => quote!(()),
        },
        Kind::Bool => quote!(bool),
        Kind::F64 => quote!(f64),
        Kind::F32 => quote!(f32),
        Kind::I32 => quote!(i32),
        Kind::U32 => quote!(u32),
        Kind::Handle(..) => quote!(u64),
        Kind::Str(_) | Kind::F64s | Kind::Strs(_) | Kind::Values(_) | Kind::Handles(..) => {
            quote!(*const u8)
        }
    }
}

/// The type a QuickJS function takes or returns for it (handles travel as numbers).
fn js_ty(k: &Kind) -> TokenStream2 {
    match k {
        Kind::Handle(..) => quote!(::iron_api::rquickjs::Value<'js>),
        other => abi_ty(other),
    }
}

/// Turns the ABI value `x` into the Rust parameter type.
fn from_abi(k: &Kind, x: TokenStream2) -> TokenStream2 {
    match k {
        Kind::Handle(_, rust) => quote!(<#rust as ::iron_api::Handle>::from_raw(#x)),
        _ => x,
    }
}

/// The type a QuickJS function takes for a parameter.
fn js_param_ty(k: &Kind) -> TokenStream2 {
    match k {
        Kind::Str(_) | Kind::Enum(_) | Kind::OptEnum(_) => quote!(::std::string::String),
        Kind::Value(_) => quote!(::iron_api::rquickjs::Object<'js>),
        Kind::Opt(inner) => {
            let t = js_param_ty(inner);
            quote!(::std::option::Option<#t>)
        }
        Kind::OptRecord(_) => quote!(::std::option::Option<::iron_api::rquickjs::Value<'js>>),
        Kind::F64s => quote!(::std::vec::Vec<f64>),
        Kind::Strs(_) => quote!(::std::vec::Vec<::std::string::String>),
        Kind::Callback(_) => quote!(::iron_api::rquickjs::Function<'js>),
        Kind::Values(_) => quote!(::std::vec::Vec<::iron_api::rquickjs::Object<'js>>),
        Kind::Handles(..) => quote!(::std::vec::Vec<::iron_api::rquickjs::Value<'js>>),
        other => js_ty(other),
    }
}

/// One entry of `#[optional(..)]`: a name, and perhaps `= value`.
struct OptItem {
    name: Ident,
    default: Option<f64>,
}

impl syn::parse::Parse for OptItem {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let name: Ident = input.parse()?;
        let default = if input.peek(Token![=]) {
            input.parse::<Token![=]>()?;
            let e: Expr = input.parse()?;
            let (neg, lit) = match &e {
                Expr::Unary(u) if matches!(u.op, syn::UnOp::Neg(_)) => (true, &*u.expr),
                other => (false, other),
            };
            let Expr::Lit(l) = lit else {
                return Err(syn::Error::new(e.span(), "a number or true / false"));
            };
            let v = match &l.lit {
                Lit::Bool(b) => f64::from(u8::from(b.value)),
                Lit::Int(i) => i.base10_parse::<f64>()?,
                Lit::Float(f) => f.base10_parse::<f64>()?,
                other => return Err(syn::Error::new(other.span(), "a number or true / false")),
            };
            Some(if neg { -v } else { v })
        } else {
            None
        };
        Ok(OptItem { name, default })
    }
}

/// The value a left-out optional parameter has when it was given a default.
fn js_default_value(k: &Kind, v: f64) -> TokenStream2 {
    match k {
        Kind::Bool => {
            let b = v != 0.0;
            quote!(#b)
        }
        Kind::F64 => quote!(#v),
        Kind::F32 => {
            let v = v as f32;
            quote!(#v)
        }
        Kind::I32 | Kind::U32 => {
            let v = v as i64;
            quote!(#v as _)
        }
        _ => quote!(compile_error!("this parameter type cannot have a default")),
    }
}

/// The value a left-out optional parameter has (what a compiled call passes).
fn js_default(k: &Kind) -> TokenStream2 {
    match k {
        Kind::Bool => quote!(false),
        Kind::F64 => quote!(f64::NAN),
        Kind::F32 => quote!(f32::NAN),
        Kind::I32 | Kind::U32 => quote!(0),
        Kind::Handle(..) => quote!(::iron_api::rquickjs::Value::new_null(ctx.clone())),
        Kind::F64s | Kind::Strs(_) | Kind::Values(_) | Kind::Handles(..) => {
            quote!(::std::vec::Vec::new())
        }
        Kind::Str(_) | Kind::Enum(_) | Kind::OptEnum(_) => quote!(::std::string::String::new()),
        _ => quote!(compile_error!("this parameter type cannot be optional")),
    }
}

/// The type a QuickJS function returns for a result.
fn js_ret_ty(k: &Kind) -> TokenStream2 {
    match k {
        Kind::Str(_) | Kind::Enum(_) => quote!(::std::string::String),
        Kind::Value(_) | Kind::Record(_) => quote!(::iron_api::rquickjs::Object<'js>),
        Kind::List(_) => quote!(::iron_api::rquickjs::Array<'js>),
        Kind::Opt(inner) => {
            let t = js_ret_ty(inner);
            quote!(::std::option::Option<#t>)
        }
        Kind::OptRecord(_) => quote!(::iron_api::rquickjs::Value<'js>),
        other => js_ty(other),
    }
}

/// The Rust argument built from the JS parameter `x` (a value reads its fields from the object).
fn js_arg(k: &Kind, x: TokenStream2, args: &Args, opt: bool) -> TokenStream2 {
    match k {
        // A left-out value is NaN in every field (what a compiled call passes).
        Kind::Value(i) if opt => {
            let v = &args.values[*i];
            let n = v.fields.len();
            let rust = &v.rust;
            let names: Vec<&String> = v.fields.iter().map(|(n, _)| n).collect();
            quote! {
                <#rust as ::iron_api::ValueType<#n>>::from_parts(match &#x {
                    Some(o) => ::iron_api::qjs::value_parts(&ctx, o, [ #( #names ),* ])?,
                    None => [f64::NAN; #n],
                })
            }
        }
        Kind::Str(false) => quote!(&#x),
        Kind::Str(true) => x,
        // A name; an empty one (a left-out parameter) is `None`.
        Kind::OptEnum(i) => {
            let e = &args.enums[*i];
            let parse = enum_fn(&e.ts, "parse");
            let ts = &e.ts;
            quote! {
                {
                    let s: ::std::string::String = #x;
                    if s.is_empty() {
                        ::std::option::Option::None
                    } else {
                        match #parse(&s) {
                            ::std::option::Option::Some(v) => ::std::option::Option::Some(v),
                            ::std::option::Option::None => {
                                return Err(::iron_api::rquickjs::Exception::throw_type(
                                    &ctx,
                                    &::std::format!("`{}` is not a {}", s, #ts),
                                ));
                            }
                        }
                    }
                }
            }
        }
        Kind::OptRecord(i) => {
            let from = format_ident!("__from_json_{}", args.records[*i].ts);
            quote! {
                match &#x {
                    Some(v) if !v.is_null() && !v.is_undefined() => {
                        match ctx.json_stringify(v.clone())? {
                            Some(t) => ::iron_api::json_record(&t.to_string()?, #from),
                            None => None,
                        }
                    }
                    _ => None,
                }
            }
        }
        Kind::Opt(inner) => match &**inner {
            Kind::Value(i) => {
                let v = &args.values[*i];
                let n = v.fields.len();
                let rust = &v.rust;
                let names: Vec<&String> = v.fields.iter().map(|(n, _)| n).collect();
                quote! {
                    match &#x {
                        Some(o) => Some(<#rust as ::iron_api::ValueType<#n>>::from_parts(
                            ::iron_api::qjs::value_parts(&ctx, o, [ #( #names ),* ])?
                        )),
                        None => None,
                    }
                }
            }
            _ => x,
        },
        // A name; an empty one (a left-out parameter) is the first value.
        Kind::Enum(i) => {
            let e = &args.enums[*i];
            let parse = enum_fn(&e.ts, "parse");
            let first = enum_first(e);
            let ts = &e.ts;
            quote! {
                {
                    let s: ::std::string::String = #x;
                    if s.is_empty() {
                        #first
                    } else {
                        match #parse(&s) {
                            ::std::option::Option::Some(v) => v,
                            ::std::option::Option::None => {
                                return Err(::iron_api::rquickjs::Exception::throw_type(
                                    &ctx,
                                    &::std::format!("`{}` is not a {}", s, #ts),
                                ));
                            }
                        }
                    }
                }
            }
        }
        Kind::Value(i) => {
            let v = &args.values[*i];
            let n = v.fields.len();
            let rust = &v.rust;
            let names: Vec<&String> = v.fields.iter().map(|(n, _)| n).collect();
            quote! {
                <#rust as ::iron_api::ValueType<#n>>::from_parts(
                    ::iron_api::qjs::value_parts(&ctx, &#x, [ #( #names ),* ])?
                )
            }
        }
        Kind::Callback(ks) => {
            let tys: Vec<TokenStream2> = ks
                .iter()
                .map(|k| match k {
                    Kind::Handle(_, rust) => quote!(::iron_api::callback::HandleArg<#rust>),
                    Kind::Str(_) => quote!(::std::string::String),
                    Kind::Bool => quote!(bool),
                    Kind::I32 => quote!(i32),
                    Kind::U32 => quote!(u32),
                    _ => quote!(f64),
                })
                .collect();
            let names: Vec<Ident> = (0..ks.len()).map(|i| format_ident!("a{i}")).collect();
            let pushes: Vec<TokenStream2> = ks
                .iter()
                .zip(&names)
                .map(|(k, n)| match k {
                    Kind::Handle(ts, _) => quote! {
                        args.push_arg(::iron_api::qjs::handle_new(
                            ctx, #ts, ::iron_api::Handle::to_raw(#n.0),
                        )?)?;
                    },
                    Kind::Str(_) | Kind::Bool => quote!(args.push_arg(#n)?;),
                    _ => quote!(args.push_arg(#n as f64)?;),
                })
                .collect();
            let count = ks.len();
            quote! {
                {
                    let kept = ::iron_api::qjs::keep_function(#x);
                    ::iron_api::Callback::<( #( #tys, )* )>::from_fn(move |( #( #names, )* )| {
                        ::iron_api::qjs::call_function(&kept, #count, |ctx, args| {
                            #( #pushes )*
                            Ok(())
                        });
                    })
                }
            }
        }
        Kind::F64s => quote!(&#x),
        Kind::Strs(false) => quote!(&#x),
        Kind::Strs(true) => x,
        Kind::Values(i) => {
            let v = &args.values[*i];
            let n = v.fields.len();
            let rust = &v.rust;
            let names: Vec<&String> = v.fields.iter().map(|(n, _)| n).collect();
            quote! {
                &{
                    let mut items = ::std::vec::Vec::with_capacity(#x.len());
                    for o in &#x {
                        items.push(<#rust as ::iron_api::ValueType<#n>>::from_parts(
                            ::iron_api::qjs::value_parts(&ctx, o, [ #( #names ),* ])?
                        ));
                    }
                    items
                }
            }
        }
        Kind::Handles(ts, rust) => quote! {
            &{
                let mut items = ::std::vec::Vec::with_capacity(#x.len());
                for o in &#x {
                    items.push(<#rust as ::iron_api::Handle>::from_raw(
                        ::iron_api::qjs::handle_arg(&ctx, o, #ts)?
                    ));
                }
                items
            }
        },
        Kind::Handle(ts, rust) => quote! {
            <#rust as ::iron_api::Handle>::from_raw(::iron_api::qjs::handle_arg(&ctx, &#x, #ts)?)
        },
        other => from_abi(other, x),
    }
}

/// The body of a QuickJS function: the call, as the JS result.
fn js_result(k: &Kind, call: TokenStream2, args: &Args) -> TokenStream2 {
    match k {
        Kind::Value(i) => {
            let v = &args.values[*i];
            let n = v.fields.len();
            let rust = &v.rust;
            let names: Vec<&String> = v.fields.iter().map(|(n, _)| n).collect();
            let idx: Vec<usize> = (0..n).collect();
            let ts = &v.ts;
            let _ = &names;
            // The marker first says "these are the fields", not constructor arguments.
            quote! {
                let parts = <#rust as ::iron_api::ValueType<#n>>::to_parts(#call);
                let class: ::iron_api::rquickjs::function::Constructor =
                    ctx.globals().get(#ts)?;
                let raw: ::iron_api::rquickjs::Value = ctx.globals().get("__iron_raw")?;
                let mut args = ::iron_api::rquickjs::function::Args::new(ctx.clone(), 1 + #n);
                args.push_arg(raw)?;
                #( args.push_arg(parts[#idx])?; )*
                class.construct_args(args)
            }
        }
        Kind::Handle(ts, _) => {
            quote!(::iron_api::qjs::handle_new(&ctx, #ts, ::iron_api::Handle::to_raw(#call)))
        }
        Kind::Enum(i) => {
            let name = enum_fn(&args.enums[*i].ts, "name");
            quote!(Ok(#name(#call).to_string()))
        }
        Kind::Opt(inner) => match &**inner {
            Kind::Value(i) => {
                let build = format_ident!("__js_value_{}", args.values[*i].ts);
                quote! {
                    match #call {
                        Some(v) => Ok(Some(#build(&ctx, v)?)),
                        None => Ok(None),
                    }
                }
            }
            _ => quote!(Ok(#call)),
        },
        Kind::OptRecord(i) => {
            let build = format_ident!("__js_rec_{}", args.records[*i].ts);
            quote! {
                match #call {
                    Some(r) => #build(&ctx, &r).map(|o| o.into_value()),
                    None => Ok(::iron_api::rquickjs::Value::new_null(ctx.clone())),
                }
            }
        }
        Kind::Record(i) => {
            let build = format_ident!("__js_rec_{}", args.records[*i].ts);
            quote! {
                let r = #call;
                #build(&ctx, &r)
            }
        }
        Kind::List(inner) => {
            let elem = match &**inner {
                Kind::Value(i) => {
                    let build = format_ident!("__js_value_{}", args.values[*i].ts);
                    quote!(#build(&ctx, *item)?)
                }
                Kind::Record(i) => {
                    let build = format_ident!("__js_rec_{}", args.records[*i].ts);
                    quote!(#build(&ctx, item)?)
                }
                Kind::Str(_) => quote!(item.as_str()),
                Kind::Enum(i) => {
                    let name = enum_fn(&args.enums[*i].ts, "name");
                    quote!(#name(*item))
                }
                Kind::Handle(ts, _) => {
                    quote!(::iron_api::qjs::handle_new(&ctx, #ts, ::iron_api::Handle::to_raw(*item))?)
                }
                Kind::F64 => quote!(*item),
                _ => {
                    unreachable!("classify only builds lists of strings, values, enums and records")
                }
            };
            quote! {
                let items = #call;
                let arr = ::iron_api::rquickjs::Array::new(ctx.clone())?;
                for (n, item) in items.iter().enumerate() {
                    arr.set(n, #elem)?;
                }
                Ok(arr)
            }
        }
        _ => quote!(Ok(#call)),
    }
}

/// `tangent_mode` for `TangentMode`.
fn snake(s: &str) -> String {
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                out.push('_');
            }
            out.extend(c.to_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

/// The helper function `<snake>_<what>` generated for an enum (`parse`, `name`, `index`,
/// `from_index`).
fn enum_fn(ts: &str, what: &str) -> Ident {
    format_ident!("{}_{}", snake(ts), what)
}

/// The first variant of an enum: what a left-out or unknown value is.
fn enum_first(e: &EnumDecl) -> TokenStream2 {
    let rust = &e.rust;
    let v = &e.variants[0];
    quote!(#rust::#v)
}

fn to_abi(k: &Kind, x: TokenStream2, args: &Args) -> TokenStream2 {
    match k {
        Kind::Handle(..) => quote!(::iron_api::Handle::to_raw(#x)),
        Kind::Enum(i) => {
            let index = enum_fn(&args.enums[*i].ts, "index");
            quote!(#index(#x))
        }
        Kind::Opt(inner) if matches!(**inner, Kind::F64) => quote!(#x.unwrap_or(f64::NAN)),
        _ => x,
    }
}

fn camel(s: &str) -> String {
    let s = s.strip_prefix("r#").unwrap_or(s);
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

/// The attributes this macro reads from a function (they are removed from the output).
fn is_ours(a: &syn::Attribute) -> bool {
    [
        "property",
        "optional",
        "method",
        "getter",
        "js_name",
        "options",
        "rest",
        "nullable",
        "ts_type",
        "like",
        "subscription",
        "promise",
    ]
    .iter()
    .any(|n| a.path().is_ident(n))
}

/// `#[js_name = "graphicsDeviceVendorID"]`: the script name when `snake_case` to `camelCase` does
/// not spell it.
fn js_name_attr(attrs: &[syn::Attribute]) -> Option<String> {
    attrs.iter().find_map(|a| {
        if a.path().is_ident("js_name")
            && let Meta::NameValue(nv) = &a.meta
            && let Expr::Lit(l) = &nv.value
            && let Lit::Str(s) = &l.lit
        {
            Some(s.value())
        } else {
            None
        }
    })
}

/// `#[ts_type = "\"low\" | \"high\""]`: the TypeScript type of a property when the Rust type
/// (`String`) is wider (a string-literal union).
fn ts_type_attr(attrs: &[syn::Attribute]) -> Option<String> {
    attrs.iter().find_map(|a| {
        if a.path().is_ident("ts_type")
            && let Meta::NameValue(nv) = &a.meta
            && let Expr::Lit(l) = &nv.value
            && let Lit::Str(s) = &l.lit
        {
            Some(s.value())
        } else {
            None
        }
    })
}

fn doc_of(attrs: &[syn::Attribute]) -> String {
    let mut lines = Vec::new();
    for a in attrs {
        if a.path().is_ident("doc")
            && let Meta::NameValue(nv) = &a.meta
            && let Expr::Lit(l) = &nv.value
            && let Lit::Str(s) = &l.lit
        {
            lines.push(s.value().trim().to_string());
        }
    }
    lines.join("\n")
}

/// How a function is called from a script.
#[derive(Clone, Copy, PartialEq)]
enum Tag {
    Static,
    Method,
    Get,
    Set,
}

struct Api {
    /// The Rust function.
    name: Ident,
    doc: String,
    /// The script name (for a property, the property's).
    js_name: String,
    /// camelCase of the Rust name: the symbol's last part.
    stem: String,
    /// The handle class (TypeScript name, Rust type) of an `impl` function.
    class: Option<(String, Type)>,
    tag: Tag,
    /// An `impl` function that takes `self` (a static of the class has none).
    receiver: bool,
    /// A method of a value class: the receiver (first parameter) travels as the value's fields.
    value_recv: Option<(Ident, Kind)>,
    /// Not including the receiver.
    params: Vec<(Ident, Kind)>,
    /// Names of the trailing parameters a call may leave out (`#[optional(a, b = 1.5)]`).
    optional: Vec<String>,
    /// The defaults given (`b = 1.5`; a flag is 0 or 1).
    defaults: Vec<(String, f64)>,
    /// `#[options]`: QuickJS may pass the optional parameters as one object, by camelCase name.
    options: bool,
    /// `#[options(after = p)]`: the object comes right after parameter `p` and fills the ones
    /// after it (`new Spline(points, { closed: true })`).
    options_after: Option<String>,
    /// `#[options(name = "TextureOptions")]`: the declared type of the options object.
    options_name: Option<String>,
    /// `#[options(.., only)]`: only the options form is declared.
    options_only: bool,
    /// `/// name: text` lines of the doc: the doc of a parameter (the key of an options object).
    param_docs: Vec<(String, String)>,
    /// `#[rest(p)]`: the last parameter takes every remaining argument.
    rest: Option<String>,
    /// `#[nullable]`: the TypeScript type of a handle result (or of the setter's value) is `T | null`.
    nullable: bool,
    /// `#[nullable(undefined)]`: declared `T | undefined` instead of `T | null`.
    undefined: bool,
    /// `#[ts_type = ".."]`: the declared TypeScript type of a property (a string-literal union).
    ts_type: Option<String>,
    /// `#[like]`: the value parameters are declared `<Value>Like`.
    like: bool,
    /// `#[subscription(off = "off_x", ts = "..")]`: the function takes a callback and returns a
    /// token; scripts get a function that stops the listener (`off_x(token)`). (Rust name of the
    /// stopping function, the declared TypeScript type of the callback when it is not the
    /// callback's own.)
    subscription: Option<(String, Option<String>)>,
    /// `#[promise]`: the last two parameters are the resolve and reject callbacks; scripts call it
    /// without them and get a `Promise` (resolved with the first callback's argument).
    promise: bool,
    ret: Kind,
}

fn api_of(
    sig: &syn::Signature,
    attrs: &[syn::Attribute],
    args: &Args,
    class: Option<(String, Type)>,
) -> syn::Result<Api> {
    if sig.asyncness.is_some() || !sig.generics.params.is_empty() {
        return Err(syn::Error::new(
            sig.span(),
            "an iron_api function cannot be async or generic",
        ));
    }
    let mut has_receiver = false;
    let mut params = Vec::new();
    for a in &sig.inputs {
        match a {
            FnArg::Receiver(r) => {
                if class.is_none() || r.reference.is_some() || r.mutability.is_some() {
                    return Err(syn::Error::new(
                        r.span(),
                        "a class function takes `self` by value (the handle is Copy)",
                    ));
                }
                has_receiver = true;
            }
            FnArg::Typed(pt) => {
                let Pat::Ident(id) = &*pt.pat else {
                    return Err(syn::Error::new(
                        pt.pat.span(),
                        "a plain parameter name is expected",
                    ));
                };
                let k = match classify(&pt.ty, args)? {
                    // `Vec<String>` as a parameter is a `string[]` the function owns.
                    Kind::List(inner) if matches!(*inner, Kind::Str(true)) => Kind::Strs(true),
                    Kind::Opt(inner) if matches!(*inner, Kind::Str(_)) => {
                        return Err(syn::Error::new(
                            pt.ty.span(),
                            "`Option<String>` is only a result (a parameter that may be left out is `#[optional(..)]`)",
                        ));
                    }
                    k => k,
                };
                params.push((id.ident.clone(), k));
            }
        }
    }
    let ret = match &sig.output {
        ReturnType::Default => Kind::Unit,
        ReturnType::Type(_, t) => classify(t, args)?,
    };
    let property = attrs.iter().any(|a| a.path().is_ident("property"));
    let tag = match (has_receiver, property) {
        // A namespace property: `#[property] pub fn color() -> Color` (and `set_color(v)`).
        // A static property of a handle class: `impl Window { #[property] pub fn main() -> Window }`.
        (false, true) if params.is_empty() => Tag::Get,
        (_, true) if !has_receiver || !params.is_empty() => {
            return Err(syn::Error::new(
                sig.span(),
                "#[property] marks a getter: `fn name(self) -> T`",
            ));
        }
        (true, true) => Tag::Get,
        (true, false) => Tag::Method,
        (false, _) => Tag::Static,
    };
    let stem = camel(sig.ident.to_string().trim_start_matches("r#"));
    let mut optional = Vec::new();
    let mut defaults = Vec::new();
    for at in attrs.iter().filter(|a| a.path().is_ident("optional")) {
        let items = at.parse_args_with(Punctuated::<OptItem, Token![,]>::parse_terminated)?;
        for it in items {
            if !params.iter().any(|(p, _)| *p == it.name) {
                return Err(syn::Error::new(it.name.span(), "no such parameter"));
            }
            optional.push(it.name.to_string());
            if let Some(d) = it.default {
                defaults.push((it.name.to_string(), d));
            }
        }
    }
    // Only trailing parameters can be left out (a promise's resolve and reject callbacks follow).
    let promise = attrs.iter().any(|a| a.path().is_ident("promise"));
    let user_len = params.len() - if promise { 2.min(params.len()) } else { 0 };
    let first_opt = params
        .iter()
        .position(|(p, _)| optional.contains(&p.to_string()));
    if let Some(i) = first_opt
        && params[i..user_len.max(i)]
            .iter()
            .any(|(p, _)| !optional.contains(&p.to_string()))
    {
        return Err(syn::Error::new(
            sig.span(),
            "#[optional(..)] parameters must be the last ones",
        ));
    }
    let options = attrs.iter().any(|a| a.path().is_ident("options"));
    let mut options_after = None;
    let mut options_name = None;
    let mut options_only = false;
    for at in attrs.iter().filter(|a| a.path().is_ident("options")) {
        if let Meta::List(l) = &at.meta {
            let metas = Punctuated::<Meta, Token![,]>::parse_terminated.parse2(l.tokens.clone())?;
            for m in metas {
                match m {
                    Meta::Path(p) if p.is_ident("only") => options_only = true,
                    Meta::NameValue(nv) if nv.path.is_ident("after") => {
                        let Expr::Path(p) = &nv.value else {
                            return Err(syn::Error::new(nv.value.span(), "a parameter name"));
                        };
                        let name = p
                            .path
                            .get_ident()
                            .map(|i| i.to_string())
                            .unwrap_or_default();
                        if !params.iter().any(|(p, _)| *p == name) {
                            return Err(syn::Error::new(nv.value.span(), "no such parameter"));
                        }
                        options_after = Some(name);
                    }
                    Meta::NameValue(nv) if nv.path.is_ident("name") => {
                        let Expr::Lit(l) = &nv.value else {
                            return Err(syn::Error::new(nv.value.span(), "a type name string"));
                        };
                        let Lit::Str(s) = &l.lit else {
                            return Err(syn::Error::new(nv.value.span(), "a type name string"));
                        };
                        options_name = Some(s.value());
                    }
                    other => {
                        return Err(syn::Error::new(
                            other.span(),
                            "expected `after = param`, `name = \"Type\"` or `only`",
                        ));
                    }
                }
            }
        }
    }
    // `/// title: The title bar text.`: the doc of a parameter (an option key), taken out of the
    // function's own doc.
    let mut doc = doc_of(attrs);
    let mut param_docs = Vec::new();
    if options {
        let mut kept = Vec::new();
        for line in doc.lines() {
            if let Some((name, text)) = line.split_once(": ")
                && params.iter().any(|(p, _)| p == name.trim())
            {
                param_docs.push((name.trim().to_string(), text.trim().to_string()));
            } else {
                kept.push(line);
            }
        }
        doc = kept.join("\n");
    }
    let mut rest = None;
    for at in attrs.iter().filter(|a| a.path().is_ident("rest")) {
        let id: Ident = at.parse_args()?;
        if params.last().is_none_or(|(p, _)| *p != id) {
            return Err(syn::Error::new(
                id.span(),
                "#[rest(..)] names the last parameter",
            ));
        }
        rest = Some(id.to_string());
    }
    if options && optional.is_empty() {
        return Err(syn::Error::new(
            sig.span(),
            "#[options] needs #[optional(..)] parameters to read from the object",
        ));
    }
    if options
        && params
            .iter()
            .any(|(p, k)| optional.contains(&p.to_string()) && matches!(k, Kind::Value(_)))
    {
        return Err(syn::Error::new(
            sig.span(),
            "#[options] cannot have optional value-class parameters (a value is itself a plain object)",
        ));
    }
    if promise {
        let ok = params.len() >= 2
            && matches!(&params[params.len() - 2].1, Kind::Callback(ks) if ks.len() <= 1)
            && matches!(&params[params.len() - 1].1, Kind::Callback(ks) if matches!(ks.as_slice(), [Kind::Str(_)]))
            && matches!(ret, Kind::Unit);
        if !ok {
            return Err(syn::Error::new(
                sig.span(),
                "a promise function ends with `resolve: Callback<(T,)>` and `reject: Callback<(String,)>` and returns nothing",
            ));
        }
    }
    let mut subscription = None;
    for at in attrs.iter().filter(|a| a.path().is_ident("subscription")) {
        let mut off = None;
        let mut ts = None;
        let metas = at.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?;
        for m in metas {
            match m {
                Meta::NameValue(nv) if nv.path.is_ident("off") => {
                    let Expr::Lit(l) = &nv.value else {
                        return Err(syn::Error::new(
                            nv.value.span(),
                            "the name of the function that stops it",
                        ));
                    };
                    let Lit::Str(s) = &l.lit else {
                        return Err(syn::Error::new(
                            nv.value.span(),
                            "the name of the function that stops it",
                        ));
                    };
                    off = Some(s.value());
                }
                Meta::NameValue(nv) if nv.path.is_ident("ts") => {
                    let Expr::Lit(l) = &nv.value else {
                        return Err(syn::Error::new(nv.value.span(), "a TypeScript type string"));
                    };
                    let Lit::Str(s) = &l.lit else {
                        return Err(syn::Error::new(nv.value.span(), "a TypeScript type string"));
                    };
                    ts = Some(s.value());
                }
                other => {
                    return Err(syn::Error::new(
                        other.span(),
                        "expected `off = \"stop_fn\"` or `ts = \"(..) => void\"`",
                    ));
                }
            }
        }
        let Some(off) = off else {
            return Err(syn::Error::new(
                at.span(),
                "#[subscription(off = \"..\")] names the function that stops it",
            ));
        };
        if !(params.len() == 1
            && matches!(params[0].1, Kind::Callback(_))
            && matches!(ret, Kind::U32 | Kind::F64))
        {
            return Err(syn::Error::new(
                sig.span(),
                "a subscription takes one `Callback` and returns its token (`u32`)",
            ));
        }
        subscription = Some((off, ts));
    }
    Ok(Api {
        name: sig.ident.clone(),
        doc,
        js_name: js_name_attr(attrs).unwrap_or_else(|| stem.clone()),
        stem,
        class,
        tag,
        receiver: has_receiver,
        value_recv: None,
        params,
        optional,
        defaults,
        options,
        options_after,
        options_name,
        options_only,
        param_docs,
        rest,
        nullable: attrs.iter().any(|a| a.path().is_ident("nullable")),
        undefined: attrs.iter().any(|a| {
            a.path().is_ident("nullable")
                && matches!(&a.meta, Meta::List(l) if l.tokens.to_string() == "undefined")
        }),
        ts_type: ts_type_attr(attrs),
        like: attrs.iter().any(|a| a.path().is_ident("like")),
        subscription,
        promise,
        ret,
    })
}

/// `#[method(Vector3)] pub fn add(a: Vec3, b: Vec3) -> Vec3` and `#[getter(Vector3)]`: a free
/// function whose first parameter is the value, seen by scripts as a method or a getter of the
/// value class (`a.add(b)`, `a.magnitude`).
fn value_method(
    sig: &syn::Signature,
    attrs: &[syn::Attribute],
    args: &Args,
) -> syn::Result<Option<Api>> {
    let Some(at) = attrs
        .iter()
        .find(|a| a.path().is_ident("method") || a.path().is_ident("getter"))
    else {
        return Ok(None);
    };
    let getter = at.path().is_ident("getter");
    let ts: Ident = at.parse_args()?;
    let ts = ts.to_string();
    let Some(vi) = args.values.iter().position(|v| v.ts == ts) else {
        return Err(syn::Error::new(
            at.span(),
            format!("`{ts}` is not declared in `values(..)`"),
        ));
    };
    let mut a = api_of(sig, attrs, args, None)?;
    if a.params.is_empty() || !matches!(a.params[0].1, Kind::Value(i) if i == vi) {
        return Err(syn::Error::new(
            sig.span(),
            format!("the first parameter of a `{ts}` method is the `{ts}` itself"),
        ));
    }
    if getter && a.params.len() != 1 {
        return Err(syn::Error::new(
            sig.span(),
            "#[getter(..)] takes only the value: `fn name(v: Vec3) -> T`",
        ));
    }
    a.value_recv = Some(a.params.remove(0));
    a.class = Some((ts, args.values[vi].rust.clone()));
    a.tag = if getter { Tag::Get } else { Tag::Method };
    Ok(Some(a))
}

/// The shim's parameters, and the Rust argument built from them, for one API parameter. A value
/// type is passed as its fields, one scalar each (`p_0, p_1, p_2`).
fn shim_param(name: &Ident, k: &Kind, args: &Args) -> (Vec<TokenStream2>, TokenStream2) {
    match k {
        Kind::Value(i) => {
            let v = &args.values[*i];
            let part = scalar_ty(&v.fields[0].1);
            let n = v.fields.len();
            let names: Vec<Ident> = (0..n).map(|j| format_ident!("{}_{}", name, j)).collect();
            let params = names.iter().map(|p| quote!(#p: #part)).collect();
            let rust = &v.rust;
            let conv = quote! {
                <#rust as ::iron_api::ValueType<#n>>::from_parts([ #( #names as f64 ),* ])
            };
            (params, conv)
        }
        Kind::Enum(i) => {
            let e = &args.enums[*i];
            let from = enum_fn(&e.ts, "from_index");
            let first = enum_first(e);
            (
                vec![quote!(#name: i32)],
                quote!(#from(#name).unwrap_or(#first)),
            )
        }
        Kind::OptEnum(i) => {
            let e = &args.enums[*i];
            let from = enum_fn(&e.ts, "from_index");
            (vec![quote!(#name: i32)], quote!(#from(#name)))
        }
        Kind::OptRecord(i) => {
            let from = format_ident!("__from_json_{}", args.records[*i].ts);
            (
                vec![quote!(#name: *const u8)],
                quote!(unsafe { ::iron_api::script_json_record(#name, #from) }),
            )
        }
        Kind::Opt(inner) => match &**inner {
            Kind::Value(i) => {
                let v = &args.values[*i];
                let part = scalar_ty(&v.fields[0].1);
                let n = v.fields.len();
                let names: Vec<Ident> = (0..n).map(|j| format_ident!("{}_{}", name, j)).collect();
                let params = names.iter().map(|p| quote!(#p: #part)).collect();
                let rust = &v.rust;
                let conv = quote! {
                    {
                        let parts = [ #( #names as f64 ),* ];
                        if parts[0].is_nan() {
                            ::std::option::Option::None
                        } else {
                            ::std::option::Option::Some(
                                <#rust as ::iron_api::ValueType<#n>>::from_parts(parts)
                            )
                        }
                    }
                };
                (params, conv)
            }
            _ => (
                vec![quote!(#name: f64)],
                quote!(if #name.is_nan() {
                    ::std::option::Option::None
                } else {
                    ::std::option::Option::Some(#name)
                }),
            ),
        },
        Kind::Str(owned) => {
            // The script string's object; `script_str` reads it (no copy unless it has a lone
            // surrogate).
            let conv = if *owned {
                quote!(unsafe { ::iron_api::script_str(#name) }.into_owned())
            } else {
                quote!(&*unsafe { ::iron_api::script_str(#name) })
            };
            (vec![quote!(#name: *const u8)], conv)
        }
        Kind::Callback(ks) => {
            let tys: Vec<TokenStream2> = ks
                .iter()
                .map(|k| match k {
                    Kind::Handle(_, rust) => quote!(::iron_api::callback::HandleArg<#rust>),
                    Kind::Str(_) => quote!(::std::string::String),
                    Kind::Bool => quote!(bool),
                    Kind::I32 => quote!(i32),
                    Kind::U32 => quote!(u32),
                    _ => quote!(f64),
                })
                .collect();
            (
                vec![quote!(#name: *const u8)],
                quote!(unsafe { ::iron_api::Callback::<( #( #tys, )* )>::from_script(#name) }),
            )
        }
        Kind::F64s => (
            vec![quote!(#name: *const u8)],
            quote!(unsafe { ::iron_api::script_f64s(#name) }),
        ),
        Kind::Strs(owned) => {
            let read = quote!(unsafe { ::iron_api::script_strs(#name) });
            (
                vec![quote!(#name: *const u8)],
                if *owned { read } else { quote!(&#read) },
            )
        }
        Kind::Values(i) => {
            let v = &args.values[*i];
            let n = v.fields.len();
            let rust = &v.rust;
            (
                vec![quote!(#name: *const u8)],
                quote!(&unsafe { ::iron_api::script_values::<#n, #rust>(#name) }),
            )
        }
        Kind::Handles(_, rust) => (
            vec![quote!(#name: *const u8)],
            quote! {
                &unsafe { ::iron_api::script_handles(#name) }
                    .into_iter()
                    .map(<#rust as ::iron_api::Handle>::from_raw)
                    .collect::<::std::vec::Vec<_>>()
            },
        ),
        other => {
            let t = abi_ty(other);
            (vec![quote!(#name: #t)], from_abi(other, quote!(#name)))
        }
    }
}

// ---- record fields: the types a field may have, and the JSON / JavaScript code for each ----

/// The type of a record field.
#[derive(Clone)]
enum RecTy {
    F64,
    Bool,
    Str,
    /// A declared value class (index into `Args::values`); `true` when written `<Value>Like` (a
    /// plain `{ x, y, z }` object does in the declaration).
    Value(usize, bool),
    /// A declared enum (index into `Args::enums`).
    Enum(usize),
    /// `Option<T>`: the key may be left out (`Option<Option<T>>`: or be `null`).
    Opt(Box<RecTy>),
    /// A list of number pairs (`Vec<(f64, f64)>`), `[number, number][]`.
    Pairs,
}

fn parse_rec_ty(s: &str, args: &Args) -> Result<RecTy, String> {
    if let Some(inner) = s.strip_prefix("Option<").and_then(|r| r.strip_suffix('>')) {
        return Ok(RecTy::Opt(Box::new(parse_rec_ty(inner, args)?)));
    }
    Ok(match s {
        "f64" => RecTy::F64,
        "bool" => RecTy::Bool,
        "string" => RecTy::Str,
        "pairs" => RecTy::Pairs,
        other => {
            if let Some(i) = args.values.iter().position(|v| v.ts == other) {
                RecTy::Value(i, false)
            } else if let Some(i) = other
                .strip_suffix("Like")
                .and_then(|b| args.values.iter().position(|v| v.ts == b))
            {
                RecTy::Value(i, true)
            } else if let Some(i) = args.enums.iter().position(|e| e.ts == other) {
                RecTy::Enum(i)
            } else {
                return Err(format!(
                    "record field type `{other}` is f64, bool, string, pairs, a declared value class (or `<Value>Like`) or enum, or an `Option` of those"
                ));
            }
        }
    })
}

/// Whether the numbers and strings of a record can travel flat (no optional fields or lists).
fn rec_flat(t: &RecTy) -> bool {
    !matches!(t, RecTy::Opt(_) | RecTy::Pairs)
}

/// The manifest name of a field type.
fn rec_ty_name(t: &RecTy, args: &Args) -> String {
    match t {
        RecTy::F64 => "f64".into(),
        RecTy::Bool => "bool".into(),
        RecTy::Str => "string".into(),
        RecTy::Value(i, false) => format!("value:{}", args.values[*i].ts),
        RecTy::Value(i, true) => format!("like:{}", args.values[*i].ts),
        RecTy::Enum(i) => format!("enum:{}", args.enums[*i].ts),
        RecTy::Opt(inner) => format!("opt:{}", rec_ty_name(inner, args)),
        RecTy::Pairs => "list:pair".into(),
    }
}

/// The TypeScript type of a field type.
fn rec_ty_ts(t: &RecTy, args: &Args) -> String {
    match t {
        RecTy::F64 => "number".into(),
        RecTy::Bool => "boolean".into(),
        RecTy::Str => "string".into(),
        RecTy::Value(i, false) => args.values[*i].ts.clone(),
        RecTy::Value(i, true) => format!("{}Like", args.values[*i].ts),
        RecTy::Enum(i) => args.enums[*i].ts.clone(),
        RecTy::Opt(inner) => format!("{} | null", rec_ty_ts(inner, args)),
        RecTy::Pairs => "[number, number][]".into(),
    }
}

/// The manifest `Ty` of a field type.
fn rec_ty_manifest(t: &RecTy, args: &Args) -> TokenStream2 {
    match t {
        RecTy::F64 => quote!(::iron_api::Ty::F64),
        RecTy::Bool => quote!(::iron_api::Ty::Bool),
        RecTy::Str => quote!(::iron_api::Ty::Str),
        RecTy::Value(i, false) => {
            let n = &args.values[*i].ts;
            quote!(::iron_api::Ty::Value(#n))
        }
        RecTy::Value(i, true) => {
            let n = &args.values[*i].ts;
            let like = format!("{n}Like");
            quote!(::iron_api::Ty::Like(#n, #like))
        }
        RecTy::Enum(i) => {
            let n = &args.enums[*i].ts;
            quote!(::iron_api::Ty::Enum(#n))
        }
        RecTy::Opt(inner) => {
            let name = rec_ty_name(inner, args);
            let ts = rec_ty_ts(t, args);
            quote!(::iron_api::Ty::Opt(#name, #ts))
        }
        RecTy::Pairs => quote!(::iron_api::Ty::List("pair", "[number, number][]")),
    }
}

/// Statements that push a field's numbers to `out` and strings to `strs` (flat types only);
/// `x` is a place of the field's type.
fn rec_flat_stmt(t: &RecTy, x: TokenStream2, args: &Args) -> TokenStream2 {
    match t {
        RecTy::F64 => quote!(out.push(#x);),
        RecTy::Bool => quote!(out.push(if #x { 1.0 } else { 0.0 });),
        RecTy::Str => quote!(strs.push(#x.clone());),
        RecTy::Value(i, _) => {
            let v = &args.values[*i];
            let (rust, n) = (&v.rust, v.fields.len());
            quote!(out.extend_from_slice(&<#rust as ::iron_api::ValueType<#n>>::to_parts(#x));)
        }
        RecTy::Enum(i) => {
            let index = enum_fn(&args.enums[*i].ts, "index");
            quote!(out.push(#index(#x) as f64);)
        }
        RecTy::Opt(_) | RecTy::Pairs => quote!(compile_error!("not a flat record field");),
    }
}

/// The JSON value of a field of type `t` held by the reference `x` (not optional).
fn rec_json_value(t: &RecTy, x: TokenStream2, args: &Args) -> TokenStream2 {
    match t {
        RecTy::F64 | RecTy::Bool => quote!(::iron_api::serde_json::json!(*#x)),
        RecTy::Str => quote!(::iron_api::serde_json::json!(#x)),
        RecTy::Value(i, _) => {
            let v = &args.values[*i];
            let (rust, n) = (&v.rust, v.fields.len());
            quote!(::iron_api::serde_json::json!(
                <#rust as ::iron_api::ValueType<#n>>::to_parts(*#x).to_vec()
            ))
        }
        RecTy::Enum(i) => {
            let name = enum_fn(&args.enums[*i].ts, "name");
            quote!(::iron_api::serde_json::json!(#name(*#x)))
        }
        RecTy::Pairs => quote!(::iron_api::serde_json::Value::Array(
            #x.iter()
                .map(|p| ::iron_api::serde_json::json!([p.0, p.1]))
                .collect()
        )),
        RecTy::Opt(_) => quote!(compile_error!("nested option")),
    }
}

/// Statements that put a field into the JSON map `m` under `key` (`x` is a reference).
fn rec_json_stmt(t: &RecTy, key: &str, x: TokenStream2, args: &Args) -> TokenStream2 {
    match t {
        RecTy::Opt(inner) => match &**inner {
            RecTy::Opt(inner2) => {
                let v = rec_json_value(inner2, quote!(y), args);
                quote! {
                    match #x {
                        ::std::option::Option::Some(::std::option::Option::Some(y)) => {
                            m.insert(#key.to_string(), #v);
                        }
                        ::std::option::Option::Some(::std::option::Option::None) => {
                            m.insert(#key.to_string(), ::iron_api::serde_json::Value::Null);
                        }
                        ::std::option::Option::None => {}
                    }
                }
            }
            _ => {
                let v = rec_json_value(inner, quote!(y), args);
                quote! {
                    if let ::std::option::Option::Some(y) = #x {
                        m.insert(#key.to_string(), #v);
                    }
                }
            }
        },
        _ => {
            let v = rec_json_value(t, x, args);
            quote!(m.insert(#key.to_string(), #v);)
        }
    }
}

/// An expression: the field's value read from the JSON value `x` (a `&Value`), as a `Result`.
fn rec_parse_value(t: &RecTy, key: &str, x: TokenStream2, args: &Args) -> TokenStream2 {
    let bad = quote!(|| ::std::format!("`{}` has the wrong type", #key));
    match t {
        RecTy::F64 => quote!(#x.as_f64().ok_or_else(#bad)),
        RecTy::Bool => quote!(#x.as_bool().ok_or_else(#bad)),
        RecTy::Str => quote!(#x.as_str().map(::std::string::ToString::to_string).ok_or_else(#bad)),
        RecTy::Value(i, _) => {
            let v = &args.values[*i];
            let (rust, n) = (&v.rust, v.fields.len());
            let names: Vec<&String> = v.fields.iter().map(|(n, _)| n).collect();
            quote! {
                ::iron_api::json_value_parts::<#n>(#x, [ #( #names ),* ])
                    .map(<#rust as ::iron_api::ValueType<#n>>::from_parts)
                    .ok_or_else(#bad)
            }
        }
        RecTy::Enum(i) => {
            let parse = enum_fn(&args.enums[*i].ts, "parse");
            quote!(#x.as_str().and_then(#parse).ok_or_else(#bad))
        }
        RecTy::Pairs => quote!(::iron_api::json_pairs(#x).ok_or_else(#bad)),
        RecTy::Opt(_) => quote!(compile_error!("nested option")),
    }
}

/// An expression: the field read from the JSON object `obj`, as a `Result`.
fn rec_parse_field(t: &RecTy, key: &str, args: &Args) -> TokenStream2 {
    match t {
        RecTy::Opt(inner) => match &**inner {
            RecTy::Opt(inner2) => {
                let v = rec_parse_value(inner2, key, quote!(x), args);
                quote! {
                    match obj.get(#key) {
                        ::std::option::Option::None => ::std::result::Result::Ok(::std::option::Option::None),
                        ::std::option::Option::Some(x) if x.is_null() => {
                            ::std::result::Result::Ok(::std::option::Option::Some(::std::option::Option::None))
                        }
                        ::std::option::Option::Some(x) => #v.map(|y| ::std::option::Option::Some(::std::option::Option::Some(y))),
                    }
                }
            }
            _ => {
                let v = rec_parse_value(inner, key, quote!(x), args);
                quote! {
                    match obj.get(#key) {
                        ::std::option::Option::None | ::std::option::Option::Some(::iron_api::serde_json::Value::Null) => {
                            ::std::result::Result::Ok(::std::option::Option::None)
                        }
                        ::std::option::Option::Some(x) => #v.map(::std::option::Option::Some),
                    }
                }
            }
        },
        _ => {
            let v = rec_parse_value(t, key, quote!(x), args);
            quote! {
                match obj.get(#key) {
                    ::std::option::Option::Some(x) => #v,
                    ::std::option::Option::None => {
                        ::std::result::Result::Err(::std::format!("`{}` is missing", #key))
                    }
                }
            }
        }
    }
}

/// Statements that set the QuickJS object `o`'s key from a field (`x` is a reference).
fn rec_js_stmt(t: &RecTy, key: &str, x: TokenStream2, args: &Args) -> TokenStream2 {
    match t {
        RecTy::F64 | RecTy::Bool => quote!(o.set(#key, *#x)?;),
        RecTy::Str => quote!(o.set(#key, #x.as_str())?;),
        RecTy::Value(i, _) => {
            let build = format_ident!("__js_value_{}", args.values[*i].ts);
            quote!(o.set(#key, #build(ctx, *#x)?)?;)
        }
        RecTy::Enum(i) => {
            let name = enum_fn(&args.enums[*i].ts, "name");
            quote!(o.set(#key, #name(*#x))?;)
        }
        RecTy::Pairs => quote! {
            {
                let list = ::iron_api::rquickjs::Array::new(ctx.clone())?;
                for (i, p) in #x.iter().enumerate() {
                    let pair = ::iron_api::rquickjs::Array::new(ctx.clone())?;
                    pair.set(0, p.0)?;
                    pair.set(1, p.1)?;
                    list.set(i, pair)?;
                }
                o.set(#key, list)?;
            }
        },
        RecTy::Opt(inner) => match &**inner {
            RecTy::Opt(inner2) => {
                let set = rec_js_stmt(inner2, key, quote!(y), args);
                quote! {
                    match #x {
                        ::std::option::Option::Some(::std::option::Option::Some(y)) => { #set }
                        ::std::option::Option::Some(::std::option::Option::None) => {
                            o.set(#key, ::iron_api::rquickjs::Value::new_null(ctx.clone()))?;
                        }
                        ::std::option::Option::None => {}
                    }
                }
            }
            _ => {
                let set = rec_js_stmt(inner, key, quote!(y), args);
                quote! {
                    if let ::std::option::Option::Some(y) = #x { #set }
                }
            }
        },
    }
}

fn kind_tokens(t: Tag) -> TokenStream2 {
    match t {
        Tag::Static => quote!(::iron_api::FnKind::Static),
        Tag::Method => quote!(::iron_api::FnKind::Method),
        Tag::Get => quote!(::iron_api::FnKind::Get),
        Tag::Set => quote!(::iron_api::FnKind::Set),
    }
}

/// QuickJS functions by (owner, name); each version is (closure, fewest arguments, most).
type JsFns = std::collections::BTreeMap<(String, String), Vec<(TokenStream2, usize, usize)>>;

fn expand(attr: TokenStream2, item: TokenStream2) -> syn::Result<TokenStream2> {
    let args = parse_args(attr)?;
    let module: ItemMod = syn::parse2(item)?;
    let Some((_, items)) = &module.content else {
        return Err(syn::Error::new(
            module.span(),
            "#[iron_api] needs an inline module: `pub mod name { ... }`",
        ));
    };
    // The module's items as they will be emitted: `#[property]` markers are ours, not Rust's.
    let mut out_items: Vec<Item> = items.clone();
    let mut apis: Vec<Api> = Vec::new();
    // The doc comment of each handle class: the one on its first `impl` block.
    let mut class_docs: std::collections::BTreeMap<String, String> = Default::default();
    for it in out_items.iter_mut() {
        match it {
            Item::Fn(f) if matches!(f.vis, Visibility::Public(_)) => {
                match value_method(&f.sig, &f.attrs, &args)? {
                    Some(a) => {
                        apis.push(a);
                        f.attrs.retain(|a| !is_ours(a));
                    }
                    None => {
                        apis.push(api_of(&f.sig, &f.attrs, &args, None)?);
                        f.attrs.retain(|a| !is_ours(a));
                    }
                }
            }
            Item::Impl(imp) if imp.trait_.is_none() => {
                let Some(rust) = rust_name_of(&imp.self_ty) else {
                    continue;
                };
                let Some((ts, _)) = args.handles.iter().find(|(_, r)| *r == rust) else {
                    continue;
                };
                let class = Some((ts.clone(), (*imp.self_ty).clone()));
                let d = doc_of(&imp.attrs);
                if !d.is_empty() {
                    class_docs.entry(ts.clone()).or_insert(d);
                }
                for ii in imp.items.iter_mut() {
                    if let syn::ImplItem::Fn(f) = ii
                        && matches!(f.vis, Visibility::Public(_))
                    {
                        apis.push(api_of(&f.sig, &f.attrs, &args, class.clone())?);
                        f.attrs.retain(|a| !is_ours(a));
                    }
                }
            }
            _ => {}
        }
    }
    // `set_x(self, v)` next to a `#[property] fn x(self) -> T` is the property's setter.
    // (owner, instance member?, name, type); a static property of a class is not an instance one.
    let getters: Vec<(String, bool, String, String)> = apis
        .iter()
        .filter(|a| a.tag == Tag::Get)
        .map(|a| {
            (
                a.class
                    .as_ref()
                    .map_or_else(|| args.namespace.clone(), |c| c.0.clone()),
                a.receiver || a.value_recv.is_some(),
                a.js_name.clone(),
                pair_ty(&a.ret, &args),
            )
        })
        .collect();
    for a in apis.iter_mut() {
        let class = &a
            .class
            .as_ref()
            .map_or_else(|| args.namespace.clone(), |c| c.0.clone());
        let rust = a.name.to_string();
        // A method of a handle class, or a plain function of the namespace.
        let settable = if a.class.is_some() && a.receiver {
            a.tag == Tag::Method
        } else {
            a.tag == Tag::Static
        };
        if settable
            && a.value_recv.is_none()
            && let Some(prop) = rust.strip_prefix("set_")
            && a.params.len() == 1
        {
            let prop = camel(prop);
            let ty = pair_ty(&a.params[0].1, &args);
            let instance = a.class.is_some() && a.receiver;
            if getters
                .iter()
                .any(|(c, i, n, t)| c == class && *i == instance && *n == prop && *t == ty)
            {
                a.tag = Tag::Set;
                a.js_name = prop;
            }
        }
    }

    // A static function and a static property of one class must not share a script name.
    for a in apis.iter().filter(|a| a.tag == Tag::Static) {
        let owner = a.class.as_ref().map(|c| &c.0);
        for g in apis.iter().filter(|g| g.tag == Tag::Get && !g.receiver) {
            if g.class.as_ref().map(|c| &c.0) == owner
                && g.value_recv.is_none()
                && g.js_name == a.js_name
            {
                return Err(syn::Error::new(
                    a.name.span(),
                    format!(
                        "`{}` is both a static function and a static property of the same class",
                        a.js_name
                    ),
                ));
            }
        }
    }

    let ns = &args.namespace;
    let handle_names: Vec<&String> = args.handles.iter().map(|(ts, _)| ts).collect();
    let value_defs: Vec<TokenStream2> = args
        .values
        .iter()
        .map(|v| {
            let ts = &v.ts;
            let fields: Vec<TokenStream2> = v
                .fields
                .iter()
                .map(|(n, t)| {
                    let ty = scalar_manifest(t);
                    quote!(::iron_api::ValueField { name: #n, ty: #ty })
                })
                .collect();
            quote!(::iron_api::ValueDef { name: #ts, fields: &[ #( #fields ),* ] })
        })
        .collect();

    let mut ns_entries = Vec::new();
    let mut class_order: Vec<String> = Vec::new();
    let mut class_entries: Vec<Vec<TokenStream2>> = Vec::new();
    let mut shims = Vec::new();
    // `("iron_X_f", iron_X_f as usize)` for each shim: the table a JIT resolves imports from.
    let mut symbol_entries: Vec<TokenStream2> = Vec::new();
    let mut js_sets = Vec::new();
    // Namespace functions by script name: a name with several versions (overloads) dispatches by
    // the number of arguments.
    // Keyed by (owner, name); the owner is "" for the namespace, else a handle class.
    let mut js_fns: JsFns = std::collections::BTreeMap::new();
    // (class, name, closure): methods of handle classes, put on the prototype.
    let mut js_methods: Vec<(String, String, TokenStream2, usize, usize)> = Vec::new();
    // Properties of handle classes: (class, name) to (getter, setter).
    let mut js_cprops: std::collections::BTreeMap<
        (String, String),
        (Option<TokenStream2>, Option<TokenStream2>),
    > = std::collections::BTreeMap::new();
    // Static properties of handle classes: (class, name) to (getter, setter).
    let mut js_sprops: std::collections::BTreeMap<
        (String, String),
        (Option<TokenStream2>, Option<TokenStream2>),
    > = std::collections::BTreeMap::new();
    let mut js_hidden: Vec<TokenStream2> = Vec::new();
    let mut js_value_methods: std::collections::BTreeMap<String, Vec<(String, bool, String)>> =
        std::collections::BTreeMap::new();
    let mut js_props: std::collections::BTreeMap<
        String,
        (Option<TokenStream2>, Option<TokenStream2>),
    > = std::collections::BTreeMap::new();
    for a in &apis {
        let name = &a.name;
        let rust_name = name.to_string().trim_start_matches("r#").to_string();
        let js_name = &a.js_name;
        let owner = a
            .class
            .as_ref()
            .map(|(ts, _)| ts.clone())
            .unwrap_or_else(|| ns.clone());
        let symbol = format!("iron_{owner}_{}", a.stem);
        let symbol_ident = format_ident!("{}", symbol);
        symbol_entries.push(quote!((#symbol, #symbol_ident as *const () as usize)));
        let doc = &a.doc;
        let has_receiver = a.class.is_some() && a.receiver && a.value_recv.is_none();

        // The manifest entry (a receiver is listed first, as `this`).
        let mut pnames: Vec<String> = Vec::new();
        let mut ptys: Vec<TokenStream2> = Vec::new();
        if has_receiver {
            let ts = &a.class.as_ref().expect("receiver implies a class").0;
            pnames.push("this".into());
            ptys.push(quote!(::iron_api::Ty::Handle(#ts)));
        }
        if let Some((_, k)) = &a.value_recv {
            pnames.push("this".into());
            ptys.push(manifest_ty(k, &args));
        }
        let mut popts: Vec<bool> = vec![false; pnames.len()];
        let mut prests: Vec<bool> = vec![false; pnames.len()];
        let mut pdocs: Vec<String> = vec![String::new(); pnames.len()];
        let mut pdefaults: Vec<TokenStream2> =
            vec![quote!(::std::option::Option::None); pnames.len()];
        for (n, k) in &a.params {
            pnames.push(camel(&n.to_string()));
            ptys.push(manifest_ty(k, &args));
            popts.push(a.optional.contains(&n.to_string()));
            prests.push(a.rest.as_deref() == Some(n.to_string().as_str()));
            pdocs.push(
                a.param_docs
                    .iter()
                    .find(|(p, _)| n == p)
                    .map(|(_, d)| d.clone())
                    .unwrap_or_default(),
            );
            pdefaults.push(match a.defaults.iter().find(|d| *n == d.0) {
                Some((_, v)) => quote!(::std::option::Option::Some(#v)),
                None => quote!(::std::option::Option::None),
            });
        }
        let ret_m = manifest_ty(&a.ret, &args);
        let kind = kind_tokens(a.tag);
        let nullable = a.nullable;
        let undefined = a.undefined;
        let options_only = a.options_only;
        let options_name = match &a.options_name {
            Some(n) => quote!(::std::option::Option::Some(#n)),
            None => quote!(::std::option::Option::None),
        };
        let like = a.like;
        let promise = a.promise;
        let subscription = match &a.subscription {
            Some((off, ts)) => {
                let off = camel(off);
                let ts = ts.clone().unwrap_or_default();
                quote!(::std::option::Option::Some((#off, #ts)))
            }
            None => quote!(::std::option::Option::None),
        };
        let ts_type = match &a.ts_type {
            Some(t) => quote!(::std::option::Option::Some(#t)),
            None => quote!(::std::option::Option::None),
        };
        let options_after = match &a.options_after {
            Some(p) => {
                let i = a
                    .params
                    .iter()
                    .position(|(n, _)| *n == *p)
                    .expect("checked in api_of");
                quote!(::std::option::Option::Some(#i))
            }
            None => quote!(::std::option::Option::None),
        };
        let entry = quote! {
            ::iron_api::Function {
                kind: #kind,
                name: #js_name,
                rust_name: #rust_name,
                symbol: #symbol,
                doc: #doc,
                params: &[ #( ::iron_api::Param { name: #pnames, ty: #ptys, optional: #popts, default: #pdefaults, rest: #prests, doc: #pdocs } ),* ],
                ret: #ret_m,
                options_after: #options_after,
                nullable: #nullable,
                undefined: #undefined,
                options_name: #options_name,
                options_only: #options_only,
                ts_type: #ts_type,
                like: #like,
                subscription: #subscription,
                promise: #promise,
            }
        };
        match &a.class {
            None => ns_entries.push(entry),
            Some((ts, _)) => {
                let i = match class_order.iter().position(|c| c == ts) {
                    Some(i) => i,
                    None => {
                        class_order.push(ts.clone());
                        class_entries.push(Vec::new());
                        class_order.len() - 1
                    }
                };
                class_entries[i].push(entry);
            }
        }

        // The extern "C" shim.
        let mut abi_params: Vec<TokenStream2> = Vec::new();
        let mut call_args: Vec<TokenStream2> = Vec::new();
        let receiver = if has_receiver {
            abi_params.push(quote!(this: u64));
            let (_, rust) = a.class.as_ref().expect("receiver implies a class");
            Some(quote!(<#rust as ::iron_api::Handle>::from_raw(this)))
        } else {
            None
        };
        for (n, k) in a.value_recv.iter().chain(&a.params) {
            let (ps, arg) = shim_param(n, k, &args);
            abi_params.extend(ps);
            call_args.push(arg);
        }
        let call = match (&receiver, &a.class) {
            (None, Some(_)) if a.value_recv.is_some() => quote!(#name( #( #call_args ),* )),
            (Some(r), _) => quote!(#r.#name( #( #call_args ),* )),
            (None, Some((_, rust))) => quote!(<#rust>::#name( #( #call_args ),* )),
            (None, None) => quote!(#name( #( #call_args ),* )),
        };
        if let Kind::Opt(inner) = &a.ret
            && let Kind::Value(i) = &**inner
        {
            // An optional value: NaN in every field is `None`.
            let v = &args.values[*i];
            let part = scalar_ty(&v.fields[0].1);
            let n = v.fields.len();
            let rust = &v.rust;
            let idx: Vec<usize> = (0..n).collect();
            shims.push(quote! {
                /// # Safety
                /// `out` points to room for the value's fields.
                #[unsafe(no_mangle)]
                pub unsafe extern "C" fn #symbol_ident( #( #abi_params, )* out: *mut #part ) {
                    let parts = match #call {
                        ::std::option::Option::Some(v) => {
                            <#rust as ::iron_api::ValueType<#n>>::to_parts(v)
                        }
                        ::std::option::Option::None => [f64::NAN; #n],
                    };
                    #( unsafe { *out.add(#idx) = parts[#idx] as #part; } )*
                }
            });
        } else if let Kind::Value(i) = &a.ret {
            // A value result is written through a trailing pointer, one scalar per field.
            let v = &args.values[*i];
            let part = scalar_ty(&v.fields[0].1);
            let n = v.fields.len();
            let rust = &v.rust;
            let idx: Vec<usize> = (0..n).collect();
            shims.push(quote! {
                /// # Safety
                /// `out` points to room for the value's fields.
                #[unsafe(no_mangle)]
                pub unsafe extern "C" fn #symbol_ident( #( #abi_params, )* out: *mut #part ) {
                    let parts = <#rust as ::iron_api::ValueType<#n>>::to_parts(#call);
                    #( unsafe { *out.add(#idx) = parts[#idx] as #part; } )*
                }
            });
        } else if let Kind::Record(i) = &a.ret {
            // A record result: its numbers stay in `iron_api`'s per-thread buffer.
            let flat = format_ident!("__flat_{}", args.records[*i].ts);
            shims.push(quote! {
                #[unsafe(no_mangle)]
                pub extern "C" fn #symbol_ident( #( #abi_params ),* ) {
                    let r = #call;
                    let mut out = ::std::vec::Vec::new();
                    let mut strs = ::std::vec::Vec::new();
                    #flat(&r, &mut out, &mut strs);
                    ::iron_api::return_flat(out);
                    ::iron_api::return_strings(strs);
                }
            });
        } else if let Kind::List(inner) = &a.ret
            && matches!(**inner, Kind::Str(_))
        {
            // A list of strings: the strings stay in `iron_api`'s per-thread list and the caller
            // reads them with `iron_api_result_str(i)`; the shim returns how many there are.
            let unsafe_kw = if a.params.iter().any(|(_, k)| {
                matches!(
                    k,
                    Kind::Str(_)
                        | Kind::Callback(_)
                        | Kind::F64s
                        | Kind::Strs(_)
                        | Kind::Values(_)
                        | Kind::OptRecord(_)
                        | Kind::Handles(..)
                )
            }) {
                quote!(unsafe)
            } else {
                quote!()
            };
            shims.push(quote! {
                /// # Safety
                /// Array and string arguments are live script objects (or null).
                #[unsafe(no_mangle)]
                pub #unsafe_kw extern "C" fn #symbol_ident( #( #abi_params ),* ) -> u32 {
                    let items: ::std::vec::Vec<::std::string::String> = #call;
                    let count = items.len() as u32;
                    ::iron_api::return_strings(items);
                    count
                }
            });
        } else if let Kind::List(inner) = &a.ret {
            // A list result: the numbers of every element, one after the other, stay in the
            // buffer; the shim returns how many elements there are.
            let push = match &**inner {
                Kind::Value(i) => {
                    let v = &args.values[*i];
                    let n = v.fields.len();
                    let rust = &v.rust;
                    quote!(out.extend_from_slice(&<#rust as ::iron_api::ValueType<#n>>::to_parts(*item));)
                }
                Kind::Record(i) => {
                    let flat = format_ident!("__flat_{}", args.records[*i].ts);
                    quote!(#flat(item, &mut out, &mut strs);)
                }
                Kind::Enum(i) => {
                    let index = enum_fn(&args.enums[*i].ts, "index");
                    quote!(out.push(#index(*item) as f64);)
                }
                Kind::Handle(..) => quote!(ids.push(::iron_api::Handle::to_raw(*item));),
                Kind::F64 => quote!(out.push(*item);),
                _ => unreachable!("classify only builds lists of values, enums and records"),
            };
            let unsafe_kw = if a.params.iter().any(|(_, k)| {
                matches!(
                    k,
                    Kind::Str(_)
                        | Kind::Callback(_)
                        | Kind::F64s
                        | Kind::Strs(_)
                        | Kind::Values(_)
                        | Kind::OptRecord(_)
                        | Kind::Handles(..)
                )
            }) {
                quote!(unsafe)
            } else {
                quote!()
            };
            shims.push(quote! {
                /// # Safety
                /// Array and string arguments are live script objects (or null).
                #[unsafe(no_mangle)]
                pub #unsafe_kw extern "C" fn #symbol_ident( #( #abi_params ),* ) -> u32 {
                    let items = #call;
                    let mut out = ::std::vec::Vec::new();
                    let mut strs: ::std::vec::Vec<::std::string::String> = ::std::vec::Vec::new();
                    #[allow(unused_mut)]
                    let mut ids: ::std::vec::Vec<u64> = ::std::vec::Vec::new();
                    for item in &items {
                        #push
                    }
                    ::iron_api::return_flat(out);
                    ::iron_api::return_strings(strs);
                    ::iron_api::return_ids(ids);
                    items.len() as u32
                }
            });
        } else if let Kind::OptRecord(i) = &a.ret {
            // An optional record result: its JSON text (`null` for none).
            let to = format_ident!("__to_json_{}", args.records[*i].ts);
            shims.push(quote! {
                /// # Safety
                /// `out_len` is valid for a write.
                #[unsafe(no_mangle)]
                pub unsafe extern "C" fn #symbol_ident( #( #abi_params, )* out_len: *mut i64 ) -> *const u8 {
                    let text = match #call {
                        ::std::option::Option::Some(r) => #to(&r).to_string(),
                        ::std::option::Option::None => ::std::string::String::from("null"),
                    };
                    unsafe { ::iron_api::return_string(text, out_len) }
                }
            });
        } else if let Kind::Opt(inner) = &a.ret
            && matches!(**inner, Kind::Str(_))
        {
            // An optional string: `None` is an empty text and a flag the caller reads.
            shims.push(quote! {
                /// # Safety
                /// `out_len` is valid for a write.
                #[unsafe(no_mangle)]
                pub unsafe extern "C" fn #symbol_ident( #( #abi_params, )* out_len: *mut i64 ) -> *const u8 {
                    unsafe { ::iron_api::return_string_opt(#call, out_len) }
                }
            });
        } else if let Kind::Str(_) = &a.ret {
            // A string result: the text stays in `iron_api`'s per-thread buffer and the caller
            // copies it into a script string at once.
            shims.push(quote! {
                /// # Safety
                /// `out_len` is valid for a write.
                #[unsafe(no_mangle)]
                pub unsafe extern "C" fn #symbol_ident( #( #abi_params, )* out_len: *mut i64 ) -> *const u8 {
                    unsafe { ::iron_api::return_string(#call, out_len) }
                }
            });
        } else {
            let abi_ret = abi_ty(&a.ret);
            let body = to_abi(&a.ret, call, &args);
            // A string parameter is a pointer to a script string: the shim is unsafe to call.
            if a.params.iter().any(|(_, k)| {
                matches!(
                    k,
                    Kind::Str(_)
                        | Kind::Callback(_)
                        | Kind::F64s
                        | Kind::Strs(_)
                        | Kind::Values(_)
                        | Kind::OptRecord(_)
                        | Kind::Handles(..)
                )
            }) {
                shims.push(quote! {
                    /// # Safety
                    /// String arguments are live script strings (or null).
                    #[unsafe(no_mangle)]
                    pub unsafe extern "C" fn #symbol_ident( #( #abi_params ),* ) -> #abi_ret {
                        #body
                    }
                });
            } else {
                shims.push(quote! {
                    #[unsafe(no_mangle)]
                    pub extern "C" fn #symbol_ident( #( #abi_params ),* ) -> #abi_ret {
                        #body
                    }
                });
            }
        }

        // The QuickJS function. A string is a JS string, a value is an object with the fields'
        // names (or an array of them), a handle is an opaque object (`null` for 0), a result is a
        // new one. A handle class's members go on its prototype (methods, accessors) or on the
        // class (statics).
        if args.quickjs {
            let handle_class = a.class.as_ref().filter(|_| a.value_recv.is_none());
            // A value method takes the value first (an object), then its own parameters.
            let n_user = a.params.len() - if a.promise { 2 } else { 0 };
            let all_params: Vec<&(Ident, Kind)> =
                a.value_recv.iter().chain(&a.params[..n_user]).collect();
            let is_opt = |n: &Ident| a.optional.contains(&n.to_string());
            let recv_param = if has_receiver {
                quote!(this: ::iron_api::rquickjs::function::This<::iron_api::rquickjs::Value<'js>>,)
            } else {
                quote!()
            };
            let recv_bind = match handle_class {
                Some((ts, rust)) if has_receiver => quote! {
                    let recv = <#rust as ::iron_api::Handle>::from_raw(
                        ::iron_api::qjs::recv_arg(&ctx, &this.0, #ts)?
                    );
                },
                _ => quote!(),
            };
            let call_path = match handle_class {
                Some(_) if has_receiver => quote!(recv.#name),
                Some((_, rust)) => quote!(<#rust>::#name),
                None => quote!(#name),
            };
            let js_params: Vec<TokenStream2> = all_params
                .iter()
                .map(|(n, k)| {
                    let t = js_param_ty(k);
                    if is_opt(n) {
                        quote!(#n: ::iron_api::rquickjs::function::Opt<::std::option::Option<#t>>)
                    } else {
                        quote!(#n: #t)
                    }
                })
                .collect();
            // A left-out parameter takes the default the compiled call passes.
            let js_defaults: Vec<TokenStream2> = all_params
                .iter()
                .filter(|(n, _)| is_opt(n))
                .map(|(n, k)| {
                    if matches!(k, Kind::Value(_)) {
                        return quote!(let #n = #n.0.flatten(););
                    }
                    let d = match a.defaults.iter().find(|d| *n == d.0) {
                        Some((_, v)) => js_default_value(k, *v),
                        None => js_default(k),
                    };
                    quote!(let #n = #n.0.flatten().unwrap_or(#d);)
                })
                .collect();
            let js_args: Vec<TokenStream2> = all_params
                .iter()
                .map(|(n, k)| js_arg(k, quote!(#n), &args, is_opt(n)))
                .collect();
            let (js_ret, js_body) = if a.promise {
                // A real promise; the engine settles it through two callbacks.
                let res_kind = a.params[a.params.len() - 2].1.clone();
                let rej_kind = a.params[a.params.len() - 1].1.clone();
                let res_cb = js_arg(&res_kind, quote!(__resolve), &args, false);
                let rej_cb = js_arg(&rej_kind, quote!(__rejecter), &args, false);
                (
                    quote!(::iron_api::rquickjs::Promise<'js>),
                    quote! {
                        let (__promise, __resolve, __reject) = ::iron_api::rquickjs::Promise::new(&ctx)?;
                        let __rejecter = ::iron_api::qjs::error_rejecter(&ctx, __reject)?;
                        #call_path( #( #js_args, )* #res_cb, #rej_cb );
                        Ok(__promise)
                    },
                )
            } else if let Some((off, _)) = &a.subscription {
                // The token goes into a function that stops the listener.
                let off_fn = format_ident!("{}", off);
                (
                    quote!(::iron_api::rquickjs::Function<'js>),
                    quote! {
                        let token = #call_path( #( #js_args ),* );
                        ::iron_api::rquickjs::Function::new(ctx.clone(), move || {
                            let _ = #off_fn(token);
                        })
                    },
                )
            } else {
                (
                    js_ret_ty(&a.ret),
                    js_result(&a.ret, quote!(#call_path( #( #js_args ),* )), &args),
                )
            };
            // rquickjs takes a few typed parameters; a function with more reads its arguments from
            // the rest list one by one.
            let first_opt = all_params
                .iter()
                .position(|(n, _)| is_opt(n))
                .unwrap_or(all_params.len());
            // `#[options]`: a call with exactly one argument past the required ones, a plain object,
            // gives the optional parameters by name. Positional calls keep working.
            // With `after = p` the object comes right after `p`, and `p` and the ones before it
            // are always positional.
            let after_idx: Option<usize> = a.options_after.as_ref().map(|p| {
                all_params
                    .iter()
                    .position(|(n, _)| *n == *p)
                    .expect("checked in api_of")
            });
            let opts_at = after_idx.map_or(first_opt, |i| i + 1);
            let opts_prelude = if a.options {
                quote! {
                    let __opts: ::std::option::Option<::iron_api::rquickjs::Object<'js>> =
                        if rest.0.len() == #opts_at + 1 {
                            ::iron_api::qjs::options_object(&ctx, &rest.0[#opts_at])
                        } else {
                            None
                        };
                }
            } else {
                quote!()
            };
            let closure = if all_params.len() + usize::from(has_receiver) > 4
                || a.options
                || a.rest.is_some()
            {
                let binds: Vec<TokenStream2> = all_params
                    .iter()
                    .enumerate()
                    .map(|(i, (n, k))| {
                        let t = js_param_ty(k);
                        // The argument at this position, or (an options call) the property of the
                        // object; undefined and null mean "left out".
                        let slot = if a.options && !after_idx.is_some_and(|ai| i <= ai) {
                            let key = camel(&n.to_string());
                            quote! {
                                match &__opts {
                                    Some(o) => {
                                        let v: ::iron_api::rquickjs::Value<'js> = o.get(#key)?;
                                        if v.is_undefined() || v.is_null() { None } else { Some(v) }
                                    }
                                    None => rest.0.get(#i).filter(|v| !v.is_undefined() && !v.is_null()).cloned(),
                                }
                            }
                        } else {
                            quote! {
                                rest.0.get(#i).filter(|v| !v.is_undefined() && !v.is_null()).cloned()
                            }
                        };
                        if a.rest.as_deref() == Some(n.to_string().as_str()) {
                            // Every remaining argument.
                            quote! {
                                let #n: #t = {
                                    let mut items = ::std::vec::Vec::new();
                                    for v in rest.0.iter().skip(#i) {
                                        items.push(::iron_api::rquickjs::FromJs::from_js(&ctx, v.clone())?);
                                    }
                                    items
                                };
                            }
                        } else if is_opt(n) && matches!(k, Kind::Value(_)) {
                            quote! {
                                let #n: ::std::option::Option<#t> = match #slot {
                                    Some(v) => {
                                        Some(::iron_api::rquickjs::FromJs::from_js(&ctx, v)?)
                                    }
                                    None => None,
                                };
                            }
                        } else if is_opt(n) {
                            let d = match a.defaults.iter().find(|d| *n == d.0) {
                                Some((_, v)) => js_default_value(k, *v),
                                None => js_default(k),
                            };
                            quote! {
                                let #n: #t = match #slot {
                                    Some(v) => {
                                        ::iron_api::rquickjs::FromJs::from_js(&ctx, v)?
                                    }
                                    None => #d,
                                };
                            }
                        } else {
                            quote! {
                                let #n: #t = match rest.0.get(#i) {
                                    Some(v) => ::iron_api::rquickjs::FromJs::from_js(&ctx, v.clone())?,
                                    None => {
                                        return Err(::iron_api::rquickjs::Exception::throw_type(
                                            &ctx,
                                            "missing argument",
                                        ));
                                    }
                                };
                            }
                        }
                    })
                    .collect();
                quote! {
                    | ctx: ::iron_api::rquickjs::Ctx<'js>,
                      #recv_param
                      rest: ::iron_api::rquickjs::function::Rest<::iron_api::rquickjs::Value<'js>> |
                        -> ::iron_api::rquickjs::Result<#js_ret> {
                        #recv_bind
                        #opts_prelude
                        #( #binds )*
                        #js_body
                    }
                }
            } else {
                quote! {
                    | ctx: ::iron_api::rquickjs::Ctx<'js>, #recv_param #( #js_params ),* |
                        -> ::iron_api::rquickjs::Result<#js_ret> {
                        let _ = &ctx;
                        #recv_bind
                        #( #js_defaults )*
                        #js_body
                    }
                }
            };
            if let Some((ts, _)) = a.class.as_ref().filter(|_| a.value_recv.is_some()) {
                let hidden = format!("__iron_vm_{ts}_{js_name}");
                js_value_methods.entry(ts.clone()).or_default().push((
                    js_name.clone(),
                    a.tag == Tag::Get,
                    hidden.clone(),
                ));
                js_hidden.push(quote! {
                    ctx.globals().set(
                        #hidden,
                        ::iron_api::rquickjs::Function::new(ctx.clone(), #closure)?,
                    )?;
                });
                continue;
            }
            // A value class's `create` is also what `new X(..)` calls; so is a handle class's.
            if a.tag == Tag::Static && js_name == "create" {
                let ctor_of = match handle_class {
                    Some((ts, _)) => Some(ts.clone()),
                    None if args.values.iter().any(|v| v.ts == args.namespace) => {
                        Some(args.namespace.clone())
                    }
                    None => None,
                };
                if let Some(ts) = ctor_of {
                    let hidden = if handle_class.is_some() {
                        format!("__iron_hcreate_{ts}")
                    } else {
                        format!("__iron_create_{ts}")
                    };
                    js_hidden.push(quote! {
                        ctx.globals().set(
                            #hidden,
                            ::iron_api::rquickjs::Function::new(ctx.clone(), #closure)?,
                        )?;
                    });
                }
            }
            // "" is the namespace object, else the handle class.
            let target = handle_class.map(|(ts, _)| ts.clone()).unwrap_or_default();
            match (a.tag, handle_class.is_some()) {
                // A static property of the class: an accessor on the class object.
                (Tag::Get, true) if !has_receiver => {
                    js_sprops.entry((target, js_name.clone())).or_default().0 = Some(closure)
                }
                (Tag::Set, true) if !has_receiver => {
                    js_sprops.entry((target, js_name.clone())).or_default().1 = Some(closure)
                }
                (Tag::Get, true) => {
                    js_cprops.entry((target, js_name.clone())).or_default().0 = Some(closure)
                }
                (Tag::Set, true) => {
                    js_cprops.entry((target, js_name.clone())).or_default().1 = Some(closure)
                }
                (Tag::Method, _) => {
                    let max = a.params.len();
                    let min = max
                        - a.params
                            .iter()
                            .filter(|(n, _)| a.optional.contains(&n.to_string()))
                            .count();
                    js_methods.push((target, js_name.clone(), closure, min, max));
                }
                // A namespace property: an accessor on the namespace object.
                (Tag::Get, false) => js_props.entry(js_name.clone()).or_default().0 = Some(closure),
                (Tag::Set, false) => js_props.entry(js_name.clone()).or_default().1 = Some(closure),
                (Tag::Static, _) => {
                    let max = a.params.len() + usize::from(a.value_recv.is_some());
                    let min = max
                        - a.params
                            .iter()
                            .filter(|(n, _)| a.optional.contains(&n.to_string()))
                            .count();
                    js_fns
                        .entry((target, js_name.clone()))
                        .or_default()
                        .push((closure, min, max));
                }
            }
        }
    }
    for ((target, name), versions) in &js_fns {
        // The namespace object, or the handle class (which is the namespace object when they
        // share a name).
        let holder = if target.is_empty() || *target == args.namespace {
            quote!(ns)
        } else {
            quote!(ctx.globals().get::<_, ::iron_api::rquickjs::Object>(#target)?)
        };
        if let [(closure, _, _)] = versions.as_slice() {
            js_sets.push(quote! {
                #holder.set(
                    #name,
                    ::iron_api::rquickjs::Function::new(ctx.clone(), #closure)?,
                )?;
            });
            continue;
        }
        // Several versions: each is a hidden function, and a dispatcher picks one by argument count.
        let mut branches = String::new();
        for (i, (closure, min, max)) in versions.iter().enumerate() {
            let hidden = format!("__iron_ov_{}_{target}_{name}_{i}", args.namespace);
            js_hidden.push(quote! {
                ctx.globals().set(
                    #hidden,
                    ::iron_api::rquickjs::Function::new(ctx.clone(), #closure)?,
                )?;
            });
            branches +=
                &format!("if (n >= {min} && n <= {max}) return globalThis.{hidden}(...a); ");
        }
        let src = format!(
            "(function(...a) {{ const n = a.length; {branches}throw new TypeError('{name}: no version takes ' + n + ' arguments'); }})"
        );
        js_sets.push(quote! {
            #holder.set(#name, ctx.eval::<::iron_api::rquickjs::Function, _>(#src)?)?;
        });
    }

    // Methods and accessors of handle classes go on the prototype (not enumerable, so a handle
    // has no own or listed properties).
    let mut method_groups: JsFns = std::collections::BTreeMap::new();
    for (target, name, closure, min, max) in &js_methods {
        method_groups
            .entry((target.clone(), name.clone()))
            .or_default()
            .push((closure.clone(), *min, *max));
    }
    for ((target, name), versions) in &method_groups {
        let holder = if *target == args.namespace {
            quote!(ns)
        } else {
            quote!(ctx.globals().get::<_, ::iron_api::rquickjs::Object>(#target)?)
        };
        // One version is the function; several are hidden functions behind a dispatcher by
        // argument count that passes `this` on.
        let func = if let [(closure, _, _)] = versions.as_slice() {
            quote!(::iron_api::rquickjs::Function::new(ctx.clone(), #closure)?)
        } else {
            let mut branches = String::new();
            for (i, (closure, min, max)) in versions.iter().enumerate() {
                let hidden = format!("__iron_ovm_{}_{target}_{name}_{i}", args.namespace);
                js_hidden.push(quote! {
                    ctx.globals().set(
                        #hidden,
                        ::iron_api::rquickjs::Function::new(ctx.clone(), #closure)?,
                    )?;
                });
                branches += &format!(
                    "if (n >= {min} && n <= {max}) return globalThis.{hidden}.call(this, ...a); "
                );
            }
            let src = format!(
                "(function(...a) {{ const n = a.length; {branches}throw new TypeError('{name}: no version takes ' + n + ' arguments'); }})"
            );
            quote!(ctx.eval::<::iron_api::rquickjs::Function, _>(#src)?)
        };
        js_sets.push(quote! {
            {
                let proto: ::iron_api::rquickjs::Object = #holder.get("prototype")?;
                proto.prop(
                    #name,
                    ::iron_api::rquickjs::object::Property::from(#func)
                        .writable()
                        .configurable(),
                )?;
            }
        });
    }
    for ((target, name), (get, set)) in &js_cprops {
        let holder = if *target == args.namespace {
            quote!(ns)
        } else {
            quote!(ctx.globals().get::<_, ::iron_api::rquickjs::Object>(#target)?)
        };
        let acc = match (get, set) {
            (Some(g), Some(st)) => {
                quote!(::iron_api::rquickjs::object::Accessor::from(#g).set(#st).configurable())
            }
            (Some(g), None) => {
                quote!(::iron_api::rquickjs::object::Accessor::from(#g).configurable())
            }
            (None, Some(st)) => {
                quote!(::iron_api::rquickjs::object::Accessor::new_set(#st).configurable())
            }
            (None, None) => continue,
        };
        js_sets.push(quote! {
            {
                let proto: ::iron_api::rquickjs::Object = #holder.get("prototype")?;
                proto.prop(#name, #acc)?;
            }
        });
    }

    // Static accessors on the class object (not enumerable): `Window.main` reads as a property.
    for ((target, name), (get, set)) in &js_sprops {
        let holder = if *target == args.namespace {
            quote!(ns)
        } else {
            quote!(ctx.globals().get::<_, ::iron_api::rquickjs::Object>(#target)?)
        };
        let acc = match (get, set) {
            (Some(g), Some(st)) => {
                quote!(::iron_api::rquickjs::object::Accessor::from(#g).set(#st).configurable())
            }
            (Some(g), None) => {
                quote!(::iron_api::rquickjs::object::Accessor::from(#g).configurable())
            }
            (None, Some(st)) => {
                quote!(::iron_api::rquickjs::object::Accessor::new_set(#st).configurable())
            }
            (None, None) => continue,
        };
        js_sets.push(quote! { #holder.prop(#name, #acc)?; });
    }

    for (name, (get, set)) in &js_props {
        let acc = match (get, set) {
            (Some(g), Some(st)) => {
                quote!(::iron_api::rquickjs::object::Accessor::from(#g).set(#st).enumerable())
            }
            (Some(g), None) => {
                quote!(::iron_api::rquickjs::object::Accessor::from(#g).enumerable())
            }
            (None, Some(st)) => {
                quote!(::iron_api::rquickjs::object::Accessor::new_set(#st).enumerable())
            }
            (None, None) => continue,
        };
        js_sets.push(quote! { ns.prop(#name, #acc)?; });
    }
    // One JS class per value type: a constructor taking the fields, and its methods and getters.
    let mut js_class_stmts: Vec<TokenStream2> = Vec::new();
    let mut ns_init = quote!(::iron_api::rquickjs::Object::new(ctx.clone())?);
    for v in &args.values {
        let ts = &v.ts;
        let fields: Vec<&String> = v.fields.iter().map(|(n, _)| n).collect();
        let ctor_params: Vec<String> = fields.iter().map(|f| format!("{f} = 0")).collect();
        let ctor_body: Vec<String> = fields.iter().map(|f| format!("this.{f} = {f};")).collect();
        // The class is made once (another module may have made it already) and each module adds
        // its methods to the prototype.
        // `new X(..)`: the engine's own results come with a marker first (the fields); a script's
        // call goes through the static `create` when the namespace has one, else the fields.
        let plain: Vec<String> = fields
            .iter()
            .enumerate()
            .map(|(i, f)| format!("this.{f} = a[{i}] === undefined ? 0 : a[{i}];"))
            .collect();
        let raw: Vec<String> = fields
            .iter()
            .enumerate()
            .map(|(i, f)| format!("this.{f} = a[{}];", i + 1))
            .collect();
        let copy: Vec<String> = fields
            .iter()
            .map(|f| format!("this.{f} = v.{f};"))
            .collect();
        let _ = (&ctor_params, &ctor_body);
        // `toJSON` writes the compact array of numbers and `X.fromJSON` reads it back (or the
        // object form, left-out fields 0 and alpha 1): the same as compiled code.
        let to_json = fields
            .iter()
            .map(|f| format!("this.{f}"))
            .collect::<Vec<_>>()
            .join(", ");
        let from_obj = fields
            .iter()
            .map(|f| format!("v.{f} ?? {}", if f.as_str() == "a" { 1 } else { 0 }))
            .collect::<Vec<_>>()
            .join(", ");
        let count = fields.len();
        let mut src = format!(
            "globalThis.{ts} ??= class {ts} {{ constructor(...a) {{ if (a[0] === globalThis.__iron_raw) {{ {} }} else if (globalThis.__iron_create_{ts}) {{ const v = globalThis.__iron_create_{ts}(...a); {} }} else {{ {} }} }} toJSON() {{ return [{to_json}]; }} static fromJSON(v) {{ if (Array.isArray(v)) {{ if (v.length !== {count}) throw new TypeError('the array has the wrong number of values'); return new {ts}(globalThis.__iron_raw, ...v); }} if (v === null || typeof v !== 'object') throw new TypeError('expected an array of numbers or an object'); return new {ts}(globalThis.__iron_raw, {from_obj}); }} }};",
            raw.join(" "),
            copy.join(" "),
            plain.join(" ")
        );
        for (name, is_get, hidden) in js_value_methods.get(ts).into_iter().flatten() {
            if *is_get {
                src += &format!(
                    " Object.defineProperty({ts}.prototype, '{name}', {{ get() {{ return {hidden}(this); }}, configurable: true }});"
                );
            } else {
                src += &format!(
                    " Object.defineProperty({ts}.prototype, '{name}', {{ value(...a) {{ return {hidden}(this, ...a); }}, configurable: true, writable: true }});"
                );
            }
        }
        if *ts == args.namespace {
            // The namespace object is the class itself: `Color.white` is a static of `Color`.
            let src = format!("{src} {ts}");
            ns_init = quote!(ctx.eval::<::iron_api::rquickjs::Object, _>(#src)?);
        } else {
            js_class_stmts.push(quote!(ctx.eval::<(), _>(#src)?;));
        }
    }
    // One JS class per handle type: an opaque object (see `iron_api::qjs`), made once; each module
    // adds its members to the prototype. `new X(..)` calls the static `create`.
    for (ts, _) in &args.handles {
        // A namespace object already holding the name (the scene's `Scene` helper functions next to
        // the `Scene` class) is merged into the class, which wins.
        let src = format!(
            "{{ const old = globalThis.{ts}; if (typeof old !== 'function') {{ globalThis.{ts} = class {ts} {{ constructor(...a) {{ const c = globalThis.__iron_hcreate_{ts}; if (!c) throw new TypeError('{ts} cannot be constructed'); return c(...a); }} }}; if (old) for (const k of Object.getOwnPropertyNames(old)) Object.defineProperty(globalThis.{ts}, k, Object.getOwnPropertyDescriptor(old, k)); }} }} Object.defineProperty({ts}.prototype, Symbol.toStringTag, {{ value: '{ts}', configurable: true }});"
        );
        if *ts == args.namespace {
            // The namespace object is the class itself, as for a value class.
            let src = format!("{src} {ts}");
            ns_init = quote!(ctx.eval::<::iron_api::rquickjs::Object, _>(#src)?);
        } else {
            js_class_stmts.push(quote!(ctx.eval::<(), _>(#src)?;));
            // `Scene.GameObject` is the same class as the global.
            js_sets.push(quote! {
                ns.set(#ts, ctx.globals().get::<_, ::iron_api::rquickjs::Value>(#ts)?)?;
            });
        }
    }
    // ---- enums: manifest entries, helper functions, `iron_<Enum>_name` / `_parse` shims ----
    let mut enum_defs: Vec<TokenStream2> = Vec::new();
    let mut enum_items: Vec<TokenStream2> = Vec::new();
    for e in args.enums.iter().filter(|e| !e.external) {
        let ts = &e.ts;
        let names = e.names();
        let rust = &e.rust;
        let variants = &e.variants;
        let idx: Vec<i32> = (0..variants.len() as i32).collect();
        let doc = if e.doc.is_empty() {
            items
                .iter()
                .find_map(|it| match it {
                    Item::Enum(en) if Some(en.ident.to_string()) == rust_name_of(&e.rust) => {
                        Some(doc_of(&en.attrs))
                    }
                    _ => None,
                })
                .unwrap_or_default()
        } else {
            e.doc.clone()
        };
        enum_defs
            .push(quote!(::iron_api::EnumDef { name: #ts, doc: #doc, values: &[ #( #names ),* ] }));
        let parse = enum_fn(ts, "parse");
        let name = enum_fn(ts, "name");
        let index = enum_fn(ts, "index");
        let from_index = enum_fn(ts, "from_index");
        let names_const = format_ident!("{}_NAMES", snake(ts).to_uppercase());
        let name_sym = format_ident!("iron_{}_name", ts);
        let parse_sym = format_ident!("iron_{}_parse", ts);
        symbol_entries
            .push(quote!((concat!("iron_", #ts, "_name"), #name_sym as *const () as usize)));
        symbol_entries
            .push(quote!((concat!("iron_", #ts, "_parse"), #parse_sym as *const () as usize)));
        enum_items.push(quote! {
            /// The names scripts see, in index order.
            pub const #names_const: &[&str] = &[ #( #names ),* ];

            /// The value with this script name.
            pub fn #parse(s: &str) -> ::std::option::Option<#rust> {
                match s {
                    #( #names => ::std::option::Option::Some(#rust::#variants), )*
                    _ => ::std::option::Option::None,
                }
            }

            /// The script name of a value.
            pub fn #name(v: #rust) -> &'static str {
                match v {
                    #( #rust::#variants => #names, )*
                }
            }

            /// The index of a value: what travels across the C ABI.
            pub fn #index(v: #rust) -> i32 {
                match v {
                    #( #rust::#variants => #idx, )*
                }
            }

            /// The value at an index, or none.
            pub fn #from_index(i: i32) -> ::std::option::Option<#rust> {
                match i {
                    #( #idx => ::std::option::Option::Some(#rust::#variants), )*
                    _ => ::std::option::Option::None,
                }
            }

            /// `TypeName.name(i)` for compiled code: the script name of an index, or "".
            ///
            /// # Safety
            /// `out_len` is valid for a write.
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn #name_sym(i: i32, out_len: *mut i64) -> *const u8 {
                let text = #from_index(i).map_or("", #name);
                unsafe { ::iron_api::return_string(text, out_len) }
            }

            /// `TypeName.parse(s)` for compiled code: the index of a script name, or -1.
            ///
            /// # Safety
            /// `s` is a live script string (or null).
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn #parse_sym(s: *const u8) -> i32 {
                #parse(&unsafe { ::iron_api::script_str(s) }).map_or(-1, #index)
            }
        });
        if args.quickjs {
            js_sets.push(quote! {
                {
                    let object = match ctx.globals().get::<_, ::iron_api::rquickjs::Value>(#ts)? {
                        v if v.is_object() && !v.is_function() => v.into_object().expect("checked"),
                        _ => ::iron_api::rquickjs::Object::new(ctx.clone())?,
                    };
                    object.set("name", ::iron_api::rquickjs::Function::new(
                        ctx.clone(),
                        |i: i32| -> ::std::option::Option<::std::string::String> {
                            #from_index(i).map(|v| #name(v).to_string())
                        },
                    ))?;
                    object.set("parse", ::iron_api::rquickjs::Function::new(
                        ctx.clone(),
                        |s: ::std::string::String| -> i32 { #parse(&s).map_or(-1, #index) },
                    ))?;
                    ctx.globals().set(#ts, object)?;
                }
            });
        }
    }

    // ---- records: manifest entries, `__flat_<R>` (the numbers) and `__js_rec_<R>` (the object) ----
    let mut record_defs: Vec<TokenStream2> = Vec::new();
    let mut record_items: Vec<TokenStream2> = Vec::new();
    for r in &args.records {
        let ts = &r.ts;
        let rust = &r.rust;
        let (item_doc, item_field_docs) = items
            .iter()
            .find_map(|it| match it {
                Item::Struct(st) if Some(st.ident.to_string()) == rust_name_of(&r.rust) => {
                    let docs: Vec<(String, String)> = st
                        .fields
                        .iter()
                        .filter_map(|f| f.ident.as_ref().map(|i| (i.to_string(), doc_of(&f.attrs))))
                        .collect();
                    Some((doc_of(&st.attrs), docs))
                }
                _ => None,
            })
            .unwrap_or_default();
        let rec_doc = if r.doc.is_empty() {
            item_doc
        } else {
            r.doc.clone()
        };
        // The declaration's docs win over the struct's.
        let mut field_docs = item_field_docs;
        for (n, _, d, _) in &r.fields {
            if !d.is_empty() {
                field_docs.retain(|(k, _)| *n != *k);
                field_docs.push((n.to_string(), d.clone()));
            }
        }
        let flat = format_ident!("__flat_{}", ts);
        let build = format_ident!("__js_rec_{}", ts);
        let from_json = format_ident!("__from_json_{}", ts);
        let to_json = format_ident!("__to_json_{}", ts);
        let mut fields = Vec::new();
        let mut flat_stmts = Vec::new();
        let mut js_stmts = Vec::new();
        let mut json_stmts = Vec::new();
        let mut parse_fields = Vec::new();
        let mut flat_ok = true;
        for (fname, fty, _, fkey) in &r.fields {
            let key = fkey.clone().unwrap_or_else(|| camel(&fname.to_string()));
            let doc = field_docs
                .iter()
                .find(|(n, _)| *fname == *n)
                .map(|(_, d)| d.clone())
                .unwrap_or_default();
            let ty = match parse_rec_ty(fty, &args) {
                Ok(t) => t,
                Err(e) => return Err(syn::Error::new(fname.span(), e)),
            };
            let mty = rec_ty_manifest(&ty, &args);
            fields.push(quote!(::iron_api::RecordField { name: #key, ty: #mty, doc: #doc }));
            flat_ok &= rec_flat(&ty);
            if rec_flat(&ty) {
                flat_stmts.push(rec_flat_stmt(&ty, quote!(r.#fname), &args));
            }
            js_stmts.push(rec_js_stmt(&ty, &key, quote!((&r.#fname)), &args));
            json_stmts.push(rec_json_stmt(&ty, &key, quote!((&r.#fname)), &args));
            let parse = rec_parse_field(&ty, &key, &args);
            parse_fields.push(quote!(#fname: #parse?));
        }
        if flat_ok {
            record_items.push(quote! {
                #[allow(dead_code)]
                fn #flat(
                    r: &#rust,
                    out: &mut ::std::vec::Vec<f64>,
                    strs: &mut ::std::vec::Vec<::std::string::String>,
                ) {
                    let _ = (&out, &strs);
                    #( #flat_stmts )*
                }
            });
        }
        // The JSON forms: how a record travels as an argument or an optional result.
        record_items.push(quote! {
            #[allow(dead_code, unused_variables)]
            fn #from_json(v: &::iron_api::serde_json::Value) -> ::std::result::Result<#rust, ::std::string::String> {
                let ::std::option::Option::Some(obj) = v.as_object() else {
                    return ::std::result::Result::Err(::std::string::String::from("not an object"));
                };
                ::std::result::Result::Ok(#rust { #( #parse_fields ),* })
            }

            #[allow(dead_code)]
            fn #to_json(r: &#rust) -> ::iron_api::serde_json::Value {
                let mut m = ::iron_api::serde_json::Map::new();
                #( #json_stmts )*
                ::iron_api::serde_json::Value::Object(m)
            }
        });
        record_defs.push(
            quote!(::iron_api::RecordDef { name: #ts, doc: #rec_doc, fields: &[ #( #fields ),* ] }),
        );
        if args.quickjs {
            record_items.push(quote! {
                #[allow(dead_code)]
                fn #build<'js>(
                    ctx: &::iron_api::rquickjs::Ctx<'js>,
                    r: &#rust,
                ) -> ::iron_api::rquickjs::Result<::iron_api::rquickjs::Object<'js>> {
                    let o = ::iron_api::rquickjs::Object::new(ctx.clone())?;
                    #( #js_stmts )*
                    Ok(o)
                }
            });
        }
    }
    // A JS value-class instance from the Rust value, for results that hold values.
    if args.quickjs {
        for v in &args.values {
            let ts = &v.ts;
            let rust = &v.rust;
            let n = v.fields.len();
            let idx: Vec<usize> = (0..n).collect();
            let build = format_ident!("__js_value_{}", ts);
            record_items.push(quote! {
                #[allow(dead_code)]
                fn #build<'js>(
                    ctx: &::iron_api::rquickjs::Ctx<'js>,
                    v: #rust,
                ) -> ::iron_api::rquickjs::Result<::iron_api::rquickjs::Object<'js>> {
                    let parts = <#rust as ::iron_api::ValueType<#n>>::to_parts(v);
                    let class: ::iron_api::rquickjs::function::Constructor =
                        ctx.globals().get(#ts)?;
                    let raw: ::iron_api::rquickjs::Value = ctx.globals().get("__iron_raw")?;
                    let mut args = ::iron_api::rquickjs::function::Args::new(ctx.clone(), 1 + #n);
                    args.push_arg(raw)?;
                    #( args.push_arg(parts[#idx])?; )*
                    class.construct_args(args)
                }
            });
        }
    }
    let module_doc = doc_of(&module.attrs);
    let like = args.like;
    let alias_names: Vec<&String> = args.aliases.iter().map(|(a, _)| a).collect();
    let alias_targets: Vec<&String> = args.aliases.iter().map(|(_, t)| t).collect();

    let quickjs = if args.quickjs {
        quote! {
            /// Installs the namespace as a QuickJS global, with typed arguments (no `Value` array,
            /// no op queue). A handle is an opaque object of its class there (see `iron_api::qjs`).
            pub fn register_quickjs<'js>(
                ctx: &::iron_api::rquickjs::Ctx<'js>,
            ) -> ::iron_api::rquickjs::Result<()> {
                if ctx.globals().get::<_, ::iron_api::rquickjs::Value>("__iron_raw")?.is_undefined() {
                    ctx.globals().set("__iron_raw", ::iron_api::rquickjs::Object::new(ctx.clone())?)?;
                }
                #( #js_class_stmts )*
                let ns = #ns_init;
                #( #js_hidden )*
                #( #js_sets )*
                ::iron_api::qjs::install_namespace(ctx, #ns, ns)?;
                #(
                    ctx.globals().set(
                        #alias_names,
                        ctx.globals().get::<_, ::iron_api::rquickjs::Value>(#alias_targets)?,
                    )?;
                )*
                Ok(())
            }
        }
    } else {
        quote!()
    };

    let class_defs: Vec<TokenStream2> = class_order
        .iter()
        .zip(&class_entries)
        .map(|(c, fs)| {
            let doc = class_docs.get(c).cloned().unwrap_or_default();
            quote!(::iron_api::ClassDef { name: #c, doc: #doc, functions: &[ #( #fs ),* ] })
        })
        .collect();

    let attrs = &module.attrs;
    let vis = &module.vis;
    let ident = &module.ident;
    Ok(quote! {
        #(#attrs)*
        #vis mod #ident {
            #(#out_items)*

            /// What this namespace exports to scripts.
            pub const MANIFEST: ::iron_api::Namespace = ::iron_api::Namespace {
                symbols: ::iron_api::SymbolTable(iron_symbols),
                name: #ns,
                doc: #module_doc,
                like: #like,
                handles: &[ #( #handle_names ),* ],
                enums: &[ #( #enum_defs ),* ],
                records: &[ #( #record_defs ),* ],
                aliases: &[ #( (#alias_names, #alias_targets) ),* ],
                values: &[ #( #value_defs ),* ],
                classes: &[ #( #class_defs ),* ],
                functions: &[ #( #ns_entries ),* ],
            };

            #( #shims )*

            #( #enum_items )*
            #( #record_items )*

            /// The shims' names and addresses, for a JIT.
            pub fn iron_symbols() -> Vec<(&'static str, usize)> {
                vec![ #( #symbol_entries ),* ]
            }

            #quickjs
        }
    })
}
