# The sine-circle demo through iron_ts: a metric and a swap-out check

Written 2026-10-19. Benchmarks: `cargo run --release -p iron_ts_tests --bin sine_bench 1000`, `--bin ui_bench 1000`,
`--bin bench` (from `tools/`, bun on the PATH). Machine: the development PC, release build, best of 3 to 7 runs.

## What is measured

**The demo's own class, verbatim.** [sine_demo.ts](../../tools/iron_ts_tests/bench/sine_demo.ts) is the `SineCircle` class
copied from `examples/sine-circle-demo` at commit `9dc2f51` ("a point every degree with the original's smoothing"), before
`LineRenderer2D` and the poly shapes: 361 points a degree through `at(d)`, a Catmull-Rom spline with 3 steps per segment, and
one `new Vector3(..)` per output point (1,084 a frame). The same source is run by:

- QuickJS, bundled by bun together with the real `@iron/engine` (so `Vector3` is the engine's class);
- bun (JIT), for reference;
- `iron_ts`, where `Vector3` is the engine API's value class (`Scene` manifest) and the code is native.

| 1,000 frames | total | per frame | checksum |
|---|---|---|---|
| QuickJS (engine `Vector3`) | 1,205 ms | 1.205 ms | 1084858.0403 |
| bun | 44.6 ms | 0.045 ms | 1084858.0403 |
| iron_ts native | 36.1 ms | **0.036 ms** | 1084858.0403 |

Native is **33x** QuickJS; the checksums agree to every printed digit (the maths is the same f64 maths; the points are not
narrowed to f32 until they cross into the engine).

**The hand-typed copy** ([sine.ts](../../tools/iron_ts_tests/bench/sine.ts), flat `[x, y, ...]` output as `LineRenderer2D` sends it):
QuickJS 1.04 ms/frame, +op to Rust 1.12 ms, bun 0.045, native 0.043 (+copy 0.042): 24x, as measured on 2026-10-18.

**Zombie sim** (`bench`, 10,000 steps, 10 zombies, 50 bullets): QuickJS 39.9 us/step, bun 3.7, native 0.90: 44x.

**UI** (`ui_bench`, a list of keyed rows into a real `UiTree`, the same app written for React and for the native reconciler;
React runs in QuickJS through `@iron/react`, commits decoded and applied by Rust; the native column includes building the
descriptions in compiled code, the diff and applying the ops):

| 1,000 rows | React prod / QuickJS | React dev / QuickJS | native TSX | prod / native |
|---|---|---|---|---|
| mount | 63.9 ms | 99.4 | 8.1 | 7.8x |
| change one row | 29.1 | 66.0 | 2.5 | 11.8x |
| change every row | 27.2 | 65.9 | 2.4 | 11.4x |
| reverse | 100.5 | 140.8 | 3.0 | 33.5x |

At 100 rows the ratios are 6x to 16x (mount 3.5 ms against 0.6). Both sides emit the same ops (7,002 for the 1,000-row
sequence). Neither side memoises rows: every render rebuilds all descriptions, so "change one row" costs the same as "change
every row" in both.

## Swap-out: what the verbatim class needed

The goal is that gameplay code is a swap-out: the same TypeScript runs in QuickJS or compiled. Compiling the class as
written found these differences; the compiler was changed for the first, the others remain.

1. **Field initialisers without an annotation** (`radius = FORM_H / 3;`, `center = { x: CENTER.x, y: CENTER.y };`):
   iron_ts only inferred literals and `new Class()`. Fixed: the type is now found from the syntax (arithmetic, a top-level
   `const` without annotation, `C.x` of a constant object, object literals of those; `M9-TS-592`). A field initialised by a
   call still needs an annotation.
2. **`Color`** (`color = Color.white`) is not part of the compiled engine API: only `Scene`'s `Vector3` and `Quaternion` are.
   The bench copy drops that field. For a real swap-out `Color` (and `Gizmos`, `Camera`, `input`, `Time`, `audio`, ...) must be
   declared with `#[iron_api]` like `Scene`; until then a component using them does not compile.
3. **`import ... from "@iron/engine"`** is accepted and ignored by iron_ts; the names must exist in the manifest. In QuickJS the
   import is real. (Nothing to change in the source.)
4. **Value semantics**: `Vector3` is an immutable value class in compiled code (`v.x = 1` would be an error); the engine's
   QuickJS class is mutable. The demo never mutates one.
5. **Cost model**: compiled code allocates each `Vector3` as a GC object like QuickJS does; no escape analysis yet, so the
   per-point allocation (1,084 a frame) is still there. It dominates the remaining 36 us.
6. **Stress GC** (`IRON_TS_GC=stress`, a collection at every allocation) is a test mode: 6.1 ms/frame, not a performance figure.

Not exercised here but different: exceptions (native faults take the process down unless caught by `try`), `Math.random`
sequences (not seeded the same), and floating-point formatting (`toString` of numbers is exact JS formatting in both).

## What to do next for the swap-out

- Declare the engine classes the demos use (`Color`, `Gizmos`, `Camera`, `Time`, `input`) with `#[iron_api]`, then compile
  the whole `SineDemo` component (it extends `Component`, uses `@property`, `register`), not just the curve.
- Run the same component under both backends from one bundle: QuickJS through the typed `register_quickjs` bindings,
  compiled through the shims (the Scene namespace already has both, `M9-API-26`).
