// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// Mathf, compiled against the native API and run in bun against js/engine/src/math.ts. Every
// number is printed in full, so a different last bit is a failure.

export function constants(): string {
  return `${Mathf.PI},${Mathf.Deg2Rad},${Mathf.Rad2Deg},${Mathf.Epsilon}`;
}

export function interpolation(): string {
  const xs = [-2.5, -1, -0.5, 0, 0.3, 0.5, 1, 1.7, 3.14159, 100];
  let s = "";
  for (const x of xs) {
    s += `${Mathf.clamp(x, -1, 1)},${Mathf.clamp01(x)},${Mathf.lerp(2, 10, x)},${Mathf.lerpUnclamped(2, 10, x)},`;
    s += `${Mathf.inverseLerp(2, 10, x)},${Mathf.smoothStep(0, 5, x)},${Mathf.repeat(x, 1.5)},${Mathf.pingPong(x, 2)};`;
  }
  s += `${Mathf.inverseLerp(3, 3, 5)},${Mathf.clamp(Mathf.PI, 4, 1)}`;
  return s;
}

export function stepping(): string {
  let s = "";
  for (const cur of [0, 5, -3.5]) {
    for (const target of [0, 5, 10, -10]) {
      s += `${Mathf.moveTowards(cur, target, 2)},${Mathf.moveTowards(cur, target, 100)};`;
    }
  }
  return s;
}

export function angles(): string {
  let s = "";
  for (const a of [0, 90, 170, 350, 359, -170, 720]) {
    for (const b of [0, 10, 180, 190, 270, -90, 361]) {
      s += `${Mathf.deltaAngle(a, b)},${Mathf.lerpAngle(a, b, 0.25)},${Mathf.moveTowardsAngle(a, b, 30)};`;
    }
  }
  return s;
}

export function comparisons(): string {
  return `${Mathf.approximately(1, 1 + 1e-7)},${Mathf.approximately(1, 1.001)},${Mathf.approximately(0, 1e-7)},${Mathf.approximately(1e6, 1e6 + 0.5)},${Mathf.sign(3)},${Mathf.sign(0)},${Mathf.sign(-0.1)}`;
}

export function powers(): string {
  let s = "";
  for (const v of [-4, 0, 0.5, 1, 2, 3, 5, 8, 100, 1000, 1024, 1025]) {
    s += `${Mathf.isPowerOfTwo(v)},${Mathf.nextPowerOfTwo(v)},${Mathf.closestPowerOfTwo(v)};`;
  }
  return s;
}

export function noise(): string {
  let s = "";
  for (let i = 0; i < 12; i++) {
    s += `${Mathf.perlinNoise(i * 0.37, i * 0.91 - 2)},`;
  }
  s += `${Mathf.perlinNoise(0, 0)},${Mathf.perlinNoise(-1.5, 3.25)},${Mathf.perlinNoise(300.7, -255.2)}`;
  return s;
}

export function colourSpace(): string {
  let s = "";
  for (const c of [0, 0.02, 0.04045, 0.1, 0.5, 0.8, 1]) {
    s += `${Mathf.gammaToLinearSpace(c)},${Mathf.linearToGammaSpace(c)};`;
  }
  return s;
}

export function damping(): string {
  let x = 0;
  let v = 0;
  let s = "";
  for (let i = 0; i < 12; i++) {
    const r = Mathf.smoothDamp(x, 10, v, 0.4, 0.05);
    x = r.x;
    v = r.y;
    s += `${x},${v};`;
  }
  const capped = Mathf.smoothDamp(0, 10, 0, 0.4, 0.05, 5);
  return `${s}${capped.x},${capped.y}`;
}
