// C#-style `DateTime` and `TimeSpan` (as in s&box), on top of JS `Date`.
//
// Resolution is one millisecond, like JS `Date` and engine time. There are no
// ticks (100 ns), microseconds or nanoseconds: values are whole milliseconds,
// and `f` format digits beyond three print as zeros. Use them for dates, durations,
// cooldowns and timestamps, not for profiling sub-millisecond work.

const MS_PER_SECOND = 1000;
const MS_PER_MINUTE = 60_000;
const MS_PER_HOUR = 3_600_000;
const MS_PER_DAY = 86_400_000;

/** A length of time, in whole milliseconds (C#'s `TimeSpan`, without ticks). */
export class TimeSpan {
  /** The length in milliseconds (whole, may be negative). */
  readonly totalMilliseconds: number;

  constructor(milliseconds: number) {
    this.totalMilliseconds = Math.round(milliseconds);
  }

  static readonly zero = new TimeSpan(0);

  /** `new TimeSpan(h, m, s)` in C#; also `TimeSpan.of(d, h, m, s, ms)`. */
  static of(days: number, hours = 0, minutes = 0, seconds = 0, milliseconds = 0): TimeSpan {
    return new TimeSpan(days * MS_PER_DAY + hours * MS_PER_HOUR + minutes * MS_PER_MINUTE + seconds * MS_PER_SECOND + milliseconds);
  }
  static fromDays(d: number): TimeSpan {
    return new TimeSpan(d * MS_PER_DAY);
  }
  static fromHours(h: number): TimeSpan {
    return new TimeSpan(h * MS_PER_HOUR);
  }
  static fromMinutes(m: number): TimeSpan {
    return new TimeSpan(m * MS_PER_MINUTE);
  }
  static fromSeconds(s: number): TimeSpan {
    return new TimeSpan(s * MS_PER_SECOND);
  }
  static fromMilliseconds(ms: number): TimeSpan {
    return new TimeSpan(ms);
  }

  /**
   * Parses `[-][d.]hh:mm[:ss[.fff]]` (C#'s constant format). Throws on anything else.
   */
  static parse(text: string): TimeSpan {
    const m = /^\s*(-)?(?:(\d+)\.)?(\d{1,2}):(\d{1,2})(?::(\d{1,2})(?:\.(\d{1,7}))?)?\s*$/.exec(text);
    if (!m) throw new Error(`not a TimeSpan: "${text}"`);
    const [, neg, d, h, min, s, frac] = m;
    if (Number(h) > 23 || Number(min) > 59 || Number(s ?? 0) > 59) throw new Error(`TimeSpan out of range: "${text}"`);
    // Fractions finer than a millisecond are dropped.
    const ms = frac ? Number(frac.padEnd(3, "0").slice(0, 3)) : 0;
    const span = TimeSpan.of(Number(d ?? 0), Number(h), Number(min), Number(s ?? 0), ms);
    return neg ? span.negate() : span;
  }

  private part(unit: number, mod: number): number {
    return Math.trunc(this.totalMilliseconds / unit) % mod;
  }
  /** Whole days (C#'s `Days`). */
  get days(): number {
    return Math.trunc(this.totalMilliseconds / MS_PER_DAY);
  }
  get hours(): number {
    return this.part(MS_PER_HOUR, 24);
  }
  get minutes(): number {
    return this.part(MS_PER_MINUTE, 60);
  }
  get seconds(): number {
    return this.part(MS_PER_SECOND, 60);
  }
  get milliseconds(): number {
    return this.totalMilliseconds % 1000;
  }
  get totalDays(): number {
    return this.totalMilliseconds / MS_PER_DAY;
  }
  get totalHours(): number {
    return this.totalMilliseconds / MS_PER_HOUR;
  }
  get totalMinutes(): number {
    return this.totalMilliseconds / MS_PER_MINUTE;
  }
  get totalSeconds(): number {
    return this.totalMilliseconds / MS_PER_SECOND;
  }

  add(t: TimeSpan): TimeSpan {
    return new TimeSpan(this.totalMilliseconds + t.totalMilliseconds);
  }
  subtract(t: TimeSpan): TimeSpan {
    return new TimeSpan(this.totalMilliseconds - t.totalMilliseconds);
  }
  multiply(factor: number): TimeSpan {
    return new TimeSpan(this.totalMilliseconds * factor);
  }
  divide(divisor: number): TimeSpan {
    return new TimeSpan(this.totalMilliseconds / divisor);
  }
  negate(): TimeSpan {
    return new TimeSpan(-this.totalMilliseconds);
  }
  /** The absolute length. */
  duration(): TimeSpan {
    return new TimeSpan(Math.abs(this.totalMilliseconds));
  }
  compareTo(t: TimeSpan): number {
    return Math.sign(this.totalMilliseconds - t.totalMilliseconds);
  }
  equals(t: TimeSpan): boolean {
    return this.totalMilliseconds === t.totalMilliseconds;
  }

