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

struct Args {
    namespace: String,
    quickjs: bool,
    /// (TypeScript name, Rust type name).
    handles: Vec<(String, String)>,
    values: Vec<ValueDecl>,
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
        handles: Vec::new(),
        values: Vec::new(),
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
                    "expected `namespace = \"..\"`, `quickjs`, `handles(..)` or `values(..)`",
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
    /// `&[V]` for a declared value type (index into `Args::values`): a script `Vector3[]`.
    Values(usize),
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
        if let Some(i) = args
            .values
            .iter()
            .position(|v| rust_name_of(&v.rust).as_deref() == elem.as_deref())
        {
            return Ok(Kind::Values(i));
        }
        return Err(syn::Error::new(
            ty.span(),
            "a slice parameter is `&[f64]` or `&[V]` for a type declared in `values(..)`",
        ));
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
        Kind::Values(i) => {
            let ts = &args.values[*i].ts;
            let arr = format!("{ts}[]");
            quote!(::iron_api::Ty::Values(#ts, #arr))
        }
    }
}

/// The C ABI type of a value (a value type is passed as its fields, see `shim_param`).
fn abi_ty(k: &Kind) -> TokenStream2 {
    match k {
        Kind::Unit | Kind::Value(_) => quote!(()),
        Kind::Bool => quote!(bool),
        Kind::F64 => quote!(f64),
        Kind::F32 => quote!(f32),
        Kind::I32 => quote!(i32),
        Kind::U32 => quote!(u32),
        Kind::Handle(..) => quote!(u64),
        Kind::Str(_) | Kind::F64s | Kind::Values(_) => quote!(*const u8),
    }
}

/// The type a QuickJS function takes or returns for it (handles travel as numbers).
fn js_ty(k: &Kind) -> TokenStream2 {
    match k {
        Kind::Handle(..) => quote!(f64),
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
        Kind::Str(_) => quote!(::std::string::String),
        Kind::Value(_) => quote!(::iron_api::rquickjs::Object<'js>),
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
        Kind::Handle(..) => quote!(0.0),
        Kind::Str(_) => quote!(::std::string::String::new()),
        _ => quote!(compile_error!("this parameter type cannot be optional")),
    }
}

/// The type a QuickJS function returns for a result.
fn js_ret_ty(k: &Kind) -> TokenStream2 {
    match k {
        Kind::Str(_) => quote!(::std::string::String),
        Kind::Value(_) => quote!(::iron_api::rquickjs::Object<'js>),
        other => js_ty(other),
    }
}

/// The Rust argument built from the JS parameter `x` (a value reads its fields from the object).
fn js_arg(k: &Kind, x: TokenStream2, args: &Args) -> TokenStream2 {
    match k {
        Kind::Str(false) => quote!(&#x),
        Kind::Str(true) => x,
        Kind::Value(i) => {
            let v = &args.values[*i];
            let n = v.fields.len();
            let rust = &v.rust;
            let names: Vec<&String> = v.fields.iter().map(|(n, _)| n).collect();
            quote! {
                <#rust as ::iron_api::ValueType<#n>>::from_parts([
                    #( #x.get::<_, f64>(#names)? ),*
                ])
            }
        }
        other => from_js(other, x),
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
        other => {
            let e = to_js(other, call);
            quote!(Ok(#e))
        }
    }
}

fn from_js(k: &Kind, x: TokenStream2) -> TokenStream2 {
    match k {
        Kind::Handle(..) => from_abi(k, quote!(#x as u64)),
        _ => x,
    }
}

fn to_abi(k: &Kind, x: TokenStream2) -> TokenStream2 {
    match k {
        Kind::Handle(..) => quote!(::iron_api::Handle::to_raw(#x)),
        _ => x,
    }
}

fn to_js(k: &Kind, x: TokenStream2) -> TokenStream2 {
    match k {
        Kind::Handle(..) => quote!(::iron_api::Handle::to_raw(#x) as f64),
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
    ["property", "optional", "method", "getter", "js_name"]
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
    /// A method of a value class: the receiver (first parameter) travels as the value's fields.
    value_recv: Option<(Ident, Kind)>,
    /// Not including the receiver.
    params: Vec<(Ident, Kind)>,
    /// Names of the trailing parameters a call may leave out (`#[optional(a, b = 1.5)]`).
    optional: Vec<String>,
    /// The defaults given (`b = 1.5`; a flag is 0 or 1).
    defaults: Vec<(String, f64)>,
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
                params.push((id.ident.clone(), classify(&pt.ty, args)?));
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
        (false, true) if class.is_none() && params.is_empty() => Tag::Get,
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
    // Only trailing parameters can be left out.
    let first_opt = params
        .iter()
        .position(|(p, _)| optional.contains(&p.to_string()));
    if let Some(i) = first_opt
        && params[i..]
            .iter()
            .any(|(p, _)| !optional.contains(&p.to_string()))
    {
        return Err(syn::Error::new(
            sig.span(),
            "#[optional(..)] parameters must be the last ones",
        ));
    }
    Ok(Api {
        name: sig.ident.clone(),
        doc: doc_of(attrs),
        js_name: js_name_attr(attrs).unwrap_or_else(|| stem.clone()),
        stem,
        class,
        tag,
        value_recv: None,
        params,
        optional,
        defaults,
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
        Kind::F64s => (
            vec![quote!(#name: *const u8)],
            quote!(unsafe { ::iron_api::script_f64s(#name) }),
        ),
        Kind::Values(i) => {
            let v = &args.values[*i];
            let n = v.fields.len();
            let rust = &v.rust;
            (
                vec![quote!(#name: *const u8)],
                quote!(&unsafe { ::iron_api::script_values::<#n, #rust>(#name) }),
            )
        }
        other => {
            let t = abi_ty(other);
            (vec![quote!(#name: #t)], from_abi(other, quote!(#name)))
        }
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
    let getters: Vec<(String, String, String)> = apis
        .iter()
        .filter(|a| a.tag == Tag::Get)
        .map(|a| {
            (
                a.class
                    .as_ref()
                    .map_or_else(|| args.namespace.clone(), |c| c.0.clone()),
                a.js_name.clone(),
                manifest_ty(&a.ret, &args).to_string(),
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
        let settable = if a.class.is_some() {
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
            let ty = manifest_ty(&a.params[0].1, &args).to_string();
            if getters
                .iter()
                .any(|(c, n, t)| c == class && *n == prop && *t == ty)
            {
                a.tag = Tag::Set;
                a.js_name = prop;
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
    let mut js_sets = Vec::new();
    // Namespace functions by script name: a name with several versions (overloads) dispatches by
    // the number of arguments.
    let mut js_fns: std::collections::BTreeMap<String, Vec<(TokenStream2, usize, usize)>> =
        std::collections::BTreeMap::new();
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
        let doc = &a.doc;
        let has_receiver = a.class.is_some()
            && matches!(a.tag, Tag::Method | Tag::Get | Tag::Set)
            && a.value_recv.is_none();

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
        let mut pdefaults: Vec<TokenStream2> =
            vec![quote!(::std::option::Option::None); pnames.len()];
        for (n, k) in &a.params {
            pnames.push(camel(&n.to_string()));
            ptys.push(manifest_ty(k, &args));
            popts.push(a.optional.contains(&n.to_string()));
            pdefaults.push(match a.defaults.iter().find(|d| *n == d.0) {
                Some((_, v)) => quote!(::std::option::Option::Some(#v)),
                None => quote!(::std::option::Option::None),
            });
        }
        let ret_m = manifest_ty(&a.ret, &args);
        let kind = kind_tokens(a.tag);
        let entry = quote! {
            ::iron_api::Function {
                kind: #kind,
                name: #js_name,
                rust_name: #rust_name,
                symbol: #symbol,
                doc: #doc,
                params: &[ #( ::iron_api::Param { name: #pnames, ty: #ptys, optional: #popts, default: #pdefaults } ),* ],
                ret: #ret_m,
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
        if let Kind::Value(i) = &a.ret {
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
            let body = to_abi(&a.ret, call);
            // A string parameter is a pointer to a script string: the shim is unsafe to call.
            if a.params.iter().any(|(_, k)| matches!(k, Kind::Str(_))) {
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

        // The QuickJS function (namespace functions only: a handle is a plain number there). A
        // string is a JS string, a value is an object with the fields' names, a result is a new one.
        if args.quickjs && (a.class.is_none() || a.value_recv.is_some()) {
            // A value method takes the value first (an object), then its own parameters.
            let all_params: Vec<&(Ident, Kind)> = a.value_recv.iter().chain(&a.params).collect();
            let is_opt = |n: &Ident| a.optional.contains(&n.to_string());
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
                    let d = match a.defaults.iter().find(|d| *n == d.0) {
                        Some((_, v)) => js_default_value(k, *v),
                        None => js_default(k),
                    };
                    quote!(let #n = #n.0.flatten().unwrap_or(#d);)
                })
                .collect();
            let js_args: Vec<TokenStream2> = all_params
                .iter()
                .map(|(n, k)| js_arg(k, quote!(#n), &args))
                .collect();
            let js_ret = js_ret_ty(&a.ret);
            let js_body = js_result(&a.ret, quote!(#name( #( #js_args ),* )), &args);
            // rquickjs takes a few typed parameters; a function with more reads its arguments from
            // the rest list one by one.
            let closure = if all_params.len() > 4 {
                let binds: Vec<TokenStream2> = all_params
                    .iter()
                    .enumerate()
                    .map(|(i, (n, k))| {
                        let t = js_param_ty(k);
                        if is_opt(n) {
                            let d = match a.defaults.iter().find(|d| *n == d.0) {
                                Some((_, v)) => js_default_value(k, *v),
                                None => js_default(k),
                            };
                            quote! {
                                let #n: #t = match rest.0.get(#i) {
                                    Some(v) if !v.is_undefined() && !v.is_null() => {
                                        ::iron_api::rquickjs::FromJs::from_js(&ctx, v.clone())?
                                    }
                                    _ => #d,
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
                      rest: ::iron_api::rquickjs::function::Rest<::iron_api::rquickjs::Value<'js>> |
                        -> ::iron_api::rquickjs::Result<#js_ret> {
                        #( #binds )*
                        #js_body
                    }
                }
            } else {
                quote! {
                    | ctx: ::iron_api::rquickjs::Ctx<'js>, #( #js_params ),* |
                        -> ::iron_api::rquickjs::Result<#js_ret> {
                        let _ = &ctx;
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
            // A value class's `create` is also what `new X(..)` calls.
            if a.class.is_none()
                && a.tag == Tag::Static
                && js_name == "create"
                && args.values.iter().any(|v| v.ts == args.namespace)
            {
                let hidden = format!("__iron_create_{}", args.namespace);
                js_hidden.push(quote! {
                    ctx.globals().set(
                        #hidden,
                        ::iron_api::rquickjs::Function::new(ctx.clone(), #closure)?,
                    )?;
                });
            }
            match a.tag {
                // A namespace property: an accessor on the namespace object.
                Tag::Get => js_props.entry(js_name.clone()).or_default().0 = Some(closure),
                Tag::Set => js_props.entry(js_name.clone()).or_default().1 = Some(closure),
                _ => {
                    let max = a.params.len() + usize::from(a.value_recv.is_some());
                    let min = max
                        - a.params
                            .iter()
                            .filter(|(n, _)| a.optional.contains(&n.to_string()))
                            .count();
                    js_fns
                        .entry(js_name.clone())
                        .or_default()
                        .push((closure, min, max));
                }
            }
        }
    }
    for (name, versions) in &js_fns {
        if let [(closure, _, _)] = versions.as_slice() {
            js_sets.push(quote! {
                ns.set(
                    #name,
                    ::iron_api::rquickjs::Function::new(ctx.clone(), #closure)?,
                )?;
            });
            continue;
        }
        // Several versions: each is a hidden function, and a dispatcher picks one by argument count.
        let mut branches = String::new();
        for (i, (closure, min, max)) in versions.iter().enumerate() {
            let hidden = format!("__iron_ov_{}_{name}_{i}", args.namespace);
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
            ns.set(#name, ctx.eval::<::iron_api::rquickjs::Function, _>(#src)?)?;
        });
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
        let mut src = format!(
            "globalThis.{ts} ??= class {ts} {{ constructor(...a) {{ if (a[0] === globalThis.__iron_raw) {{ {} }} else if (globalThis.__iron_create_{ts}) {{ const v = globalThis.__iron_create_{ts}(...a); {} }} else {{ {} }} }} }};",
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
    let quickjs = if args.quickjs {
        quote! {
            /// Installs the namespace as a QuickJS global, with typed arguments (no `Value` array,
            /// no op queue). Handle classes are not available there: a handle is a plain number.
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
                ctx.globals().set(#ns, ns)?;
                Ok(())
            }
        }
    } else {
        quote!()
    };

    let class_defs: Vec<TokenStream2> = class_order
        .iter()
        .zip(&class_entries)
        .map(|(c, fs)| quote!(::iron_api::ClassDef { name: #c, functions: &[ #( #fs ),* ] }))
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
                name: #ns,
                handles: &[ #( #handle_names ),* ],
                values: &[ #( #value_defs ),* ],
                classes: &[ #( #class_defs ),* ],
                functions: &[ #( #ns_entries ),* ],
            };

            #( #shims )*

            #quickjs
        }
    })
}
