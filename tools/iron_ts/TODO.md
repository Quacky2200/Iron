# iron_ts: working file

Tracks the ahead-of-time TypeScript compiler (`tools/iron_ts`, `tools/iron_ts_rt`,
`tools/iron_ts_tests`). The repo-wide plan is `TODO.md` M9.b2; **this file is where progress is
kept up to date** and where a fresh session (or a context after compaction) starts. Update the
"Now" and "Log" sections at the end of every step. Design and measurements:
`docs/research/iron-ts-spike.md`. Test IDs: `TESTS.md` section "M9.b" (`M9-TS-*`, next free: **562**).

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
| `iron_ts/src/lower/{strings,object,maps,json,regexp,date,intl,prelude}.rs`, `throws.rs` | strings and numbers; `Object.*`/spread; `Map`/`Set`; typed JSON; `RegExp` and the regex string methods; `Date`; `Intl`; TS prelude classes (`Error`s, `RegExp`, `Date`, `Intl_*`); which functions can throw |
| `iron_ts_rt/src/{string,string_more,map,json,regexp,date,intl,intl_number}.rs` | runtime: WTF-8 strings, collections, JSON, `regress`, `jiff`, ICU4X |
| `iron_ts/src/lower/scan.rs`, `closure.rs` | which variables closures capture; lifting closures to functions, function values |
| `iron_ts/src/ir.rs` | the IR and its dump |
| `iron_ts/src/frames.rs` | shadow-stack frame plan; **the spill rule is the GC's correctness argument** |
| `iron_ts/src/codegen.rs` | IR to Cranelift object (frames, spills, `eval_args`, registration in `ts_init`) |
| `iron_ts/src/layout.rs` | generated Rust bindings (`HDR_*`, `TAG_*`, structs, `extern "C"`) |
| `iron_ts_rt/src/lib.rs` | GC heap, roots, arrays/strings/closures/cells, libm calls |
| `iron_ts_tests/` | `build.rs` compiles `fixtures/sim.ts` plus `bench/{sine,gap,closures,globals,loops,switches,destructure,params,mathlib,arraylib,classes,inherit,props,engine}.ts` (prefix and struct base each); `tests/{sim_port,gc,closures,globals,loops,switches,destructure,params,mathlib,arraylib,classes,inherit,props,engine}.rs`; `src/bin/{bench,sine_bench}.rs` |

Adding a test module: a `bench/<name>.ts`, an entry in `build.rs`'s list `(name, prefix, struct_base)`
(bases are 1000 apart: sim 0, sine 1000, gap 2000, closures 3000, globals 4000, loops 5000, switches 6000, destructure 7000, params 8000, mathlib 9000, arraylib 10000, classes 11000, inherit 12000, engine 14000, scene 15000, strings 16000, errors 17000, stdlib 18000, collections 19000, json 20000, regexp 21000, regexp2 22000, date 23000, intl 24000, numfmt 25000, next 26000), a `pub mod` in
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

**State (2026-10-19):** the standard library list is delivered (strings, collections, JSON, `RegExp`, `Date`, `Intl`, generators, `Promise`/`async`/timers, typed arrays, `Temporal`, ...; `M9-TS-1..555`, detail in the Log and `docs/iron-ts.md`). Compiled components for the Rust host are built (`M9-TS-556..561`, `NativeHost`). Next free test ID **562**.
**Next (user's priority):** (1) a design note for a Preact-style component/effect UI in Rust (STOP item "UI without React"; the existing React layer only produces `__iron_commit` ops and consumes `__iron_ui_event`, so the new layer must emit the same ops into `iron_ui::UiTree`, no second tree); (2) toward iron_api testing of iron_ts as the QuickJS replacement: `@property` save/load through `PROPS_<Class>`, `getComponent`/`addComponent`, QuickJS bindings for values and strings, annotating `iron_physics`/`iron_audio`, wiring `enter` + `NativeHost` into `iron_app` (another session's area: hand-over contract in boundary note section 21).
**Open stdlib items (niche):** `Intl.*.formatRange`, `FinalizationRegistry`, `globalThis`, `Error.stack`, `Duration.total` with `relativeTo`, `for await` / async generators, `SharedArrayBuffer`/Atomics, Proxy (not until needed), optional `icu-full`.

Next, in order:

1. **Strings** (design `docs/research/iron-ts-strings.md`, decided: stored as WTF-8, behaviour exactly JavaScript's; lone surrogates become U+FFFD at the
   engine boundary and `localeCompare`/`Intl`/`RegExp` are compile errors until built, as the note proposes, not yet confirmed by the user).
   - [x] Slice 1a, runtime core (`iron_ts_rt/src/string.rs`): inline-bytes `Str` with ASCII / well-formed flags and UTF-16 length, cursor, `charCodeAt`,
     `codePointAt`, `charAt`, `slice`, `substring`, `indexOf`/`lastIndexOf`/`includes`/`startsWith`/`endsWith`, `<` ordering, concat, `fromCharCode`/`fromCodePoint`;
     differential tests against bun (`tests/gen_strings.ts` -> `strings_expected.json`, `M9-TS-162..172`).
   - [x] Slice 1b, rest of the runtime (`iron_ts_rt/src/string_more.rs`): `trim*`, `split`/`join`, n-ary concat for template literals, `repeat`, `padStart/End`, `at`,
     `toUpperCase/LowerCase`, `String(number)`, `toFixed`, `toString(radix)` (integers), `parseInt`/`parseFloat`/`Number(s)`; differential tests `M9-TS-173..177`.
     Not done: `toString(radix)` of fractions, `toPrecision`/`toExponential`, `normalize`, `localeCompare` (compile errors in slice 2).
   - [x] Slice 2, compiler (`lower/strings.rs`, `codegen.rs`): `string` locals/fields/arrays/params, literals as static objects (writable data, no heap id), `+`/`+=`,
     template literals, `===`/`<` (code unit order), `s.length`, the methods of slice 1, `String(x)`/`Number(s)`/`parseInt`/`parseFloat`, `String.fromCharCode/fromCodePoint`,
     `join`, truthiness, `switch`; a literal is a union tag or a `string` by context (`let` and array literals widen); `M9-TS-178..182`.
     Known gaps: `typeof`, `codePointAt` past the end gives NaN (not the null NaN), `s?.length`, string results of object-literal fields without a declared type,
     `join` of non-string arrays, `replace`/`match`/`localeCompare`/`normalize` (compile errors), template/`+` chains allocate one string per `+`.
   - [x] Slice 3, boundary: `string` in `#[iron_api]` (`&str` parameter read from the script string object, no copy unless it has a lone surrogate, which becomes U+FFFD;
     `String` result through a per-thread buffer the compiled call copies into a new script string), manifest/d.ts `string`, `GameObject.name` / `find` in `iron_scene`;
     `M9-API-21..22`, `M9-TS-183..186`. Strings are not available in the QuickJS bindings (a compile error in the macro). `string | null` parameters are not supported.
     **The strings item is done.** Remaining string work lives in the standard library item below.
2. **Annotate `iron_physics`, `iron_audio`, `iron_script`**; wire `iron_scene::api::enter` into `iron_app` (needs its owner:
   another session works there); retire the ops the new API replaces.
