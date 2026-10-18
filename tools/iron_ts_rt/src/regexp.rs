//! `RegExp`. The engine is the `regress` crate (ECMAScript regular expressions: lookahead and
//! lookbehind, backreferences, named groups, the `i m s u v` flags, Unicode property escapes).
//!
//! A compiled regex is kept in a per-thread registry and named by a number (the `id` field of the
//! program's `RegExp` object, which is an ordinary class compiled from the prelude); equal
//! (pattern, flags) pairs share one entry. Positions are UTF-16 code units, as in JavaScript:
//! ASCII strings are matched as bytes (unit = byte), others as UTF-16 text (`u`/`v` regexes match
//! surrogate pairs as one character, the others see two units).
//!
//! The functions return the pieces of a match; the compiler builds the match object and keeps
//! `lastIndex` in the object's own field.

use super::string::*;
use super::{Array, ts_rt_arr_new_gc, ts_rt_arr_push};
use regress::{Flags, Regex};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

struct Entry {
    re: Regex,
    sticky: bool,
    unicode: bool,
    /// The canonical `flags` string (`dgimsuvy` order).
    flags: String,
    /// Per capture group, in order: its name.
    names: Vec<Option<String>>,
}

thread_local! {
    static REGEXES: RefCell<Vec<Rc<Entry>>> = const { RefCell::new(Vec::new()) };
    static CACHE: RefCell<HashMap<(String, String), usize>> = RefCell::new(HashMap::new());
    /// The start and end (UTF-16 units) of the last match `exec` found.
    static LAST: Cell<(f64, f64)> = const { Cell::new((0.0, 0.0)) };
}

/// The capture groups of a pattern, in order, with their names.
fn scan_groups(p: &str) -> Vec<Option<String>> {
    let cs: Vec<char> = p.chars().collect();
    let mut out = Vec::new();
    let (mut i, mut in_class) = (0, false);
    while i < cs.len() {
        match cs[i] {
            '\\' => i += 1,
            '[' if !in_class => in_class = true,
            ']' if in_class => in_class = false,
            '(' if !in_class => {
                if cs.get(i + 1) == Some(&'?') {
                    if cs.get(i + 2) == Some(&'<') && !matches!(cs.get(i + 3), Some('=' | '!')) {
                        let name: String = cs[i + 3..].iter().take_while(|c| **c != '>').collect();
                        out.push(Some(name));
                    }
                } else {
                    out.push(None);
                }
            }
            _ => {}
        }
        i += 1;
    }
    out
}

/// The JavaScript flags of a flags string, canonical order; None if it is not a valid set.
fn parse_flags(f: &str) -> Option<(String, Flags)> {
    let (mut seen, mut flags) = ([false; 8], Flags::default());
    for c in f.chars() {
        let i = "dgimsuvy".find(c)?;
        if std::mem::replace(&mut seen[i], true) {
            return None;
        }
        match c {
            'i' => flags.icase = true,
            'm' => flags.multiline = true,
            's' => flags.dot_all = true,
            'u' => flags.unicode = true,
            'v' => flags.unicode_sets = true,
            _ => {}
        }
    }
    if seen[5] && seen[6] {
        return None;
    }
    let canon: String = "dgimsuvy"
        .chars()
        .enumerate()
        .filter(|(i, _)| seen[*i])
        .map(|(_, c)| c)
        .collect();
    Some((canon, flags))
}

/// The text JavaScriptCore gives for what `regress` reports (what scripts see in `e.message`).
fn jsc_message(msg: &str, pattern: &str) -> String {
    let text = match msg {
        "Unbalanced parenthesis" => {
            // More opening than closing parentheses: one is missing at the end.
            let (mut open, mut close, mut in_class, mut esc) = (0i32, 0i32, false, false);
            for c in pattern.chars() {
                if std::mem::take(&mut esc) {
                    continue;
                }
                match c {
                    '\\' => esc = true,
                    '[' => in_class = true,
                    ']' => in_class = false,
                    '(' if !in_class => open += 1,
                    ')' if !in_class => close += 1,
                    _ => {}
                }
            }
            if open > close {
                "missing )"
            } else {
                "unmatched parentheses"
            }
        }
        "Unbalanced bracket" => "missing terminating ] for character class",
        "Invalid atom character" => "nothing to repeat",
        "Duplicate capture group name" => "duplicate group specifier name",
        "Invalid quantifier" => "numbers out of order in {} quantifier",
        "Incomplete escape" => r"\ at end of pattern",
        "Quantifier not allowed here" => "invalid quantifier",
        m if m.starts_with("Range values reversed") => "range out of order in character class",
        m if m.starts_with("Backreference to invalid named capture group") => {
            r"invalid \k<> named backreference"
        }
        other => other,
    };
    format!("Invalid regular expression: {text}")
}

