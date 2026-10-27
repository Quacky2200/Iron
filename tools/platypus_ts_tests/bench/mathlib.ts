// SPDX-License-Identifier: Apache-2.0

// The Math library surface and the operators that go with it. Functions return arrays so the
// native tests can compare every value, including the sign of zero.

export function ceilTrunc(): number[] {
  return [
    Math.ceil(1.2),
    Math.ceil(-1.2),
    Math.ceil(-0.5),
    Math.trunc(2.7),
    Math.trunc(-2.7),
    Math.trunc(-0.2),
    Math.floor(-0.5),
    Math.ceil(5),
  ];
}

/** JS rounds halves toward +Infinity and keeps the sign of zero. */
export function rounds(): number[] {
  return [
    Math.round(0.5),
    Math.round(-0.5),
    Math.round(1.5),
    Math.round(-1.5),
    Math.round(2.5),
    Math.round(-2.5),
    Math.round(2.4),
    Math.round(-2.6),
    Math.round(0.49999999999999994),
    Math.round(-0.3),
    Math.round(4503599627370497),
    Math.round(7),
  ];
}

/** `**` is right associative; `Math.pow(1, Infinity)` is NaN in JS (C's pow gives 1). */
export function powers(): number[] {
  return [
    Math.pow(2, 10),
    2 ** -1,
    3 ** 2 ** 2,
    (-2) ** 3,
    Math.pow(-8, 1 / 3),
    Math.pow(1, Infinity),
    Math.pow(-1, -Infinity),
    Math.pow(0, 0),
    Math.pow(NaN, 0),
    Math.pow(2, NaN),
    Math.pow(0, -1),
    Math.pow(10, 2),
  ];
}

export function bits(): number[] {
  return [
    Math.clz32(1),
    Math.clz32(0),
    Math.clz32(-1),
    Math.clz32(0x80000000),
    Math.clz32(255),
    ~5,
    ~-1,
    ~0,
    ~1.9,
    Math.fround(0.1) === 0.1 ? 1 : 0,
    Math.fround(5.5),
    Math.fround(16777217),
  ];
}

/** Scaled and rounded so the last bit of the C library's answer does not matter. */
export function transcendental(): number[] {
  const r = (x: number) => Math.round(x * 1e9);
  return [
    r(Math.tan(1)),
    r(Math.asin(0.5)),
    r(Math.acos(0.5)),
    r(Math.atan(2)),
    r(Math.exp(1)),
    r(Math.log(10)),
    r(Math.log2(10)),
    r(Math.log10(1000)),
    r(Math.cbrt(27)),
    r(Math.cbrt(-8)),
    r(Math.LN2),
    r(Math.LN10),
    r(Math.LOG2E),
    r(Math.LOG10E),
    r(Math.SQRT1_2),
  ];
}

/** `??=`, `||=`, `&&=` and `**=` assign only when they must. */
export function assignments(): number[] {
  let a: number | null = null;
  a ??= 5;
  a ??= 9;
  let b = 0;
  b ||= 7;
  b ||= 8;
  let c = 3;
  c &&= 4;
  let d = 0;
  d &&= 99;
  let e = 2;
  e **= 5;
  return [a, b, c, d, e];
}

/** A target with side effects is evaluated once. */
export function assignOnce(): number[] {
  const xs: (number | null)[] = [null, 4];
  let i = 0;
  xs[i++] ??= 10;
  xs[i++] ??= 20;
  return [xs[0]!, xs[1]!, i];
}

/** Math.random() is in [0, 1). The values themselves are not compared. */
export function randomInRange(n: number): number {
  let bad = 0;
  let sum = 0;
  for (let i = 0; i < n; i++) {
    const r = Math.random();
    if (!(r >= 0 && r < 1)) bad += 1;
    sum += r;
  }
  return bad * 1000000 + Math.round((sum / n) * 10);
}
