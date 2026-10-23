# iron_ts: start-up and hot reload (research notes)

Status: discussion notes, 2026-10-23. Nothing here is built; the measurement is queued. Related: `iron-ts-type-checking.md`, `docs/benchmarks.md`, `docs/iron-build.md` ("`game.irbundle`").

## 1. The problem

emoji starts in 2.2 s native against 1.2 s on QuickJS (about 4,700 lines of its own source plus the preludes it uses). The large TypeScript `game.irbundle` adds about a second at every launch. Script run time is not the issue (native script time is 5 to 25x below QuickJS, `docs/benchmarks.md`).

## 2. What the build produces today

`dist/game.irbundle` is JSON holding the bundled TypeScript source text (`source`). `iron-build build` resolves modules, strips imports, runs `tsc` and writes that text; it does **not** compile to machine code. At launch `iron_app` compiles the text in process with the Cranelift JIT, so every launch pays parse, inference/lowering (including the preludes the game references, loaded by the `wants_*` checks), Cranelift code generation and JIT linking. An `--aot` path (`dist/game.o` plus `game_bindings.rs`) exists but needs a native link step.

The hot-reload path recompiles the source in process when it differs (the old code is kept on error), which is why the compiler runs at launch.

## 3. The idea (user)

Compile ahead of time and keep a checksum: if the source is unchanged, load the stored result; recompile at run time only when it differs (hot reload, or a new host build). The dev loop is unaffected.

## 4. Cache levels (rising effort)

1. **Cache the intermediate form** (parsed/typed/lowered IR). Skips parsing, inference and lowering; still runs Cranelift at launch. Simple and safe.
2. **Cache relocatable object code.** Cranelift can emit object files (`--aot` already writes `dist/game.o`). A small loader maps the object and patches host function addresses (they change per host build, so raw JIT output cannot be stored). Skips the whole compiler; needs a loader per platform object format.
3. **Link the object into the host at package time.** Fully AOT; needs a linker step in `iron-build package`. The right answer for shipped games, and for iOS, which forbids writable-executable memory (the doc already outlines it).

**Cache key:** source hash + compiler version + host engine-interface version. On mismatch compile from the source text, as now; on a cache error, fall back the same way.

## 5. Measure first (Decided)

The split between parse, lowering, code generation and linking is **not yet measured**. The incremental-sync agent was asked to report, for emoji, fog-world, godrays-room and hello, the time in: bundle read, oxc parse, inference/lowering (including referenced preludes), Cranelift code generation, JIT finalise, engine/asset load and first frame, plus which of those a cache of (a) lowered IR or (b) relocatable object code would remove. Choose between levels 1 and 2 from those numbers; do not design before them.

It was also asked to split the QuickJS "script" column into script proper and engine ops inside script calls (and animation, spline and camera follow, which native counts under "sync"), so native and QuickJS script time can be compared like for like.

## 6. Open items

- Start-up split measurement (queued).
- Decision between cache levels 1 and 2 (level 3 for packaged games).
- Hot-reload behaviour with a stale cache (key mismatch, partial write: write atomically as `game.irbundle` already is).
- Whether preludes should be chosen by reachability rather than by source text mentions.
