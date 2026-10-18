//! Strings: WTF-8 bytes with JavaScript's UTF-16 behaviour (design: `docs/research/iron-ts-strings.md`).
//!
//! A string is stored as UTF-8, plus the three-byte form of a lone surrogate (WTF-8), so every
//! JavaScript string has exactly one byte form and a string without lone surrogates is valid UTF-8
//! (Rust can borrow it as `&str`). What scripts observe is JavaScript's: `length`, indices, `slice`
//! and `charCodeAt` count UTF-16 code units, and ordering is by code unit. The conversions between
//! unit indices and byte offsets are what this module is for: ASCII strings are O(1), others use a
//! one-entry cursor so forward scans cost O(1) per step.
//!
//! Layout: `Str { header, byte_len, utf16_len, flags, cursor_unit, cursor_byte }` and the bytes
//! right after it, one GC object (a leaf for the collector). A string is immutable except for its
//! cursor, which only caches a position (it lives in writable memory).
//!
//! Functions named `*_gc` are for generated code: they may collect first (every live pointer is
//! rooted by the caller) and copy their arguments before they do. The others never collect and are
//! for the host.

use super::{HDR_STR, Heap, maybe_collect, new_obj, with_heap};

/// Every byte is below 0x80 (so unit index = byte offset).
pub const FLAG_ASCII: u64 = 1;
/// No lone surrogates: the bytes are valid UTF-8.
pub const FLAG_WELL_FORMED: u64 = 2;

#[repr(C)]
pub struct Str {
    pub header: u64,
    pub byte_len: i64,
    /// `length` in UTF-16 code units.
    pub utf16_len: i64,
    pub flags: u64,
    /// The last code point located: its unit index and byte offset (always a boundary).
    cursor_unit: i64,
    cursor_byte: i64,
}

const DATA: usize = std::mem::size_of::<Str>();

// ---- WTF-8 helpers ----

/// Units and flags of a WTF-8 byte string, in one pass.
fn scan(b: &[u8]) -> (i64, u64) {
    if b.is_ascii() {
        return (b.len() as i64, FLAG_ASCII | FLAG_WELL_FORMED);
    }
    let mut units = 0i64;
    let mut wf = true;
    for (i, &x) in b.iter().enumerate() {
        if x < 0x80 {
            units += 1;
        } else if x >= 0xF0 {
            units += 2;
        } else if x >= 0xC0 {
            units += 1;
            // ED A0..BF xx is a surrogate.
            if x == 0xED && b.get(i + 1).is_some_and(|&y| y >= 0xA0) {
                wf = false;
            }
        }
    }
    (units, if wf { FLAG_WELL_FORMED } else { 0 })
}

/// The code point at `p` (a boundary) and its length in bytes.
pub fn wtf8_decode(b: &[u8], p: usize) -> (u32, usize) {
    let x = b[p] as u32;
    let c = |i: usize| b[p + i] as u32 & 0x3F;
    if x < 0x80 {
        (x, 1)
    } else if x < 0xE0 {
        (((x & 0x1F) << 6) | c(1), 2)
    } else if x < 0xF0 {
        (((x & 0x0F) << 12) | (c(1) << 6) | c(2), 3)
    } else {
        (((x & 0x07) << 18) | (c(1) << 12) | (c(2) << 6) | c(3), 4)
    }
}

/// Appends the WTF-8 form of `cp` (a surrogate gets its three-byte form).
fn encode(out: &mut Vec<u8>, cp: u32) {
    if cp < 0x80 {
        out.push(cp as u8);
    } else if cp < 0x800 {
        out.push(0xC0 | (cp >> 6) as u8);
        out.push(0x80 | (cp & 0x3F) as u8);
    } else if cp < 0x10000 {
        out.push(0xE0 | (cp >> 12) as u8);
        out.push(0x80 | ((cp >> 6) & 0x3F) as u8);
        out.push(0x80 | (cp & 0x3F) as u8);
    } else {
        out.push(0xF0 | (cp >> 18) as u8);
        out.push(0x80 | ((cp >> 12) & 0x3F) as u8);
        out.push(0x80 | ((cp >> 6) & 0x3F) as u8);
        out.push(0x80 | (cp & 0x3F) as u8);
    }
}

