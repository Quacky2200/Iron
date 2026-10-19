// Temporal (ISO calendar only): Duration, PlainDate, PlainTime, PlainDateTime, Instant, Now.
// Compiled in front of programs that mention `Temporal.` (see prelude.rs / temporal.rs). Written in
// the dialect the compiler accepts: no overloads and no unions, so the lowerer routes
// `Temporal.X.from(v)` and string arguments to the parse functions here.

interface Temporal_DiffOptions {
  largestUnit?: string;
  smallestUnit?: string;
}

interface Temporal_TotalOptions {
  unit: string;
}

interface Temporal_DateLike {
  year?: number;
  month?: number;
  day?: number;
}

interface Temporal_TimeLike {
  hour?: number;
  minute?: number;
  second?: number;
  millisecond?: number;
  microsecond?: number;
  nanosecond?: number;
}

interface Temporal_DateTimeLike {
  year?: number;
  month?: number;
  day?: number;
  hour?: number;
  minute?: number;
  second?: number;
  millisecond?: number;
  microsecond?: number;
  nanosecond?: number;
}

interface Temporal_DurationLike {
  years?: number;
  months?: number;
  weeks?: number;
  days?: number;
  hours?: number;
  minutes?: number;
  seconds?: number;
  milliseconds?: number;
  microseconds?: number;
  nanoseconds?: number;
}

// ---- calendar arithmetic ----

function __tpIsLeap(y: number): boolean {
  return (y % 4 === 0 && y % 100 !== 0) || y % 400 === 0;
}

function __tpDaysInMonth(y: number, m: number): number {
  if (m === 2) return __tpIsLeap(y) ? 29 : 28;
  if (m === 4 || m === 6 || m === 9 || m === 11) return 30;
  return 31;
}

// Days since 1970-01-01 of a civil date (proleptic Gregorian).
function __tpDays(y: number, m: number, d: number): number {
  const yy = m <= 2 ? y - 1 : y;
  const era = Math.floor(yy / 400);
  const yoe = yy - era * 400;
  const mp = (m + 9) % 12;
  const doy = Math.floor((153 * mp + 2) / 5) + d - 1;
  const doe = yoe * 365 + Math.floor(yoe / 4) - Math.floor(yoe / 100) + doy;
  return era * 146097 + doe - 719468;
}

class Temporal_Ymd {
  year: number = 0;
  month: number = 0;
  day: number = 0;
}

function __tpCivil(days: number): Temporal_Ymd {
  const z = days + 719468;
  const era = Math.floor(z / 146097);
  const doe = z - era * 146097;
  const yoe = Math.floor((doe - Math.floor(doe / 1460) + Math.floor(doe / 36524) - Math.floor(doe / 146096)) / 365);
  const doy = doe - (365 * yoe + Math.floor(yoe / 4) - Math.floor(yoe / 100));
  const mp = Math.floor((5 * doy + 2) / 153);
  const d = doy - Math.floor((153 * mp + 2) / 5) + 1;
  const m = mp < 10 ? mp + 3 : mp - 9;
  const r = new Temporal_Ymd();
  r.year = yoe + era * 400 + (m <= 2 ? 1 : 0);
  r.month = m;
  r.day = d;
  return r;
}

function __tpPad(n: number, width: number): string {
  return String(n).padStart(width, "0");
}

function __tpYearString(y: number): string {
  if (y >= 0 && y <= 9999) return __tpPad(y, 4);
  return (y < 0 ? "-" : "+") + __tpPad(Math.abs(y), 6);
}

function __tpFraction(ms: number, us: number, ns: number): string {
  const total = ms * 1000000 + us * 1000 + ns;
  if (total === 0) return "";
  let s = __tpPad(total, 9);
  while (s.endsWith("0")) s = s.slice(0, s.length - 1);
  return "." + s;
}

function __tpSign(n: number): number {
  return n > 0 ? 1 : n < 0 ? -1 : 0;
}

// ---- parsing ----

class Temporal_Cursor {
  s: string;
  at: number = 0;
  constructor(s: string) {
    this.s = s;
  }
  more(): boolean {
    return this.at < this.s.length;
  }
  peek(): string {
    return this.at < this.s.length ? this.s.charAt(this.at) : "";
  }
  eat(c: string): boolean {
    if (this.peek() === c) {
      this.at += 1;
      return true;
    }
    return false;
  }
  digits(n: number): number {
    let v = 0;
    for (let i = 0; i < n; i++) {
      const c = this.s.charCodeAt(this.at + i);
      if (!(c >= 48 && c <= 57)) throw new RangeError("invalid ISO string: " + this.s);
      v = v * 10 + (c - 48);
    }
    this.at += n;
    return v;
  }
  isDigit(): boolean {
    const c = this.s.charCodeAt(this.at);
    return c >= 48 && c <= 57;
  }
}

function __tpBad(s: string): RangeError {
  return new RangeError("invalid ISO 8601 string: " + s);
}

function __tpParseYear(c: Temporal_Cursor): number {
  if (c.peek() === "+" || c.peek() === "-") {
    const neg = c.peek() === "-";
    c.at += 1;
    const y = c.digits(6);
    if (neg && y === 0) throw __tpBad(c.s);
    return neg ? -y : y;
  }
  return c.digits(4);
}

function __tpParseDateAt(c: Temporal_Cursor): Temporal_PlainDate {
  const y = __tpParseYear(c);
  const dash = c.eat("-");
  const m = c.digits(2);
  if (dash && !c.eat("-")) throw __tpBad(c.s);
  const d = c.digits(2);
  return new Temporal_PlainDate(y, m, d);
}

// Reads HH[:MM[:SS[.fff]]] into a time.
function __tpParseTimeAt(c: Temporal_Cursor): Temporal_PlainTime {
  const h = c.digits(2);
  let mi = 0;
  let se = 0;
  let ms = 0;
  let us = 0;
  let ns = 0;
  const colon = c.eat(":");
  if (colon || c.isDigit()) {
    mi = c.digits(2);
    const colon2 = colon ? c.eat(":") : c.isDigit();
    if (colon2) {
      se = c.digits(2);
      if (se === 60) se = 59;
      if (c.eat(".") || c.eat(",")) {
        let frac = 0;
        let n = 0;
        while (c.isDigit() && n < 9) {
          frac = frac * 10 + (c.s.charCodeAt(c.at) - 48);
          c.at += 1;
          n += 1;
        }
        if (n === 0) throw __tpBad(c.s);
        while (n < 9) {
          frac *= 10;
          n += 1;
        }
        ms = Math.floor(frac / 1000000);
        us = Math.floor(frac / 1000) % 1000;
        ns = frac % 1000;
      }
    }
  }
  return new Temporal_PlainTime(h, mi, se, ms, us, ns);
}

function __tpParseDate(s: string): Temporal_PlainDate {
  const c = new Temporal_Cursor(s);
  const d = __tpParseDateAt(c);
  if (c.peek() === "T" || c.peek() === "t" || c.peek() === " ") {
    c.at += 1;
    __tpParseTimeAt(c);
    skipZone(c);
  }
  if (c.more()) throw __tpBad(s);
  return d;
}

function skipZone(c: Temporal_Cursor): void {
  if (c.peek() === "Z" || c.peek() === "z") {
    c.at += 1;
  } else if (c.peek() === "+" || c.peek() === "-") {
    c.at += 1;
    c.digits(2);
    c.eat(":");
    if (c.isDigit()) c.digits(2);
  }
  if (c.peek() === "[") {
    while (c.more() && c.peek() !== "]") c.at += 1;
    c.eat("]");
  }
}

function __tpParseTime(s: string): Temporal_PlainTime {
  const c = new Temporal_Cursor(s);
  if (c.peek() === "T" || c.peek() === "t") c.at += 1;
  const t = __tpParseTimeAt(c);
  skipZone(c);
  if (c.more()) throw __tpBad(s);
  return t;
}

function __tpParseDateTime(s: string): Temporal_PlainDateTime {
  const c = new Temporal_Cursor(s);
  const d = __tpParseDateAt(c);
  let t = new Temporal_PlainTime(0, 0, 0, 0, 0, 0);
  if (c.peek() === "T" || c.peek() === "t" || c.peek() === " ") {
    c.at += 1;
    t = __tpParseTimeAt(c);
    skipZone(c);
  }
  if (c.more()) throw __tpBad(s);
  return new Temporal_PlainDateTime(d.year, d.month, d.day, t.hour, t.minute, t.second, t.millisecond, t.microsecond, t.nanosecond);
}


// ---- rounding ----

interface Temporal_RoundOptions {
  smallestUnit?: string;
  largestUnit?: string;
  roundingMode?: string;
  roundingIncrement?: number;
}

function __tpRoundOpts(unit: string): Temporal_RoundOptions {
  return { smallestUnit: unit };
}

