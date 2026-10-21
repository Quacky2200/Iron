// Signature poses: what each emoji does when it's on stage that nobody else would. Tears for the
// laughing face, a pickaxe for the diamond, a barking dog with a bone, a bulb for the thinker.
// Each is an `Act` (motion plus cues) built from small helpers, using sprite particles (Noto
// emoji flying about), line particles, tints and the sounds in assets/sfx.
import { cracks } from "./hud.tsx";
import { GROUND, HEIGHT } from "./config.ts";
import { type Act, clamp01, type Ctx, type Cue, envelope, keys, lerp, type Move, smooth, still } from "./moves.ts";
import { POSES } from "./poses.ts";
import type { Sfx } from "./sfx.ts";

const R = HEIGHT;
const rnd = (a: number, b: number) => a + Math.random() * (b - a);
const RAD = Math.PI / 180;

interface P {
  x: number;
  y: number;
}
const head = (c: Ctx): P => ({ x: c.at().x, y: c.at().y + R * 0.46 });
const eyes = (c: Ctx): [P, P] => {
  const p = c.at();
  return [{ x: p.x - R * 0.2, y: p.y + R * 0.08 }, { x: p.x + R * 0.2, y: p.y + R * 0.08 }];
};
const mouth = (c: Ctx): P => ({ x: c.at().x, y: c.at().y - R * 0.12 });
const feet = (c: Ctx): P => ({ x: c.at().x, y: GROUND + 0.2 });

interface Spray {
  /** Degrees: 0 is right, 90 is up. */
  angle?: [number, number];
  speed?: [number, number];
  gravity?: number;
  scale?: number | [number, number];
  life?: [number, number];
  spin?: number;
  drag?: number;
  sway?: number;
  z?: number;
}

/** Throws `n` sprites of `code` out of a point. */
function spray(c: Ctx, code: string, n: number, from: P, o: Spray = {}): void {
  for (let i = 0; i < n; i++) {
    const a = rnd(...(o.angle ?? [0, 360])) * RAD;
    const s = rnd(...(o.speed ?? [2, 5]));
    c.scene.sprites.emit({
      code,
      x: from.x + rnd(-0.1, 0.1),
      y: from.y + rnd(-0.1, 0.1),
      vx: Math.cos(a) * s,
      vy: Math.sin(a) * s,
      gravity: o.gravity ?? 0,
      drag: o.drag,
      spin: o.spin === undefined ? undefined : rnd(-o.spin, o.spin),
      scale: o.scale ?? 0.4,
      life: rnd(...(o.life ?? [1, 1.6])),
      sway: o.sway,
      z: o.z,
    });
  }
}

/** Sprites that drift up from around a point. */
function drift(c: Ctx, code: string, n: number, from: P, o: { width?: number; rise?: [number, number]; scale?: number | [number, number]; life?: [number, number]; sway?: number; spin?: number } = {}): void {
  for (let i = 0; i < n; i++) {
    c.scene.sprites.emit({
      code,
      x: from.x + rnd(-(o.width ?? 1), o.width ?? 1),
      y: from.y + rnd(-0.2, 0.2),
      vy: rnd(...(o.rise ?? [1, 2])),
      scale: o.scale ?? 0.45,
      life: rnd(...(o.life ?? [1.8, 2.6])),
      sway: o.sway ?? 0.3,
      spin: o.spin === undefined ? undefined : rnd(-o.spin, o.spin),
    });
  }
}

/** Things raining down from the top of the stage over the emoji. */
function rain(c: Ctx, code: string, n: number, o: { scale?: number; spin?: number; gravity?: number } = {}): void {
  for (let i = 0; i < n; i++) {
    c.scene.sprites.emit({
      code,
      x: c.at().x + rnd(-4.5, 4.5),
      y: rnd(4.5, 6.5),
      vy: -rnd(1, 3),
      gravity: o.gravity ?? 3,
      scale: o.scale ?? 0.45,
      spin: rnd(-(o.spin ?? 200), o.spin ?? 200),
      sway: 0.3,
      life: 2.6,
    });
  }
}

const times = (list: number[], run: (c: Ctx, i: number) => void): Cue[] => list.map((t, i): Cue => [t, (c) => run(c, i)]);
const sound = (t: number, name: Sfx, volume = 1, rate = 1): Cue => [t, (c) => c.sfx(name, volume, rate)];

/** An act from a motion function, which fills in a move for time `t` (`u` is 0..1 through it). */
function act(name: string, dur: number, motion: (m: Move, t: number, u: number) => void, cues: Cue[] = []): Act {
  return {
    name,
    dur,
    cues,
    at(t) {
      const m = still();
      motion(m, t, clamp01(t / dur));
      return m;
    },
  };
}

