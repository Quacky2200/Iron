// SPDX-License-Identifier: Apache-2.0

//! `Intl`, step I4: `Intl.PluralRules`, `Intl.ListFormat` and `Intl.RelativeTimeFormat` on ICU4X.
//! Options arrive as the compiler's `key=value;` string. Errors are `RangeError`s with
//! JavaScriptCore's text.

use super::*;
use fixed_decimal::{Decimal, FloatPrecision};
use icu::experimental::relativetime::options::Numeric;
use icu::experimental::relativetime::{RelativeTimeFormatter, RelativeTimeFormatterOptions};
use icu::list::ListFormatter;
use icu::list::options::{ListFormatterOptions, ListLength};
use icu::locale::Locale;
use icu::plurals::{PluralCategory, PluralRules};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

fn opts(options: &str) -> HashMap<String, String> {
    options
        .split(';')
        .filter(|s| !s.is_empty())
        .filter_map(|kv| kv.split_once('='))
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

fn text(s: *const Str) -> String {
    // SAFETY: callers pass live strings.
    unsafe { (*s).to_string_lossy().to_string() }
}

fn str_from(s: &str) -> *mut Str {
    // SAFETY: valid bytes.
    unsafe { ts_rt_str_new_gc(s.as_ptr(), s.len() as i64) }
}

fn oneof<'a>(
    o: &'a HashMap<String, String>,
    key: &str,
    allowed: &[&str],
    either: bool,
) -> Result<Option<&'a str>, String> {
    match o.get(key) {
        None => Ok(None),
        Some(v) if allowed.contains(&v.as_str()) => Ok(Some(v.as_str())),
        Some(_) => {
            let q: Vec<String> = allowed.iter().map(|a| format!("\"{a}\"")).collect();
            let list = match q.as_slice() {
                [a, b] => format!("{a} or {b}"),
                [rest @ .., last] => format!("{}, or {last}", rest.join(", ")),
                [] => String::new(),
            };
            Err(format!(
                "{key} must be {}{list}",
                if either { "either " } else { "" }
            ))
        }
    }
}

// ---- PluralRules ----

struct Plural {
    rules: PluralRules,
    min_fraction: Option<i16>,
    max_fraction: Option<i16>,
}

thread_local! {
    static PLURALS: RefCell<Vec<Rc<Plural>>> = const { RefCell::new(Vec::new()) };
    static LISTS: RefCell<Vec<Rc<ListFormatter>>> = const { RefCell::new(Vec::new()) };
    static RTFS: RefCell<Vec<Rc<Rtf>>> = const { RefCell::new(Vec::new()) };
}

fn fraction(o: &HashMap<String, String>, key: &str) -> Result<Option<i16>, String> {
    match o.get(key) {
        None => Ok(None),
        Some(v) => match v.parse::<f64>() {
            Ok(n) if (0.0..=100.0).contains(&n) => Ok(Some(n.floor() as i16)),
            _ => Err(format!("{key} is out of range")),
        },
    }
}

fn build_plural(locales: &str, options: &str) -> Result<Plural, String> {
    let locale: Locale = super::intl::resolve_locale(locales)?;
    let o = opts(options);
    let kind = oneof(&o, "type", &["cardinal", "ordinal"], false)?;
    let prefs = (&locale).into();
    let rules = match kind {
        Some("ordinal") => PluralRules::try_new_ordinal(prefs),
        _ => PluralRules::try_new_cardinal(prefs),
    }
    .map_err(|e| e.to_string())?;
    Ok(Plural {
        rules,
        min_fraction: fraction(&o, "minimumFractionDigits")?,
        max_fraction: fraction(&o, "maximumFractionDigits")?,
    })
}

/// `new Intl.PluralRules(locales, options)`.
///
/// # Safety
/// Both must be live strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_plural_new(locales: *const Str, options: *const Str) -> f64 {
    match build_plural(&text(locales), &text(options)) {
        Ok(p) => PLURALS.with(|v| {
            let mut v = v.borrow_mut();
            v.push(Rc::new(p));
            (v.len() - 1) as f64
        }),
        Err(m) => {
            raise(1.0, &m);
            0.0
        }
    }
}

