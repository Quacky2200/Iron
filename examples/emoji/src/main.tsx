// Emoji celebrity: a spinner wheel picks an emoji, the red curtain opens and it comes on in one
// of nine ways (walking, hopping, rushing, crawling with sparks, peeking from behind the
// curtain, dropping in, rolling, popping up, rising), strikes a pose in the spotlight while its
// name appears, then leaves the same way. About a hundred and five performers; two of them are
// drawn in code and watch the mouse.
//
// The motion is in motion.ts as pure functions of time; this file is the director that turns
// a `Move` into transforms, a shadow, sparks and curtain moves.
import { Camera, Color, Component, GameObject, input, LineRenderer2D, register, Screen, Shapes, Vector2, Vector3, WorldPanel } from "@iron/engine";
import { createRoot } from "@iron/react";
import { CAST, type Star } from "./cast.ts";
import { GROUND, HEIGHT, LIGHT, PANEL, view } from "./config.ts";
import { Fx } from "./fx.ts";
import { caption, eyes, Face, Hud, shown, wheel, type WheelState } from "./hud.tsx";
import { type Gait, GAITS, type Move, POSE_TIME, poseMove } from "./motion.ts";
import { buildStage, Curtain, Mote } from "./stage.tsx";

type Phase = "wheel" | "enter" | "pose" | "exit" | "close";

// Wheel timeline, seconds.
const W_IN = 0.7;
const W_SPIN = 4.6;
const W_GLOW = 1.3;
const W_OUT = 0.6;
const W_TOTAL = W_IN + W_SPIN + W_GLOW + W_OUT;
/** The curtain is drawn back this long before the emoji appears. */
const LEAD = 1;

const pick = <T,>(list: T[]): T => list[Math.floor(Math.random() * list.length)]!;
const clamp01 = (v: number) => Math.min(1, Math.max(0, v));
const smoothstep = (t: number) => {
  const c = clamp01(t);
  return c * c * (3 - 2 * c);
};
type Rgba = [number, number, number, number];
const SPARK: Rgba[] = [[1, 0.85, 0.3, 1], [1, 0.6, 0.15, 1], [1, 0.95, 0.7, 1]];
const CONFETTI: Rgba[] = [[1, 0.35, 0.4, 1], [1, 0.85, 0.3, 1], [0.3, 0.8, 1, 1], [0.5, 1, 0.5, 1], [1, 0.5, 0.9, 1]];

class Parade extends Component {
  private actor!: GameObject;
  private art!: GameObject;
  private shadow!: GameObject;
  private panel!: WorldPanel;
  private fx!: Fx;
  private curtain!: Curtain;

  private phase: Phase = "wheel";
  private t = 0;
  private clock = 0;
  private star: Star = CAST[0]!;
  private last = -1;
  private gait: Gait = GAITS[0]!;
  private outGait: Gait = GAITS[0]!;
  private pose: Star["poses"][number] = "bow";
  private side = 1;
  private outSide = -1;
  private shownCode = "";

  // Wheel.
  private items: Star[] = [];
  private winner = 0;
  private finalAngle = 0;
  private lastPeg = 0;
  private flapAge = 9;

  // Motion.
  private heading = 1;
  private px = 0;
  private carry = 0;
  private wasThump = 0;
  private wasPuff = false;
  private lit = -1;
  private litAlpha = -1;

  // Eyes.
  private mouse = new Vector2(-1, -1);
  private idle = 9;
  private look = { x: 0, y: 0 };
  private blinkIn = 2;
  private blinkAge = 9;
  private blinks = 0;

  override onAwake(): void {
    view.half = 5 * Screen.aspect;
    new GameObject("Main Camera", { position: new Vector3(0, 0, 10) }).addComponent(Camera, {
      orthographic: true,
      orthographicSize: 5,
      backgroundColor: Color.hex("#04050a"),
    });
    buildStage();

    this.shadow = new GameObject("Shadow", { position: new Vector3(0, GROUND, 0.1) });
    this.shadow.addComponent(LineRenderer2D, { points: Shapes.ellipse(Vector2.zero, 2.4, 0.35, 40), fillColor: new Color(0, 0, 0, 0.45), width: 0, loop: true });

    this.actor = new GameObject("Emoji", { position: new Vector3(99, GROUND, 0) });
    this.art = new GameObject("Art", { position: new Vector3(0, HEIGHT / 2, 0), parent: this.actor });
    this.panel = this.art.addComponent(WorldPanel, { width: PANEL, height: PANEL, pixelsPerMeter: PANEL / HEIGHT, resolution: 1.5, interactive: false });
    createRoot(this.panel).render(<Face />);

    this.fx = new Fx(90);
    this.curtain = new Curtain();
    createRoot().render(<Hud />);
    this.beginWheel();
  }

