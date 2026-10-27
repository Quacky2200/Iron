// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

//! Time zone queries for `Temporal.ZonedDateTime`, on jiff: a zone name checked and made canonical,
//! the UTC offset at an instant, and a local date and time turned into an instant (with the four
//! `disambiguation` choices).

use super::*;
use jiff::{Timestamp, civil, tz::TimeZone};

fn text(s: *const Str) -> String {
    // SAFETY: callers pass live strings.
    unsafe { (*s).to_string_lossy().to_string() }
}

fn str_from(s: &str) -> *mut Str {
    // SAFETY: valid bytes.
    unsafe { ts_rt_str_new_gc(s.as_ptr(), s.len() as i64) }
}

fn zone(name: &str) -> Option<TimeZone> {
    // An offset such as `+01:00` or `-0530` is a zone too.
    if let Some(sign) = name.chars().next().filter(|c| *c == '+' || *c == '-') {
        let digits: String = name[1..].chars().filter(|c| *c != ':').collect();
        if digits.len() == 2 || digits.len() == 4 {
            let h: i32 = digits[..2].parse().ok()?;
            let m: i32 = digits.get(2..4).map_or(Some(0), |m| m.parse().ok())?;
            let secs = (h * 3600 + m * 60) * if sign == '-' { -1 } else { 1 };
            return jiff::tz::Offset::from_seconds(secs)
                .ok()
                .map(TimeZone::fixed);
        }
        return None;
    }
    TimeZone::get(name).ok()
}

/// The canonical name of a zone (`europe/paris` is `Europe/Paris`), or a `RangeError`.
///
/// # Safety
/// `name` must be a live string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_tz_check_gc(name: *const Str) -> *mut Str {
    let n = text(name);
    match zone(&n) {
        Some(z) => str_from(z.iana_name().unwrap_or(&n)),
        None => {
            raise(1.0, &format!("invalid time zone: {n}"));
            str_from("")
        }
    }
}

/// The UTC offset in seconds of `name` at `epoch_secs`.
///
/// # Safety
/// `name` must be a live string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_tz_offset(name: *const Str, epoch_secs: f64) -> f64 {
    let Some(z) = zone(&text(name)) else {
        raise(1.0, "invalid time zone");
        return 0.0;
    };
    match Timestamp::from_second(epoch_secs as i64) {
        Ok(t) => f64::from(z.to_offset(t).seconds()),
        Err(_) => {
            raise(1.0, "epoch out of range");
            0.0
        }
    }
}

/// The instant (seconds since the epoch) of a local date (days since 1970-01-01) and time of day
/// (seconds) in `name`. `mode`: 0 compatible, 1 earlier, 2 later, 3 reject.
///
/// # Safety
/// `name` must be a live string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_tz_local(name: *const Str, days: f64, secs: f64, mode: f64) -> f64 {
    let Some(z) = zone(&text(name)) else {
        raise(1.0, "invalid time zone");
        return 0.0;
    };
    let total = days as i64 * 86_400 + secs as i64;
    let Ok(ts) = Timestamp::from_second(total) else {
        raise(1.0, "date out of range");
        return 0.0;
    };
    let dt: civil::DateTime = TimeZone::UTC.to_datetime(ts);
    let amb = z.to_ambiguous_zoned(dt);
    let r = match mode as u8 {
        0 => amb.compatible(),
        1 => amb.earlier(),
        2 => amb.later(),
        _ => amb.unambiguous(),
    };
    match r {
        Ok(zd) => zd.timestamp().as_second() as f64,
        Err(_) => {
            raise(
                1.0,
                "the local time is ambiguous or does not exist in this time zone",
            );
            0.0
        }
    }
}

/// The system time zone's name.
#[unsafe(no_mangle)]
pub extern "C" fn ts_rt_tz_system_gc() -> *mut Str {
    let z = super::date::current_zone();
    str_from(z.iana_name().unwrap_or("UTC"))
}
