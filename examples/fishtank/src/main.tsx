// Fish tank: an aquarium to play with. Fish of four kinds school with their own kind
// (separation, alignment, cohesion), wander, keep off the glass and flee from the pointer.
// Click to drop food: it sinks, and the nearest fish race for it and grow a little when they
// eat. Right-click adds a fish where you click; Space taps the glass and scatters everyone.
// V toggles vsync.
//
// Everything is 2D shapes (`LineRenderer2D`, `Shapes`, `Spline`) under an orthographic
// camera; the readout and help are React.
import {
  Camera,
  Color,
  Component,
  GameObject,
  input,
  LineRenderer2D,
  QualitySettings,
  Quaternion,
  register,
  Shapes,
  Spline,
  Vector2,
  Vector3,
} from "@iron/engine";
import { createRoot } from "@iron/react";
import { useEffect, useState } from "react";

// ---- The tank ----

const LEFT = -8.3;
const RIGHT = 8.3;
const FLOOR = -3.1;
const SURFACE = 3.7;
const WATER_TOP = Color.hex("#2a86b8");
const WATER_DEEP = Color.hex("#0a2a4a");
const SAND = Color.hex("#c9ad74");
const SAND_DARK = Color.hex("#9c8352");

const mix = (a: Color, b: Color, t: number) => new Color(a.r + (b.r - a.r) * t, a.g + (b.g - a.g) * t, a.b + (b.b - a.b) * t, 1);

function shape(name: string, look: Partial<LineRenderer2D>, at = Vector3.zero, parent?: GameObject): LineRenderer2D {
  const go = new GameObject(name, parent ? { position: at, parent } : { position: at });
  return go.addComponent(LineRenderer2D, { width: 0.04, color: Color.white, ...look });
}

// ---- Fish ----

interface Species {
  name: string;
  body: Color;
  fin: Color;
  size: number;
  speed: number;
  /** How long the body is for its height (sleeker fish are longer). */
  length: number;
}

const SPECIES: Species[] = [
  { name: "clownfish", body: Color.hex("#ff7a2f"), fin: Color.hex("#ffffff"), size: 0.55, speed: 1.4, length: 1 },
  { name: "tang", body: Color.hex("#2f6fff"), fin: Color.hex("#ffd23f"), size: 0.75, speed: 1.6, length: 1.15 },
  { name: "neon", body: Color.hex("#3fd6e0"), fin: Color.hex("#ff3d6e"), size: 0.32, speed: 2.2, length: 1.4 },
  { name: "angelfish", body: Color.hex("#ffe9a8"), fin: Color.hex("#3b3b3b"), size: 0.9, speed: 1.0, length: 0.8 },
];

/** A fish body pointing right, about one unit long. */
function body(length: number): Vector2[] {
  const s = new Spline(
    [
      new Vector3(0.5 * length, 0, 0),
      new Vector3(0.15 * length, 0.22, 0),
      new Vector3(-0.28 * length, 0.14, 0),
      new Vector3(-0.42 * length, 0, 0),
      new Vector3(-0.28 * length, -0.14, 0),
      new Vector3(0.15 * length, -0.22, 0),
    ],
    { closed: true },
  );
  return s.sample(8).map((p) => new Vector2(p.x, p.y));
}

class Fish {
  readonly go: GameObject;
  private readonly tail: GameObject;
  pos: Vector2;
  vel: Vector2;
  grown = 1;
  private angle = 0;
  private wander = Math.random() * Math.PI * 2;
  private beat = Math.random() * 10;

