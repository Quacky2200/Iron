// SPDX-License-Identifier: Apache-2.0

//! `Intl`, step I3a: `Intl.DateTimeFormat` and `Date.prototype.toLocaleString/DateString/TimeString`
//! on ICU4X field sets (the `icu-lite` backend; see `docs/research/iron-ts-intl-datetime.md`).
//! ECMA-402 options are mapped to the nearest ICU4X field set: `dateStyle`/`timeStyle` exactly;
//! component options (`year`, `month`, `weekday`, `hour` ...) by which fields are present and how
//! wide the month or weekday is. ICU4X has no `availableFormats`, so rare combinations are
//! rejected or differ from ICU4C; `docs/differences.md` lists them.

use super::*;
use icu::calendar::Gregorian;
use icu::datetime::fieldsets::builder::{DateFields, FieldSetBuilder, ZoneStyle};
use icu::datetime::fieldsets::enums::CompositeFieldSet;
use icu::datetime::input::{Date, Time, ZonedDateTime};
use icu::datetime::options::{Length, SubsecondDigits, TimePrecision, YearStyle};
use icu::datetime::pattern::{DateTimePattern, FixedCalendarDateTimeNames};
use icu::datetime::{DateTimeFormatter, DateTimeFormatterPreferences};
use icu::locale::preferences::extensions::unicode::keywords::HourCycle;
use jiff::tz::TimeZone;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

struct DtFmt {
    formatter: DateTimeFormatter<CompositeFieldSet>,
    /// `timeZone`, or None for the program's zone.
    zone: Option<TimeZone>,
    /// The nearest field set's pattern with the widths the options asked for (`numeric` or
    /// `2-digit` month and day, the year, the weekday and month names), formatted through typed
    /// names; None when the pattern already fits or the calendar is not Gregorian.
    adjusted: Option<Adjusted>,
    /// The locale as `resolvedOptions` shows it, and the zone's name.
    locale_tag: String,
    zone_name: String,
}

struct Adjusted {
    pattern: DateTimePattern,
    names: RefCell<FixedCalendarDateTimeNames<Gregorian, CompositeFieldSet>>,
}

/// The widths the component options asked for.
#[derive(Default, Clone)]
struct Widths {
    year: Option<String>,
    month: Option<String>,
    day: Option<String>,
    weekday: Option<String>,
    hour: Option<String>,
    minute: Option<String>,
}

/// Rewrites the field widths of a CLDR pattern (`dd.MM.y` to `d.M.y`, `E` to `EEEE`). Only widths
/// of the same kind change: numeric stays numeric, text stays text.
fn want(width: &Option<String>) -> Option<&str> {
    width.as_deref()
}

fn adjust_pattern(pattern: &str, w: &Widths, pad: Option<super::intl_pad::Padding>) -> String {
    let has_year = pattern.contains('y');
    let numeric_month = pattern.contains('M') && !pattern.contains("MMM");
    let has_seconds = pattern.contains('s');
    let two_digit_minute = w.minute.as_deref() == Some("2-digit");
    // Does the reference engine pad this numeric field in the locale's own pattern?
    let pads = |month: bool, day: bool, hour: bool| {
        pad.map(|p| {
            if month {
                if has_year { p.month } else { p.month_md }
            } else if day {
                // A day beside a month name is not padded.
                if !numeric_month {
                    false
                } else if has_year {
                    p.day
                } else {
                    p.day_md
                }
            } else if hour {
                match (has_seconds, two_digit_minute) {
                    (false, false) => p.hm_numeric,
                    (false, true) => p.hm_two_digit,
                    (true, false) => p.hms_numeric,
                    (true, true) => p.hms_two_digit,
                }
            } else {
                false
            }
        })
    };
    let chars: Vec<char> = pattern.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    let mut quoted = false;
    while i < chars.len() {
        let c = chars[i];
        if c == '\'' {
            quoted = !quoted;
            out.push(c);
            i += 1;
            continue;
        }
        if quoted || !c.is_ascii_alphabetic() {
            out.push(c);
            i += 1;
            continue;
        }
        let mut n = 1;
        while i + n < chars.len() && chars[i + n] == c {
            n += 1;
        }
        let count = match c {
            'y' => match want(&w.year) {
                Some("2-digit") => 2,
                Some(_) if n <= 2 => 1,
                _ => n,
            },
            'M' | 'L' => match want(&w.month) {
                Some("numeric") if n <= 2 => match pads(true, false, false) {
                    Some(true) => 2,
                    Some(false) => 1,
                    None => n,
                },
                Some("2-digit") if n <= 2 => 2,
                Some("short") if n >= 3 => 3,
                Some("long") if n >= 3 => 4,
                Some("narrow") if n >= 3 => 5,
                _ => n,
            },
            'd' => match want(&w.day) {
                Some("numeric") => match pads(false, true, false) {
                    Some(true) => 2,
                    Some(false) => 1,
                    None => n,
                },
                Some("2-digit") => 2,
                _ => n,
            },
            'E' | 'c' => match want(&w.weekday) {
                Some("short") => 3,
                Some("long") => 4,
                Some("narrow") => 5,
                _ => n,
            },
            'h' | 'H' | 'K' | 'k' => match want(&w.hour) {
                Some("2-digit") => 2,
                // A 12-hour clock never pads; a 24-hour one as the reference engine does.
                Some("numeric") if c == 'H' || c == 'k' => match pads(false, false, true) {
                    Some(true) => 2,
                    Some(false) => 1,
                    None => n,
                },
                _ => n,
            },
            _ => n,
        };
        for _ in 0..count {
            out.push(c);
        }
        i += n;
    }
    out
}