fn units_of(cp: u32) -> i64 {
    if cp >= 0x10000 { 2 } else { 1 }
}

fn high_unit(cp: u32) -> u32 {
    0xD800 + ((cp - 0x10000) >> 10)
}

fn low_unit(cp: u32) -> u32 {
    0xDC00 + ((cp - 0x10000) & 0x3FF)
}

fn prev_start(b: &[u8], mut p: usize) -> usize {
    p -= 1;
    while b[p] & 0xC0 == 0x80 {
        p -= 1;
    }
    p
}

/// How many UTF-16 units a byte contributes (a continuation byte none).
fn weight(x: u8) -> i64 {
    if x < 0x80 {
        1
    } else if x < 0xC0 {
        0
    } else if x < 0xF0 {
        1
    } else {
        2
    }
}

/// The UTF-16 code units of WTF-8 bytes.
fn units(b: &[u8]) -> impl Iterator<Item = u16> + '_ {
    let mut p = 0;
    let mut low: Option<u16> = None;
    std::iter::from_fn(move || {
        if let Some(l) = low.take() {
            return Some(l);
        }
        if p >= b.len() {
            return None;
        }
        let (cp, n) = wtf8_decode(b, p);
        p += n;
        if cp >= 0x10000 {
            low = Some(low_unit(cp) as u16);
            Some(high_unit(cp) as u16)
        } else {
            Some(cp as u16)
        }
    })
}

/// WTF-8 for a sequence of UTF-16 units (pairs join, lone surrogates keep their own form).
pub fn wtf8_from_utf16(u: &[u16]) -> Vec<u8> {
    let mut out = Vec::with_capacity(u.len());
    let mut i = 0;
    while i < u.len() {
        let x = u[i] as u32;
        if (0xD800..0xDC00).contains(&x)
            && let Some(&y) = u.get(i + 1)
            && (0xDC00..0xE000).contains(&(y as u32))
        {
            encode(
                &mut out,
                0x10000 + ((x - 0xD800) << 10) + (y as u32 - 0xDC00),
            );
            i += 2;
            continue;
        }
        encode(&mut out, x);
        i += 1;
    }
    out
}

/// JavaScript's ToIntegerOrInfinity.
fn to_int(x: f64) -> f64 {
    if x.is_nan() { 0.0 } else { x.trunc() }
}

/// An index clamped to `0..=len` (`substring`, search positions).
fn clamp_index(x: f64, len: i64) -> i64 {
    to_int(x).clamp(0.0, len as f64) as i64
}

/// An index where a negative value counts from the end (`slice`).
fn relative_index(x: f64, len: i64) -> i64 {
    let t = to_int(x);
    if t < 0.0 {
        (len as f64 + t).max(0.0) as i64
    } else {
        t.min(len as f64) as i64
    }
}

// ---- reading a Str ----

impl Str {
    /// # Safety
    /// `p` must point to a live string.
    pub unsafe fn from_raw<'a>(p: *const Str) -> &'a Str {
        unsafe { &*p }
    }

    pub fn as_bytes(&self) -> &[u8] {
        // SAFETY: the bytes follow the header in the same allocation.
        unsafe {
            std::slice::from_raw_parts(
                (self as *const Str as *const u8).add(DATA),
                self.byte_len as usize,
            )
        }
    }

    pub fn is_ascii(&self) -> bool {
        self.flags & FLAG_ASCII != 0
    }

    pub fn is_well_formed(&self) -> bool {
        self.flags & FLAG_WELL_FORMED != 0
    }

