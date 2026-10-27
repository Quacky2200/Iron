// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// Destructuring: object and array patterns in declarations, parameters, loops and callbacks,
// nested patterns and defaults.
export interface Vec {
  x: number;
  y: number;
}
export interface Box {
  min: Vec;
  max: Vec;
  label?: number;
  next?: Box;
}

export function makeBox(a: number, b: number, c: number, d: number): Box {
  return { min: { x: a, y: b }, max: { x: c, y: d } };
}

export function dot(a: Vec, b: Vec): number {
  const { x: ax, y: ay } = a;
  const { x: bx, y: by } = b;
  return ax * bx + ay * by;
}

/** Nested patterns: `min: { x, y }` and a plain `max`. */
export function nested(b: Box): number {
  const {
    min: { x, y },
    max,
  } = b;
  return x + y + max.x * 100 + max.y * 1000;
}

/** A parameter that is a pattern. */
export function parameterPattern({ x, y }: Vec): number {
  return x * 10 + y;
}

/** A default replaces a missing (null) property; a present one wins. */
export function defaults(b: Box): number {
  const { label = 7 } = b;
  return label;
}

/** An optional object property whose pattern default is an object. */
export function nestedDefault(b: Box): number {
  const { next = makeBox(1, 2, 3, 4) } = b;
  return next.max.y;
}

/** Array patterns: `c` has a default for a short array. */
export function arrayPattern(xs: number[]): number {
  const [a, b, c = 5] = xs;
  return a * 100 + b * 10 + c;
}

/** Holes are skipped. */
export function arrayHole(xs: number[]): number {
  const [, second, , fourth] = xs;
  return second * 10 + fourth;
}

/** `for (const { x, y } of items)`. */
export function forOfObjects(items: Vec[]): number {
  let s = 0;
  for (const { x, y } of items) s += x * y;
  return s;
}

/** `for (const [a, b] of rows)`. */
export function forOfArrays(rows: number[][]): number {
  let s = 0;
  for (const [a, b] of rows) s += a - b;
  return s;
}

/** Patterns as callback parameters of inlined array methods. */
export function callbackPatterns(ps: Vec[]): number {
  let s = 0;
  ps.forEach(({ x, y }) => {
    s += x + y * 2;
  });
  const xs = ps.map(({ x }) => x);
  const big = ps.filter(({ x, y }) => x + y > 5);
  return s * 1000 + xs.length * 100 + big.length;
}

/** A closure with a pattern parameter, and a closure capturing a destructured name. */
export function closures(): number {
  const diff = ({ x, y }: Vec) => x - y;
  const { x, y } = { x: 10, y: 3 };
  const add = (n: number) => n + x + y;
  return diff({ x: 9, y: 4 }) * 100 + add(1);
}

/** The captured destructured variable is a cell: the closure sees later writes. */
export function capturedPatternVariable(): number {
  let [a, b] = [1, 2];
  const bump = () => {
    a += 10;
  };
  bump();
  b += a;
  return a * 100 + b;
}

function make(i: number): Vec {
  return { x: i, y: i * 2 };
}

/** Allocation in the initialiser of a pattern, inside a loop. */
export function allocating(n: number): number {
  let s = 0;
  for (let i = 0; i < n; i++) {
    const { x, y } = make(i);
    const [p, q] = [x + 1, y + 1];
    s += p * q - x * y;
  }
  return s;
}

// Zero-argument drivers, so the native tests need no objects built on the Rust side.
export function demoDot(): number {
  return dot({ x: 1, y: 2 }, { x: 3, y: 4 });
}
export function demoNested(): number {
  return nested(makeBox(1, 2, 3, 4));
}
export function demoParam(): number {
  return parameterPattern({ x: 4, y: 2 });
}
export function demoDefaults(): number {
  const a = makeBox(1, 2, 3, 4);
  const b = makeBox(1, 2, 3, 4);
  b.label = 9;
  return defaults(a) * 100 + defaults(b);
}
export function demoNestedDefault(): number {
  const a = makeBox(1, 2, 3, 4);
  const b = makeBox(1, 2, 3, 4);
  b.next = makeBox(0, 0, 0, 50);
  return nestedDefault(a) * 100 + nestedDefault(b);
}
export function demoArrays(): number {
  return arrayPattern([1, 2, 3]) * 1000 + arrayPattern([1, 2]);
}
export function demoHole(): number {
  return arrayHole([1, 2, 3, 4]);
}
export function demoLoops(): number {
  return forOfObjects([{ x: 1, y: 2 }, { x: 3, y: 4 }]) * 100 + forOfArrays([[5, 1], [7, 2]]);
}
export function demoCallbacks(): number {
  return callbackPatterns([{ x: 1, y: 2 }, { x: 5, y: 1 }]);
}
