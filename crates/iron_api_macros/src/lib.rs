//! `#[iron_api]`: see the `iron_api` crate.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{
    Expr, FnArg, Ident, Item, ItemFn, ItemMod, Lit, Meta, Pat, ReturnType, Token, Type, Visibility,
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
    if args.quickjs && !args.values.is_empty() {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            "value types are not available in the QuickJS bindings yet: drop `quickjs` or `values(..)`",
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
}

fn classify(ty: &Type, args: &Args) -> syn::Result<Kind> {
    if let Type::Tuple(t) = ty
        && t.elems.is_empty()
    {
        return Ok(Kind::Unit);
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
        Kind::Handle(ts, _) => quote!(::iron_api::Ty::Handle(#ts)),
        Kind::Value(i) => {
            let ts = &args.values[*i].ts;
            quote!(::iron_api::Ty::Value(#ts))
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

fn doc_of(f: &ItemFn) -> String {
    let mut lines = Vec::new();
    for a in &f.attrs {
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

struct Api {
    f: ItemFn,
    js_name: String,
    params: Vec<(Ident, Kind)>,
    ret: Kind,
}

fn api_of(f: &ItemFn, args: &Args) -> syn::Result<Api> {
    if f.sig.asyncness.is_some() || !f.sig.generics.params.is_empty() {
        return Err(syn::Error::new(
            f.sig.span(),
            "an iron_api function cannot be async or generic",
        ));
    }
    let mut params = Vec::new();
    for a in &f.sig.inputs {
        let FnArg::Typed(pt) = a else {
            return Err(syn::Error::new(
                a.span(),
                "an iron_api function takes no `self`",
            ));
        };
        let Pat::Ident(id) = &*pt.pat else {
            return Err(syn::Error::new(
                pt.pat.span(),
                "a plain parameter name is expected",
            ));
        };
        params.push((id.ident.clone(), classify(&pt.ty, args)?));
    }
    let ret = match &f.sig.output {
        ReturnType::Default => Kind::Unit,
        ReturnType::Type(_, t) => classify(t, args)?,
    };
    Ok(Api {
        js_name: camel(&f.sig.ident.to_string()),
        f: f.clone(),
        params,
        ret,
    })
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
        other => {
            let t = abi_ty(other);
            (vec![quote!(#name: #t)], from_abi(other, quote!(#name)))
        }
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
    let mut apis = Vec::new();
    for it in items {
        if let Item::Fn(f) = it
            && matches!(f.vis, Visibility::Public(_))
        {
            apis.push(api_of(f, &args)?);
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
    let mut entries = Vec::new();
    let mut shims = Vec::new();
    let mut js_sets = Vec::new();
    for a in &apis {
        let name = &a.f.sig.ident;
        let rust_name = name.to_string();
        let js_name = &a.js_name;
        let symbol = format!("iron_{ns}_{js_name}");
        let symbol_ident = format_ident!("{}", symbol);
        let doc = doc_of(&a.f);
        let pnames: Vec<String> = a.params.iter().map(|(n, _)| n.to_string()).collect();
        let ptys: Vec<TokenStream2> = a
            .params
            .iter()
            .map(|(_, k)| manifest_ty(k, &args))
            .collect();
        let ret_m = manifest_ty(&a.ret, &args);
        entries.push(quote! {
            ::iron_api::Function {
                name: #js_name,
                rust_name: #rust_name,
                symbol: #symbol,
                doc: #doc,
                params: &[ #( ::iron_api::Param { name: #pnames, ty: #ptys } ),* ],
                ret: #ret_m,
            }
        });

        // The extern "C" shim.
        let mut abi_params: Vec<TokenStream2> = Vec::new();
        let mut call_args: Vec<TokenStream2> = Vec::new();
        for (n, k) in &a.params {
            let (ps, arg) = shim_param(n, k, &args);
            abi_params.extend(ps);
            call_args.push(arg);
        }
        let call = quote!(#name( #( #call_args ),* ));
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
        } else {
            let abi_ret = abi_ty(&a.ret);
            let body = to_abi(&a.ret, call);
            shims.push(quote! {
                #[unsafe(no_mangle)]
                pub extern "C" fn #symbol_ident( #( #abi_params ),* ) -> #abi_ret {
                    #body
                }
            });
        }

        // The QuickJS function.
        if args.quickjs {
            let js_params: Vec<TokenStream2> = a
                .params
                .iter()
                .map(|(n, k)| {
                    let t = js_ty(k);
                    quote!(#n: #t)
                })
                .collect();
            let js_args: Vec<TokenStream2> = a
                .params
                .iter()
                .map(|(n, k)| from_js(k, quote!(#n)))
                .collect();
            let js_ret = js_ty(&a.ret);
            let js_body = to_js(&a.ret, quote!(#name( #( #js_args ),* )));
            js_sets.push(quote! {
                ns.set(
                    #js_name,
                    ::iron_api::rquickjs::Function::new(
                        ctx.clone(),
                        | #( #js_params ),* | -> #js_ret { #js_body },
                    )?,
                )?;
            });
        }
    }

    let quickjs = if args.quickjs {
        quote! {
            /// Installs the namespace as a QuickJS global, with typed arguments (no `Value` array,
            /// no op queue).
            pub fn register_quickjs<'js>(
                ctx: &::iron_api::rquickjs::Ctx<'js>,
            ) -> ::iron_api::rquickjs::Result<()> {
                let ns = ::iron_api::rquickjs::Object::new(ctx.clone())?;
                #( #js_sets )*
                ctx.globals().set(#ns, ns)?;
                Ok(())
            }
        }
    } else {
        quote!()
    };

    let attrs = &module.attrs;
    let vis = &module.vis;
    let ident = &module.ident;
    Ok(quote! {
        #(#attrs)*
        #vis mod #ident {
            #(#items)*

            /// What this namespace exports to scripts.
            pub const MANIFEST: ::iron_api::Namespace = ::iron_api::Namespace {
                name: #ns,
                handles: &[ #( #handle_names ),* ],
                values: &[ #( #value_defs ),* ],
                functions: &[ #( #entries ),* ],
            };

            #( #shims )*

            #quickjs
        }
    })
}