    /// Borrowed when the string has no lone surrogates (the normal case).
    pub fn as_str(&self) -> Option<&str> {
        // SAFETY: well formed WTF-8 is valid UTF-8.
        self.is_well_formed()
            .then(|| unsafe { std::str::from_utf8_unchecked(self.as_bytes()) })
    }

    /// For the engine side: lone surrogates become U+FFFD.
    pub fn to_string_lossy(&self) -> String {
        if let Some(s) = self.as_str() {
            return s.to_owned();
        }
        let b = self.as_bytes();
        let mut out = String::with_capacity(b.len());
        let mut p = 0;
        while p < b.len() {
            let (cp, n) = wtf8_decode(b, p);
            out.push(char::from_u32(cp).unwrap_or('\u{FFFD}'));
            p += n;
        }
        out
    }

    pub fn to_utf16(&self) -> Vec<u16> {
        units(self.as_bytes()).collect()
    }

    fn set_cursor(&self, unit: i64, byte: usize) {
        // SAFETY: a string is only touched by its own thread's code; the cursor is a cache.
        unsafe {
            let m = self as *const Str as *mut Str;
            (*m).cursor_unit = unit;
            (*m).cursor_byte = byte as i64;
        }
    }

    /// The code point that holds unit `i` (`0..=utf16_len`): its byte offset and its first unit.
    /// For `i == utf16_len` that is the end of the string.
    fn locate(&self, i: i64) -> (usize, i64) {
        let b = self.as_bytes();
        if self.is_ascii() {
            return (i as usize, i);
        }
        if i >= self.utf16_len {
            return (b.len(), self.utf16_len);
        }
        let cursor = (self.cursor_unit, self.cursor_byte as usize);
        // Start from the nearest of the start, the cursor and the end.
        let mut best = (0i64, 0usize);
        for a in [(0, 0), cursor, (self.utf16_len, b.len())] {
            if (a.0 - i).abs() < (best.0 - i).abs() {
                best = a;
            }
        }
        let (mut u, mut p) = best;
        if u > i {
            while u > i {
                p = prev_start(b, p);
                u -= units_of(wtf8_decode(b, p).0);
            }
        } else {
            while p < b.len() {
                let (cp, n) = wtf8_decode(b, p);
                if u + units_of(cp) > i {
                    break;
                }
                u += units_of(cp);
                p += n;
            }
        }
        self.set_cursor(u, p);
        (p, u)
    }

    /// The unit index of a code point boundary.
    fn unit_at(&self, p: usize) -> i64 {
        let b = self.as_bytes();
        if self.is_ascii() {
            return p as i64;
        }
        if p >= b.len() {
            return self.utf16_len;
        }
        let cursor = (self.cursor_unit, self.cursor_byte as usize);
        let mut best = (0i64, 0usize);
        for a in [(0, 0), cursor, (self.utf16_len, b.len())] {
            if a.1.abs_diff(p) < best.1.abs_diff(p) {
                best = a;
            }
        }
        let (u, q) = best;
        let n = if q <= p {
            u + b[q..p].iter().map(|&x| weight(x)).sum::<i64>()
        } else {
            u - b[p..q].iter().map(|&x| weight(x)).sum::<i64>()
        };
        self.set_cursor(n, p);
        n
    }

    /// The units `[a, b)` as WTF-8 bytes (`0 <= a, b <= utf16_len`). A cut through a surrogate pair
    /// leaves the lone surrogate, as in JavaScript.
    pub fn sub_units(&self, a: i64, b: i64) -> Vec<u8> {
        if a >= b {
            return Vec::new();
        }
        let bytes = self.as_bytes();
        if self.is_ascii() {
            return bytes[a as usize..b as usize].to_vec();
        }
        let (pa, ua) = self.locate(a);
        let (pb, ub) = self.locate(b);
        let mut out = Vec::with_capacity(pb - pa);
        if pa == pb {
            // Inside one pair: the high half only.
            encode(&mut out, high_unit(wtf8_decode(bytes, pa).0));
            return out;
        }
        let mut start = pa;
        if a > ua {
            let (cp, n) = wtf8_decode(bytes, pa);
            encode(&mut out, low_unit(cp));
            start = pa + n;
        }
        out.extend_from_slice(&bytes[start..pb]);
        if b > ub {
            encode(&mut out, high_unit(wtf8_decode(bytes, pb).0));
        }
        out
    }

