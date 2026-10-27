// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// Symbol values: Symbol(), description, toString, identity, Symbol.for, Symbol.keyFor, typeof.
// Every exported function without parameters returns a string; `gen_symbols_expected.ts` runs
// this file in bun.

export function identity(): string {
  const a = Symbol("id");
  const b = Symbol("id");
  const c = a;
  return [a === b, a === c, a !== b].join(",");
}

export function describe(): string {
  const a = Symbol("tag");
  return a.description + "|" + a.toString() + "|" + String(a.description === "tag");
}

export function registry(): string {
  const a = Symbol.for("app.key");
  const b = Symbol.for("app.key");
  const c = Symbol("app.key");
  return String(a === b) + "," + String(a === c) + "," + Symbol.keyFor(a) + "," + String(Symbol.keyFor(c) === undefined);
}

export function typeofSymbol(): string {
  const a = Symbol("x");
  return typeof a + "," + typeof 1 + "," + (typeof a === "symbol");
}

export function inClasses(): string {
  const kind: symbol = Symbol("kind");
  const items: symbol[] = [kind, Symbol("other"), Symbol("k")];
  let n = 0;
  for (const s of items) {
    if (s === kind) n += 1;
  }
  return n + "," + items.length + "," + items[2].toString();
}
