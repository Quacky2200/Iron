// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// Splines (Unity's Splines package, the core of it): a path through knots, smooth unless told
// otherwise. The maths is native (`platypus_math::spline`, the same code compiled games call); the
// app installs `Spline` as a global class. This file types it for bundled games and adds the
// `SplineAnimate` component, which stays a script component (a bundled game's `GameObject` is
// the op-based one in `scene.ts`, so the native component cannot be attached to it) but leaves
// the path maths to the native `SplineAnimate.pose` / `progress` / `facing`.

import type { Quaternion, Vector3, Vector3Like } from "./math.ts";
import { property } from "./property.ts";
import { Component, type GameObject } from "./scene.ts";

/** `auto`: smooth through the neighbours; `bezier`: the knot's own handles; `linear`: none. */
export type TangentMode = "auto" | "bezier" | "linear";

export interface SplineKnot {
  position: Vector3;
  /** The handle before the knot, relative to it (used in `bezier` mode). */
  tangentIn: Vector3;
  /** The handle after the knot, relative to it (used in `bezier` mode). */
  tangentOut: Vector3;
  mode: TangentMode;
}

/** A path through knots, every segment a cubic Bézier. `evaluate(t)` goes by distance (t 0 to 1). */
export interface Spline {
  /** A copy of every knot (edit with `setPosition`, `setTangents`, `setMode`, ...). */
  readonly knots: SplineKnot[];
  readonly count: number;
  /** Whether the last knot joins back to the first. */
  closed: boolean;
  /** How strongly `auto` knots bend: 0 is straight lines, 0.5 Catmull-Rom. */
  tension: number;
  readonly segmentCount: number;
  /** Goes up on every change. */
  readonly version: number;
  readonly length: number;
  addKnot(position: Vector3Like, mode?: TangentMode, tangentIn?: Vector3Like, tangentOut?: Vector3Like): number;
  insertKnot(index: number, position: Vector3Like, mode?: TangentMode, tangentIn?: Vector3Like, tangentOut?: Vector3Like): number;
  removeKnot(index: number): void;
  setPosition(index: number, position: Vector3Like): void;
  setTangents(index: number, tangentIn: Vector3Like, tangentOut: Vector3Like): void;
  setMode(index: number, mode: TangentMode): void;
  handleOut(index: number): Vector3;
  handleIn(index: number): Vector3;
  segmentPoint(i: number, u: number): Vector3;
  segmentDerivative(i: number, u: number): Vector3;
  locate(t: number): { segment: number; u: number };
  evaluate(t: number): Vector3;
  tangent(t: number): Vector3;
  sample(perSegment?: number): Vector3[];
  nearest(p: Vector3Like): { t: number; point: Vector3; distance: number };
  /** Forgets the spline (a handle is a number; nothing frees it by itself). */
  destroy(): void;
}

export const Spline: {
  new (points?: readonly Vector3Like[], options?: { closed?: boolean; tension?: number; mode?: TangentMode }): Spline;
} = (globalThis as unknown as { Spline: never }).Spline;

type LoopMode = "loop" | "pingPong" | "once";

/** The native half of `SplineAnimate` (the class of that name, installed by the app). */
interface NativeSplineAnimate {
  pose(spline: Spline, elapsed: number, duration: number, loopMode: LoopMode): { position: Vector3; direction: Vector3 };
  progress(elapsed: number, duration: number, loopMode: LoopMode): number;
  facing(direction: Vector3): Quaternion;
}

const native = (globalThis as unknown as { SplineAnimate: NativeSplineAnimate }).SplineAnimate;

/**
 * Moves its object along a spline (Unity's `SplineAnimate`): the whole path in `duration`
 * seconds at a steady speed, looping, back and forth, or once. `alignment: "path"` turns the
 * object to face along it (its +x on a flat, 2D spline; its forward otherwise). With a
 * `container`, the spline's points are in that object's space, so moving it moves the path.
 */
export class SplineAnimate extends Component {
  /** The path (not saved with scenes: set it from code). */
  spline: Spline | null = null;
  @property container: GameObject | null = null;
  @property duration = 5;
  @property loopMode: LoopMode = "loop";
  @property alignment: "none" | "path" = "path";
  @property playing = true;
  /** Seconds played so far (set it to jump). */
  @property elapsed = 0;

  /** How far along the path it is now (0 to 1, by distance). */
  get t(): number {
    return native.progress(this.elapsed, this.duration, this.loopMode);
  }

  override onUpdate(dt: number): void {
    if (this.playing) this.elapsed += dt;
    this.place();
  }

  /** Puts the object where `t` says (call it after moving the spline mid-frame). */
  place(): void {
    const s = this.spline;
    if (!s || s.count === 0) return;
    const { position, direction } = native.pose(s, this.elapsed, this.duration, this.loopMode);
    const c = this.container?.transform;
    this.transform.position = c ? c.transformPoint(position) : position;
    if (this.alignment !== "path" || direction.magnitude < 1e-9) return;
    this.transform.rotation = native.facing(c ? c.transformDirection(direction) : direction);
  }
}
