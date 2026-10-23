// SPDX-License-Identifier: Apache-2.0

// Value types for gameplay code, named and behaving like Unity's (and s&box's):
// `Vector2`, `Vector3`, `Vector4`, `Vector2Int`/`Vector3Int` (points on a grid),
// `Quaternion` (Euler angles in degrees), `Rect`, `RectInt`, `Bounds`, `Ray`, `Plane`,
// `Color` and `Mathf`. Instances are immutable: methods return new values, so
// `transform.position.add(v)` never changes the transform by accident.
// Units: metres, degrees for Euler angles, y up, right-handed (-z forward). Being
// right-handed, positive pitch (about +x) tips forward up, where Unity's looks down.

const DEG = Math.PI / 180;

/** Anything with x and y. */
export interface Vector2Like {
  readonly x: number;
  readonly y: number;
}

/** Anything with x, y and z. */
export interface Vector3Like {
  readonly x: number;
  readonly y: number;
  readonly z: number;
}

/** Anything with x, y, z and w. */
export interface QuaternionLike {
  readonly x: number;
  readonly y: number;
  readonly z: number;
  readonly w: number;
}

export const Mathf = {
  PI: Math.PI,
  Deg2Rad: DEG,
  Rad2Deg: 1 / DEG,
  Epsilon: 1e-6,
  clamp: (v: number, min: number, max: number) => Math.min(max, Math.max(min, v)),
  clamp01: (v: number) => Math.min(1, Math.max(0, v)),
  lerp: (a: number, b: number, t: number) => a + (b - a) * Math.min(1, Math.max(0, t)),
  lerpUnclamped: (a: number, b: number, t: number) => a + (b - a) * t,
  inverseLerp: (a: number, b: number, v: number) => (a === b ? 0 : Math.min(1, Math.max(0, (v - a) / (b - a)))),
  moveTowards: (current: number, target: number, maxDelta: number) =>
    Math.abs(target - current) <= maxDelta ? target : current + Math.sign(target - current) * maxDelta,
  smoothStep: (from: number, to: number, t: number) => {
    const x = Math.min(1, Math.max(0, t));
    return from + (to - from) * x * x * (3 - 2 * x);
  },
  /** The shortest difference between two angles in degrees, -180 to 180. */
  deltaAngle: (a: number, b: number) => {
    const d = (((b - a) % 360) + 540) % 360 - 180;
    return d === -180 ? 180 : d;
  },
  repeat: (t: number, length: number) => t - Math.floor(t / length) * length,
  pingPong: (t: number, length: number) => {
    const r = t - Math.floor(t / (length * 2)) * length * 2;
    return length - Math.abs(r - length);
  },
  approximately: (a: number, b: number) => Math.abs(a - b) < Math.max(1e-6 * Math.max(Math.abs(a), Math.abs(b)), 1e-6),
  /** 1 for zero or more, -1 below (Unity's `Sign`, which is never 0). */
  sign: (v: number) => (v >= 0 ? 1 : -1),
  /** Lerps between angles in degrees the short way round. */
  lerpAngle: (a: number, b: number, t: number) => a + Mathf.deltaAngle(a, b) * Math.min(1, Math.max(0, t)),
  /** Turns from `current` towards `target` (degrees) by at most `maxDelta`. */
  moveTowardsAngle: (current: number, target: number, maxDelta: number) => {
    const d = Mathf.deltaAngle(current, target);
    return Math.abs(d) <= maxDelta ? current + d : current + Math.sign(d) * maxDelta;
  },
  /**
   * Eases `current` towards `target` like a critically damped spring, taking about
   * `smoothTime` seconds. Pass the returned velocity back in on the next call:
   * `[x, v] = Mathf.smoothDamp(x, goal, v, 0.3, Time.deltaTime)`.
   */
  smoothDamp: (current: number, target: number, velocity: number, smoothTime: number, deltaTime: number, maxSpeed = Infinity): [number, number] => {
    const st = Math.max(0.0001, smoothTime);
    const omega = 2 / st;
    const x = omega * deltaTime;
    const exp = 1 / (1 + x + 0.48 * x * x + 0.235 * x * x * x);
    const maxChange = maxSpeed * st;
    const change = Math.min(maxChange, Math.max(-maxChange, current - target));
    const goal = current - change;
    const temp = (velocity + omega * change) * deltaTime;
    let v = (velocity - omega * temp) * exp;
    let out = goal + (change + temp) * exp;
    // Don't overshoot.
    if (target - current > 0 === out > target) {
      out = target;
      v = (out - target) / deltaTime;
    }
    return [out, v];
  },
  /** Smooth 2D noise, about 0 to 1 (Unity's `PerlinNoise`). Same inputs, same output. */
  perlinNoise: (x: number, y: number) => perlin2(x, y) * 0.5 + 0.5,
  isPowerOfTwo: (v: number) => v > 0 && (v & (v - 1)) === 0,
  nextPowerOfTwo: (v: number) => (v <= 1 ? 1 : 2 ** Math.ceil(Math.log2(v))),
  closestPowerOfTwo: (v: number) => (v <= 1 ? 1 : 2 ** Math.round(Math.log2(v))),
  /** sRGB (as written in colour pickers) to linear and back, per channel. */
  gammaToLinearSpace: (c: number) => (c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4),
  linearToGammaSpace: (c: number) => (c <= 0.0031308 ? c * 12.92 : 1.055 * c ** (1 / 2.4) - 0.055),
};

// Classic Perlin noise on a fixed permutation (Ken Perlin's reference table).
const PERM = (() => {
  const p = [
    151, 160, 137, 91, 90, 15, 131, 13, 201, 95, 96, 53, 194, 233, 7, 225, 140, 36, 103, 30, 69, 142, 8, 99, 37, 240,
    21, 10, 23, 190, 6, 148, 247, 120, 234, 75, 0, 26, 197, 62, 94, 252, 219, 203, 117, 35, 11, 32, 57, 177, 33, 88, 237,
    149, 56, 87, 174, 20, 125, 136, 171, 168, 68, 175, 74, 165, 71, 134, 139, 48, 27, 166, 77, 146, 158, 231, 83, 111,
    229, 122, 60, 211, 133, 230, 220, 105, 92, 41, 55, 46, 245, 40, 244, 102, 143, 54, 65, 25, 63, 161, 1, 216, 80, 73,
    209, 76, 132, 187, 208, 89, 18, 169, 200, 196, 135, 130, 116, 188, 159, 86, 164, 100, 109, 198, 173, 186, 3, 64, 52,
    217, 226, 250, 124, 123, 5, 202, 38, 147, 118, 126, 255, 82, 85, 212, 207, 206, 59, 227, 47, 16, 58, 17, 182, 189,
    28, 42, 223, 183, 170, 213, 119, 248, 152, 2, 44, 154, 163, 70, 221, 153, 101, 155, 167, 43, 172, 9, 129, 22, 39, 253,
    19, 98, 108, 110, 79, 113, 224, 232, 178, 185, 112, 104, 218, 246, 97, 228, 251, 34, 242, 193, 238, 210, 144, 12, 191,
    179, 162, 241, 81, 51, 145, 235, 249, 14, 239, 107, 49, 192, 214, 31, 181, 199, 106, 157, 184, 84, 204, 176, 115, 121,
    50, 45, 127, 4, 150, 254, 138, 236, 205, 93, 222, 114, 67, 29, 24, 72, 243, 141, 128, 195, 78, 66, 215, 61, 156, 180,
  ];
  return [...p, ...p];
})();

