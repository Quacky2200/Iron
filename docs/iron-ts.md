# iron_ts: the ahead-of-time TypeScript compiler

Code: [tools/iron_ts](../tools/iron_ts/src) (compiler), [tools/iron_ts_rt](../tools/iron_ts_rt/src/lib.rs)
(runtime and GC), [tools/iron_ts_tests](../tools/iron_ts_tests) (fixtures, tests, benchmarks).
Progress and next steps: [tools/iron_ts/TODO.md](../tools/iron_ts/TODO.md). Why it exists, the
measurements and the design notes: [iron-ts-spike.md](research/iron-ts-spike.md),
[iron-ts-classes.md](research/iron-ts-classes.md), [iron-ts-engine-boundary.md](research/iron-ts-engine-boundary.md).

This is a **spike**. It compiles a typed subset of TypeScript (gameplay rules written as plain
functions and classes) to a native object file. It is not wired into `iron`, the build scripts or
the engine workspace: `tools/` is its own cargo workspace, so engine builds never see it. The
engine still runs gameplay as JavaScript in QuickJS ([scripting.md](scripting.md)).

## Running it

Run cargo from `tools/` (shell setup as in [CLAUDE.md](../CLAUDE.md)).

```bash
cd tools && cargo fmt --all
cd tools && cargo clippy --workspace --all-targets -- -D warnings
cd tools && cargo test --workspace
cd tools && IRON_TS_GC=stress cargo test -p iron_ts_tests      # collect at every allocation
cd tools && cargo run --release -p iron_ts_tests --bin bench   # QuickJS vs bun vs native, 10,000 steps
cd tools && cargo run --release -p iron_ts_tests --bin sine_bench 2000
```

`IRON_TS_GC` is read by `gc_mode_from_env` in [iron_ts_tests/src/lib.rs](../tools/iron_ts_tests/src/lib.rs):
`stress` collects at every allocation of generated code and poisons freed memory, `off` never
collects on its own. Any change to lowering, codegen or the runtime should also pass in stress mode.

The CLI (`cargo build --release -p iron_ts`, binary `tools/target/release/iron_ts`):

```
iron_ts file.ts [--dump-ir] [--prefix p_] [--struct-base N] [-o out.obj] [--bindings out.rs]
                [--opt none|speed|speed_and_size] [--time N]
```

| Flag | Effect |
|---|---|
| `--dump-ir` | Print the typed IR (`ir::dump`) |
| `--prefix p_` | Exported functions are named `ts_<prefix><name>` (default no prefix) |
| `--struct-base N` | Number object headers from `N` (default 0); modules sharing one process need ranges at least 1000 apart |
| `-o out.obj` | Write the object file |
| `--bindings out.rs` | Write the generated Rust bindings |
| `--opt` | Cranelift `opt_level`, default `speed` |
| `--time N` | Compile `N` times and print the best time of each phase |

## Pipeline

```
 .ts source --oxc parser--> AST --lower/--> Iron IR (ir.rs) --codegen.rs (Cranelift)--> object file
                                                  |                                      + linked with iron_ts_rt
                                                  +--layout.rs--> Rust bindings (repr(C) structs, extern "C")
```

`compile_with(src, opt, prefix, struct_base)` in [lib.rs](../tools/iron_ts/src/lib.rs) runs it on a
thread with a 64 MiB stack (lowering recurses with source nesting) and returns an `Output`: object
bytes, bindings text, IR dump and the time of each phase.

- **No type checker.** The compiler reads the source's own annotations and infers the rest bottom
  up, pushing the expected type into literals. The program must already pass `tsc --strict`; narrowing
  is trusted, not proved.
- **The IR** is structured (blocks, loops, labelled `Break`/`Continue`, no CFG) and every expression
  carries its type. Numbers are `f64`, bools `i8`, string-literal union members `i32` (`Ty::Lit`),
  everything else a 64-bit pointer. `T | null` is the pointer with 0 as null; `number | null` is an
  `f64` with one reserved NaN payload (`NULL_NUM_BITS`).
- **Objects** are `[header][8-byte slot per field]`; every field is one 8-byte slot, so offsets are
  trivial and the generated Rust structs match exactly.
