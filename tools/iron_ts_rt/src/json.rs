//! `JSON.stringify` and `JSON.parse` from the program's types.
//!
//! The program is typed and closed, so the compiler describes the static type of the value in a
//! small byte string (a *descriptor*) and these functions walk it: `stringify` reads a script value
//! of that type, `parse` builds one. The descriptor:
//!
//! ```text
//! desc  := 0 number | 1 boolean | 2 string
//!        | 3 literal-union: count u32, (len u32, text)*        (the module's tag texts)
//!        | 4 array: elements-are-pointers u8, desc
//!        | 5 nullable: desc
//!        | 6 object: n u32, (struct header u64, body offset u32)*   (the class and its subclasses)
//!        | 7 map or set                                      (stringifies as {}; cannot be parsed)
//!        | 8 value class: count u8, struct header u64, (len u8, name)*   (stringifies as [x,y,z];
//!          parses that or {"x":..,"y":..,"z":..}, left-out fields are 0)
//! body  := n u32, field*                                     (offset is from the blob start)
//! field := slot u32, omit-if-null u8, name len u32, name, desc
//! blob  := total length u32, root desc offset u32, ...      (so a blob is passed as one pointer)
//! ```
//!
//! `parse` follows JavaScriptCore's parser for what it accepts and the text of its errors
//! (`SyntaxError: JSON Parse error: ...`). Where the JSON does not have the shape of the type, the
//! error is a `TypeError`: the program is typed, so the data must match.

use super::string::*;
use super::{Array, Heap, maybe_collect, new_array, new_obj, with_heap};

const NULL_NUM_BITS: u64 = 0x7FF8_DEAD_BEEF_0001;

const D_NUM: u8 = 0;
const D_BOOL: u8 = 1;
const D_STR: u8 = 2;
const D_LIT: u8 = 3;
const D_ARR: u8 = 4;
const D_NULLABLE: u8 = 5;
const D_OBJ: u8 = 6;
const D_VALUE: u8 = 8;

/// The whole blob behind a pointer to it (its first word is its length).
///
/// # Safety
/// `p` must point to a blob made by the compiler.
unsafe fn blob_slice<'a>(p: *const u8) -> &'a [u8] {
    // SAFETY: by contract the first four bytes are the length.
    unsafe {
        let n = u32::from_le_bytes(*(p as *const [u8; 4])) as usize;
        std::slice::from_raw_parts(p, n)
    }
}

fn u32_at(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}

fn u64_at(b: &[u8], o: usize) -> u64 {
    u64::from_le_bytes(b[o..o + 8].try_into().unwrap())
}

/// The byte length of the descriptor at `o`.
fn desc_len(b: &[u8], o: usize) -> usize {
    match b[o] {
        D_LIT => {
            let n = u32_at(b, o + 1) as usize;
            let mut p = o + 5;
            for _ in 0..n {
                p += 4 + u32_at(b, p) as usize;
            }
            p - o
        }
        D_ARR => 2 + desc_len(b, o + 2),
        D_NULLABLE => 1 + desc_len(b, o + 1),
        D_OBJ => 5 + 12 * u32_at(b, o + 1) as usize,
        D_VALUE => {
            let mut p = o + 10;
            for _ in 0..b[o + 1] {
                p += 1 + b[p] as usize;
            }
            p - o
        }
        _ => 1,
    }
}

struct Field<'a> {
    slot: usize,
    omit_null: bool,
    name: &'a [u8],
    desc: usize,
}

/// The fields of the object body at `o`.
fn fields(b: &[u8], o: usize) -> Vec<Field<'_>> {
    let n = u32_at(b, o) as usize;
    let mut p = o + 4;
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        let slot = u32_at(b, p) as usize;
        let omit_null = b[p + 4] != 0;
        let len = u32_at(b, p + 5) as usize;
        let name = &b[p + 9..p + 9 + len];
        let desc = p + 9 + len;
        p = desc + desc_len(b, desc);
        out.push(Field {
            slot,
            omit_null,
            name,
            desc,
        });
    }
    out
}

