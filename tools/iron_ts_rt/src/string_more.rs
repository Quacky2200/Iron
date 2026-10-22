//! Strings, part two: trimming, padding, repetition, case mapping, `split`/`join`, and the
//! conversions between strings and numbers. Same rules as `string.rs`: JavaScript's results,
//! WTF-8 storage, `_gc` functions for generated code.

use super::string::compare_bytes;
use super::string::*;
use super::{ts_rt_arr_new_gc, ts_rt_arr_push};

fn st<'a>(p: *const Str) -> &'a Str {
    // SAFETY: callers pass live strings (the extern functions' contract).
    unsafe { &*p }
}

/// The longest string JavaScriptCore makes (2^31 - 1 units).
const MAX_STRING: f64 = 2147483647.0;

/// JavaScript's WhiteSpace and LineTerminator code points.
fn is_space(cp: u32) -> bool {
    matches!(
        cp,
        9..=13
            | 0x20
            | 0xA0
            | 0x1680
            | 0x2000..=0x200A
            | 0x2028
            | 0x2029
            | 0x202F
            | 0x205F
            | 0x3000
            | 0xFEFF
    )
}

/// `bytes` without leading and/or trailing white space.
fn trim_bytes(b: &[u8], start: bool, end: bool) -> &[u8] {
    let (mut s, mut e) = (0, b.len());
    if start {
        while s < e {
            let (cp, n) = wtf8_decode(b, s);
            if !is_space(cp) {
                break;
            }
            s += n;
        }
    }
    if end {
        while e > s {
            let mut q = e - 1;
            while b[q] & 0xC0 == 0x80 {
                q -= 1;
            }
            if !is_space(wtf8_decode(b, q).0) {
                break;
            }
            e = q;
        }
    }
    &b[s..e]
}

/// Maps the well formed stretches of a string and leaves lone surrogates alone.
fn map_text(b: &[u8], f: impl Fn(&str) -> String) -> Vec<u8> {
    let mut out = Vec::with_capacity(b.len());
    let mut run_start = 0;
    let mut p = 0;
    while p < b.len() {
        let (cp, n) = wtf8_decode(b, p);
        if (0xD800..0xE000).contains(&cp) {
            // SAFETY: the stretch before a lone surrogate is valid UTF-8.
            let run = unsafe { std::str::from_utf8_unchecked(&b[run_start..p]) };
            out.extend_from_slice(f(run).as_bytes());
            out.extend_from_slice(&b[p..p + n]);
            run_start = p + n;
        }
        p += n;
    }
    // SAFETY: the rest is valid UTF-8.
    let run = unsafe { std::str::from_utf8_unchecked(&b[run_start..]) };
    out.extend_from_slice(f(run).as_bytes());
    out
}

/// The kind of error a runtime function failed with, for `__rtThrow` in the prelude to raise.
pub const ERR_RANGE: f64 = 1.0;

thread_local! {
    static RT_ERROR: std::cell::RefCell<Option<(f64, String)>> = const { std::cell::RefCell::new(None) };
}

/// A runtime function failed the way JavaScript's would throw: record it (the function returns a
/// harmless value and the compiled code throws right after the call).
pub(crate) fn raise(kind: f64, message: &str) {
    RT_ERROR.with(|e| *e.borrow_mut() = Some((kind, message.to_string())));
}

/// 0, or the kind of the runtime error recorded by the last call (1 `RangeError`).
#[unsafe(no_mangle)]
pub extern "C" fn ts_rt_error_kind() -> f64 {
    RT_ERROR.with(|e| e.borrow().as_ref().map_or(0.0, |(k, _)| *k))
}

/// The recorded runtime error's message; clears the error.
#[unsafe(no_mangle)]
pub extern "C" fn ts_rt_error_message_gc() -> *mut Str {
    let m = RT_ERROR
        .with(|e| e.borrow_mut().take())
        .map(|(_, m)| m)
        .unwrap_or_default();
    new_gc(m.into_bytes())
}

fn new_gc(bytes: Vec<u8>) -> *mut Str {
    // SAFETY: `bytes` is valid WTF-8.
    unsafe { ts_rt_str_new_gc(bytes.as_ptr(), bytes.len() as i64) }
}

// ---- trim, repeat, pad, case ----

