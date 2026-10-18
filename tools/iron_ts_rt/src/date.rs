//! `Date`: a time value is milliseconds since the epoch as an `f64` (NaN for an invalid date), and the
//! calendar maths follow the ECMAScript spec (MakeDay, MakeTime, TimeClip ...). Local time goes
//! through `jiff`'s time zone database: the OS zone, or the one set with `ts_rt_date_set_zone`.
//! Where an instant has two local readings (a clock set back) the earlier is used, and a time
//! skipped by a clock set forward is read with the offset from before the jump, as engines do.

use super::*;
use jiff::{Timestamp, civil, tz::TimeZone};
use std::cell::RefCell;

const MS_PER_DAY: f64 = 86_400_000.0;
const MAX_TIME: f64 = 8.64e15;

thread_local! {
    static ZONE: RefCell<Option<TimeZone>> = const { RefCell::new(None) };
}

fn zone() -> TimeZone {
    ZONE.with(|z| z.borrow_mut().get_or_insert_with(TimeZone::system).clone())
}

/// Sets the zone `Date` reads local time in (an IANA name such as `Europe/London`). Returns false
/// for a name the database does not know.
///
/// # Safety
/// `name` must point to `len` readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_date_set_zone(name: *const u8, len: i64) -> bool {
    // SAFETY: by contract.
    let bytes = unsafe { std::slice::from_raw_parts(name, len as usize) };
    let Ok(name) = std::str::from_utf8(bytes) else {
        return false;
    };
    match TimeZone::get(name) {
        Ok(tz) => {
            ZONE.with(|z| *z.borrow_mut() = Some(tz));
            true
        }
        Err(_) => false,
    }
}

// ---- the calendar ----

fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// year, month (0-11), date, hours, minutes, seconds, milliseconds, weekday of a time value.
fn fields(t: f64) -> [f64; 8] {
    let days = (t / MS_PER_DAY).floor();
    let in_day = (t - days * MS_PER_DAY) as i64;
    let (y, m, d) = civil_from_days(days as i64);
    [
        y as f64,
        (m - 1) as f64,
        d as f64,
        (in_day / 3_600_000) as f64,
        (in_day / 60_000 % 60) as f64,
        (in_day / 1000 % 60) as f64,
        (in_day % 1000) as f64,
        (days + 4.0).rem_euclid(7.0),
    ]
}

fn make_day(y: f64, m: f64, d: f64) -> f64 {
    if !(y.is_finite() && m.is_finite() && d.is_finite()) {
        return f64::NAN;
    }
    let (y, m, d) = (y.trunc(), m.trunc(), d.trunc());
    let ym = y + (m / 12.0).floor();
    if ym.abs() > 1e7 {
        return f64::NAN;
    }
    let mn = m.rem_euclid(12.0) as i64;
    days_from_civil(ym as i64, mn + 1, 1) as f64 + d - 1.0
}

fn make_time(h: f64, m: f64, s: f64, ms: f64) -> f64 {
    if !(h.is_finite() && m.is_finite() && s.is_finite() && ms.is_finite()) {
        return f64::NAN;
    }
    h.trunc() * 3_600_000.0 + m.trunc() * 60_000.0 + s.trunc() * 1000.0 + ms.trunc()
}

fn make_date(day: f64, time: f64) -> f64 {
    let t = day * MS_PER_DAY + time;
    if t.is_finite() { t } else { f64::NAN }
}

fn time_clip(t: f64) -> f64 {
    if !t.is_finite() || t.abs() > MAX_TIME {
        f64::NAN
    } else {
        t.trunc() + 0.0
    }
}

// ---- local time ----

/// The Gregorian calendar repeats exactly every 400 years (a whole number of weeks), so a time
/// beyond jiff's range (years -9999..9999) has the same zone rules as the one a few cycles closer.
const CYCLE_MS: f64 = 12_622_780_800_000.0;
const JIFF_MIN: f64 = -377_705_023_201_000.0;
const JIFF_MAX: f64 = 253_402_207_200_000.0;

