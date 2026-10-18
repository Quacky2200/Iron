//! Compiles the zombie game's sim.ts with `iron_ts` and links the object into this crate's targets.
use std::path::PathBuf;

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let src_path = manifest.join("fixtures/sim.ts");
    println!("cargo:rerun-if-changed={}", src_path.display());
    println!("cargo:rerun-if-changed=build.rs");
    let src = std::fs::read_to_string(&src_path).expect("read sim.ts");
    let t = std::time::Instant::now();
    let out = iron_ts::compile_source(&src, "speed").unwrap_or_else(|e| panic!("iron_ts: {e}"));
    println!(
        "cargo:warning=iron_ts compiled sim.ts in {:?} (lower {:?}, cranelift {:?})",
        t.elapsed(),
        out.lower_time,
        out.codegen_time
    );
    let dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let obj = dir.join("sim.obj");
    std::fs::write(&obj, &out.object).unwrap();
    std::fs::write(dir.join("sim_bindings.rs"), &out.bindings).unwrap();
    std::fs::write(dir.join("sim.ir.txt"), &out.ir).unwrap();
    println!("cargo:rustc-link-arg={}", obj.display());

    // Other modules linked next to it (symbols `ts_<prefix>*`, bindings `<name>_bindings.rs`).
    for (name, prefix, base) in [
        ("sine", "sine_", 1000),
        ("gap", "gap_", 2000),
        ("closures", "clo_", 3000),
        ("globals", "glob_", 4000),
        ("loops", "loop_", 5000),
        ("switches", "sw_", 6000),
        ("destructure", "des_", 7000),
    ] {
        let path = manifest.join(format!("bench/{name}.ts"));
        println!("cargo:rerun-if-changed={}", path.display());
        let out = iron_ts::compile_with(
            &std::fs::read_to_string(&path).unwrap(),
            "speed",
            prefix,
            base,
        )
        .unwrap_or_else(|e| panic!("iron_ts {name}: {e}"));
        let obj = dir.join(format!("{name}.obj"));
        std::fs::write(&obj, &out.object).unwrap();
        std::fs::write(dir.join(format!("{name}_bindings.rs")), &out.bindings).unwrap();
        println!("cargo:rustc-link-arg={}", obj.display());
    }
}
