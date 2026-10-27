// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// Map, Set, WeakMap and WeakSet in compiled code. Every exported function without parameters returns
// a number or a string; `gen_collections_expected.ts` runs this file in bun and
// `tests/collections.rs` compares.

class Node {
  id: number;
  constructor(id: number) {
    this.id = id;
  }
}

export function mapBasics(): string {
  const m = new Map<string, number>();
  m.set("a", 1).set("b", 2).set("c", 3);
  m.set("a", 10);
  const out: string[] = [];
  out.push(String(m.size));
  out.push(String(m.get("a")));
  out.push(String(m.get("zz") ?? -1));
  out.push(String(m.has("b")) + String(m.has("q")));
  out.push(String(m.delete("b")) + String(m.delete("b")));
  out.push(Array.from(m.keys()).join(","));
  out.push([...m.values()].join(","));
  let t = "";
  for (const [k, v] of m) t += k + "=" + v + ";";
  m.forEach((v, k) => {
    t += k + v;
  });
  for (const k of m.keys()) t += k;
  for (const v of m.values()) t += v;
  out.push(t);
  m.clear();
  out.push(String(m.size));
  return out.join("|");
}

export function numberKeys(): string {
  const m = new Map<number, string>();
  m.set(NaN, "nan").set(0, "zero").set(-0, "negzero").set(1.5, "x");
  return m.size + ":" + m.get(NaN) + ":" + m.get(0) + ":" + m.get(1.5);
}

export function setBasics(): string {
  const s = new Set<number>([3, 1, 3, 2, 1]);
  const out: string[] = [];
  out.push(String(s.size));
  out.push([...s].join(","));
  s.add(9).add(1);
  out.push(Array.from(s).join(","));
  out.push(String(s.has(2)) + String(s.delete(2)) + String(s.has(2)));
  let t = "";
  for (const x of s) t += x + ";";
  s.forEach((v) => {
    t += v;
  });
  out.push(t);
  const words = new Set(["b", "a", "b"]);
  out.push([...words].join(""));
  return out.join("|");
}

export function objectKeys(): string {
  const a = new Node(1);
  const b = new Node(2);
  const m = new Map<Node, string>();
  m.set(a, "A");
  m.set(b, "B");
  m.set(a, "A2");
  const seen = new Set<Node>([a, b, a]);
  return m.size + ":" + m.get(a) + ":" + m.get(b) + ":" + seen.size + ":" + String(seen.has(new Node(1)));
}

export function mutateWhileIterating(): string {
  const s = new Set<number>([1, 2, 3, 4]);
  let out = "";
  for (const x of s) {
    out += x + ",";
    if (x === 1) {
      s.delete(2);
      s.add(5);
    }
    if (x === 3) s.delete(3);
    if (x === 5) s.add(6);
  }
  const m = new Map<number, number>();
  for (let i = 0; i < 100; i++) m.set(i, i * 2);
  let sum = 0;
  for (const [k, v] of m) {
    if (k % 2 === 0) m.delete(k + 1);
    sum += v;
  }
  return out + sum + ":" + m.size;
}

export function stringKeysAndArrayValues(): string {
  const m = new Map<string, number[]>();
  m.set("a" + "bc", [1]);
  m.get("abc")!.push(2);
  m.set("x", []);
  return m.get("abc")!.join("+") + ":" + m.size + ":" + String(m.has("ab" + "c"));
}

export function manyEntries(): number {
  const m = new Map<string, number>();
  for (let i = 0; i < 2000; i++) m.set("key" + i, i);
  for (let i = 0; i < 2000; i += 2) m.delete("key" + i);
  let total = 0;
  for (const [k, v] of m) total += v + k.length;
  for (let i = 0; i < 500; i++) m.set("again" + i, i);
  return total * 1000 + m.size;
}

export function weak(): string {
  const wm = new WeakMap<Node, number>();
  const n = new Node(1);
  wm.set(n, 5);
  const ws = new WeakSet<Node>();
  ws.add(n);
  return String(wm.get(n)) + String(wm.has(n)) + String(ws.has(n)) + String(ws.has(new Node(2)));
}
