// Intl, step I3a: Date.prototype.toLocaleString/DateString/TimeString and Intl.DateTimeFormat.
// Every exported function without parameters returns a string; `gen_datefmt_expected.ts` runs this
// file in bun once per time zone and `tests/datefmt.rs` compares. The option sets are written out
// because Iron reads options as literals.

const INSTANTS = [1792405800123, 0, 1772955000000, 1783000000000, 951782400000, 1798761599000];

function fail(f: () => string): string {
  try {
    return f();
  } catch (e) {
    return (e as Error).name + ": " + (e as Error).message;
  }
}

export function defaultsPerLocale(): string {
  let out = "";
  for (const t of INSTANTS) {
    const d = new Date(t);
    out += [
      d.toLocaleString("en-US"),
      d.toLocaleString("en-GB"),
      d.toLocaleString("de-DE"),
      d.toLocaleString("fr-FR"),
      d.toLocaleString("es-ES"),
      d.toLocaleString("it-IT"),
      d.toLocaleString("pt-BR"),
      d.toLocaleString("nl-NL"),
      d.toLocaleString("sv-SE"),
      d.toLocaleString("pl-PL"),
      d.toLocaleString("ru-RU"),
      d.toLocaleString("ja-JP"),
      d.toLocaleString("ko-KR"),
      d.toLocaleString("zh-CN"),
      d.toLocaleString("hi-IN"),
      d.toLocaleDateString("en-US"),
      d.toLocaleDateString("en-GB"),
      d.toLocaleDateString("de-DE"),
      d.toLocaleDateString("fr-FR"),
      d.toLocaleDateString("es-ES"),
      d.toLocaleDateString("it-IT"),
      d.toLocaleDateString("pt-BR"),
      d.toLocaleDateString("nl-NL"),
      d.toLocaleDateString("sv-SE"),
      d.toLocaleDateString("pl-PL"),
      d.toLocaleDateString("ru-RU"),
      d.toLocaleDateString("ja-JP"),
      d.toLocaleDateString("ko-KR"),
      d.toLocaleDateString("zh-CN"),
      d.toLocaleDateString("hi-IN"),
      d.toLocaleTimeString("en-US"),
      d.toLocaleTimeString("en-GB"),
      d.toLocaleTimeString("de-DE"),
      d.toLocaleTimeString("fr-FR"),
      d.toLocaleTimeString("es-ES"),
      d.toLocaleTimeString("it-IT"),
      d.toLocaleTimeString("pt-BR"),
      d.toLocaleTimeString("nl-NL"),
      d.toLocaleTimeString("sv-SE"),
      d.toLocaleTimeString("pl-PL"),
      d.toLocaleTimeString("ru-RU"),
      d.toLocaleTimeString("ja-JP"),
      d.toLocaleTimeString("ko-KR"),
      d.toLocaleTimeString("zh-CN"),
      d.toLocaleTimeString("hi-IN"),
    ].join(" ; ") + "\n";
  }
  return out;
}

export function dateStyle_full(): string {
  let out = "";
  for (const t of INSTANTS) {
    const d = new Date(t);
    out += [
      d.toLocaleDateString("en-US", { dateStyle: "full" }),
      d.toLocaleDateString("en-GB", { dateStyle: "full" }),
      d.toLocaleDateString("de-DE", { dateStyle: "full" }),
      d.toLocaleDateString("fr-FR", { dateStyle: "full" }),
      d.toLocaleDateString("es-ES", { dateStyle: "full" }),
      d.toLocaleDateString("it-IT", { dateStyle: "full" }),
      d.toLocaleDateString("pt-BR", { dateStyle: "full" }),
      d.toLocaleDateString("nl-NL", { dateStyle: "full" }),
      d.toLocaleDateString("sv-SE", { dateStyle: "full" }),
      d.toLocaleDateString("pl-PL", { dateStyle: "full" }),
      d.toLocaleDateString("ru-RU", { dateStyle: "full" }),
      d.toLocaleDateString("ja-JP", { dateStyle: "full" }),
      d.toLocaleDateString("ko-KR", { dateStyle: "full" }),
      d.toLocaleDateString("zh-CN", { dateStyle: "full" }),
      d.toLocaleDateString("hi-IN", { dateStyle: "full" }),
    ].join(" ; ") + "\n";
  }
  return out;
}

