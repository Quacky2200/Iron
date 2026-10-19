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
pub const INTL_NUMBER: &str = r#"class Intl_NumberFormat {
  id: number = 0;
  format: (n: number) => string = (n: number): string => __numberFormat(this.id, n);
}
"#;
pub const INTL_DATETIME: &str = r#"class Intl_DateTimeFormat {
  id: number = 0;
  format: (d: Date) => string = (d: Date): string => __dtfFormat(this.id, d.t);
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