  /**
   * C#'s constant format, `[-][d.]hh:mm:ss[.fff]` (milliseconds only when non-zero), or a
   * custom format with `d`, `h`/`hh`, `m`/`mm`, `s`/`ss`, `f`..`fffffff`, `'literal'`
   * and `\c` (sign isn't printed by custom formats, as in C#).
   */
  toString(format?: string): string {
    const abs = this.duration();
    const pad = (n: number, w = 2) => String(n).padStart(w, "0");
    if (!format || format === "c") {
      const sign = this.totalMilliseconds < 0 ? "-" : "";
      const day = abs.days ? `${abs.days}.` : "";
      const frac = abs.milliseconds ? `.${pad(abs.milliseconds, 3)}` : "";
      return `${sign}${day}${pad(abs.hours)}:${pad(abs.minutes)}:${pad(abs.seconds)}${frac}`;
    }
    return formatTokens(format, (token) => {
      const c = token[0];
      const n = token.length;
      switch (c) {
        case "d":
          return pad(abs.days, n);
        case "h":
          return pad(abs.hours, Math.min(n, 2));
        case "m":
          return pad(abs.minutes, Math.min(n, 2));
        case "s":
          return pad(abs.seconds, Math.min(n, 2));
        case "f":
        case "F":
          return fraction(abs.milliseconds, n, c === "F");
        default:
          return undefined;
      }
    });
  }
}

export type DateTimeKind = "local" | "utc";

const MONTHS = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
const DAYS = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];

/**
 * A point in time with millisecond resolution (C#'s `DateTime`, without ticks), in
 * local time or UTC. Stored as milliseconds since 1970-01-01 UTC, like a JS `Date`.
 * Months are 1 to 12, as in C# (not 0 to 11 as in JS).
 */
export class DateTime {
  /** Milliseconds since the Unix epoch (UTC), as from `Date.getTime()`. */
  readonly epochMilliseconds: number;
  readonly kind: DateTimeKind;

  /** From calendar fields (`month` 1 to 12), in local time unless `kind` says UTC. */
  constructor(
    year: number,
    month = 1,
    day = 1,
    hour = 0,
    minute = 0,
    second = 0,
    millisecond = 0,
    kind: DateTimeKind = "local",
  ) {
    const date =
      kind === "utc"
        ? new Date(Date.UTC(year, month - 1, day, hour, minute, second, millisecond))
        : new Date(year, month - 1, day, hour, minute, second, millisecond);
    // Years 0-99 are taken literally, not as 19xx.
    if (year >= 0 && year < 100) {
      if (kind === "utc") date.setUTCFullYear(year);
      else date.setFullYear(year);
    }
    this.epochMilliseconds = date.getTime();
    this.kind = kind;
    if (Number.isNaN(this.epochMilliseconds)) throw new Error("invalid DateTime");
  }

  /** From epoch milliseconds. */
  static fromEpoch(ms: number, kind: DateTimeKind = "local"): DateTime {
    const dt = Object.create(DateTime.prototype) as DateTime;
    Object.assign(dt, { epochMilliseconds: Math.round(ms), kind });
    return dt;
  }

  /** From a JS `Date` (the same instant). */
  static fromDate(date: Date, kind: DateTimeKind = "local"): DateTime {
    return DateTime.fromEpoch(date.getTime(), kind);
  }

  /** The current local time, from the system clock (milliseconds). */
  static get now(): DateTime {
    return DateTime.fromEpoch(Date.now(), "local");
  }
  static get utcNow(): DateTime {
    return DateTime.fromEpoch(Date.now(), "utc");
  }
  /** Midnight today, local time. */
  static get today(): DateTime {
    return DateTime.now.date;
  }

  /** Parses an ISO 8601 date/time (what `Date.parse` accepts). A `Z` or offset makes it UTC. */
  static parse(text: string): DateTime {
    const ms = Date.parse(text);
    if (Number.isNaN(ms)) throw new Error(`not a DateTime: "${text}"`);
    return DateTime.fromEpoch(ms, /(Z|[+-]\d\d:?\d\d)$/i.test(text.trim()) ? "utc" : "local");
  }

  static isLeapYear(year: number): boolean {
    return (year % 4 === 0 && year % 100 !== 0) || year % 400 === 0;
  }

  static daysInMonth(year: number, month: number): number {
    return [31, DateTime.isLeapYear(year) ? 29 : 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31][month - 1]!;
  }

  /** A JS `Date` for the same instant. */
  toDate(): Date {
    return new Date(this.epochMilliseconds);
  }

