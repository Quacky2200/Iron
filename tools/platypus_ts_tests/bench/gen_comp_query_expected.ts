// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// Runs every exported function of comp_query.ts in bun against the original engine TypeScript
// (scene.ts, math.ts) and writes what they return.
//   bun bench/gen_comp_query_expected.ts
import * as e0 from "../../../js/engine/src/scene.ts";
import * as e1 from "../../../js/engine/src/math.ts";

Object.assign(globalThis, e0);
Object.assign(globalThis, e1);
// What the engine sets up around the game (info, clock ticks), if the program needs any.
if (await Bun.file(import.meta.dir + "/comp_query_prelude.ts").exists()) await import("./comp_query_prelude.ts");
const m = await import("./comp_query.ts");

const out: Record<string, unknown> = {};
for (const [name, fn] of Object.entries(m)) {
  if (typeof fn !== "function") continue;
  (globalThis as any).__bench_reset?.(); // a program that keeps state starts each function fresh
  const v = (fn as () => unknown)();
  out[name] = typeof v === "string" ? { s: Array.from({ length: v.length }, (_, i) => v.charCodeAt(i)) } : v;
}
await Bun.write(import.meta.dir + "/comp_query_expected.json", JSON.stringify(out));
console.log("wrote", Object.keys(out).length, "results");
