// Things that happen to the whole stage: rain and storms (rain is a CSS overlay drawn over the
// emoji; lightning is a bolt behind it plus a flash over everything), the room blushing red with
// a heartbeat, disco lights and a dropping disco ball, tomatoes thrown from the audience, and a
// pool of water to fall into. The director starts and stops them; `update` runs them.
import { Color, GameObject, LineRenderer2D, Shapes, Vector2, Vector3, WorldPanel } from "@iron/engine";
import { createRoot, Image } from "@iron/react";
import { GROUND, view } from "./config.ts";
import type { Fx } from "./fx.ts";
import { cracks, overlay, type Photo, photos, Shard, splats } from "./hud.tsx";
import type { Ctx } from "./moves.ts";
import { sfx } from "./sfx.ts";
import { Sprites } from "./sprites.tsx";
import type { Stage } from "./stage.tsx";

type Host = Pick<Ctx, "sfx" | "shake" | "smoke" | "jolt">;
type Rgba = [number, number, number, number];

const FIRE: Rgba[] = [[1, 0.85, 0.3, 1], [1, 0.55, 0.12, 1], [1, 0.28, 0.05, 0.9]];
const CONFETTI: Rgba[] = [[1, 0.35, 0.4, 1], [1, 0.85, 0.3, 1], [0.3, 0.8, 1, 1], [0.5, 1, 0.5, 1], [1, 0.5, 0.9, 1], [1, 1, 1, 1]];
const pickOf = <T,>(list: T[]): T => list[Math.floor(Math.random() * list.length)]!;
const colour = (hue: number, sat = 0.75): Rgba => {
  const c = hsv(((hue % 1) + 1) % 1, sat, 1, 1);
  return [c.r, c.g, c.b, 1];
};

interface Rocket {
  x: number;
  tx: number;
  ty: number;
  t: number;
  hue: number;
  kind: "peony" | "ring" | "willow" | "double";
}

interface Shard3 {
  go: GameObject;
  panel: WorldPanel;
  on: boolean;
  t: number;
  x: number;
  y: number;
  vx: number;
  vy: number;
  spin: number;
  size: number;
}

const rnd = (a: number, b: number) => a + Math.random() * (b - a);

function hsv(h: number, s: number, v: number, a: number): Color {
  const k = (n: number) => (n + h * 6) % 6;
  const f = (n: number) => v - v * s * Math.max(0, Math.min(k(n), 4 - k(n), 1));
  return new Color(f(5), f(3), f(1), a);
}

const fill = (go: GameObject, points: Vector2[], color: Color) => go.addComponent(LineRenderer2D, { points, fillColor: color, width: 0, loop: true });
const outline = (go: GameObject, points: Vector2[], color: Color, width: number, loop = true) => go.addComponent(LineRenderer2D, { points, color, width, loop });

interface Tomato {
  go: GameObject;
  on: boolean;
  t: number;
  from: Vector2;
  to: Vector2;
}

export class Effects {
  private fx: Fx;
  private host: Host;
  private stage: Stage;
  private clock = 0;
  /** Emoji flying about as particles and props. */
  readonly sprites = new Sprites(48);
  private abductT = -1;

  private timers: { t: number; run: () => void }[] = [];
  private flash = 0;

  // Weather.
  private storm = false;
  /** Haunted: purple flashes and bolts, and an organ playing. */
  private purple = false;
  private nextStrike = 0;
  private bolts: LineRenderer2D[] = [];
  private boltLife = 0;

  // Disco.
  private mode: "lights" | "ball" | null = null;
  private discoT = 0;
  private beams: { go: GameObject; line: LineRenderer2D; hue: number }[] = [];
  private ball: GameObject;
  private facets: GameObject[] = [];
  private dots: { go: GameObject; line: LineRenderer2D; angle: number; radius: number; hue: number; speed: number }[] = [];

  // Tomatoes.
  private tomatoes: Tomato[] = [];
  private pending = 0;

  // Fireworks, explosion.
  private rockets: Rocket[] = [];
  private shards: Shard3[] = [];
  private wave: GameObject;
  private waveLine: LineRenderer2D;
  private waveT = -1;
  private waveAt = new Vector2(0, 0);

  // Pool.
  private water: GameObject;
  private ripples: { go: GameObject; line: LineRenderer2D }[] = [];
  private poolT = -1;
  private poolX = 0;

  // Night.
  /** 0 spotlit stage .. 1 night sky; the director dims the emoji by it. */
  nightLevel = 0;
  private nightTarget = 0;
  private moon: GameObject;
  private moonHalo: LineRenderer2D[] = [];
  private moonT = -1;
  private moonOut = 0;

