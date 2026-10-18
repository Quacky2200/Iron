//! `iron_ts <file.ts> [--dump-ir] [-o out.o] [--bindings out.rs] [--opt none|speed] [--time N]`:
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
    let compile = || match iron_ts::compile_source(&src, &opt) {
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
