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
  throw new Error(message);
}
"#;

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
        || src.contains(".toString("))
        && !src.contains("class Error")
}
