// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// Temporal.ZonedDateTime on IANA zones. Every exported function without parameters returns a
// string; `gen_zoned_expected.ts` runs this file in bun.

export function parseAndPrint(): string {
  return [
    Temporal.ZonedDateTime.from("2024-03-15T12:30:00+01:00[Europe/Paris]").toString(),
    Temporal.ZonedDateTime.from("2024-07-15T12:30:00[Europe/Paris]").toString(),
    Temporal.ZonedDateTime.from("2024-01-15T00:00:00Z[UTC]").toString(),
    Temporal.ZonedDateTime.from("2024-03-15T12:30:45.123[Asia/Kolkata]").toString(),
    Temporal.ZonedDateTime.from("2024-03-15T12:30:00[america/new_york]").toString(),
    Temporal.ZonedDateTime.from("2024-03-15T12:30:00+05:30[+05:30]").toString(),
  ].join(",");
}

export function fields(): string {
  const z = Temporal.ZonedDateTime.from("2024-03-15T12:30:45.5+01:00[Europe/Paris]");
  return [z.year, z.month, z.day, z.hour, z.minute, z.second, z.millisecond, z.dayOfWeek, z.dayOfYear, z.epochMilliseconds].join(",") + "|" + z.offset + "|" + z.timeZoneId + "|" + z.offsetNanoseconds + "|" + z.epochNanoseconds.toString();
}

export function springForward(): string {
  const z = Temporal.ZonedDateTime.from("2024-03-10T01:30:00[America/New_York]");
  return [
    z.add({ hours: 1 }).toString(),
    z.add({ days: 1 }).toString(),
    z.add({ minutes: 30 }).toString(),
    z.add({ hours: 24 }).toString(),
    Temporal.ZonedDateTime.from("2024-03-10T02:30:00[America/New_York]").toString(),
    String(z.hoursInDay),
    z.startOfDay().toString(),
  ].join(",");
}

export function fallBack(): string {
  const z = Temporal.ZonedDateTime.from("2024-11-03T01:30:00[America/New_York]");
  const later = Temporal.ZonedDateTime.from("2024-11-03T01:30:00-05:00[America/New_York]");
  return [z.toString(), later.toString(), z.add({ hours: 1 }).toString(), String(z.hoursInDay), z.startOfDay().toString(), z.toInstant().toString()].join(",");
}

export function convert(): string {
  const z = Temporal.ZonedDateTime.from("2024-03-10T01:30:00[America/New_York]");
  return [
    z.withTimeZone("Asia/Tokyo").toString(),
    z.toInstant().toString(),
    z.toPlainDateTime().toString(),
    z.toPlainDate().toString(),
    z.toPlainTime().toString(),
    Temporal.Instant.from("2024-03-15T12:00:00Z").toZonedDateTimeISO("Asia/Kolkata").toString(),
    Temporal.PlainDateTime.from("2024-03-15T12:00").toZonedDateTime("Europe/London").toString(),
    Temporal.PlainDateTime.from("2024-07-15T12:00").toZonedDateTime("Europe/London").toString(),
  ].join(",");
}

export function arithmetic(): string {
  const a = Temporal.ZonedDateTime.from("2024-03-15T12:00:00+01:00[Europe/Paris]");
  const b = Temporal.ZonedDateTime.from("2024-03-16T14:30:00+01:00[Europe/Paris]");
  return [
    a.until(b).toString(),
    b.since(a).toString(),
    a.until(b, { largestUnit: "minute" }).toString(),
    a.add({ months: 1, hours: 2 }).toString(),
    a.subtract({ years: 1, days: 15 }).toString(),
    a.with({ hour: 8, minute: 15 }).toString(),
    a.with({ day: 31 }).toString(),
    String(Temporal.ZonedDateTime.compare(a, b)),
    String(a.equals(a) ? 1 : 0),
    a.until(b, { largestUnit: "day" }).toString(),
  ].join(",");
}

export function fromFields(): string {
  const z = Temporal.ZonedDateTime.from({ year: 2024, month: 3, day: 15, hour: 12, minute: 30, timeZone: "Europe/Paris" });
  const y = Temporal.ZonedDateTime.from({ year: 2024, month: 12, day: 25, timeZone: "America/Los_Angeles" });
  return z.toString() + "," + y.toString();
}

export function errors(): string {
  const out: string[] = [];
  for (const s of ["2024-03-15T12:30:00[Mars/Base]", "2024-03-15T12:30:00", "2024-03-15T12:30:00+05:00[Europe/Paris]"]) {
    try {
      Temporal.ZonedDateTime.from(s);
      out.push("ok");
    } catch (e) {
      out.push((e as Error).name);
    }
  }
  return out.join(",");
}

export function nowZoned(): string {
  const z = Temporal.Now.zonedDateTimeISO("Asia/Tokyo");
  return (z.year >= 2024 ? 1 : 0) + "," + z.timeZoneId + "," + z.offset;
}
