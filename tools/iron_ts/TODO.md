# iron_ts: working file

Tracks the ahead-of-time TypeScript compiler (`tools/iron_ts`, `tools/iron_ts_rt`,
`tools/iron_ts_tests`). The repo-wide plan is `TODO.md` M9.b2; **this file is where progress is
kept up to date** and where a fresh session (or a context after compaction) starts. Update the
"Now" and "Log" sections at the end of every step. Design and measurements:
`docs/research/iron-ts-spike.md`. Test IDs: `TESTS.md` section "M9.b" (`M9-TS-*`, next free: **64**).

## How to pick this up

- Branch `claude/recursing-wright-856ffa` (worktree `.claude/worktrees/recursing-wright-856ffa`).
  Commit hashes were rewritten once by another session; do not rely on old ones.
- Do not touch `crates/iron_render` or `crates/iron_app` (another session works there).
- Shared notes folder for hand-overs: `C:\Users\quack\AppData\Local\Claude-Data\claude_pzs8sxrjxfjjc\side-session-notes\local_988df2ba-1803-40db-870f-74c0086082a3`.
- Shell: `export PATH="$HOME/.cargo/bin:$HOME/.bun/bin:$PATH"`. `tools/` is its own cargo workspace;
  run cargo from `tools/`.

```bash
cd tools && cargo fmt --all
```
```bash
cd tools && cargo clippy --workspace --all-targets -- -D warnings
```
```bash
cd tools && cargo test --workspace
```
```bash
cd tools && IRON_TS_GC=stress cargo test -p iron_ts_tests
```
```bash
cd tools && cargo run --release -p iron_ts_tests --bin bench
```
```bash
cd tools && cargo run --release -p iron_ts_tests --bin sine_bench 2000
```

- Every change that touches lowering, codegen or the runtime is also run with `IRON_TS_GC=stress`
  (collect at every allocation, poison freed memory). It has caught real bugs three times.
- CLI: `target/release/iron_ts.exe file.ts --dump-ir [--prefix p_] [--struct-base N] [-o out.obj]`.
  Build it with `cargo build --release -p iron_ts`.
