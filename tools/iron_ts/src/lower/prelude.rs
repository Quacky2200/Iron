//! TypeScript source compiled in front of a program that mentions errors: the `Error` classes.
//! They are ordinary classes of the program (the collector, `instanceof` and field access treat them
//! like any other), so `throw new TypeError("x")` and `catch (e) { e.message }` need no special
//! runtime support. Line numbers in messages skip the prelude.

/// The standard error classes (`name` is an own field here; `message` is empty by default).
pub const ERRORS: &str = r#"interface ErrorOptions {
  cause?: Error;
}
class Error {
  name: string = "Error";
  message: string;
  cause: Error | null = null;
  constructor(message: string = "", options: ErrorOptions | null = null) {
    this.message = message;
    if (options !== null) this.cause = options.cause ?? null;
  }
  toString(): string {
    return this.message === "" ? this.name : this.name + ": " + this.message;
  }
}
class EvalError extends Error {
  constructor(message: string = "", options: ErrorOptions | null = null) {
    super(message, options);
    this.name = "EvalError";
  }
}
class RangeError extends Error {
  constructor(message: string = "", options: ErrorOptions | null = null) {
    super(message, options);
    this.name = "RangeError";
  }
}
class ReferenceError extends Error {
  constructor(message: string = "", options: ErrorOptions | null = null) {
    super(message, options);
    this.name = "ReferenceError";
  }
}
class SyntaxError extends Error {
  constructor(message: string = "", options: ErrorOptions | null = null) {
    super(message, options);
    this.name = "SyntaxError";
  }
}
class TypeError extends Error {
  constructor(message: string = "", options: ErrorOptions | null = null) {
    super(message, options);
    this.name = "TypeError";
  }
}
class URIError extends Error {
  constructor(message: string = "", options: ErrorOptions | null = null) {
    super(message, options);
    this.name = "URIError";
  }
}
class AggregateError extends Error {
  errors: Error[] = [];
  constructor(errors: Error[], message: string = "", options: ErrorOptions | null = null) {
    super(message, options);
    this.name = "AggregateError";
    this.errors = errors.slice();
  }
}
function __rtThrow(kind: number, message: string): void {
  if (kind === 1) throw new RangeError(message);
  if (kind === 2) throw new TypeError(message);
  if (kind === 3) throw new SyntaxError(message);
  if (kind === 4) throw new URIError(message);
  throw new Error(message);
}
"#;

/// The regular expression classes: the object of a `RegExp` (a plain object with `lastIndex`) and
/// the result of a match (see `lower/regexp.rs`). The compiler fills them in; there is no
/// constructor here.
pub const REGEXP: &str = r#"class RegExp {
  source: string = "";
  flags: string = "";
  global: boolean = false;
  ignoreCase: boolean = false;
  multiline: boolean = false;
  dotAll: boolean = false;
  unicode: boolean = false;
  unicodeSets: boolean = false;
  sticky: boolean = false;
  hasIndices: boolean = false;
  lastIndex: number = 0;
  id: number = 0;
}
class RegExpMatchArray {
  matches: (string | null)[] = [];
  index: number = 0;
  input: string = "";
  groups: Map<string, string | null> | null = null;
}
"#;

/// The `Date` class: its one field is the time value (see `lower/date.rs`).
pub const DATE: &str = r#"class Date {
  t: number = NaN;
  toString(): string {
    return __dateToString(this.t, true);
  }
}
"#;