  constructor(
    readonly kind: Species,
    at: Vector2,
    tank: GameObject,
  ) {
    this.pos = at;
    const a = Math.random() * Math.PI * 2;
    this.vel = new Vector2(Math.cos(a), Math.sin(a) * 0.3).scale(kind.speed * 0.5);
    this.go = new GameObject(kind.name, { position: new Vector3(at.x, at.y, 0), parent: tank });
    const edge = mix(kind.body, Color.black, 0.35);
    // The tail, behind the body, beats from its root.
    this.tail = new GameObject("tail", { position: new Vector3(-0.38 * kind.length, 0, 0), parent: this.go });
    shape("tail fin", { points: [new Vector2(0, 0), new Vector2(-0.36, 0.24), new Vector2(-0.28, 0), new Vector2(-0.36, -0.24)], loop: true, width: 0.025, color: edge, fillColor: kind.fin, sortingOrder: 10 }, Vector3.zero, this.tail);
    shape("body", { points: body(kind.length), loop: true, width: 0.03, color: edge, fillColor: kind.body, sortingOrder: 11 }, Vector3.zero, this.go);
    // A dorsal fin and a stripe or two.
    shape("dorsal", { points: [new Vector2(-0.1, 0.17), new Vector2(0.12 * kind.length, 0.36), new Vector2(0.18 * kind.length, 0.18)], loop: true, width: 0.02, color: edge, fillColor: kind.fin, sortingOrder: 10 }, Vector3.zero, this.go);
    if (kind.name === "clownfish" || kind.name === "angelfish") {
      for (const x of [0.22, -0.1]) {
        shape("stripe", { points: Shapes.ellipse(new Vector2(x * kind.length, 0), 0.05, 0.17, 10), loop: true, width: 0, fillColor: kind.fin, sortingOrder: 12 }, Vector3.zero, this.go);
      }
    }
    if (kind.name === "neon") {
      shape("stripe", { points: [new Vector2(-0.32, 0), new Vector2(0.4, 0.02)], width: 0.06, color: kind.fin, sortingOrder: 12 }, Vector3.zero, this.go);
    }
    shape("eye", { points: Shapes.circle(new Vector2(0.3 * kind.length, 0.06), 0.045, 10), loop: true, width: 0.015, color: Color.white, fillColor: Color.hex("#111111"), sortingOrder: 13 }, Vector3.zero, this.go);
    this.place(0);
  }

  /** One step: flocking, wandering, the glass, food and the pointer. */
  step(dt: number, tank: Tank): void {
    const k = this.kind;
    let steer = new Vector2(0, 0);
    let ahead = new Vector2(0, 0);
    let centre = new Vector2(0, 0);
    let mates = 0;
    for (const o of tank.fish) {
      if (o === this) continue;
      const d = this.pos.sub(o.pos);
      const dist = d.magnitude;
      if (dist < 0.55 * (k.size + o.kind.size) && dist > 1e-4) steer = steer.add(d.scale(1.8 / (dist * dist + 0.05)));
      if (o.kind === k && dist < 2.4) {
        ahead = ahead.add(o.vel);
        centre = centre.add(o.pos);
        mates++;
      }
    }
    if (mates > 0) {
      steer = steer.add(ahead.scale(1 / mates).sub(this.vel).scale(0.6));
      steer = steer.add(centre.scale(1 / mates).sub(this.pos).scale(0.35));
    }
    // A slow wander so loners still roam.
    this.wander += (Math.random() - 0.5) * dt * 3;
    steer = steer.add(new Vector2(Math.cos(this.wander), Math.sin(this.wander) * 0.4).scale(0.5));
    // Off the glass, the sand and the surface.
    const margin = 1.2;
    if (this.pos.x < LEFT + margin) steer = steer.add(new Vector2((LEFT + margin - this.pos.x) * 3, 0));
    if (this.pos.x > RIGHT - margin) steer = steer.add(new Vector2((RIGHT - margin - this.pos.x) * 3, 0));
    if (this.pos.y < FLOOR + 0.9) steer = steer.add(new Vector2(0, (FLOOR + 0.9 - this.pos.y) * 4));
    if (this.pos.y > SURFACE - 0.6) steer = steer.add(new Vector2(0, (SURFACE - 0.6 - this.pos.y) * 4));
    // Food: head for the nearest pellet, and eat it on reaching it.
    let speedUp = 1;
    const food = tank.nearestFood(this.pos);
    if (food && food.pos.distance(this.pos) < 5) {
      const to = food.pos.sub(this.pos);
      steer = steer.add(to.normalized.scale(3.2));
      speedUp = 1.6;
      if (to.magnitude < 0.25 * k.size + 0.15) {
        tank.eat(food);
        this.grown = Math.min(1.5, this.grown + 0.06);
      }
    }
    // The pointer, and taps on the glass, scare them.
    const scare = tank.scareFrom(this.pos);
    if (scare) {
      steer = steer.add(scare.scale(6));
      speedUp = 2.2;
    }
    this.vel = this.vel.add(steer.scale(dt));
    const max = k.speed * speedUp;
    const speed = this.vel.magnitude;
    if (speed > max) this.vel = this.vel.scale(max / speed);
    if (speed < k.speed * 0.25) this.vel = this.vel.scale((k.speed * 0.25) / Math.max(speed, 1e-4));
    // Fish swim mostly level: climbing or diving at most about 25 degrees.
    const rise = Math.abs(this.vel.x) * 0.47;
    if (Math.abs(this.vel.y) > rise) this.vel = new Vector2(this.vel.x === 0 ? 0.1 : this.vel.x, Math.sign(this.vel.y) * Math.max(rise, 0.05));
    this.pos = this.pos.add(this.vel.scale(dt));
    this.place(dt);
  }

