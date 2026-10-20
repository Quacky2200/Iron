//! The engine API as the compiler sees it: the JSON manifest `iron_api` generates
//! (`Namespace::to_json`), read into a table of functions the TypeScript can call directly.
//!
//! ```json
//! {"namespace":"Demo","functions":[{"name":"add","symbol":"iron_Demo_add",
//!   "params":[{"name":"a","type":"f64"}],"returns":"f64"}]}
//! ```
//!
//! Types are `void`, `bool`, `f64`, `f32`, `i32`, `u32`, `handle:Name` and `value:Name`.
//!
//! A namespace may also list handle `classes`: each function has a `kind` (`static`, `method`, `get`,
//! `set`), and the ones that act on an object take it as their first parameter, named `this`.

use crate::ir::ExtTy;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug)]
pub struct ExternFn {
    pub symbol: String,
    pub params: Vec<(String, ExtTy)>,
    pub ret: ExtTy,
    /// Index of the first parameter a call may leave out (`params.len()`: none).
    pub optional_from: usize,
    /// Per parameter: the value a left-out one takes (a flag is 0 or 1), if the API says.
    pub defaults: Vec<Option<f64>>,
}

/// The functions of a handle class, by how scripts call them. Methods, getters and setters take
/// the receiver as their first parameter.
#[derive(Clone, Debug, Default)]
pub struct ClassFns {
    pub statics: HashMap<String, ExternFn>,
    pub methods: HashMap<String, ExternFn>,
    pub getters: HashMap<String, ExternFn>,
    pub setters: HashMap<String, ExternFn>,
}

#[derive(Clone, Debug, Default)]
pub struct Manifest {
    /// Namespace -> function name -> function.
    pub namespaces: HashMap<String, HashMap<String, ExternFn>>,
    /// Handle type names (`Entity`): types scripts can name.
    pub handles: HashSet<String>,
    /// Value classes (`Vector3`): their fields in order, each a scalar type.
    pub values: HashMap<String, Vec<(String, ExtTy)>>,
    /// Handle classes (`GameObject`) by name.
    pub classes: HashMap<String, ClassFns>,
    /// Functions declared more than once under one name, picked by the argument types.
    pub overloads: HashMap<(String, String), Vec<ExternFn>>,
    /// Properties of a namespace (`Time.deltaTime`, `Gizmos.color = c`): namespace -> name -> accessors.
    pub ns_props: HashMap<String, HashMap<String, NsProp>>,
}

/// A namespace property: its getter and setter (neither takes a receiver).
#[derive(Clone, Debug, Default)]
pub struct NsProp {
    pub getter: Option<ExternFn>,
    pub setter: Option<ExternFn>,
}

fn ext_ty(s: &str) -> Result<ExtTy, String> {
    Ok(match s {
        "void" => ExtTy::Void,
        "bool" => ExtTy::Bool,
        "f64" => ExtTy::F64,
        "f32" => ExtTy::F32,
        "i32" => ExtTy::I32,
        "u32" => ExtTy::U32,
        "string" => ExtTy::Str,
        other => {
            if other == "ptr" {
                ExtTy::Ptr
            } else if other == "f64s" {
                ExtTy::F64Arr
            } else if let Some(n) = other.strip_prefix("values:") {
                ExtTy::ValueArr(n.to_string())
            } else if let Some(n) = other.strip_prefix("handles:") {
                ExtTy::HandleArr(n.to_string())
            } else if let Some(inner) = other.strip_prefix("fn:") {
                ExtTy::Fn(Box::new(ext_ty(inner)?))
            } else if let Some(name) = other.strip_prefix("handle:") {
                ExtTy::Handle(name.to_string())
            } else if let Some(name) = other.strip_prefix("value:") {
                ExtTy::Value(name.to_string())
            } else {
                return Err(format!("manifest: unknown type `{other}`"));
            }
        }
    })
}

