// The lava lamp: wax blobs rise when they're hot, cool at the top and sink, wobbling as they
// go. Each blob is a closed spline whose knots breathe, filled in one colour brighter than
// white, so blobs that touch melt into one shape and glow through bloom. They're reactive: the
// mouse pushes them away. The time glows beside the lamp.
import { type Camera, Color, type DateTime, GameObject, input, type LineRenderer2D, RenderSettings, Shapes, Spline, type TextMesh, Vector2, Vector3 } from "@iron/engine";

import { type Face, label, mix, obj, shape, two, under } from "./common.ts";

const LAMP = new Vector3(-2.6, 0, 0);
const GLASS_TOP = 3.4;
const GLASS_BOTTOM = -2.6;
const LAVA = new Color(1.45, 0.3, 0.07, 1);
const LAVA_EDGE = new Color(1.8, 0.55, 0.12, 1);
const LIQUID = Color.hex("#2a0f2e");
const METAL = Color.hex("#3a3d45");
const METAL_EDGE = Color.hex("#8b909c");
const BACK = Color.hex("#0b0710");

/** The glass's half-width at height `y` (lamp space): narrow at the top, widest low down. */
const glassWidth = (y: number) => {
  const a = (y - GLASS_BOTTOM) / (GLASS_TOP - GLASS_BOTTOM);
  return 0.62 + 0.72 * Math.sin(Math.PI * Math.min(1, Math.max(0, a * 0.82 + 0.08)));
};

interface Blob {
  line: LineRenderer2D;
  spline: Spline;
  x: number;
  y: number;
  vx: number;
  vy: number;
  r: number;
  heat: number;
  phase: number;
}

export class LavaFace implements Face {
  readonly name = "Lava lamp";
  readonly root = new GameObject("Lava lamp face");
  private blobs: Blob[] = [];
  private pool!: LineRenderer2D;
  private camera: Camera | null = null;
  private background: Color | null = null;
  private time!: TextMesh;
  private seconds!: TextMesh;
  private shown = "";

  constructor() {
    under(this.root, () => this.build());
  }

  private build(): void {
    // The glass, filled with the dark liquid, then the cap and the base over it.
    const right: Vector2[] = [];
    for (let i = 0; i <= 40; i++) {
      const y = GLASS_BOTTOM + ((GLASS_TOP - GLASS_BOTTOM) * i) / 40;
      right.push(new Vector2(glassWidth(y), y));
    }
    const outline = [...right, ...right.slice().reverse().map((p) => new Vector2(-p.x, p.y))];
    shape("liquid", { points: outline, loop: true, width: 0.05, color: mix(LIQUID, LAVA_EDGE, 0.08), fillColor: LIQUID, sortingOrder: 0 }, LAMP);
    // The wax pooled at the bottom.
    this.pool = shape("pool", { loop: true, width: 0, fillColor: LAVA, sortingOrder: 1 }, LAMP);
    for (let i = 0; i < 7; i++) {
      const knots = Array.from({ length: 10 }, () => Vector3.zero);
      this.blobs.push({
        // Fill only: blobs that touch melt into one shape.
        line: shape(`blob ${i}`, { loop: true, width: 0, fillColor: LAVA, sortingOrder: 1 }, LAMP),
        spline: new Spline(knots, { closed: true }),
        x: (Math.random() - 0.5) * 0.8,
        y: GLASS_BOTTOM + 0.6 + Math.random() * 4.5,
        vx: 0,
        vy: 0,
        r: 0.28 + Math.random() * 0.3,
        heat: Math.random(),
        phase: Math.random() * 10,
      });
    }
    shape("cap", { points: [new Vector2(-0.62, 0), new Vector2(0.62, 0), new Vector2(0.4, 1.0), new Vector2(-0.4, 1.0)], loop: true, width: 0.04, color: METAL_EDGE, fillColor: METAL, sortingOrder: 5 }, LAMP.add(new Vector3(0, GLASS_TOP - 0.02, 0)));
    shape("base", { points: [new Vector2(-0.95, 0), new Vector2(0.95, 0), new Vector2(1.35, -1.55), new Vector2(-1.35, -1.55)], loop: true, width: 0.04, color: METAL_EDGE, fillColor: METAL, sortingOrder: 5 }, LAMP.add(new Vector3(0, GLASS_BOTTOM + 0.02, 0)));
    this.time = label("time", { fontSize: 96, fontWeight: 700, color: LAVA_EDGE }, new Vector3(2.4, 0.5, 0));
    this.seconds = label("seconds", { fontSize: 30, color: new Color(0.95, 0.3, 0.14, 1) }, new Vector3(2.4, -0.6, 0));
  }

