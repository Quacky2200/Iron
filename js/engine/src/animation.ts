// SPDX-License-Identifier: Apache-2.0

// Animation, Unity's way: `AnimationCurve`s (keyframes with tangents) gathered into an
// `AnimationClip`, each curve driving one channel of an object under the animated one
// (its transform, or a number on one of its components). Two players:
//
// - `Animation` (the legacy component): named clips, `play`, `crossFade`, `stop`.
// - `Animator` with an `AnimatorController`: parameters, states (clips or 1D blend
//   trees), and transitions on conditions or exit time, cross-fading.
//
// Every frame, after all components' `onUpdate` and before `onLateUpdate`, the engine
// calls `__platypus_animate(dt)`: each player advances, blends its clips by weight and writes
// the result (each transform once). Clip events call a method on the animated object's
// components.

import { Quaternion, Vector3 } from "./math.ts";
import { property } from "./property.ts";
import { Component, type ComponentClass, GameObject } from "./scene.ts";

/** One key on a curve (Unity's `Keyframe`): tangents are slopes (value per second). */
export class Keyframe {
  constructor(
    public time: number,
    public value: number,
    /** The slope arriving at the key; `Infinity` holds the previous value (a step). */
    public inTangent = 0,
    /** The slope leaving the key; `Infinity` holds this value until the next key. */
    public outTangent = 0,
  ) {}
}

/** What happens past a curve's or clip's ends (Unity's `WrapMode`). */
export type WrapMode = "once" | "loop" | "pingPong" | "clampForever";

/** Time `t` folded into 0..length by a wrap mode. `once` and `clampForever` clamp. */
function wrap(t: number, length: number, mode: WrapMode): number {
  if (length <= 0) return 0;
  if (mode === "loop") return ((t % length) + length) % length;
  if (mode === "pingPong") {
    const p = ((t % (2 * length)) + 2 * length) % (2 * length);
    return p > length ? 2 * length - p : p;
  }
  return Math.min(length, Math.max(0, t));
}

/**
 * A value over time (Unity's `AnimationCurve`): keys joined by cubic Hermite
 * segments shaped by their tangents. Before the first key and after the last, the wrap
 * modes apply (`clampForever` by default: the end values hold).
 */
export class AnimationCurve {
  keys: Keyframe[];
  preWrapMode: WrapMode = "clampForever";
  postWrapMode: WrapMode = "clampForever";

  constructor(...keys: Keyframe[]) {
    this.keys = [...keys].sort((a, b) => a.time - b.time);
  }

  /** A straight line from (t0, v0) to (t1, v1). */
  static linear(t0: number, v0: number, t1: number, v1: number): AnimationCurve {
    const slope = t1 === t0 ? 0 : (v1 - v0) / (t1 - t0);
    return new AnimationCurve(new Keyframe(t0, v0, 0, slope), new Keyframe(t1, v1, slope, 0));
  }

  /** From (t0, v0) to (t1, v1), easing in and out (flat tangents). */
  static easeInOut(t0: number, v0: number, t1: number, v1: number): AnimationCurve {
    return new AnimationCurve(new Keyframe(t0, v0), new Keyframe(t1, v1));
  }

  /** `value` from t0 to t1. */
  static constant(t0: number, t1: number, value: number): AnimationCurve {
    return new AnimationCurve(new Keyframe(t0, value), new Keyframe(t1, value));
  }

  /**
   * Keys through points with tangents made to match: `linear` (straight segments),
   * `smooth` (Catmull-Rom-like slopes) or `step` (each value holds until the next).
   */
  static fromPoints(points: readonly (readonly [number, number])[], mode: "linear" | "smooth" | "step" = "linear"): AnimationCurve {
    const keys = points.map(([t, v]) => new Keyframe(t, v));
    for (let i = 0; i < keys.length; i++) {
      const prev = keys[i - 1];
      const next = keys[i + 1];
      const k = keys[i]!;
      const slope = (a: Keyframe, b: Keyframe) => (b.time === a.time ? 0 : (b.value - a.value) / (b.time - a.time));
      if (mode === "step") {
        k.inTangent = k.outTangent = Infinity;
      } else if (mode === "linear") {
        k.inTangent = prev ? slope(prev, k) : 0;
        k.outTangent = next ? slope(k, next) : 0;
      } else {
        const s = prev && next ? slope(prev, next) : prev ? slope(prev, k) : next ? slope(k, next) : 0;
        k.inTangent = k.outTangent = s;
      }
    }
    return new AnimationCurve(...keys);
  }