- Commits: one per step, message ends with `Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>`;
  run fmt, clippy, tests (and the zombie fixture's `bun test` if `fixtures/` changed) first; update
  `TESTS.md` and `TODO.md`; keep `docs/research/iron-ts-spike.md` current.
- Machine gotchas: bash heredocs mangle apostrophes and backslashes. For multi-line edits write a
  Python script to the scratchpad with the Write tool and run it (that is how most edits here were
  made). Files are CRLF on disk: read with `newline=''` and keep the line ending. rustfmt reflows
  long lines, so patch text that matches the formatted code. A running `cargo build` of the same
  target locks files; do not edit during a background build.
- Panics inside `extern "C"` abort the process; test fallible logic through a Rust-level function.

## Map of the code

| Path | What |
|---|---|
| `iron_ts/src/lower/mod.rs` | program-level lowering: declarations, `ensure_lowered`, bindings (`Binding::Direct/Boxed`), `lower_body_into` |
| `iron_ts/src/lower/types.rs` | annotations to IR types, structs, `coerce`, `to_bool` |
| `iron_ts/src/lower/expr.rs` | expressions, places (assignment targets), calls, `Math` |
| `iron_ts/src/lower/stmt.rs`, `arrays.rs` | statements; array methods with inlined callbacks, `Iter`, `emit_return` |
| `iron_ts/src/lower/scan.rs`, `closure.rs` | which variables closures capture; lifting closures to functions, function values |
| `iron_ts/src/ir.rs` | the IR and its dump |
| `iron_ts/src/frames.rs` | shadow-stack frame plan; **the spill rule is the GC's correctness argument** |
| `iron_ts/src/codegen.rs` | IR to Cranelift object (frames, spills, `eval_args`, registration in `ts_init`) |
| `iron_ts/src/layout.rs` | generated Rust bindings (`HDR_*`, `TAG_*`, structs, `extern "C"`) |
| `iron_ts_rt/src/lib.rs` | GC heap, roots, arrays/strings/closures/cells, libm calls |
| `iron_ts_tests/` | `build.rs` compiles `fixtures/sim.ts` plus `bench/{sine,gap,closures,globals,loops}.ts` (prefix and struct base each); `tests/{sim_port,gc,closures,globals,loops}.rs`; `src/bin/{bench,sine_bench}.rs` |

Adding a test module: a `bench/<name>.ts`, an entry in `build.rs`'s list `(name, prefix, struct_base)`
(bases are 1000 apart: sim 0, sine 1000, gap 2000, closures 3000, globals 4000, loops 5000, next 6000), a `pub mod` in
`src/lib.rs`, a test file. Expected numbers come from running the same TS in bun.

## Rules we decided (do not undo without asking)

- A box in `TODO.md` is ticked only when the whole item is done; finished parts are ticked sub-items.
- Items marked **STOP** get a written design note and the user's review before code.
- GC roots: a pointer held while a later operand can allocate is spilled to a frame slot unless it is
  a local no later operand assigns, a global or null (`frames.rs`). Any new IR node that evaluates
  operands must use `eval_args` and be counted in `frames.rs` (`count_expr`/`count_stmts`,
  `may_alloc`, `children`).
- Each module linked into one program gets its own `struct_base`; headers carry the heap id.
- Numbers are f64 everywhere; `number | null` uses a reserved NaN payload.
- Measured claims go in the doc with how they were measured; correct the doc when one turns out wrong
  (the "33 bullets in flight" claim was wrong and was fixed).

## Now

**Classes: design note written (`docs/research/iron-ts-classes.md`), waiting for the user's answers to its
section 11.** No class code until then (STOP item). Meanwhile the non-STOP stages go ahead, each with its
design paragraph written first (spike doc, one section per stage) and its own commit:

1. ~~`for(;;)`, `do...while`, labels~~ done
2. `switch` and enums (doc: spike doc section 13)
3. default and rest parameters, destructuring, spread
4. library surface: `Math`, array methods

Mark the stage you are on here and tick it in `TODO.md` M9.b2 only when the whole sub-item is done.

## Backlog

The full list with sub-tasks, sizes and STOP gates is `TODO.md` M9.b2. Open top-level items: type
sidecar, GC (verifier, cheaper frames, escape analysis, pauses), closures, classes, generics, structural typing, strings, language features, library
surface, JS-semantics conformance, `JsValue`, direct engine calls, hybrid release builds,
performance, cross targets and CI.

## Log

Newest first. One line per finished step: what, test IDs, numbers if measured.

- 2026-10-18 Loops (`M9-TS-57..63`): `for(;;)`, `do...while`, labelled `break`/`continue`, labelled blocks,
  per-iteration `let` cells in `for` heads (spike doc section 12).
- 2026-10-18 Classes design note written (`docs/research/iron-ts-classes.md`); awaiting review.
- 2026-10-18 Mutable module-level variables (`M9-TS-47..56`): `let`/`var` are data slots (never folded),
  `Place::Global`, `GlobalVar` read node, `let x: T | undefined;`, closures stored in and called
  through globals; a `let` global is a stable GC operand only when no later operand calls or stores
  (two hazard tests fail under the old rule). Found on the way: global slots were 1-byte aligned.
- 2026-10-18 Spike tracking file created (this file). Closures, GC review, GC and the sine benchmark
  are done as described below; next is the first task above.
- 2026-10-18 TODO split into honest sub-tasks; measured feature counts (spike doc section 11).
  Gameplay code is classes: `extends Component`, `@property`, `this`, getters/setters.
- 2026-10-18 Closures (`M9-TS-34..46`): name scan, cells, lifted lambdas, function values; all run
  normally, in stress mode and with collection off. Busy sim 0.94 us/step.
- 2026-10-18 GC review (`M9-TS-29..33`): pointer comparisons rooted, struct id ranges per module,
  heap ids in headers, array data in the threshold, frame-plan overrun guard.
- 2026-10-18 GC root analysis tightened (`M9-TS-27, 28`): a field read or reassigned local held as an
  argument was freed; both tests failed first.
- 2026-10-18 Precise mark-sweep GC with shadow-stack roots, globals, host handles, stress mode
  (`M9-TS-20..26`). Busy sim 0.88 us/step; sine curve 0.043 ms/frame.
- 2026-10-18 Sine-circle curve benchmark (`M9-TS-19`): native ~25x QuickJS (0.045 vs 1.13 ms/frame),
  level with bun. Constant folding, integer `%` fast path, symbol prefixes.
- 2026-10-18 Spike: oxc to typed IR to Cranelift object; `sim.ts` compiles, 13/13 ported tests
  (`M9-TS-01..13`), busy sim ~46x QuickJS and 4.5x bun (50 parked bullets, 10 zombies).
