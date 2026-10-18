# Spike: `iron_ts`, TypeScript gameplay code compiled ahead of time

Status: spike done (2026-10-18). It proves the pipeline on one real file,
`examples/topdown-zombie-game/src/sim.ts` (678 lines of pure game rules). It is not wired into
`iron` or the build scripts, and it lives outside the engine workspace in `tools/`.

Context: QuickJS is an interpreter (no JIT, which iOS requires). A sine demo that built meshes in
script ran at ~30 ms a frame until the work moved to Rust. Shipped games should run their gameplay
scripts as native code, with no long builds; dev builds stay TypeScript to JavaScript on QuickJS
with hot reload. See TODO.md M9.b.

## 1. Result in short

- `tools/iron_ts` parses `sim.ts` with oxc, lowers it to a typed IR, compiles that with Cranelift
  and writes an object file. **All 30 functions of `sim.ts` compile**, no fallback needed.
- `tools/iron_ts_tests` links that object and runs ports of **all 13 cases of `sim.test.ts`**: 13/13
  pass (`cargo test --workspace` in `tools/`, IDs `M9-TS-01`..`13`).
- A busy-state benchmark gives the **same checksum in QuickJS, bun and native code**, so the native
  code is not just fast, it computes what the TypeScript does.

| Engine | 10,000 steps | per step | vs QuickJS |
|---|---|---|---|
| QuickJS (`iron_script`, rquickjs 0.14) | 204 ms | 20.4 µs | 1x |
| bun (JavaScriptCore JIT) | 24.0 ms | 2.40 µs | 8.5x |
| `iron_ts` native (Cranelift `speed`) | 4.10 ms | 0.41 µs | **50x** |

Scenario (`tools/iron_ts_tests/bench/bench.ts`, mirrored in `bench.rs`): 1 player standing still,
MP5 on auto-fire with unlimited ammo and health, zombies respawning to keep ~10 alive, bullets
in flight (~33: the gun fires every 0.12 s and a bullet lives 4 s, so 50 is not reachable
without changing the rules), 1/60 s steps. Best of three runs on the dev machine (Windows 11, release
build). bun's number includes JIT warm-up inside the 10,000 steps; QuickJS and native have none.
The step call itself comes from the host in each case (JS loop, Rust loop).

## 2. How types flow in

The compiler **does not run the TypeScript checker**. It reads the source's own annotations and
infers the rest bottom-up, with the expected type pushed down into literals:

- Declared types: interfaces, type aliases, parameter and return annotations, `T | null`,
  string-literal unions, `X["field"]`, `(a: number) => R` function types, `Uint8Array`, `T[]`,
  `Set<T>`, `Record<string, V>` after `satisfies`.
- Inferred: `const x = expr` (type of the expression), arrow-function return types (the function is
  lowered on demand when first called), `map` callback result types, `find` results, `??`, `?:`.