/// The `Intl` classes (a collator is an id into the runtime's table; see `lower/intl.rs`).
pub const INTL_COLLATOR: &str = r#"class Intl_Collator {
  id: number = 0;
  compare: (a: string, b: string) => number = (a: string, b: string): number => __collatorCompare(this.id, a, b);
}
"#;
pub const INTL_PLURAL: &str = r#"class Intl_PluralRules {
  id: number = 0;
  select: (n: number) => string = (n: number): string => __pluralSelect(this.id, n);
}
"#;
pub const INTL_LIST: &str = r#"class Intl_ListFormat {
  id: number = 0;
  format: (items: string[]) => string = (items: string[]): string => __listFormat(this.id, items);
}
"#;
pub const INTL_RTF: &str = r#"class Intl_RelativeTimeFormat {
  id: number = 0;
  format: (value: number, unit: string) => string = (value: number, unit: string): string => __rtfFormat(this.id, value, unit);
}
"#;
pub const INTL_PARTS: &str = r#"class Intl_Part {
  type: string;
  value: string;
  constructor(type: string, value: string) {
    this.type = type;
    this.value = value;
  }
}
class Intl_Resolved {
  locale: string = "";
  calendar: string | null = null;
  numberingSystem: string | null = null;
  timeZone: string | null = null;
  style: string | null = null;
  currency: string | null = null;
  notation: string | null = null;
  signDisplay: string | null = null;
  minimumIntegerDigits: number = 0;
  minimumFractionDigits: number = 0;
  maximumFractionDigits: number = 0;
  minimumSignificantDigits: number = 0;
  maximumSignificantDigits: number = 0;
}
function __resolved(s: string): Intl_Resolved {
  const r = new Intl_Resolved();
  for (const kv of s.split(";")) {
    const cut = kv.indexOf("=");
    if (cut < 0) continue;
    const k = kv.slice(0, cut);
    const v = kv.slice(cut + 1);
    if (k === "locale") r.locale = v;
    else if (k === "calendar") r.calendar = v;
    else if (k === "numberingSystem") r.numberingSystem = v;
    else if (k === "timeZone") r.timeZone = v;
    else if (k === "style") r.style = v;
    else if (k === "currency") r.currency = v;
    else if (k === "notation") r.notation = v;
    else if (k === "signDisplay") r.signDisplay = v;
    else if (k === "minimumIntegerDigits") r.minimumIntegerDigits = Number(v);
    else if (k === "minimumFractionDigits") r.minimumFractionDigits = Number(v);
    else if (k === "maximumFractionDigits") r.maximumFractionDigits = Number(v);
    else if (k === "minimumSignificantDigits") r.minimumSignificantDigits = Number(v);
    else if (k === "maximumSignificantDigits") r.maximumSignificantDigits = Number(v);
  }
  return r;
}
type Intl_NumberFormatPart = Intl_Part;
type Intl_DateTimeFormatPart = Intl_Part;
function __decodeParts(s: string): Intl_Part[] {
  const out: Intl_Part[] = [];
  if (s.length === 0) return out;
  for (const rec of s.split("\u0001")) {
    const cut = rec.indexOf("\u0002");
    out.push(new Intl_Part(rec.slice(0, cut), rec.slice(cut + 1)));
  }
  return out;
}
"#;
pub const INTL_NUMBER: &str = r#"class Intl_NumberFormat {
  id: number = 0;
  format: (n: number) => string = (n: number): string => __numberFormat(this.id, n);
  formatToParts: (n: number) => Intl_Part[] = (n: number): Intl_Part[] => __decodeParts(__numberFormatParts(this.id, n));
  resolvedOptions: () => Intl_Resolved = (): Intl_Resolved => __resolved(__intlResolved(this.id, 1));
}
"#;
pub const INTL_DATETIME: &str = r#"class Intl_DateTimeFormat {
  id: number = 0;
  format: (d: Date) => string = (d: Date): string => __dtfFormat(this.id, d.t);
  formatToParts: (d: Date) => Intl_Part[] = (d: Date): Intl_Part[] => __decodeParts(__dtfParts(this.id, d.t));
  resolvedOptions: () => Intl_Resolved = (): Intl_Resolved => __resolved(__intlResolved(this.id, 0));
}
"#;