/** An existing pose with more going on. */
const withCues = (base: Act, extra: Cue[], name: string): Act => ({ ...base, name, cues: [...(base.cues ?? []), ...extra] });

const tintTo = (m: Move, r: number, g: number, b: number, k: number): void => {
  m.tint = [lerp(1, r, k), lerp(1, g, k), lerp(1, b, k)];
};
const shiver = (m: Move, t: number, deg: number, hz: number, k = 1): void => {
  m.rot = deg * Math.sin(Math.PI * 2 * hz * t) * k;
};

// ---- Faces ----

const tearsSpray = (c: Ctx): void => {
  const [l, r] = eyes(c);
  spray(c, "1f4a7", 4, l, { angle: [100, 160], speed: [3, 6], gravity: 13, scale: [0.3, 0.4], life: [0.9, 1.2] });
  spray(c, "1f4a7", 4, r, { angle: [20, 80], speed: [3, 6], gravity: 13, scale: [0.3, 0.4], life: [0.9, 1.2] });
};

const laughing = (m: Move, t: number, u: number): void => {
  const env = envelope(u, 0.1);
  shiver(m, t, 7, 7, env);
  m.y = 0.25 * Math.abs(Math.sin(Math.PI * 3.5 * t)) * env;
  m.sy = 1 - 0.04 * Math.abs(Math.sin(Math.PI * 3.5 * t)) * env;
};

const tears = act("tears", 4, laughing, [sound(0.1, "laugh"), ...times([0.2, 0.7, 1.2, 1.7, 2.2, 2.7], (c) => tearsSpray(c))]);
const rofl = withCues(POSES.flip, [sound(0.1, "laugh"), ...times([0.3, 0.9, 1.5, 2.1], (c) => tearsSpray(c))], "rofl");

const heartEyes = withCues(POSES.beat, [sound(0.3, "ooh", 0.6), ...times([0.3, 1.2, 2.2], (c) => {
  const [l, r] = eyes(c);
  spray(c, "2764", 3, l, { angle: [90, 160], speed: [2, 4], gravity: -0.5, scale: [0.25, 0.5], life: [1.1, 1.6], spin: 120 });
  spray(c, "2764", 3, r, { angle: [20, 90], speed: [2, 4], gravity: -0.5, scale: [0.25, 0.5], life: [1.1, 1.6], spin: 120 });
})], "heartpop");

const sobbing = act("sobbing", 4.4, (m, t, u) => {
  const env = envelope(u, 0.1);
  shiver(m, t, 4, 5, env);
  m.y = 0.12 * Math.abs(Math.sin(Math.PI * 2.5 * t)) * env;
}, [sound(0.1, "sob"), ...times(Array.from({ length: 14 }, (_, i) => 0.2 + i * 0.28), (c) => tearsSpray(c))]);

const angry = act("angry", 3.8, (m, t, u) => {
  const env = envelope(u, 0.1);
  shiver(m, t, 5, 15, env);
  m.sx = 1 + 0.03 * Math.sin(t * 70) * env;
  tintTo(m, 1.25, 0.65, 0.6, env * (0.6 + 0.4 * Math.sin(t * 9)));
}, [
  sound(0.2, "growl"),
  ...times([0.3, 1.3, 2.3], (c) => {
    const p = head(c);
    for (const dx of [-1.3, 1.3]) {
      c.scene.sprites.emit({ code: "1f4a2", x: p.x + dx, y: p.y - 0.4, life: 1.0, anim: (t) => ({ x: p.x + dx, y: p.y - 0.4 + 0.5 * t, scale: 0.9 * (1 - Math.exp(-t * 12) * Math.cos(t * 25)), rot: dx * 8, alpha: 1 - clamp01((t - 0.7) / 0.3) }) });
    }
    c.smoke(p.x - 0.6, p.y - 0.2, 7, [0.9, 0.8, 0.8, 0.5], 1.2);
    c.smoke(p.x + 0.6, p.y - 0.2, 7, [0.9, 0.8, 0.8, 0.5], 1.2);
  }),
]);

const swearing = act("swearing", 3.8, (m, t, u) => {
  const env = envelope(u, 0.1);
  shiver(m, t, 6, 13, env);
  tintTo(m, 1.2, 0.7, 0.65, env);
}, [
  ...times([0.4, 1.0, 1.7], (c, i) => {
    c.sfx("bleep", 0.8, 1 + 0.1 * i);
    const p = head(c);
    spray(c, "1f4a2", 1, p, { angle: [60, 120], speed: [1, 2], scale: [0.5, 0.8], life: [0.7, 1] });
  }),
]);