fn is_null(b: &[u8], inner: usize, bits: u64) -> bool {
    if b[inner] == D_NUM {
        bits == NULL_NUM_BITS
    } else {
        bits == 0
    }
}

// ---- stringify ----

struct Out<'a> {
    b: &'a [u8],
    gap: Vec<u8>,
    indent: Vec<u8>,
    s: Vec<u8>,
    bodies: std::collections::HashMap<usize, Vec<Field<'a>>>,
}

fn quote(out: &mut Vec<u8>, bytes: &[u8]) {
    out.push(b'"');
    let mut p = 0;
    while p < bytes.len() {
        let x = bytes[p];
        match x {
            b'"' => out.extend_from_slice(b"\\\""),
            b'\\' => out.extend_from_slice(b"\\\\"),
            8 => out.extend_from_slice(b"\\b"),
            12 => out.extend_from_slice(b"\\f"),
            b'\n' => out.extend_from_slice(b"\\n"),
            b'\r' => out.extend_from_slice(b"\\r"),
            b'\t' => out.extend_from_slice(b"\\t"),
            0..=0x1F => out.extend_from_slice(format!("\\u{:04x}", x).as_bytes()),
            // A lone surrogate (ED A0..BF xx) is written as an escape.
            0xED if bytes.get(p + 1).is_some_and(|&y| y >= 0xA0) => {
                let (cp, n) = wtf8_decode(bytes, p);
                out.extend_from_slice(format!("\\u{:04x}", cp).as_bytes());
                p += n;
                continue;
            }
            _ => out.push(x),
        }
        p += 1;
    }
    out.push(b'"');
}

impl<'a> Out<'a> {
    fn newline(&mut self) {
        if !self.gap.is_empty() {
            self.s.push(b'\n');
            self.s.extend_from_slice(&self.indent);
        }
    }

