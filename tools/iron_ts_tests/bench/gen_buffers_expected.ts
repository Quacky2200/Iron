// Runs every exported function of buffers.ts in bun and writes what they return.
//   bun bench/gen_buffers_expected.ts
import * as m from "./buffers.ts";

const out: Record<string, unknown> = {};
for (const [name, fn] of Object.entries(m)) {
  if (typeof fn !== "function") continue;
  const v = (fn as () => unknown)();
  out[name] = typeof v === "string" ? { s: Array.from({ length: v.length }, (_, i) => v.charCodeAt(i)) } : v;
}
await Bun.write(import.meta.dir + "/buffers_expected.json", JSON.stringify(out));
console.log("wrote", Object.keys(out).length, "results");
