//! The engine API as the compiler sees it: the JSON manifest `iron_api` generates
//! (`Namespace::to_json`), read into a table of functions the TypeScript can call directly.
//!
//! ```json
//! {"namespace":"Demo","functions":[{"name":"add","symbol":"iron_Demo_add",
//!   "params":[{"name":"a","type":"f64"}],"returns":"f64"}]}
//! ```
//!
//! Types are `void`, `bool`, `f64`, `f32`, `i32`, `u32` and `handle:Name`.

use crate::ir::ExtTy;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug)]
pub struct ExternFn {
    pub symbol: String,
    pub params: Vec<(String, ExtTy)>,
    pub ret: ExtTy,
}

#[derive(Clone, Debug, Default)]
pub struct Manifest {
    /// Namespace -> function name -> function.
    pub namespaces: HashMap<String, HashMap<String, ExternFn>>,
    /// Handle type names (`Entity`): types scripts can name.
    pub handles: HashSet<String>,
}

fn ext_ty(s: &str) -> Result<ExtTy, String> {
    Ok(match s {
        "void" => ExtTy::Void,
        "bool" => ExtTy::Bool,
        "f64" => ExtTy::F64,
        "f32" => ExtTy::F32,
        "i32" => ExtTy::I32,
        "u32" => ExtTy::U32,
        other => match other.strip_prefix("handle:") {
            Some(name) => ExtTy::Handle(name.to_string()),
            None => return Err(format!("manifest: unknown type `{other}`")),
        },
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
        let mut fns = HashMap::new();
        for f in v["functions"]
            .as_array()
            .ok_or("manifest: missing `functions`")?
        {
            let name = f["name"]
                .as_str()
                .ok_or("manifest: a function needs a name")?;
            let symbol = f["symbol"]
                .as_str()
                .ok_or("manifest: a function needs a symbol")?
                .to_string();
            let mut params = Vec::new();
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
                params.push((pn, pt));
            }
            let ret = ext_ty(f["returns"].as_str().ok_or("manifest: missing `returns`")?)?;
            if let ExtTy::Handle(h) = &ret {
                self.handles.insert(h.clone());
            }
            fns.insert(
                name.to_string(),
                ExternFn {
                    symbol,
                    params,
                    ret,
                },
            );
        }
        self.namespaces.insert(ns, fns);
        Ok(())
    }
}