/// How many cycles to add to bring `t` into jiff's range.
fn cycles_into_range(t: f64) -> f64 {
    if t > JIFF_MAX {
        -((t - JIFF_MAX) / CYCLE_MS).ceil()
    } else if t < JIFF_MIN {
        ((JIFF_MIN - t) / CYCLE_MS).ceil()
    } else {
        0.0
    }
}

fn clamp_ts(t: f64) -> Timestamp {
    let ms = (t + cycles_into_range(t) * CYCLE_MS).clamp(JIFF_MIN, JIFF_MAX) as i64;
    Timestamp::from_millisecond(ms).expect("clamped to jiff's range")
}

/// The local offset from UTC at an instant, in milliseconds.
fn offset_ms(t: f64) -> f64 {
    zone().to_offset(clamp_ts(t)).seconds() as f64 * 1000.0
}

/// UTC(t): the instant whose local reading is `local`.
fn utc_from_local(local: f64) -> f64 {
    if !local.is_finite() {
        return f64::NAN;
    }
    let days = (local / MS_PER_DAY).floor();
    let in_day = (local - days * MS_PER_DAY) as i64;
    let (y, m, d) = civil_from_days(days as i64);
    let (h, mi, s, ms) = (
        in_day / 3_600_000,
        in_day / 60_000 % 60,
        in_day / 1000 % 60,
        in_day % 1000,
    );
    if !(-9000..=9000).contains(&y) {
        // Beyond jiff's range: the same wall time a few 400-year cycles away.
        let k = if y > 0 {
            -((y - 8000) as f64 / 400.0).ceil()
        } else {
            ((-8000 - y) as f64 / 400.0).ceil()
        };
        return utc_from_local(local + k * CYCLE_MS) - k * CYCLE_MS;
    }
    let dt = civil::DateTime::new(
        y as i16,
        m as i8,
        d as i8,
        h as i8,
        mi as i8,
        s as i8,
        (ms * 1_000_000) as i32,
    );
    match dt.and_then(|dt| zone().to_ambiguous_timestamp(dt).compatible()) {
        Ok(ts) => ts.as_millisecond() as f64,
        Err(_) => local - offset_ms(local),
    }
}

fn to_local(t: f64) -> f64 {
    t + offset_ms(t)
}

// ---- the functions compiled code calls ----

/// `Date.now()`.
#[unsafe(no_mangle)]
pub extern "C" fn ts_rt_date_now() -> f64 {
    let d = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    d.as_millis() as f64
}

/// One field of a time value: 0 year, 1 month, 2 date, 3 hours, 4 minutes, 5 seconds,
/// 6 milliseconds, 7 weekday; in local time or UTC.
#[unsafe(no_mangle)]
pub extern "C" fn ts_rt_date_field(t: f64, field: f64, local: bool) -> f64 {
    if t.is_nan() {
        return f64::NAN;
    }
    fields(if local { to_local(t) } else { t })[field as usize]
}

/// `getTimezoneOffset()`: minutes UTC is ahead of local time.
#[unsafe(no_mangle)]
pub extern "C" fn ts_rt_date_offset(t: f64) -> f64 {
    if t.is_nan() {
        return f64::NAN;
    }
    // Whole minutes toward zero, as engines report an old local mean time (-4:56:02 is 296).
    -(offset_ms(t) / 60_000.0).trunc() + 0.0
}

fn from_fields(f: [f64; 7], local: bool) -> f64 {
    let tv = make_date(
        make_day(f[0], f[1], f[2]),
        make_time(f[3], f[4], f[5], f[6]),
    );
    time_clip(if local { utc_from_local(tv) } else { tv })
}

/// `new Date(y, m, d, h, mi, s, ms)` and `Date.UTC(..)`: a year of 0 to 99 means 1900 to 1999.
#[unsafe(no_mangle)]
pub extern "C" fn ts_rt_date_from_parts(
    y: f64,
    mo: f64,
    d: f64,
    h: f64,
    mi: f64,
    s: f64,
    ms: f64,
    local: bool,
) -> f64 {
    let y = if y.is_nan() {
        y
    } else if (0.0..=99.0).contains(&y.trunc()) {
        1900.0 + y.trunc()
    } else {
        y
    };
    from_fields([y, mo, d, h, mi, s, ms], local)
}

