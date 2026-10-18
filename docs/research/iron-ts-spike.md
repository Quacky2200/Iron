# Spike: `iron_ts`, TypeScript gameplay code compiled ahead of time

Status: spike done (2026-10-18), then a garbage collector (same day). It proves the pipeline on one real file,
the zombie game's `sim.ts` (678 lines of pure game rules, kept as `tools/iron_ts_tests/fixtures/sim.ts`
since the game moved to the zombie-horders repository's `iron` branch). It is not wired into
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
| QuickJS (`iron_script`, rquickjs 0.14) | 406 ms | 40.6 µs | 1x |
| bun (JavaScriptCore JIT) | 43.8 ms | 4.38 µs | 9x |
| `iron_ts` native (Cranelift `speed`, with the GC of section 6) | 8.8 ms | 0.88 µs | **46x** |

Scenario (`tools/iron_ts_tests/bench/bench.ts`, mirrored in `busy_state()`): 1 player standing
still, MP5 on auto-fire with unlimited ammo and health, zombies respawning to keep 10 alive, **50
parked bullets** (never move, never expire) next to the MP5's own (2 or 3 in flight: they hit a
zombie within a few frames), 1/60 s steps. Best of three runs on the dev machine (Windows 11,
release build). bun's number includes JIT warm-up inside the 10,000 steps; QuickJS and native have
none. The step call itself comes from the host in each case (JS loop, Rust loop). (An earlier
version of this document claimed about 33 bullets in flight; that was wrong, only the parked ones
make a crowd.)

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
| `Cell(T)` | a variable shared with closures: a heap box holding one `T` | pointer |
| `Nullable(T)` | `T \| null \| undefined` | same as `T`; null is a zero pointer, or for numbers a reserved NaN payload (`0x7FF8DEADBEEF0001`) |
| `Null`, `Void` | the `null` literal before coercion; no value | none |
| `JsValue` | **reserved** for dynamic values, never produced yet | (NaN-boxed) |

A discriminated union of object types (`SimEvent`) becomes **one struct with the union of all
variants' fields**, so `e.kind === "reload"` is a tag compare and `e.player` a field read.
Nullable-number fields are initialised to null, not 0.

Statements: `Assign`, `StoreField`, `StoreIndex`, `StoreGlobal`, `If`, `Block`, `Loop`, `Break`,
`Continue`, `Return`, `Eval`. Expressions: constants, `Var`, `Global` (a `const`), `GlobalVar`
(a `let`/`var`), unary / binary / compare
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

- Classes, `this`, methods, generics, destructuring, default/rest parameters, `switch`, `try` /
  `throw`, template literals, general string code (`+`, `.length`, `===` on `string`), `Map`,
  `typeof`, `for(;;)` with an update clause, `for...in`, `Array.from`, array/object spread, getters.
- `JsValue` (anything dynamic: `any`, unions of unrelated types, property access by computed name).
- Known deviations: `NaN` payload for `null` means `null === null` of two nullable numbers is false
  (compare with the literal `null`, which is exact); numbers beyond ±2^63 in bit operations; `==`
  is `===`; `undefined` and `null` are one value; using a `number | null` as a condition (`if (z.attackTimer)`)
  evaluates its operand twice, which only matters if the operand has side effects.

### Closures

Closures that are values (stored, passed, returned, called later) work: arrow functions and
function expressions, capturing variables of the functions around them, nested to any depth.
Array-method callbacks that are written inline as arrows are still compiled into the loop and
never become objects.

- **Which variables are shared** (`lower/scan.rs`): before a function is lowered it is scanned
  with oxc's visitor for closures that are not inlined callbacks. A closure's free names (what it
  uses and does not declare) are variables of the code around it, and every local with such a name
  lives in a cell instead of a register (`Binding::Boxed`). The scan is by name, so it can only
  over-box, never miss. Parameters, `let`/`const`, `for...of` variables and inlined-callback
  parameters are all boxed this way; a `let`/`const` inside a loop body gets a new cell every
  iteration, which is JavaScript's per-iteration binding.
