// Ways off the stage: most of the entrances played backwards, plus a few that only work going
// out (turning round and waddling away, blasting off, freezing and sinking into a pool, ...).
import { HEIGHT } from "./config.ts";
import { ENTRANCES } from "./entrances.ts";
import { type Act, clamp01, type Cue, lerp, rewind, smooth, still } from "./moves.ts";

/** Turns its back and waddles off. */
const walkaway: Act = {
  name: "walkaway",
  dur: 7,
  weight: 10,
  cues: [[0.15, (c) => c.sfx("whoosh", 0.25, 1.6)]],
  at(t) {
    const m = still();
    const turn = clamp01(t / 0.7);
    m.spin = Math.max(0.04, Math.abs(Math.cos(Math.PI * turn)));
    m.mirror = turn >= 0.5;
    if (m.mirror) m.tint = [0.72, 0.72, 0.78];
    const u = clamp01((t - 0.9) / 6);
    m.d = 1 - u ** 1.5;
    m.stride = ((1 - m.d) * 14) / 1.15;
    const step = Math.sin(m.stride * Math.PI);
    const amp = clamp01(u * 8);
    m.rot = 7 * step * amp;
    m.y = 0.12 * Math.abs(step) * amp;
    return m;
  },
};

/** Rumbles and shakes, then blasts off upwards on a tail of fire (rare). */
const blastoff: Act = {
  name: "blastoff",
  dur: 4.4,
  weight: 1.5,
  cues: [
    [0, (c) => {
      c.sfx("rumble", 1);
      c.shake(0.05, 1.6);
    }],
    [1.45, (c) => c.sfx("rocket", 1)],
    [1.6, (c) => c.shake(0.14, 2.4)],
  ],
  at(t) {
    const m = still();
    m.x = 0;
    if (t < 1.6) {
      const k = t / 1.6;
      m.rot = (1.5 + 3 * k) * Math.sin(t * 157);
      m.sy = 1 + 0.01 * Math.sin(t * 90);
      m.fx = { kind: "fire", rate: 30 * k };
      m.ground = 30 * k;
      return m;
    }
    const tau = t - 1.6;
    m.y = 5.5 * tau * tau;
    m.sy = 1 + 0.08 * clamp01(tau * 2);
    m.sx = 1 - 0.04 * clamp01(tau * 2);
    m.rot = 1.5 * Math.sin(t * 90);
    m.fx = { kind: "fire", rate: 150 };
    m.ground = tau < 1 ? 80 : 0;
    return m;
  },
};

/** Freezes solid, shivering, then drops through the floor into a pool of water. */
const plunge: Act = {
  name: "plunge",
  dur: 4.2,
  weight: 5,
  cues: [
    [0, (c) => c.sfx("freeze", 0.9)],
    [1.5, (c) => c.scene.pool(c.at().x)],
    [1.95, (c) => c.sfx("slide_down", 0.4, 2.2)],
    [2.0, (c) => {
      c.sfx("splash", 1);
      c.scene.splash(c.at().x);
      c.shake(0.05, 0.3);
    }],
  ],
  at(t) {
    const m = still();
    m.x = 0;
    const f = smooth(t / 1.2);
    m.tint = [lerp(1, 0.62, f), lerp(1, 0.86, f), 1];
    m.rot = t < 1.95 ? 2.4 * Math.sin(t * 170) * f : 0;
    if (t >= 1.95) {
      const tau = t - 1.95;
      m.y = Math.max(-(HEIGHT + 0.3), -22 * tau * tau);
      const sunk = clamp01(-m.y / (HEIGHT * 0.6));
      m.alpha = 1 - sunk;
      m.sx = m.sy = 1 - 0.25 * clamp01(-m.y / HEIGHT);
    }
    return m;
  },
};

/**
 * The rocket's own launch: the flame goes out and it turns upright, a countdown from 3, then
 * ignition with the NASA "liftoff" call, the flame roaring back, and up it goes.
 */
