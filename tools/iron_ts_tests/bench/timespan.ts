// SPDX-License-Identifier: Apache-2.0

// TimeSpan, compiled against the native API and run in bun against js/engine/src/time.ts.

function ms(t: TimeSpan): string {
  return String(t.totalMilliseconds);
}

export function construction(): string {
  const a = new TimeSpan(1500.4);
  const b = new TimeSpan(-2.5);
  const c = new TimeSpan(2.5);
  const d = TimeSpan.of(1, 2, 3, 4, 5);
  const e = TimeSpan.of(2);
  const f = TimeSpan.of(0, 0, 0, 1.0004);
  return [ms(a), ms(b), ms(c), ms(d), ms(e), ms(f), ms(TimeSpan.zero)].join(",");
}

export function factories(): string {
  return [
    ms(TimeSpan.fromDays(0.5)),
    ms(TimeSpan.fromHours(25)),
    ms(TimeSpan.fromMinutes(1.5)),
    ms(TimeSpan.fromSeconds(90.0004)),
    ms(TimeSpan.fromSeconds(-0.0005)),
    ms(TimeSpan.fromMilliseconds(12.5)),
  ].join(",");
}

export function parts(): string {
  const t = TimeSpan.of(3, 4, 5, 6, 789);
  const n = t.negate();
  let s = `${t.days},${t.hours},${t.minutes},${t.seconds},${t.milliseconds}`;
  s += `|${n.days},${n.hours},${n.minutes},${n.seconds},${n.milliseconds}`;
  s += `|${t.totalDays.toFixed(6)},${t.totalHours.toFixed(5)},${t.totalMinutes.toFixed(4)},${t.totalSeconds.toFixed(3)}`;
  return s;
}

export function arithmetic(): string {
  const a = TimeSpan.of(0, 1, 30);
  const b = TimeSpan.fromMinutes(45);
  return [
    ms(a.add(b)),
    ms(a.subtract(b)),
    ms(b.subtract(a)),
    ms(a.multiply(1.5)),
    ms(a.multiply(0.0001)),
    ms(a.divide(7)),
    ms(a.negate()),
    ms(b.subtract(a).duration()),
    String(a.compareTo(b)),
    String(b.compareTo(a)),
    String(a.compareTo(a)),
    String(a.equals(TimeSpan.fromMinutes(90))),
    String(a.equals(b)),
  ].join(",");
}

export function formats(): string {
  const t = TimeSpan.of(2, 3, 4, 5, 60);
  const n = TimeSpan.of(0, 0, 0, 59).negate();
  const day = TimeSpan.of(10, 23, 59, 59, 999);
  return [
    t.toString(),
    n.toString(),
    day.toString(),
    TimeSpan.zero.toString(),
    t.toString("c"),
    t.toString("d\\.hh\\:mm"),
    t.toString("hh':'mm':'ss"),
    t.toString("h m s"),
    t.toString("fff"),
    t.toString("FFF"),
    t.toString("ffffffff"),
    n.toString("hh:mm:ss"),
    day.toString("dddd 'days' hh"),
    t.toString("x yy"),
  ].join("|");
}

export function parsing(): string {
  return [
    ms(TimeSpan.parse("1.02:03:04.5")),
    ms(TimeSpan.parse("-00:00:30")),
    ms(TimeSpan.parse("5:07")),
    ms(TimeSpan.parse(" 12:34:56.1234567 ")),
    ms(TimeSpan.parse("10.23:59")),
    ms(TimeSpan.parse("0:0:0.9")),
    ms(TimeSpan.parse("-3.04:05:06")),
  ].join(",");
}
