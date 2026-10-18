# iron_ts: a native JavaScript standard library (design note, for review)

Status: **proposal, no code yet** (a STOP item of `tools/iron_ts/TODO.md`, item 4 of "Now"). Written
2026-10-18 after the request: "add a bare minimum JS framework such as Intl / Date / String / Array /
Object and anything other common JS you can think of, perhaps RegExp / `.match`".

Status (2026-10-18): tier 1 is done, steps A-H: `throw`, `try`/`catch`/`finally` and the Error classes
(section 4.1; see "Exceptions" in [../iron-ts.md](../iron-ts.md)), `join` of number arrays, `typeof`, the
`Number`/`Math`/`Array` gaps, `console`, and `Object.*` with object spread and rest (see "Standard library
tier 1, steps E-H" there; tests M9-TS-199..205). Tier 2 is done: `Map`/`Set`/`WeakMap`/`WeakSet` (an insertion-ordered
hash table with live, exact iteration; tests M9-TS-206..214) and `JSON.stringify`/`JSON.parse` driven
by the static types, with JavaScriptCore's parse errors (tests M9-TS-215..221); see "Standard library
tier 2" in [../iron-ts.md](../iron-ts.md). Next: tier 3 (`RegExp`, `Date`, `Intl`).
The tables below describe the state before this work.

## 1. The rule

Same as for strings: **a script gets JavaScript's answer, exactly**. Whatever is not built is a compile
error that says what is missing, never a different result. Every piece gets differential tests against
bun (the harness in `tools/iron_ts_rt/tests/gen_strings.ts` is the pattern: bun writes the expected
data, a Rust test replays it on the runtime).

Because the program is closed (all of it is compiled together), the static types are known, which
makes several pieces cheaper than in an interpreter (typed `JSON`, typed `Object.keys`, may-throw
analysis).

## 2. What exists today

