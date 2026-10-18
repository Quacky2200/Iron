// Splines (Unity's Splines package, the core of it): a path through knots, smooth unless told
// otherwise. A knot's tangents are automatic (a cardinal spline through its neighbours;
// `tension` 0.5 is Catmull-Rom, like GDI's curves), set by hand as Bézier handles, or left
// out (straight segments). Every segment is a cubic Bézier.
//
// `evaluate(t)` goes by distance (t 0 to 1 along the whole length), so things move along a
// spline at a steady speed; `segmentPoint(i, u)` goes by each segment's own parameter.
// `sample` gives points for `LineRenderer2D`; `SplineAnimate` moves an object along a spline.

import { Quaternion, Vector3, type Vector3Like } from "./math.ts";
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

/** Length-table samples per segment (more: closer to true distances). */
const STEPS = 32;

export class Spline {
  private _knots: SplineKnot[] = [];
  private _closed: boolean;
  private _tension: number;
  /** Distance along the spline at each table sample (`STEPS` per segment, plus the end). */
  private table: number[] | null = null;
  /** Goes up on every change, so anything drawing the spline can tell it moved. */
  version = 0;

  constructor(points: readonly Vector3Like[] = [], options: { closed?: boolean; tension?: number; mode?: TangentMode } = {}) {
    this._closed = options.closed ?? false;
    this._tension = options.tension ?? 0.5;
    for (const p of points) this.addKnot(p, options.mode ?? "auto");
  }

  get knots(): readonly SplineKnot[] {
    return this._knots;
  }

  get count(): number {
    return this._knots.length;
  }

  /** Whether the last knot joins back to the first. */
  get closed(): boolean {
    return this._closed;
  }
  set closed(on: boolean) {
    this._closed = on;
    this.changed();
  }

  /** How strongly `auto` knots bend: 0 is straight lines, 0.5 Catmull-Rom. */
  get tension(): number {
    return this._tension;
  }
  set tension(t: number) {
    this._tension = t;
    this.changed();
  }

  get segmentCount(): number {
    const n = this._knots.length;
    if (n < 2) return 0;
    return this._closed ? n : n - 1;
  }

  /** Adds a knot at the end; returns its index. */
  addKnot(position: Vector3Like, mode: TangentMode = "auto", tangentIn?: Vector3Like, tangentOut?: Vector3Like): number {
    return this.insertKnot(this._knots.length, position, mode, tangentIn, tangentOut);
  }

  /** Puts a knot before index `index` (the count: at the end); returns its index. */
  insertKnot(index: number, position: Vector3Like, mode: TangentMode = "auto", tangentIn?: Vector3Like, tangentOut?: Vector3Like): number {
    const i = Math.max(0, Math.min(this._knots.length, Math.trunc(index)));
    const out = tangentOut ? Vector3.from(tangentOut) : Vector3.zero;
    this._knots.splice(i, 0, {
      position: Vector3.from(position),
      tangentIn: tangentIn ? Vector3.from(tangentIn) : out.scale(-1),
      tangentOut: out,
      mode,
    });
    this.changed();
    return i;
  }

  removeKnot(index: number): void {
    if (index < 0 || index >= this._knots.length) return;
    this._knots.splice(index, 1);
    this.changed();
  }

  setPosition(index: number, position: Vector3Like): void {
    const k = this._knots[index];
    if (!k) return;
    k.position = Vector3.from(position);
    this.changed();
  }

  /** Sets a knot's handles (relative to it) and makes it a `bezier` knot. */
  setTangents(index: number, tangentIn: Vector3Like, tangentOut: Vector3Like): void {
    const k = this._knots[index];
    if (!k) return;
    k.tangentIn = Vector3.from(tangentIn);
    k.tangentOut = Vector3.from(tangentOut);
    k.mode = "bezier";
    this.changed();
  }

  /** Changes a knot's mode; a knot turning `bezier` keeps the handles it had (auto ones included). */
  setMode(index: number, mode: TangentMode): void {
    const k = this._knots[index];
    if (!k || k.mode === mode) return;
    if (mode === "bezier" && k.mode !== "bezier") {
      k.tangentIn = this.handleIn(index);
      k.tangentOut = this.handleOut(index);
    }
    k.mode = mode;
    this.changed();
  }

