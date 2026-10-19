// Runs every exported function of assets.ts in bun against the original engine TypeScript
// (assets.ts) and writes what they return.
//   bun bench/gen_assets_expected.ts
import * as e0 from "../../../js/engine/src/assets.ts";

Object.assign(globalThis, e0);
// What the engine sets up around the game (info, clock ticks), if the program needs any.
if (await Bun.file(import.meta.dir + "/assets_prelude.ts").exists()) await import("./assets_prelude.ts");
const m = await import("./assets.ts");

const out: Record<string, unknown> = {};
for (const [name, fn] of Object.entries(m)) {
  if (typeof fn !== "function") continue;
  (globalThis as any).__bench_reset?.(); // a program that keeps state starts each function fresh
  const v = (fn as () => unknown)();
  out[name] = typeof v === "string" ? { s: Array.from({ length: v.length }, (_, i) => v.charCodeAt(i)) } : v;
}
await Bun.write(import.meta.dir + "/assets_expected.json", JSON.stringify(out));
console.log("wrote", Object.keys(out).length, "results");
