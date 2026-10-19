# iron_ts: the ahead-of-time TypeScript compiler

Code: [tools/iron_ts](../tools/iron_ts/src) (compiler), [tools/iron_ts_rt](../tools/iron_ts_rt/src/lib.rs)
(runtime and GC), [tools/iron_ts_tests](../tools/iron_ts_tests) (fixtures, tests, benchmarks).
Progress and next steps: [tools/iron_ts/TODO.md](../tools/iron_ts/TODO.md). Why it exists, the
measurements and the design notes: [iron-ts-spike.md](research/iron-ts-spike.md),
[iron-ts-classes.md](research/iron-ts-classes.md), [iron-ts-engine-boundary.md](research/iron-ts-engine-boundary.md).
Design note for `Intl.DateTimeFormat` options (step I3a is built on it): [iron-ts-intl-datetime.md](research/iron-ts-intl-datetime.md).

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
                [--opt none|speed|speed_and_size] [--time N] [--manifest api.json]...
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
| `--manifest api.json` | Repeatable. An `#[iron_api]` namespace the source may call (the JSON from `Namespace::to_json`, see [iron-api.md](iron-api.md)); the library entry point is `compile_with_manifests(src, opt, prefix, struct_base, &manifests)` |

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
- **Strings.** `string` is a pointer to the runtime's `Str` (`Ty::Str`). A literal is a `Ty::Lit` tag
  in a string-literal-union context and a `Ty::Str` static object when the context is `string`
  (`ExprKind::Str(i)`: bytes in the object file's writable data, header `HDR_STR` with no heap id, so
  the collector ignores it; `Module.strs`). A tag used as a string converts (`ExprKind::TagStr`: a table
  of the tags' texts, or a static string when the tag is constant). `let` initialisers and array
  literals widen a literal to `string`; `const` keeps the literal type. Runtime calls are
  `RtFn::Str(symbol)`: a generic call to a `ts_rt_str_*` function; names ending `_gc` may collect and
  count as allocating in the root analysis (`frames.rs`). See "Strings in the compiler" below.
- **Objects** are `[header][8-byte slot per field]`; every field is one 8-byte slot, so offsets are
  trivial and the generated Rust structs match exactly.
- **Codegen** targets the host triple (no PIC, no unwind info). Exported functions are `ts_<prefix><name>`
  with the C calling convention; `ts_<prefix>init` runs the top-level initialisers and registers
  the struct pointer masks and global roots with the runtime. Runtime routines are imports
  (`ts_rt_*`).
- **Property tables.** For each class with `@property` fields the bindings also hold
  `pub const PROPS_<Class>: &[(&str, usize, &str)]`: name, byte offset in the object, type name
  (`number`, `boolean`, `string`, `object:<Class>`, ..., `?` appended when nullable), base class
  first. A host reads and writes properties at those offsets with no script running (a scene loader
  can set a field before any code does). The IR carries them as `Module::props`.
- **Engine calls.** `Module::externs` holds one `ExternDecl` per called symbol: a flattened scalar
  parameter list (a value class is already split into its fields) and, for a value result, an
  optional out pointer `(scalar type, count)` that codegen appends as the last parameter.
- **Value-class members.** The manifest parser adds a class named like a value class (`Vector3`) to
  `ClassFns` of that name without registering a handle type. `value_member` in
  [lower/expr.rs](../tools/iron_ts/src/lower/expr.rs) finds the method or getter of the struct behind
  a value class; `lower_method` (lower/arrays.rs) and `member_of` call it as a direct extern call with
  the receiver first (`a.add(b)`, `v.magnitude`), before any user-defined target is tried. Fields
  still win over getters. Not supported: static functions on value classes, operators, compound
  assignment on value properties.
- **String arguments and results.** A `string` in the manifest is `ExtTy::Str`. As an argument the
  call passes the script string pointer, and codegen adds a trailing stack slot for the length
  when the extern has a string result. The callee borrows the bytes (`iron_api::script_str`), so
  the string stays rooted for the call; a lone surrogate reaches the engine as U+FFFD. A string result
  comes back as bytes plus length from a per-thread buffer and is copied at once into a new script
  string with `ts_rt_str_new_gc`. That call can collect, so the root analysis (`frames.rs`) counts
  it as allocating. See [iron-api.md](iron-api.md#strings). Not supported: `string | null`
  parameters.
- **Linking** is the host's job: the test crate's `build.rs` writes the `.obj`, passes it with
  `cargo:rustc-link-arg`, and includes the bindings.

## Source files

| File | What |
|---|---|
| [manifest.rs](../tools/iron_ts/src/manifest.rs) | Reads the `iron_api` JSON manifests into a table of callable functions (`Manifest`, `ExternFn`), the handle type names, the value classes (`values`) and the handle classes (`classes`: `ClassFns` with `statics`, `methods`, `getters`, `setters`) |
| [lib.rs](../tools/iron_ts/src/lib.rs) | `compile_source` / `compile_with_prefix` / `compile_with` / `compile_with_manifests`, `Output`; unit tests for lowering errors (`M9-TS-14..16`, `55`, `56`, `70`, `84`, `110`, `119`) |
| [ir.rs](../tools/iron_ts/src/ir.rs) | The IR: `Ty`, `Module` (with `vtables` and `itables`), `Func`, `Stmt`, `Expr`/`ExprKind` (including `CallVirtual` and `InstanceOf`), `RtFn` (runtime calls), `field_offset`, `dump` |
| [frames.rs](../tools/iron_ts/src/frames.rs) | Shadow-stack frame plan and the spill rule (see GC roots below) |
| [codegen.rs](../tools/iron_ts/src/codegen.rs) | IR to Cranelift object: frames and spills, `eval_args`, `ts_init`, runtime calls, the method tables and `instanceof` byte tables (local data), indirect calls |
| [layout.rs](../tools/iron_ts/src/layout.rs) | Rust bindings: `HDR_*` header constants, `#[repr(C)]` structs, `TAG_*`, `NULL_NUM_BITS`, `extern "C"` declarations |
| [bin/iron_ts.rs](../tools/iron_ts/src/bin/iron_ts.rs) | The CLI above |
| [lower/mod.rs](../tools/iron_ts/src/lower/mod.rs) | `lower_source`: collecting declarations, `lower_fn` (lowered on demand), bindings (direct or boxed), module-level variables |
| [lower/types.rs](../tools/iron_ts/src/lower/types.rs) | Annotation to IR type, struct layouts (a union of objects becomes one struct), `coerce`, `to_bool` |
| [lower/expr.rs](../tools/iron_ts/src/lower/expr.rs) | Expressions, assignment targets (places), calls, `new`, optional chains, `Math`, constant folding |
| [lower/stmt.rs](../tools/iron_ts/src/lower/stmt.rs) | Statements: variables, destructuring, loops, labels, `switch` |
| [lower/arrays.rs](../tools/iron_ts/src/lower/arrays.rs) | Array methods; callbacks are inlined into loops, `Iter` for `for...of` and spread |
| [lower/scan.rs](../tools/iron_ts/src/lower/scan.rs) | Which names do closures capture (by name, so it only over-approximates) |
| [lower/closure.rs](../tools/iron_ts/src/lower/closure.rs) | Closures lifted to `lambda(env, params...)`, function values, cells |
| [lower/strings.rs](../tools/iron_ts/src/lower/strings.rs) | Strings: `+`/`+=`, template literals, comparisons, `length`, string and number methods, `String`/`Number`/`parseInt`/`parseFloat`, `String.fromCharCode`/`fromCodePoint`, truthiness |
| [lower/maps.rs](../tools/iron_ts/src/lower/maps.rs) | `Map`, `Set`, `WeakMap`, `WeakSet`: construction, methods, live `for...of`, snapshots for spread and `Array.from` |
| [lower/json.rs](../tools/iron_ts/src/lower/json.rs) | `JSON.stringify` / `JSON.parse`: the type descriptor blobs (format documented at the top of the file) |
| [lower/clone.rs](../tools/iron_ts/src/lower/clone.rs) | `structuredClone`: one lifted clone function per type |
| [lower/regexp.rs](../tools/iron_ts/src/lower/regexp.rs) | `RegExp`: literals, `new RegExp`, `test`/`exec`, the match object, and the String methods that take a regex (`match`, `matchAll`, `search`, `replace`, `replaceAll`, `split`) |
| [lower/date.rs](../tools/iron_ts/src/lower/date.rs) | `Date`: constructors, `Date.now`/`UTC`/`parse`, getters and setters (local and UTC), the string conversions, and arithmetic and comparison on the time value |
| [lower/intl.rs](../tools/iron_ts/src/lower/intl.rs) | `Intl`, steps I1 to I4: `localeCompare`, `normalize`, `toLocaleUpperCase`/`toLocaleLowerCase`, `new Intl.Collator`, `NumberFormat`, `DateTimeFormat`, `PluralRules`, `ListFormat` and `RelativeTimeFormat` (`lower_new_intl`); locale and options arguments are read at compile time (`options_spec`) |
| [iron_ts_rt/src/intl_number.rs](../tools/iron_ts_rt/src/intl_number.rs) | The `NumberFormat` runtime: option parsing, rounding and padding on a `fixed_decimal` `Decimal`, ICU4X formatting, scientific/engineering by hand |
| [iron_ts_rt/src/intl_date.rs](../tools/iron_ts_rt/src/intl_date.rs) | The `DateTimeFormat` runtime and `Date.prototype.toLocale*String`: ECMA-402 option defaults, mapping options to an ICU4X field set, pattern width rewriting (`adjust_pattern`), time zones through jiff |
| [iron_ts_rt/src/intl_misc.rs](../tools/iron_ts_rt/src/intl_misc.rs) | The `PluralRules`, `ListFormat` and `RelativeTimeFormat` runtime (options parsing, tables of formatters by id, bun's `RangeError` texts) |
| [iron_ts_rt/src/intl_segment.rs](../tools/iron_ts_rt/src/intl_segment.rs) | The `Intl.Segmenter` runtime: grapheme, word and sentence boundaries from ICU4X's segmenter, returned to the prelude as arrays of pieces |
| [iron_ts_rt/src/intl_locale.rs](../tools/iron_ts_rt/src/intl_locale.rs) | The `Intl.Locale` runtime: canonical tags, subtags and unicode extension keys, `maximize` / `minimize` |
| [iron_ts_rt/src/intl_names.rs](../tools/iron_ts_rt/src/intl_names.rs) | The `Intl.DisplayNames` runtime: region, script and language names from CLDR data |
| [lower/temporal.rs](../tools/iron_ts/src/lower/temporal.rs) | `Temporal`: the `Temporal.X` to `Temporal_X` text rewrite, `Temporal.X.from(v)` routing, string arguments where a Temporal object is expected |
| [lower/temporal.ts](../tools/iron_ts/src/lower/temporal.ts) | The `Temporal_*` classes, written in TypeScript and compiled in front of programs that mention `Temporal.` |
| [iron_ts_rt/src/tz.rs](../tools/iron_ts_rt/src/tz.rs) | IANA time zones for `ZonedDateTime`: canonical zone name, offset at an instant, local time to instant (compatible disambiguation), fixed-offset zones |
| [lower/bigint.rs](../tools/iron_ts/src/lower/bigint.rs) | `bigint`: literals, binary and unary operators, comparison with bigints and numbers, `BigInt(x)`, `BigInt.asIntN`/`asUintN`, `toString(radix)`, `Number(big)` |
| [iron_ts_rt/src/bigint.rs](../tools/iron_ts_rt/src/bigint.rs) | The `BigInt` runtime: the object layout, operations on `num_bigint::BigInt`, parsing, conversions, JavaScriptCore's error texts |
| [iron_ts_rt/src/uri.rs](../tools/iron_ts_rt/src/uri.rs) | `encodeURI` / `decodeURI` and their Component forms, `escape` / `unescape`; `URIError`s with JavaScriptCore's texts |
| [iron_ts_rt/src/intl_pad.rs](../tools/iron_ts_rt/src/intl_pad.rs) | Generated by `bench/gen_intl_pad_table.ts` from the reference engine's ICU4C: which locales pad a numeric month, day and hour (`Padding`, 31 locales) |
| [throws.rs](../tools/iron_ts/src/throws.rs) | `analyze`: which functions can throw (`Module.may_throw`, `any_throw`), by fixpoint over the call graph |
| [lower/prelude.rs](../tools/iron_ts/src/lower/prelude.rs) | The Error classes, `RegExp` / `RegExpMatchArray`, `Date`, `Intl_Collator`, `Intl_NumberFormat`, `Intl_DateTimeFormat`, `Intl_PluralRules`, `Intl_ListFormat` and `Intl_RelativeTimeFormat`, as TypeScript source, compiled in front of programs that mention errors, regexes (or the string methods that take them), `Date`, `Intl` or bigints (each `Intl` class only when the program mentions it) |
| [lower/enums.rs](../tools/iron_ts/src/lower/enums.rs) | Enums folded into constants (numeric) or string tags |
| [lower/classes.rs](../tools/iron_ts/src/lower/classes.rs) | Classes: fields, constructors, methods, accessors, statics, `#private`, inheritance (prefix layout, virtual dispatch, `instanceof`) |
| [lower/iter.rs](../tools/iron_ts/src/lower/iter.rs) | Well-known symbol member names (`symbol_key`, `well_known`) and the iterator protocol over classes with `@@iterator` |
| [lower/generators.rs](../tools/iron_ts/src/lower/generators.rs) | `desugar` (generators by source rewriting, before parsing), the generated `Generator$K` classes, the `__co*` intrinsics; `Coroutine` is a prelude handle class |
| [iron_ts_rt/src/coro.rs](../tools/iron_ts_rt/src/coro.rs) | Stackful coroutines (corosensei): resume/yield, shadow-stack relinking, GC scanning of suspended frames |
| [lower/asyncs.rs](../tools/iron_ts/src/lower/asyncs.rs) | `desugar` for `async` functions, methods and arrows (before parsing, after generators), `await`, `new Promise<T>`; generates the `Promise$K` classes and their helper functions as TypeScript text |
| [lower/promise.rs](../tools/iron_ts/src/lower/promise.rs) | Routes `__await`, `__asyncRet`, `.then` / `.catch` / `.finally` (by the callback's return type) and `Promise.resolve` / `reject` / `all` / `race` onto the generated helpers |
| [iron_ts_rt/src/event.rs](../tools/iron_ts_rt/src/event.rs) | The microtask queue and the timers (`queueMicrotask`, `setTimeout`, `setInterval`), and the host entry points `ts_rt_microtasks_run`, `ts_rt_timers_advance`, `ts_rt_run_loop` |

## Supported language

| Feature | Lowering |
|---|---|
| Types | `number`, `boolean`, `string`, interfaces and type aliases as structs, discriminated unions as one struct with every variant's fields, string-literal unions as tags, `T \| null`, `T[]`, the typed arrays (see "Typed arrays"), `bigint` (see "BigInt"), `Map<K, V>`, `Set<T>`, `WeakMap`, `WeakSet` (see "Standard library tier 2"), `RegExp` and `Date` (see "Standard library tier 3"), `Record<string, V>` after `satisfies`, function types |
| Functions | `function` and arrow declarations; return types inferred; calls are direct |
| Default, optional, rest parameters | The caller fills in defaults (lowered in a scope of the earlier parameters), a missing optional is null, the rest array is built at the call site |
| Closures | Captured variables live in heap cells; the closure is `{func, env}` with `env` an array of cells; called as `func(env, args...)`. A top-level function used as a value gets a wrapper |
| Array callbacks | `map`, `filter`, `forEach`, `find`, `some`, `every` (and `reduce`, `findIndex`, `includes`) are index loops with an arrow callback's body inlined; no closure object. A non-arrow callback is called as a closure per element |
| Array library | `push`, `pop`, `shift`, `unshift`, `slice`, `splice`, `concat`, `reverse`, `fill`, `sort(cmp)` and default `sort()` (stable merge sort in the runtime), `indexOf`, `at`, `length = n`, `Array.from/of/isArray`, `new Array(n)`; see "Standard library tier 1" for the rest |
| Loops and labels | `while`, `for(;;)`, `do...while`, `for...of`, labelled `break`/`continue` and blocks; per-iteration `let` cells when a closure captures the variable |
| `switch` | The clause index is chosen by `===` compares, then the bodies run in order (fall-through, `default` anywhere) |
| Enums | Numeric and string enums fold to constants, no run-time cost |
| Destructuring | Declarations, parameters, `for...of` variables and callbacks; defaults via `IsNull` |
| Spread | Arrays, objects (field by field, in source order, new shapes allowed) and calls into a rest parameter |
| Module variables | A literal `const` is folded; any other `const`, and every `let`/`var`, is a data slot written by `ts_init` (`Place::Global`) |
| `Math` and operators | Inline instructions or `ts_rt_*` (libm, JS `pow`, `random`); `**`, `~`, `??=`, `\|\|=`, `&&=`, bit operations with ToInt32 |
| Classes | A struct plus functions: constructor `C$init(this, ...)` (`new C(x)` allocates, then calls it), methods with `this` as first parameter, getters/setters, statics as globals, `#private`, parameter properties, `this` in arrows; `extends`, `super(...)`, `super.m()`, overriding, `abstract`, virtual dispatch, `instanceof` (see below); methods as values (`obj.m` is a closure over the receiver, so unlike JS a detached method keeps its `this`); arrow-function fields (the constructor scan includes field initialisers); `@property` on instance fields as compile-time metadata |
| Optional chaining, `!`, `??`, `?:` | Inline null tests |
| Engine API calls | With a manifest, `Namespace.fn(args)` is `CallExtern`: a direct call of the manifest's symbol (`iron_<Namespace>_<fn>`) with the manifest's C signature, arguments checked by count and type. Numbers stay `f64` in scripts and convert at the call: `f32` demotes in and promotes out, `i32`/`u32` use a saturating truncation, `bool` is a byte. Handles are `Ty::Handle(name)`: a 64-bit id, not a GC pointer (not traced or rooted, `===` compares ids). In a `T \| null` slot 0 is null, so a live handle is never 0. An engine function must not call back into scripts or allocate in the script heap. Handle classes (the manifest's `classes`, see [iron-api.md](iron-api.md#handle-classes)) are called through the same path: `emit_extern` makes the call from already-lowered arguments and `call_ext_fn` adds the receiver in front of the call's own arguments (counts are checked without it). `obj.prop` on a handle is the getter's call (`member_of`); `obj.prop = v` is a `Place::Ext` whose write calls the setter (a property with no setter, or an unknown one, is a compile error; compound assignment on a value property is not supported); `obj.method(..)` (`lower_method`) calls the method; `Class.f(..)` on a class name that is not a local is the static. Value classes (`new Vector3(1, 2, 3)`) are ordinary structs with one `number` field per manifest field; as an argument one is passed as its flattened field reads (every argument goes through a temporary first, to keep evaluation order), and a result is a new object filled through a trailing out pointer by `CallExternInto` (a stack slot, then the scalars widened into the fields) |

**Strings in the compiler** ([lower/strings.rs](../tools/iron_ts/src/lower/strings.rs), slice 2).
- `+` and `+=` convert numbers, booleans and null by JavaScript's rules (`ts_rt_num_to_str_gc` and
  static texts); two constants fold at compile time. Template literals are one `concat_n_gc` over an
  array of parts. A chain `a + b + c` still allocates once per `+`.
- `===`/`!==` call `ts_rt_str_eq` (null-safe; codegen emits it for `Str` operands); `<`, `<=`, `>`, `>=`
  call `ts_rt_str_compare` (returns `f64`, code unit order). `s.length` is `ts_rt_str_length`.
  Truthiness is "non-empty" (and non-null).
- String methods map to runtime functions by a table, with default arguments filled in by the
  compiler (`slice`, `substring`, `indexOf`, `padStart`, `split`, `trim`, `toUpperCase`, `at`, ...).
  Number methods `toString` and `toFixed`; globals `String`, `Number`, `parseInt`, `parseFloat`;
  `String.fromCharCode` (`ts_rt_str_from_char_code_gc`) and `String.fromCodePoint`; `join` of a
  `string[]`.
- Compile errors: unsupported string methods (`localeCompare`, `normalize`, `replace`, `match`, ...).
  String literals with lone surrogates work (oxc encodes them as U+FFFD plus hex; `Module::strs` is
  WTF-8 bytes).
- `join` of a `number[]` is `ts_rt_str_join_nums_gc`; a top-level `let s = "x"` widens to `string`.
  Runtime functions that JavaScript makes throw raise a runtime error (see "Exceptions").
- Not done: `codePointAt` past the end (gives plain NaN, not undefined), optional chaining on
  strings, object-literal fields with no declared type, `join` of other non-string arrays.
- The binding generator emits `TAG_*` constants only for texts that are valid, distinct identifiers.
- Tests: `bench/strings.ts` with `tests/strings.rs` (M9-TS-178..182, prefix `str_`, base 16000);
  expected values come from `bench/gen_strings_expected.ts` (bun) into `strings_expected.json`.

**Exceptions** ([throws.rs](../tools/iron_ts/src/throws.rs), [lower/prelude.rs](../tools/iron_ts/src/lower/prelude.rs),
the `Try` handling in [lower/stmt.rs](../tools/iron_ts/src/lower/stmt.rs); design in
[iron-ts-stdlib.md](research/iron-ts-stdlib.md) section 4.1). `throw`, `try`/`catch`/`finally` and the
Error classes. Exceptions are checked returns, not unwinding, and only in functions that can throw.
- **Throw.** `Stmt::Throw(e)` stores the exception object in a per-thread slot of the runtime
  (`ts_rt_throw`; the slot is a GC root) and jumps to the innermost handler, or, with none, to the
  function's exception exit: pop the shadow-stack frame and return a zero value. `Stmt::Try(body,
  catch_var, handler)` sets the handler for its body.
- **Checks.** `throws.rs` computes `Module.may_throw` by a fixpoint over the call graph, counting
  only functions reachable from the exported functions and `ts_init` (closures through `MakeClosure`,
  virtual calls through every method-table entry). A closure or virtual call may throw if anything
  reachable throws (`Module.any_throw`). After such a call codegen calls `ts_rt_exc_pending()` and
  branches to the handler or the exit. **Cost model: a function that cannot throw, and a program with
  no `throw` at all, pay nothing** (no check, no extra code); a call to a function that can throw costs
  one load and a branch.
- **Catch.** `ts_rt_exc_take()` moves the exception into the catch local, typed `Error` (the base
  class); `e instanceof RangeError` narrows like any class. Only Error objects can be thrown (a
  compile error otherwise).
- **Host.** An uncaught exception is `ts_rt_exc_pending()` after an exported function returns; the host
  takes the error object with `ts_rt_exc_take()`.
- **`finally`.** Each `try ... finally` has a completion record: a `kind` variable and a `done`
  label. A `return`/`break`/`continue` in the try or catch stores its exit (and return value) and
  leaves to `done`; the finalizer then runs once and the recorded exit is taken through any enclosing
  finalizers (`FinCtx`, `Exit`, `fin_applies`, `leave_through_finally`, `emit_jump`). An exception
  takes a catch-all handler that runs the finalizer and rethrows.
- **Error classes.** A TypeScript prelude (`lower/prelude.rs`) is compiled in front of programs that
  mention errors: `Error` (`name`, `message`, `toString`), `EvalError`, `RangeError`, `ReferenceError`,
  `SyntaxError`, `TypeError`, `URIError`, and `__rtThrow`. Line numbers in compile messages skip the
  prelude.
- **Runtime errors.** Runtime functions that JavaScript makes throw (`repeat(-1)`, `toFixed(101)`,
  `toString(37)`, `fromCodePoint(-1)`, absurd padding) record a pending runtime error
  (`ts_rt_error_kind`, `ts_rt_error_message_gc`; messages equal bun's) and return a harmless value.
  The compiled call is followed by a check that calls `__rtThrow(kind, message)`.
- `cause` and `AggregateError` are built (see "URI functions and error causes"). Not built: `error.stack`,
  `Error(...)` without `new`, throwing non-Error values.
- Tests: `bench/errors.ts` with `tests/errors.rs` (M9-TS-187..197, prefix `err_`, base 17000);
  expected values come from `bench/gen_errors_expected.ts` (bun) into `errors_expected.json`.

**Standard library tier 1, steps E-H** ([lower/object.rs](../tools/iron_ts/src/lower/object.rs), the expression and
call lowering in `lower/expr.rs`, `arrays.rs`, `strings.rs`; design in [iron-ts-stdlib.md](research/iron-ts-stdlib.md)).
JavaScript's answer exactly; anything not built is a compile error.
- **`typeof`.** Static, from the type; the result is a string-literal tag, so comparisons fold to
  constants. `typeof undeclared` is `"undefined"`. The compiler does not tell null from undefined: a
  nullable value that is null reports `"undefined"`, and only the literal `typeof null` is `"object"`.
- **`Number` and `Math`.** `Number.isInteger/isNaN/isFinite/isSafeInteger/parseFloat/parseInt`, the
  constants (`EPSILON`, `MAX_SAFE_INTEGER`, `MIN_SAFE_INTEGER`, `MAX_VALUE`, `MIN_VALUE`,
  `POSITIVE_INFINITY`, `NEGATIVE_INFINITY`, `NaN`), global `isNaN`/`isFinite`; `Math.sinh cosh tanh
  asinh acosh atanh expm1 log1p` (runtime libm functions).
- **`Array`.** `toSorted toReversed findLast findLastIndex reduceRight flat flatMap`; `sort()` with no
  comparator (numbers and strings, compared as strings in code unit order, stable:
  `ts_rt_arr_sort_default`); `for (const [i, x] of xs.entries())` and `keys()`/`values()` in `for...of`;
  array rest in patterns (`[first, ...others]`).
- **`console`.** `log info debug warn error` of strings, numbers, booleans and null, joined by spaces and
  sent to a host hook (`ts_rt_set_console`; stderr by default; levels 0 to 4). Objects are a compile error.
- **`Object`.** Keys are the struct's fields, fixed at compile time. `Object.keys/values/assign/freeze/
  hasOwn`, `for (const [k, v] of Object.entries(o))`; `values` and `entries` need one common value type;
  `hasOwn` and `in` need a string literal key. Object spread in source order with new shapes
  (`{ ...a, z: 3 }`) and object rest (`const { x, ...rest } = o`). `freeze` does not reject later writes.
  Class fields initialised with a string literal are `string`.
- **Fixed.** A `break` inside a callback inlined into a `try`/`finally`; the string scan cursor is one
  atomic word.
- Not built: Array `with`, `toSpliced`, `copyWithin`, `Array.from({length})`, `Object.fromEntries`,
  `console` of objects.
- Tests: `bench/stdlib.ts` with `tests/stdlib.rs` (M9-TS-199..205); expected values come from
  `bench/gen_stdlib_expected.ts` (bun) into `stdlib_expected.json`.

**Standard library tier 2: `Map`, `Set`, `WeakMap`, `WeakSet`** ([lower/maps.rs](../tools/iron_ts/src/lower/maps.rs),
runtime in [iron_ts_rt/src/map.rs](../tools/iron_ts_rt/src/map.rs); design in
[iron-ts-stdlib.md](research/iron-ts-stdlib.md)).
- **Types.** `Ty::Map(K, V)` and `Ty::Set(K)`. Keys are numbers, strings, booleans, handles or objects
  (by identity); a string-literal union key is stored as a string. The old stub that treated `Set<T>` as
  an array is gone. A collection needs its types from a `new Map<K, V>()` argument, an annotation, or
  `new Map([[k, v], ...])` / `new Set(array)` (element type inferred); otherwise it is a compile error.
- **Raw words.** Keys and values cross into the runtime as 64-bit words: new IR nodes `ExprKind::Bits`
  (typed value to word) and `Unbits` (word back to the type). The map's key kind (0 numbers, 1 strings,
  2 objects, 3 plain words) tells equality, hashing and the collector which words are pointers.
- **Methods.** `get` (a nullable result for pointer and number values; a compile error for boolean or
  literal-union values, where null cannot be told from a value), `set`/`add` (chained), `has`, `delete`,
  `clear`, `size`, `forEach`, `keys()`/`values()` as snapshot arrays, spread and `Array.from` of a Set or
  of `map.keys()`, and live `for (const [k, v] of map)`, `for (const x of set)` and
  `.entries()/.keys()/.values()` in `for...of`. A `for...of` and `forEach` keep a cursor (below), so
  deleting and adding during the loop behave as in JavaScript.
- **Weak collections** are the strong versions: a script cannot tell the difference except by memory.
- Not built: `new Map(otherMap)`, map spread, `Map.groupBy`, `entries()` outside `for...of`.
- Tests: `bench/collections.ts` with `tests/collections.rs` (M9-TS-209..213), expected values from
  `bench/gen_collections_expected.ts` (bun) into `collections_expected.json`; runtime unit tests
  M9-TS-206..208 in `map.rs`; compile errors M9-TS-214.

**Standard library tier 2: `JSON`** ([lower/json.rs](../tools/iron_ts/src/lower/json.rs), runtime in
[iron_ts_rt/src/json.rs](../tools/iron_ts_rt/src/json.rs)). Driven by the program's static types.
- **Descriptors.** For each type the compiler writes a byte blob (`Module::blobs`, `ExprKind::Blob`):
  a kind byte per type (number, boolean, string, literal union, array, nullable, object) and, for an
  object, a header word and its fields. Struct header words are written as id + 1; the module's
  `struct_base` is added at code generation. The runtime (`ts_rt_json_stringify_gc`,
  `ts_rt_json_parse_gc`) walks the blob.
- **`JSON.stringify(v, null, indent)`.** Numbers (non-finite becomes `null`, `-0` becomes `0`), booleans,
  strings with JSON escapes (lone surrogates as `\udXXX`), literal unions, arrays, nullable values
  (`null`), objects in key order (array-index keys first), `Map`/`Set` as `{}`. A field declared with
  `?` (`Field.optional`) is left out when null. A subclass instance prints its own fields: the
  descriptor lists the class and its subclasses and the object's header picks one. Indentation by a
  number (clamped to 10) or a string.
- **`JSON.parse`.** Needs its type from the context: `const x: T = JSON.parse(s)` or `JSON.parse(s) as
  T`. Objects, arrays and strings are built without collecting mid-parse (one collection check
  before). Fields the type lacks are skipped. Syntax errors follow JavaScriptCore's parser and
  messages exactly (`SyntaxError: JSON Parse error: ...`; 67 malformed texts checked against bun). A
  shape mismatch (wrong kind, missing property, string outside a literal union) is a `TypeError`, raised
  only after the whole text proved to be valid JSON, so a syntax error anywhere wins.
- **Errors.** Runtime errors reach the program through `__rtThrow(kind, message)` (1 `RangeError`,
  2 `TypeError`, 3 `SyntaxError`, 4 `URIError`).
- Also in this step: `join` of boolean arrays.
- Not built: reviver, replacer, `toJSON`. See [differences.md](differences.md).
- Tests: `bench/json.ts` with `tests/json.rs` (M9-TS-215..221), expected values from
  `bench/gen_json_expected.ts` (bun) into `json_expected.json`.

**Standard library tier 3: `RegExp`, step R1** ([lower/regexp.rs](../tools/iron_ts/src/lower/regexp.rs), runtime in
[iron_ts_rt/src/regexp.rs](../tools/iron_ts_rt/src/regexp.rs)). The engine is the `regress` crate (an
ECMAScript regex engine; `iron_ts_rt` uses it with the `utf16` feature). `iron_ts` depends on `iron_ts_rt`
only to validate regex literals at compile time (`regex_validate`).
- **Runtime.** A per-thread registry of compiled regexes, each named by a number; equal (pattern, flags)
  share one entry. Positions are UTF-16 units: an ASCII string is matched as bytes, any other as UTF-16
  text; `u`/`v` regexes match by code point pairs. Sticky `y` checks that the match starts at `lastIndex`.
  Capture group names are scanned from the pattern. Functions: `ts_rt_regex_new`, `_flags_gc`,
  `_source_gc`, `_exec_gc`, `_match_index`, `_match_end`, `_groups_gc`. Syntax errors use JavaScriptCore's
  wording (mapped from regress's messages); bad flags say `Invalid flags supplied to RegExp constructor.`
- **Objects.** A regex is an ordinary object of the prelude class `RegExp` with the fields `source`,
  `flags`, `global`, `ignoreCase`, `multiline`, `dotAll`, `unicode`, `unicodeSets`, `sticky`, `hasIndices`,
  `lastIndex` and `id`. `/re/flags` literals (validated at compile time), `new RegExp(p, f)`, `RegExp(p, f)`
  and the copy `new RegExp(re, f)` build it.
- **`test` and `exec`.** Both follow `lastIndex` for `g`/`y` regexes and reset it on a miss. `exec` returns
  a `RegExpMatchArray` (`matches: (string | null)[]`, `index`, `input`, `groups: Map<string, string | null>
  | null`). The match object is also seen as its array (`match_view`): `m[i]`, `m.length`, destructuring,
  spread, `for...of` and the array methods. `m.groups.name` reads the groups map by name (a property read on
  a `Map<string, V>`). `String(re)` and objects with `toString()` convert to strings.
- **Limits** (see [differences.md](differences.md)): a group that did not take part is null where JavaScript
  has undefined, so `m[1].length` dereferences null instead of throwing a `TypeError`; `groups === null`
  is true where JavaScript has undefined; tuple types are not supported.
- Tests: `bench/regexp.ts` with `tests/regexp.rs` (M9-TS-222..231), expected values from
  `bench/gen_regexp_expected.ts` (bun) into `regexp_expected.json`.

**Step R2: the String methods that take a regular expression** (section "the String methods that take a regular
expression" in [lower/regexp.rs](../tools/iron_ts/src/lower/regexp.rs), hooked in from
[lower/strings.rs](../tools/iron_ts/src/lower/strings.rs); runtime appended to
[iron_ts_rt/src/regexp.rs](../tools/iron_ts_rt/src/regexp.rs)).
- **Methods.** `search`, `match`, `matchAll`, `replace`, `replaceAll` and `split` accept a `RegExp` or a string.
  A string becomes a regex: `match`, `search` and `matchAll` treat it as a pattern; `replace`, `replaceAll` and
  `split` match it literally (`ts_rt_regex_escape_gc`). Results are bun's, with UTF-16 indices.
- **`match`.** With a `g` regex it returns the matched texts and resets `lastIndex` to 0; without `g` it returns
  the match object, as `exec` does. **`matchAll`** is eager: it returns an array of match objects (so `for...of`,
  spread and `map` work) from a copy of the regex. A non-global regex throws bun's `TypeError`, as does
  `replaceAll` with a non-global regex.
- **Replacement strings** support `$$`, `$&`, `` $` ``, `$'`, `$n`, `$nn` and `$<name>` (`GetSubstitution`).
- **Replacement functions** are inlined. The parameters are (match, groups..., offset, string, namedGroups). A
  function with more than the match as a parameter needs the regex's groups at compile time: a regex literal in
  the call, `new RegExp("literal")`, or a function-local `const` declared from one. Otherwise, and for too many
  parameters, it is a compile error; rest parameters are not supported.
- **`split(re, limit)`** includes the captures; `split(string, limit)` now works too.
- **Runtime.** `ts_rt_regex_search`, `_match_all_gc`, `_replace_gc`, `_split_gc`, `_escape_gc` and `_advance`
  (the next index after an empty match, a whole pair under `u`/`v`); `join` works on `(string | null)[]`.
- **Limits:** a group that did not take part is a null element; module-level `const` regexes carry no group
  information for replacement functions (see [differences.md](differences.md)).
- Tests: `bench/regexp2.ts` with `tests/regexp2.rs` (M9-TS-232..239), expected values from
  `bench/gen_regexp2_expected.ts` (bun) into `regexp2_expected.json`.

**Standard library tier 3: `Date`** ([lower/date.rs](../tools/iron_ts/src/lower/date.rs), runtime in
[iron_ts_rt/src/date.rs](../tools/iron_ts_rt/src/date.rs); small hooks in `lower/{expr,arrays,strings,prelude,mod}.rs`).
- **Object.** `Date` is a prelude class with one field, `t`: the time value in ms (NaN is an invalid date). It
  is added when the source mentions `Date`. `===` is identity; `-`, `+d`, `Number(d)`, `<` and `>` use `t`.
- **Supported.** `new Date()`, `(ms)`, `(string)`, `(date)` and `(y, m, d?, h?, mi?, s?, ms?)` (plain
  arguments, at most seven); `Date.now`, `Date.UTC`, `Date.parse`; every getter and setter, local and UTC;
  `getTime`, `valueOf`, `setTime`, `getTimezoneOffset`; `toISOString` (RangeError `Invalid Date` for an invalid
  date, as JavaScriptCore), `toJSON`, `toUTCString` / `toGMTString`, `toDateString`.
- **Calendar.** MakeDay, MakeTime and TimeClip follow the spec. Local time uses jiff's tz database (jiff 0.2
  in `iron_ts_rt`): the OS zone, or the IANA name given to `ts_rt_date_set_zone` (the tests use it; a host can
  too). A repeated local time (clock set back) reads as the earlier instant; a skipped one (clock set forward)
  is read with the offset from before the jump, as engines do. Times beyond jiff's range (years +-9999) are
  mapped by whole 400-year Gregorian cycles. `getTimezoneOffset` truncates to whole minutes toward zero (old
  local mean time), like engines.
- **Parsing** (`ts_rt_date_parse`). ISO forms first: a date alone is UTC, a date-time without an offset is
  local; expanded years, `24:00`, lowercase `t`/`z`, a space separator, offsets with or without a colon, a
  fraction truncated to ms. Then a legacy tokenizer: month names by their first three letters, weekdays
  ignored, `M/D/Y`, `Y/M/D`, `Y-M-D`, AM/PM, `GMT`/`UTC`/`Z`/`EST`/`EDT`/`CST`/`CDT`/`MST`/`MDT`/`PST`/`PDT`,
  `+hhmm`, parenthesised comments, and two-digit years (0-49 are 20xx, 50-99 are 19xx). Checked against bun
  for about 76 strings in five zones.
- **Runtime.** `ts_rt_date_now`, `_field`, `_offset`, `_from_parts`, `_set`, `_clip`, `_iso_gc`,
  `_utc_string_gc`, `_date_string_gc`, `_to_string_gc`, `_parse`, `_set_zone`.
- **`toString` / `toTimeString`** (`ts_rt_date_to_string_gc`; `String(date)`, template strings and `date + x`
  use the same text): `Mon Oct 19 2026 10:30:00 GMT-0400 (Eastern Daylight Time)`, equal to bun's. The prelude
  `Date` class has a `toString` method (`__dateToString`), so generic stringification works, and `date + x`
  concatenates that string like JavaScript. The offset is written in whole minutes toward zero (`GMT-0456`
  for old local mean time). The zone name (`zone_name`) comes from ICU4X's root English long specific names
  through the zone's IANA id. For old dates, and dates whose raw offset has no name, the name follows whether
  the zone was on daylight time then: a recent winter or summer day with the same daylight state is used for
  the lookup, as engines do. A zone without an IANA id (a fixed offset) prints `GMT+hh:mm`.
- **Compile errors until Intl step I3:** `toLocaleString`, `toLocaleDateString`, `toLocaleTimeString`.
- **Limits** (see [differences.md](differences.md)): `Date()` without `new`; a few exotic legacy strings;
  `Date` inside typed JSON.
- Tests: `bench/date.ts` with `tests/date.rs` (M9-TS-240..253, 276), expected values from
  `bench/gen_date_expected.ts` (bun) into `date_expected.json`.

**Standard library tier 3: `Intl`, step I1** ([lower/intl.rs](../tools/iron_ts/src/lower/intl.rs), runtime in
[iron_ts_rt/src/intl.rs](../tools/iron_ts_rt/src/intl.rs); hooks in `lower/{strings,expr,types,prelude,mod,arrays}.rs`).
The runtime uses ICU4X (`icu` 2.x in `iron_ts_rt`) with its compiled CLDR data.
- **Supported.** `String.prototype.localeCompare(that, locales?, options?)`, `normalize(form?)` (NFC, NFD, NFKC,
  NFKD; lone surrogates are left alone), `toLocaleUpperCase` / `toLocaleLowerCase(locales?)` (language-sensitive:
  `tr`, `az`, `lt`, `el`, ...) and `new Intl.Collator(locales?, options?)` with `.compare(a, b)`.
- **Collator object.** `Intl.Collator` is the prelude class `Intl_Collator` (the type name maps to it, in
  `types.rs`), added when the source mentions `Intl.`. It holds an id into the runtime's collator table;
  `compare` is a closure field, so `arr.sort(c.compare)` works. No other `Collator` method is built.
- **Locales.** A string, or an array literal or array of strings; the first tag wins. A bad tag raises
  `RangeError: invalid language tag: <tag>`. No argument means the default locale, which the host sets with
  `ts_rt_intl_set_locales("en-US,de,ja")` (the first is the default; `en-US` if never called). The per-game
  list from `package.json` is not wired to it yet (that needs `iron_app`).
- **Options** (collation): `usage`, `sensitivity` (base, accent, case, variant), `numeric`, `caseFirst`,
  `ignorePunctuation`. The argument must be an object literal with literal values: the compiler turns it into
  a `key=value;` string, so the runtime never sees an object. Anything else is a compile error. Bad values
  raise bun's `RangeError` text.
- **Runtime.** `ts_rt_intl_set_locales`, `ts_rt_str_normalize_gc`, `ts_rt_str_locale_case_gc`,
  `ts_rt_collator_new`, `ts_rt_collator_compare`, `ts_rt_str_locale_compare`.
- **Checked against bun** for en, de, sv, tr, ja and zh sorts, special casing and normalization.
- **Limits** (see [differences.md](differences.md)): all locales' data is linked (no trimming yet); an
  unsupported locale id falls back to root collation; `resolvedOptions`, `supportedLocalesOf` and the other
  `Collator` methods are not built; ICU4X's CLDR version can order a few things differently from the engine's.
- **Still to come:** `Locale`, `DisplayNames`, `Segmenter` (the other classes are in steps I2 to I4 below).
- Tests: `bench/intl.ts` with `tests/intl.rs` (M9-TS-254..263), expected values from
  `bench/gen_intl_expected.ts` (bun) into `intl_expected.json`.

**Standard library tier 3: `Intl`, step I2: `NumberFormat`** ([lower/intl.rs](../tools/iron_ts/src/lower/intl.rs),
runtime in [iron_ts_rt/src/intl_number.rs](../tools/iron_ts_rt/src/intl_number.rs); hooks in
`lower/{strings,prelude,expr,mod}.rs`). `iron_ts_rt` enables ICU4X's `unstable` feature (the experimental
currency and percent formatters) and `fixed_decimal` with `ryu`.
- **Supported.** `Number.prototype.toLocaleString(locales?, options?)` (in `strings.rs`) and
  `new Intl.NumberFormat(locales?, options?)` with `.format(n)`. `Intl.NumberFormat` is the prelude class
  `Intl_NumberFormat`; `format` is a closure field, so `numbers.map(nf.format)` works. Locales and the
  options literal are read at compile time like the collator's (`lower_new_intl`, `options_spec` are shared).
- **Options.** `style` (`decimal`, `percent`, `currency`; `unit` raises a `RangeError` for now), `currency`,
  `currencyDisplay` (`symbol`, `narrowSymbol`, `code`, `name`), `currencySign` (`standard`, `accounting`),
  `notation` (`standard`, `scientific`, `engineering`, `compact`), `compactDisplay`, `signDisplay` (all five),
  `useGrouping`, `minimumIntegerDigits`, `minimumFractionDigits` / `maximumFractionDigits`,
  `minimumSignificantDigits` / `maximumSignificantDigits`, `roundingMode` (all nine), `trailingZeroDisplay`.
  Defaults follow ECMA-402: currency digits come from an ISO 4217 table; compact keeps whole numbers from two
  integer digits and otherwise two significant digits.
- **How.** Rounding and padding are done on a `fixed_decimal::Decimal`; ICU4X then formats it (decimal,
  compact with the exponent chosen first, percent). Currency and non-finite values use a placeholder: the
  style's formatter formats the digit 7 and the compiler-side code swaps in the number string, because
  ICU4X's currency formatter forces the currency's own fraction digits. Scientific and engineering are
  assembled by hand (`1.235E6`, with the locale's digits and decimal mark). NaN and infinity are text (`NaN`,
  `∞`; NaN is localized only for `ar` and `ru`).
- **`formatToParts`.** `nf.formatToParts(n)` formats the number, then splits the string using the locale's digits,
  decimal separator and group separator (`number_parts` in `intl_number.rs`). Types: `integer`, `group`, `decimal`,
  `fraction`, `minusSign`, `plusSign`, `percentSign`, `currency`, `compact`, `exponentSeparator`, `exponentInteger`,
  `exponentMinusSign`, `nan`, `infinity` and `literal`. The result is `Intl.NumberFormatPart[]`, the prelude class
  `Intl_Part { type, value }` (`INTL_PARTS` in `lower/prelude.rs`, decoded from a separator-joined string).
- **`resolvedOptions`.** `nf.resolvedOptions()` gives the prelude class `Intl_Resolved` (`ts_rt_number_format_resolved_gc`
  in `intl_number.rs`, passed as `key=value;` pairs and parsed in the prelude): `locale`, `numberingSystem` (always
  `latn`), `style`, `currency`, `notation`, `signDisplay`, `minimumIntegerDigits`, and either the fraction digits or
  the significant digits that apply. Fields not listed (`currencyDisplay`, `useGrouping`, `roundingMode`,
  `roundingIncrement`, `trailingZeroDisplay`, ...) are not in the result. The numeric fields are `0` when they do
  not apply (JavaScript leaves them out).
- **Limits** (see [differences.md](differences.md)): `style: "unit"`, compact currency,
  `formatRange` and `supportedLocalesOf` are not built; Number values only (no BigInt,
  no strings).
- Checked against bun for 15 locales and many option combinations.
- Tests: `bench/numfmt.ts` with `tests/numfmt.rs` (M9-TS-264..275), expected values from
  `bench/gen_numfmt_expected.ts` (bun) into `numfmt_expected.json`.

**Standard library tier 3: `Intl`, step I3a: date formatting** ([lower/intl.rs](../tools/iron_ts/src/lower/intl.rs),
runtime in [iron_ts_rt/src/intl_date.rs](../tools/iron_ts_rt/src/intl_date.rs); hooks in
`lower/{date,strings,prelude,expr,mod}.rs`, `Date::icu_parts` in `date.rs`). Design and the open choice between
ICU4X lite and a full build: [iron-ts-intl-datetime.md](research/iron-ts-intl-datetime.md).
- **Supported.** `Date.prototype.toLocaleString` / `toLocaleDateString` / `toLocaleTimeString(locales?, options?)`
  and `new Intl.DateTimeFormat(locales?, options?)` with `.format(date)`. `format` is a closure field like the
  collator's and NumberFormat's; the prelude class is `Intl_DateTimeFormat`. The `Intl` prelude classes are now added
  one by one, so a program only gets the ones it mentions. Locales and the options literal are read at compile
  time (`options_spec`); options must be an object literal with literal values.
- **Options.** `dateStyle` / `timeStyle`, the components `weekday year month day hour minute second
  fractionalSecondDigits`, `hour12`, `hourCycle`, `timeZone` (an IANA name through jiff; the default is the
  program's zone) and `timeZoneName` (six widths). `era`, `dayPeriod`, `calendar` and `numberingSystem` raise a
  `RangeError`. Defaults follow ECMA-402 ToDateTimeOptions per entry point: `toLocaleString` date and time,
  `toLocaleDateString` date, `toLocaleTimeString` time, the constructor date. `dateStyle` / `timeStyle` exclude
  components, and a style the entry point does not allow is a `TypeError` with JavaScriptCore's text.
- **How.** The options map to the nearest ICU4X field set (`FieldSetBuilder`: date fields, length from the month or
  weekday width, time precision, zone style) and a `DateTimeFormatter<CompositeFieldSet>`. Styles use that
  formatter directly (ICU4X's patterns are exact). For component options the formatter's pattern is read and its
  numeric and text field widths are rewritten to the requested ones (`adjust_pattern`: `dd.MM.y` to `d.M.y`,
  2-digit, weekday and month name widths), then formatted through `FixedCalendarDateTimeNames` with the new
  pattern (Gregorian locales only). Whether a numeric month, day or hour is padded comes from `intl_pad.rs`,
  generated from the reference engine's ICU4C for 31 main-market locales (other locales keep ICU4X's padding).
  The table is the first piece of the dev-time `availableFormats` generator planned in the design note. U+202F is
  replaced by a plain space, as engines do.
- **Time zones.** ICU4X gets the real offset and instant, so an offset with no zone name prints `GMT+01:00`. That
  differs from `Date.toString`, which looks up the name of a recent day with the same daylight state.
- **`formatToParts`.** `dtf.formatToParts(date)` asks ICU4X for part annotations through a `PartsWrite` sink
  (`intl_parts.rs`). Types: `era`, `year`, `month`, `day`, `weekday`, `dayPeriod`, `hour`, `minute`, `second`,
  `timeZoneName` and `literal`; there is no `fractionalSecond` part. The result is `Intl.DateTimeFormatPart[]`
  (the same `Intl_Part` class).
- **`resolvedOptions`.** `dtf.resolvedOptions()` gives `Intl_Resolved` (`ts_rt_dtf_resolved_gc` in `intl_date.rs`):
  `locale` (the first requested tag, or the default), `calendar` (`gregory`), `numberingSystem` (`latn`) and
  `timeZone` (the canonical IANA name; the system zone when none was given, `UTC` for `utc`). The component fields
  (`year`, `hour`, `hour12`, `hourCycle`, `dateStyle`, ...) are not in the result.
- **Limits** (see [differences.md](differences.md), "Intl date formatting"): `formatRange`,
  `era`, `dayPeriod` and other calendars are not built; component combinations ICU4X cannot
  match differ from ICU4C. Exactness would come from `icu-full` or a generated table.
- Tests: `bench/datefmt.ts` (made by `bench/gen_datefmt_program.py`) with `tests/datefmt.rs` (M9-TS-277..286), five
  time zones and 15 locales, expected values from `bench/gen_datefmt_expected.ts` (bun) into
  `datefmt_expected.json`. Columns ICU4X cannot match are dropped from the test and listed in `differences.md`.

**Standard library tier 3: `Intl`, step I4: PluralRules, ListFormat, RelativeTimeFormat**
([lower/intl.rs](../tools/iron_ts/src/lower/intl.rs), runtime in
[iron_ts_rt/src/intl_misc.rs](../tools/iron_ts_rt/src/intl_misc.rs); hooks in `lower/{expr,mod,prelude}.rs`).
- **Supported.** `new Intl.PluralRules(locales?, { type, minimumFractionDigits, maximumFractionDigits }).select(n)`
  (cardinal or ordinal; NaN and infinity are `"other"`), `new Intl.ListFormat(locales?, { style, type }).format(items)`
  (`items` is a `string[]`: arrays only, not iterables) and
  `new Intl.RelativeTimeFormat(locales?, { style, numeric }).format(value, unit)` (the unit is singular or plural,
  `"day"` or `"days"`). Bad option values, units and tags raise `RangeError` with bun's texts.
- **Classes.** The prelude classes are `Intl_PluralRules`, `Intl_ListFormat` and `Intl_RelativeTimeFormat`, each
  added only when the source mentions it. Like the collator's `compare`, the method is a closure field over the
  runtime's table id, so it can be detached (`const f = lf.format`) or passed as a callback.
- **Options.** Locales and the options literal are read at compile time (`options_spec`). `options_spec` now looks
  through `as`, `satisfies`, `!` and parentheses, so `{ type: "ordinal" as any }` is still a literal.
- **Runtime.** `ts_rt_plural_new` / `_select_gc`, `ts_rt_list_new` / `_format_gc`, `ts_rt_rtf_new` / `_format_gc`,
  on ICU4X's plural rules, list formatter and relative-time formatter.
- **Limits** (see [differences.md](differences.md)): `supportedLocalesOf`, `resolvedOptions`, `formatToParts`,
  `formatRange` and `selectRange` are not built (`Locale`, `DisplayNames` and `Segmenter` came later, below).
- Checked against bun in 17 to 20 locales (every result equal).
- Tests: `bench/intl4.ts` (made by `bench/gen_intl4_program.py`) with `tests/intl4.rs` (M9-TS-296..312), expected
  values from `bench/gen_intl4_expected.ts` (bun) into `intl4_expected.json`.

**Standard library tier 3: `Intl.Segmenter`, `Intl.Locale`, `Intl.DisplayNames`**
(runtime in [iron_ts_rt/src/intl_segment.rs](../tools/iron_ts_rt/src/intl_segment.rs),
[intl_locale.rs](../tools/iron_ts_rt/src/intl_locale.rs) and [intl_names.rs](../tools/iron_ts_rt/src/intl_names.rs);
prelude classes `INTL_SEGMENTER`, `INTL_LOCALE` and `INTL_DISPLAYNAMES` in `lower/prelude.rs`).
- **Segmenter.** `new Intl.Segmenter(locales?, { granularity })` with `grapheme`, `word` or `sentence` boundaries from
  ICU4X's segmenter. `segment(text)` returns an iterable of `{ segment, index, input, isWordLike }`, plus
  `containing(i)`. `isWordLike` is `false`, not `undefined`, for the non-word granularities.
- **Locale.** `new Intl.Locale(tag, options?)` gives the canonical `toString()`, `language`, `script`, `region`,
  `baseName`, and `calendar`, `collation`, `hourCycle`, `caseFirst`, `numeric` and `numberingSystem` (from the tag's
  `-u-` extension or the options), and `maximize()` / `minimize()`. `"und"` maximizes to `en-Latn-US` by a special
  case. Getters of absent parts give `null`, not `undefined`.
- **DisplayNames.** `new Intl.DisplayNames(locales, { type })` for `region`, `script` and `language` only, with
  `.of(code)`. CLDR names can differ from bun's (for example `zh-Hans`).
- **Options.** As for the other `Intl` classes: object literals with literal values, read at compile time; a
  `{} as X` cast is looked through.
- **Limits** (see [differences.md](differences.md)): `formatRange`, `resolvedOptions` and
  `Intl.supportedValuesOf` are not built; `DisplayNames` has no `currency`,
  `calendar` or `dateTimeField` type and no `fallback: "none"`.
- **`Intl.getCanonicalLocales`.** Takes a string or an array of strings, canonicalizes each tag (as `Intl.Locale`
  does), drops duplicates and throws a `RangeError` for an invalid tag (`INTL_CANON` in `lower/prelude.rs`).
- Tests: M9-TS-456..479; `getCanonicalLocales` M9-TS-516..536; `formatToParts` M9-TS-537..549; `resolvedOptions` M9-TS-550..555.

**Temporal, ISO calendar** ([lower/temporal.rs](../tools/iron_ts/src/lower/temporal.rs), classes in
[lower/temporal.ts](../tools/iron_ts/src/lower/temporal.ts), zones in [iron_ts_rt/src/tz.rs](../tools/iron_ts_rt/src/tz.rs)).
- **Rewrite.** `Temporal.X` becomes the prelude class `Temporal_X`, as a text rewrite of the program before parsing (a
  string literal containing `Temporal.Name` is rewritten too). The classes are plain TypeScript in `temporal.ts`,
  compiled in front of any program that mentions `Temporal.`.
- **Classes.** `Duration`, `PlainDate`, `PlainTime`, `PlainDateTime`, `Instant` (bigint nanoseconds),
  `ZonedDateTime` (nanoseconds plus a zone name) and `Now`. `tz.rs` supplies the canonical zone name, the offset at
  an instant, local time to instant with the `compatible` disambiguation, and fixed-offset zones such as `+05:30`.
- **`from`.** The lowerer routes `Temporal.X.from(v)`: a string goes to the parse function, an object literal builds
  the class from the literal and then validates and constrains it, any other object is copied. A `ZonedDateTime`
  literal needs `timeZone`. A string is also accepted where a Temporal object is expected
  (`date.equals("2024-01-01")`), and an object literal passed as a `Duration` or `PlainDate` parameter is typed as the
  class itself.
- **`PlainYearMonth`.** `add` / `subtract` take years and months only (a duration with units below months is a
  `RangeError`), `until` / `since` with `year` or `month`, `toPlainDate({ day })`, `with`, `compare`, `equals`.
- **`PlainMonthDay`.** Reference year 1972; `with`, `toPlainDate({ year })`, `equals`. There is no `month` getter,
  as in the engines.
- **`round`.** On `PlainTime`, `PlainDateTime`, `Instant` and `ZonedDateTime`: `smallestUnit`, `roundingMode`
  (all nine modes) and `roundingIncrement`; `round("minute")` string shorthand goes through
  `Temporal_RoundOptions`. `Duration.round` handles time units only, with `largestUnit` and no `relativeTo`.
  Rounding is not available in `until` / `since`.
- **Limits** (see [differences.md](differences.md)): no other calendars, `toLocaleString`, `total` with
  `relativeTo`, rounding in `until` / `since`, `overflow: "reject"`, or `disambiguation` / `offset` options.
- Checked against bun. Tests: `bench/temporal.ts` and `bench/zoned.ts` with `tests/temporal.rs` and `tests/zoned.rs`
  (M9-TS-480..515); `PlainYearMonth`, `PlainMonthDay` and `round` M9-TS-516..536.

**URI functions and error causes** ([iron_ts_rt/src/uri.rs](../tools/iron_ts_rt/src/uri.rs), hooks in
`lower/{strings,prelude}.rs`).
- **URI functions.** `encodeURI`, `encodeURIComponent`, `decodeURI` and `decodeURIComponent` (the non-component
  forms keep the reserved characters `; / ? : @ & = + $ , #`; `decodeURI` leaves escapes of them in place), and the
  legacy `escape` / `unescape`. They are runtime calls (`ts_rt_uri_encode_gc`, `ts_rt_uri_decode_gc`,
  `ts_rt_escape_gc`, `ts_rt_unescape_gc`) and may throw.
- **URIError.** Kind 4 of `__rtThrow`. A lone surrogate gives "String contained an illegal UTF-16 sequence."; a bad
  or truncated escape, an overlong, surrogate or too-large UTF-8 sequence gives "URI error", as JavaScriptCore does.
- **Error cause.** `new Error(message, { cause })` (every Error class takes the second argument; the
  `ErrorOptions` interface is in the prelude). The field is `cause: Error | null`, so a cause must be an `Error`
  (anything else is not supported). `undefined` and `null` are not told apart: an absent cause reads null, test it
  with `== null`.
- **AggregateError.** `new AggregateError(errors, message?, options?)` with `errors: Error[]` (a copy of the
  argument) and `name` `"AggregateError"`.
- **Not built:** `Error.stack` and `Error.captureStackTrace`.
- Tests: `bench/uri.ts` with `tests/uri.rs` and `bench/errors2.ts` with `tests/errors2.rs` (M9-TS-287..295), expected
  values from `bench/gen_uri_expected.ts` and `bench/gen_errors2_expected.ts` (bun). `bench/add_module.py` scaffolds
  a new bun-compared test module.

**Typed arrays** ([lower/typed.rs](../tools/iron_ts/src/lower/typed.rs), runtime in
[iron_ts_rt/src/typed.rs](../tools/iron_ts_rt/src/typed.rs), element access in `typed_load` / `typed_store` in
`codegen.rs`).
- **Types.** `Ty::Typed(TypedKind)` for `Int8Array`, `Uint8Array`, `Uint8ClampedArray`, `Int16Array`, `Uint16Array`,
  `Int32Array`, `Uint32Array`, `Float32Array` and `Float64Array`. The object is the array header with the `HDR_BYTES`
  tag: `len` elements, `cap` bytes, raw little-endian data. The compiler passes the kind code (the index in
  `TypedKind::ALL`) to the runtime functions.
- **Elements.** Reads and stores are inline code, loaded or stored by width and converted to or from a double. A store
  wraps integers as `ToInt8` .. `ToUint32`, rounds to `f32` for `Float32Array`, and clamps and rounds half to even for
  `Uint8ClampedArray`. NaN, infinities and values of 2^63 or more store 0 in the integer kinds.
- **Constructors.** `new X()`, `new X(length)`, `new X(number[])`, `new X(typed)` and `X.from(...)` with the same three
  sources.
- **Members.** `length`, `byteLength`; `fill`, `set(source, offset)`, `slice`, `reverse`, `sort` (numeric, with or
  without a comparator), `toSorted`, `toReversed`, `toString`; `for...of` and spread.
- **Everything else** (`indexOf`, `join`, `reduce`, `every`, `map`, `filter`, ...) runs on a `number[]` copy; `map` and
  `filter` convert the result back to the same typed kind.
- **GC.** A runtime function that allocates copies its source out first, because a temporary source is not rooted.
- Tests: `bench/typed.ts` with `tests/typed.rs` (M9-TS-313..359, 47 tests, bun-compared, run in normal, stress and
  GC-off modes), expected values from `bench/gen_typed_expected.ts` and `bench/gen_typed_program.py`.
- **Buffers and views.** `ArrayBuffer` and `DataView` are prelude classes
  ([lower/prelude.rs](../tools/iron_ts/src/lower/prelude.rs)) over a `Uint8Array` that owns the bytes.
  `new Float32Array(buffer, byteOffset, length)`, `subarray`, `.buffer` and `.byteOffset` make views that share
  memory through the runtime header `HDR_VIEW`: a typed array whose `data` points into the owner's block and which
  keeps the owner alive. It has the same field layout as `Array`, so inline element access is unchanged.
  `.buffer` of an owning array returns a new `ArrayBuffer` object each time over the same memory (not `===`).
- **DataView.** `get` / `set` for `Int8` .. `Float64`, big-endian by default; out-of-range access is a `RangeError`
  (the messages differ from bun's).
- **Bytes across the engine boundary.** `ts_rt_typed_from_host_gc(kind, ptr, byte_len)` copies host bytes into a typed
  array (root it until it is handed over); `ts_rt_typed_data(a)` and `ts_rt_typed_byte_len(a, kind)` read it back
  (valid until the next allocation). Generated Rust bindings for exported functions with typed-array parameters are
  not done.
- Tests: `bench/buffers.ts` with `tests/buffers.rs` (M9-TS-431..441).
- **Not built:** see "Typed arrays" in [differences.md](differences.md).

**`structuredClone`** ([lower/clone.rs](../tools/iron_ts/src/lower/clone.rs)). One lifted function per type, so a
recursive type gives a recursive function. Strings, numbers, booleans and bigints are shared; arrays, objects and
typed arrays are copied. A class instance keeps its class (JavaScript gives a plain object), and fields that only a
derived class declares stay shared. `Map`, `Set` and functions are compile errors; cyclic data does not end.
Tests: `bench/clone.ts` with `tests/clone.rs` (M9-TS-442..455, with `weak.rs`).

**`WeakRef<T>`** is generated by `prelude::weak_refs` for every class of the program and holds its target strongly,
so `deref()` always finds it. `WeakMap` and `WeakSet` are `Map` and `Set` (strong). There is no
`FinalizationRegistry` and no `globalThis`. Tests: `bench/weak.ts` with `tests/weak.rs`.

**BigInt** ([lower/bigint.rs](../tools/iron_ts/src/lower/bigint.rs), runtime in
[iron_ts_rt/src/bigint.rs](../tools/iron_ts_rt/src/bigint.rs); the `num-bigint` crate; hooks in
`lower/{expr,arrays,types,prelude}.rs`).
- **Type.** `Ty::BigInt`, a leaf GC object (nothing to trace): header `HDR_BIGINT`, sign (-1, 0 or 1), byte count,
  then the magnitude bytes, little endian. Zero has no bytes; truthiness is "sign is not 0".
- **GC.** Every operation reads its operands into `num_bigint::BigInt` values first and allocates the result last
  (`read` / `alloc`), because a temporary operand is not rooted.
- **Literals.** `123n`, `0xFFn`, `0b101n`, `0o17n`, `1_000n` compile to a runtime parse of the literal's text
  (`ts_rt_bigint_literal_gc`).
- **Operators.** `+ - * / % **` (division truncates, the remainder takes the dividend's sign), `& | ^ ~ << >>` (two's
  complement semantics, `>>` rounds toward negative infinity), unary `-`, `++`/`--` and compound assignment. Binary
  operations are one call `ts_rt_bigint_op_gc(op, a, b)`. `>>>` and unary `+` are compile errors, as is mixing a
  bigint with a number in arithmetic (JavaScript throws a `TypeError` at run time).
- **Comparison.** `< <= > >= == != === !==` between bigints, and between a bigint and a number, exactly (no rounding
  through a double; NaN is never equal or ordered, infinities order beyond every bigint).
- **Conversions.** `BigInt(number | string | boolean | bigint)`, `Number(big)` (nearest double), `String(big)`,
  template strings, `big.toString(radix)`, `valueOf`, `BigInt.asIntN(bits, big)` / `asUintN`, `typeof` (`"bigint"`).
- **Errors** (JavaScriptCore's texts, raised like other runtime errors): `RangeError` "Not an integer",
  "0 is an invalid divisor value.", "Negative exponent is not allowed", "toString() radix argument must be between 2
  and 36", "Out of memory: BigInt generated from this operation is too big"; `SyntaxError` "Failed to parse String to
  BigInt".
- **Prelude.** The Error classes are compiled in when the source mentions `BigInt` or `bigint`, or has a literal like
  `10n`.
- **Related fix.** A `map` whose callback returns string literals (`v ? "1" : "0"`) now makes a `string[]`.
- Tests: `bench/bigint.ts` with `tests/bigint.rs` (M9-TS-360..368, bun-compared), expected values from
  `bench/gen_bigint_expected.ts`; M9-TS-369 is a unit test in `lib.rs`.
- **Not built:** see "BigInt" in [differences.md](differences.md).

**Inheritance.** `class B extends A` gives `B`'s struct the fields of `A` first (prefix layout), so a
`B` pointer is an `A` pointer and a subclass can be used wherever its base is expected. The compiler
sees the whole program, so a method call `a.m()` is resolved exactly: if one implementation is
reachable from the static class of `a` and its subclasses, it is a direct `Call`. Otherwise it is
`CallVirtual(table, args)`: codegen reads the struct id from the receiver's header and indexes
`Module::vtables[table]`, a local data table with one 8-byte function address per struct id, and
makes an indirect call. `super.m()` is always direct. `x instanceof C` is `InstanceOf(x, table)`:
null is false, otherwise the struct id indexes `Module::itables[table]`, one byte per struct id (1 for
`C` and its subclasses). A subclass must be declared after its base. Details and limits:
[iron-ts-classes.md](research/iron-ts-classes.md) section 14.

Not supported (each is a compile error naming the source line): decorators other than a bare `@property` on an instance field (no `@property` options, none on accessors), class expressions, generics, extending engine classes (`Component`),
throwing anything but an `Error` object, unsupported string methods (see above), `for...in`, object rest of unknown shapes, destructuring assignment to existing variables, `for await`, async generators, `any`/`JsValue` and anything
dynamic. `instanceof` works on the program's own classes only. Known
deviations from JavaScript (`null === null` of two nullable numbers, `==` as `===`, libm last-bit
differences) are listed in [iron-ts-spike.md](research/iron-ts-spike.md) sections 4 and 15.

## Symbol, iterators and generators

- **Well-known symbols.** `[Symbol.iterator]()` in a class is a method with the internal name `@@iterator`
  (`symbol_key` maps the computed key, `well_known` recognises `Symbol.<name>`). `obj[Symbol.iterator]()` is
  an ordinary call of it.
- **Symbol values** (prelude class `Symbol_`, [lower/prelude.rs](../tools/iron_ts/src/lower/prelude.rs)). `Symbol(desc)`
  makes an object with identity; `description` (null, not `undefined`, when none), `toString()`,
  `Symbol.for(key)` / `Symbol.keyFor(sym)` (null for an unregistered symbol), `typeof sym === "symbol"` and
  the `symbol` type work. Symbols are not usable as property or Map keys, and the other well-known symbols
  are not available as values.
- **Iterator protocol for classes.** `for...of`, array spread and `Array.from` over a value whose class has
  `@@iterator` call it, then `next()` until `done` (an `IteratorResult` object of the program's own types).
  Arrays, strings, `Map`, `Set` and typed arrays keep their loops.
- **Coroutines** ([iron_ts_rt/src/coro.rs](../tools/iron_ts_rt/src/coro.rs)). A coroutine runs a `() => void`
  closure on its own native stack (corosensei, 256 KB). The shadow stack (the linked list of frames living in
  each function's native frame) needs no change, because the GC roots are not on the native stack: the
  coroutine's first frame is relinked under the resumer's frame on every resume; when the coroutine object
  (header `HDR_CORO`) is marked, a suspended coroutine's frames are scanned by walking `saved_top` down to
  `bottom`; a coroutine swept while suspended gets `force_reset` (no unwinding: compiled frames have no
  destructors). Not available on WebAssembly (not a target in the next year).
- **Generators** ([lower/generators.rs](../tools/iron_ts/src/lower/generators.rs)) are a source rewrite
  (`generators::desugar`, before parsing). `function*` and generator methods become normal functions whose body
  is `return __genNew$K(() => { ... })`; every `yield x` becomes `__yield$K(x)`, and `yield* e;` (statement)
  a `for...of` loop of yields. For each yield type `K` the program mentions, the classes `Generator$K`,
  `Iterator$K`, `IteratorResult$K` and helper functions are generated as TypeScript text and put in front of
  the program. The annotations `Generator<T,R,N>`, `IterableIterator<T>`, `Iterable<T>`, `Iterator<T>` and
  `IteratorResult<T>` map to them. The intrinsics `__coNew`, `__coResume`, `__coYield<N>`, `__coValue<T>`,
  `__coError` and `__coFinish` are lowered in `generators.rs`. Closures, captures, `this`, `try/finally` and
  checked-return exceptions are the ordinary machinery: an exception thrown in the body is left pending by
  the coroutine and rethrown by `next()`. Struct names with `$` are made Rust-safe in the generated bindings.
- **`throw`, `return`, return values.** `generator.throw(e)` raises `e` at the suspended `yield`.
  `generator.return(v)` raises a marker error (`__GenReturn$K`) there, so `finally` blocks run; `catch`
  clauses inside generator bodies are rewritten to rethrow the marker. `return value` works for
  `Generator<T, T>` (the return type must equal the yield type, or be void); the `value` of a finished
  generator without one is the zero value.
- **Limits:** the return type must be written (untyped generators are an error); `yield*` only as a
  statement (not as a value); a `yield` inside another `yield`'s operand is an error;
  `IteratorResult.value` is the zero value (0, null) instead of `undefined` when done; `Iterable<T>`
  annotations accept generators only. See [differences.md](differences.md).
- Tests: `M9-TS-376..393` (`bench/generators.ts` with `tests/generators.rs`, bun-compared; the Rust coroutine
  test is `M9-TS-381`); `M9-TS-394..398` symbols, `M9-TS-399..407` `throw` / `return` / return values
  (`bench/symbols.ts`, `bench/generators2.ts`).

## Promise, async/await and timers

- **`Promise<T>`** is a prelude class `Promise$K` per value type text (`K` is the mangled type), written as
  TypeScript text in front of the program together with helper functions (`__fulfill$K`, `__then$K$V`,
  ...) by [lower/asyncs.rs](../tools/iron_ts/src/lower/asyncs.rs). `void` shares `number`'s class;
  `number`, `string` and `boolean` always exist; `T[]` too when `Promise.all` appears. A `Promise<T>` needs
  `T` written somewhere in the program, and awaiting needs the value's `Promise<T>` class to exist.
- **`async` rewrite** (`asyncs::desugar`, after the generator rewrite and before parsing). An `async`
  function, method or arrow becomes `__async$K((__p) => { ... })`, a coroutine closure; `return x` becomes
  `__asyncRet$K(__p, x)`, `await e` becomes `__await(e)` (the coroutine suspends and a reaction on the
  promise resumes it), and `new Promise<T>(f)` becomes `__newPromise$K(f)`.
- **Routing** ([lower/promise.rs](../tools/iron_ts/src/lower/promise.rs)). `__await`, `__asyncRet`,
  `.then(f)`, `.catch(f)`, `.finally(f)`, `Promise.resolve`, `Promise.reject`, `Promise.all` and
  `Promise.race` are typed calls into the generated helpers. The callback's return type picks the helper:
  `__then$K$V` for a value of type `V`, `__thenP` when it returns a promise (flattened), `__thenV` for void.
- **Microtasks and timers** ([iron_ts_rt/src/event.rs](../tools/iron_ts_rt/src/event.rs)). A FIFO microtask
  queue (`queueMicrotask`, promise reactions) and timers (`setTimeout`, `setInterval`, `clearTimeout`,
  `clearInterval`); no thread and no OS event loop. Timers run on virtual time and are at least 1 ms, like
  browsers. Queued closures and timers are GC roots. An exception escaping a queued job is dropped.
- **Host contract.** After every call into script the host calls `ts_rt_microtasks_run()`; once per frame it
  calls `ts_rt_timers_advance(now_ms)` with the game clock (a paused game pauses its timers);
  `ts_rt_run_loop()` runs both until nothing is left (tests and tools). `ts_rt_events_pending()` reports
  what is queued. Wiring this into `iron_app` is not done.
- **Tick order equals bun's:** returning a promise from `then` costs 3 ticks, `await` 1, and `finally`
  settles 2 jobs later; the tests compare the order of logged events with bun.
- **Also changed.** A function type `(x?: T) => void` makes the parameter nullable and a call may leave it
  out; a closure may take fewer parameters than its expected type. `IRON_TS_DUMP_SRC=1` prints the program
  after the rewrites; `IRON_TS_TRACE=1` prints function names during codegen; parse errors give the line
  (counting the prelude).
- **Limits:** `then(f, g)` needs both callbacks to return the same kind of result; `Promise.allSettled` results are `PromiseSettledResult<T>` objects with all of `status`/`value`/`reason`;
  no `for await`, async generators or top-level `await`; `Promise.reject(e)` is `Promise<number>` unless
  written `Promise.reject<T>(e)`; `resolve()` with no value only for `Promise<void>` / `Promise<number>`;
  `resolve(v)` takes a value, not a promise; unhandled rejections are silent; `setTimeout` takes no extra
  arguments; `Promise<Promise<T>>` is unsupported. See [differences.md](differences.md).
- **Callback literals.** A callback that returns a string literal is typed `string` (inferred callback return types
  widen literals).
- Tests: `M9-TS-408..427` (`bench/promises.ts` with `tests/promises.rs`, bun-compared; the source is
  generated from `bench/promises_src.ts` by `gen_promises_program.py`).

## Runtime and GC

[iron_ts_rt/src/lib.rs](../tools/iron_ts_rt/src/lib.rs) is an ordinary Rust crate the generated code
calls into. Its header comment is the long version.

- **Heap.** One heap per thread. Blocks up to 512 bytes come from 1 MiB chunks with size-class free
  lists; larger ones use the system allocator. Nothing moves, so a pointer Rust holds stays valid while
  the object is reachable. Every object is tracked with its size.
- **Headers.** A header is `mark (top bit) | heap id | kind`. The kind is the struct id + 1 for
  structs, or one of `HDR_ARRAY`, `HDR_ARRAY_PTR`, `HDR_BYTES`, `HDR_STR`, `HDR_CLOSURE`,
  `HDR_CLOSURE_ENV`, `HDR_CELL`, `HDR_CELL_PTR`, `HDR_MAP`, `HDR_BIGINT`. The heap id keeps a collector away from other
  threads' objects reached through a shared global slot.
- **Tracing.** Which slots of a struct are pointers is a mask the generated `ts_init` registers
  (`ts_rt_register_struct`). Pointer arrays, cells and script closure environments are traced.
- **Maps** ([map.rs](../tools/iron_ts_rt/src/map.rs)). `Map` and `Set` are one object kind, `HDR_MAP`
  (`MapObj`: key kind, whether values are pointers, counts, next sequence number), with two raw blocks
  outside the object: the entries in insertion order (`key`, `value`, `seq`, `hash`, `deleted`; deleted
  ones stay as tombstones) and an open-addressing index of entry numbers. A rebuild keeps the order and
  drops tombstones. A Set is a map with unused values. Equality is SameValueZero: `NaN` is a key, `-0` is
  `0`, strings by content (hash cached in the string's spare header word), objects, handles and booleans
  by identity. The collector traces the live entries' keys and values (by key kind and `val_ptr`) and
  frees the blocks with the object. **Iteration is live and exact:** every entry gets an increasing
  sequence number, and an iterator is the last number it gave out plus a position hint;
  `ts_rt_map_next(map, seq, hint)` finds the first entry with a larger number, so deleting, adding and
  rebuilding under a loop need no fix-up. Entry points: `ts_rt_map_new_gc`, `find`, `set`, `delete`,
  `clear`, `size`, `key_at`/`value_at`/`seq_at`, `next`, `keys_gc`/`values_gc` (snapshot arrays).
- **Roots.** (1) The shadow stack: a generated function with pointer locals links a `Frame` with one
  slot per pointer local plus spill slots (`ts_rt_frame_push` / `ts_rt_frame_pop`) and mirrors every
  pointer assignment into it. (2) Globals, registered by address (`ts_rt_add_global_root`). (3) Host
  roots, for Rust code that keeps script pointers across calls (`ts_rt_root_add` / `ts_rt_root_remove`).
  (4) Temp roots, for values a runtime routine holds while it collects (a new cell's value, a closure
  environment, the sort's merge buffer). Registers and Cranelift's own spill slots are never scanned.
- **The spill rule** ([frames.rs](../tools/iron_ts/src/frames.rs)). A pointer evaluated early and held
  while a later operand can allocate is copied to a frame slot, unless it is a local no later operand
  assigns, a global, or null (`spill_flags`, `is_stable`, `may_alloc`). Field reads are never trusted.
`CallVirtual` and `InstanceOf` are counted like calls (`CallVirtual` also allocates, as a call). The
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
  (see below), cells and closures, math (`sin`, `cos`, `pow`, `random` ...), collection
  control (`gc_collect`, `gc_stress`, `gc_enable`, `allocated`) and roots. Panics inside `extern "C"`
  abort the process, so fallible logic is tested through a Rust-level function.
- **Strings** ([string.rs](../tools/iron_ts_rt/src/string.rs); design in
  [iron-ts-strings.md](research/iron-ts-strings.md)). Stored as WTF-8 (UTF-8 plus the three-byte form
  of lone surrogates) in one GC object, `Str { header, byte_len, utf16_len, flags, cursor_unit,
  cursor_byte }` with the bytes inline (a leaf for the collector). Flags `ASCII` and `WELL_FORMED`: an
  ASCII string maps unit index to byte offset directly; for other text a one-entry cursor (last unit and
  byte position) makes a forward scan O(1) per step. What a script sees is JavaScript's exactly:
  `length`, indices, `slice` and `charCodeAt` count UTF-16 units, ordering is by code unit, lone
  surrogates are values, and a slice through a surrogate pair leaves lone surrogates that rejoin on
  concat. Host-side routines never collect: `ts_rt_str_new` (UTF-8/WTF-8 bytes), `ts_rt_str_from_utf16`,
  `ts_rt_str_concat`. Routines for generated code may collect first and are suffixed `_gc`
  (`concat_gc`, `char_at_gc`, `slice_gc`, `substring_gc`, `from_code_point_gc`). The rest return
  numbers or bools: `eq`, `compare`, `length`, `char_code_at`, `code_point_at`, `index_of`,
  `last_index_of`, `includes`, `starts_with`, `ends_with`. Rust code reads a string through `Str`:
  `as_bytes`, `as_str` (borrowed when well formed), `to_string_lossy` (lone surrogates become U+FFFD),
  `to_utf16`.
  Part two ([string_more.rs](../tools/iron_ts_rt/src/string_more.rs), slice 1b), generated code calls
  the `_gc` ones: `trim_gc`/`trim_start_gc`/`trim_end_gc` (JavaScript's white space set), `repeat_gc`
  (a negative or infinite count is a fault, a RangeError), `pad_start_gc`/`pad_end_gc`,
  `to_upper_gc`/`to_lower_gc` (Rust's Unicode tables with special casing and final sigma; lone
  surrogates pass through), `at_gc` (null for undefined), `concat_n_gc` (template literals: one
  allocation from an array of parts), `split_gc` (a new array of strings) and `join_gc` (a null element
  is empty). Number conversion: `ts_rt_num_to_str_gc` (`String(x)`: shortest round-trip digits in
  JavaScript's layout, exponent form from 1e21 and below 1e-7), `ts_rt_num_to_fixed_gc` (`toFixed`;
  exact ties round up as in JavaScript, where Rust rounds to even), `ts_rt_num_to_radix_gc` (integers
  only; a fraction in another radix is a fault for now), and the parsers `ts_rt_str_to_number`
  (`Number(s)`), `ts_rt_str_parse_float`, `ts_rt_str_parse_int(s, radix)`. Not done yet: `toString(radix)`
  of fractions, `toPrecision`/`toExponential`, `normalize`, `localeCompare`, RegExp and the engine boundary
  (slice 3). Slice 2 changed `ts_rt_str_eq` to be null-safe, `ts_rt_str_compare` to return `f64`, and
  added `ts_rt_str_from_char_code_gc`.
  Tests: [tools/iron_ts_rt/tests/strings.rs](../tools/iron_ts_rt/tests/strings.rs) (M9-TS-162..172) and
  [strings_more.rs](../tools/iron_ts_rt/tests/strings_more.rs) (M9-TS-173..177), both differential:
  `tests/gen_strings.ts` (run with bun) writes `strings_expected.json` and
  `strings_more_expected.json` from a 24-string corpus (part two adds number and parse cases) and the
  Rust tests repeat the same operations on the runtime and compare against bun's output.
- **Host side.** `link_anchor()` keeps the exports linked into an executable that only reaches them from
  generated code; `gc_stats()` reports collections and live bytes. Several modules in one process each
  need their own `struct_base`; registering a different table under a taken id is refused.

## Tests

[iron_ts_tests](../tools/iron_ts_tests) holds the TypeScript sources and the Rust that drives them.

| Path | What |
|---|---|
| `fixtures/sim.ts`, `sim.test.ts` | The zombie game's simulation (678 lines) and its original bun tests |
| `bench/*.ts` | One TypeScript module per language area (`closures`, `loops`, `switches`, `classes`, `inherit` ...); `bench.ts` and `sine_bench.ts` are the JavaScript sides of the benchmarks (run in QuickJS and bun, not compiled) |
| `bench/engine.ts` | Calls the `Demo` engine API; `build.rs` compiles it with the manifest of [tools/iron_demo_api](../tools/iron_demo_api/src/lib.rs) (a small world of entities written with `#[iron_api]`), whose shims the test binaries link |
| `bench/scene.ts` | Drives the real scene: `build.rs` compiles it (prefix `scn_`, base 15000) with `iron_scene`'s manifest (`iron_scene::api::scene_api::MANIFEST`) and the tests run it against a real `Scene` made current with `iron_scene::api::enter`; `iron_ts_tests::scene_anchor()` keeps the `Scene` shims linked |
| `build.rs` | Compiles `fixtures/sim.ts` and every `bench/<name>.ts` in a list of `(name, prefix, struct_base)` at build time, writes the `.obj` and `<name>_bindings.rs` to `OUT_DIR` and passes the objects to the linker |
| `src/lib.rs` | One `pub mod` per module that `include!`s its bindings, plus helpers (`root`, `push_f64s`, `gc_mode_from_env`, builders for the sim's state); `demo_api_anchor()` uses `iron_demo_api` so the linker keeps its shims |
| `tests/*.rs` | `sim_port` (13 ported cases), `gc`, and one file per language area |
| `src/bin/{bench,sine_bench}.rs` | The benchmarks: the same work in QuickJS, bun and native code, one checksum for all |

Each language-area test calls the generated functions through the bindings (`ts_<prefix><name>`) inside
`in_every_gc_mode`, which runs the body normally, with `ts_rt_gc_stress(true)` and with
`ts_rt_gc_enable(false)`. **Expected numbers come from running the same TypeScript in bun**, so the
tests check that native code computes what JavaScript does. Test IDs are `M9-TS-*` in
[TESTS.md](../TESTS.md) (section "M9.b"); the next free number is in `tools/iron_ts/TODO.md`.

Tests that use program globals (module variables such as a log array) take turns through a mutex, because the
globals are shared between test threads.

To add a test module:

1. Write `bench/<name>.ts` (exported functions only get symbols; run it in bun to get the expected numbers).
2. Add `("<name>", "<prefix>_", <struct_base>)` to the list in `build.rs`. Bases are 1000 apart (sim
   0, sine 1000, ... classes 11000, inherit 12000, props 13000, engine 14000, scene 15000, strings 16000, errors 17000); take the next free one.
3. Add a `pub mod` for it in `src/lib.rs` that includes `<name>_bindings.rs`.
4. Add `tests/<name>.rs`: call `ts_<prefix>init()` first, use `in_every_gc_mode`, and tag each test with
   its `// M9-TS-nnn` ID.
5. Add the IDs to `TESTS.md`, then run fmt, clippy, `cargo test --workspace` and the stress run.