function perlin2(x: number, y: number): number {
  const xi = Math.floor(x) & 255;
  const yi = Math.floor(y) & 255;
  const xf = x - Math.floor(x);
  const yf = y - Math.floor(y);
  const fade = (t: number) => t * t * t * (t * (t * 6 - 15) + 10);
  const grad = (h: number, gx: number, gy: number) => ((h & 1) === 0 ? gx : -gx) + ((h & 2) === 0 ? gy : -gy);
  const u = fade(xf);
  const v = fade(yf);
  const aa = PERM[PERM[xi]! + yi]!;
  const ab = PERM[PERM[xi]! + yi + 1]!;
  const ba = PERM[PERM[xi + 1]! + yi]!;
  const bb = PERM[PERM[xi + 1]! + yi + 1]!;
  const x1 = grad(aa, xf, yf) + (grad(ba, xf - 1, yf) - grad(aa, xf, yf)) * u;
  const x2 = grad(ab, xf, yf - 1) + (grad(bb, xf - 1, yf - 1) - grad(ab, xf, yf - 1)) * u;
  // The 2D gradients above reach about ±1.
  return x1 + (x2 - x1) * v;
}

export class Vector2 implements Vector2Like {
  constructor(
    readonly x = 0,
    readonly y = 0,
  ) {}

  static readonly zero = new Vector2(0, 0);
  static readonly one = new Vector2(1, 1);
  static readonly up = new Vector2(0, 1);
  static readonly down = new Vector2(0, -1);
  static readonly left = new Vector2(-1, 0);
  static readonly right = new Vector2(1, 0);

  static from(v: Vector2Like): Vector2 {
    return v instanceof Vector2 ? v : new Vector2(v.x, v.y);
  }

  add(v: Vector2Like): Vector2 {
    return new Vector2(this.x + v.x, this.y + v.y);
  }
  sub(v: Vector2Like): Vector2 {
    return new Vector2(this.x - v.x, this.y - v.y);
  }
  scale(s: number): Vector2 {
    return new Vector2(this.x * s, this.y * s);
  }
  /** Component-wise product. */
  mul(v: Vector2Like): Vector2 {
    return new Vector2(this.x * v.x, this.y * v.y);
  }
  dot(v: Vector2Like): number {
    return this.x * v.x + this.y * v.y;
  }
  get magnitude(): number {
    return Math.hypot(this.x, this.y);
  }
  get sqrMagnitude(): number {
    return this.x * this.x + this.y * this.y;
  }
  /** Length 1 in the same direction (zero stays zero). */
  get normalized(): Vector2 {
    const m = this.magnitude;
    return m > 0 ? this.scale(1 / m) : Vector2.zero;
  }
  distance(v: Vector2Like): number {
    return Math.hypot(this.x - v.x, this.y - v.y);
  }
  lerp(v: Vector2Like, t: number): Vector2 {
    const k = Mathf.clamp01(t);
    return new Vector2(this.x + (v.x - this.x) * k, this.y + (v.y - this.y) * k);
  }
  moveTowards(target: Vector2Like, maxDistance: number): Vector2 {
    const d = Vector2.from(target).sub(this);
    const m = d.magnitude;
    return m <= maxDistance || m === 0 ? Vector2.from(target) : this.add(d.scale(maxDistance / m));
  }
  /** Rotated counter-clockwise by `degrees`. */
  rotate(degrees: number): Vector2 {
    const c = Math.cos(degrees * DEG);
    const s = Math.sin(degrees * DEG);
    return new Vector2(this.x * c - this.y * s, this.x * s + this.y * c);
  }
  /** The angle between two directions, 0 to 180 degrees. */
  static angle(a: Vector2Like, b: Vector2Like): number {
    const d = Vector2.from(a).magnitude * Vector2.from(b).magnitude;
    return d === 0 ? 0 : Math.acos(Mathf.clamp(Vector2.from(a).dot(b) / d, -1, 1)) / DEG;
  }
  /** Signed angle from `a` to `b`, -180 to 180 degrees (counter-clockwise positive). */
  static signedAngle(a: Vector2Like, b: Vector2Like): number {
    return Math.atan2(a.x * b.y - a.y * b.x, a.x * b.x + a.y * b.y) / DEG;
  }
  equals(v: Vector2Like, epsilon = 1e-6): boolean {
    return Math.abs(this.x - v.x) <= epsilon && Math.abs(this.y - v.y) <= epsilon;
  }
  toVector3(z = 0): Vector3 {
    return new Vector3(this.x, this.y, z);
  }
  toArray(): [number, number] {
    return [this.x, this.y];
  }
  toString(): string {
    return `(${this.x}, ${this.y})`;
  }
}

export class Vector3 implements Vector3Like {
  constructor(
    readonly x = 0,
    readonly y = 0,
    readonly z = 0,
  ) {}

  static readonly zero = new Vector3(0, 0, 0);
  static readonly one = new Vector3(1, 1, 1);
  static readonly up = new Vector3(0, 1, 0);
  static readonly down = new Vector3(0, -1, 0);
  static readonly left = new Vector3(-1, 0, 0);
  static readonly right = new Vector3(1, 0, 0);
  /** -z: where cameras and objects look with no rotation. */
  static readonly forward = new Vector3(0, 0, -1);
  static readonly back = new Vector3(0, 0, 1);

  static from(v: Vector3Like): Vector3 {
    return v instanceof Vector3 ? v : new Vector3(v.x, v.y, v.z);
  }