  /** Adds a key, keeping them in time order; returns its index. */
  addKey(time: number | Keyframe, value = 0): number {
    const key = time instanceof Keyframe ? time : new Keyframe(time, value);
    let i = this.keys.findIndex((k) => k.time > key.time);
    if (i < 0) i = this.keys.length;
    this.keys.splice(i, 0, key);
    return i;
  }

  removeKey(index: number): void {
    this.keys.splice(index, 1);
  }

  get length(): number {
    return this.keys.length;
  }

  /** The curve's value at `time`. */
  evaluate(time: number): number {
    const keys = this.keys;
    if (keys.length === 0) return 0;
    if (keys.length === 1) return keys[0]!.value;
    const first = keys[0]!.time;
    const last = keys[keys.length - 1]!.time;
    const span = last - first;
    let t = time;
    if (t < first) t = first + wrap(t - first, span, this.preWrapMode);
    else if (t > last) t = first + wrap(t - first, span, this.postWrapMode);
    // The segment holding t (binary search).
    let lo = 0;
    let hi = keys.length - 1;
    while (hi - lo > 1) {
      const mid = (lo + hi) >> 1;
      if (keys[mid]!.time <= t) lo = mid;
      else hi = mid;
    }
    const a = keys[lo]!;
    const b = keys[hi]!;
    const dt = b.time - a.time;
    if (dt <= 0) return b.value;
    if (!Number.isFinite(a.outTangent) || !Number.isFinite(b.inTangent)) return t >= b.time ? b.value : a.value;
    const s = (t - a.time) / dt;
    const s2 = s * s;
    const s3 = s2 * s;
    return (
      (2 * s3 - 3 * s2 + 1) * a.value +
      (s3 - 2 * s2 + s) * a.outTangent * dt +
      (-2 * s3 + 3 * s2) * b.value +
      (s3 - s2) * b.inTangent * dt
    );
  }

  /** @internal As plain data, for saving. */
  _toJSON(): unknown {
    const n = (x: number) => (Number.isFinite(x) ? x : "Infinity");
    return {
      keys: this.keys.map((k) => [k.time, k.value, n(k.inTangent), n(k.outTangent)]),
      pre: this.preWrapMode,
      post: this.postWrapMode,
    };
  }

  /** @internal */
  static _fromJSON(data: unknown): AnimationCurve {
    const d = data as { keys?: unknown[][]; pre?: WrapMode; post?: WrapMode };
    const num = (x: unknown) => (x === "Infinity" ? Infinity : Number(x));
    const curve = new AnimationCurve(...(d.keys ?? []).map((k) => new Keyframe(num(k[0]), num(k[1]), num(k[2]), num(k[3]))));
    curve.preWrapMode = d.pre ?? "clampForever";
    curve.postWrapMode = d.post ?? "clampForever";
    return curve;
  }
}

/** A method called on the animated object's components when a clip passes `time`. */
export interface AnimationEvent {
  time: number;
  functionName: string;
  /** Passed to the method, if given. */
  parameter?: unknown;
}

/** What a curve drives: `"Transform"`, `"GameObject"` (`active`) or a component class (by name). */
export type CurveTarget = "Transform" | "GameObject" | string | ComponentClass<Component>;

interface Binding {
  path: string;
  type: string;
  property: string;
  curve: AnimationCurve;
}

/**
 * Animation data (Unity's `AnimationClip`): curves by relative path ("" is the animated
 * object, "Arm/Hand" a descendant by names), target and property. Transform properties:
 * `localPosition.x|y|z`, `localScale.x|y|z`, `localRotation.x|y|z|w` (a quaternion,
 * normalized after blending) or `localEulerAngles.x|y|z`. Components: any number
 * property by name, or one field of a value (`color.r`, `size.x`); booleans count as
 * on above 0.5. `"GameObject"`, `active`: turns the object on and off.
 */
export class AnimationClip {
  /** What happens past the end when played (`once` stops at the end, holding it). */
  wrapMode: WrapMode = "once";
  frameRate = 60;
  readonly events: AnimationEvent[] = [];
  /** @internal */
  readonly _bindings: Binding[] = [];

  /** @internal Its curves grouped per object, and its length: worked out once, until it changes. */
  _tracks: Track[] | null = null;
  private _length: number | null = null;

  constructor(public name = "clip") {}

  /** Sets (or with `null` removes) the curve for one property. */
  setCurve(relativePath: string, type: CurveTarget, propertyName: string, curve: AnimationCurve | null): void {
    const typeName = typeof type === "string" ? type : type.name;
    const i = this._bindings.findIndex((b) => b.path === relativePath && b.type === typeName && b.property === propertyName);
    if (i >= 0) this._bindings.splice(i, 1);
    if (curve) this._bindings.push({ path: relativePath, type: typeName, property: propertyName, curve });
    this._tracks = null;
    this._length = null;
  }

