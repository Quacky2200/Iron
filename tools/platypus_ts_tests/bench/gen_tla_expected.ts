// SPDX-License-Identifier: Apache-2.0

// Runs the cases of tla_cases.ts in bun, as the body of an async function (so the top-level
// awaits are ordinary awaits), and writes tla_expected.json.
//   bun bench/gen_tla_expected.ts
import { readFileSync, writeFileSync } from "node:fs";

const text = readFileSync(new URL("./tla_cases.ts", import.meta.url), "utf8");
const parts = text.split(/^\/\/## (\w+)\r?\n/m);
const transpiler = new Bun.Transpiler({ loader: "ts" });
const AsyncFunction = (async () => {}).constructor as new (body: string) => () => Promise<string>;
const out: Record<string, string> = {};
for (let i = 1; i < parts.length; i += 2) {
  const name = parts[i]!;
  const js = transpiler.transformSync(parts[i + 1]!.replace(/export function/g, "function"));
  try {
    out[name] = await new AsyncFunction(js + "\nreturn result();")();
  } catch (e) {
    out[name] = "threw:" + (e as Error).message;
  }
}
writeFileSync(new URL("./tla_expected.json", import.meta.url), JSON.stringify(out, null, 1) + "\n");
console.log(Object.keys(out).length, "cases");
