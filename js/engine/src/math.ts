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
};

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
  add(v: Vector4): Vector4 {
    return new Vector4(this.x + v.x, this.y + v.y, this.z + v.z, this.w + v.w);
  }
  scale(s: number): Vector4 {
    return new Vector4(this.x * s, this.y * s, this.z * s, this.w * s);
  }
  dot(v: Vector4): number {
    return this.x * v.x + this.y * v.y + this.z * v.z + this.w * v.w;
  }
  toArray(): [number, number, number, number] {
    return [this.x, this.y, this.z, this.w];
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

  /** Spherical interpolation, the short way round. */
  slerp(q: QuaternionLike, t: number): Quaternion {
    const k = Mathf.clamp01(t);
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
  /** From a `#rgb`, `#rgba`, `#rrggbb` or `#rrggbbaa` hex string (sRGB, as written). */
  static hex(hex: string): Color {
    let h = hex.replace(/^#/, "");
    if (h.length === 3 || h.length === 4) h = [...h].map((c) => c + c).join("");
    const n = (i: number) => Number.parseInt(h.slice(i, i + 2), 16) / 255;
    return new Color(n(0), n(2), n(4), h.length >= 8 ? n(6) : 1);
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
