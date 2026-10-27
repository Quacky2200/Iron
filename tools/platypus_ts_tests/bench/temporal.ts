// SPDX-License-Identifier: Apache-2.0

// Temporal (ISO calendar): Duration, PlainDate, PlainTime, PlainDateTime, Instant. Every exported
// function without parameters returns a string; `gen_temporal_expected.ts` runs this file in bun.

export function dateFields(): string {
  const d = Temporal.PlainDate.from("2024-03-15");
  return [d.year, d.month, d.day, d.dayOfWeek, d.dayOfYear, d.weekOfYear, d.yearOfWeek, d.daysInMonth, d.daysInYear, d.monthsInYear, d.inLeapYear ? 1 : 0].join(",") + "|" + d.monthCode + "|" + d.calendarId + "|" + d.toString();
}

export function dateWeekYears(): string {
  const out: string[] = [];
  for (const s of ["2024-12-30", "2021-01-03", "2020-12-31", "2016-01-01", "2015-12-31", "2026-01-01"]) {
    const d = Temporal.PlainDate.from(s);
    out.push(d.weekOfYear + "/" + d.yearOfWeek + "/" + d.dayOfWeek);
  }
  return out.join(",");
}

export function dateConstrain(): string {
  const a = Temporal.PlainDate.from({ year: 2024, month: 13, day: 40 });
  const b = Temporal.PlainDate.from({ year: 2023, month: 2, day: 31 });
  return a.toString() + "," + b.toString();
}

export function dateAdd(): string {
  const d = Temporal.PlainDate.from("2024-01-31");
  return [
    d.add({ months: 1 }).toString(),
    d.add({ months: 13 }).toString(),
    d.add({ years: 1, days: 1 }).toString(),
    d.add({ weeks: 2 }).toString(),
    d.subtract({ months: 2 }).toString(),
    d.subtract({ years: 4 }).toString(),
    d.add({ hours: 48 }).toString(),
    d.add({ days: -31 }).toString(),
    Temporal.PlainDate.from("2024-02-29").add({ years: 1 }).toString(),
  ].join(",");
}

export function dateWith(): string {
  const d = Temporal.PlainDate.from("2024-03-15");
  return [d.with({ day: 1 }).toString(), d.with({ month: 2, day: 31 }).toString(), d.with({ year: 2023 }).toString()].join(",");
}

export function dateUntil(): string {
  const a = Temporal.PlainDate.from("2024-01-31");
  const b = Temporal.PlainDate.from("2024-03-01");
  return [
    a.until(b).toString(),
    a.until(b, { largestUnit: "month" }).toString(),
    a.until(b, { largestUnit: "week" }).toString(),
    a.until(b, { largestUnit: "year" }).toString(),
    b.until(a).toString(),
    b.until(a, { largestUnit: "month" }).toString(),
    a.since(b).toString(),
    Temporal.PlainDate.from("2020-02-29").until("2024-02-28", { largestUnit: "year" }).toString(),
    Temporal.PlainDate.from("2000-01-01").until("2024-06-15", { largestUnit: "year" }).toString(),
  ].join(",");
}

export function dateCompare(): string {
  const a = Temporal.PlainDate.from("2024-01-31");
  const b = Temporal.PlainDate.from("2024-03-01");
  return [Temporal.PlainDate.compare(a, b), Temporal.PlainDate.compare(b, a), Temporal.PlainDate.compare(a, a), a.equals("2024-01-31") ? 1 : 0, a.equals(b) ? 1 : 0].join(",");
}

export function dateErrors(): string {
  const out: string[] = [];
  for (const s of ["2024-02-30", "2024-13-01", "20240315x", "hello", "2024-3-5"]) {
    try {
      Temporal.PlainDate.from(s);
      out.push("ok");
    } catch (e) {
      out.push((e as Error).name);
    }
  }
  try {
    new Temporal.PlainDate(2023, 2, 29);
    out.push("ok");
  } catch (e) {
    out.push((e as Error).name);
  }
  return out.join(",");
}

export function dateFormats(): string {
  return [
    Temporal.PlainDate.from("20240315").toString(),
    Temporal.PlainDate.from("2024-03-15T10:30:00").toString(),
    Temporal.PlainDate.from("+012024-03-15").toString(),
    new Temporal.PlainDate(5, 1, 2).toString(),
    new Temporal.PlainDate(-44, 3, 15).toString(),
    new Temporal.PlainDate(12345, 3, 15).toString(),
  ].join(",");
}

