// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// WeakRef, WeakMap and WeakSet. Weak references keep their target here (a collector may), so
// the programs only look at what is still reachable. Every exported function without parameters
// returns a string; `gen_weak_expected.ts` runs this file in bun.

class Box {
  n: number;
  constructor(n: number) {
    this.n = n;
  }
}

class Big extends Box {
  extra: string;
  constructor(n: number, extra: string) {
    super(n);
    this.extra = extra;
  }
}

export function weakRef(): string {
  const b = new Box(5);
  const r = new WeakRef(b);
  const got = r.deref();
  b.n = 6;
  return got === undefined ? "gone" : "n=" + got.n + " same=" + (got === b);
}

export function typedWeakRef(): string {
  const r: WeakRef<Big> = new WeakRef(new Big(1, "x"));
  const g = r.deref();
  if (g === undefined) return "gone";
  g.n += 1;
  return g.n + g.extra;
}

export function weakRefsInAnArray(): string {
  const refs: WeakRef<Box>[] = [];
  const keep: Box[] = [];
  for (let i = 0; i < 50; i++) {
    const b = new Box(i);
    keep.push(b);
    refs.push(new WeakRef(b));
  }
  let sum = 0;
  for (const r of refs) {
    const b = r.deref();
    if (b !== undefined) sum += b.n;
  }
  return String(sum);
}

export function weakMap(): string {
  const m = new WeakMap<Box, string>();
  const a = new Box(1);
  const b = new Box(2);
  m.set(a, "first");
  m.set(b, "second");
  const out = [m.get(a), m.get(b), String(m.has(a)), String(m.has(new Box(1)))];
  m.delete(a);
  out.push(String(m.has(a)));
  return out.join(",");
}

export function weakSet(): string {
  const s = new WeakSet<Box>();
  const a = new Box(1);
  const b = new Box(2);
  s.add(a);
  s.add(a);
  s.add(b);
  const out = [String(s.has(a)), String(s.has(b)), String(s.has(new Box(1)))];
  s.delete(b);
  out.push(String(s.has(b)));
  return out.join(",");
}
