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

  /**
   * Sends one UI commit (a list of `[opcode, ...args]` ops) to the engine, for the
   * screen's UI tree or (with `target`) a world panel's.
   */
  function __iron_commit(ops: unknown[][], target?: number): void;

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
  var __iron_create: (className: string, id: number, objectId?: number, objectName?: string) => void;
  var __iron_dispatch: (id: number, method: string, ...args: unknown[]) => unknown;
  var __iron_destroy: (id: number) => void;
  var __iron_hooks: (id: number) => string[];
  // Once per fixed tick, before components update: advances `Time`, returns the scaled dt.
  var __iron_tick: (dt: number) => number;
  var __iron_input: (type: string, ...args: unknown[]) => void;
  // After each fixed tick: moved bodies, contacts, raycast answers, characters (physics.ts).
  // The screen size in UI pixels and the scale (screen.ts).
  var __iron_screen: (width: number, height: number, scale: number) => void;
  // Objects the engine moved (scene.ts).
  var __iron_scene: (moved: number[]) => void;
  // Playing sounds that finished (audio.ts).
  var __iron_audio: (ended: number[]) => void;
  // Textures the engine loaded: [id, width, height], 0 × 0 when it failed (textures.ts).
  // Particle systems whose count or state changed: [id, count, alive 0/1, ...] (particles.ts).
  var __iron_particles: (changed: number[]) => void;
  // Models the engine loaded: [id, description JSON or null, ...] (models.ts).
  var __iron_models: (loaded: unknown[]) => void;
  var __iron_textures: (loaded: [number, number, number][]) => void;
  var __iron_physics: (moved: number[], contacts: number[], rays: number[], characters: number[]) => void;

  // Installed by @iron/react, called by Rust: a UI event for the element path
  // (target first). Returns whether the default action was prevented.
  var __iron_ui_event: ((type: string, path: number[], data: number[], strings?: string[]) => boolean) | undefined;
}

export {};