    fn value(&mut self, d: usize, bits: u64) {
        let b = self.b;
        match b[d] {
            D_NUM => {
                let x = f64::from_bits(bits);
                if x.is_finite() {
                    self.s
                        .extend_from_slice(super::number_to_string(x).as_bytes());
                } else {
                    self.s.extend_from_slice(b"null");
                }
            }
            D_BOOL => self
                .s
                .extend_from_slice(if bits != 0 { b"true" } else { b"false" }),
            D_STR => {
                // SAFETY: a string slot holds a live string.
                let st = unsafe { &*(bits as *const Str) };
                quote(&mut self.s, st.as_bytes());
            }
            D_LIT => {
                let n = u32_at(b, d + 1) as usize;
                let mut p = d + 5;
                for i in 0..n {
                    let len = u32_at(b, p) as usize;
                    if i as u64 == bits {
                        quote(&mut self.s, &b[p + 4..p + 4 + len]);
                        return;
                    }
                    p += 4 + len;
                }
                self.s.extend_from_slice(b"null");
            }
            D_ARR => {
                // SAFETY: an array slot holds a live array.
                let a = unsafe { &*(bits as *const Array) };
                if a.is_empty() {
                    self.s.extend_from_slice(b"[]");
                    return;
                }
                self.s.push(b'[');
                let outer = self.indent.len();
                self.indent.extend_from_slice(&self.gap.clone());
                for i in 0..a.len() {
                    if i > 0 {
                        self.s.push(b',');
                    }
                    self.newline();
                    // SAFETY: in range; slots are 8 bytes.
                    let slot = unsafe { a.get::<u64>(i) };
                    self.value(d + 2, slot);
                }
                self.indent.truncate(outer);
                self.newline();
                self.s.push(b']');
            }
            D_NULLABLE => {
                if is_null(b, d + 1, bits) {
                    self.s.extend_from_slice(b"null");
                } else {
                    self.value(d + 1, bits);
                }
            }
            D_VALUE => {
                // A value class (`Vector3`, `Color`, ...) is its numbers, compact: `[1,2,3]`.
                if bits == 0 {
                    self.s.extend_from_slice(b"null");
                    return;
                }
                self.s.push(b'[');
                for i in 0..b[d + 1] as usize {
                    if i > 0 {
                        self.s.push(b',');
                    }
                    // SAFETY: a value object has `count` number slots after its header.
                    let x = f64::from_bits(unsafe { *((bits as *const u64).add(1 + i)) });
                    if x.is_finite() {
                        self.s
                            .extend_from_slice(super::number_to_string(x).as_bytes());
                    } else {
                        self.s.extend_from_slice(b"null");
                    }
                }
                self.s.push(b']');
            }
            D_OBJ => {
                if bits == 0 {
                    self.s.extend_from_slice(b"null");
                    return;
                }
                // The object's own class (it may be a subclass of the static type).
                // SAFETY: an object slot holds a live object whose first word is its header.
                let header = unsafe { *(bits as *const u64) } & 0xFFFF_FFFF;
                let n = u32_at(b, d + 1) as usize;
                let mut body = None;
                for i in 0..n {
                    if u64_at(b, d + 5 + 12 * i) == header {
                        body = Some(u32_at(b, d + 5 + 12 * i + 8) as usize);
                    }
                }
                let Some(body) = body else {
                    self.s.extend_from_slice(b"{}");
                    return;
                };
                let fs = self
                    .bodies
                    .entry(body)
                    .or_insert_with(|| fields(b, body))
                    .iter()
                    .map(|f| (f.slot, f.omit_null, f.name, f.desc))
                    .collect::<Vec<_>>();
                self.s.push(b'{');
                let outer = self.indent.len();
                self.indent.extend_from_slice(&self.gap.clone());
                let mut any = false;
                for (slot, omit, name, fd) in fs {
                    // SAFETY: the slot is inside the object (`slot` < the field count).
                    let v = unsafe { *((bits as *const u64).add(1 + slot)) };
                    if omit && b[fd] == D_NULLABLE && is_null(b, fd + 1, v) {
                        continue;
                    }
                    if any {
                        self.s.push(b',');
                    }
                    any = true;
                    self.newline();
                    quote(&mut self.s, name);
                    self.s.push(b':');
                    if !self.gap.is_empty() {
                        self.s.push(b' ');
                    }
                    self.value(fd, v);
                }
                self.indent.truncate(outer);
                if any {
                    self.newline();
                }
                self.s.push(b'}');
            }
            _ => self.s.extend_from_slice(b"{}"),
        }
    }
}

/// `JSON.stringify(value, null, space)`: `bits` is the value's raw word, `space_num` the numeric
/// indent (NaN for none) and `space_str` a string indent (null for none).
///
/// # Safety
/// `blob` is the descriptor of the value's type and `bits` a value of it.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_json_stringify_gc(
    blob: *const u8,
    bits: u64,
    space_num: f64,
    space_str: *const Str,
) -> *mut Str {
    // SAFETY: by contract.
    let b = unsafe { blob_slice(blob) };
    let gap: Vec<u8> = if !space_str.is_null() {
        // SAFETY: a live string; the first ten UTF-16 units.
        let s = unsafe { &*space_str };
        let u = s.to_utf16();
        wtf8_from_utf16(&u[..u.len().min(10)])
    } else if space_num.is_finite() {
        vec![b' '; space_num.trunc().clamp(0.0, 10.0) as usize]
    } else {
        Vec::new()
    };
    let mut o = Out {
        b,
        gap,
        indent: Vec::new(),
        s: Vec::new(),
        bodies: Default::default(),
    };
    o.value(u32_at(b, 4) as usize, bits);
    let bytes = o.s;
    with_heap(|h| {
        maybe_collect(h);
        alloc_str_pub(h, &bytes)
    })
}

// ---- parse ----

type PErr = (f64, String);