  // Fashion shoot.
  private shoot = false;
  private nextPhoto = 0;
  private photoId = 0;

  constructor(fx: Fx, host: Host, stage: Stage) {
    this.fx = fx;
    this.host = host;
    this.stage = stage;

    this.moon = new GameObject("Moon", { position: new Vector3(5.3, 9, -0.6) });
    this.moon.enabled = false;
    const moonPanel = this.moon.addComponent(WorldPanel, { width: 320, height: 320, pixelsPerMeter: 100, resolution: 1, interactive: false });
    createRoot(moonPanel).render(<Image src="assets/emoji/1f319.svg" style={{ width: 320, height: 320 }} />);
    for (const r of [2.0, 2.8, 3.7]) {
      const halo = new GameObject(`Moon halo ${r}`, { parent: this.moon, position: new Vector3(0, 0, -0.1) });
      this.moonHalo.push(fill(halo, Shapes.circle(Vector2.zero, r, 40), new Color(1, 0.95, 0.7, 0)));
    }

    for (let i = 0; i < 2; i++) {
      const go = new GameObject(`Bolt ${i}`, { position: new Vector3(0, 0, -0.8) });
      go.enabled = false;
      this.bolts.push(outline(go, [Vector2.zero, Vector2.zero], i ? new Color(0.7, 0.8, 1, 0.3) : new Color(1, 1, 0.95, 1), i ? 0.5 : 0.14, false));
    }

    for (let i = 0; i < 5; i++) {
      const go = new GameObject(`Beam ${i}`, { position: new Vector3(-6 + i * 3, 5.4, -1.2) });
      go.enabled = false;
      const hue = i / 5;
      const line = fill(go, [Vector2.zero, new Vector2(-1.1, -10), new Vector2(1.1, -10)], hsv(hue, 0.8, 1, 0.1));
      this.beams.push({ go, line, hue });
    }

    // The ball: a silver disc, facet lines that turn, and a string.
    this.ball = new GameObject("Disco ball", { position: new Vector3(0, 7.5, -0.6) });
    this.ball.enabled = false;
    fill(new GameObject("Disc", { parent: this.ball }), Shapes.circle(Vector2.zero, 0.95, 40), new Color(0.72, 0.76, 0.85, 1));
    outline(new GameObject("String", { parent: this.ball }), [new Vector2(0, 0.95), new Vector2(0, 8)], new Color(0.6, 0.6, 0.65, 1), 0.03, false);
    for (let k = 0; k < 6; k++) {
      const facet = new GameObject(`Facet ${k}`, { parent: this.ball });
      outline(facet, Shapes.circle(Vector2.zero, 0.95, 32), new Color(0.35, 0.38, 0.5, 0.9), 0.025);
      this.facets.push(facet);
    }
    for (const y of [-0.5, 0, 0.5]) {
      const half = Math.sqrt(0.95 * 0.95 - y * y);
      outline(new GameObject("Band", { parent: this.ball }), [new Vector2(-half, y), new Vector2(half, y)], new Color(0.35, 0.38, 0.5, 0.9), 0.025, false);
    }
    for (let i = 0; i < 38; i++) {
      const go = new GameObject(`Glint ${i}`, { position: new Vector3(0, 0, -0.9) });
      go.enabled = false;
      const hue = Math.random();
      const line = fill(go, Shapes.circle(Vector2.zero, rnd(0.08, 0.2), 10), hsv(hue, 0.5, 1, 0.8));
      this.dots.push({ go, line, angle: Math.random() * 6.28, radius: rnd(2.5, 11), hue, speed: rnd(0.3, 0.9) * (Math.random() < 0.5 ? -1 : 1) });
    }

    for (let i = 0; i < 4; i++) {
      const go = new GameObject(`Tomato ${i}`, { position: new Vector3(99, 0, 0.6) });
      go.enabled = false;
      const panel = go.addComponent(WorldPanel, { width: 160, height: 160, pixelsPerMeter: 160, resolution: 1, interactive: false });
      createRoot(panel).render(<Image src="assets/emoji/1f345.svg" style={{ width: 160, height: 160 }} />);
      this.tomatoes.push({ go, on: false, t: 0, from: Vector2.zero, to: Vector2.zero });
    }

    this.wave = new GameObject("Shockwave", { position: new Vector3(0, 0, -0.5) });
    this.wave.enabled = false;
    this.waveLine = outline(this.wave, Shapes.circle(Vector2.zero, 1, 48), new Color(1, 0.85, 0.5, 1), 0.22);
    for (let i = 0; i < 8; i++) {
      const go = new GameObject(`Shard ${i}`, { position: new Vector3(99, 0, 0.5) });
      go.enabled = false;
      const panel = go.addComponent(WorldPanel, { width: 120, height: 120, pixelsPerMeter: 120, resolution: 1, interactive: false });
      createRoot(panel).render(<Shard />);
      this.shards.push({ go, panel, on: false, t: 0, x: 0, y: 0, vx: 0, vy: 0, spin: 0, size: 1 });
    }

    this.water = new GameObject("Water", { position: new Vector3(0, GROUND - 0.1, 0.25) });
    this.water.enabled = false;
    fill(this.water, Shapes.ellipse(Vector2.zero, 3.4, 0.62, 48), new Color(0.2, 0.55, 0.95, 0.8));
    for (let i = 0; i < 3; i++) {
      const go = new GameObject(`Ripple ${i}`, { position: new Vector3(0, GROUND - 0.1, 0.26) });
      go.enabled = false;
      this.ripples.push({ go, line: outline(go, Shapes.ellipse(Vector2.zero, 3.4, 0.62, 48), new Color(0.85, 0.95, 1, 0.8), 0.06) });
    }
  }

