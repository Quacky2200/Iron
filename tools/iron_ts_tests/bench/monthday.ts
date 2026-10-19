// Temporal.PlainYearMonth and Temporal.PlainMonthDay. Every exported function without parameters
// returns a string; `gen_monthday_expected.ts` runs this file in bun.

export function yearMonthBasics(): string {
  const ym = Temporal.PlainYearMonth.from("2024-03");
  return [ym.year, ym.month, ym.daysInMonth, ym.daysInYear, ym.monthsInYear, ym.inLeapYear ? 1 : 0].join(",") + "|" + ym.monthCode + "|" + ym.toString() + "|" + new Temporal.PlainYearMonth(2023, 12).toString();
}

export function yearMonthMath(): string {
  const ym = Temporal.PlainYearMonth.from("2024-03");
  return [
    ym.add({ months: 11 }).toString(),
    ym.subtract({ years: 1, months: 3 }).toString(),
    ym.add({ months: -3 }).toString(),
    Temporal.PlainYearMonth.from("2024-01").subtract({ months: 1 }).toString(),
    ym.with({ month: 12 }).toString(),
    ym.with({ year: 1999 }).toString(),
  ].join(",");
}

export function yearMonthDiff(): string {
  const ym = Temporal.PlainYearMonth.from("2024-03");
  return [
    ym.until("2025-01").toString(),
    ym.until("2025-05").toString(),
    ym.until("2025-05", { largestUnit: "month" }).toString(),
    ym.since("2023-12").toString(),
    ym.since("2025-05").toString(),
    ym.until("2023-01", { largestUnit: "month" }).toString(),
    ym.until("2024-03").toString(),
  ].join(",");
}

export function yearMonthCompare(): string {
  const ym = Temporal.PlainYearMonth.from("2024-03");
  return [Temporal.PlainYearMonth.compare(ym, "2024-04"), Temporal.PlainYearMonth.compare(ym, "2024-03"), Temporal.PlainYearMonth.compare(ym, "2023-12"), ym.equals("2024-03") ? 1 : 0, ym.equals("2024-04") ? 1 : 0].join(",");
}

export function yearMonthConversions(): string {
  const ym = Temporal.PlainYearMonth.from("2024-02");
  return [ym.toPlainDate({ day: 31 }).toString(), ym.toPlainDate({ day: 5 }).toString(), Temporal.PlainDate.from("2024-03-15").toPlainYearMonth().toString(), Temporal.PlainYearMonth.from({ year: 2024, month: 13 }).toString()].join(",");
}

export function yearMonthErrors(): string {
  const out: string[] = [];
  for (const s of ["2024-13", "hello", "2024-00"]) {
    try {
      Temporal.PlainYearMonth.from(s);
      out.push("ok");
    } catch (e) {
      out.push((e as Error).name);
    }
  }
  try {
    Temporal.PlainYearMonth.from("2024-03").add({ days: 5 });
    out.push("ok");
  } catch (e) {
    out.push((e as Error).name);
  }
  return out.join(",");
}

export function monthDayBasics(): string {
  const md = Temporal.PlainMonthDay.from("03-15");
  return md.day + "|" + md.monthCode + "|" + md.toString() + "|" + Temporal.PlainMonthDay.from("--02-29").toString() + "|" + Temporal.PlainMonthDay.from({ month: 2, day: 30 }).toString() + "|" + new Temporal.PlainMonthDay(12, 25).toString();
}

export function monthDayMath(): string {
  const md = Temporal.PlainMonthDay.from("03-15");
  return [md.with({ day: 20 }).toString(), md.toPlainDate({ year: 2025 }).toString(), Temporal.PlainMonthDay.from("02-29").toPlainDate({ year: 2023 }).toString(), Temporal.PlainMonthDay.from("02-29").toPlainDate({ year: 2024 }).toString(), md.equals("03-15") ? "1" : "0", md.equals("03-16") ? "1" : "0", Temporal.PlainDate.from("2024-03-15").toPlainMonthDay().toString()].join(",");
}

export function monthDayErrors(): string {
  const out: string[] = [];
  for (const s of ["02-30", "13-01", "x"]) {
    try {
      Temporal.PlainMonthDay.from(s);
      out.push("ok");
    } catch (e) {
      out.push((e as Error).name);
    }
  }
  return out.join(",");
}