// The size of a unit in nanoseconds (day, hour .. nanosecond).
function __tpUnitSize(unit: string): bigint {
  if (unit === "day" || unit === "days") return 86400000000000n;
  if (unit === "hour" || unit === "hours") return 3600000000000n;
  if (unit === "minute" || unit === "minutes") return 60000000000n;
  if (unit === "second" || unit === "seconds") return 1000000000n;
  if (unit === "millisecond" || unit === "milliseconds") return 1000000n;
  if (unit === "microsecond" || unit === "microseconds") return 1000n;
  if (unit === "nanosecond" || unit === "nanoseconds") return 1n;
  throw new RangeError("invalid unit " + unit);
}

// Rounds `v` to a multiple of `size` by a Temporal rounding mode.
function __tpRound(v: bigint, size: bigint, mode: string): bigint {
  let q = v / size;
  if (v % size < 0n) q -= 1n;
  const down = q * size;
  const r = v - down;
  if (r === 0n) return v;
  const up = down + size;
  const neg = v < 0n;
  if (mode === "ceil") return up;
  if (mode === "floor") return down;
  if (mode === "trunc") return neg ? up : down;
  if (mode === "expand") return neg ? down : up;
  const twice = r * 2n;
  if (twice < size) return down;
  if (twice > size) return up;
  if (mode === "halfCeil") return up;
  if (mode === "halfFloor") return down;
  if (mode === "halfTrunc") return neg ? up : down;
  if (mode === "halfEven") return q % 2n === 0n ? down : up;
  if (mode === "halfExpand") return neg ? down : up;
  throw new RangeError("invalid rounding mode " + mode);
}

function __tpMode(o: Temporal_RoundOptions): string {
  const m = o.roundingMode;
  if (m === undefined) return "halfExpand";
  if (m !== "ceil" && m !== "floor" && m !== "trunc" && m !== "expand" && m !== "halfCeil" && m !== "halfFloor" && m !== "halfTrunc" && m !== "halfExpand" && m !== "halfEven") {
    throw new RangeError("invalid rounding mode " + m);
  }
  return m;
}

function __tpUnitOf(o: Temporal_RoundOptions): string {
  if (o.smallestUnit === undefined) throw new RangeError("smallestUnit is required");
  return o.smallestUnit;
}

function __tpIncrement(o: Temporal_RoundOptions): bigint {
  return o.roundingIncrement === undefined ? 1n : BigInt(o.roundingIncrement);
}

// ---- Duration ----

class Temporal_Duration {
  years: number = 0;
  months: number = 0;
  weeks: number = 0;
  days: number = 0;
  hours: number = 0;
  minutes: number = 0;
  seconds: number = 0;
  milliseconds: number = 0;
  microseconds: number = 0;
  nanoseconds: number = 0;
  constructor(
    years: number = 0,
    months: number = 0,
    weeks: number = 0,
    days: number = 0,
    hours: number = 0,
    minutes: number = 0,
    seconds: number = 0,
    milliseconds: number = 0,
    microseconds: number = 0,
    nanoseconds: number = 0,
  ) {
    this.years = years;
    this.months = months;
    this.weeks = weeks;
    this.days = days;
    this.hours = hours;
    this.minutes = minutes;
    this.seconds = seconds;
    this.milliseconds = milliseconds;
    this.microseconds = microseconds;
    this.nanoseconds = nanoseconds;
    this.check();
  }
  fields(): number[] {
    return [this.years, this.months, this.weeks, this.days, this.hours, this.minutes, this.seconds, this.milliseconds, this.microseconds, this.nanoseconds];
  }
  check(): void {
    let sign = 0;
    for (const v of this.fields()) {
      if (!Number.isFinite(v) || v !== Math.trunc(v)) throw new RangeError("durations cannot have fractional or non-finite values");
      const s = __tpSign(v);
      if (s !== 0) {
        if (sign !== 0 && s !== sign) throw new RangeError("mixed-sign values not allowed as duration fields");
        sign = s;
      }
    }
  }
  get sign(): number {
    for (const v of this.fields()) {
      if (v !== 0) return v > 0 ? 1 : -1;
    }
    return 0;
  }
  get blank(): boolean {
    return this.sign === 0;
  }
  negated(): Temporal_Duration {
    const f = this.fields();
    return new Temporal_Duration(-f[0] + 0, -f[1] + 0, -f[2] + 0, -f[3] + 0, -f[4] + 0, -f[5] + 0, -f[6] + 0, -f[7] + 0, -f[8] + 0, -f[9] + 0);
  }
  abs(): Temporal_Duration {
    return this.sign < 0 ? this.negated() : this;
  }
  with(like: Temporal_DurationLike): Temporal_Duration {
    const y = like.years === undefined ? this.years : like.years;
    const mo = like.months === undefined ? this.months : like.months;
    const w = like.weeks === undefined ? this.weeks : like.weeks;
    const d = like.days === undefined ? this.days : like.days;
    const h = like.hours === undefined ? this.hours : like.hours;
    const mi = like.minutes === undefined ? this.minutes : like.minutes;
    const s = like.seconds === undefined ? this.seconds : like.seconds;
    const ms = like.milliseconds === undefined ? this.milliseconds : like.milliseconds;
    const us = like.microseconds === undefined ? this.microseconds : like.microseconds;
    const ns = like.nanoseconds === undefined ? this.nanoseconds : like.nanoseconds;
    return new Temporal_Duration(y, mo, w, d, h, mi, s, ms, us, ns);
  }
  // Rank of the largest nonzero unit: 0 years .. 9 nanoseconds (9 when blank).
  largest(): number {
    const f = this.fields();
    for (let i = 0; i < 10; i++) {
      if (f[i] !== 0) return i;
    }
    return 9;
  }
  // The days-and-time part as nanoseconds.
  timeNs(): bigint {
    let t = BigInt(this.days) * 24n + BigInt(this.hours);
    t = t * 60n + BigInt(this.minutes);
    t = t * 60n + BigInt(this.seconds);
    return t * 1000000000n + BigInt(this.milliseconds) * 1000000n + BigInt(this.microseconds) * 1000n + BigInt(this.nanoseconds);
  }
  addSigned(other: Temporal_Duration, sign: number): Temporal_Duration {
    if (this.years !== 0 || this.months !== 0 || this.weeks !== 0 || other.years !== 0 || other.months !== 0 || other.weeks !== 0) {
      throw new RangeError("a starting point is required for adding or subtracting years, months or weeks");
    }
    const largest = Math.min(this.largest(), other.largest());
    let total = this.timeNs() + BigInt(sign) * other.timeNs();
    // Units from the largest (3 days) down, truncating toward zero.
    const sizes: bigint[] = [86400000000000n, 3600000000000n, 60000000000n, 1000000000n, 1000000n, 1000n, 1n];
    const out: number[] = [0, 0, 0, 0, 0, 0, 0];
    const first = Math.max(largest, 3) - 3;
    for (let i = first; i < 7; i++) {
      const q = total / sizes[i];
      out[i] = Number(q);
      total = total - q * sizes[i];
    }
    return new Temporal_Duration(0, 0, 0, out[0], out[1], out[2], out[3], out[4], out[5], out[6]);
  }
  add(other: Temporal_Duration): Temporal_Duration {
    return this.addSigned(other, 1);
  }
  subtract(other: Temporal_Duration): Temporal_Duration {
    return this.addSigned(other, -1);
  }
  total(options: Temporal_TotalOptions): number {
    const u = options.unit;
    if (this.years !== 0 || this.months !== 0 || this.weeks !== 0) {
      throw new RangeError("a starting point is required for years, months and weeks");
    }
    const t = this.timeNs();
    let size = 1n;
    if (u === "day" || u === "days") size = 86400000000000n;
    else if (u === "hour" || u === "hours") size = 3600000000000n;
    else if (u === "minute" || u === "minutes") size = 60000000000n;
    else if (u === "second" || u === "seconds") size = 1000000000n;
    else if (u === "millisecond" || u === "milliseconds") size = 1000000n;
    else if (u === "microsecond" || u === "microseconds") size = 1000n;
    else if (u === "nanosecond" || u === "nanoseconds") size = 1n;
    else throw new RangeError("invalid unit " + u);
    const whole = t / size;
    const rest = t - whole * size;
    return Number(whole) + Number(rest) / Number(size);
  }
  round(options: Temporal_RoundOptions): Temporal_Duration {
    if (this.years !== 0 || this.months !== 0 || this.weeks !== 0) {
      throw new RangeError("a starting point is required for years, months and weeks");
    }
    let unit = "nanosecond";
    if (options.smallestUnit !== undefined) unit = options.smallestUnit;
    const size = __tpUnitSize(unit);
    const rounded = __tpRound(this.timeNs(), size * __tpIncrement(options), __tpMode(options));
    // The largest unit: the larger of the existing one and the smallest asked for.
    let largest = "";
    if (options.largestUnit !== undefined && options.largestUnit !== "auto") largest = options.largestUnit;
    if (largest === "") {
      const names = ["day", "hour", "minute", "second", "millisecond", "microsecond", "nanosecond"];
      let own = Math.max(this.largest(), 3) - 3;
      let want = 0;
      for (let i = 0; i < 7; i++) {
        if (__tpUnitSize(names[i]) === size) want = i;
      }
      largest = names[Math.min(own, want)];
    }
    const names2 = ["day", "hour", "minute", "second", "millisecond", "microsecond", "nanosecond"];
    let first = 0;
    for (let i = 0; i < 7; i++) {
      if (__tpUnitSize(names2[i]) === __tpUnitSize(largest)) first = i;
    }
    let total = rounded;
    const out: number[] = [0, 0, 0, 0, 0, 0, 0];
    for (let i = first; i < 7; i++) {
      const s = __tpUnitSize(names2[i]);
      const q = total / s;
      out[i] = Number(q);
      total = total - q * s;
    }
    return new Temporal_Duration(0, 0, 0, out[0] + 0, out[1] + 0, out[2] + 0, out[3] + 0, out[4] + 0, out[5] + 0, out[6] + 0);
  }
  toString(): string {
    if (this.sign === 0) return "PT0S";
    const neg = this.sign < 0;
    const a = this.abs();
    let date = "";
    if (a.years !== 0) date += a.years + "Y";
    if (a.months !== 0) date += a.months + "M";
    if (a.weeks !== 0) date += a.weeks + "W";
    if (a.days !== 0) date += a.days + "D";
    let time = "";
    if (a.hours !== 0) time += a.hours + "H";
    if (a.minutes !== 0) time += a.minutes + "M";
    // Sub-second units fold into the seconds.
    const sub = a.milliseconds * 1000000 + a.microseconds * 1000 + a.nanoseconds;
    const whole = a.seconds + Math.floor(sub / 1000000000);
    const rest = sub % 1000000000;
    if (whole !== 0 || rest !== 0) {
      time += whole + __tpFraction(Math.floor(rest / 1000000), Math.floor(rest / 1000) % 1000, rest % 1000) + "S";
    }
    return (neg ? "-" : "") + "P" + date + (time === "" ? "" : "T" + time);
  }
  toJSON(): string {
    return this.toString();
  }
}