  /** The curves, as (path, type, property, curve). */
  get curves(): readonly Readonly<Binding>[] {
    return this._bindings;
  }

  /** The last key's time over all curves (or the last event's). */
  get length(): number {
    if (this._length === null) {
      let end = 0;
      for (const b of this._bindings) end = Math.max(end, b.curve.keys.at(-1)?.time ?? 0);
      for (const e of this.events) end = Math.max(end, e.time);
      this._length = end;
    }
    return this._length;
  }

  addEvent(event: AnimationEvent): void {
    this.events.push({ ...event });
    this.events.sort((a, b) => a.time - b.time);
    this._length = null;
  }

  /** Poses `go` as the clip is at `time` (Unity's `SampleAnimation`). */
  sampleAnimation(go: GameObject, time: number): void {
    const pose = new Pose();
    pose.add(go, this, wrap(time, this.length, this.wrapMode), 1);
    pose.apply();
  }

  /** As plain data (for files and scenes). */
  toJSON(): unknown {
    return {
      name: this.name,
      wrapMode: this.wrapMode,
      frameRate: this.frameRate,
      curves: this._bindings.map((b) => ({ path: b.path, type: b.type, property: b.property, curve: b.curve._toJSON() })),
      events: this.events.map((e) => ({ ...e })),
    };
  }

  static fromJSON(data: unknown): AnimationClip {
    const d = data as {
      name?: string;
      wrapMode?: WrapMode;
      frameRate?: number;
      curves?: { path: string; type: string; property: string; curve: unknown }[];
      events?: AnimationEvent[];
    };
    const clip = new AnimationClip(d.name ?? "clip");
    clip.wrapMode = d.wrapMode ?? "once";
    clip.frameRate = d.frameRate ?? 60;
    for (const c of d.curves ?? []) clip.setCurve(c.path, c.type, c.property, AnimationCurve._fromJSON(c.curve));
    for (const e of d.events ?? []) clip.addEvent(e);
    return clip;
  }
}

/** The object at a path ("" itself, "A/B" by child names) under `root`. */
function resolve(root: GameObject, path: string): GameObject | undefined {
  if (path === "") return root;
  let go: GameObject | undefined = root;
  for (const name of path.split("/")) {
    go = go?.findChild(name);
    if (!go) return undefined;
  }
  return go;
}

/** Each root's objects by path, found once (looked up again if one is destroyed). */
const resolved = new WeakMap<GameObject, Map<string, GameObject | null>>();
function resolveCached(root: GameObject, path: string): GameObject | undefined {
  let paths = resolved.get(root);
  if (!paths) {
    paths = new Map();
    resolved.set(root, paths);
  }
  const hit = paths.get(path);
  if (hit && !hit.destroyed) return hit;
  // A miss is looked up again each frame: the object may appear (a model still loading).
  const go = resolve(root, path);
  paths.set(path, go ?? null);
  return go;
}

/** A component on `go` by class name. */
function componentNamed(go: GameObject, name: string): Component | undefined {
  return go.components.find((c) => c.constructor.name === name);
}

const AXES: Record<string, number> = { x: 0, y: 1, z: 2, w: 3 };

/** One object's curves in a clip: transform channels by axis, and anything else. */
interface Track {
  path: string;
  p: (AnimationCurve | undefined)[];
  r: (AnimationCurve | undefined)[];
  s: (AnimationCurve | undefined)[];
  e: (AnimationCurve | undefined)[];
  other: Binding[];
}

/** A clip's curves grouped per object (once per clip, until its curves change). */
function tracksOf(clip: AnimationClip): Track[] {
  if (clip._tracks) return clip._tracks;
  const byPath = new Map<string, Track>();
  for (const b of clip._bindings) {
    let t = byPath.get(b.path);
    if (!t) {
      t = { path: b.path, p: [], r: [], s: [], e: [], other: [] };
      byPath.set(b.path, t);
    }
    const [field, axis] = b.property.split(".") as [string, string | undefined];
    const i = axis === undefined ? -1 : (AXES[axis] ?? -1);
    const list =
      b.type !== "Transform" ? null : field === "localPosition" ? t.p : field === "localRotation" ? t.r : field === "localScale" ? t.s : field === "localEulerAngles" ? t.e : null;
    if (list && i >= 0 && (list === t.r || i < 3)) list[i] = b.curve;
    else if (b.type !== "Transform") t.other.push(b);
  }
  clip._tracks = [...byPath.values()];
  return clip._tracks;
}