  add(v: Vector3Like): Vector3 {
    return new Vector3(this.x + v.x, this.y + v.y, this.z + v.z);
  }
  sub(v: Vector3Like): Vector3 {
    return new Vector3(this.x - v.x, this.y - v.y, this.z - v.z);
  }
  scale(s: number): Vector3 {
    return new Vector3(this.x * s, this.y * s, this.z * s);
  }
  /** Component-wise product. */
  mul(v: Vector3Like): Vector3 {
    return new Vector3(this.x * v.x, this.y * v.y, this.z * v.z);
  }
  /** Component-wise quotient. */
  div(v: Vector3Like): Vector3 {
    return new Vector3(this.x / v.x, this.y / v.y, this.z / v.z);
  }
  negate(): Vector3 {
    return new Vector3(-this.x, -this.y, -this.z);
  }
  dot(v: Vector3Like): number {
    return this.x * v.x + this.y * v.y + this.z * v.z;
  }
  cross(v: Vector3Like): Vector3 {
    return new Vector3(this.y * v.z - this.z * v.y, this.z * v.x - this.x * v.z, this.x * v.y - this.y * v.x);
  }
  get magnitude(): number {
    return Math.hypot(this.x, this.y, this.z);
  }
  get sqrMagnitude(): number {
    return this.x * this.x + this.y * this.y + this.z * this.z;
  }
  /** Length 1 in the same direction (zero stays zero). */
  get normalized(): Vector3 {
    const m = this.magnitude;
    return m > 0 ? this.scale(1 / m) : Vector3.zero;
  }
  distance(v: Vector3Like): number {
    return Math.hypot(this.x - v.x, this.y - v.y, this.z - v.z);
  }
  lerp(v: Vector3Like, t: number): Vector3 {
    const k = Mathf.clamp01(t);
    return new Vector3(this.x + (v.x - this.x) * k, this.y + (v.y - this.y) * k, this.z + (v.z - this.z) * k);
  }
  moveTowards(target: Vector3Like, maxDistance: number): Vector3 {
    const d = Vector3.from(target).sub(this);
    const m = d.magnitude;
    return m <= maxDistance || m === 0 ? Vector3.from(target) : this.add(d.scale(maxDistance / m));
  }
  /** This vector projected onto `normal`'s direction. */
  project(onto: Vector3Like): Vector3 {
    const n = Vector3.from(onto);
    const sq = n.sqrMagnitude;
    return sq === 0 ? Vector3.zero : n.scale(this.dot(n) / sq);
  }
  /** This vector with its component along `normal` removed. */
  projectOnPlane(normal: Vector3Like): Vector3 {
    return this.sub(this.project(normal));
  }
  /** Bounced off a surface with this `normal` (unit length). */
  reflect(normal: Vector3Like): Vector3 {
    return this.sub(Vector3.from(normal).scale(2 * this.dot(normal)));
  }
  /** At most `max` long. */
  clampMagnitude(max: number): Vector3 {
    const m = this.magnitude;
    return m > max ? this.scale(max / m) : this;
  }
  /** The angle between two directions, 0 to 180 degrees. */
  static angle(a: Vector3Like, b: Vector3Like): number {
    const va = Vector3.from(a);
    const d = va.magnitude * Vector3.from(b).magnitude;
    return d === 0 ? 0 : Math.acos(Mathf.clamp(va.dot(b) / d, -1, 1)) / DEG;
  }
  equals(v: Vector3Like, epsilon = 1e-6): boolean {
    return Math.abs(this.x - v.x) <= epsilon && Math.abs(this.y - v.y) <= epsilon && Math.abs(this.z - v.z) <= epsilon;
  }
  toVector2(): Vector2 {
    return new Vector2(this.x, this.y);
  }
  toArray(): [number, number, number] {
    return [this.x, this.y, this.z];
  }
  toString(): string {
    return `(${this.x}, ${this.y}, ${this.z})`;
  }
}

export class Vector4 {
  constructor(
    readonly x = 0,
    readonly y = 0,
    readonly z = 0,
    readonly w = 0,
  ) {}
  static readonly zero = new Vector4();
  static readonly one = new Vector4(1, 1, 1, 1);
  /** A point (`w` 1) or, with `w` 0, a direction. */
  static fromVector3(v: Vector3Like, w = 1): Vector4 {
    return new Vector4(v.x, v.y, v.z, w);
  }
  add(v: Vector4): Vector4 {
    return new Vector4(this.x + v.x, this.y + v.y, this.z + v.z, this.w + v.w);
  }
  sub(v: Vector4): Vector4 {
    return new Vector4(this.x - v.x, this.y - v.y, this.z - v.z, this.w - v.w);
  }
  scale(s: number): Vector4 {
    return new Vector4(this.x * s, this.y * s, this.z * s, this.w * s);
  }
  /** Component-wise product. */
  mul(v: Vector4): Vector4 {
    return new Vector4(this.x * v.x, this.y * v.y, this.z * v.z, this.w * v.w);
  }
  dot(v: Vector4): number {
    return this.x * v.x + this.y * v.y + this.z * v.z + this.w * v.w;
  }
  get magnitude(): number {
    return Math.hypot(this.x, this.y, this.z, this.w);
  }
  get normalized(): Vector4 {
    const m = this.magnitude;
    return m > 0 ? this.scale(1 / m) : Vector4.zero;
  }
  lerp(v: Vector4, t: number): Vector4 {
    return this.add(v.sub(this).scale(Mathf.clamp01(t)));
  }
  equals(v: Vector4, epsilon = 1e-6): boolean {
    return this.sub(v).magnitude <= epsilon;
  }
  toVector3(): Vector3 {
    return new Vector3(this.x, this.y, this.z);
  }
  toArray(): [number, number, number, number] {
    return [this.x, this.y, this.z, this.w];
  }
  toString(): string {
    return `(${this.x}, ${this.y}, ${this.z}, ${this.w})`;
  }
}

/** A position and rotation together, like Unity's `Pose` (spawn points, attachments). */
export class Pose {
  readonly position: Vector3;
  readonly rotation: Quaternion;
  constructor(position: Vector3Like = Vector3.zero, rotation: QuaternionLike = Quaternion.identity) {
    this.position = Vector3.from(position);
    this.rotation = Quaternion.from(rotation);
  }
  // A getter: `Quaternion` is defined further down this file.
  static get identity(): Pose {
    return new Pose();
  }
  get forward(): Vector3 {
    return this.rotation.rotate(Vector3.forward);
  }
  get right(): Vector3 {
    return this.rotation.rotate(Vector3.right);
  }
  get up(): Vector3 {
    return this.rotation.rotate(Vector3.up);
  }
  /** This pose, given relative to `parent`, in the parent's space (Unity's `GetTransformedBy`). */
  transformedBy(parent: Pose): Pose {
    return new Pose(parent.position.add(parent.rotation.rotate(this.position)), parent.rotation.mul(this.rotation));
  }
  /** A point from this pose's local space to the world. */
  transformPoint(p: Vector3Like): Vector3 {
    return this.position.add(this.rotation.rotate(p));
  }
  inverseTransformPoint(p: Vector3Like): Vector3 {
    return this.rotation.inverse().rotate(Vector3.from(p).sub(this.position));
  }
  lerp(to: Pose, t: number): Pose {
    return new Pose(this.position.lerp(to.position, t), this.rotation.slerp(to.rotation, t));
  }
}

