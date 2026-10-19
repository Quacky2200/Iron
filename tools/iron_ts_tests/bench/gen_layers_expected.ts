// Runs every exported function of layers.ts in bun against the original engine TypeScript
// (layers.ts) and writes what they return.
//   bun bench/gen_layers_expected.ts
import * as e0 from "../../../js/engine/src/layers.ts";

Object.assign(globalThis, e0);
// What the engine sets up around the game (info, clock ticks), if the program needs any.
if (await Bun.file(import.meta.dir + "/layers_prelude.ts").exists()) await import("./layers_prelude.ts");
const m = await import("./layers.ts");

const out: Record<string, unknown> = {};
for (const [name, fn] of Object.entries(m)) {
  if (typeof fn !== "function") continue;
  const v = (fn as () => unknown)();
  out[name] = typeof v === "string" ? { s: Array.from({ length: v.length }, (_, i) => v.charCodeAt(i)) } : v;
}
await Bun.write(import.meta.dir + "/layers_expected.json", JSON.stringify(out));
console.log("wrote", Object.keys(out).length, "results");
