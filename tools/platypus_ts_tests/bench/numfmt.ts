// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// Intl, step I2: Number.prototype.toLocaleString and Intl.NumberFormat. Every exported function
// without parameters returns a string; `gen_numfmt_expected.ts` runs this file in bun and
// `tests/numfmt.rs` compares.

const XS = [0, 1, -1, 0.5, 2.5, 1234.5678, 1234567.891, -9876543.21, 0.000123, 99999.9999, 1e21, -0, NaN, Infinity, -Infinity, 123456789012345680000, 0.1 + 0.2, 1000, 9999, 12345];

function fail(f: () => string): string {
  try {
    return f();
  } catch (e) {
    return (e as Error).name + ": " + (e as Error).message;
  }
}

export function decimalLocales(): string {
  let out = "";
  for (const x of XS) {
    out += [
      x.toLocaleString(), x.toLocaleString("en-US"), x.toLocaleString("de-DE"), x.toLocaleString("fr-FR"), x.toLocaleString("en-IN"),
      x.toLocaleString("ja-JP"), x.toLocaleString("ar-EG"), x.toLocaleString("hi-IN"), x.toLocaleString("es-ES"), x.toLocaleString("pt-BR"),
      x.toLocaleString("ru-RU"), x.toLocaleString("de-CH"), x.toLocaleString("sv-SE"), x.toLocaleString("pl-PL"), x.toLocaleString("zh-CN"),
    ].join(" ; ") + "\n";
  }
  return out;
}

export function fractionDigits(): string {
  let out = "";
  for (const x of XS) {
    out += [
      x.toLocaleString("en-US", { maximumFractionDigits: 0 }), x.toLocaleString("en-US", { minimumFractionDigits: 2 }),
      x.toLocaleString("en-US", { minimumFractionDigits: 1, maximumFractionDigits: 2 }), x.toLocaleString("en-US", { maximumFractionDigits: 5 }),
      x.toLocaleString("en-US", { minimumIntegerDigits: 3 }), x.toLocaleString("en-US", { minimumFractionDigits: 4, maximumFractionDigits: 4 }),
    ].join(" ; ") + "\n";
  }
  return out;
}

export function significantDigits(): string {
  let out = "";
  for (const x of XS) {
    out += [
      x.toLocaleString("en-US", { maximumSignificantDigits: 3 }), x.toLocaleString("en-US", { minimumSignificantDigits: 5 }),
      x.toLocaleString("en-US", { minimumSignificantDigits: 2, maximumSignificantDigits: 4 }), x.toLocaleString("en-US", { maximumSignificantDigits: 1 }),
    ].join(" ; ") + "\n";
  }
  return out;
}

export function percentAndSign(): string {
  let out = "";
  for (const x of [0, 0.256, 1, -0.5, 0.0049, 12.3456, NaN, Infinity]) {
    out += [
      x.toLocaleString("en-US", { style: "percent" }), x.toLocaleString("de-DE", { style: "percent" }), x.toLocaleString("fr-FR", { style: "percent" }),
      x.toLocaleString("en-US", { style: "percent", minimumFractionDigits: 1 }), x.toLocaleString("en-US", { signDisplay: "always" }),
      x.toLocaleString("en-US", { signDisplay: "exceptZero" }), x.toLocaleString("en-US", { signDisplay: "never" }), x.toLocaleString("en-US", { signDisplay: "negative" }),
    ].join(" ; ") + "\n";
  }
  return out;
}

export function currency(): string {
  let out = "";
  for (const x of [0, 1, -1, 1234.5, 1234567.891, -0.005, 0.005, 1e9]) {
    out += [
      x.toLocaleString("en-US", { style: "currency", currency: "USD" }), x.toLocaleString("de-DE", { style: "currency", currency: "EUR" }),
      x.toLocaleString("ja-JP", { style: "currency", currency: "JPY" }), x.toLocaleString("en-GB", { style: "currency", currency: "GBP" }),
      x.toLocaleString("fr-FR", { style: "currency", currency: "EUR" }), x.toLocaleString("en-IN", { style: "currency", currency: "INR" }),
      x.toLocaleString("en-US", { style: "currency", currency: "EUR" }), x.toLocaleString("en-US", { style: "currency", currency: "JPY" }),
      x.toLocaleString("en-US", { style: "currency", currency: "KWD" }), x.toLocaleString("pt-BR", { style: "currency", currency: "BRL" }),
    ].join(" ; ") + "\n";
  }
  return out;
}

export function currencyDisplay(): string {
  const x = 1234.5;
  return [
    x.toLocaleString("en-US", { style: "currency", currency: "USD", currencyDisplay: "code" }),
    x.toLocaleString("en-US", { style: "currency", currency: "USD", currencyDisplay: "name" }),
    x.toLocaleString("en-US", { style: "currency", currency: "USD", currencyDisplay: "narrowSymbol" }),
    x.toLocaleString("en-CA", { style: "currency", currency: "USD" }),
    x.toLocaleString("en-CA", { style: "currency", currency: "USD", currencyDisplay: "narrowSymbol" }),
    (-x).toLocaleString("en-US", { style: "currency", currency: "USD", currencySign: "accounting" }),
    x.toLocaleString("de-DE", { style: "currency", currency: "USD", currencyDisplay: "name" }),
    x.toLocaleString("en-US", { style: "currency", currency: "usd" }),
    x.toLocaleString("en-US", { style: "currency", currency: "EUR", minimumFractionDigits: 0 }),
    x.toLocaleString("en-US", { style: "currency", currency: "USD", maximumFractionDigits: 0 }),
  ].join("\n");
}

