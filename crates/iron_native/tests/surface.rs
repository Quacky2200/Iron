//! Every function of every manifest exists in QuickJS. IDs are in TESTS.md (M9-API-*).
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
                    let target = if kind == "static" {
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
