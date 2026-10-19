# iron_ts: `Intl.DateTimeFormat` and the Date strings (design note, for review)

Step I3 of the Intl work. Not built; this note exists because the choice below changes what the
compiled code prints and how much data ships. Steps I1 (collation, normalize, case) and I2
(NumberFormat) are done and compared against bun.

## What needs it

- `Intl.DateTimeFormat(locales, options).format(date)` (and `formatToParts`, `formatRange`,
  `resolvedOptions`).
- `Date.prototype.toLocaleString`, `toLocaleDateString`, `toLocaleTimeString`.
- `Date.prototype.toString` / `toTimeString`: `Mon Oct 19 2026 10:30:00 GMT-0400 (Eastern Daylight
  Time)`. Only the zone name needs locale data (the root-locale long specific name).
- `Date` inside `String(date)`, `` `${date}` `` and `date + ""`, which need `toString`.

## What ICU4X gives

`icu_datetime` 2.x formats from fixed "semantic" field sets: a date part (`D`, `MD`, `YMD`, `DE`,
`MDE`, `YMDE`, `E`, `M`, `YM`, `Y`), a time precision, a zone style, and one of three lengths (short,
medium, long). The patterns behind them are CLDR's `dateFormats`/`timeFormats`/glue patterns plus a
few derived ones. It has **no** `availableFormats` data and no skeleton matcher (the planned
`fieldbag` module is only a design document in 2.3). ECMA-402 components (`{ year: "numeric",
month: "2-digit", day: "2-digit" }`, `{ weekday: "long", hour: "numeric" }`, `fractionalSecondDigits`,
`dayPeriod`, `hour12`) are matched against `availableFormats` by ICU4C's pattern generator, so some
outputs cannot be reproduced from ICU4X data alone. For example `toLocaleDateString()` in en-US is
`10/19/2026` (skeleton `yMd`), while ICU4X's short date is `10/19/26`.

Time zone names (`SpecificLong`, `GenericLong`, `LocalizedOffset`, exemplar cities) are available
and need an IANA id; jiff has it.

## Options

**A. ICU4X field sets only (small, approximate).**
`dateStyle`/`timeStyle` are exact. Component bags map to the nearest field set; widths ICU4X does not
offer are patched in the pattern string (`yy` to `y`, `MM` to `M`) so the common cases
(`toLocaleDateString()`, `{ month: "long", day: "numeric" }`, `{ hour: "2-digit", minute: "2-digit" }`)
match. Rare combinations (weekday with year only, `dayPeriod`, `fractionalSecondDigits` in odd
places, non-Gregorian calendars) are `RangeError`s or approximations, listed in `docs/differences.md`.
Cost: about two days; no new data pipeline.

**B. Own skeleton matcher over generated CLDR data (exact, larger).**
A build step reads CLDR JSON (`cldr-dates-full`, from npm) for the game's locale list and writes
compact tables of `availableFormats`, `dateFormats`, `timeFormats`, glue patterns and
`intervalFormats`; a Rust port of UTS 35's skeleton matching (about 400 lines) picks and adjusts the
pattern; names (months, days, day periods, eras) come from ICU4X through `TypedDateTimeNames`, and
numbers through the I2 digits. Output equals ICU4C's for every locale in the list. Cost: about a week;
a data pipeline that needs network and a pinned CLDR version, and a few hundred KB per locale set.

**D. `rust_icu` (ICU4C bindings), as the runtime.**
The user asked for this. ICU4C is what JavaScriptCore/V8 use, so `udatpg` (the pattern generator) and
`availableFormats` give exactly bun's output for every option, and `Intl.NumberFormat`, the collator
and the rest would match without the ICU4X workarounds in I1/I2. Costs: it links the native ICU4C
library, which breaks the "one static object, no native dependencies" goal of the AOT backend (the
cross targets in M9.c, the release packaging, WASM/consoles): on Windows the system `icu.dll` has
unversioned symbol names while `rust_icu` expects versioned ones, so Windows builds need a vendored
ICU build; the data is about 30 MB (trimmable only by rebuilding ICU's data with a filter); it is two
ICU stacks if ICU4X stays for collation/number formatting; the crates trail the ICU releases and are
community-maintained. It also puts a C++ toolchain in every build of a game.
D as a **dev-time data generator** is cheaper: a script on a machine with ICU4C (or `cldr-json`)
dumps the `availableFormats`, `intervalFormats` and glue patterns for the game's locales into the
tables option B reads, so the runtime stays pure Rust.

**C. Both, in order.** Do A now (it unlocks `Date.toString`, `toLocale*String` and the common
`DateTimeFormat` uses), keep the design open for B, and do B only for the locales and options a
game turns out to need. Recommended.

## Recommendation

C, with B's data produced by the dev-time generator (D as a tool, not as a runtime dependency). Start with A in these slices, each bun-compared in five zones and several locales:

1. `Date.toString`, `toTimeString`, `String(date)`, template strings (zone names only; no locale
   data beyond the root names), then `date + x`.
2. `dateStyle` / `timeStyle`, `toLocaleDateString` / `toLocaleTimeString` / `toLocaleString` with no
   options, and `new Intl.DateTimeFormat(locale, options).format`.
3. Component bags for the common field sets, `hour12` / `hourCycle`, `timeZone`, `timeZoneName`.
4. `formatToParts`, `formatRange`, `resolvedOptions`.

## Question for review

Is A-then-maybe-B (option C) right, should the exact route (B, data from CLDR JSON or a `rust_icu` dev-time dump) be done up front, or do you want `rust_icu` as the runtime (D) despite the native dependency?
