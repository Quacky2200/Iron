// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

//! `Intl`, step I1: `normalize`, `localeCompare`, `Intl.Collator` and locale-aware case mapping, on
//! ICU4X (compiled data for every locale). The locales a program may use come from the host with
//! `ts_rt_intl_set_locales` (a comma-separated list; the first is the default, `en-US` when never
//! set). Errors are `RangeError`s with JavaScriptCore's text.

use super::*;
use icu::casemap::CaseMapper;
use icu::collator::options::{AlternateHandling, CaseLevel, CollatorOptions, Strength};
use icu::collator::preferences::{CollationCaseFirst, CollationNumericOrdering};
use icu::collator::{CollatorBorrowed, CollatorPreferences};
use icu::locale::Locale;
use icu::normalizer::{ComposingNormalizerBorrowed, DecomposingNormalizerBorrowed};
use std::cell::RefCell;
use std::collections::HashMap;

thread_local! {
    static DEFAULTS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    static COLLATORS: RefCell<Vec<CollatorBorrowed<'static>>> = const { RefCell::new(Vec::new()) };
    static COLLATOR_IDS: RefCell<HashMap<(String, String), usize>> = RefCell::new(HashMap::new());
}

/// Sets the locales the program may use (`"en-US,de,ja"`); the first is the default.
///
/// # Safety
/// `list` must point to `len` readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_intl_set_locales(list: *const u8, len: i64) {
    // SAFETY: by contract.
    let bytes = unsafe { std::slice::from_raw_parts(list, len as usize) };
    let list = String::from_utf8_lossy(bytes);
    DEFAULTS.with(|d| {
        *d.borrow_mut() = list
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    });
}

fn default_locale() -> Locale {
    let first = DEFAULTS.with(|d| d.borrow().first().cloned());
    first
        .and_then(|t| t.parse().ok())
        .unwrap_or_else(|| "en-US".parse().expect("a valid tag"))
}

/// The locale a list of tags ("de", "xx,de") selects: the first tag, or the default for none.
pub(crate) fn resolve_locale(spec: &str) -> Result<Locale, String> {
    let mut first = None;
    for tag in spec.split(',').filter(|t| !t.is_empty()) {
        let loc: Locale = tag
            .parse()
            .map_err(|_| format!("invalid language tag: {tag}"))?;
        first.get_or_insert(loc);
    }
    Ok(first.unwrap_or_else(default_locale))
}

fn text(s: *const Str) -> String {
    // SAFETY: callers pass live strings.
    unsafe { (*s).to_string_lossy().to_string() }
}

fn new_str(s: &str) -> *mut Str {
    // SAFETY: valid bytes.
    unsafe { ts_rt_str_new_gc(s.as_ptr(), s.len() as i64) }
}

/// Maps the well-formed parts of a string and keeps lone surrogates as they are.
fn map_segments(s: &Str, f: impl Fn(&str) -> String) -> Vec<u8> {
    if s.is_well_formed() {
        return f(&s.to_string_lossy()).into_bytes();
    }
    let units = s.to_utf16();
    let mut out: Vec<u16> = Vec::with_capacity(units.len());
    let mut run = String::new();
    let flush = |run: &mut String, out: &mut Vec<u16>| {
        if !run.is_empty() {
            out.extend(f(run).encode_utf16());
            run.clear();
        }
    };
    for r in char::decode_utf16(units.iter().copied()) {
        match r {
            Ok(c) => run.push(c),
            Err(e) => {
                flush(&mut run, &mut out);
                out.push(e.unpaired_surrogate());
            }
        }
    }
    flush(&mut run, &mut out);
    wtf8_from_utf16(&out)
}

fn new_bytes(b: Vec<u8>) -> *mut Str {
    // SAFETY: valid WTF-8.
    unsafe { ts_rt_str_new_gc(b.as_ptr(), b.len() as i64) }
}

/// `s.normalize(form)`: `form` is `NFC`, `NFD`, `NFKC` or `NFKD`.
///
/// # Safety
/// Both must be live strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_str_normalize_gc(s: *const Str, form: *const Str) -> *mut Str {
    // SAFETY: by contract.
    let (s, form) = unsafe { (&*s, text(form)) };
    let run: fn(&str) -> String = match form.as_str() {
        "NFC" => |t| {
            ComposingNormalizerBorrowed::new_nfc()
                .normalize(t)
                .into_owned()
        },
        "NFD" => |t| {
            DecomposingNormalizerBorrowed::new_nfd()
                .normalize(t)
                .into_owned()
        },
        "NFKC" => |t| {
            ComposingNormalizerBorrowed::new_nfkc()
                .normalize(t)
                .into_owned()
        },
        "NFKD" => |t| {
            DecomposingNormalizerBorrowed::new_nfkd()
                .normalize(t)
                .into_owned()
        },
        _ => {
            raise(1.0, "argument does not match any normalization form");
            return new_str("");
        }
    };
    new_bytes(map_segments(s, run))
}