  /** Faces where it's going, belly down, its tail beating with its speed. */
  private place(dt: number): void {
    const target = (Math.atan2(this.vel.y, this.vel.x) * 180) / Math.PI;
    let diff = ((target - this.angle + 540) % 360) - 180;
    if (dt === 0) diff = target - this.angle;
    this.angle += diff * Math.min(1, dt * 5 || 1);
    const left = Math.abs(((this.angle + 540) % 360) - 180) > 90;
    const s = this.kind.size * this.grown;
    this.go.transform.position = new Vector3(this.pos.x, this.pos.y, 0);
    this.go.transform.rotation = Quaternion.euler(0, 0, this.angle);
    this.go.transform.localScale = new Vector3(s, left ? -s : s, 1);
    this.beat += dt * (6 + this.vel.magnitude * 6);
    this.tail.transform.localRotation = Quaternion.euler(0, 0, Math.sin(this.beat) * 22);
  }
}

// ---- The tank's scenery, food and the rest ----

interface Pellet {
  go: GameObject;
  pos: Vector2;
  age: number;
  drift: number;
}

class Tank extends Component {
  fish: Fish[] = [];
  private food: Pellet[] = [];
  private camera!: Camera;
  private plants: { line: LineRenderer2D; base: Vector2; height: number; phase: number }[] = [];
  private bubbles: { go: GameObject; pos: Vector2; speed: number; phase: number }[] = [];
  private shafts: GameObject[] = [];
  private tapAt = -Infinity;
  private tapFrom = new Vector2(0, 0);
  private mouse: Vector2 | null = null;
  private t = 0;
  fps = 60;

