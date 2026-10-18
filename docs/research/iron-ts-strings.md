# iron_ts: strings (design note, for review)

Status: **proposal, no code yet** (a STOP item of `tools/iron_ts/TODO.md`). Written 2026-10-18 after
the decision in the engine-boundary note (section 7) that string-taking engine functions wait for it.

## 1. Decision from the review

Strings are **UTF-8 inside Iron**. The TypeScript source is read as it is (oxc gives UTF-8 text with
JavaScript's own escapes already decoded), and the compiled program and the engine both hold strings
as UTF-8 bytes. UTF-16 exists in JavaScript for historical reasons, and nothing in Iron needs it:
Rust, the OS file APIs, the renderer's text shaper (Parley) and the asset paths are all UTF-8. The cost
is bytes: a CJK character is 3 bytes in UTF-8 and 2 in UTF-16, which is fine.

What changes is not the storage but a handful of JavaScript behaviours that are defined in UTF-16
code units. Section 3 lists them and proposes what compiled code does instead. Please check section 3
and the questions in section 6; the rest is mechanics.

## 2. Representation

The runtime already has `Str { header, len, data }` (`iron_ts_rt`), a GC object that points at a
separate byte buffer. Proposal:

- **Always valid UTF-8.** Every string the program can make is valid UTF-8: literals are checked by
  the compiler, engine strings are `&str` (Rust guarantees it), and string operations keep it valid.
  The one JavaScript value that cannot be represented, a lone surrogate (`"\uD800"`), is a compile
  error in a literal and impossible at run time (section 3).
- **One allocation, not two.** `Str { header, len, hash_or_flags, bytes... }` with the bytes inline
  after the header, so a string is a single GC object (cheaper to allocate, no second raw buffer to
  free). The collector already treats `HDR_STR` as a leaf (no pointers inside).
- **Literals** live in the data section as static `Str` objects (header marked static), so using a
  literal allocates nothing and comparing a literal is a pointer or length-and-bytes compare.
- **Immutable.** Concatenation allocates a new string. A template literal `` `a${x}b` `` is lowered
  to one runtime call that measures, allocates once and writes the pieces (not a chain of `+`).
  Repeated `s += x` in a loop is quadratic, as in a naive engine; ropes are a later optimisation and
  the representation leaves room (a flags word).
- **Equality** is length then bytes. **Hashing** (for `Map<string, T>` and object keys later) is
  computed lazily and cached in the spare word.
- `string | null`: the null pointer, like every other pointer type.
- **String-literal unions** (`"idle" | "run"`) stay small integers as today (`Ty::Lit`); converting a
  union member to a `string` (to print it or pass it to the engine) uses the compiler's table of tags,
  which already exists.

## 3. Where JavaScript semantics differ, and what we do

JavaScript counts, indexes and orders strings in UTF-16 code units. In UTF-8 the same operations
need a choice. For text that is entirely ASCII (almost all game identifiers, paths and tags) every
choice below gives the same answer as JavaScript.

| Operation | JavaScript | Proposal |
|---|---|---|
| `s.length` | UTF-16 code units | **Code units are what scripts expect, so compute them:** the length in UTF-16 units, cached after the first call for non-ASCII strings. ASCII-only strings (flag set at creation) answer in O(1). |
| `s[i]`, `s.charAt(i)`, `s.charCodeAt(i)`, `s.slice(a, b)`, `s.substring`, `s.indexOf` results | by code unit index | Same units as `length`, so a program that works on ASCII works identically. For a non-ASCII string an index is converted to a byte offset by scanning (O(n)), unless the string is flagged ASCII (O(1)). An index that falls between the two halves of a surrogate pair (an emoji) is **a runtime fault**, never a lone surrogate. |
| `s.codePointAt(i)`, `for (const ch of s)`, `[...s]` | code points | Direct on UTF-8 (the natural unit). Recommended in the docs for non-ASCII text. |
| `<`, `>`, `localeCompare`-free sorting | UTF-16 code unit order | **UTF-8 byte order (code point order)**. It differs from JavaScript only when comparing an astral character (emoji) with a BMP character at U+E000 and above. We accept this difference and list it in `docs/differences.md`. |
| lone surrogates (`"\uD800"`, `String.fromCharCode(0xD800)`) | allowed | Not representable. A literal is a compile error; `fromCharCode` of a surrogate is a runtime fault; a lone surrogate from JSON or `JSON.parse` input is replaced with U+FFFD. |
| `toUpperCase`, `toLowerCase`, `normalize`, `localeCompare`, `Intl` | full Unicode, locale | Not in the first slice; ASCII-only case mapping first, full Unicode from Rust's tables later. |
| `number.toString()`, `parseFloat` | shortest round-trip | Same algorithm as the runtime's number printer (Rust's `ryu`-style shortest output); tested against bun for a table of values. |