struct Parser<'a, 'h> {
    b: &'a [u8],
    s: &'a [u8],
    p: usize,
    h: &'h mut Heap,
    /// The first shape mismatch: reported once the whole text is known to be valid JSON (a
    /// syntax error anywhere wins, as in JavaScript where the shape is never checked).
    mismatch: Option<PErr>,
}

const SYNTAX: f64 = 3.0;
const TYPE: f64 = 2.0;

fn syntax(msg: &str) -> PErr {
    (SYNTAX, format!("JSON Parse error: {msg}"))
}

fn type_err(msg: &str) -> PErr {
    (TYPE, format!("JSON.parse: {msg}"))
}

/// Punctuation JavaScriptCore's lexer knows as tokens (`Unexpected token`); anything else that is
/// not a value is `Unrecognized token`.
fn known_punct(c: u8) -> bool {
    matches!(
        c,
        b'{' | b'}' | b'[' | b']' | b':' | b',' | b'.' | b'(' | b')' | b';' | b'='
    )
}

impl Parser<'_, '_> {
    /// Records a shape mismatch and carries on with a placeholder.
    fn soft(&mut self, e: PErr) -> Result<u64, PErr> {
        if self.mismatch.is_none() {
            self.mismatch = Some(e);
        }
        Ok(0)
    }

    fn ws(&mut self) {
        while self.p < self.s.len() && matches!(self.s[self.p], b' ' | b'\t' | b'\n' | b'\r') {
            self.p += 1;
        }
    }

    fn peek(&self) -> Option<u8> {
        self.s.get(self.p).copied()
    }

    /// The error for a character where a value is expected.
    fn bad_start(&self) -> PErr {
        let Some(c) = self.peek() else {
            return syntax("Unexpected EOF");
        };
        if c == b'\'' {
            return syntax("Single quotes (') are not allowed in JSON");
        }
        if c.is_ascii_alphabetic() || c == b'_' || c == b'$' {
            let mut q = self.p;
            while q < self.s.len()
                && (self.s[q].is_ascii_alphanumeric() || self.s[q] == b'_' || self.s[q] == b'$')
            {
                q += 1;
            }
            let word = String::from_utf8_lossy(&self.s[self.p..q]).into_owned();
            return syntax(&format!("Unexpected identifier \"{word}\""));
        }
        let ch = String::from_utf8_lossy(
            &self.s[self.p..self.p + utf8_len(c).min(self.s.len() - self.p)],
        )
        .into_owned();
        if known_punct(c) {
            syntax(&format!("Unexpected token '{ch}'"))
        } else {
            syntax(&format!("Unrecognized token '{ch}'"))
        }
    }

    /// The error for something that is not a string where a property name is expected.
    fn bad_key(&self, first: bool) -> PErr {
        if self.peek() == Some(b'\'') {
            return syntax("Single quotes (') are not allowed in JSON");
        }
        syntax(if first {
            "Expected '}'"
        } else {
            "Property name must be a string literal"
        })
    }

    /// The error for text after the value: an unknown character is an unrecognized token, the rest
    /// is just unparseable.
    fn trailing(&self) -> PErr {
        let c = self.s[self.p];
        if known_punct(c)
            || c.is_ascii_alphanumeric()
            || matches!(c, b'_' | b'$' | b'-' | b'"' | b'\'')
        {
            return syntax("Unable to parse JSON string");
        }
        let n = utf8_len(c).min(self.s.len() - self.p);
        let ch = String::from_utf8_lossy(&self.s[self.p..self.p + n]).into_owned();
        syntax(&format!("Unrecognized token '{ch}'"))
    }

    fn keyword(&mut self, word: &[u8]) -> bool {
        if self.s[self.p..].starts_with(word) {
            self.p += word.len();
            true
        } else {
            false
        }
    }

