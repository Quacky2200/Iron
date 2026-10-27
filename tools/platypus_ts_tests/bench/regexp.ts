// SPDX-License-Identifier: Apache-2.0

// Regular expressions in compiled code: literals, `new RegExp`, `test` and `exec`, `lastIndex`,
// flags, named groups, match objects. Every exported function without parameters returns a string;
// `gen_regexp_expected.ts` runs this file in bun and `tests/regexp.rs` compares.

export function basics(): string {
  const re = /(\d+)-(\d+)/;
  const m = re.exec("tel 555-1234 end");
  if (!m) return "no match";
  const miss = re.exec("none");
  return (
    m[0] + "|" + m[1] + "|" + m[2] + "|" + m.index + "|" + m.length + "|" + m.input + "|" +
    (miss === null) + "|" + re.test("x") + re.test("9-9") + "|" + re.source + "|" + re.flags + re.global
  );
}

export function globalLoop(): string {
  const re = /a(b)?/g;
  let out = "";
  const text = "ab a ab";
  let m = re.exec(text);
  while (m !== null) {
    out += m.index + ":" + m[0] + ":" + (m[1] ?? "-") + ":" + re.lastIndex + ";";
    m = re.exec(text);
  }
  return out + re.lastIndex;
}

export function namedGroups(): string {
  const m = /(?<year>\d{4})-(?<month>\d{2})/u.exec("on 2026-10-19");
  if (!m) return "no match";
  const g = m.groups!;
  const none = /x/.exec("x")!;
  return g.year + "/" + g.month + "/" + m[1] + "/" + (m.groups?.day ?? "none") + "/" + (none.groups === undefined);
}

export function flags(): string {
  const y = /a/y;
  y.lastIndex = 1;
  const first = y.test("ba");
  const afterFirst = y.lastIndex;
  const second = y.test("ba");
  return [
    String(/^b/m.test("a\nb")),
    String(/^b/.test("a\nb")),
    String(/a.b/s.test("a\nb")),
    String(/a.b/.test("a\nb")),
    String(/AB/i.test("ab")),
    String(/AB/.test("ab")),
    String(first),
    String(afterFirst),
    String(second),
    String(y.lastIndex),
  ].join(",");
}

export function unicode(): string {
  const smile = "😀b";
  return [
    String(/^.$/u.test("😀")),
    String(/^.$/.test("😀")),
    String(/\u{1F600}/u.test(smile)),
    String(/b/.exec(smile)!.index),
    String(/\p{L}+/u.exec("é1ß")![0]),
    String(/[\u{1F600}-\u{1F64F}]/u.test(smile)),
  ].join(",");
}

export function engineFeatures(): string {
  return [
    String(/(?<=\$)\d+/.exec("cost $42")![0]),
    String(/(?<!\$)\b\d+/.exec("$7 and 8")![0]),
    String(/(a)\1/.test("aa")),
    String(/(a)\1/.test("ab")),
    String(/^(?:a|ab)+$/.test("abab")),
    String(/a{2,3}?/.exec("aaaa")![0]),
    String(/(\w+)\s(\w+)/.exec("hello world")!.length),
    String(/[^\d\s]+/.exec("12 ab3")![0]),
    String(/\bfoo\b/.test("a foo b")),
  ].join(",");
}

export function dynamic(): string {
  const parts = "x+,gi,a|b,,a/b".split(",");
  const r1 = new RegExp(parts[0]!, "ig");
  const r2 = RegExp(parts[2]!);
  const r3 = new RegExp(r1, "m");
  const r4 = new RegExp("a/b");
  return [r1.flags, r1.source, String(r1.test("XXx")), String(r2.test("b")), r3.flags, r3.source, r4.source, String(r1)].join("|");
}

export function errors(): string {
  let out = "";
  const pats = ["(", "a", "a", "[", "a**", "(?<n>a)(?<n>b)", ")", "a{2,1}", "\\", "(?<=a)+", "[b-a]"];
  const fls = ["", "gg", "x", "", "", "", "", "", "", "", ""];
  for (let i = 0; i < pats.length; i++) {
    try {
      new RegExp(pats[i]!, fls[i]!);
      out += "ok;";
    } catch (e) {
      out += (e as Error).name + ":" + (e as Error).message + ";";
    }
  }
  return out;
}

export function matchAsArray(): string {
  const m = /(\d)(\d)(x)?/.exec("a12b")!;
  const [all, first, second, third] = m;
  const copy = [...m];
  let each = "";
  for (const part of m) each += (part ?? "_") + ".";
  return all + "|" + first + "|" + second + "|" + (third ?? "undef") + "|" + copy.length + "|" + each + "|" + m.map((p) => p ?? "?").join("+");
}
