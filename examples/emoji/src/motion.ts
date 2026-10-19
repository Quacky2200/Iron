// How an emoji moves. A `Gait` is an entrance (the same motion played backwards is its exit);
// a pose is what it does under the spotlight. Both are pure functions of time that return a
// `Move`: the director turns that into transforms, shadows, sparks and curtain moves.
//
// Squash and stretch keep their volume (sx grows as sy shrinks) and stay modest, so nothing
// ever looks flattened. Rotation is "lean forward": the director flips it to the way the
// emoji is heading.
import type { PoseName } from "./cast.ts";
import { GATHER, HEIGHT, view } from "./config.ts";

export interface Move {
  /** Progress along the way in from the edge (0) to the centre (1). Ignored when `x` is set. */
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
  /** 0..1 as it hits the floor: the director puffs dust. */
  thump: number;
  /** True for a moment as it appears in a burst of confetti. */
  puff: boolean;
  fx?: { kind: "sparks" | "dust" | "streaks"; rate: number };
}

export const still = (): Move => ({ d: 1, y: 0, sx: 1, sy: 1, rot: 0, spin: 1, centre: 0, alpha: 1, thump: 0, puff: false });

const clamp01 = (v: number) => Math.min(1, Math.max(0, v));
const smooth = (t: number) => {
  const c = clamp01(t);
  return c * c * (3 - 2 * c);
};
const lerp = (a: number, b: number, t: number) => a + (b - a) * t;
/** 0 up to 1 and back to 0: ramps over `edge` of the time at each end. */
const envelope = (u: number, edge = 0.15) => smooth(u / edge) * (1 - smooth((u - (1 - edge)) / edge));

/** Cosine-eased keyframes: [time, value]. */
function keys(t: number, frames: [number, number][]): number {
  const first = frames[0]!;
  if (t <= first[0]) return first[1];
  for (let i = 1; i < frames.length; i++) {
    const [t1, v1] = frames[i]!;
    const [t0, v0] = frames[i - 1]!;
    if (t <= t1) return lerp(v0, v1, smooth((t - t0) / (t1 - t0)));
  }
  return frames[frames.length - 1]![1];
}

export interface Gait {
  name: string;
  dur: number;
  at(t: number): Move;
}

/** Steps along the floor, a waddle with a slow stop. */
const walk: Gait = {
  name: "walk",
  dur: 7,
  at(t) {
    const m = still();
    const u = clamp01(t / 7);
    m.d = 1 - (1 - u) ** 1.6;
    const amp = clamp01((1 - u) * 5);
    const step = Math.sin((m.d * 14 * Math.PI) / 1.15);
    m.rot = (3 + 7 * step) * amp;
    m.y = 0.12 * Math.abs(step) * amp;
    m.sy = 1 - 0.03 * Math.abs(step) * amp;
    m.sx = 1 + 0.02 * Math.abs(step) * amp;
    return m;
  },
};

/** Seven hops; it only moves while it's in the air. */
const hop: Gait = {
  name: "hop",
  dur: 5.25,
  at(t) {
    const N = 7;
    const T = 0.75;
    const AIR = 0.7;
    const m = still();
    const h = Math.min(Math.floor(t / T), N - 1);
    const f = clamp01((t - h * T) / T);
    let travel = 1;
    if (f < AIR) {
      const a = f / AIR;
      const lift = 4 * a * (1 - a);
      travel = smooth(a);
      m.y = 1.1 * lift;
      m.sy = 1 + 0.06 * lift;
      m.sx = 1 - 0.03 * lift;
      m.rot = 6 * Math.cos(Math.PI * a);
    } else {
      const g = (f - AIR) / (1 - AIR);
      const squash = Math.sin(Math.PI * g);
      m.sy = 1 - 0.14 * squash;
      m.sx = 1 + 0.08 * squash;
      m.thump = squash;
    }
    m.d = (h + travel) / N;
    if (t >= N * T) m.d = 1;
    return m;
  },
};

