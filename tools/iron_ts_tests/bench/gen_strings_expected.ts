// Runs every exported function of strings.ts in bun and writes what they return, for
// `tests/strings.rs` to compare with the compiled code. A string is its UTF-16 units.
//   bun bench/gen_strings_expected.ts
import * as m from "./strings.ts";

const out: Record<string, unknown> = {};
for (const [name, fn] of Object.entries(m)) {
  if (typeof fn !== "function") continue;
  const v = (fn as () => unknown)();
  out[name] = typeof v === "string" ? { s: Array.from({ length: v.length }, (_, i) => v.charCodeAt(i)) } : v;
}
await Bun.write(import.meta.dir + "/strings_expected.json", JSON.stringify(out));
console.log("wrote", Object.keys(out).length, "results");