  // ---- Starting and stopping ----

  weather(kind: "none" | "rain" | "storm"): void {
    const rain = kind !== "none";
    this.storm = kind === "storm";
    this.nextStrike = rnd(0.8, 1.8);
    overlay.set({ ...overlay.get(), rain, storm: this.storm });
    if (rain) sfx.loop("rain", 0.8);
    else sfx.stop("rain", 0.8);
  }

  /** A lightning bolt (down onto `x`, or somewhere random), a white flash and thunder. */
  strike(x?: number): void {
    const target = x ?? rnd(-view.half * 0.7, view.half * 0.7);
    const top = target + rnd(-2, 2);
    const points: Vector2[] = [];
    const steps = 12;
    for (let i = 0; i <= steps; i++) {
      const k = i / steps;
      points.push(new Vector2(top + (target - top) * k + (i === 0 || i === steps ? 0 : rnd(-0.7, 0.7)), 5.5 + (GROUND + 0.1 - 5.5) * k));
    }
    for (const b of this.bolts) {
      b.points = points;
      b.gameObject.enabled = true;
    }
    this.bolts[0]!.color = this.purple ? new Color(0.85, 0.6, 1, 1) : new Color(1, 1, 0.95, 1);
    this.bolts[1]!.color = this.purple ? new Color(0.6, 0.3, 0.95, 0.35) : new Color(0.7, 0.8, 1, 0.3);
    overlay.set({ ...overlay.get(), tint: this.purple ? "#b45cff" : "#ffffff" });
    this.boltLife = 0.22;
    this.flash = 0.9;
    this.host.shake(0.1, 0.5);
    if (x !== undefined) this.host.sfx("zap", 0.8);
    this.later(0.12, () => (this.flash = 0.8));
    this.later(0.35, () => this.host.sfx("thunder", 1));
  }

  /** A haunted storm: rain, dark clouds, purple lightning and thunder, and a pipe organ. */
  haunt(on: boolean): void {
    this.purple = on;
    this.weather(on ? "storm" : "none");
    if (on) {
      this.nextStrike = 0.4;
      sfx.loop("organ", 0.55);
    } else {
      sfx.stop("organ", 0.8);
    }
  }

  /** A quick white flash over everything (a jump scare). */
  flashNow(level: number): void {
    this.flash = Math.max(this.flash, level);
  }

  love(on: boolean): void {
    overlay.set({ ...overlay.get(), love: on });
    if (on) sfx.loop("heartbeat", 0.9);
    else sfx.stop("heartbeat", 0.6);
  }

  disco(mode: "lights" | "ball" | null): void {
    this.mode = mode;
    this.discoT = 0;
    if (mode) sfx.loop("disco", 0.55);
    else sfx.stop("disco", 0.6);
    if (!mode) {
      for (const b of this.beams) b.go.enabled = false;
      for (const d of this.dots) d.go.enabled = false;
    }
  }

  /** `count` tomatoes thrown from the audience at the emoji. */
  throwTomatoes(count: number): void {
    this.pending += count;
    for (let i = 0; i < count; i++) this.later(0.3 + i * rnd(0.4, 0.7), () => this.launch());
  }

  pool(x: number): void {
    this.poolX = x;
    this.poolT = 0;
    this.water.enabled = true;
  }

  splash(x: number): void {
    for (let i = 0; i < 34; i++) {
      this.fx.emit({ x: x + rnd(-1.5, 1.5), y: GROUND, vx: rnd(-3, 3), vy: rnd(4, 10), life: rnd(0.5, 0.9), color: [0.55, 0.8, 1, 0.9], len: 0.08, width: 0.12, gravity: 18 });
    }
    for (let i = 0; i < 8; i++) {
      this.fx.emit({ x: x + rnd(-1.6, 1.6), y: GROUND - 0.2, vx: rnd(-0.3, 0.3), vy: rnd(0.8, 2), life: rnd(0.8, 1.4), color: [0.9, 0.97, 1, 0.6], len: 0.01, width: rnd(0.12, 0.25), gravity: -1 });
    }
  }