/** One object's blended transform: weighted sums and the weight per channel. */
interface Sums {
  p: number[];
  pw: number[];
  r: number[];
  rw: number;
  s: number[];
  sw: number[];
  e: number[];
  ew: number[];
}

/**
 * Blended values for one frame: per object, transform channels (weighted sums, and
 * quaternions sign-aligned then normalized), and other properties.
 */
class Pose {
  private transforms = new Map<GameObject, Sums>();
  private props = new Map<string, { target: Component | GameObject; property: string; value: number; weight: number }>();

  add(root: GameObject, clip: AnimationClip, time: number, weight: number): void {
    this.addClip(root, clip, time, weight, false);
  }

  /** As `add`, each rotation flipped to the side of what's summed so far (q and -q are one turn). */
  addAligned(root: GameObject, clip: AnimationClip, time: number, weight: number): void {
    this.addClip(root, clip, time, weight, true);
  }

  private addClip(root: GameObject, clip: AnimationClip, time: number, weight: number, align: boolean): void {
    if (weight <= 0) return;
    for (const track of tracksOf(clip)) {
      const go = resolveCached(root, track.path);
      if (!go) continue;
      if (track.p.length || track.r.length || track.s.length || track.e.length) {
        let t = this.transforms.get(go);
        if (!t) {
          t = { p: [0, 0, 0], pw: [0, 0, 0], r: [0, 0, 0, 0], rw: 0, s: [0, 0, 0], sw: [0, 0, 0], e: [0, 0, 0], ew: [0, 0, 0] };
          this.transforms.set(go, t);
        }
        for (let i = 0; i < 3; i++) {
          const pc = track.p[i];
          if (pc) {
            t.p[i]! += pc.evaluate(time) * weight;
            t.pw[i]! += weight;
          }
          const sc = track.s[i];
          if (sc) {
            t.s[i]! += sc.evaluate(time) * weight;
            t.sw[i]! += weight;
          }
          const ec = track.e[i];
          if (ec) {
            t.e[i]! += ec.evaluate(time) * weight;
            t.ew[i]! += weight;
          }
        }
        if (track.r.length) {
          const q = [0, 1, 2, 3].map((i) => track.r[i]?.evaluate(time) ?? 0);
          const dot = q[0]! * t.r[0]! + q[1]! * t.r[1]! + q[2]! * t.r[2]! + q[3]! * t.r[3]!;
          const sign = align && dot < 0 ? -1 : 1;
          for (let i = 0; i < 4; i++) t.r[i]! += sign * q[i]! * weight;
          t.rw += weight;
        }
      }
      for (const b of track.other) {
        const target = b.type === "GameObject" ? go : componentNamed(go, b.type);
        if (!target) continue;
        const value = b.curve.evaluate(time);
        const key = `${go.id}/${b.type}/${b.property}`;
        const prev = this.props.get(key);
        if (prev) {
          prev.value += value * weight;
          prev.weight += weight;
        } else {
          this.props.set(key, { target, property: b.property, value: value * weight, weight });
        }
      }
    }
  }

  apply(): void {
    for (const [go, t] of this.transforms) {
      if (go.destroyed) continue;
      const tr = go.transform;
      const avg = (sum: number[], w: number[], current: Vector3) =>
        w[0] || w[1] || w[2] ? new Vector3(w[0] ? sum[0]! / w[0] : current.x, w[1] ? sum[1]! / w[1] : current.y, w[2] ? sum[2]! / w[2] : current.z) : current;
      const position = avg(t.p, t.pw, tr.localPosition);
      const scale = avg(t.s, t.sw, tr.localScale);
      let rotation = tr.localRotation;
      if (t.rw > 0) rotation = new Quaternion(t.r[0]!, t.r[1]!, t.r[2]!, t.r[3]!).normalized;
      else if (t.ew[0] || t.ew[1] || t.ew[2]) {
        const e = avg(t.e, t.ew, tr.localEulerAngles);
        rotation = Quaternion.euler(e.x, e.y, e.z);
      }
      tr._setLocal(position, rotation, scale);
    }
    for (const p of this.props.values()) {
      const value = p.value / p.weight;
      if (p.target instanceof GameObject) {
        if (p.property === "active") p.target.enabled = value > 0.5;
        continue;
      }
      setProperty(p.target, p.property, value);
    }
  }
}

