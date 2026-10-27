# platypus_ts: strings (design note, for review)

Status: **slices 1a, 1b, 2 and 3 are implemented; the strings item is done** (2026-10-18). RegExp, Intl,
localeCompare, normalize and the rest are in the standard-library item of tools/platypus_ts/TODO.md.
- **Slice 3, the engine boundary:** a `string` manifest type, `&str` borrowed in, `String` out through an
  out length, `GameObject.name`/`find`; tests M9-API-21..22, M9-TS-183..186; see section 20 of
  [platypus-ts-engine-boundary.md](platypus-ts-engine-boundary.md).
- **Slice 2** is compiler support (`lower/strings.rs`, `RtFn::Str`, static literals, `+`/templates/
  compare/methods; tests M9-TS-178..182); known gaps: `typeof`, `codePointAt` past the end, optional
  chaining on strings, object-literal fields without a declared type, `join` of non-string arrays, one
  allocation per `+` in a chain.
- **Slices 1a and 1b:** the runtime side in
`tools/platypus_ts_rt/src/string.rs` (WTF-8 `Str` with inline bytes, flags, one-entry cursor; concat, eq,
compare, length, charCodeAt/codePointAt, charAt, slice, substring, indexOf/lastIndexOf/includes/
startsWith/endsWith, fromCodePoint) with tests M9-TS-162..172, and `string_more.rs` (trim, repeat, pad,
case mapping, at, template concat, split/join, Number to string/toFixed/integer radix, Number/parseFloat/
parseInt) with tests M9-TS-173..177, all differential against bun. Not yet: toString(radix) of
fractions, toPrecision/toExponential, normalize, localeCompare, RegExp.

The rest of this note is the design as proposed. Written 2026-10-18, revised the same day after review.
The engine-boundary note (section 7) made string-taking engine functions wait for this.

## 1. Decisions from the review

1. **Strings are stored as UTF-8 bytes** (the Rust-native encoding: `&str`, `String`, file paths,
   Parley, wgpu labels). UTF-16 is only how JavaScript *describes* strings, so it is never stored.
2. **Observable behaviour is JavaScript's, exactly, with no runtime differences.** A script gives
   the answer a JavaScript developer expects: `"héllo".length`, `"😀".length === 2`, `s[i]`,
   `charCodeAt`, `slice` by UTF-16 index, `<` ordering by code unit, lone surrogates allowed. If
   compiled code and bun disagree, that is a bug, not a documented difference. The overhead is paid
   inside the string type, not by the developer (section 3).

The two decisions are compatible because of one storage rule: **WTF-8**. WTF-8 is UTF-8 plus the
three-byte encodings of lone surrogates, so every JavaScript string (any sequence of UTF-16 code
units) has exactly one byte form, and a string without lone surrogates (almost all of them) *is*
valid UTF-8, byte for byte. SpiderMonkey and Servo use the same idea.

## 2. Representation

The runtime has `Str { header, len, data }` (`platypus_ts_rt`), a GC object pointing at a separate
buffer. Proposal:

```
Str { header, byte_len, flags, utf16_len, cursor_utf16, cursor_byte, bytes... }
```

- **One allocation** with the bytes inline (no second raw buffer). `HDR_STR` is already a leaf for
  the collector. Literals are static `Str` objects in the data section (no allocation to use one).
- **flags**: `ASCII` (every byte < 0x80), `WELL_FORMED` (no lone surrogates, so valid UTF-8).
  Both are computed once when the string is made, in the same pass that measures `utf16_len`
  (vectorised for the ASCII check).
- **`utf16_len`** is stored, so `s.length` is a load, always.
- **A one-entry cursor** (`cursor_utf16` <-> `cursor_byte`) remembers the last index converted.
  `for (let i = 0; i < s.length; i++) s.charCodeAt(i)` and every forward scan is then O(1) per step
  even for non-ASCII text; a random access into a non-ASCII string costs one scan from the nearest of
  start, cursor and end. ASCII strings (identifiers, paths, tags: nearly everything) are always O(1).
- **Immutable.** `a + b` allocates a new string; a template literal `` `a${x}b` `` is one runtime call
  that measures, allocates once and writes the pieces. Concatenation that joins a trailing lone high
  surrogate to a leading lone low surrogate must re-encode the pair as one four-byte sequence (the
  WTF-8 rule), which keeps byte equality equal to JavaScript equality; that check is two byte tests
  at the seam. `s += x` in a loop is quadratic as in a naive engine; ropes are a later
  optimisation the flags word leaves room for.
- **Equality** is length then bytes (valid because WTF-8 is canonical). **Hash** is computed lazily
  and cached for `Map<string, T>` and string-keyed records.
- `string | null` is the null pointer; string-literal unions (`"idle" | "run"`) stay small integers
  (`Ty::Lit`) and convert to a `Str` through the compiler's tag table when a `string` is needed.

## 3. JavaScript semantics, operation by operation

All of these are as JavaScript, on UTF-16 code units. The implementation notes say how, without
storing UTF-16.