  private get d(): Date {
    return new Date(this.epochMilliseconds);
  }
  private get utc(): boolean {
    return this.kind === "utc";
  }

  get year(): number {
    return this.utc ? this.d.getUTCFullYear() : this.d.getFullYear();
  }
  /** 1 to 12. */
  get month(): number {
    return (this.utc ? this.d.getUTCMonth() : this.d.getMonth()) + 1;
  }
  get day(): number {
    return this.utc ? this.d.getUTCDate() : this.d.getDate();
  }
  get hour(): number {
    return this.utc ? this.d.getUTCHours() : this.d.getHours();
  }
  get minute(): number {
    return this.utc ? this.d.getUTCMinutes() : this.d.getMinutes();
  }
  get second(): number {
    return this.utc ? this.d.getUTCSeconds() : this.d.getSeconds();
  }
  get millisecond(): number {
    return this.utc ? this.d.getUTCMilliseconds() : this.d.getMilliseconds();
  }
  /** 0 (Sunday) to 6 (Saturday), like C#'s `DayOfWeek`. */
  get dayOfWeek(): number {
    return this.utc ? this.d.getUTCDay() : this.d.getDay();
  }
  /** 1 to 366. */
  get dayOfYear(): number {
    let n = this.day;
    for (let m = 1; m < this.month; m++) n += DateTime.daysInMonth(this.year, m);
    return n;
  }
  /** The same day at midnight. */
  get date(): DateTime {
    return new DateTime(this.year, this.month, this.day, 0, 0, 0, 0, this.kind);
  }
  /** Time since midnight. */
  get timeOfDay(): TimeSpan {
    return TimeSpan.of(0, this.hour, this.minute, this.second, this.millisecond);
  }

  add(t: TimeSpan): DateTime {
    return DateTime.fromEpoch(this.epochMilliseconds + t.totalMilliseconds, this.kind);
  }
  addMilliseconds(ms: number): DateTime {
    return this.add(new TimeSpan(ms));
  }
  addSeconds(s: number): DateTime {
    return this.add(TimeSpan.fromSeconds(s));
  }
  addMinutes(m: number): DateTime {
    return this.add(TimeSpan.fromMinutes(m));
  }
  addHours(h: number): DateTime {
    return this.add(TimeSpan.fromHours(h));
  }
  /** Calendar days (across a daylight-saving change, local times keep their clock time). */
  addDays(days: number): DateTime {
    return this.withFields((d) => (this.utc ? d.setUTCDate(d.getUTCDate() + days) : d.setDate(d.getDate() + days)));
  }
  /** Calendar months; the day is clamped to the month's length (Jan 31 + 1 month = Feb 28/29), as in C#. */
  addMonths(months: number): DateTime {
    const total = this.year * 12 + (this.month - 1) + Math.trunc(months);
    const year = Math.floor(total / 12);
    const month = (total % 12) + 1;
    const day = Math.min(this.day, DateTime.daysInMonth(year, month));
    return new DateTime(year, month, day, this.hour, this.minute, this.second, this.millisecond, this.kind);
  }
  addYears(years: number): DateTime {
    return this.addMonths(years * 12);
  }

  private withFields(change: (d: Date) => void): DateTime {
    const d = this.d;
    change(d);
    return DateTime.fromEpoch(d.getTime(), this.kind);
  }

  /** The time between two dates (`a.subtract(b)` is `a - b`), or this date minus a span. */
  subtract(other: DateTime): TimeSpan;
  subtract(span: TimeSpan): DateTime;
  subtract(x: DateTime | TimeSpan): TimeSpan | DateTime {
    return x instanceof DateTime
      ? new TimeSpan(this.epochMilliseconds - x.epochMilliseconds)
      : DateTime.fromEpoch(this.epochMilliseconds - x.totalMilliseconds, this.kind);
  }

  toUniversalTime(): DateTime {
    return DateTime.fromEpoch(this.epochMilliseconds, "utc");
  }
  toLocalTime(): DateTime {
    return DateTime.fromEpoch(this.epochMilliseconds, "local");
  }

  compareTo(other: DateTime): number {
    return Math.sign(this.epochMilliseconds - other.epochMilliseconds);
  }
  /** The same instant (kinds may differ). */
  equals(other: DateTime): boolean {
    return this.epochMilliseconds === other.epochMilliseconds;
  }

  /** Minutes east of UTC for this date in its kind (0 for UTC). */
  private get offsetMinutes(): number {
    return this.utc ? 0 : -this.d.getTimezoneOffset();
  }

  /** ISO 8601 with milliseconds and offset (C#'s `"o"`, with 3 fraction digits). */
  toISOString(): string {
    return this.toString("o");
  }

