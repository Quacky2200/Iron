// Globals provided by the Iron script runtime (crates/iron_script).
// Hand-written for M1; to be generated from Rust later.

declare global {
  interface Console {
    log(...args: unknown[]): void;
    info(...args: unknown[]): void;
    debug(...args: unknown[]): void;
    warn(...args: unknown[]): void;
    error(...args: unknown[]): void;
  }
  var console: Console;

  /** Queues an op for the engine; drained by Rust once per frame. */
  function __iron_op(name: string, ...args: unknown[]): void;

  // Installed by @iron/engine, called by Rust.
  var __iron_registered: () => string[];
  var __iron_create: (className: string, id: number) => void;
  var __iron_dispatch: (id: number, method: string, ...args: unknown[]) => unknown;
  var __iron_destroy: (id: number) => void;
}

export {};