const scream = act("scream", 3.6, (m, t, u) => {
  const env = envelope(u, 0.08);
  const b = Math.abs(Math.sin(Math.PI * 5 * t)) * env;
  m.sx = 1 + 0.06 * b;
  m.sy = 1 + 0.08 * b;
  shiver(m, t, 3, 18, env);
}, [
  [0.1, (c) => {
    c.sfx("scream", 1);
    c.shake(0.08, 1);
    const p = head(c);
    spray(c, "2757", 4, p, { angle: [50, 130], speed: [3, 6], gravity: 4, scale: [0.4, 0.7], life: [0.9, 1.3], spin: 60 });
  }],
]);

const cold = act("cold", 4, (m, t, u) => {
  const env = envelope(u, 0.15);
  shiver(m, t, 3.5, 17, env);
  tintTo(m, 0.65, 0.88, 1.05, env);
}, [sound(0.2, "chatter"), sound(1.6, "chatter"), ...times([0.1, 0.5, 0.9, 1.3, 1.7, 2.1, 2.5], (c) => rain(c, "2744", 5, { scale: 0.4, gravity: 1.2, spin: 120 }))]);

const sick = act("sick", 4, (m, t, u) => {
  const env = envelope(u, 0.15);
  tintTo(m, 0.8, 1.05, 0.7, env);
  m.rot = t > 1 && t < 2.2 ? 9 + 3 * Math.sin(t * 60) : 3 * Math.sin(t * 3) * env;
}, [
  sound(0.9, "gulp"),
  [1.2, (c) => {
    c.sfx("squirt", 0.8, 0.6);
    c.sfx("splat", 0.8, 0.8);
    const p = mouth(c);
    c.smoke(p.x + 1.2, p.y - 0.8, 24, [0.55, 0.85, 0.25, 0.75], 1.6);
  }],
]);

const sleeping = act("sleeping", 5, (m, t) => {
  m.sy = 1 + 0.03 * Math.sin(Math.PI * 2 * 0.45 * t);
  m.sx = 1 - 0.015 * Math.sin(Math.PI * 2 * 0.45 * t);
  m.rot = 2 * Math.sin(Math.PI * 2 * 0.2 * t);
}, [sound(0.4, "snore", 0.8), sound(2.7, "snore", 0.8, 0.95), ...times([0.5, 1.4, 2.3, 3.2, 4.1], (c) => drift(c, "1f4a4", 1, { x: head(c).x + 1.2, y: head(c).y - 0.3 }, { width: 0.2, scale: [0.4, 0.9], life: [2.4, 3], rise: [0.9, 1.3], sway: 0.35 }))]);

const thinking = act("thinking", 4.4, (m, t, u) => {
  m.rot = 5 * Math.sin(Math.PI * 2 * 0.4 * t) * envelope(u, 0.15);
}, [
  [0.4, (c) => {
    c.sfx("hum", 0.7);
    const p = head(c);
    c.scene.sprites.emit({ code: "1f4ad", x: p.x + 1.8, y: p.y + 0.6, life: 2.3, anim: (t) => ({ x: p.x + 1.8, y: p.y + 0.6 + 0.1 * Math.sin(t * 3), scale: 1.7 * (1 - Math.exp(-t * 8) * Math.cos(t * 14)), alpha: 1 - clamp01((t - 1.9) / 0.4) }) });
  }],
  ...times([1.0, 1.5], (c) => drift(c, "2753", 1, { x: head(c).x + 1.8, y: head(c).y + 0.7 }, { width: 0.3, scale: 0.5, life: [1, 1.4], rise: [1, 1.4] })),
  [2.4, (c) => {
    c.sfx("ting", 1);
    const p = head(c);
    c.scene.sprites.emit({ code: "1f4a1", x: p.x + 1.8, y: p.y + 0.6, life: 1.9, anim: (t) => ({ x: p.x + 1.8, y: p.y + 0.6, scale: 1.2 * (1 - Math.exp(-t * 10) * Math.cos(t * 18)), alpha: 1 - clamp01((t - 1.5) / 0.4) }) });
    spray(c, "2728", 6, { x: p.x + 1.8, y: p.y + 0.6 }, { speed: [1, 3], scale: [0.2, 0.4], life: [0.5, 0.9] });
  }],
]);