  /**
   * Formats like C#'s `DateTime.ToString` with the invariant culture.
   * - Standard: `"o"`/`"O"` round trip, `"s"` sortable, `"u"` universal sortable, `"d"` short
   *   date, `"D"` long date, `"t"`/`"T"` short/long time, `"g"`/`"G"` general, `"f"`/`"F"` full.
   * - Custom: `yyyy yy y MMMM MMM MM M dddd ddd dd d HH H hh h mm m ss s fff ff f FFF tt t zzz zz
   *   z K`, `'literal'` and `\c`. Digits after the third `f` print as zeros (milliseconds only).
   */
  toString(format = "G"): string {
    const standard: Record<string, string> = {
      d: "MM/dd/yyyy",
      D: "dddd, dd MMMM yyyy",
      t: "HH:mm",
      T: "HH:mm:ss",
      g: "MM/dd/yyyy HH:mm",
      G: "MM/dd/yyyy HH:mm:ss",
      f: "dddd, dd MMMM yyyy HH:mm",
      F: "dddd, dd MMMM yyyy HH:mm:ss",
      s: "yyyy'-'MM'-'dd'T'HH':'mm':'ss",
      o: "yyyy'-'MM'-'dd'T'HH':'mm':'ss'.'fffK",
      O: "yyyy'-'MM'-'dd'T'HH':'mm':'ss'.'fffK",
    };
    if (format === "u") return this.toUniversalTime().toString("yyyy'-'MM'-'dd HH':'mm':'ss'Z'");
    const pattern = format.length === 1 ? (standard[format] ?? format) : format;
    const pad = (n: number, w = 2) => String(n).padStart(w, "0");
    const offset = () => {
      const m = this.offsetMinutes;
      const sign = m < 0 ? "-" : "+";
      const a = Math.abs(m);
      return { sign, h: Math.trunc(a / 60), m: a % 60 };
    };
    return formatTokens(pattern, (token) => {
      const c = token[0];
      const n = token.length;
      switch (c) {
        case "y":
          return n === 1 ? String(this.year % 100) : n === 2 ? pad(this.year % 100) : pad(this.year, n);
        case "M":
          return n >= 4 ? MONTHS[this.month - 1] : n === 3 ? MONTHS[this.month - 1]!.slice(0, 3) : pad(this.month, n);
        case "d":
          return n >= 4 ? DAYS[this.dayOfWeek] : n === 3 ? DAYS[this.dayOfWeek]!.slice(0, 3) : pad(this.day, n);
        case "H":
          return pad(this.hour, Math.min(n, 2));
        case "h":
          return pad(this.hour % 12 || 12, Math.min(n, 2));
        case "m":
          return pad(this.minute, Math.min(n, 2));
        case "s":
          return pad(this.second, Math.min(n, 2));
        case "f":
        case "F":
          return fraction(this.millisecond, n, c === "F");
        case "t":
          return (this.hour < 12 ? "AM" : "PM").slice(0, n === 1 ? 1 : 2);
        case "z": {
          const o = offset();
          return n === 1 ? `${o.sign}${o.h}` : n === 2 ? `${o.sign}${pad(o.h)}` : `${o.sign}${pad(o.h)}:${pad(o.m)}`;
        }
        case "K": {
          if (this.utc) return "Z";
          const o = offset();
          return `${o.sign}${pad(o.h)}:${pad(o.m)}`;
        }
        default:
          return undefined;
      }
    });
  }
}

/**
 * Fraction-of-a-second digits from milliseconds: `f` keeps trailing zeros, `F` drops
 * them. Beyond three digits there's nothing finer than a millisecond, so zeros.
 */
function fraction(ms: number, digits: number, trim: boolean): string {
  let s = String(ms).padStart(3, "0").padEnd(digits, "0").slice(0, digits);
  if (trim) s = s.replace(/0+$/, "");
  return s;
}

/**
 * Runs a C#-style custom format: runs of the same letter are tokens handed to
 * `render` (which returns `undefined` for letters it doesn't know, printed as is);
 * `'...'`/`"..."` are literals and `\c` escapes one character.
 */
function formatTokens(format: string, render: (token: string) => string | undefined): string {
  let out = "";
  for (let i = 0; i < format.length; ) {
    const c = format[i]!;
    if (c === "\\" && i + 1 < format.length) {
      out += format[i + 1];
      i += 2;
      continue;
    }
    if (c === "'" || c === '"') {
      const end = format.indexOf(c, i + 1);
      const stop = end < 0 ? format.length : end;
      out += format.slice(i + 1, stop);
      i = stop + 1;
      continue;
    }
    let j = i;
    while (j < format.length && format[j] === c) j++;
    const token = format.slice(i, j);
    const rendered = /[a-zA-Z]/.test(c) ? render(token) : undefined;
    out += rendered ?? token;
    i = j;
  }
  return out;
}