export function dateStyle_long(): string {
  let out = "";
  for (const t of INSTANTS) {
    const d = new Date(t);
    out += [
      d.toLocaleDateString("en-US", { dateStyle: "long" }),
      d.toLocaleDateString("en-GB", { dateStyle: "long" }),
      d.toLocaleDateString("de-DE", { dateStyle: "long" }),
      d.toLocaleDateString("fr-FR", { dateStyle: "long" }),
      d.toLocaleDateString("es-ES", { dateStyle: "long" }),
      d.toLocaleDateString("it-IT", { dateStyle: "long" }),
      d.toLocaleDateString("pt-BR", { dateStyle: "long" }),
      d.toLocaleDateString("nl-NL", { dateStyle: "long" }),
      d.toLocaleDateString("sv-SE", { dateStyle: "long" }),
      d.toLocaleDateString("pl-PL", { dateStyle: "long" }),
      d.toLocaleDateString("ru-RU", { dateStyle: "long" }),
      d.toLocaleDateString("ja-JP", { dateStyle: "long" }),
      d.toLocaleDateString("ko-KR", { dateStyle: "long" }),
      d.toLocaleDateString("zh-CN", { dateStyle: "long" }),
      d.toLocaleDateString("hi-IN", { dateStyle: "long" }),
    ].join(" ; ") + "\n";
  }
  return out;
}

export function dateStyle_medium(): string {
  let out = "";
  for (const t of INSTANTS) {
    const d = new Date(t);
    out += [
      d.toLocaleDateString("en-US", { dateStyle: "medium" }),
      d.toLocaleDateString("en-GB", { dateStyle: "medium" }),
      d.toLocaleDateString("de-DE", { dateStyle: "medium" }),
      d.toLocaleDateString("fr-FR", { dateStyle: "medium" }),
      d.toLocaleDateString("es-ES", { dateStyle: "medium" }),
      d.toLocaleDateString("it-IT", { dateStyle: "medium" }),
      d.toLocaleDateString("pt-BR", { dateStyle: "medium" }),
      d.toLocaleDateString("nl-NL", { dateStyle: "medium" }),
      d.toLocaleDateString("sv-SE", { dateStyle: "medium" }),
      d.toLocaleDateString("pl-PL", { dateStyle: "medium" }),
      d.toLocaleDateString("ru-RU", { dateStyle: "medium" }),
      d.toLocaleDateString("ja-JP", { dateStyle: "medium" }),
      d.toLocaleDateString("ko-KR", { dateStyle: "medium" }),
      d.toLocaleDateString("zh-CN", { dateStyle: "medium" }),
      d.toLocaleDateString("hi-IN", { dateStyle: "medium" }),
    ].join(" ; ") + "\n";
  }
  return out;
}

export function dateStyle_short(): string {
  let out = "";
  for (const t of INSTANTS) {
    const d = new Date(t);
    out += [
      d.toLocaleDateString("en-US", { dateStyle: "short" }),
      d.toLocaleDateString("en-GB", { dateStyle: "short" }),
      d.toLocaleDateString("de-DE", { dateStyle: "short" }),
      d.toLocaleDateString("fr-FR", { dateStyle: "short" }),
      d.toLocaleDateString("es-ES", { dateStyle: "short" }),
      d.toLocaleDateString("it-IT", { dateStyle: "short" }),
      d.toLocaleDateString("pt-BR", { dateStyle: "short" }),
      d.toLocaleDateString("nl-NL", { dateStyle: "short" }),
      d.toLocaleDateString("sv-SE", { dateStyle: "short" }),
      d.toLocaleDateString("ru-RU", { dateStyle: "short" }),
      d.toLocaleDateString("ja-JP", { dateStyle: "short" }),
      d.toLocaleDateString("ko-KR", { dateStyle: "short" }),
      d.toLocaleDateString("zh-CN", { dateStyle: "short" }),
      d.toLocaleDateString("hi-IN", { dateStyle: "short" }),
    ].join(" ; ") + "\n";
  }
  return out;
}

