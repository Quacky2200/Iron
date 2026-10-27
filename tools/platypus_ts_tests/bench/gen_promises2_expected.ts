// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// Runs the cases of promises2_cases.ts in bun and writes promises2_expected.json.
//   bun bench/gen_promises2_expected.ts
import { readFileSync, writeFileSync } from "node:fs";

const text = readFileSync(new URL("./promises2_cases.ts", import.meta.url), "utf8");
const parts = text.split(/^\/\/## (\w+)\r?\n/m);
const transpiler = new Bun.Transpiler({ loader: "ts" });
const out: Record<string, string> = {};
for (let i = 1; i < parts.length; i += 2) {
  const name = parts[i]!;
  const js = transpiler.transformSync(parts[i + 1]!);
  try {
    out[name] = await new Function(js + "\nreturn t();")();
  } catch (e) {
    out[name] = "rejected:" + (e as Error).message;
  }
}
writeFileSync(new URL("./promises2_expected.json", import.meta.url), JSON.stringify(out, null, 1) + "\n");
console.log(Object.keys(out).length, "cases");