    /// The unit at `i`, or None out of range.
    fn char_code_at(&self, i: f64) -> Option<u32> {
        let i = to_int(i);
        if i < 0.0 || i >= self.utf16_len as f64 {
            return None;
        }
        let i = i as i64;
        if self.is_ascii() {
            return Some(self.as_bytes()[i as usize] as u32);
        }
        let (p, u) = self.locate(i);
        let cp = wtf8_decode(self.as_bytes(), p).0;
        Some(if cp < 0x10000 {
            cp
        } else if i == u {
            high_unit(cp)
        } else {
            low_unit(cp)
        })
    }

    fn code_point_at(&self, i: f64) -> Option<u32> {
        let i = to_int(i);
        if i < 0.0 || i >= self.utf16_len as f64 {
            return None;
        }
        let i = i as i64;
        if self.is_ascii() {
            return Some(self.as_bytes()[i as usize] as u32);
        }
        let (p, u) = self.locate(i);
        let cp = wtf8_decode(self.as_bytes(), p).0;
        // The low half of a pair is the lone low surrogate.
        Some(if cp >= 0x10000 && i > u {
            low_unit(cp)
        } else {
            cp
        })
    }

    /// Where `needle` first occurs at or after unit `from`, in units.
    fn index_of(&self, needle: &Str, from: f64) -> Option<i64> {
        let from = clamp_index(from, self.utf16_len);
        if needle.byte_len == 0 {
            return Some(from);
        }
        if needle.is_well_formed() {
            let (p, u) = self.locate(from);
            let start = if from > u {
                // `from` is the low half of a pair: a well formed needle cannot start there.
                p + wtf8_decode(self.as_bytes(), p).1
            } else {
                p
            };
            let hay = &self.as_bytes()[start..];
            let at = find_bytes(hay, needle.as_bytes())?;
            return Some(self.unit_at(start + at));
        }
        let (h, n) = (self.to_utf16(), needle.to_utf16());
        (from as usize..=h.len().saturating_sub(n.len()))
            .find(|&k| h.len() >= n.len() && h[k..k + n.len()] == n[..])
            .map(|k| k as i64)
    }

    fn last_index_of(&self, needle: &Str, from: f64) -> Option<i64> {
        let (h, n) = (self.to_utf16(), needle.to_utf16());
        if n.len() > h.len() {
            return None;
        }
        let from = if from.is_nan() {
            f64::INFINITY
        } else {
            to_int(from)
        };
        let max = (h.len() - n.len()) as f64;
        let mut k = from.clamp(0.0, max) as usize;
        loop {
            if h[k..k + n.len()] == n[..] {
                return Some(k as i64);
            }
            if k == 0 {
                return None;
            }
            k -= 1;
        }
    }

    fn starts_with(&self, prefix: &Str, pos: f64) -> bool {
        let at = clamp_index(pos, self.utf16_len);
        if prefix.utf16_len > self.utf16_len - at {
            return false;
        }
        if prefix.byte_len == 0 {
            return true;
        }
        if prefix.is_well_formed() {
            let (p, u) = self.locate(at);
            return u == at && self.as_bytes()[p..].starts_with(prefix.as_bytes());
        }
        let (h, n) = (self.to_utf16(), prefix.to_utf16());
        h[at as usize..].starts_with(&n)
    }

    fn ends_with(&self, suffix: &Str, end: f64) -> bool {
        let end = if end.is_nan() {
            self.utf16_len
        } else {
            clamp_index(end, self.utf16_len)
        };
        let start = end - suffix.utf16_len;
        if start < 0 {
            return false;
        }
        if suffix.byte_len == 0 {
            return true;
        }
        if suffix.is_well_formed() {
            let ((ps, us), (pe, ue)) = (self.locate(start), self.locate(end));
            return us == start && ue == end && &self.as_bytes()[ps..pe] == suffix.as_bytes();
        }
        let (h, n) = (self.to_utf16(), suffix.to_utf16());
        h[start as usize..end as usize] == n[..]
    }
}