/** Sets a number property, or one field of a value property (copied, then set back). */
function setProperty(target: Component, property: string, value: number): void {
  const record = target as unknown as Record<string, unknown>;
  const dot = property.indexOf(".");
  if (dot < 0) {
    const current = record[property];
    record[property] = typeof current === "boolean" ? value > 0.5 : value;
    return;
  }
  const head = property.slice(0, dot);
  const field = property.slice(dot + 1);
  const current = record[head];
  if (current === null || typeof current !== "object") return;
  const copy = Object.assign(Object.create(Object.getPrototypeOf(current)), current) as Record<string, unknown>;
  copy[field] = value;
  record[head] = copy;
}

/** Calls clip events between two times on the object's components. */
function fireEvents(go: GameObject, clip: AnimationClip, from: number, to: number, length: number, wrapMode: WrapMode): void {
  if (clip.events.length === 0 || to <= from) return;
  // Unwrapped times: events fire once per pass through a loop.
  const passes = wrapMode === "loop" && length > 0 ? Math.floor(to / length) - Math.floor(from / length) : 0;
  const local = (t: number) => (wrapMode === "loop" && length > 0 ? ((t % length) + length) % length : t);
  const a = local(from);
  const b = local(to);
  for (const e of clip.events) {
    const hit = passes === 0 ? e.time > a && e.time <= b : e.time > a || e.time <= b || passes > 1;
    if (!hit) continue;
    for (const c of [...go.components]) {
      const fn = (c as unknown as Record<string, unknown>)[e.functionName];
      if (typeof fn === "function") fn.call(c, e.parameter);
    }
  }
}

/** A clip playing on an `Animation`, with its time, speed and weight (Unity's `AnimationState`). */
export class AnimationState {
  time = 0;
  speed = 1;
  weight = 0;
  enabled = false;
  wrapMode: WrapMode;
  /** @internal Weight change per second while fading. */
  _fade = 0;
  /** @internal The weight it fades to. */
  _target = 0;
  constructor(readonly name: string, readonly clip: AnimationClip) {
    this.wrapMode = clip.wrapMode;
  }
  get length(): number {
    return this.clip.length;
  }
  /** Time over length (past 1 after a loop). */
  get normalizedTime(): number {
    return this.length > 0 ? this.time / this.length : 0;
  }
  set normalizedTime(t: number) {
    this.time = t * this.length;
  }
}

/**
 * Plays clips on its object (Unity's legacy `Animation`): `play` one, `crossFade` to
 * another over a time, `stop`. `clip` plays on its own when `playAutomatically`.
 */
export class Animation extends Component {
  /** The default clip. */
  @property clip: AnimationClip | null = null;
  @property clips: AnimationClip[] = [];
  @property playAutomatically = true;
  private states = new Map<string, AnimationState>();

  override onStart(): void {
    if (this.playAutomatically && this.clip) this.play(this.clip.name);
  }

  /** Adds a clip under a name (its own by default). */
  addClip(clip: AnimationClip, name = clip.name): void {
    if (!this.clips.includes(clip)) this.clips.push(clip);
    this.states.set(name, new AnimationState(name, clip));
  }

  /** The state of a clip by name. */
  state(name: string): AnimationState | undefined {
    if (!this.states.has(name)) {
      const clip = [...this.clips, ...(this.clip ? [this.clip] : [])].find((c) => c.name === name);
      if (clip) this.states.set(name, new AnimationState(name, clip));
    }
    return this.states.get(name);
  }

  /** Plays a clip from the start at full weight, stopping the others. Returns false if unknown. */
  play(name = this.clip?.name ?? ""): boolean {
    const s = this.state(name);
    if (!s) return false;
    for (const other of this.states.values()) {
      if (other !== s) {
        other.enabled = false;
        other.weight = 0;
      }
    }
    s.enabled = true;
    s.time = 0;
    s.weight = 1;
    s._fade = 0;
    s._target = 1;
    return true;
  }

  /** Fades a clip in over `fadeLength` seconds while the others fade out. */
  crossFade(name: string, fadeLength = 0.3): void {
    const s = this.state(name);
    if (!s) return;
    if (!s.enabled) {
      s.enabled = true;
      s.time = 0;
    }
    const rate = 1 / Math.max(1e-4, fadeLength);
    s._target = 1;
    s._fade = rate;
    for (const other of this.states.values()) {
      if (other !== s && other.enabled) {
        other._target = 0;
        other._fade = rate;
      }
    }
  }

  stop(name?: string): void {
    for (const s of this.states.values()) {
      if (name === undefined || s.name === name) {
        s.enabled = false;
        s.weight = 0;
        s.time = 0;
      }
    }
  }

