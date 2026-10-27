// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// What an emoji does in the spotlight. Every pose starts and ends at rest.
import type { PoseName } from "./cast.ts";
import { GROUND, HEIGHT } from "./config.ts";
import { type Act, clamp01, type Ctx, type Cue, envelope, keys, lerp, smooth, still } from "./moves.ts";

const DEFAULT_TIME = 3.8;

const bow: Act = {
  name: "bow",
  dur: DEFAULT_TIME,
  at(t) {
    const m = still();
    const u = t / DEFAULT_TIME;
    const b = smooth(u / 0.25) * (1 - smooth((u - 0.7) / 0.25));
    m.rot = 22 * b;
    m.sy = 1 - 0.1 * b;
    m.sx = 1 + 0.04 * b;
    return m;
  },
};

/** A wink is a quick swap to the winking face, with a nod each time. */
const wink: Act = {
  name: "wink",
  dur: DEFAULT_TIME,
  at(t) {
    const m = still();
    const u = t / DEFAULT_TIME;
    const env = envelope(u);
    if ((t > 0.7 && t < 1.1) || (t > 2.3 && t < 2.7)) m.swap = "1f609";
    m.rot = 4 * Math.sin(Math.PI * 4 * u) * env;
    m.sy = 1 - 0.03 * Math.abs(Math.sin(Math.PI * u * 4)) * env;
    return m;
  },
};

/** Hops up and turns about its own axis twice, like a coin in the air. */
const spin: Act = {
  name: "spin",
  dur: DEFAULT_TIME,
  cues: [[0.05, (c) => {
    c.sfx("whoosh", 0.6, 1.2);
    c.burst(c.at().x, c.at().y, 22);
  }]],
  at(t) {
    const m = still();
    const u = t / DEFAULT_TIME;
    const e = smooth(u / 0.85);
    m.centre = 1;
    m.y = 1.6 * Math.sin(Math.PI * e);
    m.spin = Math.max(0.03, Math.abs(Math.cos(Math.PI * 4 * e)));
    const land = Math.sin(Math.PI * clamp01((u - 0.85) / 0.12));
    m.sy = 1 - 0.1 * land;
    m.thump = land;
    return m;
  },
};

const pump: Act = {
  name: "pump",
  dur: DEFAULT_TIME,
  cues: [[0.2, (c) => c.burst(c.at().x, c.at().y, 22)]],
  at(t) {
    const m = still();
    const u = t / DEFAULT_TIME;
    const beat = Math.abs(Math.sin(Math.PI * 3 * u)) * envelope(u);
    m.y = 0.4 * beat;
    m.sy = 1 + 0.08 * beat;
    m.sx = 1 - 0.04 * beat;
    return m;
  },
};

const sway: Act = {
  name: "sway",
  dur: DEFAULT_TIME,
  at(t) {
    const m = still();
    const u = t / DEFAULT_TIME;
    m.rot = 10 * Math.sin(Math.PI * 2 * 1.4 * u) * envelope(u);
    return m;
  },
};

const shake: Act = {
  name: "shake",
  dur: DEFAULT_TIME,
  at(t) {
    const m = still();
    const u = t / DEFAULT_TIME;
    m.rot = 7 * Math.sin(Math.PI * 2 * 11 * u) * envelope(u);
    m.sx = 1 + 0.03 * Math.sin(Math.PI * 2 * 11 * u + 1) * envelope(u);
    return m;
  },
};

const float: Act = {
  name: "float",
  dur: DEFAULT_TIME,
  cues: [[0, (c) => c.sfx("sparkle", 0.35, 0.8)]],
  at(t) {
    const m = still();
    const env = envelope(t / DEFAULT_TIME, 0.2);
    m.centre = 1;
    m.y = (0.5 + 0.35 * Math.sin(Math.PI * 2 * 1.1 * t)) * env;
    m.rot = 5 * Math.sin(Math.PI * 2 * 0.6 * t) * env;
    return m;
  },
};

