//! The JIT: the same code generation as the object writer, into the running process
//! (`iron_ts::jit`). IDs are in TESTS.md (M9-TS-1900.., M9-API-600..).
use iron_api::Handle;
use iron_scene::Scene;
use iron_scene::api::scene_api as api;
use iron_ts::jit::{JitOptions, compile_and_load};
use iron_ts_rt::{NativeHost, link_anchor};

/// Compiled programs share the thread's heap and globals: one test at a time.
fn bench(name: &str) -> String {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("bench");
    std::fs::read_to_string(dir.join(name)).unwrap()
}

fn fnv(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, b| {
        (h ^ *b as u64).wrapping_mul(0x100_0000_01b3)
    })
}

// M9-TS-1900
//
// Making code generation generic over the Cranelift module type must not change the object file:
// the sizes and hashes below were taken from the output before the change.
#[test]
fn the_object_writer_output_is_unchanged_by_the_generic_codegen() {
    for (file, len, hash) in [
        ("sine.ts", 5797, 0xecf0_6a7f_a936_0f7c),
        ("strings.ts", 30284, 0xf748_7f33_a239_453d),
        ("classes.ts", 16441, 0xac99_c71c_2bad_5f21),
    ] {
        let out = iron_ts::compile_source(&bench(file), "speed").unwrap();
        assert_eq!(out.object.len(), len, "{file}");
        assert_eq!(fnv(&out.object), hash, "{file}");
    }
}

// M9-TS-1901
//
// A plain program compiled in the process: an exported function is called through its address.
#[test]
fn a_program_compiled_in_process_runs() {
    let _serial = iron_ts_tests::serial();
    assert_ne!(link_anchor(), 0);
    let game = compile_and_load(
        "export function add(a: number, b: number): number { return a * 2 + b; }\n\
         export function fib(n: number): number { return n < 2 ? n : fib(n - 1) + fib(n - 2); }",
        &JitOptions {
            prefix: "jit1_",
            struct_base: 90_000,
            ..Default::default()
        },
    )
    .unwrap();
    game.init().unwrap();
    let add: unsafe extern "C" fn(f64, f64) -> f64 =
        unsafe { std::mem::transmute(game.function("add").unwrap()) };
    let fib: unsafe extern "C" fn(f64) -> f64 =
        unsafe { std::mem::transmute(game.function("fib").unwrap()) };
    assert_eq!(unsafe { add(3.0, 4.0) }, 10.0);
    assert_eq!(unsafe { fib(15.0) }, 610.0);
    assert!(game.function_names().contains(&"add".to_string()));
}

// M9-TS-1902
//
// Strings, arrays and the runtime's collector work from JIT code (imports of `ts_rt_*` are
// resolved from the generated table).
#[test]
fn jit_code_uses_the_runtime() {
    let _serial = iron_ts_tests::serial();
    let game = compile_and_load(
        "export function words(n: number): number {\n\
           const parts: string[] = [];\n\
           for (let i = 0; i < n; i++) parts.push(`w${i}`);\n\
           return parts.join(\",\").length + Math.floor(n / 3) + Math.ceil(n / 3);\n\
         }",
        &JitOptions {
            prefix: "jit2_",
            struct_base: 91_000,
            ..Default::default()
        },
    )
    .unwrap();
    game.init().unwrap();
    let f: unsafe extern "C" fn(f64) -> f64 =
        unsafe { std::mem::transmute(game.function("words").unwrap()) };
    // "w0,w1,...,w9" is 10 * 2 + 9 = 29, plus 3 and 4.
    assert_eq!(unsafe { f(10.0) }, 36.0);
}

// M9-TS-1903
//
// A missing symbol is reported by name before anything runs.
#[test]
fn an_unresolved_engine_function_is_an_error_naming_it() {
    let manifest = r#"{"namespace":"Nope","functions":[{"name":"gone","symbol":"iron_Nope_gone","params":[],"returns":"f64"}]}"#;
    let e = compile_and_load(
        "export function f(): number { return Nope.gone(); }",
        &JitOptions {
            prefix: "jit3_",
            struct_base: 92_000,
            manifests: &[manifest.to_string()],
            ..Default::default()
        },
    );
    match e {
        Ok(_) => panic!("expected an error"),
        Err(e) => assert!(e.contains("iron_Nope_gone"), "{e}"),
    }
}