const mindBlown = act("mindblown", 4, (m, t) => {
  if (t < 0.8) shiver(m, t, 3, 20, t / 0.8);
  else {
    m.sy = 1 + 0.06 * Math.exp(-(t - 0.8) * 3);
    shiver(m, t, 5, 12, Math.exp(-(t - 0.8) * 2));
  }
}, [
  [0.8, (c) => {
    c.sfx("boom", 0.9, 1.2);
    c.shake(0.2, 0.7);
    const p = head(c);
    c.burst(p.x, p.y, 40);
    spray(c, "1f4a5", 1, p, { speed: [0, 0], scale: [1.2, 2.2], life: [0.5, 0.6] });
    spray(c, "1f525", 6, p, { angle: [60, 120], speed: [3, 6], gravity: 2, scale: [0.4, 0.7], life: [0.7, 1.1] });
    c.smoke(p.x, p.y, 14, [0.5, 0.5, 0.55, 0.6], 1.6);
  }],
]);

const devil = act("devil", 4, (m, t, u) => {
  const env = envelope(u, 0.15);
  tintTo(m, 1.15, 0.65, 0.6, env);
  shiver(m, t, 3, 9, env);
}, [
  sound(0.2, "evil", 0.9),
  ...times(Array.from({ length: 12 }, (_, i) => 0.2 + i * 0.3), (c) => {
    const p = feet(c);
    for (const dx of [-2.2, 2.2]) drift(c, "1f525", 1, { x: p.x + dx, y: p.y }, { width: 0.5, scale: [0.5, 0.9], life: [1.1, 1.5], rise: [1.5, 2.5], sway: 0.15 });
  }),
]);

const ghost = act("ghost", 4.4, (m, t, u) => {
  const env = envelope(u, 0.15);
  m.centre = 1;
  m.y = (0.4 + 0.3 * Math.sin(Math.PI * 2 * 0.8 * t)) * env;
  m.alpha = 0.6 + 0.4 * Math.abs(Math.sin(Math.PI * 2 * 0.5 * t));
  tintTo(m, 0.85, 0.9, 1.1, env);
}, [sound(0.1, "wail", 0.9)]);

const skull = act("skull", 3.8, (m, t, u) => {
  const env = envelope(u, 0.1);
  m.sy = 1 - 0.04 * (Math.sin(Math.PI * 2 * 5 * t) > 0 ? 1 : 0) * env;
  shiver(m, t, 2, 9, env);
}, [sound(0.2, "bones"), sound(1.8, "bones", 0.9, 1.1)]);

const clown = act("clown", 4, (m, t, u) => {
  const env = envelope(u, 0.1);
  m.y = 0.5 * Math.abs(Math.sin(Math.PI * 1.6 * t)) * env;
  m.rot = 8 * Math.sin(Math.PI * 2 * 0.8 * t) * env;
}, [
  [0.3, (c) => {
    c.sfx("honk", 1);
    c.burst(c.at().x, c.at().y, 30);
  }],
  ...times([1.0, 1.3, 1.6, 2.4], (c, i) => c.sfx("squeak", 0.9, 0.9 + 0.15 * i)),
  [2.4, (c) => c.burst(c.at().x, c.at().y, 30)],
]);

const robot = act("robot", 4, (m, t, u) => {
  const env = envelope(u, 0.1);
  const q = Math.floor(t * 6) / 6;
  m.rot = 14 * Math.sin(Math.PI * 2 * 0.6 * q) * env;
  m.y = 0.15 * (Math.floor(t * 3) % 2) * env;
  m.sx = 1 + 0.03 * (Math.floor(t * 4) % 2) * env;
}, [sound(0.2, "servo", 0.8), sound(1.5, "servo", 0.8, 1.15), sound(2.6, "servo", 0.8, 0.9), ...times([0.9, 1.0, 2.1], (c, i) => c.sfx("blip", 0.7, 1 + 0.3 * i))]);

// ---- Animals ----

const monkey = act("monkey", 4, (m, t, u) => {
  const env = envelope(u, 0.1);
  m.y = 0.6 * Math.abs(Math.sin(Math.PI * 2 * t)) * env;
  m.rot = 8 * Math.sin(Math.PI * 2 * t) * env;
}, [
  sound(0.2, "monkey"),
  [1.2, (c) => {
    const p = c.at();
    const side = Math.random() < 0.5 ? -1 : 1;
    c.scene.sprites.emit({
      code: "1f34c",
      x: p.x + side * 5,
      y: p.y + 2,
      life: 2.4,
      anim: (t) => (t < 0.9 ? { x: p.x + side * (5 - 3.2 * t / 0.9), y: p.y + 2 - 1.5 * (t / 0.9) + 1.8 * Math.sin(Math.PI * t / 0.9), scale: 0.9, rot: 600 * t } : { x: p.x + side * 1.8, y: p.y + 0.5 + 0.2 * Math.abs(Math.sin((t - 0.9) * 8)), scale: 0.9, rot: 20 * Math.sin((t - 0.9) * 8), alpha: 1 - clamp01((t - 1.9) / 0.5) }),
    });
  }],
  sound(1.2, "boing", 0.5, 1.4),
]);