/// The `set*` methods: replaces `count` fields from `first` on (in the order year, month, date,
/// hours, minutes, seconds, milliseconds) with `a0..` and returns the new time value.
#[unsafe(no_mangle)]
pub extern "C" fn ts_rt_date_set(
    t: f64,
    first: f64,
    count: f64,
    local: bool,
    a0: f64,
    a1: f64,
    a2: f64,
    a3: f64,
) -> f64 {
    let first = first as usize;
    let base = if t.is_nan() {
        // Only setFullYear works on an invalid date (as if it were the epoch).
        if first != 0 {
            return f64::NAN;
        }
        0.0
    } else if local {
        to_local(t)
    } else {
        t
    };
    let mut f = fields(base);
    for (i, a) in [a0, a1, a2, a3]
        .into_iter()
        .take(count as usize)
        .enumerate()
    {
        f[first + i] = a;
    }
    from_fields([f[0], f[1], f[2], f[3], f[4], f[5], f[6]], local)
}

/// `Date.prototype.setTime` and `new Date(number)`.
#[unsafe(no_mangle)]
pub extern "C" fn ts_rt_date_clip(t: f64) -> f64 {
    time_clip(t)
}

const DAYS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

fn year_text(y: i64) -> String {
    if y < 0 {
        format!("-{:04}", -y)
    } else {
        format!("{y:04}")
    }
}

fn new_str(s: &str) -> *mut Str {
    // SAFETY: valid bytes.
    unsafe { ts_rt_str_new_gc(s.as_ptr(), s.len() as i64) }
}

/// `toISOString()` for a valid time value.
#[unsafe(no_mangle)]
pub extern "C" fn ts_rt_date_iso_gc(t: f64) -> *mut Str {
    if t.is_nan() {
        raise(1.0, "Invalid Date");
        return new_str("");
    }
    let f = fields(t);
    let y = f[0] as i64;
    let year = if (0..=9999).contains(&y) {
        format!("{y:04}")
    } else {
        format!("{}{:06}", if y < 0 { '-' } else { '+' }, y.abs())
    };
    new_str(&format!(
        "{year}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
        f[1] as i64 + 1,
        f[2] as i64,
        f[3] as i64,
        f[4] as i64,
        f[5] as i64,
        f[6] as i64
    ))
}

/// `toUTCString()`: `Mon, 19 Oct 2026 12:00:00 GMT`.
#[unsafe(no_mangle)]
pub extern "C" fn ts_rt_date_utc_string_gc(t: f64) -> *mut Str {
    if t.is_nan() {
        return new_str("Invalid Date");
    }
    let f = fields(t);
    new_str(&format!(
        "{}, {:02} {} {} {:02}:{:02}:{:02} GMT",
        DAYS[f[7] as usize],
        f[2] as i64,
        MONTHS[f[1] as usize],
        year_text(f[0] as i64),
        f[3] as i64,
        f[4] as i64,
        f[5] as i64
    ))
}

/// `toDateString()`: `Mon Oct 19 2026` (local).
#[unsafe(no_mangle)]
pub extern "C" fn ts_rt_date_date_string_gc(t: f64) -> *mut Str {
    if t.is_nan() {
        return new_str("Invalid Date");
    }
    let f = fields(to_local(t));
    new_str(&format!(
        "{} {} {:02} {}",
        DAYS[f[7] as usize],
        MONTHS[f[1] as usize],
        f[2] as i64,
        year_text(f[0] as i64)
    ))
}

// ---- parsing ----

/// `Date.parse(string)` / `new Date(string)`: the ISO format, then the legacy forms engines accept
/// (`Oct 19 2026 10:30 PM`, `10/19/2026`, `Mon, 19 Oct 2026 10:30:00 GMT+0100` ...).
///
/// # Safety
/// `s` must be a live string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_date_parse(s: *const Str) -> f64 {
    // SAFETY: by contract.
    let text = unsafe { (*s).to_string_lossy() };
    // The ISO form is strict about spaces around it; the legacy forms skip them.
    parse_iso(&text)
        .or_else(|| parse_legacy(text.trim()))
        .map_or(f64::NAN, |(local, v, off)| {
            time_clip(match off {
                Some(o) => v - o as f64 * 60_000.0,
                None if local => utc_from_local(v),
                None => v,
            })
        })
}

