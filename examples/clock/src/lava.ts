// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// The lava lamp: wax blobs rise when they're hot, cool at the top and sink. Each blob's outline
// is a circle pushed in and out by a slow noise field, sampled where each point is in the lamp,
// so the wax bulges and folds organically and neighbouring blobs bulge alike. The wax has a
// life: blobs bud from the pool (joined to it by a neck) and lift off when they're grown, two
// that meet merge (the smaller drains into the larger, which grows by the same area), others
// push gently apart, and cooled blobs sink back and melt into the pool. One colour brighter
// than white fills everything, so touching shapes read as one and glow through bloom. The mouse
// pushes blobs away. The time glows beside it.
//
// The blob look (outlines pushed by noise sampled in the scene, blobs growing from the ends and
// merging) is after "Lava Lamp" by Corevision: https://codepen.io/corevision/pen/ZYzBqxx
// (inspiration only; the pen is published under CodePen's default MIT licence, and no other
// licence is stated on it. This file is written for Platypus and is under Apache-2.0.)

import { Face, label, mix, shape, two, under } from "./common.ts";

const LAMP = new Vector3(-2.6, 0, 0);
const GLASS_TOP = 3.4;
const GLASS_BOTTOM = -2.6;
const LAVA = new Color(1.45, 0.3, 0.07, 1);
const LAVA_EDGE = new Color(1.8, 0.55, 0.12, 1);
const LIQUID = Color.hex("#2a0f2e");
const METAL = Color.hex("#3a3d45");
const METAL_EDGE = Color.hex("#8b909c");
const BACK = Color.hex("#0b0710");
/** The base's top half-width: the glass's foot sits inside it. */
const BASE_TOP = 0.95;
/** Points round each blob. */
const POINTS = 56;
/** How far a blob's neck reaches to the pool or to the blob absorbing it. */
const REACH = 0.45;
/** Blob shapes kept ready (more than ever live at once). */
const SLOTS = 9;
/** The biggest a blob grows by merging; bigger pairs push apart instead. */
const MAX_R = 0.72;

/** The glass's half-width at height `y` (lamp space): narrow at the ends, widest low down. */
const glassWidth = (y: number) => {
  const a = (y - GLASS_BOTTOM) / (GLASS_TOP - GLASS_BOTTOM);
  return 0.62 + 0.72 * Math.sin(Math.PI * Math.min(1, Math.max(0, a * 0.82 + 0.08)));
};

/** A point kept inside the glass (wax can't bulge through it). */
const inside = (x: number, y: number) => {
  const cy = Math.max(GLASS_BOTTOM + 0.03, Math.min(GLASS_TOP - 0.03, y));
  const room = glassWidth(cy) - 0.04;
  return new Vector2(Math.max(-room, Math.min(room, x)), cy);
};

// ---- Smooth 2D noise (gradient noise, -1 to 1) ----

const PERM: number[] = (() => {
  const p: number[] = [];
  for (let i = 0; i < 256; i++) p.push(i);
  let seed = 1337;
  for (let i = 255; i > 0; i--) {
    seed = (seed * 16807) % 2147483647;
    const j = seed % (i + 1);
    const swap = p[i]!;
    p[i] = p[j]!;
    p[j] = swap;
  }
  const doubled: number[] = [];
  for (let k = 0; k < 2; k++) for (let i = 0; i < 256; i++) doubled.push(p[i]!);
  return doubled;
})();

function noise(x: number, y: number): number {
  const xi = Math.floor(x);
  const yi = Math.floor(y);
  const fx = x - xi;
  const fy = y - yi;
  const grad = (ix: number, iy: number, dx: number, dy: number) => {
    const h = PERM[(PERM[ix & 255]! + iy) & 255]! & 7;
    const gx = [1, -1, 1, -1, 1, -1, 0, 0][h]! + (h >= 6 ? 0.7 : 0);
    const gy = [1, 1, -1, -1, 0, 0, 1, -1][h]!;
    return gx * dx + gy * dy;
  };
  const fade = (k: number) => k * k * k * (k * (k * 6 - 15) + 10);
  const u = fade(fx);
  const v = fade(fy);
  const a = grad(xi, yi, fx, fy) + (grad(xi + 1, yi, fx - 1, fy) - grad(xi, yi, fx, fy)) * u;
  const b = grad(xi, yi + 1, fx, fy - 1) + (grad(xi + 1, yi + 1, fx - 1, fy - 1) - grad(xi, yi + 1, fx, fy - 1)) * u;
  return (a + (b - a) * v) * 0.9;
}

