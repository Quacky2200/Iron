// Game time, the Unity / s&box way: `Time` (deltaTime, time, frameCount, timeScale),
// s&box's `TimeSince` / `TimeUntil`, and awaitable delays (`Task`).
//
// The engine advances `Time` once per frame (`__iron_tick`, called by Rust before the
// components' `onUpdate`). Times are in seconds as numbers but come from millisecond
// clocks: there's no sub-millisecond (µs/ns) timing; don't use these to profile.

let time = 0;
let unscaledTime = 0;
let deltaTime = 0;
let unscaledDeltaTime = 0;
let frameCount = 0;
let timeScale = 1;
const startedAt = Date.now();
/** Called at the start of each tick, after `Time` moved on (input uses it). */
const tickListeners: (() => void)[] = [];

export const Time = {
  /** Seconds of game time since the start (scaled by `timeScale`). */
  get time(): number {
    return time;
  },
  /** Seconds since the start, ignoring `timeScale`. */
  get unscaledTime(): number {
    return unscaledTime;
  },
  /** This frame's length in seconds, scaled by `timeScale` (what `onUpdate` gets). */
  get deltaTime(): number {
    return deltaTime;
  },
  get unscaledDeltaTime(): number {
    return unscaledDeltaTime;
  },
  /** The fixed tick length (1/60 s): what `onFixedUpdate` gets. */
  get fixedDeltaTime(): number {
    return 1 / 60;
  },
  /** Frames run so far. */
  get frameCount(): number {
    return frameCount;
  },
  /** Speed of game time: 0 pauses `deltaTime`, 0.5 is slow motion. Physics isn't scaled yet. */
  get timeScale(): number {
    return timeScale;
  },
  set timeScale(s: number) {
    timeScale = Number.isFinite(s) && s >= 0 ? s : 1;
  },
  /** Wall-clock seconds since the game started (millisecond resolution). */
  get realtimeSinceStartup(): number {
    return (Date.now() - startedAt) / 1000;
  },
};

/** @internal Runs `fn` at the start of every tick. */
export function onTick(fn: () => void): void {
  tickListeners.push(fn);
}

/** Resets the clock. Only for tests. */
export function _resetTimeForTests() {
  time = unscaledTime = deltaTime = unscaledDeltaTime = frameCount = 0;
  timeScale = 1;
}

globalThis.__iron_tick = (dt: number) => {
  unscaledDeltaTime = dt;
  deltaTime = dt * timeScale;
  unscaledTime += dt;
  time += deltaTime;
  frameCount++;
  for (const fn of tickListeners) fn();
  return deltaTime;
};

/**
 * Seconds since something happened, s&box style:
 * `lastShot = TimeSince.now(); ... if (lastShot.seconds > 0.5) fire();`
 */
export class TimeSince {
  private constructor(private readonly at: number) {}
  /** Starting now. */
  static now(): TimeSince {
    return new TimeSince(time);
  }
  /** As if it happened `seconds` ago. */
  static ago(seconds: number): TimeSince {
    return new TimeSince(time - seconds);
  }
  /** Seconds of game time since then. */
  get seconds(): number {
    return time - this.at;
  }
  /** Shorthand for `seconds > s`. */
  over(s: number): boolean {
    return this.seconds > s;
  }
}

/**
 * Seconds until something happens: `cooldown = TimeUntil.in(2); ... if (cooldown.done) ...`
 */
export class TimeUntil {
  private constructor(
    private readonly at: number,
    private readonly length: number,
  ) {}
  static in(seconds: number): TimeUntil {
    return new TimeUntil(time + seconds, seconds);
  }
  /** Seconds left (0 once done). */
  get seconds(): number {
    return Math.max(0, this.at - time);
  }
  get done(): boolean {
    return time >= this.at;
  }
  /** 0 at the start, 1 when done. */
  get fraction(): number {
    return this.length <= 0 ? 1 : Math.min(1, 1 - (this.at - time) / this.length);
  }
}

/** Awaitable waits on engine time (s&box's `Task.Delay`, `GameTask.DelaySeconds`). */
export const Task = {
  /** Resolves after `ms` milliseconds of engine time. */
  delay(ms: number): Promise<void> {
    return new Promise((resolve) => setTimeout(resolve, ms));
  },
  delaySeconds(seconds: number): Promise<void> {
    return Task.delay(seconds * 1000);
  },
  /** Resolves at the start of the next tick. */
  frame(): Promise<void> {
    return new Promise((resolve) => {
      const at = frameCount;
      const check = () => {
        if (frameCount > at) resolve();
        else pending.push(check);
      };
      pending.push(check);
    });
  },
};

let pending: (() => void)[] = [];
onTick(() => {
  const now = pending;
  pending = [];
  for (const fn of now) fn();
});