Fault policy follows the engine-boundary note: native faults crash for now, with a message.

## 4. Across the engine boundary

- **Into the engine** (`GameObject.find(name: string)`, `PlayerPrefs.getInt(key)`): the shim takes a
  pointer and a length (`name_ptr: *const u8, name_len: usize`) and the macro builds a `&str`
  (checked once with `from_utf8_unchecked` because the compiler guarantees validity; a debug build
  checks). The `&str` is **borrowed for the call only**: the engine must copy it if it keeps it. The
  script string cannot move (the collector is non-moving) and cannot be collected during the call (it
  is rooted by the caller's frame), so no copy is needed to make the call.
- **Out of the engine** (`go.name`): the shim returns through an out pointer to a freshly allocated
  script string, using a non-collecting constructor (as the value-class results do). The Rust side
  returns `String` or `&str`; the macro copies it into the script heap.
- **Manifest and `.d.ts`:** a new type `string` (`"string"` in the JSON). It maps to TypeScript's
  `string` unchanged, and to one more pair of ABI parameters (pointer, length) or an out pointer.
- **QuickJS:** the generated bindings use rquickjs `String`, converted to and from `&str`. QuickJS
  strings are UTF-16 internally and convert on the way; this is the existing, unchanged cost of the
  optional QuickJS path.

## 5. What this unlocks, and slices

1. **Runtime:** inline-bytes `Str`, flags (ASCII), `length` in UTF-16 units, concat, template
   literals, equality, comparison, `indexOf`, `slice`, `charCodeAt`, `startsWith`/`endsWith`/
   `includes`, `trim`, `split`/`join` on arrays of strings, `toString` of numbers, `String(x)`.
   Differential tests against bun on a corpus of ASCII, BMP and astral strings.
2. **Compiler:** `string` locals, fields, arrays and parameters (the pointer type already exists);
   literals in the data section; the operations above lowered to runtime calls; `string | null`.
3. **Boundary:** the `string` manifest type, the macro and the compiler's argument and result
   conversion; then annotate the first string-taking engine functions (`GameObject.name`,
   `GameObject.find`) in `iron_scene`.
4. **Later:** `Map<string, T>` and string-keyed object records (hash cache), ropes, full Unicode case
   mapping, `RegExp` (a Rust regex engine behind a small API, the syntax subset to be decided).

Estimated size: slice 1 and 2 are the bulk (a few hundred lines of runtime and a day of tests);
slice 3 is small because the value-class and handle plumbing is already there.

## 6. Questions for you

1. **`length` and indices in UTF-16 units (proposed) or in code points / bytes?** Units keep ported
   JavaScript (bun libraries) working unchanged and match what TypeScript programmers expect; it
   costs an O(n) scan for non-ASCII strings. Code points or bytes would be simpler and faster but
   `"héllo".length` would differ from JavaScript.
2. **Ordering by code point (proposed) instead of UTF-16 code units.** One documented difference
   that only shows with emoji against U+E000..U+FFFF. OK?
3. **Lone surrogates are not representable** (compile error or fault, U+FFFD from JSON). OK?
4. **Engine strings are borrowed for the call** and the engine copies what it keeps. OK, or should the
   engine side receive an owned `String` always (simpler to use, one copy per call)?
5. Do you want **`RegExp`** in the first set of slices, or only after the language-features backlog?
