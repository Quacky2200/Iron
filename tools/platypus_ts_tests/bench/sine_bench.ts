// SPDX-License-Identifier: Apache-2.0

// Frame loop for the QuickJS and bun runs of the sine-circle curve: the demo's per-frame work.
// QuickJS gets the real `__platypus_op`, so the cost of crossing into Rust is included.
import { curveFlat, type SineType } from "./sine.ts";

declare const __platypus_op: ((name: string, ...args: unknown[]) => void) | undefined;
const TYPES: SineType[] = ["pi", "normal", "doubleSine", "doubleCos"];

/** Frame f's parameters: a slow drift so no frame repeats. */
export function frameParams(f: number): [SineType, number, number, number, number, number, number] {
  return [TYPES[Math.floor(f / 50) % 4]!, 100 + (f % 120), 10 + (f % 70), 5 + (f % 60), (f * 3) % 360, 504 + (f % 40), 364 + (f % 30)];
}

/** `send`: also pass the points to `__platypus_op`, as LineRenderer2D does. */
export function frames(n: number, send: boolean): number {
  let sum = 0;
  for (let f = 0; f < n; f++) {
    const [t, r, fr, d, rot, cx, cy] = frameParams(f);
    const flat = curveFlat(t, r, fr, d, rot, cx, cy);
    if (send && typeof __platypus_op !== "undefined") __platypus_op("render.stroke", 1, 2, 0.05, [1, 0, 0, 1], true, flat);
    sum += flat[flat.length - 2]! + flat[flat.length - 1]! + flat.length;
  }
  return sum;
}

(globalThis as { __sine?: (n: number, send: boolean) => number }).__sine = frames;

if (typeof process !== "undefined" && process.argv[1]?.endsWith("sine_bench.ts")) {
  const n = Number(process.argv[2] ?? 1000);
  frames(50, false);
  const t = performance.now();
  const sum = frames(n, false);
  console.log(JSON.stringify({ engine: "bun", frames: n, ms: performance.now() - t, checksum: sum }));
}