const dog = act("dog", 4, (m, t, u) => {
  const env = envelope(u, 0.1);
  m.rot = 6 * Math.sin(Math.PI * 2 * 7 * t) * env * (t > 1.4 ? 1 : 0.3);
  m.y = 0.3 * Math.abs(Math.sin(Math.PI * 2 * 1.4 * t)) * env;
}, [
  sound(0.3, "bark"),
  [1.5, (c) => {
    const p = c.at();
    const x = p.x + rnd(-2.5, 2.5);
    c.scene.sprites.emit({
      code: "1f9b4",
      x,
      y: 5.5,
      life: 2.2,
      anim: (t) => {
        // Falls and bounces twice.
        const fall = Math.max(0, 5.5 - 14 * t * t);
        const y0 = GROUND + 0.4;
        const y = t < 0.78 ? Math.max(y0, fall) : y0 + Math.abs(Math.sin((t - 0.78) * 7)) * 1.2 * Math.exp(-(t - 0.78) * 2);
        return { x, y, scale: 1.1, rot: 200 * t, alpha: 1 - clamp01((t - 1.7) / 0.5) };
      },
    });
  }],
  sound(2.1, "thud", 0.5),
  sound(2.5, "bark", 0.9, 1.1),
]);

const cat = act("cat", 4, (m, t, u) => {
  const env = envelope(u, 0.15);
  m.rot = 6 * Math.sin(Math.PI * 2 * 0.5 * t) * env;
  m.sy = 1 + 0.03 * Math.sin(Math.PI * 2 * 1.5 * t) * env;
}, [
  sound(0.3, "meow"),
  [1.4, (c) => {
    const p = c.at();
    const side = Math.random() < 0.5 ? -1 : 1;
    c.scene.sprites.emit({ code: "1f41f", x: p.x, y: p.y, life: 2, anim: (t) => ({ x: p.x + side * (4.5 - 3.4 * t), y: p.y + 1.8 * Math.sin(Math.PI * t / 1.4) - 0.4, scale: 1.0, rot: side * (-30 + 90 * t), alpha: 1 - clamp01((t - 1.5) / 0.4) }) });
  }],
  sound(2.6, "meow", 0.8, 1.15),
]);

const unicorn = act("unicorn", 4.2, (m, t, u) => {
  const env = envelope(u, 0.1);
  m.centre = 1;
  m.rot = 360 * smooth((t - 0.5) / 1.6) * (t < 2.1 ? 1 : 0);
  m.y = 0.9 * Math.sin(Math.PI * clamp01((t - 0.5) / 1.6)) * env;
}, [
  sound(0.1, "harp"),
  [0.2, (c) => {
    const p = c.at();
    c.scene.sprites.emit({ code: "1f308", x: p.x, y: p.y + 0.6, life: 3.6, z: -0.3, anim: (t) => ({ x: p.x, y: p.y + 0.6, scale: 8 * smooth(t / 0.7), alpha: 0.85 * clamp01(t / 0.5) * (1 - clamp01((t - 3.0) / 0.6)) }) });
  }],
  ...times([0.4, 0.9, 1.4, 1.9, 2.4], (c) => rain(c, "2728", 4, { scale: 0.4, gravity: 0.8, spin: 90 })),
]);

const frog = act("frog", 4, (m, t, u) => {
  const env = envelope(u, 0.1);
  const hop = (s: number): number => 1.0 * Math.sin(Math.PI * clamp01((t - s) / 0.5));
  m.y = hop(0.5) + hop(1.6) + hop(2.7);
  m.sy = 1 + 0.07 * (hop(0.5) + hop(1.6) + hop(2.7)) - 0.04 * env;
}, [sound(0.2, "ribbit"), sound(1.3, "ribbit", 0.9, 1.1), ...times([0.5, 1.6, 2.7], (c) => c.sfx("boing", 0.4, 1.5)), ...times([1.0, 2.1], (c) => drift(c, "1fae7", 2, mouth(c), { width: 0.6, scale: 0.3, life: [1, 1.5] }))]);

