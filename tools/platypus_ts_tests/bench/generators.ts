// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// Generators: function*, yield, yield*, methods, [Symbol.iterator], next(sent), try/finally,
// exceptions, interleaving. Every exported function without parameters returns a string;
// `gen_generators_expected.ts` runs this file in bun.

function* count(n: number): Generator<number> {
  for (let i = 0; i < n; i++) yield i;
}

function* fib(): Generator<number> {
  let a = 0;
  let b = 1;
  while (true) {
    yield a;
    const t = a + b;
    a = b;
    b = t;
  }
}

function* words(): Generator<string> {
  yield "alpha";
  yield "beta";
  yield "gamma";
}

class Box {
  v: number;
  constructor(v: number) {
    this.v = v;
  }
}

function* boxes(n: number): Generator<Box> {
  for (let i = 1; i <= n; i++) yield new Box(i * i);
}

function* both(): Generator<number> {
  yield 100;
  yield* count(3);
  yield* [7, 8];
  yield 200;
}

class Tree {
  value: number;
  left: Tree | null;
  right: Tree | null;
  constructor(value: number, left: Tree | null, right: Tree | null) {
    this.value = value;
    this.left = left;
    this.right = right;
  }
  *[Symbol.iterator](): Generator<number> {
    if (this.left !== null) yield* this.left;
    yield this.value;
    if (this.right !== null) yield* this.right;
  }
}

class Counter {
  step: number;
  constructor(step: number) {
    this.step = step;
  }
  *upTo(limit: number): Generator<number> {
    for (let i = 0; i < limit; i += this.step) yield i;
  }
}

export function basics(): string {
  const out: string[] = [];
  for (const x of count(5)) out.push(String(x));
  const it = count(3);
  const a = it.next();
  const b = it.next();
  const c = it.next();
  const d = it.next();
  const e = it.next();
  out.push([a.value, b.value, c.value].join("/") + String(a.done) + String(c.done) + String(d.done) + String(e.done));
  return out.join(",");
}

export function infinite(): string {
  const out: number[] = [];
  for (const x of fib()) {
    if (x > 200) break;
    out.push(x);
  }
  return out.join(",");
}

export function strings(): string {
  let out = "";
  for (const w of words()) out += w.toUpperCase() + ";";
  return out + [...words()].join("+") + Array.from(words()).length;
}

export function objects(): string {
  let sum = 0;
  for (const b of boxes(5)) sum += b.v;
  return String(sum) + [...boxes(3)].map((b) => b.v).join(",");
}

export function delegation(): string {
  return [...both()].join(",");
}

export function methodsAndTrees(): string {
  const t = new Tree(5, new Tree(2, new Tree(1, null, null), new Tree(3, null, null)), new Tree(8, null, new Tree(9, null, null)));
  const inOrder: number[] = [];
  for (const v of t) inOrder.push(v);
  const c = new Counter(3);
  return inOrder.join(",") + "|" + [...c.upTo(10)].join(",") + "|" + [...t].length;
}

export function interleaved(): string {
  const a = count(4);
  const b = words();
  let out = "";
  out += a.next().value + b.next().value;
  out += a.next().value + b.next().value;
  out += a.next().value;
  out += b.next().value;
  out += a.next().done + "" + b.next().done;
  return out;
}

export function earlyReturn(): string {
  const it = count(10);
  const first = it.next().value;
  const r = it.return();
  const after = it.next();
  return first + "," + r.done + "," + after.done;
}

function* withFinally(log: string[]): Generator<number> {
  try {
    log.push("start");
    yield 1;
    yield 2;
    log.push("end");
  } finally {
    log.push("finally");
  }
}

function* thrower(): Generator<number> {
  yield 1;
  throw new Error("boom");
}

export function tryAndThrow(): string {
  const log: string[] = [];
  for (const x of withFinally(log)) log.push("got" + x);
  let caught = "";
  try {
    for (const x of thrower()) caught += x;
  } catch (e) {
    caught += "|" + (e as Error).message;
  }
  const t2 = thrower();
  t2.next();
  let second = "";
  try {
    t2.next();
  } catch (e) {
    second = (e as Error).message;
  }
  return log.join(",") + "/" + caught + "/" + second + "/" + t2.next().done;
}

function* echo(): Generator<number, void, number | undefined> {
  let got = yield 1;
  while (got !== undefined && got < 100) {
    got = yield got * 2;
  }
}

export function sending(): string {
  const it = echo();
  const a = it.next().value;
  const b = it.next(5).value;
  const c = it.next(7).value;
  const d = it.next(500).done;
  return a + "," + b + "," + c + "," + d;
}

export function manyGenerators(): string {
  let total = 0;
  for (let i = 0; i < 300; i++) {
    const g = count(5);
    g.next();
    g.next();
    total += g.next().value;
    if (i % 3 === 0) {
      for (const x of count(3)) total += x;
    }
  }
  const keep: Generator<number>[] = [];
  for (let i = 0; i < 20; i++) keep.push(count(i + 3));
  let sum = 0;
  for (const g of keep) {
    g.next();
    sum += g.next().value;
  }
  return total + "," + sum;
}
