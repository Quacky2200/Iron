// Emoji parade: a spinner wheel picks an emoji, the red curtain opens and it comes on in one
// of thirteen ways, strikes a pose in the spotlight while its name appears, the crowd reacts
// (applause and cheers with cannons and balloons, boos and thrown tomatoes, gasps, aws, laughs), and it leaves in one of
// eighteen. Some have a show of their own (a heart blushes the room red with a heartbeat, the
// poop has a fart and a sad trombone, the party popper a kazoo, the lightning bolt a storm).
// About a hundred and five performers; two of them are drawn in code and watch the mouse.
//
// The motion is in entrances.ts, exits.ts and poses.ts as pure functions of time with cues
// (sounds and effects at set moments); this file is the director that turns a `Move` into
// transforms, a shadow, particles and curtain moves, and runs the cues. Sounds are
// synthesised by tools/make_sfx.py.
import { Camera, Color, Component, GameObject, input, LineRenderer2D, register, Screen, Shapes, Vector2, Vector3, WorldPanel } from "@iron/engine";
import { createRoot } from "@iron/react";
import { CAST, type Star } from "./cast.ts";
import { BIG, GROUND, HEIGHT, LIGHT, PANEL, view } from "./config.ts";
import { Effects } from "./effects.tsx";
import { ENTRANCES } from "./entrances.ts";
import { EXITS } from "./exits.ts";
import { Fx } from "./fx.ts";
import { caption, eyes, Face, Hud, overlay, rocket, shown, wheel, type WheelState } from "./hud.tsx";
import type { Act, Ctx, Cue, Move } from "./moves.ts";
import { POSES } from "./poses.ts";
import { SIGNATURES } from "./signatures.ts";
import { sfx } from "./sfx.ts";
import { randomReaction, randomScene, type Reaction, type Scene, SPECIAL } from "./specials.ts";
import { buildStage, Curtain, Mote, type Stage } from "./stage.tsx";

type Phase = "wheel" | "enter" | "pose" | "exit" | "miss" | "close";

// Wheel timeline, seconds.
const W_IN = 0.7;
const W_SPIN = 4.6;
const W_GLOW = 1.3;
const W_OUT = 0.6;
const W_TOTAL = W_IN + W_SPIN + W_GLOW + W_OUT;
/** The curtain is drawn back this long before the emoji appears. */
const LEAD = 1;

const rnd = (a: number, b: number) => a + Math.random() * (b - a);
const pick = <T,>(list: T[]): T => list[Math.floor(Math.random() * list.length)]!;
const clamp01 = (v: number) => Math.min(1, Math.max(0, v));
const smoothstep = (t: number) => {
  const c = clamp01(t);
  return c * c * (3 - 2 * c);
};
function weighted<T extends { weight?: number }>(list: T[]): T {
  let r = Math.random() * list.reduce((n, a) => n + (a.weight ?? 1), 0);
  for (const a of list) {
    const w = a.weight ?? 1;
    if (w <= 0) continue;
    r -= w;
    if (r <= 0) return a;
  }
  return list[0]!;
}
const named = (list: Act[], name: string): Act | undefined => list.find((a) => a.name === name);

type Rgba = [number, number, number, number];
const SPARK: Rgba[] = [[1, 0.85, 0.3, 1], [1, 0.6, 0.15, 1], [1, 0.95, 0.7, 1]];
const FIRE: Rgba[] = [[1, 0.85, 0.3, 1], [1, 0.55, 0.12, 1], [1, 0.28, 0.05, 0.9]];
const CONFETTI: Rgba[] = [[1, 0.35, 0.4, 1], [1, 0.85, 0.3, 1], [0.3, 0.8, 1, 1], [0.5, 1, 0.5, 1], [1, 0.5, 0.9, 1]];

class Parade extends Component {
  private cam!: GameObject;
  private actor!: GameObject;
  private art!: GameObject;
  private shadow!: GameObject;
  private panel!: WorldPanel;
  private fx!: Fx;
  private curtain!: Curtain;
  private effects!: Effects;
  private stage!: Stage;
  private ctx!: Ctx;