function __tpParseDuration(s: string): Temporal_Duration {
  const c = new Temporal_Cursor(s);
  let sign = 1;
  if (c.eat("-")) sign = -1;
  else c.eat("+");
  if (!(c.eat("P") || c.eat("p"))) throw __tpBad(s);
  const v: number[] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
  let inTime = false;
  let any = false;
  while (c.more()) {
    if (c.eat("T") || c.eat("t")) {
      inTime = true;
      continue;
    }
    let n = 0;
    let digits = 0;
    while (c.isDigit()) {
      n = n * 10 + (c.s.charCodeAt(c.at) - 48);
      c.at += 1;
      digits += 1;
    }
    if (digits === 0) throw __tpBad(s);
    let frac = 0;
    let fracDigits = 0;
    if (c.peek() === "." || c.peek() === ",") {
      c.at += 1;
      while (c.isDigit() && fracDigits < 9) {
        frac = frac * 10 + (c.s.charCodeAt(c.at) - 48);
        c.at += 1;
        fracDigits += 1;
      }
      if (fracDigits === 0) throw __tpBad(s);
      while (fracDigits < 9) {
        frac *= 10;
        fracDigits += 1;
      }
    }
    const u = c.peek().toUpperCase();
    c.at += 1;
    any = true;
    if (!inTime) {
      if (frac !== 0) throw __tpBad(s);
      if (u === "Y") v[0] = n;
      else if (u === "M") v[1] = n;
      else if (u === "W") v[2] = n;
      else if (u === "D") v[3] = n;
      else throw __tpBad(s);
    } else if (u === "H") {
      v[4] = n;
      if (frac !== 0) {
        // Fractions of an hour or minute spill into smaller units.
        const m = Math.floor((frac * 60) / 1000000000);
        v[5] = m;
        frac = (frac * 60) % 1000000000;
        v[6] = Math.floor((frac * 60) / 1000000000);
        const f2 = (frac * 60) % 1000000000;
        v[7] = Math.floor(f2 / 1000000);
        v[8] = Math.floor(f2 / 1000) % 1000;
        v[9] = f2 % 1000;
      }
    } else if (u === "M") {
      v[5] = n;
      if (frac !== 0) {
        v[6] = Math.floor((frac * 60) / 1000000000);
        const f2 = (frac * 60) % 1000000000;
        v[7] = Math.floor(f2 / 1000000);
        v[8] = Math.floor(f2 / 1000) % 1000;
        v[9] = f2 % 1000;
      }
    } else if (u === "S") {
      v[6] = n;
      v[7] = Math.floor(frac / 1000000);
      v[8] = Math.floor(frac / 1000) % 1000;
      v[9] = frac % 1000;
    } else {
      throw __tpBad(s);
    }
  }
  if (!any) throw __tpBad(s);
  return new Temporal_Duration(sign * v[0], sign * v[1], sign * v[2], sign * v[3], sign * v[4], sign * v[5], sign * v[6], sign * v[7], sign * v[8], sign * v[9]);
}

function __tpDurationFrom(d: Temporal_Duration): Temporal_Duration {
  return new Temporal_Duration(d.years, d.months, d.weeks, d.days, d.hours, d.minutes, d.seconds, d.milliseconds, d.microseconds, d.nanoseconds);
}

// ---- PlainTime ----

class Temporal_PlainTime {
  hour: number = 0;
  minute: number = 0;
  second: number = 0;
  millisecond: number = 0;
  microsecond: number = 0;
  nanosecond: number = 0;
  constructor(hour: number = 0, minute: number = 0, second: number = 0, millisecond: number = 0, microsecond: number = 0, nanosecond: number = 0) {
    this.hour = hour;
    this.minute = minute;
    this.second = second;
    this.millisecond = millisecond;
    this.microsecond = microsecond;
    this.nanosecond = nanosecond;
    this.check();
  }
  check(): void {
    if (
      !(this.hour >= 0 && this.hour <= 23 && this.minute >= 0 && this.minute <= 59 && this.second >= 0 && this.second <= 59) ||
      !(this.millisecond >= 0 && this.millisecond <= 999 && this.microsecond >= 0 && this.microsecond <= 999 && this.nanosecond >= 0 && this.nanosecond <= 999)
    ) {
      throw new RangeError("invalid time");
    }
  }
  // Nanoseconds since midnight.
  ns(): number {
    return (((this.hour * 60 + this.minute) * 60 + this.second) * 1000 + this.millisecond) * 1000000 + this.microsecond * 1000 + this.nanosecond;
  }
  static fromNs(ns: number): Temporal_PlainTime {
    const day = 86400000000000;
    let n = ns % day;
    if (n < 0) n += day;
    const h = Math.floor(n / 3600000000000);
    n -= h * 3600000000000;
    const mi = Math.floor(n / 60000000000);
    n -= mi * 60000000000;
    const s = Math.floor(n / 1000000000);
    n -= s * 1000000000;
    return new Temporal_PlainTime(h, mi, s, Math.floor(n / 1000000), Math.floor(n / 1000) % 1000, n % 1000);
  }
  with(like: Temporal_TimeLike): Temporal_PlainTime {
    return new Temporal_PlainTime(
      like.hour === undefined ? this.hour : like.hour,
      like.minute === undefined ? this.minute : like.minute,
      like.second === undefined ? this.second : like.second,
      like.millisecond === undefined ? this.millisecond : like.millisecond,
      like.microsecond === undefined ? this.microsecond : like.microsecond,
      like.nanosecond === undefined ? this.nanosecond : like.nanosecond,
    );
  }
  addSigned(d: Temporal_Duration, sign: number): Temporal_PlainTime {
    // Only the time-of-day part of the duration matters (it wraps around midnight).
    const t = Number((d.timeNs() - BigInt(d.days) * 86400000000000n) % 86400000000000n);
    return Temporal_PlainTime.fromNs(this.ns() + sign * t);
  }
  add(d: Temporal_Duration): Temporal_PlainTime {
    return this.addSigned(d, 1);
  }
  subtract(d: Temporal_Duration): Temporal_PlainTime {
    return this.addSigned(d, -1);
  }
  diff(other: Temporal_PlainTime, sign: number, opts: Temporal_DiffOptions | null): Temporal_Duration {
    let ns = sign * (other.ns() - this.ns());
    let unit = "hour";
    if (opts !== null && opts.largestUnit !== undefined && opts.largestUnit !== "auto") unit = opts.largestUnit;
    const size = __tpUnitNs(unit);
    const out: number[] = [0, 0, 0, 0, 0, 0];
    const sizes = [3600000000000, 60000000000, 1000000000, 1000000, 1000, 1];
    const start = __tpUnitIndex(unit);
    for (let i = start; i < 6; i++) {
      const q = Math.trunc(ns / sizes[i]);
      out[i] = q;
      ns -= q * sizes[i];
    }
    return new Temporal_Duration(0, 0, 0, 0, out[0] + 0, out[1] + 0, out[2] + 0, out[3] + 0, out[4] + 0, out[5] + 0);
  }
  until(other: Temporal_PlainTime, opts: Temporal_DiffOptions | null = null): Temporal_Duration {
    return this.diff(other, 1, opts);
  }
  since(other: Temporal_PlainTime, opts: Temporal_DiffOptions | null = null): Temporal_Duration {
    return this.diff(other, 1, opts).negated();
  }
  round(options: Temporal_RoundOptions): Temporal_PlainTime {
    const size = __tpUnitSize(__tpUnitOf(options));
    if (size > 3600000000000n) throw new RangeError("invalid unit for PlainTime.round");
    const r = __tpRound(BigInt(this.ns()), size * __tpIncrement(options), __tpMode(options));
    return Temporal_PlainTime.fromNs(Number(r % 86400000000000n));
  }
  equals(other: Temporal_PlainTime): boolean {
    return this.ns() === other.ns();
  }
  static compare(a: Temporal_PlainTime, b: Temporal_PlainTime): number {
    return __tpSign(a.ns() - b.ns());
  }
  toString(): string {
    let s = __tpPad(this.hour, 2) + ":" + __tpPad(this.minute, 2) + ":" + __tpPad(this.second, 2);
    return s + __tpFraction(this.millisecond, this.microsecond, this.nanosecond);
  }
  toJSON(): string {
    return this.toString();
  }
}