const pig = act("pig", 3.8, (m, t, u) => {
  const env = envelope(u, 0.15);
  shiver(m, t, 4, 10, env * (Math.sin(t * 3) > 0.2 ? 1 : 0.2));
  m.sy = 1 - 0.03 * Math.abs(Math.sin(Math.PI * 2 * 1.5 * t)) * env;
}, [sound(0.3, "oink"), sound(1.4, "oink", 1, 1.15), sound(2.4, "oink", 0.9, 0.9), ...times([0.4, 1.5, 2.5], (c) => c.smoke(feet(c).x, feet(c).y, 10, [0.45, 0.32, 0.2, 0.7], 1.3))]);

const trex = act("trex", 4.2, (m, t, u) => {
  const env = envelope(u, 0.1);
  m.y = [0.3, 0.9, 1.5].reduce((n, s) => n + 0.15 * Math.sin(Math.PI * clamp01((t - s) / 0.25)), 0);
  m.sy = 1 + 0.07 * Math.sin(Math.PI * clamp01((t - 1.9) / 1.4)) * env;
  shiver(m, t, 3, 12, Math.sin(Math.PI * clamp01((t - 1.9) / 1.6)) * env);
}, [...times([0.4, 1.0, 1.6], (c) => {
  c.sfx("thud", 1, 0.6);
  c.shake(0.1, 0.4);
}), sound(2.0, "roar", 1, 0.7), [2.2, (c) => c.shake(0.14, 1.2)]]);

const octopus = act("octopus", 3.8, (m, t, u) => {
  const env = envelope(u, 0.15);
  m.centre = 1;
  m.rot = 14 * Math.sin(Math.PI * 2 * 1.4 * t) * env;
  m.y = 0.2 * Math.sin(Math.PI * 2 * 1.4 * t) * env;
}, [
  [0.6, (c) => {
    c.sfx("splash", 0.7, 1.3);
    const p = c.at();
    c.smoke(p.x, p.y - 0.5, 32, [0.07, 0.07, 0.15, 0.85], 2.8);
    drift(c, "1fae7", 6, p, { width: 2, scale: 0.4, life: [1.4, 2.2] });
  }],
]);

// ---- Objects and creatures ----

const alien = act("alien", 3.8, (m, t, u) => {
  // Looks about warily; no bounce, so it goes into the beam from standing.
  m.rot = 6 * Math.sin(Math.PI * 2 * 0.45 * t) * envelope(u, 0.15);
}, [sound(0.2, "ufo", 0.5, 1.6)]);

const pistol = act("pistol", 3.6, (m, t) => {
  let k = 0;
  for (const s of [0.5, 1.1, 1.7]) k += Math.exp(-(t - s) * 14) * (t > s ? 1 : 0);
  m.rot = -8 * k;
  m.x = -0.1 * k;
}, times([0.5, 1.1, 1.7], (c) => {
  c.sfx("squirt", 0.9, rnd(0.95, 1.1));
  const p = c.at();
  spray(c, "1f4a7", 12, { x: p.x + R * 0.4, y: p.y + 0.5 }, { angle: [5, 25], speed: [6, 9], gravity: 6, scale: [0.25, 0.4], life: [0.8, 1.1] });
}));

const fire = act("fire", 4, (m, t, u) => {
  const env = envelope(u, 0.15);
  const flick = 0.5 + 0.5 * Math.sin(t * 17) * Math.sin(t * 7.3);
  tintTo(m, 1.3, 1.0, 0.65, env * flick);
  m.sy = 1 + 0.04 * Math.sin(t * 13) * env;
  m.sx = 1 - 0.02 * Math.sin(t * 13) * env;
}, [sound(0.2, "crackle", 0.8), sound(1.5, "sizzle", 0.4), ...times(Array.from({ length: 10 }, (_, i) => 0.3 + i * 0.35), (c) => drift(c, "1f525", 1, feet(c), { width: 2, scale: [0.3, 0.6], life: [0.9, 1.4], rise: [1.5, 3], sway: 0.2 }))]);