  private phase: Phase = "wheel";
  private t = 0;
  private clock = 0;
  private star: Star = CAST[0]!;
  private last = -1;
  private enterAct!: Act;
  private exitAct!: Act;
  private poseAct!: Act;
  private current!: Act;
  private cues: Cue[] = [];
  private cueAt = 0;
  private scene: Scene | undefined;
  private reaction: Reaction = "none";
  private reactAt = 0.5;
  private reacted = false;
  private side = 1;
  private outSide = -1;
  private shownCode = "";
  private shownMirror = false;
  private big = false;

  // Wheel.
  private items: Star[] = [];
  private winner = 0;
  private finalAngle = 0;
  private lastPeg = 0;
  private flapAge = 9;
  private wheelStage = 0;

  // Motion.
  private heading = 1;
  private px = 0;
  private py = 0;
  private carry = 0;
  private groundCarry = 0;
  private wasThump = 0;
  private wasPuff = false;
  private lastStride: number | null = null;
  private litKey = "";
  private jolt = 0;
  private shakeAmount = 0;
  private shakeLeft = 0;
  private shakeTotal = 1;

  // Eyes.
  private mouse = new Vector2(-1, -1);
  private idle = 9;
  private look = { x: 0, y: 0 };
  private blinkIn = 2;
  private blinkAge = 9;
  private blinks = 0;

  override onAwake(): void {
    view.half = 5 * Screen.aspect;
    this.cam = new GameObject("Main Camera", { position: new Vector3(0, 0, 10) });
    const camera = this.cam.addComponent(Camera, { orthographic: true, orthographicSize: 5, backgroundColor: Color.hex("#04050a") });
    this.stage = buildStage(camera);
    sfx.preload();

    this.shadow = new GameObject("Shadow", { position: new Vector3(0, GROUND, 0.1) });
    this.shadow.addComponent(LineRenderer2D, { points: Shapes.ellipse(Vector2.zero, 2.4, 0.35, 40), fillColor: new Color(0, 0, 0, 0.45), width: 0, loop: true });

    this.actor = new GameObject("Emoji", { position: new Vector3(99, GROUND, 0) });
    this.art = new GameObject("Art", { position: new Vector3(0, HEIGHT / 2, 0), parent: this.actor });
    this.panel = this.art.addComponent(WorldPanel, { width: PANEL, height: PANEL, pixelsPerMeter: PANEL / HEIGHT, resolution: 1.5, interactive: false });
    createRoot(this.panel).render(<Face />);

    this.fx = new Fx(450);
    this.curtain = new Curtain();
    const self = this;
    this.ctx = {
      sfx: (name, volume = 1, rate = 1) => void sfx.play(name, volume, rate),
      stop: (name, fade = 0.3) => sfx.stopPlaying(name, fade),
      shake: (amount, seconds) => {
        if (amount >= self.shakeAmount * (self.shakeLeft / self.shakeTotal)) {
          self.shakeAmount = amount;
          self.shakeLeft = self.shakeTotal = Math.max(0.01, seconds);
        }
      },
      burst: (x, y, count) => self.burst(x, y, count),
      smoke: (x, y, count, color, size) => self.smoke(x, y, count, color, size),
      jolt: (amount) => void (self.jolt = Math.max(self.jolt, amount)),
      at: () => ({ x: self.px, y: GROUND + HEIGHT / 2 + self.py }),
      get scene() {
        return self.effects;
      },
    };
    this.effects = new Effects(this.fx, this.ctx, this.stage);
    createRoot().render(<Hud />);
    this.beginWheel();
  }

  // ---- The wheel ----