thread_local! {
    static FORMATS: RefCell<Vec<Rc<DtFmt>>> = const { RefCell::new(Vec::new()) };
    static IDS: RefCell<HashMap<(String, String, i64), usize>> = RefCell::new(HashMap::new());
}

type Opts = HashMap<String, String>;

fn parse_opts(options: &str) -> Opts {
    options
        .split(';')
        .filter(|s| !s.is_empty())
        .filter_map(|kv| kv.split_once('='))
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

fn pick<'a>(o: &'a Opts, key: &str, allowed: &[&str]) -> Result<Option<&'a str>, String> {
    match o.get(key) {
        None => Ok(None),
        Some(v) if allowed.contains(&v.as_str()) => Ok(Some(v.as_str())),
        Some(_) => {
            let list: Vec<String> = allowed.iter().map(|a| format!("\"{a}\"")).collect();
            let text = match list.split_last() {
                Some((last, rest)) if !rest.is_empty() => format!("{}, or {last}", rest.join(", ")),
                Some((last, _)) => last.clone(),
                None => String::new(),
            };
            Err(format!("{key} must be {text}"))
        }
    }
}

/// ToDateTimeOptions: the date and time fields a formatter shows, with the defaults of the entry
/// point (`kind`: 0 `toLocaleString`, 1 `toLocaleDateString`, 2 `toLocaleTimeString`, 3 the
/// `DateTimeFormat` constructor).
fn with_defaults(o: &mut Opts, kind: i64) -> Result<(), String> {
    let has = |o: &Opts, ks: &[&str]| ks.iter().any(|k| o.contains_key(*k));
    let styled = has(o, &["dateStyle", "timeStyle"]);
    if styled {
        if kind == 1 && o.contains_key("timeStyle") {
            return Err(
                "TypeError: timeStyle is specified while formatting date is requested".into(),
            );
        }
        if kind == 2 && o.contains_key("dateStyle") {
            return Err(
                "TypeError: dateStyle is specified while formatting time is requested".into(),
            );
        }
        for k in [
            "weekday",
            "year",
            "month",
            "day",
            "era",
            "dayPeriod",
            "hour",
            "minute",
            "second",
            "fractionalSecondDigits",
            "timeZoneName",
        ] {
            if o.contains_key(k) {
                return Err("TypeError: dateStyle and timeStyle may not be used with other DateTimeFormat options".into());
            }
        }
        return Ok(());
    }
    let mut need = true;
    if kind != 2 && has(o, &["weekday", "year", "month", "day"]) {
        need = false;
    }
    if kind != 1
        && has(
            o,
            &[
                "dayPeriod",
                "hour",
                "minute",
                "second",
                "fractionalSecondDigits",
            ],
        )
    {
        need = false;
    }
    if need && kind != 2 {
        for k in ["year", "month", "day"] {
            o.insert(k.into(), "numeric".into());
        }
    }
    if need && (kind == 0 || kind == 2) {
        for k in ["hour", "minute", "second"] {
            o.insert(k.into(), "numeric".into());
        }
    }
    Ok(())
}