macro_rules! trimmer {
    ($name:ident, $start:expr, $end:expr) => {
        /// `s.trim()`, `s.trimStart()` or `s.trimEnd()`.
        ///
        /// # Safety
        /// `s` must be a live string.
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name(s: *const Str) -> *mut Str {
            new_gc(trim_bytes(st(s).as_bytes(), $start, $end).to_vec())
        }
    };
}
trimmer!(ts_rt_str_trim_gc, true, true);
trimmer!(ts_rt_str_trim_start_gc, true, false);
trimmer!(ts_rt_str_trim_end_gc, false, true);

/// `s.repeat(n)`. A negative or infinite count is a fault (a `RangeError`).
///
/// # Safety
/// `s` must be a live string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_str_repeat_gc(s: *const Str, n: f64) -> *mut Str {
    let n = if n.is_nan() { 0.0 } else { n.trunc() };
    if !(n >= 0.0 && n.is_finite()) {
        raise(
            ERR_RANGE,
            "String.prototype.repeat argument must be greater than or equal to 0 and not be Infinity",
        );
        return new_gc(Vec::new());
    }
    let s = st(s);
    if n * (s.byte_len as f64) >= MAX_STRING {
        raise(ERR_RANGE, "Out of memory");
        return new_gc(Vec::new());
    }
    let mut out = Vec::with_capacity(s.byte_len as usize * n as usize);
    for _ in 0..n as usize {
        append_wtf8(&mut out, s.as_bytes());
    }
    new_gc(out)
}

fn pad(s: &Str, target: f64, filler: &Str, at_start: bool) -> *mut Str {
    let target = if target.is_nan() { 0.0 } else { target.trunc() };
    let have = s.utf16_len as f64;
    if target <= have || filler.byte_len == 0 {
        return new_gc(s.as_bytes().to_vec());
    }
    if target >= MAX_STRING {
        raise(ERR_RANGE, "Out of memory");
        return new_gc(Vec::new());
    }
    let fill = (target - have) as usize;
    let pad_units: Vec<u16> = filler.to_utf16().into_iter().cycle().take(fill).collect();
    let pad_bytes = wtf8_from_utf16(&pad_units);
    let mut out = Vec::with_capacity(pad_bytes.len() + s.byte_len as usize);
    if at_start {
        append_wtf8(&mut out, &pad_bytes);
        append_wtf8(&mut out, s.as_bytes());
    } else {
        append_wtf8(&mut out, s.as_bytes());
        append_wtf8(&mut out, &pad_bytes);
    }
    new_gc(out)
}

/// `s.padStart(target, filler)`.
///
/// # Safety
/// Both must be live strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_str_pad_start_gc(
    s: *const Str,
    target: f64,
    filler: *const Str,
) -> *mut Str {
    pad(st(s), target, st(filler), true)
}

/// `s.padEnd(target, filler)`.
///
/// # Safety
/// Both must be live strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_str_pad_end_gc(
    s: *const Str,
    target: f64,
    filler: *const Str,
) -> *mut Str {
    pad(st(s), target, st(filler), false)
}

/// `s.toUpperCase()`.
///
/// # Safety
/// `s` must be a live string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_str_to_upper_gc(s: *const Str) -> *mut Str {
    new_gc(map_text(st(s).as_bytes(), |t| t.to_uppercase()))
}

/// `s.toLowerCase()` (with the final sigma rule).
///
/// # Safety
/// `s` must be a live string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_str_to_lower_gc(s: *const Str) -> *mut Str {
    new_gc(map_text(st(s).as_bytes(), |t| t.to_lowercase()))
}

/// `s.at(i)`: null (undefined) out of range, negative counts from the end.
///
/// # Safety
/// `s` must be a live string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_str_at_gc(s: *const Str, i: f64) -> *mut Str {
    let len = st(s).utf16_len as f64;
    let t = if i.is_nan() { 0.0 } else { i.trunc() };
    let k = if t < 0.0 { len + t } else { t };
    if k < 0.0 || k >= len {
        return std::ptr::null_mut();
    }
    unsafe { ts_rt_str_char_at_gc(s, k) }
}

// ---- n-ary concatenation, split, join ----

/// The strings `parts[0..n]` joined (a template literal): one allocation.
///
/// # Safety
/// `parts` must point to `n` live strings; a null part is the empty string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_str_concat_n_gc(parts: *const *const Str, n: i64) -> *mut Str {
    let mut out = Vec::new();
    for i in 0..n.max(0) as usize {
        // SAFETY: by contract.
        let p = unsafe { *parts.add(i) };
        if !p.is_null() {
            append_wtf8(&mut out, st(p).as_bytes());
        }
    }
    new_gc(out)
}

