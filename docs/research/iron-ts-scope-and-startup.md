# iron_ts: scope, type checking, and start-up (research notes)

Status: discussion notes, 2026-10-23. Nothing here is built. Decisions are marked **Decided**; the rest are options with a recommendation. Related: `iron-dynamic-values.md` (the `any`/`unknown` design), `iron-generated-dts-plan.md`, `docs/benchmarks.md`.

## 1. Why this came up

The benchmark (`docs/benchmarks.md`) shows native script time 5 to 25x below QuickJS, but a few examples (fog-world, six clock faces) run slower than QuickJS because of `sync_native_render`, not the language. The same discussion raised how much of TypeScript Iron should carry, whether to own a type checker, and why emoji starts in 2.2 s native against 1.2 s on QuickJS.

## 2. Type checking

- **Today:** `tsc --strict` is the only real checker (`iron-build` runs it when found, skipped as `Tsc::Missing` otherwise). iron_ts infers types for code generation and trusts that `tsc` already accepted the programme (narrowing, union membership, and in the dynamic-value design `unknown`'s restrictions).
- **Why a full checker is hard:** structural assignability (recursive, with cycles), generic inference with contextual typing and overloads, type-level programming (conditional, mapped, template literal types, `infer`, `keyof`) which the standard library itself uses, flow-sensitive narrowing, declaration merging, variance. The long tail of edge cases is the cost.
- **Copying the native TypeScript port (`typescript-go`, `tsgo`):** TypeScript is Apache-2.0 (verify the port's licence and NOTICE file before copying anything). It is Go, so Iron would be a rewrite not a copy, wired to `oxc`'s AST instead of its own parser/binder, needing TypeScript's `lib.*.d.ts` files, and it would have to track TypeScript's behaviour changes. Not recommended. No finished Rust checker is known (`stc` was abandoned; unverified recently).
- **Options, cheapest first:**
  1. Use `tsgo` as the optional external checker (no code copied; needs a short investigation of its output format and CLI). Recommended.
  2. Give the compiler its own TypeScript-style diagnostics for the subset it lowers (`file(line,col): error`), as an `iron-build --check` fast mode. Risk: more permissive than `tsc` lets programmes through that the compiler then trusts; stricter rejects valid ones.
  3. Port the checker (months). Only if a Node-free, `tsc`-free toolchain becomes a goal.
- **Reasoning:** type-level features only matter to a checker. While `tsc`/`tsgo` checks the programme Iron gets them free, and the compiler needs only the resolved types. Owning a checker is the one move that imports the metaprogramming complexity.

## 3. tsconfig

Each example has a hand-written `tsconfig.json` (`extends ../../tsconfig.base.json`, `paths` for `src/modules`, `include: ["src"]`). The build already writes its own `dist/.tsc/tsconfig.json` and the generated `types/iron.d.ts`. The example file matters only to editors.

- **Proposal:** the build uses an embedded default (strict mode, `paths` for `src/modules`, `include` from `project.json`). It writes a git-ignored `tsconfig.json` next to the generated `types/iron.d.ts` for editor support; `iron-build init` creates it for new games; a game may ship its own to override. Examples then need only `project.json` and `src/`.
- `tsconfig.base.json` pins `target: ES2022`, `lib: ["ES2022"]`, `types: []`: that is already the scope statement.

## 4. Scope (the complexity budget)

User concern: not adding language complexity or a meta-programming system that games (mostly components) do not need. **Proposed policy** for a "Scope" section in `docs/iron-ts.md` (not yet written):

1. **Core:** classes and interfaces, components, enums, unions and literal types, generics (monomorphised), closures, async/promises, arrays, maps, sets, destructuring, ESM modules.
2. **Added on demand only:** a feature is added when a shipped example, or a package someone really copies in, needs it, with a bun-compared test.
3. **Out of scope:** our own type-level evaluator or checker; decorators beyond `register`/`property`; namespaces and declaration merging; Proxy, prototype manipulation, `eval`.
4. Unsupported features give a clear error (already the case).

**ES2022 conformance table:** one row per ES2022 built-in or syntax (`Array.prototype.flat`, `Object.entries`, optional chaining, private fields...), status done/partial/out of scope, a bun-compared test per row, and a test that fails if a row is marked done without a test. The tsconfig target is the scope statement. Do not move files into an `es2022/` directory: `lower/` is already split by feature and a rename would cause merge conflicts for no gain.

**Dynamic `any`:** decided to build (see `iron-dynamic-values.md`), but under this policy only the first phases (`catch (e: unknown)`, untyped `JSON.parse`, `Record<string, any>`, property access) are planned unless a need appears; later phases stay optional.

## 5. Shrinking the language for speed (Lua-style)

Considered and not recommended as a performance measure:

- Unused features cost nothing at run time: unreferenced top-level code is not compiled, dynamic paths are paid only by values that use them, and the large preludes are already loaded on demand (`wants_*` checks in `lower/prelude.rs`: `wants_intl`, `wants_date`, `wants_regexp`, `wants_symbol`, `wants_task`, `wants_timers`, `wants_arraybuffer`; `Temporal` is loaded when the source rewrites to it, `lower/mod.rs`). The check works on source text, not reachability.
- Script time is already 0.02 to 0.3 ms per frame in most examples; the slow cases came from engine sync.
- Speed that is still on the table is codegen (unions are heap structs that allocate when wrapped, closures, exceptions, GC roots), not breadth.
- The real argument for a smaller language is maintenance and risk, which the scope policy addresses.

## 6. Comparison with C# / Unity (IL2CPP)

C# gives a mature typed language, compiler, runtime and GC; Unity only adds an engine. Iron owns a compiler, GC, standard library and declarations generator. IL2CPP also restricts the language (reflection, runtime code generation, some generics), which is the same trade as Iron's defined subset. TypeScript's reasons remain: React UI, a language web developers know, small builds without a .NET runtime, fast hot reload, one language for editor and game. Embedding .NET (as Godot's C# or s&box) would discard the compiler, UI and declarations work. What keeps the cost bounded: a small supported language, external type checking, a bounded `any` slice, and the engine's weight in Rust.

## 7. Start-up: what the build produces, and caching

- **Today:** `dist/game.irbundle` is JSON holding the bundled TypeScript source text (`source`). `iron-build build` resolves modules, strips imports, runs `tsc` and writes that text; it does **not** compile to machine code. At launch `iron_app` compiles the text in process with the Cranelift JIT. Every launch pays parse, inference/lowering, Cranelift code generation and JIT linking. An `--aot` path (`dist/game.o` plus `game_bindings.rs`) exists but needs a native link step.
- **Measured:** emoji starts in 2.2 s native against 1.2 s QuickJS (about 4,700 lines of its own source plus the preludes it uses). The split between parse, lowering, code generation and linking is **not yet measured**: the incremental-sync agent was asked to report it for emoji, fog-world, godrays-room and hello.
- **Hot reload (user):** the source is recompiled at run time when it differs; a checksum cache should make an unchanged game load without compiling. The dev path stays as it is.
- **Cache levels (rising effort):**
  1. Cache the intermediate form (parsed/typed/lowered IR): skips parsing, inference and lowering, still runs Cranelift at launch. Simple, safe.
  2. Cache relocatable object code (Cranelift can emit object files; `--aot` already writes `dist/game.o`): a small loader maps the object and patches host function addresses (they change per host build, so raw JIT output cannot be stored). Skips the whole compiler; needs a loader per platform object format.
  3. Link the object into the host at package time: fully AOT, needs a linker step in `iron-build package`; the right answer for shipped games (and iOS, which forbids writable-executable memory).
- **Cache key:** source hash + compiler version + host engine-interface version; on mismatch compile from the source text.
- **Decision:** measure first (**Decided**), then choose between levels 1 and 2.

## 8. Open items

- Start-up split measurement (queued with the incremental-sync agent).
- Write the scope policy and conformance table into `docs/iron-ts.md`.
- Embedded `tsconfig` default plus generated editor `tsconfig.json`.
- `tsgo` investigation (CLI, output format, licence).
- Optional `iron-build --check` fast mode with TypeScript-style diagnostics.
- Compare native and QuickJS script time like for like (script vs engine ops vs animation/sync).