  isPlaying(name?: string): boolean {
    return [...this.states.values()].some((s) => s.enabled && (name === undefined || s.name === name));
  }

  /** @internal Advances by `dt` and adds this frame's pose. */
  _animate(dt: number): void {
    const pose = new Pose();
    for (const s of this.states.values()) {
      if (!s.enabled) continue;
      if (s._fade) {
        const step = s._fade * dt;
        s.weight = s.weight < s._target ? Math.min(s._target, s.weight + step) : Math.max(s._target, s.weight - step);
        if (s.weight === s._target) s._fade = 0;
        if (s.weight <= 0 && s._target === 0) {
          s.enabled = false;
          continue;
        }
      }
      const before = s.time;
      s.time += dt * s.speed;
      fireEvents(this.gameObject, s.clip, before, s.time, s.length, s.wrapMode);
      if (s.wrapMode === "once" && s.time >= s.length) {
        // Holds the last frame this time, then stops.
        pose.addAligned(this.gameObject, s.clip, s.length, s.weight);
        s.enabled = false;
        continue;
      }
      pose.addAligned(this.gameObject, s.clip, wrap(s.time, s.length, s.wrapMode), s.weight);
    }
    pose.apply();
  }
}

// ---- Animator ----

export type AnimatorParameterType = "float" | "int" | "bool" | "trigger";

export interface AnimatorParameter {
  name: string;
  type: AnimatorParameterType;
  defaultValue?: number | boolean;
}

/** A 1D blend tree: clips placed along a parameter, the two around its value blended. */
export interface BlendTree {
  blendParameter: string;
  children: { motion: AnimationClip; threshold: number; timeScale?: number }[];
}

export interface AnimatorState {
  name: string;
  motion: AnimationClip | BlendTree;
  speed?: number;
  /** A float parameter multiplying `speed`. */
  speedParameter?: string;
}

export type ConditionMode = "if" | "ifNot" | "greater" | "less" | "equals" | "notEqual";

export interface AnimatorCondition {
  parameter: string;
  mode: ConditionMode;
  threshold?: number;
}

export interface AnimatorTransition {
  /** A state's name, or `"Any"` (from any state, except into itself). */
  from: string;
  to: string;
  conditions?: AnimatorCondition[];
  /** Only once the source state's normalized time passes `exitTime`. */
  hasExitTime?: boolean;
  exitTime?: number;
  /** The cross-fade, in seconds. */
  duration?: number;
  /** Where the destination starts, normalized. */
  offset?: number;
}

/** States, parameters and transitions an `Animator` runs (Unity's `AnimatorController`, one layer). */
export class AnimatorController {
  parameters: AnimatorParameter[] = [];
  states: AnimatorState[] = [];
  transitions: AnimatorTransition[] = [];
  /** The first state's name when not set. */
  defaultState: string | null = null;

  constructor(init: Partial<Pick<AnimatorController, "parameters" | "states" | "transitions" | "defaultState">> = {}) {
    Object.assign(this, init);
  }

  addParameter(name: string, type: AnimatorParameterType, defaultValue?: number | boolean): this {
    this.parameters.push({ name, type, defaultValue });
    return this;
  }

  addState(name: string, motion: AnimationClip | BlendTree, options: Omit<AnimatorState, "name" | "motion"> = {}): this {
    this.states.push({ name, motion, ...options });
    return this;
  }

  addTransition(t: AnimatorTransition): this {
    this.transitions.push(t);
    return this;
  }

  /** As plain data (clips inline). */
  toJSON(): unknown {
    const motion = (m: AnimationClip | BlendTree) =>
      m instanceof AnimationClip
        ? { clip: m.toJSON() }
        : { blendParameter: m.blendParameter, children: m.children.map((c) => ({ ...c, motion: c.motion.toJSON() })) };
    return {
      parameters: this.parameters,
      states: this.states.map((s) => ({ ...s, motion: motion(s.motion) })),
      transitions: this.transitions,
      defaultState: this.defaultState,
    };
  }

  static fromJSON(data: unknown): AnimatorController {
    const d = data as {
      parameters?: AnimatorParameter[];
      states?: (Omit<AnimatorState, "motion"> & { motion: { clip?: unknown; blendParameter?: string; children?: { motion: unknown; threshold: number; timeScale?: number }[] } })[];
      transitions?: AnimatorTransition[];
      defaultState?: string | null;
    };
    return new AnimatorController({
      parameters: d.parameters ?? [],
      transitions: d.transitions ?? [],
      defaultState: d.defaultState ?? null,
      states: (d.states ?? []).map((s) => ({
        ...s,
        motion: s.motion.clip
          ? AnimationClip.fromJSON(s.motion.clip)
          : {
              blendParameter: s.motion.blendParameter ?? "",
              children: (s.motion.children ?? []).map((c) => ({ ...c, motion: AnimationClip.fromJSON(c.motion) })),
            },
      })),
    });
  }
}