const flip: Act = {
  name: "flip",
  dur: DEFAULT_TIME,
  cues: [[0.4, (c) => c.sfx("whoosh", 0.5, 1.3)]],
  at(t) {
    const m = still();
    const u = t / DEFAULT_TIME;
    const e = smooth((u - 0.1) / 0.5);
    m.centre = 1;
    m.y = 1.6 * Math.sin(Math.PI * e);
    m.rot = 360 * e;
    const land = Math.sin(Math.PI * clamp01((u - 0.6) / 0.14));
    m.sy = 1 - 0.1 * land;
    m.thump = land;
    return m;
  },
};

const dizzy: Act = {
  name: "dizzy",
  dur: DEFAULT_TIME,
  at(t) {
    const m = still();
    const env = envelope(t / DEFAULT_TIME);
    m.centre = 1;
    m.rot = 12 * Math.sin(Math.PI * 2 * 1.3 * t) * env;
    m.x = 0.35 * Math.sin(Math.PI * 2 * 0.9 * t) * env;
    m.y = 0.1 * Math.sin(Math.PI * 2 * 1.6 * t) * env;
    return m;
  },
};

const watch: Act = {
  name: "watch",
  dur: DEFAULT_TIME,
  at(t) {
    const m = still();
    m.rot = 3 * Math.sin(Math.PI * 2 * 0.5 * t) * envelope(t / DEFAULT_TIME);
    m.sy = 1 + 0.015 * Math.sin(Math.PI * 2 * 0.9 * t);
    return m;
  },
};

/** Beats like a heart, a lub and a dub, in time with the heartbeat the room plays. */
const beat: Act = {
  name: "beat",
  dur: 5.2,
  at(t) {
    const m = still();
    const local = t % 0.86;
    const s = 0.13 * Math.exp(-local * 11) + (local > 0.3 ? 0.09 * Math.exp(-(local - 0.3) * 11) : 0);
    const env = clamp01(t / 0.4) * clamp01((5.2 - t) / 0.4);
    m.sx = 1 + s * env;
    m.sy = 1 + s * env;
    m.centre = 1;
    return m;
  },
};

/** Strains, lets one rip with a green cloud, looks sheepish and gets the sad trombone. */
const fart: Act = {
  name: "fart",
  dur: 4.8,
  cues: [
    [0.5, (c) => {
      c.sfx("fart", 1);
      c.smoke(c.at().x - 1.6, GROUND + 0.8, 44, [0.55, 0.85, 0.2, 0.7]);
      c.shake(0.03, 0.9);
    }],
    [1.1, (c) => c.smoke(c.at().x - 2, GROUND + 1.0, 30, [0.55, 0.85, 0.2, 0.6])],
    [2.2, (c) => c.sfx("wah", 0.9)],
    [2.6, (c) => c.sfx("laugh", 0.7)],
  ],
  at(t) {
    const m = still();
    if (t < 0.5) {
      const k = t / 0.5;
      m.sx = 1 + 0.06 * k;
      m.rot = 1.2 * Math.sin(t * 120) * k;
    } else if (t < 1.7) {
      m.rot = 6 + 3 * Math.sin(t * 220);
      m.y = 0.12 * Math.abs(Math.sin(t * 55));
      m.sx = 1.04;
    } else {
      const k = smooth((t - 1.7) / 0.5) * (1 - smooth((t - 4.2) / 0.5));
      m.rot = -4 * k;
      m.sy = 1 - 0.04 * k;
    }
    const sick = smooth((t - 0.5) / 0.4) * (1 - smooth((t - 2.6) / 1.0));
    m.tint = [lerp(1, 0.85, sick), 1, lerp(1, 0.7, sick)];
    return m;
  },
};

