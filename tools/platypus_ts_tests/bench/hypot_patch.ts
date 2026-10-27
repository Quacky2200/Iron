// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// `Math.hypot` of three or more numbers: the engines differ in the last bit, so the original
// TypeScript runs with the algorithm the native math uses (scale by the largest, sum the squares).
const original = Math.hypot;
Math.hypot = (...a: number[]): number => {
  if (a.length < 3) return original(...a);
  let max = 0;
  for (const x of a) {
    if (!Number.isFinite(x)) return original(...a);
    max = Math.max(max, Math.abs(x));
  }
  if (max === 0) return 0;
  let sum = 0;
  for (const x of a) {
    const n = x / max;
    sum += n * n;
  }
  return Math.sqrt(sum) * max;
};
