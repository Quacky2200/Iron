// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// The standard library in compiled code: `typeof`, `Number` statics, `Math` extras, `Array`
// methods. Every exported function without parameters returns a number or a string;
// `gen_stdlib_expected.ts` runs this file in bun and `tests/stdlib.rs` compares.

export function typeofs(): string {
  const n = 1;
  const s = "x";
  const b = true;
  const f = () => 1;
  const o = { a: 1 };
  const arr = [1];
  const u: number | undefined = undefined;
  const m: number | undefined = 5;
  const kinds = [typeof n, typeof s, typeof b, typeof f, typeof o, typeof arr, typeof u, typeof m];
  // @ts-ignore: an undeclared name is "undefined"
  kinds.push(typeof notDeclared);
  let flags = "";
  if (typeof n === "number") flags += "n";
  if (typeof s !== "number") flags += "s";
  if (typeof m === "undefined") flags += "X";
  return kinds.join(",") + "|" + flags;
}

export function numberStatics(): string {
  const parts = [
    String(Number.isInteger(5)),
    String(Number.isInteger(5.5)),
    String(Number.isNaN(NaN)),
    String(Number.isNaN(1)),
    String(Number.isFinite(1 / 0)),
    String(Number.isFinite(3)),
    String(Number.isSafeInteger(2 ** 53)),
    String(Number.isSafeInteger(2 ** 53 - 1)),
    String(isNaN(Number("abc"))),
    String(isFinite(Number("12"))),
    String(Number.EPSILON),
    String(Number.MAX_SAFE_INTEGER),
    String(Number.MIN_VALUE),
    String(Number.MAX_VALUE),
    String(Number.parseFloat("2.5kg")),
    String(Number.parseInt("12", 8)),
  ];
  return parts.join(",");
}

export function mathExtras(): string {
  const xs = [
    Math.sinh(1),
    Math.cosh(1),
    Math.tanh(0.5),
    Math.asinh(1),
    Math.acosh(2),
    Math.atanh(0.5),
    Math.expm1(0.00001),
    Math.log1p(0.00001),
  ];
  return xs.map((x) => x.toFixed(10)).join(",");
}

export function arrayGaps(): string {
  const xs = [3, 1, 2, 10, 9];
  const out: string[] = [];
  out.push(xs.toSorted().join(","));
  out.push(xs.toSorted((a, b) => a - b).join(","));
  out.push(xs.toReversed().join(","));
  out.push(xs.join(","));
  out.push(String(xs.findLast((x) => x > 2)));
  out.push(String(xs.findLastIndex((x) => x > 2)));
  out.push(String(xs.reduceRight((acc, x) => acc * 10 + x, 0)));
  out.push([[1, 2], [3], []].flat().join(","));
  out.push([1, 2, 3].flatMap((x) => [x, x * 10]).join(","));
  const words = ["pear", "apple", "fig"];
  out.push(words.sort().join(" "));
  const nums = [10, 9, 1, 100];
  out.push(nums.sort().join(","));
  let t = "";
  for (const [i, x] of xs.entries()) t += i + ":" + x + ";";
  for (const i of xs.keys()) t += i;
  for (const x of xs.values()) t += x;
  out.push(t);
  return out.join("|");
}

export function logs(): number {
  console.log("a", 1, true);
  console.warn("w:" + 2);
  console.error("e", null);
  console.log();
  return 4;
}

interface Point {
  x: number;
  y: number;
}

class Particle {
  x = 1;
  y = 2;
  name = "p";
  #secret = 3;
  move(): number {
    return this.#secret;
  }
}

export function objects(): string {
  const p: Point = { x: 1, y: 2 };
  const out: string[] = [];
  out.push(Object.keys(p).join(","));
  out.push(Object.values(p).join(","));
  let t = "";
  for (const [k, v] of Object.entries(p)) t += k + "=" + v + ";";
  out.push(t);
  const q = { ...p, z: 3 };
  out.push(Object.keys(q).join(",") + ":" + q.z);
  const r = { ...p, x: 10 };
  out.push(r.x + "," + r.y);
  const back = { a: 1, ...p, b: 2 };
  out.push(Object.keys(back).join(","));
  const { x, ...rest } = q;
  out.push(x + "|" + Object.keys(rest).join(",") + "|" + rest.z);
  const target: Point = { x: 0, y: 0 };
  Object.assign(target, { y: 5 }, { x: 7 });
  out.push(target.x + "," + target.y);
  out.push(String("x" in p) + String("w" in p) + String(Object.hasOwn(p, "y")));
  const [first, ...others] = [1, 2, 3];
  out.push(first + ":" + others.join("+"));
  const frozen = Object.freeze({ a: 1 });
  out.push(String(frozen.a));
  const pa = new Particle();
  out.push(Object.keys(pa).join(",") + "|" + String("move" in pa) + String("#secret" in pa) + pa.move());
  return out.join(" | ");
}