  /** Big text in the middle of the screen for a moment ("3", "2", "1", "LIFTOFF!"). */
  countdown(text: string): void {
    overlay.set({ ...overlay.get(), countdown: text });
    this.later(1.1, () => {
      if (overlay.get().countdown === text) overlay.set({ ...overlay.get(), countdown: null });
    });
  }

  /** A firework: a rocket climbs to (`x`, `y`) with a whistle and bursts. */
  firework(x: number, y: number, kind?: Rocket["kind"]): void {
    this.host.sfx("whistle", 0.6, rnd(0.9, 1.15));
    this.rockets.push({ x: x + rnd(-1, 1), tx: x, ty: y, t: 0, hue: Math.random(), kind: kind ?? pickOf<Rocket["kind"]>(["peony", "ring", "willow", "double"]) });
  }

  /** Cannons at both sides of the stage fire confetti and streamers; a big one also brings balloons and bunting. */
  celebrate(big: boolean): void {
    for (const side of [-1, 1]) {
      const x = side * (view.half - 0.6);
      this.later(side < 0 ? 0 : 0.12, () => this.host.sfx("pop", 0.8, side < 0 ? 1 : 1.12));
      for (let i = 0; i < (big ? 70 : 26); i++) {
        const a = (rnd(58, 82) * Math.PI) / 180;
        const sp = rnd(9, 15);
        this.fx.emit({ x, y: GROUND - 0.4, vx: -side * Math.cos(a) * sp, vy: Math.sin(a) * sp, life: rnd(1.6, 2.6), color: pickOf(CONFETTI), len: rnd(0.2, 0.4), width: 0.1, gravity: 9, drag: 0.55 });
      }
      for (let i = 0; i < (big ? 24 : 8); i++) {
        const a = (rnd(52, 85) * Math.PI) / 180;
        const sp = rnd(8, 13);
        this.fx.emit({ x, y: GROUND - 0.4, vx: -side * Math.cos(a) * sp, vy: Math.sin(a) * sp, life: rnd(2, 3), color: pickOf(CONFETTI), len: rnd(0.7, 1.2), width: 0.09, gravity: 2.4, drag: 1.5 });
      }
    }
    if (big) {
      overlay.set({ ...overlay.get(), party: true });
      this.later(6, () => overlay.set({ ...overlay.get(), party: false }));
    }
  }

  /** The emoji blows up: fireball, smoke, a shockwave, sparks and pieces of itself. */
  explode(x: number, y: number): void {
    this.host.sfx("explosion", 1);
    this.host.shake(0.45, 1.3);
    this.flash = 1;
    for (let i = 0; i < 70; i++) {
      const a = rnd(0, Math.PI * 2);
      const sp = rnd(1.5, 8);
      this.fx.emit({ x: x + rnd(-0.5, 0.5), y: y + rnd(-0.5, 0.5), vx: Math.cos(a) * sp, vy: Math.sin(a) * sp, life: rnd(0.4, 0.9), color: pickOf(FIRE), len: 0.01, width: rnd(0.4, 0.9), drag: 2.2 });
    }
    for (let i = 0; i < 40; i++) {
      this.fx.emit({ x: x + rnd(-1, 1), y: y + rnd(-1, 1), vx: rnd(-2.5, 2.5), vy: rnd(0.5, 3), life: rnd(1.2, 2), color: [0.18, 0.17, 0.17, 0.7], len: 0.01, width: rnd(0.5, 1), gravity: -0.6, drag: 1 });
    }
    for (let i = 0; i < 46; i++) {
      const a = rnd(0, Math.PI * 2);
      const sp = rnd(6, 13);
      this.fx.emit({ x, y, vx: Math.cos(a) * sp, vy: Math.sin(a) * sp, life: rnd(0.5, 1.1), color: pickOf(FIRE), len: rnd(0.25, 0.5), width: 0.08, gravity: 12 });
    }
    this.waveAt = new Vector2(x, y);
    this.waveT = 0;
    this.wave.enabled = true;
    for (const sh of this.shards) {
      const a = rnd(0.2, Math.PI - 0.2);
      const sp = rnd(4, 9);
      Object.assign(sh, { on: true, t: 0, x, y, vx: Math.cos(a) * sp * (Math.random() < 0.5 ? -1 : 1), vy: Math.sin(a) * sp + 3, spin: rnd(-700, 700), size: rnd(0.45, 0.8) });
      sh.go.enabled = true;
    }
  }

