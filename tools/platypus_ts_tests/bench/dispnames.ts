// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// Intl.DisplayNames for regions, scripts and languages. Every exported function without parameters
// returns a string; `gen_dispnames_expected.ts` runs this file in bun.

export function regions(): string {
  const n = new Intl.DisplayNames("en", { type: "region" });
  return [n.of("US"), n.of("GB"), n.of("DE"), n.of("JP"), n.of("419"), n.of("ZZ")].join("|");
}

export function regionsInOtherLocales(): string {
  const fr = new Intl.DisplayNames("fr", { type: "region" });
  const de = new Intl.DisplayNames("de", { type: "region" });
  const ja = new Intl.DisplayNames("ja", { type: "region" });
  return [fr.of("US"), fr.of("DE"), de.of("FR"), de.of("US"), ja.of("JP")].join("|");
}

export function scripts(): string {
  const n = new Intl.DisplayNames("en", { type: "script" });
  return [n.of("Latn"), n.of("Cyrl"), n.of("Hans"), n.of("Arab")].join("|");
}

export function languages(): string {
  const n = new Intl.DisplayNames("en", { type: "language" });
  return [n.of("fr"), n.of("de"), n.of("ja"), n.of("en-US"), n.of("en-GB"), n.of("pt-BR")].join("|");
}

export function languagesStandard(): string {
  const n = new Intl.DisplayNames("en", { type: "language", languageDisplay: "standard" });
  return [n.of("en-US"), n.of("en-GB"), n.of("pt-BR")].join("|");
}

export function languagesInFrench(): string {
  const n = new Intl.DisplayNames("fr", { type: "language" });
  return [n.of("en"), n.of("es"), n.of("de")].join("|");
}

export function missingType(): string {
  try {
    new Intl.DisplayNames("en", {} as Intl.DisplayNamesOptions);
    return "no";
  } catch (e) {
    return (e as Error).name;
  }
}