/** Dashes in leaning forward with speed lines, then skids to a stop in a cloud of dust. */
const rush: Gait = {
  name: "rush",
  dur: 1.7,
  at(t) {
    const m = still();
    const u = clamp01(t / 1.7);
    m.d = 1 - (1 - u) ** 3;
    const sp = (1 - u) ** 2;
    const brake = Math.sin(Math.PI * clamp01((u - 0.55) / 0.45));
    m.rot = 24 * sp - 8 * brake;
    m.sx = 1 + 0.1 * sp + 0.05 * brake;
    m.sy = 1 - 0.05 * sp - 0.08 * brake;
    m.y = 0.05 * Math.sin(t * 45) * sp;
    if (sp > 0.2) m.fx = { kind: "streaks", rate: 70 * sp };
    else if (u > 0.5 && u < 0.97) m.fx = { kind: "dust", rate: 40 * brake };
    return m;
  },
};

/** Drags itself along the floor on its belly, throwing sparks, then pushes up onto its feet. */
const crawl: Gait = {
  name: "crawl",
  dur: 8.5,
  at(t) {
    const m = still();
    const u = clamp01(t / 8.5);
    m.d = 1 - (1 - u) ** 1.25;
    const rise = smooth((u - 0.8) / 0.2);
    const k = 1 - rise;
    const w = Math.sin((m.d * 14 * 2 * Math.PI) / 0.8);
    m.sy = lerp(1, 0.66 - 0.04 * w, k) + 0.05 * Math.sin(Math.PI * rise);
    m.sx = lerp(1, 1.14 + 0.08 * w, k) - 0.03 * Math.sin(Math.PI * rise);
    m.rot = (11 + 4 * w) * k;
    if (k > 0.4) m.fx = { kind: "sparks", rate: 90 * k };
    return m;
  },
};

/** Hides behind the half-drawn curtain, peeks out twice, then comes out for good. */
const peek: Gait = {
  name: "peek",
  dur: 6,
  at(t) {
    const m = still();
    const gap = view.half * GATHER * 0.28;
    const hidden = -(gap + HEIGHT / 2 + 0.3);
    m.x = keys(t, [
      [0, hidden], [0.8, hidden], [1.7, hidden + 1.5], [2.3, hidden + 1.5], [2.9, hidden],
      [3.5, hidden], [4.2, hidden + 2.4], [4.9, hidden + 2.4], [6, 0],
    ]);
    m.open = keys(t, [[0, 0.28], [4.9, 0.28], [5.8, 1]]);
    m.rot = keys(t, [[0, 0], [1.7, 7], [2.3, 7], [2.9, 0], [4.2, 12], [4.9, 10], [6, 0]]);
    m.y = 0.9 * Math.sin(Math.PI * clamp01((t - 4.9) / 1.1));
    return m;
  },
};

/** Falls from above, lands flat and bounces a few times. */
const drop: Gait = {
  name: "drop",
  dur: 2.6,
  at(t) {
    const m = still();
    m.x = 0;
    const fall = 0.55;
    if (t < fall) {
      m.y = 9 * (1 - (t / fall) ** 2);
      m.sy = 1.15;
      m.sx = 0.92;
      return m;
    }
    const tau = t - fall;
    const decay = Math.exp(-2.2 * tau);
    m.y = 2 * Math.abs(Math.sin((Math.PI * tau) / 0.7)) * decay;
    const g = clamp01(1 - m.y / 0.35) * decay;
    m.sy = 1 - 0.3 * g + (m.y > 0.5 ? 0.05 : 0);
    m.sx = 1 + 0.2 * g;
    m.thump = g;
    return m;
  },
};

/** Rolls in like a wheel and rocks to a stop. */
const roll: Gait = {
  name: "roll",
  dur: 3.6,
  at(t) {
    const m = still();
    const u = clamp01(t / 3.6);
    m.d = 1 - (1 - u) ** 1.8;
    m.rot = 720 * m.d;
    m.centre = 1;
    if (u < 0.9) m.fx = { kind: "dust", rate: 22 * (1 - u) };
    return m;
  },
};

/** Appears where it stands in a burst of confetti, growing with a springy overshoot. */
const pop: Gait = {
  name: "pop",
  dur: 1.4,
  at(t) {
    const m = still();
    m.x = 0;
    const u = clamp01(t / 0.9);
    const c = 2.4;
    const s = 1 + (c + 1) * (u - 1) ** 3 + c * (u - 1) ** 2;
    m.sx = s;
    m.sy = s;
    m.alpha = clamp01(t / 0.12);
    m.rot = 12 * Math.sin(t * 14) * Math.exp(-3 * t);
    m.puff = t < 0.07;
    return m;
  },
};