  private beginWheel(): void {
    this.phase = "wheel";
    this.t = 0;
    this.wheelStage = 0;
    this.actor.enabled = false;
    this.shadow.enabled = false;
    caption.set(null);
    this.effects.clear();
    sfx.stopAll(0.2);

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
      if (this.wheelStage < 1) {
        this.wheelStage = 1;
        sfx.play("drumroll", 0.7);
      }
      const u = (t - W_IN) / W_SPIN;
      const angle = this.finalAngle * (1 - (1 - u) ** 3.4);
      // A peg every 45 degrees kicks the pointer's flap back, with a tick.
      const peg = Math.floor((angle + 22.5) / 45);
      if (peg !== this.lastPeg) {
        this.lastPeg = peg;
        this.flapAge = 0;
        sfx.play("tick", 0.8, 0.9 + 0.3 * u);
      }
      this.flapAge += dt;
      this.drawWheel(angle, 1, 1, 0, -28 * Math.exp(-this.flapAge * 13));
    } else if (t < W_IN + W_SPIN + W_GLOW) {
      if (this.wheelStage < 2) {
        this.wheelStage = 2;
        sfx.play("ding", 0.9);
        sfx.play("tada", 0.7);
      }
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
    const special = SPECIAL[star.code] ?? {};
    this.star = star;
    this.enterAct = (special.enter && named(ENTRANCES, special.enter)) || weighted(ENTRANCES);
    this.exitAct = (special.exit && named(EXITS, special.exit)) || weighted(EXITS);
    this.reaction = special.react ?? randomReaction();
    // Plain applause and cheering are for taking a bow.
    const plain = this.reaction === "applause" || this.reaction === "cheer";
    const poseName = special.pose ?? (plain ? "bow" : pick(star.poses));
    const pose = (POSES as Record<string, Act>)[poseName] ?? SIGNATURES[poseName] ?? POSES.bow;
    // A longer stay repeats the pose.
    this.poseAct = special.extra ? { ...pose, dur: pose.dur + special.extra, at: (t) => pose.at(t % pose.dur) } : pose;
    this.scene = special.scene ?? (special.calm ? undefined : randomScene());
    this.reactAt = special.reactAt ?? 0.5;
    this.reacted = false;
    this.side = Math.random() < 0.5 ? 1 : -1;
    // Usually leaves across the stage; sometimes goes back the way it came.
    this.outSide = Math.random() < 0.7 ? -this.side : this.side;
    this.heading = -this.side;
    this.px = 99;
    this.py = 0;
    this.shownCode = star.code;
    this.shownMirror = false;
    rocket.set({ turn: 0, flame: 0, power: 0, stars: 1 });
    // The rocket gets a bigger panel (see config.ts).
    this.big = star.code === "1f680";
    this.panel.width = this.panel.height = this.big ? BIG : PANEL;
    shown.set({ code: star.code, look: star.look ?? null, mirror: false, big: this.big });
    this.startPhase("enter", this.enterAct, -LEAD);
  }

  private startPhase(phase: Phase, act: Act | null, t = 0): void {
    this.phase = phase;
    this.t = t;
    if (act) this.current = act;
    this.cues = act?.cues ?? [];
    this.cueAt = 0;
    this.lastStride = null;
  }

  private startScene(): void {
    switch (this.scene) {
      case "rain":
        this.effects.weather("rain");
        break;
      case "storm":
        this.effects.weather("storm");
        break;
      case "love":
        this.effects.love(true);
        break;
      case "lights":
        this.effects.disco("lights");
        break;
      case "ball":
        this.effects.disco("ball");
        break;
      case "night":
        this.effects.night(true);
        break;
      case "haunt":
        this.effects.haunt(true);
        break;
      case "fashion":
        this.effects.fashion(true);
        break;
    }
  }

  /** The crowd's reaction. */
  private react(kind: Reaction): void {
    const { x, y } = this.ctx.at();
    switch (kind) {
      case "applause":
        sfx.play("clap", 0.9);
        break;
      case "cheer":
        sfx.play("cheer", 0.9);
        sfx.play("clap", 0.5);
        break;
      case "party":
        sfx.play("cheer", 0.9);
        sfx.play("clap", 0.5);
        this.burst(x, y, 40);
        this.effects.celebrate(true);
        break;
      case "boo":
        sfx.play("boo", 0.9);
        this.effects.throwTomatoes(3 + Math.floor(Math.random() * 3));
        break;
      case "gasp":
        sfx.play("gasp", 0.9);
        this.jolt = Math.max(this.jolt, 0.6);
        break;
      case "aww":
        sfx.play("aww", 0.9);
        break;
      case "laugh":
        sfx.play("laugh", 0.9);
        break;
    }
  }

