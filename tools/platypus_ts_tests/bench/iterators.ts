// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// The iterator protocol on classes: [Symbol.iterator](), next(), for...of, spread, Array.from.
// Every exported function without parameters returns a string; `gen_iterators_expected.ts` runs
// this file in bun.

interface Step {
  done: boolean;
  value: number;
}

class RangeIter {
  at: number;
  to: number;
  constructor(from: number, to: number) {
    this.at = from;
    this.to = to;
  }
  next(): Step {
    if (this.at >= this.to) return { done: true, value: 0 };
    const v = this.at;
    this.at = v + 1;
    return { done: false, value: v };
  }
}

class Range {
  from: number;
  to: number;
  constructor(from: number, to: number) {
    this.from = from;
    this.to = to;
  }
  [Symbol.iterator](): RangeIter {
    return new RangeIter(this.from, this.to);
  }
}

class Countdown {
  n: number;
  constructor(n: number) {
    this.n = n;
  }
  next(): Step {
    if (this.n <= 0) return { done: true, value: 0 };
    this.n = this.n - 1;
    return { done: false, value: this.n + 1 };
  }
  [Symbol.iterator](): Countdown {
    return this;
  }
}

interface WordStep {
  done: boolean;
  value: string;
}

class Words {
  words: string[];
  constructor(text: string) {
    this.words = text.split(" ");
  }
  [Symbol.iterator](): WordIter {
    return new WordIter(this.words);
  }
}

class WordIter {
  words: string[];
  i = 0;
  constructor(words: string[]) {
    this.words = words;
  }
  next(): WordStep {
    if (this.i >= this.words.length) return { done: true, value: "" };
    const w = this.words[this.i] as string;
    this.i = this.i + 1;
    return { done: false, value: w };
  }
}

export function forOf(): string {
  let out = "";
  for (const x of new Range(3, 8)) out += x + ",";
  out += "|";
  for (const x of new Range(0, 0)) out += "never" + x;
  for (const x of new Countdown(4)) out += x + ";";
  for (const w of new Words("the quick brown fox")) out += w.toUpperCase() + " ";
  return out;
}

export function breakContinue(): string {
  let out = "";
  for (const x of new Range(0, 20)) {
    if (x % 2 === 0) continue;
    if (x > 11) break;
    out += x + ",";
  }
  let n = 0;
  for (const a of new Range(0, 4)) for (const b of new Range(0, 3)) n += a * b;
  return out + "|" + n;
}

export function spreadAndFrom(): string {
  const a = [...new Range(1, 5)];
  const b = [0, ...new Countdown(3), 99];
  const c = Array.from(new Range(10, 13));
  const d = [...new Words("a b c")];
  return [a.join(), b.join(), c.join(), d.join("+"), String(a.length)].join("|");
}

export function manual(): string {
  const it = new Range(5, 8)[Symbol.iterator]();
  let out = "";
  let r = it.next();
  while (!r.done) {
    out += r.value + ";";
    r = it.next();
  }
  const again = new Range(1, 3)[Symbol.iterator]();
  return out + again.next().value + again.next().value + again.next().done;
}

export function exhausted(): string {
  const c = new Countdown(2);
  let first = 0;
  for (const x of c) first += x;
  let second = 0;
  for (const x of c) second += x;
  return first + "," + second;
}
