// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

//! `encodeURI`, `encodeURIComponent`, `decodeURI`, `decodeURIComponent`, `escape` and `unescape`.
//! The errors are `URIError`s with JavaScriptCore's texts (kind 4 of `__rtThrow`).

use super::*;

const UNRESERVED_MARKS: &str = "-_.!~*'()";
const RESERVED: &str = ";/?:@&=+$,#";

fn new_bytes(b: Vec<u8>) -> *mut Str {
    // SAFETY: valid WTF-8.
    unsafe { ts_rt_str_new_gc(b.as_ptr(), b.len() as i64) }
}

fn fail(message: &str) -> *mut Str {
    raise(4.0, message);
    new_bytes(Vec::new())
}

/// `encodeURIComponent(s)` (`component`) and `encodeURI(s)`.
///
/// # Safety
/// `s` must be a live string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_uri_encode_gc(s: *const Str, component: bool) -> *mut Str {
    // SAFETY: by contract.
    let units = unsafe { (*s).to_utf16() };
    let mut out = String::with_capacity(units.len());
    for r in char::decode_utf16(units.iter().copied()) {
        let Ok(c) = r else {
            return fail("String contained an illegal UTF-16 sequence.");
        };
        if c.is_ascii_alphanumeric()
            || UNRESERVED_MARKS.contains(c)
            || (!component && RESERVED.contains(c))
        {
            out.push(c);
        } else {
            let mut buf = [0u8; 4];
            for b in c.encode_utf8(&mut buf).bytes() {
                out.push_str(&format!("%{b:02X}"));
            }
        }
    }
    new_bytes(out.into_bytes())
}

fn hex(b: u8) -> Option<u8> {
    (b as char).to_digit(16).map(|d| d as u8)
}

/// `decodeURIComponent(s)` (`component`) and `decodeURI(s)`, which leaves escapes of reserved
/// characters alone.
///
/// # Safety
/// `s` must be a live string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_uri_decode_gc(s: *const Str, component: bool) -> *mut Str {
    // SAFETY: by contract.
    let units = unsafe { (*s).to_utf16() };
    let mut out: Vec<u16> = Vec::with_capacity(units.len());
    let mut i = 0;
    let byte_at = |i: usize| -> Option<u8> {
        if units.get(i) != Some(&(b'%' as u16)) {
            return None;
        }
        let (h, l) = (units.get(i + 1)?, units.get(i + 2)?);
        let (h, l) = (u8::try_from(*h).ok()?, u8::try_from(*l).ok()?);
        Some(hex(h)? * 16 + hex(l)?)
    };
    while i < units.len() {
        if units[i] != b'%' as u16 {
            out.push(units[i]);
            i += 1;
            continue;
        }
        let Some(first) = byte_at(i) else {
            return fail("URI error");
        };
        if first < 0x80 {
            if !component && RESERVED.contains(first as char) {
                out.extend_from_slice(&units[i..i + 3]);
            } else {
                out.push(u16::from(first));
            }
            i += 3;
            continue;
        }
        let len = match first {
            0xC2..=0xDF => 2,
            0xE0..=0xEF => 3,
            0xF0..=0xF4 => 4,
            _ => return fail("URI error"),
        };
        let mut bytes = vec![first];
        for k in 1..len {
            match byte_at(i + 3 * k) {
                Some(b) if b & 0xC0 == 0x80 => bytes.push(b),
                _ => return fail("URI error"),
            }
        }
        // from_utf8 rejects overlong forms, surrogates and values past U+10FFFF.
        let Ok(text) = std::str::from_utf8(&bytes) else {
            return fail("URI error");
        };
        out.extend(text.encode_utf16());
        i += 3 * len;
    }
    new_bytes(wtf8_from_utf16(&out))
}

/// The legacy `escape(s)`.
///
/// # Safety
/// `s` must be a live string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_escape_gc(s: *const Str) -> *mut Str {
    // SAFETY: by contract.
    let units = unsafe { (*s).to_utf16() };
    let mut out = String::new();
    for u in units {
        match u {
            0..=0x7F
                if (u as u8 as char).is_ascii_alphanumeric()
                    || "@*_+-./".contains(u as u8 as char) =>
            {
                out.push(u as u8 as char)
            }
            0..=0xFF => out.push_str(&format!("%{u:02X}")),
            _ => out.push_str(&format!("%u{u:04X}")),
        }
    }
    new_bytes(out.into_bytes())
}

/// The legacy `unescape(s)`.
///
/// # Safety
/// `s` must be a live string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_unescape_gc(s: *const Str) -> *mut Str {
    // SAFETY: by contract.
    let units = unsafe { (*s).to_utf16() };
    let digit = |u: Option<&u16>| u.and_then(|u| u8::try_from(*u).ok()).and_then(hex);
    let mut out: Vec<u16> = Vec::with_capacity(units.len());
    let mut i = 0;
    while i < units.len() {
        if units[i] == b'%' as u16 {
            if units.get(i + 1) == Some(&(b'u' as u16)) {
                let ds: Vec<Option<u8>> = (2..6).map(|k| digit(units.get(i + k))).collect();
                if ds.iter().all(Option::is_some) {
                    let v = ds
                        .iter()
                        .fold(0u16, |a, d| a * 16 + u16::from(d.unwrap_or(0)));
                    out.push(v);
                    i += 6;
                    continue;
                }
            } else if let (Some(h), Some(l)) = (digit(units.get(i + 1)), digit(units.get(i + 2))) {
                out.push(u16::from(h * 16 + l));
                i += 3;
                continue;
            }
        }
        out.push(units[i]);
        i += 1;
    }
    new_bytes(wtf8_from_utf16(&out))
}