  override onAwake(): void {
    tank = this;
    this.camera = new GameObject("Main Camera", { position: new Vector3(0, 0, 10) }).addComponent(Camera, {
      orthographic: true,
      orthographicSize: 5,
      backgroundColor: Color.hex("#121722"),
    });
    const root = this.gameObject;
    // The water, in bands from the lit surface to the deep.
    for (let i = 0; i < 12; i++) {
      const h = (SURFACE - FLOOR + 1.6) / 12;
      const top = SURFACE - i * h;
      shape(`water ${i}`, { points: Shapes.rectangle(LEFT, top - h - 0.02, RIGHT - LEFT, h + 0.04), loop: true, width: 0, fillColor: mix(WATER_TOP, WATER_DEEP, i / 11), sortingOrder: -20 }, Vector3.zero, root);
    }
    for (let i = 0; i < 4; i++) {
      const go = new GameObject(`shaft ${i}`, { position: new Vector3(-6 + i * 4, SURFACE, 0), parent: root });
      shape("shaft", { points: [new Vector2(-0.5, 0), new Vector2(0.6, 0), new Vector2(2.4, -7.5), new Vector2(0.9, -7.5)], loop: true, width: 0, fillColor: mix(WATER_DEEP, Color.hex("#9ad6f2"), 0.17), sortingOrder: -19 }, Vector3.zero, go);
      this.shafts.push(go);
    }
    // The sand, rocks, a chest and plants.
    const sand: Vector2[] = [new Vector2(LEFT, FLOOR - 2)];
    for (let i = 0; i <= 34; i++) {
      const x = LEFT + ((RIGHT - LEFT) * i) / 34;
      sand.push(new Vector2(x, FLOOR + 0.15 * Math.sin(x * 0.8) + 0.08 * Math.sin(x * 2.3)));
    }
    sand.push(new Vector2(RIGHT, FLOOR - 2));
    shape("sand", { points: sand, loop: true, width: 0.04, color: SAND_DARK, fillColor: SAND, sortingOrder: 1 }, Vector3.zero, root);
    const rocks: [number, number, number][] = [[-6.4, 0.9, 0.6], [-5.6, 0.55, 0.4], [4.2, 1.1, 0.75], [5.3, 0.6, 0.45], [-1.4, 0.5, 0.35]];
    rocks.forEach(([x, w, h], i) => {
      const c = mix(Color.hex("#5f6670"), Color.hex("#8a8f96"), (i % 3) / 2);
      shape(`rock ${i}`, { points: Shapes.ellipse(Vector2.zero, w, h, 20), loop: true, width: 0.04, color: mix(c, Color.black, 0.3), fillColor: c, sortingOrder: 3 }, new Vector3(x, FLOOR + h * 0.45, 0), root);
    });
    const chest = new GameObject("chest", { position: new Vector3(1.6, FLOOR + 0.35, 0), parent: root });
    shape("chest box", { points: Shapes.rectangle(-0.6, -0.35, 1.2, 0.6), loop: true, width: 0.04, color: Color.hex("#3d2412"), fillColor: Color.hex("#6b3f1e"), sortingOrder: 3 }, Vector3.zero, chest);
    shape("chest lid", { points: Shapes.pie(new Vector2(0, 0.25), 0.6, 0.35, 0, 180), loop: true, width: 0.04, color: Color.hex("#3d2412"), fillColor: Color.hex("#7d4a22"), sortingOrder: 3 }, Vector3.zero, chest);
    shape("chest band", { points: [new Vector2(-0.6, 0.05), new Vector2(0.6, 0.05)], width: 0.08, color: Color.hex("#d1a33b"), sortingOrder: 4 }, Vector3.zero, chest);
    shape("chest lock", { points: Shapes.rectangle(-0.08, -0.08, 0.16, 0.18), loop: true, width: 0.02, color: Color.hex("#8a6a1f"), fillColor: Color.hex("#e3b94a"), sortingOrder: 5 }, Vector3.zero, chest);
    const greens = ["#2f8a4e", "#45a35c", "#277042", "#5bbf6f", "#3a9b7a"].map((h) => Color.hex(h));
    for (let i = 0; i < 11; i++) {
      const x = LEFT + 0.6 + i * 1.5 + (i % 2) * 0.4;
      this.plants.push({ line: shape(`plant ${i}`, { width: 0.14 + (i % 3) * 0.05, color: greens[i % 5]!, sortingOrder: 2 }, Vector3.zero, root), base: new Vector2(x, FLOOR), height: 1.6 + (i % 4) * 0.7, phase: i * 1.4 });
    }
    // The air stone, and its bubbles.
    shape("air stone", { points: Shapes.roundedRectangle(-0.3, -0.1, 0.6, 0.2, 0.08), loop: true, width: 0.03, color: Color.hex("#3b3f46"), fillColor: Color.hex("#6a7079"), sortingOrder: 4 }, new Vector3(6.6, FLOOR + 0.12, 0), root);
    for (let i = 0; i < 26; i++) {
      const go = new GameObject(`bubble ${i}`, { parent: root });
      const r = 0.04 + (i % 4) * 0.03;
      shape("bubble", { points: Shapes.circle(Vector2.zero, r, 12), loop: true, width: 0.018, color: Color.hex("#d8f2ff"), sortingOrder: 20 }, Vector3.zero, go);
      this.bubbles.push({ go, pos: new Vector2(6.6, FLOOR + (i / 26) * (SURFACE - FLOOR)), speed: 1.1 + (i % 5) * 0.25, phase: i * 0.7 });
    }
    // The glass: a frame round the tank.
    shape("glass", { points: Shapes.rectangle(LEFT, FLOOR - 1.5, RIGHT - LEFT, SURFACE - FLOOR + 1.5), loop: true, width: 0.12, color: Color.hex("#9fb6c6"), sortingOrder: 30 }, Vector3.zero, root);
    shape("surface", { points: [new Vector2(LEFT, SURFACE), new Vector2(RIGHT, SURFACE)], width: 0.05, color: Color.hex("#bfe6fa"), sortingOrder: 29 }, Vector3.zero, root);

    // A few of each kind.
    SPECIES.forEach((kind, i) => {
      const n = kind.name === "neon" ? 8 : kind.name === "angelfish" ? 3 : 5;
      for (let j = 0; j < n; j++) this.addFish(kind, new Vector2(-5 + i * 3 + Math.random(), -1 + Math.random() * 3));
    });
    notify();
  }