- Contextual: an object literal takes its fields' types from the expected struct (`emit(state,
  { kind: "reload", player: p.id })` is a `SimEvent`), `[]` takes its element type, a string
  literal takes the literal union's tag.

Why: `sim.ts` annotates every declaration, so a checker would add a Node/Bun dependency and a JSON
bridge for nothing. The program is assumed to pass `tsc --strict` already (`bun run typecheck`);
the compiler trusts narrowing (a `Zombie | null` that tsc narrowed is read as a `Zombie`) and does
not prove it.

Limit, and the plan for it: anything inferred through generics, overloads, conditional types or
control-flow narrowing of unions will not work without the real checker. The next step is a Bun
sidecar that runs the TypeScript compiler API and dumps, per source span, the resolved type of every
expression and symbol as JSON, which the lowering then reads instead of its own inference. The IR
types below are what that JSON would be mapped to.

oxc_semantic is not used; scopes are tracked by the lowering itself (a stack of name to local maps),
which is simpler for this subset.

## 3. Iron IR

`tools/iron_ts/src/ir.rs`. Small, typed and **structured** (blocks, loops, labelled
break/continue) instead of a CFG: lowering from TypeScript stays direct, and
`cranelift-frontend` builds the CFG in one pass. Every expression carries its type.

Types (`Ty`):

| IR type | Meaning | Machine value |
|---|---|---|
| `Num` | `number` | f64 |
| `Bool` | `boolean` | i8 |
| `Lit` | a member of a string-literal union (global tag number) | i32 |
| `Str` | `string` (runtime exists, lowering of string code is future work) | pointer |
| `Obj(StructId)` | interface / type alias / inline type / object literal, a struct with a fixed layout | pointer |
| `Arr(T)`, `Bytes` | `T[]`, `Set<T>`, `Uint8Array` | pointer to an array header |
| `Fn(sig)` | function-typed value | pointer to a closure `{fnptr, env}` |
| `Nullable(T)` | `T \| null \| undefined` | same as `T`; null is a zero pointer, or for numbers a reserved NaN payload (`0x7FF8DEADBEEF0001`) |
| `Null`, `Void` | the `null` literal before coercion; no value | none |
| `JsValue` | **reserved** for dynamic values, never produced yet | (NaN-boxed) |

A discriminated union of object types (`SimEvent`) becomes **one struct with the union of all
variants' fields**, so `e.kind === "reload"` is a tag compare and `e.player` a field read.
Nullable-number fields are initialised to null, not 0.

Statements: `Assign`, `StoreField`, `StoreIndex`, `StoreGlobal`, `If`, `Block`, `Loop`, `Break`,
`Continue`, `Return`, `Eval`. Expressions: constants, `Var`, `Global`, unary / binary / compare
ops, lazy `And` / `Or` / `Cond`, `IsNull`, `Cast` (representation-identical retype), `Field`,
`Index`, `Len`, `NewObj`, `Call`, `CallClosure`, `Rt` (runtime calls) and `Seq(stmts, value)`,
which lets a lowering emit a loop in the middle of an expression.

Example, `xs.forEach((x) => { if (x > limit) return; s += x; })` lowers to (callback inlined, `return`
becomes `break` out of the callback's block):

```
loop L1 {
  if Ge(v4, len(v3)) { break L1 }
  v5 = cast<f64>(v3[v4])        // element, v4 is the f64 index
  v6 = v4
  v4 = Add(v4, 1)
  block L2 {                    // the callback body
    if Gt(v5, v1) { break L2 }
    v2 = Add(v2, v5)
  }
}
```

Built-ins emitted as IR or Cranelift instructions rather than calls into a library:

- `Math.sqrt abs floor sign min max hypot imul` and `PI SQRT2 E`. `sqrt`, `abs`, `floor`, `fmin`,
  `fmax` are single instructions (`fmin`/`fmax` propagate NaN like JS). `hypot` is
  `sqrt(x*x + y*y)` (within an ulp of libm, no call). `sin cos atan2` and `%` (fmod) call the
  runtime, which calls libm.
- Bit operations (`^ | & << >> >>>`, `Math.imul`) go through ToInt32 (f64 to i64 saturating, then to
  i32), operate in i32, and convert back to f64: exact for the mulberry32 RNG.
- Array `length`, indexing (bounds-checked, out of range reads null), `for...of`, and the methods
  `push forEach filter find some every map includes has` **lowered to loops with the callback
  inlined**. No closure object is built and the callback sees the enclosing function's locals
  directly. `new Set(xs)` shares the array and `has` is a linear search (fine for tiny sets).
  `Uint8Array.fill` calls the runtime.
- Object spread `{...gun}` copies every field slot; `?.` and `??` become a labelled block and a
  conditional; `x++`, compound assignment and assignment to members evaluate the object once.

## 4. What compiled and what did not

Compiled (all of `sim.ts`): interfaces and type aliases, a discriminated union, string-literal
unions and `Set` over them, nullable numbers and objects, nested inline types, top-level
constants (numbers inlined; objects, `as const satisfies Record<string, Gun>` and arrays built by
a generated `ts_init`), arrow-function and `function` declarations with inferred return types,
callbacks passed in (`walls: (map: number) => Walls`, called through a closure), `for...of`,
`while`, `break` / `continue`, optional chaining, `!`, spread of objects, the array methods above.

Not supported (each is a compile error naming the source line, `M9-TS-15`):

- Closures **created** in script and capturing variables (the IR has the closure value and the
  call; there are no capture cells yet). Callbacks to array methods are inlined, which covers the
  common case.
- Classes, `this`, methods, generics, destructuring, default/rest parameters, `switch`, `try` /
  `throw`, template literals, general string code (`+`, `.length`, `===` on `string`), `Map`,
  `typeof`, `for(;;)` with an update clause, `for...in`, `Array.from`, array/object spread, getters.
- `JsValue` (anything dynamic: `any`, unions of unrelated types, property access by computed name).
- Known deviations: `NaN` payload for `null` means `null === null` of two nullable numbers is false
  (compare with the literal `null`, which is exact); numbers beyond ±2^63 in bit operations; `==`
  is `===`; `undefined` and `null` are one value; using a `number | null` as a condition (`if (z.attackTimer)`)
  evaluates its operand twice, which only matters if the operand has side effects.

## 5. Backend and linking

`codegen.rs`: IR to Cranelift 0.136 (`cranelift-frontend` variables, `cranelift-module`,
`cranelift-object`) for the host triple, `opt_level = speed`, no PIC, no unwind info. Exported
TypeScript functions become `ts_<name>` with the platform's C calling convention (Windows x64
fastcall here; `extern "C"` on the Rust side); `ts_init` runs the top-level initialisers.
Runtime functions are imports (`ts_rt_alloc`, `ts_rt_arr_push`, ...). Cranelift also targets
x86_64 and aarch64 Linux, macOS and Android, and aarch64 iOS (pure AOT, no JIT), through
`cranelift-object`'s ELF / Mach-O / COFF writers; only the host was exercised.

`iron_ts` also writes **Rust bindings** (`layout.rs`): `#[repr(C)]` structs with the exact slot
layout, `TAG_*` constants for string literals, `NULL_NUM_BITS`, and `extern "C"` declarations of
the exported functions. The tests include that file, so the host reads and writes fields of
script state directly (`(*player).score`). This is the shape of the future engine boundary
(section 9).