/** A party popper's own show: bang, streamers, a slide whistle and a kazoo fanfare. */
const party: Act = {
  name: "party",
  dur: 4.6,
  cues: [
    [0, (c) => {
      c.sfx("pop", 1);
      c.burst(c.at().x, c.at().y, 90);
      c.scene.celebrate(true);
    }],
    [0.12, (c) => c.sfx("slide", 0.8)],
    [0.9, (c) => {
      c.sfx("kazoo", 0.9);
      c.burst(c.at().x, c.at().y, 40);
    }],
    [2.4, (c) => {
      c.sfx("sparkle", 0.7);
      c.burst(c.at().x, c.at().y, 40);
    }],
  ],
  at(t) {
    const m = still();
    const env = envelope(t / 4.6, 0.1);
    m.y = 0.55 * Math.abs(Math.sin(Math.PI * 1.7 * t)) * env;
    m.rot = 9 * Math.sin(Math.PI * 2 * 0.85 * t) * env;
    m.sy = 1 + 0.05 * Math.abs(Math.sin(Math.PI * 1.7 * t)) * env;
    return m;
  },
};

/** Struck by lightning: a flash, a jolt, then it smoulders and recovers. */
const zap: Act = {
  name: "zap",
  dur: 4,
  cues: [
    [0.5, (c) => c.scene.strike(c.at().x)],
    [0.7, (c) => c.smoke(c.at().x, c.at().y + HEIGHT * 0.4, 14, [0.2, 0.2, 0.2, 0.5])],
    [1.2, (c) => c.smoke(c.at().x, c.at().y + HEIGHT * 0.5, 10, [0.25, 0.25, 0.25, 0.4])],
  ],
  at(t) {
    const m = still();
    if (t < 0.5) m.rot = 1.5 * Math.sin(t * 150) * (t / 0.5);
    if (t >= 0.5) {
      const k = Math.exp(-(t - 0.5) * 1.6);
      m.rot = 6 * Math.sin(t * 200) * k;
      m.sx = 1 + 0.03 * Math.sin(t * 170) * k;
    }
    const burnt = smooth((t - 0.55) / 0.15) * (1 - smooth((t - 2.6) / 1.0));
    const flash = t > 0.5 && t < 0.62 ? 1 : 0;
    const c = lerp(1, 0.4, burnt);
    m.tint = [c + flash, c + flash, c + flash];
    return m;
  },
};

function photoCues(): Cue[] {
  const out: Cue[] = [];
  for (const t of [0.4, 1.4, 2.4, 3.4, 4.4]) out.push([t, (c: Ctx) => c.scene.photos(3)]);
  return out;
}

/** A model on the runway: a new pose every second, a burst of camera flashes on each. */
const model: Act = {
  name: "model",
  dur: 5.8,
  cues: photoCues(),
  at(t) {
    const m = still();
    m.rot = keys(t, [
      [0, 0], [0.4, 0], [0.52, -10], [1.4, -10], [1.52, 8], [2.4, 8], [2.52, -6], [3.4, -6], [3.52, 11], [4.4, 11], [4.52, -4], [5.2, -4], [5.5, 0], [5.8, 0],
    ]);
    // Each pose lands with a small pop.
    let pop = 0;
    for (const s of [0.4, 1.4, 2.4, 3.4, 4.4]) if (t > s) pop += Math.exp(-(t - s) * 12) * (t - s < 1 ? 1 : 0);
    m.sy = 1 + 0.05 * pop;
    m.sx = 1 - 0.025 * pop;
    m.y = 0.15 * pop;
    return m;
  },
};

function makePoses(): Map<string, Act> {
  const m = new Map<string, Act>();
  m.set("model", model);
  m.set("bow", bow);
  m.set("wink", wink);
  m.set("spin", spin);
  m.set("pump", pump);
  m.set("sway", sway);
  m.set("shake", shake);
  m.set("float", float);
  m.set("flip", flip);
  m.set("dizzy", dizzy);
  m.set("watch", watch);
  m.set("beat", beat);
  m.set("fart", fart);
  m.set("party", party);
  m.set("zap", zap);
  return m;
}

export const POSES: Map<string, Act> = makePoses();