/// The pieces of `s` between occurrences of `sep` (an empty `sep` gives the UTF-16 units).
fn split_pieces(s: &Str, sep: &Str) -> Vec<Vec<u8>> {
    let mut pieces: Vec<Vec<u8>> = Vec::new();
    if sep.byte_len == 0 {
        for i in 0..s.utf16_len {
            pieces.push(s.sub_units(i, i + 1));
        }
    } else if sep.is_well_formed() {
        let (hay, nd) = (s.as_bytes(), sep.as_bytes());
        let (mut from, mut p) = (0, 0);
        while p + nd.len() <= hay.len() {
            if &hay[p..p + nd.len()] == nd {
                pieces.push(hay[from..p].to_vec());
                p += nd.len();
                from = p;
            } else {
                p += 1;
            }
        }
        pieces.push(hay[from..].to_vec());
    } else {
        let (h, n) = (s.to_utf16(), sep.to_utf16());
        let (mut from, mut p) = (0, 0);
        while p + n.len() <= h.len() {
            if h[p..p + n.len()] == n[..] {
                pieces.push(wtf8_from_utf16(&h[from..p]));
                p += n.len();
                from = p;
            } else {
                p += 1;
            }
        }
        pieces.push(wtf8_from_utf16(&h[from..]));
    }
    pieces
}

/// `s.split(sep)` into a new array of strings.
///
/// # Safety
/// Both must be live strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_str_split_gc(s: *const Str, sep: *const Str) -> *mut super::Array {
    let pieces = split_pieces(st(s), st(sep));
    let arr = ts_rt_arr_new_gc(pieces.len() as i64, 1);
    for piece in &pieces {
        // SAFETY: neither call collects, so the unrooted array and strings stay valid.
        unsafe {
            let p = ts_rt_str_new(piece.as_ptr(), piece.len() as i64);
            ts_rt_arr_push(arr, p as u64);
        }
    }
    arr
}

/// `[...s]` / `Array.from(s)`: the code points of `s`, each as a string (a lone surrogate stays
/// alone).
///
/// # Safety
/// `s` must be a live string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_str_chars_gc(s: *const Str) -> *mut super::Array {
    let units = st(s).to_utf16();
    let mut pieces: Vec<Vec<u8>> = Vec::new();
    let mut i = 0;
    while i < units.len() {
        let pair = (0xD800..0xDC00).contains(&units[i])
            && units
                .get(i + 1)
                .is_some_and(|l| (0xDC00..0xE000).contains(l));
        let n = if pair { 2 } else { 1 };
        pieces.push(wtf8_from_utf16(&units[i..i + n]));
        i += n;
    }
    let arr = ts_rt_arr_new_gc(pieces.len() as i64, 1);
    for piece in &pieces {
        // SAFETY: neither call collects, so the unrooted array and strings stay valid.
        unsafe {
            let p = ts_rt_str_new(piece.as_ptr(), piece.len() as i64);
            ts_rt_arr_push(arr, p as u64);
        }
    }
    arr
}

/// `parts.join(sep)`; a null element is empty.
///
/// # Safety
/// `parts` must be an array of strings (or null) and `sep` a live string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_str_join_gc(
    parts: *const super::Array,
    sep: *const Str,
) -> *mut Str {
    // SAFETY: by contract.
    let a = unsafe { &*parts };
    let sep = st(sep).as_bytes();
    let mut out = Vec::new();
    for i in 0..a.len() {
        if i > 0 {
            append_wtf8(&mut out, sep);
        }
        // SAFETY: slots of a pointer array hold string pointers.
        let p: *const Str = unsafe { a.get::<*const Str>(i) };
        if !p.is_null() {
            append_wtf8(&mut out, st(p).as_bytes());
        }
    }
    new_gc(out)
}

/// `booleans.join(sep)` for an array of booleans.
///
/// # Safety
/// `parts` must be an array of booleans and `sep` a live string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_str_join_bools_gc(
    parts: *const super::Array,
    sep: *const Str,
) -> *mut Str {
    // SAFETY: by contract.
    let a = unsafe { &*parts };
    let sep = st(sep).as_bytes();
    let mut out = Vec::new();
    for i in 0..a.len() {
        if i > 0 {
            append_wtf8(&mut out, sep);
        }
        // SAFETY: slots of a boolean array hold 0 or 1.
        let x: u64 = unsafe { a.get::<u64>(i) };
        out.extend_from_slice(if x != 0 { b"true" } else { b"false" });
    }
    new_gc(out)
}

