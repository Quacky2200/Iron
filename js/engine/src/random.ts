// SPDX-License-Identifier: Apache-2.0

// Random numbers, Unity style (`Random.value`, `Random.range`, `Random.insideUnitSphere`,
// ...) with a seedable generator, plus C#'s `System.Random` as `Rng` (`next`,
// `nextDouble`). Not for security: use it for gameplay only.

import { Quaternion, Vector2, Vector3 } from "./math.ts";

/** A seedable generator (mulberry32: fast, 32-bit state, good enough for games). */
export class Rng {
  private state: number;

  /** Seeded from the clock unless given a seed; the same seed gives the same sequence. */
  constructor(seed: number = (Date.now() ^ (Math.random() * 0x100000000)) >>> 0) {
    this.state = seed >>> 0;
  }

  /** 0 (inclusive) to 1 (exclusive). */
  nextDouble(): number {
    this.state = (this.state + 0x6d2b79f5) >>> 0;
    let t = this.state;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 0x100000000;
  }

  /**
   * C#'s `Next`: an integer from `min` (inclusive) to `max` (exclusive); with one
   * argument, 0 to `max`; with none, 0 to 2^31-1.
   */
  next(minOrMax?: number, max?: number): number {
    const [lo, hi] = max === undefined ? [0, minOrMax ?? 0x7fffffff] : [minOrMax ?? 0, max];
    if (hi <= lo) return lo;
    return lo + Math.floor(this.nextDouble() * (hi - lo));
  }

  /** A float from `min` (inclusive) to `max` (exclusive), like Unity's float `Random.Range`. */
  float(min: number, max: number): number {
    return min + this.nextDouble() * (max - min);
  }

  /** An integer from `min` (inclusive) to `max` (exclusive), like Unity's int `Random.Range`. */
  int(min: number, max: number): number {
    return this.next(Math.ceil(min), Math.floor(max));
  }

  bool(chance = 0.5): boolean {
    return this.nextDouble() < chance;
  }

  /** A random item from a list (`undefined` when empty). */
  pick<T>(items: readonly T[]): T | undefined {
    return items.length ? items[this.next(items.length)] : undefined;
  }

  /** Shuffles a list in place (Fisher–Yates) and returns it. */
  shuffle<T>(items: T[]): T[] {
    for (let i = items.length - 1; i > 0; i--) {
      const j = this.next(i + 1);
      [items[i], items[j]] = [items[j]!, items[i]!];
    }
    return items;
  }

  /** A point inside a circle of radius 1. */
  insideUnitCircle(): Vector2 {
    const angle = this.nextDouble() * Math.PI * 2;
    const r = Math.sqrt(this.nextDouble());
    return new Vector2(Math.cos(angle) * r, Math.sin(angle) * r);
  }

  /** A point on a sphere of radius 1. */
  onUnitSphere(): Vector3 {
    const z = this.nextDouble() * 2 - 1;
    const angle = this.nextDouble() * Math.PI * 2;
    const r = Math.sqrt(1 - z * z);
    return new Vector3(Math.cos(angle) * r, Math.sin(angle) * r, z);
  }

  /** A point inside a sphere of radius 1. */
  insideUnitSphere(): Vector3 {
    return this.onUnitSphere().scale(Math.cbrt(this.nextDouble()));
  }

  /** A uniformly random rotation. */
  rotation(): Quaternion {
    const [u1, u2, u3] = [this.nextDouble(), this.nextDouble(), this.nextDouble()];
    const a = Math.sqrt(1 - u1);
    const b = Math.sqrt(u1);
    return new Quaternion(
      a * Math.sin(2 * Math.PI * u2),
      a * Math.cos(2 * Math.PI * u2),
      b * Math.sin(2 * Math.PI * u3),
      b * Math.cos(2 * Math.PI * u3),
    );
  }
}

let shared = new Rng();

/** Unity's static `Random`, on one shared generator. */
export const Random = {
  /** Reseeds the shared generator, for repeatable runs. */
  initState(seed: number): void {
    shared = new Rng(seed);
  },
  /** 0 to 1. */
  get value(): number {
    return shared.nextDouble();
  },
  /** A float from `min` to `max`. For integers use `rangeInt` (max exclusive). */
  range(min: number, max: number): number {
    return shared.float(min, max);
  },
  rangeInt(min: number, max: number): number {
    return shared.int(min, max);
  },
  bool(chance?: number): boolean {
    return shared.bool(chance);
  },
  pick<T>(items: readonly T[]): T | undefined {
    return shared.pick(items);
  },
  shuffle<T>(items: T[]): T[] {
    return shared.shuffle(items);
  },
  get insideUnitCircle(): Vector2 {
    return shared.insideUnitCircle();
  },
  get insideUnitSphere(): Vector3 {
    return shared.insideUnitSphere();
  },
  get onUnitSphere(): Vector3 {
    return shared.onUnitSphere();
  },
  get rotation(): Quaternion {
    return shared.rotation();
  },
};
