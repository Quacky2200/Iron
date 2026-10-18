//! Compiles the zombie game's sim.ts with `iron_ts` and links the object into this crate's targets.
use std::path::PathBuf;

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let src_path = manifest.join("../../examples/topdown-zombie-game/src/sim.ts");
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

    // The sine-circle curve maths, linked next to it (symbols `ts_sine_*`).
    let sine_path = manifest.join("bench/sine.ts");
    println!("cargo:rerun-if-changed={}", sine_path.display());
    let sine = iron_ts::compile_with_prefix(
        &std::fs::read_to_string(&sine_path).unwrap(),
        "speed",
        "sine_",
    )
    .unwrap_or_else(|e| panic!("iron_ts sine: {e}"));
    let obj = dir.join("sine.obj");
    std::fs::write(&obj, &sine.object).unwrap();
    std::fs::write(dir.join("sine_bindings.rs"), &sine.bindings).unwrap();
    println!("cargo:rustc-link-arg={}", obj.display());
}