type Parsed = Option<(bool, f64, Option<i32>)>;

fn digits(b: &[u8], i: &mut usize, n: usize) -> Option<i64> {
    if *i + n > b.len() || !b[*i..*i + n].iter().all(u8::is_ascii_digit) {
        return None;
    }
    let v = b[*i..*i + n]
        .iter()
        .fold(0i64, |a, d| a * 10 + (d - b'0') as i64);
    *i += n;
    Some(v)
}

/// `YYYY[-MM[-DD]][Thh:mm[:ss[.fff]][Z|+hh:mm]]` (and a space for the `T`). A date alone is UTC, a
/// date with a time and no offset is local.
fn parse_iso(s: &str) -> Parsed {
    let b = s.as_bytes();
    let mut i = 0;
    let year = match b.first()? {
        b'+' | b'-' => {
            let neg = b[0] == b'-';
            i = 1;
            let y = digits(b, &mut i, 6)?;
            if neg && y == 0 {
                return None;
            }
            if neg { -y } else { y }
        }
        _ => digits(b, &mut i, 4)?,
    };
    let (mut month, mut day) = (1, 1);
    if b.get(i) == Some(&b'-') {
        i += 1;
        month = digits(b, &mut i, 2)?;
        if b.get(i) == Some(&b'-') {
            i += 1;
            day = digits(b, &mut i, 2)?;
        }
    }
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    let (mut h, mut mi, mut sec, mut ms) = (0, 0, 0, 0.0);
    let mut has_time = false;
    let mut offset = None;
    if matches!(b.get(i), Some(b'T' | b't' | b' ')) {
        i += 1;
        has_time = true;
        h = digits(b, &mut i, 2)?;
        if b.get(i) != Some(&b':') {
            return None;
        }
        i += 1;
        mi = digits(b, &mut i, 2)?;
        if b.get(i) == Some(&b':') {
            i += 1;
            sec = digits(b, &mut i, 2)?;
            if b.get(i) == Some(&b'.') {
                i += 1;
                let start = i;
                while b.get(i).is_some_and(u8::is_ascii_digit) {
                    i += 1;
                }
                if i == start {
                    return None;
                }
                let frac = &s[start..i.min(start + 3)];
                ms = frac.parse::<f64>().ok()? * 10f64.powi(3 - frac.len() as i32);
            }
        }
        match b.get(i) {
            Some(b'Z' | b'z') => {
                i += 1;
                offset = Some(0);
            }
            Some(sign @ (b'+' | b'-')) => {
                i += 1;
                let hh = digits(b, &mut i, 2)?;
                if b.get(i) == Some(&b':') {
                    i += 1;
                }
                let mm = digits(b, &mut i, 2)?;
                if hh > 23 || mm > 59 {
                    return None;
                }
                let o = (hh * 60 + mm) as i32;
                offset = Some(if *sign == b'-' { -o } else { o });
            }
            _ => {}
        }
        if h > 24 || mi > 59 || sec > 59 || (h == 24 && (mi, sec, ms as i64) != (0, 0, 0)) {
            return None;
        }
    }
    if i != b.len() {
        return None;
    }
    let v = make_date(
        make_day(year as f64, (month - 1) as f64, day as f64),
        make_time(h as f64, mi as f64, sec as f64, ms),
    );
    // A bare date is UTC; with a time and no offset it is local.
    Some((has_time, v, if has_time { offset } else { Some(0) }))
}

enum Tok {
    Num(i64, usize),
    Word(String),
    Sym(char),
}