// 0 hour .. 5 nanosecond.
function __tpUnitIndex(unit: string): number {
  if (unit === "hour" || unit === "hours") return 0;
  if (unit === "minute" || unit === "minutes") return 1;
  if (unit === "second" || unit === "seconds") return 2;
  if (unit === "millisecond" || unit === "milliseconds") return 3;
  if (unit === "microsecond" || unit === "microseconds") return 4;
  if (unit === "nanosecond" || unit === "nanoseconds") return 5;
  throw new RangeError("invalid unit " + unit);
}

function __tpUnitNs(unit: string): number {
  if (unit === "hour" || unit === "hours") return 3600000000000;
  if (unit === "minute" || unit === "minutes") return 60000000000;
  if (unit === "second" || unit === "seconds") return 1000000000;
  if (unit === "millisecond" || unit === "milliseconds") return 1000000;
  if (unit === "microsecond" || unit === "microseconds") return 1000;
  if (unit === "nanosecond" || unit === "nanoseconds") return 1;
  throw new RangeError("invalid unit " + unit);
}

function __tpTimeFrom(t: Temporal_PlainTime): Temporal_PlainTime {
  return new Temporal_PlainTime(t.hour, t.minute, t.second, t.millisecond, t.microsecond, t.nanosecond);
}

// ---- PlainDate ----

class Temporal_PlainDate {
  year: number = 1970;
  month: number = 1;
  day: number = 1;
  constructor(year: number, month: number, day: number) {
    if (!(Number.isInteger(year) && Number.isInteger(month) && Number.isInteger(day))) throw new RangeError("invalid date");
    if (month < 1 || month > 12 || day < 1 || day > __tpDaysInMonth(year, month)) throw new RangeError("invalid date");
    this.year = year;
    this.month = month;
    this.day = day;
  }
  get calendarId(): string {
    return "iso8601";
  }
  get monthCode(): string {
    return "M" + __tpPad(this.month, 2);
  }
  get daysInMonth(): number {
    return __tpDaysInMonth(this.year, this.month);
  }
  get daysInYear(): number {
    return __tpIsLeap(this.year) ? 366 : 365;
  }
  get monthsInYear(): number {
    return 12;
  }
  get daysInWeek(): number {
    return 7;
  }
  get inLeapYear(): boolean {
    return __tpIsLeap(this.year);
  }
  epochDays(): number {
    return __tpDays(this.year, this.month, this.day);
  }
  get dayOfWeek(): number {
    const z = this.epochDays();
    return (((z % 7) + 7 + 3) % 7) + 1;
  }
  get dayOfYear(): number {
    return this.epochDays() - __tpDays(this.year, 1, 1) + 1;
  }
  // ISO week number and week-year.
  isoWeek(): number[] {
    const wd = this.dayOfWeek;
    const doy = this.dayOfYear;
    let week = Math.floor((doy - wd + 10) / 7);
    let year = this.year;
    if (week < 1) {
      year -= 1;
      const dec31 = new Temporal_PlainDate(year, 12, 31);
      const w = dec31.isoWeek();
      return [w[0], year];
    }
    if (week === 53) {
      const jan1Next = __tpDays(year + 1, 1, 1);
      const wdJan1 = (((jan1Next % 7) + 7 + 3) % 7) + 1;
      if (wdJan1 <= 4) {
        week = 1;
        year += 1;
      }
    }
    return [week, year];
  }
  get weekOfYear(): number {
    return this.isoWeek()[0];
  }
  get yearOfWeek(): number {
    return this.isoWeek()[1];
  }
  static fromEpochDays(z: number): Temporal_PlainDate {
    const c = __tpCivil(z);
    return new Temporal_PlainDate(c.year, c.month, c.day);
  }
  with(like: Temporal_DateLike): Temporal_PlainDate {
    const y = like.year === undefined ? this.year : like.year;
    const m = like.month === undefined ? this.month : like.month;
    const d = like.day === undefined ? this.day : like.day;
    return Temporal_PlainDate.constrained(y, m, d);
  }
  // Clamps month and day into range ("constrain" overflow).
  static constrained(y: number, m: number, d: number): Temporal_PlainDate {
    const mm = Math.min(12, Math.max(1, m));
    const dd = Math.min(__tpDaysInMonth(y, mm), Math.max(1, d));
    return new Temporal_PlainDate(y, mm, dd);
  }
  addSigned(dur: Temporal_Duration, sign: number): Temporal_PlainDate {
    let y = this.year + sign * dur.years;
    let m = this.month + sign * dur.months;
    // Normalise the month into 1..12 carrying into the year.
    const m0 = m - 1;
    y += Math.floor(m0 / 12);
    m = (((m0 % 12) + 12) % 12) + 1;
    const base = Temporal_PlainDate.constrained(y, m, this.day);
    const secs = dur.hours * 3600 + dur.minutes * 60 + dur.seconds;
    const extra = Math.trunc(secs / 86400);
    return Temporal_PlainDate.fromEpochDays(base.epochDays() + sign * (dur.weeks * 7 + dur.days + extra));
  }
  add(d: Temporal_Duration): Temporal_PlainDate {
    return this.addSigned(d, 1);
  }
  subtract(d: Temporal_Duration): Temporal_PlainDate {
    return this.addSigned(d, -1);
  }
  diff(other: Temporal_PlainDate, sign: number, opts: Temporal_DiffOptions | null): Temporal_Duration {
    let unit = "day";
    if (opts !== null && opts.largestUnit !== undefined && opts.largestUnit !== "auto") unit = opts.largestUnit;
    const from = sign > 0 ? this : other;
    const to = sign > 0 ? other : this;
    const dir = Temporal_PlainDate.compare(from, to) <= 0 ? 1 : -1;
    let years = 0;
    let months = 0;
    let weeks = 0;
    let days = 0;
    if (unit === "year" || unit === "years" || unit === "month" || unit === "months") {
      let y = to.year - from.year;
      // Back off while the candidate year surpasses `to`.
      if (unit === "year" || unit === "years") {
        while (y !== 0 && __tpSurpasses(dir, from.year + y, from.month, from.day, to)) y -= dir;
        years = y;
      }
      let mid = Temporal_PlainDate.constrained(from.year + years, from.month, from.day);
      let m = (to.year - mid.year) * 12 + (to.month - mid.month);
      while (m !== 0 && __tpSurpasses(dir, mid.year + Math.floor((mid.month - 1 + m) / 12), (((mid.month - 1 + m) % 12) + 12) % 12 + 1, from.day, to)) m -= dir;
      months = m;
      const total = mid.month - 1 + months;
      mid = Temporal_PlainDate.constrained(mid.year + Math.floor(total / 12), (((total % 12) + 12) % 12) + 1, from.day);
      days = to.epochDays() - mid.epochDays();
    } else {
      days = to.epochDays() - from.epochDays();
      if (unit === "week" || unit === "weeks") {
        weeks = Math.trunc(days / 7);
        days -= weeks * 7;
      }
    }
    return new Temporal_Duration(sign * years + 0, sign * months + 0, sign * weeks + 0, sign * days + 0);
  }
  until(other: Temporal_PlainDate, opts: Temporal_DiffOptions | null = null): Temporal_Duration {
    return this.diff(other, 1, opts);
  }
  since(other: Temporal_PlainDate, opts: Temporal_DiffOptions | null = null): Temporal_Duration {
    return this.diff(other, 1, opts).negated();
  }
  equals(other: Temporal_PlainDate): boolean {
    return this.year === other.year && this.month === other.month && this.day === other.day;
  }
  static compare(a: Temporal_PlainDate, b: Temporal_PlainDate): number {
    return __tpSign(a.epochDays() - b.epochDays());
  }
  toPlainYearMonth(): Temporal_PlainYearMonth {
    return new Temporal_PlainYearMonth(this.year, this.month);
  }
  toPlainMonthDay(): Temporal_PlainMonthDay {
    return new Temporal_PlainMonthDay(this.month, this.day);
  }
  toPlainDateTime(time: Temporal_PlainTime | null = null): Temporal_PlainDateTime {
    const t = time === null ? new Temporal_PlainTime(0, 0, 0, 0, 0, 0) : time;
    return new Temporal_PlainDateTime(this.year, this.month, this.day, t.hour, t.minute, t.second, t.millisecond, t.microsecond, t.nanosecond);
  }
  toString(): string {
    return __tpYearString(this.year) + "-" + __tpPad(this.month, 2) + "-" + __tpPad(this.day, 2);
  }
  toJSON(): string {
    return this.toString();
  }
}