fn find_bytes(hay: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.len() > hay.len() {
        return None;
    }
    if needle.len() == 1 {
        return hay.iter().position(|&x| x == needle[0]);
    }
    hay.windows(needle.len()).position(|w| w == needle)
}

/// Appends `b` to `out`; a trailing lone high surrogate of `out` and a leading lone low one of `b`
/// become one pair (the WTF-8 rule that keeps byte equality equal to JavaScript equality).
pub fn append_wtf8(out: &mut Vec<u8>, b: &[u8]) {
    let lone_high =
        out.len() >= 3 && out[out.len() - 3] == 0xED && (0xA0..0xB0).contains(&out[out.len() - 2]);
    let lone_low = b.len() >= 3 && b[0] == 0xED && (0xB0..0xC0).contains(&b[1]);
    if lone_high && lone_low {
        let at = out.len() - 3;
        let hi = wtf8_decode(out, at).0;
        let lo = wtf8_decode(b, 0).0;
        out.truncate(at);
        encode(out, 0x10000 + ((hi - 0xD800) << 10) + (lo - 0xDC00));
        out.extend_from_slice(&b[3..]);
    } else {
        out.extend_from_slice(b);
    }
}

/// `a` and `b` joined.
fn concat_bytes(a: &[u8], b: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(a.len() + b.len());
    out.extend_from_slice(a);
    append_wtf8(&mut out, b);
    out
}

/// Ordering by UTF-16 code unit, as JavaScript's `<`: byte order is the order of code points,
/// which is the same except for a code point above U+FFFF against one in U+D800..U+FFFF.
fn compare_bytes(a: &[u8], b: &[u8]) -> std::cmp::Ordering {
    let n = a.len().min(b.len());
    let Some(k) = (0..n).find(|&k| a[k] != b[k]) else {
        return a.len().cmp(&b.len());
    };
    let mut s = k;
    while s > 0 && a[s] & 0xC0 == 0x80 {
        s -= 1;
    }
    let (ca, cb) = (wtf8_decode(a, s).0, wtf8_decode(b, s).0);
    let high = |c: u32| (0xD800..0x10000).contains(&c);
    if (ca >= 0x10000 && high(cb)) || (cb >= 0x10000 && high(ca)) {
        return units(&a[s..]).cmp(units(&b[s..]));
    }
    ca.cmp(&cb)
}

// ---- allocating ----

/// A string object holding `bytes` (valid WTF-8).
fn alloc_str(h: &mut Heap, bytes: &[u8]) -> *mut Str {
    let (utf16_len, flags) = scan(bytes);
    let p = new_obj(h, HDR_STR, DATA + bytes.len()) as *mut Str;
    // SAFETY: `DATA + len` bytes were allocated (zeroed).
    unsafe {
        (*p).byte_len = bytes.len() as i64;
        (*p).utf16_len = utf16_len;
        (*p).flags = flags;
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), (p as *mut u8).add(DATA), bytes.len());
    }
    p
}

/// For generated code: collects if due, then allocates. `bytes` was copied out of the arguments
/// first, so they need not survive the collection.
fn alloc_gc(bytes: Vec<u8>) -> *mut Str {
    with_heap(|h| {
        maybe_collect(h);
        alloc_str(h, &bytes)
    })
}

fn st<'a>(p: *const Str) -> &'a Str {
    // SAFETY: callers pass live strings (the extern functions' contract).
    unsafe { &*p }
}

fn nan_or(x: Option<u32>) -> f64 {
    x.map_or(f64::NAN, |v| v as f64)
}

// ---- the runtime's C functions ----