fn tokenize(s: &str) -> Option<Vec<Tok>> {
    let c: Vec<char> = s.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < c.len() {
        let ch = c[i];
        if ch.is_whitespace() {
            i += 1;
        } else if ch == '(' {
            // A comment, e.g. the `(BST)` after a zone offset.
            let mut depth = 0;
            while i < c.len() {
                match c[i] {
                    '(' => depth += 1,
                    ')' => depth -= 1,
                    _ => {}
                }
                i += 1;
                if depth == 0 {
                    break;
                }
            }
        } else if ch.is_ascii_digit() {
            let start = i;
            while i < c.len() && c[i].is_ascii_digit() {
                i += 1;
            }
            if i - start > 9 {
                return None;
            }
            let text: String = c[start..i].iter().collect();
            out.push(Tok::Num(text.parse().ok()?, i - start));
        } else if ch.is_ascii_alphabetic() {
            let start = i;
            while i < c.len() && c[i].is_ascii_alphabetic() {
                i += 1;
            }
            out.push(Tok::Word(
                c[start..i].iter().collect::<String>().to_ascii_lowercase(),
            ));
        } else if ",/:.+-".contains(ch) {
            out.push(Tok::Sym(ch));
            i += 1;
        } else {
            return None;
        }
    }
    Some(out)
}

fn zone_abbreviation(w: &str) -> Option<i32> {
    Some(match w {
        "gmt" | "utc" | "ut" | "z" => 0,
        "est" => -300,
        "edt" => -240,
        "cst" => -360,
        "cdt" => -300,
        "mst" => -420,
        "mdt" => -360,
        "pst" => -480,
        "pdt" => -420,
        _ => return None,
    })
}