export function timeStyle_full(): string {
  let out = "";
  for (const t of INSTANTS) {
    const d = new Date(t);
    out += [
      d.toLocaleTimeString("en-US", { timeStyle: "full" }),
      d.toLocaleTimeString("en-GB", { timeStyle: "full" }),
      d.toLocaleTimeString("de-DE", { timeStyle: "full" }),
      d.toLocaleTimeString("fr-FR", { timeStyle: "full" }),
      d.toLocaleTimeString("it-IT", { timeStyle: "full" }),
      d.toLocaleTimeString("pt-BR", { timeStyle: "full" }),
      d.toLocaleTimeString("nl-NL", { timeStyle: "full" }),
      d.toLocaleTimeString("sv-SE", { timeStyle: "full" }),
      d.toLocaleTimeString("pl-PL", { timeStyle: "full" }),
      d.toLocaleTimeString("ru-RU", { timeStyle: "full" }),
      d.toLocaleTimeString("hi-IN", { timeStyle: "full" }),
    ].join(" ; ") + "\n";
  }
  return out;
}

export function timeStyle_long(): string {
  let out = "";
  for (const t of INSTANTS) {
    const d = new Date(t);
    out += [
      d.toLocaleTimeString("en-US", { timeStyle: "long" }),
      d.toLocaleTimeString("en-GB", { timeStyle: "long" }),
      d.toLocaleTimeString("de-DE", { timeStyle: "long" }),
      d.toLocaleTimeString("it-IT", { timeStyle: "long" }),
      d.toLocaleTimeString("pt-BR", { timeStyle: "long" }),
      d.toLocaleTimeString("nl-NL", { timeStyle: "long" }),
      d.toLocaleTimeString("sv-SE", { timeStyle: "long" }),
      d.toLocaleTimeString("pl-PL", { timeStyle: "long" }),
      d.toLocaleTimeString("ru-RU", { timeStyle: "long" }),
      d.toLocaleTimeString("ja-JP", { timeStyle: "long" }),
      d.toLocaleTimeString("hi-IN", { timeStyle: "long" }),
    ].join(" ; ") + "\n";
  }
  return out;
}

export function timeStyle_medium(): string {
  let out = "";
  for (const t of INSTANTS) {
    const d = new Date(t);
    out += [
      d.toLocaleTimeString("en-US", { timeStyle: "medium" }),
      d.toLocaleTimeString("en-GB", { timeStyle: "medium" }),
      d.toLocaleTimeString("de-DE", { timeStyle: "medium" }),
      d.toLocaleTimeString("fr-FR", { timeStyle: "medium" }),
      d.toLocaleTimeString("it-IT", { timeStyle: "medium" }),
      d.toLocaleTimeString("pt-BR", { timeStyle: "medium" }),
      d.toLocaleTimeString("nl-NL", { timeStyle: "medium" }),
      d.toLocaleTimeString("sv-SE", { timeStyle: "medium" }),
      d.toLocaleTimeString("pl-PL", { timeStyle: "medium" }),
      d.toLocaleTimeString("ru-RU", { timeStyle: "medium" }),
      d.toLocaleTimeString("ja-JP", { timeStyle: "medium" }),
      d.toLocaleTimeString("ko-KR", { timeStyle: "medium" }),
      d.toLocaleTimeString("zh-CN", { timeStyle: "medium" }),
      d.toLocaleTimeString("hi-IN", { timeStyle: "medium" }),
    ].join(" ; ") + "\n";
  }
  return out;
}