/** A point or offset on an integer grid (tiles, cells, pixels). */
export class Vector2Int implements Vector2Like {
  readonly x: number;
  readonly y: number;
  constructor(x = 0, y = 0) {
    this.x = Math.trunc(x);
    this.y = Math.trunc(y);
  }
  static readonly zero = new Vector2Int(0, 0);
  static readonly one = new Vector2Int(1, 1);
  static readonly up = new Vector2Int(0, 1);
  static readonly down = new Vector2Int(0, -1);
  static readonly left = new Vector2Int(-1, 0);
  static readonly right = new Vector2Int(1, 0);
  static floor(v: Vector2Like): Vector2Int {
    return new Vector2Int(Math.floor(v.x), Math.floor(v.y));
  }
  static round(v: Vector2Like): Vector2Int {
    return new Vector2Int(Math.round(v.x), Math.round(v.y));
  }
  add(v: Vector2Like): Vector2Int {
    return new Vector2Int(this.x + v.x, this.y + v.y);
  }
  sub(v: Vector2Like): Vector2Int {
    return new Vector2Int(this.x - v.x, this.y - v.y);
  }
  scale(s: number): Vector2Int {
    return new Vector2Int(this.x * s, this.y * s);
  }
  /** Grid distance in steps along the axes. */
  manhattan(v: Vector2Like): number {
    return Math.abs(this.x - v.x) + Math.abs(this.y - v.y);
  }
  equals(v: Vector2Like): boolean {
    return this.x === v.x && this.y === v.y;
  }
  toVector2(): Vector2 {
    return new Vector2(this.x, this.y);
  }
  toString(): string {
    return `(${this.x}, ${this.y})`;
  }
}

/** A point or offset on an integer 3D grid (voxels). */
export class Vector3Int implements Vector3Like {
  readonly x: number;
  readonly y: number;
  readonly z: number;
  constructor(x = 0, y = 0, z = 0) {
    this.x = Math.trunc(x);
    this.y = Math.trunc(y);
    this.z = Math.trunc(z);
  }
  static readonly zero = new Vector3Int(0, 0, 0);
  static readonly one = new Vector3Int(1, 1, 1);
  static floor(v: Vector3Like): Vector3Int {
    return new Vector3Int(Math.floor(v.x), Math.floor(v.y), Math.floor(v.z));
  }
  static round(v: Vector3Like): Vector3Int {
    return new Vector3Int(Math.round(v.x), Math.round(v.y), Math.round(v.z));
  }
  add(v: Vector3Like): Vector3Int {
    return new Vector3Int(this.x + v.x, this.y + v.y, this.z + v.z);
  }
  sub(v: Vector3Like): Vector3Int {
    return new Vector3Int(this.x - v.x, this.y - v.y, this.z - v.z);
  }
  equals(v: Vector3Like): boolean {
    return this.x === v.x && this.y === v.y && this.z === v.z;
  }
  toVector3(): Vector3 {
    return new Vector3(this.x, this.y, this.z);
  }
  toString(): string {
    return `(${this.x}, ${this.y}, ${this.z})`;
  }
}

export class Quaternion implements QuaternionLike {
  constructor(
    readonly x = 0,
    readonly y = 0,
    readonly z = 0,
    readonly w = 1,
  ) {}

  static readonly identity = new Quaternion(0, 0, 0, 1);

  static from(q: QuaternionLike): Quaternion {
    return q instanceof Quaternion ? q : new Quaternion(q.x, q.y, q.z, q.w);
  }

  /** A rotation of `degrees` about `axis`. */
  static angleAxis(degrees: number, axis: Vector3Like): Quaternion {
    const a = Vector3.from(axis).normalized;
    const half = (degrees * DEG) / 2;
    const s = Math.sin(half);
    return new Quaternion(a.x * s, a.y * s, a.z * s, Math.cos(half));
  }

  /**
   * Euler angles in degrees, like Unity: `z` about the forward axis, then `x` about
   * the right axis, then `y` about the up axis (roll, pitch, yaw).
   */
  static euler(x: number, y: number, z: number): Quaternion;
  static euler(angles: Vector3Like): Quaternion;
  static euler(x: number | Vector3Like, y = 0, z = 0): Quaternion {
    const a = typeof x === "number" ? new Vector3(x, y, z) : x;
    return Quaternion.angleAxis(a.y, Vector3.up)
      .mul(Quaternion.angleAxis(a.x, Vector3.right))
      .mul(Quaternion.angleAxis(a.z, new Vector3(0, 0, 1)));
  }

  /** This rotation as Euler angles in degrees (each 0 to 360), the inverse of `euler`. */
  get eulerAngles(): Vector3 {
    const { x, y, z, w } = this;
    // Rotation matrix terms for the YXZ order.
    const m12 = 2 * (y * z - w * x);
    const sx = Mathf.clamp(-m12, -1, 1);
    const ax = Math.asin(sx);
    let ay: number;
    let az: number;
    if (Math.abs(sx) < 0.9999) {
      ay = Math.atan2(2 * (x * z + w * y), 1 - 2 * (x * x + y * y));
      az = Math.atan2(2 * (x * y + w * z), 1 - 2 * (x * x + z * z));
    } else {
      // Gimbal lock: put it all in yaw.
      ay = Math.atan2(-2 * (x * z - w * y), 1 - 2 * (y * y + z * z));
      az = 0;
    }
    const wrap = (r: number) => Mathf.repeat(r / DEG, 360);
    return new Vector3(wrap(ax), wrap(ay), wrap(az));
  }

  /** A rotation that points forward (-z) along `direction`, keeping `up` as close to up as it can. */
  static lookRotation(direction: Vector3Like, up: Vector3Like = Vector3.up): Quaternion {
    const f = Vector3.from(direction).normalized;
    if (f.sqrMagnitude === 0) return Quaternion.identity;
    let r = f.cross(up).normalized;
    if (r.sqrMagnitude === 0) r = f.cross(Math.abs(f.x) < 0.9 ? Vector3.right : Vector3.up).normalized;
    const u = r.cross(f);
    // Matrix columns: right, up, back (-forward).
    const [m00, m01, m02] = [r.x, u.x, -f.x];
    const [m10, m11, m12] = [r.y, u.y, -f.y];
    const [m20, m21, m22] = [r.z, u.z, -f.z];
    const trace = m00 + m11 + m22;
    if (trace > 0) {
      const s = Math.sqrt(trace + 1) * 2;
      return new Quaternion((m21 - m12) / s, (m02 - m20) / s, (m10 - m01) / s, s / 4);
    }
    if (m00 > m11 && m00 > m22) {
      const s = Math.sqrt(1 + m00 - m11 - m22) * 2;
      return new Quaternion(s / 4, (m01 + m10) / s, (m02 + m20) / s, (m21 - m12) / s);
    }
    if (m11 > m22) {
      const s = Math.sqrt(1 + m11 - m00 - m22) * 2;
      return new Quaternion((m01 + m10) / s, s / 4, (m12 + m21) / s, (m02 - m20) / s);
    }
    const s = Math.sqrt(1 + m22 - m00 - m11) * 2;
    return new Quaternion((m02 + m20) / s, (m12 + m21) / s, s / 4, (m10 - m01) / s);
  }

