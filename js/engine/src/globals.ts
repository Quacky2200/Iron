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

  /** Sends one UI commit (a list of `[opcode, ...args]` ops) to the engine. */
  function __iron_commit(ops: unknown[][]): void;

  // Timers run on engine time: they fire during the frame's `tick`.
  function setTimeout<A extends unknown[]>(
    fn: (...args: A) => void,
    delay?: number,
    ...args: A
  ): number;
  function clearTimeout(id: number | undefined): void;
  function queueMicrotask(fn: () => void): void;

  // Set at bundle time with `bun build --define`.
  var process: { env: { NODE_ENV: "development" | "production" } };

  // Installed by @iron/engine, called by Rust.
  var __iron_registered: () => string[];
  var __iron_create: (className: string, id: number) => void;
  var __iron_dispatch: (id: number, method: string, ...args: unknown[]) => unknown;
  var __iron_destroy: (id: number) => void;
}

export {};