  addFish(kind: Species, at: Vector2): void {
    this.fish.push(new Fish(kind, at, this.gameObject));
    notify();
  }

  nearestFood(p: Vector2): Pellet | null {
    let best: Pellet | null = null;
    let d = Infinity;
    for (const f of this.food) {
      const dd = f.pos.distance(p);
      if (dd < d) {
        d = dd;
        best = f;
      }
    }
    return best;
  }

  eat(p: Pellet): void {
    p.go.destroy();
    this.food = this.food.filter((f) => f !== p);
    eaten++;
    notify();
  }

  /** Which way to flee from the pointer or a tap on the glass, if either is close. */
  scareFrom(p: Vector2): Vector2 | null {
    let away: Vector2 | null = null;
    if (this.mouse) {
      const d = p.sub(this.mouse);
      if (d.magnitude < 1.4) away = d.normalized.scale((1.4 - d.magnitude) / 1.4);
    }
    const since = this.t - this.tapAt;
    if (since < 0.8) {
      const d = p.sub(this.tapFrom);
      const push = d.normalized.scale(1 - since / 0.8);
      away = away ? away.add(push) : push;
    }
    return away;
  }

  private drop(at: Vector2): void {
    for (let i = 0; i < 6; i++) {
      const go = new GameObject("pellet", { parent: this.gameObject });
      shape("pellet", { points: Shapes.circle(Vector2.zero, 0.05, 8), loop: true, width: 0.015, color: Color.hex("#5a3a12"), fillColor: Color.hex("#b8772f"), sortingOrder: 15 }, Vector3.zero, go);
      const pos = new Vector2(at.x + (Math.random() - 0.5) * 0.8, Math.min(SURFACE - 0.1, at.y + (Math.random() - 0.5) * 0.3));
      this.food.push({ go, pos, age: 0, drift: Math.random() * 6 });
    }
  }