/** Floats up from below the stage, swaying. */
const rise: Gait = {
  name: "rise",
  dur: 4.2,
  at(t) {
    const m = still();
    m.x = 0;
    const u = clamp01(t / 4.2);
    m.y = -(HEIGHT + 1.4) * (1 - smooth(u));
    m.rot = 8 * Math.sin(t * 3.6) * (1 - u);
    m.sy = 1 - 0.07 * Math.sin(Math.PI * clamp01((u - 0.8) / 0.2));
    return m;
  },
};

export const GAITS: Gait[] = [walk, hop, rush, crawl, peek, drop, roll, pop, rise];

export const POSE_TIME = 3.8;

/** What it does in the spotlight; `t` runs 0..POSE_TIME and every pose ends at rest. */
export function poseMove(name: PoseName, t: number): Move {
  const m = still();
  const u = clamp01(t / POSE_TIME);
  const env = envelope(u, 0.15);
  switch (name) {
    case "bow": {
      const bow = smooth(u / 0.25) * (1 - smooth((u - 0.7) / 0.25));
      m.rot = 22 * bow;
      m.sy = 1 - 0.1 * bow;
      m.sx = 1 + 0.04 * bow;
      break;
    }
    case "wink":
      // A wink is a quick swap to the winking face, with a nod each time.
      if ((t > 0.7 && t < 1.1) || (t > 2.3 && t < 2.7)) m.swap = "1f609";
      m.rot = 4 * Math.sin(Math.PI * 2 * u * 2) * env;
      m.sy = 1 - 0.03 * Math.abs(Math.sin(Math.PI * u * 4)) * env;
      break;
    case "spin": {
      // Hops up and turns about its own axis twice, like a coin in the air.
      const e = smooth(u / 0.85);
      m.centre = 1;
      m.y = 1.6 * Math.sin(Math.PI * e);
      m.spin = Math.max(0.03, Math.abs(Math.cos(Math.PI * 4 * e)));
      const land = Math.sin(Math.PI * clamp01((u - 0.85) / 0.12));
      m.sy = 1 - 0.1 * land;
      m.thump = land;
      break;
    }
    case "pump": {
      const beat = Math.abs(Math.sin(Math.PI * 3 * u)) * env;
      m.y = 0.4 * beat;
      m.sy = 1 + 0.08 * beat;
      m.sx = 1 - 0.04 * beat;
      break;
    }
    case "sway":
      m.rot = 10 * Math.sin(Math.PI * 2 * 1.4 * u) * env;
      break;
    case "shake":
      m.rot = 7 * Math.sin(Math.PI * 2 * 11 * u) * env;
      m.sx = 1 + 0.03 * Math.sin(Math.PI * 2 * 11 * u + 1) * env;
      break;
    case "float":
      m.centre = 1;
      m.y = (0.5 + 0.35 * Math.sin(Math.PI * 2 * 1.1 * t)) * env;
      m.rot = 5 * Math.sin(Math.PI * 2 * 0.6 * t) * env;
      break;
    case "flip": {
      const e = smooth((u - 0.1) / 0.5);
      m.centre = 1;
      m.y = 1.6 * Math.sin(Math.PI * e);
      m.rot = 360 * e;
      const land = Math.sin(Math.PI * clamp01((u - 0.6) / 0.14));
      m.sy = 1 - 0.1 * land;
      m.thump = land;
      break;
    }
    case "dizzy":
      m.centre = 1;
      m.rot = 12 * Math.sin(Math.PI * 2 * 1.3 * t) * env;
      m.x = 0.35 * Math.sin(Math.PI * 2 * 0.9 * t) * env;
      m.y = 0.1 * Math.sin(Math.PI * 2 * 1.6 * t) * env;
      break;
    case "watch":
      m.rot = 3 * Math.sin(Math.PI * 2 * 0.5 * t) * env;
      m.sy = 1 + 0.015 * Math.sin(Math.PI * 2 * 0.9 * t);
      break;
  }
  return m;
}
