// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// Dates: construction, getters and setters in local time and UTC, parsing, ISO and UTC strings,
// arithmetic. Every exported function without parameters returns a string. `gen_date_expected.ts`
// runs this file in bun once per time zone and `tests/date.rs` compares (the zone list is the same).

function all(d: Date): string {
  return (
    d.getFullYear() + "-" + d.getMonth() + "-" + d.getDate() + " " + d.getHours() + ":" + d.getMinutes() +
    ":" + d.getSeconds() + "." + d.getMilliseconds() + " w" + d.getDay() + " | " + d.getUTCFullYear() +
    "-" + d.getUTCMonth() + "-" + d.getUTCDate() + " " + d.getUTCHours() + ":" + d.getUTCMinutes() + ":" +
    d.getUTCSeconds() + "." + d.getUTCMilliseconds() + " w" + d.getUTCDay() + " | " + d.getTimezoneOffset()
  );
}

export function fieldsOfInstants(): string {
  const times = [0, 1792405800123, 1772955000000, 1793511000000, -62198755200000, 8.64e15, -8.64e15, 951782400000, 1711846800000];
  let out = "";
  for (const t of times) out += all(new Date(t)) + "\n";
  return out;
}

export function utcAndOverflow(): string {
  return [
    Date.UTC(2026, 9, 19, 10, 30, 15, 123), Date.UTC(2026), Date.UTC(99, 0), Date.UTC(100, 0), Date.UTC(2026, 12, 1),
    Date.UTC(2026, -1, 1), Date.UTC(2026, 0, 0), Date.UTC(2026, 0, 32), Date.UTC(2026, 0, 1, 24), Date.UTC(2026, 0, 1, 0, 0, 0, 1500),
    Date.UTC(2026.9, 1.9, 1.9), Date.UTC(NaN), Date.UTC(275760, 8, 13), Date.UTC(275760, 8, 13, 0, 0, 0, 1), Date.UTC(-271821, 3, 20),
    Date.UTC(0, 0), Date.UTC(-1, 0), Date.UTC(1969, 11, 31, 23, 59, 59, 999),
  ].join(",");
}

export function constructLocal(): string {
  const parts = [
    new Date(2026, 9, 19), new Date(2026, 9, 19, 10, 30, 15, 123), new Date(99, 0), new Date(2026, 12, 31), new Date(2026, 0, 0),
    new Date(2026, 2, 8, 2, 30), new Date(2026, 2, 8, 3, 30), new Date(2026, 10, 1, 1, 30), new Date(2026, 10, 1, 0, 59, 59, 999),
    new Date(2026, 2, 29, 1, 30), new Date(2026, 9, 25, 1, 30), new Date(2026, 9, 4, 2, 15), new Date(2026, 3, 5, 1, 45),
    new Date(1970, 0, 1), new Date(1900, 0, 1), new Date(-1, 0, 1), new Date(12345, 6, 1),
  ];
  let out = "";
  for (const d of parts) out += d.getTime() + ",";
  return out;
}

export function constructOther(): string {
  const a = new Date(1792405800000);
  const b = new Date(a);
  const c = new Date("2026-10-19T10:30:00Z");
  const d = new Date(NaN);
  const e = new Date(8.64e15 + 1);
  const f = new Date(1.9);
  const g = new Date(-1.9);
  const h = new Date("nonsense");
  return [a.getTime(), b.getTime(), c.getTime(), d.getTime(), e.getTime(), f.getTime(), g.getTime(), h.getTime()].join(",") + "," + (a === b) + "," + (a.getTime() === b.getTime());
}

export function setters(): string {
  const d = new Date(2026, 0, 31, 10, 30, 15, 123);
  let out = "";
  out += d.setMonth(1) + "|" + all(d) + "\n";
  out += d.setDate(0) + "|" + all(d) + "\n";
  out += d.setHours(25, 61, 61, 1001) + "|" + all(d) + "\n";
  out += d.setMinutes(-1) + "|" + d.setSeconds(90, 5) + "|" + d.setMilliseconds(-5) + "\n";
  out += d.setFullYear(2024, 1, 29) + "|" + all(d) + "\n";
  out += d.setFullYear(2025) + "|" + all(d) + "\n";
  out += d.setUTCHours(23, 59, 59, 999) + "|" + d.setUTCDate(32) + "|" + d.setUTCMonth(13, 1) + "|" + d.setUTCFullYear(1999, 11, 31) + "\n";
  out += d.setUTCMinutes(0, 0, 0) + "|" + d.setUTCSeconds(1) + "|" + d.setUTCMilliseconds(2) + "\n";
  out += d.setTime(86400000) + "|" + d.getTime() + "|" + d.setTime(8.64e15 + 1) + "|" + d.getTime() + "\n";
  const bad = new Date(NaN);
  out += bad.setHours(1) + "|" + bad.setMonth(1) + "|" + bad.setFullYear(2000) + "|" + all(bad) + "\n";
  const x = new Date(2026, 9, 19);
  out += x.setDate(NaN) + "|" + x.getTime() + "\n";
  const dst = new Date(2026, 2, 8, 1, 30);
  out += dst.setHours(2) + "|" + dst.setHours(3) + "|" + dst.setMinutes(30) + "|" + all(dst) + "\n";
  return out;
}

