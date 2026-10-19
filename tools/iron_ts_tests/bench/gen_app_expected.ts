// Runs every exported function of app.ts in bun against the original engine TypeScript
// (app.ts) and writes what they return.
//   bun bench/gen_app_expected.ts
import * as e0 from "../../../js/engine/src/app.ts";

Object.assign(globalThis, e0);
// What the engine sets up around the game (info, clock ticks), if the program needs any.
if (await Bun.file(import.meta.dir + "/app_prelude.ts").exists()) await import("./app_prelude.ts");
const m = await import("./app.ts");

const out: Record<string, unknown> = {};
for (const [name, fn] of Object.entries(m)) {
  if (typeof fn !== "function") continue;
  const v = (fn as () => unknown)();
  out[name] = typeof v === "string" ? { s: Array.from({ length: v.length }, (_, i) => v.charCodeAt(i)) } : v;
}
await Bun.write(import.meta.dir + "/app_expected.json", JSON.stringify(out));
console.log("wrote", Object.keys(out).length, "results");