- **Codegen** targets the host triple (no PIC, no unwind info). Exported functions are `ts_<prefix><name>`
  with the C calling convention; `ts_<prefix>init` runs the top-level initialisers and registers
  the struct pointer masks and global roots with the runtime. Runtime routines are imports
  (`ts_rt_*`).
- **Linking** is the host's job: the test crate's `build.rs` writes the `.obj`, passes it with
  `cargo:rustc-link-arg`, and includes the bindings.

## Source files

| File | What |
|---|---|
| [lib.rs](../tools/iron_ts/src/lib.rs) | `compile_source` / `compile_with_prefix` / `compile_with`, `Output`; unit tests for lowering errors (`M9-TS-14..16`, `55`, `56`, `70`, `84`, `110`, `119`) |
| [ir.rs](../tools/iron_ts/src/ir.rs) | The IR: `Ty`, `Module`, `Func`, `Stmt`, `Expr`/`ExprKind`, `RtFn` (runtime calls), `field_offset`, `dump` |
| [frames.rs](../tools/iron_ts/src/frames.rs) | Shadow-stack frame plan and the spill rule (see GC roots below) |
| [codegen.rs](../tools/iron_ts/src/codegen.rs) | IR to Cranelift object: frames and spills, `eval_args`, `ts_init`, runtime calls |
| [layout.rs](../tools/iron_ts/src/layout.rs) | Rust bindings: `HDR_*` header constants, `#[repr(C)]` structs, `TAG_*`, `NULL_NUM_BITS`, `extern "C"` declarations |
| [bin/iron_ts.rs](../tools/iron_ts/src/bin/iron_ts.rs) | The CLI above |
| [lower/mod.rs](../tools/iron_ts/src/lower/mod.rs) | `lower_source`: collecting declarations, `lower_fn` (lowered on demand), bindings (direct or boxed), module-level variables |
| [lower/types.rs](../tools/iron_ts/src/lower/types.rs) | Annotation to IR type, struct layouts (a union of objects becomes one struct), `coerce`, `to_bool` |
| [lower/expr.rs](../tools/iron_ts/src/lower/expr.rs) | Expressions, assignment targets (places), calls, `new`, optional chains, `Math`, constant folding |
| [lower/stmt.rs](../tools/iron_ts/src/lower/stmt.rs) | Statements: variables, destructuring, loops, labels, `switch` |
| [lower/arrays.rs](../tools/iron_ts/src/lower/arrays.rs) | Array methods; callbacks are inlined into loops, `Iter` for `for...of` and spread |
| [lower/scan.rs](../tools/iron_ts/src/lower/scan.rs) | Which names do closures capture (by name, so it only over-approximates) |
| [lower/closure.rs](../tools/iron_ts/src/lower/closure.rs) | Closures lifted to `lambda(env, params...)`, function values, cells |
| [lower/enums.rs](../tools/iron_ts/src/lower/enums.rs) | Enums folded into constants (numeric) or string tags |
| [lower/classes.rs](../tools/iron_ts/src/lower/classes.rs) | Plain classes: fields, constructors, methods, accessors, statics, `#private` |

## Supported language