  /** The rotation from one direction to another. */
  static fromToRotation(from: Vector3Like, to: Vector3Like): Quaternion {
    const a = Vector3.from(from).normalized;
    const b = Vector3.from(to).normalized;
    const d = a.dot(b);
    if (d < -0.999999) {
      let axis = Vector3.right.cross(a);
      if (axis.sqrMagnitude < 1e-6) axis = Vector3.up.cross(a);
      return Quaternion.angleAxis(180, axis);
    }
    const c = a.cross(b);
    return new Quaternion(c.x, c.y, c.z, 1 + d).normalized;
  }

  /** Rotation `q` followed by this one (like multiplying matrices: `this * q`). */
  mul(q: QuaternionLike): Quaternion {
    return new Quaternion(
      this.w * q.x + this.x * q.w + this.y * q.z - this.z * q.y,
      this.w * q.y - this.x * q.z + this.y * q.w + this.z * q.x,
      this.w * q.z + this.x * q.y - this.y * q.x + this.z * q.w,
      this.w * q.w - this.x * q.x - this.y * q.y - this.z * q.z,
    );
  }

  /** The opposite rotation (for unit quaternions). */
  inverse(): Quaternion {
    return new Quaternion(-this.x, -this.y, -this.z, this.w);
  }

  /** Rotates a vector. */
  rotate(v: Vector3Like): Vector3 {
    // v + 2w(q × v) + 2(q × (q × v))
    const q = new Vector3(this.x, this.y, this.z);
    const t = q.cross(v).scale(2);
    return Vector3.from(v).add(t.scale(this.w)).add(q.cross(t));
  }

  get normalized(): Quaternion {
    const len = Math.hypot(this.x, this.y, this.z, this.w);
    return len > 0 ? new Quaternion(this.x / len, this.y / len, this.z / len, this.w / len) : Quaternion.identity;
  }

  /** Spherical interpolation, the short way round (`t` clamped to 0..1). */
  slerp(q: QuaternionLike, t: number): Quaternion {
    return this.slerpBy(q, Mathf.clamp01(t));
  }

  /** Spherical interpolation from `a` to `b` by `t` (clamped to 0..1), the short way round (Unity's `Quaternion.Slerp`). */
  static slerp(a: QuaternionLike, b: QuaternionLike, t: number): Quaternion {
    return Quaternion.from(a).slerpBy(b, Mathf.clamp01(t));
  }

  /** As `slerp`, but `t` isn't clamped: past 1 it carries on turning the same way. */
  static slerpUnclamped(a: QuaternionLike, b: QuaternionLike, t: number): Quaternion {
    return Quaternion.from(a).slerpBy(b, t);
  }

  /** Linear interpolation, normalised (cheaper than `slerp`, not at constant speed); `t` clamped to 0..1. */
  static lerp(a: QuaternionLike, b: QuaternionLike, t: number): Quaternion {
    return Quaternion.lerpUnclamped(a, b, Mathf.clamp01(t));
  }

  /** As `lerp`, but `t` isn't clamped. */
  static lerpUnclamped(a: QuaternionLike, b: QuaternionLike, t: number): Quaternion {
    // The short way round: flip `b` when it's on the other side.
    const s = Quaternion.dot(a, b) < 0 ? -1 : 1;
    return new Quaternion(
      a.x + (s * b.x - a.x) * t,
      a.y + (s * b.y - a.y) * t,
      a.z + (s * b.z - a.z) * t,
      a.w + (s * b.w - a.w) * t,
    ).normalized;
  }

  /** Turns `from` towards `to` by at most `maxDegrees`. */
  static rotateTowards(from: QuaternionLike, to: QuaternionLike, maxDegrees: number): Quaternion {
    return Quaternion.from(from).rotateTowards(to, maxDegrees);
  }

  /** The dot product of two rotations (1 or -1: the same rotation). */
  static dot(a: QuaternionLike, b: QuaternionLike): number {
    return a.x * b.x + a.y * b.y + a.z * b.z + a.w * b.w;
  }

  private slerpBy(q: QuaternionLike, k: number): Quaternion {
    let { x, y, z, w } = q;
    let cos = this.x * x + this.y * y + this.z * z + this.w * w;
    if (cos < 0) {
      [x, y, z, w, cos] = [-x, -y, -z, -w, -cos];
    }
    if (cos > 0.9995) {
      return new Quaternion(
        this.x + (x - this.x) * k,
        this.y + (y - this.y) * k,
        this.z + (z - this.z) * k,
        this.w + (w - this.w) * k,
      ).normalized;
    }
    const angle = Math.acos(cos);
    const a = Math.sin((1 - k) * angle) / Math.sin(angle);
    const b = Math.sin(k * angle) / Math.sin(angle);
    return new Quaternion(this.x * a + x * b, this.y * a + y * b, this.z * a + z * b, this.w * a + w * b);
  }

  /** Turns towards `target` by at most `maxDegrees`. */
  rotateTowards(target: QuaternionLike, maxDegrees: number): Quaternion {
    const angle = Quaternion.angle(this, target);
    return angle === 0 ? Quaternion.from(target) : this.slerp(target, Math.min(1, maxDegrees / angle));
  }

  /** The angle between two rotations, in degrees. */
  static angle(a: QuaternionLike, b: QuaternionLike): number {
    const d = Math.abs(a.x * b.x + a.y * b.y + a.z * b.z + a.w * b.w);
    return (2 * Math.acos(Math.min(1, d))) / DEG;
  }

  toArray(): [number, number, number, number] {
    return [this.x, this.y, this.z, this.w];
  }
}

/**
 * A 4×4 matrix (column-major, like Unity's `Matrix4x4` and glam's `Mat4`), for
 * cameras and transforms. Projections match the renderer: right-handed, depth 0 to 1.
 */
export class Matrix4x4 {
  /** 16 numbers, column by column. */
  readonly m: readonly number[];

  constructor(m: readonly number[]) {
    if (m.length !== 16) throw new Error("Matrix4x4 needs 16 numbers");
    this.m = m;
  }