// Does the candidate (y, m, d) lie beyond `to` in direction `dir`?
function __tpSurpasses(dir: number, y: number, m: number, d: number, to: Temporal_PlainDate): boolean {
  if (y !== to.year) return dir * (y - to.year) > 0;
  if (m !== to.month) return dir * (m - to.month) > 0;
  return dir * (d - to.day) > 0;
}

function __tpDateFrom(d: Temporal_PlainDate): Temporal_PlainDate {
  return Temporal_PlainDate.constrained(d.year, d.month, d.day);
}

// ---- PlainDateTime ----

class Temporal_PlainDateTime {
  year: number = 1970;
  month: number = 1;
  day: number = 1;
  hour: number = 0;
  minute: number = 0;
  second: number = 0;
  millisecond: number = 0;
  microsecond: number = 0;
  nanosecond: number = 0;
  constructor(
    year: number,
    month: number,
    day: number,
    hour: number = 0,
    minute: number = 0,
    second: number = 0,
    millisecond: number = 0,
    microsecond: number = 0,
    nanosecond: number = 0,
  ) {
    const d = new Temporal_PlainDate(year, month, day);
    const t = new Temporal_PlainTime(hour, minute, second, millisecond, microsecond, nanosecond);
    this.year = d.year;
    this.month = d.month;
    this.day = d.day;
    this.hour = t.hour;
    this.minute = t.minute;
    this.second = t.second;
    this.millisecond = t.millisecond;
    this.microsecond = t.microsecond;
    this.nanosecond = t.nanosecond;
  }
  get calendarId(): string {
    return "iso8601";
  }
  get monthCode(): string {
    return "M" + __tpPad(this.month, 2);
  }
  get dayOfWeek(): number {
    return this.toPlainDate().dayOfWeek;
  }
  get dayOfYear(): number {
    return this.toPlainDate().dayOfYear;
  }
  get daysInMonth(): number {
    return __tpDaysInMonth(this.year, this.month);
  }
  get daysInYear(): number {
    return __tpIsLeap(this.year) ? 366 : 365;
  }
  get inLeapYear(): boolean {
    return __tpIsLeap(this.year);
  }
  get weekOfYear(): number {
    return this.toPlainDate().weekOfYear;
  }
  toPlainDate(): Temporal_PlainDate {
    return new Temporal_PlainDate(this.year, this.month, this.day);
  }
  toPlainTime(): Temporal_PlainTime {
    return new Temporal_PlainTime(this.hour, this.minute, this.second, this.millisecond, this.microsecond, this.nanosecond);
  }
  with(like: Temporal_DateTimeLike): Temporal_PlainDateTime {
    const d = this.toPlainDate().with({
      year: like.year,
      month: like.month,
      day: like.day,
    });
    return new Temporal_PlainDateTime(
      d.year,
      d.month,
      d.day,
      like.hour === undefined ? this.hour : like.hour,
      like.minute === undefined ? this.minute : like.minute,
      like.second === undefined ? this.second : like.second,
      like.millisecond === undefined ? this.millisecond : like.millisecond,
      like.microsecond === undefined ? this.microsecond : like.microsecond,
      like.nanosecond === undefined ? this.nanosecond : like.nanosecond,
    );
  }
  // Nanoseconds since the epoch as a day count and a time of day.
  addSigned(dur: Temporal_Duration, sign: number): Temporal_PlainDateTime {
    const calendar = new Temporal_Duration(dur.years, dur.months, 0, 0, 0, 0, 0, 0, 0, 0);
    const date = this.toPlainDate().addSigned(calendar, sign);
    // Weeks and days, then the time part with carries into days.
    let timeNs = BigInt(this.toPlainTime().ns());
    const delta = BigInt(sign) * (BigInt(dur.weeks * 7 + dur.days) * 86400000000000n + dur.timeNs() - BigInt(dur.days) * 86400000000000n);
    timeNs += delta;
    const day = 86400000000000n;
    let dayShift = timeNs / day;
    let rest = timeNs - dayShift * day;
    if (rest < 0n) {
      rest += day;
      dayShift -= 1n;
    }
    const nd = Temporal_PlainDate.fromEpochDays(date.epochDays() + Number(dayShift));
    const t = Temporal_PlainTime.fromNs(Number(rest));
    return new Temporal_PlainDateTime(nd.year, nd.month, nd.day, t.hour, t.minute, t.second, t.millisecond, t.microsecond, t.nanosecond);
  }
  add(d: Temporal_Duration): Temporal_PlainDateTime {
    return this.addSigned(d, 1);
  }
  subtract(d: Temporal_Duration): Temporal_PlainDateTime {
    return this.addSigned(d, -1);
  }
  diff(other: Temporal_PlainDateTime, sign: number, opts: Temporal_DiffOptions | null): Temporal_Duration {
    let unit = "day";
    if (opts !== null && opts.largestUnit !== undefined && opts.largestUnit !== "auto") unit = opts.largestUnit;
    const aD = this.toPlainDate();
    let bD = other.toPlainDate();
    let timeNs = other.toPlainTime().ns() - this.toPlainTime().ns();
    const dateSign = __tpSign(bD.epochDays() - aD.epochDays());
    // Keep the date and time parts the same sign: borrow a day from the date.
    if (dateSign !== 0 && __tpSign(timeNs) === -dateSign) {
      bD = Temporal_PlainDate.fromEpochDays(bD.epochDays() - dateSign);
      timeNs += dateSign * 86400000000000;
    }
    const calendarUnit = unit === "year" || unit === "years" || unit === "month" || unit === "months" || unit === "week" || unit === "weeks" || unit === "day" || unit === "days";
    const sizes = [3600000000000, 60000000000, 1000000000, 1000000, 1000, 1];
    const out: number[] = [0, 0, 0, 0, 0, 0];
    let dateDur = new Temporal_Duration(0, 0, 0, 0);
    let first = 0;
    if (calendarUnit) {
      dateDur = aD.until(bD, opts);
    } else {
      // Smaller than a day: the day difference joins the time as nanoseconds.
      first = __tpUnitIndex(unit);
      const days = bD.epochDays() - aD.epochDays();
      let total = BigInt(days) * 86400000000000n + BigInt(timeNs);
      const big: bigint[] = [3600000000000n, 60000000000n, 1000000000n, 1000000n, 1000n, 1n];
      for (let i = first; i < 6; i++) {
        const q = total / big[i];
        out[i] = Number(q);
        total = total - q * big[i];
      }
      return new Temporal_Duration(0, 0, 0, 0, out[0] + 0, out[1] + 0, out[2] + 0, out[3] + 0, out[4] + 0, out[5] + 0);
    }
    let rest = timeNs;
    for (let i = 0; i < 6; i++) {
      const q = Math.trunc(rest / sizes[i]);
      out[i] = q;
      rest -= q * sizes[i];
    }
    return new Temporal_Duration(dateDur.years, dateDur.months, dateDur.weeks, dateDur.days, out[0] + 0, out[1] + 0, out[2] + 0, out[3] + 0, out[4] + 0, out[5] + 0);
  }
  until(other: Temporal_PlainDateTime, opts: Temporal_DiffOptions | null = null): Temporal_Duration {
    return this.diff(other, 1, opts);
  }
  since(other: Temporal_PlainDateTime, opts: Temporal_DiffOptions | null = null): Temporal_Duration {
    return this.diff(other, 1, opts).negated();
  }
  round(options: Temporal_RoundOptions): Temporal_PlainDateTime {
    const size = __tpUnitSize(__tpUnitOf(options));
    const day = 86400000000000n;
    const total = BigInt(this.toPlainDate().epochDays()) * day + BigInt(this.toPlainTime().ns());
    const r = __tpRound(total, size * __tpIncrement(options), __tpMode(options));
    let days = r / day;
    let rest = r - days * day;
    if (rest < 0n) {
      rest += day;
      days -= 1n;
    }
    const d = Temporal_PlainDate.fromEpochDays(Number(days));
    const t = Temporal_PlainTime.fromNs(Number(rest));
    return new Temporal_PlainDateTime(d.year, d.month, d.day, t.hour, t.minute, t.second, t.millisecond, t.microsecond, t.nanosecond);
  }
  equals(other: Temporal_PlainDateTime): boolean {
    return Temporal_PlainDateTime.compare(this, other) === 0;
  }
  static compare(a: Temporal_PlainDateTime, b: Temporal_PlainDateTime): number {
    const d = Temporal_PlainDate.compare(a.toPlainDate(), b.toPlainDate());
    if (d !== 0) return d;
    return Temporal_PlainTime.compare(a.toPlainTime(), b.toPlainTime());
  }
  toZonedDateTime(tz: string): Temporal_ZonedDateTime {
    return Temporal_ZonedDateTime.fromLocal(this, __tzCheck(tz), 0);
  }
  toString(): string {
    return this.toPlainDate().toString() + "T" + this.toPlainTime().toString();
  }
  toJSON(): string {
    return this.toString();
  }
}