/// `toLocaleUpperCase(locales)` / `toLocaleLowerCase(locales)`.
///
/// # Safety
/// Both must be live strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_str_locale_case_gc(
    s: *const Str,
    locales: *const Str,
    upper: bool,
) -> *mut Str {
    // SAFETY: by contract.
    let s = unsafe { &*s };
    let loc = match resolve_locale(&text(locales)) {
        Ok(l) => l,
        Err(m) => {
            raise(1.0, &m);
            return new_str("");
        }
    };
    let id = loc.id;
    let cm = CaseMapper::new();
    new_bytes(map_segments(s, |t| {
        if upper {
            cm.uppercase_to_string(t, &id).into_owned()
        } else {
            cm.lowercase_to_string(t, &id).into_owned()
        }
    }))
}

fn make_collator(locales: &str, options: &str) -> Result<CollatorBorrowed<'static>, String> {
    let loc = resolve_locale(locales)?;
    let mut prefs = CollatorPreferences::from(&loc);
    let mut opts = CollatorOptions::default();
    for kv in options.split(';').filter(|s| !s.is_empty()) {
        let (k, v) = kv.split_once('=').unwrap_or((kv, ""));
        match (k, v) {
            ("usage", "sort" | "search") => {}
            ("usage", _) => return Err("usage must be either \"sort\" or \"search\"".into()),
            ("sensitivity", "base") => opts.strength = Some(Strength::Primary),
            ("sensitivity", "accent") => opts.strength = Some(Strength::Secondary),
            ("sensitivity", "case") => {
                opts.strength = Some(Strength::Primary);
                opts.case_level = Some(CaseLevel::On);
            }
            ("sensitivity", "variant") => opts.strength = Some(Strength::Tertiary),
            ("sensitivity", _) => {
                return Err(
                    "sensitivity must be either \"base\", \"accent\", \"case\", or \"variant\""
                        .into(),
                );
            }
            ("numeric", "true") => prefs.numeric_ordering = Some(CollationNumericOrdering::True),
            ("numeric", "false") => prefs.numeric_ordering = Some(CollationNumericOrdering::False),
            ("ignorePunctuation", "true") => {
                opts.alternate_handling = Some(AlternateHandling::Shifted)
            }
            ("ignorePunctuation", "false") => {
                opts.alternate_handling = Some(AlternateHandling::NonIgnorable)
            }
            ("caseFirst", "upper") => prefs.case_first = Some(CollationCaseFirst::Upper),
            ("caseFirst", "lower") => prefs.case_first = Some(CollationCaseFirst::Lower),
            ("caseFirst", "false") => prefs.case_first = Some(CollationCaseFirst::False),
            ("caseFirst", _) => {
                return Err("caseFirst must be either \"upper\", \"lower\", or \"false\"".into());
            }
            _ => {}
        }
    }
    CollatorBorrowed::try_new(prefs, opts).map_err(|e| format!("collation data unavailable: {e}"))
}

fn collator_id(locales: &str, options: &str) -> Result<usize, String> {
    let key = (locales.to_string(), options.to_string());
    if let Some(id) = COLLATOR_IDS.with(|m| m.borrow().get(&key).copied()) {
        return Ok(id);
    }
    let c = make_collator(locales, options)?;
    let id = COLLATORS.with(|v| {
        let mut v = v.borrow_mut();
        v.push(c);
        v.len() - 1
    });
    COLLATOR_IDS.with(|m| m.borrow_mut().insert(key, id));
    Ok(id)
}

fn order(a: &str, b: &str, id: usize) -> f64 {
    COLLATORS.with(|v| match v.borrow()[id].compare(a, b) {
        std::cmp::Ordering::Less => -1.0,
        std::cmp::Ordering::Equal => 0.0,
        std::cmp::Ordering::Greater => 1.0,
    })
}

/// `new Intl.Collator(locales, options)`: an id for `ts_rt_collator_compare`. `options` is the
/// compiler's `key=value;key=value` form of the options object.
///
/// # Safety
/// Both must be live strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_collator_new(locales: *const Str, options: *const Str) -> f64 {
    match collator_id(&text(locales), &text(options)) {
        Ok(id) => id as f64,
        Err(m) => {
            raise(1.0, &m);
            0.0
        }
    }
}

/// `collator.compare(a, b)`: -1, 0 or 1.
///
/// # Safety
/// Both must be live strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_collator_compare(id: f64, a: *const Str, b: *const Str) -> f64 {
    order(&text(a), &text(b), id as usize)
}

/// `a.localeCompare(b, locales, options)`.
///
/// # Safety
/// All must be live strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_str_locale_compare(
    a: *const Str,
    b: *const Str,
    locales: *const Str,
    options: *const Str,
) -> f64 {
    match collator_id(&text(locales), &text(options)) {
        Ok(id) => order(&text(a), &text(b), id),
        Err(m) => {
            raise(1.0, &m);
            0.0
        }
    }
}