/// `numbers.join(sep)` for an array of numbers.
///
/// # Safety
/// `parts` must be an array of numbers and `sep` a live string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_str_join_nums_gc(
    parts: *const super::Array,
    sep: *const Str,
) -> *mut Str {
    // SAFETY: by contract.
    let a = unsafe { &*parts };
    let sep = st(sep).as_bytes();
    let mut out = Vec::new();
    for i in 0..a.len() {
        if i > 0 {
            append_wtf8(&mut out, sep);
        }
        // SAFETY: slots of a number array hold f64s.
        let x: f64 = unsafe { a.get::<f64>(i) };
        out.extend_from_slice(number_to_string(x).as_bytes());
    }
    new_gc(out)
}

/// `xs.sort()` with no comparator: by the string form of each element (UTF-16 code unit order),
/// stable. `kind` 0 is an array of numbers, 1 an array of strings.
///
/// # Safety
/// `a` must be an array of numbers (kind 0) or of strings (kind 1).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_arr_sort_default(a: *mut super::Array, kind: f64) {
    // SAFETY: by contract.
    let a = unsafe { &mut *a };
    let n = a.len();
    if n < 2 {
        return;
    }
    if kind == 0.0 {
        // SAFETY: slots of a number array hold f64s.
        let mut items: Vec<(String, f64)> = (0..n)
            .map(|i| {
                let x = unsafe { a.get::<f64>(i) };
                (number_to_string(x), x)
            })
            .collect();
        items.sort_by(|p, q| p.0.as_bytes().cmp(q.0.as_bytes()));
        for (i, (_, x)) in items.into_iter().enumerate() {
            // SAFETY: in range.
            unsafe { *(a.data as *mut f64).add(i) = x };
        }
    } else {
        // SAFETY: slots of a string array hold string pointers (or null: sorted last).
        let mut items: Vec<*const Str> =
            (0..n).map(|i| unsafe { a.get::<*const Str>(i) }).collect();
        items.sort_by(|p, q| match (p.is_null(), q.is_null()) {
            (true, true) => std::cmp::Ordering::Equal,
            (true, false) => std::cmp::Ordering::Greater,
            (false, true) => std::cmp::Ordering::Less,
            (false, false) => compare_bytes(st(*p).as_bytes(), st(*q).as_bytes()),
        });
        for (i, p) in items.into_iter().enumerate() {
            // SAFETY: in range.
            unsafe { *(a.data as *mut *const Str).add(i) = p };
        }
    }
}

// ---- console ----

/// Where `console.log` and friends go: the level (0 log, 1 info, 2 debug, 3 warn, 4 error) and the
/// text (UTF-8, valid for the call only).
pub type ConsoleHook = extern "C" fn(level: i32, text: *const u8, len: usize);

thread_local! {
    static CONSOLE: std::cell::Cell<Option<ConsoleHook>> = const { std::cell::Cell::new(None) };
}

/// Sets this thread's console (None: standard error). The engine points it at its log.
#[unsafe(no_mangle)]
pub extern "C" fn ts_rt_set_console(hook: Option<ConsoleHook>) {
    CONSOLE.with(|c| c.set(hook));
}

/// `console.log(..)` and the other levels, with the arguments already joined by spaces.
///
/// # Safety
/// `msg` must be a live string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_console(level: f64, msg: *const Str) {
    let text = st(msg).to_string_lossy();
    match CONSOLE.with(|c| c.get()) {
        Some(hook) => hook(level as i32, text.as_ptr(), text.len()),
        None => eprintln!("{text}"),
    }
}

// ---- numbers and strings ----