  /** The picture breaks into pieces that fall away (the glassy cousin of `explode`). */
  shatter(x: number, y: number): void {
    for (const sh of this.shards) {
      const a = rnd(0.3, Math.PI - 0.3);
      const sp = rnd(2, 5);
      Object.assign(sh, { on: true, t: 0, x: x + rnd(-0.6, 0.6), y: y + rnd(-0.6, 0.6), vx: Math.cos(a) * sp * (Math.random() < 0.5 ? -1 : 1), vy: Math.sin(a) * sp + 1, spin: rnd(-400, 400), size: rnd(0.35, 0.6) });
      sh.go.enabled = true;
    }
    for (let i = 0; i < 16; i++) {
      const a = rnd(0, Math.PI * 2);
      const sp = rnd(2, 6);
      this.sprites.emit({ code: "2728", x, y, vx: Math.cos(a) * sp, vy: Math.sin(a) * sp, drag: 1.2, scale: [0.3, 0.6], life: rnd(0.6, 1.1) });
    }
  }

  /**
   * Where the saucer is at `T` seconds into an abduction (null once it's gone): it drops in
   * (high, so the emoji rising to meet it doesn't overlap it), hangs level, then flies off,
   * tipping from level to its flight angle as it goes. Its picture is drawn tilted about 14
   * degrees clockwise, which the 14 here takes out.
   */
  private shipAt(T: number): { x: number; y: number; scale: number; rot: number } | null {
    if (T > 4.5) return null;
    const drop = 1 - (1 - Math.min(1, T / 0.7)) ** 3;
    const go = Math.min(1, Math.max(0, (T - 3.4) / 0.9));
    const tip = go * go * (3 - 2 * go);
    return { x: 15 * go * go, y: 5.8 - 2.6 * drop + 0.05 * Math.sin(T * 4) + 3 * go * go, scale: 0.8 + 1.8 * drop, rot: 14 - 26 * tip };
  }

  /**
   * An abduction (about five seconds): a flying saucer drops into view and its blue beam hangs
   * from its underside down over the emoji; when the emoji is in, the beam thins to a column and
   * is absorbed back into the saucer as it flies off; the spotlight then comes back.
   */
  ufo(): void {
    this.abductT = 0;
    this.sprites.emit({
      code: "1f6f8-ship",
      x: 0,
      y: 5.5,
      scale: 0.8,
      life: 5,
      z: -0.2,
      anim: (t) => {
        const p = this.shipAt(t);
        return p ? { x: p.x, y: p.y, scale: p.scale, rot: p.rot } : null;
      },
    });
  }

  private updateAbduct(dt: number): void {
    if (this.abductT < 0) return;
    this.abductT += dt;
    const T = this.abductT;
    const sm = (v: number) => {
      const c = Math.min(1, Math.max(0, v));
      return c * c * (3 - 2 * c);
    };
    const ship = this.shipAt(T);
    if (ship) {
      // Hangs from the saucer's underside (a little left of the middle of its picture).
      const arrive = sm(T / 0.7);
      const from = { x: ship.x - 0.03 * ship.scale, y: ship.y - 0.11 * ship.scale, top: 0.7 + (0.35 * ship.scale - 0.7) * arrive };
      const blue = sm(T / 0.9) * (1 - sm((T - 4) / 0.4));
      // Opens as it drops in, snaps to a thin column as the saucer leaves, and is drawn up into it.
      const width = (1 - 0.35 * arrive) * (T < 3.4 ? 1 : 1 - 0.85 * sm((T - 3.4) / 0.15));
      const reach = T < 3.55 ? 1 : 1 - sm((T - 3.55) / 0.75);
      this.stage.setBeam(blue, width, from, reach);
    } else {
      // The saucer has gone: the spotlight comes back.
      this.stage.setBeam(0, sm((T - 4.6) / 0.5));
    }
    if (T > 5.2) {
      this.stage.setBeam(0, 1);
      this.abductT = -1;
    }
  }

  /** The spotlight fades away into a night sky: stars, a moon that pops down, crickets and an owl's hoot. */
  night(on: boolean): void {
    this.nightTarget = on ? 1 : 0;
    if (on) {
      this.moonT = 0;
      this.moonOut = 0;
      this.moon.enabled = true;
      sfx.loop("crickets", 0.35);
      this.later(0.55, () => this.host.sfx("boing", 0.6, 1.5));
      this.later(1.2, () => this.host.sfx("hoot", 0.9));
      this.later(4.2, () => this.host.sfx("hoot", 0.7, 0.93));
    } else {
      this.moonOut = 1;
      sfx.stop("crickets", 1);
    }
  }