  static readonly identity = new Matrix4x4([1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1]);

  /** Element at `row`, `col`. */
  get(row: number, col: number): number {
    return this.m[col * 4 + row]!;
  }

  /** Translation, rotation and scale (applied scale, then rotation, then translation). */
  static trs(position: Vector3Like, rotation: QuaternionLike, scale: Vector3Like = Vector3.one): Matrix4x4 {
    const { x, y, z, w } = rotation;
    const [xx, yy, zz] = [x * x, y * y, z * z];
    const [xy, xz, yz, wx, wy, wz] = [x * y, x * z, y * z, w * x, w * y, w * z];
    return new Matrix4x4([
      (1 - 2 * (yy + zz)) * scale.x, 2 * (xy + wz) * scale.x, 2 * (xz - wy) * scale.x, 0,
      2 * (xy - wz) * scale.y, (1 - 2 * (xx + zz)) * scale.y, 2 * (yz + wx) * scale.y, 0,
      2 * (xz + wy) * scale.z, 2 * (yz - wx) * scale.z, (1 - 2 * (xx + yy)) * scale.z, 0,
      position.x, position.y, position.z, 1,
    ]);
  }

  /** A perspective projection: vertical field of view in degrees. */
  static perspective(fovDegrees: number, aspect: number, near: number, far: number): Matrix4x4 {
    const f = 1 / Math.tan((fovDegrees * DEG) / 2);
    const r = far / (near - far);
    return new Matrix4x4([f / aspect, 0, 0, 0, 0, f, 0, 0, 0, 0, r, -1, 0, 0, r * near, 0]);
  }

  /** An orthographic projection of the box between the planes. */
  static orthographic(left: number, right: number, bottom: number, top: number, near: number, far: number): Matrix4x4 {
    const rw = 1 / (right - left);
    const rh = 1 / (top - bottom);
    const r = 1 / (near - far);
    return new Matrix4x4([2 * rw, 0, 0, 0, 0, 2 * rh, 0, 0, 0, 0, r, 0, -(left + right) * rw, -(top + bottom) * rh, r * near, 1]);
  }

  /** `this * other`: `other` applies first. */
  mul(other: Matrix4x4): Matrix4x4 {
    const a = this.m;
    const b = other.m;
    const out = new Array<number>(16);
    for (let c = 0; c < 4; c++) {
      for (let r = 0; r < 4; r++) {
        out[c * 4 + r] =
          a[r]! * b[c * 4]! + a[4 + r]! * b[c * 4 + 1]! + a[8 + r]! * b[c * 4 + 2]! + a[12 + r]! * b[c * 4 + 3]!;
      }
    }
    return new Matrix4x4(out);
  }

  /** The inverse (identity-ish garbage if it has none; check `determinant`). */
  get inverse(): Matrix4x4 {
    const m = this.m as number[];
    const inv = new Array<number>(16);
    inv[0] = m[5]! * m[10]! * m[15]! - m[5]! * m[11]! * m[14]! - m[9]! * m[6]! * m[15]! + m[9]! * m[7]! * m[14]! + m[13]! * m[6]! * m[11]! - m[13]! * m[7]! * m[10]!;
    inv[4] = -m[4]! * m[10]! * m[15]! + m[4]! * m[11]! * m[14]! + m[8]! * m[6]! * m[15]! - m[8]! * m[7]! * m[14]! - m[12]! * m[6]! * m[11]! + m[12]! * m[7]! * m[10]!;
    inv[8] = m[4]! * m[9]! * m[15]! - m[4]! * m[11]! * m[13]! - m[8]! * m[5]! * m[15]! + m[8]! * m[7]! * m[13]! + m[12]! * m[5]! * m[11]! - m[12]! * m[7]! * m[9]!;
    inv[12] = -m[4]! * m[9]! * m[14]! + m[4]! * m[10]! * m[13]! + m[8]! * m[5]! * m[14]! - m[8]! * m[6]! * m[13]! - m[12]! * m[5]! * m[10]! + m[12]! * m[6]! * m[9]!;
    inv[1] = -m[1]! * m[10]! * m[15]! + m[1]! * m[11]! * m[14]! + m[9]! * m[2]! * m[15]! - m[9]! * m[3]! * m[14]! - m[13]! * m[2]! * m[11]! + m[13]! * m[3]! * m[10]!;
    inv[5] = m[0]! * m[10]! * m[15]! - m[0]! * m[11]! * m[14]! - m[8]! * m[2]! * m[15]! + m[8]! * m[3]! * m[14]! + m[12]! * m[2]! * m[11]! - m[12]! * m[3]! * m[10]!;
    inv[9] = -m[0]! * m[9]! * m[15]! + m[0]! * m[11]! * m[13]! + m[8]! * m[1]! * m[15]! - m[8]! * m[3]! * m[13]! - m[12]! * m[1]! * m[11]! + m[12]! * m[3]! * m[9]!;
    inv[13] = m[0]! * m[9]! * m[14]! - m[0]! * m[10]! * m[13]! - m[8]! * m[1]! * m[14]! + m[8]! * m[2]! * m[13]! + m[12]! * m[1]! * m[10]! - m[12]! * m[2]! * m[9]!;
    inv[2] = m[1]! * m[6]! * m[15]! - m[1]! * m[7]! * m[14]! - m[5]! * m[2]! * m[15]! + m[5]! * m[3]! * m[14]! + m[13]! * m[2]! * m[7]! - m[13]! * m[3]! * m[6]!;
    inv[6] = -m[0]! * m[6]! * m[15]! + m[0]! * m[7]! * m[14]! + m[4]! * m[2]! * m[15]! - m[4]! * m[3]! * m[14]! - m[12]! * m[2]! * m[7]! + m[12]! * m[3]! * m[6]!;
    inv[10] = m[0]! * m[5]! * m[15]! - m[0]! * m[7]! * m[13]! - m[4]! * m[1]! * m[15]! + m[4]! * m[3]! * m[13]! + m[12]! * m[1]! * m[7]! - m[12]! * m[3]! * m[5]!;
    inv[14] = -m[0]! * m[5]! * m[14]! + m[0]! * m[6]! * m[13]! + m[4]! * m[1]! * m[14]! - m[4]! * m[2]! * m[13]! - m[12]! * m[1]! * m[6]! + m[12]! * m[2]! * m[5]!;
    inv[3] = -m[1]! * m[6]! * m[11]! + m[1]! * m[7]! * m[10]! + m[5]! * m[2]! * m[11]! - m[5]! * m[3]! * m[10]! - m[9]! * m[2]! * m[7]! + m[9]! * m[3]! * m[6]!;
    inv[7] = m[0]! * m[6]! * m[11]! - m[0]! * m[7]! * m[10]! - m[4]! * m[2]! * m[11]! + m[4]! * m[3]! * m[10]! + m[8]! * m[2]! * m[7]! - m[8]! * m[3]! * m[6]!;
    inv[11] = -m[0]! * m[5]! * m[11]! + m[0]! * m[7]! * m[9]! + m[4]! * m[1]! * m[11]! - m[4]! * m[3]! * m[9]! - m[8]! * m[1]! * m[7]! + m[8]! * m[3]! * m[5]!;
    inv[15] = m[0]! * m[5]! * m[10]! - m[0]! * m[6]! * m[9]! - m[4]! * m[1]! * m[10]! + m[4]! * m[2]! * m[9]! + m[8]! * m[1]! * m[6]! - m[8]! * m[2]! * m[5]!;
    const det = m[0]! * inv[0]! + m[1]! * inv[4]! + m[2]! * inv[8]! + m[3]! * inv[12]!;
    if (Math.abs(det) < 1e-12) return Matrix4x4.identity;
    return new Matrix4x4(inv.map((v) => v / det));
  }

