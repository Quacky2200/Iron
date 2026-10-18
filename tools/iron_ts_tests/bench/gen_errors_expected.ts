// Runs every exported function of errors.ts in bun and writes what they return (an uncaught
// exception as `{ "threw": "<name>: <message>" }`) for `tests/errors.rs`.
//   bun bench/gen_errors_expected.ts
import * as m from "./errors.ts";

const out: Record<string, unknown> = {};
for (const [name, fn] of Object.entries(m)) {
  if (typeof fn !== "function") continue;
  try {
    const v = (fn as () => unknown)();
    out[name] = typeof v === "string" ? { s: Array.from({ length: v.length }, (_, i) => v.charCodeAt(i)) } : v;
  } catch (e) {
    out[name] = { threw: String(e) };
  }
}
await Bun.write(import.meta.dir + "/errors_expected.json", JSON.stringify(out));
console.log("wrote", Object.keys(out).length, "results");