const launch: Act = {
  name: "launch",
  dur: 7.6,
  weight: 0,
  cues: [
    [1.6, (c) => {
      c.sfx("rumble", 0.7);
      c.shake(0.03, 3.2);
    }],
    [1.8, (c) => {
      c.sfx("beep", 0.8);
      c.scene.countdown("3");
    }],
    [2.8, (c) => {
      c.sfx("beep", 0.8);
      c.scene.countdown("2");
    }],
    [3.8, (c) => {
      c.sfx("beep", 0.8);
      c.scene.countdown("1");
    }],
    [4.8, (c) => {
      c.sfx("beep", 0.9, 1.6);
      c.sfx("liftoff", 1);
      c.sfx("rocket", 0.5);
      c.scene.countdown("LIFTOFF!");
      c.shake(0.14, 3);
    }],
  ],
  at(t) {
    const m = still();
    m.x = 0;
    const turn = smooth((t - 0.3) / 1.1);
    const pre = clamp01((t - 1.6) / 3.2);
    m.rot = (0.3 + 1.4 * pre) * Math.sin(t * 120) * (t < 4.8 ? 1 : 0.4);
    if (t < 4.8) {
      m.rocket = { turn, flame: 0, power: 0, stars: 1 - smooth((t - 0.3) / 1) };
      return m;
    }
    const tau = t - 4.8;
    m.rocket = { turn: 1, flame: smooth(tau / 0.25), power: 1, stars: 0 };
    const go = Math.max(0, tau - 0.4);
    m.y = 5.5 * go * go;
    m.sy = 1 + 0.06 * clamp01(go * 2);
    m.ground = tau < 1.6 ? 90 : 0;
    return m;
  },
};

/** Floats up and away like the party's balloons, swaying, until it's out of the top of the stage. */
const balloonaway: Act = {
  name: "balloonaway",
  dur: 6,
  weight: 0,
  cues: [[0, (c) => c.sfx("squeak", 0.7, 0.8)], [0.3, (c) => c.sfx("whoosh", 0.4, 0.5)]],
  at(t) {
    const m = still();
    m.centre = 1;
    m.x = 0.9 * Math.sin(t * 2.2) * smooth(t / 1.2);
    m.y = 0.6 * t + 0.9 * t ** 1.9;
    m.rot = 7 * Math.sin(t * 2.2 + 1);
    m.sx = m.sy = lerp(1, 0.85, smooth(t / 5));
    return m;
  },
};

/**
 * The spotlight turns into a blue beam and the emoji zooms up it to the top, shrinking; the beam
 * snaps shut into a thin V as a saucer drops into view and zooms off.
 */
const abduct: Act = {
  name: "abduct",
  dur: 5.6,
  weight: 0,
  cues: [
    [0, (c) => {
      c.sfx("ufo", 0.9);
      c.scene.ufo();
    }],
    [3.4, (c) => c.sfx("zap", 0.7, 1.8)],
    [3.5, (c) => c.sfx("whoosh", 0.9, 1.3)],
  ],
  at(t) {
    const m = still();
    // The saucer drops in by 0.7 s;
    // It is drawn up the beam as the saucer arrives and is absorbed into it before the beam thins.
    const up = smooth((t - 0.9) / 1.9);
    m.centre = 1;
    m.y = 4.2 * up;
    m.sx = m.sy = lerp(1, 0.1, up);
    m.tint = [1 - 0.25 * up, 1 - 0.05 * up, 1 + 0.1 * up];
    m.alpha = 1 - smooth((t - 2.6) / 0.25);
    return m;
  },
};

/** Fades away where it stands, drifting up. */
const fadeout: Act = {
  name: "fadeout",
  dur: 2.2,
  weight: 6,
  cues: [[0, (c) => c.sfx("sparkle", 0.7, 0.8)]],
  at(t) {
    const m = still();
    m.x = 0;
    m.alpha = 1 - smooth(t / 1.8);
    m.y = 0.6 * smooth(t / 2.2);
    m.sx = m.sy = lerp(1, 0.95, smooth(t / 2.2));
    return m;
  },
};

