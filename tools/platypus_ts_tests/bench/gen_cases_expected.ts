// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// Runs the cases (`//## name`, each defining `function t(): string`) of a cases file in bun and
// writes <name>_expected.json next to it.
//   bun bench/gen_cases_expected.ts unions_cases.ts
import { existsSync, readFileSync, writeFileSync } from "node:fs";

const file = process.argv[2]!;
const text = readFileSync(new URL("./" + file, import.meta.url), "utf8");
const parts = text.split(/^\/\/## (\w+)\r?\n/m);
const transpiler = new Bun.Transpiler({ loader: "ts" });
// Stand-ins for the engine APIs a case calls (`<name>_prelude.js` next to `<name>_cases.ts`).
const preludeFile = new URL("./" + file.replace(/_cases\.ts$/, "_prelude.js"), import.meta.url);
const prelude = existsSync(preludeFile) ? readFileSync(preludeFile, "utf8") + "\n" : "";
const out: Record<string, string> = {};
for (let i = 1; i < parts.length; i += 2) {
  const name = parts[i]!;
  const js = transpiler.transformSync(parts[i + 1]!);
  try {
    out[name] = new Function(prelude + js + "\nreturn t();")();
  } catch (e) {
    out[name] = "threw:" + (e as Error).message;
  }
}
const base = file.replace(/_cases\.ts$/, "");
writeFileSync(new URL("./" + base + "_expected.json", import.meta.url), JSON.stringify(out, null, 1) + "\n");
console.log(Object.keys(out).length, "cases");