    fn string(&mut self) -> Result<Vec<u8>, PErr> {
        // At the opening quote.
        self.p += 1;
        let mut out = Vec::new();
        loop {
            let Some(c) = self.peek() else {
                return Err(syntax("Unterminated string"));
            };
            match c {
                b'"' => {
                    self.p += 1;
                    return Ok(out);
                }
                0..=0x1F => return Err(syntax("Unterminated string")),
                b'\\' => {
                    self.p += 1;
                    let Some(e) = self.peek() else {
                        return Err(syntax("Unterminated string"));
                    };
                    self.p += 1;
                    let unit: Option<u16> = match e {
                        b'"' => Some(b'"' as u16),
                        b'\\' => Some(b'\\' as u16),
                        b'/' => Some(b'/' as u16),
                        b'b' => Some(8),
                        b'f' => Some(12),
                        b'n' => Some(10),
                        b'r' => Some(13),
                        b't' => Some(9),
                        b'u' => {
                            let mut v = 0u32;
                            for k in 0..4 {
                                match self.s.get(self.p + k) {
                                    None | Some(b'"') => {
                                        return Err(syntax("\\u must be followed by 4 hex digits"));
                                    }
                                    Some(&h) => match (h as char).to_digit(16) {
                                        Some(d) => v = v * 16 + d,
                                        None => {
                                            let raw = String::from_utf8_lossy(
                                                &self.s[self.p - 2..(self.p + 4).min(self.s.len())],
                                            )
                                            .into_owned();
                                            return Err(syntax(&format!(
                                                "\"{raw}\" is not a valid unicode escape"
                                            )));
                                        }
                                    },
                                }
                            }
                            self.p += 4;
                            Some(v as u16)
                        }
                        _ => None,
                    };
                    let Some(u) = unit else {
                        let ch = String::from_utf8_lossy(&[e]).into_owned();
                        return Err(syntax(&format!("Invalid escape character {ch}")));
                    };
                    append_wtf8(&mut out, &wtf8_from_utf16(&[u]));
                }
                _ => {
                    out.push(c);
                    self.p += 1;
                }
            }
        }
    }

    fn number(&mut self) -> Result<f64, PErr> {
        let start = self.p;
        if self.peek() == Some(b'-') {
            self.p += 1;
        }
        match self.peek() {
            Some(b'0') => self.p += 1,
            Some(b'1'..=b'9') => {
                while self.peek().is_some_and(|c| c.is_ascii_digit()) {
                    self.p += 1;
                }
            }
            _ => return Err(syntax("Invalid number")),
        }
        if self.peek() == Some(b'.') {
            self.p += 1;
            if !self.peek().is_some_and(|c| c.is_ascii_digit()) {
                return Err(syntax("Invalid digits after decimal point"));
            }
            while self.peek().is_some_and(|c| c.is_ascii_digit()) {
                self.p += 1;
            }
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            let mut q = self.p + 1;
            if matches!(self.s.get(q), Some(b'+' | b'-')) {
                q += 1;
            }
            if self.s.get(q).is_some_and(|c| c.is_ascii_digit()) {
                while self.s.get(q).is_some_and(|c| c.is_ascii_digit()) {
                    q += 1;
                }
                self.p = q;
            }
        }
        let text = std::str::from_utf8(&self.s[start..self.p]).unwrap();
        Ok(text.parse::<f64>().unwrap_or(f64::NAN))
    }