export function timeStyle_short(): string {
  let out = "";
  for (const t of INSTANTS) {
    const d = new Date(t);
    out += [
      d.toLocaleTimeString("en-US", { timeStyle: "short" }),
      d.toLocaleTimeString("en-GB", { timeStyle: "short" }),
      d.toLocaleTimeString("de-DE", { timeStyle: "short" }),
      d.toLocaleTimeString("fr-FR", { timeStyle: "short" }),
      d.toLocaleTimeString("it-IT", { timeStyle: "short" }),
      d.toLocaleTimeString("pt-BR", { timeStyle: "short" }),
      d.toLocaleTimeString("nl-NL", { timeStyle: "short" }),
      d.toLocaleTimeString("sv-SE", { timeStyle: "short" }),
      d.toLocaleTimeString("pl-PL", { timeStyle: "short" }),
      d.toLocaleTimeString("ru-RU", { timeStyle: "short" }),
      d.toLocaleTimeString("ja-JP", { timeStyle: "short" }),
      d.toLocaleTimeString("ko-KR", { timeStyle: "short" }),
      d.toLocaleTimeString("zh-CN", { timeStyle: "short" }),
      d.toLocaleTimeString("hi-IN", { timeStyle: "short" }),
    ].join(" ; ") + "\n";
  }
  return out;
}

export function styleCombos(): string {
  let out = "";
  for (const t of INSTANTS) {
    const d = new Date(t);
    out += [
      d.toLocaleString("en-US", { dateStyle: "full", timeStyle: "short" }),
      d.toLocaleString("en-US", { dateStyle: "long", timeStyle: "medium" }),
      d.toLocaleString("en-US", { dateStyle: "medium", timeStyle: "medium" }),
      d.toLocaleString("en-US", { dateStyle: "short", timeStyle: "short" }),
      d.toLocaleString("en-US", { dateStyle: "full", timeStyle: "full" }),
      d.toLocaleString("en-GB", { dateStyle: "full", timeStyle: "short" }),
      d.toLocaleString("en-GB", { dateStyle: "long", timeStyle: "medium" }),
      d.toLocaleString("en-GB", { dateStyle: "medium", timeStyle: "medium" }),
      d.toLocaleString("en-GB", { dateStyle: "short", timeStyle: "short" }),
      d.toLocaleString("en-GB", { dateStyle: "full", timeStyle: "full" }),
      d.toLocaleString("de-DE", { dateStyle: "full", timeStyle: "short" }),
      d.toLocaleString("de-DE", { dateStyle: "long", timeStyle: "medium" }),
      d.toLocaleString("de-DE", { dateStyle: "medium", timeStyle: "medium" }),
      d.toLocaleString("de-DE", { dateStyle: "short", timeStyle: "short" }),
      d.toLocaleString("de-DE", { dateStyle: "full", timeStyle: "full" }),
      d.toLocaleString("fr-FR", { dateStyle: "full", timeStyle: "short" }),
      d.toLocaleString("fr-FR", { dateStyle: "long", timeStyle: "medium" }),
      d.toLocaleString("fr-FR", { dateStyle: "medium", timeStyle: "medium" }),
      d.toLocaleString("fr-FR", { dateStyle: "short", timeStyle: "short" }),
      d.toLocaleString("fr-FR", { dateStyle: "full", timeStyle: "full" }),
      d.toLocaleString("ja-JP", { dateStyle: "full", timeStyle: "short" }),
      d.toLocaleString("ja-JP", { dateStyle: "long", timeStyle: "medium" }),
      d.toLocaleString("ja-JP", { dateStyle: "medium", timeStyle: "medium" }),
      d.toLocaleString("ja-JP", { dateStyle: "short", timeStyle: "short" }),
    ].join(" ; ") + "\n";
  }
  return out;
}