fn build(locales: &str, options: &str, kind: i64) -> Result<DtFmt, String> {
    let locale = super::intl::resolve_locale(locales)?;
    let mut o = parse_opts(options);
    pick(&o, "calendar", &["gregory"])?;
    pick(&o, "numberingSystem", &["latn"])?;
    let zone = match o.get("timeZone") {
        Some(name) => {
            let n = if name.eq_ignore_ascii_case("utc") {
                "UTC"
            } else {
                name.as_str()
            };
            Some(TimeZone::get(n).map_err(|_| format!("invalid time zone: {name}"))?)
        }
        None => None,
    };
    with_defaults(&mut o, kind)?;
    let mut prefs: DateTimeFormatterPreferences = (&locale).into();
    match (
        o.get("hour12").map(String::as_str),
        pick(&o, "hourCycle", &["h11", "h12", "h23", "h24"])?,
    ) {
        (Some("true"), _) => prefs.hour_cycle = Some(HourCycle::H12),
        (Some("false"), _) => prefs.hour_cycle = Some(HourCycle::H23),
        (_, Some("h11")) => prefs.hour_cycle = Some(HourCycle::H11),
        (_, Some("h12")) => prefs.hour_cycle = Some(HourCycle::H12),
        (_, Some("h23")) => prefs.hour_cycle = Some(HourCycle::H23),
        (_, Some("h24")) => prefs.hour_cycle = Some(HourCycle::H23),
        _ => {}
    }
    let mut b = FieldSetBuilder::new();
    let mut widths = Widths::default();
    let date_style = pick(&o, "dateStyle", &["full", "long", "medium", "short"])?;
    let time_style = pick(&o, "timeStyle", &["full", "long", "medium", "short"])?;
    if date_style.is_some() || time_style.is_some() {
        match date_style {
            Some("full") => {
                b.length = Some(Length::Long);
                b.date_fields = Some(DateFields::YMDE);
            }
            Some("long") => {
                b.length = Some(Length::Long);
                b.date_fields = Some(DateFields::YMD);
            }
            Some("medium") => {
                b.length = Some(Length::Medium);
                b.date_fields = Some(DateFields::YMD);
            }
            Some(_) => {
                b.length = Some(Length::Short);
                b.date_fields = Some(DateFields::YMD);
            }
            None => {}
        }
        match time_style {
            Some("full") => {
                b.time_precision = Some(TimePrecision::Second);
                b.zone_style = Some(ZoneStyle::SpecificLong);
            }
            Some("long") => {
                b.time_precision = Some(TimePrecision::Second);
                b.zone_style = Some(ZoneStyle::SpecificShort);
            }
            Some("medium") => b.time_precision = Some(TimePrecision::Second),
            Some(_) => b.time_precision = Some(TimePrecision::Minute),
            None => {}
        }
    } else {
        let weekday = pick(&o, "weekday", &["narrow", "short", "long"])?;
        let era = pick(&o, "era", &["narrow", "short", "long"])?;
        let year = pick(&o, "year", &["2-digit", "numeric"])?;
        let month = pick(
            &o,
            "month",
            &["2-digit", "numeric", "narrow", "short", "long"],
        )?;
        let day = pick(&o, "day", &["2-digit", "numeric"])?;
        let hour = pick(&o, "hour", &["2-digit", "numeric"])?;
        let minute = pick(&o, "minute", &["2-digit", "numeric"])?;
        widths = Widths {
            year: year.map(str::to_string),
            month: month.map(str::to_string),
            day: day.map(str::to_string),
            weekday: weekday.map(str::to_string),
            hour: hour.map(str::to_string),
            minute: minute.map(str::to_string),
        };
        let second = pick(&o, "second", &["2-digit", "numeric"])?;
        if era.is_some() || o.contains_key("dayPeriod") {
            return Err("RangeError: era and dayPeriod are not supported yet".into());
        }
        b.date_fields = match (
            year.is_some(),
            month.is_some(),
            day.is_some(),
            weekday.is_some(),
        ) {
            (true, true, true, true) => Some(DateFields::YMDE),
            (true, true, true, false) => Some(DateFields::YMD),
            (false, true, true, true) => Some(DateFields::MDE),
            (false, true, true, false) => Some(DateFields::MD),
            (false, false, true, true) => Some(DateFields::DE),
            (false, false, true, false) => Some(DateFields::D),
            (false, false, false, true) => Some(DateFields::E),
            (false, true, false, false) => Some(DateFields::M),
            (true, true, false, false) => Some(DateFields::YM),
            (true, false, false, false) => Some(DateFields::Y),
            (false, false, false, false) => None,
            _ => {
                return Err(
                    "RangeError: this combination of date options is not supported yet".into(),
                );
            }
        };
        // The width of the month (or weekday) picks the length.
        b.length = Some(match (month, weekday) {
            (Some("long"), _) | (None, Some("long")) => Length::Long,
            (Some("short" | "narrow"), _) | (None, Some("short" | "narrow")) => Length::Medium,
            (Some(_), _) => Length::Short,
            (None, _) => Length::Medium,
        });
        if year == Some("numeric") && b.length == Some(Length::Short) {
            b.year_style = Some(YearStyle::Full);
        }
        let subsecond = o
            .get("fractionalSecondDigits")
            .and_then(|v| v.parse::<u8>().ok());
        b.time_precision = match (
            hour.is_some(),
            minute.is_some(),
            second.is_some(),
            subsecond,
        ) {
            (false, false, false, None) => None,
            (true, false, false, None) => Some(TimePrecision::Hour),
            (true, true, false, None) => Some(TimePrecision::Minute),
            (true, true, true, None) => Some(TimePrecision::Second),
            (true, true, true, Some(n)) => Some(
                SubsecondDigits::try_from_int(n)
                    .map(TimePrecision::Subsecond)
                    .ok_or("RangeError: fractionalSecondDigits value is out of range.")?,
            ),
            _ => {
                return Err(
                    "RangeError: this combination of time options is not supported yet".into(),
                );
            }
        };
        b.zone_style = match pick(
            &o,
            "timeZoneName",
            &[
                "short",
                "long",
                "shortOffset",
                "longOffset",
                "shortGeneric",
                "longGeneric",
            ],
        )? {
            Some("short") => Some(ZoneStyle::SpecificShort),
            Some("long") => Some(ZoneStyle::SpecificLong),
            Some("shortOffset") => Some(ZoneStyle::LocalizedOffsetShort),
            Some("longOffset") => Some(ZoneStyle::LocalizedOffsetLong),
            Some("shortGeneric") => Some(ZoneStyle::GenericShort),
            Some("longGeneric") => Some(ZoneStyle::GenericLong),
            _ => None,
        };
    }
    let fieldset = b
        .build_composite()
        .map_err(|e| format!("RangeError: {e}"))?;
    let formatter =
        DateTimeFormatter::try_new(prefs, fieldset).map_err(|e| format!("RangeError: {e}"))?;
    let gregorian = !matches!(
        locale.id.language.as_str(),
        "th" | "fa" | "ps" | "ar" | "he"
    ) && !o.contains_key("calendar");
    let adjusted = if gregorian
        && (widths.year.is_some()
            || widths.month.is_some()
            || widths.day.is_some()
            || widths.weekday.is_some()
            || widths.hour.is_some())
    {
        adjust_for(&formatter, &locale, &widths, zone.as_ref())
    } else {
        None
    };
    let zone_name = match &zone {
        Some(z) => z.iana_name().unwrap_or("UTC").to_string(),
        None => super::date::current_zone()
            .iana_name()
            .unwrap_or("UTC")
            .to_string(),
    };
    Ok(DtFmt {
        formatter,
        zone,
        adjusted,
        locale_tag: locale.to_string(),
        zone_name,
    })
}

