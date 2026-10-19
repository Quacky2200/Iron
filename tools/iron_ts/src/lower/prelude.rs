//! TypeScript source compiled in front of a program that mentions errors: the `Error` classes.
//! They are ordinary classes of the program (the collector, `instanceof` and field access treat them
//! like any other), so `throw new TypeError("x")` and `catch (e) { e.message }` need no special
//! runtime support. Line numbers in messages skip the prelude.

/// The standard error classes (`name` is an own field here; `message` is empty by default).
pub const ERRORS: &str = r#"class Error {
  name: string = "Error";
  message: string;
  constructor(message: string = "") {
    this.message = message;
  }
  toString(): string {
    return this.message === "" ? this.name : this.name + ": " + this.message;
  }
}
class EvalError extends Error {
  constructor(message: string = "") {
    super(message);
    this.name = "EvalError";
  }
}
class RangeError extends Error {
  constructor(message: string = "") {
    super(message);
    this.name = "RangeError";
  }
}
class ReferenceError extends Error {
  constructor(message: string = "") {
    super(message);
    this.name = "ReferenceError";
  }
}
class SyntaxError extends Error {
  constructor(message: string = "") {
    super(message);
    this.name = "SyntaxError";
  }
}
class TypeError extends Error {
  constructor(message: string = "") {
    super(message);
    this.name = "TypeError";
  }
}
class URIError extends Error {
  constructor(message: string = "") {
    super(message);
    this.name = "URIError";
  }
}
function __rtThrow(kind: number, message: string): void {
  if (kind === 1) throw new RangeError(message);
  if (kind === 2) throw new TypeError(message);
  if (kind === 3) throw new SyntaxError(message);
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
        || src.contains("JSON."))
        && !src.contains("class Error")
}