| Feature | Lowering |
|---|---|
| Types | `number`, `boolean`, interfaces and type aliases as structs, discriminated unions as one struct with every variant's fields, string-literal unions as tags, `T \| null`, `T[]`, `Uint8Array`, `Set<T>` (an array), `Record<string, V>` after `satisfies`, function types |
| Functions | `function` and arrow declarations; return types inferred; calls are direct |
| Default, optional, rest parameters | The caller fills in defaults (lowered in a scope of the earlier parameters), a missing optional is null, the rest array is built at the call site |
| Closures | Captured variables live in heap cells; the closure is `{func, env}` with `env` an array of cells; called as `func(env, args...)`. A top-level function used as a value gets a wrapper |
| Array callbacks | `map`, `filter`, `forEach`, `find`, `some`, `every` (and `reduce`, `findIndex`, `includes`) are index loops with an arrow callback's body inlined; no closure object. A non-arrow callback is called as a closure per element |
| Array library | `push`, `pop`, `shift`, `unshift`, `slice`, `splice`, `concat`, `reverse`, `fill`, `sort(cmp)` (stable merge sort in the runtime), `indexOf`, `at`, `length = n`, `Array.from/of/isArray`, `new Array(n)` |
| Loops and labels | `while`, `for(;;)`, `do...while`, `for...of`, labelled `break`/`continue` and blocks; per-iteration `let` cells when a closure captures the variable |
| `switch` | The clause index is chosen by `===` compares, then the bodies run in order (fall-through, `default` anywhere) |
| Enums | Numeric and string enums fold to constants, no run-time cost |
| Destructuring | Declarations, parameters, `for...of` variables and callbacks; defaults via `IsNull` |
| Spread | Arrays, objects (field by field) and calls into a rest parameter |
| Module variables | A literal `const` is folded; any other `const`, and every `let`/`var`, is a data slot written by `ts_init` (`Place::Global`) |
| `Math` and operators | Inline instructions or `ts_rt_*` (libm, JS `pow`, `random`); `**`, `~`, `??=`, `\|\|=`, `&&=`, bit operations with ToInt32 |
| Classes | A struct plus functions: constructor `C$new`, methods with `this` as first parameter, getters/setters, statics as globals, `#private`, parameter properties, `this` in arrows |
| Optional chaining, `!`, `??`, `?:` | Inline null tests |

Not supported (each is a compile error naming the source line): `extends`/`super`, abstract classes,
decorators (`@property`), methods used as values, `instanceof`, `typeof`, generics, `try`/`throw`,
template literals and general string code, `Map`, `for...in`, rest elements in patterns,
destructuring assignment to existing variables, `async`, `any`/`JsValue` and anything dynamic. Known
deviations from JavaScript (`null === null` of two nullable numbers, `==` as `===`, libm last-bit
differences) are listed in [iron-ts-spike.md](research/iron-ts-spike.md) sections 4 and 15.

## Runtime and GC

[iron_ts_rt/src/lib.rs](../tools/iron_ts_rt/src/lib.rs) is an ordinary Rust crate the generated code
calls into. Its header comment is the long version.

- **Heap.** One heap per thread. Blocks up to 512 bytes come from 1 MiB chunks with size-class free
  lists; larger ones use the system allocator. Nothing moves, so a pointer Rust holds stays valid while
  the object is reachable. Every object is tracked with its size.
- **Headers.** A header is `mark (top bit) | heap id | kind`. The kind is the struct id + 1 for
  structs, or one of `HDR_ARRAY`, `HDR_ARRAY_PTR`, `HDR_BYTES`, `HDR_STR`, `HDR_CLOSURE`,
  `HDR_CLOSURE_ENV`, `HDR_CELL`, `HDR_CELL_PTR`. The heap id keeps a collector away from other
  threads' objects reached through a shared global slot.
- **Tracing.** Which slots of a struct are pointers is a mask the generated `ts_init` registers
  (`ts_rt_register_struct`). Pointer arrays, cells and script closure environments are traced.