- **Lifting**: the closure body becomes an ordinary IR function `lambda(env, params...)`. Its
  captured variables are locals bound to the cells it pulls out of `env` at entry, so reads and
  writes go through the cell and every closure over a variable, and the function that owns it,
  agree. `MakeClosure(fn, env)` builds the object; the env is an array of cells.
- **Calls**: `CallClosure` loads `func` and `env` and calls `func(env, args...)`. Callees can be a
  local, a field (`counter.inc()`), a call result (`compose(f, g)(10)`), an element (`fs[0]!()`)
  or a parenthesised arrow (an immediately called one).
- **Function values**: a top-level function used as a value gets a small wrapper that adds the
  environment parameter (`xs.map(double)`, `apply(double, 21)`). A non-arrow callback to an array
  method is called as a closure per element.
- **Types**: parameters of a closure take their types from the context (the parameter type of
  the function it is passed to, a declared `(n: number) => number`, a return type) or their own
  annotations. A closure that calls itself needs its variable annotated with a function type.
- **Collector**: `ts_rt_cell_new_gc` and `ts_rt_closure_new_gc` keep their argument alive while
  they collect (a value being boxed and an environment are in no frame yet), cells and
  environments are traced, and `MakeClosure`/`NewCell` count as allocations in the frame plan.
- Tests: `tests/closures.rs` runs every demo in `bench/closures.ts` normally, with a collection at
  every allocation and with collection off, against the numbers bun computes from the same file
  (`M9-TS-34`..`46`).
- **Module-level variables** (`M9-TS-47`..`56`): a `const` with a literal value is folded into its
  uses; any other `const`, and every `let`/`var` (even `let n = 0`), is an 8-byte data slot written
  by `ts_init`. A `let` can be assigned from any function and closure (`Place::Global`, lowered to
  `StoreGlobal`; reads are `GlobalVar`), may be declared without a value when it has a type
  (`let x: T | undefined;` starts as null, a number as 0), and may hold a closure (`handler(n)`
  calls it). Assigning a `const` is an error. Running `ts_init` again resets them. Globals are
  process-wide: a host that runs several scripts in threads must not share one module instance.
  A fix on the way: the slots had alignment 1 and the collector reads them as pointers, so a
  neighbouring non-pointer global could make one misaligned (a crash under the debug build).
- Not done: closures that use `this` or `arguments`, getters, closures created at module level
  (top-level `const f = () => ...` is a function), named function expressions referring to their
  own name, and capturing variables from a generator/async function (neither exists).

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

## 6. Memory model: a precise, non-moving mark-sweep GC

`tools/iron_ts_rt` (`src/lib.rs` has the long version). Per-thread heap; objects are individually
tracked, so a sweep can free them.

- **Object**: `[header][8-byte slot per field]`. The header is the struct id + 1 (so
  `HDR_<Name>` constants are in the generated bindings); arrays, bytes, strings and closures have
  fixed kind words above 0xA11A0000. The top bit is the mark. Every slot is 8 bytes (bool and tag
  zero-extended), which keeps field offsets trivial and the Rust layouts exact.
- **Array**: `{header, len, cap, data}` with `data` a separate block owned by the array (doubling
  growth frees the old block at once). A pointer-element array has its own header kind and is
  traced; number, bool and tag arrays are not. `Uint8Array`: the same header with a byte per
  element.
- **Closure**: `{header, fnptr, env}`, called as `fnptr(env, args...)`. A closure made by script
  (`HDR_CLOSURE_ENV`) has an `env` that is a GC array of cells and is traced; one the host makes
  (`HDR_CLOSURE`) has host memory in `env`, untraced. **Cell**: `{header, value}`
  (`HDR_CELL_PTR` when the value is a pointer). **String**: `{header, len, utf8 ptr}`.
- **Blocks**: up to 512 bytes from 1 MiB chunks with size-class free lists (16-byte granules);
  larger ones from the system allocator. Nothing moves, so a pointer Rust holds stays valid while
  the object is reachable.