export function componentsEnUs(): string {
  let out = "";
  for (const t of INSTANTS) {
    const d = new Date(t);
    out += [
      d.toLocaleString("en-US", { year: "numeric", month: "long", day: "numeric" }),
      d.toLocaleString("en-US", { month: "short", day: "numeric" }),
      d.toLocaleString("en-US", { weekday: "long" }),
      d.toLocaleString("en-US", { year: "numeric", month: "2-digit", day: "2-digit" }),
      d.toLocaleString("en-US", { month: "long", year: "numeric" }),
      d.toLocaleString("en-US", { weekday: "short", month: "short", day: "numeric" }),
      d.toLocaleString("en-US", { weekday: "long", year: "numeric", month: "long", day: "numeric" }),
      d.toLocaleString("en-US", { year: "numeric" }),
      d.toLocaleString("en-US", { month: "long" }),
      d.toLocaleString("en-US", { day: "numeric" }),
      d.toLocaleString("en-US", { month: "numeric", day: "numeric" }),
      d.toLocaleString("en-US", { year: "2-digit", month: "numeric", day: "numeric" }),
      d.toLocaleString("en-US", { hour: "numeric", minute: "2-digit" }),
      d.toLocaleString("en-US", { hour: "2-digit", minute: "2-digit", hour12: false }),
      d.toLocaleString("en-US", { hour: "numeric", hour12: true }),
      d.toLocaleString("en-US", { hour: "numeric", minute: "numeric", second: "numeric" }),
      d.toLocaleString("en-US", { hour: "numeric", minute: "2-digit", hourCycle: "h23" }),
      d.toLocaleString("en-US", { hour: "numeric", minute: "numeric", second: "numeric", fractionalSecondDigits: 3 }),
    ].join(" ; ") + "\n";
  }
  return out;
}

export function componentsEnGb(): string {
  let out = "";
  for (const t of INSTANTS) {
    const d = new Date(t);
    out += [
      d.toLocaleString("en-GB", { year: "numeric", month: "long", day: "numeric" }),
      d.toLocaleString("en-GB", { month: "short", day: "numeric" }),
      d.toLocaleString("en-GB", { weekday: "long" }),
      d.toLocaleString("en-GB", { year: "numeric", month: "2-digit", day: "2-digit" }),
      d.toLocaleString("en-GB", { month: "long", year: "numeric" }),
      d.toLocaleString("en-GB", { weekday: "short", month: "short", day: "numeric" }),
      d.toLocaleString("en-GB", { weekday: "long", year: "numeric", month: "long", day: "numeric" }),
      d.toLocaleString("en-GB", { year: "numeric" }),
      d.toLocaleString("en-GB", { month: "long" }),
      d.toLocaleString("en-GB", { day: "numeric" }),
      d.toLocaleString("en-GB", { month: "numeric", day: "numeric" }),
      d.toLocaleString("en-GB", { year: "2-digit", month: "numeric", day: "numeric" }),
      d.toLocaleString("en-GB", { hour: "numeric", minute: "2-digit" }),
      d.toLocaleString("en-GB", { hour: "2-digit", minute: "2-digit", hour12: false }),
      d.toLocaleString("en-GB", { hour: "numeric", hour12: true }),
      d.toLocaleString("en-GB", { hour: "numeric", minute: "numeric", second: "numeric" }),
      d.toLocaleString("en-GB", { hour: "numeric", minute: "2-digit", hourCycle: "h23" }),
      d.toLocaleString("en-GB", { hour: "numeric", minute: "numeric", second: "numeric", fractionalSecondDigits: 3 }),
    ].join(" ; ") + "\n";
  }
  return out;
}