  /** A knot's handle after it, relative to it, whatever its mode (zero when straight). */
  handleOut(index: number): Vector3 {
    const k = this._knots[index];
    if (!k) return Vector3.zero;
    if (k.mode === "bezier") return k.tangentOut;
    // Straight: a third of the way to the next knot, so the segment keeps an even speed.
    if (k.mode === "linear") return this.neighbour(index, 1).sub(k.position).scale(1 / 3);
    // GDI+'s cardinal handles: a third of `tension` times the chord across the knot.
    return this.neighbour(index, 1).sub(this.neighbour(index, -1)).scale(this._tension / 3);
  }

  /** A knot's handle before it, relative to it, whatever its mode. */
  handleIn(index: number): Vector3 {
    const k = this._knots[index];
    if (!k) return Vector3.zero;
    if (k.mode === "bezier") return k.tangentIn;
    if (k.mode === "linear") return this.neighbour(index, -1).sub(k.position).scale(1 / 3);
    return this.handleOut(index).scale(-1);
  }

  /** The knot `step` along from `index` (wrapping when closed, else the end knot itself). */
  private neighbour(index: number, step: number): Vector3 {
    const n = this._knots.length;
    const i = index + step;
    return (this._closed ? this._knots[((i % n) + n) % n]! : this._knots[Math.max(0, Math.min(n - 1, i))]!).position;
  }

  /** Segment `i`'s four Bézier points: from knot `i` to the next. */
  private controls(i: number): [Vector3, Vector3, Vector3, Vector3] {
    const n = this._knots.length;
    const j = (i + 1) % n;
    const p0 = this._knots[i]!.position;
    const p3 = this._knots[j]!.position;
    return [p0, p0.add(this.handleOut(i)), p3.add(this.handleIn(j)), p3];
  }

  /** A point on segment `i` (0 to `segmentCount` - 1), `u` 0 to 1 by the segment's own parameter. */
  segmentPoint(i: number, u: number): Vector3 {
    if (this.segmentCount === 0) return this._knots[0]?.position ?? Vector3.zero;
    const [p0, p1, p2, p3] = this.controls(Math.max(0, Math.min(this.segmentCount - 1, i)));
    const v = 1 - u;
    return p0
      .scale(v * v * v)
      .add(p1.scale(3 * v * v * u))
      .add(p2.scale(3 * v * u * u))
      .add(p3.scale(u * u * u));
  }

  /** The direction (not normalized) segment `i` is going at `u`. */
  segmentDerivative(i: number, u: number): Vector3 {
    if (this.segmentCount === 0) return Vector3.zero;
    const [p0, p1, p2, p3] = this.controls(Math.max(0, Math.min(this.segmentCount - 1, i)));
    const v = 1 - u;
    const d = p1
      .sub(p0)
      .scale(3 * v * v)
      .add(p2.sub(p1).scale(6 * v * u))
      .add(p3.sub(p2).scale(3 * u * u));
    // Where the handles sit on the knots, the direction comes from the chord.
    return d.magnitude > 1e-9 ? d : p3.sub(p0);
  }

  private lengths(): number[] {
    if (this.table) return this.table;
    const table = [0];
    let total = 0;
    for (let i = 0; i < this.segmentCount; i++) {
      let prev = this.segmentPoint(i, 0);
      for (let s = 1; s <= STEPS; s++) {
        const p = this.segmentPoint(i, s / STEPS);
        total += p.distance(prev);
        table.push(total);
        prev = p;
      }
    }
    this.table = table;
    return table;
  }

  /** The whole length. */
  get length(): number {
    const t = this.lengths();
    return t[t.length - 1]!;
  }

  /** Where `t` (0 to 1 by distance) falls: a segment and its own parameter. */
  locate(t: number): { segment: number; u: number } {
    const segments = this.segmentCount;
    if (segments === 0) return { segment: 0, u: 0 };
    const table = this.lengths();
    const total = table[table.length - 1]!;
    const want = Math.max(0, Math.min(1, t)) * total;
    // The first sample at or past `want` (binary search over the increasing distances).
    let lo = 0;
    let hi = table.length - 1;
    while (lo < hi) {
      const mid = (lo + hi) >> 1;
      if (table[mid]! < want) lo = mid + 1;
      else hi = mid;
    }
    const j = Math.max(1, lo);
    const a = table[j - 1]!;
    const b = table[j]!;
    const frac = b > a ? (want - a) / (b - a) : 0;
    const step = j - 1 + frac;
    const segment = Math.min(segments - 1, Math.floor(step / STEPS));
    return { segment, u: (step - segment * STEPS) / STEPS };
  }

