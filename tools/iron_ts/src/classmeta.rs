// SPDX-License-Identifier: Apache-2.0

//! The component class table as data. The generated bindings' `classes()` is Rust text, so only a
//! program linked at build time can use it; a host that JIT-compiles a game reads this instead.

use crate::ir::*;
use crate::layout::prop_kind;

/// One registered component class. Symbols are exported function names: a JIT looks up their
/// addresses.
#[derive(Clone, Debug)]
pub struct ClassMeta {
    pub name: String,
    /// The class struct id: what `getComponent(Class)` passes.
    pub sid: u64,
    /// The struct ids of the class's ancestors.
    pub bases: Vec<u64>,
    pub create: String,
    /// The nine hooks of `iron_ts_rt::NativeHost` (awake, start, update, ...), in its order.
    pub hooks: [Option<String>; 9],
    /// `onCollisionEnter/Exit/Stay` and `onTriggerEnter/Exit/Stay`.
    pub contact_hooks: [Option<String>; 6],
    /// The `@property` table: name, byte offset, type name.
    pub props: Vec<(String, usize, String)>,
}

const HOOKS: [&str; 9] = [
    "onAwake",
    "onStart",
    "onUpdate",
    "onLateUpdate",
    "onFixedUpdate",
    "onPreRender",
    "onDestroy",
    "onApplicationFocus",
    "onApplicationPause",
];

/// The table `layout::component_table` writes as Rust text, as data.
pub fn class_meta(m: &Module) -> Vec<ClassMeta> {
    let sym = |n: &str| format!("ts_{}{}", m.prefix, n);
    let mut out = Vec::new();
    for f in m.funcs.iter().filter(|f| f.exported) {
        let Some(class) = f.name.strip_prefix("iron_new_") else {
            continue;
        };
        let find = |h: &str| {
            let name = format!("iron_hook_{class}_{h}");
            m.funcs
                .iter()
                .any(|g| g.exported && g.name == name)
                .then(|| sym(&name))
        };
        let mut hooks: [Option<String>; 9] = Default::default();
        for (i, h) in HOOKS.iter().enumerate() {
            hooks[i] = find(h);
        }
        let mut contact_hooks: [Option<String>; 6] = Default::default();
        for (i, h) in crate::lower::components::CONTACT_HOOKS.iter().enumerate() {
            contact_hooks[i] = find(h);
        }
        let sid = match f.ret.strip() {
            Ty::Obj(sid) => *sid,
            _ => 0,
        };
        let props = m
            .props
            .iter()
            .filter(|t| t.class == sid)
            .flat_map(|t| &t.entries)
            .map(|e| {
                (
                    e.name.clone(),
                    field_offset(e.index) as usize,
                    prop_kind(m, &e.ty),
                )
            })
            .collect();
        let bases = m
            .class_bases
            .iter()
            .find(|(s, _)| *s == sid)
            .map(|(_, b)| b.iter().map(|x| *x as u64).collect())
            .unwrap_or_default();
        out.push(ClassMeta {
            name: class.to_string(),
            sid: sid as u64,
            bases,
            create: sym(&f.name),
            hooks,
            contact_hooks,
            props,
        });
    }
    out
}