/// The nearest pattern with the requested widths, and the typed names to format it.
fn adjust_for(
    formatter: &DateTimeFormatter<CompositeFieldSet>,
    locale: &icu::locale::Locale,
    widths: &Widths,
    zone: Option<&TimeZone>,
) -> Option<Adjusted> {
    // A sample instant gives the pattern the formatter would use.
    let tz = zone.cloned().unwrap_or_else(super::date::current_zone);
    let (y, mo, d, h, mi, s, ms, _, zinfo) = super::date::icu_parts(&tz, 981_173_106_000.0);
    let input = ZonedDateTime {
        date: Date::try_new_iso(y, mo, d).ok()?,
        time: Time::try_new(h, mi, s, ms * 1_000_000).ok()?,
        zone: zinfo,
    };
    let base = formatter.format(&input).pattern().to_string();
    let tag = locale.to_string();
    let adjusted = adjust_pattern(&base, widths, super::intl_pad::padding(&tag));
    if adjusted == base {
        return None;
    }
    let pattern: DateTimePattern = adjusted.parse().ok()?;
    let names =
        FixedCalendarDateTimeNames::<Gregorian, CompositeFieldSet>::try_new(locale.into()).ok()?;
    Some(Adjusted {
        pattern,
        names: RefCell::new(names),
    })
}