  /** A fashion shoot: a groove and camera flashes popping all over (more when `shots` is asked for). */
  fashion(on: boolean): void {
    this.shoot = on;
    this.nextPhoto = 0.6;
    if (on) sfx.loop("runway", 0.5);
    else sfx.stop("runway", 0.8);
  }

  /** `n` flashes in quick succession: the shutter, a white flash and starbursts on the screen. */
  photos(n: number): void {
    for (let i = 0; i < n; i++) {
      this.later(i * 0.08, () => {
        this.host.sfx("shutter", 0.6, rnd(0.92, 1.12));
        this.flash = Math.max(this.flash, 0.16);
        const shot: Photo = { id: ++this.photoId, left: `${rnd(3, 90)}%`, top: `${rnd(12, 80)}%`, size: rnd(140, 260) };
        photos.set([...photos.get(), shot]);
        this.later(0.35, () => photos.set(photos.get().filter((p) => p.id !== shot.id)));
      });
    }
  }

  /** Everything off, ready for the next act. */
  clear(): void {
    this.sprites.clear();
    cracks.set(0);
    this.abductT = -1;
    this.stage.setBeam(0, 1);
    this.night(false);
    this.nightLevel = 0;
    this.nightTarget = 0;
    this.moon.enabled = false;
    this.moonT = -1;
    this.stage.setNight(0, 0);
    this.fashion(false);
    photos.set([]);
    this.weather("none");
    this.love(false);
    this.disco(null);
    this.storm = false;
    this.timers = [];
    this.pending = 0;
    this.flash = 0;
    overlay.set({ rain: false, storm: false, love: false, party: false, flash: 0, tint: "#ffffff", countdown: null });
    this.purple = false;
    this.rockets = [];
    for (const sh of this.shards) {
      sh.on = false;
      sh.go.enabled = false;
    }
    this.wave.enabled = false;
    this.waveT = -1;
    splats.set([]);
    for (const t of this.tomatoes) {
      t.on = false;
      t.go.enabled = false;
    }
    for (const b of this.bolts) b.gameObject.enabled = false;
    this.ball.enabled = false;
    this.water.enabled = false;
    for (const r of this.ripples) r.go.enabled = false;
    this.poolT = -1;
  }

  // ---- Running ----

  private later(t: number, run: () => void): void {
    this.timers.push({ t, run });
  }

  private launch(): void {
    const tomato = this.tomatoes.find((x) => !x.on);
    this.pending--;
    if (!tomato) return;
    const side = Math.random() < 0.5 ? -1 : 1;
    tomato.on = true;
    tomato.t = 0;
    tomato.from = new Vector2(side * (view.half + 1), rnd(-6.5, -4.5));
    tomato.to = new Vector2(rnd(-1.2, 1.2), 0);
    tomato.go.enabled = true;
  }

  update(dt: number, at: { x: number; y: number }): void {
    for (const t of [...this.timers]) {
      t.t -= dt;
      if (t.t <= 0) {
        this.timers.splice(this.timers.indexOf(t), 1);
        t.run();
      }
    }

    if (this.storm) {
      this.nextStrike -= dt;
      if (this.nextStrike <= 0) {
        this.strike();
        this.nextStrike = this.purple ? rnd(1.2, 2.6) : rnd(2, 4.5);
      }
    }
    if (this.boltLife > 0) {
      this.boltLife -= dt;
      if (this.boltLife <= 0) for (const b of this.bolts) b.gameObject.enabled = false;
    }
    const ov = overlay.get();
    if (this.flash > 0 || ov.flash > 0) {
      this.flash = Math.max(0, this.flash - dt * 3.2);
      if (Math.abs(ov.flash - this.flash) > 0.01 || (this.flash === 0 && ov.flash !== 0)) overlay.set({ ...ov, flash: this.flash });
    }

    this.updateDisco(dt);
    this.updateTomatoes(dt, at);
    this.updatePool(dt);
    this.updateRockets(dt);
    this.updateBlast(dt);
    this.updateNight(dt);
    this.sprites.update(dt);
    this.updateAbduct(dt);
    if (this.shoot) {
      this.nextPhoto -= dt;
      if (this.nextPhoto <= 0) {
        this.photos(1 + Math.floor(Math.random() * 3));
        this.nextPhoto = rnd(0.35, 1.1);
      }
    }
  }