function __tpDateTimeFrom(d: Temporal_PlainDateTime): Temporal_PlainDateTime {
  return new Temporal_PlainDateTime(d.year, d.month, d.day, d.hour, d.minute, d.second, d.millisecond, d.microsecond, d.nanosecond);
}

// ---- Instant ----

class Temporal_Instant {
  ns: bigint = 0n;
  get epochNanoseconds(): bigint {
    return this.ns;
  }
  get epochMilliseconds(): number {
    let q = this.ns / 1000000n;
    if (this.ns % 1000000n < 0n) q -= 1n;
    return Number(q);
  }
  static fromNs(ns: bigint): Temporal_Instant {
    const limit = 8640000000000000000000n;
    if (ns > limit || ns < -limit) throw new RangeError("epoch nanoseconds out of range");
    const i = new Temporal_Instant();
    i.ns = ns;
    return i;
  }
  static fromEpochMilliseconds(ms: number): Temporal_Instant {
    return Temporal_Instant.fromNs(BigInt(ms) * 1000000n);
  }
  static fromEpochNanoseconds(ns: bigint): Temporal_Instant {
    return Temporal_Instant.fromNs(ns);
  }
  addSigned(d: Temporal_Duration, sign: number): Temporal_Instant {
    if (d.years !== 0 || d.months !== 0 || d.weeks !== 0) throw new RangeError("a starting point is required for years, months and weeks");
    return Temporal_Instant.fromNs(this.ns + BigInt(sign) * d.timeNs());
  }
  add(d: Temporal_Duration): Temporal_Instant {
    return this.addSigned(d, 1);
  }
  subtract(d: Temporal_Duration): Temporal_Instant {
    return this.addSigned(d, -1);
  }
  diff(other: Temporal_Instant, sign: number, opts: Temporal_DiffOptions | null): Temporal_Duration {
    let unit = "second";
    if (opts !== null && opts.largestUnit !== undefined && opts.largestUnit !== "auto") unit = opts.largestUnit;
    let total = BigInt(sign) * (other.ns - this.ns);
    const sizes: bigint[] = [3600000000000n, 60000000000n, 1000000000n, 1000000n, 1000n, 1n];
    const start = __tpUnitIndex(unit);
    const out: number[] = [0, 0, 0, 0, 0, 0];
    for (let i = start; i < 6; i++) {
      const q = total / sizes[i];
      out[i] = Number(q);
      total = total - q * sizes[i];
    }
    return new Temporal_Duration(0, 0, 0, 0, out[0] + 0, out[1] + 0, out[2] + 0, out[3] + 0, out[4] + 0, out[5] + 0);
  }
  until(other: Temporal_Instant, opts: Temporal_DiffOptions | null = null): Temporal_Duration {
    return this.diff(other, 1, opts);
  }
  since(other: Temporal_Instant, opts: Temporal_DiffOptions | null = null): Temporal_Duration {
    return this.diff(other, 1, opts).negated();
  }
  round(options: Temporal_RoundOptions): Temporal_Instant {
    const size = __tpUnitSize(__tpUnitOf(options));
    if (size > 3600000000000n) throw new RangeError("invalid unit for Instant.round");
    return Temporal_Instant.fromNs(__tpRound(this.ns, size * __tpIncrement(options), __tpMode(options)));
  }
  equals(other: Temporal_Instant): boolean {
    return this.ns === other.ns;
  }
  static compare(a: Temporal_Instant, b: Temporal_Instant): number {
    return a.ns < b.ns ? -1 : a.ns > b.ns ? 1 : 0;
  }
  toZonedDateTimeISO(tz: string): Temporal_ZonedDateTime {
    return Temporal_ZonedDateTime.make(this.ns, tz);
  }
  toString(): string {
    let days = this.ns / 86400000000000n;
    let rest = this.ns % 86400000000000n;
    if (rest < 0n) {
      rest += 86400000000000n;
      days -= 1n;
    }
    const d = Temporal_PlainDate.fromEpochDays(Number(days));
    const t = Temporal_PlainTime.fromNs(Number(rest));
    return d.toString() + "T" + t.toString() + "Z";
  }
  toJSON(): string {
    return this.toString();
  }
}

function __tpParseInstant(s: string): Temporal_Instant {
  const c = new Temporal_Cursor(s);
  const d = __tpParseDateAt(c);
  if (!(c.eat("T") || c.eat("t") || c.eat(" "))) throw __tpBad(s);
  const t = __tpParseTimeAt(c);
  let offsetNs = 0;
  if (c.peek() === "Z" || c.peek() === "z") {
    c.at += 1;
  } else if (c.peek() === "+" || c.peek() === "-") {
    const neg = c.peek() === "-";
    c.at += 1;
    const oh = c.digits(2);
    c.eat(":");
    const om = c.isDigit() ? c.digits(2) : 0;
    offsetNs = (oh * 3600 + om * 60) * 1000000000;
    if (neg) offsetNs = -offsetNs;
  } else {
    throw __tpBad(s);
  }
  if (c.peek() === "[") {
    while (c.more() && c.peek() !== "]") c.at += 1;
    c.eat("]");
  }
  if (c.more()) throw __tpBad(s);
  const ns = BigInt(d.epochDays()) * 86400000000000n + BigInt(t.ns()) - BigInt(offsetNs);
  return Temporal_Instant.fromNs(ns);
}

function __tpInstantFrom(i: Temporal_Instant): Temporal_Instant {
  return Temporal_Instant.fromNs(i.ns);
}

class Temporal_Now {
  static timeZoneId(): string {
    return __tzSystem();
  }
  static zonedDateTimeISO(tz: string | null = null): Temporal_ZonedDateTime {
    return Temporal_ZonedDateTime.make(BigInt(Date.now()) * 1000000n, tz === null ? __tzSystem() : tz);
  }
  static instant(): Temporal_Instant {
    return Temporal_Instant.fromEpochMilliseconds(Date.now());
  }
  static plainDateISO(): Temporal_PlainDate {
    return Temporal_PlainDate.fromEpochDays(Math.floor(Date.now() / 86400000));
  }
  static plainTimeISO(): Temporal_PlainTime {
    return Temporal_PlainTime.fromNs((Date.now() % 86400000) * 1000000);
  }
  static plainDateTimeISO(): Temporal_PlainDateTime {
    const ms = Date.now();
    const d = Temporal_PlainDate.fromEpochDays(Math.floor(ms / 86400000));
    const t = Temporal_PlainTime.fromNs((ms % 86400000) * 1000000);
    return new Temporal_PlainDateTime(d.year, d.month, d.day, t.hour, t.minute, t.second, t.millisecond, t.microsecond, t.nanosecond);
  }
}

// ---- ZonedDateTime ----

interface Temporal_ZonedLike {
  year: number;
  month: number;
  day: number;
  hour?: number;
  minute?: number;
  second?: number;
  millisecond?: number;
  microsecond?: number;
  nanosecond?: number;
  timeZone: string;
}

