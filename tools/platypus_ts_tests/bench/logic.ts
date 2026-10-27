// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// `&&` and `||` give the operand (not a boolean) when both sides have one type; in conditions they
// stay tests. Every exported function without parameters returns a string; `gen_logic_expected.ts`
// runs this file in bun.

class Box {
  v: number;
  constructor(v: number) {
    this.v = v;
  }
}

function orNum(x: number, y: number): number {
  return x || y;
}

function andNum(x: number, y: number): number {
  return x && y;
}

function orStr(x: string, y: string): string {
  return x || y;
}

function andStr(x: string, y: string): string {
  return x && y;
}

function orNullable(x: number | undefined, y: number): number {
  return x || y;
}

function orObj(a: Box | null, b: Box): Box {
  return a || b;
}

function andObj(a: Box | null, b: Box): Box | null {
  return a && b;
}

export function numbers(): string {
  const xs = [0, 1, -1, 2.5, NaN, 7];
  let out = "";
  for (const x of xs) for (const y of xs) out += orNum(x, y) + "," + andNum(x, y) + ";";
  return out;
}

export function strings(): string {
  const xs = ["", "a", "bb", "0"];
  let out = "";
  for (const x of xs) for (const y of xs) out += "[" + orStr(x, y) + "|" + andStr(x, y) + "]";
  return out;
}

export function nullables(): string {
  const a = orNullable(undefined, 5) + "," + orNullable(0, 5) + "," + orNullable(3, 5) + "," + orNullable(NaN, 5);
  const b = new Box(1);
  const c = new Box(2);
  const x = orObj(null, c).v + "," + orObj(b, c).v + "," + (andObj(null, c) === null) + "," + (andObj(b, c) as Box).v;
  return a + "|" + x;
}

export function chains(): string {
  const a = 0;
  const b = 0;
  const c = 9;
  const d = 3;
  return [a || b || c, a || d || c, d && c && 1, d && a && c, a || (b && c), (a || d) && c].map((v) => String(v)).join(",");
}

export function conditions(): string {
  const o: Box | null = new Box(4);
  const n: Box | null = null;
  let out = "";
  if (o && o.v > 3) out += "A";
  if (n && n.v > 3) out += "B";
  if (!(n || false)) out += "C";
  if (o && !n) out += "D";
  const t = o && o.v > 100 ? "x" : "y";
  let i = 0;
  while (i < 10 && !(i === 5)) i++;
  const e = n === null || n.v > 2;
  out += t + i + e + (o && o.v > 3);
  return out;
}

function name(n: string | undefined): string {
  return n || "anonymous";
}

function count(n: number | undefined): number {
  return n || 10;
}

export function defaults(): string {
  return [name(undefined), name(""), name("zed"), String(count(undefined)), String(count(0)), String(count(4))].join(",");
}