pub const INTL_DISPLAYNAMES: &str = r#"class Intl_DisplayNames {
  id: number = 0;
  of: (code: string) => string = (code: string): string => __displayNameOf(this.id, code);
}
"#;
pub const INTL_CANON: &str = r#"function __canonLocalesN(tags: string[]): string[] {
  const out: string[] = [];
  for (const t of tags) {
    const c = __localeCanon(t);
    if (!out.includes(c)) out.push(c);
  }
  return out;
}
function __canonLocales1(tag: string): string[] {
  return [__localeCanon(tag)];
}
"#;
pub const INTL_LOCALE: &str = r#"class Intl_Locale {
  tag: string = "";
  toString(): string { return this.tag; }
  get baseName(): string { return __localeOp(this.tag, 4); }
  get language(): string { return __localeOp(this.tag, 1); }
  get script(): string | null { return __localePart(this.tag, 2); }
  get region(): string | null { return __localePart(this.tag, 3); }
  get calendar(): string | null { return __localePart(this.tag, 7); }
  get numberingSystem(): string | null { return __localePart(this.tag, 8); }
  get hourCycle(): string | null { return __localePart(this.tag, 9); }
  get collation(): string | null { return __localePart(this.tag, 10); }
  get caseFirst(): string | null { return __localePart(this.tag, 11); }
  get numeric(): boolean { return __localeOp(this.tag, 12) === "1"; }
  maximize(): Intl_Locale { return __newLocale(__localeOp(this.tag, 5)); }
  minimize(): Intl_Locale { return __newLocale(__localeOp(this.tag, 6)); }
}
function __newLocale(tag: string): Intl_Locale {
  const l = new Intl_Locale();
  l.tag = tag;
  return l;
}
function __localePart(tag: string, op: number): string | null {
  const v = __localeOp(tag, op);
  if (v === "") return null;
  return v;
}
"#;
pub const INTL_SEGMENTER: &str = r#"class Intl_SegmentData {
  segment: string;
  index: number;
  input: string;
  isWordLike: boolean;
  constructor(segment: string, index: number, input: string, isWordLike: boolean) {
    this.segment = segment;
    this.index = index;
    this.input = input;
    this.isWordLike = isWordLike;
  }
}
interface Intl_SegResult { done: boolean; value: Intl_SegmentData; }
class Intl_SegIter {
  items: Intl_SegmentData[];
  at: number = 0;
  constructor(items: Intl_SegmentData[]) { this.items = items; }
  next(): Intl_SegResult {
    if (this.at >= this.items.length) {
      const end: Intl_SegResult = { done: true, value: new Intl_SegmentData("", 0, "", false) };
      return end;
    }
    const r: Intl_SegResult = { done: false, value: this.items[this.at] };
    this.at += 1;
    return r;
  }
}
class Intl_Segments {
  items: Intl_SegmentData[];
  constructor(items: Intl_SegmentData[]) { this.items = items; }
  [Symbol.iterator](): Intl_SegIter { return new Intl_SegIter(this.items); }
  containing(index: number = 0): Intl_SegmentData | null {
    for (const s of this.items) {
      if (index >= s.index && index < s.index + s.segment.length) return s;
    }
    return null;
  }
}
class Intl_Segmenter {
  id: number = 0;
  segment(input: string): Intl_Segments {
    const b = __segments(this.id, input);
    const items: Intl_SegmentData[] = [];
    for (let i = 0; i + 2 < b.length; i += 3) {
      items.push(new Intl_SegmentData(input.substring(b[i], b[i + 1]), b[i], input, b[i + 2] === 1));
    }
    return new Intl_Segments(items);
  }
}
"#;

/// Does the program use `Intl` or a locale-aware string method (they raise `RangeError`s)?
pub fn wants_intl(src: &str) -> bool {
    src.contains("Intl.")
        || src.contains(".localeCompare(")
        || src.contains(".normalize(")
        || src.contains(".toLocaleUpperCase(")
        || src.contains(".toLocaleLowerCase(")
}

/// The handle class of a generator's coroutine (an opaque runtime object).
pub const COROUTINE: &str = "class Coroutine {}\nclass __GenReturn extends Error {}\n";