function __tpFloorDiv(a: bigint, b: bigint): bigint {
  let q = a / b;
  if (a % b < 0n) q -= 1n;
  return q;
}

class Temporal_ZonedDateTime {
  ns: bigint = 0n;
  tz: string = "UTC";
  static make(ns: bigint, tz: string): Temporal_ZonedDateTime {
    const limit = 8640000000000000000000n;
    if (ns > limit || ns < -limit) throw new RangeError("epoch nanoseconds out of range");
    const z = new Temporal_ZonedDateTime();
    z.ns = ns;
    z.tz = __tzCheck(tz);
    return z;
  }
  constructor() {}
  get timeZoneId(): string {
    return this.tz;
  }
  get calendarId(): string {
    return "iso8601";
  }
  get epochNanoseconds(): bigint {
    return this.ns;
  }
  get epochMilliseconds(): number {
    return Number(__tpFloorDiv(this.ns, 1000000n));
  }
  offsetSeconds(): number {
    return __tzOffset(this.tz, Number(__tpFloorDiv(this.ns, 1000000000n)));
  }
  get offsetNanoseconds(): number {
    return this.offsetSeconds() * 1000000000;
  }
  get offset(): string {
    return __tpOffsetString(this.offsetSeconds());
  }
  local(): Temporal_PlainDateTime {
    const t = this.ns + BigInt(this.offsetSeconds()) * 1000000000n;
    const days = __tpFloorDiv(t, 86400000000000n);
    const rest = t - days * 86400000000000n;
    const d = Temporal_PlainDate.fromEpochDays(Number(days));
    const tm = Temporal_PlainTime.fromNs(Number(rest));
    return new Temporal_PlainDateTime(d.year, d.month, d.day, tm.hour, tm.minute, tm.second, tm.millisecond, tm.microsecond, tm.nanosecond);
  }
  get year(): number {
    return this.local().year;
  }
  get month(): number {
    return this.local().month;
  }
  get day(): number {
    return this.local().day;
  }
  get hour(): number {
    return this.local().hour;
  }
  get minute(): number {
    return this.local().minute;
  }
  get second(): number {
    return this.local().second;
  }
  get millisecond(): number {
    return this.local().millisecond;
  }
  get microsecond(): number {
    return this.local().microsecond;
  }
  get nanosecond(): number {
    return this.local().nanosecond;
  }
  get monthCode(): string {
    return this.local().monthCode;
  }
  get dayOfWeek(): number {
    return this.local().dayOfWeek;
  }
  get dayOfYear(): number {
    return this.local().dayOfYear;
  }
  get weekOfYear(): number {
    return this.local().weekOfYear;
  }
  get daysInMonth(): number {
    return this.local().daysInMonth;
  }
  get daysInYear(): number {
    return this.local().daysInYear;
  }
  get inLeapYear(): boolean {
    return this.local().inLeapYear;
  }
  // Local date and time to an instant in `tz`; mode 0 compatible, 1 earlier, 2 later, 3 reject.
  static fromLocal(dt: Temporal_PlainDateTime, tz: string, mode: number): Temporal_ZonedDateTime {
    const timeNs = dt.toPlainTime().ns();
    const whole = Math.floor(timeNs / 1000000000);
    const secs = __tzLocal(tz, dt.toPlainDate().epochDays(), whole, mode);
    return Temporal_ZonedDateTime.make(BigInt(secs) * 1000000000n + BigInt(timeNs - whole * 1000000000), tz);
  }
  toInstant(): Temporal_Instant {
    return Temporal_Instant.fromNs(this.ns);
  }
  toPlainDateTime(): Temporal_PlainDateTime {
    return this.local();
  }
  toPlainDate(): Temporal_PlainDate {
    return this.local().toPlainDate();
  }
  toPlainTime(): Temporal_PlainTime {
    return this.local().toPlainTime();
  }
  withTimeZone(tz: string): Temporal_ZonedDateTime {
    return Temporal_ZonedDateTime.make(this.ns, tz);
  }
  startOfDay(): Temporal_ZonedDateTime {
    const l = this.local();
    return Temporal_ZonedDateTime.fromLocal(new Temporal_PlainDateTime(l.year, l.month, l.day, 0, 0, 0, 0, 0, 0), this.tz, 0);
  }
  get hoursInDay(): number {
    const l = this.local();
    const start = this.startOfDay();
    const next = Temporal_PlainDate.fromEpochDays(l.toPlainDate().epochDays() + 1);
    const end = Temporal_ZonedDateTime.fromLocal(new Temporal_PlainDateTime(next.year, next.month, next.day, 0, 0, 0, 0, 0, 0), this.tz, 0);
    return Number(end.ns - start.ns) / 3600000000000;
  }
  with(like: Temporal_DateTimeLike): Temporal_ZonedDateTime {
    return Temporal_ZonedDateTime.fromLocal(this.local().with(like), this.tz, 0);
  }
  addSigned(d: Temporal_Duration, sign: number): Temporal_ZonedDateTime {
    let base = this.ns;
    if (d.years !== 0 || d.months !== 0 || d.weeks !== 0 || d.days !== 0) {
      const l = this.local();
      const moved = l.toPlainDate().addSigned(new Temporal_Duration(d.years, d.months, d.weeks, d.days), sign);
      const dt = new Temporal_PlainDateTime(moved.year, moved.month, moved.day, l.hour, l.minute, l.second, l.millisecond, l.microsecond, l.nanosecond);
      base = Temporal_ZonedDateTime.fromLocal(dt, this.tz, 0).ns;
    }
    const time = new Temporal_Duration(0, 0, 0, 0, d.hours, d.minutes, d.seconds, d.milliseconds, d.microseconds, d.nanoseconds);
    return Temporal_ZonedDateTime.make(base + BigInt(sign) * time.timeNs(), this.tz);
  }
  add(d: Temporal_Duration): Temporal_ZonedDateTime {
    return this.addSigned(d, 1);
  }
  subtract(d: Temporal_Duration): Temporal_ZonedDateTime {
    return this.addSigned(d, -1);
  }
  until(other: Temporal_ZonedDateTime, opts: Temporal_DiffOptions | null = null): Temporal_Duration {
    let unit = "hour";
    if (opts !== null && opts.largestUnit !== undefined && opts.largestUnit !== "auto") unit = opts.largestUnit;
    const calendar = unit === "year" || unit === "years" || unit === "month" || unit === "months" || unit === "week" || unit === "weeks" || unit === "day" || unit === "days";
    if (!calendar) {
      const o: Temporal_DiffOptions = { largestUnit: unit };
      return Temporal_Instant.fromNs(this.ns).until(Temporal_Instant.fromNs(other.ns), o);
    }
    // Calendar units: the difference of the local date-times (exact unless a clock change lies between).
    const diff = this.local().until(other.local(), opts);
    return diff;
  }
  since(other: Temporal_ZonedDateTime, opts: Temporal_DiffOptions | null = null): Temporal_Duration {
    return this.until(other, opts).negated();
  }
  round(options: Temporal_RoundOptions): Temporal_ZonedDateTime {
    const unit = __tpUnitOf(options);
    if (unit === "day" || unit === "days") {
      const start = this.startOfDay();
      const l = this.local();
      const next = Temporal_PlainDate.fromEpochDays(l.toPlainDate().epochDays() + 1);
      const end = Temporal_ZonedDateTime.fromLocal(new Temporal_PlainDateTime(next.year, next.month, next.day, 0, 0, 0, 0, 0, 0), this.tz, 0);
      const len = end.ns - start.ns;
      const r = __tpRound(this.ns - start.ns, len, __tpMode(options));
      return Temporal_ZonedDateTime.make(start.ns + r, this.tz);
    }
    const local = this.local().round(options);
    return Temporal_ZonedDateTime.fromLocal(local, this.tz, 0);
  }
  equals(other: Temporal_ZonedDateTime): boolean {
    return this.ns === other.ns && this.tz === other.tz;
  }
  static compare(a: Temporal_ZonedDateTime, b: Temporal_ZonedDateTime): number {
    return a.ns < b.ns ? -1 : a.ns > b.ns ? 1 : 0;
  }
  toString(): string {
    return this.local().toString() + this.offset + "[" + this.tz + "]";
  }
  toJSON(): string {
    return this.toString();
  }
}

function __tpOffsetString(secs: number): string {
  const sign = secs < 0 ? "-" : "+";
  const a = Math.abs(secs);
  const h = Math.floor(a / 3600);
  const m = Math.floor(a / 60) % 60;
  const s = a % 60;
  return sign + __tpPad(h, 2) + ":" + __tpPad(m, 2) + (s === 0 ? "" : ":" + __tpPad(s, 2));
}

