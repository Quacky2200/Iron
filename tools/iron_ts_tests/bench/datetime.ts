// SPDX-License-Identifier: Apache-2.0

// DateTime in New York time (the zone the generator and the test both set), compiled against the
// native API and run in bun against js/engine/src/time.ts.

function iso(d: DateTime): string {
  return d.toString("o");
}

export function fields(): string {
  const d = new DateTime(2024, 1, 31, 13, 45, 30, 123);
  const u = new DateTime(2024, 6, 15, 8, 5, 9, 7, "utc");
  const f = (x: DateTime) =>
    `${x.year}-${x.month}-${x.day} ${x.hour}:${x.minute}:${x.second}.${x.millisecond} dow${x.dayOfWeek} doy${x.dayOfYear} ${x.kind}`;
  return `${f(d)}|${f(u)}|${f(d.date)}|${d.timeOfDay.toString()}|${u.timeOfDay.totalMilliseconds}`;
}

export function epoch(): string {
  const d = new DateTime(2024, 7, 4, 12, 0, 0);
  const u = new DateTime(2024, 7, 4, 12, 0, 0, 0, "utc");
  const e = DateTime.fromEpoch(1700000000123.6);
  const eu = DateTime.fromEpoch(0, "utc");
  return `${d.epochMilliseconds},${u.epochMilliseconds},${e.epochMilliseconds},${iso(e)},${iso(eu)},${eu.kind}`;
}

export function formats(): string {
  const d = new DateTime(2024, 3, 5, 9, 7, 3, 45);
  const p = new DateTime(2024, 12, 25, 21, 30, 59, 900, "utc");
  const out: string[] = [];
  for (const f of ["o", "s", "u", "d", "D", "t", "T", "g", "G", "f", "F"]) out.push(`${d.toString(f)}~${p.toString(f)}`);
  out.push(d.toString());
  out.push(d.toISOString());
  out.push(d.toString("yyyy yy y MMMM MMM MM M dddd ddd dd d"));
  out.push(d.toString("HH H hh h mm m ss s fff ff f FFF F tt t"));
  out.push(p.toString("HH H hh h tt t zzz zz z K"));
  out.push(d.toString("zzz zz z K 'at' \\y yyyy"));
  return out.join("|");
}

export function arithmetic(): string {
  const d = new DateTime(2024, 1, 31, 10, 30);
  const out: string[] = [];
  out.push(iso(d.addMonths(1)));
  out.push(iso(d.addMonths(13)));
  out.push(iso(d.addMonths(-2)));
  out.push(iso(d.addYears(1)));
  out.push(iso(new DateTime(2024, 2, 29, 5).addYears(1)));
  out.push(iso(d.addDays(40)));
  out.push(iso(d.addDays(-31.9)));
  out.push(iso(d.addHours(30)));
  out.push(iso(d.addMinutes(-90)));
  out.push(iso(d.addSeconds(0.5)));
  out.push(iso(d.addMilliseconds(1.5)));
  out.push(iso(d.add(TimeSpan.of(1, 1))));
  out.push(iso(d.subtractTimeSpan(TimeSpan.fromHours(2))));
  const e = new DateTime(2024, 3, 1);
  out.push(e.subtract(d).toString());
  out.push(d.subtract(e).toString());
  out.push(String(d.compareTo(e)));
  out.push(String(e.compareTo(d)));
  out.push(String(d.equals(d.toUniversalTime())));
  out.push(String(d.equals(e)));
  return out.join("|");
}

export function daylightSaving(): string {
  const out: string[] = [];
  // 2024-03-10 02:30 does not exist in New York (clocks jump 02:00 to 03:00).
  out.push(iso(new DateTime(2024, 3, 10, 2, 30)));
  // 2024-11-03 01:30 happens twice; the first one is used.
  out.push(iso(new DateTime(2024, 11, 3, 1, 30)));
  const before = new DateTime(2024, 3, 9, 12, 0);
  out.push(iso(before.addDays(1)));
  out.push(iso(before.addHours(24)));
  const after = new DateTime(2024, 11, 2, 12, 0);
  out.push(iso(after.addDays(1)));
  out.push(iso(after.addHours(24)));
  out.push(iso(new DateTime(2024, 7, 1).toUniversalTime()));
  out.push(iso(new DateTime(2024, 1, 1).toUniversalTime()));
  out.push(iso(new DateTime(2024, 1, 1, 0, 0, 0, 0, "utc").toLocalTime()));
  return out.join("|");
}

export function carrying(): string {
  return [
    iso(new DateTime(2024, 13, 1)),
    iso(new DateTime(2024, 0, 15)),
    iso(new DateTime(2024, 2, 30)),
    iso(new DateTime(2024, 1, 0)),
    iso(new DateTime(2024, 1, 1, 25, 61, 61, 1001)),
    iso(new DateTime(50, 6, 1, 0, 0, 0, 0, "utc")),
    iso(new DateTime(99, 12, 31, 23, 59, 59, 999, "utc")),
    iso(new DateTime(1969, 12, 31, 23, 59, 59, 500, "utc")),
  ].join("|");
}

export function parsing(): string {
  const out: string[] = [];
  for (const t of [
    "2024-05-03",
    "2024-05",
    "2024",
    "2024-05-03T10:20",
    "2024-05-03T10:20:30",
    "2024-05-03T10:20:30.5",
    "2024-05-03T10:20:30.123456",
    "2024-05-03T10:20:30Z",
    "2024-05-03T10:20:30+02:00",
    "2024-05-03T10:20:30-0530",
    "2024-05-03 10:20:30",
    "2024-05-03T00:00:00.000Z",
    "2024-02-29T23:59:59Z",
  ]) {
    const d = DateTime.parse(t);
    out.push(`${d.epochMilliseconds}/${d.kind}`);
  }
  return out.join("|");
}

export function statics(): string {
  return `${DateTime.isLeapYear(2024)},${DateTime.isLeapYear(1900)},${DateTime.isLeapYear(2000)},${DateTime.daysInMonth(2024, 2)},${DateTime.daysInMonth(2023, 2)},${DateTime.daysInMonth(2024, 12)}`;
}