| Area | Built | Missing (all compile errors today) |
|---|---|---|
| Language | functions, closures, classes (inheritance, accessors, statics), enums, destructuring, `switch`, loops, string-literal unions, optional chaining on objects, template literals | `try`/`catch`/`throw`, `typeof`, generators, `async`/`await`, iterators protocol, getters on object literals, spread of objects, `label: ` edge cases |
| `String` | everything in `docs/research/iron-ts-strings.md` slices 1-3 | `replace`/`replaceAll`/`match`/`matchAll`/`search`/`split(regex)` (RegExp), `localeCompare`, `normalize`, `toLocale*`, `String.raw`, tagged templates |
| `Number` | `toString(radix)` (integers), `toFixed`, `Number()`, `parseInt`, `parseFloat` | `toPrecision`, `toExponential`, `toLocaleString`, `Number.isInteger`/`isFinite`/`isNaN`/`isSafeInteger`, `EPSILON`, `MAX_SAFE_INTEGER`, fractional `toString(radix)` |
| `Math` | `abs acos asin atan atan2 cbrt ceil clz32 cos exp floor fround hypot imul log log10 log2 pow random round sign sin sqrt tan trunc` (and `min`/`max`) | `sinh cosh tanh asinh acosh atanh expm1 log1p` |
| `Array` | `at concat every fill filter find findIndex forEach from includes indexOf isArray join lastIndexOf map of pop push reduce reverse shift slice some sort splice unshift` | `flat flatMap findLast findLastIndex reduceRight copyWithin keys values entries toSorted toReversed toSpliced with`, `join` of non-strings, default `sort()` (by string), `Array.from` with a map function on iterables |
| `Set` | an array underneath (`has` is linear) | a real hash set, `Map`, `WeakMap`, `WeakSet` |
| `Object` | object literals as structs | `keys values entries assign freeze fromEntries hasOwn`, object spread / rest, `in` |
| `JSON` | - | `stringify`, `parse` |
| `Date` | - | everything |
| `Intl` | - | everything |
| `RegExp` | - | everything |
| `Error` | - | `Error` types, `try`/`catch`, stack |
| `console` | - | `log warn error info debug` (to the engine's `Debug`) |
| Other | `Uint8Array` | other typed arrays, `Symbol`, `structuredClone`, `globalThis`, timers (`setTimeout`, in the engine's event loop item) |

## 3. Proposed order

Each tier is shippable on its own; later tiers need earlier ones. Sizes are rough (S: a day,
M: a few days, L: a week or more).

**Tier 1, language core (everything else needs it)**
1. `try` / `catch` / `finally` / `throw` and the `Error` classes (L). See the decision in section 4.1.
2. `typeof`, `Object.keys/values/entries/assign/freeze/fromEntries/hasOwn`, object spread and rest, `in`
   (M). `Object.keys(p)` on a struct is a constant array of its field names; on a record it is
   the record's keys.
3. `Array` gaps, `Number` statics, `Math` gaps (M, mostly mechanical, each differential-tested).
4. `console.*` to the engine log (S).

**Tier 2, data**
5. `Map` / `Set` / `WeakMap` / `WeakSet` as real hash tables with JavaScript's equality (SameValueZero)
   and insertion order, keys numbers, strings (cached hash) and objects (M).
6. `JSON.stringify` / `JSON.parse` (M). See 4.2.

**Tier 3, text and time**
7. `RegExp` (L). See 4.3. With it: `match`, `matchAll`, `replace`, `replaceAll`, `search`, `split(regex)`,
   `exec`, `test`, named groups, sticky / global / unicode flags, `lastIndex`.
8. `Date` (M). See 4.4.
9. `Intl` and the `toLocale*` methods, `localeCompare`, `normalize` (L). See 4.5.

**Tier 4, with other items**
10. `Symbol.iterator`, generators, `Promise` / `async` / timers (the open `async` item and the event loop).
11. Typed arrays, `structuredClone`, `globalThis`.

## 4. Decisions needed

### 4.1 Exceptions

The compiler has no unwinding (`unwind_info=false`), and the shadow stack of GC frames must be popped
when control leaves a function early. Options:

- **Checked returns, only where needed (recommended).** The program is closed, so the compiler computes
  which functions can throw (they contain `throw`, call a may-throw function, or call an engine
  function declared as throwing). A call to a may-throw function is followed by a branch on a
  per-thread "pending exception" word; the handler (or the function's return path) pops its frame and
  propagates. Functions that cannot throw pay nothing. Faults that are `RangeError`s today (`repeat(-1)`,
  `toFixed(101)`) become catchable exceptions raised by the runtime.
- Native unwinding (Cranelift unwind info plus the platform unwinder): zero cost on the happy path but
  platform-specific, and it would need frame cleanup tables for the shadow stack.
- Leave exceptions out: scripts in this style (`Result` values) work, but ported JavaScript libraries
  often `throw`.

### 4.2 `JSON`

The program's types are static, so `JSON.parse(text) as T` / `const x: T = JSON.parse(text)` can generate
a **decoder from `T`** (fields, arrays, nullable, unions of literals) that fails with a `SyntaxError`
or a type error where the data does not match, and `JSON.stringify(x)` can generate an **encoder from
the static type**. A value of type `unknown` / `any` would need a dynamic value type (`JsValue` exists
in the IR but nothing builds it). Proposal: typed decode / encode first, a dynamic `JsonValue` union
(null, boolean, number, string, array, object) as the type of an untyped `JSON.parse`. Both follow
JavaScript's output formatting exactly (`stringify` with indentation, key order, `toJSON`, lone
surrogates escaped as `\udXXX`).

### 4.3 `RegExp`

Options:
- **Use the `regress` crate (recommended):** an ECMAScript regular-expression engine in Rust (used by Boa),
  with JavaScript syntax: lookahead and lookbehind, backreferences, named groups, `u`/`i`/`m`/`s`/`y`
  flags, Unicode property escapes. It can match over UTF-16 and over ASCII. Strings that are ASCII (the
  `ASCII` flag is set) match with unit index = byte index; others are matched as a UTF-16 buffer
  (converted once per call, cached on the regexp for the same string).
- Write our own backtracking engine: more control, more risk, much more work.

Regex literals are validated at compile time (a bad pattern is a compile error) and compiled once at
program start. Replacement patterns (`$1`, `$<name>`, `$&`, `` $` ``, `$'`) and the `v` flag follow
the specification; features `regress` lacks stay compile errors.

### 4.4 `Date`

`Date` is a double (milliseconds since the epoch) plus a time zone. UTC arithmetic, `Date.now()`,
`toISOString`, `getTime`, the `getUTC*`/`setUTC*` family and ISO-format parsing are plain code. The
open questions are **local time** (`getHours`, `toString`, `new Date(2026, 9, 18)`) and the legacy
`Date.parse` formats. Proposal: local time from the operating system's time zone through the `jiff`
crate (the IANA database, correct historical offsets), and `Date.parse` implemented for the ISO
format and the two formats the specification requires (`toString` and `toUTCString` output),
anything else a `NaN` result with the specification's fallback being implementation-defined (bun
accepts many more; we would match bun for the formats in a test corpus and document the rest).

### 4.5 `Intl`, `localeCompare`, `normalize`, `toLocale*`

JavaScript's results come from ICU. Options:
- **ICU4X (`icu` crates) with a configurable data set (recommended).** `Intl.NumberFormat`,
  `DateTimeFormat`, `Collator` (so `localeCompare` and `sort` with a collator), `PluralRules`,
  `ListFormat`, `RelativeTimeFormat` and `normalize` all exist there. The data is compiled in as a
  blob; the game lists its locales in `package.json` (default `en`) and release packaging includes
  only those. A locale the game did not list falls back to the default, as JavaScript falls back
  from an unavailable locale. Differential tests compare against bun for the shipped locales.
- Hand-written `en-US` only: small, but anything else would silently differ, against the rule.

ICU4X and bun's ICU can differ across versions of the Unicode / CLDR data; the tests pin the
cases and record the version.

## 5. What I need from you

1. **Exceptions:** OK with checked returns in may-throw functions only (4.1)?
2. **RegExp:** OK with the `regress` crate (4.3)?
3. **Intl:** OK with ICU4X and a per-game locale list (4.5)?
4. **JSON:** OK with typed encode/decode plus a dynamic `JsonValue` (4.2)?
5. **Order:** tiers 1 to 3 as listed, or do you want `RegExp` and `Intl` ahead of `Map`/`Set`/`JSON`?
