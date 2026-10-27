# platypus_ts: replacing `tsc`, tsconfig and language scope (research notes)

Status: discussion notes, 2026-10-23. Nothing here is built. Decisions are marked **Decided**; the rest are options with a recommendation. Related: `platypus-ts-startup-and-hot-reload.md`, `platypus-dynamic-values.md` (the `any`/`unknown` design), `platypus-generated-dts-plan.md`.

## 1. Type checking today

`tsc --strict` is the only real checker (`platypus-build` runs it when found, skipped as `Tsc::Missing` otherwise). platypus_ts infers types for code generation and trusts that `tsc` already accepted the programme (narrowing, union membership, and in the dynamic-value design `unknown`'s restrictions).

## 2. Replacing `tsc`

- **Why a full checker is hard:** structural assignability (recursive, with cycles), generic inference with contextual typing and overloads, type-level programming (conditional, mapped, template literal types, `infer`, `keyof`) which the standard library itself uses, flow-sensitive narrowing, declaration merging, variance. The long tail of edge cases is the cost.
- **Copying the native TypeScript port (`typescript-go`, `tsgo`):** TypeScript is Apache-2.0 (verify the port's licence and NOTICE file before copying anything). It is Go, so Platypus would be a rewrite, not a copy, wired to `oxc`'s AST instead of its own parser/binder, needing TypeScript's `lib.*.d.ts` files, and it would have to track TypeScript's behaviour changes. Not recommended. No finished Rust checker is known (`stc` was abandoned; unverified recently).
- **Options, cheapest first:**
  1. Use `tsgo` as the optional external checker (no code copied; needs a short investigation of its output format and CLI). Recommended.
  2. Give the compiler its own TypeScript-style diagnostics for the subset it lowers (`file(line,col): error`), as a `platypus-build --check` fast mode. A "naive" checker for Platypus's subset is feasible because lowering already has `coerce`, `unify` and narrowing; unsupported constructs stay an "unsupported TypeScript" error. Risks: more permissive than `tsc` lets programmes through that the compiler then trusts (miscompiles); stricter rejects valid ones; message wording differs; editors use `tsserver` regardless. If it agrees with `tsc` on every example and the test corpus, `tsc` can become optional for good.
  3. Port the checker (months). Only if a Node-free, `tsc`-free toolchain becomes a goal.
- **Reasoning:** type-level features only matter to a checker. While `tsc`/`tsgo` checks the programme Platypus gets them free, and the compiler needs only the resolved types. Owning a checker is the one move that imports the metaprogramming complexity.

## 3. tsconfig

Each example has a hand-written `tsconfig.json` (`extends ../../tsconfig.base.json`, `paths` for `src/modules`, `include: ["src"]`). The build already writes its own `dist/.tsc/tsconfig.json` and the generated `types/platypus.d.ts`. The example file matters only to editors and ties examples to a repo-relative base file.

- **Proposal:** the build uses an embedded default (strict mode, `paths` for `src/modules`, `include` from `project.json`). It writes a git-ignored `tsconfig.json` next to the generated `types/platypus.d.ts` for editor support; `platypus-build init` creates it for new games; a game may ship its own to override. Examples then need only `project.json` and `src/`.
- `tsconfig.base.json` pins `target: ES2022`, `lib: ["ES2022"]`, `types: []`: that is already the scope statement.

## 4. Scope (the complexity budget)

User concern: not adding language complexity or a meta-programming system that games (mostly components) do not need. **Proposed policy** for a "Scope" section in `docs/platypus-ts.md` (not yet written):

1. **Core:** classes and interfaces, components, enums, unions and literal types, generics (monomorphised), closures, async/promises, arrays, maps, sets, destructuring, ESM modules.
2. **Added on demand only:** a feature is added when a shipped example, or a package someone really copies in, needs it, with a bun-compared test.
3. **Out of scope:** our own type-level evaluator or checker; decorators beyond `register`/`property`; namespaces and declaration merging; Proxy, prototype manipulation, `eval`.
4. Unsupported features give a clear error (already the case).

**ES2022 conformance table:** one row per ES2022 built-in or syntax (`Array.prototype.flat`, `Object.entries`, optional chaining, private fields...), status done/partial/out of scope, a bun-compared test per row, and a test that fails if a row is marked done without a test. The tsconfig target is the scope statement. Do not move files into an `es2022/` directory: `lower/` is already split by feature and a rename would cause merge conflicts for no gain.

**Dynamic `any`:** decided to build (see `platypus-dynamic-values.md`), but under this policy only the first phases (`catch (e: unknown)`, untyped `JSON.parse`, `Record<string, any>`, property access) are planned unless a need appears.

## 5. Shrinking the language for speed (Lua-style)

Considered and not recommended as a performance measure:

- Unused features cost nothing at run time: unreferenced top-level code is not compiled, dynamic paths are paid only by values that use them, and the large preludes are already loaded on demand (`wants_*` checks in `lower/prelude.rs`; `Temporal` when the source rewrites to it, `lower/mod.rs`). The check works on source text, not reachability.
- Script time is already 0.02 to 0.3 ms per frame in most examples; the slow cases came from engine sync, not the language.
- Speed still on the table is codegen (unions are heap structs that allocate when wrapped, closures, exceptions, GC roots), not breadth.
- The real argument for a smaller language is maintenance and risk, which the scope policy addresses.

## 6. Comparison with C# / Unity (IL2CPP)

C# gives a mature typed language, compiler, runtime and GC; Unity only adds an engine. Platypus owns a compiler, GC, standard library and declarations generator. IL2CPP also restricts the language (reflection, runtime code generation, some generics), the same trade as Platypus's defined subset. TypeScript's reasons remain: React UI, a language web developers know, small builds without a .NET runtime, fast hot reload, one language for editor and game. Embedding .NET (as Godot's C# or s&box) would discard the compiler, UI and declarations work. What keeps the cost bounded: a small supported language, external type checking, a bounded `any` slice, and the engine's weight in Rust.

## 7. Open items

- `tsgo` investigation (CLI, output format, licence).
- Embedded `tsconfig` default plus generated editor `tsconfig.json`.
- Write the scope policy and conformance table into `docs/platypus-ts.md`.
- Optional `platypus-build --check` fast mode with TypeScript-style diagnostics.
