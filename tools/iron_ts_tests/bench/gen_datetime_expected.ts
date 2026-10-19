// Runs every exported function of datetime.ts in bun against the original engine TypeScript
// (time.ts) and writes what they return.
//   bun bench/gen_datetime_expected.ts
import * as e0 from "../../../js/engine/src/time.ts";

Object.assign(globalThis, e0);
// What the engine sets up around the game (info, clock ticks), if the program needs any.
if (await Bun.file(import.meta.dir + "/datetime_prelude.ts").exists()) await import("./datetime_prelude.ts");
const m = await import("./datetime.ts");

const out: Record<string, unknown> = {};
for (const [name, fn] of Object.entries(m)) {
  if (typeof fn !== "function") continue;
  const v = (fn as () => unknown)();
  out[name] = typeof v === "string" ? { s: Array.from({ length: v.length }, (_, i) => v.charCodeAt(i)) } : v;
}
await Bun.write(import.meta.dir + "/datetime_expected.json", JSON.stringify(out));
console.log("wrote", Object.keys(out).length, "results");