| Operation | Behaviour | How |
|---|---|---|
| `s.length` | UTF-16 units | the stored `utf16_len` |
| `s[i]`, `charAt`, `charCodeAt`, `at` | the unit at `i`, including half of a surrogate pair | convert `i` to a byte offset (O(1) ASCII, cursor otherwise); decode the code point there and return the high or low half as asked. No lone surrogate is created to answer. |
| `codePointAt`, `for...of`, `[...s]`, `Array.from(s)` | code points, a lone surrogate as itself | decode directly |
| `slice`, `substring`, `substr`, `split("")`, `padStart`... | by unit index, **may cut a pair in half** | convert both ends to byte offsets; a cut inside a four-byte sequence produces the WTF-8 three-byte form of the lone surrogate(s). Exactly JavaScript's result. |
| `indexOf`, `lastIndexOf`, `includes`, `startsWith`, `endsWith` | byte search (memchr/two-way) | results converted back to unit indices (O(1) ASCII; scan from the cursor otherwise) |
| `<`, `>`, `<=`, `>=`, `sort()` default | **UTF-16 code unit order** | byte comparison is wrong only when comparing a four-byte sequence (lead byte F0..F4) against a three-byte sequence of U+E000..U+FFFF (lead byte EE/EF). The comparison runs memcmp and, at the first differing byte, applies the one-line correction when either lead byte is F0..F4 and the other EE/EF. Exact, and free for ASCII. |
| `"\uD800"`, `String.fromCharCode(0xD800)`, `fromCodePoint` | lone surrogates are values | stored as the WTF-8 three-byte form |
| `toUpperCase`, `toLowerCase` | full Unicode, special casing | Rust's tables (`char::to_uppercase`, handles `ß` -> `SS`, final sigma rule needs care); lone surrogates pass through |
| `localeCompare`, `normalize`, `Intl.*` | locale / Unicode | `normalize` via a Rust normalisation crate; `localeCompare` and `Intl` are their own later item (needs a decision on ICU data size); until they exist, calling them is a compile error, never a silently different answer |
| `number.toString()`, `String(x)`, `parseFloat`, `Number(s)` | shortest round-trip decimal, exponent rules | the runtime's number printer; tested against bun on a table of edge values (`1e21`, `-0`, `0.1+0.2`, `5e-324`) |
| `JSON.stringify` / `parse` | escapes lone surrogates as `\udXXX` | follows the spec, later item |
| `RegExp` | JS syntax, UTF-16 index semantics, `u` and `v` flags | later item, own design note |

Tests: every row has a differential test that runs the same TypeScript in bun and in compiled code
over a corpus of ASCII, Latin-1, BMP, CJK, astral (emoji) and lone-surrogate strings, in the normal,
GC-stress and collection-off modes, like the language-area tests today.

## 4. Across the engine boundary

The engine speaks Rust: `&str` and `String` (valid UTF-8).

- **Into the engine** (`GameObject.find(name)`, `PlayerPrefs.getInt(key)`): the shim takes pointer and
  length. For a `WELL_FORMED` string (the normal case) the macro builds a **borrowed `&str` over the
  script's bytes, no copy**. The collector is non-moving and the caller's frame roots the string, so
  it stays valid for the call. The engine copies what it keeps. A string with a lone surrogate (not
  valid UTF-8) is converted to a temporary `String` with U+FFFD replacing the lone surrogates (what
  every JavaScript host does when it hands a string to a UTF-8 API); the common path stays free.
- **Out of the engine** (`go.name`): the Rust side returns `String` or `&str` (valid UTF-8, hence
  `WELL_FORMED`); the shim copies it into a new script string through the non-collecting constructor,
  measuring its UTF-16 length in the same pass.
- **Manifest and `.d.ts`:** a new type `string`; TypeScript sees `string`. The compiler passes the
  (pointer, length) pair or an out pointer, like value classes.
- **QuickJS:** unchanged cost, rquickjs `String` converted to and from `&str` (QuickJS stores
  Latin-1 or UTF-16 internally).

## 5. Slices

1. **Runtime:** the new `Str`, flags and length in one pass, cursor, `+`/template literals, equality,
   ordering, `charCodeAt`/`codePointAt`/`at`/index, `slice`/`substring`, `indexOf`/`includes`/
   `startsWith`/`endsWith`, `trim`, `split`/`join`, `repeat`, `padStart/End`, case mapping,
   `String(number)`, `parseInt/parseFloat/Number`, `fromCharCode/fromCodePoint`; differential tests.
2. **Compiler:** `string` locals, fields, arrays, parameters and returns; literals in the data
   section (a literal with a lone surrogate escape is fine now); the operations as runtime calls;
   `string | null`; `typeof`.
3. **Boundary:** the `string` manifest type, macro and compiler conversion; then `GameObject.name`
   and `GameObject.find` in `platypus_scene` (the first string-taking engine functions).
4. **Later, each its own item:** `Map<string, T>` and string-keyed records, ropes, `RegExp`,
   `JSON`, `normalize`, `localeCompare`/`Intl`.

## 6. What I still need from you

1. **OK with WTF-8 as the storage** (UTF-8 for everything except lone surrogates, which have the
   three-byte form)? It is the only way to keep JavaScript's exact results *and* hand Rust `&str`
   without copying.
2. **Calling `localeCompare` / `Intl` / `RegExp` before they exist is a compile error** (never a
   different answer). OK? Or should the first slices include a minimal `localeCompare` (code point
   order, which differs from real locale rules and would break "no differences")?
3. **Lone surrogates at the engine boundary become U+FFFD** (as every JS host does). OK?