    /// Checks the syntax of a value without building it (a field the type does not have).
    fn skip(&mut self) -> Result<(), PErr> {
        self.ws();
        match self.peek() {
            Some(b'{') => {
                self.p += 1;
                self.ws();
                if self.peek() == Some(b'}') {
                    self.p += 1;
                    return Ok(());
                }
                let mut first = true;
                loop {
                    self.ws();
                    if self.peek() != Some(b'"') {
                        return Err(self.bad_key(first));
                    }
                    first = false;
                    self.string()?;
                    self.ws();
                    if self.peek() != Some(b':') {
                        return Err(syntax(
                            "Expected ':' before value in object property definition",
                        ));
                    }
                    self.p += 1;
                    self.skip()?;
                    self.ws();
                    match self.peek() {
                        Some(b',') => self.p += 1,
                        Some(b'}') => {
                            self.p += 1;
                            return Ok(());
                        }
                        _ => return Err(syntax("Expected '}'")),
                    }
                }
            }
            Some(b'[') => {
                self.p += 1;
                self.ws();
                if self.peek() == Some(b']') {
                    self.p += 1;
                    return Ok(());
                }
                loop {
                    self.skip()?;
                    self.ws();
                    match self.peek() {
                        Some(b',') => {
                            self.p += 1;
                            self.ws();
                            if self.peek() == Some(b']') {
                                return Err(syntax(
                                    "Unexpected comma at the end of array expression",
                                ));
                            }
                        }
                        Some(b']') => {
                            self.p += 1;
                            return Ok(());
                        }
                        _ => return Err(syntax("Expected ']'")),
                    }
                }
            }
            Some(b'"') => self.string().map(|_| ()),
            Some(b'-' | b'0'..=b'9') => self.number().map(|_| ()),
            Some(b't') if self.keyword(b"true") => Ok(()),
            Some(b'f') if self.keyword(b"false") => Ok(()),
            Some(b'n') if self.keyword(b"null") => Ok(()),
            _ => Err(self.bad_start()),
        }
    }

    /// A number value (inside a value class's array).
    fn value_num(&mut self) -> Result<f64, PErr> {
        self.ws();
        match self.peek() {
            Some(b'-' | b'0'..=b'9') => self.number(),
            Some(b'"' | b'{' | b'[' | b't' | b'f' | b'n') => {
                self.skip()?;
                self.soft(type_err("expected a number")).map(|_| 0.0)
            }
            _ => Err(self.bad_start()),
        }
    }