  /** The point `t` of the way along (0 to 1, by distance). */
  evaluate(t: number): Vector3 {
    const { segment, u } = this.locate(t);
    return this.segmentPoint(segment, u);
  }

  /** The direction the spline goes at `t` (by distance), normalized. */
  tangent(t: number): Vector3 {
    const { segment, u } = this.locate(t);
    return this.segmentDerivative(segment, u).normalized;
  }

  /** Points along the whole spline, `perSegment` to a segment; open ones include the last knot. */
  sample(perSegment = 16): Vector3[] {
    const out: Vector3[] = [];
    const segments = this.segmentCount;
    if (segments === 0) return this._knots.map((k) => k.position);
    for (let i = 0; i < segments; i++) {
      for (let s = 0; s < perSegment; s++) out.push(this.segmentPoint(i, s / perSegment));
    }
    if (!this._closed) out.push(this._knots[this._knots.length - 1]!.position);
    return out;
  }

  /** The nearest point on the spline to `p`: its `t` (by distance), the point and how far. */
  nearest(p: Vector3Like): { t: number; point: Vector3; distance: number } {
    const target = Vector3.from(p);
    const total = this.length;
    if (this.segmentCount === 0 || total === 0) {
      const point = this._knots[0]?.position ?? Vector3.zero;
      return { t: 0, point, distance: point.distance(target) };
    }
    // Coarse: every table sample; then narrow down around the best.
    const samples = this.segmentCount * STEPS;
    let best = 0;
    let bestD = Infinity;
    for (let s = 0; s <= samples; s++) {
      const d = this.evaluate(s / samples).distance(target);
      if (d < bestD) {
        bestD = d;
        best = s / samples;
      }
    }
    let step = 1 / samples;
    for (let k = 0; k < 20; k++) {
      step /= 2;
      for (const t of [best - step, best + step]) {
        const c = this._closed ? ((t % 1) + 1) % 1 : Math.max(0, Math.min(1, t));
        const d = this.evaluate(c).distance(target);
        if (d < bestD) {
          bestD = d;
          best = c;
        }
      }
    }
    const point = this.evaluate(best);
    return { t: best, point, distance: point.distance(target) };
  }

  private changed(): void {
    this.table = null;
    this.version++;
  }
}

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
  @property loopMode: "loop" | "pingPong" | "once" = "loop";
  @property alignment: "none" | "path" = "path";
  @property playing = true;
  /** Seconds played so far (set it to jump). */
  @property elapsed = 0;

  /** How far along the path it is now (0 to 1, by distance). */
  get t(): number {
    const d = Math.max(1e-6, this.duration);
    const x = this.elapsed / d;
    if (this.loopMode === "once") return Math.max(0, Math.min(1, x));
    if (this.loopMode === "pingPong") {
      const m = ((x % 2) + 2) % 2;
      return m > 1 ? 2 - m : m;
    }
    return ((x % 1) + 1) % 1;
  }

  override onUpdate(dt: number): void {
    if (this.playing) this.elapsed += dt;
    this.place();
  }

  /** Puts the object where `t` says (call it after moving the spline mid-frame). */
  place(): void {
    const s = this.spline;
    if (!s || s.count === 0) return;
    const t = this.t;
    const at = s.evaluate(t);
    let dir = s.tangent(t);
    // Going back along a ping-pong, face the other way.
    if (this.loopMode === "pingPong" && ((this.elapsed / Math.max(1e-6, this.duration)) % 2 + 2) % 2 > 1) dir = dir.scale(-1);
    const c = this.container?.transform;
    this.transform.position = c ? c.transformPoint(at) : at;
    if (this.alignment !== "path" || dir.magnitude < 1e-9) return;
    const d = c ? c.transformDirection(dir) : dir;
    this.transform.rotation =
      Math.abs(d.z) < 1e-6 ? Quaternion.euler(0, 0, (Math.atan2(d.y, d.x) * 180) / Math.PI) : Quaternion.lookRotation(d, Vector3.up);
  }
}