/** The wax's wobble at a point (lamp space) and time: two octaves, drifting slowly upward. */
const wobble = (x: number, y: number, t: number) => noise(x * 1.3, y * 1.3 - t * 0.22) + 0.35 * noise(x * 3.1 + 5.2, y * 3.1 - t * 0.5);

type Stage = "budding" | "free" | "merging" | "melting";

interface Blob {
  line: LineRenderer2D;
  x: number;
  y: number;
  vx: number;
  vy: number;
  r: number;
  /** The size it buds to before lifting off. */
  grow: number;
  heat: number;
  phase: number;
  stage: Stage;
  /** The blob absorbing this one (while merging). */
  into: Blob | null;
}

/**
 * A neck between two round shapes `reach` or less apart: two curves pinched toward the line
 * between them, thinner as they part. Empty when they're too far apart.
 */
function neck(ax: number, ay: number, ar: number, bx: number, by: number, br: number, reach: number): Vector2[] {
  const dx = bx - ax;
  const dy = by - ay;
  const d = Math.hypot(dx, dy);
  const gap = d - ar - br;
  if (gap > reach || d < 1e-4 || d < Math.abs(ar - br) + 0.05) return [];
  const k = Math.max(0, Math.min(1, (reach - gap) / reach));
  const ux = dx / d;
  const uy = dy / d;
  const nx = -uy;
  const ny = ux;
  // Where the neck leaves each shape: further round the closer they are.
  const spread = 0.55 + 0.45 * k;
  const ca = Math.cos(spread);
  const sa = Math.sin(spread);
  const waist = Math.min(ar, br) * (0.08 + 0.62 * k * k);
  const mx = (ax + ar * ux + bx - br * ux) / 2;
  const my = (ay + ar * uy + by - br * uy) / 2;
  const side = (s: number): Vector2[] => {
    const p0 = new Vector2(ax + ar * (ux * ca + nx * sa * s), ay + ar * (uy * ca + ny * sa * s));
    const p2 = new Vector2(bx + br * (-ux * ca + nx * sa * s), by + br * (-uy * ca + ny * sa * s));
    // The control point sits inside the waist, so the sides curve in.
    const c = new Vector2(mx + nx * waist * s, my + ny * waist * s);
    const out: Vector2[] = [];
    for (let i = 0; i <= 8; i++) {
      const q = i / 8;
      const w0 = (1 - q) * (1 - q);
      const w1 = 2 * (1 - q) * q;
      const w2 = q * q;
      out.push(new Vector2(w0 * p0.x + w1 * c.x + w2 * p2.x, w0 * p0.y + w1 * c.y + w2 * p2.y));
    }
    return out;
  };
  // Down one side, back along the other, closed across each shape's inside.
  const first = side(1);
  const second = side(-1);
  const all: Vector2[] = [];
  for (let i = 0; i < first.length; i++) all.push(first[i]!);
  for (let i = second.length - 1; i >= 0; i--) all.push(second[i]!);
  all.push(new Vector2(ax, ay));
  return all;
}

export class LavaFace extends Face {
  readonly name = "Lava lamp";
  readonly root: GameObject = new GameObject("Lava lamp face");
  private blobs: Blob[] = [];
  /** Shapes not in use by a blob. */
  private free: LineRenderer2D[] = [];
  /** How many of `free` are in use as a stack (the rest of the array keeps its place). */
  private freeTop = 0;
  private necks: LineRenderer2D[] = [];
  private budIn = 0;
  private pool!: LineRenderer2D;
  private camera: Camera | null = null;
  private background: Color | null = null;
  private time!: TextMesh;
  private seconds!: TextMesh;
  private shown = "";

  constructor() {
    super();
    under(this.root, () => this.build());
  }

