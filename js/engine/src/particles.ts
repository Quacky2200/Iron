// SPDX-License-Identifier: Apache-2.0

// `ParticleSystem` (Unity's, with its main, emission, shape, over-lifetime, texture
// sheet and renderer settings as plain fields). The engine simulates and draws the
// particles; settings are sent when the component is added and again before a frame
// whenever they changed, so nested edits (`ps.shape.radius = 2`) just work.

import { property } from "./property.ts";
import type { Color } from "./math.ts";
import { Component } from "./scene.ts";
import type { Texture } from "./textures.ts";

/** A fixed value or a random one between two, per particle (Unity's `MinMaxCurve` constants). */
export type MinMax = number | readonly [number, number];

/** Where particles start and which way they go: always from the object, along its up (+y). */
export type ParticleShape =
  | { type: "point" }
  | { type: "sphere"; radius: number }
  | { type: "hemisphere"; radius: number }
  /** Spreads up to `angle` degrees from straight up, from a disc of `radius`. */
  | { type: "cone"; angle: number; radius: number }
  | { type: "box"; size: { x: number; y: number; z: number } };

export interface ParticleBurst {
  /** Seconds into each cycle. */
  time: number;
  count: number;
  /** How many times it fires (default 1; 0 keeps firing every `interval`). */
  cycles?: number;
  /** Seconds between firings. */
  interval?: number;
}

const systems = new Map<number, ParticleSystem>();

/**
 * Emits and draws particles from its GameObject (Unity's `ParticleSystem`): sparks,
 * smoke, fire, magic. Particles face the camera; `blendMode: "additive"` makes them glow.
 * Shapes emit along the object's up (+y), not Unity's +z.
 */
export class ParticleSystem extends Component {
  // Main.
  /** Seconds per cycle. */
  @property
  duration = 5;
  @property
  loop = true;
  @property
  playOnAwake = true;
  @property
  startDelay = 0;
  @property
  startLifetime: MinMax = 5;
  @property
  startSpeed: MinMax = 5;
  @property
  startSize: MinMax = 1;
  /** Degrees. */
  @property
  startRotation: MinMax = 0;
  /** Degrees per second. */
  @property
  angularVelocity: MinMax = 0;
  /** One colour, or a random mix of two per particle (linear). */
  @property
  startColor: Color | readonly [Color, Color] | null = null;
  /** Multiplies gravity (9.81 m/s² down). */
  @property
  gravityModifier = 0;
  /** Fraction of speed lost per second. */
  @property
  drag = 0;
  /** `world`: particles stay where they were emitted when the object moves. */
  @property
  simulationSpace: "local" | "world" = "local";
  @property
  maxParticles = 1000;
  @property
  randomSeed = 1;
  // Emission and shape.
  @property
  rateOverTime = 10;
  @property
  bursts: ParticleBurst[] = [];
  @property
  shape: ParticleShape = { type: "cone", angle: 25, radius: 1 };
  // Over lifetime: keys from 0 (birth) to 1 (death), linear between them.
  @property
  sizeOverLifetime: { time: number; value: number }[] | null = null;
  /** Multiplies the start colour (alpha too: fade out with alpha 0 at time 1). */
  @property
  colorOverLifetime: { time: number; color: Color }[] | null = null;
  // Renderer.
  /** The particle image (a soft round spot by default). */
  @property
  texture: Texture | null = null;
  @property
  blendMode: "alpha" | "additive" = "alpha";
  /** A flipbook: the texture cut into `tilesX × tilesY` frames, played `cycles` times per life. */
  @property
  textureSheet: { tilesX: number; tilesY: number; cycles?: number } | null = null;
  /** What happens once it has stopped and its particles are gone. */
  @property
  stopAction: "none" | "destroy" = "none";
  /** Called once it has stopped and its particles are gone. */
  onStopped?: () => void;

  private _particleCount = 0;
  private alive = false;
  private paused = false;
  private sent = "";

  /** Live particles (as of the last frame). */
  get particleCount(): number {
    return this._particleCount;
  }

  /** Emitting or has particles left, and not paused. */
  get isPlaying(): boolean {
    return this.alive && !this.paused;
  }

  get isPaused(): boolean {
    return this.paused;
  }