export function timeBasics(): string {
  const t = Temporal.PlainTime.from("12:30:45.123456789");
  return [t.hour, t.minute, t.second, t.millisecond, t.microsecond, t.nanosecond].join(",") + "|" + t.toString() + "|" + Temporal.PlainTime.from("09:05").toString() + "|" + new Temporal.PlainTime(1, 2, 3, 4).toString();
}

export function timeMath(): string {
  const t = Temporal.PlainTime.from("23:30");
  return [
    t.add({ hours: 1 }).toString(),
    t.add({ minutes: 45 }).toString(),
    t.subtract({ hours: 24 }).toString(),
    t.subtract({ minutes: 31 }).toString(),
    t.add({ seconds: 30, milliseconds: 500 }).toString(),
    t.with({ minute: 5 }).toString(),
    Temporal.PlainTime.from("10:00").until("12:30:15").toString(),
    Temporal.PlainTime.from("10:00").since("12:30:15").toString(),
    Temporal.PlainTime.from("10:00").until("12:30:15", { largestUnit: "minute" }).toString(),
  ].join(",");
}

export function timeCompare(): string {
  const a = Temporal.PlainTime.from("10:00");
  const b = Temporal.PlainTime.from("10:00:00.000000001");
  return [Temporal.PlainTime.compare(a, b), Temporal.PlainTime.compare(b, a), a.equals("10:00") ? 1 : 0].join(",");
}

export function timeErrors(): string {
  const out: string[] = [];
  for (const s of ["24:00", "12:60", "12:30:61x", "nope"]) {
    try {
      Temporal.PlainTime.from(s);
      out.push("ok");
    } catch (e) {
      out.push((e as Error).name);
    }
  }
  return out.join(",");
}

export function dateTimeBasics(): string {
  const dt = Temporal.PlainDateTime.from("2024-03-15T12:30:45.5");
  return [dt.year, dt.month, dt.day, dt.hour, dt.minute, dt.second, dt.millisecond, dt.dayOfWeek, dt.dayOfYear].join(",") + "|" + dt.toString() + "|" + dt.toPlainDate().toString() + "|" + dt.toPlainTime().toString() + "|" + Temporal.PlainDateTime.from("2024-03-15").toString();
}

export function dateTimeMath(): string {
  const dt = Temporal.PlainDateTime.from("2024-01-31T23:30");
  return [
    dt.add({ hours: 1 }).toString(),
    dt.add({ months: 1, minutes: 45 }).toString(),
    dt.subtract({ days: 1, hours: 23, minutes: 31 }).toString(),
    dt.add({ weeks: 1, seconds: 1 }).toString(),
    dt.with({ hour: 0, day: 15 }).toString(),
    dt.subtract({ years: 1 }).toString(),
  ].join(",");
}

export function dateTimeDiff(): string {
  const a = Temporal.PlainDateTime.from("2024-01-31T10:00");
  const b = Temporal.PlainDateTime.from("2024-03-02T08:30");
  return [
    a.until(b).toString(),
    a.until(b, { largestUnit: "hour" }).toString(),
    a.until(b, { largestUnit: "month" }).toString(),
    b.until(a).toString(),
    a.since(b).toString(),
    String(Temporal.PlainDateTime.compare(a, b)),
    String(a.equals(a) ? 1 : 0),
  ].join(",");
}

export function dateToDateTime(): string {
  const d = Temporal.PlainDate.from("2024-03-15");
  return [d.toPlainDateTime().toString(), d.toPlainDateTime("13:45").toString()].join(",");
}

export function durationBasics(): string {
  const d = new Temporal.Duration(1, 2, 3, 4, 5, 6, 7, 8, 9, 10);
  return [d.years, d.months, d.weeks, d.days, d.hours, d.minutes, d.seconds, d.milliseconds, d.microseconds, d.nanoseconds, d.sign, d.blank ? 1 : 0].join(",") + "|" + d.toString();
}

export function durationStrings(): string {
  return [
    Temporal.Duration.from("P1Y2M3W4DT5H6M7.008S").toString(),
    Temporal.Duration.from("PT1.5S").toString(),
    Temporal.Duration.from("PT0S").toString(),
    Temporal.Duration.from("-P1D").toString(),
    Temporal.Duration.from("PT36H").toString(),
    Temporal.Duration.from("P1DT0.5H").toString(),
    Temporal.Duration.from({ milliseconds: 1500 }).toString(),
    Temporal.Duration.from({ hours: 1, minutes: 90 }).toString(),
    new Temporal.Duration(0, 0, 0, 0, 0, 0, 0, 0, 0, 1).toString(),
    new Temporal.Duration(0, 0, 0, 0, 0, 0, 0, 0, 1500, 2).toString(),
  ].join(",");
}