  override onUpdate(rawDt: number): void {
    const dt = Math.min(rawDt, 0.05);
    view.half = 5 * Screen.aspect;
    this.clock += dt;
    this.t += dt;
    this.fx.update(dt);
    this.effects.update(dt, { x: this.px, y: GROUND + HEIGHT / 2 + this.py });
    this.shakeCamera(dt);

    let m: Move;
    let side = this.side;
    switch (this.phase) {
      case "wheel":
        this.tickWheel(dt);
        this.curtain.approach(0, 6, dt);
        return;
      case "enter":
        m = this.enterAct.at(Math.max(0, this.t));
        this.runCues();
        if (this.t >= this.enterAct.dur) {
          if (this.enterAct.miss) {
            // It rolled straight past and crashed: it never gets its moment.
            this.actor.enabled = false;
            this.shadow.enabled = false;
            this.startPhase("miss", null);
            caption.set(`${this.star.name} missed the show!`);
            return;
          }
          this.startPhase("pose", this.poseAct);
          caption.set(this.star.name);
          this.startScene();
        }
        break;
      case "pose":
        m = this.poseAct.at(this.t);
        side = 1;
        this.runCues();
        if (!this.reacted && this.t >= this.reactAt) {
          this.reacted = true;
          this.react(this.reaction);
        }
        if (this.t >= this.poseAct.dur) {
          caption.set(null);
          this.effects.weather("none");
          this.effects.love(false);
          this.effects.disco(null);
          this.effects.night(false);
          this.effects.fashion(false);
          this.effects.haunt(false);
          this.heading = -this.outSide;
          this.startPhase("exit", this.exitAct);
        }
        break;
      case "exit":
        m = this.exitAct.at(this.t);
        side = this.outSide;
        this.runCues();
        if (this.t >= this.exitAct.dur) {
          this.actor.enabled = false;
          this.shadow.enabled = false;
          this.startPhase("close", null);
        }
        break;
      case "miss":
        this.curtain.approach(1, 3, dt);
        if (this.t > 2.8) {
          caption.set(null);
          this.startPhase("close", null);
        }
        return;
      case "close":
        this.curtain.approach(0, 3, dt);
        if (this.curtain.open < 0.04) this.beginWheel();
        return;
    }

    this.curtain.approach(m.open ?? 1, 5, dt);
    this.apply(m, side, dt);
  }

  private runCues(): void {
    while (this.cueAt < this.cues.length && this.cues[this.cueAt]![0] <= this.t) this.cues[this.cueAt++]![1](this.ctx);
  }