    fn value(&mut self, d: usize) -> Result<u64, PErr> {
        self.ws();
        let b = self.b;
        match b[d] {
            D_NULLABLE => {
                if self.s[self.p..].starts_with(b"null") && {
                    self.p += 4;
                    true
                } {
                    return Ok(if b[d + 1] == D_NUM { NULL_NUM_BITS } else { 0 });
                }
                self.value(d + 1)
            }
            D_NUM => match self.peek() {
                Some(b'-' | b'0'..=b'9') => Ok(self.number()?.to_bits()),
                Some(b'"' | b'{' | b'[' | b't' | b'f' | b'n') => {
                    self.skip()?;
                    self.soft(type_err("expected a number"))
                }
                _ => Err(self.bad_start()),
            },
            D_BOOL => {
                if self.keyword(b"true") {
                    Ok(1)
                } else if self.keyword(b"false") {
                    Ok(0)
                } else if self
                    .peek()
                    .is_some_and(|c| matches!(c, b'"' | b'{' | b'[' | b'n' | b'-' | b'0'..=b'9'))
                {
                    self.skip()?;
                    self.soft(type_err("expected a boolean"))
                } else {
                    Err(self.bad_start())
                }
            }
            D_STR | D_LIT => {
                if self.peek() != Some(b'"') {
                    return match self.peek() {
                        Some(b'{' | b'[' | b't' | b'f' | b'n' | b'-' | b'0'..=b'9') => {
                            self.skip()?;
                            self.soft(type_err("expected a string"))
                        }
                        _ => Err(self.bad_start()),
                    };
                }
                let text = self.string()?;
                if b[d] == D_STR {
                    Ok(alloc_str_pub(self.h, &text) as u64)
                } else {
                    let n = u32_at(b, d + 1) as usize;
                    let mut p = d + 5;
                    for i in 0..n {
                        let len = u32_at(b, p) as usize;
                        if b[p + 4..p + 4 + len] == text[..] {
                            return Ok(i as u64);
                        }
                        p += 4 + len;
                    }
                    self.soft(type_err("the string is not one of the allowed values"))
                }
            }
            D_ARR => {
                if self.peek() != Some(b'[') {
                    return match self.peek() {
                        Some(b'{' | b'"' | b't' | b'f' | b'n' | b'-' | b'0'..=b'9') => {
                            self.skip()?;
                            self.soft(type_err("expected an array"))
                        }
                        _ => Err(self.bad_start()),
                    };
                }
                self.p += 1;
                let mut items: Vec<u64> = Vec::new();
                self.ws();
                if self.peek() == Some(b']') {
                    self.p += 1;
                } else {
                    loop {
                        items.push(self.value(d + 2)?);
                        self.ws();
                        match self.peek() {
                            Some(b',') => {
                                self.p += 1;
                                self.ws();
                                if self.peek() == Some(b']') {
                                    return Err(syntax(
                                        "Unexpected comma at the end of array expression",
                                    ));
                                }
                            }
                            Some(b']') => {
                                self.p += 1;
                                break;
                            }
                            _ => return Err(syntax("Expected ']'")),
                        }
                    }
                }
                let a = new_array(self.h, items.len() as i64, b[d + 1] != 0);
                for v in items {
                    // SAFETY: just made; pushing never collects.
                    unsafe { super::ts_rt_arr_push(a, v) };
                }
                Ok(a as u64)
            }
            D_VALUE if self.peek() == Some(b'{') => {
                // The object form: fields by name; unknown keys are skipped, missing ones are 0.
                let count = b[d + 1] as usize;
                let mut names: Vec<&[u8]> = Vec::new();
                let mut p = d + 10;
                for _ in 0..count {
                    let len = b[p] as usize;
                    names.push(&b[p + 1..p + 1 + len]);
                    p += 1 + len;
                }
                let mut nums = vec![0.0f64; count];
                self.p += 1;
                self.ws();
                if self.peek() == Some(b'}') {
                    self.p += 1;
                } else {
                    let mut first = true;
                    loop {
                        self.ws();
                        if self.peek() != Some(b'"') {
                            return Err(self.bad_key(first));
                        }
                        first = false;
                        let key = self.string()?;
                        self.ws();
                        if self.peek() != Some(b':') {
                            return Err(syntax(
                                "Expected ':' before value in object property definition",
                            ));
                        }
                        self.p += 1;
                        match names.iter().position(|n| *n == &key[..]) {
                            Some(i) => nums[i] = self.value_num()?,
                            None => self.skip()?,
                        }
                        self.ws();
                        match self.peek() {
                            Some(b',') => self.p += 1,
                            Some(b'}') => {
                                self.p += 1;
                                break;
                            }
                            _ => return Err(syntax("Expected '}'")),
                        }
                    }
                }
                let o = new_obj(self.h, u64_at(b, d + 2), 8 + 8 * count);
                for (i, x) in nums.iter().enumerate() {
                    // SAFETY: just made, with `count` slots.
                    unsafe { *((o as *mut f64).add(1 + i)) = *x };
                }
                Ok(o as u64)
            }
            D_VALUE => {
                if self.peek() != Some(b'[') {
                    return match self.peek() {
                        Some(b'"' | b't' | b'f' | b'n' | b'-' | b'0'..=b'9') => {
                            self.skip()?;
                            self.soft(type_err("expected an array of numbers"))
                        }
                        _ => Err(self.bad_start()),
                    };
                }
                self.p += 1;
                let count = b[d + 1] as usize;
                let mut nums: Vec<f64> = Vec::new();
                self.ws();
                if self.peek() == Some(b']') {
                    self.p += 1;
                } else {
                    loop {
                        nums.push(self.value_num()?);
                        self.ws();
                        match self.peek() {
                            Some(b',') => self.p += 1,
                            Some(b']') => {
                                self.p += 1;
                                break;
                            }
                            _ => return Err(syntax("Expected ']'")),
                        }
                    }
                }
                if nums.len() != count {
                    return self.soft(type_err("the array has the wrong number of values"));
                }
                let o = new_obj(self.h, u64_at(b, d + 2), 8 + 8 * count);
                for (i, x) in nums.iter().enumerate() {
                    // SAFETY: just made, with `count` slots.
                    unsafe { *((o as *mut f64).add(1 + i)) = *x };
                }
                Ok(o as u64)
            }
            D_OBJ => {
                if self.peek() != Some(b'{') {
                    return match self.peek() {
                        Some(b'[' | b'"' | b't' | b'f' | b'n' | b'-' | b'0'..=b'9') => {
                            self.skip()?;
                            self.soft(type_err("expected an object"))
                        }
                        _ => Err(self.bad_start()),
                    };
                }
                // The static class itself (the first entry); a parsed object is never a subclass.
                let header = u64_at(b, d + 5);
                let body = u32_at(b, d + 5 + 8) as usize;
                let fs = fields(b, body);
                let mut vals: Vec<Option<u64>> = vec![None; fs.len()];
                self.p += 1;
                self.ws();
                if self.peek() == Some(b'}') {
                    self.p += 1;
                } else {
                    let mut first = true;
                    loop {
                        self.ws();
                        if self.peek() != Some(b'"') {
                            return Err(self.bad_key(first));
                        }
                        first = false;
                        let key = self.string()?;
                        self.ws();
                        if self.peek() != Some(b':') {
                            return Err(syntax(
                                "Expected ':' before value in object property definition",
                            ));
                        }
                        self.p += 1;
                        match fs.iter().position(|f| f.name == &key[..]) {
                            Some(i) => vals[i] = Some(self.value(fs[i].desc)?),
                            None => self.skip()?,
                        }
                        self.ws();
                        match self.peek() {
                            Some(b',') => self.p += 1,
                            Some(b'}') => {
                                self.p += 1;
                                break;
                            }
                            _ => return Err(syntax("Expected '}'")),
                        }
                    }
                }
                let obj = new_obj(self.h, header, 8 * (1 + fs.len())) as *mut u64;
                for (i, f) in fs.iter().enumerate() {
                    let v = match vals[i] {
                        Some(v) => v,
                        None if b[f.desc] == D_NULLABLE => {
                            if b[f.desc + 1] == D_NUM {
                                NULL_NUM_BITS
                            } else {
                                0
                            }
                        }
                        None => {
                            let name = String::from_utf8_lossy(f.name).into_owned();
                            return self.soft(type_err(&format!("missing property \"{name}\"")));
                        }
                    };
                    // SAFETY: `slot` is inside the object just allocated.
                    unsafe { *obj.add(1 + f.slot) = v };
                }
                Ok(obj as u64)
            }
            _ => self.soft(type_err("this type cannot be parsed")),
        }
    }
}