/// `String(x)` for a number: the shortest digits that round-trip, in JavaScript's layout.
pub fn number_to_string(x: f64) -> String {
    if x.is_nan() {
        return "NaN".into();
    }
    if x == 0.0 {
        return "0".into();
    }
    if x.is_infinite() {
        return if x > 0.0 { "Infinity" } else { "-Infinity" }.into();
    }
    let sign = if x < 0.0 { "-" } else { "" };
    let sci = format!("{:e}", x.abs());
    let (mant, exp) = sci.split_once('e').expect("exponent form");
    let digits: String = mant.chars().filter(|c| *c != '.').collect();
    let k = digits.len() as i32;
    let n = exp.parse::<i32>().expect("exponent") + 1;
    let body = if k <= n && n <= 21 {
        format!("{digits}{}", "0".repeat((n - k) as usize))
    } else if 0 < n && n <= 21 {
        format!("{}.{}", &digits[..n as usize], &digits[n as usize..])
    } else if -6 < n && n <= 0 {
        format!("0.{}{digits}", "0".repeat((-n) as usize))
    } else {
        let e = n - 1;
        let es = format!("{}{}", if e < 0 { '-' } else { '+' }, e.abs());
        if k == 1 {
            format!("{digits}e{es}")
        } else {
            format!("{}.{}e{es}", &digits[..1], &digits[1..])
        }
    };
    format!("{sign}{body}")
}

/// Adds one unit in the last place of a decimal string like `12.345`.
fn increment_decimal(s: &str) -> String {
    let mut b: Vec<u8> = s.bytes().collect();
    let mut i = b.len();
    loop {
        if i == 0 {
            b.insert(0, b'1');
            break;
        }
        i -= 1;
        match b[i] {
            b'.' => continue,
            b'9' => b[i] = b'0',
            d => {
                b[i] = d + 1;
                break;
            }
        }
    }
    String::from_utf8(b).unwrap()
}

/// `x.toFixed(digits)`: rounds half up on the exact value of the double (Rust rounds ties to even).
pub fn number_to_fixed(x: f64, digits: usize) -> String {
    if !x.is_finite() || x.abs() >= 1e21 {
        return number_to_string(x);
    }
    let sign = if x < 0.0 { "-" } else { "" };
    let ax = x.abs();
    let mut body = format!("{ax:.digits$}");
    let probe = format!("{ax:.*}", digits + 1);
    if probe.ends_with('5') {
        // Only an exact tie differs: the exact expansion ends right after that 5.
        let exact = format!("{ax:.1100}");
        let cut = exact.find('.').unwrap() + 1 + digits + 1;
        if exact[cut..].bytes().all(|c| c == b'0') && exact.as_bytes()[cut - 1] == b'5' {
            let trunc = &exact[..cut - 1];
            let trunc = trunc.strip_suffix('.').unwrap_or(trunc);
            body = increment_decimal(trunc);
        }
    }
    format!("{sign}{body}")
}

/// `String(x)`.
#[unsafe(no_mangle)]
pub extern "C" fn ts_rt_num_to_str_gc(x: f64) -> *mut Str {
    new_gc(number_to_string(x).into_bytes())
}

/// `x.toFixed(digits)`; `digits` outside 0..=100 is a fault (a `RangeError`).
#[unsafe(no_mangle)]
pub extern "C" fn ts_rt_num_to_fixed_gc(x: f64, digits: f64) -> *mut Str {
    let d = if digits.is_nan() { 0.0 } else { digits.trunc() };
    if !(0.0..=100.0).contains(&d) {
        raise(ERR_RANGE, "toFixed() argument must be between 0 and 100");
        return new_gc(Vec::new());
    }
    new_gc(number_to_fixed(x, d as usize).into_bytes())
}

/// `x.toString(radix)` for an integer `x`; a fraction in another radix is a fault for now.
#[unsafe(no_mangle)]
pub extern "C" fn ts_rt_num_to_radix_gc(x: f64, radix: f64) -> *mut Str {
    let r = if radix.is_nan() { 10.0 } else { radix.trunc() };
    if !(2.0..=36.0).contains(&r) {
        raise(
            ERR_RANGE,
            "toString() radix argument must be between 2 and 36",
        );
        return new_gc(Vec::new());
    }
    if r == 10.0 || !x.is_finite() || x == 0.0 {
        return ts_rt_num_to_str_gc(x);
    }
    assert!(
        x.fract() == 0.0 && x.abs() < 9.007199254740992e15,
        "toString(radix) of a fractional or very large number is not supported yet"
    );
    let mut n = x.abs() as u64;
    let mut digits = Vec::new();
    while n > 0 {
        digits.push(std::char::from_digit((n % r as u64) as u32, r as u32).unwrap() as u8);
        n /= r as u64;
    }
    if x < 0.0 {
        digits.push(b'-');
    }
    digits.reverse();
    new_gc(digits)
}

