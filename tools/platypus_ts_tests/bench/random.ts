// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// The seedable generator and Unity's static `Random`, compiled against the native API and run in
// bun against js/engine/src/random.ts. The shared generator is reseeded first, so both runs see the
// same numbers.

function fmt(x: number): string {
  return x.toFixed(10);
}

export function doubles(): string {
  const r = new Rng(12345);
  const out: string[] = [];
  for (let i = 0; i < 6; i++) out.push(fmt(r.nextDouble()));
  return out.join(",");
}

export function integers(): string {
  const r = new Rng(7);
  const out: number[] = [];
  out.push(r.next(10));
  out.push(r.next(5, 9));
  out.push(r.next());
  out.push(r.next(3, 3));
  out.push(r.int(1, 7.9));
  out.push(r.int(-3.5, 3.5));
  return out.join(",");
}

export function floatsAndBools(): string {
  const r = new Rng(99);
  let s = fmt(r.float(-1, 1)) + "," + fmt(r.float(10, 20));
  for (let i = 0; i < 8; i++) s += r.bool() ? "T" : "F";
  for (let i = 0; i < 8; i++) s += r.bool(0.9) ? "T" : "F";
  return s;
}

export function seedsWrap(): string {
  const a = new Rng(4294967296 + 5);
  const b = new Rng(5);
  const c = new Rng(-1);
  return fmt(a.nextDouble()) + "," + fmt(b.nextDouble()) + "," + fmt(c.nextDouble());
}

export function twoGeneratorsAreIndependent(): string {
  const a = new Rng(1);
  const b = new Rng(1);
  const x = a.nextDouble();
  a.nextDouble();
  const y = b.nextDouble();
  return (x === y ? "same" : "different") + "," + fmt(b.nextDouble() - a.nextDouble());
}

export function shared(): string {
  Random.initState(2024);
  const out: string[] = [];
  out.push(fmt(Random.value));
  out.push(fmt(Random.range(5, 6)));
  out.push(String(Random.rangeInt(0, 100)));
  out.push(Random.bool() ? "T" : "F");
  out.push(Random.bool(0.1) ? "T" : "F");
  return out.join(",");
}

function s3(v: Vector3): string {
  return `(${v.x},${v.y},${v.z})`;
}

export function vectors(): string {
  const r = new Rng(31337);
  let s = "";
  for (let i = 0; i < 5; i++) {
    const c = r.insideUnitCircle();
    s += `(${c.x},${c.y})${s3(r.onUnitSphere())}${s3(r.insideUnitSphere())}`;
    const q = r.rotation();
    s += `(${q.x},${q.y},${q.z},${q.w});`;
  }
  Random.initState(77);
  const a = Random.insideUnitCircle;
  const b = Random.insideUnitSphere;
  const c = Random.onUnitSphere;
  const d = Random.rotation;
  return `${s}${a.x},${a.y}|${s3(b)}|${s3(c)}|${d.x},${d.w}`;
}