/** Where an animator is in a state (Unity's `AnimatorStateInfo`). */
export interface AnimatorStateInfo {
  name: string;
  /** Time over the state's length (past 1 after loops). */
  normalizedTime: number;
  /** The state's length in seconds. */
  length: number;
  isName(name: string): boolean;
}

interface Playing {
  state: AnimatorState;
  /**
   * How far through the state, over its length (past 1 after loops), as Unity keeps it: a
   * blend tree's length changes with its weights, so seconds would jump about in its cycle.
   */
  norm: number;
}

/**
 * Runs an `AnimatorController` on its object (Unity's `Animator`): set parameters and the
 * controller's transitions move between states, cross-fading; `play` and `crossFade` jump
 * directly.
 */
export class Animator extends Component {
  @property controller: AnimatorController | null = null;
  /** Multiplies every state's speed. */
  @property speed = 1;
  private values = new Map<string, number | boolean>();
  private current: Playing | null = null;
  /** The state being faded to, and how far (0..1) over how long. */
  private next: (Playing & { progress: number; duration: number }) | null = null;

  private ensure(): void {
    if (this.current || !this.controller) return;
    for (const p of this.controller.parameters) {
      if (!this.values.has(p.name)) this.values.set(p.name, p.defaultValue ?? (p.type === "bool" || p.type === "trigger" ? false : 0));
    }
    const name = this.controller.defaultState ?? this.controller.states[0]?.name;
    const state = this.controller.states.find((s) => s.name === name);
    if (state) this.current = { state, norm: 0 };
  }

  setFloat(name: string, value: number): void {
    this.values.set(name, value);
  }
  getFloat(name: string): number {
    this.ensure();
    return Number(this.values.get(name) ?? 0);
  }
  setInteger(name: string, value: number): void {
    this.values.set(name, Math.trunc(value));
  }
  getInteger(name: string): number {
    return Math.trunc(this.getFloat(name));
  }
  setBool(name: string, value: boolean): void {
    this.values.set(name, value);
  }
  getBool(name: string): boolean {
    this.ensure();
    return this.values.get(name) === true;
  }
  /** On until a transition uses it. */
  setTrigger(name: string): void {
    this.values.set(name, true);
  }
  resetTrigger(name: string): void {
    this.values.set(name, false);
  }

  /** Jumps to a state (no fade), at a normalized time. */
  play(stateName: string, normalizedTime = 0): void {
    this.ensure();
    const state = this.controller?.states.find((s) => s.name === stateName);
    if (!state) return;
    this.current = { state, norm: normalizedTime };
    this.next = null;
  }

  /** Fades to a state over `duration` seconds. */
  crossFade(stateName: string, duration = 0.25, normalizedTime = 0): void {
    this.ensure();
    const state = this.controller?.states.find((s) => s.name === stateName);
    if (!state || !this.current) {
      if (state) this.play(stateName, normalizedTime);
      return;
    }
    this.next = { state, norm: normalizedTime, progress: 0, duration: Math.max(1e-4, duration) };
  }

  get isInTransition(): boolean {
    return this.next !== null;
  }

  getCurrentAnimatorStateInfo(): AnimatorStateInfo {
    this.ensure();
    return this.info(this.current);
  }

  getNextAnimatorStateInfo(): AnimatorStateInfo {
    return this.info(this.next);
  }

  private info(p: Playing | null): AnimatorStateInfo {
    const length = p ? this.lengthOf(p.state) : 0;
    const name = p?.state.name ?? "";
    return { name, length, normalizedTime: p && length > 0 ? p.norm : 0, isName: (n) => n === name };
  }