  override onUpdate(dt: number): void {
    const step = Math.min(dt, 1 / 30);
    this.t += dt;
    this.fps += (1 / Math.max(dt, 1e-3) - this.fps) * 0.1;
    if (input.getKeyDown("KeyV")) QualitySettings.vSyncCount = QualitySettings.vSyncCount > 0 ? 0 : 1;

    // The pointer, in the tank (only while it's inside the glass).
    const m = input.mousePosition;
    const w = this.camera.screenToWorldPoint({ x: m.x, y: m.y, z: 10 });
    const inside = w.x > LEFT && w.x < RIGHT && w.y > FLOOR - 1.5 && w.y < SURFACE + 0.6;
    this.mouse = inside ? new Vector2(w.x, w.y) : null;
    if (this.mouse && input.getMouseButtonDown(0)) this.drop(new Vector2(this.mouse.x, Math.max(this.mouse.y, FLOOR + 0.5)));
    if (this.mouse && input.getMouseButtonDown(1) && this.mouse.y > FLOOR && this.mouse.y < SURFACE) {
      this.addFish(SPECIES[Math.floor(Math.random() * SPECIES.length)]!, this.mouse);
    }
    if (input.getKeyDown("Space")) {
      this.tapAt = this.t;
      this.tapFrom = new Vector2(Math.random() < 0.5 ? LEFT : RIGHT, 0);
    }

    for (const f of this.fish) f.step(step, this);

    // Food sinks, drifting, and goes when it's lain on the sand a while.
    for (const p of this.food) {
      p.age += dt;
      if (p.pos.y > FLOOR + 0.1) p.pos = new Vector2(p.pos.x + Math.sin(this.t * 2 + p.drift) * 0.1 * dt, p.pos.y - 0.32 * dt);
      p.go.transform.position = new Vector3(p.pos.x, p.pos.y, 0);
    }
    const stale = this.food.filter((p) => p.age > 25);
    for (const p of stale) p.go.destroy();
    if (stale.length) this.food = this.food.filter((p) => p.age <= 25);

    for (const pl of this.plants) {
      const pts: Vector2[] = [];
      for (let j = 0; j <= 10; j++) {
        const f = j / 10;
        pts.push(new Vector2(pl.base.x + Math.sin(this.t * 0.8 + pl.phase + f * 2) * 0.4 * f * f, pl.base.y + f * pl.height));
      }
      pl.line.points = pts;
    }
    for (const b of this.bubbles) {
      b.pos = new Vector2(b.pos.x, b.pos.y + b.speed * dt);
      if (b.pos.y > SURFACE) b.pos = new Vector2(6.6, FLOOR + 0.2);
      b.go.transform.position = new Vector3(b.pos.x + Math.sin(this.t * 3 + b.phase) * 0.1, b.pos.y, 0);
    }
    this.shafts.forEach((s, i) => {
      s.transform.position = new Vector3(-6 + i * 4 + Math.sin(this.t * 0.2 + i) * 0.7, SURFACE, 0);
    });
    if (Math.floor(this.t * 4) !== Math.floor((this.t - dt) * 4)) notify();
  }
}

// ---- React: the readout and help ----

let tank: Tank | null = null;
let eaten = 0;
const listeners = new Set<() => void>();
function notify() {
  for (const f of listeners) f();
}

function Hud() {
  const [, setTick] = useState(0);
  useEffect(() => {
    const update = () => setTick((n) => n + 1);
    listeners.add(update);
    return () => {
      listeners.delete(update);
    };
  }, []);
  const t = tank;
  if (!t) return null;
  return (
    <box style={{ width: "100%", height: "100%", pointerEvents: "none" }}>
      <box style={{ position: "absolute", left: 18, top: 14, flexDirection: "column", gap: 4 }}>
        <text style={{ color: "#e8f6ff", fontSize: 20, fontWeight: 700 }}>{`${t.fish.length} fish · ${eaten} pellets eaten`}</text>
        <text style={{ color: "rgba(232, 246, 255, 0.65)", fontSize: 13 }}>Click: feed · Right-click: add a fish · Space: tap the glass · V: vsync</text>
      </box>
      <text style={{ position: "absolute", right: 18, top: 14, color: "rgba(232, 246, 255, 0.5)", fontSize: 12 }}>{`${Math.round(t.fps)} FPS`}</text>
    </box>
  );
}

register({ Tank });
createRoot().render(<Hud />);
