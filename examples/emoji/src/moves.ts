// What an act (an entrance, an exit or a pose) is, and the small maths they share.
//
// An act is a pure function of time that returns a `Move`: the director turns that into
// transforms, shadows, sparks and curtain moves. Things that happen once (a sound, a shake of
// the camera, a splash) are `cues`: the director runs each when the act's clock passes it.
//
// Squash and stretch keep their volume (sx grows as sy shrinks) and stay modest, so nothing
// ever looks flattened. Rotation is "lean forward": the director flips it to the way the
// emoji is heading.
import type { Effects } from "./effects.tsx";
import type { Sfx } from "./sfx.ts";

export interface Move {
  /** Progress along the way in from the edge (0) to the centre (1); above 1 it carries on out the far side. Ignored when `x` is set. */
  d: number;
  /** An exact x instead, as seen entering from the left (mirrored when entering from the right). */
  x?: number;
  /** Lift above the floor. */
  y: number;
  sx: number;
  sy: number;
  /** Degrees, leaning forward. */
  rot: number;
  /** Turning about its vertical axis: 1 facing us, near 0 edge on. */
  spin: number;
  /** 0 pivots at the feet, 1 at the middle (for rolling and flipping). */
  centre: number;
  alpha: number;
  /** How open the curtain wants to be (1 when not set). */
  open?: number;
  /** Show this Noto file instead (a wink). */
  swap?: string;
  /** Multiplies the light on it (icy blue, wet grey, scorched). */
  tint?: [number, number, number];
  /** The picture shown back to front (turned round). */
  mirror?: boolean;
  /** Steps taken so far: a footstep sound for each whole one. */
  stride?: number;
  /** 0..1 as it hits the floor: the director puffs dust. */
  thump: number;
  /** True for a moment as it appears in a burst of confetti. */
  puff: boolean;
  /** The rocket's own animation: how far it has turned upright (0..1), its flame (0..1), how hard it burns (0..1) and its sparkles (0..1). */
  rocket?: { turn: number; flame: number; power: number; stars: number };
  /** Dust per second kicked up at the floor under it. */
  ground?: number;
  fx?: { kind: "sparks" | "dust" | "streaks" | "fire"; rate: number };
}

export const still = (): Move => ({ d: 1, y: 0, sx: 1, sy: 1, rot: 0, spin: 1, centre: 0, alpha: 1, thump: 0, puff: false });

/** What cues can ask of the show. */
export interface Ctx {
  sfx(name: Sfx, volume?: number, rate?: number): void;
  /** Ends a sound still playing (fading out), for sounds that belong to a movement that has stopped. */
  stop(name: Sfx, fade?: number): void;
  shake(amount: number, seconds: number): void;
  /** Confetti shot out of a point. */
  burst(x: number, y: number, count: number): void;
  smoke(x: number, y: number, count: number, color: [number, number, number, number], size?: number): void;
  /** A knock: the emoji jolts and settles. */
  jolt(amount: number): void;
  /** Where the emoji is: x, and y at its middle. */
  at(): { x: number; y: number };
  scene: Effects;
}

export type Cue = [t: number, run: (c: Ctx) => void];

export interface Act {
  name: string;
  dur: number;
  at(t: number): Move;
  cues?: Cue[];
  /** How often it's picked among its kind. */
  weight?: number;
  /** The sound of a landing (a thud by default). */
  land?: Sfx;
  /** It doesn't stay: the emoji misses its own show. */
  miss?: boolean;
}

export const clamp01 = (v: number) => Math.min(1, Math.max(0, v));
export const smooth = (t: number) => {
  const c = clamp01(t);
  return c * c * (3 - 2 * c);
};
export const lerp = (a: number, b: number, t: number) => a + (b - a) * t;
/** 0 up to 1 and back to 0: ramps over `edge` of the time at each end. */
export const envelope = (u: number, edge = 0.15) => smooth(u / edge) * (1 - smooth((u - (1 - edge)) / edge));

/** Keyframes [time, value], eased with a cosine between each pair. */
export function keys(t: number, frames: [number, number][]): number {
  const first = frames[0]!;
  if (t <= first[0]) return first[1];
  for (let i = 1; i < frames.length; i++) {
    const [t1, v1] = frames[i]!;
    const [t0, v0] = frames[i - 1]!;
    if (t <= t1) return lerp(v0, v1, smooth((t - t0) / (t1 - t0)));
  }
  return frames[frames.length - 1]![1];
}

/** Keyframes joined by straight lines (for steady running). */
export function line(t: number, frames: [number, number][]): number {
  const first = frames[0]!;
  if (t <= first[0]) return first[1];
  for (let i = 1; i < frames.length; i++) {
    const [t1, v1] = frames[i]!;
    const [t0, v0] = frames[i - 1]!;
    if (t <= t1) return lerp(v0, v1, (t - t0) / (t1 - t0));
  }
  return frames[frames.length - 1]![1];
}

/** An act played backwards, for leaving the way it came (its cues don't replay). */
export const rewind = (a: Act): Act => ({ name: `${a.name} (back)`, dur: a.dur, at: (t) => a.at(Math.max(0, a.dur - t)), weight: a.weight, land: a.land });
