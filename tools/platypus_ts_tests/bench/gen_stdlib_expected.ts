// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// Runs every exported function of stdlib.ts in bun and writes what they return.
//   bun bench/gen_stdlib_expected.ts
import * as m from "./stdlib.ts";

const out: Record<string, unknown> = {};
for (const [name, fn] of Object.entries(m)) {
  if (typeof fn !== "function") continue;
  const v = (fn as () => unknown)();
  out[name] = typeof v === "string" ? { s: Array.from({ length: v.length }, (_, i) => v.charCodeAt(i)) } : v;
}
await Bun.write(import.meta.dir + "/stdlib_expected.json", JSON.stringify(out));
console.log("wrote", Object.keys(out).length, "results");