export function componentsDe(): string {
  let out = "";
  for (const t of INSTANTS) {
    const d = new Date(t);
    out += [
      d.toLocaleString("de-DE", { year: "numeric", month: "long", day: "numeric" }),
      d.toLocaleString("de-DE", { weekday: "long" }),
      d.toLocaleString("de-DE", { year: "numeric", month: "2-digit", day: "2-digit" }),
      d.toLocaleString("de-DE", { month: "long", year: "numeric" }),
      d.toLocaleString("de-DE", { weekday: "long", year: "numeric", month: "long", day: "numeric" }),
      d.toLocaleString("de-DE", { year: "numeric" }),
      d.toLocaleString("de-DE", { month: "long" }),
      d.toLocaleString("de-DE", { day: "numeric" }),
      d.toLocaleString("de-DE", { month: "numeric", day: "numeric" }),
      d.toLocaleString("de-DE", { year: "2-digit", month: "numeric", day: "numeric" }),
      d.toLocaleString("de-DE", { hour: "numeric", minute: "2-digit" }),
      d.toLocaleString("de-DE", { hour: "2-digit", minute: "2-digit", hour12: false }),
      d.toLocaleString("de-DE", { hour: "numeric", minute: "numeric", second: "numeric" }),
      d.toLocaleString("de-DE", { hour: "numeric", minute: "2-digit", hourCycle: "h23" }),
      d.toLocaleString("de-DE", { hour: "numeric", minute: "numeric", second: "numeric", fractionalSecondDigits: 3 }),
    ].join(" ; ") + "\n";
  }
  return out;
}

export function componentsFr(): string {
  let out = "";
  for (const t of INSTANTS) {
    const d = new Date(t);
    out += [
      d.toLocaleString("fr-FR", { year: "numeric", month: "long", day: "numeric" }),
      d.toLocaleString("fr-FR", { month: "short", day: "numeric" }),
      d.toLocaleString("fr-FR", { weekday: "long" }),
      d.toLocaleString("fr-FR", { year: "numeric", month: "2-digit", day: "2-digit" }),
      d.toLocaleString("fr-FR", { month: "long", year: "numeric" }),
      d.toLocaleString("fr-FR", { weekday: "short", month: "short", day: "numeric" }),
      d.toLocaleString("fr-FR", { weekday: "long", year: "numeric", month: "long", day: "numeric" }),
      d.toLocaleString("fr-FR", { year: "numeric" }),
      d.toLocaleString("fr-FR", { month: "long" }),
      d.toLocaleString("fr-FR", { day: "numeric" }),
      d.toLocaleString("fr-FR", { month: "numeric", day: "numeric" }),
      d.toLocaleString("fr-FR", { year: "2-digit", month: "numeric", day: "numeric" }),
      d.toLocaleString("fr-FR", { hour: "numeric", minute: "2-digit" }),
      d.toLocaleString("fr-FR", { hour: "2-digit", minute: "2-digit", hour12: false }),
      d.toLocaleString("fr-FR", { hour: "numeric", hour12: true }),
      d.toLocaleString("fr-FR", { hour: "numeric", minute: "numeric", second: "numeric" }),
      d.toLocaleString("fr-FR", { hour: "numeric", minute: "2-digit", hourCycle: "h23" }),
      d.toLocaleString("fr-FR", { hour: "numeric", minute: "numeric", second: "numeric", fractionalSecondDigits: 3 }),
    ].join(" ; ") + "\n";
  }
  return out;
}

export function componentsJa(): string {
  let out = "";
  for (const t of INSTANTS) {
    const d = new Date(t);
    out += [
      d.toLocaleString("ja-JP", { year: "numeric", month: "long", day: "numeric" }),
      d.toLocaleString("ja-JP", { weekday: "long" }),
      d.toLocaleString("ja-JP", { year: "numeric", month: "2-digit", day: "2-digit" }),
      d.toLocaleString("ja-JP", { month: "long", year: "numeric" }),
      d.toLocaleString("ja-JP", { weekday: "long", year: "numeric", month: "long", day: "numeric" }),
      d.toLocaleString("ja-JP", { year: "numeric" }),
      d.toLocaleString("ja-JP", { month: "long" }),
      d.toLocaleString("ja-JP", { day: "numeric" }),
      d.toLocaleString("ja-JP", { month: "numeric", day: "numeric" }),
      d.toLocaleString("ja-JP", { year: "2-digit", month: "numeric", day: "numeric" }),
      d.toLocaleString("ja-JP", { hour: "numeric", minute: "2-digit" }),
      d.toLocaleString("ja-JP", { hour: "2-digit", minute: "2-digit", hour12: false }),
      d.toLocaleString("ja-JP", { hour: "numeric", minute: "numeric", second: "numeric" }),
      d.toLocaleString("ja-JP", { hour: "numeric", minute: "2-digit", hourCycle: "h23" }),
      d.toLocaleString("ja-JP", { hour: "numeric", minute: "numeric", second: "numeric", fractionalSecondDigits: 3 }),
    ].join(" ; ") + "\n";
  }
  return out;
}