- **Roots.** (1) The shadow stack: a generated function with pointer locals links a `Frame` with one
  slot per pointer local plus spill slots (`ts_rt_frame_push` / `ts_rt_frame_pop`) and mirrors every
  pointer assignment into it. (2) Globals, registered by address (`ts_rt_add_global_root`). (3) Host
  roots, for Rust code that keeps script pointers across calls (`ts_rt_root_add` / `ts_rt_root_remove`).
  (4) Temp roots, for values a runtime routine holds while it collects (a new cell's value, a closure
  environment, the sort's merge buffer). Registers and Cranelift's own spill slots are never scanned.
- **The spill rule** ([frames.rs](../tools/iron_ts/src/frames.rs)). A pointer evaluated early and held
  while a later operand can allocate is copied to a frame slot, unless it is a local no later operand
  assigns, a global, or null (`spill_flags`, `is_stable`, `may_alloc`). Field reads are never trusted. The
  slot count computed here is the number codegen's `eval_args` consumes, and codegen refuses a
  function whose spills would overrun its frame. A new IR node that evaluates operands must use
  `eval_args` and be counted in `count_expr`/`count_stmts`, `may_alloc` and `children`.
- **When it collects.** Only inside allocations by generated code (`ts_rt_obj_new_gc`,
  `ts_rt_arr_new_gc`, `ts_rt_bytes_new_gc`, cells, closures) once the larger of 512 KiB and the live size
  has been handed out, or when the host calls `ts_rt_gc_collect`. Host-side constructors
  (`ts_rt_obj_new`, `ts_rt_arr_new`, `ts_rt_arr_push`) never collect, so Rust can build a graph from
  unrooted pieces and root it afterwards. `ts_rt_gc_enable(false)` stops automatic collection.
- **Stress mode** (`ts_rt_gc_stress(true)`): collect at every allocation and fill freed memory with
  `0xDD`, so a missing root crashes loudly. The root analysis is a conservative approximation, not a proof.
- **Entry points** are `#[unsafe(no_mangle)] extern "C"` functions named `ts_rt_*`: allocation, arrays
  (`push`, `set_len`, `insert`, `drop`, `splice`, `slice`, `append`, `reverse`, `fill`, `sort`), strings
  (`str_new`, `concat`, `eq`), cells and closures, math (`sin`, `cos`, `pow`, `random` ...), collection
  control (`gc_collect`, `gc_stress`, `gc_enable`, `allocated`) and roots. Panics inside `extern "C"`
  abort the process, so fallible logic is tested through a Rust-level function.
- **Host side.** `link_anchor()` keeps the exports linked into an executable that only reaches them from
  generated code; `gc_stats()` reports collections and live bytes. Several modules in one process each
  need their own `struct_base`; registering a different table under a taken id is refused.

## Tests

[iron_ts_tests](../tools/iron_ts_tests) holds the TypeScript sources and the Rust that drives them.

| Path | What |
|---|---|
| `fixtures/sim.ts`, `sim.test.ts` | The zombie game's simulation (678 lines) and its original bun tests |
| `bench/*.ts` | One TypeScript module per language area (`closures`, `loops`, `switches`, `classes` ...); `bench.ts` and `sine_bench.ts` are the JavaScript sides of the benchmarks (run in QuickJS and bun, not compiled) |
| `build.rs` | Compiles `fixtures/sim.ts` and every `bench/<name>.ts` in a list of `(name, prefix, struct_base)` at build time, writes the `.obj` and `<name>_bindings.rs` to `OUT_DIR` and passes the objects to the linker |
| `src/lib.rs` | One `pub mod` per module that `include!`s its bindings, plus helpers (`root`, `push_f64s`, `gc_mode_from_env`, builders for the sim's state) |
| `tests/*.rs` | `sim_port` (13 ported cases), `gc`, and one file per language area |
| `src/bin/{bench,sine_bench}.rs` | The benchmarks: the same work in QuickJS, bun and native code, one checksum for all |

Each language-area test calls the generated functions through the bindings (`ts_<prefix><name>`) inside
`in_every_gc_mode`, which runs the body normally, with `ts_rt_gc_stress(true)` and with
`ts_rt_gc_enable(false)`. **Expected numbers come from running the same TypeScript in bun**, so the
tests check that native code computes what JavaScript does. Test IDs are `M9-TS-*` in
[TESTS.md](../TESTS.md) (section "M9.b"); the next free number is in `tools/iron_ts/TODO.md`.

To add a test module:

1. Write `bench/<name>.ts` (exported functions only get symbols; run it in bun to get the expected numbers).
2. Add `("<name>", "<prefix>_", <struct_base>)` to the list in `build.rs`. Bases are 1000 apart (sim
   0, sine 1000, ... classes 11000); take the next free one.
3. Add a `pub mod` for it in `src/lib.rs` that includes `<name>_bindings.rs`.
4. Add `tests/<name>.rs`: call `ts_<prefix>init()` first, use `in_every_gc_mode`, and tag each test with
   its `// M9-TS-nnn` ID.
5. Add the IDs to `TESTS.md`, then run fmt, clippy, `cargo test --workspace` and the stress run.
