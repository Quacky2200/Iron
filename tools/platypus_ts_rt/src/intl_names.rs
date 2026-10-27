// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

//! `Intl.DisplayNames` on ICU4X (experimental display names): the names of regions, scripts and
//! languages in a locale. Currencies, calendars and date-time fields are not built.

use super::*;
use icu::experimental::displaynames::multi::{
    LocaleDisplayNamesFormatter, RegionDisplayNames, ScriptDisplayNames,
};
use icu::experimental::displaynames::{DisplayNamesOptions, LanguageDisplay, Style};
use icu::locale::Locale;
use icu::locale::subtags::{Region, Script};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

enum Names {
    Region(RegionDisplayNames),
    Script(ScriptDisplayNames),
    Language(Box<LocaleDisplayNamesFormatter>),
}

thread_local! {
    static NAMES: RefCell<Vec<Rc<Names>>> = const { RefCell::new(Vec::new()) };
}

fn text(s: *const Str) -> String {
    // SAFETY: callers pass live strings.
    unsafe { (*s).to_string_lossy().to_string() }
}

fn str_from(s: &str) -> *mut Str {
    // SAFETY: valid bytes.
    unsafe { ts_rt_str_new_gc(s.as_ptr(), s.len() as i64) }
}

/// `new Intl.DisplayNames(locales, { type, style, languageDisplay })`: an id. `type` is required.
///
/// # Safety
/// Both arguments must be live strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_display_names_new(locales: *const Str, options: *const Str) -> f64 {
    let spec = text(options);
    let o: HashMap<&str, &str> = spec
        .split(';')
        .filter_map(|kv| kv.split_once('='))
        .collect();
    let locales = text(locales);
    let first = locales
        .split(',')
        .find(|l| !l.is_empty())
        .unwrap_or("en-US");
    let Ok(locale) = Locale::try_from_str(first) else {
        raise(1.0, "Incorrect locale information provided");
        return 0.0;
    };
    let mut options = DisplayNamesOptions::default();
    options.style = match o.get("style").copied() {
        None | Some("long") => Some(Style::Long),
        Some("short") => Some(Style::Short),
        Some("narrow") => Some(Style::Narrow),
        Some(_) => {
            raise(
                1.0,
                "style must be either \"narrow\", \"short\", or \"long\"",
            );
            return 0.0;
        }
    };
    options.language_display = match o.get("languageDisplay").copied() {
        None | Some("dialect") => LanguageDisplay::Dialect,
        Some("standard") => LanguageDisplay::Standard,
        Some(_) => {
            raise(
                1.0,
                "languageDisplay must be either \"dialect\" or \"standard\"",
            );
            return 0.0;
        }
    };
    let prefs = (&locale).into();
    let made = match o.get("type").copied() {
        Some("region") => RegionDisplayNames::try_new(prefs, options).map(Names::Region),
        Some("script") => ScriptDisplayNames::try_new(prefs, options).map(Names::Script),
        Some("language") => LocaleDisplayNamesFormatter::try_new(prefs, options)
            .map(|f| Names::Language(Box::new(f))),
        None => {
            raise(2.0, "type is required");
            return 0.0;
        }
        Some("currency" | "calendar" | "dateTimeField") => {
            raise(
                1.0,
                "this DisplayNames type is not built (region, script and language are)",
            );
            return 0.0;
        }
        Some(_) => {
            raise(
                1.0,
                "type must be either \"language\", \"region\", \"script\", \"currency\", \"calendar\", or \"dateTimeField\"",
            );
            return 0.0;
        }
    };
    match made {
        Ok(n) => NAMES.with(|t| {
            let mut t = t.borrow_mut();
            t.push(Rc::new(n));
            (t.len() - 1) as f64
        }),
        Err(_) => {
            raise(1.0, "no display names for this locale");
            0.0
        }
    }
}

/// `displayNames.of(code)`.
///
/// # Safety
/// `code` must be a live string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_display_names_of_gc(id: f64, code: *const Str) -> *mut Str {
    let Some(names) = NAMES.with(|t| t.borrow().get(id as usize).cloned()) else {
        return str_from("");
    };
    let code = text(code);
    let out = match &*names {
        Names::Region(n) => match code.parse::<Region>() {
            Ok(r) => n.of(r).map(str::to_string).unwrap_or(code),
            Err(_) => {
                raise(1.0, "invalid region code");
                return str_from("");
            }
        },
        Names::Script(n) => match code.parse::<Script>() {
            Ok(s) => n.of(s).map(str::to_string).unwrap_or(code),
            Err(_) => {
                raise(1.0, "invalid script code");
                return str_from("");
            }
        },
        Names::Language(n) => match Locale::try_from_str(&code) {
            Ok(l) => n.of(&l).to_string(),
            Err(_) => {
                raise(1.0, "invalid language code");
                return str_from("");
            }
        },
    };
    str_from(&out)
}