export function timeZoneNames(): string {
  let out = "";
  for (const t of INSTANTS) {
    const d = new Date(t);
    out += [
      d.toLocaleString("en-US", { timeZoneName: "short" }),
      d.toLocaleString("en-US", { timeZoneName: "long" }),
      d.toLocaleString("en-US", { timeZoneName: "shortOffset" }),
      d.toLocaleString("en-US", { timeZoneName: "longOffset" }),
      d.toLocaleTimeString("en-US", { hour: "numeric", minute: "2-digit", timeZoneName: "short" }),
      d.toLocaleTimeString("en-US", { hour: "numeric", minute: "2-digit", timeZoneName: "long" }),
      d.toLocaleTimeString("en-US", { hour: "numeric", minute: "2-digit", timeZoneName: "shortOffset" }),
      d.toLocaleTimeString("en-US", { hour: "numeric", minute: "2-digit", timeZoneName: "longOffset" }),
      d.toLocaleString("de-DE", { timeStyle: "full", dateStyle: "short" }),
    ].join(" ; ") + "\n";
  }
  return out;
}

export function timeZoneOption(): string {
  let out = "";
  for (const t of INSTANTS) {
    const d = new Date(t);
    out += [
      d.toLocaleString("en-US", { timeZone: "UTC" }),
      d.toLocaleString("en-US", { timeZone: "Asia/Tokyo" }),
      d.toLocaleString("en-US", { timeZone: "Australia/Lord_Howe", timeStyle: "full" }),
      d.toLocaleDateString("en-US", { timeZone: "Pacific/Kiritimati" }),
    ].join(" ; ") + "\n";
  }
  return out;
}

export function formatObject(): string {
  const a = new Intl.DateTimeFormat("en-US", { dateStyle: "medium", timeStyle: "short" });
  const b = new Intl.DateTimeFormat("de-DE");
  const c = new Intl.DateTimeFormat("en-GB");
  const d = new Date(1792405800123);
  const fmt = a.format;
  return [a.format(d), b.format(d), c.format(d), fmt(d), [new Date(0), d].map(b.format).join(",")].join("|");
}

export function invalidDate(): string {
  const bad = new Date(NaN);
  return [bad.toLocaleString("en-US"), bad.toLocaleDateString("en-US"), bad.toLocaleTimeString("en-US"), fail(() => new Intl.DateTimeFormat("en-US").format(bad))].join("|");
}

export function errors(): string {
  const d = new Date(0);
  return [
    fail(() => d.toLocaleString("en-US", { dateStyle: "huge" })),
    fail(() => d.toLocaleString("en-US", { month: "huge" })),
    fail(() => d.toLocaleString("en-US", { timeZone: "Not/AZone" })),
    fail(() => d.toLocaleString("en-US", { dateStyle: "short", year: "numeric" })),
    fail(() => d.toLocaleDateString("en-US", { timeStyle: "short" })),
    fail(() => d.toLocaleTimeString("en-US", { dateStyle: "short" })),
    fail(() => d.toLocaleString("not a tag")),
    fail(() => d.toLocaleString("en-US", { hourCycle: "h99" })),
  ].join("\n");
}