export function compact(): string {
  let out = "";
  for (const x of [0, 1, 12, 123, 999, 1000, 1234, 12345, 123456, 999999, 1234567, 1e9, 1.5e12, -1234567, 0.5, 1.5, 15.5, 0.0123]) {
    out += [
      x.toLocaleString("en-US", { notation: "compact" }), x.toLocaleString("en-US", { notation: "compact", compactDisplay: "long" }),
      x.toLocaleString("de-DE", { notation: "compact" }), x.toLocaleString("de-DE", { notation: "compact", compactDisplay: "long" }),
      x.toLocaleString("ja-JP", { notation: "compact" }), x.toLocaleString("fr-FR", { notation: "compact" }), x.toLocaleString("en-IN", { notation: "compact" }),
      x.toLocaleString("en-US", { notation: "compact", maximumFractionDigits: 2 }),
    ].join(" ; ") + "\n";
  }
  return out;
}

export function scientific(): string {
  let out = "";
  for (const x of [0, 1, 1234567, 0.000123, -45.6789, 9999.9999, 1e21, 123, 0.5]) {
    out += [
      x.toLocaleString("en-US", { notation: "scientific" }), x.toLocaleString("en-US", { notation: "engineering" }),
      x.toLocaleString("de-DE", { notation: "scientific" }), x.toLocaleString("en-US", { notation: "scientific", maximumFractionDigits: 1 }),
    ].join(" ; ") + "\n";
  }
  return out;
}

export function grouping(): string {
  let out = "";
  for (const x of [999, 1000, 1234, 12345, 123456, 1234567]) {
    out += [
      x.toLocaleString("en-US", { useGrouping: false }), x.toLocaleString("en-US", { useGrouping: true }), x.toLocaleString("en-US", { useGrouping: "min2" }),
      x.toLocaleString("es-ES"), x.toLocaleString("pl-PL"), x.toLocaleString("pt-PT"),
    ].join(" ; ") + "\n";
  }
  return out;
}

export function rounding(): string {
  let out = "";
  for (const x of [0.5, 1.5, 2.5, -0.5, -1.5, -2.5, 1.45, 1.55, 2.675]) {
    out += [
      x.toLocaleString("en-US", { maximumFractionDigits: 0 }), x.toLocaleString("en-US", { maximumFractionDigits: 0, roundingMode: "ceil" }),
      x.toLocaleString("en-US", { maximumFractionDigits: 0, roundingMode: "floor" }), x.toLocaleString("en-US", { maximumFractionDigits: 0, roundingMode: "expand" }),
      x.toLocaleString("en-US", { maximumFractionDigits: 0, roundingMode: "trunc" }), x.toLocaleString("en-US", { maximumFractionDigits: 0, roundingMode: "halfCeil" }),
      x.toLocaleString("en-US", { maximumFractionDigits: 0, roundingMode: "halfFloor" }), x.toLocaleString("en-US", { maximumFractionDigits: 0, roundingMode: "halfTrunc" }),
      x.toLocaleString("en-US", { maximumFractionDigits: 0, roundingMode: "halfEven" }), x.toLocaleString("en-US", { maximumFractionDigits: 1 }),
      x.toLocaleString("en-US", { minimumFractionDigits: 2, trailingZeroDisplay: "stripIfInteger" }),
    ].join(" ; ") + "\n";
  }
  return out;
}

export function formatObject(): string {
  const a = new Intl.NumberFormat("en-US", { style: "currency", currency: "USD" });
  const b = new Intl.NumberFormat("de-DE");
  const c = new Intl.NumberFormat();
  const fmt = a.format;
  return [a.format(1234.5), b.format(1234567.891), c.format(0.5), fmt(99), [1, 2.5, 1000].map(a.format).join(",")].join("|");
}

export function errors(): string {
  return [
    fail(() => (1).toLocaleString("en-US", { style: "foo" })),
    fail(() => (1).toLocaleString("en-US", { style: "currency" })),
    fail(() => (1).toLocaleString("en-US", { style: "currency", currency: "US" })),
    fail(() => (1).toLocaleString("en-US", { currencyDisplay: "x" })),
    fail(() => (1).toLocaleString("en-US", { notation: "x" })),
    fail(() => (1).toLocaleString("en-US", { signDisplay: "x" })),
    fail(() => (1).toLocaleString("en-US", { minimumFractionDigits: 101 })),
    fail(() => (1).toLocaleString("en-US", { minimumFractionDigits: 3, maximumFractionDigits: 2 })),
    fail(() => (1).toLocaleString("en-US", { maximumSignificantDigits: 0 })),
    fail(() => (1).toLocaleString("not a tag")),
  ].join("\n");
}
