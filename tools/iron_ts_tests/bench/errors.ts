// Exceptions in compiled code: `throw`, `try` / `catch`, error classes, unwinding through calls
// and closures. Every exported function without parameters returns a number or a string;
// `gen_errors_expected.ts` runs this file in bun and `tests/errors.rs` compares.

class ValidationError extends Error {
  field: string;
  constructor(field: string, message: string) {
    super(message);
    this.name = "ValidationError";
    this.field = field;
  }
}

function divide(a: number, b: number): number {
  if (b === 0) throw new RangeError("divide by zero");
  return a / b;
}

function checked(n: number): number {
  if (n < 0) throw new ValidationError("n", "negative: " + n);
  return divide(100, n) + 1;
}

export function basic(): string {
  let out = "";
  try {
    out += checked(4);
  } catch (e) {
    out += "caught " + (e as Error).message;
  }
  try {
    out += "|" + checked(0);
  } catch (e) {
    out += "|" + (e as Error).name + ":" + (e as Error).message;
  }
  try {
    out += "|" + checked(-3);
  } catch (e) {
    out += "|" + (e as Error).toString();
  }
  return out;
}

export function kinds(): string {
  let out = "";
  for (const n of [-1, 0, 5]) {
    try {
      checked(n);
      out += "ok;";
    } catch (e) {
      if (e instanceof ValidationError) out += "validation(" + e.field + ");";
      else if (e instanceof RangeError) out += "range;";
      else out += "other;";
    }
  }
  return out;
}

function deep(depth: number): number {
  const label = "level-" + depth;
  if (depth === 0) throw new Error("bottom");
  return deep(depth - 1) + label.length;
}

export function unwinding(): string {
  try {
    deep(6);
    return "no";
  } catch (e) {
    return (e as Error).message + "!";
  }
}

export function rethrow(): string {
  try {
    try {
      checked(0);
    } catch (e) {
      throw new Error("wrapped: " + (e as Error).message);
    }
    return "no";
  } catch (e) {
    return (e as Error).message;
  }
}

export function inCallback(): string {
  let out = "";
  try {
    const r = [1, 2, 3].map((x) => divide(10, x - 2));
    out += r.length;
  } catch (e) {
    out += "map threw " + (e as Error).name;
  }
  const f = (n: number): number => {
    if (n > 2) throw new TypeError("too big " + n);
    return n * 2;
  };
  try {
    out += "|" + f(1) + f(2) + f(3);
  } catch (e) {
    out += "|" + (e as Error).message;
  }
  return out;
}

export function loops(): number {
  let total = 0;
  for (let i = -2; i < 6; i++) {
    try {
      total += checked(i);
    } catch (e) {
      if (e instanceof RangeError) continue;
      total += 1000;
      if (i < -1) break;
    }
  }
  return total;
}

export function clean(): number {
  let n = 0;
  try {
    n += 1;
  } catch (e) {
    n += 100;
  }
  return n;
}

export function many(): number {
  let caught = 0;
  let text = 0;
  for (let i = 0; i < 200; i++) {
    try {
      if (i % 3 === 0) throw new Error("err-" + i + "-" + "x".repeat(i % 5));
      text += ("ok-" + i).length;
    } catch (e) {
      caught++;
      text += (e as Error).message.length;
    }
  }
  return caught * 100000 + text;
}

export function uncaught(): number {
  checked(-1);
  return 1;
}

let log = "";

function withFinally(n: number): number {
  try {
    if (n === 0) throw new Error("zero");
    if (n === 1) return 10;
    log += "body;";
  } catch (e) {
    log += "catch;";
    return 20;
  } finally {
    log += "finally;";
  }
  return 30;
}

export function finallyOrder(): string {
  log = "";
  const r = [0, 1, 2].map((n) => withFinally(n));
  return r.join(",") + "|" + log;
}

export function finallyLoops(): string {
  let out = "";
  for (let i = 0; i < 5; i++) {
    try {
      if (i === 1) continue;
      if (i === 3) break;
      out += "t" + i;
    } finally {
      out += "f" + i;
    }
  }
  return out;
}

let trail = "";

function nestedReturn(): number {
  try {
    try {
      return 1;
    } finally {
      trail += "inner;";
    }
  } finally {
    trail += "outer;";
  }
}

export function nestedFinally(): string {
  trail = "";
  const r = nestedReturn();
  return r + ":" + trail;
}

export function finallyWhileThrowing(): string {
  let out = "";
  try {
    try {
      throw new Error("a");
    } finally {
      out += "cleanup;";
    }
  } catch (e) {
    out += "caught " + (e as Error).message;
  }
  return out;
}

function overrides(): number {
  try {
    return 1;
  } finally {
    return 2;
  }
}

export function finallyOverrides(): number {
  return overrides();
}

export function labelled(): string {
  let out = "";
  outer: for (let i = 0; i < 3; i++) {
    for (let j = 0; j < 3; j++) {
      try {
        if (j === 1) continue outer;
        out += i + "" + j + ";";
      } finally {
        out += "f;";
      }
    }
  }
  return out;
}

export function runtimeErrors(): string {
  let out = "";
  const neg = -1;
  const big = 101;
  try {
    "x".repeat(neg);
  } catch (e) {
    out += (e as Error).name + ":" + (e as Error).message + ";";
  }
  try {
    (1).toFixed(big);
  } catch (e) {
    out += (e as Error).name + ":" + (e as Error).message + ";";
  }
  try {
    (255).toString(37);
  } catch (e) {
    out += (e as Error).name + ":" + (e as Error).message + ";";
  }
  try {
    String.fromCodePoint(neg);
  } catch (e) {
    out += (e as Error).name + ":" + (e as Error).message + ";";
  }
  try {
    out += "ab".repeat(2) + (7).toFixed(2) + "z".padStart(3, "-");
  } catch (e) {
    out += "bad";
  }
  return out;
}

export function runtimeErrorEscapes(): number {
  return "x".repeat(-1).length;
}

/** A `break` inside an array callback's own loop is not routed through the `finally` around the call. */
export function callbackLoopInFinally(): string {
  let out = "";
  try {
    const r = [1, 2, 3].map((x) => {
      let s = 0;
      for (let i = 0; i < 10; i++) {
        if (i === x) break;
        s += i;
      }
      return s;
    });
    out += r.join(",");
  } finally {
    out += "|done";
  }
  return out;
}
