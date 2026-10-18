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