/// `ArrayBuffer` and `DataView` over a `Uint8Array` that owns the bytes (typed arrays made from a
/// buffer are views of it, `iron_ts_rt/src/typed.rs`).
pub const ARRAYBUFFER: &str = r#"class ArrayBuffer {
  bytes: Uint8Array;
  constructor(byteLength: number) { this.bytes = new Uint8Array(byteLength); }
  get byteLength(): number { return this.bytes.length; }
  slice(begin: number = 0, end: number = 2147483647): ArrayBuffer { return __wrapBuffer(this.bytes.slice(begin, end)); }
}
function __wrapBuffer(b: Uint8Array): ArrayBuffer {
  const r = new ArrayBuffer(0);
  r.bytes = b;
  return r;
}
class DataView {
  buf: ArrayBuffer;
  bytes: Uint8Array;
  constructor(buffer: ArrayBuffer, byteOffset: number = 0, byteLength: number = -1) {
    this.buf = buffer;
    if (byteLength < 0) this.bytes = new Uint8Array(buffer, byteOffset);
    else this.bytes = new Uint8Array(buffer, byteOffset, byteLength);
  }
  get buffer(): ArrayBuffer { return this.buf; }
  get byteLength(): number { return this.bytes.length; }
  get byteOffset(): number { return this.bytes.byteOffset; }
  getInt8(o: number): number { return __dvGet(this.bytes, o, 0, true); }
  setInt8(o: number, v: number): void { __dvSet(this.bytes, o, 0, true, v); }
  getUint8(o: number): number { return __dvGet(this.bytes, o, 1, true); }
  setUint8(o: number, v: number): void { __dvSet(this.bytes, o, 1, true, v); }
  getInt16(o: number, little: boolean = false): number { return __dvGet(this.bytes, o, 3, little); }
  setInt16(o: number, v: number, little: boolean = false): void { __dvSet(this.bytes, o, 3, little, v); }
  getUint16(o: number, little: boolean = false): number { return __dvGet(this.bytes, o, 4, little); }
  setUint16(o: number, v: number, little: boolean = false): void { __dvSet(this.bytes, o, 4, little, v); }
  getInt32(o: number, little: boolean = false): number { return __dvGet(this.bytes, o, 5, little); }
  setInt32(o: number, v: number, little: boolean = false): void { __dvSet(this.bytes, o, 5, little, v); }
  getUint32(o: number, little: boolean = false): number { return __dvGet(this.bytes, o, 6, little); }
  setUint32(o: number, v: number, little: boolean = false): void { __dvSet(this.bytes, o, 6, little, v); }
  getFloat32(o: number, little: boolean = false): number { return __dvGet(this.bytes, o, 7, little); }
  setFloat32(o: number, v: number, little: boolean = false): void { __dvSet(this.bytes, o, 7, little, v); }
  getFloat64(o: number, little: boolean = false): number { return __dvGet(this.bytes, o, 8, little); }
  setFloat64(o: number, v: number, little: boolean = false): void { __dvSet(this.bytes, o, 8, little, v); }
}
"#;

/// Does the program use buffers?
pub fn wants_arraybuffer(src: &str) -> bool {
    (src.contains("ArrayBuffer") || src.contains("DataView") || src.contains(".buffer"))
        && !src.contains("class ArrayBuffer")
}

/// `WeakRef<T>` for every class of the program: it holds its target strongly (a collector may keep
/// a weakly held object as long as it likes), so `deref()` always finds it.
pub fn weak_refs(src: &str) -> String {
    if !src.contains("WeakRef") || src.contains("class WeakRef") {
        return String::new();
    }
    let mut out = String::new();
    let mut rest = src;
    while let Some(i) = rest.find("class ") {
        rest = &rest[i + 6..];
        let name: String = rest
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect();
        if name.is_empty() || name.contains('$') {
            continue;
        }
        out += &format!(
            "class WeakRef${name} {{ target: {name}; constructor(t: {name}) {{ this.target = t; }} deref(): {name} | null {{ return this.target; }} }}\n\
function __newWeakRef${name}(t: {name}): WeakRef${name} {{ return new WeakRef${name}(t); }}\n"
        );
    }
    out
}