  private updateNight(dt: number): void {
    this.clock += dt;
    const was = this.nightLevel;
    const step = dt / 1.4;
    this.nightLevel = was < this.nightTarget ? Math.min(this.nightTarget, was + step) : Math.max(this.nightTarget, was - step);
    if (this.nightLevel > 0 || was > 0) this.stage.setNight(this.nightLevel, this.clock);
    if (!this.moon.enabled) return;

    // The moon pops down on to the sky with a bounce, and goes back up when the day returns.
    let y: number;
    if (this.moonOut) {
      y = this.moon.transform.position.y + dt * 7;
      if (y > 9) this.moon.enabled = false;
    } else {
      this.moonT += dt;
      const f = Math.min(1, Math.max(0, (this.moonT - 0.5) / 0.9));
      const settle = f >= 1 ? -0.4 * Math.exp(-(this.moonT - 1.4) * 3) * Math.sin((this.moonT - 1.4) * 11) : 0;
      y = 1.7 + 7.3 * (1 - f) ** 2 + settle;
    }
    this.moon.transform.position = new Vector3(5.3, y, -0.6);
    this.moon.transform.localEulerAngles = new Vector3(0, 0, 6 * Math.sin(this.clock * 1.1));
    const glow = this.nightLevel;
    this.moonHalo.forEach((h, i) => (h.fillColor = new Color(1, 0.95, 0.7, glow * [0.08, 0.06, 0.04][i]!)));
  }

  private updateRockets(dt: number): void {
    for (const r of [...this.rockets]) {
      r.t += dt / 0.65;
      const e = 1 - (1 - Math.min(1, r.t)) ** 2;
      const x = r.x + (r.tx - r.x) * e;
      const y = GROUND + (r.ty - GROUND) * e;
      for (let i = 0; i < 2; i++) {
        this.fx.emit({ x, y, vx: rnd(-0.5, 0.5), vy: rnd(-2.5, -0.5), life: 0.4, color: [1, 0.85, 0.5, 0.9], len: 0.12, width: 0.07, gravity: 2 });
      }
      if (r.t >= 1) {
        this.rockets.splice(this.rockets.indexOf(r), 1);
        this.burstRocket(r);
      }
    }
  }

  private burstRocket(r: Rocket): void {
    const spray = (n: number, speed: [number, number], life: [number, number], gravity: number, drag: number, hue: number, even = false, gold = false) => {
      for (let i = 0; i < n; i++) {
        const a = even ? (i / n) * Math.PI * 2 : rnd(0, Math.PI * 2);
        const sp = rnd(speed[0], speed[1]) * 1.2;
        this.fx.emit({
          x: r.tx,
          y: r.ty,
          vx: Math.cos(a) * sp,
          vy: Math.sin(a) * sp,
          life: rnd(life[0], life[1]),
          color: gold ? [1, 0.8, 0.35, 1] : colour(hue + rnd(-0.04, 0.04)),
          len: 0.26,
          width: 0.11,
          gravity,
          drag,
        });
      }
    };
    if (r.kind === "peony") spray(72, [5, 8.5], [1.3, 1.9], 3.5, 1.1, r.hue);
    else if (r.kind === "ring") spray(46, [7.2, 7.2], [1.3, 1.8], 3.5, 1.1, r.hue, true);
    else if (r.kind === "willow") spray(64, [2.5, 6], [2.2, 3], 2, 0.5, 0, false, true);
    else {
      spray(56, [5, 8], [1.3, 1.9], 3.5, 1.1, r.hue);
      spray(34, [4.2, 4.2], [1.3, 1.8], 3.5, 1.1, r.hue + 0.5, true);
    }
    this.host.sfx("boom", 0.85, rnd(0.9, 1.1));
    this.later(0.3, () => this.host.sfx("crackle", 0.5, rnd(0.9, 1.2)));
    this.host.shake(0.05, 0.3);
    this.flash = Math.max(this.flash, 0.18);
  }

  private updateBlast(dt: number): void {
    if (this.waveT >= 0) {
      this.waveT += dt;
      const u = this.waveT / 0.6;
      if (u >= 1) {
        this.wave.enabled = false;
        this.waveT = -1;
      } else {
        const sc = 0.4 + 14 * (1 - (1 - u) ** 2);
        this.wave.transform.position = new Vector3(this.waveAt.x, this.waveAt.y, -0.5);
        this.wave.transform.localScale = new Vector3(sc, sc, 1);
        this.waveLine.color = new Color(1, 0.85, 0.5, 1 - u);
      }
    }
    for (const sh of this.shards) {
      if (!sh.on) continue;
      sh.t += dt;
      sh.vy -= 14 * dt;
      sh.x += sh.vx * dt;
      sh.y += sh.vy * dt;
      if (sh.t > 1.9) {
        sh.on = false;
        sh.go.enabled = false;
        continue;
      }
      sh.go.transform.position = new Vector3(sh.x, sh.y, 0.5);
      sh.go.transform.localEulerAngles = new Vector3(0, 0, sh.spin * sh.t);
      sh.go.transform.localScale = new Vector3(sh.size, sh.size, 1);
      const a = Math.min(1, Math.max(0, 1 - (sh.t - 1.0) / 0.8));
      sh.panel.tint = new Color(1, 1, 1, a);
    }
  }