  /** The state's clips with their weights (a blend tree's two neighbours). */
  private clips(state: AnimatorState): { clip: AnimationClip; weight: number; timeScale: number }[] {
    const m = state.motion;
    if (m instanceof AnimationClip) return [{ clip: m, weight: 1, timeScale: 1 }];
    const children = [...m.children].sort((a, b) => a.threshold - b.threshold);
    if (children.length === 0) return [];
    const v = this.getFloat(m.blendParameter);
    const first = children[0]!;
    const last = children[children.length - 1]!;
    if (v <= first.threshold) return [{ clip: first.motion, weight: 1, timeScale: first.timeScale ?? 1 }];
    if (v >= last.threshold) return [{ clip: last.motion, weight: 1, timeScale: last.timeScale ?? 1 }];
    const i = children.findIndex((c) => c.threshold > v);
    const a = children[i - 1]!;
    const b = children[i]!;
    const t = (v - a.threshold) / (b.threshold - a.threshold);
    return [
      { clip: a.motion, weight: 1 - t, timeScale: a.timeScale ?? 1 },
      { clip: b.motion, weight: t, timeScale: b.timeScale ?? 1 },
    ];
  }

  /** A state's length: its clip's, or its blend's weighted lengths. */
  private lengthOf(state: AnimatorState): number {
    return this.clips(state).reduce((l, c) => l + c.weight * c.clip.length / c.timeScale, 0);
  }

  private stateSpeed(state: AnimatorState): number {
    const factor = state.speedParameter ? this.getFloat(state.speedParameter) : 1;
    return (state.speed ?? 1) * factor * this.speed;
  }

  private passes(c: AnimatorCondition): boolean {
    const v = this.values.get(c.parameter);
    const n = Number(v ?? 0);
    const t = c.threshold ?? 0;
    switch (c.mode) {
      case "if":
        return v === true;
      case "ifNot":
        return v !== true;
      case "greater":
        return n > t;
      case "less":
        return n < t;
      case "equals":
        return n === t;
      case "notEqual":
        return n !== t;
    }
  }

  /** Starts the first transition out of the current state whose conditions hold. */
  private checkTransitions(): void {
    if (!this.controller || !this.current || this.next) return;
    const from = this.current;
    const normalized = this.lengthOf(from.state) > 0 ? from.norm : 1;
    for (const t of this.controller.transitions) {
      if (t.from !== from.state.name && !(t.from === "Any" && t.to !== from.state.name)) continue;
      if (t.hasExitTime && normalized < (t.exitTime ?? 1)) continue;
      const conditions = t.conditions ?? [];
      if (!t.hasExitTime && conditions.length === 0) continue;
      if (!conditions.every((c) => this.passes(c))) continue;
      // Triggers are used up.
      for (const c of conditions) {
        if (this.controller.parameters.find((p) => p.name === c.parameter)?.type === "trigger") this.values.set(c.parameter, false);
      }
      const state = this.controller.states.find((s) => s.name === t.to);
      if (!state) continue;
      const duration = t.duration ?? 0.25;
      const start = t.offset ?? 0;
      if (duration <= 0) this.current = { state, norm: start };
      else this.next = { state, norm: start, progress: 0, duration };
      return;
    }
  }

  /** @internal Advances by `dt` and poses the object. */
  _animate(dt: number): void {
    this.ensure();
    if (!this.current) return;
    this.checkTransitions();
    const pose = new Pose();
    const run = (p: Playing, weight: number) => {
      const before = p.norm;
      const length = this.lengthOf(p.state);
      if (length > 0) p.norm += (dt * this.stateSpeed(p.state)) / length;
      for (const c of this.clips(p.state)) {
        // Blended clips share the state's normalized time, so their steps stay in phase.
        const t = length > 0 ? p.norm * c.clip.length : 0;
        const t0 = length > 0 ? before * c.clip.length : 0;
        const mode = c.clip.wrapMode === "once" ? "clampForever" : c.clip.wrapMode;
        if (c.weight * weight >= 0.5) fireEvents(this.gameObject, c.clip, t0, t, c.clip.length, mode);
        pose.addAligned(this.gameObject, c.clip, wrap(t, c.clip.length, mode), c.weight * weight);
      }
    };
    if (this.next) {
      this.next.progress = Math.min(1, this.next.progress + dt / this.next.duration);
      const w = this.next.progress;
      run(this.current, 1 - w);
      run(this.next, w);
      if (w >= 1) {
        this.current = { state: this.next.state, norm: this.next.norm };
        this.next = null;
      }
    } else {
      run(this.current, 1);
    }
    pose.apply();
  }
}

/** Every frame, after `onUpdate`: players advance and pose their objects. */
globalThis.__platypus_animate = (dt: number) => {
  const visit = (go: GameObject) => {
    if (!go.activeInHierarchy) return;
    for (const c of go.components) {
      if (c instanceof Animation || c instanceof Animator) {
        try {
          c._animate(dt);
        } catch (e) {
          console.error(`animation on ${go.name}: ${e}`);
        }
      }
    }
    for (const child of go.children) visit(child);
  };
  for (const root of GameObject.roots) visit(root);
};
