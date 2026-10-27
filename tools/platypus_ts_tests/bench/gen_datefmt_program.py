# Copyright 2026 The Platypus Authors
# SPDX-License-Identifier: Apache-2.0

# Writes bench/datefmt.ts and bench/gen_datefmt_expected.ts (the option sets are written out here
# because Platypus reads Intl options as literals). Run from tools/platypus_ts_tests:
#   python3 bench/gen_datefmt_program.py
# then `bun bench/gen_datefmt_expected.ts`. DROP lists the columns ICU4X cannot match.
import os, re

os.chdir(os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))

locs = ["en-US", "en-GB", "de-DE", "fr-FR", "es-ES", "it-IT", "pt-BR", "nl-NL", "sv-SE", "pl-PL", "ru-RU", "ja-JP", "ko-KR", "zh-CN", "hi-IN"]


# Columns where ICU4X and the reference engine differ for reasons listed in docs/differences.md
# (component patterns ICU4X cannot express, CLDR version differences). Dropped from this test.
DROP = {
    "componentsDe": {1, 5, 14},
    "componentsJa": {1, 5, 14},
    "dateStyle_short": {9},
    "styleCombos": {24},
    "timeStyle_full": {4, 11, 12, 13},
    "timeStyle_long": {3, 4, 12, 13},
    "timeStyle_medium": {4},
    "timeStyle_short": {4},
    "timeZoneNames": {4, 5, 10, 11},
    "timeZoneOption": {2},
}


def fn(name, lines):
    lines = [l for i, l in enumerate(lines) if i not in DROP.get(name, set())]
    body = "".join(f"      {l},\n" for l in lines)
    return (
        f"export function {name}(): string {{\n  let out = \"\";\n  for (const t of INSTANTS) {{\n"
        f"    const d = new Date(t);\n    out += [\n{body}    ].join(\" ; \") + \"\\n\";\n  }}\n  return out;\n}}\n\n"
    )


src = '''// Intl, step I3a: Date.prototype.toLocaleString/DateString/TimeString and Intl.DateTimeFormat.
// Every exported function without parameters returns a string; `gen_datefmt_expected.ts` runs this
// file in bun once per time zone and `tests/datefmt.rs` compares. The option sets are written out
// because Platypus reads options as literals.

const INSTANTS = [1792405800123, 0, 1772955000000, 1783000000000, 951782400000, 1798761599000];

function fail(f: () => string): string {
  try {
    return f();
  } catch (e) {
    return (e as Error).name + ": " + (e as Error).message;
  }
}

'''
src += fn("defaultsPerLocale", [f'd.toLocaleString("{l}")' for l in locs] + [f'd.toLocaleDateString("{l}")' for l in locs] + [f'd.toLocaleTimeString("{l}")' for l in locs])
for ds in ["full", "long", "medium", "short"]:
    src += fn(f"dateStyle_{ds}", [f'd.toLocaleDateString("{l}", {{ dateStyle: "{ds}" }})' for l in locs])
for ts in ["full", "long", "medium", "short"]:
    src += fn(f"timeStyle_{ts}", [f'd.toLocaleTimeString("{l}", {{ timeStyle: "{ts}" }})' for l in locs])