  private build(): void {
    // The glass, filled with the dark liquid, then the cap and the base over it.
    const right: Vector2[] = [];
    for (let i = 0; i <= 40; i++) {
      const y = GLASS_BOTTOM + ((GLASS_TOP - GLASS_BOTTOM) * i) / 40;
      right.push(new Vector2(glassWidth(y), y));
    }
    const outline: Vector2[] = [];
    for (let i = 0; i < right.length; i++) outline.push(right[i]!);
    for (let i = right.length - 1; i >= 0; i--) outline.push(new Vector2(-right[i]!.x, right[i]!.y));
    shape("liquid", { points: outline, loop: true, width: 0.05, color: mix(LIQUID, LAVA_EDGE, 0.08), fillColor: LIQUID, sortingOrder: 0 }, LAMP);
    // The wax pooled at the bottom.
    this.pool = shape("pool", { loop: true, width: 0, fillColor: LAVA, sortingOrder: 1 }, LAMP);
    // Fill only: blobs that touch read as one shape.
    for (let i = 0; i < SLOTS; i++) this.free.push(shape(`blob ${i}`, { loop: true, width: 0, fillColor: LAVA, sortingOrder: 1 }, LAMP));
    this.freeTop = SLOTS;
    for (let i = 0; i < SLOTS; i++) this.necks.push(shape(`neck ${i}`, { loop: true, width: 0, fillColor: LAVA, sortingOrder: 1 }, LAMP));
    // A lamp that's been on a while: blobs already up in the glass, rising and sinking.
    const start: number[][] = [[0.6, 1.8, 0.9], [0.48, -0.6, 0.2], [0.4, 0.4, 0.7], [0.32, 2.6, 0.1], [0.27, -1.2, 0.95]];
    for (let i = 0; i < start.length; i++) {
      const k = start[i]!;
      this.spawn((i % 2 ? 1 : -1) * 0.25, k[1]!, k[0]!, k[0]!, k[2]!, "free");
    }
    // The cap sits on the glass: as wide as its top, narrowing upward.
    const capBottom = glassWidth(GLASS_TOP) + 0.03;
    shape("cap", { points: [new Vector2(-capBottom, 0), new Vector2(capBottom, 0), new Vector2(capBottom * 0.62, 1.0), new Vector2(-capBottom * 0.62, 1.0)], loop: true, width: 0.04, color: METAL_EDGE, fillColor: METAL, sortingOrder: 5 }, LAMP.add(new Vector3(0, GLASS_TOP - 0.02, 0)));
    shape("base", { points: [new Vector2(-BASE_TOP, 0), new Vector2(BASE_TOP, 0), new Vector2(1.35, -1.55), new Vector2(-1.35, -1.55)], loop: true, width: 0.04, color: METAL_EDGE, fillColor: METAL, sortingOrder: 5 }, LAMP.add(new Vector3(0, GLASS_BOTTOM + 0.02, 0)));
    this.time = label("time", { fontSize: 96, fontWeight: 700, color: LAVA_EDGE }, new Vector3(2.4, 0.5, 0));
    this.seconds = label("seconds", { fontSize: 30, color: new Color(0.95, 0.3, 0.14, 1) }, new Vector3(2.4, -0.6, 0));
  }

  override show(camera: Camera): void {
    this.camera = camera;
    this.background = camera.backgroundColor;
    camera.backgroundColor = BACK;
    RenderSettings.bloom = { threshold: 0.65, intensity: 1.3, scatter: 0.78 };
  }

  override hide(camera: Camera): void {
    if (this.background) camera.backgroundColor = this.background;
    RenderSettings.bloom = null;
  }

  private spawn(x: number, y: number, r: number, grow: number, heat: number, stage: Stage): void {
    if (this.freeTop === 0) return;
    this.freeTop--;
    const line = this.free[this.freeTop]!;
    this.blobs.push({ line, x, y, vx: 0, vy: 0, r, grow, heat, phase: Math.random() * 10, stage, into: null });
  }

  private retire(b: Blob): void {
    b.line.points = [];
    this.free[this.freeTop] = b.line;
    this.freeTop++;
    for (let i = 0; i < this.blobs.length; i++) {
      if (this.blobs[i] === b) {
        this.blobs.splice(i, 1);
        break;
      }
    }
    for (const o of this.blobs) if (o.into === b) o.into = null;
  }

