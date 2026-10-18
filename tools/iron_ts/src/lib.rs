//! `iron_ts`: an ahead-of-time compiler for Iron's TypeScript gameplay code.
//! TypeScript (oxc) -> typed Iron IR -> Cranelift object file. See `docs/research/iron-ts-spike.md`.

pub mod codegen;
pub mod frames;
pub mod ir;
pub mod layout;
pub mod lower;
pub mod manifest;

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
    compile_with_manifests(src, opt, prefix, struct_base, &[])
}

/// Like `compile_with`, with engine API manifests (the JSON `iron_api` generates): the TypeScript
/// can call `Namespace.function(...)` and the call is a direct call of the function's symbol.
pub fn compile_with_manifests(
    src: &str,
    opt: &str,
    prefix: &str,
    struct_base: u32,
    manifests: &[String],
) -> Result<Output, String> {
    // Lowering recurses with the source's nesting; the main thread's 1 MiB is too little on Windows.
    std::thread::scope(|s| {
        std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn_scoped(s, || {
                compile_inner(src, opt, prefix, struct_base, manifests)
            })
            .map_err(|e| e.to_string())?
            .join()
            .map_err(|_| "compiler panicked".to_string())?
    })
}

fn compile_inner(
    src: &str,
    opt: &str,
    prefix: &str,
    struct_base: u32,
    manifests: &[String],
) -> Result<Output, String> {
    let t0 = std::time::Instant::now();
    let mut module = lower::lower_source_with(src, manifests)?;
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

    // M9-TS-70
    #[test]
    fn bad_enums_and_jumps_are_errors_with_a_reason() {
        let e = lower::lower_source(
            "export function f(n: number): number { enum E { A = n } return E.A; }",
        );
        assert!(e.is_err());
        let e = lower::lower_source("enum E { A, B } export function f(): number { return E.C; }")
            .err()
            .unwrap();
        assert!(e.contains("no member `C`"), "{e}");
        let e = lower::lower_source(
            "enum E { A = \"a\", B } export function f(): number { return 1; }",
        )
        .err()
        .unwrap();
        assert!(e.contains("initialiser"), "{e}");
        let e = lower::lower_source("export function f(): number { continue; return 1; }")
            .err()
            .unwrap();
        assert!(e.contains("continue outside a loop"), "{e}");
        let e = lower::lower_source(
            "export function f(): number { sw: switch (1) { case 1: for (;;) { continue sw; } } return 1; }",
        )
        .err()
        .unwrap();
        assert!(e.contains("not a loop"), "{e}");
    }

    // M9-TS-84
    #[test]
    fn calls_with_the_wrong_number_of_arguments_are_errors() {
        let src = "function f(a: number, b: number): number { return a + b; }";
        let e = lower::lower_source(&format!(
            "{src} export function g(): number {{ return f(1); }}"
        ))
        .err()
        .unwrap();
        assert!(e.contains("arguments"), "{e}");
        let e = lower::lower_source(&format!(
            "{src} export function g(): number {{ return f(1, 2, 3); }}"
        ))
        .err()
        .unwrap();
        assert!(e.contains("at most 2"), "{e}");
        let e = lower::lower_source(
            "function f(a: number, b: number): number { return a + b; } \
             export function g(xs: number[]): number { return f(...xs); }",
        )
        .err()
        .unwrap();
        assert!(e.contains("rest parameter"), "{e}");
    }

    // M9-TS-110
    #[test]
    fn array_calls_that_cannot_match_javascript_are_errors_that_say_why() {
        let e =
            lower::lower_source("export function f(xs: number[]): number[] { return xs.sort(); }")
                .err()
                .unwrap();
        assert!(e.contains("comparator"), "{e}");
        let e = lower::lower_source(
            "export function f(xs: number[]): number { return xs.reduce((a, x) => a + x); }",
        )
        .err()
        .unwrap();
        assert!(e.contains("initial value"), "{e}");
        let e = lower::lower_source(
            "export function f(xs: boolean[]): boolean | undefined { return xs.pop(); }",
        )
        .err()
        .unwrap();
        assert!(e.contains("pop"), "{e}");
    }

    // M9-TS-119
    #[test]
    fn class_features_that_are_not_built_yet_are_errors_that_say_so() {
        let ext = lower::lower_source(
            "class B extends Component { y = 2; } export function f(): number { return new B().y; }",
        )
        .err()
        .unwrap();
        assert!(ext.contains("not a class of this program"), "{ext}");
        let dec = lower::lower_source(
            "class A { @sealed x = 1; } export function f(): number { return new A().x; }",
        )
        .err()
        .unwrap();
        assert!(dec.contains("only `@property`"), "{dec}");
        let method = lower::lower_source(
            "class A { @property m(): number { return 1; } } export function f(): number { return new A().m(); }",
        )
        .err()
        .unwrap();
        assert!(method.contains("decorators"), "{method}");
        let value = lower::lower_source(
            "class A { m(): number { return 1; } } export function f(): number { return new A().nope; }",
        )
        .err()
        .unwrap();
        assert!(value.contains("no field `nope`"), "{value}");
        let ann = lower::lower_source(
            "class A { x = [1]; } export function f(): number { return 1 + new A().x.length; }",
        )
        .err()
        .unwrap();
        assert!(ann.contains("needs a type annotation"), "{ann}");
        let this = lower::lower_source("export function f(): number { return this.x; }")
            .err()
            .unwrap();
        assert!(this.contains("`this`"), "{this}");
    }

    // M9-TS-134
    #[test]
    fn inheritance_mistakes_are_errors_that_say_what_is_wrong() {
        let abs = lower::lower_source(
            "abstract class A { abstract m(): number; } export function f(): number { return new A().m(); }",
        )
        .err()
        .unwrap();
        assert!(abs.contains("abstract class A"), "{abs}");
        let order = lower::lower_source(
            "class B extends A { } class A { x = 1; } export function f(): number { return new B().x; }",
        )
        .err()
        .unwrap();
        assert!(order.contains("declare the base class"), "{order}");
        let retype = lower::lower_source(
            "class A { x = 1; } class B extends A { x = true; } export function f(): number { return 1; }",
        )
        .err()
        .unwrap();
        assert!(retype.contains("changes its type"), "{retype}");
        let inst = lower::lower_source(
            "export function f(x: number): boolean { return x instanceof Array; }",
        )
        .err()
        .unwrap();
        assert!(inst.contains("instanceof"), "{inst}");
    }

    // M9-TS-146
    #[test]
    fn calls_of_the_engine_api_are_checked_against_the_manifest() {
        let manifest = r#"{"namespace":"Eng","handles":["Thing"],"functions":[
            {"name":"add","symbol":"iron_Eng_add","params":[{"name":"a","type":"f64"},{"name":"b","type":"f64"}],"returns":"f64"},
            {"name":"make","symbol":"iron_Eng_make","params":[],"returns":"handle:Thing"}]}"#
            .to_string();
        let ok = lower::lower_source_with(
            "export function f(): number { const t: Thing = Eng.make(); return Eng.add(1, 2) + (t === Eng.make() ? 1 : 0); }",
            std::slice::from_ref(&manifest),
        )
        .unwrap();
        // `make` is used twice but declared once.
        let mut symbols: Vec<&str> = ok.externs.iter().map(|d| d.symbol.as_str()).collect();
        symbols.sort();
        assert_eq!(symbols, ["iron_Eng_add", "iron_Eng_make"]);
        let missing = lower::lower_source_with(
            "export function f(): number { return Eng.nope(1); }",
            std::slice::from_ref(&manifest),
        )
        .err()
        .unwrap();
        assert!(missing.contains("no function `nope`"), "{missing}");
        let count = lower::lower_source_with(
            "export function f(): number { return Eng.add(1); }",
            std::slice::from_ref(&manifest),
        )
        .err()
        .unwrap();
        assert!(count.contains("1 arguments, 2 expected"), "{count}");
        let bad =
            lower::lower_source_with("export function f(): number { return 1; }", &["{".into()])
                .err()
                .unwrap();
        assert!(bad.contains("manifest"), "{bad}");
    }

    // M9-TS-150
    #[test]
    fn value_classes_of_the_manifest_are_checked() {
        let manifest = r#"{"namespace":"V","handles":[],"values":[
            {"name":"Pair","fields":[{"name":"a","type":"f32"},{"name":"b","type":"f32"}]}],
            "functions":[
            {"name":"swap","symbol":"iron_V_swap","params":[{"name":"p","type":"value:Pair"}],"returns":"value:Pair"}]}"#
            .to_string();
        let ok = lower::lower_source_with(
            "export function f(): number { const q = V.swap(new Pair(1, 2)); return q.a * 10 + q.b; }",
            std::slice::from_ref(&manifest),
        )
        .unwrap();
        // The parameter is flattened to its two fields, and the result comes back through a pointer.
        assert_eq!(ok.externs[0].params.len(), 2);
        assert_eq!(ok.externs[0].out.as_ref().map(|o| o.1), Some(2));
        let few = lower::lower_source_with(
            "export function f(): number { return new Pair(1).a; }",
            std::slice::from_ref(&manifest),
        )
        .err()
        .unwrap();
        assert!(few.contains("1 arguments, 2 expected"), "{few}");
        let wrong = lower::lower_source_with(
            "export function f(): number { return V.swap(3).a; }",
            std::slice::from_ref(&manifest),
        )
        .err()
        .unwrap();
        assert!(wrong.contains("type mismatch"), "{wrong}");
    }
}