/// The forms with month names or slashes. Returns (local, local-or-UTC time value, offset).
fn parse_legacy(s: &str) -> Parsed {
    let t = tokenize(s)?;
    let mut i = 0;
    let (mut year, mut month, mut day): (Option<i64>, Option<i64>, Option<i64>) =
        (None, None, None);
    let (mut h, mut mi, mut sec, mut ms) = (0i64, 0i64, 0i64, 0f64);
    let mut has_time = false;
    let mut pm: Option<bool> = None;
    let mut offset: Option<i32> = None;
    let mut loose: Vec<(i64, usize)> = Vec::new();
    let two_digit = |y: i64, n: usize| {
        if n <= 2 && y < 50 {
            y + 2000
        } else if n <= 2 && y < 100 {
            y + 1900
        } else {
            y
        }
    };
    while i < t.len() {
        match &t[i] {
            Tok::Num(a, na) => {
                match (t.get(i + 1), has_time) {
                    // 10:30[:15[.25]]
                    (Some(Tok::Sym(':')), false) => {
                        let Some(Tok::Num(m, _)) = t.get(i + 2) else {
                            return None;
                        };
                        h = *a;
                        mi = *m;
                        has_time = true;
                        i += 3;
                        if matches!(t.get(i), Some(Tok::Sym(':'))) {
                            let Some(Tok::Num(sv, _)) = t.get(i + 1) else {
                                return None;
                            };
                            sec = *sv;
                            i += 2;
                            if matches!(t.get(i), Some(Tok::Sym('.'))) {
                                let Some(Tok::Num(f, nf)) = t.get(i + 1) else {
                                    return None;
                                };
                                let text = format!("{f:0width$}", width = nf);
                                let frac = &text[..text.len().min(3)];
                                ms = frac.parse::<f64>().ok()? * 10f64.powi(3 - frac.len() as i32);
                                i += 2;
                            }
                        }
                        continue;
                    }
                    // 10/19/2026, 2026/10/19, 2026-1-5
                    (Some(Tok::Sym(sep @ ('/' | '-'))), _)
                        if year.is_none() && day.is_none() && month.is_none() =>
                    {
                        let Some(Tok::Num(b, nb)) = t.get(i + 2) else {
                            return None;
                        };
                        let mut parts = vec![(*a, *na), (*b, *nb)];
                        let mut j = i + 3;
                        if matches!(t.get(j), Some(Tok::Sym(c)) if c == sep)
                            && let Some(Tok::Num(c3, n3)) = t.get(j + 1)
                        {
                            parts.push((*c3, *n3));
                            j += 2;
                        }
                        if *na >= 3 || parts[0].0 > 31 {
                            // year/month/day
                            year = Some(parts[0].0);
                            month = Some(parts[1].0);
                            day = Some(parts.get(2).map_or(1, |p| p.0));
                        } else if parts.len() == 3 {
                            month = Some(parts[0].0);
                            day = Some(parts[1].0);
                            year = Some(two_digit(parts[2].0, parts[2].1));
                        } else {
                            return None;
                        }
                        i = j;
                        continue;
                    }
                    _ => loose.push((*a, *na)),
                }
                i += 1;
            }
            Tok::Word(w) => {
                let w = w.as_str();
                if let Some(k) = month_index(w) {
                    if month.is_some() {
                        return None;
                    }
                    month = Some(k + 1);
                } else if weekday_index(w).is_some() {
                } else if w == "am" || w == "pm" {
                    pm = Some(w == "pm");
                } else {
                    // A zone name; a sign after GMT/UTC is read as its offset next.
                    offset = Some(zone_abbreviation(w)?);
                }
                i += 1;
            }
            Tok::Sym(sign @ ('+' | '-')) if has_time || offset.is_some() => {
                let Some(Tok::Num(v, n)) = t.get(i + 1) else {
                    return None;
                };
                let (hh, mm, used) = if *n <= 2 {
                    if matches!(t.get(i + 2), Some(Tok::Sym(':'))) {
                        let Some(Tok::Num(m, _)) = t.get(i + 3) else {
                            return None;
                        };
                        (*v, *m, 4)
                    } else {
                        (*v, 0, 2)
                    }
                } else {
                    (*v / 100, *v % 100, 2)
                };
                let o = (hh * 60 + mm) as i32;
                offset = Some(if *sign == '-' { -o } else { o });
                i += used;
            }
            Tok::Sym(',' | '.') => i += 1,
            Tok::Sym(_) => return None,
        }
    }
    // Loose numbers: the day and the year around a month name.
    match (month, loose.as_slice()) {
        (Some(_), []) if day.is_none() => return None,
        (Some(_), [(a, na)]) if day.is_none() && year.is_none() => {
            if *na >= 3 || *a > 31 {
                year = Some(*a);
                day = Some(1);
            } else {
                return None;
            }
        }
        (Some(_), [(a, na), (b, nb)]) if day.is_none() && year.is_none() => {
            if *na >= 3 || *a > 31 {
                year = Some(*a);
                day = Some(*b);
            } else {
                day = Some(*a);
                year = Some(two_digit(*b, *nb));
            }
        }
        (None, []) if year.is_some() => {}
        _ if loose.is_empty() => {}
        _ => return None,
    }
    let (year, month, day) = (year?, month?, day?);
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    if let Some(is_pm) = pm {
        if !has_time || !(1..=12).contains(&h) {
            return None;
        }
        h = h % 12 + if is_pm { 12 } else { 0 };
    }
    if h > 24 || mi > 59 || sec > 59 || (h == 24 && (mi, sec) != (0, 0)) {
        return None;
    }
    let v = make_date(
        make_day(year as f64, (month - 1) as f64, day as f64),
        make_time(h as f64, mi as f64, sec as f64, ms),
    );
    Some((true, v, offset))
}

fn month_index(w: &str) -> Option<i64> {
    const FULL: [&str; 12] = [
        "january",
        "february",
        "march",
        "april",
        "may",
        "june",
        "july",
        "august",
        "september",
        "october",
        "november",
        "december",
    ];
    // The first three letters decide; the rest of the word is ignored ("Octember" is October).
    if w.len() < 3 || !w.is_ascii() {
        return None;
    }
    FULL.iter().position(|m| m[..3] == w[..3]).map(|k| k as i64)
}

fn weekday_index(w: &str) -> Option<usize> {
    const FULL: [&str; 7] = [
        "sunday",
        "monday",
        "tuesday",
        "wednesday",
        "thursday",
        "friday",
        "saturday",
    ];
    if w.len() < 3 || !w.is_ascii() {
        return None;
    }
    FULL.iter().position(|d| d[..3] == w[..3])
}