  /** The pool's surface height at `x` (lamp space). */
  private poolTop(x: number, t: number): number {
    return GLASS_BOTTOM + 0.36 + 0.13 * wobble(x, GLASS_BOTTOM, t * 0.8);
  }

  update(now: DateTime, t: number, dt: number): void {
    const step = Math.min(dt, 1 / 30);
    // The mouse, in lamp space.
    let mouse: Vector2 | null = null;
    if (this.camera) {
      const m = input.mousePosition;
      const p = this.camera.screenToWorldPoint({ x: m.x, y: m.y, z: 10 });
      mouse = new Vector2(p.x - LAMP.x, p.y - LAMP.y);
    }
    // A new blob buds from the pool now and then, more often when few are about.
    this.budIn -= step;
    let live = 0;
    for (const b of this.blobs) if (b.stage !== "melting") live++;
    if (this.budIn <= 0 && live < 6) {
      const x = (Math.random() - 0.5) * 0.9;
      this.spawn(x, this.poolTop(x, t), 0.04, 0.3 + Math.random() * 0.28, 1, "budding");
      this.budIn = live < 3 ? 2 : 5 + Math.random() * 5;
    }

    // Merging: blobs that meet join (both free, and not too big together). The smaller one
    // drains into the larger.
    for (const a of this.blobs) {
      for (const b of this.blobs) {
        if (a === b || a.stage !== "free" || b.stage !== "free") continue;
        const d = Math.hypot(a.x - b.x, a.y - b.y);
        if (d < (a.r + b.r) * 0.8 && Math.hypot(a.r, b.r) <= MAX_R) {
          const small = a.r < b.r ? a : b;
          const big = a.r < b.r ? b : a;
          small.stage = "merging";
          small.into = big;
        } else if (d < (a.r + b.r) * 1.05 && d > 1e-4) {
          // Too big to join: they nudge apart, like the pen this is after.
          const push = ((a.r + b.r) * 1.05 - d) * 0.6 * step;
          a.x += ((a.x - b.x) / d) * push;
          a.y += ((a.y - b.y) / d) * push;
        }
      }
    }

    for (const b of this.blobs.slice()) {
      if (b.stage === "budding") {
        // Growing out of the pool, half sunk in it, until it's big enough to lift off.
        b.r = Math.min(b.grow, b.r + 0.07 * step);
        b.y = this.poolTop(b.x, t) + b.r * 0.2;
        if (b.r >= b.grow) {
          b.stage = "free";
          b.vy = 0.04;
        }
      } else if (b.stage === "merging") {
        const into = b.into;
        if (!into) {
          b.stage = "free";
        } else {
          // Area moves from the small blob to the big one; the small one slides into it.
          const before = b.r;
          // Slowly, as wax does: a few seconds to drain.
          b.r = Math.max(0, b.r - 0.075 * step);
          into.r = Math.min(MAX_R, Math.sqrt(into.r * into.r + before * before - b.r * b.r));
          b.x += (into.x - b.x) * Math.min(1, step * 0.7);
          b.y += (into.y - b.y) * Math.min(1, step * 0.7);
          if (b.r < 0.03) {
            this.retire(b);
            continue;
          }
        }
      } else if (b.stage === "melting") {
        // Back into the pool: sinking as it shrinks.
        b.r = Math.max(0, b.r - 0.12 * step);
        b.y = Math.max(this.poolTop(b.x, t) - b.r, b.y - 0.15 * step);
        if (b.r < 0.03) {
          this.retire(b);
          continue;
        }
      }
      if (b.stage === "free") this.move(b, t, step, mouse);
      this.outline(b, t);
    }

    // Necks: a budding or melting blob to the pool, a merging one to the blob taking it in.
    const necks: Vector2[][] = [];
    for (const b of this.blobs) {
      if (b.stage === "budding" || b.stage === "melting") {
        const big = b.r * 1.25 + 0.1;
        necks.push(neck(b.x, b.y, b.r * 0.95, b.x, this.poolTop(b.x, t) - big + 0.05, big, REACH));
      } else if (b.stage === "merging" && b.into) {
        necks.push(neck(b.x, b.y, b.r * 0.95, b.into.x, b.into.y, b.into.r * 0.95, REACH));
      }
    }
    for (let i = 0; i < this.necks.length; i++) {
      const n = this.necks[i]!;
      // Only change what changed: an empty neck stays empty.
      if (i < necks.length && necks[i]!.length > 0) {
        const p = necks[i]!;
        const moved: Vector2[] = [];
        for (let k = 0; k < p.length; k++) moved.push(inside(p[k]!.x, p[k]!.y));
        n.points = moved;
      } else if (n.pointCount > 0) n.points = [];
    }

    // The pool at the bottom, heaving with the same field.
    const pool: Vector2[] = [];
    const base = GLASS_BOTTOM + 0.04;
    const w = glassWidth(base) - 0.04;
    for (let i = 0; i <= 28; i++) {
      const x = -w + (2 * w * i) / 28;
      pool.push(new Vector2(x, this.poolTop(x, t)));
    }
    pool.push(new Vector2(w * 0.95, base));
    pool.push(new Vector2(-w * 0.95, base));
    this.pool.points = pool;

    const text = `${two(now.hour)}:${two(now.minute)}`;
    if (text !== this.shown) {
      this.shown = text;
      this.time.text = text;
      this.seconds.text = now.toString("dddd");
    }
  }

