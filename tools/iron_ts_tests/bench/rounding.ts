// SPDX-License-Identifier: Apache-2.0

// Temporal rounding: PlainTime, PlainDateTime, Instant, ZonedDateTime and Duration `round`. Every
// exported function without parameters returns a string; `gen_rounding_expected.ts` runs this in bun.

export function timeUnits(): string {
  const t = Temporal.PlainTime.from("12:34:56.789012345");
  return [
    t.round({ smallestUnit: "hour" }).toString(),
    t.round({ smallestUnit: "minute" }).toString(),
    t.round({ smallestUnit: "second" }).toString(),
    t.round({ smallestUnit: "millisecond" }).toString(),
    t.round({ smallestUnit: "microsecond" }).toString(),
    t.round("minute").toString(),
    Temporal.PlainTime.from("23:59:59.9").round({ smallestUnit: "second" }).toString(),
  ].join(",");
}

export function timeModes(): string {
  const t = Temporal.PlainTime.from("10:30:30");
  const u = Temporal.PlainTime.from("10:31:30");
  const out: string[] = [];
  for (const m of ["ceil", "floor", "trunc", "expand", "halfCeil", "halfFloor", "halfTrunc", "halfExpand", "halfEven"]) {
    out.push(t.round({ smallestUnit: "minute", roundingMode: m }).toString().slice(0, 5) + "/" + u.round({ smallestUnit: "minute", roundingMode: m }).toString().slice(0, 5));
  }
  return out.join(",");
}

export function timeIncrements(): string {
  const t = Temporal.PlainTime.from("12:37:20");
  return [
    t.round({ smallestUnit: "minute", roundingIncrement: 15 }).toString(),
    t.round({ smallestUnit: "minute", roundingIncrement: 30, roundingMode: "floor" }).toString(),
    t.round({ smallestUnit: "second", roundingIncrement: 20 }).toString(),
    t.round({ smallestUnit: "hour", roundingIncrement: 6 }).toString(),
  ].join(",");
}

export function dateTime(): string {
  const dt = Temporal.PlainDateTime.from("2024-02-28T23:45:30.5");
  return [
    dt.round({ smallestUnit: "day" }).toString(),
    dt.round({ smallestUnit: "hour" }).toString(),
    dt.round({ smallestUnit: "minute", roundingMode: "floor" }).toString(),
    Temporal.PlainDateTime.from("2024-02-28T12:00").round({ smallestUnit: "day" }).toString(),
    Temporal.PlainDateTime.from("2024-02-28T11:59:59").round({ smallestUnit: "day" }).toString(),
    Temporal.PlainDateTime.from("2023-12-31T23:59:59.9").round({ smallestUnit: "second" }).toString(),
    Temporal.PlainDateTime.from("2024-02-28T10:00").round({ smallestUnit: "day", roundingMode: "ceil" }).toString(),
  ].join(",");
}

export function instant(): string {
  const i = Temporal.Instant.from("2024-03-15T12:34:56.789Z");
  return [
    i.round({ smallestUnit: "second" }).toString(),
    i.round({ smallestUnit: "minute" }).toString(),
    i.round({ smallestUnit: "hour", roundingMode: "ceil" }).toString(),
    i.round({ smallestUnit: "millisecond", roundingIncrement: 500 }).toString(),
    Temporal.Instant.from("1969-12-31T23:59:59.6Z").round({ smallestUnit: "second" }).toString(),
  ].join(",");
}

export function zoned(): string {
  const z = Temporal.ZonedDateTime.from("2024-03-10T13:20:00[America/New_York]");
  const w = Temporal.ZonedDateTime.from("2024-03-10T11:00:00[America/New_York]");
  return [
    z.round({ smallestUnit: "day" }).toString(),
    w.round({ smallestUnit: "day" }).toString(),
    z.round({ smallestUnit: "hour" }).toString(),
    z.round({ smallestUnit: "minute", roundingIncrement: 15, roundingMode: "floor" }).toString(),
  ].join(",");
}

export function duration(): string {
  const d = Temporal.Duration.from({ hours: 1, minutes: 47, seconds: 30 });
  return [
    d.round({ smallestUnit: "hour" }).toString(),
    d.round({ smallestUnit: "minute" }).toString(),
    d.round({ smallestUnit: "minute", roundingMode: "floor" }).toString(),
    d.round({ smallestUnit: "second", largestUnit: "minute" }).toString(),
    d.round({ largestUnit: "second", smallestUnit: "second" }).toString(),
    Temporal.Duration.from({ hours: 50 }).round({ smallestUnit: "hour", largestUnit: "day" }).toString(),
    Temporal.Duration.from({ milliseconds: 1500 }).round({ smallestUnit: "second" }).toString(),
    Temporal.Duration.from({ minutes: 59, seconds: 59 }).round({ smallestUnit: "minute" }).toString(),
  ].join(",");
}

export function errors(): string {
  const out: string[] = [];
  try {
    Temporal.PlainTime.from("10:00").round({ smallestUnit: "day" });
    out.push("ok");
  } catch (e) {
    out.push((e as Error).name);
  }
  try {
    Temporal.PlainTime.from("10:00").round({ smallestUnit: "minute", roundingMode: "sideways" });
    out.push("ok");
  } catch (e) {
    out.push((e as Error).name);
  }
  try {
    Temporal.PlainTime.from("10:00").round({ roundingMode: "floor" });
    out.push("ok");
  } catch (e) {
    out.push((e as Error).name);
  }
  return out.join(",");
}