  private updateDisco(dt: number): void {
    if (!this.mode) {
      if (this.ball.enabled) {
        // The ball is hoisted back up when the lights go.
        const p = this.ball.transform.position;
        const y = p.y + dt * 6;
        this.ball.transform.position = new Vector3(0, y, p.z);
        if (y > 9) this.ball.enabled = false;
      }
      return;
    }
    this.discoT += dt;
    const t = this.discoT;
    const beat = (t % 0.5) / 0.5;
    if (this.mode === "lights") {
      this.beams.forEach((b, i) => {
        const on = t > i * 0.5 + 0.1;
        b.go.enabled = on;
        if (!on) return;
        b.go.transform.localEulerAngles = new Vector3(0, 0, 18 * Math.sin(t * 0.9 + i * 1.3));
        b.line.fillColor = hsv((b.hue + t * 0.07) % 1, 1, 1, 0.14 + 0.3 * Math.exp(-beat * 6));
      });
    }
    const showBall = this.mode === "ball";
    const dotsOn = showBall || (this.mode === "lights" && t > 1);
    this.ball.enabled = showBall;
    if (showBall) {
      // Drops in on its string with a springy overshoot.
      const fall = Math.min(1, t / 0.9);
      const settle = fall >= 1 ? -0.35 * Math.exp(-(t - 0.9) * 3) * Math.sin((t - 0.9) * 12) : 0;
      const y = 2.3 + 5.3 * (1 - fall) ** 2 + settle;
      this.ball.transform.position = new Vector3(0, y, -0.6);
      this.facets.forEach((f, k) => (f.transform.localScale = new Vector3(Math.abs(Math.cos(t * 1.4 + (k * Math.PI) / 6)), 1, 1)));
    }
    const baseY = showBall ? this.ball.transform.position.y : 3;
    this.dots.forEach((d) => {
      d.go.enabled = dotsOn;
      if (!dotsOn) return;
      const a = d.angle + t * d.speed;
      d.go.transform.position = new Vector3(Math.cos(a) * d.radius, Math.max(-4.4, Math.min(4.8, baseY - 1 + Math.sin(a) * d.radius * 0.5)), -0.9);
      d.line.fillColor = hsv((d.hue + t * 0.1) % 1, 0.55, 1, 0.35 + 0.45 * Math.abs(Math.sin(t * 5 + d.angle * 9)));
    });
  }

  private updateTomatoes(dt: number, at: { x: number; y: number }): void {
    for (const t of this.tomatoes) {
      if (!t.on) continue;
      t.t += dt / 0.7;
      const u = Math.min(1, t.t);
      const target = new Vector2(at.x + t.to.x, at.y + t.to.y);
      const x = t.from.x + (target.x - t.from.x) * u;
      const y = t.from.y + (target.y - t.from.y) * u + 2.8 * Math.sin(Math.PI * u);
      t.go.transform.position = new Vector3(x, y, 0.6);
      t.go.transform.localEulerAngles = new Vector3(0, 0, u * 900);
      if (t.t >= 1) {
        t.on = false;
        t.go.enabled = false;
        this.splat(target.x, target.y);
      }
    }
  }

  private splat(x: number, y: number): void {
    this.host.sfx("splat", 1, rnd(0.9, 1.15));
    this.host.smoke(x, y, 16, [0.85, 0.12, 0.08, 0.85]);
    this.host.jolt(1);
    const list = splats.get();
    // Where it landed on the picture, in the panel's pixels.
    list.push({ x: rnd(120, 360), y: rnd(100, 340), size: rnd(90, 160), rot: rnd(0, 360) });
    splats.set([...list]);
  }

  private updatePool(dt: number): void {
    if (this.poolT < 0) return;
    this.poolT += dt;
    const grow = Math.min(1, this.poolT / 0.4);
    const k = 1 - (1 - grow) ** 3;
    this.water.transform.position = new Vector3(this.poolX, GROUND - 0.1, 0.25);
    this.water.transform.localScale = new Vector3(k, k, 1);
    this.ripples.forEach((r, i) => {
      const phase = (this.poolT * 0.5 + i / 3) % 1;
      r.go.enabled = this.poolT > 0.3;
      r.go.transform.position = new Vector3(this.poolX, GROUND - 0.1, 0.26);
      const s = 0.25 + 0.75 * phase;
      r.go.transform.localScale = new Vector3(s * k, s * k, 1);
      r.line.color = new Color(0.85, 0.95, 1, 0.8 * (1 - phase));
    });
  }
}
