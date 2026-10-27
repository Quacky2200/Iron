// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// Runs the cases of tla_modules_cases.txt in bun as real ES modules (each case's files go into a
// temporary folder, `main.ts` is imported and its `result()` read once the whole graph has been
// evaluated) and writes tla_modules_expected.json.
//   bun bench/gen_tla_modules_expected.ts
import { mkdirSync, mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { pathToFileURL } from "node:url";

const text = readFileSync(new URL("./tla_modules_cases.txt", import.meta.url), "utf8");
const cases = text.split(/^\/\/## (\w+)\r?\n/m);
const out: Record<string, string> = {};
for (let i = 1; i < cases.length; i += 2) {
  const name = cases[i]!;
  const dir = mkdtempSync(join(tmpdir(), "tlam-"));
  const files = cases[i + 1]!.split(/^\/\/@ ([\w./-]+)\r?\n/m);
  for (let j = 1; j < files.length; j += 2) {
    const path = join(dir, files[j]!);
    mkdirSync(dirname(path), { recursive: true });
    writeFileSync(path, files[j + 1]!);
  }
  try {
    const mod = await import(pathToFileURL(join(dir, "main.ts")).href);
    out[name] = mod.result();
  } catch (e) {
    out[name] = "threw:" + (e as Error).message;
  }
}
writeFileSync(new URL("./tla_modules_expected.json", import.meta.url), JSON.stringify(out, null, 1) + "\n");
console.log(Object.keys(out).length, "cases");