fn raise_for(m: &str) {
    match m.strip_prefix("TypeError: ") {
        Some(t) => raise(2.0, t),
        None => raise(1.0, m.strip_prefix("RangeError: ").unwrap_or(m)),
    }
}

fn id_for(locales: &str, options: &str, kind: i64) -> Result<usize, String> {
    let key = (locales.to_string(), options.to_string(), kind);
    if let Some(id) = IDS.with(|m| m.borrow().get(&key).copied()) {
        return Ok(id);
    }
    let f = Rc::new(build(locales, options, kind)?);
    let id = FORMATS.with(|v| {
        let mut v = v.borrow_mut();
        v.push(f);
        v.len() - 1
    });
    IDS.with(|m| m.borrow_mut().insert(key, id));
    Ok(id)
}

fn format(f: &DtFmt, t: f64) -> Result<String, String> {
    let zone = f.zone.clone().unwrap_or_else(super::date::current_zone);
    let (y, mo, d, h, mi, s, ms, off_secs, zinfo) = super::date::icu_parts(&zone, t);
    let date = Date::try_new_iso(y, mo, d).map_err(|e| format!("RangeError: {e}"))?;
    let time = Time::try_new(h, mi, s, ms * 1_000_000).map_err(|e| format!("RangeError: {e}"))?;
    let _ = off_secs;
    let input = ZonedDateTime {
        date,
        time,
        zone: zinfo,
    };
    if let Some(adj) = &f.adjusted {
        let gregorian =
            Date::try_new_gregorian(y, mo, d).map_err(|e| format!("RangeError: {e}"))?;
        let input = ZonedDateTime {
            date: gregorian,
            time: input.time,
            zone: input.zone,
        };
        let mut names = adj.names.borrow_mut();
        if let Ok(formatter) = names.include_for_pattern(&adj.pattern) {
            let shown = formatter.format(&input);
            let text = match writeable::TryWriteable::try_write_to_string(&shown) {
                Ok(t) => t.into_owned(),
                Err((_, t)) => t.into_owned(),
            };
            return Ok(plain_spaces(&text));
        }
    }
    Ok(plain_spaces(&f.formatter.format(&input).to_string()))
}