/** Winds up leaning back, then zips off in a cloud of dust. */
const zoom: Act = {
  name: "zoom",
  dur: 1.9,
  weight: 6,
  cues: [
    [0.05, (c) => c.sfx("skid", 0.5, 1.5)],
    [0.6, (c) => {
      c.sfx("whoosh", 1);
      c.sfx("scamper", 0.6, 1.3);
      c.smoke(c.at().x, -3.6, 10, [0.8, 0.75, 0.65, 0.4]);
    }],
  ],
  at(t) {
    const m = still();
    if (t < 0.6) {
      const a = smooth(t / 0.6);
      m.sy = 1 - 0.1 * a;
      m.sx = 1 + 0.06 * a;
      m.rot = -14 * a;
      return m;
    }
    const u = clamp01((t - 0.6) / 1.1);
    m.d = 1 - u ** 2.2;
    m.rot = 26;
    m.sx = 1.1;
    m.sy = 0.94;
    m.fx = { kind: "streaks", rate: 80 };
    m.ground = u < 0.3 ? 80 : 0;
    return m;
  },
};

/** Rolls off the stage faster and faster and crashes in the wings. */
const rolloff: Act = {
  name: "rolloff",
  dur: 3.4,
  weight: 3,
  cues: [
    [0, (c) => c.sfx("whoosh", 0.7)],
    [2.3, (c) => {
      c.sfx("crash", 1);
      c.shake(0.3, 0.8);
    }],
  ],
  at(t) {
    const m = still();
    const u = clamp01(t / 2.6);
    m.d = 1 - 1.15 * u ** 1.6;
    m.rot = 540 * u;
    m.centre = 1;
    if (u < 1) m.fx = { kind: "dust", rate: 20 };
    return m;
  },
};

/** A firework finale: rockets, willows and a volley of bursts while it dances, then it fades into the smoke. */
const fireworks: Act = {
  name: "fireworks",
  dur: 7.2,
  weight: 3,
  cues: [
    [0.2, (c) => c.scene.firework(-4, 1.2)],
    [0.7, (c) => c.scene.firework(3.8, 0.5)],
    [1.3, (c) => c.scene.firework(-1, 2.2)],
    [1.9, (c) => {
      c.scene.firework(5, 1.6);
      c.scene.firework(-5.5, 0.8);
    }],
    [2.6, (c) => {
      c.sfx("cheer", 0.8);
      c.scene.celebrate(true);
    }],
    [3.1, (c) => c.scene.firework(0, 2.1, "willow")],
    ...Array.from({ length: 9 }, (_, i): Cue => [4.0 + i * 0.12, (c) => c.scene.firework(-7.5 + i * 1.9, 0.1 + (i % 3) * 1.0, i % 3 === 1 ? "ring" : undefined)]),
  ],
  at(t) {
    const m = still();
    m.x = 0;
    const env = smooth(t / 0.5);
    m.y = 0.5 * Math.abs(Math.sin(Math.PI * 1.5 * t)) * env;
    m.rot = 6 * Math.sin(Math.PI * 2 * 0.8 * t) * env;
    m.alpha = 1 - smooth((t - 5.6) / 1.4);
    return m;
  },
};

/** Swells, flashing red and ticking faster, then blows up. */
const explode: Act = {
  name: "explode",
  dur: 3.6,
  weight: 2,
  cues: [
    [0, (c) => c.sfx("fuse", 1)],
    [1.9, (c) => {
      const p = c.at();
      c.scene.explode(p.x, p.y);
    }],
  ],
  at(t) {
    const m = still();
    m.x = 0;
    if (t >= 1.9) {
      m.alpha = 0;
      return m;
    }
    const u = t / 1.9;
    const pulse = 0.5 + 0.5 * Math.sin(6 * t + (14 * t * t) / 1.9);
    m.sx = m.sy = 1 + 0.28 * u * u + 0.04 * pulse;
    m.rot = (1 + 4 * u) * Math.sin(t * 90);
    m.tint = [1 + 0.8 * pulse * u, 1 - 0.5 * pulse * u, 1 - 0.5 * pulse * u];
    return m;
  },
};

const back = ["walk", "hop", "rush", "crawl", "peek", "drop", "roll", "pop", "rise", "fade"];
export const EXITS: Act[] = [...ENTRANCES.filter((a) => back.includes(a.name)).map(rewind), walkaway, blastoff, plunge, fadeout, zoom, rolloff, fireworks, explode, launch, balloonaway, abduct];