  get determinant(): number {
    const m = this.m;
    const sub = (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number) =>
      a * (e * i - f * h) - b * (d * i - f * g) + c * (d * h - e * g);
    return (
      m[0]! * sub(m[5]!, m[9]!, m[13]!, m[6]!, m[10]!, m[14]!, m[7]!, m[11]!, m[15]!) -
      m[4]! * sub(m[1]!, m[9]!, m[13]!, m[2]!, m[10]!, m[14]!, m[3]!, m[11]!, m[15]!) +
      m[8]! * sub(m[1]!, m[5]!, m[13]!, m[2]!, m[6]!, m[14]!, m[3]!, m[7]!, m[15]!) -
      m[12]! * sub(m[1]!, m[5]!, m[9]!, m[2]!, m[6]!, m[10]!, m[3]!, m[7]!, m[11]!)
    );
  }

  /** A point through the matrix, divided by w (for projections). */
  multiplyPoint(p: Vector3Like): Vector3 {
    const m = this.m;
    const x = m[0]! * p.x + m[4]! * p.y + m[8]! * p.z + m[12]!;
    const y = m[1]! * p.x + m[5]! * p.y + m[9]! * p.z + m[13]!;
    const z = m[2]! * p.x + m[6]! * p.y + m[10]! * p.z + m[14]!;
    const w = m[3]! * p.x + m[7]! * p.y + m[11]! * p.z + m[15]!;
    return w !== 0 && w !== 1 ? new Vector3(x / w, y / w, z / w) : new Vector3(x, y, z);
  }

  /** A point through the matrix, ignoring projection (faster, for transforms). */
  multiplyPoint3x4(p: Vector3Like): Vector3 {
    const m = this.m;
    return new Vector3(
      m[0]! * p.x + m[4]! * p.y + m[8]! * p.z + m[12]!,
      m[1]! * p.x + m[5]! * p.y + m[9]! * p.z + m[13]!,
      m[2]! * p.x + m[6]! * p.y + m[10]! * p.z + m[14]!,
    );
  }

  /** A direction through the matrix (no translation). */
  multiplyVector(v: Vector3Like): Vector3 {
    const m = this.m;
    return new Vector3(
      m[0]! * v.x + m[4]! * v.y + m[8]! * v.z,
      m[1]! * v.x + m[5]! * v.y + m[9]! * v.z,
      m[2]! * v.x + m[6]! * v.y + m[10]! * v.z,
    );
  }

  /** The full 4D product (w kept). */
  multiplyVector4(v: Vector4): Vector4 {
    const m = this.m;
    return new Vector4(
      m[0]! * v.x + m[4]! * v.y + m[8]! * v.z + m[12]! * v.w,
      m[1]! * v.x + m[5]! * v.y + m[9]! * v.z + m[13]! * v.w,
      m[2]! * v.x + m[6]! * v.y + m[10]! * v.z + m[14]! * v.w,
      m[3]! * v.x + m[7]! * v.y + m[11]! * v.z + m[15]! * v.w,
    );
  }

  get transpose(): Matrix4x4 {
    const out = new Array<number>(16);
    for (let r = 0; r < 4; r++) for (let c = 0; c < 4; c++) out[r * 4 + c] = this.m[c * 4 + r]!;
    return new Matrix4x4(out);
  }
}

/** A ray: an origin and a direction (unit length). */
export class Ray {
  readonly origin: Vector3;
  readonly direction: Vector3;
  constructor(origin: Vector3Like, direction: Vector3Like) {
    this.origin = Vector3.from(origin);
    this.direction = Vector3.from(direction).normalized;
  }
  getPoint(distance: number): Vector3 {
    return this.origin.add(this.direction.scale(distance));
  }
}

/** An infinite plane: points `p` with `normal · p + distance = 0`, like Unity's. */
export class Plane {
  readonly normal: Vector3;
  readonly distance: number;
  constructor(normal: Vector3Like, distance = 0) {
    this.normal = Vector3.from(normal).normalized;
    this.distance = distance;
  }
  /** The plane through `point` facing along `normal`. */
  static fromPoint(normal: Vector3Like, point: Vector3Like): Plane {
    const n = Vector3.from(normal).normalized;
    return new Plane(n, -n.dot(point));
  }
  /** Signed distance from the plane (positive on the side the normal faces). */
  getDistanceToPoint(point: Vector3Like): number {
    return this.normal.dot(point) + this.distance;
  }
  getSide(point: Vector3Like): boolean {
    return this.getDistanceToPoint(point) > 0;
  }
  closestPointOnPlane(point: Vector3Like): Vector3 {
    return Vector3.from(point).sub(this.normal.scale(this.getDistanceToPoint(point)));
  }
  /** How far along the ray it meets the plane, or `null` if it's parallel or behind. */
  raycast(ray: Ray): number | null {
    const denom = this.normal.dot(ray.direction);
    if (Math.abs(denom) < 1e-9) return null;
    const t = -this.getDistanceToPoint(ray.origin) / denom;
    return t >= 0 ? t : null;
  }
  /** Where the ray meets the plane, or `null`. */
  intersect(ray: Ray): Vector3 | null {
    const t = this.raycast(ray);
    return t === null ? null : ray.getPoint(t);
  }
  /** The plane through three points (counter-clockwise seen from the side the normal faces). */
  static fromPoints(a: Vector3Like, b: Vector3Like, c: Vector3Like): Plane {
    const va = Vector3.from(a);
    return Plane.fromPoint(Vector3.from(b).sub(va).cross(Vector3.from(c).sub(va)), va);
  }
  /** The same plane facing the other way. */
  get flipped(): Plane {
    return new Plane(this.normal.negate(), -this.distance);
  }
  /** This plane moved by `translation`. */
  translate(translation: Vector3Like): Plane {
    return new Plane(this.normal, this.distance - this.normal.dot(translation));
  }
  /** Whether two points are on the same side. */
  sameSide(a: Vector3Like, b: Vector3Like): boolean {
    const da = this.getDistanceToPoint(a);
    const db = this.getDistanceToPoint(b);
    return (da > 0 && db > 0) || (da <= 0 && db <= 0);
  }
  /** A direction flattened onto the plane. */
  projectVector(v: Vector3Like): Vector3 {
    return Vector3.from(v).projectOnPlane(this.normal);
  }
  /** Unity's `SetNormalAndPosition`, as a new plane. */
  static setNormalAndPosition(normal: Vector3Like, point: Vector3Like): Plane {
    return Plane.fromPoint(normal, point);
  }
}

