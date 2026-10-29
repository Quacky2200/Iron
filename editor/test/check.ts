// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// A small test kit for the editor's model tests, compiled natively like the editor
// (`tools/platypus_ts_tests/tests/ui/editor_models.rs` runs each file's `run()`): values are
// compared as JSON text, and `report()` lists what failed (empty when everything passed).

const failures: string[] = [];
let current = "";

/** Runs one test; an exception fails it. */
export function test(name: string, f: () => void): void {
  current = name;
  try {
    f();
  } catch (e) {
    failures.push(`${name}: threw ${e instanceof Error ? e.message : "an exception"}`);
  }
}

function fail(what: string, got: string, want: string): void {
  failures.push(`${current}: ${what}: got ${got}, want ${want}`);
}

/** The same JSON text. */
export function eq<T>(got: T, want: T, what: string): void {
  const g = JSON.stringify(got);
  const w = JSON.stringify(want);
  if (g !== w) fail(what, g, w);
}

/** Equal text (strings are compared as they are). */
export function same(got: string, want: string, what: string): void {
  if (got !== want) fail(what, got, want);
}

/** Within half a unit of the `digits`-th decimal, as bun's `toBeCloseTo`. */
export function close(got: number, want: number, what: string, digits = 2): void {
  if (!(Math.abs(got - want) < 0.5 * 10 ** -digits)) fail(what, `${got}`, `${want}`);
}

export function yes(cond: boolean, what: string): void {
  if (!cond) fail(what, "false", "true");
}

/** What failed, one line each; empty when all passed. */
export function report(): string {
  const out = failures.join("\n");
  failures.length = 0;
  return out;
}