/// Checks a pattern and flags without compiling for use: the message of the `SyntaxError`.
pub fn regex_validate(pattern: &str, flags: &str) -> Result<(), String> {
    let Some((_, fl)) = parse_flags(flags) else {
        return Err("Invalid flags supplied to RegExp constructor.".into());
    };
    Regex::with_flags(pattern, fl)
        .map(|_| ())
        .map_err(|e| jsc_message(&e.to_string(), pattern))
}

fn entry(id: f64) -> Rc<Entry> {
    REGEXES.with(|r| r.borrow()[id as usize].clone())
}

/// Compiles `source` with `flags` (or finds it again): the regex id, or -1 with a `SyntaxError`
/// recorded.
///
/// # Safety
/// Both must be live strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_regex_new(source: *const Str, flags: *const Str) -> f64 {
    // SAFETY: by contract.
    let (src, fl) = unsafe { ((*source).to_string_lossy(), (*flags).to_string_lossy()) };
    if let Some(id) = CACHE.with(|c| c.borrow().get(&(src.clone(), fl.clone())).copied()) {
        return id as f64;
    }
    let Some((canon, parsed)) = parse_flags(&fl) else {
        super::raise(3.0, "Invalid flags supplied to RegExp constructor.");
        return -1.0;
    };
    let re = match Regex::with_flags(&src, parsed) {
        Ok(re) => re,
        Err(e) => {
            super::raise(3.0, &jsc_message(&e.to_string(), &src));
            return -1.0;
        }
    };
    let e = Entry {
        re,
        sticky: canon.contains('y'),
        unicode: canon.contains('u') || canon.contains('v'),
        names: scan_groups(&src),
        flags: canon,
    };
    let id = REGEXES.with(|r| {
        let mut r = r.borrow_mut();
        r.push(Rc::new(e));
        r.len() - 1
    });
    CACHE.with(|c| c.borrow_mut().insert((src, fl), id));
    id as f64
}

/// The `source` property: the pattern with `/` and line terminators escaped (EscapeRegExpPattern),
/// `(?:)` for the empty pattern.
///
/// # Safety
/// `source` must be a live string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_regex_source_gc(source: *const Str) -> *mut Str {
    // SAFETY: by contract.
    let p = unsafe { (*source).to_string_lossy() };
    let mut out = String::new();
    let (mut in_class, mut esc) = (false, false);
    for c in p.chars() {
        if std::mem::take(&mut esc) {
            out.push(c);
            continue;
        }
        match c {
            '\\' => {
                esc = true;
                out.push(c);
            }
            '[' => {
                in_class = true;
                out.push(c);
            }
            ']' => {
                in_class = false;
                out.push(c);
            }
            '/' if !in_class => out.push_str("\\/"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\u{2028}' => out.push_str("\\u2028"),
            '\u{2029}' => out.push_str("\\u2029"),
            _ => out.push(c),
        }
    }
    if out.is_empty() {
        out.push_str("(?:)");
    }
    // SAFETY: valid bytes.
    unsafe { ts_rt_str_new_gc(out.as_ptr(), out.len() as i64) }
}

/// The canonical `flags` string of a regex.
#[unsafe(no_mangle)]
pub extern "C" fn ts_rt_regex_flags_gc(id: f64) -> *mut Str {
    let f = entry(id).flags.clone();
    // SAFETY: valid bytes.
    unsafe { ts_rt_str_new_gc(f.as_ptr(), f.len() as i64) }
}