  // ---- The wheel ----

  private beginWheel(): void {
    this.phase = "wheel";
    this.t = 0;
    this.actor.enabled = false;
    this.shadow.enabled = false;
    caption.set(null);

    // Eight different performers, one of them the winner (never the one just seen).
    const pool = CAST.map((_, i) => i).filter((i) => i !== this.last);
    this.items = [];
    while (this.items.length < 8) {
      const k = Math.floor(Math.random() * pool.length);
      this.items.push(CAST[pool.splice(k, 1)[0]!]!);
    }
    this.winner = Math.floor(Math.random() * 8);
    this.last = CAST.indexOf(this.items[this.winner]!);
    // Segment i sits i*45 degrees round; turning the wheel by -i*45 brings it under the pointer.
    this.finalAngle = 360 * 5 - this.winner * 45 + (Math.random() - 0.5) * 24;
    this.lastPeg = 0;
    this.flapAge = 9;
    this.drawWheel(0, 0.4, 0, 0);
  }

  private drawWheel(angle: number, scale: number, fade: number, glow: number, flap = 0): void {
    const state: WheelState = { items: this.items.map((s) => s.chip), angle, flap, glow, scale, fade, winner: this.winner };
    wheel.set(state);
  }

  private tickWheel(dt: number): void {
    const t = this.t;
    if (t < W_IN) {
      const e = 1 - (1 - t / W_IN) ** 3;
      this.drawWheel(0, 0.4 + 0.6 * e, e, 0);
    } else if (t < W_IN + W_SPIN) {
      const u = (t - W_IN) / W_SPIN;
      const angle = this.finalAngle * (1 - (1 - u) ** 3.4);
      // A peg every 45 degrees kicks the pointer's flap back.
      const peg = Math.floor((angle + 22.5) / 45);
      if (peg !== this.lastPeg) {
        this.lastPeg = peg;
        this.flapAge = 0;
      }
      this.flapAge += dt;
      this.drawWheel(angle, 1, 1, 0, -28 * Math.exp(-this.flapAge * 13));
    } else if (t < W_IN + W_SPIN + W_GLOW) {
      const g = smoothstep((t - W_IN - W_SPIN) / 0.5);
      this.drawWheel(this.finalAngle, 1, 1, g, 0);
    } else if (t < W_TOTAL) {
      const u = (t - W_IN - W_SPIN - W_GLOW) / W_OUT;
      this.drawWheel(this.finalAngle, 1 + 0.15 * u, 1 - u, 1, 0);
    } else {
      wheel.set(null);
      this.beginAct(this.items[this.winner]!);
    }
  }

  // ---- The act ----

  private beginAct(star: Star): void {
    this.star = star;
    this.gait = pick(GAITS);
    this.outGait = pick(GAITS);
    this.pose = pick(star.poses);
    this.side = Math.random() < 0.5 ? 1 : -1;
    // Usually leaves across the stage; sometimes goes back the way it came.
    this.outSide = Math.random() < 0.7 ? -this.side : this.side;
    this.phase = "enter";
    this.t = -LEAD;
    this.heading = -this.side;
    this.px = 99;
    this.shownCode = star.code;
    shown.set({ code: star.code, look: star.look ?? null });
  }