Linking: the test crate's `build.rs` calls `iron_ts::compile_source`, writes the `.obj` and passes it
with `cargo:rustc-link-arg`; the platform linker (MSVC `link.exe` here) does the rest, with
`iron_ts_rt` as an ordinary Rust dependency. A game build would do the same for the `iron` binary.

## 6. Memory model

`tools/iron_ts_rt`: a **bump arena, never freed**. Allocations come from zeroed 4 MiB chunks (large
ones get their own), thread local so parallel tests have separate heaps. `reset()` frees everything.

- Object: `[header = struct id + 1][8-byte slot per field]`. Every slot is 8 bytes (bool and tag
  are zero-extended), which keeps field offsets trivial and Rust layouts exact.
- Array: `{header, len, cap, data}`; growth doubles into a new block and leaves the old one
  (garbage). `Uint8Array`: the same header with a byte per element.
- Closure: `{header, fnptr, env}`, called as `fnptr(env, args...)`.
- String: `{header, len, utf8 ptr}` with concat and compare in the runtime, not yet used by lowering.

Cost: the zombie sim allocates **2.8 KB per step** (28 MiB for 10,000 steps): a fresh `events`
array every step, `filter` results, `p.aim = {...}`, `{...GUN}`. An hour of play at 60 Hz is ~600
MB. Enough for a spike and for short sessions; not enough to ship.

What a real collector needs, and what is already there:

1. A header on every object: present (struct id), arrays have one (kind), so a tracer can walk
   the heap. Pointer slots are known per struct from the IR (`Obj`, `Arr`, `Fn`, `Str`, nullable
   forms), so a precise tracer needs no conservative scanning of the heap.
2. **Roots on the native stack.** Two options: Cranelift stack maps (`declare_value_needs_stack_map`,
   safepoints at calls; precise, needs per-call-site maps and a stack walker per platform) or a
   **shadow stack / handle scope** in generated code (push pointer locals to a side stack around
   calls that can allocate; simple, portable to iOS and Android, a few percent slower). Recommend the
   shadow stack first.