/// The number of capture groups.
#[unsafe(no_mangle)]
pub extern "C" fn ts_rt_regex_group_count(id: f64) -> f64 {
    entry(id).names.len() as f64
}

/// One search: the ranges (UTF-16 units) of the whole match and each capture group.
pub fn search(id: f64, text: &Str, start: usize) -> Option<Vec<Option<(usize, usize)>>> {
    let e = entry(id);
    let len = text.utf16_len as usize;
    if start > len {
        return None;
    }
    let m = if text.is_ascii() {
        // SAFETY: ASCII is valid UTF-8.
        let s = unsafe { std::str::from_utf8_unchecked(text.as_bytes()) };
        e.re.find_from_ascii(s, start).next()
    } else {
        let u = text.to_utf16();
        if e.unicode {
            e.re.find_from_utf16(&u, start).next()
        } else {
            e.re.find_from_ucs2(&u, start).next()
        }
    }?;
    if e.sticky && m.start() != start {
        return None;
    }
    let mut out = vec![Some((m.start(), m.end()))];
    out.extend(
        m.captures
            .iter()
            .map(|c| c.as_ref().map(|r| (r.start, r.end))),
    );
    Some(out)
}

fn piece(text: &Str, range: Option<(usize, usize)>) -> Option<Vec<u8>> {
    range.map(|(a, b)| text.sub_units(a as i64, b as i64))
}

/// Builds an array of strings (null for a group that did not take part) from pieces.
fn string_array(pieces: Vec<Option<Vec<u8>>>) -> *mut Array {
    let arr = ts_rt_arr_new_gc(pieces.len() as i64, 1);
    for p in &pieces {
        let v = match p {
            // SAFETY: neither call collects, so the unrooted array and strings stay valid.
            Some(b) => (unsafe { ts_rt_str_new(b.as_ptr(), b.len() as i64) }) as u64,
            None => 0,
        };
        // SAFETY: as above.
        unsafe { ts_rt_arr_push(arr, v) };
    }
    arr
}

/// `re.exec(text)` from unit `start`: the strings of the match and its groups, or null. On a match
/// the start and end are available from `ts_rt_regex_match_index` / `ts_rt_regex_match_end`.
///
/// # Safety
/// `text` must be a live string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_regex_exec_gc(id: f64, text: *const Str, start: f64) -> *mut Array {
    // SAFETY: by contract.
    let t = unsafe { &*text };
    let start = if start.is_nan() {
        0.0
    } else {
        start.trunc().max(0.0)
    } as usize;
    let Some(ranges) = search(id, t, start) else {
        return std::ptr::null_mut();
    };
    let (a, b) = ranges[0].expect("the whole match");
    LAST.with(|l| l.set((a as f64, b as f64)));
    string_array(ranges.into_iter().map(|r| piece(t, r)).collect())
}

/// The index of the last match.
#[unsafe(no_mangle)]
pub extern "C" fn ts_rt_regex_match_index() -> f64 {
    LAST.with(|l| l.get().0)
}

/// The end of the last match (the next `lastIndex`).
#[unsafe(no_mangle)]
pub extern "C" fn ts_rt_regex_match_end() -> f64 {
    LAST.with(|l| l.get().1)
}

/// The `groups` object of a match: a map from group name to string (null if it did not take
/// part), or null when the regex has no named groups.
///
/// # Safety
/// `matches` is the array `ts_rt_regex_exec_gc` returned.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_regex_groups_gc(
    id: f64,
    matches: *const Array,
) -> *mut super::MapObj {
    let e = entry(id);
    if e.names.iter().all(|n| n.is_none()) {
        return std::ptr::null_mut();
    }
    // SAFETY: by contract.
    let a = unsafe { &*matches };
    let map = super::ts_rt_map_new_gc(1.0, 1.0);
    for (i, name) in e.names.iter().enumerate() {
        let Some(name) = name else { continue };
        // SAFETY: no collection happens here (host constructors), so the new strings are safe.
        unsafe {
            let k = ts_rt_str_new(name.as_ptr(), name.len() as i64);
            let v: u64 = a.get::<u64>(i + 1);
            super::ts_rt_map_set(map, k as u64, v);
        }
    }
    map
}