- **Tracing**: which slots of a struct are pointers is a mask per struct that the generated
  `ts_init` registers (`ts_rt_register_struct`). `T | null` pointers are traced (zero is skipped);
  numbers, `number | null` and tags are not.

Roots:

1. **Shadow stack** in generated code. A function that has pointer locals (or needs temporaries)
   reserves a Cranelift stack slot `{prev, n, slots[n]}`, links it with `ts_rt_frame_push` and
   unlinks it with `ts_rt_frame_pop` before each return. Every assignment to a pointer local also
   stores the pointer into its frame slot, and pointer parameters are stored in the prologue. A
   pointer value computed early and held while a *later* operand allocates (an argument list, an
   index) is spilled into an extra slot; `frames.rs` decides this once, for both the slot count and
   the code, with a conservative "can this expression allocate" test. Registers and Cranelift's own
   spill slots are never scanned.
2. **Globals**: `ts_init` registers the address of every pointer global.
3. **Host handles**: Rust keeps script pointers across calls into generated code by rooting them
   (`ts_rt_root_add` / `ts_rt_root_remove`). The bindings' `HDR_*` and `ts_rt_obj_new` let the host
   build script objects.

When it runs: only inside allocations made by generated code (`ts_rt_obj_new_gc`,
`ts_rt_arr_new_gc`, `ts_rt_bytes_new_gc`), once 512 KiB (or as much as survived the last
collection) has been handed out since the previous one; or when the host calls `ts_rt_gc_collect`.
The host-side constructors and `ts_rt_arr_push` never collect, so Rust can build a graph out of
unrooted pieces and root it afterwards, and a pointer being pushed is never freed mid-push.

**Stress mode** (`ts_rt_gc_stress`, or `IRON_TS_GC=stress` for the tests and benchmarks) collects at
every allocation of generated code and fills freed memory with 0xDD. A missing root then crashes
loudly instead of corrupting memory rarely. All of the following pass in it with the same results
as QuickJS and bun: the 13 ported tests, 1,500 steps of the busy scenario (37,000 collections), and
100 frames of the sine curve. That is the evidence for the root analysis; there is no proof.

Cost, on the busy scenario (10,000 steps, 65 MiB handed out in all, 124 collections, about 117 KiB
alive after each): 0.88 µs a step with the collector, 1.03 µs with automatic collection switched off
(shadow stack still on; the free lists keep memory warm, which is why collecting is not slower).
The sine curve went from 0.030 ms to 0.043 ms a frame against the bump arena it replaced: each
frame makes about 1,450 small objects and the collector sweeps them. Pointer-heavy functions pay
two runtime calls (frame push and pop) per call.

Known gaps:

- The root analysis is a conservative approximation, not a proof. Writing tests against it found
  holes, all now fixed and covered (stress mode, `bench/gap.ts`, `tests/gc.rs`):
  a pointer read from a field and held as an argument while a later argument overwrites the field
  and allocates (`f(h.item, replace(h))`, `M9-TS-27`); a local reassigned while its old value waits
  as an argument (`f(a, (a = g(), h()))`, `M9-TS-28`); a pointer waiting to be *compared* with
  `===`, which could come out equal to a new object that took the freed block (`M9-TS-29`).
  JavaScript keeps an evaluated operand alive, so the native code must. Now only a local that no
  later operand assigns, a `const` global and null are trusted; every other pointer held while a
  later operand can allocate is spilled to a frame slot. A `let` global is trusted only when no later
  operand contains a call (the callee may assign it) or a store to it: `sum(current, swap())` with
  `swap` assigning `current` freed the old item (`M9-TS-48`, `49`; both fail if `let` globals are
  trusted like constants). The code generator also refuses to compile a
  function whose spills would overrun its frame (the plan and the walk must agree).
  What it still assumes: no pointer lives only in a register or Cranelift spill slot across a
  call, other than as an operand of the node being evaluated. That holds for the IR the lowering
  produces; it is not checked.
- Several modules in one program: struct ids in object headers must not overlap, or the second
  module's `ts_init` overwrites the first's table of pointer slots (`M9-TS-30`). Each module is
  compiled with its own `struct_base` (`compile_with`, `--struct-base`); registering a different
  table under a taken id is refused (`M9-TS-33`).
