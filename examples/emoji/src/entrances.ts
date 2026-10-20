// Ways on to the stage. Each is an `Act` over time `t`, ending still at the centre (apart from
// `rollcrash`, which misses the show).
import { GATHER, GROUND, HEIGHT, view } from "./config.ts";
import { type Act, clamp01, keys, lerp, line, smooth, still } from "./moves.ts";

const walk: Act = {
  name: "walk",
  dur: 7,
  weight: 10,
  at(t) {
    const m = still();
    const u = clamp01(t / 7);
    m.d = 1 - (1 - u) ** 1.6;
    const amp = clamp01((1 - u) * 5);
    m.stride = (m.d * 14) / 1.15;
    const step = Math.sin(m.stride * Math.PI);
    m.rot = (3 + 7 * step) * amp;
    m.y = 0.12 * Math.abs(step) * amp;
    m.sy = 1 - 0.03 * Math.abs(step) * amp;
    m.sx = 1 + 0.02 * Math.abs(step) * amp;
    return m;
  },
};

/** Seven hops; it only moves while it's in the air. */
const hop: Act = {
  name: "hop",
  dur: 5.25,
  weight: 10,
  land: "boing",
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
const rush: Act = {
  name: "rush",
  dur: 1.7,
  weight: 8,
  cues: [
    [0, (c) => c.sfx("whoosh", 0.8)],
    [0.1, (c) => c.sfx("scamper", 0.5, 1.4)],
    [0.95, (c) => c.sfx("skid", 0.9)],
  ],
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
const crawl: Act = {
  name: "crawl",
  dur: 8.5,
  weight: 6,
  // Two scrapes back to back, ended as it pushes itself up onto its feet (u = 0.8, at 6.8 s).
  cues: [
    [0.3, (c) => c.sfx("drag", 0.8)],
    [3.6, (c) => c.sfx("drag", 0.8)],
    [6.8, (c) => c.stop("drag", 0.25)],
  ],
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
const peek: Act = {
  name: "peek",
  dur: 6,
  weight: 8,
  cues: [[4.9, (c) => c.sfx("whoosh", 0.4, 1.2)]],
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
const drop: Act = {
  name: "drop",
  dur: 2.6,
  weight: 6,
  cues: [[0, (c) => c.sfx("slide_down", 0.5, 1.8)]],
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

/** Falls from above and lands with a single heavy squash, no bounce. */
const plop: Act = {
  name: "plop",
  dur: 2.2,
  weight: 6,
  cues: [[0, (c) => c.sfx("slide_down", 0.5, 1.9)], [0.58, (c) => c.shake(0.05, 0.25)]],
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
    const s = Math.exp(-4 * tau) * Math.cos(11 * tau);
    m.sy = 1 - 0.3 * s;
    m.sx = 1 + 0.2 * s;
    m.thump = clamp01(s);
    return m;
  },
};

/** Rolls in like a wheel and rocks to a stop. */
const roll: Act = {
  name: "roll",
  dur: 3.6,
  weight: 6,
  cues: [[0, (c) => c.sfx("whoosh", 0.6)], [0.2, (c) => c.sfx("scamper", 0.4, 0.7)]],
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

/** Appears in a poof of smoke, growing with a springy overshoot. */
const pop: Act = {
  name: "pop",
  dur: 1.4,
  weight: 6,
  cues: [[0, (c) => {
    // It stands at x = 0, and the director hasn't placed it yet on this first frame.
    c.sfx("poof", 1);
    c.smoke(0, GROUND + HEIGHT / 2, 34, [0.88, 0.88, 0.9, 0.75], 2.2);
    c.smoke(0, GROUND + HEIGHT / 2 - 1.2, 18, [0.7, 0.7, 0.74, 0.7], 2.6);
  }]],
  at(t) {
    const m = still();
    m.x = 0;
    const u = clamp01(t / 0.9);
    const c = 2.4;
    const s = 1 + (c + 1) * (u - 1) ** 3 + c * (u - 1) ** 2;
    m.sx = s;
    m.sy = s;
    m.alpha = clamp01(t / 0.08);
    m.rot = 12 * Math.sin(t * 14) * Math.exp(-3 * t);
    return m;
  },
};

/** Floats up from below the stage, swaying. */
const rise: Act = {
  name: "rise",
  dur: 4.2,
  weight: 6,
  cues: [[0, (c) => c.sfx("whoosh", 0.4, 0.5)], [3.2, (c) => c.sfx("sparkle", 0.5, 0.8)]],
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

/** Fades in where it stands, drifting down into place. */
const fade: Act = {
  name: "fade",
  dur: 2.4,
  weight: 7,
  cues: [[0.15, (c) => c.sfx("sparkle", 0.8)]],
  at(t) {
    const m = still();
    m.x = 0;
    m.alpha = smooth(t / 1.2);
    const k = smooth(t / 2.2);
    m.y = 0.6 * (1 - k);
    m.sx = lerp(0.94, 1, k);
    m.sy = lerp(0.94, 1, k);
    return m;
  },
};

/** Runs in, overshoots, skids, runs back the other way, again, and finally skids into the middle. */
const scramble: Act = {
  name: "scramble",
  dur: 6,
  weight: 7,
  cues: [
    [0, (c) => c.sfx("scamper", 0.8)],
    [1.2, (c) => c.sfx("skid", 0.8)], [1.45, (c) => c.sfx("scamper", 0.8)],
    [2.6, (c) => c.sfx("skid", 0.7)], [2.85, (c) => c.sfx("scamper", 0.8)],
    [3.8, (c) => c.sfx("skid", 0.6)], [4.05, (c) => c.sfx("scamper", 0.7)],
    [4.9, (c) => c.sfx("skid", 0.6)], [5.2, (c) => c.sfx("scamper", 0.6)],
    [5.85, (c) => c.sfx("skid", 0.5, 1.2)],
  ],
  at(t) {
    const m = still();
    const edge = view.half + HEIGHT / 2 + 0.8;
    const stops = [1.2, 2.6, 3.8, 4.9, 5.9];
    const path: [number, number][] = [
      [0, -edge], [1.2, 4], [1.45, 4], [2.6, -3.5], [2.85, -3.5], [3.8, 2.2], [4.05, 2.2], [4.9, -0.9], [5.2, -0.9], [5.9, 0], [6, 0],
    ];
    m.x = line(t, path);
    const v = (line(t + 0.02, path) - line(t - 0.02, path)) / 0.04;
    // Brake into each stop, then lean into the next run.
    let brake = 0;
    for (const s of stops) brake = Math.max(brake, Math.sin(Math.PI * clamp01((t - (s - 0.28)) / 0.5)));
    m.rot = Math.min(22, Math.abs(v) * 1.5) - 16 * brake;
    m.sx = 1 + 0.08 * brake;
    m.sy = 1 - 0.08 * brake;
    m.y = 0.07 * Math.abs(Math.sin(t * 30)) * clamp01(Math.abs(v) / 3);
    if (brake > 0.2) m.fx = { kind: "dust", rate: 60 * brake };
    return m;
  },
};

/** Rolls straight across the stage too fast to stop and crashes off the far side: it misses its own show. */
const rollcrash: Act = {
  name: "rollcrash",
  dur: 3.6,
  weight: 2,
  miss: true,
  cues: [
    [0, (c) => c.sfx("whoosh", 0.8)],
    [0.2, (c) => c.sfx("scamper", 0.5, 0.8)],
    [2.55, (c) => {
      c.sfx("crash", 1);
      c.shake(0.3, 0.8);
    }],
    [2.75, (c) => c.sfx("slide_down", 0.4, 1.5)],
  ],
  at(t) {
    const m = still();
    const u = clamp01(t / 2.7);
    m.d = 2 * u ** 1.3;
    m.rot = 900 * (m.d / 2);
    m.centre = 1;
    if (u < 1) m.fx = { kind: "dust", rate: 25 };
    return m;
  },
};

export const ENTRANCES: Act[] = [walk, hop, rush, crawl, peek, drop, plop, roll, pop, rise, fade, scramble, rollcrash];
