// SPDX-License-Identifier: Apache-2.0

// resolvedOptions of Intl.DateTimeFormat and Intl.NumberFormat (the common fields). Every exported
// function without parameters returns a string; `gen_resolved_expected.ts` runs this in bun.

function nz(v: string | undefined): string {
  return v === undefined ? "-" : v;
}

export function dateTimeFormat(): string {
  const o = new Intl.DateTimeFormat("en-US", { timeZone: "UTC" }).resolvedOptions();
  return [o.locale, nz(o.calendar), nz(o.numberingSystem), nz(o.timeZone)].join(",");
}

export function dateTimeZones(): string {
  const a = new Intl.DateTimeFormat("de-DE", { timeZone: "europe/berlin" }).resolvedOptions();
  const b = new Intl.DateTimeFormat("ja-JP", { timeZone: "Asia/Tokyo" }).resolvedOptions();
  const c = new Intl.DateTimeFormat("en-GB", { timeZone: "utc" }).resolvedOptions();
  return [a.timeZone, b.timeZone, c.timeZone, a.locale, b.locale, c.locale].join(",");
}

export function defaultLocale(): string {
  const o = new Intl.DateTimeFormat(undefined, { timeZone: "UTC" }).resolvedOptions();
  return o.locale.length > 0 ? "has-locale" : "none";
}

export function numberFormat(): string {
  const o = new Intl.NumberFormat("en-US").resolvedOptions();
  return [o.locale, nz(o.numberingSystem), nz(o.style), nz(o.notation), nz(o.signDisplay), String(o.minimumIntegerDigits), String(o.minimumFractionDigits), String(o.maximumFractionDigits)].join(",");
}

export function numberCurrency(): string {
  const o = new Intl.NumberFormat("de-DE", { style: "currency", currency: "EUR" }).resolvedOptions();
  return [o.locale, nz(o.style), nz(o.currency), String(o.minimumFractionDigits), String(o.maximumFractionDigits)].join(",");
}

export function numberOptions(): string {
  const o = new Intl.NumberFormat("en-US", { style: "percent", maximumFractionDigits: 1, minimumIntegerDigits: 2, signDisplay: "always", notation: "compact" }).resolvedOptions();
  return [nz(o.style), nz(o.notation), nz(o.signDisplay), String(o.minimumIntegerDigits), String(o.maximumFractionDigits)].join(",");
}
