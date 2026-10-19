// Intl.NumberFormat / DateTimeFormat formatToParts. Every exported function without parameters
// returns a string; `gen_fparts_expected.ts` runs this file in bun.

function show(parts: Intl.NumberFormatPart[]): string {
  const out: string[] = [];
  for (const p of parts) out.push(p.type + ":" + p.value);
  return out.join("|");
}

export function decimal(): string {
  const f = new Intl.NumberFormat("en-US");
  return [show(f.formatToParts(1234567.891)), show(f.formatToParts(-0.5)), show(f.formatToParts(42))].join(" ## ");
}

export function currency(): string {
  const f = new Intl.NumberFormat("en-US", { style: "currency", currency: "USD" });
  const g = new Intl.NumberFormat("de-DE", { style: "currency", currency: "EUR" });
  return [show(f.formatToParts(-1234.5)), show(g.formatToParts(1234.5)), show(f.formatToParts(0))].join(" ## ");
}

export function percent(): string {
  const f = new Intl.NumberFormat("en-US", { style: "percent" });
  return [show(f.formatToParts(0.256)), show(f.formatToParts(-1.5))].join(" ## ");
}

export function otherLocales(): string {
  const de = new Intl.NumberFormat("de-DE");
  const fr = new Intl.NumberFormat("fr-FR");
  const ar = new Intl.NumberFormat("ar-EG");
  const hi = new Intl.NumberFormat("hi-IN");
  return [show(de.formatToParts(1234567.5)), show(fr.formatToParts(1234567.5)), show(ar.formatToParts(1234.5)), show(hi.formatToParts(1234567.5))].join(" ## ");
}

export function signs(): string {
  const f = new Intl.NumberFormat("en-US", { signDisplay: "always" });
  return [show(f.formatToParts(5)), show(f.formatToParts(-5)), show(f.formatToParts(0))].join(" ## ");
}

export function compact(): string {
  const f = new Intl.NumberFormat("en-US", { notation: "compact" });
  return [show(f.formatToParts(1234)), show(f.formatToParts(1500000)), show(f.formatToParts(999))].join(" ## ");
}

export function scientific(): string {
  const f = new Intl.NumberFormat("en-US", { notation: "scientific" });
  return [show(f.formatToParts(123456)), show(f.formatToParts(0.00012))].join(" ## ");
}

export function special(): string {
  const f = new Intl.NumberFormat("en-US");
  return [show(f.formatToParts(NaN)), show(f.formatToParts(Infinity)), show(f.formatToParts(-Infinity))].join(" ## ");
}

function showDate(parts: Intl.DateTimeFormatPart[]): string {
  const out: string[] = [];
  for (const p of parts) out.push(p.type + ":" + p.value);
  return out.join("|");
}

export function dateEnglish(): string {
  const f = new Intl.DateTimeFormat("en-US", { timeZone: "UTC" });
  const d = new Date(Date.UTC(2024, 2, 5, 14, 7, 9));
  return showDate(f.formatToParts(d));
}

export function dateLong(): string {
  const f = new Intl.DateTimeFormat("en-US", { timeZone: "UTC", dateStyle: "full", timeStyle: "short" });
  const d = new Date(Date.UTC(2024, 2, 5, 14, 7, 9));
  return showDate(f.formatToParts(d));
}

export function dateFields(): string {
  const f = new Intl.DateTimeFormat("en-GB", { timeZone: "Europe/London", weekday: "short", year: "numeric", month: "short", day: "numeric", hour: "2-digit", minute: "2-digit", second: "2-digit" });
  const d = new Date(Date.UTC(2024, 6, 15, 8, 5, 3));
  return showDate(f.formatToParts(d));
}

export function dateOtherLocales(): string {
  const d = new Date(Date.UTC(2024, 2, 5, 14, 7, 9));
  const de = new Intl.DateTimeFormat("de-DE", { timeZone: "UTC", dateStyle: "long", timeStyle: "medium" });
  const ja = new Intl.DateTimeFormat("ja-JP", { timeZone: "UTC", dateStyle: "long" });
  return [showDate(de.formatToParts(d)), showDate(ja.formatToParts(d))].join(" ## ");
}

export function dateHour12(): string {
  const f = new Intl.DateTimeFormat("en-US", { timeZone: "UTC", hour: "numeric", minute: "2-digit", hour12: true });
  const d = new Date(Date.UTC(2024, 2, 5, 14, 7, 9));
  return showDate(f.formatToParts(d));
}