  /** A free blob: heat, buoyancy, drift, the mouse and the glass. */
  private move(b: Blob, t: number, step: number, mouse: Vector2 | null): void {
    // Heated near the bottom, cooled near the top: hot wax rises, cool wax sinks. Big blobs
    // hold their heat longer, so they travel further.
    const a = (b.y - GLASS_BOTTOM) / (GLASS_TOP - GLASS_BOTTOM);
    const inertia = 0.35 / b.r;
    b.heat += (a < 0.22 ? 0.3 : a > 0.8 ? -0.4 : -0.03) * step * inertia;
    b.heat = Math.max(0, Math.min(1, b.heat));
    b.vy += ((b.heat - 0.5) * 0.55 - b.vy * 0.7) * step;
    b.vx += (Math.sin(t * 0.3 + b.phase) * 0.06 - b.vx * 0.9) * step;
    if (mouse) {
      const dx = b.x - mouse.x;
      const dy = b.y - mouse.y;
      const dist = Math.hypot(dx, dy);
      if (dist < 1.6 && dist > 1e-3) {
        const push = (1.6 - dist) * 2.4 * step;
        b.vx += (dx / dist) * push;
        b.vy += (dy / dist) * push;
      }
    }
    b.x += b.vx * step;
    b.y += b.vy * step;
    // Kept inside the glass (a hot blob low down sits in the pool, as if growing out of it).
    b.y = Math.max(GLASS_BOTTOM + b.r * 0.35, Math.min(GLASS_TOP - b.r * 0.9, b.y));
    const room = glassWidth(b.y) - b.r * 0.85;
    if (Math.abs(b.x) > room) {
      b.x = Math.sign(b.x) * Math.max(0, room);
      b.vx *= -0.3;
    }
    // Sinking low and cool: it melts back into the pool.
    if (b.vy < 0 && b.heat < 0.3 && b.y - b.r < this.poolTop(b.x, t) + 0.08) b.stage = "melting";
  }

  /** A blob's outline (see the top of the file). */
  private outline(b: Blob, t: number): void {
    // The outline: a circle pushed by the noise field where each point is, stretched with
    // its speed into a drop that trails behind it.
    const dir = Math.sign(b.vy) || 1;
    const tail = Math.min(0.45, Math.abs(b.vy) * 1.4);
    const pts: Vector2[] = [];
    for (let k = 0; k < POINTS; k++) {
      const ang = (k / POINTS) * Math.PI * 2;
      const cx = Math.cos(ang);
      const cy = Math.sin(ang);
      const behind = Math.max(0, -cy * dir);
      const sx = b.x + cx * b.r;
      const sy = b.y + cy * b.r;
      const rr = b.r * (1 + 0.3 * wobble(sx, sy, t + b.phase * 0.1)) * (1 + tail * behind * behind);
      pts.push(inside(b.x + (cx * rr) / (1 + tail * 0.25), b.y + cy * rr));
    }
    b.line.points = pts;
  }
}
