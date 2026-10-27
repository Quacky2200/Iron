// SPDX-License-Identifier: Apache-2.0

//! Every function of every manifest exists in QuickJS. IDs are in TESTS.md (M9-API-*).
mod common;
use serde_json::Value;

fn with_all<R>(f: impl FnOnce(&rquickjs::Ctx<'_>) -> R) -> R {
    let rt = rquickjs::Runtime::new().unwrap();
    let ctx = rquickjs::Context::full(&rt).unwrap();
    ctx.with(|ctx| {
        iron_native::register_quickjs(&ctx).unwrap();
        f(&ctx)
    })
}

// M9-API-120
//
// The manifests are what compiled code can call; every namespace function, class static, method and
// property of them has a QuickJS counterpart (so a script runs the same API both ways).
#[test]
fn every_manifest_function_exists_in_quickjs() {
    with_all(|ctx| {
        let mut missing = Vec::new();
        let mut count = 0;
        for text in iron_native::manifests() {
            let m: Value = serde_json::from_str(&text).unwrap();
            let ns = m["namespace"].as_str().unwrap().to_string();
            for f in m["functions"].as_array().into_iter().flatten() {
                let name = f["name"].as_str().unwrap();
                count += 1;
                let ok: bool = ctx
                    .eval(format!(
                        "(() => {{ const d = Object.getOwnPropertyDescriptor({ns}, '{name}'); \
                         return !!d && (typeof d.value === 'function' || !!d.get || !!d.set); }})()"
                    ))
                    .unwrap_or(false);
                if !ok {
                    missing.push(format!("{ns}.{name}"));
                }
            }
            for c in m["classes"].as_array().into_iter().flatten() {
                let class = c["name"].as_str().unwrap();
                // Value classes' members are on the prototype too.
                for f in c["functions"].as_array().unwrap() {
                    let name = f["name"].as_str().unwrap();
                    let kind = f["kind"].as_str().unwrap();
                    count += 1;
                    // A getter or setter without `this` is a static property of the class.
                    let has_this = f["params"][0]["name"].as_str() == Some("this");
                    let target = if kind == "static" || (!has_this && kind != "method") {
                        class.to_string()
                    } else {
                        format!("{class}.prototype")
                    };
                    let ok: bool = ctx
                        .eval(format!(
                            "(() => {{ const d = Object.getOwnPropertyDescriptor({target}, '{name}'); \
                             return !!d && (typeof d.value === 'function' || !!d.get || !!d.set); }})()"
                        ))
                        .unwrap_or(false);
                    if !ok {
                        missing.push(format!("{class}.{kind}.{name}"));
                    }
                }
            }
        }
        assert!(
            missing.is_empty(),
            "{} of {count} missing: {missing:?}",
            missing.len()
        );
        assert!(count > 1500, "{count}");
    });
}

// M9-API-144
//
// Every component class with `attach` (and assets with a no-argument `create`): each property that
// has a getter and a setter reads, writes the same value back and reads it again unchanged. That
// covers enum-like strings, numbers, booleans, value classes, handles and the null of a handle.
#[test]
fn every_component_property_round_trips() {
    common::with_world(|ctx| {
        let mut failures = Vec::new();
        let mut checked = 0;
        for text in iron_native::manifests() {
            let m: Value = serde_json::from_str(&text).unwrap();
            for c in m["classes"].as_array().into_iter().flatten() {
                let class = c["name"].as_str().unwrap();
                let funcs = c["functions"].as_array().unwrap();
                let has = |kind: &str, name: &str| {
                    funcs.iter().any(|f| f["kind"] == kind && f["name"] == name)
                };
                let make = if has("static", "attach") {
                    format!("{class}.attach(GameObject.create())")
                } else if funcs.iter().any(|f| {
                    f["kind"] == "static"
                        && f["name"] == "create"
                        && f["params"]
                            .as_array()
                            .is_some_and(|p| p.iter().all(|x| x["optional"] == true))
                }) && !matches!(class, "GameObject" | "Rng")
                {
                    format!("{class}.create()")
                } else {
                    continue;
                };
                let props: Vec<&str> = funcs
                    .iter()
                    .filter(|f| f["kind"] == "get")
                    .map(|f| f["name"].as_str().unwrap())
                    .filter(|n| has("set", n))
                    .collect();
                if props.is_empty() {
                    continue;
                }
                let code = format!(
                    "(() => {{ const o = {make}; const bad = []; \
                     for (const p of {props:?}) {{ \
                       try {{ const v = o[p]; const before = JSON.stringify(v); o[p] = v; \
                              const after = JSON.stringify(o[p]); \
                              if (before !== after) bad.push(p + ' ' + before + ' -> ' + after); \
                       }} catch (e) {{ bad.push(p + ' threw ' + e.message); }} }} \
                     return bad.join('; '); }})()"
                );
                let bad: String = common::js(ctx, &code);
                checked += props.len();
                if !bad.is_empty() {
                    failures.push(format!("{class}: {bad}"));
                }
            }
        }
        assert!(failures.is_empty(), "{failures:#?}");
        assert!(checked > 200, "{checked}");
    });
}