/// `rules.select(n)`: `zero`, `one`, `two`, `few`, `many` or `other`.
#[unsafe(no_mangle)]
pub extern "C" fn ts_rt_plural_select_gc(id: f64, n: f64) -> *mut Str {
    let p = PLURALS.with(|v| v.borrow()[id as usize].clone());
    if !n.is_finite() {
        return str_from("other");
    }
    let Ok(mut d) = Decimal::try_from_f64(n.abs(), FloatPrecision::RoundTrip) else {
        return str_from("other");
    };
    if let Some(max) = p.max_fraction {
        d.round(-max);
        d.trim_end();
    }
    if let Some(min) = p.min_fraction {
        d.pad_end(-min);
    }
    let name = match p.rules.category_for(&d) {
        PluralCategory::Zero => "zero",
        PluralCategory::One => "one",
        PluralCategory::Two => "two",
        PluralCategory::Few => "few",
        PluralCategory::Many => "many",
        PluralCategory::Other => "other",
    };
    str_from(name)
}

// ---- ListFormat ----

fn build_list(locales: &str, options: &str) -> Result<ListFormatter, String> {
    let locale: Locale = super::intl::resolve_locale(locales)?;
    let o = opts(options);
    let style = oneof(&o, "style", &["long", "short", "narrow"], true)?;
    let kind = oneof(&o, "type", &["conjunction", "disjunction", "unit"], true)?;
    let mut lo = ListFormatterOptions::default();
    lo.length = Some(match style {
        Some("short") => ListLength::Short,
        Some("narrow") => ListLength::Narrow,
        _ => ListLength::Wide,
    });
    let prefs = (&locale).into();
    match kind {
        Some("disjunction") => ListFormatter::try_new_or(prefs, lo),
        Some("unit") => ListFormatter::try_new_unit(prefs, lo),
        _ => ListFormatter::try_new_and(prefs, lo),
    }
    .map_err(|e| e.to_string())
}

/// `new Intl.ListFormat(locales, options)`.
///
/// # Safety
/// Both must be live strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_list_new(locales: *const Str, options: *const Str) -> f64 {
    match build_list(&text(locales), &text(options)) {
        Ok(f) => LISTS.with(|v| {
            let mut v = v.borrow_mut();
            v.push(Rc::new(f));
            (v.len() - 1) as f64
        }),
        Err(m) => {
            raise(1.0, &m);
            0.0
        }
    }
}

/// `list.format(items)`.
///
/// # Safety
/// `items` must be an array of live strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_list_format_gc(id: f64, items: *const Array) -> *mut Str {
    let f = LISTS.with(|v| v.borrow()[id as usize].clone());
    // SAFETY: slots of a pointer array hold string pointers.
    let a = unsafe { &*items };
    let parts: Vec<String> = (0..a.len())
        .map(|i| text(unsafe { a.get::<*const Str>(i) }))
        .collect();
    str_from(&f.format_to_string(parts.iter().map(String::as_str)))
}

// ---- RelativeTimeFormat ----

struct Rtf {
    locale: Locale,
    length: &'static str,
    numeric: Numeric,
    by_unit: RefCell<HashMap<&'static str, Rc<RelativeTimeFormatter>>>,
}

fn build_rtf(locales: &str, options: &str) -> Result<Rtf, String> {
    let locale: Locale = super::intl::resolve_locale(locales)?;
    let o = opts(options);
    let length = match oneof(&o, "style", &["long", "short", "narrow"], true)? {
        Some("short") => "short",
        Some("narrow") => "narrow",
        _ => "long",
    };
    let numeric = match oneof(&o, "numeric", &["always", "auto"], true)? {
        Some("auto") => Numeric::Auto,
        _ => Numeric::Always,
    };
    Ok(Rtf {
        locale,
        length,
        numeric,
        by_unit: RefCell::new(HashMap::new()),
    })
}