- Module globals are shared between threads but heaps are per thread. A header carries its heap's
  id, and a collector ignores objects of other heaps, so one thread cannot set mark bits in
  another's objects and make it skip them (`M9-TS-31`). Script objects still must not be used
  across threads.
- The collection threshold counts array data, not only object headers; a script holding 8 MB of
  numbers would otherwise collect every 512 KiB (`M9-TS-32`).
- No incremental or generational collection; a collection walks the whole live heap. Fine for
  100 KiB of state; a game with 100 MB of script state would want generations.
- Closure `env` pointers are host memory. Closures created in script (not yet supported) will need
  captured cells that are GC objects.
- The frame push/pop could be cheaper: the top-of-stack pointer is thread local behind a call; a
  global (single script thread) or a register would make it a store.
- Memory is never returned to the OS (chunks stay), only reused.

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
| QuickJS, compute only | 1.13 ms | 1x |
| QuickJS, compute + `__iron_op` + draining the op into Rust | 1.26 ms | (0.12 ms is the crossing) |
| bun (JIT) | 0.048 ms | 24x |
| native, compute (with the GC) | 0.045 ms | **25x** |
| native, compute + Rust copying the points out of script memory into its own `Vec<f32>` | 0.043 ms | **26x** |

What it says:

- Most of the old cost was the **calculation in the interpreter** (1.14 ms of 1.26), not the
  boundary; but the boundary is not free: ~0.12 ms here to move 2,166 numbers as `Value`s, and the
  real app adds its own conversion (the demo's commit message measured ~0.2 ms).
- With native code there is no `Value` conversion. The array lives in script memory with a known
  layout (`{header, len, cap, data}` of f64); Rust reads the `data` slice in place. The "copy"
  row includes narrowing to f32 and costs nothing measurable. A direct engine call
  (`render.stroke(points: *const f64, n)`) would remove even that copy.
- The frame is allocation heavy (361 + 1,083 small objects and a growing array), so it exercises
  the allocator. With the first version's bump arena reset after every frame it cost 30 µs a frame
  (80 µs before the reset kept its first chunk instead of handing 4 MiB back to the OS each time).
  With the GC of section 6 (nothing to call: it collects itself) it costs 43 µs, about the same as
  bun. Escape analysis would let frame temporaries skip the heap again.
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
| 1. Language coverage | ~~Closures with capture cells~~ (done, section 4), classes + methods + `this`, destructuring, defaults, `switch`, template literals and strings, `Map`, generics by monomorphisation, enums | 6-8 weeks |
| 2. Real types | Bun sidecar that dumps the TypeScript checker's per-expression types as JSON; map to IR types; drop the in-compiler inference | 3-4 weeks |
| 3. Direct engine calls | Expose engine functions as `extern "C"` with shared layouts instead of `__iron_op` value conversion. `Vector3` = glam `Vec3` (a value type in the IR: 3 x f32 in registers or `f32x4`), `Quaternion`, `Color`; `Mathf` and vector arithmetic as intrinsics emitted inline; component lifecycle (`onUpdate`) entered through a function table; physics/scene/audio ops as direct calls | 4-5 weeks |
| 4. GC | **Spike done** (section 6: shadow stack, non-moving mark-sweep, stress mode). Left: generations or incrementality if heaps get big, escape analysis for frame temporaries, cheaper frames, GC-allocated closure cells | 1-2 weeks |
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
| `tools/iron_ts/src/codegen.rs`, `frames.rs` | IR to Cranelift object; shadow-stack frame planning |
| `tools/iron_ts/src/layout.rs` | generated Rust bindings |
| `tools/iron_ts/src/bin/iron_ts.rs` | CLI: `iron_ts file.ts -o out.obj --bindings out.rs [--opt none\|speed] [--time N] [--dump-ir]` |
| `tools/iron_ts_rt/` | runtime (GC heap, roots, arrays, strings, closures, libm calls) |
| `tools/iron_ts_tests/` | build script (compile + link `sim.ts`), `tests/sim_port.rs` (13 ports), `tests/gc.rs` (collector under the sim), `bench/bench.ts` and `src/bin/bench.rs` (the benchmark) |

Run (from `tools/`, with `PATH` including cargo and bun):

```bash
cargo test --workspace
```
```bash
cargo run --release -p iron_ts_tests --bin bench
```

## 11. What real gameplay code needs (measured 2026-10-18)

Line and feature counts, to order the work after the spike. Counted with `grep` over non-test
`.ts`/`.tsx` files; approximate (a count is lines that match, not uses).

| Feature | `js/engine/src` (11,468 lines) | arcade (799) | zombie game (1,375) | platformer (274) |
|---|---|---|---|---|
| `class` / `extends` | 87 / 58 | 4 / 4 | 1 / 1 | 4 / 4 |
| `@property` decorators | 187 | 7 | 0 | 0 |
| `this.` | 1,416 | 37 | 93 | 24 |
| getters / setters | 212 | 2 | 1 | 0 |
| `??` and `?.` | 301 | 4 | 27 | 0 |
| object / array spread | 96 | 6 | 18 | 0 |
| `Map` / `Set` | 64 | 1 | 13 | 1 |
| template literals | 84 | 9 | 17 | n/a |
| string methods (`split slice indexOf ...`) | 51 | 3 | 0 | n/a |
| `try` / `throw` | 22 / 25 | 0 | 0 | 0 |
| `instanceof` / `typeof` | 49 / 45 | 0 | 0 | 0 |
| `switch` | 7 | 0 | 0 | 0 |
| `async` / `await` | 6 | 0 | 0 | 0 |
| JSX files | 0 | 1 | 1 | 0 |

Reading it:

- **Classes dominate.** A game's gameplay is classes that extend the engine's `Component`, with
  `@property` fields and `override`d lifecycle methods. A compiler without classes compiles
  helpers like `sim.ts` (which the zombie game deliberately separated from `main.tsx`) and not the
  game. Classes and the engine boundary have to be designed together (`Component` and `Vector3` are
  themselves TypeScript classes today).
- **Strings and templates** are everywhere in the engine and a little in games; the UTF-16
  question needs an answer before any string work.
- **Module-level mutable variables** (`let demo: SineDemo` assigned from methods) are common in the
  examples; supported since `M9-TS-47`..`56` (section 4).
- `try`/`throw` is used by the engine itself but not by gameplay code written in the examples; it
  can wait behind classes and strings.
- UI (JSX) is one file per game; it stays on QuickJS (the hybrid plan).
- `Map`/`Set` are frequent enough that array-backed sets are only a stopgap.

## 12. Loops and labels (stage 1 of the language features)

Design, written before the code. The IR has `Loop(label)` (an infinite loop; `Continue(label)` jumps
to its top, `Break(label)` leaves it) and `Block(label)` (`Break(label)` leaves it). Source loops
map onto those:

| Source | IR |
|---|---|
| `while (t) b` | `Loop L { if !t break L; b }`, `continue` = `Continue L` |
| `for (init; t; u) b` | `init; Loop L { if !t break L; Block C { b }; u }`, `continue` = `Break C` |
| `do b while (t)` | `Loop L { Block C { b }; if !t break L }`, `continue` = `Break C` |
| `for (x of xs) b` | as before (the iterator's head runs the step) |
| `name: <loop>` | the loop takes the name; `break name` / `continue name` look it up |
| `name: { ... }` | `Block B { ... }`, `break name` = `Break B` |

A `continue` therefore has to run the update (`for`) or the test (`do...while`), which is why the
body sits in its own block and `continue` leaves that block instead of jumping to the loop top.
Lowering keeps a stack of `LoopLabels { name, brk, cont, unlabeled_break }` per function: an
unlabelled `break` takes the innermost loop (or later switch), an unlabelled `continue` the
innermost loop, a labelled one the entry with that name. Inlined array callbacks start a fresh
stack (a `break` there is an error, as in JS).

**Per-iteration bindings.** JS gives a `let` in a `for` head a new binding each iteration. When a
closure captures it, it is boxed in a cell (closures section above); the lowering then copies the
value into a new cell at the end of the body, before the update, so a closure made in iteration k
keeps iteration k's value and the update changes the next iteration's copy. Uncaptured variables
are plain locals and cost nothing.

Not done here: `for...in`, `for await`, `for (x of ...)` over an existing variable or a
destructuring pattern (destructuring is stage 3).

Tests: `tests/loops.rs` against the numbers bun computes from `bench/loops.ts`, in all three GC
modes (`M9-TS-57`..`63`).

## 13. `switch` and enums (stage 2)

Design, written before the code.

**Enums.** `enum` and `const enum` are the same here and cost nothing at run time. A numeric enum is
a `number`; its members are constants (`Up = 0`, the next member is the previous plus one, an
initialiser may be any constant expression over earlier members: `B = A << 1`). A string enum is a
union of string tags (`Ty::Lit`), so `Mood.Sad` is the tag of `"sad"` and compares with a string
literal. The enum's name is a type (`d: Dir`), `Dir.Up` folds to its value wherever it appears.
Constant folding in `arith` learned `%` and the bit operations (`&`, `|`, `^`, `<<`, `>>`, `>>>`
with JavaScript's ToInt32), because enum initialisers and flag combinations are constants.
Not done: reverse mapping (`Dir[0]`), computed or non-constant members, enums merged across
declarations, `declare enum`.

**`switch`.** JavaScript compares with `===` in source order, falls through into the next clause
unless a `break` leaves the switch, and `default` may sit anywhere (it runs only if nothing
matches, and falls through into whatever follows it). The IR has no jump table, so:

```text
tmp = discriminant            (evaluated once)
idx = index of default, or the clause count
Block sel { if tmp === c0 { idx = 0; break sel }  if tmp === c1 { idx = 1; break sel }  ... }
Block brk { if idx <= 0 { body0 }  if idx <= 1 { body1 }  ... }
```

`idx` picks the clause, then every body from there on runs in source order until a `break`
(`Break brk`) leaves the block. That gives fall-through and a default in the middle with no special
cases. Case expressions are evaluated lazily, in order, and may allocate. `continue` inside a switch
skips the switch's entry (it has no continue target) and reaches the enclosing loop; `break label`
works on a labelled switch. All clauses share one scope, as in JS.

A linear chain of compares is fine for the switches in gameplay code (a handful of clauses); a
dense numeric switch could become a Cranelift jump table later (Performance).

Tests: `tests/switches.rs` against bun's numbers from `bench/switches.ts`, in all three GC modes
(`M9-TS-64`..`69`); error cases are a unit test (`M9-TS-70`).

## 14. Destructuring, default and rest parameters, spread (stage 3)

Three features that share one idea: take a value apart, or fill in what is missing, at the
boundary of a declaration or a call. Each is its own commit.

### 14.1 Destructuring (done)

`bind_pattern(pattern, value, out)` binds the names of a pattern to parts of an already lowered
value, recursively:

| Pattern | Lowering |
|---|---|
| `x` | a new local (a cell when a closure captures it) |
| `{ a, b: p }` | the value in a temporary; `a` is a field read; `p` is bound to the field `b` |
| `[p, , q]` | the value in a temporary; element `i` is an `Index`; holes are skipped |
| `p = d` | `Cond(IsNull(v), d, v)` when `v` can be null (a missing property or a short array); a value that cannot be null ignores the default |

It is used by variable declarations (`const { x, y } = p`), parameters of functions, closures and
inlined array callbacks (`({ x }) => x`), and `for...of` variables (`for (const [a, b] of rows)`).
An array element reads as null past the end (a zero pointer, or the null NaN for numbers), which is
what lets `[a, b, c = 5]` give `c` its default. The default expression is lowered where the
pattern is, so it may allocate or call; the temporaries are ordinary locals, rooted by the frame
plan like any other.

Not done: rest elements (`{ a, ...rest }`, `[a, ...rest]`; the object case needs an anonymous
struct for the remainder, the array case needs `slice`), destructuring assignment to existing
variables (`[a, b] = [b, a]`), computed keys, tuple types.

Tests: `tests/destructure.rs` against bun's numbers from `bench/destructure.ts`, in all three GC
modes (`M9-TS-71`..`76`).

### 14.2 Default, optional and rest parameters (done)

JavaScript evaluates a default when the call happens, in the callee's scope (it can see earlier
parameters). The compiler sees every direct call of a declared function, so the **caller** fills
the missing arguments in:

- `p = d`: the default `d` is lowered at the call site. The given arguments are first stored in
  temporaries, and `d` is lowered in a scope that holds only the parameters before it (by name,
  bound to those temporaries) plus the module's names, so `hi = lo + 10` works and the caller's own
  locals cannot leak in. The result is `Seq([tmp = arg..., tmp = default...], Call(f, tmps))`.
  When every argument is given, nothing changes (no temporaries).
- `x?: T`: the parameter is `T | null` (a zero pointer or the null NaN); a missing argument is null.
- `...xs: T[]`: the function takes one more parameter, an array. The call site builds it from the
  remaining arguments, including `f(a, ...b)`, always as a **new** array (JS makes a fresh array for
  a rest parameter, so writes to it must not reach the caller's array).

The callee therefore never sees a missing argument, and the compiled function is an ordinary
function with all its parameters. The cost is one rule: a function with defaults, optional or rest
parameters that is used **as a value** (`xs.map(f)`, stored in a variable) has to be called with
all its parameters, because the function type lists them all. That is a compile error with the
function's name, not a silent difference.

Not done: closures with defaults called with fewer arguments (arrow functions in variables; the
call goes through the function type), spreads into fixed parameters (`f(...[1, 2])`), parameter
properties.

### 14.3 Spread (arrays and calls, done; objects were done earlier)

`[...a, x, ...b]` builds a new array (`build_array`): a push for each value and, for a spread, a
loop over the source (`begin_iter`, the same loop `for...of` uses). The element type comes from the
expected type or the first element. Object spread (`{ ...a, x: 1 }`) was already lowered field by
field. Spread in a call only feeds a rest parameter (14.2). Not done: spreading a `Set`/`Map`/string
(sets are arrays here, so `[...set]` works; maps and strings wait for the strings item).

Tests: `tests/params.rs` against bun's numbers from `bench/params.ts`, in all three GC modes
(`M9-TS-77`..`83`); argument-count errors are a unit test (`M9-TS-84`).

## 15. Library surface (stage 4)

### 15.1 `Math` and numeric operators (done)

Every `Math` function the compiler knows becomes either an inline machine operation or a call to a
small runtime function (`ts_rt_*`, in `iron_ts_rt`), never a call into JavaScript.

| Function | Lowering | Notes |
|---|---|---|
| `sqrt abs floor ceil trunc` | one Cranelift instruction | `ceil(-0.5)` is `-0`, as in JS |
| `round` | `floor(x)`, plus 1 when `x - floor(x) >= 0.5`, then `copysign(_, x)` | JS rounds halves toward +Infinity (`round(-2.5)` is `-2`), keeps `-0`, and `round(0.49999999999999994)` is `0` (`floor(x + 0.5)` gets that one wrong) |
| `fround` | demote to f32 and promote | |
| `clz32` | ToInt32 then `clz` | |
| `sign min max hypot imul atan2 sin cos` | as before | |
| `tan asin acos atan exp log log2 log10 cbrt` | a runtime function over the C library | |
| `pow`, `**`, `**=` | `ts_rt_pow` | C's `pow` except where JS differs: `1 ** NaN` and `(+-1) ** Infinity` are NaN |
| `random` | `ts_rt_random`: xorshift128+, per thread | seeded from the clock on first use; `ts_rt_random_seed(n)` makes a run repeatable |
| `PI E SQRT2 LN2 LN10 LOG2E LOG10E SQRT1_2` | constants | |

Operators added with them: `**`, `**=`, unary `~` (`ToInt32(x) ^ -1`), and the logical assignments
`??=`, `||=`, `&&=`. A logical assignment is `if (cond(target)) target = value`: the target is
evaluated once (`xs[i++] ??= 10` bumps `i` once) and the right side only when it is needed.
Constants fold: `2 ** 10` and `1 << 3` are numbers at compile time (needed by enum members).

**Known difference.** V8 computes `sin cos tan asin acos atan exp log ...` with its own port of
fdlibm; we call the platform's C library. Both are within an ulp of the true value but can differ in
the last bit, so a program that prints these to 17 digits or hashes them can differ. The tests
compare these functions after scaling to nine decimals. This belongs to the conformance item.

Not done: `sinh cosh tanh asinh acosh atanh log1p expm1`, `Math.max()`/`Math.min()` with no
arguments or an array spread, `in`, `void`, `delete`, optional call `f?.()`.

Tests: `tests/mathlib.rs` compares bit for bit with bun's values from `bench/mathlib.ts`
(`M9-TS-85`..`91`).

### 15.2 Array methods (done)

Methods that move elements are small runtime functions over raw 8-byte slots (so one routine serves
numbers and pointers); methods that run script code per element are loops with the callback
inlined, as `map`/`filter` already were.

| Method | Lowering |
|---|---|
| `pop`, `shift` | `Index` (null past the end) plus `ts_rt_arr_set_len` / `ts_rt_arr_drop` when non-empty; the result is `T \| null` |
| `unshift(...v)` | `ts_rt_arr_insert` per value, returns the new length |
| `slice(s, e)`, `concat(...)`, `splice(s, n, ...items)` | `ts_rt_arr_slice`, `ts_rt_arr_append`, `ts_rt_arr_splice` (returns the removed elements as a new array); negative indexes count from the end, as in JS |
| `reverse`, `fill(v, s, e)` | in place, return the same array |
| `sort(cmp)` | `ts_rt_arr_sort`: a stable merge sort in place, calling the comparator closure |
| `indexOf`, `lastIndexOf`, `findIndex`, `at`, `reduce` | inline loops (`reduce` binds the accumulator to the callback's first parameter) |
| `xs.length = n` | `ts_rt_arr_set_len`: truncates, or grows with zero slots |
| `Array.from({ length: n }, (_, i) => v)`, `Array.from(xs)`, `Array.of(...)`, `Array.isArray(x)`, `new Array(n)` | a loop, a copy, a literal, a constant, `ArrNew` + `set_len` |

**The GC and `sort`.** The comparator is script code: it allocates, so it can collect, while the
runtime holds the array, the closure and a merge buffer that no frame knows about. The sort
registers all three as temporary roots, zeroes the merge buffer first (a collection traces every
slot of an array, and a fresh buffer holds stale data), and the frame plan treats `ArrSort` as an
allocating operation. A stress-mode test in the runtime (`M9-TS-109`) fails when the roots are
left out: the elements that only the buffer holds come back poisoned. A zero-filled buffer was a
real bug found by the first stress run of the TypeScript test (`M9-TS-99`).

**Differences from JS, on purpose.**
- `sort()` without a comparator compares by string value in JS (`[10, 9].sort()` is `[10, 9]`).
  That needs number-to-string conversion, so it is a compile error that asks for a comparator.
- `reduce` without an initial value is an error (JS throws on an empty array).
- `new Array(n)` has no holes: slots are 0 or null, where JS reads `undefined`.
- `pop`/`shift`/`at` are only available over numbers and pointers (a `boolean[]` has no null).
- `splice` always builds the removed-elements array, even in a statement that ignores it.

Not done: `flat`, `flatMap`, `entries`, `keys`, `join` (strings), `findLast`, `reduceRight`,
`copyWithin`, `Array.from(xs, fn)` (use `xs.map(fn)`), typed arrays other than `Uint8Array`.

Tests: `tests/arraylib.rs` against bun's numbers from `bench/arraylib.ts` in all three GC modes
(`M9-TS-92`..`108`), the runtime test `M9-TS-109`, the error cases `M9-TS-110`.
