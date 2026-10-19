// Runs every test of promises.ts in bun, one after the other, lets timers and microtasks finish,
// and writes each result. Keys are the `_result` functions' names without the suffix.
//   bun bench/gen_promises_expected.ts
import * as m from "./promises.ts";

const out: Record<string, unknown> = {};
const names = Object.keys(m).filter((n) => !n.endsWith("_result"));
for (const name of names) {
  (m as Record<string, () => string>)[name]();
  await new Promise((r) => setTimeout(r, 150));
  const v = (m as Record<string, () => string>)[name + "_result"]();
  out[name + "_result"] = { s: Array.from({ length: v.length }, (_, i) => v.charCodeAt(i)) };
}
await Bun.write(import.meta.dir + "/promises_expected.json", JSON.stringify(out));
console.log("wrote", Object.keys(out).length, "results");
