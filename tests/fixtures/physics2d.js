(() => {
  // ../../js/engine/src/audio.ts
  var playing = new Map;
  var cache = new Map;
  globalThis.__iron_audio = (ended) => {
    for (const id of ended) {
      const p = playing.get(id);
      if (!p)
        continue;
      playing.delete(id);
      p.playing = false;
      try {
        p.onEnded?.();
      } catch (e) {
        const err = e instanceof Error ? e : new Error(String(e));
        console.error(`[audio] error in onEnded: ${err.message}${err.stack ? `
${err.stack}` : ""}`);
      }
    }
  };
  // ../../js/engine/src/math.ts
  var DEG = Math.PI / 180;
  var Mathf = {
    PI: Math.PI,
    Deg2Rad: DEG,
    Rad2Deg: 1 / DEG,
    Epsilon: 0.000001,
    clamp: (v, min, max) => Math.min(max, Math.max(min, v)),
    clamp01: (v) => Math.min(1, Math.max(0, v)),
    lerp: (a, b, t) => a + (b - a) * Math.min(1, Math.max(0, t)),
    lerpUnclamped: (a, b, t) => a + (b - a) * t,
    inverseLerp: (a, b, v) => a === b ? 0 : Math.min(1, Math.max(0, (v - a) / (b - a))),
    moveTowards: (current, target, maxDelta) => Math.abs(target - current) <= maxDelta ? target : current + Math.sign(target - current) * maxDelta,
    smoothStep: (from, to, t) => {
      const x = Math.min(1, Math.max(0, t));
      return from + (to - from) * x * x * (3 - 2 * x);
    },
    deltaAngle: (a, b) => {
      const d = ((b - a) % 360 + 540) % 360 - 180;
      return d === -180 ? 180 : d;
    },
    repeat: (t, length) => t - Math.floor(t / length) * length,
    pingPong: (t, length) => {
      const r = t - Math.floor(t / (length * 2)) * length * 2;
      return length - Math.abs(r - length);
    },
    approximately: (a, b) => Math.abs(a - b) < Math.max(0.000001 * Math.max(Math.abs(a), Math.abs(b)), 0.000001),
    sign: (v) => v >= 0 ? 1 : -1,
    lerpAngle: (a, b, t) => a + Mathf.deltaAngle(a, b) * Math.min(1, Math.max(0, t)),
    moveTowardsAngle: (current, target, maxDelta) => {
      const d = Mathf.deltaAngle(current, target);
      return Math.abs(d) <= maxDelta ? current + d : current + Math.sign(d) * maxDelta;
    },
    smoothDamp: (current, target, velocity, smoothTime, deltaTime, maxSpeed = Infinity) => {
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
      if (target - current > 0 === out > target) {
        out = target;
        v = (out - target) / deltaTime;
      }
      return [out, v];
    },
    perlinNoise: (x, y) => perlin2(x, y) * 0.5 + 0.5,
    isPowerOfTwo: (v) => v > 0 && (v & v - 1) === 0,
    nextPowerOfTwo: (v) => v <= 1 ? 1 : 2 ** Math.ceil(Math.log2(v)),
    closestPowerOfTwo: (v) => v <= 1 ? 1 : 2 ** Math.round(Math.log2(v)),
    gammaToLinearSpace: (c) => c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4,
    linearToGammaSpace: (c) => c <= 0.0031308 ? c * 12.92 : 1.055 * c ** (1 / 2.4) - 0.055
  };
  var PERM = (() => {
    const p = [
      151,
      160,
      137,
      91,
      90,
      15,
      131,
      13,
      201,
      95,
      96,
      53,
      194,
      233,
      7,
      225,
      140,
      36,
      103,
      30,
      69,
      142,
      8,
      99,
      37,
      240,
      21,
      10,
      23,
      190,
      6,
      148,
      247,
      120,
      234,
      75,
      0,
      26,
      197,
      62,
      94,
      252,
      219,
      203,
      117,
      35,
      11,
      32,
      57,
      177,
      33,
      88,
      237,
      149,
      56,
      87,
      174,
      20,
      125,
      136,
      171,
      168,
      68,
      175,
      74,
      165,
      71,
      134,
      139,
      48,
      27,
      166,
      77,
      146,
      158,
      231,
      83,
      111,
      229,
      122,
      60,
      211,
      133,
      230,
      220,
      105,
      92,
      41,
      55,
      46,
      245,
      40,
      244,
      102,
      143,
      54,
      65,
      25,
      63,
      161,
      1,
      216,
      80,
      73,
      209,
      76,
      132,
      187,
      208,
      89,
      18,
      169,
      200,
      196,
      135,
      130,
      116,
      188,
      159,
      86,
      164,
      100,
      109,
      198,
      173,
      186,
      3,
      64,
      52,
      217,
      226,
      250,
      124,
      123,
      5,
      202,
      38,
      147,
      118,
      126,
      255,
      82,
      85,
      212,
      207,
      206,
      59,
      227,
      47,
      16,
      58,
      17,
      182,
      189,
      28,
      42,
      223,
      183,
      170,
      213,
      119,
      248,
      152,
      2,
      44,
      154,
      163,
      70,
      221,
      153,
      101,
      155,
      167,
      43,
      172,
      9,
      129,
      22,
      39,
      253,
      19,
      98,
      108,
      110,
      79,
      113,
      224,
      232,
      178,
      185,
      112,
      104,
      218,
      246,
      97,
      228,
      251,
      34,
      242,
      193,
      238,
      210,
      144,
      12,
      191,
      179,
      162,
      241,
      81,
      51,
      145,
      235,
      249,
      14,
      239,
      107,
      49,
      192,
      214,
      31,
      181,
      199,
      106,
      157,
      184,
      84,
      204,
      176,
      115,
      121,
      50,
      45,
      127,
      4,
      150,
      254,
      138,
      236,
      205,
      93,
      222,
      114,
      67,
      29,
      24,
      72,
      243,
      141,
      128,
      195,
      78,
      66,
      215,
      61,
      156,
      180
    ];
    return [...p, ...p];
  })();
  function perlin2(x, y) {
    const xi = Math.floor(x) & 255;
    const yi = Math.floor(y) & 255;
    const xf = x - Math.floor(x);
    const yf = y - Math.floor(y);
    const fade = (t) => t * t * t * (t * (t * 6 - 15) + 10);
    const grad = (h, gx, gy) => ((h & 1) === 0 ? gx : -gx) + ((h & 2) === 0 ? gy : -gy);
    const u = fade(xf);
    const v = fade(yf);
    const aa = PERM[PERM[xi] + yi];
    const ab = PERM[PERM[xi] + yi + 1];
    const ba = PERM[PERM[xi + 1] + yi];
    const bb = PERM[PERM[xi + 1] + yi + 1];
    const x1 = grad(aa, xf, yf) + (grad(ba, xf - 1, yf) - grad(aa, xf, yf)) * u;
    const x2 = grad(ab, xf, yf - 1) + (grad(bb, xf - 1, yf - 1) - grad(ab, xf, yf - 1)) * u;
    return x1 + (x2 - x1) * v;
  }

  class Vector2 {
    x;
    y;
    constructor(x = 0, y = 0) {
      this.x = x;
      this.y = y;
    }
    static zero = new Vector2(0, 0);
    static one = new Vector2(1, 1);
    static up = new Vector2(0, 1);
    static down = new Vector2(0, -1);
    static left = new Vector2(-1, 0);
    static right = new Vector2(1, 0);
    static from(v) {
      return v instanceof Vector2 ? v : new Vector2(v.x, v.y);
    }
    add(v) {
      return new Vector2(this.x + v.x, this.y + v.y);
    }
    sub(v) {
      return new Vector2(this.x - v.x, this.y - v.y);
    }
    scale(s) {
      return new Vector2(this.x * s, this.y * s);
    }
    mul(v) {
      return new Vector2(this.x * v.x, this.y * v.y);
    }
    dot(v) {
      return this.x * v.x + this.y * v.y;
    }
    get magnitude() {
      return Math.hypot(this.x, this.y);
    }
    get sqrMagnitude() {
      return this.x * this.x + this.y * this.y;
    }
    get normalized() {
      const m = this.magnitude;
      return m > 0 ? this.scale(1 / m) : Vector2.zero;
    }
    distance(v) {
      return Math.hypot(this.x - v.x, this.y - v.y);
    }
    lerp(v, t) {
      const k = Mathf.clamp01(t);
      return new Vector2(this.x + (v.x - this.x) * k, this.y + (v.y - this.y) * k);
    }
    moveTowards(target, maxDistance) {
      const d = Vector2.from(target).sub(this);
      const m = d.magnitude;
      return m <= maxDistance || m === 0 ? Vector2.from(target) : this.add(d.scale(maxDistance / m));
    }
    rotate(degrees) {
      const c = Math.cos(degrees * DEG);
      const s = Math.sin(degrees * DEG);
      return new Vector2(this.x * c - this.y * s, this.x * s + this.y * c);
    }
    static angle(a, b) {
      const d = Vector2.from(a).magnitude * Vector2.from(b).magnitude;
      return d === 0 ? 0 : Math.acos(Mathf.clamp(Vector2.from(a).dot(b) / d, -1, 1)) / DEG;
    }
    static signedAngle(a, b) {
      return Math.atan2(a.x * b.y - a.y * b.x, a.x * b.x + a.y * b.y) / DEG;
    }
    equals(v, epsilon = 0.000001) {
      return Math.abs(this.x - v.x) <= epsilon && Math.abs(this.y - v.y) <= epsilon;
    }
    toVector3(z = 0) {
      return new Vector3(this.x, this.y, z);
    }
    toArray() {
      return [this.x, this.y];
    }
    toString() {
      return `(${this.x}, ${this.y})`;
    }
  }

  class Vector3 {
    x;
    y;
    z;
    constructor(x = 0, y = 0, z = 0) {
      this.x = x;
      this.y = y;
      this.z = z;
    }
    static zero = new Vector3(0, 0, 0);
    static one = new Vector3(1, 1, 1);
    static up = new Vector3(0, 1, 0);
    static down = new Vector3(0, -1, 0);
    static left = new Vector3(-1, 0, 0);
    static right = new Vector3(1, 0, 0);
    static forward = new Vector3(0, 0, -1);
    static back = new Vector3(0, 0, 1);
    static from(v) {
      return v instanceof Vector3 ? v : new Vector3(v.x, v.y, v.z);
    }
    add(v) {
      return new Vector3(this.x + v.x, this.y + v.y, this.z + v.z);
    }
    sub(v) {
      return new Vector3(this.x - v.x, this.y - v.y, this.z - v.z);
    }
    scale(s) {
      return new Vector3(this.x * s, this.y * s, this.z * s);
    }
    mul(v) {
      return new Vector3(this.x * v.x, this.y * v.y, this.z * v.z);
    }
    div(v) {
      return new Vector3(this.x / v.x, this.y / v.y, this.z / v.z);
    }
    negate() {
      return new Vector3(-this.x, -this.y, -this.z);
    }
    dot(v) {
      return this.x * v.x + this.y * v.y + this.z * v.z;
    }
    cross(v) {
      return new Vector3(this.y * v.z - this.z * v.y, this.z * v.x - this.x * v.z, this.x * v.y - this.y * v.x);
    }
    get magnitude() {
      return Math.hypot(this.x, this.y, this.z);
    }
    get sqrMagnitude() {
      return this.x * this.x + this.y * this.y + this.z * this.z;
    }
    get normalized() {
      const m = this.magnitude;
      return m > 0 ? this.scale(1 / m) : Vector3.zero;
    }
    distance(v) {
      return Math.hypot(this.x - v.x, this.y - v.y, this.z - v.z);
    }
    lerp(v, t) {
      const k = Mathf.clamp01(t);
      return new Vector3(this.x + (v.x - this.x) * k, this.y + (v.y - this.y) * k, this.z + (v.z - this.z) * k);
    }
    moveTowards(target, maxDistance) {
      const d = Vector3.from(target).sub(this);
      const m = d.magnitude;
      return m <= maxDistance || m === 0 ? Vector3.from(target) : this.add(d.scale(maxDistance / m));
    }
    project(onto) {
      const n = Vector3.from(onto);
      const sq = n.sqrMagnitude;
      return sq === 0 ? Vector3.zero : n.scale(this.dot(n) / sq);
    }
    projectOnPlane(normal) {
      return this.sub(this.project(normal));
    }
    reflect(normal) {
      return this.sub(Vector3.from(normal).scale(2 * this.dot(normal)));
    }
    clampMagnitude(max) {
      const m = this.magnitude;
      return m > max ? this.scale(max / m) : this;
    }
    static angle(a, b) {
      const va = Vector3.from(a);
      const d = va.magnitude * Vector3.from(b).magnitude;
      return d === 0 ? 0 : Math.acos(Mathf.clamp(va.dot(b) / d, -1, 1)) / DEG;
    }
    equals(v, epsilon = 0.000001) {
      return Math.abs(this.x - v.x) <= epsilon && Math.abs(this.y - v.y) <= epsilon && Math.abs(this.z - v.z) <= epsilon;
    }
    toVector2() {
      return new Vector2(this.x, this.y);
    }
    toArray() {
      return [this.x, this.y, this.z];
    }
    toString() {
      return `(${this.x}, ${this.y}, ${this.z})`;
    }
  }

  class Vector4 {
    x;
    y;
    z;
    w;
    constructor(x = 0, y = 0, z = 0, w = 0) {
      this.x = x;
      this.y = y;
      this.z = z;
      this.w = w;
    }
    static zero = new Vector4;
    static one = new Vector4(1, 1, 1, 1);
    static fromVector3(v, w = 1) {
      return new Vector4(v.x, v.y, v.z, w);
    }
    add(v) {
      return new Vector4(this.x + v.x, this.y + v.y, this.z + v.z, this.w + v.w);
    }
    sub(v) {
      return new Vector4(this.x - v.x, this.y - v.y, this.z - v.z, this.w - v.w);
    }
    scale(s) {
      return new Vector4(this.x * s, this.y * s, this.z * s, this.w * s);
    }
    mul(v) {
      return new Vector4(this.x * v.x, this.y * v.y, this.z * v.z, this.w * v.w);
    }
    dot(v) {
      return this.x * v.x + this.y * v.y + this.z * v.z + this.w * v.w;
    }
    get magnitude() {
      return Math.hypot(this.x, this.y, this.z, this.w);
    }
    get normalized() {
      const m = this.magnitude;
      return m > 0 ? this.scale(1 / m) : Vector4.zero;
    }
    lerp(v, t) {
      return this.add(v.sub(this).scale(Mathf.clamp01(t)));
    }
    equals(v, epsilon = 0.000001) {
      return this.sub(v).magnitude <= epsilon;
    }
    toVector3() {
      return new Vector3(this.x, this.y, this.z);
    }
    toArray() {
      return [this.x, this.y, this.z, this.w];
    }
    toString() {
      return `(${this.x}, ${this.y}, ${this.z}, ${this.w})`;
    }
  }
  class Vector2Int {
    x;
    y;
    constructor(x = 0, y = 0) {
      this.x = Math.trunc(x);
      this.y = Math.trunc(y);
    }
    static zero = new Vector2Int(0, 0);
    static one = new Vector2Int(1, 1);
    static up = new Vector2Int(0, 1);
    static down = new Vector2Int(0, -1);
    static left = new Vector2Int(-1, 0);
    static right = new Vector2Int(1, 0);
    static floor(v) {
      return new Vector2Int(Math.floor(v.x), Math.floor(v.y));
    }
    static round(v) {
      return new Vector2Int(Math.round(v.x), Math.round(v.y));
    }
    add(v) {
      return new Vector2Int(this.x + v.x, this.y + v.y);
    }
    sub(v) {
      return new Vector2Int(this.x - v.x, this.y - v.y);
    }
    scale(s) {
      return new Vector2Int(this.x * s, this.y * s);
    }
    manhattan(v) {
      return Math.abs(this.x - v.x) + Math.abs(this.y - v.y);
    }
    equals(v) {
      return this.x === v.x && this.y === v.y;
    }
    toVector2() {
      return new Vector2(this.x, this.y);
    }
    toString() {
      return `(${this.x}, ${this.y})`;
    }
  }

  class Vector3Int {
    x;
    y;
    z;
    constructor(x = 0, y = 0, z = 0) {
      this.x = Math.trunc(x);
      this.y = Math.trunc(y);
      this.z = Math.trunc(z);
    }
    static zero = new Vector3Int(0, 0, 0);
    static one = new Vector3Int(1, 1, 1);
    static floor(v) {
      return new Vector3Int(Math.floor(v.x), Math.floor(v.y), Math.floor(v.z));
    }
    static round(v) {
      return new Vector3Int(Math.round(v.x), Math.round(v.y), Math.round(v.z));
    }
    add(v) {
      return new Vector3Int(this.x + v.x, this.y + v.y, this.z + v.z);
    }
    sub(v) {
      return new Vector3Int(this.x - v.x, this.y - v.y, this.z - v.z);
    }
    equals(v) {
      return this.x === v.x && this.y === v.y && this.z === v.z;
    }
    toVector3() {
      return new Vector3(this.x, this.y, this.z);
    }
    toString() {
      return `(${this.x}, ${this.y}, ${this.z})`;
    }
  }

  class Quaternion {
    x;
    y;
    z;
    w;
    constructor(x = 0, y = 0, z = 0, w = 1) {
      this.x = x;
      this.y = y;
      this.z = z;
      this.w = w;
    }
    static identity = new Quaternion(0, 0, 0, 1);
    static from(q) {
      return q instanceof Quaternion ? q : new Quaternion(q.x, q.y, q.z, q.w);
    }
    static angleAxis(degrees, axis) {
      const a = Vector3.from(axis).normalized;
      const half = degrees * DEG / 2;
      const s = Math.sin(half);
      return new Quaternion(a.x * s, a.y * s, a.z * s, Math.cos(half));
    }
    static euler(x, y = 0, z = 0) {
      const a = typeof x === "number" ? new Vector3(x, y, z) : x;
      return Quaternion.angleAxis(a.y, Vector3.up).mul(Quaternion.angleAxis(a.x, Vector3.right)).mul(Quaternion.angleAxis(a.z, new Vector3(0, 0, 1)));
    }
    get eulerAngles() {
      const { x, y, z, w } = this;
      const m12 = 2 * (y * z - w * x);
      const sx = Mathf.clamp(-m12, -1, 1);
      const ax = Math.asin(sx);
      let ay;
      let az;
      if (Math.abs(sx) < 0.9999) {
        ay = Math.atan2(2 * (x * z + w * y), 1 - 2 * (x * x + y * y));
        az = Math.atan2(2 * (x * y + w * z), 1 - 2 * (x * x + z * z));
      } else {
        ay = Math.atan2(-2 * (x * z - w * y), 1 - 2 * (y * y + z * z));
        az = 0;
      }
      const wrap = (r) => Mathf.repeat(r / DEG, 360);
      return new Vector3(wrap(ax), wrap(ay), wrap(az));
    }
    static lookRotation(direction, up = Vector3.up) {
      const f = Vector3.from(direction).normalized;
      if (f.sqrMagnitude === 0)
        return Quaternion.identity;
      let r = f.cross(up).normalized;
      if (r.sqrMagnitude === 0)
        r = f.cross(Math.abs(f.x) < 0.9 ? Vector3.right : Vector3.up).normalized;
      const u = r.cross(f);
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
    static fromToRotation(from, to) {
      const a = Vector3.from(from).normalized;
      const b = Vector3.from(to).normalized;
      const d = a.dot(b);
      if (d < -0.999999) {
        let axis = Vector3.right.cross(a);
        if (axis.sqrMagnitude < 0.000001)
          axis = Vector3.up.cross(a);
        return Quaternion.angleAxis(180, axis);
      }
      const c = a.cross(b);
      return new Quaternion(c.x, c.y, c.z, 1 + d).normalized;
    }
    mul(q) {
      return new Quaternion(this.w * q.x + this.x * q.w + this.y * q.z - this.z * q.y, this.w * q.y - this.x * q.z + this.y * q.w + this.z * q.x, this.w * q.z + this.x * q.y - this.y * q.x + this.z * q.w, this.w * q.w - this.x * q.x - this.y * q.y - this.z * q.z);
    }
    inverse() {
      return new Quaternion(-this.x, -this.y, -this.z, this.w);
    }
    rotate(v) {
      const q = new Vector3(this.x, this.y, this.z);
      const t = q.cross(v).scale(2);
      return Vector3.from(v).add(t.scale(this.w)).add(q.cross(t));
    }
    get normalized() {
      const len = Math.hypot(this.x, this.y, this.z, this.w);
      return len > 0 ? new Quaternion(this.x / len, this.y / len, this.z / len, this.w / len) : Quaternion.identity;
    }
    slerp(q, t) {
      const k = Mathf.clamp01(t);
      let { x, y, z, w } = q;
      let cos = this.x * x + this.y * y + this.z * z + this.w * w;
      if (cos < 0) {
        [x, y, z, w, cos] = [-x, -y, -z, -w, -cos];
      }
      if (cos > 0.9995) {
        return new Quaternion(this.x + (x - this.x) * k, this.y + (y - this.y) * k, this.z + (z - this.z) * k, this.w + (w - this.w) * k).normalized;
      }
      const angle = Math.acos(cos);
      const a = Math.sin((1 - k) * angle) / Math.sin(angle);
      const b = Math.sin(k * angle) / Math.sin(angle);
      return new Quaternion(this.x * a + x * b, this.y * a + y * b, this.z * a + z * b, this.w * a + w * b);
    }
    rotateTowards(target, maxDegrees) {
      const angle = Quaternion.angle(this, target);
      return angle === 0 ? Quaternion.from(target) : this.slerp(target, Math.min(1, maxDegrees / angle));
    }
    static angle(a, b) {
      const d = Math.abs(a.x * b.x + a.y * b.y + a.z * b.z + a.w * b.w);
      return 2 * Math.acos(Math.min(1, d)) / DEG;
    }
    toArray() {
      return [this.x, this.y, this.z, this.w];
    }
  }

  class Matrix4x4 {
    m;
    constructor(m) {
      if (m.length !== 16)
        throw new Error("Matrix4x4 needs 16 numbers");
      this.m = m;
    }
    static identity = new Matrix4x4([1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1]);
    get(row, col) {
      return this.m[col * 4 + row];
    }
    static trs(position, rotation, scale = Vector3.one) {
      const { x, y, z, w } = rotation;
      const [xx, yy, zz] = [x * x, y * y, z * z];
      const [xy, xz, yz, wx, wy, wz] = [x * y, x * z, y * z, w * x, w * y, w * z];
      return new Matrix4x4([
        (1 - 2 * (yy + zz)) * scale.x,
        2 * (xy + wz) * scale.x,
        2 * (xz - wy) * scale.x,
        0,
        2 * (xy - wz) * scale.y,
        (1 - 2 * (xx + zz)) * scale.y,
        2 * (yz + wx) * scale.y,
        0,
        2 * (xz + wy) * scale.z,
        2 * (yz - wx) * scale.z,
        (1 - 2 * (xx + yy)) * scale.z,
        0,
        position.x,
        position.y,
        position.z,
        1
      ]);
    }
    static perspective(fovDegrees, aspect, near, far) {
      const f = 1 / Math.tan(fovDegrees * DEG / 2);
      const r = far / (near - far);
      return new Matrix4x4([f / aspect, 0, 0, 0, 0, f, 0, 0, 0, 0, r, -1, 0, 0, r * near, 0]);
    }
    static orthographic(left, right, bottom, top, near, far) {
      const rw = 1 / (right - left);
      const rh = 1 / (top - bottom);
      const r = 1 / (near - far);
      return new Matrix4x4([2 * rw, 0, 0, 0, 0, 2 * rh, 0, 0, 0, 0, r, 0, -(left + right) * rw, -(top + bottom) * rh, r * near, 1]);
    }
    mul(other) {
      const a = this.m;
      const b = other.m;
      const out = new Array(16);
      for (let c = 0;c < 4; c++) {
        for (let r = 0;r < 4; r++) {
          out[c * 4 + r] = a[r] * b[c * 4] + a[4 + r] * b[c * 4 + 1] + a[8 + r] * b[c * 4 + 2] + a[12 + r] * b[c * 4 + 3];
        }
      }
      return new Matrix4x4(out);
    }
    get inverse() {
      const m = this.m;
      const inv = new Array(16);
      inv[0] = m[5] * m[10] * m[15] - m[5] * m[11] * m[14] - m[9] * m[6] * m[15] + m[9] * m[7] * m[14] + m[13] * m[6] * m[11] - m[13] * m[7] * m[10];
      inv[4] = -m[4] * m[10] * m[15] + m[4] * m[11] * m[14] + m[8] * m[6] * m[15] - m[8] * m[7] * m[14] - m[12] * m[6] * m[11] + m[12] * m[7] * m[10];
      inv[8] = m[4] * m[9] * m[15] - m[4] * m[11] * m[13] - m[8] * m[5] * m[15] + m[8] * m[7] * m[13] + m[12] * m[5] * m[11] - m[12] * m[7] * m[9];
      inv[12] = -m[4] * m[9] * m[14] + m[4] * m[10] * m[13] + m[8] * m[5] * m[14] - m[8] * m[6] * m[13] - m[12] * m[5] * m[10] + m[12] * m[6] * m[9];
      inv[1] = -m[1] * m[10] * m[15] + m[1] * m[11] * m[14] + m[9] * m[2] * m[15] - m[9] * m[3] * m[14] - m[13] * m[2] * m[11] + m[13] * m[3] * m[10];
      inv[5] = m[0] * m[10] * m[15] - m[0] * m[11] * m[14] - m[8] * m[2] * m[15] + m[8] * m[3] * m[14] + m[12] * m[2] * m[11] - m[12] * m[3] * m[10];
      inv[9] = -m[0] * m[9] * m[15] + m[0] * m[11] * m[13] + m[8] * m[1] * m[15] - m[8] * m[3] * m[13] - m[12] * m[1] * m[11] + m[12] * m[3] * m[9];
      inv[13] = m[0] * m[9] * m[14] - m[0] * m[10] * m[13] - m[8] * m[1] * m[14] + m[8] * m[2] * m[13] + m[12] * m[1] * m[10] - m[12] * m[2] * m[9];
      inv[2] = m[1] * m[6] * m[15] - m[1] * m[7] * m[14] - m[5] * m[2] * m[15] + m[5] * m[3] * m[14] + m[13] * m[2] * m[7] - m[13] * m[3] * m[6];
      inv[6] = -m[0] * m[6] * m[15] + m[0] * m[7] * m[14] + m[4] * m[2] * m[15] - m[4] * m[3] * m[14] - m[12] * m[2] * m[7] + m[12] * m[3] * m[6];
      inv[10] = m[0] * m[5] * m[15] - m[0] * m[7] * m[13] - m[4] * m[1] * m[15] + m[4] * m[3] * m[13] + m[12] * m[1] * m[7] - m[12] * m[3] * m[5];
      inv[14] = -m[0] * m[5] * m[14] + m[0] * m[6] * m[13] + m[4] * m[1] * m[14] - m[4] * m[2] * m[13] - m[12] * m[1] * m[6] + m[12] * m[2] * m[5];
      inv[3] = -m[1] * m[6] * m[11] + m[1] * m[7] * m[10] + m[5] * m[2] * m[11] - m[5] * m[3] * m[10] - m[9] * m[2] * m[7] + m[9] * m[3] * m[6];
      inv[7] = m[0] * m[6] * m[11] - m[0] * m[7] * m[10] - m[4] * m[2] * m[11] + m[4] * m[3] * m[10] + m[8] * m[2] * m[7] - m[8] * m[3] * m[6];
      inv[11] = -m[0] * m[5] * m[11] + m[0] * m[7] * m[9] + m[4] * m[1] * m[11] - m[4] * m[3] * m[9] - m[8] * m[1] * m[7] + m[8] * m[3] * m[5];
      inv[15] = m[0] * m[5] * m[10] - m[0] * m[6] * m[9] - m[4] * m[1] * m[10] + m[4] * m[2] * m[9] + m[8] * m[1] * m[6] - m[8] * m[2] * m[5];
      const det = m[0] * inv[0] + m[1] * inv[4] + m[2] * inv[8] + m[3] * inv[12];
      if (Math.abs(det) < 0.000000000001)
        return Matrix4x4.identity;
      return new Matrix4x4(inv.map((v) => v / det));
    }
    get determinant() {
      const m = this.m;
      const sub = (a, b, c, d, e, f, g, h, i) => a * (e * i - f * h) - b * (d * i - f * g) + c * (d * h - e * g);
      return m[0] * sub(m[5], m[9], m[13], m[6], m[10], m[14], m[7], m[11], m[15]) - m[4] * sub(m[1], m[9], m[13], m[2], m[10], m[14], m[3], m[11], m[15]) + m[8] * sub(m[1], m[5], m[13], m[2], m[6], m[14], m[3], m[7], m[15]) - m[12] * sub(m[1], m[5], m[9], m[2], m[6], m[10], m[3], m[7], m[11]);
    }
    multiplyPoint(p) {
      const m = this.m;
      const x = m[0] * p.x + m[4] * p.y + m[8] * p.z + m[12];
      const y = m[1] * p.x + m[5] * p.y + m[9] * p.z + m[13];
      const z = m[2] * p.x + m[6] * p.y + m[10] * p.z + m[14];
      const w = m[3] * p.x + m[7] * p.y + m[11] * p.z + m[15];
      return w !== 0 && w !== 1 ? new Vector3(x / w, y / w, z / w) : new Vector3(x, y, z);
    }
    multiplyPoint3x4(p) {
      const m = this.m;
      return new Vector3(m[0] * p.x + m[4] * p.y + m[8] * p.z + m[12], m[1] * p.x + m[5] * p.y + m[9] * p.z + m[13], m[2] * p.x + m[6] * p.y + m[10] * p.z + m[14]);
    }
    multiplyVector(v) {
      const m = this.m;
      return new Vector3(m[0] * v.x + m[4] * v.y + m[8] * v.z, m[1] * v.x + m[5] * v.y + m[9] * v.z, m[2] * v.x + m[6] * v.y + m[10] * v.z);
    }
    multiplyVector4(v) {
      const m = this.m;
      return new Vector4(m[0] * v.x + m[4] * v.y + m[8] * v.z + m[12] * v.w, m[1] * v.x + m[5] * v.y + m[9] * v.z + m[13] * v.w, m[2] * v.x + m[6] * v.y + m[10] * v.z + m[14] * v.w, m[3] * v.x + m[7] * v.y + m[11] * v.z + m[15] * v.w);
    }
    get transpose() {
      const out = new Array(16);
      for (let r = 0;r < 4; r++)
        for (let c = 0;c < 4; c++)
          out[r * 4 + c] = this.m[c * 4 + r];
      return new Matrix4x4(out);
    }
  }
  class Color {
    r;
    g;
    b;
    a;
    constructor(r = 0, g = 0, b = 0, a = 1) {
      this.r = r;
      this.g = g;
      this.b = b;
      this.a = a;
    }
    static white = new Color(1, 1, 1);
    static black = new Color(0, 0, 0);
    static red = new Color(1, 0, 0);
    static green = new Color(0, 1, 0);
    static blue = new Color(0, 0, 1);
    static yellow = new Color(1, 0.92, 0.016);
    static clear = new Color(0, 0, 0, 0);
    static hex(hex) {
      let h = hex.replace(/^#/, "");
      if (h.length === 3 || h.length === 4)
        h = [...h].map((c) => c + c).join("");
      const n = (i) => Number.parseInt(h.slice(i, i + 2), 16) / 255;
      const lin = (i) => Mathf.gammaToLinearSpace(n(i));
      return new Color(lin(0), lin(2), lin(4), h.length >= 8 ? n(6) : 1);
    }
    static fromSrgb(r, g, b, a = 1) {
      return new Color(Mathf.gammaToLinearSpace(r), Mathf.gammaToLinearSpace(g), Mathf.gammaToLinearSpace(b), a);
    }
    toHex() {
      const h = (v) => Math.round(Mathf.clamp01(v) * 255).toString(16).padStart(2, "0");
      const g = (v) => h(Mathf.linearToGammaSpace(Mathf.clamp01(v)));
      return `#${g(this.r)}${g(this.g)}${g(this.b)}${h(this.a)}`;
    }
    lerp(c, t) {
      const k = Mathf.clamp01(t);
      return new Color(this.r + (c.r - this.r) * k, this.g + (c.g - this.g) * k, this.b + (c.b - this.b) * k, this.a + (c.a - this.a) * k);
    }
    withAlpha(a) {
      return new Color(this.r, this.g, this.b, a);
    }
    toArray() {
      return [this.r, this.g, this.b, this.a];
    }
  }

  // ../../js/engine/src/scene.ts
  class Component {
    id;
    gameObject;
    get transform() {
      return this.gameObject.transform;
    }
    getComponent(cls) {
      return this.gameObject.getComponent(cls);
    }
    destroy() {
      this.gameObject.removeComponent(this);
    }
  }
  var classes = new Map;
  var names = new Map;
  var instances = new Map;
  var objects = new Map;
  var roots = [];
  var nextObject = 1;
  var nextComponent = 1 << 24;
  function register(components) {
    for (const [name, cls] of Object.entries(components)) {
      classes.set(name, cls);
      names.set(cls, name);
    }
  }
  function nameOf(cls) {
    let name = names.get(cls);
    if (!name) {
      name = cls.name;
      if (!name)
        throw new Error("component classes need a name; register them with register({ ... })");
      names.set(cls, name);
    }
    return name;
  }
  function instantiate(cls, id, object) {
    const instance = new cls;
    Object.assign(instance, { id, gameObject: object });
    instances.set(id, instance);
    object.components.push(instance);
    return instance;
  }
  var vec = (v) => [v.x, v.y, v.z];
  var quat = (q) => [q.x, q.y, q.z, q.w];
  var scaleListeners = [];

  class Transform {
    gameObject;
    _position = Vector3.zero;
    _rotation = Quaternion.identity;
    _scale = Vector3.one;
    constructor(gameObject) {
      this.gameObject = gameObject;
    }
    send() {
      __iron_op("scene.local", this.gameObject.id, vec(this._position), quat(this._rotation), vec(this._scale));
    }
    _set(position, rotation, scale) {
      this._position = position;
      this._rotation = rotation;
      this._scale = scale;
    }
    get localPosition() {
      return this._position;
    }
    set localPosition(v) {
      this._position = Vector3.from(v);
      this.send();
    }
    get localRotation() {
      return this._rotation;
    }
    set localRotation(q) {
      this._rotation = Quaternion.from(q).normalized;
      this.send();
    }
    get localScale() {
      return this._scale;
    }
    set localScale(v) {
      const old = this._scale;
      this._scale = Vector3.from(v);
      this.send();
      if (!old.equals(this._scale))
        for (const f of scaleListeners)
          f(this.gameObject);
    }
    get localEulerAngles() {
      return this._rotation.eulerAngles;
    }
    set localEulerAngles(v) {
      this.localRotation = Quaternion.euler(v);
    }
    parentWorld() {
      const p = this.gameObject.parent?.transform;
      return p ? { position: p.position, rotation: p.rotation, scale: p.lossyScale } : { position: Vector3.zero, rotation: Quaternion.identity, scale: Vector3.one };
    }
    get position() {
      const p = this.parentWorld();
      return p.position.add(p.rotation.rotate(p.scale.mul(this._position)));
    }
    set position(v) {
      const p = this.parentWorld();
      this._position = p.rotation.inverse().rotate(Vector3.from(v).sub(p.position)).div(p.scale);
      this.send();
    }
    get rotation() {
      return this.parentWorld().rotation.mul(this._rotation);
    }
    set rotation(q) {
      this._rotation = this.parentWorld().rotation.inverse().mul(q).normalized;
      this.send();
    }
    get eulerAngles() {
      return this.rotation.eulerAngles;
    }
    set eulerAngles(v) {
      this.rotation = Quaternion.euler(v);
    }
    get lossyScale() {
      return this.parentWorld().scale.mul(this._scale);
    }
    get forward() {
      return this.rotation.rotate(Vector3.forward);
    }
    get right() {
      return this.rotation.rotate(Vector3.right);
    }
    get up() {
      return this.rotation.rotate(Vector3.up);
    }
    translate(delta, space = "world") {
      const d = space === "self" ? this.rotation.rotate(delta) : Vector3.from(delta);
      this.position = this.position.add(d);
    }
    rotate(euler, space = "self") {
      const q = Quaternion.euler(euler);
      this.rotation = space === "self" ? this.rotation.mul(q) : q.mul(this.rotation);
    }
    lookAt(target, up = Vector3.up) {
      this.rotation = Quaternion.lookRotation(Vector3.from(target).sub(this.position), up);
    }
    transformPoint(p) {
      return this.position.add(this.rotation.rotate(this.lossyScale.mul(p)));
    }
    inverseTransformPoint(p) {
      return this.rotation.inverse().rotate(Vector3.from(p).sub(this.position)).div(this.lossyScale);
    }
    transformDirection(d) {
      return this.rotation.rotate(d);
    }
    get parent() {
      return this.gameObject.parent?.transform ?? null;
    }
  }

  class GameObject {
    id;
    transform;
    components = [];
    children = [];
    _name;
    _parent = null;
    _enabled = true;
    _destroyed = false;
    constructor(name = "GameObject", options = {}) {
      this.id = nextObject++;
      this._name = name;
      this.transform = new Transform(this);
      this.transform._set(Vector3.from(options.position ?? Vector3.zero), Quaternion.from(options.rotation ?? Quaternion.identity).normalized, Vector3.from(options.scale ?? Vector3.one));
      objects.set(this.id, this);
      const parent = options.parent ?? null;
      this._parent = parent;
      (parent ? parent.children : roots).push(this);
      const t = this.transform;
      __iron_op("scene.create", this.id, name, parent?.id ?? null, vec(t._position), quat(t._rotation), vec(t._scale));
    }
    static _adopt(id, name) {
      const go = Object.create(GameObject.prototype);
      Object.assign(go, {
        id,
        components: [],
        children: [],
        _name: name,
        _parent: null,
        _enabled: true,
        _destroyed: false
      });
      Object.assign(go, { transform: new Transform(go) });
      objects.set(id, go);
      roots.push(go);
      return go;
    }
    get name() {
      return this._name;
    }
    set name(name) {
      this._name = name;
      __iron_op("scene.name", this.id, name);
    }
    get parent() {
      return this._parent;
    }
    setParent(parent, keepWorld = true) {
      for (let p = parent;p; p = p._parent) {
        if (p === this)
          throw new Error(`can't parent ${this._name} under itself`);
      }
      const t = this.transform;
      const position = t.position;
      const rotation = t.rotation;
      const siblings = this._parent ? this._parent.children : roots;
      siblings.splice(siblings.indexOf(this), 1);
      this._parent = parent;
      (parent ? parent.children : roots).push(this);
      __iron_op("scene.parent", this.id, parent?.id ?? null);
      if (keepWorld) {
        t.position = position;
        t.rotation = rotation;
      }
      for (const f of scaleListeners)
        f(this);
    }
    get enabled() {
      return this._enabled;
    }
    set enabled(on) {
      this._enabled = on;
      __iron_op("scene.enabled", this.id, on);
    }
    get activeInHierarchy() {
      for (let o = this;o; o = o._parent)
        if (!o._enabled)
          return false;
      return true;
    }
    get destroyed() {
      return this._destroyed;
    }
    addComponent(cls, props) {
      const name = nameOf(cls);
      const id = nextComponent++;
      const instance = instantiate(cls, id, this);
      if (props)
        Object.assign(instance, props);
      __iron_op("scene.addComponent", this.id, id, name);
      instance.onAwake?.();
      return instance;
    }
    getComponent(cls) {
      return this.components.find((c) => c instanceof cls);
    }
    getComponents(cls) {
      return this.components.filter((c) => c instanceof cls);
    }
    getComponentInChildren(cls) {
      return this.getComponent(cls) ?? this.children.map((c) => c.getComponentInChildren(cls)).find((c) => c);
    }
    removeComponent(component) {
      const i = this.components.indexOf(component);
      if (i < 0)
        return;
      this.components.splice(i, 1);
      __iron_op("scene.removeComponent", component.id);
    }
    destroy() {
      if (this._destroyed)
        return;
      __iron_op("scene.destroy", this.id);
      forget(this);
    }
    findChild(name) {
      return this.children.find((c) => c._name === name);
    }
    static find(name) {
      const walk = (list) => {
        for (const o of list) {
          if (o._name === name)
            return o;
          const found = walk(o.children);
          if (found)
            return found;
        }
        return;
      };
      return walk(roots);
    }
    static get roots() {
      return roots;
    }
    static byId(id) {
      return objects.get(id);
    }
    static createPrimitive(type, options) {
      if (!primitiveFactory)
        throw new Error("createPrimitive needs @iron/engine's index (primitives.ts) loaded");
      return primitiveFactory(type, options);
    }
    static findObjectOfType(cls) {
      for (const c of instances.values())
        if (c instanceof cls)
          return c;
      return;
    }
  }
  var primitiveFactory = null;
  function setPrimitiveFactory(f) {
    primitiveFactory = f;
  }
  function forget(go) {
    const siblings = go.parent ? go.parent.children : roots;
    const i = siblings.indexOf(go);
    if (i >= 0)
      siblings.splice(i, 1);
    const stack = [go];
    while (stack.length) {
      const o = stack.pop();
      o._destroyed = true;
      objects.delete(o.id);
      stack.push(...o.children);
    }
  }
  globalThis.__iron_registered = () => [...classes.keys()];
  globalThis.__iron_create = (className, id, objectId, objectName) => {
    const cls = classes.get(className);
    if (!cls)
      throw new Error(`unknown component class '${className}'`);
    const object = objectId === undefined || objectId === null ? GameObject._adopt(-id - 1, className) : objects.get(objectId) ?? GameObject._adopt(objectId, objectName ?? className);
    instantiate(cls, id, object).onAwake?.();
  };
  var HOOKS = ["onStart", "onUpdate", "onLateUpdate", "onFixedUpdate", "onPreRender", "onDestroy"];
  globalThis.__iron_hooks = (id) => {
    const instance = instances.get(id);
    return instance ? HOOKS.filter((h) => typeof instance[h] === "function") : [...HOOKS];
  };
  globalThis.__iron_dispatch = (id, method, ...args) => {
    const instance = instances.get(id);
    if (!instance)
      throw new Error(`no component with id ${id}`);
    const fn = instance[method];
    if (typeof fn === "function")
      return fn.apply(instance, args);
    return;
  };
  globalThis.__iron_destroy = (id) => {
    const instance = instances.get(id);
    instances.delete(id);
    if (!instance)
      return;
    const list = instance.gameObject.components;
    const i = list.indexOf(instance);
    if (i >= 0)
      list.splice(i, 1);
  };
  globalThis.__iron_scene = (moved) => {
    for (let i = 0;i + 10 < moved.length; i += 11) {
      const go = objects.get(moved[i]);
      if (!go)
        continue;
      go.transform._set(new Vector3(moved[i + 1], moved[i + 2], moved[i + 3]), new Quaternion(moved[i + 4], moved[i + 5], moved[i + 6], moved[i + 7]), new Vector3(moved[i + 8], moved[i + 9], moved[i + 10]));
    }
  };

  // ../../js/engine/src/audio-components.ts
  var clips = new Map;
  // ../../js/engine/src/clock.ts
  var time = 0;
  var unscaledTime = 0;
  var deltaTime = 0;
  var unscaledDeltaTime = 0;
  var frameCount = 0;
  var timeScale = 1;
  var startedAt = Date.now();
  var tickListeners = [];
  function onTick(fn) {
    tickListeners.push(fn);
  }
  globalThis.__iron_tick = (dt) => {
    unscaledDeltaTime = dt;
    deltaTime = dt * timeScale;
    unscaledTime += dt;
    time += deltaTime;
    frameCount++;
    for (const fn of tickListeners)
      fn();
    return deltaTime;
  };
  var pending = [];
  onTick(() => {
    const now = pending;
    pending = [];
    for (const fn of now)
      fn();
  });

  // ../../js/engine/src/physics.ts
  var vec2 = (v) => v ? [v.x, v.y, v.z] : null;
  var quat2 = (q) => q ? [q.x, q.y, q.z, q.w] : null;
  var nextBody = 1;
  var nextCollider = 1;
  var nextQuery = 1;
  var bodies = new Map;
  var colliders = new Map;
  var queries = new Map;

  class PhysicsCollider {
    id;
    body;
    sensor;
    constructor(body, options) {
      this.id = nextCollider++;
      this.body = body;
      this.sensor = !!options.sensor;
      const s = options.shape;
      let shape;
      let dims;
      let indices = null;
      if ("box" in s)
        [shape, dims] = ["box", [s.box.x, s.box.y, s.box.z]];
      else if ("ball" in s)
        [shape, dims] = ["ball", [s.ball]];
      else if ("capsule" in s)
        [shape, dims] = ["capsule", [s.capsule.halfHeight, s.capsule.radius]];
      else if ("cylinder" in s)
        [shape, dims] = ["cylinder", [s.cylinder.halfHeight, s.cylinder.radius]];
      else if ("convex" in s)
        [shape, dims] = ["convex", s.convex.flatMap((p) => [p.x, p.y, p.z])];
      else if ("renderMesh" in s) {
        const r = s.renderMesh;
        [shape, dims] = ["renderMesh", [r.mesh, r.convex ? 1 : 0, r.scale.x, r.scale.y, r.scale.z]];
      } else {
        [shape, dims] = ["mesh", s.mesh.points.flatMap((p) => [p.x, p.y, p.z])];
        indices = s.mesh.indices;
      }
      colliders.set(this.id, this);
      __iron_op("physics.collider", this.id, body?.id ?? null, shape, dims, vec2(options.position), quat2(options.rotation), [
        options.friction ?? 0.5,
        options.restitution ?? 0,
        options.density ?? 1,
        this.sensor,
        true,
        indices
      ]);
    }
    setFriction(friction) {
      __iron_op("physics.colliderSet", this.id, "friction", friction);
    }
    setRestitution(restitution) {
      __iron_op("physics.colliderSet", this.id, "restitution", restitution);
    }
    setSensor(sensor) {
      this.sensor = sensor;
      __iron_op("physics.colliderSet", this.id, "sensor", sensor);
    }
    setOffset(position, rotation) {
      __iron_op("physics.colliderSet", this.id, "offset", vec2(position), quat2(rotation));
    }
    remove() {
      if (!colliders.delete(this.id))
        return;
      __iron_op("physics.removeCollider", this.id);
    }
  }

  class PhysicsBody {
    id;
    kind;
    position;
    rotation;
    velocity;
    angularVelocity = Vector3.zero;
    grounded = false;
    onCollisionStart;
    onCollisionEnd;
    constructor(options = {}) {
      this.id = nextBody++;
      this.kind = options.kind ?? "dynamic";
      this.position = Vector3.from(options.position ?? Vector3.zero);
      this.rotation = Quaternion.from(options.rotation ?? Quaternion.identity);
      this.velocity = Vector3.from(options.velocity ?? Vector3.zero);
      bodies.set(this.id, this);
      __iron_op("physics.body", this.id, this.kind, vec2(this.position), quat2(this.rotation), vec2(this.velocity), [
        options.gravityScale ?? 1,
        options.linearDamping ?? 0,
        options.angularDamping ?? 0,
        !!options.ccd,
        !!options.lockRotations
      ]);
    }
    addCollider(options) {
      return new PhysicsCollider(this, options);
    }
    setPosition(p) {
      this.position = Vector3.from(p);
      __iron_op("physics.set", this.id, "position", vec2(p));
    }
    setRotation(q) {
      this.rotation = Quaternion.from(q);
      __iron_op("physics.set", this.id, "rotation", quat2(q));
    }
    setVelocity(v) {
      this.velocity = Vector3.from(v);
      __iron_op("physics.set", this.id, "velocity", vec2(v));
    }
    setAngularVelocity(v) {
      __iron_op("physics.set", this.id, "angularVelocity", vec2(v));
    }
    moveTo(p) {
      __iron_op("physics.set", this.id, "target", vec2(p));
    }
    applyImpulse(impulse) {
      __iron_op("physics.force", this.id, "impulse", vec2(impulse));
    }
    applyTorqueImpulse(impulse) {
      __iron_op("physics.force", this.id, "torqueImpulse", vec2(impulse));
    }
    addForce(force) {
      __iron_op("physics.force", this.id, "force", vec2(force));
    }
    addTorque(torque) {
      __iron_op("physics.force", this.id, "torque", vec2(torque));
    }
    clearForces() {
      __iron_op("physics.clearForces", this.id);
    }
    applyImpulseAtPoint(impulse, point) {
      __iron_op("physics.force", this.id, "impulseAt", vec2(impulse), vec2(point));
    }
    setKind(kind) {
      this.kind = kind;
      __iron_op("physics.set", this.id, "kind", kind);
    }
    setMass(kg) {
      __iron_op("physics.set", this.id, "mass", kg);
    }
    setGravityScale(scale) {
      __iron_op("physics.set", this.id, "gravityScale", scale);
    }
    setDamping(linear, angular) {
      __iron_op("physics.set", this.id, "linearDamping", linear);
      __iron_op("physics.set", this.id, "angularDamping", angular);
    }
    setCcd(on) {
      __iron_op("physics.set", this.id, "ccd", on);
    }
    setAllowedAxes(translation, rotation) {
      __iron_op("physics.set", this.id, "constraints", [...translation, ...rotation]);
    }
    sleep() {
      __iron_op("physics.set", this.id, "sleep");
    }
    wakeUp() {
      __iron_op("physics.set", this.id, "wake");
    }
    attachTo(objectId) {
      __iron_op("physics.attach", this.id, objectId);
    }
    setEnabled(enabled) {
      __iron_op("physics.set", this.id, "enabled", enabled);
    }
    makeCharacter(options = {}) {
      __iron_op("physics.character", this.id, options.maxSlope ?? 45, options.snapToGround ?? null, options.stepHeight ?? null);
    }
    move(delta) {
      __iron_op("physics.move", this.id, vec2(delta));
    }
    remove() {
      if (!bodies.delete(this.id))
        return;
      for (const [id, c] of colliders)
        if (c.body === this)
          colliders.delete(id);
      __iron_op("physics.removeBody", this.id);
    }
  }
  var nextJoint = 1;

  class PhysicsJoint {
    kind;
    id;
    constructor(kind, a, b, anchorA, anchorB, params = []) {
      this.kind = kind;
      this.id = nextJoint++;
      __iron_op("physics.joint", this.id, kind, a.id, b?.id ?? null, vec2(anchorA), vec2(anchorB), params);
    }
    remove() {
      __iron_op("physics.removeJoint", this.id);
    }
  }
  var physics = {
    createJoint(kind, a, b, anchorA, anchorB, params) {
      return new PhysicsJoint(kind, a, b, anchorA, anchorB, params);
    },
    createBody(options) {
      return new PhysicsBody(options);
    },
    addCollider(options) {
      return new PhysicsCollider(null, options);
    },
    setGravity(g) {
      __iron_op("physics.gravity", vec2(g));
    },
    raycast(origin, direction, maxDistance = 1000) {
      const id = nextQuery++;
      const len = Math.hypot(direction.x, direction.y, direction.z) || 1;
      const dir = { x: direction.x / len, y: direction.y / len, z: direction.z / len };
      return new Promise((resolve) => {
        queries.set(id, { origin: { ...origin }, dir, resolve });
        __iron_op("physics.raycast", id, vec2(origin), vec2(dir), maxDistance);
      });
    },
    body(id) {
      return bodies.get(id);
    }
  };
  var contactListeners = [];
  function report(fn, event) {
    if (!fn)
      return;
    try {
      fn(event);
    } catch (e) {
      const err = e instanceof Error ? e : new Error(String(e));
      console.error(`[physics] error in collision handler: ${err.message}${err.stack ? `
${err.stack}` : ""}`);
    }
  }
  globalThis.__iron_physics = (moved, contacts, rays, characters) => {
    for (let i = 0;i + 13 < moved.length; i += 14) {
      const b = bodies.get(moved[i]);
      if (!b)
        continue;
      b.position = new Vector3(moved[i + 1], moved[i + 2], moved[i + 3]);
      b.rotation = new Quaternion(moved[i + 4], moved[i + 5], moved[i + 6], moved[i + 7]);
      b.velocity = new Vector3(moved[i + 8], moved[i + 9], moved[i + 10]);
      b.angularVelocity = new Vector3(moved[i + 11], moved[i + 12], moved[i + 13]);
    }
    for (let i = 0;i + 1 < characters.length; i += 2) {
      const b = bodies.get(characters[i]);
      if (b)
        b.grounded = characters[i + 1] === 1;
    }
    for (let i = 0;i + 4 < contacts.length; i += 5) {
      const a = colliders.get(contacts[i + 1]);
      const b = colliders.get(contacts[i + 2]);
      if (!a || !b)
        continue;
      const started = contacts[i] === 1;
      const sensor = contacts[i + 3] === 1;
      const speed = contacts[i + 4];
      for (const listener of contactListeners) {
        try {
          listener(a, b, started, sensor, speed);
        } catch (e) {
          const err = e instanceof Error ? e : new Error(String(e));
          console.error(`[physics] error in collision callback: ${err.message}${err.stack ? `
${err.stack}` : ""}`);
        }
      }
      for (const [self, other] of [
        [a, b],
        [b, a]
      ]) {
        const body = self.body;
        if (!body)
          continue;
        report(started ? body.onCollisionStart : body.onCollisionEnd, { collider: self, other, sensor, speed });
      }
    }
    for (let i = 0;i + 5 < rays.length; i += 6) {
      const q = queries.get(rays[i]);
      if (!q)
        continue;
      queries.delete(rays[i]);
      const collider = colliders.get(rays[i + 1]);
      if (!collider) {
        q.resolve(null);
        continue;
      }
      const distance = rays[i + 2];
      q.resolve({
        collider,
        distance,
        point: new Vector3(q.origin.x + q.dir.x * distance, q.origin.y + q.dir.y * distance, q.origin.z + q.dir.z * distance),
        normal: new Vector3(rays[i + 3], rays[i + 4], rays[i + 5])
      });
    }
  };

  // ../../js/engine/src/screen.ts
  var width = 1280;
  var height = 720;
  var scale = 1;
  globalThis.__iron_screen = (w, h, s) => {
    width = w;
    height = h;
    scale = s;
  };

  // ../../js/engine/src/rendering.ts
  var rgba = (c) => [c.r, c.g, c.b, c.a];
  class Material {
    color;
    emission;
    smoothness;
    unlit;
    mainTexture;
    mainTextureScale;
    mainTextureOffset;
    constructor(color = Color.white, emission = Color.black, smoothness = 0.5, unlit = false, mainTexture = null, mainTextureScale = Vector2.one, mainTextureOffset = Vector2.zero) {
      this.color = color;
      this.emission = emission;
      this.smoothness = smoothness;
      this.unlit = unlit;
      this.mainTexture = mainTexture;
      this.mainTextureScale = mainTextureScale;
      this.mainTextureOffset = mainTextureOffset;
    }
    withColor(color) {
      return new Material(color, this.emission, this.smoothness, this.unlit, this.mainTexture, this.mainTextureScale, this.mainTextureOffset);
    }
    withTexture(texture, scale = Vector2.one, offset = Vector2.zero) {
      return new Material(this.color, this.emission, this.smoothness, this.unlit, texture, new Vector2(scale.x, scale.y), new Vector2(offset.x, offset.y));
    }
  }
  class MeshRenderer extends Component {
    _mesh = "cube";
    _material = new Material;
    started = false;
    get mesh() {
      return this._mesh;
    }
    set mesh(m) {
      this._mesh = m;
      this.send();
    }
    get material() {
      return this._material;
    }
    set material(m) {
      this._material = m;
      this.send();
    }
    send() {
      if (!this.started)
        return;
      const m = this._material;
      __iron_op("render.meshRenderer", this.id, this.gameObject.id, typeof this._mesh === "string" ? this._mesh : this._mesh.id, rgba(m.color), [m.emission.r, m.emission.g, m.emission.b], m.smoothness, m.unlit, m.mainTexture?.id ?? null, [m.mainTextureScale.x, m.mainTextureScale.y, m.mainTextureOffset.x, m.mainTextureOffset.y]);
    }
    onAwake() {
      this.started = true;
      this.send();
    }
    onDestroy() {
      __iron_op("render.remove", this.id);
    }
  }
  var ambient = new Color(0.2, 0.2, 0.22);

  // ../../js/engine/src/physics-components.ts
  var RigidbodyConstraints = {
    None: 0,
    FreezePositionX: 2,
    FreezePositionY: 4,
    FreezePositionZ: 8,
    FreezeRotationX: 16,
    FreezeRotationY: 32,
    FreezeRotationZ: 64,
    FreezePosition: 14,
    FreezeRotation: 112,
    FreezeAll: 126
  };

  class PhysicsMaterial {
    dynamicFriction;
    bounciness;
    constructor(dynamicFriction = 0.6, bounciness = 0) {
      this.dynamicFriction = dynamicFriction;
      this.bounciness = bounciness;
    }
  }
  var objectPhysics = new Map;
  var colliderOwners = new Map;
  function _objectPhysics(go) {
    return objectPhysics.get(go);
  }
  function _physicsFor(go) {
    return physicsFor(go);
  }
  function _releasePhysics(go) {
    releasePhysics(go);
  }
  function physicsFor(go) {
    let p = objectPhysics.get(go);
    if (!p) {
      const t = go.transform;
      const body = physics.createBody({ kind: "fixed", position: t.position, rotation: t.rotation });
      body.attachTo(go.id);
      p = { body, rigidbody: null, colliders: new Set };
      objectPhysics.set(go, p);
    }
    return p;
  }
  function releasePhysics(go) {
    const p = objectPhysics.get(go);
    if (!p || p.rigidbody || p.colliders.size > 0)
      return;
    p.body.remove();
    objectPhysics.delete(go);
  }

  class Collider extends Component {
    _center = Vector3.zero;
    _isTrigger = false;
    _material = new PhysicsMaterial;
    builtScale = null;
    handle = null;
    get center() {
      return this._center;
    }
    set center(c) {
      this._center = Vector3.from(c);
      this.handle?.setOffset(this._center.mul(this.gameObject.transform.lossyScale));
    }
    get isTrigger() {
      return this._isTrigger;
    }
    set isTrigger(on) {
      this._isTrigger = on;
      this.handle?.setSensor(on);
    }
    get material() {
      return this._material;
    }
    set material(m) {
      this._material = m;
      this.handle?.setFriction(m.dynamicFriction);
      this.handle?.setRestitution(m.bounciness);
    }
    get attachedRigidbody() {
      const rb = objectPhysics.get(this.gameObject)?.rigidbody;
      return rb instanceof Rigidbody ? rb : null;
    }
    get _is2D() {
      return false;
    }
    shapeRotation() {
      return;
    }
    rebuild() {
      if (!this.handle)
        return;
      this.handle.remove();
      colliderOwners.delete(this.handle.id);
      this.create();
    }
    _rescale() {
      if (this.handle && this.builtScale && !this.builtScale.equals(this.gameObject.transform.lossyScale)) {
        this.rebuild();
      }
    }
    create() {
      const p = physicsFor(this.gameObject);
      const scale = this.gameObject.transform.lossyScale;
      this.builtScale = scale;
      this.handle = p.body.addCollider({
        shape: this.shape(scale),
        position: this._center.mul(scale),
        rotation: this.shapeRotation(),
        sensor: this._isTrigger,
        friction: this._material.dynamicFriction,
        restitution: this._material.bounciness
      });
      colliderOwners.set(this.handle.id, this);
      p.colliders.add(this);
      p.rigidbody?._sendMass();
    }
    onAwake() {
      this.create();
    }
    onDestroy() {
      const p = objectPhysics.get(this.gameObject);
      p?.colliders.delete(this);
      if (this.handle) {
        colliderOwners.delete(this.handle.id);
        this.handle.remove();
        this.handle = null;
      }
      p?.rigidbody?._sendMass();
      releasePhysics(this.gameObject);
    }
  }

  class BoxCollider extends Collider {
    _size = Vector3.one;
    get size() {
      return this._size;
    }
    set size(s) {
      this._size = Vector3.from(s);
      this.rebuild();
    }
    shape(scale) {
      const h = this._size.mul(scale).scale(0.5);
      return { box: { x: Math.abs(h.x), y: Math.abs(h.y), z: Math.abs(h.z) } };
    }
  }

  class SphereCollider extends Collider {
    _radius = 0.5;
    get radius() {
      return this._radius;
    }
    set radius(r) {
      this._radius = r;
      this.rebuild();
    }
    shape(scale) {
      return { ball: this._radius * Math.max(Math.abs(scale.x), Math.abs(scale.y), Math.abs(scale.z)) };
    }
  }

  class CapsuleCollider extends Collider {
    _radius = 0.5;
    _height = 2;
    get radius() {
      return this._radius;
    }
    set radius(r) {
      this._radius = r;
      this.rebuild();
    }
    get height() {
      return this._height;
    }
    set height(h) {
      this._height = h;
      this.rebuild();
    }
    shape(scale) {
      const radius = this._radius * Math.max(Math.abs(scale.x), Math.abs(scale.z));
      const halfHeight = Math.max(0, this._height * Math.abs(scale.y) / 2 - radius);
      return { capsule: { halfHeight, radius } };
    }
  }

  class MeshCollider extends Collider {
    sharedMesh = null;
    points = [];
    indices = [];
    convex = false;
    shape(scale) {
      if (this.sharedMesh) {
        return { renderMesh: { mesh: this.sharedMesh.id, convex: this.convex, scale } };
      }
      const pts = this.points.map((p) => Vector3.from(p).mul(scale));
      return this.convex ? { convex: pts } : { mesh: { points: pts, indices: this.indices } };
    }
    setMesh(points, indices, convex = this.convex) {
      this.points = points;
      this.indices = indices;
      this.convex = convex;
      this.rebuild();
    }
  }

  class Rigidbody extends Component {
    _mass = 1;
    _drag = 0;
    _angularDrag = 0.05;
    _useGravity = true;
    _isKinematic = false;
    _constraints = RigidbodyConstraints.None;
    _continuous = false;
    p = null;
    get body() {
      return this.p?.body ?? null;
    }
    get mass() {
      return this._mass;
    }
    set mass(kg) {
      this._mass = Math.max(0.000001, kg);
      this._sendMass();
    }
    get drag() {
      return this._drag;
    }
    set drag(d) {
      this._drag = d;
      this.p?.body.setDamping(this._drag, this._angularDrag);
    }
    get angularDrag() {
      return this._angularDrag;
    }
    set angularDrag(d) {
      this._angularDrag = d;
      this.p?.body.setDamping(this._drag, this._angularDrag);
    }
    get useGravity() {
      return this._useGravity;
    }
    set useGravity(on) {
      this._useGravity = on;
      this.p?.body.setGravityScale(on ? 1 : 0);
    }
    get isKinematic() {
      return this._isKinematic;
    }
    set isKinematic(on) {
      this._isKinematic = on;
      this.p?.body.setKind(on ? "kinematic" : "dynamic");
    }
    get constraints() {
      return this._constraints;
    }
    set constraints(flags) {
      this._constraints = flags;
      this.sendConstraints();
    }
    get collisionDetection() {
      return this._continuous ? "continuous" : "discrete";
    }
    set collisionDetection(mode) {
      this._continuous = mode === "continuous";
      this.p?.body.setCcd(this._continuous);
    }
    get velocity() {
      return this.p ? Vector3.from(this.p.body.velocity) : Vector3.zero;
    }
    set velocity(v) {
      this.p?.body.setVelocity(v);
    }
    get angularVelocity() {
      return this.p ? Vector3.from(this.p.body.angularVelocity) : Vector3.zero;
    }
    set angularVelocity(v) {
      this.p?.body.setAngularVelocity(v);
    }
    addForce(force, mode = 0 /* Force */) {
      this.p?.body.applyImpulse(this.impulse(force, mode));
    }
    addTorque(torque, mode = 0 /* Force */) {
      const dt = 1 / 60;
      const t = Vector3.from(torque);
      const instant = mode === 1 /* Impulse */ || mode === 2 /* VelocityChange */;
      this.p?.body.applyTorqueImpulse(instant ? t : t.scale(dt));
    }
    addForceAtPosition(force, position, mode = 0 /* Force */) {
      this.p?.body.applyImpulseAtPoint(this.impulse(force, mode), position);
    }
    addExplosionForce(force, position, radius, upwards = 0, mode = 1 /* Impulse */) {
      const from = Vector3.from(position).sub(new Vector3(0, upwards, 0));
      const offset = this.transform.position.sub(from);
      const distance = offset.magnitude;
      if (distance > radius)
        return;
      const falloff = 1 - Mathf.clamp01(distance / Math.max(radius, 0.000001));
      this.addForce(offset.normalized.scale(force * falloff), mode);
    }
    movePosition(position) {
      this.transform.position = position;
    }
    moveRotation(rotation) {
      this.transform.rotation = rotation;
    }
    sleep() {
      this.p?.body.sleep();
    }
    wakeUp() {
      this.p?.body.wakeUp();
    }
    impulse(force, mode) {
      const f = Vector3.from(force);
      const dt = 1 / 60;
      switch (mode) {
        case 0 /* Force */:
          return f.scale(dt);
        case 5 /* Acceleration */:
          return f.scale(this._mass * dt);
        case 1 /* Impulse */:
          return f;
        case 2 /* VelocityChange */:
          return f.scale(this._mass);
      }
    }
    _sendMass() {
      this.p?.body.setMass(this._mass);
    }
    sendConstraints() {
      const c = this._constraints;
      const free = (flag) => (c & flag) === 0;
      this.p?.body.setAllowedAxes([free(RigidbodyConstraints.FreezePositionX), free(RigidbodyConstraints.FreezePositionY), free(RigidbodyConstraints.FreezePositionZ)], [free(RigidbodyConstraints.FreezeRotationX), free(RigidbodyConstraints.FreezeRotationY), free(RigidbodyConstraints.FreezeRotationZ)]);
    }
    onAwake() {
      const p = physicsFor(this.gameObject);
      p.rigidbody = this;
      this.p = p;
      p.body.setKind(this._isKinematic ? "kinematic" : "dynamic");
      this._sendMass();
      p.body.setDamping(this._drag, this._angularDrag);
      p.body.setGravityScale(this._useGravity ? 1 : 0);
      if (this._continuous)
        p.body.setCcd(true);
      if (this._constraints)
        this.sendConstraints();
    }
    onDestroy() {
      const p = this.p;
      if (!p)
        return;
      p.rigidbody = null;
      this.p = null;
      p.body.setKind("fixed");
      releasePhysics(this.gameObject);
    }
  }
  contactListeners.push((a, b, started, sensor, speed) => {
    const ca = colliderOwners.get(a.id);
    const cb = colliderOwners.get(b.id);
    if (!ca || !cb)
      return;
    for (const [self, other] of [
      [ca, cb],
      [cb, ca]
    ]) {
      const go = self.gameObject;
      if (go.destroyed)
        continue;
      for (const component of [...go.components]) {
        if (self._is2D) {
          if (sensor) {
            (started ? component.onTriggerEnter2D : component.onTriggerExit2D)?.call(component, other);
          } else {
            const collision = {
              gameObject: other.gameObject,
              collider: other,
              rigidbody: objectPhysics.get(other.gameObject)?.rigidbody ?? null,
              relativeSpeed: speed
            };
            (started ? component.onCollisionEnter2D : component.onCollisionExit2D)?.call(component, collision);
          }
        } else if (sensor) {
          (started ? component.onTriggerEnter : component.onTriggerExit)?.call(component, other);
        } else {
          const collision = {
            gameObject: other.gameObject,
            collider: other,
            rigidbody: other.attachedRigidbody,
            relativeSpeed: speed
          };
          (started ? component.onCollisionEnter : component.onCollisionExit)?.call(component, collision);
        }
      }
    }
  });
  scaleListeners.push((go) => {
    const visit = (o) => {
      const p = objectPhysics.get(o);
      if (p)
        for (const c of [...p.colliders])
          c._rescale();
      for (const child of o.children)
        visit(child);
    };
    visit(go);
  });

  // ../../js/engine/src/physics2d.ts
  var gravity2D = new Vector2(0, -9.81);
  var Physics2D = {
    depth: 10,
    get gravity() {
      return gravity2D;
    },
    set gravity(g) {
      gravity2D = new Vector2(g.x, g.y);
      physics.setGravity({ x: g.x, y: g.y, z: 0 });
    },
    async raycast(origin, direction, distance = 1000) {
      const hit = await physics.raycast({ x: origin.x, y: origin.y, z: 0 }, { x: direction.x, y: direction.y, z: 0 }, distance);
      if (!hit)
        return null;
      return {
        collider: colliderFor(hit.collider),
        point: new Vector2(hit.point.x, hit.point.y),
        normal: new Vector2(hit.normal.x, hit.normal.y),
        distance: hit.distance
      };
    }
  };
  var colliders2D = new Map;
  function colliderFor(c) {
    return colliders2D.get(c.id) ?? null;
  }

  class Collider2D extends Collider {
    get offset() {
      return new Vector2(this.center.x, this.center.y);
    }
    set offset(o) {
      this.center = new Vector3(o.x, o.y, 0);
    }
    get _is2D() {
      return true;
    }
    get attachedRigidbody2D() {
      const rb = _objectPhysics(this.gameObject)?.rigidbody;
      return rb instanceof Rigidbody2D ? rb : null;
    }
    halfDepth() {
      return Physics2D.depth / 2;
    }
    slab(points, scale) {
      const d = this.halfDepth();
      const pts = [];
      for (const p of points) {
        pts.push(new Vector3(p.x * scale.x, p.y * scale.y, -d), new Vector3(p.x * scale.x, p.y * scale.y, d));
      }
      return { convex: pts };
    }
    create() {
      if (this.handle)
        colliders2D.delete(this.handle.id);
      super.create();
      if (this.handle)
        colliders2D.set(this.handle.id, this);
    }
    onDestroy() {
      if (this.handle)
        colliders2D.delete(this.handle.id);
      super.onDestroy();
    }
  }

  class BoxCollider2D extends Collider2D {
    _size = Vector2.one;
    get size() {
      return this._size;
    }
    set size(s) {
      this._size = new Vector2(s.x, s.y);
      this.rebuild();
    }
    shape(scale) {
      return {
        box: {
          x: Math.abs(this._size.x * scale.x / 2),
          y: Math.abs(this._size.y * scale.y / 2),
          z: this.halfDepth()
        }
      };
    }
  }

  class CircleCollider2D extends Collider2D {
    _radius = 0.5;
    get radius() {
      return this._radius;
    }
    set radius(r) {
      this._radius = r;
      this.rebuild();
    }
    shape(scale) {
      return {
        cylinder: {
          halfHeight: this.halfDepth(),
          radius: this._radius * Math.max(Math.abs(scale.x), Math.abs(scale.y))
        }
      };
    }
    shapeRotation() {
      return Quaternion.euler(90, 0, 0);
    }
  }

  class CapsuleCollider2D extends Collider2D {
    _size = new Vector2(1, 2);
    _direction = "vertical";
    get size() {
      return this._size;
    }
    set size(s) {
      this._size = new Vector2(s.x, s.y);
      this.rebuild();
    }
    get direction() {
      return this._direction;
    }
    set direction(d) {
      this._direction = d;
      this.rebuild();
    }
    shape(scale) {
      const vertical = this._direction === "vertical";
      const [w, h] = [this._size.x, this._size.y];
      const r = (vertical ? w : h) / 2;
      const half = Math.max(0, (vertical ? h : w) / 2 - r);
      const pts = [];
      for (let end = 0;end < 2; end++) {
        for (let i = 0;i <= 8; i++) {
          const a = Math.PI * (i / 8) + (end ? Math.PI : 0);
          const along = end ? -half : half;
          pts.push(vertical ? new Vector2(Math.cos(a) * r, along + Math.sin(a) * r) : new Vector2(along + Math.sin(a) * r, -Math.cos(a) * r));
        }
      }
      return this.slab(pts, scale);
    }
  }

  class PolygonCollider2D extends Collider2D {
    _points = [new Vector2(-0.5, -0.5), new Vector2(0.5, -0.5), new Vector2(0, 0.5)];
    get points() {
      return this._points;
    }
    set points(p) {
      this._points = p.map((v) => new Vector2(v.x, v.y));
      this.rebuild();
    }
    shape(scale) {
      return this.slab(this._points, scale);
    }
  }

  class EdgeCollider2D extends Collider2D {
    _points = [new Vector2(-0.5, 0), new Vector2(0.5, 0)];
    get points() {
      return this._points;
    }
    set points(p) {
      this._points = p.map((v) => new Vector2(v.x, v.y));
      this.rebuild();
    }
    shape(scale) {
      const d = this.halfDepth();
      const pts = [];
      const indices = [];
      this._points.forEach((p, i) => {
        pts.push(new Vector3(p.x * scale.x, p.y * scale.y, -d), new Vector3(p.x * scale.x, p.y * scale.y, d));
        if (i > 0) {
          const [a, b, c, e] = [(i - 1) * 2, (i - 1) * 2 + 1, i * 2, i * 2 + 1];
          indices.push(a, c, b, b, c, e);
        }
      });
      return { mesh: { points: pts, indices } };
    }
  }

  class Rigidbody2D extends Component {
    _mass = 1;
    _drag = 0;
    _angularDrag = 0.05;
    _gravityScale = 1;
    _bodyType = "dynamic";
    _freezeRotation = false;
    _continuous = false;
    p = null;
    get mass() {
      return this._mass;
    }
    set mass(kg) {
      this._mass = Math.max(0.000001, kg);
      this._sendMass();
    }
    get drag() {
      return this._drag;
    }
    set drag(d) {
      this._drag = d;
      this.p?.body.setDamping(this._drag, this._angularDrag);
    }
    get angularDrag() {
      return this._angularDrag;
    }
    set angularDrag(d) {
      this._angularDrag = d;
      this.p?.body.setDamping(this._drag, this._angularDrag);
    }
    get gravityScale() {
      return this._gravityScale;
    }
    set gravityScale(s) {
      this._gravityScale = s;
      this.p?.body.setGravityScale(s);
    }
    get bodyType() {
      return this._bodyType;
    }
    set bodyType(t) {
      this._bodyType = t;
      this.p?.body.setKind(t === "static" ? "fixed" : t);
    }
    get isKinematic() {
      return this._bodyType === "kinematic";
    }
    set isKinematic(on) {
      this.bodyType = on ? "kinematic" : "dynamic";
    }
    get freezeRotation() {
      return this._freezeRotation;
    }
    set freezeRotation(on) {
      this._freezeRotation = on;
      this.sendAxes();
    }
    get collisionDetectionMode() {
      return this._continuous ? "continuous" : "discrete";
    }
    set collisionDetectionMode(m) {
      this._continuous = m === "continuous";
      this.p?.body.setCcd(this._continuous);
    }
    get velocity() {
      const v = this.p?.body.velocity;
      return v ? new Vector2(v.x, v.y) : Vector2.zero;
    }
    set velocity(v) {
      this.p?.body.setVelocity({ x: v.x, y: v.y, z: 0 });
    }
    get angularVelocity() {
      return (this.p?.body.angularVelocity.z ?? 0) * 180 / Math.PI;
    }
    set angularVelocity(degrees) {
      this.p?.body.setAngularVelocity({ x: 0, y: 0, z: degrees * Math.PI / 180 });
    }
    get position() {
      const p = this.transform.position;
      return new Vector2(p.x, p.y);
    }
    set position(v) {
      this.transform.position = new Vector3(v.x, v.y, this.transform.position.z);
    }
    get rotation() {
      return this.transform.eulerAngles.z;
    }
    set rotation(degrees) {
      this.transform.rotation = Quaternion.euler(0, 0, degrees);
    }
    addForce(force, mode = 0 /* Force */) {
      const k = mode === 0 /* Force */ ? 1 / 60 : 1;
      this.p?.body.applyImpulse({ x: force.x * k, y: force.y * k, z: 0 });
    }
    addForceAtPosition(force, position, mode = 0 /* Force */) {
      const k = mode === 0 /* Force */ ? 1 / 60 : 1;
      this.p?.body.applyImpulseAtPoint({ x: force.x * k, y: force.y * k, z: 0 }, { x: position.x, y: position.y, z: this.transform.position.z });
    }
    addTorque(torque, mode = 0 /* Force */) {
      const k = mode === 0 /* Force */ ? 1 / 60 : 1;
      this.p?.body.applyTorqueImpulse({ x: 0, y: 0, z: torque * k });
    }
    movePosition(position) {
      this.position = position;
    }
    moveRotation(degrees) {
      this.rotation = degrees;
    }
    sleep() {
      this.p?.body.sleep();
    }
    wakeUp() {
      this.p?.body.wakeUp();
    }
    _sendMass() {
      this.p?.body.setMass(this._mass);
    }
    sendAxes() {
      this.p?.body.setAllowedAxes([true, true, false], [false, false, !this._freezeRotation]);
    }
    onAwake() {
      const p = _physicsFor(this.gameObject);
      p.rigidbody = this;
      this.p = p;
      p.body.setKind(this._bodyType === "static" ? "fixed" : this._bodyType);
      this._sendMass();
      p.body.setDamping(this._drag, this._angularDrag);
      p.body.setGravityScale(this._gravityScale);
      if (this._continuous)
        p.body.setCcd(true);
      this.sendAxes();
    }
    onDestroy() {
      const p = this.p;
      if (!p)
        return;
      p.rigidbody = null;
      this.p = null;
      p.body.setKind("fixed");
      _releasePhysics(this.gameObject);
    }
  }
  // ../../js/engine/src/particles.ts
  var systems = new Map;
  globalThis.__iron_particles = (changed) => {
    for (let i = 0;i + 2 < changed.length; i += 3) {
      systems.get(changed[i])?._report(changed[i + 1], changed[i + 2] === 1);
    }
  };
  // ../../js/engine/src/textures.ts
  var textures = new Map;
  globalThis.__iron_textures = (loaded) => {
    for (const [id, w, h] of loaded)
      textures.get(id)?._loaded(w, h);
  };
  // ../../js/engine/src/input.ts
  var keys = new Set;
  var pressedPending = new Set;
  var releasedPending = new Set;
  var pressedThisTick = new Set;
  var releasedThisTick = new Set;
  var mouseX = 0;
  var mouseY = 0;
  var deltaPending = { x: 0, y: 0 };
  var wheelPending = { x: 0, y: 0 };
  var deltaThisTick = { x: 0, y: 0 };
  var wheelThisTick = { x: 0, y: 0 };
  onTick(() => {
    pressedThisTick = pressedPending;
    releasedThisTick = releasedPending;
    pressedPending = new Set;
    releasedPending = new Set;
    deltaThisTick = deltaPending;
    wheelThisTick = wheelPending;
    deltaPending = { x: 0, y: 0 };
    wheelPending = { x: 0, y: 0 };
  });
  var pads = new Map;
  var listeners = {};
  function emit(type, event) {
    for (const fn of listeners[type] ?? []) {
      try {
        fn(event);
      } catch (e) {
        const err = e instanceof Error ? e : new Error(String(e));
        console.error(`[input] error in ${type} listener: ${err.message}${err.stack ? `
${err.stack}` : ""}`);
      }
    }
  }
  function pad(id, name = "") {
    let p = pads.get(id);
    if (!p) {
      p = { id, name, connected: true, buttons: {}, axes: {} };
      pads.set(id, p);
    }
    return p;
  }
  globalThis.__iron_input = (type, ...args) => {
    switch (type) {
      case "keydown": {
        const [code, key, repeat] = args;
        if (!repeat && !keys.has(code))
          pressedPending.add(code);
        keys.add(code);
        emit("keydown", { code, key, repeat: !!repeat });
        break;
      }
      case "keyup": {
        const [code, key] = args;
        if (keys.has(code))
          releasedPending.add(code);
        keys.delete(code);
        emit("keyup", { code, key, repeat: false });
        break;
      }
      case "mousemove":
        [mouseX, mouseY] = args;
        break;
      case "mousedelta": {
        const [dx, dy] = args;
        deltaPending.x += dx;
        deltaPending.y += dy;
        break;
      }
      case "wheel": {
        const [dx, dy] = args;
        wheelPending.x += dx;
        wheelPending.y += dy;
        break;
      }
      case "blur":
        keys.clear();
        for (const p of pads.values())
          for (const b of Object.keys(p.buttons))
            p.buttons[b] = 0;
        break;
      case "gamepadconnected": {
        const [id, name] = args;
        const p = pad(id, name);
        p.connected = true;
        emit("gamepadconnected", { gamepad: p });
        break;
      }
      case "gamepaddisconnected": {
        const p = pad(args[0]);
        p.connected = false;
        emit("gamepaddisconnected", { gamepad: p });
        break;
      }
      case "gamepadbutton": {
        const [id, button, value] = args;
        const p = pad(id);
        const was = (p.buttons[button] ?? 0) > 0.5;
        p.buttons[button] = value;
        const now = value > 0.5;
        if (now && !was)
          emit("gamepadbuttondown", { gamepad: p, button });
        if (!now && was)
          emit("gamepadbuttonup", { gamepad: p, button });
        break;
      }
      case "gamepadaxis": {
        const [id, axis, value] = args;
        const p = pad(id);
        p.axes[axis] = value;
        emit("gamepadaxis", { gamepad: p, axis, value });
        break;
      }
    }
  };
  // ../../js/engine/src/models.ts
  var models = new Map;
  var cache2 = new Map;
  globalThis.__iron_models = (loaded) => {
    for (let i = 0;i + 1 < loaded.length; i += 2) {
      const m = models.get(loaded[i]);
      const json = loaded[i + 1];
      m?._loaded(typeof json === "string" ? JSON.parse(json) : null);
    }
  };
  // ../../js/engine/src/time.ts
  var MS_PER_SECOND = 1000;
  var MS_PER_MINUTE = 60000;
  var MS_PER_HOUR = 3600000;
  var MS_PER_DAY = 86400000;

  class TimeSpan {
    totalMilliseconds;
    constructor(milliseconds) {
      this.totalMilliseconds = Math.round(milliseconds);
    }
    static zero = new TimeSpan(0);
    static of(days, hours = 0, minutes = 0, seconds = 0, milliseconds = 0) {
      return new TimeSpan(days * MS_PER_DAY + hours * MS_PER_HOUR + minutes * MS_PER_MINUTE + seconds * MS_PER_SECOND + milliseconds);
    }
    static fromDays(d) {
      return new TimeSpan(d * MS_PER_DAY);
    }
    static fromHours(h) {
      return new TimeSpan(h * MS_PER_HOUR);
    }
    static fromMinutes(m) {
      return new TimeSpan(m * MS_PER_MINUTE);
    }
    static fromSeconds(s) {
      return new TimeSpan(s * MS_PER_SECOND);
    }
    static fromMilliseconds(ms) {
      return new TimeSpan(ms);
    }
    static parse(text) {
      const m = /^\s*(-)?(?:(\d+)\.)?(\d{1,2}):(\d{1,2})(?::(\d{1,2})(?:\.(\d{1,7}))?)?\s*$/.exec(text);
      if (!m)
        throw new Error(`not a TimeSpan: "${text}"`);
      const [, neg, d, h, min, s, frac] = m;
      if (Number(h) > 23 || Number(min) > 59 || Number(s ?? 0) > 59)
        throw new Error(`TimeSpan out of range: "${text}"`);
      const ms = frac ? Number(frac.padEnd(3, "0").slice(0, 3)) : 0;
      const span = TimeSpan.of(Number(d ?? 0), Number(h), Number(min), Number(s ?? 0), ms);
      return neg ? span.negate() : span;
    }
    part(unit, mod) {
      return Math.trunc(this.totalMilliseconds / unit) % mod;
    }
    get days() {
      return Math.trunc(this.totalMilliseconds / MS_PER_DAY);
    }
    get hours() {
      return this.part(MS_PER_HOUR, 24);
    }
    get minutes() {
      return this.part(MS_PER_MINUTE, 60);
    }
    get seconds() {
      return this.part(MS_PER_SECOND, 60);
    }
    get milliseconds() {
      return this.totalMilliseconds % 1000;
    }
    get totalDays() {
      return this.totalMilliseconds / MS_PER_DAY;
    }
    get totalHours() {
      return this.totalMilliseconds / MS_PER_HOUR;
    }
    get totalMinutes() {
      return this.totalMilliseconds / MS_PER_MINUTE;
    }
    get totalSeconds() {
      return this.totalMilliseconds / MS_PER_SECOND;
    }
    add(t) {
      return new TimeSpan(this.totalMilliseconds + t.totalMilliseconds);
    }
    subtract(t) {
      return new TimeSpan(this.totalMilliseconds - t.totalMilliseconds);
    }
    multiply(factor) {
      return new TimeSpan(this.totalMilliseconds * factor);
    }
    divide(divisor) {
      return new TimeSpan(this.totalMilliseconds / divisor);
    }
    negate() {
      return new TimeSpan(-this.totalMilliseconds);
    }
    duration() {
      return new TimeSpan(Math.abs(this.totalMilliseconds));
    }
    compareTo(t) {
      return Math.sign(this.totalMilliseconds - t.totalMilliseconds);
    }
    equals(t) {
      return this.totalMilliseconds === t.totalMilliseconds;
    }
    toString(format) {
      const abs = this.duration();
      const pad = (n, w = 2) => String(n).padStart(w, "0");
      if (!format || format === "c") {
        const sign = this.totalMilliseconds < 0 ? "-" : "";
        const day = abs.days ? `${abs.days}.` : "";
        const frac = abs.milliseconds ? `.${pad(abs.milliseconds, 3)}` : "";
        return `${sign}${day}${pad(abs.hours)}:${pad(abs.minutes)}:${pad(abs.seconds)}${frac}`;
      }
      return formatTokens(format, (token) => {
        const c = token[0];
        const n = token.length;
        switch (c) {
          case "d":
            return pad(abs.days, n);
          case "h":
            return pad(abs.hours, Math.min(n, 2));
          case "m":
            return pad(abs.minutes, Math.min(n, 2));
          case "s":
            return pad(abs.seconds, Math.min(n, 2));
          case "f":
          case "F":
            return fraction(abs.milliseconds, n, c === "F");
          default:
            return;
        }
      });
    }
  }
  function fraction(ms, digits, trim) {
    let s = String(ms).padStart(3, "0").padEnd(digits, "0").slice(0, digits);
    if (trim)
      s = s.replace(/0+$/, "");
    return s;
  }
  function formatTokens(format, render) {
    let out = "";
    for (let i = 0;i < format.length; ) {
      const c = format[i];
      if (c === "\\" && i + 1 < format.length) {
        out += format[i + 1];
        i += 2;
        continue;
      }
      if (c === "'" || c === '"') {
        const end = format.indexOf(c, i + 1);
        const stop = end < 0 ? format.length : end;
        out += format.slice(i + 1, stop);
        i = stop + 1;
        continue;
      }
      let j = i;
      while (j < format.length && format[j] === c)
        j++;
      const token = format.slice(i, j);
      const rendered = /[a-zA-Z]/.test(c) ? render(token) : undefined;
      out += rendered ?? token;
      i = j;
    }
    return out;
  }
  // ../../js/engine/src/random.ts
  class Rng {
    state;
    constructor(seed = (Date.now() ^ Math.random() * 4294967296) >>> 0) {
      this.state = seed >>> 0;
    }
    nextDouble() {
      this.state = this.state + 1831565813 >>> 0;
      let t = this.state;
      t = Math.imul(t ^ t >>> 15, t | 1);
      t ^= t + Math.imul(t ^ t >>> 7, t | 61);
      return ((t ^ t >>> 14) >>> 0) / 4294967296;
    }
    next(minOrMax, max) {
      const [lo, hi] = max === undefined ? [0, minOrMax ?? 2147483647] : [minOrMax ?? 0, max];
      if (hi <= lo)
        return lo;
      return lo + Math.floor(this.nextDouble() * (hi - lo));
    }
    float(min, max) {
      return min + this.nextDouble() * (max - min);
    }
    int(min, max) {
      return this.next(Math.ceil(min), Math.floor(max));
    }
    bool(chance = 0.5) {
      return this.nextDouble() < chance;
    }
    pick(items) {
      return items.length ? items[this.next(items.length)] : undefined;
    }
    shuffle(items) {
      for (let i = items.length - 1;i > 0; i--) {
        const j = this.next(i + 1);
        [items[i], items[j]] = [items[j], items[i]];
      }
      return items;
    }
    insideUnitCircle() {
      const angle = this.nextDouble() * Math.PI * 2;
      const r = Math.sqrt(this.nextDouble());
      return new Vector2(Math.cos(angle) * r, Math.sin(angle) * r);
    }
    onUnitSphere() {
      const z = this.nextDouble() * 2 - 1;
      const angle = this.nextDouble() * Math.PI * 2;
      const r = Math.sqrt(1 - z * z);
      return new Vector3(Math.cos(angle) * r, Math.sin(angle) * r, z);
    }
    insideUnitSphere() {
      return this.onUnitSphere().scale(Math.cbrt(this.nextDouble()));
    }
    rotation() {
      const [u1, u2, u3] = [this.nextDouble(), this.nextDouble(), this.nextDouble()];
      const a = Math.sqrt(1 - u1);
      const b = Math.sqrt(u1);
      return new Quaternion(a * Math.sin(2 * Math.PI * u2), a * Math.cos(2 * Math.PI * u2), b * Math.sin(2 * Math.PI * u3), b * Math.cos(2 * Math.PI * u3));
    }
  }
  var shared = new Rng;
  // ../../js/engine/src/primitives.ts
  function createPrimitive(type, options = {}) {
    const name = options.name ?? type[0].toUpperCase() + type.slice(1);
    const go = new GameObject(name, options);
    go.addComponent(MeshRenderer, { mesh: type });
    switch (type) {
      case "cube":
        go.addComponent(BoxCollider);
        break;
      case "sphere":
        go.addComponent(SphereCollider);
        break;
      case "capsule":
        go.addComponent(CapsuleCollider);
        break;
      case "cylinder":
        go.addComponent(CapsuleCollider);
        break;
      case "plane":
        go.addComponent(BoxCollider, { size: new Vector3(10, 0.01, 10) });
        break;
      case "quad":
        go.addComponent(MeshCollider, {
          points: [new Vector3(-0.5, -0.5, 0), new Vector3(0.5, -0.5, 0), new Vector3(0.5, 0.5, 0), new Vector3(-0.5, 0.5, 0)],
          indices: [0, 1, 2, 0, 2, 3]
        });
        break;
    }
    return go;
  }
  setPrimitiveFactory(createPrimitive);
  // src/physics2d.ts
  var log = [];
  var box;
  var ball;
  var pill;
  var tri;
  var ray = "none";
  var frames = 0;

  class Logger extends Component {
    onCollisionEnter2D(c) {
      log.push(`${this.gameObject.name} hit ${c.gameObject.name}`);
    }
    onTriggerEnter2D(other) {
      log.push(`trigger ${other.gameObject.name}`);
    }
  }

  class Scene2D extends Component {
    onStart() {
      const ground = new GameObject("ground");
      ground.addComponent(EdgeCollider2D, { points: [new Vector2(-20, 0), new Vector2(20, 0)] });
      const slope = new GameObject("slope");
      slope.addComponent(EdgeCollider2D, { points: [new Vector2(3, 2), new Vector2(9, 0)] });
      box = new GameObject("box", { position: new Vector3(0, 3, 2) });
      box.addComponent(Rigidbody2D, { mass: 2 });
      box.addComponent(BoxCollider2D);
      box.addComponent(Logger);
      const zone = new GameObject("zone", { position: new Vector3(0, 1.8, 0) });
      zone.addComponent(BoxCollider2D, { size: new Vector2(2, 0.4), isTrigger: true });
      ball = new GameObject("ball", { position: new Vector3(3.6, 2.6, 0) });
      ball.addComponent(Rigidbody2D);
      ball.addComponent(CircleCollider2D, { radius: 0.3 });
      pill = new GameObject("pill", { position: new Vector3(-5, 2, -1) });
      pill.addComponent(Rigidbody2D, { freezeRotation: true });
      pill.addComponent(CapsuleCollider2D, { size: new Vector2(0.6, 2) });
      tri = new GameObject("tri", { position: new Vector3(-8, 2, 0) });
      tri.addComponent(Rigidbody2D);
      tri.addComponent(PolygonCollider2D, { points: [new Vector2(-0.5, 0), new Vector2(0.5, 0), new Vector2(0, 0.8)] });
    }
    onUpdate() {
      frames++;
      if (frames === 100) {
        Physics2D.raycast(new Vector2(0, 5), Vector2.down, 10).then((hit) => {
          ray = hit ? `${hit.collider?.gameObject.name} ${hit.distance.toFixed(2)}` : "miss";
        });
      }
    }
  }
  register({ Scene2D });
  var g = globalThis;
  g.__log2d = () => log.join(", ");
  g.__box = () => {
    const p = box.transform.position;
    const e = box.transform.eulerAngles;
    return [p.x, p.y, p.z, e.x, e.y];
  };
  g.__ball = () => [ball.transform.position.x, ball.getComponent(Rigidbody2D).angularVelocity];
  g.__pill = () => [pill.transform.position.y, pill.getComponent(Rigidbody2D).rotation];
  g.__tri_y = () => tri.transform.position.y;
  g.__ray = () => ray;
})();
