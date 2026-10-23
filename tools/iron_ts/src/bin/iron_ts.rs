// SPDX-License-Identifier: Apache-2.0

//! `iron_ts <file.ts> [--dump-ir] [-o out.o] [--bindings out.rs] [--opt none|speed] [--time N]
//! [--prefix p_] [--struct-base N] [--manifest api.json]...`:
//! compile a TypeScript module to an object file. `--time N` repeats the compile N times and
//! prints the best time of each phase.
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(file) = args.first() else {
        eprintln!(
            "usage: iron_ts <file.ts> [--dump-ir] [-o out.o] [--bindings out.rs] [--opt none|speed] [--time N]"
        );
        std::process::exit(2);
    };
    let flag = |name: &str| {
        args.iter()
            .position(|a| a == name)
            .and_then(|i| args.get(i + 1))
            .cloned()
    };
    let opt = flag("--opt").unwrap_or_else(|| "speed".into());
    let src = std::fs::read_to_string(file).expect("read source");
    let prefix = flag("--prefix").unwrap_or_default();
    let base = flag("--struct-base")
        .and_then(|b| b.parse().ok())
        .unwrap_or(0);
    // `--manifest file.json` (repeatable): the engine API the source may call.
    let manifests: Vec<String> = args
        .windows(2)
        .filter(|w| w[0] == "--manifest")
        .map(|w| std::fs::read_to_string(&w[1]).expect("read manifest"))
        .collect();
    let compile = || match iron_ts::compile_with_manifests(&src, &opt, &prefix, base, &manifests) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    };
    let out = compile();
    if let Some(n) = flag("--time").and_then(|n| n.parse::<usize>().ok()) {
        let (mut lo, mut cg) = (out.lower_time, out.codegen_time);
        for _ in 1..n {
            let o = compile();
            lo = lo.min(o.lower_time);
            cg = cg.min(o.codegen_time);
        }
        println!(
            "best of {n}: parse+types+lower {lo:?}, cranelift {cg:?}, object {} bytes",
            out.object.len()
        );
    }
    if args.iter().any(|a| a == "--dump-ir") {
        println!("{}", out.ir);
    }
    if let Some(p) = flag("-o") {
        std::fs::write(p, &out.object).expect("write object");
    }
    if let Some(p) = flag("--bindings") {
        std::fs::write(p, &out.bindings).expect("write bindings");
    }
}
