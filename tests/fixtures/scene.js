(() => {
  var __create = Object.create;
  var __defProp = Object.defineProperty;
  var __getOwnPropDesc = Object.getOwnPropertyDescriptor;
  var __name = (target, name) => {
    Object.defineProperty(target, "name", {
      value: name,
      enumerable: false,
      configurable: true
    });
    return target;
  };
  var __knownSymbol = (name, symbol) => (symbol = Symbol[name]) ? symbol : Symbol.for("Symbol." + name);
  var __typeError = (msg) => {
    throw TypeError(msg);
  };
  var __defNormalProp = (obj, key, value) => (key in obj) ? __defProp(obj, key, { enumerable: true, configurable: true, writable: true, value }) : obj[key] = value;
  var __accessCheck = (obj, member, msg) => member.has(obj) || __typeError("Cannot " + msg);
  var __privateIn = (member, obj) => Object(obj) !== obj ? __typeError('Cannot use the "in" operator on this value') : member.has(obj);
  var __privateGet = (obj, member, getter) => (__accessCheck(obj, member, "read from private field"), getter ? getter.call(obj) : member.get(obj));
  var __privateSet = (obj, member, value, setter) => (__accessCheck(obj, member, "write to private field"), setter ? setter.call(obj, value) : member.set(obj, value), value);
  var __privateMethod = (obj, member, method) => (__accessCheck(obj, member, "access private method"), method);
  var __decoratorStart = (base) => [, , , __create(base?.[__knownSymbol("metadata")] ?? null)];
  var __decoratorStrings = ["class", "method", "getter", "setter", "accessor", "field", "value", "get", "set"];
  var __expectFn = (fn) => fn !== undefined && typeof fn !== "function" ? __typeError("Function expected") : fn;
  var __decoratorContext = (kind, name, done, metadata, fns) => ({
    kind: __decoratorStrings[kind],
    name,
    metadata,
    addInitializer: (fn) => done._ ? __typeError("Already initialized") : fns.push(__expectFn(fn || null))
  });
  var __decoratorMetadata = (array, target) => __defNormalProp(target, __knownSymbol("metadata"), array[3]);
  var __runInitializers = (array, flags, self, value) => {
    for (var i = 0, fns = array[flags >> 1], n = fns && fns.length;i < n; i++)
      flags & 1 ? fns[i].call(self) : value = fns[i].call(self, value);
    return value;
  };
  var __decorateElement = (array, flags, name, decorators, target, extra) => {
    var fn, it, done, ctx, access, k = flags & 7, s = !!(flags & 8), p = !!(flags & 16);
    var j = k > 3 ? array.length + 1 : k ? s ? 1 : 2 : 0, key = __decoratorStrings[k + 5];
    var initializers = k > 3 && (array[j - 1] = []), extraInitializers = array[j] || (array[j] = []);
    var desc = k && (!p && !s && (target = target.prototype), k < 5 && (k > 3 || !p) && __getOwnPropDesc(k < 4 ? target : {
      get [name]() {
        return __privateGet(this, extra);
      },
      set [name](x) {
        __privateSet(this, extra, x);
      }
    }, name));
    k ? p && k < 4 && __name(extra, (k > 2 ? "set " : k > 1 ? "get " : "") + name) : __name(target, name);
    for (var i = decorators.length - 1;i >= 0; i--) {
      ctx = __decoratorContext(k, name, done = {}, array[3], extraInitializers);
      if (k) {
        ctx.static = s, ctx.private = p, access = ctx.access = { has: p ? (x) => __privateIn(target, x) : (x) => (name in x) };
        if (k ^ 3)
          access.get = p ? (x) => (k ^ 1 ? __privateGet : __privateMethod)(x, target, k ^ 4 ? extra : desc.get) : (x) => x[name];
        if (k > 2)
          access.set = p ? (x, y) => __privateSet(x, target, y, k ^ 4 ? extra : desc.set) : (x, y) => x[name] = y;
      }
      it = (0, decorators[i])(k ? k < 4 ? p ? extra : desc[key] : k > 4 ? undefined : { get: desc.get, set: desc.set } : target, ctx);
      done._ = 1;
      if (k ^ 4 || it === undefined)
        __expectFn(it) && (k > 4 ? initializers.unshift(it) : k ? p ? extra = it : desc[key] = it : target = it);
      else if (typeof it !== "object" || it === null)
        __typeError("Object expected");
      else
        __expectFn(fn = it.get) && (desc.get = fn), __expectFn(fn = it.set) && (desc.set = fn), __expectFn(fn = it.init) && initializers.unshift(fn);
    }
    return k || __decoratorMetadata(array, target), desc && __defProp(target, name, desc), p ? k ^ 4 ? extra : desc : target;
  };

  // ../../js/engine/src/layers.ts
  function checkLayer(layer) {
    if (!Number.isInteger(layer) || layer < 0 || layer > 31)
      throw new RangeError(`layer must be 0 to 31, got ${layer}`);
    return layer;
  }

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
      return this.slerpBy(q, Mathf.clamp01(t));
    }
    static slerp(a, b, t) {
      return Quaternion.from(a).slerpBy(b, Mathf.clamp01(t));
    }
    static slerpUnclamped(a, b, t) {
      return Quaternion.from(a).slerpBy(b, t);
    }
    static lerp(a, b, t) {
      return Quaternion.lerpUnclamped(a, b, Mathf.clamp01(t));
    }
    static lerpUnclamped(a, b, t) {
      const s = Quaternion.dot(a, b) < 0 ? -1 : 1;
      return new Quaternion(a.x + (s * b.x - a.x) * t, a.y + (s * b.y - a.y) * t, a.z + (s * b.z - a.z) * t, a.w + (s * b.w - a.w) * t).normalized;
    }
    static rotateTowards(from, to, maxDegrees) {
      return Quaternion.from(from).rotateTowards(to, maxDegrees);
    }
    static dot(a, b) {
      return a.x * b.x + a.y * b.y + a.z * b.z + a.w * b.w;
    }
    slerpBy(q, k) {
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

  class Ray {
    origin;
    direction;
    constructor(origin, direction) {
      this.origin = Vector3.from(origin);
      this.direction = Vector3.from(direction).normalized;
    }
    getPoint(distance) {
      return this.origin.add(this.direction.scale(distance));
    }
  }
  class Rect {
    x;
    y;
    width;
    height;
    constructor(x = 0, y = 0, width = 0, height = 0) {
      this.x = x;
      this.y = y;
      this.width = width;
      this.height = height;
    }
    static minMax(min, max) {
      return new Rect(min.x, min.y, max.x - min.x, max.y - min.y);
    }
    get xMin() {
      return this.x;
    }
    get yMin() {
      return this.y;
    }
    get xMax() {
      return this.x + this.width;
    }
    get yMax() {
      return this.y + this.height;
    }
    get center() {
      return new Vector2(this.x + this.width / 2, this.y + this.height / 2);
    }
    get size() {
      return new Vector2(this.width, this.height);
    }
    contains(p) {
      return p.x >= this.x && p.x < this.xMax && p.y >= this.y && p.y < this.yMax;
    }
    overlaps(r) {
      return r.xMax > this.x && r.x < this.xMax && r.yMax > this.y && r.y < this.yMax;
    }
  }

  class RectInt extends Rect {
    constructor(x = 0, y = 0, width = 0, height = 0) {
      super(Math.trunc(x), Math.trunc(y), Math.trunc(width), Math.trunc(height));
    }
    *positions() {
      for (let y = this.y;y < this.yMax; y++)
        for (let x = this.x;x < this.xMax; x++)
          yield new Vector2Int(x, y);
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
  var starting = new Set;
  var builtIns = new Map;
  var instances = new Map;
  var objects = new Map;
  var roots = [];
  var nextObject = 1;
  var nextComponent = 1 << 24;
  function register(components, options = {}) {
    for (const [name, cls] of Object.entries(components)) {
      classes.set(name, cls);
      names.set(cls, name);
      if (options.spawn === false)
        starting.delete(name);
      else
        starting.add(name);
    }
  }
  function _registerBuiltIns(components) {
    for (const [name, cls] of Object.entries(components)) {
      builtIns.set(name, cls);
      if (!names.has(cls))
        names.set(cls, name);
    }
  }
  function _classByName(name) {
    return classes.get(name) ?? builtIns.get(name);
  }
  function _patchClasses() {
    const stale = new Set;
    const fresh = new Map;
    for (const instance of instances.values()) {
      const old = instance.constructor;
      const name = names.get(old);
      if (!name || builtIns.get(name) === old)
        continue;
      const cls = classes.get(name);
      if (!cls) {
        stale.add(name);
        continue;
      }
      if (cls === old)
        continue;
      Object.setPrototypeOf(instance, cls.prototype);
      let defaults = fresh.get(cls);
      if (!defaults) {
        defaults = new cls;
        fresh.set(cls, defaults);
      }
      const own = instance;
      for (const key of Object.keys(defaults))
        if (!Object.hasOwn(own, key))
          own[key] = defaults[key];
    }
    return [...stale];
  }
  function _isBuiltIn(cls) {
    const name = names.get(cls);
    return name !== undefined && builtIns.get(name) === cls;
  }
  var restored = new Set;
  function _markRestored(component) {
    restored.add(component.id);
  }
  function _nameOf(cls) {
    return nameOf(cls);
  }
  var sceneHooks = null;
  function _setSceneHooks(hooks) {
    sceneHooks = hooks;
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
  var WORLD = { position: Vector3.zero, rotation: Quaternion.identity, scale: Vector3.one, identity: true };
  function isIdentity(w) {
    const { position: p, rotation: q, scale: s } = w;
    return p.x === 0 && p.y === 0 && p.z === 0 && q.x === 0 && q.y === 0 && q.z === 0 && q.w === 1 && s.x === 1 && s.y === 1 && s.z === 1;
  }

  class Transform {
    gameObject;
    _position = Vector3.zero;
    _rotation = Quaternion.identity;
    _scale = Vector3.one;
    constructor(gameObject) {
      this.gameObject = gameObject;
    }
    send() {
      pendingTransforms.set(this.gameObject.id, this);
    }
    _setLocal(position, rotation, scale) {
      const old = this._scale;
      this._position = position;
      this._rotation = rotation;
      this._scale = scale;
      this.send();
      if (!old.equals(scale))
        for (const f of scaleListeners)
          f(this.gameObject);
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
    world() {
      const parent = this.gameObject.parent?.transform;
      if (!parent)
        return { position: this._position, rotation: this._rotation, scale: this._scale, identity: false };
      const p = parent.world();
      if (p.identity || isIdentity(p))
        return { position: this._position, rotation: this._rotation, scale: this._scale, identity: false };
      return {
        position: p.position.add(p.rotation.rotate(p.scale.mul(this._position))),
        rotation: p.rotation.mul(this._rotation),
        scale: p.scale.mul(this._scale),
        identity: false
      };
    }
    parentWorld() {
      return this.gameObject.parent?.transform.world() ?? WORLD;
    }
    get position() {
      const p = this.parentWorld();
      if (p.identity)
        return this._position;
      return p.position.add(p.rotation.rotate(p.scale.mul(this._position)));
    }
    set position(v) {
      const p = this.parentWorld();
      if (p.identity || isIdentity(p)) {
        this._position = Vector3.from(v);
        this.send();
        return;
      }
      this._position = p.rotation.inverse().rotate(Vector3.from(v).sub(p.position)).div(p.scale);
      this.send();
    }
    get rotation() {
      const p = this.parentWorld();
      return p.identity ? this._rotation : p.rotation.mul(this._rotation);
    }
    set rotation(q) {
      const p = this.parentWorld();
      if (p.identity || isIdentity(p)) {
        this._rotation = Quaternion.from(q).normalized;
        this.send();
        return;
      }
      this._rotation = p.rotation.inverse().mul(q).normalized;
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
  var HideFlags = {
    None: 0,
    HideInHierarchy: 1,
    HideInInspector: 2,
    DontSaveInEditor: 4,
    NotEditable: 8,
    DontSaveInBuild: 16,
    DontUnloadUnusedAsset: 32,
    DontSave: 52,
    HideAndDontSave: 61
  };

  class GameObject {
    id;
    transform;
    components = [];
    children = [];
    _name;
    _parent = null;
    _enabled = true;
    _destroyed = false;
    _layer = 0;
    _scene = null;
    _model = null;
    _modelPart = false;
    _prefab = null;
    hideFlags = 0;
    constructor(name = "GameObject", options = {}) {
      this.id = nextObject++;
      this._name = name;
      this.transform = new Transform(this);
      this.transform._set(Vector3.from(options.position ?? Vector3.zero), options.rotation ? Quaternion.from(options.rotation).normalized : Quaternion.identity, Vector3.from(options.scale ?? Vector3.one));
      objects.set(this.id, this);
      const parent = options.parent ?? null;
      this._parent = parent;
      if (!parent)
        this._scene = sceneHooks?.active() ?? null;
      (parent ? parent.children : roots).push(this);
      const t = this.transform;
      flushTransforms();
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
        _destroyed: false,
        _layer: 0,
        _scene: sceneHooks?.active() ?? null,
        _model: null,
        _modelPart: false,
        _prefab: null,
        hideFlags: 0
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
      flushTransforms();
      __iron_op("scene.name", this.id, name);
    }
    get layer() {
      return this._layer;
    }
    set layer(layer) {
      this._layer = checkLayer(layer);
      flushTransforms();
      __iron_op("scene.layer", this.id, layer);
    }
    get parent() {
      return this._parent;
    }
    get scene() {
      let root = this;
      while (root._parent)
        root = root._parent;
      return root._scene;
    }
    get root() {
      let root = this;
      while (root._parent)
        root = root._parent;
      return root;
    }
    setParent(parent, keepWorld = true) {
      for (let p = parent;p; p = p._parent) {
        if (p === this)
          throw new Error(`can't parent ${this._name} under itself`);
      }
      const t = this.transform;
      const position = t.position;
      const rotation = t.rotation;
      if (!parent)
        this._scene = this.scene;
      const siblings = this._parent ? this._parent.children : roots;
      siblings.splice(siblings.indexOf(this), 1);
      this._parent = parent;
      (parent ? parent.children : roots).push(this);
      flushTransforms();
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
      flushTransforms();
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
      const instance = this._addComponentAsleep(cls, props);
      instance.onAwake?.();
      return instance;
    }
    _addComponentAsleep(cls, props) {
      const name = nameOf(cls);
      const id = nextComponent++;
      const instance = instantiate(cls, id, this);
      if (props)
        Object.assign(instance, props);
      flushTransforms();
      __iron_op("scene.addComponent", this.id, id, name);
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
    getComponentsInChildren(cls) {
      return [...this.getComponents(cls), ...this.children.flatMap((c) => c.getComponentsInChildren(cls))];
    }
    getComponentInParent(cls) {
      return this.getComponent(cls) ?? this.parent?.getComponentInParent(cls);
    }
    getComponentsInParent(cls) {
      return [...this.getComponents(cls), ...this.parent?.getComponentsInParent(cls) ?? []];
    }
    removeComponent(component) {
      const i = this.components.indexOf(component);
      if (i < 0)
        return;
      this.components.splice(i, 1);
      flushTransforms();
      __iron_op("scene.removeComponent", component.id);
    }
    destroy() {
      if (this._destroyed)
        return;
      flushTransforms();
      __iron_op("scene.destroy", this.id);
      forget(this);
    }
    static dontDestroyOnLoad(go) {
      if (!sceneHooks)
        throw new Error("dontDestroyOnLoad needs @iron/engine's scenes.ts loaded");
      sceneHooks.dontDestroyOnLoad(go);
    }
    static instantiate(original, options = {}) {
      if (!sceneHooks)
        throw new Error("instantiate needs @iron/engine's scenes.ts loaded");
      return sceneHooks.instantiate(original, options);
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
  function _forEachActiveComponent(method, f) {
    for (const c of [...instances.values()]) {
      if (typeof c[method] !== "function")
        continue;
      if (!c.gameObject?.activeInHierarchy)
        continue;
      try {
        f(c);
      } catch (e) {
        console.error(`${c.gameObject.name}: ${method}: ${e}`);
      }
    }
  }
  globalThis.__iron_registered = () => [...starting];
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
    if (!instance)
      return [...HOOKS];
    return HOOKS.filter((h) => typeof instance[h] === "function" && !(h === "onStart" && restored.has(id)));
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
  globalThis.__iron_run = (method, ids, ...args) => {
    globalThis.__iron_run_at = -1;
    for (let i = 0;i < ids.length; i++) {
      const instance = instances.get(ids[i]);
      try {
        if (!instance)
          throw new Error(`no component with id ${ids[i]}`);
        const fn = instance[method];
        if (typeof fn === "function")
          fn.apply(instance, args);
      } catch (e) {
        globalThis.__iron_run_at = i;
        throw e;
      }
    }
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
  var pendingTransforms = new Map;
  function flushTransforms() {
    if (pendingTransforms.size === 0)
      return;
    const flat = [];
    for (const [id, t] of pendingTransforms) {
      if (t.gameObject.destroyed)
        continue;
      const p = t.localPosition;
      const q = t.localRotation;
      const sc = t.localScale;
      flat.push(id, p.x, p.y, p.z, q.x, q.y, q.z, q.w, sc.x, sc.y, sc.z);
    }
    pendingTransforms.clear();
    if (flat.length)
      __iron_op("scene.locals", flat);
  }
  globalThis.__iron_flush = flushTransforms;
  globalThis.__iron_scene = (moved) => {
    for (let i = 0;i + 10 < moved.length; i += 11) {
      const go = objects.get(moved[i]);
      if (!go)
        continue;
      go.transform._set(new Vector3(moved[i + 1], moved[i + 2], moved[i + 3]), new Quaternion(moved[i + 4], moved[i + 5], moved[i + 6], moved[i + 7]), new Vector3(moved[i + 8], moved[i + 9], moved[i + 10]));
    }
  };

  // ../../js/engine/src/hot.ts
  var loading = false;
  var undo = [];
  var afterLoad = [];
  globalThis.__iron_hot_begin = () => {
    const old = undo;
    undo = [];
    for (const f of old) {
      try {
        f();
      } catch (e) {
        console.error(`reload: ${e}`);
      }
    }
    loading = true;
  };
  globalThis.__iron_hot_end = () => {
    loading = false;
    const stale = _patchClasses();
    if (stale.length) {
      console.warn(`reload: ${stale.join(", ")} ${stale.length === 1 ? "isn't" : "aren't"} registered, so ` + `${stale.length === 1 ? "its" : "their"} components keep the old code (register({ ${stale.join(", ")} }, { spawn: false }))`);
    }
    for (const f of afterLoad) {
      try {
        f();
      } catch (e) {
        console.error(`reload: ${e}`);
      }
    }
    return stale;
  };

  // ../../js/engine/src/property.ts
  var PROPERTIES = Symbol("iron.properties");
  var metadataKey = Symbol.metadata ?? Symbol.for("Symbol.metadata");
  function property(_value, context) {
    const meta = context.metadata;
    if (!Object.hasOwn(meta, PROPERTIES))
      meta[PROPERTIES] = [...meta[PROPERTIES] ?? []];
    const list = meta[PROPERTIES];
    const name = String(context.name);
    if (!list.includes(name))
      list.push(name);
  }
  function propertiesOf(cls) {
    const meta = cls[metadataKey];
    return meta?.[PROPERTIES] ?? [];
  }

  // ../../js/engine/src/clock.ts
  var time = 0;
  var unscaledTime = 0;
  var deltaTime = 0;
  var unscaledDeltaTime = 0;
  var frameCount = 0;
  var timeScale = 1;
  var startedAt = Date.now();
  var tickListeners = [];
  var Time = {
    get time() {
      return time;
    },
    get unscaledTime() {
      return unscaledTime;
    },
    get deltaTime() {
      return deltaTime;
    },
    get unscaledDeltaTime() {
      return unscaledDeltaTime;
    },
    get fixedDeltaTime() {
      return 1 / 60;
    },
    get frameCount() {
      return frameCount;
    },
    get timeScale() {
      return timeScale;
    },
    set timeScale(s) {
      timeScale = Number.isFinite(s) && s >= 0 ? s : 1;
    },
    get realtimeSinceStartup() {
      return (Date.now() - startedAt) / 1000;
    }
  };
  function onTick(fn) {
    tickListeners.push(fn);
  }
  function _timeState() {
    return { time, unscaledTime, frameCount, timeScale };
  }
  function _restoreTime(state) {
    const s = state;
    if (!s)
      return;
    time = s.time ?? time;
    unscaledTime = s.unscaledTime ?? unscaledTime;
    frameCount = s.frameCount ?? frameCount;
    timeScale = s.timeScale ?? timeScale;
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

  // ../../js/engine/src/prefs.ts
  var values = new Map(Object.entries(globalThis.__iron_info?.prefs ?? {}));
  function set(key, value) {
    values.set(key, value);
    __iron_op("prefs.set", key, value);
  }
  function _loadJson(key) {
    const v = values.get(key);
    if (typeof v !== "string")
      return null;
    try {
      return JSON.parse(v);
    } catch {
      return null;
    }
  }
  function _saveJson(key, value) {
    set(key, JSON.stringify(value));
  }

  // ../../js/engine/src/quality.ts
  var DEFAULT_LEVELS = [
    { name: "Low", antiAliasing: 1, renderScale: 0.75, shadows: "hardOnly", softShadowQuality: "low", shadowResolution: 1024, shadowDistance: 20, shadowCascades: 1, lodBias: 0.4, vSyncCount: 0, anisotropicFiltering: "disable", globalTextureMipmapLimit: 1, autoExposure: true, volumetricFog: false, hdr: true, additionalLights: 8, additionalLightShadows: false },
    { name: "Medium", antiAliasing: 2, renderScale: 1, shadows: "all", softShadowQuality: "low", shadowResolution: 2048, shadowDistance: 25, shadowCascades: 2, lodBias: 0.7, vSyncCount: 0, anisotropicFiltering: "enable", globalTextureMipmapLimit: 0, autoExposure: true, volumetricFog: true, hdr: true, additionalLights: 32, additionalLightShadows: false },
    { name: "High", antiAliasing: 4, renderScale: 1, shadows: "all", softShadowQuality: "low", shadowResolution: 2048, shadowDistance: 30, shadowCascades: 4, lodBias: 1, vSyncCount: 0, anisotropicFiltering: "enable", globalTextureMipmapLimit: 0, autoExposure: true, volumetricFog: true, hdr: true, additionalLights: -1, additionalLightShadows: true },
    { name: "Ultra", antiAliasing: 8, renderScale: 1, shadows: "all", softShadowQuality: "medium", shadowResolution: 4096, shadowDistance: 40, shadowCascades: 4, lodBias: 2, vSyncCount: 0, anisotropicFiltering: "forceEnable", globalTextureMipmapLimit: 0, autoExposure: true, volumetricFog: true, hdr: true, additionalLights: -1, additionalLightShadows: true }
  ];
  var manifest = () => {
    const q = globalThis.__iron_info?.app?.quality;
    return q && typeof q === "object" ? q : {};
  };
  function buildLevels() {
    const overrides = manifest().levels ?? {};
    return DEFAULT_LEVELS.map((l) => ({ ...l, ...overrides[l.name] ?? {}, name: l.name }));
  }
  var levels = buildLevels();
  function defaultLevel() {
    const d = manifest().default;
    if (typeof d === "string")
      return d;
    if (d && typeof d === "object") {
      const platform = globalThis.__iron_info?.system?.operatingSystemFamily ?? "";
      return d[platform] ?? d.default ?? "High";
    }
    return "High";
  }
  var levelIndex = Math.max(0, levels.findIndex((l) => l.name === defaultLevel()));
  var current = { ...levels[levelIndex] };
  function sendQuality() {
    const c = current;
    __iron_op("render.quality", 0, c.antiAliasing, c.renderScale, c.shadows, c.softShadowQuality === "high" ? 3 : c.softShadowQuality === "medium" ? 2 : 1, c.lodBias, c.vSyncCount, c.anisotropicFiltering, c.globalTextureMipmapLimit, c.volumetricFog ?? true, c.hdr ?? true, c.additionalLights ?? -1, c.additionalLightShadows ?? true);
    remember();
  }
  function sendShadows() {
    __iron_op("render.shadowSettings", 0, current.shadowDistance, current.shadowResolution, current.shadowCascades);
    remember();
  }
  function remember() {
    if (restoring)
      return;
    _saveJson("iron.quality", { level: levels[levelIndex]?.name, current });
  }
  var restoring = false;
  var listeners = new Set;
  var postHook = null;
  function _setPostHook(f) {
    postHook = f;
  }
  var QualitySettings = {
    get names() {
      return levels.map((l) => l.name);
    },
    get levels() {
      return levels.map((l) => ({ ...l }));
    },
    getQualityLevel() {
      return levelIndex;
    },
    setQualityLevel(level) {
      const i = typeof level === "number" ? level : levels.findIndex((l) => l.name === level);
      const l = levels[i];
      if (!l)
        throw new Error(`no quality level ${level}`);
      const autoWas = current.autoExposure ?? true;
      levelIndex = i;
      current = { ...l };
      sendQuality();
      sendShadows();
      if ((current.autoExposure ?? true) !== autoWas)
        postHook?.();
      for (const f of listeners)
        f(i);
    },
    onLevelChanged(f) {
      listeners.add(f);
      return () => listeners.delete(f);
    },
    get antiAliasing() {
      return current.antiAliasing;
    },
    set antiAliasing(n) {
      current.antiAliasing = n <= 1 ? 1 : n <= 2 ? 2 : n <= 4 ? 4 : 8;
      sendQuality();
    },
    get renderScale() {
      return current.renderScale;
    },
    set renderScale(s) {
      current.renderScale = Math.min(2, Math.max(0.25, s));
      sendQuality();
    },
    get shadows() {
      return current.shadows;
    },
    set shadows(m) {
      current.shadows = m;
      sendQuality();
    },
    get softShadowQuality() {
      return current.softShadowQuality;
    },
    set softShadowQuality(q) {
      current.softShadowQuality = q;
      sendQuality();
    },
    get shadowDistance() {
      return current.shadowDistance;
    },
    set shadowDistance(d) {
      current.shadowDistance = d;
      sendShadows();
    },
    get vSyncCount() {
      return current.vSyncCount;
    },
    set vSyncCount(n) {
      current.vSyncCount = n > 0 ? 1 : 0;
      sendQuality();
    },
    get lodBias() {
      return current.lodBias;
    },
    set lodBias(b) {
      current.lodBias = Math.max(0.01, b);
      sendQuality();
    },
    get anisotropicFiltering() {
      return current.anisotropicFiltering;
    },
    set anisotropicFiltering(a) {
      current.anisotropicFiltering = a;
      sendQuality();
    },
    get globalTextureMipmapLimit() {
      return current.globalTextureMipmapLimit;
    },
    set globalTextureMipmapLimit(n) {
      current.globalTextureMipmapLimit = Math.min(3, Math.max(0, Math.round(n)));
      sendQuality();
    },
    get autoExposure() {
      return current.autoExposure ?? true;
    },
    set autoExposure(on) {
      current.autoExposure = on;
      remember();
      postHook?.();
    },
    get volumetricFog() {
      return current.volumetricFog ?? true;
    },
    set volumetricFog(on) {
      current.volumetricFog = on;
      sendQuality();
    },
    get hdr() {
      return current.hdr ?? true;
    },
    set hdr(on) {
      current.hdr = on;
      sendQuality();
    },
    get additionalLights() {
      return current.additionalLights ?? -1;
    },
    set additionalLights(n) {
      current.additionalLights = n < 0 ? -1 : Math.min(256, Math.round(n));
      sendQuality();
    },
    get additionalLightShadows() {
      return current.additionalLightShadows ?? true;
    },
    set additionalLightShadows(on) {
      current.additionalLightShadows = on;
      sendQuality();
    },
    get shadowCascades() {
      return current.shadowCascades;
    },
    set shadowCascades(n) {
      current.shadowCascades = n >= 4 ? 4 : n >= 2 ? 2 : 1;
      sendShadows();
    },
    get shadowResolution() {
      return current.shadowResolution;
    },
    set shadowResolution(r) {
      current.shadowResolution = r;
      sendShadows();
    }
  };
  function _qualityState() {
    return { level: levelIndex, current };
  }
  function _restoreQuality(state) {
    const s = state;
    if (!s?.current)
      return;
    levelIndex = Math.min(levels.length - 1, Math.max(0, s.level ?? levelIndex));
    current = { ...levels[levelIndex], ...s.current };
    sendQuality();
    sendShadows();
  }
  function _applyDefaultQuality() {
    const saved = _loadJson("iron.quality");
    if (saved?.current) {
      const i = levels.findIndex((l) => l.name === saved.level);
      if (i >= 0)
        levelIndex = i;
      current = { ...levels[levelIndex], ...saved.current, vSyncCount: levels[levelIndex].vSyncCount, name: levels[levelIndex].name };
    }
    restoring = true;
    try {
      sendQuality();
      sendShadows();
    } finally {
      restoring = false;
    }
  }
  if (typeof __iron_op === "function")
    _applyDefaultQuality();

  // ../../js/engine/src/screen.ts
  var width = 1280;
  var height = 720;
  var scale = 1;
  var display = () => globalThis.__iron_info?.display ?? {};
  var mode = () => display().mode ?? "Windowed";
  var isFullScreen = (m) => m === "FullScreenWindow" || m === "ExclusiveFullScreen";
  var windowSize = () => ({
    width: display().windowWidth ?? display().width ?? 1280,
    height: display().windowHeight ?? display().height ?? 720
  });
  function setResolution(w, h, m = mode(), refreshRate = 0) {
    const chosen = typeof m === "boolean" ? m ? "FullScreenWindow" : "Windowed" : m;
    __iron_op("screen.set", chosen, Math.round(w), Math.round(h), refreshRate);
  }
  var Screen = {
    get width() {
      return width;
    },
    get height() {
      return height;
    },
    get scale() {
      return scale;
    },
    get aspect() {
      return width / Math.max(1, height);
    },
    get fullScreenMode() {
      return mode();
    },
    set fullScreenMode(m) {
      __iron_op("screen.set", m, 0, 0, 0);
    },
    get fullScreen() {
      return isFullScreen(mode());
    },
    set fullScreen(on) {
      if (on === isFullScreen(mode()))
        return;
      __iron_op("screen.set", on ? "FullScreenWindow" : "Windowed", 0, 0, 0);
    },
    setResolution,
    get windowSize() {
      return windowSize();
    },
    get resolutions() {
      return [...display().resolutions ?? []];
    },
    get currentResolution() {
      return display().currentResolution ?? { width: 0, height: 0, refreshRate: 0 };
    },
    get supportedFullScreenModes() {
      return [...display().supportedModes ?? ["ExclusiveFullScreen", "FullScreenWindow", "MaximizedWindow", "Windowed"]];
    },
    onChange(fn) {
      listeners2.add(fn);
      return () => {
        listeners2.delete(fn);
      };
    }
  };
  var listeners2 = new Set;
  globalThis.__iron_screen = (w, h, s) => {
    width = w;
    height = h;
    scale = s;
    for (const fn of listeners2) {
      try {
        fn();
      } catch (e) {
        console.error(`Screen.onChange listener failed: ${e instanceof Error ? e.stack ?? e.message : e}`);
      }
    }
  };

  // ../../js/engine/src/rendering.ts
  var rgba = (c) => [c.r, c.g, c.b, c.a];
  var cameras = [];
  var _init, _base, _dec, _dec2, _dec3, _dec4, _dec5, _dec6, _dec7, _dec8, _dec9, _dec10, _dec11, _dec12, _dec13, _dec14, _dec15;

  class Camera extends (_base = Component) {
    static {
      _init = __decoratorStart(_base);
      __decorateElement(_init, 2, "fieldOfView", _dec, this);
      __decorateElement(_init, 2, "nearClipPlane", _dec2, this);
      __decorateElement(_init, 2, "farClipPlane", _dec3, this);
      __decorateElement(_init, 2, "backgroundColor", _dec4, this);
      __decorateElement(_init, 2, "clearFlags", _dec5, this);
      __decorateElement(_init, 2, "depth", _dec6, this);
      __decorateElement(_init, 2, "orthographic", _dec7, this);
      __decorateElement(_init, 2, "orthographicSize", _dec8, this);
      __decorateElement(_init, 2, "rect", _dec9, this);
      __decorateElement(_init, 2, "cullingMask", _dec10, this);
      __decorateElement(_init, 2, "layerCullDistances", _dec11, this);
      __decorateElement(_init, 2, "layerCullSpherical", _dec12, this);
      __decorateElement(_init, 2, "antialiasing", _dec13, this);
      __decorateElement(_init, 2, "targetTexture", _dec14, this);
      __decorateElement(_init, 2, "fog", _dec15, this);
      __decoratorMetadata(_init, this);
    }
    _fov = (__runInitializers(_init, 5, this), 60);
    _near = 0.1;
    _far = 1000;
    _background = new Color(0.05, 0.06, 0.09, 1);
    _depth = 0;
    _orthographic = false;
    _orthographicSize = 5;
    _rect = new Rect(0, 0, 1, 1);
    _cullingMask = 4294967295;
    _layerCullDistances = new Array(32).fill(0);
    _layerCullSpherical = false;
    _antialiasing = "none";
    _sceneGrid = false;
    _fog = true;
    _targetTexture = null;
    started = false;
    _exposure = Number.NaN;
    _sceneBrightness = Number.NaN;
    get [(_dec = [
      property
    ], "fieldOfView")]() {
      return this._fov;
    }
    set fieldOfView(v) {
      this._fov = Math.min(179, Math.max(1, v));
      this.send();
    }
    get horizontalFieldOfView() {
      return Camera.verticalToHorizontalFieldOfView(this._fov, this.aspect);
    }
    set horizontalFieldOfView(v) {
      this.fieldOfView = Camera.horizontalToVerticalFieldOfView(v, this.aspect);
    }
    static verticalToHorizontalFieldOfView(vertical, aspect) {
      const half = vertical * Math.PI / 360;
      return Math.atan(Math.tan(half) * aspect) * 360 / Math.PI;
    }
    static horizontalToVerticalFieldOfView(horizontal, aspect) {
      const half = horizontal * Math.PI / 360;
      return Math.atan(Math.tan(half) / aspect) * 360 / Math.PI;
    }
    get [(_dec2 = [
      property
    ], "nearClipPlane")]() {
      return this._near;
    }
    set nearClipPlane(v) {
      this._near = v;
      this.send();
    }
    get [(_dec3 = [
      property
    ], "farClipPlane")]() {
      return this._far;
    }
    set farClipPlane(v) {
      this._far = v;
      this.send();
    }
    get [(_dec4 = [
      property
    ], "backgroundColor")]() {
      return this._background;
    }
    set backgroundColor(c) {
      this._background = c;
      this.send();
    }
    _clearFlags = "solidColor";
    get [(_dec5 = [
      property
    ], "clearFlags")]() {
      return this._clearFlags;
    }
    set clearFlags(f) {
      this._clearFlags = f;
      this.send();
    }
    get [(_dec6 = [
      property
    ], "depth")]() {
      return this._depth;
    }
    set depth(d) {
      this._depth = d;
      this.send();
    }
    get [(_dec7 = [
      property
    ], "orthographic")]() {
      return this._orthographic;
    }
    set orthographic(on) {
      this._orthographic = on;
      this.send();
    }
    get [(_dec8 = [
      property
    ], "orthographicSize")]() {
      return this._orthographicSize;
    }
    set orthographicSize(s) {
      this._orthographicSize = s;
      this.send();
    }
    get [(_dec9 = [
      property
    ], "rect")]() {
      return this._rect;
    }
    set rect(r) {
      this._rect = r;
      this.send();
    }
    get [(_dec10 = [
      property
    ], "cullingMask")]() {
      return this._cullingMask;
    }
    set cullingMask(mask) {
      this._cullingMask = mask >>> 0;
      this.send();
    }
    get [(_dec11 = [
      property
    ], "layerCullDistances")]() {
      return [...this._layerCullDistances];
    }
    set layerCullDistances(distances) {
      this._layerCullDistances = Array.from({ length: 32 }, (_, i) => Math.max(0, distances[i] ?? 0));
      this.send();
    }
    get [(_dec12 = [
      property
    ], "layerCullSpherical")]() {
      return this._layerCullSpherical;
    }
    set layerCullSpherical(on) {
      this._layerCullSpherical = on;
      this.send();
    }
    get [(_dec13 = [
      property
    ], "antialiasing")]() {
      return this._antialiasing;
    }
    set antialiasing(mode) {
      this._antialiasing = mode;
      this.send();
    }
    get [(_dec14 = [
      property
    ], "targetTexture")]() {
      return this._targetTexture;
    }
    set targetTexture(t) {
      this._targetTexture = t;
      this.send();
    }
    get sceneGrid() {
      return this._sceneGrid;
    }
    set sceneGrid(on) {
      this._sceneGrid = on;
      this.send();
    }
    get [(_dec15 = [
      property
    ], "fog")]() {
      return this._fog;
    }
    set fog(on) {
      this._fog = on;
      this.send();
    }
    get pixelRect() {
      return new Rect(this._rect.x * Screen.width, this._rect.y * Screen.height, this._rect.width * Screen.width, this._rect.height * Screen.height);
    }
    get aspect() {
      const r = this.pixelRect;
      return r.width / Math.max(0.000001, r.height);
    }
    static get main() {
      return cameras.find((c) => c.gameObject.activeInHierarchy && !c.gameObject.destroyed && !c._targetTexture && (c.gameObject.hideFlags & HideFlags.HideInHierarchy) === 0);
    }
    get exposure() {
      return this._exposure;
    }
    get sceneBrightness() {
      return this._sceneBrightness;
    }
    resetExposure() {
      __iron_op("render.exposureReset", this.id);
    }
    aperture = 16;
    shutterSpeed = 1 / 200;
    iso = 200;
    get physicalExposure() {
      return Math.log2(this.aperture * this.aperture / Math.max(this.shutterSpeed, 0.000001)) - Math.log2(Math.max(this.iso, 1) / 100);
    }
    static byId(id) {
      return cameras.find((c) => c.id === id);
    }
    static get allCameras() {
      return cameras.filter((c) => c.gameObject.activeInHierarchy && !c.gameObject.destroyed).sort((a, b) => a._depth - b._depth);
    }
    get projectionMatrix() {
      if (this._orthographic) {
        const h = this._orthographicSize;
        const w = h * this.aspect;
        return Matrix4x4.orthographic(-w, w, -h, h, this._near, this._far);
      }
      return Matrix4x4.perspective(this._fov, this.aspect, this._near, this._far);
    }
    get worldToCameraMatrix() {
      return Matrix4x4.trs(this.transform.position, this.transform.rotation).inverse;
    }
    get viewProjectionMatrix() {
      return this.projectionMatrix.mul(this.worldToCameraMatrix);
    }
    static ndc(v) {
      return new Vector2(v.x * 2 - 1, 1 - v.y * 2);
    }
    screenToViewportPoint(p) {
      const r = this.pixelRect;
      return new Vector3((p.x - r.x) / r.width, (p.y - r.y) / r.height, p.z);
    }
    viewportToScreenPoint(v) {
      const r = this.pixelRect;
      return new Vector3(r.x + v.x * r.width, r.y + v.y * r.height, v.z);
    }
    viewportPointToRay(v) {
      const inv = this.viewProjectionMatrix.inverse;
      const n = Camera.ndc(v);
      const near = inv.multiplyPoint(new Vector3(n.x, n.y, 0));
      const far = inv.multiplyPoint(new Vector3(n.x, n.y, 1));
      return new Ray(near, far.sub(near));
    }
    screenPointToRay(p) {
      return this.viewportPointToRay(this.screenToViewportPoint({ x: p.x, y: p.y, z: 0 }));
    }
    worldToViewportPoint(p) {
      const n = this.viewProjectionMatrix.multiplyPoint(p);
      const depth = Vector3.from(p).sub(this.transform.position).dot(this.transform.forward);
      return new Vector3((n.x + 1) / 2, (1 - n.y) / 2, depth);
    }
    worldToScreenPoint(p) {
      return this.viewportToScreenPoint(this.worldToViewportPoint(p));
    }
    viewportToWorldPoint(v) {
      const ray = this.viewportPointToRay(v);
      const forward = this.transform.forward;
      const along = ray.direction.dot(forward);
      const start = ray.origin.sub(this.transform.position).dot(forward);
      return ray.getPoint(along > 0.000001 ? (v.z - start) / along : 0);
    }
    screenToWorldPoint(p) {
      return this.viewportToWorldPoint(this.screenToViewportPoint(p));
    }
    send() {
      if (!this.started)
        return;
      const r = this._rect;
      __iron_op("render.camera", this.id, this.gameObject.id, this._fov, this._near, this._far, rgba(this._background), this._depth, this._orthographic, this._orthographicSize, [r.x, r.y, r.width, r.height], this._clearFlags === "skybox" ? "skybox" : "color", this._cullingMask, this._layerCullDistances.some((d) => d > 0) ? this._layerCullDistances : null, this._layerCullSpherical, this._antialiasing, this._targetTexture?.id ?? null, this._sceneGrid, this._fog);
    }
    onStart() {
      this.started = true;
      cameras.push(this);
      this.send();
    }
    onDestroy() {
      cameras.splice(cameras.indexOf(this), 1);
      __iron_op("render.remove", this.id);
    }
  }
  var _init2, _base2, _dec16, _dec17;

  class PlaneRenderer extends (_base2 = Component) {
    static {
      _init2 = __decoratorStart(_base2);
      __decorateElement(_init2, 2, "size", _dec16, this);
      __decorateElement(_init2, 2, "color", _dec17, this);
      __decoratorMetadata(_init2, this);
    }
    _size = (__runInitializers(_init2, 5, this), new Vector2(1, 1));
    _color = Color.white;
    started = false;
    get [(_dec16 = [
      property
    ], "size")]() {
      return this._size;
    }
    set size(v) {
      this._size = Vector2.from(v);
      this.send();
    }
    get [(_dec17 = [
      property
    ], "color")]() {
      return this._color;
    }
    set color(c) {
      this._color = c;
      this.send();
    }
    send() {
      if (!this.started)
        return;
      __iron_op("render.plane", this.id, this.gameObject.id, this._size.toArray(), rgba(this._color));
    }
    onStart() {
      this.started = true;
      this.send();
    }
    onDestroy() {
      __iron_op("render.remove", this.id);
    }
  }
  var _init3, _base3, _dec18, _dec19, _dec20, _dec21, _dec22;

  class CameraFollow extends (_base3 = Component) {
    static {
      _init3 = __decoratorStart(_base3);
      __decorateElement(_init3, 5, "target", _dec18, this);
      __decorateElement(_init3, 5, "offset", _dec19, this);
      __decorateElement(_init3, 5, "offsetInTargetSpace", _dec20, this);
      __decorateElement(_init3, 5, "smoothTime", _dec21, this);
      __decorateElement(_init3, 5, "lookAtTarget", _dec22, this);
      __decoratorMetadata(_init3, this);
    }
    [(_dec18 = [
      property
    ], "target")] = __runInitializers(_init3, 8, this, null);
    [(_dec19 = [
      property
    ], "offset")] = (__runInitializers(_init3, 11, this), __runInitializers(_init3, 12, this, new Vector3(0, 3, 6)));
    [(_dec20 = [
      property
    ], "offsetInTargetSpace")] = (__runInitializers(_init3, 15, this), __runInitializers(_init3, 16, this, false));
    [(_dec21 = [
      property
    ], "smoothTime")] = (__runInitializers(_init3, 19, this), __runInitializers(_init3, 20, this, 0.25));
    [(_dec22 = [
      property
    ], "lookAtTarget")] = (__runInitializers(_init3, 23, this), __runInitializers(_init3, 24, this, true));
    velocity = (__runInitializers(_init3, 27, this), [0, 0, 0]);
    onLateUpdate(dt) {
      const t = this.target?.transform;
      if (!t || this.target?.destroyed)
        return;
      const offset = this.offsetInTargetSpace ? t.rotation.rotate(this.offset) : this.offset;
      const goal = t.position.add(offset);
      const at = this.transform.position;
      if (this.smoothTime <= 0) {
        this.transform.position = goal;
      } else {
        const axis = (i, a, b) => {
          const [v, vel] = Mathf.smoothDamp(a, b, this.velocity[i], this.smoothTime, Math.max(dt, 0.0001));
          this.velocity[i] = vel;
          return v;
        };
        this.transform.position = new Vector3(axis(0, at.x, goal.x), axis(1, at.y, goal.y), axis(2, at.z, goal.z));
      }
      if (this.lookAtTarget)
        this.transform.lookAt(t.position);
    }
  }
  var _init4, _base4, _dec23, _dec24, _dec25, _dec26, _dec27, _dec28, _dec29, _dec30;

  class CameraOrbit extends (_base4 = Component) {
    constructor() {
      super(...arguments);
      __runInitializers(_init4, 39, this);
    }
    static {
      _init4 = __decoratorStart(_base4);
      __decorateElement(_init4, 5, "target", _dec23, this);
      __decorateElement(_init4, 5, "center", _dec24, this);
      __decorateElement(_init4, 5, "distance", _dec25, this);
      __decorateElement(_init4, 5, "yaw", _dec26, this);
      __decorateElement(_init4, 5, "pitch", _dec27, this);
      __decorateElement(_init4, 5, "minPitch", _dec28, this);
      __decorateElement(_init4, 5, "maxPitch", _dec29, this);
      __decorateElement(_init4, 5, "autoRotate", _dec30, this);
      __decoratorMetadata(_init4, this);
    }
    [(_dec23 = [
      property
    ], "target")] = __runInitializers(_init4, 8, this, null);
    [(_dec24 = [
      property
    ], "center")] = (__runInitializers(_init4, 11, this), __runInitializers(_init4, 12, this, Vector3.zero));
    [(_dec25 = [
      property
    ], "distance")] = (__runInitializers(_init4, 15, this), __runInitializers(_init4, 16, this, 8));
    [(_dec26 = [
      property
    ], "yaw")] = (__runInitializers(_init4, 19, this), __runInitializers(_init4, 20, this, 0));
    [(_dec27 = [
      property
    ], "pitch")] = (__runInitializers(_init4, 23, this), __runInitializers(_init4, 24, this, 20));
    [(_dec28 = [
      property
    ], "minPitch")] = (__runInitializers(_init4, 27, this), __runInitializers(_init4, 28, this, -80));
    [(_dec29 = [
      property
    ], "maxPitch")] = (__runInitializers(_init4, 31, this), __runInitializers(_init4, 32, this, 80));
    [(_dec30 = [
      property
    ], "autoRotate")] = (__runInitializers(_init4, 35, this), __runInitializers(_init4, 36, this, 0));
    onLateUpdate(dt) {
      this.yaw = Mathf.repeat(this.yaw + this.autoRotate * dt, 360);
      this.pitch = Mathf.clamp(this.pitch, this.minPitch, this.maxPitch);
      const center = this.target && !this.target.destroyed ? this.target.transform.position : this.center;
      const rotation = Quaternion.euler(-this.pitch, this.yaw, 0);
      this.transform.position = center.add(rotation.rotate(Vector3.back).scale(this.distance));
      this.transform.lookAt(center);
    }
  }
  var _init5, _base5, _dec31, _dec32, _dec33, _dec34, _dec35, _dec36;

  class WorldPanel extends (_base5 = Component) {
    static {
      _init5 = __decoratorStart(_base5);
      __decorateElement(_init5, 2, "width", _dec31, this);
      __decorateElement(_init5, 2, "height", _dec32, this);
      __decorateElement(_init5, 2, "pixelsPerMeter", _dec33, this);
      __decorateElement(_init5, 2, "resolution", _dec34, this);
      __decorateElement(_init5, 2, "tint", _dec35, this);
      __decorateElement(_init5, 2, "interactive", _dec36, this);
      __decoratorMetadata(_init5, this);
    }
    _width = (__runInitializers(_init5, 5, this), 800);
    _height = 600;
    _pixelsPerMeter = 500;
    _resolution = 2;
    _tint = Color.white;
    _interactive = true;
    started = false;
    get [(_dec31 = [
      property
    ], "width")]() {
      return this._width;
    }
    set width(v) {
      this._width = v;
      this.send();
    }
    get [(_dec32 = [
      property
    ], "height")]() {
      return this._height;
    }
    set height(v) {
      this._height = v;
      this.send();
    }
    get [(_dec33 = [
      property
    ], "pixelsPerMeter")]() {
      return this._pixelsPerMeter;
    }
    set pixelsPerMeter(v) {
      this._pixelsPerMeter = v;
      this.send();
    }
    get [(_dec34 = [
      property
    ], "resolution")]() {
      return this._resolution;
    }
    set resolution(v) {
      this._resolution = v;
      this.send();
    }
    get [(_dec35 = [
      property
    ], "tint")]() {
      return this._tint;
    }
    set tint(c) {
      this._tint = c;
      this.send();
    }
    get [(_dec36 = [
      property
    ], "interactive")]() {
      return this._interactive;
    }
    set interactive(on) {
      this._interactive = on;
      this.send();
    }
    get worldSize() {
      return new Vector2(this._width / this._pixelsPerMeter, this._height / this._pixelsPerMeter);
    }
    send() {
      if (!this.started)
        return;
      __iron_op("render.panel", this.id, this.gameObject.id, [this._width, this._height], this._pixelsPerMeter, this._resolution, rgba(this._tint), this._interactive);
    }
    onStart() {
      this.started = true;
      this.send();
    }
    onDestroy() {
      __iron_op("render.remove", this.id);
    }
  }
  var EXTRAS = {
    normalMap: null,
    normalScale: 1,
    metallicGlossMap: null,
    specularColor: null,
    occlusionMap: null,
    occlusionStrength: 1,
    emissionMap: null,
    detailAlbedoMap: null,
    detailAlbedoMapScale: Vector2.one,
    detailAlbedoMapOffset: Vector2.zero,
    renderingMode: "opaque",
    alphaCutoff: 0.5,
    doubleSided: false,
    gltfPacking: false
  };

  class Material {
    color;
    emission;
    smoothness;
    unlit;
    mainTexture;
    mainTextureScale;
    mainTextureOffset;
    metallic;
    normalMap = EXTRAS.normalMap;
    normalScale = EXTRAS.normalScale;
    metallicGlossMap = EXTRAS.metallicGlossMap;
    specularColor = EXTRAS.specularColor;
    occlusionMap = EXTRAS.occlusionMap;
    occlusionStrength = EXTRAS.occlusionStrength;
    emissionMap = EXTRAS.emissionMap;
    detailAlbedoMap = EXTRAS.detailAlbedoMap;
    detailAlbedoMapScale = EXTRAS.detailAlbedoMapScale;
    detailAlbedoMapOffset = EXTRAS.detailAlbedoMapOffset;
    renderingMode = EXTRAS.renderingMode;
    alphaCutoff = EXTRAS.alphaCutoff;
    doubleSided = EXTRAS.doubleSided;
    gltfPacking = EXTRAS.gltfPacking;
    constructor(color = Color.white, emission = Color.black, smoothness = 0.5, unlit = false, mainTexture = null, mainTextureScale = Vector2.one, mainTextureOffset = Vector2.zero, metallic = 0) {
      this.color = color;
      this.emission = emission;
      this.smoothness = smoothness;
      this.unlit = unlit;
      this.mainTexture = mainTexture;
      this.mainTextureScale = mainTextureScale;
      this.mainTextureOffset = mainTextureOffset;
      this.metallic = metallic;
    }
    static create(o = {}) {
      return new Material().with(o);
    }
    with(o) {
      const v2 = (v, d) => v ? new Vector2(v.x, v.y) : d;
      const m = new Material(o.color ?? this.color, o.emission ?? this.emission, o.smoothness ?? this.smoothness, o.unlit ?? this.unlit, o.mainTexture === undefined ? this.mainTexture : o.mainTexture, v2(o.mainTextureScale, this.mainTextureScale), v2(o.mainTextureOffset, this.mainTextureOffset), o.metallic ?? this.metallic);
      const out = m;
      for (const key of Object.keys(EXTRAS)) {
        const given = o[key];
        out[key] = given === undefined ? this[key] : given;
      }
      out.detailAlbedoMapScale = v2(o.detailAlbedoMapScale, this.detailAlbedoMapScale);
      out.detailAlbedoMapOffset = v2(o.detailAlbedoMapOffset, this.detailAlbedoMapOffset);
      return m;
    }
    withColor(color) {
      return this.with({ color });
    }
    withTexture(texture, scale = Vector2.one, offset = Vector2.zero) {
      return this.with({ mainTexture: texture, mainTextureScale: scale, mainTextureOffset: offset });
    }
    _extras() {
      const s = this.specularColor;
      return [
        this.normalMap?.id ?? null,
        this.normalScale,
        this.metallicGlossMap?.id ?? null,
        this.gltfPacking,
        this.occlusionMap?.id ?? null,
        this.occlusionStrength,
        this.emissionMap?.id ?? null,
        this.detailAlbedoMap?.id ?? null,
        [this.detailAlbedoMapScale.x, this.detailAlbedoMapScale.y, this.detailAlbedoMapOffset.x, this.detailAlbedoMapOffset.y],
        s ? [s.r, s.g, s.b] : null,
        this.renderingMode,
        this.alphaCutoff,
        this.doubleSided
      ];
    }
  }
  var nextMesh = 1;

  class Mesh {
    id;
    _saved = null;
    _flat = null;
    get _data() {
      if (this._flat) {
        const f = this._flat;
        this._saved = { vertices: Array.from(f.positions), triangles: Array.from(f.triangles), normals: Array.from(f.normals ?? []), uv: Array.from(f.uv ?? []) };
        this._flat = null;
      }
      return this._saved;
    }
    set _data(d) {
      this._saved = d;
      this._flat = null;
    }
    bindposes = [];
    constructor(id) {
      this.id = id;
    }
    static _engine(id) {
      return new Mesh(id);
    }
    static _reserve() {
      return new Mesh(nextMesh++);
    }
    static create(data) {
      const mesh = new Mesh(nextMesh++);
      mesh.set(data);
      return mesh;
    }
    set(data) {
      const joints = (data.boneWeights ?? []).flatMap((b) => [...b.indices]);
      const weights = (data.boneWeights ?? []).flatMap((b) => {
        const sum = b.weights.reduce((s, w) => s + w, 0);
        return sum > 0 ? b.weights.map((w) => w / sum) : [1, 0, 0, 0];
      });
      this._data = {
        vertices: data.vertices.flatMap((v) => [v.x, v.y, v.z]),
        triangles: [...data.triangles],
        normals: (data.normals ?? []).flatMap((v) => [v.x, v.y, v.z]),
        uv: (data.uv ?? []).flatMap((v) => [v.x, v.y]),
        ...joints.length ? { joints, weights } : {}
      };
      if (data.bindposes)
        this.bindposes = [...data.bindposes];
      __iron_op("render.mesh", this.id, data.vertices.flatMap((v) => [v.x, v.y, v.z]), (data.normals ?? []).flatMap((v) => [v.x, v.y, v.z]), (data.uv ?? []).flatMap((v) => [v.x, 1 - v.y]), [...data.triangles], ...joints.length ? [joints, weights] : []);
    }
    static createFlat(data) {
      const mesh = new Mesh(nextMesh++);
      mesh.setFlat(data);
      return mesh;
    }
    setFlat(data) {
      const uv = data.uv ?? [];
      const flipped = new Float32Array(uv.length);
      for (let i = 0;i < uv.length; i += 2) {
        flipped[i] = uv[i];
        flipped[i + 1] = 1 - uv[i + 1];
      }
      this._flat = data;
      __iron_op("render.mesh", this.id, data.positions, data.normals ?? [], flipped, data.triangles);
    }
    destroy() {
      __iron_op("render.mesh", this.id, null);
    }
  }
  var _init6, _base6, _dec37, _dec38, _dec39;

  class MeshRenderer extends (_base6 = Component) {
    static {
      _init6 = __decoratorStart(_base6);
      __decorateElement(_init6, 2, "shadowCastingMode", _dec37, this);
      __decorateElement(_init6, 2, "mesh", _dec38, this);
      __decorateElement(_init6, 2, "material", _dec39, this);
      __decoratorMetadata(_init6, this);
    }
    _mesh = (__runInitializers(_init6, 5, this), "cube");
    _material = new Material;
    _shadowCastingMode = "on";
    started = false;
    get [(_dec37 = [
      property
    ], "shadowCastingMode")]() {
      return this._shadowCastingMode;
    }
    set shadowCastingMode(m) {
      this._shadowCastingMode = m;
      this.send();
    }
    get [(_dec38 = [
      property
    ], "mesh")]() {
      return this._mesh;
    }
    set mesh(m) {
      this._mesh = m;
      this.send();
    }
    get [(_dec39 = [
      property
    ], "material")]() {
      return this._material;
    }
    set material(m) {
      this._material = m;
      this.send();
    }
    _skinArgs() {
      return [];
    }
    send() {
      if (!this.started)
        return;
      const m = this._material;
      __iron_op("render.meshRenderer", this.id, this.gameObject.id, typeof this._mesh === "string" ? this._mesh : this._mesh.id, rgba(m.color), [m.emission.r, m.emission.g, m.emission.b], m.smoothness, m.unlit, m.mainTexture?.id ?? null, [m.mainTextureScale.x, m.mainTextureScale.y, m.mainTextureOffset.x, m.mainTextureOffset.y], this._shadowCastingMode !== "off", m.metallic, m._extras(), ...this._skinArgs());
    }
    onAwake() {
      this.started = true;
      this.send();
    }
    onDestroy() {
      __iron_op("render.remove", this.id);
    }
  }
  var _init7, _base7, _dec40, _dec41;

  class SkinnedMeshRenderer extends (_base7 = MeshRenderer) {
    constructor() {
      super(...arguments);
      __runInitializers(_init7, 11, this);
    }
    static {
      _init7 = __decoratorStart(_base7);
      __decorateElement(_init7, 2, "bones", _dec41, this);
      __decorateElement(_init7, 5, "rootBone", _dec40, this);
      __decoratorMetadata(_init7, this);
    }
    _bones = (__runInitializers(_init7, 5, this), []);
    [(_dec40 = [
      property
    ], "rootBone")] = __runInitializers(_init7, 8, this, null);
    get [(_dec41 = [
      property
    ], "bones")]() {
      return this._bones;
    }
    set bones(b) {
      this._bones = [...b];
      this.send();
    }
    _skinArgs() {
      const mesh = this.mesh;
      const bindposes = typeof mesh === "string" ? [] : mesh.bindposes;
      const count = Math.min(this._bones.length, bindposes.length);
      return [this._bones.slice(0, count).map((b) => b.id), bindposes.slice(0, count).flatMap((m) => [...m.m])];
    }
  }
  var _init8, _base8, _dec42, _dec43, _dec44, _dec45, _dec46, _dec47, _dec48, _dec49, _dec50, _dec51, _dec52, _dec53;

  class Light extends (_base8 = Component) {
    static {
      _init8 = __decoratorStart(_base8);
      __decorateElement(_init8, 2, "type", _dec42, this);
      __decorateElement(_init8, 2, "color", _dec43, this);
      __decorateElement(_init8, 2, "intensity", _dec44, this);
      __decorateElement(_init8, 2, "range", _dec45, this);
      __decorateElement(_init8, 2, "spotAngle", _dec46, this);
      __decorateElement(_init8, 2, "innerSpotAngle", _dec47, this);
      __decorateElement(_init8, 2, "haloSize", _dec48, this);
      __decorateElement(_init8, 2, "beamIntensity", _dec49, this);
      __decorateElement(_init8, 2, "shadows", _dec50, this);
      __decorateElement(_init8, 2, "shadowStrength", _dec51, this);
      __decorateElement(_init8, 2, "cullingMask", _dec52, this);
      __decorateElement(_init8, 2, "volumetricIntensity", _dec53, this);
      __decoratorMetadata(_init8, this);
    }
    _type = (__runInitializers(_init8, 5, this), "directional");
    _color = Color.white;
    _intensity = 1;
    _range = 10;
    _spotAngle = 30;
    _innerSpotAngle = 21.8;
    _haloSize = 0;
    _beamIntensity = 0;
    _shadows = "none";
    _shadowStrength = 1;
    _cullingMask = 4294967295;
    _volumetricIntensity = 1;
    _spotShape = "cone";
    _aspectRatio = 1;
    started = false;
    get [(_dec42 = [
      property
    ], "type")]() {
      return this._type;
    }
    set type(t) {
      this._type = t;
      this.send();
    }
    get [(_dec43 = [
      property
    ], "color")]() {
      return this._color;
    }
    set color(c) {
      this._color = c;
      this.send();
    }
    get [(_dec44 = [
      property
    ], "intensity")]() {
      return this._intensity;
    }
    set intensity(i) {
      this._intensity = i;
      this.send();
    }
    get [(_dec45 = [
      property
    ], "range")]() {
      return this._range;
    }
    set range(r) {
      this._range = r;
      this.send();
    }
    get [(_dec46 = [
      property
    ], "spotAngle")]() {
      return this._spotAngle;
    }
    set spotAngle(a) {
      this._spotAngle = a;
      this.send();
    }
    get [(_dec47 = [
      property
    ], "innerSpotAngle")]() {
      return this._innerSpotAngle;
    }
    set innerSpotAngle(a) {
      this._innerSpotAngle = a;
      this.send();
    }
    get [(_dec48 = [
      property
    ], "haloSize")]() {
      return this._haloSize;
    }
    set haloSize(s) {
      this._haloSize = s;
      this.send();
    }
    get [(_dec49 = [
      property
    ], "beamIntensity")]() {
      return this._beamIntensity;
    }
    set beamIntensity(b) {
      this._beamIntensity = b;
      this.send();
    }
    get [(_dec50 = [
      property
    ], "shadows")]() {
      return this._shadows;
    }
    set shadows(s) {
      this._shadows = s;
      this.send();
    }
    get [(_dec51 = [
      property
    ], "shadowStrength")]() {
      return this._shadowStrength;
    }
    set shadowStrength(s) {
      this._shadowStrength = s;
      this.send();
    }
    get [(_dec52 = [
      property
    ], "cullingMask")]() {
      return this._cullingMask;
    }
    set cullingMask(mask) {
      this._cullingMask = mask >>> 0;
      this.send();
    }
    get [(_dec53 = [
      property
    ], "volumetricIntensity")]() {
      return this._volumetricIntensity;
    }
    set volumetricIntensity(v) {
      this._volumetricIntensity = Math.max(0, v);
      this.send();
    }
    get spotShape() {
      return this._spotShape;
    }
    set spotShape(s) {
      this._spotShape = s;
      this.send();
    }
    get aspectRatio() {
      return this._aspectRatio;
    }
    set aspectRatio(a) {
      this._aspectRatio = Math.max(0.001, a);
      this.send();
    }
    send() {
      if (!this.started)
        return;
      const c = this._color;
      __iron_op("render.light", this.id, this.gameObject.id, this._type, [c.r, c.g, c.b], this._intensity, this._range, this._spotAngle, this._innerSpotAngle, this._haloSize, this._beamIntensity, this._shadows, this._shadowStrength, this._cullingMask, this._volumetricIntensity, this._spotShape, this._aspectRatio);
    }
    onAwake() {
      this.started = true;
      allLights.add(this);
      this.send();
    }
    onDestroy() {
      allLights.delete(this);
      __iron_op("render.remove", this.id);
    }
    static illuminanceAt(point, normal) {
      let sum = 0;
      for (const l of allLights)
        sum += l.illuminanceFrom(point, normal);
      return sum;
    }
    static async measureIlluminance(point, normal) {
      const parts = [...allLights].map(async (l) => {
        const lux = l.illuminanceFrom(point, normal);
        if (lux <= 0)
          return 0;
        let dir;
        let distance;
        if (l._type === "directional") {
          const f = l.transform.forward;
          dir = { x: -f.x, y: -f.y, z: -f.z };
          distance = 1e4;
        } else {
          const p = l.transform.position;
          dir = { x: p.x - point.x, y: p.y - point.y, z: p.z - point.z };
          distance = Math.hypot(dir.x, dir.y, dir.z) - 0.05;
        }
        const len = Math.hypot(dir.x, dir.y, dir.z) || 1;
        const from = { x: point.x + dir.x / len * 0.02, y: point.y + dir.y / len * 0.02, z: point.z + dir.z / len * 0.02 };
        const hit = await physics.raycast(from, dir, distance);
        return hit ? 0 : lux;
      });
      return (await Promise.all(parts)).reduce((a, b) => a + b, 0);
    }
    illuminanceFrom(point, normal) {
      const l = this;
      if (!l.gameObject.activeInHierarchy)
        return 0;
      const forward = l.transform.forward;
      if (l._type === "directional") {
        const cos = normal ? Math.max(0, -(normal.x * forward.x + normal.y * forward.y + normal.z * forward.z)) : 1;
        return l._intensity * cos;
      }
      const p = l.transform.position;
      const dx = point.x - p.x;
      const dy = point.y - p.y;
      const dz = point.z - p.z;
      const d = Math.hypot(dx, dy, dz);
      if (d >= l._range || d === 0)
        return 0;
      const x = d / l._range;
      const fade = (1 - x ** 4) ** 2;
      let candela = l._intensity / (4 * Math.PI);
      if (l._type === "spot") {
        const c = (dx * forward.x + dy * forward.y + dz * forward.z) / d;
        const outer = Math.cos(l._spotAngle / 2 * Math.PI / 180);
        const inner = Math.cos(l._innerSpotAngle / 2 * Math.PI / 180);
        const t = Math.min(1, Math.max(0, (c - outer) / Math.max(inner - outer, 0.0001)));
        candela *= t * t * (3 - 2 * t);
      }
      const cos = normal ? Math.max(0, -(normal.x * dx + normal.y * dy + normal.z * dz) / d) : 1;
      return candela * fade / (d * d) * cos;
    }
  }
  var allLights = new Set;
  var ambient = new Color(0.2, 0.2, 0.22);
  var RenderStats = {
    meshes: 0,
    culledMeshes: 0,
    quads: 0,
    culledQuads: 0,
    fogCulled: 0,
    lights: 0,
    culledLights: 0,
    shadowMaps: 0
  };
  globalThis.__iron_render_stats = (...n) => {
    [RenderStats.meshes, RenderStats.culledMeshes, RenderStats.quads, RenderStats.culledQuads, RenderStats.fogCulled, RenderStats.lights, RenderStats.culledLights, RenderStats.shadowMaps] = n.map((v) => v ?? 0);
  };
  var bloom = null;
  var tonemapping = "none";
  var postExposure = 0;
  var METERING = { average: 0, centerWeighted: 1, spot: 2 };
  var fixedExposure = null;
  var exposureTransition = 0;
  globalThis.__iron_exposure = (id, ev, scene) => {
    const c = Camera.byId(id);
    if (!c)
      return;
    c._exposure = ev;
    c._sceneBrightness = scene ?? Number.NaN;
  };
  var autoExposure = null;
  var sendPost = () => __iron_op("render.post", 0, postExposure, tonemapping, autoExposure && QualitySettings.autoExposure ? [
    autoExposure.minEV ?? -10,
    autoExposure.maxEV ?? 20,
    autoExposure.compensation ?? 0,
    autoExposure.speedDarkToLight ?? 3,
    autoExposure.speedLightToDark ?? 3,
    METERING[autoExposure.metering ?? "centerWeighted"],
    autoExposure.lowPercent ?? 0,
    autoExposure.highPercent ?? 100,
    autoExposure.hold ? 1 : 0,
    autoExposure.darkAdaptation ?? 1,
    autoExposure.darkBelow ?? 5,
    Math.min(8, autoExposure.curve?.length ?? 0),
    ...(autoExposure.curve ?? []).slice(0, 8).flat()
  ] : null, fixedExposure, exposureTransition);
  _setPostHook(sendPost);
  var fog = false;
  var fogMode = "exponentialSquared";
  var fogColor = new Color(0.5, 0.5, 0.5);
  var fogDensity = 0.01;
  var fogStart = 0;
  var fogEnd = 300;
  var heightFog = null;
  var cloudLayer = null;
  function sendClouds() {
    const c = cloudLayer;
    const t = c?.tint;
    __iron_op("render.clouds", 0, c ? [c.altitude ?? 1500, c.size ?? 2000, c.coverage ?? 0.5, c.opacity ?? 1, c.softness ?? 0.3, t?.r ?? 0.8, t?.g ?? 0.8, t?.b ?? 0.82, c.windSpeed ?? 0, c.windDirection ?? 0, c.fadeDistance ?? 20000] : null);
  }
  var fogCulling = true;
  var DENOISING = { none: 0, reprojection: 1, gaussian: 2, both: 3 };
  var VOLUME_DETAILS = ["low", "medium", "high", "ultra"];
  var volumetricFog = null;
  function sendFog() {
    if (!fog) {
      __iron_op("render.fog", 0, null);
      return;
    }
    const h = heightFog ? [heightFog.density, heightFog.baseHeight, heightFog.falloff] : null;
    const v = volumetricFog;
    const a = v?.albedo;
    const shafts = v ? [v.intensity ?? 1, v.anisotropy ?? 0.4, v.steps ?? 4, v.distance ?? 50, a?.r ?? 1, a?.g ?? 1, a?.b ?? 1, v.sharpDistance ?? 0, v.sharpSamples ?? 32, Math.max(0, VOLUME_DETAILS.indexOf(v.resolution ?? "high")), DENOISING[v.denoising ?? "both"], v.historyWeight ?? 0.9, v.samples ?? 1, v.motionSamples ?? 2] : null;
    __iron_op("render.fog", 0, fogMode, [fogColor.r, fogColor.g, fogColor.b], fogDensity, fogStart, fogEnd, h, shafts, fogCulling);
  }
  var skybox = null;
  var ambientMode = "skybox";
  var reflectionIntensity = 1;
  function sendSky() {
    if (!skybox) {
      __iron_op("render.sky", 0, null);
      return;
    }
    const s = skybox;
    const rgb = (c) => c ? [c.r, c.g, c.b] : null;
    const sun = s.sunDirection === null ? null : s.sunDirection ? [s.sunDirection.x, s.sunDirection.y, s.sunDirection.z] : [0.4, 0.6, 0.3];
    __iron_op("render.sky", 0, rgb(s.zenith) ?? [0.16, 0.32, 0.62], rgb(s.horizon) ?? [0.62, 0.72, 0.82], rgb(s.ground) ?? [0.22, 0.2, 0.18], sun, rgb(s.sunColor) ?? [20, 18, 15], s.sunSize ?? 1.5, reflectionIntensity, ambientMode === "skybox");
  }
  var RenderSettings = {
    get ambientLight() {
      return ambient;
    },
    set ambientLight(c) {
      ambient = c;
      __iron_op("render.ambient", 0, [c.r, c.g, c.b]);
    },
    get skybox() {
      return skybox;
    },
    set skybox(s) {
      skybox = s ? { ...s } : null;
      sendSky();
    },
    get ambientMode() {
      return ambientMode;
    },
    set ambientMode(m) {
      ambientMode = m;
      if (skybox)
        sendSky();
    },
    get reflectionIntensity() {
      return reflectionIntensity;
    },
    set reflectionIntensity(i) {
      reflectionIntensity = i;
      if (skybox)
        sendSky();
    },
    get bloom() {
      return bloom;
    },
    set bloom(b) {
      bloom = b ? { ...b } : null;
      if (b)
        __iron_op("render.bloom", 0, b.threshold, b.intensity, b.scatter);
      else
        __iron_op("render.bloom", 0, null);
    },
    get tonemapping() {
      return tonemapping;
    },
    set tonemapping(t) {
      tonemapping = t;
      sendPost();
    },
    get postExposure() {
      return postExposure;
    },
    set postExposure(ev) {
      postExposure = ev;
      sendPost();
    },
    get autoExposure() {
      return autoExposure;
    },
    set autoExposure(a) {
      autoExposure = a ? { ...a, curve: a.curve?.map((p) => [p[0], p[1]]) } : null;
      sendPost();
    },
    get fixedExposure() {
      return fixedExposure;
    },
    set fixedExposure(ev) {
      fixedExposure = ev;
      sendPost();
    },
    get exposureTransition() {
      return exposureTransition;
    },
    set exposureTransition(s) {
      exposureTransition = Math.max(0, s);
      sendPost();
    },
    freezeExposure() {
      const ev = Camera.main?.exposure;
      if (ev !== undefined && Number.isFinite(ev))
        fixedExposure = ev;
      autoExposure = null;
      sendPost();
    },
    get fog() {
      return fog;
    },
    set fog(on) {
      fog = on;
      sendFog();
    },
    get fogMode() {
      return fogMode;
    },
    set fogMode(m) {
      fogMode = m;
      if (fog)
        sendFog();
    },
    get fogColor() {
      return fogColor;
    },
    set fogColor(c) {
      fogColor = c;
      if (fog)
        sendFog();
    },
    get fogDensity() {
      return fogDensity;
    },
    set fogDensity(d) {
      fogDensity = Math.max(0, d);
      if (fog)
        sendFog();
    },
    get fogStartDistance() {
      return fogStart;
    },
    set fogStartDistance(d) {
      fogStart = d;
      if (fog)
        sendFog();
    },
    get fogEndDistance() {
      return fogEnd;
    },
    set fogEndDistance(d) {
      fogEnd = d;
      if (fog)
        sendFog();
    },
    get heightFog() {
      return heightFog;
    },
    get cloudLayer() {
      return cloudLayer;
    },
    set cloudLayer(c) {
      cloudLayer = c ? { ...c } : null;
      sendClouds();
    },
    set heightFog(h) {
      heightFog = h ? { ...h } : null;
      if (fog)
        sendFog();
    },
    get fogCulling() {
      return fogCulling;
    },
    set fogCulling(on) {
      fogCulling = on;
      if (fog)
        sendFog();
    },
    get volumetricFog() {
      return volumetricFog;
    },
    set volumetricFog(v) {
      volumetricFog = v ? { ...v } : null;
      if (fog)
        sendFog();
    }
  };
  var _init9, _base9, _dec54, _dec55;

  class Mirror extends (_base9 = Component) {
    constructor() {
      super(...arguments);
      __runInitializers(_init9, 15, this);
    }
    static {
      _init9 = __decoratorStart(_base9);
      __decorateElement(_init9, 5, "normal", _dec54, this);
      __decorateElement(_init9, 5, "cullingMask", _dec55, this);
      __decoratorMetadata(_init9, this);
    }
    [(_dec54 = [
      property
    ], "normal")] = __runInitializers(_init9, 8, this, null);
    [(_dec55 = [
      property
    ], "cullingMask")] = (__runInitializers(_init9, 11, this), __runInitializers(_init9, 12, this, 4294967295));
    send() {
      const mesh = this.getComponent(MeshRenderer)?.mesh;
      const n = this.normal ?? (mesh === "quad" ? Vector3.back : Vector3.up);
      __iron_op("render.mirror", this.id, this.gameObject.id, [n.x, n.y, n.z], this.cullingMask >>> 0);
    }
    onAwake() {
      this.send();
    }
    onDestroy() {
      __iron_op("render.remove", this.id);
    }
  }
  var _init10, _base10, _dec56, _dec57, _dec58, _dec59, _dec60;

  class ReflectionProbe extends (_base10 = Component) {
    constructor() {
      super(...arguments);
      __runInitializers(_init10, 27, this);
    }
    static {
      _init10 = __decoratorStart(_base10);
      __decorateElement(_init10, 5, "size", _dec56, this);
      __decorateElement(_init10, 5, "resolution", _dec57, this);
      __decorateElement(_init10, 5, "boxProjection", _dec58, this);
      __decorateElement(_init10, 5, "cullingMask", _dec59, this);
      __decorateElement(_init10, 5, "refreshMode", _dec60, this);
      __decoratorMetadata(_init10, this);
    }
    [(_dec56 = [
      property
    ], "size")] = __runInitializers(_init10, 8, this, new Vector3(10, 10, 10));
    [(_dec57 = [
      property
    ], "resolution")] = (__runInitializers(_init10, 11, this), __runInitializers(_init10, 12, this, 128));
    [(_dec58 = [
      property
    ], "boxProjection")] = (__runInitializers(_init10, 15, this), __runInitializers(_init10, 16, this, false));
    [(_dec59 = [
      property
    ], "cullingMask")] = (__runInitializers(_init10, 19, this), __runInitializers(_init10, 20, this, 4294967295));
    [(_dec60 = [
      property
    ], "refreshMode")] = (__runInitializers(_init10, 23, this), __runInitializers(_init10, 24, this, "onAwake"));
    send() {
      const s = this.size;
      __iron_op("render.probe", this.id, this.gameObject.id, [s.x, s.y, s.z], this.resolution, this.boxProjection, this.refreshMode === "everyFrame", this.cullingMask >>> 0);
    }
    renderProbe() {
      __iron_op("render.probeRender", this.id);
    }
    onAwake() {
      this.send();
    }
    onDestroy() {
      __iron_op("render.remove", this.id);
    }
  }
  var _init11, _base11, _dec61, _dec62, _dec63, _dec64, _dec65;

  class FogVolume extends (_base11 = Component) {
    static {
      _init11 = __decoratorStart(_base11);
      __decorateElement(_init11, 2, "size", _dec61, this);
      __decorateElement(_init11, 2, "density", _dec62, this);
      __decorateElement(_init11, 2, "color", _dec63, this);
      __decorateElement(_init11, 2, "blendDistance", _dec64, this);
      __decorateElement(_init11, 2, "shape", _dec65, this);
      __decoratorMetadata(_init11, this);
    }
    _size = (__runInitializers(_init11, 5, this), new Vector3(10, 2, 10));
    _density = 0.5;
    _color = new Color(0.75, 0.78, 0.82, 1);
    _blend = 1;
    _shape = "box";
    started = false;
    get [(_dec61 = [
      property
    ], "size")]() {
      return this._size;
    }
    set size(s) {
      this._size = { x: s.x, y: s.y, z: s.z };
      this.send();
    }
    get [(_dec62 = [
      property
    ], "density")]() {
      return this._density;
    }
    set density(d) {
      this._density = Math.max(0, d);
      this.send();
    }
    get [(_dec63 = [
      property
    ], "color")]() {
      return this._color;
    }
    set color(c) {
      this._color = c;
      this.send();
    }
    get [(_dec64 = [
      property
    ], "blendDistance")]() {
      return this._blend;
    }
    set blendDistance(b) {
      this._blend = Math.max(0, b);
      this.send();
    }
    get [(_dec65 = [
      property
    ], "shape")]() {
      return this._shape;
    }
    set shape(s) {
      this._shape = s;
      this.send();
    }
    send() {
      if (!this.started)
        return;
      const s = this._size;
      const c = this._color;
      __iron_op("render.fogVolume", this.id, this.gameObject.id, [s.x, s.y, s.z], this._density, [c.r, c.g, c.b], this._blend, this._shape);
    }
    onAwake() {
      this.started = true;
      this.send();
    }
    onDestroy() {
      __iron_op("render.remove", this.id);
    }
  }
  var _init12, _base12, _dec66, _dec67;

  class LODGroup extends (_base12 = Component) {
    static {
      _init12 = __decoratorStart(_base12);
      __decorateElement(_init12, 2, "size", _dec66, this);
      __decorateElement(_init12, 2, "lods", _dec67, this);
      __decoratorMetadata(_init12, this);
    }
    _lods = (__runInitializers(_init12, 5, this), []);
    _size = 1;
    _forced = -1;
    started = false;
    get [(_dec66 = [
      property
    ], "size")]() {
      return this._size;
    }
    set size(s) {
      this._size = s;
      this.send();
    }
    get lodCount() {
      return this._lods.length;
    }
    get [(_dec67 = [
      property
    ], "lods")]() {
      return this.getLODs();
    }
    set lods(lods) {
      this.setLODs(lods);
    }
    getLODs() {
      return this._lods.map((l) => ({ ...l, renderers: [...l.renderers] }));
    }
    setLODs(lods) {
      for (let i = 1;i < lods.length; i++) {
        if (lods[i].screenRelativeTransitionHeight > lods[i - 1].screenRelativeTransitionHeight) {
          throw new Error("LOD transition heights must go down from the first level");
        }
      }
      this._lods = lods.map((l) => ({ ...l, renderers: [...l.renderers] }));
      this.send();
    }
    forceLOD(index) {
      this._forced = index;
      this.send();
    }
    send() {
      if (!this.started)
        return;
      __iron_op("render.lodGroup", this.id, this.gameObject.id, this._size, this._lods.map((l) => l.screenRelativeTransitionHeight), this._lods.map((l) => l.renderers.map((r) => r.id)), this._forced);
    }
    onAwake() {
      this.started = true;
      this.send();
    }
    onDestroy() {
      __iron_op("render.remove", this.id);
    }
  }

  // ../../js/engine/src/lines.ts
  function catmullRom(a, b, c, d, t) {
    const t2 = t * t;
    const t3 = t2 * t;
    const k = (p0, p1, p2, p3) => 0.5 * (2 * p1 + (-p0 + p2) * t + (2 * p0 - 5 * p1 + 4 * p2 - p3) * t2 + (-p0 + 3 * p1 - 3 * p2 + p3) * t3);
    return new Vector3(k(a.x, b.x, c.x, d.x), k(a.y, b.y, c.y, d.y), k(a.z, b.z, c.z, d.z));
  }
  function tubeMesh(points, radius, sides, smoothing) {
    const src = points.map((p) => Vector3.from(p));
    const path = [];
    for (let i = 0;i < src.length - 1; i++) {
      const a = src[Math.max(0, i - 1)];
      const b = src[i];
      const c = src[i + 1];
      const d = src[Math.min(src.length - 1, i + 2)];
      for (let s = 0;s <= smoothing; s++)
        path.push(catmullRom(a, b, c, d, s / (smoothing + 1)));
    }
    path.push(src[src.length - 1]);
    const n = path.length;
    const tangents = path.map((_, i) => path[Math.min(n - 1, i + 1)].sub(path[Math.max(0, i - 1)]).normalized);
    let normal = Math.abs(tangents[0].y) < 0.9 ? Vector3.up : Vector3.right;
    normal = normal.sub(tangents[0].scale(normal.dot(tangents[0]))).normalized;
    const vertices = [];
    const normals = [];
    const uv = [];
    let along = 0;
    for (let i = 0;i < n; i++) {
      const t = tangents[i];
      if (i > 0) {
        along += path[i].distance(path[i - 1]);
        const fixed = normal.sub(t.scale(normal.dot(t)));
        if (fixed.magnitude > 0.000001)
          normal = fixed.normalized;
      }
      const binormal = t.cross(normal);
      for (let s = 0;s <= sides; s++) {
        const a = s / sides * Math.PI * 2;
        const dir = normal.scale(Math.cos(a)).add(binormal.scale(Math.sin(a)));
        vertices.push(path[i].add(dir.scale(radius)));
        normals.push(dir);
        uv.push({ x: s / sides, y: along });
      }
    }
    const triangles = [];
    const ring = sides + 1;
    for (let i = 0;i < n - 1; i++) {
      for (let s = 0;s < sides; s++) {
        const a = i * ring + s;
        const b = a + ring;
        triangles.push(a, a + 1, b, a + 1, b + 1, b);
      }
    }
    return { vertices, normals, uv, triangles };
  }
  var _init13, _base13, _dec68, _dec69, _dec70, _dec71, _dec72;

  class LineRenderer extends (_base13 = Component) {
    static {
      _init13 = __decoratorStart(_base13);
      __decorateElement(_init13, 5, "positions", _dec68, this);
      __decorateElement(_init13, 5, "width", _dec69, this);
      __decorateElement(_init13, 5, "sides", _dec70, this);
      __decorateElement(_init13, 5, "smoothing", _dec71, this);
      __decorateElement(_init13, 5, "material", _dec72, this);
      __decoratorMetadata(_init13, this);
    }
    [(_dec68 = [
      property
    ], "positions")] = __runInitializers(_init13, 8, this, []);
    [(_dec69 = [
      property
    ], "width")] = (__runInitializers(_init13, 11, this), __runInitializers(_init13, 12, this, 0.05));
    [(_dec70 = [
      property
    ], "sides")] = (__runInitializers(_init13, 15, this), __runInitializers(_init13, 16, this, 8));
    [(_dec71 = [
      property
    ], "smoothing")] = (__runInitializers(_init13, 19, this), __runInitializers(_init13, 20, this, 3));
    [(_dec72 = [
      property
    ], "material")] = (__runInitializers(_init13, 23, this), __runInitializers(_init13, 24, this, new Material(Color.white, Color.black, 0.3)));
    mesh = (__runInitializers(_init13, 27, this), null);
    renderer = null;
    drawn = "";
    onAwake() {
      this.mesh = Mesh.create({ vertices: [Vector3.zero, Vector3.zero, Vector3.zero], triangles: [0, 1, 2] });
      this.renderer = this.gameObject.addComponent(MeshRenderer, { mesh: this.mesh, material: this.material });
      this.redraw();
    }
    onPreRender() {
      this.redraw();
    }
    redraw() {
      if (!this.mesh)
        return;
      if (this.renderer && this.renderer.material !== this.material)
        this.renderer.material = this.material;
      const key = `${this.width},${this.sides},${this.smoothing},${this.positions.map((p) => `${p.x},${p.y},${p.z}`).join(";")}`;
      if (key === this.drawn || this.positions.length < 2)
        return;
      this.drawn = key;
      const local = this.positions.map((p) => this.transform.inverseTransformPoint(p));
      this.mesh.set(tubeMesh(local, this.width / 2, Math.max(3, Math.round(this.sides)), Math.max(0, Math.round(this.smoothing))));
    }
    onDestroy() {
      this.mesh?.destroy();
      this.mesh = null;
    }
  }
  var _init14, _base14, _dec73, _dec74, _dec75, _dec76, _dec77, _dec78, _dec79;

  class LineRenderer2D extends (_base14 = Component) {
    static {
      _init14 = __decoratorStart(_base14);
      __decorateElement(_init14, 2, "points", _dec73, this);
      __decorateElement(_init14, 2, "width", _dec74, this);
      __decorateElement(_init14, 2, "color", _dec75, this);
      __decorateElement(_init14, 2, "loop", _dec76, this);
      __decorateElement(_init14, 2, "fillColor", _dec77, this);
      __decorateElement(_init14, 2, "fillRule", _dec78, this);
      __decorateElement(_init14, 2, "sortingOrder", _dec79, this);
      __decoratorMetadata(_init14, this);
    }
    _points = (__runInitializers(_init14, 5, this), []);
    _width = 0.05;
    _color = Color.white;
    _loop = false;
    _fillColor = null;
    _fillRule = "evenodd";
    _sortingOrder = 0;
    started = false;
    dirty = true;
    get [(_dec73 = [
      property
    ], "points")]() {
      return this._points;
    }
    set points(p) {
      this._points = p;
      this.dirty = true;
    }
    get [(_dec74 = [
      property
    ], "width")]() {
      return this._width;
    }
    set width(w) {
      this._width = Math.max(0, w);
      this.dirty = true;
    }
    get [(_dec75 = [
      property
    ], "color")]() {
      return this._color;
    }
    set color(c) {
      this._color = c;
      this.dirty = true;
    }
    get [(_dec76 = [
      property
    ], "loop")]() {
      return this._loop;
    }
    set loop(on) {
      this._loop = on;
      this.dirty = true;
    }
    get [(_dec77 = [
      property
    ], "fillColor")]() {
      return this._fillColor;
    }
    set fillColor(c) {
      this._fillColor = c;
      this.dirty = true;
    }
    get [(_dec78 = [
      property
    ], "fillRule")]() {
      return this._fillRule;
    }
    set fillRule(r) {
      this._fillRule = r;
      this.dirty = true;
    }
    get [(_dec79 = [
      property
    ], "sortingOrder")]() {
      return this._sortingOrder;
    }
    set sortingOrder(o) {
      this._sortingOrder = Math.trunc(o);
      this.dirty = true;
    }
    onAwake() {
      this.started = true;
      this.send();
    }
    onPreRender() {
      if (this.dirty)
        this.send();
    }
    onDestroy() {
      __iron_op("render.remove", this.id);
    }
    send() {
      if (!this.started)
        return;
      this.dirty = false;
      const points = this._points;
      const flat = new Array(points.length * 2);
      for (let i = 0;i < points.length; i++) {
        const p = points[i];
        flat[i * 2] = p.x;
        flat[i * 2 + 1] = p.y;
      }
      const c = this._color;
      const f = this._fillColor;
      __iron_op("render.stroke", this.id, this.gameObject.id, this._width, [c.r, c.g, c.b, c.a], this._loop, flat, f ? [f.r, f.g, f.b, f.a] : null, this._fillRule, this._sortingOrder);
    }
  }

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
  var _init15, _base15, _dec80, _dec81, _dec82;

  class Collider extends (_base15 = Component) {
    static {
      _init15 = __decoratorStart(_base15);
      __decorateElement(_init15, 2, "center", _dec80, this);
      __decorateElement(_init15, 2, "isTrigger", _dec81, this);
      __decorateElement(_init15, 2, "material", _dec82, this);
      __decoratorMetadata(_init15, this);
    }
    _center = (__runInitializers(_init15, 5, this), Vector3.zero);
    _isTrigger = false;
    _material = new PhysicsMaterial;
    builtScale = null;
    handle = null;
    get [(_dec80 = [
      property
    ], "center")]() {
      return this._center;
    }
    set center(c) {
      this._center = Vector3.from(c);
      this.handle?.setOffset(this._center.mul(this.gameObject.transform.lossyScale));
    }
    get [(_dec81 = [
      property
    ], "isTrigger")]() {
      return this._isTrigger;
    }
    set isTrigger(on) {
      this._isTrigger = on;
      this.handle?.setSensor(on);
    }
    get [(_dec82 = [
      property
    ], "material")]() {
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
  var _init16, _base16, _dec83;

  class BoxCollider extends (_base16 = Collider) {
    static {
      _init16 = __decoratorStart(_base16);
      __decorateElement(_init16, 2, "size", _dec83, this);
      __decoratorMetadata(_init16, this);
    }
    _size = (__runInitializers(_init16, 5, this), Vector3.one);
    get [(_dec83 = [
      property
    ], "size")]() {
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
  var _init17, _base17, _dec84;

  class SphereCollider extends (_base17 = Collider) {
    static {
      _init17 = __decoratorStart(_base17);
      __decorateElement(_init17, 2, "radius", _dec84, this);
      __decoratorMetadata(_init17, this);
    }
    _radius = (__runInitializers(_init17, 5, this), 0.5);
    get [(_dec84 = [
      property
    ], "radius")]() {
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
  var _init18, _base18, _dec85, _dec86;

  class CapsuleCollider extends (_base18 = Collider) {
    static {
      _init18 = __decoratorStart(_base18);
      __decorateElement(_init18, 2, "radius", _dec85, this);
      __decorateElement(_init18, 2, "height", _dec86, this);
      __decoratorMetadata(_init18, this);
    }
    _radius = (__runInitializers(_init18, 5, this), 0.5);
    _height = 2;
    get [(_dec85 = [
      property
    ], "radius")]() {
      return this._radius;
    }
    set radius(r) {
      this._radius = r;
      this.rebuild();
    }
    get [(_dec86 = [
      property
    ], "height")]() {
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
  var _init19, _base19, _dec87, _dec88, _dec89, _dec90;

  class MeshCollider extends (_base19 = Collider) {
    constructor() {
      super(...arguments);
      __runInitializers(_init19, 23, this);
    }
    static {
      _init19 = __decoratorStart(_base19);
      __decorateElement(_init19, 5, "sharedMesh", _dec87, this);
      __decorateElement(_init19, 5, "points", _dec88, this);
      __decorateElement(_init19, 5, "indices", _dec89, this);
      __decorateElement(_init19, 5, "convex", _dec90, this);
      __decoratorMetadata(_init19, this);
    }
    [(_dec87 = [
      property
    ], "sharedMesh")] = __runInitializers(_init19, 8, this, null);
    [(_dec88 = [
      property
    ], "points")] = (__runInitializers(_init19, 11, this), __runInitializers(_init19, 12, this, []));
    [(_dec89 = [
      property
    ], "indices")] = (__runInitializers(_init19, 15, this), __runInitializers(_init19, 16, this, []));
    [(_dec90 = [
      property
    ], "convex")] = (__runInitializers(_init19, 19, this), __runInitializers(_init19, 20, this, false));
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
  var _init20, _base20, _dec91, _dec92, _dec93, _dec94, _dec95, _dec96, _dec97;

  class Rigidbody extends (_base20 = Component) {
    static {
      _init20 = __decoratorStart(_base20);
      __decorateElement(_init20, 2, "mass", _dec91, this);
      __decorateElement(_init20, 2, "drag", _dec92, this);
      __decorateElement(_init20, 2, "angularDrag", _dec93, this);
      __decorateElement(_init20, 2, "useGravity", _dec94, this);
      __decorateElement(_init20, 2, "isKinematic", _dec95, this);
      __decorateElement(_init20, 2, "constraints", _dec96, this);
      __decorateElement(_init20, 2, "collisionDetection", _dec97, this);
      __decoratorMetadata(_init20, this);
    }
    _mass = (__runInitializers(_init20, 5, this), 1);
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
    get [(_dec91 = [
      property
    ], "mass")]() {
      return this._mass;
    }
    set mass(kg) {
      this._mass = Math.max(0.000001, kg);
      this._sendMass();
    }
    get [(_dec92 = [
      property
    ], "drag")]() {
      return this._drag;
    }
    set drag(d) {
      this._drag = d;
      this.p?.body.setDamping(this._drag, this._angularDrag);
    }
    get [(_dec93 = [
      property
    ], "angularDrag")]() {
      return this._angularDrag;
    }
    set angularDrag(d) {
      this._angularDrag = d;
      this.p?.body.setDamping(this._drag, this._angularDrag);
    }
    get [(_dec94 = [
      property
    ], "useGravity")]() {
      return this._useGravity;
    }
    set useGravity(on) {
      this._useGravity = on;
      this.p?.body.setGravityScale(on ? 1 : 0);
    }
    get [(_dec95 = [
      property
    ], "isKinematic")]() {
      return this._isKinematic;
    }
    set isKinematic(on) {
      this._isKinematic = on;
      this.p?.body.setKind(on ? "kinematic" : "dynamic");
    }
    get [(_dec96 = [
      property
    ], "constraints")]() {
      return this._constraints;
    }
    set constraints(flags) {
      this._constraints = flags;
      this.sendConstraints();
    }
    get [(_dec97 = [
      property
    ], "collisionDetection")]() {
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
  var CollisionFlags = { None: 0, Sides: 1, Above: 2, Below: 4 };
  var _init21, _base21, _dec98, _dec99, _dec100, _dec101;

  class CharacterController extends (_base21 = Collider) {
    static {
      _init21 = __decoratorStart(_base21);
      __decorateElement(_init21, 2, "radius", _dec98, this);
      __decorateElement(_init21, 2, "height", _dec99, this);
      __decorateElement(_init21, 2, "slopeLimit", _dec100, this);
      __decorateElement(_init21, 2, "stepOffset", _dec101, this);
      __decoratorMetadata(_init21, this);
    }
    _radius = (__runInitializers(_init21, 5, this), 0.5);
    _height = 2;
    _slopeLimit = 45;
    _stepOffset = 0.3;
    fallSpeed = 0;
    last = Vector3.zero;
    _velocity = Vector3.zero;
    get [(_dec98 = [
      property
    ], "radius")]() {
      return this._radius;
    }
    set radius(r) {
      this._radius = r;
      this.rebuild();
    }
    get [(_dec99 = [
      property
    ], "height")]() {
      return this._height;
    }
    set height(h) {
      this._height = h;
      this.rebuild();
    }
    get [(_dec100 = [
      property
    ], "slopeLimit")]() {
      return this._slopeLimit;
    }
    set slopeLimit(d) {
      this._slopeLimit = d;
      this.configure();
    }
    get [(_dec101 = [
      property
    ], "stepOffset")]() {
      return this._stepOffset;
    }
    set stepOffset(h) {
      this._stepOffset = h;
      this.configure();
    }
    get isGrounded() {
      return objectPhysics.get(this.gameObject)?.body.grounded ?? false;
    }
    get velocity() {
      return this._velocity;
    }
    shape(scale) {
      const radius = this._radius * Math.max(Math.abs(scale.x), Math.abs(scale.z));
      const halfHeight = Math.max(0, this._height * Math.abs(scale.y) / 2 - radius);
      return { capsule: { halfHeight, radius } };
    }
    configure() {
      const body = objectPhysics.get(this.gameObject)?.body;
      body?.makeCharacter({ maxSlope: this._slopeLimit, snapToGround: this._stepOffset, stepHeight: this._stepOffset });
    }
    onAwake() {
      this.create();
      objectPhysics.get(this.gameObject).body.setKind("kinematic");
      this.configure();
      this.last = this.transform.position;
    }
    onUpdate(dt) {
      const now = this.transform.position;
      this._velocity = dt > 0 ? now.sub(this.last).scale(1 / dt) : Vector3.zero;
      this.last = now;
    }
    move(motion) {
      objectPhysics.get(this.gameObject)?.body.move(motion);
      return this.isGrounded ? CollisionFlags.Below : CollisionFlags.None;
    }
    simpleMove(speed) {
      const dt = Time.deltaTime;
      this.fallSpeed = this.isGrounded ? 1 : this.fallSpeed + 9.81 * dt;
      this.move(new Vector3(speed.x * dt, -this.fallSpeed * dt, speed.z * dt));
      return this.isGrounded;
    }
  }
  var _init22, _base22, _dec102, _dec103, _dec104, _dec105;

  class Joint extends (_base22 = Component) {
    static {
      _init22 = __decoratorStart(_base22);
      __decorateElement(_init22, 5, "connectedBody", _dec102, this);
      __decorateElement(_init22, 5, "anchor", _dec103, this);
      __decorateElement(_init22, 5, "connectedAnchor", _dec104, this);
      __decorateElement(_init22, 5, "autoConfigureConnectedAnchor", _dec105, this);
      __decoratorMetadata(_init22, this);
    }
    [(_dec102 = [
      property
    ], "connectedBody")] = __runInitializers(_init22, 8, this, null);
    [(_dec103 = [
      property
    ], "anchor")] = (__runInitializers(_init22, 11, this), __runInitializers(_init22, 12, this, Vector3.zero));
    [(_dec104 = [
      property
    ], "connectedAnchor")] = (__runInitializers(_init22, 15, this), __runInitializers(_init22, 16, this, Vector3.zero));
    [(_dec105 = [
      property
    ], "autoConfigureConnectedAnchor")] = (__runInitializers(_init22, 19, this), __runInitializers(_init22, 20, this, true));
    handle = (__runInitializers(_init22, 23, this), null);
    params() {
      return [];
    }
    build() {
      this.handle?.remove();
      this.handle = null;
      const body = objectPhysics.get(this.gameObject)?.body;
      if (!body) {
        console.warn(`${this.constructor.name} on ${this.gameObject.name} needs a Rigidbody`);
        return;
      }
      const anchor = this.anchor.mul(this.transform.lossyScale);
      const other = this.connectedBody?.body ?? null;
      if (this.autoConfigureConnectedAnchor) {
        const world = this.transform.transformPoint(this.anchor);
        const t = this.connectedBody?.transform;
        this.connectedAnchor = t ? t.rotation.inverse().rotate(world.sub(t.position)) : world;
      }
      this.handle = physics.createJoint(this.kind(), body, other, anchor, this.connectedAnchor, this.params());
    }
    onStart() {
      this.build();
    }
    onDestroy() {
      this.handle?.remove();
      this.handle = null;
    }
  }

  class FixedJoint extends Joint {
    kind() {
      return "fixed";
    }
  }

  class CharacterJoint extends Joint {
    kind() {
      return "ball";
    }
  }
  var _init23, _base23, _dec106, _dec107, _dec108, _dec109, _dec110;

  class HingeJoint extends (_base23 = Joint) {
    constructor() {
      super(...arguments);
      __runInitializers(_init23, 27, this);
    }
    static {
      _init23 = __decoratorStart(_base23);
      __decorateElement(_init23, 5, "axis", _dec106, this);
      __decorateElement(_init23, 5, "useLimits", _dec107, this);
      __decorateElement(_init23, 5, "limits", _dec108, this);
      __decorateElement(_init23, 5, "useMotor", _dec109, this);
      __decorateElement(_init23, 5, "motor", _dec110, this);
      __decoratorMetadata(_init23, this);
    }
    [(_dec106 = [
      property
    ], "axis")] = __runInitializers(_init23, 8, this, new Vector3(1, 0, 0));
    [(_dec107 = [
      property
    ], "useLimits")] = (__runInitializers(_init23, 11, this), __runInitializers(_init23, 12, this, false));
    [(_dec108 = [
      property
    ], "limits")] = (__runInitializers(_init23, 15, this), __runInitializers(_init23, 16, this, { min: -90, max: 90 }));
    [(_dec109 = [
      property
    ], "useMotor")] = (__runInitializers(_init23, 19, this), __runInitializers(_init23, 20, this, false));
    [(_dec110 = [
      property
    ], "motor")] = (__runInitializers(_init23, 23, this), __runInitializers(_init23, 24, this, { targetVelocity: 0, force: 1 }));
    kind() {
      return "hinge";
    }
    params() {
      const a = this.axis;
      return [a.x, a.y, a.z, this.useLimits ? 1 : 0, this.limits.min, this.limits.max, this.useMotor ? 1 : 0, this.motor.targetVelocity, this.motor.force];
    }
    refresh() {
      if (this.handle)
        this.build();
    }
  }
  var _init24, _base24, _dec111, _dec112, _dec113;

  class SpringJoint extends (_base24 = Joint) {
    constructor() {
      super(...arguments);
      __runInitializers(_init24, 19, this);
    }
    static {
      _init24 = __decoratorStart(_base24);
      __decorateElement(_init24, 5, "spring", _dec111, this);
      __decorateElement(_init24, 5, "damper", _dec112, this);
      __decorateElement(_init24, 5, "restLength", _dec113, this);
      __decoratorMetadata(_init24, this);
    }
    [(_dec111 = [
      property
    ], "spring")] = __runInitializers(_init24, 8, this, 10);
    [(_dec112 = [
      property
    ], "damper")] = (__runInitializers(_init24, 11, this), __runInitializers(_init24, 12, this, 0.2));
    [(_dec113 = [
      property
    ], "restLength")] = (__runInitializers(_init24, 15, this), __runInitializers(_init24, 16, this, null));
    kind() {
      return "spring";
    }
    params() {
      let rest = this.restLength;
      if (rest === null) {
        const world = this.transform.transformPoint(this.anchor);
        const t = this.connectedBody?.transform;
        const other = t ? t.position.add(t.rotation.rotate(this.connectedAnchor)) : this.connectedAnchor;
        rest = world.distance(other);
      }
      return [rest, this.spring, this.damper];
    }
  }
  var _init25, _base25, _dec114;

  class RopeJoint extends (_base25 = Joint) {
    constructor() {
      super(...arguments);
      __runInitializers(_init25, 11, this);
    }
    static {
      _init25 = __decoratorStart(_base25);
      __decorateElement(_init25, 5, "maxDistance", _dec114, this);
      __decoratorMetadata(_init25, this);
    }
    [(_dec114 = [
      property
    ], "maxDistance")] = __runInitializers(_init25, 8, this, 2);
    kind() {
      return "rope";
    }
    params() {
      return [this.maxDistance];
    }
  }
  var _init26, _base26, _dec115, _dec116, _dec117, _dec118, _dec119, _dec120, _dec121;

  class Rope extends (_base26 = Component) {
    static {
      _init26 = __decoratorStart(_base26);
      __decorateElement(_init26, 5, "length", _dec115, this);
      __decorateElement(_init26, 5, "segments", _dec116, this);
      __decorateElement(_init26, 5, "radius", _dec117, this);
      __decorateElement(_init26, 5, "segmentMass", _dec118, this);
      __decorateElement(_init26, 5, "color", _dec119, this);
      __decorateElement(_init26, 5, "end", _dec120, this);
      __decorateElement(_init26, 5, "renderMode", _dec121, this);
      __decoratorMetadata(_init26, this);
    }
    [(_dec115 = [
      property
    ], "length")] = __runInitializers(_init26, 8, this, 3);
    [(_dec116 = [
      property
    ], "segments")] = (__runInitializers(_init26, 11, this), __runInitializers(_init26, 12, this, 12));
    [(_dec117 = [
      property
    ], "radius")] = (__runInitializers(_init26, 15, this), __runInitializers(_init26, 16, this, 0.04));
    [(_dec118 = [
      property
    ], "segmentMass")] = (__runInitializers(_init26, 19, this), __runInitializers(_init26, 20, this, 0.05));
    [(_dec119 = [
      property
    ], "color")] = (__runInitializers(_init26, 23, this), __runInitializers(_init26, 24, this, new Color(0.55, 0.42, 0.28, 1)));
    [(_dec120 = [
      property
    ], "end")] = (__runInitializers(_init26, 27, this), __runInitializers(_init26, 28, this, null));
    [(_dec121 = [
      property
    ], "renderMode")] = (__runInitializers(_init26, 31, this), __runInitializers(_init26, 32, this, "tube"));
    links = (__runInitializers(_init26, 35, this), []);
    joints = [];
    line = null;
    onStart() {
      const start = this.transform.position;
      const toEnd = this.end ? this.end.transform.position.sub(start) : Vector3.down.scale(this.length);
      const dir = toEnd.magnitude > 0.000001 ? toEnd.normalized : Vector3.down;
      const n = Math.max(1, Math.round(this.segments));
      const seg = this.length / n;
      const rotation = Quaternion.fromToRotation(Vector3.down, dir);
      const material = new Material(this.color, Color.black, 0.2);
      const own = objectPhysics.get(this.gameObject);
      let prev = own?.rigidbody ? own.body : null;
      let prevAnchor = prev ? Vector3.zero : start;
      for (let i = 0;i < n; i++) {
        const center = start.add(dir.scale(seg * (i + 0.5)));
        const link = new GameObject(`${this.gameObject.name} rope ${i}`, {
          position: center,
          rotation,
          scale: new Vector3(this.radius * 2, seg / 2, this.radius * 2)
        });
        if (this.renderMode === "segments")
          link.addComponent(MeshRenderer, { mesh: "capsule", material });
        link.addComponent(CapsuleCollider);
        const rb = link.addComponent(Rigidbody, { mass: this.segmentMass, angularDrag: 0.5, drag: 0.1 });
        const body = rb.body;
        this.joints.push(physics.createJoint("ball", body, prev, new Vector3(0, seg / 2, 0), prevAnchor));
        prev = body;
        prevAnchor = new Vector3(0, -seg / 2, 0);
        this.links.push(link);
      }
      if (this.end?.body && prev) {
        this.joints.push(physics.createJoint("ball", this.end.body, prev, Vector3.zero, prevAnchor));
      }
      if (this.renderMode === "tube") {
        this.line = new GameObject(`${this.gameObject.name} rope line`);
        this.line.addComponent(LineRenderer, { width: this.radius * 2, material, positions: this.points() });
      }
    }
    points() {
      if (this.links.length === 0)
        return [];
      const out = [this.links[0].transform.transformPoint(Vector3.up)];
      for (const l of this.links)
        out.push(l.transform.transformPoint(Vector3.down));
      return out;
    }
    onPreRender() {
      const r = this.line?.getComponent(LineRenderer);
      if (r) {
        r.positions = this.points();
        r.redraw();
      }
    }
    onDestroy() {
      for (const j of this.joints)
        j.remove();
      for (const l of this.links)
        l.destroy();
      this.line?.destroy();
      this.line = null;
      this.joints = [];
      this.links.length = 0;
    }
  }

  // ../../js/engine/src/cloth.ts
  var _init27, _base27, _dec122, _dec123, _dec124, _dec125, _dec126, _dec127, _dec128, _dec129, _dec130, _dec131, _dec132, _dec133, _dec134;

  class Cloth extends (_base27 = Component) {
    static {
      _init27 = __decoratorStart(_base27);
      __decorateElement(_init27, 5, "stretchingStiffness", _dec122, this);
      __decorateElement(_init27, 5, "bendingStiffness", _dec123, this);
      __decorateElement(_init27, 5, "damping", _dec124, this);
      __decorateElement(_init27, 5, "useGravity", _dec125, this);
      __decorateElement(_init27, 5, "externalAcceleration", _dec126, this);
      __decorateElement(_init27, 5, "randomAcceleration", _dec127, this);
      __decorateElement(_init27, 5, "friction", _dec128, this);
      __decorateElement(_init27, 5, "solverIterations", _dec129, this);
      __decorateElement(_init27, 5, "pins", _dec130, this);
      __decorateElement(_init27, 5, "pinRadius", _dec131, this);
      __decorateElement(_init27, 5, "pinBox", _dec132, this);
      __decorateElement(_init27, 5, "sphereColliders", _dec133, this);
      __decorateElement(_init27, 5, "capsuleColliders", _dec134, this);
      __decoratorMetadata(_init27, this);
    }
    [(_dec122 = [
      property
    ], "stretchingStiffness")] = __runInitializers(_init27, 8, this, 1);
    [(_dec123 = [
      property
    ], "bendingStiffness")] = (__runInitializers(_init27, 11, this), __runInitializers(_init27, 12, this, 0.3));
    [(_dec124 = [
      property
    ], "damping")] = (__runInitializers(_init27, 15, this), __runInitializers(_init27, 16, this, 0.1));
    [(_dec125 = [
      property
    ], "useGravity")] = (__runInitializers(_init27, 19, this), __runInitializers(_init27, 20, this, true));
    [(_dec126 = [
      property
    ], "externalAcceleration")] = (__runInitializers(_init27, 23, this), __runInitializers(_init27, 24, this, Vector3.zero));
    [(_dec127 = [
      property
    ], "randomAcceleration")] = (__runInitializers(_init27, 27, this), __runInitializers(_init27, 28, this, Vector3.zero));
    [(_dec128 = [
      property
    ], "friction")] = (__runInitializers(_init27, 31, this), __runInitializers(_init27, 32, this, 0.5));
    [(_dec129 = [
      property
    ], "solverIterations")] = (__runInitializers(_init27, 35, this), __runInitializers(_init27, 36, this, 8));
    [(_dec130 = [
      property
    ], "pins")] = (__runInitializers(_init27, 39, this), __runInitializers(_init27, 40, this, []));
    [(_dec131 = [
      property
    ], "pinRadius")] = (__runInitializers(_init27, 43, this), __runInitializers(_init27, 44, this, 0.05));
    [(_dec132 = [
      property
    ], "pinBox")] = (__runInitializers(_init27, 47, this), __runInitializers(_init27, 48, this, null));
    [(_dec133 = [
      property
    ], "sphereColliders")] = (__runInitializers(_init27, 51, this), __runInitializers(_init27, 52, this, []));
    [(_dec134 = [
      property
    ], "capsuleColliders")] = (__runInitializers(_init27, 55, this), __runInitializers(_init27, 56, this, []));
    renderer = (__runInitializers(_init27, 59, this), undefined);
    original;
    simulated = null;
    onStart() {
      this.rebuild();
    }
    rebuild() {
      const renderer = this.gameObject.components.find((c) => c.constructor.name === "MeshRenderer" || c.constructor.name === "SkinnedMeshRenderer");
      if (!renderer) {
        console.warn(`${this.gameObject.name}: Cloth needs a MeshRenderer on the same object`);
        return;
      }
      if (!this.simulated) {
        this.renderer = renderer;
        this.original = renderer.mesh;
        this.simulated = Mesh._reserve();
      }
      const source = this.original;
      const v = (x) => [x.x, x.y, x.z];
      const colliders = [];
      for (const go of this.sphereColliders) {
        const c = go.getComponent(SphereCollider);
        if (c)
          colliders.push(["sphere", go.id, c.radius, v(c.center)]);
      }
      for (const go of this.capsuleColliders) {
        const c = go.getComponent(CapsuleCollider);
        if (c)
          colliders.push(["capsule", go.id, c.radius, c.height, v(c.center)]);
      }
      const box = this.pinBox;
      __iron_op("cloth.cloth", this.id, this.gameObject.id, typeof source === "string" ? source : source.id, this.simulated.id, [this.stretchingStiffness, this.bendingStiffness, this.damping, this.friction, this.solverIterations], v(this.externalAcceleration), v(this.randomAcceleration), this.useGravity, this.pins.flatMap(v), this.pinRadius, box ? [...v(box.min), ...v(box.max)] : null, colliders);
      renderer.mesh = this.simulated;
    }
    onDestroy() {
      __iron_op("cloth.remove", this.id);
      if (this.renderer && this.original !== undefined && !this.renderer.gameObject.destroyed)
        this.renderer.mesh = this.original;
      this.simulated?.destroy();
    }
  }
  // ../../js/engine/src/assets.ts
  var listeners3 = new Set;
  var native = () => globalThis.Assets ?? {};
  var entries = () => globalThis.__iron_info?.assets ?? [];
  var clean = (path) => path.replace(/\\/g, "/").replace(/^\.\//, "");
  var Assets = {
    get all() {
      return entries();
    },
    guidToPath(guid) {
      return entries().find((e) => e.guid === guid)?.path;
    },
    pathToGuid(path) {
      const p = clean(path);
      return entries().find((e) => e.path === p)?.guid;
    },
    importer(path) {
      const p = clean(Assets.resolve(path));
      return entries().find((e) => e.path === p)?.importer ?? {};
    },
    exists(path) {
      return native().exists?.(path) ?? false;
    },
    list(glob) {
      return native().list?.(glob) ?? [];
    },
    packOf(path) {
      return native().packOf?.(path) ?? "";
    },
    resolve(src) {
      if (!src.startsWith("guid:"))
        return src;
      return Assets.guidToPath(src.slice(5)) ?? src;
    },
    onChanged(fn) {
      listeners3.add(fn);
      return () => listeners3.delete(fn);
    }
  };
  globalThis.__iron_assets_changed = (paths) => {
    for (const f of [...listeners3]) {
      try {
        f(paths);
      } catch (e) {
        console.error(`Assets.onChanged: ${e}`);
      }
    }
  };
  // ../../js/engine/src/animation.ts
  class Keyframe {
    time;
    value;
    inTangent;
    outTangent;
    constructor(time, value, inTangent = 0, outTangent = 0) {
      this.time = time;
      this.value = value;
      this.inTangent = inTangent;
      this.outTangent = outTangent;
    }
  }
  function wrap(t, length, mode) {
    if (length <= 0)
      return 0;
    if (mode === "loop")
      return (t % length + length) % length;
    if (mode === "pingPong") {
      const p = (t % (2 * length) + 2 * length) % (2 * length);
      return p > length ? 2 * length - p : p;
    }
    return Math.min(length, Math.max(0, t));
  }

  class AnimationCurve {
    keys;
    preWrapMode = "clampForever";
    postWrapMode = "clampForever";
    constructor(...keys) {
      this.keys = [...keys].sort((a, b) => a.time - b.time);
    }
    static linear(t0, v0, t1, v1) {
      const slope = t1 === t0 ? 0 : (v1 - v0) / (t1 - t0);
      return new AnimationCurve(new Keyframe(t0, v0, 0, slope), new Keyframe(t1, v1, slope, 0));
    }
    static easeInOut(t0, v0, t1, v1) {
      return new AnimationCurve(new Keyframe(t0, v0), new Keyframe(t1, v1));
    }
    static constant(t0, t1, value) {
      return new AnimationCurve(new Keyframe(t0, value), new Keyframe(t1, value));
    }
    static fromPoints(points, mode = "linear") {
      const keys = points.map(([t, v]) => new Keyframe(t, v));
      for (let i = 0;i < keys.length; i++) {
        const prev = keys[i - 1];
        const next = keys[i + 1];
        const k = keys[i];
        const slope = (a, b) => b.time === a.time ? 0 : (b.value - a.value) / (b.time - a.time);
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
    addKey(time, value = 0) {
      const key = time instanceof Keyframe ? time : new Keyframe(time, value);
      let i = this.keys.findIndex((k) => k.time > key.time);
      if (i < 0)
        i = this.keys.length;
      this.keys.splice(i, 0, key);
      return i;
    }
    removeKey(index) {
      this.keys.splice(index, 1);
    }
    get length() {
      return this.keys.length;
    }
    evaluate(time) {
      const keys = this.keys;
      if (keys.length === 0)
        return 0;
      if (keys.length === 1)
        return keys[0].value;
      const first = keys[0].time;
      const last = keys[keys.length - 1].time;
      const span = last - first;
      let t = time;
      if (t < first)
        t = first + wrap(t - first, span, this.preWrapMode);
      else if (t > last)
        t = first + wrap(t - first, span, this.postWrapMode);
      let lo = 0;
      let hi = keys.length - 1;
      while (hi - lo > 1) {
        const mid = lo + hi >> 1;
        if (keys[mid].time <= t)
          lo = mid;
        else
          hi = mid;
      }
      const a = keys[lo];
      const b = keys[hi];
      const dt = b.time - a.time;
      if (dt <= 0)
        return b.value;
      if (!Number.isFinite(a.outTangent) || !Number.isFinite(b.inTangent))
        return t >= b.time ? b.value : a.value;
      const s = (t - a.time) / dt;
      const s2 = s * s;
      const s3 = s2 * s;
      return (2 * s3 - 3 * s2 + 1) * a.value + (s3 - 2 * s2 + s) * a.outTangent * dt + (-2 * s3 + 3 * s2) * b.value + (s3 - s2) * b.inTangent * dt;
    }
    _toJSON() {
      const n = (x) => Number.isFinite(x) ? x : "Infinity";
      return {
        keys: this.keys.map((k) => [k.time, k.value, n(k.inTangent), n(k.outTangent)]),
        pre: this.preWrapMode,
        post: this.postWrapMode
      };
    }
    static _fromJSON(data) {
      const d = data;
      const num = (x) => x === "Infinity" ? Infinity : Number(x);
      const curve = new AnimationCurve(...(d.keys ?? []).map((k) => new Keyframe(num(k[0]), num(k[1]), num(k[2]), num(k[3]))));
      curve.preWrapMode = d.pre ?? "clampForever";
      curve.postWrapMode = d.post ?? "clampForever";
      return curve;
    }
  }

  class AnimationClip {
    name;
    wrapMode = "once";
    frameRate = 60;
    events = [];
    _bindings = [];
    _tracks = null;
    _length = null;
    constructor(name = "clip") {
      this.name = name;
    }
    setCurve(relativePath, type, propertyName, curve) {
      const typeName = typeof type === "string" ? type : type.name;
      const i = this._bindings.findIndex((b) => b.path === relativePath && b.type === typeName && b.property === propertyName);
      if (i >= 0)
        this._bindings.splice(i, 1);
      if (curve)
        this._bindings.push({ path: relativePath, type: typeName, property: propertyName, curve });
      this._tracks = null;
      this._length = null;
    }
    get curves() {
      return this._bindings;
    }
    get length() {
      if (this._length === null) {
        let end = 0;
        for (const b of this._bindings)
          end = Math.max(end, b.curve.keys.at(-1)?.time ?? 0);
        for (const e of this.events)
          end = Math.max(end, e.time);
        this._length = end;
      }
      return this._length;
    }
    addEvent(event) {
      this.events.push({ ...event });
      this.events.sort((a, b) => a.time - b.time);
      this._length = null;
    }
    sampleAnimation(go, time) {
      const pose = new Pose;
      pose.add(go, this, wrap(time, this.length, this.wrapMode), 1);
      pose.apply();
    }
    toJSON() {
      return {
        name: this.name,
        wrapMode: this.wrapMode,
        frameRate: this.frameRate,
        curves: this._bindings.map((b) => ({ path: b.path, type: b.type, property: b.property, curve: b.curve._toJSON() })),
        events: this.events.map((e) => ({ ...e }))
      };
    }
    static fromJSON(data) {
      const d = data;
      const clip = new AnimationClip(d.name ?? "clip");
      clip.wrapMode = d.wrapMode ?? "once";
      clip.frameRate = d.frameRate ?? 60;
      for (const c of d.curves ?? [])
        clip.setCurve(c.path, c.type, c.property, AnimationCurve._fromJSON(c.curve));
      for (const e of d.events ?? [])
        clip.addEvent(e);
      return clip;
    }
  }
  function resolve(root, path) {
    if (path === "")
      return root;
    let go = root;
    for (const name of path.split("/")) {
      go = go?.findChild(name);
      if (!go)
        return;
    }
    return go;
  }
  var resolved = new WeakMap;
  function resolveCached(root, path) {
    let paths = resolved.get(root);
    if (!paths) {
      paths = new Map;
      resolved.set(root, paths);
    }
    const hit = paths.get(path);
    if (hit && !hit.destroyed)
      return hit;
    const go = resolve(root, path);
    paths.set(path, go ?? null);
    return go;
  }
  function componentNamed(go, name) {
    return go.components.find((c) => c.constructor.name === name);
  }
  var AXES = { x: 0, y: 1, z: 2, w: 3 };
  function tracksOf(clip) {
    if (clip._tracks)
      return clip._tracks;
    const byPath = new Map;
    for (const b of clip._bindings) {
      let t = byPath.get(b.path);
      if (!t) {
        t = { path: b.path, p: [], r: [], s: [], e: [], other: [] };
        byPath.set(b.path, t);
      }
      const [field, axis] = b.property.split(".");
      const i = axis === undefined ? -1 : AXES[axis] ?? -1;
      const list = b.type !== "Transform" ? null : field === "localPosition" ? t.p : field === "localRotation" ? t.r : field === "localScale" ? t.s : field === "localEulerAngles" ? t.e : null;
      if (list && i >= 0 && (list === t.r || i < 3))
        list[i] = b.curve;
      else if (b.type !== "Transform")
        t.other.push(b);
    }
    clip._tracks = [...byPath.values()];
    return clip._tracks;
  }

  class Pose {
    transforms = new Map;
    props = new Map;
    add(root, clip, time, weight) {
      this.addClip(root, clip, time, weight, false);
    }
    addAligned(root, clip, time, weight) {
      this.addClip(root, clip, time, weight, true);
    }
    addClip(root, clip, time, weight, align) {
      if (weight <= 0)
        return;
      for (const track of tracksOf(clip)) {
        const go = resolveCached(root, track.path);
        if (!go)
          continue;
        if (track.p.length || track.r.length || track.s.length || track.e.length) {
          let t = this.transforms.get(go);
          if (!t) {
            t = { p: [0, 0, 0], pw: [0, 0, 0], r: [0, 0, 0, 0], rw: 0, s: [0, 0, 0], sw: [0, 0, 0], e: [0, 0, 0], ew: [0, 0, 0] };
            this.transforms.set(go, t);
          }
          for (let i = 0;i < 3; i++) {
            const pc = track.p[i];
            if (pc) {
              t.p[i] += pc.evaluate(time) * weight;
              t.pw[i] += weight;
            }
            const sc = track.s[i];
            if (sc) {
              t.s[i] += sc.evaluate(time) * weight;
              t.sw[i] += weight;
            }
            const ec = track.e[i];
            if (ec) {
              t.e[i] += ec.evaluate(time) * weight;
              t.ew[i] += weight;
            }
          }
          if (track.r.length) {
            const q = [0, 1, 2, 3].map((i) => track.r[i]?.evaluate(time) ?? 0);
            const dot = q[0] * t.r[0] + q[1] * t.r[1] + q[2] * t.r[2] + q[3] * t.r[3];
            const sign = align && dot < 0 ? -1 : 1;
            for (let i = 0;i < 4; i++)
              t.r[i] += sign * q[i] * weight;
            t.rw += weight;
          }
        }
        for (const b of track.other) {
          const target = b.type === "GameObject" ? go : componentNamed(go, b.type);
          if (!target)
            continue;
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
    apply() {
      for (const [go, t] of this.transforms) {
        if (go.destroyed)
          continue;
        const tr = go.transform;
        const avg = (sum, w, current) => w[0] || w[1] || w[2] ? new Vector3(w[0] ? sum[0] / w[0] : current.x, w[1] ? sum[1] / w[1] : current.y, w[2] ? sum[2] / w[2] : current.z) : current;
        const position = avg(t.p, t.pw, tr.localPosition);
        const scale = avg(t.s, t.sw, tr.localScale);
        let rotation = tr.localRotation;
        if (t.rw > 0)
          rotation = new Quaternion(t.r[0], t.r[1], t.r[2], t.r[3]).normalized;
        else if (t.ew[0] || t.ew[1] || t.ew[2]) {
          const e = avg(t.e, t.ew, tr.localEulerAngles);
          rotation = Quaternion.euler(e.x, e.y, e.z);
        }
        tr._setLocal(position, rotation, scale);
      }
      for (const p of this.props.values()) {
        const value = p.value / p.weight;
        if (p.target instanceof GameObject) {
          if (p.property === "active")
            p.target.enabled = value > 0.5;
          continue;
        }
        setProperty(p.target, p.property, value);
      }
    }
  }
  function setProperty(target, property, value) {
    const record = target;
    const dot = property.indexOf(".");
    if (dot < 0) {
      const current = record[property];
      record[property] = typeof current === "boolean" ? value > 0.5 : value;
      return;
    }
    const head = property.slice(0, dot);
    const field = property.slice(dot + 1);
    const current = record[head];
    if (current === null || typeof current !== "object")
      return;
    const copy = Object.assign(Object.create(Object.getPrototypeOf(current)), current);
    copy[field] = value;
    record[head] = copy;
  }
  function fireEvents(go, clip, from, to, length, wrapMode) {
    if (clip.events.length === 0 || to <= from)
      return;
    const passes = wrapMode === "loop" && length > 0 ? Math.floor(to / length) - Math.floor(from / length) : 0;
    const local = (t) => wrapMode === "loop" && length > 0 ? (t % length + length) % length : t;
    const a = local(from);
    const b = local(to);
    for (const e of clip.events) {
      const hit = passes === 0 ? e.time > a && e.time <= b : e.time > a || e.time <= b || passes > 1;
      if (!hit)
        continue;
      for (const c of [...go.components]) {
        const fn = c[e.functionName];
        if (typeof fn === "function")
          fn.call(c, e.parameter);
      }
    }
  }

  class AnimationState {
    name;
    clip;
    time = 0;
    speed = 1;
    weight = 0;
    enabled = false;
    wrapMode;
    _fade = 0;
    _target = 0;
    constructor(name, clip) {
      this.name = name;
      this.clip = clip;
      this.wrapMode = clip.wrapMode;
    }
    get length() {
      return this.clip.length;
    }
    get normalizedTime() {
      return this.length > 0 ? this.time / this.length : 0;
    }
    set normalizedTime(t) {
      this.time = t * this.length;
    }
  }
  var _init28, _base28, _dec135, _dec136, _dec137;

  class Animation extends (_base28 = Component) {
    static {
      _init28 = __decoratorStart(_base28);
      __decorateElement(_init28, 5, "clip", _dec135, this);
      __decorateElement(_init28, 5, "clips", _dec136, this);
      __decorateElement(_init28, 5, "playAutomatically", _dec137, this);
      __decoratorMetadata(_init28, this);
    }
    [(_dec135 = [
      property
    ], "clip")] = __runInitializers(_init28, 8, this, null);
    [(_dec136 = [
      property
    ], "clips")] = (__runInitializers(_init28, 11, this), __runInitializers(_init28, 12, this, []));
    [(_dec137 = [
      property
    ], "playAutomatically")] = (__runInitializers(_init28, 15, this), __runInitializers(_init28, 16, this, true));
    states = (__runInitializers(_init28, 19, this), new Map);
    onStart() {
      if (this.playAutomatically && this.clip)
        this.play(this.clip.name);
    }
    addClip(clip, name = clip.name) {
      if (!this.clips.includes(clip))
        this.clips.push(clip);
      this.states.set(name, new AnimationState(name, clip));
    }
    state(name) {
      if (!this.states.has(name)) {
        const clip = [...this.clips, ...this.clip ? [this.clip] : []].find((c) => c.name === name);
        if (clip)
          this.states.set(name, new AnimationState(name, clip));
      }
      return this.states.get(name);
    }
    play(name = this.clip?.name ?? "") {
      const s = this.state(name);
      if (!s)
        return false;
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
    crossFade(name, fadeLength = 0.3) {
      const s = this.state(name);
      if (!s)
        return;
      if (!s.enabled) {
        s.enabled = true;
        s.time = 0;
      }
      const rate = 1 / Math.max(0.0001, fadeLength);
      s._target = 1;
      s._fade = rate;
      for (const other of this.states.values()) {
        if (other !== s && other.enabled) {
          other._target = 0;
          other._fade = rate;
        }
      }
    }
    stop(name) {
      for (const s of this.states.values()) {
        if (name === undefined || s.name === name) {
          s.enabled = false;
          s.weight = 0;
          s.time = 0;
        }
      }
    }
    isPlaying(name) {
      return [...this.states.values()].some((s) => s.enabled && (name === undefined || s.name === name));
    }
    _animate(dt) {
      const pose = new Pose;
      for (const s of this.states.values()) {
        if (!s.enabled)
          continue;
        if (s._fade) {
          const step = s._fade * dt;
          s.weight = s.weight < s._target ? Math.min(s._target, s.weight + step) : Math.max(s._target, s.weight - step);
          if (s.weight === s._target)
            s._fade = 0;
          if (s.weight <= 0 && s._target === 0) {
            s.enabled = false;
            continue;
          }
        }
        const before = s.time;
        s.time += dt * s.speed;
        fireEvents(this.gameObject, s.clip, before, s.time, s.length, s.wrapMode);
        if (s.wrapMode === "once" && s.time >= s.length) {
          pose.addAligned(this.gameObject, s.clip, s.length, s.weight);
          s.enabled = false;
          continue;
        }
        pose.addAligned(this.gameObject, s.clip, wrap(s.time, s.length, s.wrapMode), s.weight);
      }
      pose.apply();
    }
  }

  class AnimatorController {
    parameters = [];
    states = [];
    transitions = [];
    defaultState = null;
    constructor(init = {}) {
      Object.assign(this, init);
    }
    addParameter(name, type, defaultValue) {
      this.parameters.push({ name, type, defaultValue });
      return this;
    }
    addState(name, motion, options = {}) {
      this.states.push({ name, motion, ...options });
      return this;
    }
    addTransition(t) {
      this.transitions.push(t);
      return this;
    }
    toJSON() {
      const motion = (m) => m instanceof AnimationClip ? { clip: m.toJSON() } : { blendParameter: m.blendParameter, children: m.children.map((c) => ({ ...c, motion: c.motion.toJSON() })) };
      return {
        parameters: this.parameters,
        states: this.states.map((s) => ({ ...s, motion: motion(s.motion) })),
        transitions: this.transitions,
        defaultState: this.defaultState
      };
    }
    static fromJSON(data) {
      const d = data;
      return new AnimatorController({
        parameters: d.parameters ?? [],
        transitions: d.transitions ?? [],
        defaultState: d.defaultState ?? null,
        states: (d.states ?? []).map((s) => ({
          ...s,
          motion: s.motion.clip ? AnimationClip.fromJSON(s.motion.clip) : {
            blendParameter: s.motion.blendParameter ?? "",
            children: (s.motion.children ?? []).map((c) => ({ ...c, motion: AnimationClip.fromJSON(c.motion) }))
          }
        }))
      });
    }
  }
  var _init29, _base29, _dec138, _dec139;

  class Animator extends (_base29 = Component) {
    static {
      _init29 = __decoratorStart(_base29);
      __decorateElement(_init29, 5, "controller", _dec138, this);
      __decorateElement(_init29, 5, "speed", _dec139, this);
      __decoratorMetadata(_init29, this);
    }
    [(_dec138 = [
      property
    ], "controller")] = __runInitializers(_init29, 8, this, null);
    [(_dec139 = [
      property
    ], "speed")] = (__runInitializers(_init29, 11, this), __runInitializers(_init29, 12, this, 1));
    values = (__runInitializers(_init29, 15, this), new Map);
    current = null;
    next = null;
    ensure() {
      if (this.current || !this.controller)
        return;
      for (const p of this.controller.parameters) {
        if (!this.values.has(p.name))
          this.values.set(p.name, p.defaultValue ?? (p.type === "bool" || p.type === "trigger" ? false : 0));
      }
      const name = this.controller.defaultState ?? this.controller.states[0]?.name;
      const state = this.controller.states.find((s) => s.name === name);
      if (state)
        this.current = { state, norm: 0 };
    }
    setFloat(name, value) {
      this.values.set(name, value);
    }
    getFloat(name) {
      this.ensure();
      return Number(this.values.get(name) ?? 0);
    }
    setInteger(name, value) {
      this.values.set(name, Math.trunc(value));
    }
    getInteger(name) {
      return Math.trunc(this.getFloat(name));
    }
    setBool(name, value) {
      this.values.set(name, value);
    }
    getBool(name) {
      this.ensure();
      return this.values.get(name) === true;
    }
    setTrigger(name) {
      this.values.set(name, true);
    }
    resetTrigger(name) {
      this.values.set(name, false);
    }
    play(stateName, normalizedTime = 0) {
      this.ensure();
      const state = this.controller?.states.find((s) => s.name === stateName);
      if (!state)
        return;
      this.current = { state, norm: normalizedTime };
      this.next = null;
    }
    crossFade(stateName, duration = 0.25, normalizedTime = 0) {
      this.ensure();
      const state = this.controller?.states.find((s) => s.name === stateName);
      if (!state || !this.current) {
        if (state)
          this.play(stateName, normalizedTime);
        return;
      }
      this.next = { state, norm: normalizedTime, progress: 0, duration: Math.max(0.0001, duration) };
    }
    get isInTransition() {
      return this.next !== null;
    }
    getCurrentAnimatorStateInfo() {
      this.ensure();
      return this.info(this.current);
    }
    getNextAnimatorStateInfo() {
      return this.info(this.next);
    }
    info(p) {
      const length = p ? this.lengthOf(p.state) : 0;
      const name = p?.state.name ?? "";
      return { name, length, normalizedTime: p && length > 0 ? p.norm : 0, isName: (n) => n === name };
    }
    clips(state) {
      const m = state.motion;
      if (m instanceof AnimationClip)
        return [{ clip: m, weight: 1, timeScale: 1 }];
      const children = [...m.children].sort((a, b) => a.threshold - b.threshold);
      if (children.length === 0)
        return [];
      const v = this.getFloat(m.blendParameter);
      const first = children[0];
      const last = children[children.length - 1];
      if (v <= first.threshold)
        return [{ clip: first.motion, weight: 1, timeScale: first.timeScale ?? 1 }];
      if (v >= last.threshold)
        return [{ clip: last.motion, weight: 1, timeScale: last.timeScale ?? 1 }];
      const i = children.findIndex((c) => c.threshold > v);
      const a = children[i - 1];
      const b = children[i];
      const t = (v - a.threshold) / (b.threshold - a.threshold);
      return [
        { clip: a.motion, weight: 1 - t, timeScale: a.timeScale ?? 1 },
        { clip: b.motion, weight: t, timeScale: b.timeScale ?? 1 }
      ];
    }
    lengthOf(state) {
      return this.clips(state).reduce((l, c) => l + c.weight * c.clip.length / c.timeScale, 0);
    }
    stateSpeed(state) {
      const factor = state.speedParameter ? this.getFloat(state.speedParameter) : 1;
      return (state.speed ?? 1) * factor * this.speed;
    }
    passes(c) {
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
    checkTransitions() {
      if (!this.controller || !this.current || this.next)
        return;
      const from = this.current;
      const normalized = this.lengthOf(from.state) > 0 ? from.norm : 1;
      for (const t of this.controller.transitions) {
        if (t.from !== from.state.name && !(t.from === "Any" && t.to !== from.state.name))
          continue;
        if (t.hasExitTime && normalized < (t.exitTime ?? 1))
          continue;
        const conditions = t.conditions ?? [];
        if (!t.hasExitTime && conditions.length === 0)
          continue;
        if (!conditions.every((c) => this.passes(c)))
          continue;
        for (const c of conditions) {
          if (this.controller.parameters.find((p) => p.name === c.parameter)?.type === "trigger")
            this.values.set(c.parameter, false);
        }
        const state = this.controller.states.find((s) => s.name === t.to);
        if (!state)
          continue;
        const duration = t.duration ?? 0.25;
        const start = t.offset ?? 0;
        if (duration <= 0)
          this.current = { state, norm: start };
        else
          this.next = { state, norm: start, progress: 0, duration };
        return;
      }
    }
    _animate(dt) {
      this.ensure();
      if (!this.current)
        return;
      this.checkTransitions();
      const pose = new Pose;
      const run = (p, weight) => {
        const before = p.norm;
        const length = this.lengthOf(p.state);
        if (length > 0)
          p.norm += dt * this.stateSpeed(p.state) / length;
        for (const c of this.clips(p.state)) {
          const t = length > 0 ? p.norm * c.clip.length : 0;
          const t0 = length > 0 ? before * c.clip.length : 0;
          const mode = c.clip.wrapMode === "once" ? "clampForever" : c.clip.wrapMode;
          if (c.weight * weight >= 0.5)
            fireEvents(this.gameObject, c.clip, t0, t, c.clip.length, mode);
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
  globalThis.__iron_animate = (dt) => {
    const visit = (go) => {
      if (!go.activeInHierarchy)
        return;
      for (const c of go.components) {
        if (c instanceof Animation || c instanceof Animator) {
          try {
            c._animate(dt);
          } catch (e) {
            console.error(`animation on ${go.name}: ${e}`);
          }
        }
      }
      for (const child of go.children)
        visit(child);
    };
    for (const root of GameObject.roots)
      visit(root);
  };
  // ../../js/engine/src/audio.ts
  var nextSound = 1;
  var nextPlayback = 1;
  var nextTrack = 1;
  var playing = new Map;

  class Sound {
    id;
    src;
    constructor(source, options = {}) {
      const src = Assets.resolve(source);
      this.id = nextSound++;
      this.src = src;
      if (options.stream === undefined && Assets.importer(src).stream === true)
        options = { ...options, stream: true };
      __iron_op("audio.load", this.id, src, !!options.stream);
    }
    play(options = {}) {
      return new Playback(this, options);
    }
    unload() {
      __iron_op("audio.unload", this.id);
    }
  }

  class Playback {
    id;
    sound;
    playing = true;
    onEnded;
    constructor(sound, o) {
      this.id = nextPlayback++;
      this.sound = sound;
      playing.set(this.id, this);
      __iron_op("audio.play", this.id, sound.id, o.on?.id ?? null, [
        o.volume ?? 1,
        o.rate ?? 1,
        o.pan ?? 0,
        !!o.loop,
        o.fadeIn ?? 0,
        o.start ?? 0
      ]);
    }
    setVolume(volume, fade = 0) {
      __iron_op("audio.set", this.id, "volume", volume, fade);
    }
    setRate(rate, fade = 0) {
      __iron_op("audio.set", this.id, "rate", rate, fade);
    }
    setPan(pan, fade = 0) {
      __iron_op("audio.set", this.id, "pan", pan, fade);
    }
    pause(fade = 0) {
      __iron_op("audio.pause", this.id, fade);
    }
    resume(fade = 0) {
      __iron_op("audio.resume", this.id, fade);
    }
    stop(fade = 0) {
      __iron_op("audio.stop", this.id, fade);
    }
    seek(seconds) {
      __iron_op("audio.seek", this.id, seconds);
    }
  }

  class Bus {
    id;
    constructor(volume, parent) {
      this.id = nextTrack++;
      __iron_op("audio.bus", this.id, parent?.id ?? null, volume);
    }
    setVolume(volume, fade = 0) {
      __iron_op("audio.trackVolume", this.id, volume, fade);
    }
    pause(fade = 0) {
      __iron_op("audio.trackPause", this.id, fade);
    }
    resume(fade = 0) {
      __iron_op("audio.trackResume", this.id, fade);
    }
    remove() {
      __iron_op("audio.removeTrack", this.id);
    }
  }

  class Emitter {
    id;
    constructor(position, options) {
      this.id = nextTrack++;
      __iron_op("audio.emitter", this.id, [position.x, position.y, position.z], [options.minDistance ?? 1, options.maxDistance ?? 50], options.bus?.id ?? null);
    }
    setPosition(p, glide = 0) {
      __iron_op("audio.emitterPosition", this.id, [p.x, p.y, p.z], glide);
    }
    setVolume(volume, fade = 0) {
      __iron_op("audio.trackVolume", this.id, volume, fade);
    }
    remove() {
      __iron_op("audio.removeTrack", this.id);
    }
  }
  var cache = new Map;
  var audio = {
    load(src, options) {
      return new Sound(src, options);
    },
    play(src, options) {
      let sound = cache.get(src);
      if (!sound) {
        sound = new Sound(src);
        cache.set(src, sound);
      }
      return sound.play(options);
    },
    createBus(options = {}) {
      return new Bus(options.volume ?? 1, options.parent ?? null);
    },
    createEmitter(position, options = {}) {
      return new Emitter(position, options);
    },
    setListener(position, rotation = { x: 0, y: 0, z: 0, w: 1 }) {
      __iron_op("audio.listener", [position.x, position.y, position.z], [rotation.x, rotation.y, rotation.z, rotation.w]);
    },
    setMasterVolume(volume, fade = 0) {
      __iron_op("audio.trackVolume", null, volume, fade);
    }
  };
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
  // ../../js/engine/src/audio-components.ts
  var clips = new Map;
  function clipOf(clip) {
    if (clip instanceof Sound)
      return clip;
    let sound = clips.get(clip);
    if (!sound) {
      sound = audio.load(clip);
      clips.set(clip, sound);
    }
    return sound;
  }
  var _init30, _base30, _dec140, _dec141, _dec142, _dec143, _dec144, _dec145, _dec146, _dec147, _dec148, _dec149;

  class AudioSource extends (_base30 = Component) {
    static {
      _init30 = __decoratorStart(_base30);
      __decorateElement(_init30, 2, "volume", _dec145, this);
      __decorateElement(_init30, 2, "pitch", _dec146, this);
      __decorateElement(_init30, 2, "panStereo", _dec147, this);
      __decorateElement(_init30, 2, "mute", _dec148, this);
      __decorateElement(_init30, 2, "loop", _dec149, this);
      __decorateElement(_init30, 5, "clip", _dec140, this);
      __decorateElement(_init30, 5, "playOnAwake", _dec141, this);
      __decorateElement(_init30, 5, "spatialBlend", _dec142, this);
      __decorateElement(_init30, 5, "minDistance", _dec143, this);
      __decorateElement(_init30, 5, "maxDistance", _dec144, this);
      __decoratorMetadata(_init30, this);
    }
    [(_dec140 = [
      property
    ], "clip")] = (__runInitializers(_init30, 5, this), __runInitializers(_init30, 8, this, null));
    [(_dec141 = [
      property
    ], "playOnAwake")] = (__runInitializers(_init30, 11, this), __runInitializers(_init30, 12, this, true));
    [(_dec142 = [
      property
    ], "spatialBlend")] = (__runInitializers(_init30, 15, this), __runInitializers(_init30, 16, this, 0));
    [(_dec143 = [
      property
    ], "minDistance")] = (__runInitializers(_init30, 19, this), __runInitializers(_init30, 20, this, 1));
    [(_dec144 = [
      property
    ], "maxDistance")] = (__runInitializers(_init30, 23, this), __runInitializers(_init30, 24, this, 50));
    outputBus = (__runInitializers(_init30, 27, this), null);
    _volume = 1;
    _pitch = 1;
    _panStereo = 0;
    _loop = false;
    _mute = false;
    main = null;
    paused = false;
    all = new Set;
    emitter = null;
    emitterKey = "";
    sentPosition = null;
    get [(_dec145 = [
      property
    ], "volume")]() {
      return this._volume;
    }
    set volume(v) {
      this._volume = v;
      this.eachPlaying((p) => p.setVolume(this.gain()));
    }
    get [(_dec146 = [
      property
    ], "pitch")]() {
      return this._pitch;
    }
    set pitch(r) {
      this._pitch = r;
      this.eachPlaying((p) => p.setRate(r));
    }
    get [(_dec147 = [
      property
    ], "panStereo")]() {
      return this._panStereo;
    }
    set panStereo(p) {
      this._panStereo = p;
      this.eachPlaying((pb) => pb.setPan(p));
    }
    get [(_dec148 = [
      property
    ], "mute")]() {
      return this._mute;
    }
    set mute(m) {
      this._mute = m;
      this.eachPlaying((p) => p.setVolume(this.gain()));
    }
    get [(_dec149 = [
      property
    ], "loop")]() {
      return this._loop;
    }
    set loop(l) {
      this._loop = l;
    }
    get isPlaying() {
      return !!this.main?.playing && !this.paused;
    }
    gain() {
      return this._mute ? 0 : this._volume;
    }
    eachPlaying(f) {
      for (const p of this.all)
        f(p);
    }
    output() {
      if (this.spatialBlend <= 0)
        return this.outputBus ?? undefined;
      const key = `${this.minDistance},${this.maxDistance},${this.outputBus?.id ?? ""}`;
      if (!this.emitter || key !== this.emitterKey) {
        this.emitter?.remove();
        this.all.clear();
        this.main = null;
        this.sentPosition = this.transform.position;
        this.emitter = audio.createEmitter(this.sentPosition, {
          minDistance: this.minDistance,
          maxDistance: this.maxDistance,
          bus: this.outputBus ?? undefined
        });
        this.emitterKey = key;
      }
      return this.emitter;
    }
    start(sound, options) {
      const p = sound.play({ ...options, on: this.output() });
      this.all.add(p);
      p.onEnded = () => this.all.delete(p);
      return p;
    }
    play() {
      if (!this.clip) {
        console.warn(`AudioSource on ${this.gameObject.name} has no clip`);
        return;
      }
      this.main?.stop();
      this.paused = false;
      this.main = this.start(clipOf(this.clip), {
        volume: this.gain(),
        rate: this._pitch,
        pan: this._panStereo,
        loop: this._loop
      });
    }
    stop() {
      this.main?.stop();
      this.main = null;
      this.paused = false;
    }
    pause() {
      if (!this.main?.playing || this.paused)
        return;
      this.main.pause();
      this.paused = true;
    }
    unPause() {
      if (!this.paused || !this.main)
        return;
      this.main.resume();
      this.paused = false;
    }
    playOneShot(clip, volumeScale = 1) {
      this.start(clipOf(clip), { volume: this.gain() * volumeScale, rate: this._pitch, pan: this._panStereo });
    }
    static playClipAtPoint(clip, position, volume = 1) {
      const emitter = audio.createEmitter(position);
      const p = clipOf(clip).play({ volume, on: emitter });
      p.onEnded = () => emitter.remove();
      return p;
    }
    onAwake() {
      if (this.playOnAwake && this.clip)
        this.play();
    }
    onPreRender() {
      if (!this.emitter)
        return;
      const p = this.transform.position;
      const s = this.sentPosition;
      if (s && p.x === s.x && p.y === s.y && p.z === s.z)
        return;
      this.emitter.setPosition(p);
      this.sentPosition = p;
    }
    onDestroy() {
      for (const p of this.all)
        p.stop();
      this.all.clear();
      this.main = null;
      this.emitter?.remove();
      this.emitter = null;
    }
  }
  var masterVolume = 1;

  class AudioListener extends Component {
    sent = null;
    static get volume() {
      return masterVolume;
    }
    static set volume(v) {
      masterVolume = v;
      audio.setMasterVolume(v);
    }
    onAwake() {
      this.send();
    }
    onPreRender() {
      this.send();
    }
    send() {
      const p = this.transform.position;
      const r = this.transform.rotation;
      const s = this.sent;
      if (s && s.x === p.x && s.y === p.y && s.z === p.z && s.rx === r.x && s.ry === r.y && s.rz === r.z && s.rw === r.w) {
        return;
      }
      audio.setListener(p, r);
      this.sent = { x: p.x, y: p.y, z: p.z, rx: r.x, ry: r.y, rz: r.z, rw: r.w };
    }
  }
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
  var _init31, _base31, _dec150;

  class Collider2D extends (_base31 = Collider) {
    constructor() {
      super(...arguments);
      __runInitializers(_init31, 5, this);
    }
    static {
      _init31 = __decoratorStart(_base31);
      __decorateElement(_init31, 2, "offset", _dec150, this);
      __decoratorMetadata(_init31, this);
    }
    get [(_dec150 = [
      property
    ], "offset")]() {
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
  var _init32, _base32, _dec151;

  class BoxCollider2D extends (_base32 = Collider2D) {
    static {
      _init32 = __decoratorStart(_base32);
      __decorateElement(_init32, 2, "size", _dec151, this);
      __decoratorMetadata(_init32, this);
    }
    _size = (__runInitializers(_init32, 5, this), Vector2.one);
    get [(_dec151 = [
      property
    ], "size")]() {
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
  var _init33, _base33, _dec152;

  class CircleCollider2D extends (_base33 = Collider2D) {
    static {
      _init33 = __decoratorStart(_base33);
      __decorateElement(_init33, 2, "radius", _dec152, this);
      __decoratorMetadata(_init33, this);
    }
    _radius = (__runInitializers(_init33, 5, this), 0.5);
    get [(_dec152 = [
      property
    ], "radius")]() {
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
  var _init34, _base34, _dec153, _dec154;

  class CapsuleCollider2D extends (_base34 = Collider2D) {
    static {
      _init34 = __decoratorStart(_base34);
      __decorateElement(_init34, 2, "size", _dec153, this);
      __decorateElement(_init34, 2, "direction", _dec154, this);
      __decoratorMetadata(_init34, this);
    }
    _size = (__runInitializers(_init34, 5, this), new Vector2(1, 2));
    _direction = "vertical";
    get [(_dec153 = [
      property
    ], "size")]() {
      return this._size;
    }
    set size(s) {
      this._size = new Vector2(s.x, s.y);
      this.rebuild();
    }
    get [(_dec154 = [
      property
    ], "direction")]() {
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
  var _init35, _base35, _dec155;

  class PolygonCollider2D extends (_base35 = Collider2D) {
    static {
      _init35 = __decoratorStart(_base35);
      __decorateElement(_init35, 2, "points", _dec155, this);
      __decoratorMetadata(_init35, this);
    }
    _points = (__runInitializers(_init35, 5, this), [new Vector2(-0.5, -0.5), new Vector2(0.5, -0.5), new Vector2(0, 0.5)]);
    get [(_dec155 = [
      property
    ], "points")]() {
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
  var _init36, _base36, _dec156;

  class EdgeCollider2D extends (_base36 = Collider2D) {
    static {
      _init36 = __decoratorStart(_base36);
      __decorateElement(_init36, 2, "points", _dec156, this);
      __decoratorMetadata(_init36, this);
    }
    _points = (__runInitializers(_init36, 5, this), [new Vector2(-0.5, 0), new Vector2(0.5, 0)]);
    get [(_dec156 = [
      property
    ], "points")]() {
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
  var _init37, _base37, _dec157, _dec158, _dec159, _dec160, _dec161, _dec162, _dec163;

  class Rigidbody2D extends (_base37 = Component) {
    static {
      _init37 = __decoratorStart(_base37);
      __decorateElement(_init37, 2, "mass", _dec157, this);
      __decorateElement(_init37, 2, "drag", _dec158, this);
      __decorateElement(_init37, 2, "angularDrag", _dec159, this);
      __decorateElement(_init37, 2, "gravityScale", _dec160, this);
      __decorateElement(_init37, 2, "bodyType", _dec161, this);
      __decorateElement(_init37, 2, "freezeRotation", _dec162, this);
      __decorateElement(_init37, 2, "collisionDetectionMode", _dec163, this);
      __decoratorMetadata(_init37, this);
    }
    _mass = (__runInitializers(_init37, 5, this), 1);
    _drag = 0;
    _angularDrag = 0.05;
    _gravityScale = 1;
    _bodyType = "dynamic";
    _freezeRotation = false;
    _continuous = false;
    p = null;
    get [(_dec157 = [
      property
    ], "mass")]() {
      return this._mass;
    }
    set mass(kg) {
      this._mass = Math.max(0.000001, kg);
      this._sendMass();
    }
    get [(_dec158 = [
      property
    ], "drag")]() {
      return this._drag;
    }
    set drag(d) {
      this._drag = d;
      this.p?.body.setDamping(this._drag, this._angularDrag);
    }
    get [(_dec159 = [
      property
    ], "angularDrag")]() {
      return this._angularDrag;
    }
    set angularDrag(d) {
      this._angularDrag = d;
      this.p?.body.setDamping(this._drag, this._angularDrag);
    }
    get [(_dec160 = [
      property
    ], "gravityScale")]() {
      return this._gravityScale;
    }
    set gravityScale(s) {
      this._gravityScale = s;
      this.p?.body.setGravityScale(s);
    }
    get [(_dec161 = [
      property
    ], "bodyType")]() {
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
    get [(_dec162 = [
      property
    ], "freezeRotation")]() {
      return this._freezeRotation;
    }
    set freezeRotation(on) {
      this._freezeRotation = on;
      this.sendAxes();
    }
    get [(_dec163 = [
      property
    ], "collisionDetectionMode")]() {
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
  var _init38, _base38, _dec164, _dec165, _dec166, _dec167, _dec168, _dec169, _dec170, _dec171, _dec172, _dec173, _dec174, _dec175, _dec176, _dec177, _dec178, _dec179, _dec180, _dec181, _dec182, _dec183, _dec184, _dec185, _dec186, _dec187;

  class ParticleSystem extends (_base38 = Component) {
    static {
      _init38 = __decoratorStart(_base38);
      __decorateElement(_init38, 5, "duration", _dec164, this);
      __decorateElement(_init38, 5, "loop", _dec165, this);
      __decorateElement(_init38, 5, "playOnAwake", _dec166, this);
      __decorateElement(_init38, 5, "startDelay", _dec167, this);
      __decorateElement(_init38, 5, "startLifetime", _dec168, this);
      __decorateElement(_init38, 5, "startSpeed", _dec169, this);
      __decorateElement(_init38, 5, "startSize", _dec170, this);
      __decorateElement(_init38, 5, "startRotation", _dec171, this);
      __decorateElement(_init38, 5, "angularVelocity", _dec172, this);
      __decorateElement(_init38, 5, "startColor", _dec173, this);
      __decorateElement(_init38, 5, "gravityModifier", _dec174, this);
      __decorateElement(_init38, 5, "drag", _dec175, this);
      __decorateElement(_init38, 5, "simulationSpace", _dec176, this);
      __decorateElement(_init38, 5, "maxParticles", _dec177, this);
      __decorateElement(_init38, 5, "randomSeed", _dec178, this);
      __decorateElement(_init38, 5, "rateOverTime", _dec179, this);
      __decorateElement(_init38, 5, "bursts", _dec180, this);
      __decorateElement(_init38, 5, "shape", _dec181, this);
      __decorateElement(_init38, 5, "sizeOverLifetime", _dec182, this);
      __decorateElement(_init38, 5, "colorOverLifetime", _dec183, this);
      __decorateElement(_init38, 5, "texture", _dec184, this);
      __decorateElement(_init38, 5, "blendMode", _dec185, this);
      __decorateElement(_init38, 5, "textureSheet", _dec186, this);
      __decorateElement(_init38, 5, "stopAction", _dec187, this);
      __decoratorMetadata(_init38, this);
    }
    [(_dec164 = [
      property
    ], "duration")] = __runInitializers(_init38, 8, this, 5);
    [(_dec165 = [
      property
    ], "loop")] = (__runInitializers(_init38, 11, this), __runInitializers(_init38, 12, this, true));
    [(_dec166 = [
      property
    ], "playOnAwake")] = (__runInitializers(_init38, 15, this), __runInitializers(_init38, 16, this, true));
    [(_dec167 = [
      property
    ], "startDelay")] = (__runInitializers(_init38, 19, this), __runInitializers(_init38, 20, this, 0));
    [(_dec168 = [
      property
    ], "startLifetime")] = (__runInitializers(_init38, 23, this), __runInitializers(_init38, 24, this, 5));
    [(_dec169 = [
      property
    ], "startSpeed")] = (__runInitializers(_init38, 27, this), __runInitializers(_init38, 28, this, 5));
    [(_dec170 = [
      property
    ], "startSize")] = (__runInitializers(_init38, 31, this), __runInitializers(_init38, 32, this, 1));
    [(_dec171 = [
      property
    ], "startRotation")] = (__runInitializers(_init38, 35, this), __runInitializers(_init38, 36, this, 0));
    [(_dec172 = [
      property
    ], "angularVelocity")] = (__runInitializers(_init38, 39, this), __runInitializers(_init38, 40, this, 0));
    [(_dec173 = [
      property
    ], "startColor")] = (__runInitializers(_init38, 43, this), __runInitializers(_init38, 44, this, null));
    [(_dec174 = [
      property
    ], "gravityModifier")] = (__runInitializers(_init38, 47, this), __runInitializers(_init38, 48, this, 0));
    [(_dec175 = [
      property
    ], "drag")] = (__runInitializers(_init38, 51, this), __runInitializers(_init38, 52, this, 0));
    [(_dec176 = [
      property
    ], "simulationSpace")] = (__runInitializers(_init38, 55, this), __runInitializers(_init38, 56, this, "local"));
    [(_dec177 = [
      property
    ], "maxParticles")] = (__runInitializers(_init38, 59, this), __runInitializers(_init38, 60, this, 1000));
    [(_dec178 = [
      property
    ], "randomSeed")] = (__runInitializers(_init38, 63, this), __runInitializers(_init38, 64, this, 1));
    [(_dec179 = [
      property
    ], "rateOverTime")] = (__runInitializers(_init38, 67, this), __runInitializers(_init38, 68, this, 10));
    [(_dec180 = [
      property
    ], "bursts")] = (__runInitializers(_init38, 71, this), __runInitializers(_init38, 72, this, []));
    [(_dec181 = [
      property
    ], "shape")] = (__runInitializers(_init38, 75, this), __runInitializers(_init38, 76, this, { type: "cone", angle: 25, radius: 1 }));
    [(_dec182 = [
      property
    ], "sizeOverLifetime")] = (__runInitializers(_init38, 79, this), __runInitializers(_init38, 80, this, null));
    [(_dec183 = [
      property
    ], "colorOverLifetime")] = (__runInitializers(_init38, 83, this), __runInitializers(_init38, 84, this, null));
    [(_dec184 = [
      property
    ], "texture")] = (__runInitializers(_init38, 87, this), __runInitializers(_init38, 88, this, null));
    [(_dec185 = [
      property
    ], "blendMode")] = (__runInitializers(_init38, 91, this), __runInitializers(_init38, 92, this, "alpha"));
    [(_dec186 = [
      property
    ], "textureSheet")] = (__runInitializers(_init38, 95, this), __runInitializers(_init38, 96, this, null));
    [(_dec187 = [
      property
    ], "stopAction")] = (__runInitializers(_init38, 99, this), __runInitializers(_init38, 100, this, "none"));
    onStopped = (__runInitializers(_init38, 103, this), undefined);
    _particleCount = 0;
    alive = false;
    paused = false;
    sent = "";
    get particleCount() {
      return this._particleCount;
    }
    get isPlaying() {
      return this.alive && !this.paused;
    }
    get isPaused() {
      return this.paused;
    }
    pairs() {
      const range = (m) => typeof m === "number" ? m : [m[0], m[1]];
      const c = this.startColor;
      const colors = c === null ? [1, 1, 1, 1] : ("r" in c) ? [c.r, c.g, c.b, c.a] : c.flatMap((x) => [x.r, x.g, x.b, x.a]);
      const s = this.shape;
      const shape = s.type === "point" ? ["point"] : s.type === "cone" ? ["cone", s.angle, s.radius] : s.type === "box" ? ["box", s.size.x, s.size.y, s.size.z] : [s.type, s.radius];
      const out = [
        "duration",
        this.duration,
        "looping",
        this.loop,
        "startDelay",
        this.startDelay,
        "lifetime",
        range(this.startLifetime),
        "speed",
        range(this.startSpeed),
        "size",
        range(this.startSize),
        "rotation",
        range(this.startRotation),
        "angularVelocity",
        range(this.angularVelocity),
        "color",
        colors,
        "gravity",
        this.gravityModifier,
        "drag",
        this.drag,
        "worldSpace",
        this.simulationSpace === "world",
        "maxParticles",
        this.maxParticles,
        "rate",
        this.rateOverTime,
        "bursts",
        this.bursts.flatMap((b) => [b.time, b.count, b.cycles ?? 1, b.interval ?? 0.01]),
        "shape",
        shape,
        "texture",
        this.texture?.id ?? null,
        "additive",
        this.blendMode === "additive",
        "seed",
        this.randomSeed
      ];
      if (this.sizeOverLifetime)
        out.push("sizeOverLifetime", this.sizeOverLifetime.flatMap((k) => [k.time, k.value]));
      if (this.colorOverLifetime) {
        out.push("colorOverLifetime", this.colorOverLifetime.flatMap((k) => [k.time, k.color.r, k.color.g, k.color.b, k.color.a]));
      }
      const sheet = this.textureSheet;
      if (sheet)
        out.push("sheet", [sheet.tilesX, sheet.tilesY, sheet.cycles ?? 1]);
      return out;
    }
    sync(playOnAwake = false) {
      const pairs = this.pairs();
      const key = JSON.stringify(pairs);
      if (key === this.sent)
        return;
      this.sent = key;
      __iron_op("render.particles", this.id, this.gameObject.id, pairs, playOnAwake);
    }
    control(action, count) {
      this.sync();
      __iron_op("render.particleControl", this.id, action, ...count === undefined ? [] : [count]);
    }
    play() {
      this.paused = false;
      this.alive = true;
      this.control("play");
    }
    stop(clear = false) {
      this.control(clear ? "stopClear" : "stop");
    }
    pause() {
      this.paused = true;
      this.control("pause");
    }
    clear() {
      this.control("clear");
    }
    emit(count) {
      this.control("emit", count);
    }
    onAwake() {
      systems.set(this.id, this);
      this.alive = this.playOnAwake;
      this.sync(this.playOnAwake);
    }
    onPreRender() {
      this.sync();
    }
    onDestroy() {
      systems.delete(this.id);
      __iron_op("render.remove", this.id);
    }
    _report(count, alive) {
      const wasAlive = this.alive;
      this._particleCount = count;
      this.alive = alive;
      if (wasAlive && !alive) {
        try {
          this.onStopped?.();
        } catch (e) {
          console.error(`[particles] error in onStopped: ${e instanceof Error ? e.message : String(e)}`);
        }
        if (this.stopAction === "destroy")
          this.gameObject.destroy();
      }
    }
  }
  globalThis.__iron_particles = (changed) => {
    for (let i = 0;i + 2 < changed.length; i += 3) {
      systems.get(changed[i])?._report(changed[i + 1], changed[i + 2] === 1);
    }
  };
  // ../../js/engine/src/textures.ts
  var nextTexture = 1;
  var textures = new Map;

  class Texture {
    id;
    src;
    width = 0;
    height = 0;
    isLoaded = false;
    failed = false;
    _filterMode;
    _wrapMode;
    _mipmaps;
    _anisoLevel;
    linear;
    pixels = null;
    waiting = [];
    constructor(src, options) {
      this.id = nextTexture++;
      this.src = src;
      this._filterMode = options.filterMode ?? "bilinear";
      this._wrapMode = options.wrapMode ?? "repeat";
      this._mipmaps = options.mipmaps ?? true;
      this._anisoLevel = options.anisoLevel ?? 1;
      this.linear = options.linear ?? false;
      textures.set(this.id, this);
    }
    static _engine(id, width, height) {
      const t = new Texture(null, {});
      textures.delete(t.id);
      nextTexture--;
      t.id = id;
      t.width = width;
      t.height = height;
      t.isLoaded = true;
      return t;
    }
    static load(src, options = {}) {
      const path = Assets.resolve(src);
      const imported = Assets.importer(path);
      const pick = (k) => options[k] ?? imported[k];
      const t = new Texture(path, {
        filterMode: pick("filterMode"),
        wrapMode: pick("wrapMode"),
        mipmaps: pick("mipmaps"),
        anisoLevel: pick("anisoLevel"),
        linear: pick("linear")
      });
      t.send();
      return t;
    }
    static create(width, height, colors, options = {}) {
      if (colors.length !== width * height) {
        throw new Error(`Texture.create: ${colors.length} colours for ${width}x${height}`);
      }
      const t = new Texture(null, options);
      const px = new Array(width * height * 4);
      const byte = options.linear ? (v) => Math.round(Mathf.clamp01(v) * 255) : (v) => Math.round(Mathf.clamp01(Mathf.linearToGammaSpace(Mathf.clamp01(v))) * 255);
      for (let y = 0;y < height; y++) {
        const row = height - 1 - y;
        for (let x = 0;x < width; x++) {
          const c = colors[y * width + x];
          const i = (row * width + x) * 4;
          px[i] = byte(c.r);
          px[i + 1] = byte(c.g);
          px[i + 2] = byte(c.b);
          px[i + 3] = Math.round(Mathf.clamp01(c.a) * 255);
        }
      }
      t.pixels = px;
      t.width = width;
      t.height = height;
      t.send();
      return t;
    }
    static _fromBytes(width, height, bytes, options = {}) {
      const t = new Texture(null, options);
      t.pixels = bytes;
      t.width = width;
      t.height = height;
      t.send();
      return t;
    }
    get _bytes() {
      return this.pixels;
    }
    get filterMode() {
      return this._filterMode;
    }
    set filterMode(f) {
      this._filterMode = f;
      this.send();
    }
    get wrapMode() {
      return this._wrapMode;
    }
    set wrapMode(w) {
      this._wrapMode = w;
      this.send();
    }
    get anisoLevel() {
      return this._anisoLevel;
    }
    set anisoLevel(n) {
      this._anisoLevel = Math.min(16, Math.max(1, Math.round(n)));
      this.send();
    }
    whenLoaded() {
      if (this.isLoaded)
        return Promise.resolve(this);
      if (this.failed)
        return Promise.reject(new Error(`texture ${this.src ?? this.id} failed to load`));
      return new Promise((resolve, reject) => {
        this.waiting.push((t) => t.failed ? reject(new Error(`texture ${t.src ?? t.id} failed to load`)) : resolve(t));
      });
    }
    destroy() {
      textures.delete(this.id);
      __iron_op("render.textureRemove", this.id);
    }
    send() {
      __iron_op("render.texture", this.id, this.src, this._filterMode, this._wrapMode, this._mipmaps, ...this.pixels ? [this.width, this.height, this.pixels] : [null, null, null], this._anisoLevel, this.linear);
    }
    _loaded(width, height) {
      this.failed = width === 0;
      this.isLoaded = !this.failed;
      if (!this.failed) {
        this.width = width;
        this.height = height;
      }
      const waiting = this.waiting;
      this.waiting = [];
      for (const f of waiting)
        f(this);
    }
  }

  class RenderTexture extends Texture {
    constructor(width, height) {
      super(null, { wrapMode: "clamp", mipmaps: false });
      this.width = Math.max(1, Math.round(width));
      this.height = Math.max(1, Math.round(height));
      this.isLoaded = true;
      this.send();
    }
    setSize(width, height) {
      this.width = Math.max(1, Math.round(width));
      this.height = Math.max(1, Math.round(height));
      this.send();
    }
    send() {
      if (!this.isLoaded)
        return;
      __iron_op("render.renderTexture", this.id, this.width, this.height);
    }
  }
  globalThis.__iron_textures = (loaded) => {
    for (const [id, w, h] of loaded)
      textures.get(id)?._loaded(w, h);
  };

  class Sprite {
    texture;
    rect;
    pivot;
    pixelsPerUnit;
    constructor(texture, rect, pivot, pixelsPerUnit) {
      this.texture = texture;
      this.rect = rect;
      this.pivot = pivot;
      this.pixelsPerUnit = pixelsPerUnit;
    }
    static create(texture, rect = null, pivot = new Vector2(0.5, 0.5), pixelsPerUnit = 100) {
      return new Sprite(texture, rect, new Vector2(pivot.x, pivot.y), pixelsPerUnit);
    }
    get size() {
      const w = this.rect?.width ?? this.texture.width;
      const h = this.rect?.height ?? this.texture.height;
      return new Vector2(w / this.pixelsPerUnit, h / this.pixelsPerUnit);
    }
    static grid(texture, columns, rows, pivot, pixelsPerUnit) {
      const w = texture.width / columns;
      const h = texture.height / rows;
      const out = [];
      for (let r = 0;r < rows; r++) {
        for (let c = 0;c < columns; c++) {
          out.push(Sprite.create(texture, new Rect(c * w, texture.height - (r + 1) * h, w, h), pivot, pixelsPerUnit));
        }
      }
      return out;
    }
  }
  var _init39, _base39, _dec188, _dec189, _dec190, _dec191, _dec192;

  class SpriteRenderer extends (_base39 = Component) {
    static {
      _init39 = __decoratorStart(_base39);
      __decorateElement(_init39, 2, "sprite", _dec188, this);
      __decorateElement(_init39, 2, "color", _dec189, this);
      __decorateElement(_init39, 2, "flipX", _dec190, this);
      __decorateElement(_init39, 2, "flipY", _dec191, this);
      __decorateElement(_init39, 2, "sortingOrder", _dec192, this);
      __decoratorMetadata(_init39, this);
    }
    _sprite = (__runInitializers(_init39, 5, this), null);
    _color = Color.white;
    _flipX = false;
    _flipY = false;
    _sortingOrder = 0;
    started = false;
    shown = false;
    get [(_dec188 = [
      property
    ], "sprite")]() {
      return this._sprite;
    }
    set sprite(s) {
      this._sprite = s;
      this.send();
    }
    get [(_dec189 = [
      property
    ], "color")]() {
      return this._color;
    }
    set color(c) {
      this._color = c;
      this.send();
    }
    get [(_dec190 = [
      property
    ], "flipX")]() {
      return this._flipX;
    }
    set flipX(f) {
      this._flipX = f;
      this.send();
    }
    get [(_dec191 = [
      property
    ], "flipY")]() {
      return this._flipY;
    }
    set flipY(f) {
      this._flipY = f;
      this.send();
    }
    get [(_dec192 = [
      property
    ], "sortingOrder")]() {
      return this._sortingOrder;
    }
    set sortingOrder(o) {
      this._sortingOrder = Math.trunc(o);
      this.send();
    }
    send() {
      if (!this.started)
        return;
      const s = this._sprite;
      if (!s) {
        if (this.shown)
          __iron_op("render.remove", this.id);
        this.shown = false;
        return;
      }
      const c = this._color;
      const r = s.rect;
      __iron_op("render.sprite", this.id, this.gameObject.id, s.texture.id, r ? [r.x, r.y, r.width, r.height] : null, [s.pivot.x, s.pivot.y], s.pixelsPerUnit, [c.r, c.g, c.b, c.a], this._flipX, this._flipY, this._sortingOrder);
      this.shown = true;
    }
    onAwake() {
      this.started = true;
      this.send();
    }
    onDestroy() {
      __iron_op("render.remove", this.id);
    }
  }
  // ../../js/engine/src/input.ts
  class DomainInput {
    keys = new Set;
    pressedPending = new Set;
    releasedPending = new Set;
    pressedThisTick = new Set;
    releasedThisTick = new Set;
    mouseX = 0;
    mouseY = 0;
    deltaPending = { x: 0, y: 0 };
    wheelPending = { x: 0, y: 0 };
    deltaThisTick = { x: 0, y: 0 };
    wheelThisTick = { x: 0, y: 0 };
    pads = new Map;
    listeners = {};
    tick() {
      this.pressedThisTick = this.pressedPending;
      this.releasedThisTick = this.releasedPending;
      this.pressedPending = new Set;
      this.releasedPending = new Set;
      this.deltaThisTick = this.deltaPending;
      this.wheelThisTick = this.wheelPending;
      this.deltaPending = { x: 0, y: 0 };
      this.wheelPending = { x: 0, y: 0 };
    }
  }
  var MAIN_DOMAIN = 1;
  var domains = new Map;
  function domainState(d) {
    let s = domains.get(d);
    if (!s) {
      s = new DomainInput;
      domains.set(d, s);
    }
    return s;
  }
  function cur() {
    const f = globalThis.__iron_domain;
    return domainState(typeof f === "function" ? f() : MAIN_DOMAIN);
  }
  onTick(() => domainState(MAIN_DOMAIN).tick());
  function emit(type, event) {
    for (const fn of cur().listeners[type] ?? []) {
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
    const pads = cur().pads;
    let p = pads.get(id);
    if (!p) {
      p = { id, name, connected: true, buttons: {}, axes: {} };
      pads.set(id, p);
    }
    return p;
  }
  globalThis.__iron_input = (type, ...args) => {
    switch (type) {
      case "tick":
        cur().tick();
        break;
      case "keydown": {
        const [code, key, repeat] = args;
        if (!repeat && !cur().keys.has(code))
          cur().pressedPending.add(code);
        cur().keys.add(code);
        emit("keydown", { code, key, repeat: !!repeat });
        break;
      }
      case "keyup": {
        const [code, key] = args;
        if (cur().keys.has(code))
          cur().releasedPending.add(code);
        cur().keys.delete(code);
        emit("keyup", { code, key, repeat: false });
        break;
      }
      case "mousemove":
        [cur().mouseX, cur().mouseY] = args;
        break;
      case "mousedelta": {
        const [dx, dy] = args;
        cur().deltaPending.x += dx;
        cur().deltaPending.y += dy;
        break;
      }
      case "wheel": {
        const [dx, dy] = args;
        cur().wheelPending.x += dx;
        cur().wheelPending.y += dy;
        break;
      }
      case "blur":
        cur().keys.clear();
        for (const p of cur().pads.values())
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
  var nextModel = 1;
  var models = new Map;
  var cache2 = new Map;

  class Model {
    id;
    src;
    isLoaded = false;
    failed = false;
    materials = [];
    meshes = [];
    animations = [];
    description = null;
    waiting = [];
    pending = [];
    constructor(src) {
      this.id = nextModel++;
      this.src = src;
      models.set(this.id, this);
      __iron_op("render.model", this.id, src);
    }
    static load(source) {
      const src = Assets.resolve(source);
      let m = cache2.get(src);
      if (!m) {
        m = new Model(src);
        cache2.set(src, m);
      }
      return m;
    }
    whenLoaded() {
      if (this.isLoaded)
        return Promise.resolve(this);
      if (this.failed)
        return Promise.reject(new Error(`model ${this.src} failed to load`));
      return new Promise((resolve, reject) => {
        this.waiting.push((m) => m.failed ? reject(new Error(`model ${m.src} failed to load`)) : resolve(m));
      });
    }
    instantiate(options = {}) {
      const name = this.src.split("/").pop()?.replace(/\.(glb|gltf)$/i, "") ?? "Model";
      const root = new GameObject(name, {
        position: options.position,
        rotation: options.rotation,
        scale: options.scale,
        parent: options.parent
      });
      this._buildInto(root, options);
      return root;
    }
    _buildInto(root, options = {}) {
      root._model = options.colliders && options.colliders !== "none" ? { src: this.src, colliders: options.colliders } : { src: this.src };
      if (this.description)
        this.build(root, options);
      else if (!this.failed)
        this.pending.push({ options, root });
    }
    build(root, options) {
      const d = this.description;
      const objects = new Map;
      const skinned = [];
      const make = (index, parent) => {
        const n = d.nodes[index];
        const go = new GameObject(n.name || `Node ${index}`, {
          parent,
          position: new Vector3(...n.position),
          rotation: new Quaternion(...n.rotation),
          scale: new Vector3(...n.scale)
        });
        go._modelPart = true;
        objects.set(index, go);
        n.primitives.forEach((p, i) => {
          const holder = n.primitives.length === 1 ? go : new GameObject(`${go.name} ${i}`, { parent: go });
          holder._modelPart = true;
          const mesh = Mesh._engine(p.mesh);
          const material = p.material === null ? new Material : this.materials[p.material] ?? new Material;
          const skin = n.skin == null ? undefined : d.skins?.[n.skin];
          if (skin)
            skinned.push({ holder, mesh, material, skin });
          else
            holder.addComponent(MeshRenderer, { mesh, material });
          if (options.colliders === "mesh" || options.colliders === "convex") {
            holder.addComponent(MeshCollider, { sharedMesh: mesh, convex: options.colliders === "convex" });
          }
        });
        for (const c of n.children)
          make(c, go);
      };
      for (const r of d.roots)
        make(r, root);
      for (const s of skinned) {
        s.mesh.bindposes = Array.from({ length: s.skin.joints.length }, (_, i) => new Matrix4x4(s.skin.inverseBind.slice(i * 16, i * 16 + 16)));
        const bones = s.skin.joints.map((j) => objects.get(j)).filter((b) => b !== undefined);
        s.holder.addComponent(SkinnedMeshRenderer, { mesh: s.mesh, material: s.material, bones, rootBone: bones[0] ?? null });
      }
      if (this.animations.length && !root.getComponent(Animation)) {
        root.addComponent(Animation, { clip: this.animations[0], clips: [...this.animations] });
      }
    }
    clips(d) {
      const paths = new Map;
      const walk = (index, prefix) => {
        const n = d.nodes[index];
        const path = prefix + (n.name || `Node ${index}`);
        paths.set(index, path);
        for (const c of n.children)
          walk(c, `${path}/`);
      };
      for (const r of d.roots)
        walk(r, "");
      return (d.animations ?? []).map((a, i) => {
        const clip = new AnimationClip(a.name || `Animation ${i}`);
        clip.wrapMode = "loop";
        for (const ch of a.channels) {
          const path = paths.get(ch.node);
          if (path === undefined)
            continue;
          const [property, size] = ch.path === "translation" ? ["localPosition", 3] : ch.path === "rotation" ? ["localRotation", 4] : ["localScale", 3];
          const values = ch.path === "rotation" ? continuous(ch.values, ch.interpolation === "cubic") : ch.values;
          for (let k = 0;k < size; k++) {
            const curve = ch.interpolation === "cubic" ? cubic(ch.times, values, size, k) : AnimationCurve.fromPoints(ch.times.map((t, i) => [t, values[i * size + k]]), ch.interpolation === "step" ? "step" : "linear");
            clip.setCurve(path, "Transform", `${property}.${"xyzw"[k]}`, curve);
          }
        }
        return clip;
      });
    }
    _loaded(description) {
      if (description) {
        this.description = description;
        this.isLoaded = true;
        this.materials = description.materials.map((m) => {
          const [r, g, b, a] = m.color;
          const texture = m.texture ? Texture._engine(...m.texture) : null;
          const map = (id) => id == null ? null : Texture._engine(id, 0, 0);
          return new Material(new Color(r, g, b, a), new Color(m.emission[0], m.emission[1], m.emission[2], 1), m.smoothness, m.unlit, texture, Vector2.one, Vector2.zero, m.metallic ?? 0).with({
            renderingMode: m.alpha === "mask" ? "cutout" : m.alpha === "blend" ? "fade" : "opaque",
            alphaCutoff: m.alphaCutoff ?? 0.5,
            doubleSided: m.doubleSided ?? false,
            normalMap: map(m.normal?.[0]),
            normalScale: m.normal?.[1] ?? 1,
            metallicGlossMap: map(m.metallicRoughness),
            occlusionMap: map(m.occlusion?.[0]),
            occlusionStrength: m.occlusion?.[1] ?? 1,
            emissionMap: map(m.emissive),
            gltfPacking: true
          });
        });
        this.meshes = description.nodes.flatMap((n) => n.primitives.map((p) => Mesh._engine(p.mesh)));
        this.animations = this.clips(description);
        for (const { root, options } of this.pending)
          if (!root.destroyed)
            this.build(root, options);
      } else {
        this.failed = true;
      }
      this.pending = [];
      const waiting = this.waiting;
      this.waiting = [];
      for (const f of waiting)
        f(this);
    }
  }
  globalThis.__iron_models = (loaded) => {
    for (let i = 0;i + 1 < loaded.length; i += 2) {
      const m = models.get(loaded[i]);
      const json = loaded[i + 1];
      m?._loaded(typeof json === "string" ? JSON.parse(json) : null);
    }
  };
  function continuous(values, cubicKeys) {
    const out = [...values];
    const stride = cubicKeys ? 12 : 4;
    const at = cubicKeys ? 4 : 0;
    for (let i = stride;i + stride <= out.length; i += stride) {
      let dot = 0;
      for (let c = 0;c < 4; c++)
        dot += out[i - stride + at + c] * out[i + at + c];
      if (dot < 0)
        for (let c = 0;c < stride; c++)
          out[i + c] = -out[i + c];
    }
    return out;
  }
  function cubic(times, values, size, k) {
    const keys = times.map((t, i) => {
      const at = (part) => values[(i * 3 + part) * size + k];
      return { t, inT: at(0), v: at(1), outT: at(2) };
    });
    const curve = new AnimationCurve;
    for (const key of keys)
      curve.addKey(key.t, key.v);
    curve.keys.forEach((key, i) => {
      key.inTangent = keys[i].inT;
      key.outTangent = keys[i].outT;
    });
    return curve;
  }
  // ../../js/engine/src/scenes.ts
  _registerBuiltIns({
    AudioListener,
    AudioSource,
    BoxCollider,
    BoxCollider2D,
    Camera,
    CameraFollow,
    CameraOrbit,
    CapsuleCollider,
    CapsuleCollider2D,
    Animation,
    Animator,
    CharacterController,
    CharacterJoint,
    Cloth,
    CircleCollider2D,
    EdgeCollider2D,
    FixedJoint,
    HingeJoint,
    Light,
    LineRenderer,
    LODGroup,
    MeshCollider,
    MeshRenderer,
    SkinnedMeshRenderer,
    Mirror,
    ParticleSystem,
    PlaneRenderer,
    PolygonCollider2D,
    ReflectionProbe,
    Rigidbody,
    Rigidbody2D,
    Rope,
    RopeJoint,
    SphereCollider,
    SpringJoint,
    SpriteRenderer,
    WorldPanel
  });
  function fromGuid(guid, src) {
    return typeof guid === "string" && Assets.guidToPath(guid) || String(src);
  }
  function toHex(bytes) {
    let out = "";
    for (const b of bytes)
      out += (b < 16 ? "0" : "") + b.toString(16);
    return out;
  }
  function fromHex(hex) {
    const out = new Array(hex.length >> 1);
    for (let i = 0;i < out.length; i++)
      out[i] = parseInt(hex.slice(i * 2, i * 2 + 2), 16);
    return out;
  }
  var warn = (message) => console.warn(`scene: ${message}`);
  var UNSAVED = Symbol("unsaved");
  var MATERIAL_EXTRAS = [
    "normalMap",
    "normalScale",
    "metallicGlossMap",
    "specularColor",
    "occlusionMap",
    "occlusionStrength",
    "emissionMap",
    "detailAlbedoMap",
    "detailAlbedoMapScale",
    "detailAlbedoMapOffset",
    "renderingMode",
    "alphaCutoff",
    "doubleSided",
    "gltfPacking"
  ];
  var DEFAULT_MATERIAL = new Material;

  class Saver {
    keepExternal;
    reload;
    ids = new Map;
    assets = {};
    assetIds = new Map;
    constructor(keepExternal, reload = false) {
      this.keepExternal = keepExternal;
      this.reload = reload;
    }
    field(v, where) {
      if (typeof v === "function" || typeof v === "symbol" || typeof v === "bigint")
        return UNSAVED;
      this.unsaved = false;
      const out = this.value(v, where);
      return this.unsaved ? UNSAVED : out;
    }
    unsaved = false;
    id(thing) {
      let id = this.ids.get(thing);
      if (id === undefined) {
        id = this.ids.size + 1;
        this.ids.set(thing, id);
      }
      return id;
    }
    asset(key, prefix, make) {
      let id = this.assetIds.get(key);
      if (!id) {
        id = `${prefix}${this.assetIds.size + 1}`;
        this.assetIds.set(key, id);
        this.assets[id] = make();
      }
      return { $asset: id };
    }
    value(v, where) {
      if (v === null || v === undefined || typeof v === "number" || typeof v === "string" || typeof v === "boolean") {
        return v ?? null;
      }
      if (Array.isArray(v))
        return v.map((x, i) => this.value(x, `${where}[${i}]`));
      if (v instanceof Vector2Int)
        return { $type: "Vector2Int", value: [v.x, v.y] };
      if (v instanceof Vector3Int)
        return { $type: "Vector3Int", value: [v.x, v.y, v.z] };
      if (v instanceof Vector2)
        return { $type: "Vector2", value: [v.x, v.y] };
      if (v instanceof Vector3)
        return { $type: "Vector3", value: [v.x, v.y, v.z] };
      if (v instanceof Vector4)
        return { $type: "Vector4", value: [v.x, v.y, v.z, v.w] };
      if (v instanceof Quaternion)
        return { $type: "Quaternion", value: [v.x, v.y, v.z, v.w] };
      if (v instanceof Color)
        return { $type: "Color", value: [v.r, v.g, v.b, v.a] };
      if (v instanceof RectInt)
        return { $type: "RectInt", value: [v.x, v.y, v.width, v.height] };
      if (v instanceof Rect)
        return { $type: "Rect", value: [v.x, v.y, v.width, v.height] };
      if (v instanceof GameObject || v instanceof Component) {
        const id = this.ids.get(v);
        const kind = v instanceof GameObject ? "GameObject" : "Component";
        if (id !== undefined)
          return { $ref: kind, id };
        if (this.keepExternal)
          return { $ref: kind, live: v instanceof GameObject ? v.id : v.id };
        return null;
      }
      if (v instanceof AnimationCurve)
        return { $type: "AnimationCurve", data: v._toJSON() };
      if (v instanceof AnimationClip)
        return { $type: "AnimationClip", data: v.toJSON() };
      if (v instanceof AnimatorController)
        return { $type: "AnimatorController", data: v.toJSON() };
      if (v instanceof Material) {
        return {
          $type: "Material",
          color: this.value(v.color, where),
          emission: this.value(v.emission, where),
          smoothness: v.smoothness,
          metallic: v.metallic,
          unlit: v.unlit,
          mainTexture: this.value(v.mainTexture, where),
          mainTextureScale: this.value(v.mainTextureScale, where),
          mainTextureOffset: this.value(v.mainTextureOffset, where),
          ...Object.fromEntries(MATERIAL_EXTRAS.filter((k) => {
            const d = DEFAULT_MATERIAL[k];
            const x = v[k];
            return !(x === d || x instanceof Vector2 && d instanceof Vector2 && x.equals(d));
          }).map((k) => [k, this.value(v[k], where)]))
        };
      }
      if (v instanceof RenderTexture) {
        return this.asset(v, "texture", () => ({ type: "RenderTexture", width: v.width, height: v.height }));
      }
      if (v instanceof Texture) {
        const bytes = v._bytes;
        if (!v.src && !bytes)
          return this.cannot(`${where}: only textures loaded from files or made with Texture.create are saved`);
        const t = v;
        const guid = v.src ? Assets.pathToGuid(v.src) : null;
        return this.asset(v, "texture", () => ({
          type: "Texture",
          ...v.src ? { src: v.src } : { width: v.width, height: v.height, pixels: toHex(bytes) },
          ...guid ? { guid } : {},
          filterMode: v.filterMode,
          wrapMode: v.wrapMode,
          mipmaps: t._mipmaps,
          ...v.anisoLevel !== 1 ? { anisoLevel: v.anisoLevel } : {},
          ...v.linear ? { linear: true } : {}
        }));
      }
      if (v instanceof Sprite) {
        return { $type: "Sprite", texture: this.value(v.texture, where), rect: this.value(v.rect, where), pivot: this.value(v.pivot, where), pixelsPerUnit: v.pixelsPerUnit };
      }
      if (v instanceof Mesh) {
        const data = v._data;
        if (!data)
          return this.cannot(`${where}: only meshes made with Mesh.create are saved (model meshes come with their model)`);
        return this.asset(v, "mesh", () => ({
          type: "Mesh",
          ...data,
          ...v.bindposes.length ? { bindposes: v.bindposes.flatMap((m) => [...m.m]) } : {}
        }));
      }
      if (v instanceof Sound) {
        const guid = Assets.pathToGuid(v.src);
        return { $type: "Sound", src: v.src, ...guid ? { guid } : {} };
      }
      const proto = Object.getPrototypeOf(v);
      if (proto === Object.prototype || proto === null) {
        const out = {};
        for (const [k, x] of Object.entries(v)) {
          if (x !== undefined)
            out[k] = this.value(x, `${where}.${k}`);
        }
        return out;
      }
      return this.cannot(`${where}: can't save a ${proto?.constructor?.name ?? "value"}`);
    }
    cannot(message) {
      if (this.reload)
        this.unsaved = true;
      else
        warn(message);
      return null;
    }
  }

  class Loader {
    data;
    things = new Map;
    assets = new Map;
    constructor(data = {}) {
      this.data = data;
    }
    asset(id) {
      if (this.assets.has(id))
        return this.assets.get(id);
      const a = this.data[id];
      let made = null;
      if (a?.type === "RenderTexture") {
        made = new RenderTexture(Number(a.width), Number(a.height));
      } else if (a?.type === "Texture") {
        const options = {
          filterMode: a.filterMode,
          wrapMode: a.wrapMode,
          mipmaps: a.mipmaps,
          anisoLevel: a.anisoLevel,
          linear: a.linear
        };
        made = typeof a.pixels === "string" ? Texture._fromBytes(Number(a.width), Number(a.height), fromHex(a.pixels), options) : Texture.load(fromGuid(a.guid, a.src), options);
      } else if (a?.type === "Mesh") {
        const flat = (k) => a[k] ?? [];
        const v3 = (f) => Array.from({ length: f.length / 3 }, (_, i) => new Vector3(f[i * 3], f[i * 3 + 1], f[i * 3 + 2]));
        const uv = flat("uv");
        const joints = flat("joints");
        const weights = flat("weights");
        const four = (f, i) => [f[i * 4], f[i * 4 + 1], f[i * 4 + 2], f[i * 4 + 3]];
        const poses = flat("bindposes");
        made = Mesh.create({
          vertices: v3(flat("vertices")),
          triangles: flat("triangles"),
          normals: v3(flat("normals")),
          uv: Array.from({ length: uv.length / 2 }, (_, i) => new Vector2(uv[i * 2], uv[i * 2 + 1])),
          boneWeights: Array.from({ length: joints.length / 4 }, (_, i) => ({ indices: four(joints, i), weights: four(weights, i) })),
          bindposes: Array.from({ length: poses.length / 16 }, (_, i) => new Matrix4x4(poses.slice(i * 16, i * 16 + 16)))
        });
      } else {
        warn(`unknown asset ${id}`);
      }
      this.assets.set(id, made);
      return made;
    }
    value(v) {
      if (v === null || typeof v !== "object")
        return v;
      if (Array.isArray(v))
        return v.map((x) => this.value(x));
      const o = v;
      if (typeof o.$asset === "string")
        return this.asset(o.$asset);
      if (typeof o.$ref === "string") {
        if (typeof o.live === "number") {
          return o.$ref === "GameObject" ? GameObject.byId(o.live) ?? null : liveComponent(o.live) ?? null;
        }
        const found = this.things.get(o.id);
        return found ?? null;
      }
      if (typeof o.$type === "string") {
        const n = o.value ?? [];
        switch (o.$type) {
          case "Vector2":
            return new Vector2(n[0], n[1]);
          case "Vector2Int":
            return new Vector2Int(n[0], n[1]);
          case "Vector3":
            return new Vector3(n[0], n[1], n[2]);
          case "Vector3Int":
            return new Vector3Int(n[0], n[1], n[2]);
          case "Vector4":
            return new Vector4(n[0], n[1], n[2], n[3]);
          case "Quaternion":
            return new Quaternion(n[0], n[1], n[2], n[3]);
          case "Color":
            return new Color(n[0], n[1], n[2], n[3]);
          case "Rect":
            return new Rect(n[0], n[1], n[2], n[3]);
          case "RectInt":
            return new RectInt(n[0], n[1], n[2], n[3]);
          case "AnimationCurve":
            return AnimationCurve._fromJSON(o.data);
          case "AnimationClip":
            return AnimationClip.fromJSON(o.data);
          case "AnimatorController":
            return AnimatorController.fromJSON(o.data);
          case "Material":
            return Material.create({
              color: this.value(o.color),
              emission: this.value(o.emission),
              smoothness: o.smoothness,
              metallic: o.metallic,
              unlit: o.unlit,
              mainTexture: this.value(o.mainTexture),
              mainTextureScale: this.value(o.mainTextureScale),
              mainTextureOffset: this.value(o.mainTextureOffset),
              ...Object.fromEntries(MATERIAL_EXTRAS.filter((k) => (k in o)).map((k) => [k, this.value(o[k])]))
            });
          case "Sprite":
            return Sprite.create(this.value(o.texture), this.value(o.rect), this.value(o.pivot), o.pixelsPerUnit);
          case "Sound":
            return new Sound(fromGuid(o.guid, o.src));
          default:
            warn(`unknown type ${o.$type}`);
            return null;
        }
      }
      const out = {};
      for (const [k, x] of Object.entries(o))
        out[k] = this.value(x);
      return out;
    }
  }
  function liveComponent(id) {
    for (const root of GameObject.roots) {
      const stack = [root];
      while (stack.length) {
        const o = stack.pop();
        const c = o.components.find((c) => c.id === id);
        if (c)
          return c;
        stack.push(...o.children);
      }
    }
    return;
  }
  function number(saver, roots) {
    const walk = (go) => {
      if (go._modelPart || unsaved(go))
        return;
      saver.id(go);
      for (const c of go.components)
        saver.id(c);
      for (const child of go.children)
        walk(child);
    };
    for (const r of roots)
      walk(r);
  }
  var unsaved = (go) => (go.hideFlags & HideFlags.DontSave) !== 0;
  function saveObject(saver, go) {
    if (go._modelPart || unsaved(go))
      return null;
    const t = go.transform;
    const p = t.localPosition;
    const r = t.localRotation;
    const s = t.localScale;
    const data = { id: saver.id(go), name: go.name };
    if (p.x !== 0 || p.y !== 0 || p.z !== 0)
      data.position = [p.x, p.y, p.z];
    if (r.x !== 0 || r.y !== 0 || r.z !== 0 || r.w !== 1)
      data.rotation = [r.x, r.y, r.z, r.w];
    if (s.x !== 1 || s.y !== 1 || s.z !== 1)
      data.scale = [s.x, s.y, s.z];
    if (!go.enabled)
      data.enabled = false;
    if (go.layer !== 0)
      data.layer = go.layer;
    if (go._model) {
      const guid = Assets.pathToGuid(go._model.src);
      data.model = { ...go._model, ...guid ? { guid } : {} };
    }
    if (go._prefab)
      data.prefab = go._prefab;
    const components = go.components.map((c) => {
      const cls = c.constructor;
      const properties = {};
      const own = c;
      const props = propertiesOf(cls);
      for (const name of props) {
        const value = own[name];
        if (value !== undefined)
          properties[name] = saver.value(value, `${go.name}.${_nameOf(cls)}.${name}`);
      }
      if (saver.reload && !_isBuiltIn(cls)) {
        for (const name of Object.keys(own)) {
          if (name === "id" || name === "gameObject" || props.includes(name) || own[name] === undefined)
            continue;
          const value = saver.field(own[name], `${go.name}.${_nameOf(cls)}.${name}`);
          if (value !== UNSAVED)
            properties[name] = value;
        }
      }
      return { id: saver.id(c), type: _nameOf(cls), properties };
    });
    if (components.length)
      data.components = components;
    const children = go.children.map((c) => saveObject(saver, c)).filter((c) => c !== null);
    if (children.length)
      data.children = children;
    return data;
  }
  function loadObjects(list, assets, parent, reload = false) {
    const loader = new Loader(assets);
    const pending = [];
    const disabled = [];
    const build = (d, under) => {
      const [p, r, s] = [d.position, d.rotation, d.scale];
      const go = new GameObject(d.name ?? "GameObject", {
        parent: under,
        position: p ? new Vector3(p[0], p[1], p[2]) : undefined,
        rotation: r ? new Quaternion(r[0], r[1], r[2], r[3]) : undefined,
        scale: s ? new Vector3(s[0], s[1], s[2]) : undefined
      });
      loader.things.set(d.id, go);
      if (d.layer)
        go.layer = d.layer;
      if (d.enabled === false)
        disabled.push(go);
      if (d.model)
        Model.load(fromGuid(d.model.guid, d.model.src))._buildInto(go, { colliders: d.model.colliders });
      if (d.prefab)
        go._prefab = d.prefab;
      for (const c of d.components ?? []) {
        const cls = _classByName(c.type);
        if (!cls) {
          warn(`${d.name}: unknown component ${c.type} (register it with register({ ${c.type} }))`);
          continue;
        }
        const component = go._addComponentAsleep(cls);
        loader.things.set(c.id, component);
        pending.push({ component, properties: c.properties ?? {} });
      }
      for (const child of d.children ?? [])
        build(child, go);
      return go;
    };
    const made = list.map((d) => build(d, parent));
    for (const { component, properties } of pending) {
      const own = component;
      const cls = component.constructor;
      const props = propertiesOf(cls);
      for (const [name, value] of Object.entries(properties)) {
        if (!props.includes(name) && !Object.hasOwn(own, name)) {
          warn(`${component.gameObject.name}: ${_nameOf(cls)} has no ${name} any more; dropped`);
          continue;
        }
        try {
          own[name] = loader.value(value);
        } catch (e) {
          warn(`${component.gameObject.name}: couldn't set ${name}: ${e}`);
        }
      }
    }
    const restored = new Set(reload ? pending.map((p) => p.component).filter((c) => !_isBuiltIn(c.constructor)) : []);
    for (const component of restored)
      _markRestored(component);
    for (const { component } of pending)
      if (!restored.has(component))
        component.onAwake?.();
    for (const component of restored) {
      try {
        component.onReload?.();
      } catch (e) {
        console.error(`${component.gameObject.name}: onReload: ${e}`);
      }
    }
    for (const go of disabled)
      go.enabled = false;
    return made;
  }
  var nextHandle = 1;

  class Scene {
    name;
    handle;
    path = null;
    isLoaded = true;
    _renderSettings = null;
    constructor(name) {
      this.name = name;
      this.handle = nextHandle++;
    }
    getRootGameObjects() {
      return GameObject.roots.filter((go) => go._scene === this);
    }
    get rootCount() {
      return this.getRootGameObjects().length;
    }
    serialize() {
      return this._serialize(false);
    }
    _serialize(reload) {
      const saver = new Saver(false, reload);
      const roots = this.getRootGameObjects();
      number(saver, roots);
      const objects = roots.map((go) => saveObject(saver, go)).filter((d) => d !== null);
      const data = { format: "iron.scene", version: 1, name: this.name, objects };
      if (this === active)
        data.renderSettings = saveRenderSettings(saver);
      if (Object.keys(saver.assets).length)
        data.assets = saver.assets;
      return data;
    }
  }
  function saveRenderSettings(saver) {
    return saver.value({
      ambientLight: RenderSettings.ambientLight,
      ambientMode: RenderSettings.ambientMode,
      reflectionIntensity: RenderSettings.reflectionIntensity,
      skybox: RenderSettings.skybox,
      bloom: RenderSettings.bloom,
      fog: RenderSettings.fog,
      fogMode: RenderSettings.fogMode,
      fogColor: RenderSettings.fogColor,
      fogDensity: RenderSettings.fogDensity,
      fogStartDistance: RenderSettings.fogStartDistance,
      fogEndDistance: RenderSettings.fogEndDistance,
      heightFog: RenderSettings.heightFog,
      cloudLayer: RenderSettings.cloudLayer,
      volumetricFog: RenderSettings.volumetricFog,
      fogCulling: RenderSettings.fogCulling
    }, "renderSettings");
  }
  function applyRenderSettings(saved) {
    const s = new Loader().value(saved);
    if (s.ambientLight instanceof Color)
      RenderSettings.ambientLight = s.ambientLight;
    if (s.ambientMode === "skybox" || s.ambientMode === "color")
      RenderSettings.ambientMode = s.ambientMode;
    if (typeof s.reflectionIntensity === "number")
      RenderSettings.reflectionIntensity = s.reflectionIntensity;
    if ("skybox" in s)
      RenderSettings.skybox = s.skybox ?? null;
    if ("bloom" in s)
      RenderSettings.bloom = s.bloom ?? null;
    if (s.fogMode === "linear" || s.fogMode === "exponential" || s.fogMode === "exponentialSquared")
      RenderSettings.fogMode = s.fogMode;
    if (s.fogColor instanceof Color)
      RenderSettings.fogColor = s.fogColor;
    if (typeof s.fogDensity === "number")
      RenderSettings.fogDensity = s.fogDensity;
    if (typeof s.fogStartDistance === "number")
      RenderSettings.fogStartDistance = s.fogStartDistance;
    if (typeof s.fogEndDistance === "number")
      RenderSettings.fogEndDistance = s.fogEndDistance;
    if ("heightFog" in s)
      RenderSettings.heightFog = s.heightFog ?? null;
    if ("cloudLayer" in s)
      RenderSettings.cloudLayer = s.cloudLayer ?? null;
    if ("volumetricFog" in s)
      RenderSettings.volumetricFog = s.volumetricFog ?? null;
    if (typeof s.fogCulling === "boolean")
      RenderSettings.fogCulling = s.fogCulling;
    if (typeof s.fog === "boolean")
      RenderSettings.fog = s.fog;
  }
  var scenes = [];
  var active;
  var keep = null;
  var loadedListeners = new Set;
  var unloadedListeners = new Set;
  var activeListeners = new Set;
  function reset() {
    scenes = [new Scene("Main")];
    active = scenes[0];
    keep = null;
    loadedListeners.clear();
    unloadedListeners.clear();
    activeListeners.clear();
  }
  reset();
  function setActive(scene) {
    if (scene === active)
      return;
    const previous = active;
    active = scene;
    if (scene._renderSettings)
      applyRenderSettings(scene._renderSettings);
    for (const f of activeListeners)
      f(previous, scene);
  }
  function unload(scene) {
    for (const go of scene.getRootGameObjects())
      go.destroy();
    scene.isLoaded = false;
    scenes = scenes.filter((s) => s !== scene);
    for (const f of unloadedListeners)
      f(scene);
  }
  function fromData(data, mode, path) {
    if (data?.format !== "iron.scene")
      throw new Error("not a scene (format isn't iron.scene)");
    if (data.version !== 1)
      throw new Error(`scene version ${data.version} isn't supported`);
    const scene = new Scene(data.name);
    scene.path = path;
    scene._renderSettings = data.renderSettings ?? null;
    const old = mode === "single" ? [...scenes] : [];
    scenes.push(scene);
    for (const s of old)
      unload(s);
    const previous = active;
    active = scene;
    try {
      loadObjects(data.objects ?? [], data.assets, null);
    } finally {
      active = previous;
    }
    if (mode === "single")
      setActive(scene);
    for (const f of loadedListeners)
      f(scene, mode);
    return scene;
  }
  var nextRead = 1;
  var reads = new Map;
  globalThis.__iron_asset_text = (results) => {
    for (let i = 0;i + 1 < results.length; i += 2) {
      const read = reads.get(results[i]);
      if (!read)
        continue;
      reads.delete(results[i]);
      const text = results[i + 1];
      if (typeof text === "string")
        read.resolve(text);
      else
        read.reject(new Error(`couldn't read ${read.path}`));
    }
  };
  function readText(path) {
    const id = nextRead++;
    return new Promise((resolve, reject) => {
      reads.set(id, { resolve, reject, path });
      __iron_op("asset.readText", id, path);
    });
  }
  var SceneManager = {
    get sceneCount() {
      return scenes.length;
    },
    getSceneAt(index) {
      const s = scenes[index];
      if (!s)
        throw new RangeError(`no scene at ${index}`);
      return s;
    },
    getSceneByName(name) {
      return scenes.find((s) => s.name === name);
    },
    getActiveScene() {
      return active;
    },
    setActiveScene(scene) {
      if (!scenes.includes(scene))
        return false;
      setActive(scene);
      return true;
    },
    createScene(name) {
      if (scenes.some((s) => s.name === name))
        throw new Error(`a scene named ${name} is already loaded`);
      const scene = new Scene(name);
      scenes.push(scene);
      return scene;
    },
    loadScene(data, mode = "single") {
      return fromData(data, mode, null);
    },
    async loadSceneAsync(source, mode = "single") {
      if (typeof source !== "string")
        return fromData(source, mode, null);
      const data = JSON.parse(await readText(source));
      return fromData(data, mode, source);
    },
    unloadScene(scene) {
      if (!scenes.includes(scene) || scenes.length === 1)
        return false;
      unload(scene);
      if (active === scene)
        setActive(scenes[0]);
      return true;
    },
    moveGameObjectToScene(go, scene) {
      if (go.parent)
        throw new Error("only root objects move between scenes");
      if (!scene.isLoaded)
        throw new Error(`scene ${scene.name} isn't loaded`);
      go._scene = scene;
    },
    mergeScenes(source, destination) {
      for (const go of source.getRootGameObjects())
        go._scene = destination;
      SceneManager.unloadScene(source);
    },
    onSceneLoaded(f) {
      loadedListeners.add(f);
      return () => loadedListeners.delete(f);
    },
    onSceneUnloaded(f) {
      unloadedListeners.add(f);
      return () => unloadedListeners.delete(f);
    },
    onActiveSceneChanged(f) {
      activeListeners.add(f);
      return () => activeListeners.delete(f);
    }
  };
  var prefabFiles = new Map;

  class Prefab {
    data;
    src;
    constructor(data, src) {
      this.data = data;
      this.src = src;
    }
    static fromObject(go) {
      const saver = new Saver(false);
      number(saver, [go]);
      const root = saveObject(saver, go);
      if (!root)
        throw new Error("parts of a model can't be prefabs on their own; use the model's root");
      delete root.position;
      delete root.rotation;
      const data = { format: "iron.prefab", version: 1, name: go.name, root };
      if (Object.keys(saver.assets).length)
        data.assets = saver.assets;
      return new Prefab(JSON.parse(JSON.stringify(data)), null);
    }
    static fromData(data, src = null) {
      const d = data;
      if (d?.format !== "iron.prefab" || !d.root)
        throw new Error(`not prefab data${src ? `: ${src}` : ""}`);
      return new Prefab(d, src);
    }
    static load(path) {
      let p = prefabFiles.get(path);
      if (!p) {
        p = readText(path).then((text) => Prefab.fromData(JSON.parse(text), path));
        prefabFiles.set(path, p);
        p.catch(() => prefabFiles.delete(path));
      }
      return p;
    }
    get name() {
      return this.data.name;
    }
    instantiate(options = {}) {
      const root = JSON.parse(JSON.stringify(this.data.root));
      if (options.position)
        root.position = [options.position.x, options.position.y, options.position.z];
      if (options.rotation)
        root.rotation = [options.rotation.x, options.rotation.y, options.rotation.z, options.rotation.w];
      if (this.src)
        root.prefab = this.src;
      return loadObjects([root], this.data.assets, options.parent ?? null)[0];
    }
    toJSON() {
      return JSON.parse(JSON.stringify(this.data));
    }
  }
  function instantiate2(original, options = {}) {
    if (original instanceof Prefab)
      return original.instantiate(options);
    const saver = new Saver(true);
    number(saver, [original]);
    const data = saveObject(saver, original);
    if (!data)
      throw new Error("parts of a model can't be copied on their own; copy the model's root");
    if (!options.parent) {
      const t = original.transform;
      const [p, r, s] = [t.position, t.rotation, t.lossyScale];
      data.position = [p.x, p.y, p.z];
      data.rotation = [r.x, r.y, r.z, r.w];
      data.scale = [s.x, s.y, s.z];
    }
    if (options.position)
      data.position = [options.position.x, options.position.y, options.position.z];
    if (options.rotation)
      data.rotation = [options.rotation.x, options.rotation.y, options.rotation.z, options.rotation.w];
    const previous = active;
    if (!options.parent)
      active = original.scene;
    try {
      return loadObjects([data], saver.assets, options.parent ?? null)[0];
    } finally {
      active = previous;
    }
  }
  function sceneNames(data) {
    const out = [];
    const walk = (o) => {
      for (const c of o.components ?? [])
        out.push(c.type);
      for (const child of o.children ?? [])
        walk(child);
    };
    for (const o of data.objects)
      walk(o);
    return out;
  }
  function saveState() {
    const state = {
      version: 1,
      scenes: scenes.map((s) => s._serialize(true)),
      active: scenes.indexOf(active),
      keep: keep && keep.rootCount > 0 ? keep._serialize(true) : null,
      quality: _qualityState(),
      time: _timeState()
    };
    return JSON.stringify(state);
  }
  function restoreState(json) {
    const state = JSON.parse(json);
    const all = [...state.scenes, ...state.keep ? [state.keep] : []];
    const missing = [...new Set(all.flatMap(sceneNames))].filter((name) => !_classByName(name));
    if (missing.length) {
      warn(`can't keep state across the reload: ${missing.join(", ")} ${missing.length === 1 ? "isn't" : "aren't"} registered ` + `(register({ ${missing.join(", ")} }, { spawn: false })); starting over`);
      if (startScene)
        loadStartScene(startScene);
      return false;
    }
    _restoreQuality(state.quality);
    _restoreTime(state.time);
    for (const s of scenes)
      s.isLoaded = false;
    scenes = [];
    const build = (data) => {
      const scene = new Scene(data.name);
      scene._renderSettings = data.renderSettings ?? null;
      const previous = active;
      active = scene;
      try {
        loadObjects(data.objects ?? [], data.assets, null, true);
      } finally {
        active = previous;
      }
      return scene;
    };
    scenes = state.scenes.map(build);
    if (state.keep)
      keep = build(state.keep);
    if (!scenes.length)
      scenes = [new Scene("Main")];
    active = scenes[state.active] ?? scenes[0];
    if (active._renderSettings)
      applyRenderSettings(active._renderSettings);
    return true;
  }
  globalThis.__iron_save_state = saveState;
  globalThis.__iron_restore_state = restoreState;
  async function loadStartScene(path) {
    const main = active;
    try {
      const scene = await SceneManager.loadSceneAsync(path, "additive");
      SceneManager.setActiveScene(scene);
      if (main.isLoaded && main.rootCount === 0)
        SceneManager.unloadScene(main);
    } catch (e) {
      console.error(`start scene ${path}: ${e}`);
    }
  }
  var startScene = globalThis.__iron_info?.app.startScene;
  if (startScene && typeof __iron_op === "function" && !globalThis.__iron_reloading)
    loadStartScene(startScene);
  _setSceneHooks({
    active: () => active,
    dontDestroyOnLoad(go) {
      const root = go.root;
      keep ??= new Scene("DontDestroyOnLoad");
      root._scene = keep;
    },
    instantiate: instantiate2,
    reset
  });
  // ../../js/engine/src/shapes.ts
  var Shapes = globalThis.Shapes;
  // ../../js/engine/src/spline.ts
  var Spline = globalThis.Spline;
  var native2 = globalThis.SplineAnimate;
  var _init40, _base40, _dec193, _dec194, _dec195, _dec196, _dec197, _dec198;

  class SplineAnimate extends (_base40 = Component) {
    constructor() {
      super(...arguments);
      __runInitializers(_init40, 31, this);
    }
    static {
      _init40 = __decoratorStart(_base40);
      __decorateElement(_init40, 5, "container", _dec193, this);
      __decorateElement(_init40, 5, "duration", _dec194, this);
      __decorateElement(_init40, 5, "loopMode", _dec195, this);
      __decorateElement(_init40, 5, "alignment", _dec196, this);
      __decorateElement(_init40, 5, "playing", _dec197, this);
      __decorateElement(_init40, 5, "elapsed", _dec198, this);
      __decoratorMetadata(_init40, this);
    }
    spline = null;
    [(_dec193 = [
      property
    ], "container")] = __runInitializers(_init40, 8, this, null);
    [(_dec194 = [
      property
    ], "duration")] = (__runInitializers(_init40, 11, this), __runInitializers(_init40, 12, this, 5));
    [(_dec195 = [
      property
    ], "loopMode")] = (__runInitializers(_init40, 15, this), __runInitializers(_init40, 16, this, "loop"));
    [(_dec196 = [
      property
    ], "alignment")] = (__runInitializers(_init40, 19, this), __runInitializers(_init40, 20, this, "path"));
    [(_dec197 = [
      property
    ], "playing")] = (__runInitializers(_init40, 23, this), __runInitializers(_init40, 24, this, true));
    [(_dec198 = [
      property
    ], "elapsed")] = (__runInitializers(_init40, 27, this), __runInitializers(_init40, 28, this, 0));
    get t() {
      return native2.progress(this.elapsed, this.duration, this.loopMode);
    }
    onUpdate(dt) {
      if (this.playing)
        this.elapsed += dt;
      this.place();
    }
    place() {
      const s = this.spline;
      if (!s || s.count === 0)
        return;
      const { position, direction } = native2.pose(s, this.elapsed, this.duration, this.loopMode);
      const c = this.container?.transform;
      this.transform.position = c ? c.transformPoint(position) : position;
      if (this.alignment !== "path" || direction.magnitude < 0.000000001)
        return;
      this.transform.rotation = native2.facing(c ? c.transformDirection(direction) : direction);
    }
  }
  // ../../js/engine/src/text-mesh.ts
  var _init41, _base41, _dec199, _dec200, _dec201, _dec202, _dec203, _dec204, _dec205, _dec206, _dec207;

  class TextMesh extends (_base41 = Component) {
    static {
      _init41 = __decoratorStart(_base41);
      __decorateElement(_init41, 2, "text", _dec199, this);
      __decorateElement(_init41, 2, "fontSize", _dec200, this);
      __decorateElement(_init41, 2, "color", _dec201, this);
      __decorateElement(_init41, 2, "font", _dec202, this);
      __decorateElement(_init41, 2, "fontWeight", _dec203, this);
      __decorateElement(_init41, 2, "alignment", _dec204, this);
      __decorateElement(_init41, 2, "maxWidth", _dec205, this);
      __decorateElement(_init41, 2, "pixelsPerUnit", _dec206, this);
      __decorateElement(_init41, 2, "resolution", _dec207, this);
      __decoratorMetadata(_init41, this);
    }
    _text = (__runInitializers(_init41, 5, this), "");
    _fontSize = 32;
    _color = Color.white;
    _font = null;
    _fontWeight = 400;
    _alignment = "center";
    _maxWidth = 0;
    _pixelsPerUnit = 100;
    _resolution = 2;
    started = false;
    dirty = true;
    get [(_dec199 = [
      property
    ], "text")]() {
      return this._text;
    }
    set text(t) {
      this._text = String(t);
      this.dirty = true;
    }
    get [(_dec200 = [
      property
    ], "fontSize")]() {
      return this._fontSize;
    }
    set fontSize(s) {
      this._fontSize = Math.max(1, s);
      this.dirty = true;
    }
    get [(_dec201 = [
      property
    ], "color")]() {
      return this._color;
    }
    set color(c) {
      this._color = c;
      this.dirty = true;
    }
    get [(_dec202 = [
      property
    ], "font")]() {
      return this._font;
    }
    set font(f) {
      this._font = f;
      this.dirty = true;
    }
    get [(_dec203 = [
      property
    ], "fontWeight")]() {
      return this._fontWeight;
    }
    set fontWeight(w) {
      this._fontWeight = w;
      this.dirty = true;
    }
    get [(_dec204 = [
      property
    ], "alignment")]() {
      return this._alignment;
    }
    set alignment(a) {
      this._alignment = a;
      this.dirty = true;
    }
    get [(_dec205 = [
      property
    ], "maxWidth")]() {
      return this._maxWidth;
    }
    set maxWidth(w) {
      this._maxWidth = Math.max(0, w);
      this.dirty = true;
    }
    get [(_dec206 = [
      property
    ], "pixelsPerUnit")]() {
      return this._pixelsPerUnit;
    }
    set pixelsPerUnit(p) {
      this._pixelsPerUnit = p;
      this.dirty = true;
    }
    get [(_dec207 = [
      property
    ], "resolution")]() {
      return this._resolution;
    }
    set resolution(r) {
      this._resolution = r;
      this.dirty = true;
    }
    onAwake() {
      this.started = true;
      this.send();
    }
    onPreRender() {
      if (this.dirty)
        this.send();
    }
    onDestroy() {
      __iron_op("render.remove", this.id);
    }
    send() {
      if (!this.started)
        return;
      this.dirty = false;
      const c = this._color;
      __iron_op("render.text", this.id, this.gameObject.id, this._text, [c.r, c.g, c.b, c.a], this._fontSize, this._font, this._fontWeight, this._alignment, this._maxWidth, this._pixelsPerUnit, this._resolution);
    }
  }
  // ../../js/engine/src/app.ts
  var focused = true;
  var paused = false;
  var focusListeners = new Set;
  globalThis.__iron_focus = (hasFocus, isPaused) => {
    const focusChanged = hasFocus !== focused;
    const pauseChanged = isPaused !== paused;
    focused = hasFocus;
    paused = isPaused;
    if (pauseChanged) {
      _forEachActiveComponent("onApplicationPause", (c) => c.onApplicationPause?.(isPaused));
    }
    if (focusChanged) {
      for (const f of [...focusListeners]) {
        try {
          f(hasFocus);
        } catch (e) {
          console.error(`Application.onFocusChanged: ${e}`);
        }
      }
      _forEachActiveComponent("onApplicationFocus", (c) => c.onApplicationFocus?.(hasFocus));
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
  // ../../js/engine/src/gizmos.ts
  var DEG2 = Math.PI / 180;
  var CIRCLE = 48;
  var lines = [];
  function push(a, b) {
    const m = Gizmos.matrix;
    const p = m.multiplyPoint3x4(a);
    const q = m.multiplyPoint3x4(b);
    const c = Gizmos.color;
    lines.push(p.x, p.y, p.z, q.x, q.y, q.z, c.r, c.g, c.b, c.a);
  }
  function basis(normal) {
    const n = normal.normalized;
    const u = n.cross(Math.abs(n.y) < 0.9 ? Vector3.up : Vector3.right).normalized;
    return [u, n.cross(u)];
  }
  var Gizmos = {
    color: Color.white,
    matrix: Matrix4x4.identity,
    drawLine(from, to) {
      push(Vector3.from(from), Vector3.from(to));
    },
    drawRay(from, direction) {
      const f = Vector3.from(from);
      push(f, f.add(direction));
    },
    drawWireCube(center, size) {
      const c = Vector3.from(center);
      const h = Vector3.from(size).scale(0.5);
      const corner = (i) => c.add(new Vector3(i & 1 ? h.x : -h.x, i & 2 ? h.y : -h.y, i & 4 ? h.z : -h.z));
      for (let i = 0;i < 8; i++) {
        for (const bit of [1, 2, 4]) {
          if ((i & bit) === 0)
            push(corner(i), corner(i | bit));
        }
      }
    },
    drawWireDisc(center, normal, radius) {
      const c = Vector3.from(center);
      const [u, v] = basis(Vector3.from(normal));
      let prev = c.add(u.scale(radius));
      for (let i = 1;i <= CIRCLE; i++) {
        const a = i / CIRCLE * Math.PI * 2;
        const p = c.add(u.scale(Math.cos(a) * radius)).add(v.scale(Math.sin(a) * radius));
        push(prev, p);
        prev = p;
      }
    },
    drawWireSphere(center, radius) {
      for (const n of [Vector3.right, Vector3.up, Vector3.back])
        Gizmos.drawWireDisc(center, n, radius);
    },
    drawFrustum(center, fov, maxRange, minRange, aspect) {
      const c = Vector3.from(center);
      const t = Math.tan(fov / 2 * DEG2);
      const rect = (d) => {
        const h = d * t;
        const w = h * aspect;
        return [new Vector3(-w, -h, -d), new Vector3(w, -h, -d), new Vector3(w, h, -d), new Vector3(-w, h, -d)].map((p) => c.add(p));
      };
      const near = rect(minRange);
      const far = rect(maxRange);
      for (let i = 0;i < 4; i++) {
        push(near[i], near[(i + 1) % 4]);
        push(far[i], far[(i + 1) % 4]);
        push(near[i], far[i]);
      }
    },
    get lineCount() {
      return lines.length / 10;
    },
    clear() {
      lines = [];
    },
    present(camera) {
      __iron_op("render.gizmos", camera.id, lines);
      lines = [];
    }
  };
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
  // ../../js/engine/src/window.ts
  var Window = globalThis.Window;
  // src/scene.ts
  var log = [];

  class Spinner extends Component {
    speed = 90;
    onStart() {
      log.push(`spinner start on ${this.gameObject.name}`);
    }
    onUpdate(dt) {
      this.transform.rotate(new Vector3(0, this.speed * dt, 0));
    }
    onDestroy() {
      log.push(`spinner destroyed at ${Math.round(this.transform.eulerAngles.y)}°`);
    }
  }

  class Game extends Component {
    turret;
    onStart() {
      log.push(`game start on ${this.gameObject.name}`);
      this.turret = new GameObject("Turret", { position: new Vector3(0, 2, 0) });
      const barrel = new GameObject("Barrel", { parent: this.turret, position: new Vector3(0, 0, -1) });
      this.turret.addComponent(Spinner, { speed: 90 });
      barrel.transform.localRotation = Quaternion.identity;
    }
    onUpdate() {
      if (Time.frameCount === 31)
        this.turret.enabled = false;
      if (Time.frameCount === 61)
        this.turret.destroy();
    }
  }
  var hooks = [];

  class Lifecycle extends Component {
    onStart() {
      hooks.push("start");
    }
    onFixedUpdate() {
      hooks.push("fixed");
    }
    onUpdate() {
      hooks.push("update");
    }
    onLateUpdate() {
      hooks.push("late");
    }
    onPreRender() {
      hooks.push("prerender");
    }
    onDestroy() {
      hooks.push("destroy");
    }
  }
  var trip = "";
  var _init42, _base42, _dec208, _dec209;

  class Door extends (_base42 = Component) {
    constructor() {
      super(...arguments);
      __runInitializers(_init42, 15, this);
    }
    static {
      _init42 = __decoratorStart(_base42);
      __decorateElement(_init42, 5, "opensTo", _dec208, this);
      __decorateElement(_init42, 5, "speed", _dec209, this);
      __decoratorMetadata(_init42, this);
    }
    [(_dec208 = [
      property
    ], "opensTo")] = __runInitializers(_init42, 8, this, null);
    [(_dec209 = [
      property
    ], "speed")] = (__runInitializers(_init42, 11, this), __runInitializers(_init42, 12, this, 1));
  }

  class SceneTrip extends Component {
    onStart() {
      GameObject.dontDestroyOnLoad(this.gameObject);
      const room = new GameObject("Room", { position: new Vector3(0, 0, -5) });
      const door = new GameObject("Door", { parent: room });
      door.addComponent(Door, { opensTo: room, speed: 3 });
      new GameObject("Lamp").addComponent(Light, { type: "point", range: 7 });
      const json = JSON.stringify(SceneManager.getActiveScene().serialize());
      const scene = SceneManager.loadScene(JSON.parse(json));
      const loaded = GameObject.find("Door").getComponent(Door);
      trip = [
        scene.getRootGameObjects().map((g) => g.name).join("+"),
        loaded.speed,
        loaded.opensTo === GameObject.find("Room"),
        GameObject.find("Lamp").getComponent(Light).range,
        this.gameObject.scene.name
      ].join(" ");
    }
  }
  var starts = 0;
  var reloads = 0;
  var _init43, _base43, _dec210, _dec211;

  class HotCounter extends (_base43 = Component) {
    static {
      _init43 = __decoratorStart(_base43);
      __decorateElement(_init43, 5, "speed", _dec210, this);
      __decorateElement(_init43, 5, "label", _dec211, this);
      __decoratorMetadata(_init43, this);
    }
    [(_dec210 = [
      property
    ], "speed")] = __runInitializers(_init43, 8, this, 1);
    [(_dec211 = [
      property
    ], "label")] = (__runInitializers(_init43, 11, this), __runInitializers(_init43, 12, this, "fresh"));
    count = (__runInitializers(_init43, 15, this), 0);
    child = null;
    onStart() {
      starts++;
      this.speed = 42;
      this.label = "started";
      this.child = new GameObject("HotChild", { parent: this.gameObject, position: new Vector3(1, 2, 3) });
    }
    onUpdate() {
      this.count++;
    }
    onReload() {
      reloads++;
    }
  }
  register({ Game, Lifecycle, Door, SceneTrip, HotCounter });
  globalThis.__hot_spawn = (n) => {
    for (let i = 0;i < n; i++) {
      const go = new GameObject(`Counter ${i}`, { position: new Vector3(i, 0, 0) });
      const c = go._addComponentAsleep(HotCounter, { speed: i, count: i });
      c.child = new GameObject(`Child ${i}`, { parent: go });
    }
  };
  globalThis.__hot_log = () => {
    const c = GameObject.findObjectOfType(HotCounter);
    return [c.speed, c.label, c.count, c.child === GameObject.find("HotChild"), starts, reloads, Time.frameCount].join(" ");
  };
  globalThis.__scene_trip = () => trip;
  globalThis.__hooks_log = () => hooks.join(" ");
  globalThis.__scene_log = () => log.join(", ");
})();