3. Collector: a non-moving mark-sweep over size-classed blocks is enough (no moving means host
   pointers into script state stay valid between calls). A cheaper first step that fits games:
   **a frame arena** for temporaries (reset each step) plus the persistent arena for state; escape
   analysis marks `filter` results and `events` as frame-scoped.
4. Write barriers are not needed for a non-incremental, non-generational mark-sweep.

## 7. Build times

For `sim.ts` (release build of `iron_ts`, best of 30 on the dev machine):

| Phase | Time |
|---|---|
| parse + types + lower (oxc, lowering) | 0.73 ms |
| Cranelift `opt_level=speed` (30 functions, 26.7 KB object) | 12.0 ms |
| Cranelift `opt_level=none` (27.5 KB object) | 9.7 ms |
| Link: incremental `cargo build --release` of the bench exe after touching `sim.ts` (build script, include, relink, which also embeds QuickJS) | ~1.1 s |

The compiler is not the cost; the platform link is. Compile time scales with the amount of code
(about 0.02 ms per line of TypeScript in total), so a 20,000-line gameplay codebase is on the order
of half a second plus the link. The first `cargo` build of the tool itself takes ~1 minute (Cranelift
and oxc), once.

## 8. Second benchmark: the sine circle

`tools/iron_ts_tests/bench/sine.ts` is the sine-circle demo's curve maths
(`examples/sine-circle-demo`: `at()` per degree, Catmull-Rom spline, 3 samples per segment) as free
functions, flattened to the 2,166-number list that `LineRenderer2D.send` hands to `render.stroke`.
It compiles with `iron_ts` unchanged (0.2 ms lower, 1.8 ms Cranelift, 4.6 KB object). One frame
= 361 points, 1,083 after the spline, with the formula, radius, frequency, depth, rotation and
centre changing each frame (`cargo run --release -p iron_ts_tests --bin sine_bench`, 2,000 frames,
best of three; all engines print the same checksum).

| Engine | per frame | vs QuickJS |
|---|---|---|
| QuickJS, compute only | 1.14 ms | 1x |
| QuickJS, compute + `__iron_op` + draining the op into Rust | 1.26 ms | (0.12 ms is the crossing) |
| bun (JIT) | 0.054 ms | 21x |
| native, compute | 0.033 ms | **35x** |
| native, compute + Rust copying the points out of script memory into its own `Vec<f32>` | 0.030 ms | **38x** |

What it says:

- Most of the old cost was the **calculation in the interpreter** (1.14 ms of 1.26), not the
  boundary; but the boundary is not free: ~0.12 ms here to move 2,166 numbers as `Value`s, and the
  real app adds its own conversion (the demo's commit message measured ~0.2 ms).
- With native code there is no `Value` conversion. The array lives in script memory with a known
  layout (`{header, len, cap, data}` of f64); Rust reads the `data` slice in place. The "copy"
  row includes narrowing to f32 and costs nothing measurable. A direct engine call
  (`render.stroke(points: *const f64, n)`) would remove even that copy.
- The frame is allocation heavy (361 + 1,083 small objects and a growing array), so it exercises
  the allocator: with a **frame arena** (`ts_rt_reset()` once Rust has the points) a frame costs
  30 µs. Before the reset kept its first chunk the same code took 80 µs, slower than bun,
  because every reset handed 4 MiB back to the OS and faulted it in again. Escape analysis (or the
  GC) is what would make that reset automatic.
- Two compiler fixes came from this benchmark: constant expressions such as
  `const UNITS_PER_PX = 10 / 729` are now folded to constants instead of becoming globals set by
  `ts_init` (forgetting to call `ts_init` gave all-zero coordinates, which the checksum caught), and
  `%` on whole numbers is an integer `srem` instead of a libm `fmod` call (the fast path is
  correct for the spline's wrap-around indexes; it did not change this benchmark's time).
- Several modules link into one program: exported symbols take a prefix (`ts_sine_curveFlat`).

## 9. Effort and next steps

Estimates for one engineer who knows the codebase. "Useful" means shipping the gameplay
modules of a typical game as native code; React UI code stays on QuickJS (see next).

| Step | What | Estimate |
|---|---|---|
| 1. Language coverage | Closures with capture cells, classes + methods + `this`, destructuring, defaults, `switch`, template literals and strings, `Map`, generics by monomorphisation, enums | 6-8 weeks |
| 2. Real types | Bun sidecar that dumps the TypeScript checker's per-expression types as JSON; map to IR types; drop the in-compiler inference | 3-4 weeks |
| 3. Direct engine calls | Expose engine functions as `extern "C"` with shared layouts instead of `__iron_op` value conversion. `Vector3` = glam `Vec3` (a value type in the IR: 3 x f32 in registers or `f32x4`), `Quaternion`, `Color`; `Mathf` and vector arithmetic as intrinsics emitted inline; component lifecycle (`onUpdate`) entered through a function table; physics/scene/audio ops as direct calls | 4-5 weeks |
| 4. GC | Shadow stack + non-moving mark-sweep, frame arena for temporaries | 3-4 weeks |
| 5. `JsValue` fallback | NaN-boxed dynamic value, a small dynamic object model, calls between native and QuickJS for code that cannot be typed (`any`, React-style code) | 6+ weeks, the largest unknown |
| 6. Packaging and targets | Cross-targets (Linux, macOS, Android, iOS), linking into the `iron` binary from the game build, Windows unwind info, source maps / debug info for crashes, CI | 3 weeks |
| 7. Hot loop intrinsics and tuning | Integer loop counters (range analysis instead of f64 indexes), inline allocation fast path, bounds-check elision, scalar replacement of small objects | 2 weeks |

Rough total for "useful": 2.5-3 months (steps 1-4 plus 6, with gameplay modules only). Everything
including the dynamic fallback: 5-7 months.

Recommended order and decisions:

1. **Make it hybrid**, not total. Compile the typed gameplay/simulation modules ("pure" files like
   `sim.ts`) and keep React and the UI reconciler on QuickJS in release builds too; the engine
   already crosses that boundary with ops and commits. This removes the JsValue fallback from the
   critical path.
2. Do the type sidecar (step 2) before widening coverage; inferring types ourselves does not scale
   past annotated code.
3. Direct engine calls (step 3) are where the large win is for real games: the benchmark above
   removed the interpreter, but `__iron_op` argument conversion still costs more than a native call.
   Share layouts by generating the Rust side (as `layout.rs` does) from the IR and `#[repr(C)]` engine
   types, so a `Vector3` written by script is a `Vec3` to the engine.
4. Keep dev builds on QuickJS and hot reload exactly as now; `iron_ts` is only a release-build step
   and can fall back to QuickJS per module when a file uses unsupported features (the compiler
   already rejects with line numbers).

## 10. Layout of the code

| Path | What |
|---|---|
| `tools/Cargo.toml` | Own workspace (not a member of the engine workspace; engine builds never see it) |
| `tools/iron_ts/src/lower/` | oxc AST to IR: `types.rs` (annotations, structs, coercions), `expr.rs`, `stmt.rs`, `arrays.rs` (methods and inlined callbacks), `mod.rs` (declarations, init) |
| `tools/iron_ts/src/ir.rs` | the IR and its dump (`iron_ts file.ts --dump-ir`) |
| `tools/iron_ts/src/codegen.rs` | IR to Cranelift object |
| `tools/iron_ts/src/layout.rs` | generated Rust bindings |
| `tools/iron_ts/src/bin/iron_ts.rs` | CLI: `iron_ts file.ts -o out.obj --bindings out.rs [--opt none\|speed] [--time N] [--dump-ir]` |
| `tools/iron_ts_rt/` | runtime (arena, arrays, strings, closures, libm calls) |
| `tools/iron_ts_tests/` | build script (compile + link `sim.ts`), `tests/sim_port.rs` (13 ports), `bench/bench.ts` and `src/bin/bench.rs` (the benchmark) |

Run (from `tools/`, with `PATH` including cargo and bun):

```bash
cargo test --workspace
```
```bash
cargo run --release -p iron_ts_tests --bin bench
```