const diamond = act("diamond", 7.2, (m, t) => {
  // Three taps with a pickaxe; each shakes it. After the third it shatters and re-forms.
  let k = 0;
  for (const s of [0.9, 1.9, 2.9]) k += Math.exp(-(t - s) * 12) * (t > s ? 1 : 0);
  m.rot = 4 * k * Math.sin(t * 70);
  m.sy = 1 - 0.03 * k;
  if (t >= 3.3 && t < 5.2) m.alpha = 0;
  if (t >= 5.2) {
    const f = clamp01((t - 5.2) / 0.5);
    m.alpha = f;
    m.sx = m.sy = 0.6 + 0.4 * f + 0.15 * Math.sin(Math.PI * f);
  }
}, [
  [0.1, (c) => cracks.set(0)],
  ...[0.9, 1.9, 2.9].flatMap((s, i): Cue[] => [
    [s - 0.55, (c) => {
      // The pickaxe swings in from the right, up and over, and strikes.
      const p = c.at();
      c.scene.sprites.emit({
        code: "26cf",
        x: p.x + 2.6,
        y: p.y + 1.5,
        life: 1.2,
        anim: (t) => {
          const swing = t < 0.35 ? smooth(t / 0.35) : t < 0.55 ? 1 - smooth((t - 0.35) / 0.2) * 1.5 : -0.5 + 0.5 * smooth((t - 0.55) / 0.3);
          return { x: p.x + 2.1 - 0.5 * swing * 0, y: p.y + 1.1 - 0.4 * (swing < 0 ? swing : 0), scale: 2.4, rot: 40 - 100 * swing, alpha: 1 - clamp01((t - 0.9) / 0.3) };
        },
      });
    }],
    [s, (c) => {
      c.sfx("dink", 1, 1 + 0.08 * i);
      c.shake(0.04, 0.2);
      const p = c.at();
      cracks.set(i + 1);
      spray(c, "2728", 5, { x: p.x + 0.2, y: p.y + 0.8 }, { speed: [2, 5], scale: [0.25, 0.5], life: [0.5, 0.9], drag: 1.5 });
      if (i < 2) c.sfx("crack", 0.6, 1 + 0.2 * i);
    }],
  ]),
  [3.3, (c) => {
    c.sfx("crack", 1);
    c.sfx("tinkle", 1);
    c.scene.shatter(c.at().x, c.at().y);
    cracks.set(0);
    c.burst(c.at().x, c.at().y, 26);
  }],
  [5.2, (c) => {
    c.sfx("ting", 1, 1.3);
    c.sfx("harp", 0.5, 1.4);
    spray(c, "2728", 12, c.at(), { speed: [2, 6], scale: [0.3, 0.7], life: [0.7, 1.2], drag: 1.2 });
  }],
]);

const balloon = act("balloon", 3.8, (m, t, u) => {
  const env = envelope(u, 0.2);
  m.centre = 1;
  m.y = (0.5 + 0.35 * Math.sin(Math.PI * 2 * 0.8 * t)) * env;
  m.rot = 6 * Math.sin(Math.PI * 2 * 0.5 * t) * env;
}, [sound(0.2, "squeak", 0.6, 0.7)]);

const invader = act("invader", 4.2, (m, t, u) => {
  const env = envelope(u, 0.08);
  const step = Math.floor(t * 3);
  // Marches a step at a time, side to side, like the arcade game.
  const pattern = [0, 1, 2, 3, 2, 1, 0, -1, -2, -3, -2, -1];
  m.x = 0.8 * pattern[step % pattern.length]! * env;
  m.y = 0.15 * (step % 2) * env;
  m.sy = 1 + 0.04 * (step % 2) * env;
}, [...times([0.34, 0.68, 1.0, 1.34, 1.68, 2.0, 2.34, 2.68, 3.0], (c, i) => c.sfx("blip", 0.7, 0.8 + 0.1 * (i % 4)))]);

const genie = act("genie", 4.4, (m, t, u) => {
  const env = envelope(u, 0.2);
  m.centre = 1;
  m.y = (0.5 + 0.3 * Math.sin(Math.PI * 2 * 0.7 * t)) * env;
  m.rot = 8 * Math.sin(Math.PI * 2 * 0.5 * t) * env;
}, [
  sound(0.2, "harp"),
  ...times(Array.from({ length: 9 }, (_, i) => 0.2 + i * 0.4), (c) => {
    c.smoke(feet(c).x + rnd(-0.6, 0.6), feet(c).y + 0.6, 5, [0.55, 0.35, 0.85, 0.5], 1.5);
    drift(c, "2728", 1, feet(c), { width: 1, scale: 0.35, life: [1.2, 1.8], rise: [1.2, 2.2] });
  }),
]);