export function durationMath(): string {
  const a = Temporal.Duration.from({ hours: 1, minutes: 30 });
  return [
    a.add({ minutes: 45 }).toString(),
    a.subtract({ hours: 2 }).toString(),
    a.negated().toString(),
    a.negated().abs().toString(),
    Temporal.Duration.from({ minutes: 90 }).add({ minutes: 30 }).toString(),
    Temporal.Duration.from({ days: 1 }).add({ hours: 25 }).toString(),
    a.with({ minutes: 5 }).toString(),
    Temporal.Duration.from({ seconds: 90 }).add({ minutes: 1 }).toString(),
  ].join(",");
}

export function durationTotal(): string {
  const a = Temporal.Duration.from({ hours: 25, minutes: 61 });
  return [a.total({ unit: "days" }), a.total({ unit: "hours" }), a.total({ unit: "minutes" }), a.total({ unit: "seconds" }), Temporal.Duration.from({ milliseconds: 1500 }).total({ unit: "seconds" })].join(",");
}

export function durationErrors(): string {
  const out: string[] = [];
  try {
    new Temporal.Duration(1, -1);
    out.push("ok");
  } catch (e) {
    out.push((e as Error).name);
  }
  try {
    new Temporal.Duration(1.5);
    out.push("ok");
  } catch (e) {
    out.push((e as Error).name);
  }
  try {
    Temporal.Duration.from("P1Y").add({ days: 1 });
    out.push("ok");
  } catch (e) {
    out.push((e as Error).name);
  }
  try {
    Temporal.Duration.from("1D");
    out.push("ok");
  } catch (e) {
    out.push((e as Error).name);
  }
  return out.join(",");
}

export function instantBasics(): string {
  const i = Temporal.Instant.from("2024-03-15T12:30:45.123456789Z");
  return [String(i.epochMilliseconds), i.epochNanoseconds.toString()].join(",") + "|" + i.toString() + "|" + Temporal.Instant.from("2024-03-15T12:30:45Z").toString() + "|" + Temporal.Instant.from("2024-03-15T12:30:45.5Z").toString() + "|" + Temporal.Instant.from("1969-12-31T23:59:59.999Z").epochMilliseconds;
}

export function instantOffsets(): string {
  return [
    Temporal.Instant.from("2024-03-15T12:30:00+02:00").toString(),
    Temporal.Instant.from("2024-03-15T12:30:00-05:30").toString(),
    Temporal.Instant.from("2024-03-15T00:30:00+01:00").toString(),
  ].join(",");
}

export function instantMath(): string {
  const i = Temporal.Instant.from("2024-03-15T12:30:00Z");
  const j = Temporal.Instant.from("2024-03-16T14:45:30.25Z");
  return [
    i.add({ hours: 12 }).toString(),
    i.subtract({ minutes: 31 }).toString(),
    i.until(j).toString(),
    i.until(j, { largestUnit: "minute" }).toString(),
    j.since(i).toString(),
    i.until(j, { largestUnit: "hour" }).toString(),
    String(Temporal.Instant.compare(i, j)),
    String(i.equals(i) ? 1 : 0),
    Temporal.Instant.fromEpochMilliseconds(0).toString(),
    Temporal.Instant.fromEpochMilliseconds(1700000000123).toString(),
  ].join(",");
}

export function instantErrors(): string {
  const out: string[] = [];
  for (const s of ["2024-03-15T12:30:00", "2024-03-15", "bad"]) {
    try {
      Temporal.Instant.from(s);
      out.push("ok");
    } catch (e) {
      out.push((e as Error).name);
    }
  }
  try {
    Temporal.Instant.from("2024-03-15T12:30:00Z").add({ years: 1 });
    out.push("ok");
  } catch (e) {
    out.push((e as Error).name);
  }
  return out.join(",");
}

export function nowIsSane(): string {
  const a = Temporal.Now.instant();
  const d = Temporal.Now.plainDateISO();
  const dt = Temporal.Now.plainDateTimeISO();
  const t = Temporal.Now.plainTimeISO();
  return (a.epochMilliseconds > 1.7e12 ? 1 : 0) + "," + (d.year >= 2024 ? 1 : 0) + "," + (dt.year >= 2024 ? 1 : 0) + "," + (t.hour >= 0 ? 1 : 0);
}
