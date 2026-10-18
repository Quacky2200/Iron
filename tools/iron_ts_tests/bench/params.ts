// Default, optional and rest parameters, and spread in array literals and calls.
export interface Item {
  v: number;
}
export type Kind = "a" | "b";

function make(v: number): Item {
  return { v };
}

function scale(x: number, k: number = 2): number {
  return x * k;
}

/** Sum of lo, lo + step, ... below hi. A default may use earlier parameters. */
function range(lo: number, hi: number = lo + 10, step: number = 1): number {
  let s = 0;
  let i = lo;
  while (i < hi) {
    s += i;
    i += step;
  }
  return s;
}

function describe(n: number, flag: boolean = false, kind: Kind = "b"): number {
  return n * 100 + (flag ? 10 : 0) + (kind === "b" ? 1 : 0);
}

function opt(x: number, y?: number): number {
  return y === undefined ? x : x + y;
}

function optItem(it?: Item): number {
  return it === undefined ? -1 : it.v;
}

function defaultObject(it: Item = { v: 5 }): number {
  return it.v;
}

/** The default calls a function that allocates. */
function defaultCall(n: number = make(3).v, m: number = n * 2): number {
  return n * 10 + m;
}

function sumAll(...xs: number[]): number {
  let s = 0;
  for (const x of xs) s += x;
  return s;
}

function head(first: number, ...rest: number[]): number {
  return first * 100 + rest.length * 10 + (rest.length > 0 ? rest[0]! : 0);
}

function sumItems(prefix: number, ...items: Item[]): number {
  let s = prefix;
  for (const it of items) s = s * 10 + it.v;
  return s;
}

function withDefaultAndRest(a: number, b: number = 5, ...more: number[]): number {
  return a * 1000 + b * 100 + more.length;
}

/** The rest parameter is a new array: writing to it does not change the caller's array. */
function overwrite(...xs: number[]): number {
  xs[0] = 99;
  return xs[0]!;
}

export function demoDefaults(): number {
  return scale(3) * 100 + scale(3, 5);
}

export function demoRange(): number {
  return range(0) * 10000 + range(5, 8) * 100 + range(0, 10, 3);
}

export function demoDescribe(): number {
  return describe(1) * 10000 + describe(2, true) * 100 + describe(3, false, "a") - 300;
}

export function demoOptional(): number {
  return opt(5) * 1000 + opt(5, 7) * 10 + optItem() + optItem(make(4)) * 100;
}

export function demoObjectDefault(): number {
  return defaultObject() * 100 + defaultObject({ v: 9 });
}

export function demoCallDefault(): number {
  return defaultCall() * 1000 + defaultCall(4) + defaultCall(1, 1) * 100000;
}

export function demoRest(): number {
  return sumAll() + sumAll(1) * 10 + sumAll(1, 2, 3) * 100 + head(1) * 1000 + head(1, 7, 8) * 100000;
}

export function demoMixed(): number {
  return withDefaultAndRest(1) * 100000 + withDefaultAndRest(1, 2) * 100 + withDefaultAndRest(1, 2, 3, 4);
}

export function demoSpreadCall(): number {
  const xs = [1, 2, 3];
  return sumAll(...xs) * 10000 + sumAll(0, ...xs, ...xs) * 100 + head(9, ...xs);
}

export function demoArraySpread(): number {
  const a = [1, 2];
  const b = [...a, 3, ...a];
  let s = 0;
  for (const x of b) s = s * 10 + x;
  const items = [make(4), ...[make(5), make(6)]];
  return s * 1000 + items.length * 100 + items[2]!.v;
}

export function demoNoAliasing(): number {
  const a = [1, 2];
  const r = overwrite(...a);
  const copy = [...a];
  copy[1] = 77;
  return r * 1000 + a[0]! * 100 + a[1]! + copy[1]! * 10000;
}

export function demoItems(): number {
  const items = [make(1), make(2)];
  return sumItems(7, ...items, make(3)) + sumItems(5);
}

/** Allocation in defaults, rest arrays and spreads, in a loop. */
export function allocating(n: number): number {
  let total = 0;
  for (let i = 0; i < n; i++) {
    const items = [make(i), make(i + 1)];
    total += sumItems(1, ...items, make(2), ...items) % 1000;
    total += defaultCall() + defaultObject();
  }
  return total;
}