/// The length of the longest decimal literal at the start of `t` (sign, `Infinity`, or digits with
/// an optional fraction and exponent); 0 if there is none.
fn decimal_prefix(t: &str) -> usize {
    let b = t.as_bytes();
    let mut i = 0;
    if matches!(b.first(), Some(b'+' | b'-')) {
        i = 1;
    }
    if t[i..].starts_with("Infinity") {
        return i + 8;
    }
    let digits = |from: usize| b[from..].iter().take_while(|c| c.is_ascii_digit()).count();
    let int = digits(i);
    let mut j = i + int;
    let mut frac = 0;
    if b.get(j) == Some(&b'.') {
        frac = digits(j + 1);
        if int > 0 || frac > 0 {
            j += 1 + frac;
        }
    }
    if int == 0 && frac == 0 {
        return 0;
    }
    if matches!(b.get(j), Some(b'e' | b'E')) {
        let mut k = j + 1;
        if matches!(b.get(k), Some(b'+' | b'-')) {
            k += 1;
        }
        let e = digits(k);
        if e > 0 {
            j = k + e;
        }
    }
    j
}

fn parse_decimal(t: &str) -> f64 {
    match t {
        "Infinity" | "+Infinity" => f64::INFINITY,
        "-Infinity" => f64::NEG_INFINITY,
        _ => t.parse::<f64>().unwrap_or(f64::NAN),
    }
}

/// The text after leading (and optionally trailing) white space. A lone surrogate is neither white
/// space nor part of a number, so converting it to U+FFFD changes no result.
fn trimmed(s: &Str, end: bool) -> String {
    String::from_utf8_lossy(trim_bytes(s.as_bytes(), true, end)).into_owned()
}

/// `Number(s)`: the whole trimmed string must be a number (or hex, octal or binary).
///
/// # Safety
/// `s` must be a live string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_str_to_number(s: *const Str) -> f64 {
    let t = trimmed(st(s), true);
    if t.is_empty() {
        return 0.0;
    }
    for (prefix, radix) in [
        ("0x", 16),
        ("0X", 16),
        ("0o", 8),
        ("0O", 8),
        ("0b", 2),
        ("0B", 2),
    ] {
        if let Some(rest) = t.strip_prefix(prefix) {
            if rest.is_empty() || !rest.chars().all(|c| c.is_digit(radix)) {
                return f64::NAN;
            }
            return rest.chars().fold(0.0, |acc, c| {
                acc * radix as f64 + c.to_digit(radix).unwrap() as f64
            });
        }
    }
    if decimal_prefix(&t) == t.len() {
        parse_decimal(&t)
    } else {
        f64::NAN
    }
}

/// `parseFloat(s)`: the longest numeric prefix after leading white space, NaN if none.
///
/// # Safety
/// `s` must be a live string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_str_parse_float(s: *const Str) -> f64 {
    let t = trimmed(st(s), false);
    match decimal_prefix(&t) {
        0 => f64::NAN,
        n => parse_decimal(&t[..n]),
    }
}

/// `parseInt(s, radix)`; pass NaN or 0 for no radix.
///
/// # Safety
/// `s` must be a live string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_str_parse_int(s: *const Str, radix: f64) -> f64 {
    let t = trimmed(st(s), false);
    let mut rest = t.as_str();
    let mut sign = 1.0;
    if let Some(r) = rest.strip_prefix('-') {
        sign = -1.0;
        rest = r;
    } else if let Some(r) = rest.strip_prefix('+') {
        rest = r;
    }
    // ToInt32 of the radix.
    let mut r = if radix.is_finite() {
        (radix.trunc() as i64 as i32) as i64
    } else {
        0
    };
    let mut strip = true;
    if r != 0 {
        if !(2..=36).contains(&r) {
            return f64::NAN;
        }
        if r != 16 {
            strip = false;
        }
    } else {
        r = 10;
    }
    if strip && (rest.starts_with("0x") || rest.starts_with("0X")) {
        rest = &rest[2..];
        r = 16;
    }
    let end = rest
        .char_indices()
        .find(|(_, c)| !c.is_digit(r as u32))
        .map_or(rest.len(), |(i, _)| i);
    if end == 0 {
        return f64::NAN;
    }
    let digits = &rest[..end];
    let v = if r == 10 {
        digits.parse::<f64>().unwrap_or(f64::NAN)
    } else {
        digits.chars().fold(0.0, |acc, c| {
            acc * r as f64 + c.to_digit(r as u32).unwrap() as f64
        })
    };
    sign * v
}