3. Extending an engine class (`Component`) and the compiled-component vertical slice; `typeof`, class expressions.
4. **A bare-minimum JavaScript standard library, native** (`docs/research/iron-ts-stdlib.md`; **decisions approved 2026-10-18**: exceptions as checked returns in
   may-throw functions only, `regress` for RegExp, ICU4X with a per-game locale list for Intl, typed JSON plus a dynamic `JsonValue`, tiers 1-3 in order;
   rule: JavaScript's exact results, what is not built is a compile error). Progress, in order:
   - Tier 1: [x] A. Error classes from a TS prelude (`lower/prelude.rs`). [x] B. `throw` / `try` / `catch` (checked returns, pending exception in the runtime, `throws.rs`). [x] C. `finally` with
     `return`/`break`/`continue` through it (a completion record per `try`). [x] D. runtime errors (`RangeError` from `repeat`/`toFixed`/`toString(radix)`/`fromCodePoint`/padding) catchable (`__rtThrow`); `M9-TS-187..197`.
     Not done: `error.stack`/`cause`, `Error(...)` without `new`, throwing non-Error values, catch variable typed `unknown`. [x] E. `typeof` (a nullable value that is null is `"undefined"`; the compiler does not tell `null` from `undefined`). [x] F. `Object.keys/values/entries (for-of)/assign/freeze/hasOwn`, `in`,
     object spread (new shapes, source order) and rest, array rest (`lower/object.rs`; `freeze` does not reject later writes). [x] G. `Array`: `toSorted toReversed findLast findLastIndex
     reduceRight flat flatMap`, default `sort()`, `entries/keys/values` in for-of; `Number` statics and constants, `isNaN`/`isFinite`; `Math` sinh..log1p. [x] H. `console.log/info/debug/warn/error`
     of strings, numbers, booleans, null (to a host hook, stderr by default). **Tier 1 is done** (`M9-TS-199..205`). Not done: `with`, `toSpliced`, `copyWithin`, `Array.from` with `{length}`, `Object.fromEntries`, `console` of objects.
   - Tier 2: [x] `Map`/`Set`/`WeakMap`/`WeakSet` (`iron_ts_rt/src/map.rs`, `lower/maps.rs`; weak = strong; `M9-TS-206..214`; not done: `new Map(map)`, `Map` of booleans/literals `get`, map spread, `groupBy`). [x] `JSON.stringify` / `JSON.parse` typed from the program's types (descriptor blobs, `iron_ts_rt/src/json.rs`, `lower/json.rs`; bun's `SyntaxError` texts; shape mismatch is a `TypeError`; `M9-TS-215..221`). Not done: a dynamic `JsonValue` for untyped parse, replacer/reviver, `toJSON`, `Date`.
   - **Next: Tier 3** (adds crates, needs network for `cargo`): [~] `RegExp`: R1 done (literals, `new RegExp`, `test`/`exec`, `lastIndex`, flags, named groups, match objects as arrays; `M9-TS-222..231`); R2 done (`String.match/matchAll/search/replace/replaceAll/split` with regexes, `$` patterns, replacement functions; `M9-TS-232..239`); the `RegExp` item is complete except `RegExp.escape` use from TS and `Symbol.replace`-style hooks (not planned). Limits: a replacement function with group parameters needs a regex literal or a function-local `const` declared from one. (regress; regex literals validated at compile time; `exec/test/match/matchAll/replace/replaceAll/split/search`, named groups, `lastIndex`, flags `gimsuyd`; UTF-16 indices: match ASCII strings by byte, others as a UTF-16 buffer). [x] `Date` (jiff; `M9-TS-240..253`: constructor forms, `Date.now/UTC/parse`, every getter and setter in local time and UTC, `toISOString/toJSON/toUTCString/toDateString`, ISO and legacy parsing, arithmetic and comparison, the OS zone or `ts_rt_date_set_zone`; bun-compared in five zones). `toString`/`toTimeString`/`String(date)`/`date + x` done with ICU4X zone names (`M9-TS-276`). Left for the Intl step (the user asked for it too): `toLocale*String`, `Intl.DateTimeFormat` and the other Intl date formatting; `Date` inside typed JSON. [ ] `Intl`, `localeCompare`, `normalize`, `toLocale*` (ICU4X).
   - Intl (the user asked for all of it, and for more locales than `en`): I2 done (`Intl.NumberFormat` and `Number.toLocaleString`: decimal, percent, currency, compact, scientific/engineering, digit options, grouping, sign display, rounding modes; `M9-TS-264..275`; not yet: `style: unit`, compact currency, `formatToParts`, `formatRange`, `resolvedOptions`, BigInt, `useGrouping: always` in locales that group from five digits). I1 done (`normalize`, `localeCompare`, `Intl.Collator` incl. `sort(c.compare)`, `toLocaleUpperCase/LowerCase`; `M9-TS-254..263`; options must be object literals; `ts_rt_intl_set_locales` takes the game's list, wiring it from `package.json` waits for `iron_app`); next: I3 `Intl.DateTimeFormat`, `Date.toString/toTimeString` (zone names), `Date.toLocale*String`; I4 done for `PluralRules`, `ListFormat`, `RelativeTimeFormat` (`M9-TS-296..312`, bun-compared in 17 to 20 locales); still open: `Locale`, `DisplayNames`, `Segmenter` (needs iterators), `supportedLocalesOf`, `resolvedOptions`, `formatToParts`. ICU4X (`icu` 2.x, compiled data for every locale; the per-game list in `package.json` picks which are accepted, `en` by default), compared against bun.
   - Done from that list (2026-10-19): `encodeURI`/`decodeURI`/`encodeURIComponent`/`decodeURIComponent`/`escape`/`unescape` (`M9-TS-287..291`), `Error` `cause` and `AggregateError` (`M9-TS-292..295`; `stack` and `captureStackTrace` are open). `BigInt` done (`M9-TS-360..369`: `Ty::BigInt`, `lower/bigint.rs`, `iron_ts_rt/src/bigint.rs` on `num-bigint`; not: `BigInt64Array`, `toLocaleString`, Map keys by value, JSON). Found on the way and fixed (`M9-TS-370..375`): `a && b` / `a || b` give the operand when both sides have one type (conditions stay boolean through `lower_cond`; mixed sides such as `obj && obj.x > 0` as a value are still boolean); a nullable string tests for empty. Still to do: (arbitrary precision, `bigint` type, literals `10n`, mixed-type errors, `BigInt.asIntN`; a runtime bignum), `encodeURI`/`decodeURI`/`encodeURIComponent`/`decodeURIComponent` (`URIError` "URI malformed"; small, do first), the missing errors (`AggregateError`, `cause`, `stack`, `Error.captureStackTrace`; the other classes exist; there is no `ParseError` in JavaScript, `SyntaxError` is it), generators and iterators done (answers recorded 2026-10-19; `M9-TS-376..393`): well-known symbol member names and the iterator protocol for classes (`lower/iter.rs`), stackful coroutines on `corosensei` (`iron_ts_rt/src/coro.rs`), generators by source rewriting (`lower/generators.rs`: `function*`, methods, `*[Symbol.iterator]()`, `yield`, `yield*` as a statement, `next(value)`, `return()`, `try/finally`, exceptions). Open: `Symbol()` values, `generator.throw()`, a returned value (`return v` in a generator is dropped), `yield*` as a value, `finally` blocks on `return()`, untyped generators, `async`/`Promise`/timers on the same coroutines (next), `Proxy` (not until needed).
   - Bytes across the engine boundary (the user asked: e.g. Rust handing an image to script, or script handing bytes to Rust): `Uint8Array` is `Ty::Bytes` already; add a manifest type `bytes` (`&[u8]` in as pointer + length, `Vec<u8>` out into a new `Uint8Array`, like strings), then `btoa`/`atob`, `TextEncoder`/`TextDecoder`, and `data:` URIs through `iron_core::decode_data_url`. Images are better passed as texture/asset handles than as bytes; bytes are for files, network and cooked data.
   - **Intl date formatting decision (2026-10-19, `docs/research/iron-ts-intl-datetime.md`):** ICU4X now with lightweight main-market data; `rust_icu` as an optional `icu-full` build.
     - [~] I3a (`icu-lite`): done (`M9-TS-277..286`): `dateStyle`/`timeStyle`, `toLocaleDateString/TimeString/String`, `new Intl.DateTimeFormat(locale, options).format`, component options (year/month/day/weekday/hour/minute/second, `hour12`/`hourCycle`, `fractionalSecondDigits`, `timeZone`, `timeZoneName`). The numeric padding of month/day/hour comes from a table generated from the reference engine (`bench/gen_intl_pad_table.ts` -> `iron_ts_rt/src/intl_pad.rs`, 31 locales). Open: `formatToParts`, `formatRange`, `resolvedOptions`, `era`, `dayPeriod`, other calendars and numbering systems, the columns listed in `docs/differences.md` ("Intl date formatting").
     - [ ] Lightweight data: generate a baked ICU4X data crate for the main-market locale list with `icu4x-datagen` and use it instead of `compiled_data` (today every locale is linked: large binary). Keep the per-game locale list from `package.json` as the accept list.
     - [ ] (optional, LATER; the user said to ignore it for now, 2026-10-19) `icu-full` cargo feature of `iron_ts_rt` (off by default): backend `rust_icu`/ICU4C for exact `availableFormats` (`udatpg_getBestPattern` via `rust_icu_sys`), all locales, calendars, `formatToParts`, `formatRange`. First a spike: build `rust_icu_sys` + `rust_icu_udat` on this Windows machine against a source-built ICU 77 (static, versioned symbols, `icu-config`, bindgen/LLVM); see the note for the prerequisites. Decide after the spike whether I1/I2 (collator, NumberFormat) also get an `icu-full` path.
   - Done (2026-10-19): `Symbol()` values (a prelude class `Symbol_`: identity, `description`, `toString`, `Symbol.for`/`keyFor`, `typeof`; `M9-TS-394..398`), `generator.throw()`, `return(v)` running `finally` blocks (a marker error that user `catch` blocks rethrow), `return value` (`Generator<T, T>`) (`M9-TS-399..407`). Not done: symbols as property keys or Map keys, other well-known symbols as values, `yield*` as a value, untyped generators.
  - Done (2026-10-19): `Promise<T>`, `async`/`await`, `new Promise<T>(...)`, `.then/.catch/.finally`, `Promise.resolve/reject/all/race/any/allSettled`, `then(f, g)`, `queueMicrotask`, `setTimeout/setInterval/clear*` on stackful coroutines with the microtask and timer queues in the runtime (`event.rs`), bun-compared tick order (`M9-TS-408..430`). Host contract: call `ts_rt_microtasks_run()` after every call into script and `ts_rt_timers_advance(now_ms)` once per frame (`ts_rt_run_loop()` for tests). Not done: `for await`/async iterators, async generators, top-level `await`, awaiting a thenable that is not a Promise, a `Promise<A | B>`, `Promise.reject` typed from context, `setTimeout` extra arguments, unhandled-rejection reports, wiring the host loop in `iron_app` (off limits).
  - Done (2026-10-19): `ArrayBuffer`, `DataView`, `new Float32Array(buffer, offset, length)`, `subarray`, `.buffer`, `.byteOffset` as views sharing memory (`HDR_VIEW`, `M9-TS-431..441`). `.buffer` of an owning array makes a new `ArrayBuffer` object each time (same memory, not `===`). Not done: `SharedArrayBuffer`, resizable buffers, `BigInt64Array`, `DataView` BigInt accessors, `Atomics`, transfer, passing buffers across the engine boundary (needs `iron_app`).
  - Done (2026-10-19): bytes across the engine boundary in the runtime (`ts_rt_typed_from_host_gc`, `ts_rt_typed_data`, `ts_rt_typed_byte_len`; the generated Rust bindings for exported functions with typed-array parameters are not done), `structuredClone` (`clone.rs`, no Map/Set/cycles, class instances keep their class), `WeakRef<T>` for program classes (strong), `WeakMap`/`WeakSet` (strong; already there) (`M9-TS-442..455`). Not done: `FinalizationRegistry`, `globalThis`, real weak references.
  - Done (2026-10-19): `Intl.Segmenter` (grapheme, word, sentence; `intl_segment.rs`), `Intl.Locale` (parts, extension keywords, options, maximize/minimize; `intl_locale.rs`), `Intl.DisplayNames` for region, script and language (`intl_names.rs`) (`M9-TS-456..479`). Differences: `"und"` maximizes to `en-Latn-US` by a special case, CLDR names can differ from bun's (`zh-Hans` is "Simplified Mandarin Chinese"), `isWordLike` is false for non-word granularities (undefined in JS), DisplayNames types currency/calendar/dateTimeField are not built, `fallback: "none"` is not built. Not done: `formatToParts`, `formatRange`, `resolvedOptions`, `Intl.supportedValuesOf`, `Intl.getCanonicalLocales`.
  - Done (2026-10-19): `Temporal` on the ISO calendar: `Duration`, `PlainDate`, `PlainTime`, `PlainDateTime`, `Instant`, `ZonedDateTime` (IANA zones and fixed offsets through jiff: `tz.rs`), `Now` (`lower/temporal.ts` is the prelude, `lower/temporal.rs` routes `Temporal.X.from(v)` and string arguments to the parse functions; `M9-TS-480..515`). Not done: `PlainYearMonth`, `PlainMonthDay`, other calendars, `round()`, `toLocaleString`, `Duration.total` with `relativeTo`, `largestUnit`/`smallestUnit` rounding options beyond `largestUnit`, `overflow: "reject"` options (constrain is the only mode), `ZonedDateTime.until` with calendar units across a clock change, `disambiguation`/`offset` options, time zone transitions API, `Temporal.Calendar`/`TimeZone` objects.
  - Done (2026-10-20): Temporal `PlainYearMonth`, `PlainMonthDay`, `round` (PlainTime, PlainDateTime, Instant, ZonedDateTime, Duration: `smallestUnit`, `roundingMode`, `roundingIncrement`, `largestUnit` for durations), `Intl.getCanonicalLocales` (`M9-TS-516..536`). Still open: rounding in `until`/`since`, `Duration.total` with `relativeTo`, `FinalizationRegistry` (no type for the held value), `globalThis`, `formatToParts`/`formatRange`/`resolvedOptions`.
  - Done (2026-10-20): `Intl.NumberFormat.formatToParts` (the formatted text is split by the locale's digits, separators, signs, percent, currency and compact suffix: `intl_number.rs`) and `Intl.DateTimeFormat.formatToParts` (ICU4X parts: `intl_parts.rs`) (`M9-TS-537..549`). Still open: `formatRange`, `resolvedOptions`, `fractionalSecond` part.
  - Done (2026-10-20): `resolvedOptions()` of `Intl.DateTimeFormat` (locale, calendar, numberingSystem, timeZone) and `Intl.NumberFormat` (locale, numberingSystem, style, currency, notation, signDisplay, integer/fraction/significant digits); the other fields (`year`, `hour12`, `roundingMode`, `useGrouping`...) are not in the result (`M9-TS-550..555`).
  - Tier 4 (the user listed these; `NaN`, `Infinity`, `isNaN`, `isFinite` already work): iterators / generators and `Symbol` (`Symbol.iterator`, `asyncIterator`, `toPrimitive`...), typed arrays: `Int8Array`..`Float64Array` and `Uint8ClampedArray` done (`M9-TS-313..359`: `Ty::Typed(kind)`, `lower/typed.rs`, `iron_ts_rt/src/typed.rs`); open: `ArrayBuffer`, `DataView`, views that share memory (`subarray`, `buffer`, `new X(buffer, offset, length)`), `BigInt64Array`, `Array.from(typed)`, `X.of`, `copyWithin`, a read past the end gives 0 instead of undefined, `Temporal` (jiff has the model), real `WeakRef` / `FinalizationRegistry` (WeakMap/WeakSet are strong now), `Promise`/`async`/timers, `structuredClone`, `globalThis`.

## Backlog

The full list with sub-tasks, sizes and STOP gates is `TODO.md` M9.b2. Open top-level items: type
sidecar, GC (verifier, cheaper frames, escape analysis, pauses), closures, classes, generics, structural typing, strings, language features, library
surface, JS-semantics conformance, `JsValue`, direct engine calls, hybrid release builds,
performance, cross targets and CI.

## Log

Newest first. One line per finished step: what, test IDs, numbers if measured.

- 2026-10-19 `Date.toString/toTimeString/String(date)/date + x` (`M9-TS-276`): ICU4X root zone names, whole-minute offsets, the name follows the daylight state for old dates. Design note for `Intl.DateTimeFormat` written, awaiting review.
- 2026-10-19 Intl I2 (`M9-TS-264..275`): `Intl.NumberFormat`, `Number.toLocaleString` (decimal, percent, currency, compact, scientific, digit options, grouping, sign display, rounding modes), 15 locales against bun.
- 2026-10-19 Intl I1 (`M9-TS-254..263`): `normalize`, `localeCompare`, `Intl.Collator` (also as `sort` comparator), `toLocaleUpperCase/LowerCase` on ICU4X.
- 2026-10-19 `Date` (`M9-TS-240..253`): spec calendar maths, `jiff` local time (OS zone or `ts_rt_date_set_zone`), constructors, statics, getters/setters, ISO and legacy parsing; five time zones against bun.
- 2026-10-19 `RegExp` R2 (`M9-TS-232..239`): `match/matchAll/search/replace/replaceAll/split` with regexes, `$` patterns, replacement functions. R1 (`M9-TS-222..231`): the `regress` engine, literals, `exec/test`, flags, named groups.
- 2026-10-19 JSON (`M9-TS-215..221`): typed `stringify`/`parse` through descriptor blobs; bun's `SyntaxError` texts (67 cases).
- 2026-10-19 `Map`/`Set`/`WeakMap`/`WeakSet` (`M9-TS-206..214`): insertion-ordered hash table, SameValueZero, exact live iteration.
- 2026-10-19 Standard library tier 1 E-H (`M9-TS-199..205`): `typeof`, `Number` statics, `Math` extras, `Array` gaps, `console`, `Object.*`, spread/rest.
- 2026-10-18 Exceptions, tier 1 A-D (`M9-TS-187..198`): `Error` classes from a prelude, `throw`/`try`/`catch`/`finally` as checked returns in may-throw functions only, runtime `RangeError`s catchable.
- 2026-10-18 Strings (`M9-TS-162..186`): WTF-8 storage with JavaScript's UTF-16 behaviour; runtime, compiler and the engine boundary (`&str`/`String`); design `docs/research/iron-ts-strings.md`.

- 2026-10-18 Array library (`M9-TS-92..110`): runtime routines for pop/shift/unshift/slice/splice/concat/reverse/fill/sort,
  inline loops for indexOf/findIndex/reduce/at, `length = n`, `Array.from/of/isArray`, `new Array(n)`. The sort
  roots its buffer; the first stress run found the merge buffer traced before it was zeroed (spike doc 15.2).
- 2026-10-18 Math library and operators (`M9-TS-85..91`): ceil/trunc/round/fround/clz32 inline, libm functions, JS `pow`,
  `random`, `**`, `~`, `??=`/`||=`/`&&=` (spike doc 15.1).
- 2026-10-18 Default, optional and rest parameters and spread (`M9-TS-77..84`): the caller fills in defaults (lowered in a
  scope of the earlier parameters) and builds the rest array; array/call spread copies (spike doc 14.2, 14.3).
- 2026-10-18 Destructuring (`M9-TS-71..76`): `bind_pattern` for declarations, parameters, `for...of`, callbacks;
  defaults via `Cond(IsNull)` (spike doc section 14.1). Rest elements and assignment patterns are open.
- 2026-10-18 `switch` and enums (`M9-TS-64..70`): clause index then ordered bodies (fall-through, default
  anywhere), enums folded to constants, bit-operation constant folding (spike doc section 13).
- 2026-10-18 `iron_scene` on `#[iron_api]` (`M9-API-12..15`, `M9-TS-151..155`): the `Scene` namespace, handle encoding that is never 0,
  `iron_api` `glam` feature; compiled TypeScript drives the real scene. Not wired into `iron_app`.
- 2026-10-18 Value classes across the boundary and the engine context (`M9-TS-147..150`, `M9-API-07..11`): `values(..)` in
  `#[iron_api]`, fields as scalars, results through a pointer (`CallExternInto`), `iron_api::context` (thread-local world).
- 2026-10-18 The compiler calls the engine API (`M9-TS-142..146`): `--manifest`, `CallExtern`, `Ty::Handle`, number conversions at the call;
  `tools/iron_demo_api` proves it end to end. Rule found: a live handle is never 0.
- 2026-10-18 `#[iron_api]` first slice in the engine workspace (`crates/iron_api`, `iron_api_macros`; `M9-API-01..06`): manifest,
  `.d.ts`, JSON, `extern "C"` shims, typed QuickJS bindings (150 ns a call against 245 ns for a simplified op).
- 2026-10-18 `@property` reflection tables (`M9-TS-139, 140`): compile-time metadata, a generated `PROPS_<Class>` constant per class.
- 2026-10-18 Methods as values (bound closures over the receiver) and arrow-function fields (`M9-TS-137, 138`).
- 2026-10-18 Inheritance (`M9-TS-120..136`): prefix layout, `$init` constructors with `super(...)`, direct calls by whole-program
  analysis and `CallVirtual` tables otherwise, `instanceof` tables, downcast by narrowing, abstract classes (classes note 14).
  Found: a `Seq` whose value was a void call never ran it.
- 2026-10-18 Plain classes (`M9-TS-111..119`): fields, constructors (parameter properties, initialiser order, early return),
  methods as `this`-first functions, getters/setters, statics as globals, `#private`, `this` in arrows (classes note 13).
- 2026-10-18 Engine-boundary answers recorded (section 13): #[iron_api] replaces __iron_op, whole engine ported to Rust.
- 2026-10-18 Engine-boundary design note written (`docs/research/iron-ts-engine-boundary.md`); awaiting review.
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