function __tpParseZoned(s: string): Temporal_ZonedDateTime {
  const c = new Temporal_Cursor(s);
  const d = __tpParseDateAt(c);
  let t = new Temporal_PlainTime(0, 0, 0, 0, 0, 0);
  let hasOffset = false;
  let offsetSecs = 0;
  if (c.eat("T") || c.eat("t") || c.eat(" ")) {
    t = __tpParseTimeAt(c);
    if (c.peek() === "Z" || c.peek() === "z") {
      c.at += 1;
      hasOffset = true;
    } else if (c.peek() === "+" || c.peek() === "-") {
      const neg = c.peek() === "-";
      c.at += 1;
      const oh = c.digits(2);
      c.eat(":");
      const om = c.isDigit() ? c.digits(2) : 0;
      offsetSecs = (oh * 3600 + om * 60) * (neg ? -1 : 1);
      hasOffset = true;
    }
  }
  if (!c.eat("[")) throw new RangeError("a time zone annotation is required: " + s);
  let tz = "";
  while (c.more() && c.peek() !== "]") {
    tz += c.peek();
    c.at += 1;
  }
  if (!c.eat("]") || c.more()) throw __tpBad(s);
  const dt = new Temporal_PlainDateTime(d.year, d.month, d.day, t.hour, t.minute, t.second, t.millisecond, t.microsecond, t.nanosecond);
  const zone = __tzCheck(tz);
  if (hasOffset) {
    const ns = BigInt(d.epochDays()) * 86400000000000n + BigInt(t.ns()) - BigInt(offsetSecs) * 1000000000n;
    const z = Temporal_ZonedDateTime.make(ns, zone);
    if (z.offsetSeconds() !== offsetSecs) throw new RangeError("the offset does not match the time zone");
    return z;
  }
  return Temporal_ZonedDateTime.fromLocal(dt, zone, 0);
}

function __tpZonedFromLike(l: Temporal_ZonedLike): Temporal_ZonedDateTime {
  const dt = new Temporal_PlainDateTime(
    l.year,
    l.month,
    l.day,
    l.hour === undefined ? 0 : l.hour,
    l.minute === undefined ? 0 : l.minute,
    l.second === undefined ? 0 : l.second,
    l.millisecond === undefined ? 0 : l.millisecond,
    l.microsecond === undefined ? 0 : l.microsecond,
    l.nanosecond === undefined ? 0 : l.nanosecond,
  );
  return Temporal_ZonedDateTime.fromLocal(dt, l.timeZone, 0);
}

function __tpZonedFrom(z: Temporal_ZonedDateTime): Temporal_ZonedDateTime {
  return Temporal_ZonedDateTime.make(z.ns, z.tz);
}

// ---- PlainYearMonth and PlainMonthDay ----

interface Temporal_YearMonthLike {
  year?: number;
  month?: number;
}

interface Temporal_MonthDayLike {
  month?: number;
  day?: number;
}

class Temporal_PlainYearMonth {
  year: number = 1970;
  month: number = 1;
  constructor(year: number, month: number) {
    if (!(Number.isInteger(year) && Number.isInteger(month)) || month < 1 || month > 12) throw new RangeError("invalid year-month");
    this.year = year;
    this.month = month;
  }
  get calendarId(): string {
    return "iso8601";
  }
  get monthCode(): string {
    return "M" + __tpPad(this.month, 2);
  }
  get daysInMonth(): number {
    return __tpDaysInMonth(this.year, this.month);
  }
  get daysInYear(): number {
    return __tpIsLeap(this.year) ? 366 : 365;
  }
  get monthsInYear(): number {
    return 12;
  }
  get inLeapYear(): boolean {
    return __tpIsLeap(this.year);
  }
  // Months since year 0, January.
  index(): number {
    return this.year * 12 + (this.month - 1);
  }
  static fromIndex(i: number): Temporal_PlainYearMonth {
    const y = Math.floor(i / 12);
    return new Temporal_PlainYearMonth(y, i - y * 12 + 1);
  }
  with(like: Temporal_YearMonthLike): Temporal_PlainYearMonth {
    const m = like.month === undefined ? this.month : Math.min(12, Math.max(1, like.month));
    return new Temporal_PlainYearMonth(like.year === undefined ? this.year : like.year, m);
  }
  addSigned(d: Temporal_Duration, sign: number): Temporal_PlainYearMonth {
    if (d.weeks !== 0 || d.days !== 0 || d.hours !== 0 || d.minutes !== 0 || d.seconds !== 0 || d.milliseconds !== 0 || d.microseconds !== 0 || d.nanoseconds !== 0) {
      throw new RangeError("Duration must not have units below months for PlainYearMonth arithmetic");
    }
    return Temporal_PlainYearMonth.fromIndex(this.index() + sign * (d.years * 12 + d.months));
  }
  add(d: Temporal_Duration): Temporal_PlainYearMonth {
    return this.addSigned(d, 1);
  }
  subtract(d: Temporal_Duration): Temporal_PlainYearMonth {
    return this.addSigned(d, -1);
  }
  until(other: Temporal_PlainYearMonth, opts: Temporal_DiffOptions | null = null): Temporal_Duration {
    let unit = "year";
    if (opts !== null && opts.largestUnit !== undefined && opts.largestUnit !== "auto") unit = opts.largestUnit;
    const total = other.index() - this.index();
    if (unit === "month" || unit === "months") return new Temporal_Duration(0, total + 0);
    return new Temporal_Duration(Math.trunc(total / 12) + 0, (total % 12) + 0);
  }
  since(other: Temporal_PlainYearMonth, opts: Temporal_DiffOptions | null = null): Temporal_Duration {
    return this.until(other, opts).negated();
  }
  equals(other: Temporal_PlainYearMonth): boolean {
    return this.index() === other.index();
  }
  static compare(a: Temporal_PlainYearMonth, b: Temporal_PlainYearMonth): number {
    return __tpSign(a.index() - b.index());
  }
  toPlainDate(like: Temporal_DateLike): Temporal_PlainDate {
    return Temporal_PlainDate.constrained(this.year, this.month, like.day === undefined ? 1 : like.day);
  }
  toString(): string {
    return __tpYearString(this.year) + "-" + __tpPad(this.month, 2);
  }
  toJSON(): string {
    return this.toString();
  }
}

function __tpParseYearMonth(s: string): Temporal_PlainYearMonth {
  const c = new Temporal_Cursor(s);
  const y = __tpParseYear(c);
  c.eat("-");
  const m = c.digits(2);
  if (c.eat("-")) c.digits(2);
  if (c.more()) throw __tpBad(s);
  return new Temporal_PlainYearMonth(y, m);
}

function __tpYearMonthFrom(v: Temporal_PlainYearMonth): Temporal_PlainYearMonth {
  return new Temporal_PlainYearMonth(v.year, Math.min(12, Math.max(1, v.month)));
}

class Temporal_PlainMonthDay {
  month: number = 1;
  day: number = 1;
  constructor(month: number, day: number) {
    // 1972 is the reference year: a leap year, so February 29 exists.
    if (!(Number.isInteger(month) && Number.isInteger(day)) || month < 1 || month > 12 || day < 1 || day > __tpDaysInMonth(1972, month)) {
      throw new RangeError("invalid month-day");
    }
    this.month = month;
    this.day = day;
  }
  get calendarId(): string {
    return "iso8601";
  }
  get monthCode(): string {
    return "M" + __tpPad(this.month, 2);
  }
  with(like: Temporal_MonthDayLike): Temporal_PlainMonthDay {
    const m = like.month === undefined ? this.month : Math.min(12, Math.max(1, like.month));
    const d = like.day === undefined ? this.day : Math.max(1, like.day);
    return new Temporal_PlainMonthDay(m, Math.min(d, __tpDaysInMonth(1972, m)));
  }
  equals(other: Temporal_PlainMonthDay): boolean {
    return this.month === other.month && this.day === other.day;
  }
  toPlainDate(like: Temporal_DateLike): Temporal_PlainDate {
    if (like.year === undefined) throw new TypeError("a year is required");
    return Temporal_PlainDate.constrained(like.year, this.month, this.day);
  }
  toString(): string {
    return __tpPad(this.month, 2) + "-" + __tpPad(this.day, 2);
  }
  toJSON(): string {
    return this.toString();
  }
}

function __tpParseMonthDay(s: string): Temporal_PlainMonthDay {
  const c = new Temporal_Cursor(s);
  c.eat("-");
  c.eat("-");
  const m = c.digits(2);
  c.eat("-");
  const d = c.digits(2);
  if (c.more()) throw __tpBad(s);
  return new Temporal_PlainMonthDay(m, d);
}

function __tpMonthDayFrom(v: Temporal_PlainMonthDay): Temporal_PlainMonthDay {
  const m = Math.min(12, Math.max(1, v.month));
  return new Temporal_PlainMonthDay(m, Math.min(Math.max(1, v.day), __tpDaysInMonth(1972, m)));
}
