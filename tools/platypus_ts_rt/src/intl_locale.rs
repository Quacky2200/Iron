// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

//! `Intl.Locale` on ICU4X: parsing and canonicalising a tag, its parts, `maximize` and `minimize`.
//! A locale is kept as its canonical tag string; every question parses it again.

use super::*;
use icu::locale::extensions::unicode::{Key, Value};
use icu::locale::{Locale, LocaleExpander};
use std::collections::HashMap;

fn text(s: *const Str) -> String {
    // SAFETY: callers pass live strings.
    unsafe { (*s).to_string_lossy().to_string() }
}

fn str_from(s: &str) -> *mut Str {
    // SAFETY: valid bytes.
    unsafe { ts_rt_str_new_gc(s.as_ptr(), s.len() as i64) }
}

fn bad() {
    raise(1.0, "Incorrect locale information provided");
}

/// Option name to the Unicode extension key it sets.
const KEYWORDS: [(&str, &str); 6] = [
    ("calendar", "ca"),
    ("collation", "co"),
    ("hourCycle", "hc"),
    ("caseFirst", "kf"),
    ("numeric", "kn"),
    ("numberingSystem", "nu"),
];

/// `new Intl.Locale(tag, options)`: the canonical tag, or a `RangeError`.
///
/// # Safety
/// Both arguments must be live strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_locale_new_gc(tag: *const Str, options: *const Str) -> *mut Str {
    let tag = text(tag);
    let spec = text(options);
    let Ok(mut loc) = Locale::try_from_str(&tag) else {
        bad();
        return str_from("");
    };
    let o: HashMap<&str, &str> = spec
        .split(';')
        .filter_map(|kv| kv.split_once('='))
        .collect();
    if let Some(l) = o.get("language") {
        match l.parse() {
            Ok(v) => loc.id.language = v,
            Err(_) => {
                bad();
                return str_from("");
            }
        }
    }
    if let Some(s) = o.get("script") {
        match s.parse() {
            Ok(v) => loc.id.script = Some(v),
            Err(_) => {
                bad();
                return str_from("");
            }
        }
    }
    if let Some(r) = o.get("region") {
        match r.parse() {
            Ok(v) => loc.id.region = Some(v),
            Err(_) => {
                bad();
                return str_from("");
            }
        }
    }
    for (name, key) in KEYWORDS {
        let Some(v) = o.get(name) else { continue };
        let v = if name == "numeric" {
            // `kn` is true when present without a value.
            if *v == "true" { "" } else { "false" }
        } else {
            v
        };
        let (Ok(k), Ok(val)) = (Key::try_from_str(key), Value::try_from_str(v)) else {
            bad();
            return str_from("");
        };
        loc.extensions.unicode.keywords.set(k, val);
    }
    str_from(&loc.to_string())
}

/// A question about a canonical tag: 0 the tag, 1 language, 2 script, 3 region, 4 base name,
/// 5 maximized tag, 6 minimized tag, 7 calendar, 8 numbering system, 9 hour cycle, 10 collation,
/// 11 case first, 12 numeric ("1" when true). The empty string means none.
///
/// # Safety
/// `tag` must be a live string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_locale_op_gc(tag: *const Str, op: f64) -> *mut Str {
    let Ok(mut loc) = Locale::try_from_str(&text(tag)) else {
        bad();
        return str_from("");
    };
    let kw = |loc: &Locale, key: &str| -> String {
        Key::try_from_str(key)
            .ok()
            .and_then(|k| {
                loc.extensions
                    .unicode
                    .keywords
                    .get(&k)
                    .map(|v| v.to_string())
            })
            .unwrap_or_default()
    };
    let out = match op as u8 {
        0 => loc.to_string(),
        1 => loc.id.language.to_string(),
        2 => loc.id.script.map(|s| s.to_string()).unwrap_or_default(),
        3 => loc.id.region.map(|s| s.to_string()).unwrap_or_default(),
        4 => loc.id.to_string(),
        5 | 6 => {
            let expander = LocaleExpander::new_common();
            if op as u8 == 5 {
                expander.maximize(&mut loc.id);
                // ICU4C's likely subtags give "und" itself `en-Latn-US`; ICU4X leaves it.
                if loc.id.language.is_unknown()
                    && loc.id.script.is_none()
                    && loc.id.region.is_none()
                    && let Ok(id) = "en-Latn-US".parse()
                {
                    loc.id = id;
                }
            } else {
                expander.minimize(&mut loc.id);
            }
            loc.to_string()
        }
        7 => kw(&loc, "ca"),
        8 => kw(&loc, "nu"),
        9 => kw(&loc, "hc"),
        10 => kw(&loc, "co"),
        11 => kw(&loc, "kf"),
        _ => {
            let has = Key::try_from_str("kn")
                .ok()
                .and_then(|k| {
                    loc.extensions
                        .unicode
                        .keywords
                        .get(&k)
                        .map(|v| v.to_string())
                })
                .is_some();
            let v = kw(&loc, "kn");
            if has && (v.is_empty() || v == "true") {
                "1".to_string()
            } else {
                String::new()
            }
        }
    };
    str_from(&out)
}