/** A 2D rectangle from its top-left (`x`, `y`) and size. */
export class Rect {
  constructor(
    readonly x = 0,
    readonly y = 0,
    readonly width = 0,
    readonly height = 0,
  ) {}
  static minMax(min: Vector2Like, max: Vector2Like): Rect {
    return new Rect(min.x, min.y, max.x - min.x, max.y - min.y);
  }
  get xMin(): number {
    return this.x;
  }
  get yMin(): number {
    return this.y;
  }
  get xMax(): number {
    return this.x + this.width;
  }
  get yMax(): number {
    return this.y + this.height;
  }
  get center(): Vector2 {
    return new Vector2(this.x + this.width / 2, this.y + this.height / 2);
  }
  get size(): Vector2 {
    return new Vector2(this.width, this.height);
  }
  contains(p: Vector2Like): boolean {
    return p.x >= this.x && p.x < this.xMax && p.y >= this.y && p.y < this.yMax;
  }
  overlaps(r: Rect): boolean {
    return r.xMax > this.x && r.x < this.xMax && r.yMax > this.y && r.y < this.yMax;
  }
}

/** A rectangle on an integer grid. */
export class RectInt extends Rect {
  constructor(x = 0, y = 0, width = 0, height = 0) {
    super(Math.trunc(x), Math.trunc(y), Math.trunc(width), Math.trunc(height));
  }
  /** Every grid point inside, row by row. */
  *positions(): IterableIterator<Vector2Int> {
    for (let y = this.y; y < this.yMax; y++) for (let x = this.x; x < this.xMax; x++) yield new Vector2Int(x, y);
  }
}

/** An axis-aligned box from its centre and size. */
export class Bounds {
  readonly center: Vector3;
  readonly size: Vector3;
  constructor(center: Vector3Like = Vector3.zero, size: Vector3Like = Vector3.zero) {
    this.center = Vector3.from(center);
    this.size = Vector3.from(size);
  }
  static minMax(min: Vector3Like, max: Vector3Like): Bounds {
    const a = Vector3.from(min);
    return new Bounds(a.add(max).scale(0.5), Vector3.from(max).sub(a));
  }
  get extents(): Vector3 {
    return this.size.scale(0.5);
  }
  get min(): Vector3 {
    return this.center.sub(this.extents);
  }
  get max(): Vector3 {
    return this.center.add(this.extents);
  }
  contains(p: Vector3Like): boolean {
    const [a, b] = [this.min, this.max];
    return p.x >= a.x && p.x <= b.x && p.y >= a.y && p.y <= b.y && p.z >= a.z && p.z <= b.z;
  }
  intersects(o: Bounds): boolean {
    const [a, b, c, d] = [this.min, this.max, o.min, o.max];
    return a.x <= d.x && b.x >= c.x && a.y <= d.y && b.y >= c.y && a.z <= d.z && b.z >= c.z;
  }
  /** Grown to include `p`. */
  encapsulate(p: Vector3Like): Bounds {
    const [a, b] = [this.min, this.max];
    return Bounds.minMax(
      new Vector3(Math.min(a.x, p.x), Math.min(a.y, p.y), Math.min(a.z, p.z)),
      new Vector3(Math.max(b.x, p.x), Math.max(b.y, p.y), Math.max(b.z, p.z)),
    );
  }
}

/** A linear RGBA colour, components 0 to 1. */
export class Color {
  constructor(
    readonly r = 0,
    readonly g = 0,
    readonly b = 0,
    readonly a = 1,
  ) {}
  static readonly white = new Color(1, 1, 1);
  static readonly black = new Color(0, 0, 0);
  static readonly red = new Color(1, 0, 0);
  static readonly green = new Color(0, 1, 0);
  static readonly blue = new Color(0, 0, 1);
  static readonly yellow = new Color(1, 0.92, 0.016);
  static readonly clear = new Color(0, 0, 0, 0);
  /**
   * From a `#rgb`, `#rgba`, `#rrggbb` or `#rrggbbaa` hex string. Hex colours are sRGB
   * (as colour pickers and CSS write them), so they're converted to linear; alpha
   * stays as is.
   */
  static hex(hex: string): Color {
    let h = hex.replace(/^#/, "");
    if (h.length === 3 || h.length === 4) h = [...h].map((c) => c + c).join("");
    const n = (i: number) => Number.parseInt(h.slice(i, i + 2), 16) / 255;
    const lin = (i: number) => Mathf.gammaToLinearSpace(n(i));
    return new Color(lin(0), lin(2), lin(4), h.length >= 8 ? n(6) : 1);
  }

  /** From sRGB components 0 to 1 (Unity's `Color` in gamma space), converted to linear. */
  static fromSrgb(r: number, g: number, b: number, a = 1): Color {
    return new Color(Mathf.gammaToLinearSpace(r), Mathf.gammaToLinearSpace(g), Mathf.gammaToLinearSpace(b), a);
  }

  /** The colour as sRGB hex, `#rrggbbaa` (the inverse of `hex`). */
  toHex(): string {
    const h = (v: number) => Math.round(Mathf.clamp01(v) * 255).toString(16).padStart(2, "0");
    const g = (v: number) => h(Mathf.linearToGammaSpace(Mathf.clamp01(v)));
    return `#${g(this.r)}${g(this.g)}${g(this.b)}${h(this.a)}`;
  }
  lerp(c: Color, t: number): Color {
    const k = Mathf.clamp01(t);
    return new Color(this.r + (c.r - this.r) * k, this.g + (c.g - this.g) * k, this.b + (c.b - this.b) * k, this.a + (c.a - this.a) * k);
  }
  withAlpha(a: number): Color {
    return new Color(this.r, this.g, this.b, a);
  }
  toArray(): [number, number, number, number] {
    return [this.r, this.g, this.b, this.a];
  }
}