/// A new string from WTF-8 bytes (valid UTF-8 from Rust is always valid WTF-8). Never collects.
///
/// # Safety
/// `data` must point to `len` readable bytes of valid WTF-8.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_str_new(data: *const u8, len: i64) -> *mut Str {
    // SAFETY: by contract.
    let bytes = unsafe { std::slice::from_raw_parts(data, len.max(0) as usize) };
    with_heap(|h| alloc_str(h, bytes))
}

/// Like `ts_rt_str_new`, for generated code: may collect first.
///
/// # Safety
/// `data` must point to `len` readable bytes of valid WTF-8 (not inside a script object).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_str_new_gc(data: *const u8, len: i64) -> *mut Str {
    // SAFETY: by contract.
    let bytes = unsafe { std::slice::from_raw_parts(data, len.max(0) as usize) }.to_vec();
    alloc_gc(bytes)
}

/// A new string from UTF-16 code units (`String.fromCharCode`). Never collects.
///
/// # Safety
/// `data` must point to `len` readable units.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_str_from_utf16(data: *const u16, len: i64) -> *mut Str {
    // SAFETY: by contract.
    let u = unsafe { std::slice::from_raw_parts(data, len.max(0) as usize) };
    let bytes = wtf8_from_utf16(u);
    with_heap(|h| alloc_str(h, &bytes))
}

/// `a + b`. Never collects.
///
/// # Safety
/// Both must be live strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_str_concat(a: *const Str, b: *const Str) -> *mut Str {
    let bytes = concat_bytes(st(a).as_bytes(), st(b).as_bytes());
    with_heap(|h| alloc_str(h, &bytes))
}

/// `a + b`, for generated code.
///
/// # Safety
/// Both must be live strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_str_concat_gc(a: *const Str, b: *const Str) -> *mut Str {
    alloc_gc(concat_bytes(st(a).as_bytes(), st(b).as_bytes()))
}

/// `a === b`.
///
/// # Safety
/// Both must be live strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_str_eq(a: *const Str, b: *const Str) -> bool {
    st(a).as_bytes() == st(b).as_bytes()
}

/// -1, 0 or 1: the order JavaScript's `<` gives (UTF-16 code units).
///
/// # Safety
/// Both must be live strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_str_compare(a: *const Str, b: *const Str) -> i32 {
    compare_bytes(st(a).as_bytes(), st(b).as_bytes()) as i32
}

/// `s.length`.
///
/// # Safety
/// `s` must be a live string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_str_length(s: *const Str) -> f64 {
    st(s).utf16_len as f64
}

/// `s.charCodeAt(i)`: NaN out of range.
///
/// # Safety
/// `s` must be a live string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_str_char_code_at(s: *const Str, i: f64) -> f64 {
    nan_or(st(s).char_code_at(i))
}

/// `s.codePointAt(i)`: NaN (undefined) out of range.
///
/// # Safety
/// `s` must be a live string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_str_code_point_at(s: *const Str, i: f64) -> f64 {
    nan_or(st(s).code_point_at(i))
}

/// `s.charAt(i)`: the unit as a string, empty out of range.
///
/// # Safety
/// `s` must be a live string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_str_char_at_gc(s: *const Str, i: f64) -> *mut Str {
    let s = st(s);
    let i = to_int(i);
    let bytes = if i < 0.0 || i >= s.utf16_len as f64 {
        Vec::new()
    } else {
        s.sub_units(i as i64, i as i64 + 1)
    };
    alloc_gc(bytes)
}

/// `s.slice(start, end)`; pass infinity for an `end` that is `undefined`.
///
/// # Safety
/// `s` must be a live string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_str_slice_gc(s: *const Str, start: f64, end: f64) -> *mut Str {
    let s = st(s);
    let a = relative_index(start, s.utf16_len);
    let b = relative_index(end, s.utf16_len);
    alloc_gc(s.sub_units(a, b))
}

