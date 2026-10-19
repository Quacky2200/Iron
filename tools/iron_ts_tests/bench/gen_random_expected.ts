// Runs every exported function of random.ts in bun against the original engine TypeScript
// (random.ts) and writes what they return.
//   bun bench/gen_random_expected.ts
import * as e0 from "../../../js/engine/src/random.ts";

Object.assign(globalThis, e0);
// What the engine sets up around the game (info, clock ticks), if the program needs any.
if (await Bun.file(import.meta.dir + "/random_prelude.ts").exists()) await import("./random_prelude.ts");
const m = await import("./random.ts");

const out: Record<string, unknown> = {};
for (const [name, fn] of Object.entries(m)) {
  if (typeof fn !== "function") continue;
  (globalThis as any).__bench_reset?.(); // a program that keeps state starts each function fresh
  const v = (fn as () => unknown)();
  out[name] = typeof v === "string" ? { s: Array.from({ length: v.length }, (_, i) => v.charCodeAt(i)) } : v;
}
await Bun.write(import.meta.dir + "/random_expected.json", JSON.stringify(out));
console.log("wrote", Object.keys(out).length, "results");