/// `setTimeout` and friends over the runtime's timers (`iron_ts_rt/src/event.rs`).
pub const TIMERS: &str = r#"function queueMicrotask(callback: () => void): void { __queueMicrotask(callback); }
function setTimeout(callback: () => void, ms: number = 0): number { return __timerAdd(callback, ms, false); }
function setInterval(callback: () => void, ms: number = 0): number { return __timerAdd(callback, ms, true); }
function clearTimeout(id: number): void { __timerClear(id); }
function clearInterval(id: number): void { __timerClear(id); }
"#;

/// Does the program use timers or microtasks (and not define them itself)?
pub fn wants_timers(src: &str) -> bool {
    (src.contains("setTimeout(")
        || src.contains("setInterval(")
        || src.contains("queueMicrotask(")
        || src.contains("clearTimeout(")
        || src.contains("clearInterval("))
        && !src.contains("function setTimeout")
}

/// `Symbol()` values: objects compared by identity. `description` is null when none was given.
pub const SYMBOL: &str = r#"class Symbol_ {
    description: string | null;
    key: string | null;
    constructor(description: string | null, key: string | null) {
        this.description = description;
        this.key = key;
    }
    toString(): string {
        const d = this.description;
        if (d === null) {
            return "Symbol()";
        }
        return "Symbol(" + d + ")";
    }
}
const __symbolRegistry: Symbol_[] = [];
function __Symbol(description: string | null = null): Symbol_ {
    return new Symbol_(description, null);
}
function __symbolFor(key: string): Symbol_ {
    for (const s of __symbolRegistry) {
        if (s.key === key) {
            return s;
        }
    }
    const made = new Symbol_(key, key);
    __symbolRegistry.push(made);
    return made;
}
function __symbolKeyFor(s: Symbol_): string | null {
    return s.key;
}
"#;

/// Does the program use symbol values?
pub fn wants_symbol(src: &str) -> bool {
    src.contains("Symbol(")
        || src.contains("Symbol.for(")
        || src.contains("Symbol.keyFor(")
        || src.contains(": symbol")
        || src.contains("symbol>")
        || src.contains("symbol[")
}

/// Does the program mention `Date` (and not define its own)?
pub fn wants_date(src: &str) -> bool {
    src.contains("Date") && !src.contains("class Date")
}

/// Does the program use regular expressions (a literal, `RegExp`, or a string method that takes
/// one)? A guess is fine: the classes are small.
pub fn wants_regexp(src: &str) -> bool {
    src.contains("RegExp")
        || [
            ".test(",
            ".exec(",
            ".match(",
            ".matchAll(",
            ".replace(",
            ".replaceAll(",
            ".search(",
            ".split(",
        ]
        .iter()
        .any(|m| src.contains(m))
}

/// Is there a literal like `10n` (a digit, then `n`, then a character that cannot continue a name)?
fn has_bigint_literal(src: &str) -> bool {
    let b = src.as_bytes();
    b.windows(3).any(|w| {
        w[0].is_ascii_digit() && w[1] == b'n' && !(w[2].is_ascii_alphanumeric() || w[2] == b'_')
    })
}

/// Does the program use errors (and not define its own `Error`)?
pub fn wanted(src: &str) -> bool {
    (src.contains("Error")
        || src.contains("throw")
        || src.contains("catch")
        || src.contains("finally")
        // The runtime functions that can throw a RangeError.
        || src.contains(".repeat(")
        || src.contains(".padStart(")
        || src.contains(".padEnd(")
        || src.contains(".toFixed(")
        || src.contains("fromCodePoint")
        || src.contains(".toString(")
        || src.contains("JSON.")
        || src.contains("URI")
        || src.contains("BigInt")
        || src.contains("bigint")
        || has_bigint_literal(src)
        || src.contains("unescape("))
        && !src.contains("class Error")
}