  private pairs(): unknown[] {
    const range = (m: MinMax) => (typeof m === "number" ? m : [m[0], m[1]]);
    const c = this.startColor;
    const colors = c === null ? [1, 1, 1, 1] : "r" in c ? [c.r, c.g, c.b, c.a] : c.flatMap((x) => [x.r, x.g, x.b, x.a]);
    const s = this.shape;
    const shape =
      s.type === "point"
        ? ["point"]
        : s.type === "cone"
          ? ["cone", s.angle, s.radius]
          : s.type === "box"
            ? ["box", s.size.x, s.size.y, s.size.z]
            : [s.type, s.radius];
    const out: unknown[] = [
      "duration", this.duration,
      "looping", this.loop,
      "startDelay", this.startDelay,
      "lifetime", range(this.startLifetime),
      "speed", range(this.startSpeed),
      "size", range(this.startSize),
      "rotation", range(this.startRotation),
      "angularVelocity", range(this.angularVelocity),
      "color", colors,
      "gravity", this.gravityModifier,
      "drag", this.drag,
      "worldSpace", this.simulationSpace === "world",
      "maxParticles", this.maxParticles,
      "rate", this.rateOverTime,
      "bursts", this.bursts.flatMap((b) => [b.time, b.count, b.cycles ?? 1, b.interval ?? 0.01]),
      "shape", shape,
      "texture", this.texture?.id ?? null,
      "additive", this.blendMode === "additive",
      "seed", this.randomSeed,
    ];
    if (this.sizeOverLifetime) out.push("sizeOverLifetime", this.sizeOverLifetime.flatMap((k) => [k.time, k.value]));
    if (this.colorOverLifetime) {
      out.push("colorOverLifetime", this.colorOverLifetime.flatMap((k) => [k.time, k.color.r, k.color.g, k.color.b, k.color.a]));
    }
    const sheet = this.textureSheet;
    if (sheet) out.push("sheet", [sheet.tilesX, sheet.tilesY, sheet.cycles ?? 1]);
    return out;
  }

  /** Sends the settings if they changed since last time. */
  private sync(playOnAwake = false): void {
    const pairs = this.pairs();
    const key = JSON.stringify(pairs);
    if (key === this.sent) return;
    this.sent = key;
    __platypus_op("render.particles", this.id, this.gameObject.id, pairs, playOnAwake);
  }

  private control(action: string, count?: number): void {
    this.sync();
    __platypus_op("render.particleControl", this.id, action, ...(count === undefined ? [] : [count]));
  }

  /** Starts (or resumes) emitting. */
  play(): void {
    this.paused = false;
    this.alive = true;
    this.control("play");
  }

  /** Stops emitting; with `clear`, removes the live particles too. */
  stop(clear = false): void {
    this.control(clear ? "stopClear" : "stop");
  }

  pause(): void {
    this.paused = true;
    this.control("pause");
  }

  clear(): void {
    this.control("clear");
  }

  /** Emits `count` particles at once (works while stopped, too). */
  emit(count: number): void {
    this.control("emit", count);
  }

  override onAwake(): void {
    systems.set(this.id, this);
    this.alive = this.playOnAwake;
    this.sync(this.playOnAwake);
  }

  override onPreRender(): void {
    this.sync();
  }

  override onDestroy(): void {
    systems.delete(this.id);
    __platypus_op("render.remove", this.id);
  }

  /** @internal The engine's report: count and whether it's alive. */
  _report(count: number, alive: boolean): void {
    const wasAlive = this.alive;
    this._particleCount = count;
    this.alive = alive;
    if (wasAlive && !alive) {
      try {
        this.onStopped?.();
      } catch (e) {
        console.error(`[particles] error in onStopped: ${e instanceof Error ? e.message : String(e)}`);
      }
      if (this.stopAction === "destroy") this.gameObject.destroy();
    }
  }
}

globalThis.__platypus_particles = (changed: number[]) => {
  for (let i = 0; i + 2 < changed.length; i += 3) {
    systems.get(changed[i]!)?._report(changed[i + 1]!, changed[i + 2] === 1);
  }
};

/** Forgets every system. Only for tests. */
export function _resetParticlesForTests() {
  systems.clear();
}
