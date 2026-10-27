// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// Writes tests/strings_expected.json: what JavaScript (bun) answers for string operations over a
// corpus of ASCII, Latin-1, BMP, CJK, astral and lone-surrogate strings. `tests/strings.rs` runs
// the same operations in the same order on the runtime's strings and compares. Regenerate with:
//   bun tests/gen_strings.ts
const corpus: string[] = [
  "", "a", "abc", "hello world", "héllo", "日本語", "😀", "a😀b", "😀😀", "\uD83D", "\uDE00",
  "x\uD83D", "\uDE00y", "a\uD83Db\uDE00c", "Ａ", "\u{1F600}", "", "\u{10000}",
  "ab\u{10FFFF}", "퟿", "ß", "日本語の本", "\uDBFF", "\uD800\uD800",
];
const units = (s: string) => Array.from({ length: s.length }, (_, i) => s.charCodeAt(i));
const rng = (a: number, b: number) => { const r: number[] = []; for (let i = a; i <= b; i++) r.push(i); return r; };
const num = (v: number | undefined) => (v === undefined || Number.isNaN(v) ? null : v);

const single: Record<string, unknown> = {};
single.length = corpus.map((s) => s.length);
single.charCodeAt = corpus.map((s) => rng(-1, s.length + 1).map((i) => num(s.charCodeAt(i))));
single.codePointAt = corpus.map((s) => rng(-1, s.length + 1).map((i) => num(s.codePointAt(i))));
single.charAt = corpus.map((s) => rng(-1, s.length + 1).map((i) => units(s.charAt(i))));
const grid = (s: string) => rng(-2, s.length + 1);
single.slice = corpus.map((s) => grid(s).flatMap((a) => grid(s).map((b) => units(s.slice(a, b)))));
single.slice1 = corpus.map((s) => grid(s).map((a) => units(s.slice(a))));
single.substring = corpus.map((s) => grid(s).flatMap((a) => grid(s).map((b) => units(s.substring(a, b)))));

const pair: Record<string, unknown[]> = {
  compare: [], eq: [], concat: [], indexOf: [], lastIndexOf: [], includes: [], startsWith: [], endsWith: [],
};
for (const s of corpus) {
  for (const t of corpus) {
    const r = rng(-1, s.length + 1);
    pair.compare.push(s < t ? -1 : s > t ? 1 : 0);
    pair.eq.push(s === t);
    pair.concat.push(units(s + t));
    pair.indexOf.push(r.map((f) => s.indexOf(t, f)));
    pair.lastIndexOf.push([s.lastIndexOf(t), ...r.map((f) => s.lastIndexOf(t, f))]);
    pair.includes.push(r.map((f) => s.includes(t, f)));
    pair.startsWith.push(r.map((f) => s.startsWith(t, f)));
    pair.endsWith.push([s.endsWith(t), ...r.map((f) => s.endsWith(t, f))]);
  }
}
await Bun.write(import.meta.dir + "/strings_expected.json", JSON.stringify({ corpus: corpus.map(units), single, pair }));
console.log("wrote", corpus.length, "strings");

// ---- part two: tests/strings_more_expected.json (trim, case, pad, repeat, at, split/join, numbers) ----
const enc = (v: number): number | string => (Number.isFinite(v) ? v : String(v));
const texts: string[] = [
  " a ", "\t\n a b  ", " x　", "﻿z", "", "   ", "ß", "ǅ", "ΑΣ", "Σ", "ΑΣ Β", "İ", "ﬃ",
  "ǆ", "Straße", "éÉ", "日本", "😀", "\uD83D ", " \uDE00", "ab", "a,b,,c", "ΑΣ ΑΣ", "İx",
];
const pads = ["", " ", "ab", "😀", "\uD83D"];
const seps = ["", " ", "a", "ab", "😀", "\uD83D", "\uDE00", "ΑΣ", ","];
const joiners = ["", ",", "😀", "\uD83D"];
const more: Record<string, unknown> = { texts: texts.map(units), pads: pads.map(units), seps: seps.map(units), joiners: joiners.map(units) };
more.trim = texts.map((s) => [units(s.trim()), units(s.trimStart()), units(s.trimEnd())]);
more.case = texts.map((s) => [units(s.toUpperCase()), units(s.toLowerCase())]);
more.at = texts.map((s) => rng(-3, s.length + 1).map((i) => { const c = s.at(i); return c === undefined ? null : units(c); }));
more.repeat = texts.map((s) => rng(0, 3).map((n) => units(s.repeat(n))));
more.pad = texts.map((s) => rng(0, s.length + 4).flatMap((t) => pads.flatMap((p) => [units(s.padStart(t, p)), units(s.padEnd(t, p))])));
more.split = texts.flatMap((s) => seps.map((p) => s.split(p).map(units)));
more.join = texts.flatMap((s) => seps.map((p) => joiners.map((j) => units(s.split(p).join(j)))));

const numbers = [
  0, -0, 1, -1, 0.1, 0.5, 1.5, 2.5, -2.5, 0.125, 1e21, 1e-7, 123456789012345680000, 5e-324,
  1.7976931348623157e308, NaN, Infinity, -Infinity, 0.000001, 123.456, -1.5e-10, 4.35, 2 ** 53, 1.005,
  8.345, 10.235, 1e20, 0.3, 100, 1234.5678, 0.00001234, 255, -255, 1e100, 0.1 + 0.2, 1 / 3, -1e-7,
];
more.numbers = numbers.map(enc);
more.numToString = numbers.map((x) => String(x));
more.numToFixed = numbers.map((x) => rng(0, 6).map((d) => x.toFixed(d)));
const ints = [0, 1, -1, 5, 255, -255, 1000, 123456789, 2 ** 40, -(2 ** 33)];
more.ints = ints;
more.numRadix = ints.map((x) => [2, 8, 16, 36].map((r) => x.toString(r)));

const parseCases = [
  "", " ", "42", " 42 ", "-42", "+42", "4.2e1", ".5", "5.", "-.5e-2", "1e", "1e+", "0x1F", "0X1f", "0b101",
  "0o17", "0x", "Infinity", "-Infinity", "+Infinity", "infinity", "12px", "px12", "1_000", "1 2", "٣", "1e1000",
  "-0", "0.0000001", "00012", "9007199254740993", " 12 ", "0x1F.8", "1.2.3", "--1", "NaN", "12e3abc",
  "  -7.5e2xyz", "0.1e-2", "e5", "+", "-", ".", "1.e2",
];
more.parseCases = parseCases.map(units);
more.number = parseCases.map((s) => enc(Number(s)));
more.parseFloat = parseCases.map((s) => enc(parseFloat(s)));
more.parseInt = parseCases.map((s) => [undefined, 10, 16, 2, 36, 37, 1, 0].map((r) => enc(parseInt(s, r as number))));
await Bun.write(import.meta.dir + "/strings_more_expected.json", JSON.stringify(more));
console.log("wrote part two");
