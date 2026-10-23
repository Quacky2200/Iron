// SPDX-License-Identifier: Apache-2.0

import { describe, expect, test } from "bun:test";

import { DateTime, TimeSpan } from "../src/index.ts";

// M5-TIM-01
describe("TimeSpan", () => {
  test("parts, totals, arithmetic and formats (millisecond resolution)", () => {
    const t = TimeSpan.of(1, 2, 3, 4, 5);
    expect([t.days, t.hours, t.minutes, t.seconds, t.milliseconds]).toEqual([1, 2, 3, 4, 5]);
    expect(t.totalHours).toBeCloseTo(26.0511, 3);
    expect(t.toString()).toBe("1.02:03:04.005");
    expect(TimeSpan.fromMinutes(90).toString()).toBe("01:30:00");
    expect(TimeSpan.fromSeconds(-1.5).toString()).toBe("-00:00:01.500");
    expect(TimeSpan.fromSeconds(75).toString("mm\:ss")).toBe("01:15");
    expect(TimeSpan.fromMilliseconds(1234).toString("s'.'fffff")).toBe("1.23400");
    // Sub-millisecond values round to whole milliseconds.
    expect(TimeSpan.fromSeconds(0.0004).totalMilliseconds).toBe(0);
    expect(TimeSpan.parse("1.02:03:04.0059999").equals(t)).toBe(true);
    expect(TimeSpan.parse("-00:30").totalMinutes).toBe(-30);
    expect(() => TimeSpan.parse("25:00")).toThrow();
    expect(t.add(TimeSpan.fromDays(1)).days).toBe(2);
    expect(TimeSpan.fromHours(1).compareTo(TimeSpan.fromMinutes(59))).toBe(1);
  });
});

// M5-TIM-02
describe("DateTime", () => {
  test("UTC fields, months 1-12, calendar arithmetic", () => {
    const d = new DateTime(2024, 1, 31, 13, 5, 9, 7, "utc");
    expect([d.year, d.month, d.day, d.hour, d.minute, d.second, d.millisecond]).toEqual([2024, 1, 31, 13, 5, 9, 7]);
    expect(d.dayOfWeek).toBe(3); // Wednesday
    expect(d.addMonths(1).day).toBe(29); // leap February
    expect(d.addYears(1).addMonths(1).day).toBe(28);
    expect(d.addDays(1).month).toBe(2);
    expect(d.addDays(366).toString("yyyy-MM-dd")).toBe("2025-01-31"); // 2024 is a leap year
    expect(d.date.timeOfDay.equals(TimeSpan.zero)).toBe(true);
    expect(d.timeOfDay.toString()).toBe("13:05:09.007");
    const later = d.add(TimeSpan.fromHours(36));
    expect(later.subtract(d).totalHours).toBe(36);
    expect(later.subtract(TimeSpan.fromHours(36)).equals(d)).toBe(true);
    expect(DateTime.daysInMonth(1900, 2)).toBe(28);
    expect(DateTime.daysInMonth(2000, 2)).toBe(29);
    expect(new DateTime(5, 1, 1, 0, 0, 0, 0, "utc").year).toBe(5);
  });

  test("C# formats", () => {
    const d = new DateTime(2024, 3, 7, 15, 4, 5, 60, "utc");
    expect(d.toString("o")).toBe("2024-03-07T15:04:05.060Z");
    expect(d.toString("s")).toBe("2024-03-07T15:04:05");
    expect(d.toString("u")).toBe("2024-03-07 15:04:05Z");
    expect(d.toString("d")).toBe("03/07/2024");
    expect(d.toString("D")).toBe("Thursday, 07 March 2024");
    expect(d.toString("G")).toBe("03/07/2024 15:04:05");
    expect(d.toString("ddd d MMM yy, h:mm tt")).toBe("Thu 7 Mar 24, 3:04 PM");
    expect(d.toString("HH:mm:ss.fffffff")).toBe("15:04:05.0600000");
    expect(d.toString("HH:mm:ss.FFF")).toBe("15:04:05.06");
    expect(d.toString("'Day' d 'of' yyyy")).toBe("Day 7 of 2024");
  });

  test("JS Date conversion and parsing", () => {
    const js = new Date(Date.UTC(2020, 5, 15, 12, 0, 0));
    const d = DateTime.fromDate(js, "utc");
    expect(d.month).toBe(6);
    expect(d.toDate().getTime()).toBe(js.getTime());
    const p = DateTime.parse("2020-06-15T12:00:00Z");
    expect(p.kind).toBe("utc");
    expect(p.equals(d)).toBe(true);
    // Local and UTC views of one instant are equal.
    expect(d.toLocalTime().equals(d)).toBe(true);
    expect(d.toLocalTime().toUniversalTime().hour).toBe(12);
    expect(() => DateTime.parse("nope")).toThrow();
    expect(Math.abs(DateTime.utcNow.epochMilliseconds - Date.now())).toBeLessThan(1000);
  });
});