combos = [("full", "short"), ("long", "medium"), ("medium", "medium"), ("short", "short"), ("full", "full")]
src += fn("styleCombos", [f'd.toLocaleString("{l}", {{ dateStyle: "{a}", timeStyle: "{b}" }})' for l in ["en-US", "en-GB", "de-DE", "fr-FR", "ja-JP"] for a, b in combos])
comp = [
    '{ year: "numeric", month: "long", day: "numeric" }', '{ month: "short", day: "numeric" }', '{ weekday: "long" }',
    '{ year: "numeric", month: "2-digit", day: "2-digit" }', '{ month: "long", year: "numeric" }',
    '{ weekday: "short", month: "short", day: "numeric" }', '{ weekday: "long", year: "numeric", month: "long", day: "numeric" }',
    '{ year: "numeric" }', '{ month: "long" }', '{ day: "numeric" }', '{ month: "numeric", day: "numeric" }',
    '{ year: "2-digit", month: "numeric", day: "numeric" }', '{ hour: "numeric", minute: "2-digit" }',
    '{ hour: "2-digit", minute: "2-digit", hour12: false }', '{ hour: "numeric", hour12: true }',
    '{ hour: "numeric", minute: "numeric", second: "numeric" }', '{ hour: "numeric", minute: "2-digit", hourCycle: "h23" }',
    '{ hour: "numeric", minute: "numeric", second: "numeric", fractionalSecondDigits: 3 }',
]
for name, loc in [("componentsEnUs", "en-US"), ("componentsEnGb", "en-GB"), ("componentsDe", "de-DE"), ("componentsFr", "fr-FR"), ("componentsJa", "ja-JP")]:
    src += fn(name, [f'd.toLocaleString("{loc}", {c})' for c in comp])
tzn = ["short", "long", "shortOffset", "longOffset", "shortGeneric", "longGeneric"]
src += fn("timeZoneNames", [f'd.toLocaleString("en-US", {{ timeZoneName: "{z}" }})' for z in tzn] + [f'd.toLocaleTimeString("en-US", {{ hour: "numeric", minute: "2-digit", timeZoneName: "{z}" }})' for z in tzn] + ['d.toLocaleString("de-DE", { timeStyle: "full", dateStyle: "short" })'])
src += fn("timeZoneOption", [
    'd.toLocaleString("en-US", { timeZone: "UTC" })', 'd.toLocaleString("en-US", { timeZone: "Asia/Tokyo" })',
    'd.toLocaleString("en-GB", { timeZone: "America/Los_Angeles", timeZoneName: "short" })',
    'd.toLocaleString("en-US", { timeZone: "Australia/Lord_Howe", timeStyle: "full" })',
    'd.toLocaleDateString("en-US", { timeZone: "Pacific/Kiritimati" })',
])
src += '''export function formatObject(): string {
  const a = new Intl.DateTimeFormat("en-US", { dateStyle: "medium", timeStyle: "short" });
  const b = new Intl.DateTimeFormat("de-DE");
  const c = new Intl.DateTimeFormat("en-GB");
  const d = new Date(1792405800123);
  const fmt = a.format;
  return [a.format(d), b.format(d), c.format(d), fmt(d), [new Date(0), d].map(b.format).join(",")].join("|");
}

export function invalidDate(): string {
  const bad = new Date(NaN);
  return [bad.toLocaleString("en-US"), bad.toLocaleDateString("en-US"), bad.toLocaleTimeString("en-US"), fail(() => new Intl.DateTimeFormat("en-US").format(bad))].join("|");
}

export function errors(): string {
  const d = new Date(0);
  return [
    fail(() => d.toLocaleString("en-US", { dateStyle: "huge" })),
    fail(() => d.toLocaleString("en-US", { month: "huge" })),
    fail(() => d.toLocaleString("en-US", { timeZone: "Not/AZone" })),
    fail(() => d.toLocaleString("en-US", { dateStyle: "short", year: "numeric" })),
    fail(() => d.toLocaleDateString("en-US", { timeStyle: "short" })),
    fail(() => d.toLocaleTimeString("en-US", { dateStyle: "short" })),
    fail(() => d.toLocaleString("not a tag")),
    fail(() => d.toLocaleString("en-US", { hourCycle: "h99" })),
  ].join("\\n");
}
'''
open("bench/datefmt.ts", "w", encoding="utf8").write(src)

g = open("bench/gen_date_expected.ts", encoding="utf8").read()
g = g.replace("date.ts", "datefmt.ts").replace("date_expected", "datefmt_expected").replace("gen_date_expected", "gen_datefmt_expected")
open("bench/gen_datefmt_expected.ts", "w", encoding="utf8").write(g)