  override onUpdate(rawDt: number): void {
    const dt = Math.min(rawDt, 0.05);
    view.half = 5 * Screen.aspect;
    this.clock += dt;
    this.t += dt;
    this.fx.update(dt);

    let m: Move;
    let side = this.side;
    switch (this.phase) {
      case "wheel":
        this.tickWheel(dt);
        this.curtain.approach(0, 6, dt);
        return;
      case "enter":
        m = this.gait.at(Math.max(0, this.t));
        if (this.t >= this.gait.dur) {
          this.phase = "pose";
          this.t = 0;
          caption.set(this.star.name);
          if (this.pose === "pump" || this.pose === "spin") this.burst(0, GROUND + HEIGHT * 0.6, 22);
        }
        break;
      case "pose":
        m = poseMove(this.pose, this.t);
        side = 1;
        if (this.t >= POSE_TIME) {
          this.phase = "exit";
          this.t = 0;
          caption.set(null);
        }
        break;
      case "exit":
        // The way in, played backwards, out of the other side (or the same one).
        m = this.outGait.at(Math.max(0, this.outGait.dur - this.t));
        side = this.outSide;
        if (this.t >= this.outGait.dur) {
          this.phase = "close";
          this.t = 0;
          this.actor.enabled = false;
          this.shadow.enabled = false;
        }
        break;
      case "close":
        this.curtain.approach(0, 3, dt);
        if (this.curtain.open < 0.04) this.beginWheel();
        return;
    }

    this.curtain.approach(m.open ?? 1, 5, dt);
    this.apply(m, side, dt);
  }

  private apply(m: Move, side: number, dt: number): void {
    const edge = view.half + HEIGHT / 2 + 0.8;
    const x = m.x !== undefined ? m.x * -side : side * edge * (1 - m.d);
    const live = this.t >= 0 || this.phase !== "enter";
    this.actor.enabled = live;
    this.shadow.enabled = live && m.y > -0.5 && m.alpha > 0.2;

    // Which way it's going decides which way it leans.
    const v = (x - this.px) / Math.max(dt, 1e-4);
    this.px = x;
    const target = v < -0.4 ? 1 : v > 0.4 ? -1 : this.heading;
    this.heading += (target - this.heading) * Math.min(1, dt * 12);
    const dir = v < 0 ? -1 : 1;

    const c = m.centre;
    const half = HEIGHT / 2;
    const t = this.actor.transform;
    t.position = new Vector3(x, GROUND + m.y + half * c, 0);
    t.localScale = new Vector3(m.sx * m.spin, m.sy, 1);
    t.localEulerAngles = new Vector3(0, 0, m.rot * this.heading);
    this.art.transform.localPosition = new Vector3(0, half * (1 - c), 0);

    // Dim outside the light (the dark is drawn behind panels, so the emoji does it itself).
    const spot = Math.exp(-((x - LIGHT.x) * (x - LIGHT.x)) / 9);
    const lit = 0.3 + 0.7 * spot;
    if (Math.abs(lit - this.lit) > 0.004 || Math.abs(m.alpha - this.litAlpha) > 0.004) {
      this.lit = lit;
      this.litAlpha = m.alpha;
      this.panel.tint = new Color(lit, lit, lit, m.alpha);
    }

    // The shadow shrinks and fades as it leaves the floor.
    const spread = Math.min(1.25, Math.max(0.3, 1 - 0.08 * m.y)) * (1 + 0.5 * (1 - m.sy)) * Math.max(0.2, m.sx * m.spin);
    this.shadow.transform.position = new Vector3(x, GROUND, 0.1);
    this.shadow.transform.localScale = new Vector3(spread, spread, 1);

    const code = m.swap ?? this.star.code;
    if (code !== this.shownCode) {
      this.shownCode = code;
      shown.set({ code, look: this.star.look ?? null });
    }
    if (this.star.look) this.watch(x, m.y, dt);

    this.emitFx(m, x, dir, dt);
  }

  // ---- Particles ----