/// `s.substring(start, end)`; pass infinity for an `end` that is `undefined`.
///
/// # Safety
/// `s` must be a live string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_str_substring_gc(s: *const Str, start: f64, end: f64) -> *mut Str {
    let s = st(s);
    let a = clamp_index(start, s.utf16_len);
    let b = clamp_index(end, s.utf16_len);
    alloc_gc(s.sub_units(a.min(b), a.max(b)))
}

/// `s.indexOf(needle, from)`: -1 when absent.
///
/// # Safety
/// Both must be live strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_str_index_of(s: *const Str, needle: *const Str, from: f64) -> f64 {
    st(s).index_of(st(needle), from).map_or(-1.0, |i| i as f64)
}

/// `s.lastIndexOf(needle, from)`: -1 when absent; pass NaN for no `from`.
///
/// # Safety
/// Both must be live strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_str_last_index_of(
    s: *const Str,
    needle: *const Str,
    from: f64,
) -> f64 {
    st(s)
        .last_index_of(st(needle), from)
        .map_or(-1.0, |i| i as f64)
}

/// `s.includes(needle, from)`.
///
/// # Safety
/// Both must be live strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_str_includes(s: *const Str, needle: *const Str, from: f64) -> bool {
    st(s).index_of(st(needle), from).is_some()
}

/// `s.startsWith(prefix, pos)`.
///
/// # Safety
/// Both must be live strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_str_starts_with(
    s: *const Str,
    prefix: *const Str,
    pos: f64,
) -> bool {
    st(s).starts_with(st(prefix), pos)
}

/// `s.endsWith(suffix, end)`; pass NaN for no `end`.
///
/// # Safety
/// Both must be live strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_str_ends_with(s: *const Str, suffix: *const Str, end: f64) -> bool {
    st(s).ends_with(st(suffix), end)
}

