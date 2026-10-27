// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// Intl.getCanonicalLocales. Every exported function without parameters returns a string;
// `gen_canon_expected.ts` runs this file in bun.

export function single(): string {
  return Intl.getCanonicalLocales("EN-us").join(",");
}

export function list(): string {
  return Intl.getCanonicalLocales(["zh-hans-cn", "en-GB", "EN-gb", "sr-latn", "en-u-ca-gregory"]).join(",");
}

export function empty(): string {
  const none: string[] = [];
  return String(Intl.getCanonicalLocales(none).length);
}

export function invalid(): string {
  try {
    Intl.getCanonicalLocales(["en", "not_a_locale"]);
    return "no";
  } catch (e) {
    return (e as Error).name;
  }
}
