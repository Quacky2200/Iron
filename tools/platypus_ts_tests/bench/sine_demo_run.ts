// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// QuickJS and bun entry for the verbatim sine-circle demo class (`sine_demo.ts`): exposes
// `__sineDemo(n)` for QuickJS, and times itself under bun (with `--preload bench/platypus_stub.ts`,
// because the engine package expects `__platypus_op`).
import { frames } from "./sine_demo.ts";

(globalThis as { __sineDemo?: (n: number) => number }).__sineDemo = frames;

if (typeof process !== "undefined" && process.argv[1]?.endsWith("sine_demo_run.ts")) {
  const n = Number(process.argv[2] ?? 1000);
  frames(50);
  const t = performance.now();
  const sum = frames(n);
  console.log(JSON.stringify({ engine: "bun", frames: n, ms: performance.now() - t, checksum: sum }));
}
