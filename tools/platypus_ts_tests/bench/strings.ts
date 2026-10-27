// SPDX-License-Identifier: Apache-2.0

// Strings in compiled code. Every exported function takes no arguments and returns a number or a
// string; `gen_strings_expected.ts` runs the same file in bun and `tests/strings.rs` compares.

type Mode = "idle" | "run";

interface Person {
  name: string;
  nick: string | null;
}

export function length(): number {
  const s = "héllo 😀";
  return s.length * 100 + "".length;
}

export function concat(): string {
  let s = "a";
  for (let i = 0; i < 5; i++) s += i;
  s += 1.5;
  return s + "!" + true + null;
}

export function template(): string {
  const n = 3;
  const name = "x";
  return `${name}-${n * 2}-${n > 2}-${"tail"}`;
}

export function chain(): string {
  return "  Hello, World  ".trim().toUpperCase().slice(0, 5) + "|" + "ß".toUpperCase() + "|" + "ΑΣ".toLowerCase();
}

export function search(): number {
  const s = "the quick brown fox";
  return (
    s.indexOf("quick") * 1000 +
    s.lastIndexOf("o") * 10 +
    (s.includes("fox") ? 1 : 0) +
    (s.startsWith("the") ? 100000 : 0) +
    (s.endsWith("fox") ? 200000 : 0) +
    s.indexOf("zzz")
  );
}

export function compare(): number {
  let r = 0;
  if ("apple" < "banana") r += 1;
  if ("b" > "a") r += 2;
  if ("x" === "x") r += 4;
  if ("x" !== "y") r += 8;
  if ("\u{1F600}" < "Ａ") r += 16; // by code unit, not code point
  if (!("abc" < "ab")) r += 32;
  if ("abc" <= "abc" && "abc" >= "abc") r += 64;
  return r;
}

export function splitJoin(): string {
  return "a,b,c".split(",").join("+") + "/" + "xyz".split("").join("-");
}

export function parse(): number {
  return parseInt("42px") + parseFloat("3.5e1x") + Number("0x10") + Number(" 7 ") + parseInt("ff", 16);
}

export function numbers(): string {
  return (1234.5678).toFixed(2) + "|" + (0.1 + 0.2) + "|" + String(1e21) + "|" + (255).toString(16) + "|" + (2.5).toFixed(0);
}

export function unicode(): number {
  const s = "a😀b";
  return s.length * 100000 + s.charCodeAt(1) * 10 + s.slice(0, 2).length + s.codePointAt(1)!;
}

export function pad(): string {
  return "7".padStart(3, "0") + "|" + "ab".padEnd(5, "xy") + "|" + "ha".repeat(3);
}

export function chars(): string {
  return String.fromCharCode(72, 105) + "abc".charAt(1) + "abc".at(-1)! + String.fromCodePoint(128512);
}

export function loop(): number {
  const s = "héllo wörld 😀";
  let sum = 0;
  for (let i = 0; i < s.length; i++) sum += s.charCodeAt(i);
  return sum;
}

export function inArrays(): string {
  const parts: string[] = [];
  for (let i = 0; i < 5; i++) parts.push("p" + i);
  const has = parts.includes("p3") ? "yes" : "no";
  return parts.join(",") + "/" + has + "/" + parts.indexOf("p2");
}

export function sorting(): string {
  const words = ["pear", "apple", "fig", "banana", "cherry"];
  words.sort((a, b) => (a < b ? -1 : a > b ? 1 : 0));
  return words.join(" ");
}

function pick(): string {
  return "b";
}

export function switching(): number {
  switch (pick()) {
    case "a":
      return 1;
    case "b":
      return 2;
    default:
      return 3;
  }
}

function orNone(x: string | null): string {
  return x ?? "none";
}

export function nullable(): string {
  return orNone(null) + "," + orNone("some");
}

export function fields(): string {
  const p: Person = { name: "zed", nick: null };
  const q: Person = { name: "amy", nick: "a" };
  return p.name + p.name.length + (p.nick === null ? "-" : p.nick) + q.name + q.nick;
}

export function literalUnion(): string {
  const mode: Mode = "run";
  let state: Mode = "idle";
  if (mode === "run") state = "run";
  return "state:" + state + (state === "run" ? "!" : "?");
}

export function truthy(): number {
  const empty = "";
  const full = "x";
  return (empty ? 1 : 0) + (full ? 2 : 0);
}

export function nested(): string {
  const a = "ab";
  const b = "cd";
  return a.repeat(2).toUpperCase() + b.repeat(3).slice(1, 4) + (a + b).padStart(8, "-") + `${a.repeat(2)}|${b.repeat(2)}`;
}

export function manyStrings(): number {
  const keep: string[] = [];
  let total = 0;
  for (let i = 0; i < 300; i++) {
    const s = "item-" + i + "-" + "x".repeat(i % 7);
    total += s.length;
    if (i % 50 === 0) keep.push(s);
  }
  return total * 1000 + keep.join("").length;
}