impl Manifest {
    /// Adds one namespace's JSON.
    pub fn add_json(&mut self, json: &str) -> Result<(), String> {
        let v: serde_json::Value =
            serde_json::from_str(json).map_err(|e| format!("manifest: {e}"))?;
        let ns = v["namespace"]
            .as_str()
            .ok_or("manifest: missing `namespace`")?
            .to_string();
        for val in v["values"].as_array().into_iter().flatten() {
            let vname = val["name"]
                .as_str()
                .ok_or("manifest: a value needs a name")?;
            let mut fields = Vec::new();
            for f in val["fields"]
                .as_array()
                .ok_or("manifest: a value needs fields")?
            {
                let fname = f["name"].as_str().ok_or("manifest: a field needs a name")?;
                let fty = ext_ty(f["type"].as_str().ok_or("manifest: a field needs a type")?)?;
                fields.push((fname.to_string(), fty));
            }
            self.values.insert(vname.to_string(), fields);
        }
        let mut fns = HashMap::new();
        for f in v["functions"]
            .as_array()
            .ok_or("manifest: missing `functions`")?
        {
            let (name, kind, func) = self.parse_fn(f)?;
            if kind == "get" || kind == "set" {
                let p = self
                    .ns_props
                    .entry(ns.clone())
                    .or_default()
                    .entry(name)
                    .or_default();
                if kind == "get" {
                    p.getter = Some(func);
                } else {
                    p.setter = Some(func);
                }
                continue;
            }
            if let Some(prev) = fns.insert(name.clone(), func.clone()) {
                let list = self.overloads.entry((ns.clone(), name)).or_default();
                if list.is_empty() {
                    list.push(prev);
                }
                list.push(func);
            }
        }
        for c in v["classes"].as_array().into_iter().flatten() {
            let cname = c["name"].as_str().ok_or("manifest: a class needs a name")?;
            // A class named like a value class adds methods to the value, not a handle type.
            if !self.values.contains_key(cname) {
                self.handles.insert(cname.to_string());
            }
            let mut class = ClassFns::default();
            for f in c["functions"]
                .as_array()
                .ok_or("manifest: a class needs `functions`")?
            {
                let (name, kind, func) = self.parse_fn(f)?;
                let table = match kind.as_str() {
                    "static" => &mut class.statics,
                    "method" => &mut class.methods,
                    "get" => &mut class.getters,
                    "set" => &mut class.setters,
                    other => return Err(format!("manifest: unknown function kind `{other}`")),
                };
                table.insert(name, func);
            }
            self.classes.insert(cname.to_string(), class);
        }
        self.namespaces.insert(ns, fns);
        Ok(())
    }

    /// One function entry: its script name, kind (`static` when absent) and signature.
    fn parse_fn(&mut self, f: &serde_json::Value) -> Result<(String, String, ExternFn), String> {
        let name = f["name"]
            .as_str()
            .ok_or("manifest: a function needs a name")?;
        let symbol = f["symbol"]
            .as_str()
            .ok_or("manifest: a function needs a symbol")?
            .to_string();
        let kind = f["kind"].as_str().unwrap_or("static").to_string();
        let mut params = Vec::new();
        let mut optional_from = usize::MAX;
        let mut defaults: Vec<Option<f64>> = Vec::new();
        for p in f["params"].as_array().ok_or("manifest: missing `params`")? {
            let pn = p["name"].as_str().unwrap_or("arg").to_string();
            let pt = ext_ty(
                p["type"]
                    .as_str()
                    .ok_or("manifest: a parameter needs a type")?,
            )?;
            if let ExtTy::Handle(h) = &pt {
                self.handles.insert(h.clone());
            }
            if p["optional"].as_bool() == Some(true) && optional_from > params.len() {
                optional_from = params.len();
            }
            defaults.push(p["default"].as_f64());
            params.push((pn, pt));
        }
        let ret = ext_ty(f["returns"].as_str().ok_or("manifest: missing `returns`")?)?;
        if let ExtTy::Handle(h) = &ret {
            self.handles.insert(h.clone());
        }
        Ok((
            name.to_string(),
            kind,
            ExternFn {
                symbol,
                optional_from: optional_from.min(params.len()),
                defaults,
                params,
                ret,
            },
        ))
    }
}