/// `new Intl.RelativeTimeFormat(locales, options)`.
///
/// # Safety
/// Both must be live strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_rtf_new(locales: *const Str, options: *const Str) -> f64 {
    match build_rtf(&text(locales), &text(options)) {
        Ok(f) => RTFS.with(|v| {
            let mut v = v.borrow_mut();
            v.push(Rc::new(f));
            (v.len() - 1) as f64
        }),
        Err(m) => {
            raise(1.0, &m);
            0.0
        }
    }
}

fn unit_name(u: &str) -> Option<&'static str> {
    Some(match u {
        "second" | "seconds" => "second",
        "minute" | "minutes" => "minute",
        "hour" | "hours" => "hour",
        "day" | "days" => "day",
        "week" | "weeks" => "week",
        "month" | "months" => "month",
        "quarter" | "quarters" => "quarter",
        "year" | "years" => "year",
        _ => return None,
    })
}

fn rtf_for(r: &Rtf, unit: &'static str) -> Result<Rc<RelativeTimeFormatter>, String> {
    if let Some(f) = r.by_unit.borrow().get(unit) {
        return Ok(f.clone());
    }
    let mut o = RelativeTimeFormatterOptions::default();
    o.numeric = r.numeric;
    let p = (&r.locale).into();
    macro_rules! pick {
        ($($len:literal, $unit:literal => $ctor:ident;)*) => {
            match (r.length, unit) {
                $(($len, $unit) => RelativeTimeFormatter::$ctor(p, o),)*
                _ => unreachable!("every length and unit is listed"),
            }
        };
    }
    let f = pick! {
        "long", "second" => try_new_long_second; "long", "minute" => try_new_long_minute;
        "long", "hour" => try_new_long_hour; "long", "day" => try_new_long_day;
        "long", "week" => try_new_long_week; "long", "month" => try_new_long_month;
        "long", "quarter" => try_new_long_quarter; "long", "year" => try_new_long_year;
        "short", "second" => try_new_short_second; "short", "minute" => try_new_short_minute;
        "short", "hour" => try_new_short_hour; "short", "day" => try_new_short_day;
        "short", "week" => try_new_short_week; "short", "month" => try_new_short_month;
        "short", "quarter" => try_new_short_quarter; "short", "year" => try_new_short_year;
        "narrow", "second" => try_new_narrow_second; "narrow", "minute" => try_new_narrow_minute;
        "narrow", "hour" => try_new_narrow_hour; "narrow", "day" => try_new_narrow_day;
        "narrow", "week" => try_new_narrow_week; "narrow", "month" => try_new_narrow_month;
        "narrow", "quarter" => try_new_narrow_quarter; "narrow", "year" => try_new_narrow_year;
    }
    .map_err(|e| e.to_string())?;
    let f = Rc::new(f);
    r.by_unit.borrow_mut().insert(unit, f.clone());
    Ok(f)
}

/// `rtf.format(value, unit)`.
///
/// # Safety
/// `unit` must be a live string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_rtf_format_gc(id: f64, value: f64, unit: *const Str) -> *mut Str {
    let r = RTFS.with(|v| v.borrow()[id as usize].clone());
    if !value.is_finite() {
        raise(1.0, "number argument must be finite");
        return str_from("");
    }
    let Some(unit) = unit_name(&text(unit)) else {
        raise(1.0, "unit argument is not a recognized unit type");
        return str_from("");
    };
    let d = Decimal::try_from_f64(value, FloatPrecision::RoundTrip);
    let f = rtf_for(&r, unit);
    match (d, f) {
        (Ok(d), Ok(f)) => str_from(&f.format(d).to_string()),
        (_, Err(m)) => {
            raise(1.0, &m);
            str_from("")
        }
        (Err(e), _) => {
            raise(1.0, &format!("{e:?}"));
            str_from("")
        }
    }
}
