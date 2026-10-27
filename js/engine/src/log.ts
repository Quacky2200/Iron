// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// Logging the way Unity (`Debug.Log`), s&box (`Log.Info`) and C# (`Console.WriteLine`)
// spell it. Everything goes to the engine's log through `console`, so it shows in the
// terminal (and the editor console later). Values print readably: vectors through
// their `toString`, errors with their stack, other objects as JSON.

/** One value as log text. */
export function formatValue(v: unknown): string {
  if (typeof v === "string") return v;
  if (v instanceof Error) return v.stack ? `${v.message}\n${v.stack}` : v.message;
  if (v === null || v === undefined || typeof v !== "object") return String(v);
  // Classes with their own `toString` (Vector3, TimeSpan, DateTime, ...).
  if (typeof v.toString === "function" && v.toString !== Object.prototype.toString && !Array.isArray(v)) {
    return v.toString();
  }
  const seen = new WeakSet<object>();
  try {
    return JSON.stringify(v, (_k, value: unknown) => {
      if (typeof value === "object" && value !== null) {
        if (seen.has(value)) return "[circular]";
        seen.add(value);
        if (value instanceof Map) return Object.fromEntries(value);
        if (value instanceof Set) return [...value];
      }
      if (typeof value === "bigint") return `${value}n`;
      if (typeof value === "function") return `[function ${value.name || "anonymous"}]`;
      return value;
    });
  } catch {
    return String(v);
  }
}

const join = (args: unknown[]) => args.map(formatValue).join(" ");

/** Unity's `Debug`. */
export const Debug = {
  log(...message: unknown[]): void {
    console.log(join(message));
  },
  logWarning(...message: unknown[]): void {
    console.warn(join(message));
  },
  logError(...message: unknown[]): void {
    console.error(join(message));
  },
  /** Logs an error with its stack. */
  logException(error: unknown): void {
    console.error(formatValue(error instanceof Error ? error : new Error(String(error))));
  },
  /** Logs an error (with where it came from) when `condition` is false. */
  assert(condition: unknown, ...message: unknown[]): void {
    if (condition) return;
    const err = new Error(message.length ? join(message) : "assertion failed");
    console.error(`Assertion failed: ${formatValue(err)}`);
  },
};

/** s&box's `Log`. */
export const Log = {
  info(...message: unknown[]): void {
    console.info(join(message));
  },
  warning(...message: unknown[]): void {
    console.warn(join(message));
  },
  error(...message: unknown[]): void {
    console.error(join(message));
  },
  /** Verbose detail, hidden unless debug logging is on (`RUST_LOG=script=debug`). */
  trace(...message: unknown[]): void {
    console.debug(join(message));
  },
};

/** C#'s `Console`, with the JS names too. */
export const Console = {
  writeLine(...message: unknown[]): void {
    console.log(join(message));
  },
  log(...message: unknown[]): void {
    console.log(join(message));
  },
  warn(...message: unknown[]): void {
    console.warn(join(message));
  },
  warning(...message: unknown[]): void {
    console.warn(join(message));
  },
  error(...message: unknown[]): void {
    console.error(join(message));
  },
};