const wizard = act("wizard", 4, (m, t, u) => {
  const env = envelope(u, 0.15);
  m.rot = -5 * env * Math.sin(Math.PI * 2 * 0.6 * t);
}, [
  [0.5, (c) => {
    const p = c.at();
    c.scene.sprites.emit({ code: "1fa84", x: p.x + 2.4, y: p.y + 1, life: 1.6, anim: (t) => ({ x: p.x + 2.2, y: p.y + 1.2, scale: 1.7, rot: 40 - 80 * Math.sin(Math.PI * clamp01(t / 0.6)), alpha: 1 - clamp01((t - 1.2) / 0.4) }) });
  }],
  [0.85, (c) => {
    c.sfx("harp", 0.9);
    spray(c, "2728", 18, { x: c.at().x + 1.6, y: c.at().y + 2.1 }, { speed: [2, 6], scale: [0.25, 0.55], life: [0.8, 1.3], drag: 1.2 });
  }],
  [2.2, (c) => {
    c.sfx("ting", 0.8);
    spray(c, "2728", 12, c.at(), { speed: [2, 5], scale: [0.25, 0.5], life: [0.8, 1.2], drag: 1.2 });
  }],
]);

const hero = act("hero", 3.8, (m, t, u) => {
  const env = envelope(u, 0.2);
  m.centre = 1;
  m.rot = 55 * env;
  m.y = (0.9 + 0.2 * Math.sin(Math.PI * 2 * 1.1 * t)) * env;
  if (env > 0.5) m.fx = { kind: "streaks", rate: 45 };
}, [sound(0.3, "whoosh", 0.9, 0.8), sound(1.8, "whoosh", 0.7)]);

const demon = act("demon", 3.8, (m, t, u) => {
  const env = envelope(u, 0.1);
  shiver(m, t, 5, 12, env);
  tintTo(m, 1.2, 0.65, 0.6, env);
}, [sound(0.2, "roar", 0.8, 1.1), sound(1.8, "evil", 0.7)]);

const money = act("money", 4.2, (m, t, u) => {
  const env = envelope(u, 0.15);
  m.sx = m.sy = 1 + 0.05 * Math.abs(Math.sin(Math.PI * 2 * 1.4 * t)) * env;
}, [
  ...times(Array.from({ length: 10 }, (_, i) => 0.2 + i * 0.3), (c, i) => {
    rain(c, "1f4b5", 3, { scale: 0.7, gravity: 2, spin: 150 });
    rain(c, "1fa99", 2, { scale: 0.45, gravity: 4, spin: 300 });
    if (i % 2 === 0) c.sfx("ting", 0.6, 0.9 + 0.1 * (i % 3));
  }),
]);

const zombie = act("zombie", 4.4, (m, t, u) => {
  // A jump scare: it lunges at the screen, then lurches about.
  const lunge = Math.exp(-Math.abs(t - 0.25) * 9);
  m.sx = m.sy = 1 + 0.3 * lunge;
  m.rot = (t < 0.7 ? 6 * Math.sin(t * 90) * lunge : 10 * Math.sin(Math.PI * 2 * 0.7 * t)) * envelope(u, 0.1);
  m.x = 0.3 * Math.sin(Math.PI * 2 * 0.35 * t) * smooth((t - 0.8) / 0.6);
  tintTo(m, 0.8, 1.05, 0.75, 1);
}, [
  [0.1, (c) => {
    c.sfx("scream", 1, 0.9);
    c.shake(0.2, 0.9);
    c.jolt(0.8);
    c.scene.flashNow(0.5);
  }],
  sound(2.3, "wail", 0.6, 0.6),
]);

const pumpkin = act("pumpkin", 5.2, (m, t, u) => {
  const env = envelope(u, 0.1);
  const s = 0.5 + 0.5 * Math.sin(t * 9) * Math.sin(t * 3.3);
  // The glow flickers between orange and purple.
  m.tint = [1 + 0.3 * s, 0.95, 1.3 - 0.35 * s].map((v) => lerp(1, v, env)) as [number, number, number];
  m.centre = 1;
  m.y = 0.3 * Math.sin(Math.PI * 2 * 0.8 * t) * env;
  m.rot = 5 * Math.sin(Math.PI * 2 * 1.5 * t) * env;
}, [
  sound(0.2, "evil", 0.9, 0.85),
  sound(1.1, "witch", 1),
  sound(3.5, "witch", 0.9, 1.1),
  [1.1, (c) => c.smoke(c.at().x, c.at().y + 1.4, 20, [0.6, 0.3, 0.9, 0.55], 2)],
  ...times([0.6, 2.0, 3.3], (c) => c.scene.strike()),
]);

export const SIGNATURES: Record<string, Act> = {
  tears, heartpop: heartEyes, sobbing, angry, swearing, scream, cold, sick, sleeping, thinking, mindblown: mindBlown, money, demon, ghost, skull, clown, robot, genie, wizard, hero,
  invader, monkey, dog, cat, unicorn, frog, pig, octopus, trex, alien, pistol, fire, diamond, balloon, zombie, pumpkin,
};