  private shakeCamera(dt: number): void {
    if (this.shakeLeft > 0) {
      this.shakeLeft = Math.max(0, this.shakeLeft - dt);
      const k = (this.shakeAmount * this.shakeLeft) / this.shakeTotal;
      this.cam.transform.position = new Vector3(rnd(-k, k), rnd(-k, k), 10);
    } else if (this.shakeAmount > 0) {
      this.shakeAmount = 0;
      this.cam.transform.position = new Vector3(0, 0, 10);
    }
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
    this.py = m.y;
    const target = v < -0.4 ? 1 : v > 0.4 ? -1 : this.heading;
    this.heading += (target - this.heading) * Math.min(1, dt * 12);
    const dir = v < 0 ? -1 : 1;

    // A knock from a tomato or a gasp: a quick shiver that settles.
    this.jolt = Math.max(0, this.jolt - dt * 2.5);
    const shiver = 9 * this.jolt * Math.sin(this.clock * 45);

    const c = m.centre;
    const half = HEIGHT / 2;
    const t = this.actor.transform;
    t.position = new Vector3(x, GROUND + m.y + half * c, 0);
    t.localScale = new Vector3(m.sx * m.spin, m.sy * (1 - 0.06 * this.jolt), 1);
    t.localEulerAngles = new Vector3(0, 0, m.rot * this.heading + shiver);
    this.art.transform.localPosition = new Vector3(0, half * (1 - c), 0);

    // Dim outside the light (the dark is drawn behind panels, so the emoji does it itself);
    // a flash of lightning lights it up.
    const spot = Math.exp(-((x - LIGHT.x) * (x - LIGHT.x)) / 9);
    // At night there is no spotlight to dim it.
    const dim = 0.3 + 0.7 * spot;
    const lit = Math.min(1.4, dim + (0.95 - dim) * this.effects.nightLevel + 0.9 * overlay.get().flash);
    const tint = m.tint ?? [1, 1, 1];
    const key = `${lit.toFixed(2)}|${tint.join(",")}|${m.alpha.toFixed(2)}`;
    if (key !== this.litKey) {
      this.litKey = key;
      this.panel.tint = new Color(lit * tint[0], lit * tint[1], lit * tint[2], m.alpha);
    }

    // The shadow shrinks as it leaves the floor.
    const spread = Math.min(1.25, Math.max(0.3, 1 - 0.08 * m.y)) * (1 + 0.5 * (1 - m.sy)) * Math.max(0.2, m.sx * m.spin);
    this.shadow.transform.position = new Vector3(x, GROUND, 0.1);
    this.shadow.transform.localScale = new Vector3(spread, spread, 1);

    const code = m.swap ?? this.star.code;
    const mirror = !!m.mirror;
    if (code !== this.shownCode || mirror !== this.shownMirror) {
      this.shownCode = code;
      this.shownMirror = mirror;
      shown.set({ code, look: this.star.look ?? null, mirror, big: this.big });
    }
    if (this.star.look) this.watch(x, m.y, dt);
    if (m.rocket) {
      const r = rocket.get();
      if (Math.abs(r.turn - m.rocket.turn) + Math.abs(r.flame - m.rocket.flame) + Math.abs(r.power - m.rocket.power) + Math.abs(r.stars - m.rocket.stars) > 0.005) rocket.set(m.rocket);
    }

    // Footsteps.
    if (m.stride !== undefined) {
      const step = Math.floor(m.stride);
      if (this.lastStride !== null && step !== this.lastStride) sfx.play("step", 0.35, rnd(0.9, 1.1));
      this.lastStride = step;
    }
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
        } else if (m.fx.kind === "fire") {
          this.fx.emit({
            x: x + rnd(-0.8, 0.8),
            y: GROUND + m.y + 0.2,
            vx: rnd(-1, 1),
            vy: -rnd(6, 12),
            life: rnd(0.25, 0.5),
            color: pick(FIRE),
            len: 0.5,
            width: 0.22,
            drag: 1.5,
          });
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
    if (m.ground) {
      this.groundCarry += m.ground * dt;
      while (this.groundCarry >= 1) {
        this.groundCarry -= 1;
        this.smoke(x + rnd(-1.5, 1.5), GROUND + 0.1, 1, [0.75, 0.72, 0.68, 0.45]);
      }
    }
    if (m.thump > 0.5 && this.wasThump <= 0.5) {
      sfx.play(this.phase === "enter" ? (this.current.land ?? "thud") : "thud", 0.6);
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

  /** Soft coloured puffs (fart clouds, tomato juice, smoke). */
  private smoke(x: number, y: number, count: number, color: Rgba, size = 1): void {
    for (let i = 0; i < count; i++) {
      this.fx.emit({
        x: x + rnd(-0.5, 0.5),
        y: y + rnd(-0.3, 0.3),
        vx: rnd(-1.8, 1.8),
        vy: rnd(0.2, 1.8),
        life: rnd(0.8, 1.4),
        color,
        len: 0.01,
        width: rnd(0.25, 0.55) * size,
        gravity: -0.4,
        drag: 1.2,
      });
    }
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