  private emitFx(m: Move, x: number, dir: number, dt: number): void {
    const behind = -dir;
    if (m.fx) {
      this.carry += m.fx.rate * dt;
      while (this.carry >= 1) {
        this.carry -= 1;
        if (m.fx.kind === "sparks") {
          this.fx.emit({
            x: x + behind * (0.8 + Math.random() * 1.6),
            y: GROUND + 0.04,
            vx: behind * (2 + Math.random() * 6),
            vy: 1.5 + Math.random() * 4,
            life: 0.3 + Math.random() * 0.4,
            color: pick(SPARK),
            len: 0.25 + Math.random() * 0.3,
            width: 0.07,
            gravity: 16,
          });
        } else if (m.fx.kind === "dust") {
          this.dust(x + behind * Math.random() * 1.5, behind);
        } else {
          this.fx.emit({
            x: x + behind * (2.4 + Math.random() * 1.4),
            y: GROUND + Math.random() * HEIGHT,
            vx: 0,
            vy: 0,
            angle: behind > 0 ? 180 : 0,
            life: 0.18 + Math.random() * 0.12,
            color: [1, 1, 1, 0.7],
            len: 1.5 + Math.random() * 2.5,
            width: 0.06,
          });
        }
      }
    }
    if (m.thump > 0.5 && this.wasThump <= 0.5) {
      for (let i = 0; i < 8; i++) this.dust(x + (Math.random() - 0.5) * 2.4, i % 2 ? 1 : -1);
    }
    this.wasThump = m.thump;
    if (m.puff && !this.wasPuff) this.burst(x, GROUND + HEIGHT / 2, 34);
    this.wasPuff = m.puff;
  }

  private dust(x: number, way: number): void {
    this.fx.emit({
      x,
      y: GROUND + 0.1,
      vx: way * (0.5 + Math.random() * 2) + (Math.random() - 0.5),
      vy: 0.3 + Math.random() * 1.2,
      life: 0.7 + Math.random() * 0.5,
      color: [0.8, 0.75, 0.65, 0.35],
      len: 0.01,
      width: 0.2 + Math.random() * 0.2,
      gravity: -0.5,
      drag: 1.5,
    });
  }

  /** Confetti shot out of a point. */
  private burst(x: number, y: number, count: number): void {
    for (let i = 0; i < count; i++) {
      const a = Math.random() * Math.PI * 2;
      const s = 4 + Math.random() * 6;
      this.fx.emit({ x, y, vx: Math.cos(a) * s, vy: Math.sin(a) * s, life: 0.7 + Math.random() * 0.5, color: pick(CONFETTI), len: 0.18 + Math.random() * 0.15, width: 0.07, gravity: 9, drag: 0.8 });
    }
  }

  // ---- Eyes ----

  /** Points the pupils at the mouse (or wanders when it's been still), and blinks now and then. */
  private watch(x: number, y: number, dt: number): void {
    const mp = input.mousePosition;
    if (mp.x !== this.mouse.x || mp.y !== this.mouse.y) {
      this.mouse = mp;
      this.idle = 0;
    } else {
      this.idle += dt;
    }
    let tx = (mp.x / Math.max(1, Screen.width) - 0.5) * 10 * Screen.aspect;
    let ty = (0.5 - mp.y / Math.max(1, Screen.height)) * 10;
    if (this.idle > 2.5) {
      tx = Math.sin(this.clock * 0.7) * 6;
      ty = 1 + Math.sin(this.clock * 1.3) * 2.5;
    }
    const dx = tx - x;
    const dy = ty - (GROUND + y + HEIGHT / 2);
    const dist = Math.hypot(dx, dy) || 1;
    const k = Math.min(1, dist / 4);
    const ease = Math.min(1, dt * 14);
    this.look.x += ((dx / dist) * k - this.look.x) * ease;
    this.look.y += ((-dy / dist) * k - this.look.y) * ease;

    this.blinkIn -= dt;
    if (this.blinkIn <= 0) {
      this.blinkAge = 0;
      // One in four is a double blink.
      this.blinks = Math.random() < 0.25 ? 1 : 0;
      this.blinkIn = 1.8 + Math.random() * 3.5;
    }
    this.blinkAge += dt;
    if (this.blinkAge > 0.3 && this.blinks > 0) {
      this.blinks--;
      this.blinkAge = 0;
    }
    const blink = this.blinkAge < 0.2 ? Math.sin((Math.PI * this.blinkAge) / 0.2) : 0;
    const e = eyes.get();
    if (Math.abs(e.px - this.look.x) + Math.abs(e.py - this.look.y) + Math.abs(e.blink - blink) > 0.01) {
      eyes.set({ px: this.look.x, py: this.look.y, blink });
    }
  }
}

register({ Parade, Mote });
