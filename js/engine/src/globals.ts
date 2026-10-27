// SPDX-License-Identifier: Apache-2.0

// Globals provided by the Platypus script runtime (crates/platypus_script).
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
  function __platypus_op(name: string, ...args: unknown[]): void;

  /**
   * Sends one UI commit (a list of `[opcode, ...args]` ops) to the engine, for the
   * screen's UI tree or (with `target`) a world panel's.
   */
  function __platypus_commit(ops: unknown[][], target?: number): void;

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

  // Installed by @platypus/engine, called by Rust.
  var __platypus_registered: () => string[];
  var __platypus_create: (className: string, id: number, objectId?: number, objectName?: string) => void;
  var __platypus_dispatch: (id: number, method: string, ...args: unknown[]) => unknown;
  var __platypus_run: (method: string, ids: number[], ...args: unknown[]) => void;
  var __platypus_run_at: number;
  var __platypus_destroy: (id: number) => void;
  var __platypus_hooks: (id: number) => string[];
  // Once per fixed tick, before components update: advances `Time`, returns the scaled dt.
  var __platypus_tick: (dt: number) => number;
  // Each frame after every onUpdate and before onLateUpdate: animation players (animation.ts).
  var __platypus_animate: (dt: number) => void;
  // Asset files that changed on disk, by game path (assets.ts).
  var __platypus_assets_changed: (paths: string[]) => void;
  var __platypus_input: (type: string, ...args: unknown[]) => void;
  // After each fixed tick: moved bodies, contacts, raycast answers, characters (physics.ts).
  // The screen size in UI pixels and the scale (screen.ts).
  var __platypus_screen: (width: number, height: number, scale: number) => void;
  var __platypus_render_stats: (...counts: number[]) => void;
  // Each camera's exposure read back: EV100 in use and how bright it metered (rendering.ts).
  var __platypus_exposure: (id: number, ev: number, scene: number | null) => void;
  // Objects the engine moved (scene.ts).
  var __platypus_scene: (moved: number[]) => void;
  var __platypus_flush: () => void;
  // Playing sounds that finished (audio.ts).
  var __platypus_audio: (ended: number[]) => void;
  // Textures the engine loaded: [id, width, height], 0 × 0 when it failed (textures.ts).
  // Particle systems whose count or state changed: [id, count, alive 0/1, ...] (particles.ts).
  var __platypus_particles: (changed: number[]) => void;
  // Models the engine loaded: [id, description JSON or null, ...] (models.ts).
  var __platypus_models: (loaded: unknown[]) => void;
  var __platypus_textures: (loaded: [number, number, number][]) => void;
  var __platypus_physics: (moved: number[], contacts: number[], rays: number[], characters: number[]) => void;

  // Installed by @platypus/react, called by Rust: a UI event for the element path
  // (target first). Returns whether the default action was prevented.
  var __platypus_ui_event: ((type: string, path: number[], data: number[], strings?: string[]) => boolean) | undefined;
}

export {};