/// The parts of a formatted date: `type \u{2} value` records joined by `\u{1}`.
fn format_parts(f: &DtFmt, t: f64) -> Result<String, String> {
    let zone = f.zone.clone().unwrap_or_else(super::date::current_zone);
    let (y, mo, d, h, mi, s, ms, _off, zinfo) = super::date::icu_parts(&zone, t);
    let date = Date::try_new_iso(y, mo, d).map_err(|e| format!("RangeError: {e}"))?;
    let time = Time::try_new(h, mi, s, ms * 1_000_000).map_err(|e| format!("RangeError: {e}"))?;
    let input = ZonedDateTime {
        date,
        time,
        zone: zinfo,
    };
    let mut sink = super::intl_parts::Collect::default();
    if let Some(adj) = &f.adjusted {
        let gregorian =
            Date::try_new_gregorian(y, mo, d).map_err(|e| format!("RangeError: {e}"))?;
        let input = ZonedDateTime {
            date: gregorian,
            time: input.time,
            zone: input.zone,
        };
        let mut names = adj.names.borrow_mut();
        if let Ok(formatter) = names.include_for_pattern(&adj.pattern) {
            let shown = formatter.format(&input);
            writeable::TryWriteable::try_write_to_parts(&shown, &mut sink)
                .map_err(|e| e.to_string())?
                .ok();
            let parts = sink.into_parts(plain_spaces);
            return Ok(super::intl_parts::encode(&parts));
        }
    }
    writeable::Writeable::write_to_parts(&f.formatter.format(&input), &mut sink)
        .map_err(|e| e.to_string())?;
    let parts = sink.into_parts(plain_spaces);
    Ok(super::intl_parts::encode(&parts))
}

/// `dateTimeFormat.resolvedOptions()` as `key=value;` pairs.
#[unsafe(no_mangle)]
pub extern "C" fn ts_rt_dtf_resolved_gc(id: f64) -> *mut Str {
    let f = FORMATS.with(|v| v.borrow()[id as usize].clone());
    str_from(&format!(
        "locale={};calendar=gregory;numberingSystem=latn;timeZone={};",
        f.locale_tag, f.zone_name
    ))
}

/// `dateTimeFormat.formatToParts(date)` as encoded records.
#[unsafe(no_mangle)]
pub extern "C" fn ts_rt_dtf_parts_gc(id: f64, t: f64) -> *mut Str {
    let f = FORMATS.with(|v| v.borrow()[id as usize].clone());
    match format_parts(&f, t) {
        Ok(s) => str_from(&s),
        Err(m) => {
            raise_for(&m);
            str_from("")
        }
    }
}

fn run(id: usize, t: f64) -> *mut Str {
    let f = FORMATS.with(|v| v.borrow()[id].clone());
    match format(&f, t) {
        Ok(s) => str_from(&s),
        Err(m) => {
            raise_for(&m);
            str_from("")
        }
    }
}

fn str_from(s: &str) -> *mut Str {
    // SAFETY: valid bytes.
    unsafe { ts_rt_str_new_gc(s.as_ptr(), s.len() as i64) }
}

/// `new Intl.DateTimeFormat(locales, options)`: an id for `ts_rt_dtf_format_gc`.
///
/// # Safety
/// Both must be live strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_dtf_new(locales: *const Str, options: *const Str) -> f64 {
    // SAFETY: by contract.
    let (l, o) = unsafe { ((*locales).to_string_lossy(), (*options).to_string_lossy()) };
    match id_for(&l, &o, 3) {
        Ok(id) => id as f64,
        Err(m) => {
            raise_for(&m);
            0.0
        }
    }
}

/// `dtf.format(date)`: a RangeError for an invalid date.
#[unsafe(no_mangle)]
pub extern "C" fn ts_rt_dtf_format_gc(id: f64, t: f64) -> *mut Str {
    if t.is_nan() {
        raise(1.0, "date value is not finite in DateTimeFormat format()");
        return str_from("");
    }
    run(id as usize, t)
}

/// `date.toLocaleString/DateString/TimeString(locales, options)`; `kind` 0, 1 or 2 as in
/// `with_defaults`. An invalid date gives "Invalid Date".
///
/// # Safety
/// Both must be live strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_date_locale_string_gc(
    t: f64,
    locales: *const Str,
    options: *const Str,
    kind: f64,
) -> *mut Str {
    if t.is_nan() {
        return str_from("Invalid Date");
    }
    // SAFETY: by contract.
    let (l, o) = unsafe { ((*locales).to_string_lossy(), (*options).to_string_lossy()) };
    match id_for(&l, &o, kind as i64) {
        Ok(id) => run(id, t),
        Err(m) => {
            raise_for(&m);
            str_from("")
        }
    }
}

/// Engines print a plain space where CLDR 42+ has a narrow no-break space ("6:30 AM", "2026 г."),
/// which browsers undid for compatibility.
fn plain_spaces(s: &str) -> String {
    s.replace('\u{202f}', " ")
}
