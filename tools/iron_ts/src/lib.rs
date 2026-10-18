//! `iron_ts`: an ahead-of-time compiler for Iron's TypeScript gameplay code.
//! TypeScript (oxc) -> typed Iron IR -> Cranelift object file. See `docs/research/iron-ts-spike.md`.

pub mod codegen;
pub mod ir;
pub mod layout;
pub mod lower;

/// What a compile produced.
pub struct Output {
    pub object: Vec<u8>,
    pub bindings: String,
    pub ir: String,
    /// Wall-clock time of each phase.
    pub lower_time: std::time::Duration,
    pub codegen_time: std::time::Duration,
}

/// TypeScript source -> object file bytes, Rust bindings and an IR dump.
pub fn compile_source(src: &str, opt: &str) -> Result<Output, String> {
    // Lowering recurses with the source's nesting; the main thread's 1 MiB is too little on Windows.
    std::thread::scope(|s| {
        std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn_scoped(s, || compile_inner(src, opt))
            .map_err(|e| e.to_string())?
            .join()
            .map_err(|_| "compiler panicked".to_string())?
    })
}

fn compile_inner(src: &str, opt: &str) -> Result<Output, String> {
    let t0 = std::time::Instant::now();
    let module = lower::lower_source(src)?;
    let lower_time = t0.elapsed();
    let t1 = std::time::Instant::now();
    let compiled = codegen::compile(&module, opt)?;
    let codegen_time = t1.elapsed();
    Ok(Output {
        object: compiled.object,
        bindings: layout::rust_bindings(&module),
        ir: ir::dump(&module),
        lower_time,
        codegen_time,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // M9-TS-14
    #[test]
    fn a_discriminated_union_becomes_one_struct_with_every_variants_fields() {
        let m = lower::lower_source(
            r#"export type Ev = { kind: "a"; x: number } | { kind: "b"; y: boolean };
               export function mk(): Ev { return { kind: "b", y: true }; }"#,
        )
        .unwrap();
        let ev = m.structs.iter().find(|s| s.name == "Ev").unwrap();
        let names: Vec<_> = ev.fields.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, ["kind", "x", "y"]);
        assert_eq!(m.tags, ["a", "b"]);
    }

    // M9-TS-15
    #[test]
    fn unsupported_code_is_an_error_with_a_line_number() {
        let e = lower::lower_source(
            "export function f(x: number): number {\n  return typeof x === 'number' ? 1 : 0;\n}",
        )
        .err()
        .unwrap();
        assert!(e.contains("line 2"), "{e}");
        let e = lower::lower_source("export function g(x: any): number { return 1; }")
            .err()
            .unwrap();
        assert!(e.contains("unsupported type"), "{e}");
    }

    // M9-TS-16
    #[test]
    fn callbacks_are_inlined_and_the_object_has_exported_symbols() {
        let out = compile_source("export function total(xs: number[]): number { let s = 0; xs.forEach((x) => { s += x; }); return s; }", "speed").unwrap();
        assert!(!out.ir.contains("callc"), "{}", out.ir);
        assert!(out.object.windows(8).any(|w| w == b"ts_total"));
    }
}