fn scene_manifests() -> Vec<String> {
    let mut v = vec![iron_scene::api::scene_api::MANIFEST.to_json()];
    v.extend(iron_engine_api::iron_math::manifests());
    v
}

// M9-TS-1904
//
// The component program of `bench/components.ts` (engine calls into the real scene) JIT-compiled:
// the class table is read at run time and `NativeHost` drives the components.
#[test]
fn a_jit_compiled_component_table_drives_the_native_host() {
    let _serial = iron_ts_tests::serial();
    let game = compile_and_load(
        &bench("components.ts"),
        &JitOptions {
            prefix: "jit4_",
            struct_base: 93_000,
            manifests: &scene_manifests(),
            ..Default::default()
        },
    )
    .unwrap();
    let names: Vec<_> = game.class_meta().iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, ["Mover", "Fast", "Faulty", "Physics"]);
    let mut scene = Scene::new();
    let _scope = iron_scene::api::enter(&mut scene);
    game.init().unwrap();
    let classes = game.classes().unwrap();
    let have = |c: usize| -> Vec<bool> { classes[c].hooks.iter().map(|h| h.is_some()).collect() };
    assert_eq!(
        have(0),
        [true, true, true, true, false, false, true, false, false]
    );
    let go = api::create();
    let mut host = NativeHost::new(classes);
    let id = host.spawn("Mover", Some(go.to_raw())).unwrap();
    for _ in 0..3 {
        assert!(host.update(0.5, &|_| true).is_empty());
    }
    assert_eq!(api::local_position(go).x, 3.0);
    host.destroy(id);
    api::destroy(go);
}

// M9-API-600
//
// Reading a namespace's manifest registers the addresses of its shims, and every symbol any
// manifest names resolves: nothing is listed by hand.
#[test]
fn every_manifest_function_symbol_resolves_after_reading_the_manifests() {
    let mut all = scene_manifests();
    all.extend(iron_engine_api::manifests());
    all.extend(iron_scenes::manifests());
    all.extend(iron_physics_api::manifests());
    all.extend(iron_render_api::manifests());
    let mut missing = Vec::new();
    let mut n = 0;
    for m in &all {
        let v: serde_json::Value = serde_json::from_str(m).unwrap();
        let mut stack = vec![&v];
        while let Some(x) = stack.pop() {
            match x {
                serde_json::Value::Object(o) => {
                    if let Some(s) = o.get("symbol").and_then(|s| s.as_str()) {
                        n += 1;
                        if iron_api::symbol(s).is_none() {
                            missing.push(s.to_string());
                        }
                    }
                    stack.extend(o.values());
                }
                serde_json::Value::Array(a) => stack.extend(a),
                _ => {}
            }
        }
    }
    assert!(n > 500, "only {n} symbols were found");
    assert!(missing.is_empty(), "{missing:?}");
}

// M9-API-601
//
// The runtime's own table (generated by its build script) holds the runtime functions.
#[test]
fn the_runtime_symbol_table_lists_its_exports() {
    let t = iron_ts_rt::symbols();
    assert!(t.len() > 200, "{}", t.len());
    for want in ["ts_rt_obj_new", "ts_rt_str_trim_gc", "ts_rt_tan"] {
        assert!(t.iter().any(|(n, a)| *n == want && *a != 0), "{want}");
    }
}

// M9-TS-2009
//
// A component class that no `register(..)` names is still in the class table (so
// `addComponent(Class)` finds it) and is marked as not registered.
#[test]
fn an_unregistered_component_class_is_in_the_table_but_not_registered() {
    let _serial = iron_ts_tests::serial();
    let src = "class Started extends Component { n: number = 1; onUpdate(dt: number): void { this.n++; } }\n\
               class OnlyAdded extends Started { m: number = 2; }\n\
               register({ Started });\n";
    let game = compile_and_load(
        src,
        &JitOptions {
            prefix: "jit5_",
            struct_base: 94_000,
            manifests: &scene_manifests(),
            ..Default::default()
        },
    )
    .unwrap();
    let meta: Vec<_> = game
        .class_meta()
        .iter()
        .map(|c| (c.name.as_str(), c.registered))
        .collect();
    assert_eq!(meta, [("Started", true), ("OnlyAdded", false)]);
}