export function isoAndJson(): string {
  const t = [0, 1792405800123, -1, -62198755200000, 8.64e15, -8.64e15, 253402300800000, -62167219200001, 951782400000];
  let out = "";
  for (const v of t) out += new Date(v).toISOString() + " " + new Date(v).toJSON() + "\n";
  out += new Date(NaN).toJSON() + "\n";
  try {
    new Date(NaN).toISOString();
    out += "no error";
  } catch (e) {
    out += (e as Error).name + ": " + (e as Error).message;
  }
  return out;
}

export function strings(): string {
  const t = [0, 1792405800123, -62198755200000, 8.64e15, -8.64e15, 253402300800000, 1772955000000, NaN];
  let out = "";
  for (const v of t) out += new Date(v).toUTCString() + " | " + new Date(v).toDateString() + " | " + new Date(v).toGMTString() + "\n";
  return out;
}

export function parseIso(): string {
  const xs = [
    "2026-10-19", "2026-10", "2026", "2026-10-19T10:30", "2026-10-19T10:30:15", "2026-10-19T10:30:15.5", "2026-10-19T10:30:15.123456Z",
    "2026-10-19T10:30Z", "2026-10-19T10:30:00+05:30", "2026-10-19T10:30:00-0800", "2026-10-19T24:00:00Z", "2026-10-19T24:00:01Z",
    "2026-10-19 10:30", "2026-10-19 10:30:15Z", "2026-10-19t10:30:00z", "+010000-01-01T00:00:00Z", "-000001-01-01T00:00:00Z",
    "-000000-01-01T00:00:00Z", "2026-13-01", "2026-02-30", "2026-02-29", "2026-10-19T25:00", "2026-10-19T10:60", "1970-01-01T00:00:00.000Z",
    "2026-03-08T02:30:00", "2026-11-01T01:30:00", "  2026-10-19  ", "2026-10-19T", "2026-10-19T10", "2026-1", "+275760-09-13T00:00:00.000Z",
    "+275760-09-13T00:00:00.001Z", "20261019", "2026-10-19T10:30:00.Z",
  ];
  return xs.map((s) => Date.parse(s)).join(",");
}

export function parseLegacy(): string {
  const xs = [
    "Oct 19 2026", "October 19, 2026", "19 Oct 2026", "Mon Oct 19 2026", "Mon, 19 Oct 2026 10:30:00 GMT",
    "Mon Oct 19 2026 10:30:00 GMT+0100 (BST)", "Oct 19, 2026 10:30 PM", "Oct 19, 2026 12:30 AM", "Oct 19, 2026 12:30 PM", "10/19/2026",
    "10/19/26", "10/19/49", "10/19/50", "2026/10/19", "2026/10/19 10:30", "10/19/2026 10:30:15", "19 October 2026 10:30 UTC",
    "Oct 19 2026 10:30 EST", "Oct 19 2026 10:30 PDT", "Oct 19 2026 10:30 +0200", "Oct 19 2026 10:30 GMT-0500", "Oct 2026", "2026 Oct 19",
    "12/25/2026 25:00", "foo", "", "Oct 32 2026", "Feb 30 2026", "1/1/1970", "1/1/70", "Jan 1 1970 00:00:00 GMT",
    "Thu, 01 Jan 1970 00:00:00 GMT+0000", "2026-1-5", "2026-10-5T10:30", "Oct 19 2026 10:30:15.250", "6/7/2026 3:04 pm",
    "Sun Nov 01 2026 01:30:00", "13/1/2026", "Octember 1 2026", "Oct 19 2026 13:00 PM", "Wed, 21 Oct 2026 07:28:00 GMT",
  ];
  return xs.map((s) => Date.parse(s)).join(",");
}

export function arithmetic(): string {
  const a = new Date(2026, 9, 19, 10);
  const b = new Date(2026, 9, 20, 10);
  const diff = b.valueOf() - a.valueOf();
  return [
    diff, b - a, +a, Number(b) - Number(a), (b.getTime() - a.getTime()) / 3600000, a.valueOf() * 2,
  ].join(",") + "," + (a < b) + (a > b) + (a <= a) + (a >= b) + (a === a) + (a === new Date(a)) + (a.getTime() === new Date(a).getTime());
}

export function offsets(): string {
  const ts = [Date.UTC(2026, 0, 15), Date.UTC(2026, 6, 15), Date.UTC(2026, 2, 8, 6, 59), Date.UTC(2026, 2, 8, 7), Date.UTC(2026, 10, 1, 5, 59), Date.UTC(2026, 10, 1, 6), 0, -62198755200000, 8.64e15];
  return ts.map((t) => new Date(t).getTimezoneOffset()).join(",") + "," + new Date(NaN).getTimezoneOffset();
}

export function now(): string {
  const a = Date.now();
  const b = new Date().getTime();
  return (a > 1.7e12) + "," + (b >= a) + "," + (b - a < 5000) + "," + (new Date() instanceof Date) + "," + typeof a;
}

export function clipping(): string {
  return [new Date(8.64e15).getTime(), new Date(-8.64e15).getTime(), new Date(8.64e15 + 1).getTime(), new Date(-0).getTime(), new Date(0.9).getTime(), new Date(-0.9).getTime(), 1 / new Date(-0.9).getTime()].join(",");
}

export function toStrings(): string {
  const ts = [0, 1792405800123, 1772955000000, 1783000000000, -62198755200000, 8.64e15, -8.64e15, -2208988800000, 253402300800000, NaN];
  let out = "";
  for (const t of ts) {
    const d = new Date(t);
    out += d.toString() + " | " + d.toTimeString() + " | " + String(d) + " | " + `${d}` + " | " + (d + "!") + " | " + ("<" + d) + ";";
  }
  return out;
}