  show(camera: Camera): void {
    this.camera = camera;
    this.background = camera.backgroundColor;
    camera.backgroundColor = BACK;
    RenderSettings.bloom = { threshold: 0.65, intensity: 1.3, scatter: 0.78 };
  }

  hide(camera: Camera): void {
    if (this.background) camera.backgroundColor = this.background;
    RenderSettings.bloom = null;
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
    for (const b of this.blobs) {
      // Heated near the bottom, cooled near the top: hot wax rises, cool wax sinks.
      const a = (b.y - GLASS_BOTTOM) / (GLASS_TOP - GLASS_BOTTOM);
      b.heat += (a < 0.25 ? 0.35 : a > 0.8 ? -0.45 : -0.04) * step;
      b.heat = Math.max(0, Math.min(1, b.heat));
      b.vy += ((b.heat - 0.5) * 0.9 - b.vy * 0.8) * step;
      b.vx += (Math.sin(t * 0.4 + b.phase) * 0.08 - b.vx * 0.9) * step;
      if (mouse) {
        const d = new Vector2(b.x - mouse.x, b.y - mouse.y);
        const dist = d.magnitude;
        if (dist < 1.6 && dist > 1e-3) {
          const push = (1.6 - dist) * 2.4 * step;
          b.vx += (d.x / dist) * push;
          b.vy += (d.y / dist) * push;
        }
      }
      b.x += b.vx * step;
      b.y += b.vy * step;
      // Kept inside the glass.
      b.y = Math.max(GLASS_BOTTOM + b.r * 0.6, Math.min(GLASS_TOP - b.r, b.y));
      const room = glassWidth(b.y) - b.r * 0.9;
      if (Math.abs(b.x) > room) {
        b.x = Math.sign(b.x) * room;
        b.vx *= -0.3;
      }
      // Its outline breathes and stretches with its speed.
      const stretch = 1 + Math.min(0.35, Math.abs(b.vy) * 0.6);
      for (let k = 0; k < b.spline.count; k++) {
        const ang = (k / b.spline.count) * Math.PI * 2;
        const wobble = 1 + 0.12 * Math.sin(t * 1.7 + b.phase + k * 1.9) + 0.06 * Math.sin(t * 3.1 + k * 0.7);
        const rr = b.r * wobble;
        b.spline.setPosition(k, new Vector3(b.x + (Math.cos(ang) * rr) / Math.sqrt(stretch), b.y + Math.sin(ang) * rr * stretch, 0));
      }
      b.line.points = b.spline.sample(6);
    }
    // The pool at the bottom, gently heaving.
    const pool: Vector2[] = [];
    const base = GLASS_BOTTOM + 0.04;
    const w = glassWidth(base) - 0.04;
    for (let i = 0; i <= 20; i++) {
      const x = -w + (2 * w * i) / 20;
      pool.push(new Vector2(x, base + 0.28 + 0.07 * Math.sin(t * 1.3 + i * 0.8)));
    }
    pool.push(new Vector2(w * 0.9, base), new Vector2(-w * 0.9, base));
    this.pool.points = pool;

    const text = `${two(now.hour)}:${two(now.minute)}`;
    if (text !== this.shown) {
      this.shown = text;
      this.time.text = text;
      this.seconds.text = now.toString("dddd");
    }
  }
}