/// `String.fromCodePoint(cp)`. A value that is not a code point is a fault (a `RangeError`).
#[unsafe(no_mangle)]
pub extern "C" fn ts_rt_str_from_code_point_gc(cp: f64) -> *mut Str {
    assert!(
        cp.fract() == 0.0 && (0.0..=1114111.0).contains(&cp),
        "RangeError: Invalid code point {cp}"
    );
    let mut bytes = Vec::with_capacity(4);
    encode(&mut bytes, cp as u32);
    alloc_gc(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(text: &str) -> *mut Str {
        unsafe { ts_rt_str_new(text.as_ptr(), text.len() as i64) }
    }

    fn u(units: &[u16]) -> *mut Str {
        unsafe { ts_rt_str_from_utf16(units.as_ptr(), units.len() as i64) }
    }

    fn text(p: *const Str) -> String {
        st(p).to_string_lossy()
    }

    // M9-TS-162
    #[test]
    fn length_counts_utf16_units() {
        for (t, n) in [
            ("", 0),
            ("abc", 3),
            ("héllo", 5),
            ("日本語", 3),
            ("😀", 2),
            ("a😀b", 4),
        ] {
            assert_eq!(unsafe { ts_rt_str_length(s(t)) }, n as f64, "{t}");
        }
        assert_eq!(unsafe { ts_rt_str_length(u(&[0xD83D])) }, 1.0);
    }

    // M9-TS-163
    #[test]
    fn units_of_a_pair_are_both_reachable_and_a_lone_surrogate_is_a_value() {
        let p = s("a😀b");
        let at = |i| unsafe { ts_rt_str_char_code_at(p, i) };
        assert_eq!(at(0.0), 97.0);
        assert_eq!(at(1.0), 0xD83D as f64);
        assert_eq!(at(2.0), 0xDE00 as f64);
        assert_eq!(at(3.0), 98.0);
        assert!(at(4.0).is_nan() && at(-1.0).is_nan());
        assert_eq!(unsafe { ts_rt_str_code_point_at(p, 1.0) }, 0x1F600 as f64);
        assert_eq!(unsafe { ts_rt_str_code_point_at(p, 2.0) }, 0xDE00 as f64);
    }

    // M9-TS-164
    #[test]
    fn slicing_through_a_pair_leaves_lone_surrogates_that_rejoin_on_concat() {
        let p = s("a😀b");
        let head = unsafe { ts_rt_str_slice_gc(p, 0.0, 2.0) };
        let tail = unsafe { ts_rt_str_slice_gc(p, 2.0, f64::INFINITY) };
        assert_eq!(st(head).to_utf16(), [97, 0xD83D]);
        assert_eq!(st(tail).to_utf16(), [0xDE00, 98]);
        assert!(!st(head).is_well_formed());
        let both = unsafe { ts_rt_str_concat(head, tail) };
        assert!(unsafe { ts_rt_str_eq(both, p) }, "{}", text(both));
        assert!(st(both).is_well_formed());
    }

    // M9-TS-165
    #[test]
    fn order_is_by_utf16_unit_not_code_point() {
        // U+1F600 is D83D DE00 in UTF-16, so it sorts before U+FF21 (a BMP code point above U+D800).
        let (emoji, wide) = (s("\u{1F600}"), s("\u{FF21}"));
        assert_eq!(unsafe { ts_rt_str_compare(emoji, wide) }, -1);
        assert_eq!(unsafe { ts_rt_str_compare(wide, emoji) }, 1);
        assert_eq!(unsafe { ts_rt_str_compare(s("abc"), s("abd")) }, -1);
        assert_eq!(unsafe { ts_rt_str_compare(s("ab"), s("abc")) }, -1);
    }

    // M9-TS-166
    #[test]
    fn search_converts_between_bytes_and_units() {
        let p = s("日本語の本");
        let i = unsafe { ts_rt_str_index_of(p, s("本"), 0.0) };
        assert_eq!(i, 1.0);
        assert_eq!(unsafe { ts_rt_str_index_of(p, s("本"), 2.0) }, 4.0);
        assert_eq!(
            unsafe { ts_rt_str_last_index_of(p, s("本"), f64::NAN) },
            4.0
        );
        assert!(unsafe { ts_rt_str_starts_with(p, s("日本"), 0.0) });
        assert!(unsafe { ts_rt_str_ends_with(p, s("の本"), f64::NAN) });
        // A lone low surrogate is found inside a pair, as in JavaScript.
        let e = s("x😀");
        assert_eq!(unsafe { ts_rt_str_index_of(e, u(&[0xDE00]), 0.0) }, 2.0);
    }

    // M9-TS-167
    #[test]
    fn the_cursor_keeps_a_forward_scan_cheap_and_correct() {
        let big: String = "é日😀".repeat(500);
        let p = s(&big);
        let n = unsafe { ts_rt_str_length(p) } as i64;
        let want: Vec<u16> = big.encode_utf16().collect();
        for i in 0..n {
            assert_eq!(
                unsafe { ts_rt_str_char_code_at(p, i as f64) },
                want[i as usize] as f64
            );
        }
        for i in (0..n).rev().step_by(7) {
            assert_eq!(
                unsafe { ts_rt_str_char_code_at(p, i as f64) },
                want[i as usize] as f64
            );
        }
    }

    // M9-TS-172
    #[test]
    fn a_string_is_one_collectable_object() {
        let before = crate::gc_stats().objects;
        let kept = s(&"kept ".repeat(300)); // larger than a small block
        let h = crate::ts_rt_root_add(kept as *mut u8);
        let _lost = unsafe { ts_rt_str_concat_gc(s("a"), s("😀")) };
        assert_eq!(crate::gc_stats().objects, before + 4); // kept, "a", "😀", the concatenation
        crate::ts_rt_gc_collect();
        assert_eq!(
            crate::gc_stats().objects,
            before + 1,
            "only the rooted string survives"
        );
        assert_eq!(text(kept).len(), 1500);
        crate::ts_rt_root_remove(h);
        crate::ts_rt_gc_collect();
        assert_eq!(crate::gc_stats().objects, before);
    }
}