fn utf8_len(lead: u8) -> usize {
    if lead < 0x80 {
        1
    } else if lead < 0xE0 {
        2
    } else if lead < 0xF0 {
        3
    } else {
        4
    }
}

/// `JSON.parse(text)` as the type the descriptor describes. On an error the result is 0 and the
/// runtime error is recorded (`ts_rt_error_kind`).
///
/// # Safety
/// `blob` is a descriptor, `text` a live string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_json_parse_gc(blob: *const u8, text: *const Str) -> u64 {
    // SAFETY: by contract.
    let b = unsafe { blob_slice(blob) };
    // SAFETY: a live string; collection happens before any parsing.
    let s = unsafe { &*text };
    let result = with_heap(|h| {
        maybe_collect(h);
        let mut p = Parser {
            b,
            s: s.as_bytes(),
            p: 0,
            h,
            mismatch: None,
        };
        let root = u32_at(b, 4) as usize;
        let v = p.value(root)?;
        p.ws();
        if p.p < p.s.len() {
            return Err(p.trailing());
        }
        match p.mismatch {
            Some(e) => Err(e),
            None => Ok(v),
        }
    });
    match result {
        Ok(v) => v,
        Err((kind, msg)) => {
            super::raise(kind, &msg);
            match b[u32_at(b, 4) as usize] {
                D_NUM => f64::NAN.to_bits(),
                _ => 0,
            }
        }
    }
}
