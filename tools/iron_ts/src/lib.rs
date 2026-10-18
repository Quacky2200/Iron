//! `iron_ts`: an ahead-of-time compiler for Iron's TypeScript gameplay code.
//! TypeScript (oxc) -> typed Iron IR -> Cranelift object file. See `docs/research/iron-ts-spike.md`.

pub mod codegen;
pub mod frames;
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
    compile_with_prefix(src, opt, "")
}

/// Like `compile_source`, naming exported symbols `ts_<prefix><name>`.
pub fn compile_with_prefix(src: &str, opt: &str, prefix: &str) -> Result<Output, String> {
    compile_with(src, opt, prefix, 0)
}

/// Like `compile_with_prefix`, with object headers numbered from `struct_base`. Modules that
/// share a process (and so a GC heap) need disjoint ranges: leave 1000 or more between them.
pub fn compile_with(
    src: &str,
    opt: &str,
    prefix: &str,
    struct_base: u32,
) -> Result<Output, String> {
    // Lowering recurses with the source's nesting; the main thread's 1 MiB is too little on Windows.
    std::thread::scope(|s| {
        std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn_scoped(s, || compile_inner(src, opt, prefix, struct_base))
            .map_err(|e| e.to_string())?
            .join()
            .map_err(|_| "compiler panicked".to_string())?
    })
}

fn compile_inner(src: &str, opt: &str, prefix: &str, struct_base: u32) -> Result<Output, String> {
    let t0 = std::time::Instant::now();
    let mut module = lower::lower_source(src)?;
    module.prefix = prefix.to_string();
    module.struct_base = struct_base;
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

    // M9-TS-55
    #[test]
    fn a_let_with_a_literal_initialiser_is_a_global_not_a_folded_constant() {
        let m = lower::lower_source(
            "let n = 0; const k = 2; export function f(): number { n = n + k; return n; }",
        )
        .unwrap();
        // `n` is a slot written by `f`; `k` is folded away.
        assert_eq!(m.globals.len(), 1);
        assert_eq!(m.globals[0].name, "n");
        let f = m.funcs.iter().find(|f| f.name == "f").unwrap();
        assert!(
            matches!(f.body[0], ir::Stmt::StoreGlobal(0, _)),
            "{:?}",
            f.body
        );
    }

    // M9-TS-56
    #[test]
    fn assigning_a_const_is_an_error() {
        let e =
            lower::lower_source("const k = 2; export function f(): number { k = 3; return k; }")
                .err()
                .unwrap();
        assert!(e.contains("constant"), "{e}");
        let e = lower::lower_source(
            "const o = { v: 1 }; export function f(): number { o = { v: 2 }; return o.v; }",
        )
        .err()
        .unwrap();
        assert!(e.contains("constant"), "{e}");
    }
}
