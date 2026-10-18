// Fish tank: an aquarium in the style of the aquarium channels — a calm, full-screen view into
// deep water. A perspective camera drifts slowly through layers: a hazy back wall, distant rock
// silhouettes, a sand floor that fades into the blue, light shafts, caustics shimmering on the
// sand, kelp at several depths and marine snow. There's no fog in the engine, so everything is
// mixed toward the water's colour by how far back it is (in steps, so colours rarely change).
//
// Fish of four kinds school in 3D with their own kind, swim mostly level, keep inside the tank
// and flee the pointer. Click to feed (the nearest fish race for the food and grow),
// right-click adds a fish, Space taps the glass. The readout fades in only while the mouse
// moves. V toggles vsync.
import {
  addStyleSheet,
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

// ---- The water ----

const LEFT = -9;
const RIGHT = 9;
const FLOOR = -3.2;
const TOP = 4.6;
const NEAR = 2.5;
const FAR = -8;
const HAZE = Color.hex("#0f4c6e");
const DEEP = Color.hex("#06223a");
const LIGHT = Color.hex("#7fd0ef");
const SAND = Color.hex("#c7ab72");

const mix = (a: Color, b: Color, t: number) => new Color(a.r + (b.r - a.r) * t, a.g + (b.g - a.g) * t, a.b + (b.b - a.b) * t, a.a);
/** How much of the water's colour something `z` deep takes on (0 near, about 0.8 at the back). */
const haze = (z: number) => Math.round(Math.max(0, Math.min(1, (NEAR - z) / (NEAR - FAR - 2))) ** 0.9 * 0.82 * 16) / 16;

function shape(name: string, look: Partial<LineRenderer2D>, at = Vector3.zero, parent?: GameObject): LineRenderer2D {
  const go = new GameObject(name, parent ? { position: at, parent } : { position: at });
  return go.addComponent(LineRenderer2D, { width: 0.04, color: Color.white, ...look });
}

/** A shape lying flat on the floor (its XY plane turned to face up). */
function flat(name: string, look: Partial<LineRenderer2D>, at: Vector3, parent: GameObject): LineRenderer2D {
  const line = shape(name, look, at, parent);
  line.transform.rotation = Quaternion.euler(-90, 0, 0);
  return line;
}

// ---- Fish ----

interface Species {
  name: string;
  body: Color;
  fin: Color;
  size: number;
  speed: number;
  length: number;
}

const SPECIES: Species[] = [
  { name: "clownfish", body: Color.hex("#ff7a2f"), fin: Color.hex("#ffffff"), size: 0.5, speed: 1.3, length: 1 },
  { name: "tang", body: Color.hex("#2f6fff"), fin: Color.hex("#ffd23f"), size: 0.7, speed: 1.5, length: 1.15 },
  { name: "neon", body: Color.hex("#3fd6e0"), fin: Color.hex("#ff3d6e"), size: 0.3, speed: 2.0, length: 1.4 },
  { name: "angelfish", body: Color.hex("#ffe9a8"), fin: Color.hex("#3b3b3b"), size: 0.85, speed: 0.9, length: 0.8 },
];

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

/** A part of a fish, with the colours it has up close (it's hazed by depth). */
interface Part {
  line: LineRenderer2D;
  stroke: Color;
  fill: Color | null;
}

class Fish {
  readonly go: GameObject;
  private readonly tail: GameObject;
  private parts: Part[] = [];
  private shownHaze = -1;
  pos: Vector3;
  vel: Vector3;
  grown = 1;
  private angle = 0;
  private wander = Math.random() * Math.PI * 2;
  private beat = Math.random() * 10;

  constructor(
    readonly kind: Species,
    at: Vector3,
    tank: GameObject,
  ) {
    this.pos = at;
    const a = Math.random() * Math.PI * 2;
    this.vel = new Vector3(Math.cos(a), Math.sin(a) * 0.2, (Math.random() - 0.5) * 0.3).scale(kind.speed * 0.5);
    this.go = new GameObject(kind.name, { position: at, parent: tank });
    const edge = mix(kind.body, Color.black, 0.35);
    const part = (name: string, look: Partial<LineRenderer2D>, parent: GameObject) => {
      const line = shape(name, look, Vector3.zero, parent);
      this.parts.push({ line, stroke: look.color ?? Color.white, fill: look.fillColor ?? null });
    };
    this.tail = new GameObject("tail", { position: new Vector3(-0.38 * kind.length, 0, 0), parent: this.go });
    part("tail fin", { points: [new Vector2(0, 0), new Vector2(-0.36, 0.24), new Vector2(-0.28, 0), new Vector2(-0.36, -0.24)], loop: true, width: 0.025, color: edge, fillColor: kind.fin, sortingOrder: 10 }, this.tail);
    part("body", { points: body(kind.length), loop: true, width: 0.03, color: edge, fillColor: kind.body, sortingOrder: 11 }, this.go);
    part("dorsal", { points: [new Vector2(-0.1, 0.17), new Vector2(0.12 * kind.length, 0.36), new Vector2(0.18 * kind.length, 0.18)], loop: true, width: 0.02, color: edge, fillColor: kind.fin, sortingOrder: 10 }, this.go);
    if (kind.name === "clownfish" || kind.name === "angelfish") {
      for (const x of [0.22, -0.1]) part("stripe", { points: Shapes.ellipse(new Vector2(x * kind.length, 0), 0.05, 0.17, 10), loop: true, width: 0, color: kind.fin, fillColor: kind.fin, sortingOrder: 12 }, this.go);
    }
    if (kind.name === "neon") part("stripe", { points: [new Vector2(-0.32, 0), new Vector2(0.4, 0.02)], width: 0.06, color: kind.fin, sortingOrder: 12 }, this.go);
    part("eye", { points: Shapes.circle(new Vector2(0.3 * kind.length, 0.06), 0.045, 10), loop: true, width: 0.015, color: Color.white, fillColor: Color.hex("#111111"), sortingOrder: 13 }, this.go);
    this.place(0);
  }

  step(dt: number, tank: Tank): void {
    const k = this.kind;
    let steer = Vector3.zero;
    let ahead = Vector3.zero;
    let centre = Vector3.zero;
    let mates = 0;
    for (const o of tank.fish) {
      if (o === this) continue;
      const d = this.pos.sub(o.pos);
      const dist = d.magnitude;
      if (dist < 0.55 * (k.size + o.kind.size) && dist > 1e-4) steer = steer.add(d.scale(1.8 / (dist * dist + 0.05)));
      if (o.kind === k && dist < 2.6) {
        ahead = ahead.add(o.vel);
        centre = centre.add(o.pos);
        mates++;
      }
    }
    if (mates > 0) {
      steer = steer.add(ahead.scale(1 / mates).sub(this.vel).scale(0.6));
      steer = steer.add(centre.scale(1 / mates).sub(this.pos).scale(0.3));
    }
    this.wander += (Math.random() - 0.5) * dt * 3;
    steer = steer.add(new Vector3(Math.cos(this.wander), Math.sin(this.wander) * 0.35, Math.sin(this.wander * 0.7) * 0.4).scale(0.5));
    // Inside the tank: the sides, the floor, the surface, the front and the back.
    const keep = (value: number, lo: number, hi: number, margin: number) => (value < lo + margin ? (lo + margin - value) * 3 : value > hi - margin ? (hi - margin - value) * 3 : 0);
    steer = steer.add(new Vector3(keep(this.pos.x, LEFT, RIGHT, 1.5), keep(this.pos.y, FLOOR, TOP, 0.9), keep(this.pos.z, FAR, NEAR, 1)));
    let speedUp = 1;
    const food = tank.nearestFood(this.pos);
    if (food && food.pos.distance(this.pos) < 5) {
      const to = food.pos.sub(this.pos);
      steer = steer.add(to.normalized.scale(3.2));
      speedUp = 1.6;
      if (to.magnitude < 0.25 * k.size + 0.18) {
        tank.eat(food);
        this.grown = Math.min(1.5, this.grown + 0.06);
      }
    }
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
    // Mostly level, and mostly across rather than towards the glass (seen side on).
    const across = Math.max(Math.abs(this.vel.x), 0.1);
    this.vel = new Vector3(this.vel.x === 0 ? 0.1 : this.vel.x, Math.max(-across * 0.45, Math.min(across * 0.45, this.vel.y)), Math.max(-across * 0.6, Math.min(across * 0.6, this.vel.z)));
    this.pos = this.pos.add(this.vel.scale(dt));
    this.place(dt);
  }

  private place(dt: number): void {
    const target = (Math.atan2(this.vel.y, this.vel.x) * 180) / Math.PI;
    const diff = ((target - this.angle + 540) % 360) - 180;
    this.angle = dt === 0 ? target : this.angle + diff * Math.min(1, dt * 5);
    const left = Math.abs(((this.angle + 540) % 360) - 180) > 90;
    const s = this.kind.size * this.grown;
    this.go.transform.position = this.pos;
    this.go.transform.rotation = Quaternion.euler(0, 0, this.angle);
    this.go.transform.localScale = new Vector3(s, left ? -s : s, 1);
    this.beat += dt * (6 + this.vel.magnitude * 6);
    this.tail.transform.localRotation = Quaternion.euler(0, 0, Math.sin(this.beat) * 22);
    // Hazed by depth (only when the step changes); deeper fish draw first.
    const h = haze(this.pos.z);
    if (h !== this.shownHaze) {
      this.shownHaze = h;
      const order = Math.round(this.pos.z * 10) * 10;
      this.parts.forEach((p, i) => {
        p.line.color = mix(p.stroke, HAZE, h);
        if (p.fill) p.line.fillColor = mix(p.fill, HAZE, h);
        p.line.sortingOrder = order + i;
      });
    }
  }
}

// ---- The tank ----

interface Pellet {
  go: GameObject;
  pos: Vector3;
  age: number;
  drift: number;
}

class Tank extends Component {
  fish: Fish[] = [];
  private food: Pellet[] = [];
  private camera!: Camera;
  private plants: { line: LineRenderer2D; base: Vector3; height: number; phase: number }[] = [];
  private caustics: { line: LineRenderer2D; spline: Spline; centre: Vector2; size: number; phase: number }[] = [];
  private snow: { go: GameObject; pos: Vector3; phase: number }[] = [];
  private bubbles: { go: GameObject; pos: Vector3; speed: number; phase: number }[] = [];
  private shafts: GameObject[] = [];
  private tapAt = -Infinity;
  private tapFrom = Vector3.zero;
  private mouse: Vector3 | null = null;
  private lastMouse = new Vector2(-1, -1);
  private t = 0;
  fps = 60;
  hudUntil = 3;
  now = 0;

  override onAwake(): void {
    tank = this;
    this.camera = new GameObject("Main Camera", { position: new Vector3(0, 0.4, 12) }).addComponent(Camera, {
      fieldOfView: 40,
      backgroundColor: DEEP,
    });
    const root = this.gameObject;

    // The back wall of water: lighter towards the surface.
    for (let i = 0; i < 16; i++) {
      const h = 22 / 16;
      const top = 14 - i * h;
      shape(`back ${i}`, { points: Shapes.rectangle(-26, top - h - 0.05, 52, h + 0.1), loop: true, width: 0, fillColor: mix(mix(HAZE, LIGHT, 0.35), DEEP, i / 15), sortingOrder: -1000 }, new Vector3(0, 0, -10), root);
    }
    // Distant rock silhouettes, then nearer rocks.
    const rock = Color.hex("#3d4a52");
    const rocks: [number, number, number, number][] = [
      [-12, -8.5, 3.2, 2.6], [-5, -8.5, 2.2, 1.6], [3, -8.5, 3.6, 2.2], [11, -8.5, 2.8, 3.0],
      [-7, -4.5, 1.6, 1.0], [5.5, -4.5, 1.9, 1.3], [-2.5, -2.5, 0.9, 0.6],
      [7.2, 1.2, 1.2, 0.8], [-6.8, 1.6, 1.0, 0.65],
    ];
    rocks.forEach(([x, z, w, h], i) => {
      const c = mix(mix(rock, Color.hex("#6c7780"), (i % 3) / 3), HAZE, haze(z));
      shape(`rock ${i}`, { points: Shapes.ellipse(Vector2.zero, w, h, 24), loop: true, width: 0, fillColor: c, sortingOrder: Math.round(z * 100) }, new Vector3(x, FLOOR + h * 0.4, z), root);
    });
    // The floor: strips of sand from the front to the back, fading into the haze.
    for (let z = NEAR + 3; z > -11; z -= 1) {
      flat(`sand ${z}`, { points: Shapes.rectangle(-30, -0.5, 60, 1.04), loop: true, width: 0, fillColor: mix(SAND, HAZE, Math.min(0.9, haze(z) * 1.1)), sortingOrder: -900 }, new Vector3(0, FLOOR, z), root);
    }
    // Caustics: wobbling loops of light on the sand.
    for (let i = 0; i < 18; i++) {
      const centre = new Vector2(-10 + ((i * 7.3) % 20), -(((i * 3.7) % 11) - 2.5));
      const knots = Array.from({ length: 7 }, () => Vector3.zero);
      const z = centre.y;
      this.caustics.push({
        line: flat(`caustic ${i}`, { loop: true, width: 0.035, color: mix(mix(SAND, Color.white, 0.3), HAZE, haze(z) * 1.05), sortingOrder: -899 }, new Vector3(0, FLOOR + 0.01, 0), root),
        spline: new Spline(knots, { closed: true }),
        centre,
        size: 0.3 + (i % 4) * 0.12,
        phase: i * 1.3,
      });
    }
    // Light shafts from the surface.
    for (let i = 0; i < 5; i++) {
      const z = -6 + i * 1.2;
      const go = new GameObject(`shaft ${i}`, { position: new Vector3(-8 + i * 4, 9, z), parent: root });
      shape("shaft", { points: [new Vector2(-0.7, 0), new Vector2(0.8, 0), new Vector2(3.2, -13), new Vector2(1.0, -13)], loop: true, width: 0, fillColor: mix(HAZE, LIGHT, 0.2 - haze(z) * 0.1), sortingOrder: -950 + i }, Vector3.zero, go);
      this.shafts.push(go);
    }
    // Kelp at several depths.
    const greens = ["#2f8a4e", "#45a35c", "#277042", "#5bbf6f", "#3a9b7a"].map((h) => Color.hex(h));
    for (let i = 0; i < 18; i++) {
      const z = [-6.5, -4, -1.5, 1.5][i % 4]!;
      const x = -11 + ((i * 5.3) % 22);
      this.plants.push({
        line: shape(`kelp ${i}`, { width: 0.16 + (i % 3) * 0.05, color: mix(greens[i % 5]!, HAZE, haze(z)), sortingOrder: Math.round(z * 100) + 5 }, Vector3.zero, root),
        base: new Vector3(x, FLOOR, z),
        height: 2.2 + (i % 5) * 0.8,
        phase: i * 1.4,
      });
    }
    // Marine snow drifting through, and an air stone's bubbles.
    for (let i = 0; i < 70; i++) {
      const pos = new Vector3(LEFT * 1.4 + Math.random() * (RIGHT - LEFT) * 1.4, FLOOR + Math.random() * 8, FAR + Math.random() * (NEAR - FAR + 2));
      const go = new GameObject(`speck ${i}`, { position: pos, parent: root });
      const c = mix(Color.hex("#d6eef7"), HAZE, haze(pos.z));
      shape("speck", { points: Shapes.circle(Vector2.zero, 0.02 + (i % 3) * 0.012, 6), loop: true, width: 0, fillColor: c, sortingOrder: Math.round(pos.z * 100) + 50 }, Vector3.zero, go);
      this.snow.push({ go, pos, phase: Math.random() * 10 });
    }
    const stone = new Vector3(5.5, FLOOR + 0.1, -3);
    shape("air stone", { points: Shapes.roundedRectangle(-0.3, -0.1, 0.6, 0.2, 0.08), loop: true, width: 0, fillColor: mix(Color.hex("#6a7079"), HAZE, haze(stone.z)), sortingOrder: -290 }, stone, root);
    for (let i = 0; i < 24; i++) {
      const go = new GameObject(`bubble ${i}`, { parent: root });
      shape("bubble", { points: Shapes.circle(Vector2.zero, 0.04 + (i % 4) * 0.025, 12), loop: true, width: 0.018, color: mix(Color.hex("#d8f2ff"), HAZE, haze(stone.z)), sortingOrder: -280 }, Vector3.zero, go);
      this.bubbles.push({ go, pos: new Vector3(stone.x, FLOOR + (i / 24) * 9, stone.z), speed: 1.1 + (i % 5) * 0.25, phase: i * 0.7 });
    }

    SPECIES.forEach((kind, i) => {
      const n = kind.name === "neon" ? 10 : kind.name === "angelfish" ? 3 : 5;
      for (let j = 0; j < n; j++) this.addFish(kind, new Vector3(-6 + i * 3.5 + Math.random(), -1 + Math.random() * 3, -5 + Math.random() * 6));
    });
  }

  addFish(kind: Species, at: Vector3): void {
    this.fish.push(new Fish(kind, at, this.gameObject));
  }

  nearestFood(p: Vector3): Pellet | null {
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
  }

  scareFrom(p: Vector3): Vector3 | null {
    let away: Vector3 | null = null;
    if (this.mouse) {
      const d = p.sub(this.mouse);
      if (d.magnitude < 1.6) away = d.normalized.scale((1.6 - d.magnitude) / 1.6);
    }
    const since = this.t - this.tapAt;
    if (since < 0.8) {
      const d = p.sub(this.tapFrom);
      const push = d.normalized.scale(1 - since / 0.8);
      away = away ? away.add(push) : push;
    }
    return away;
  }

  private drop(at: Vector3): void {
    for (let i = 0; i < 6; i++) {
      const go = new GameObject("pellet", { parent: this.gameObject });
      shape("pellet", { points: Shapes.circle(Vector2.zero, 0.05, 8), loop: true, width: 0.015, color: Color.hex("#5a3a12"), fillColor: Color.hex("#b8772f"), sortingOrder: 400 }, Vector3.zero, go);
      const pos = new Vector3(at.x + (Math.random() - 0.5) * 0.8, Math.min(TOP - 0.2, at.y + (Math.random() - 0.5) * 0.3), at.z + (Math.random() - 0.5) * 0.6);
      this.food.push({ go, pos, age: 0, drift: Math.random() * 6 });
    }
  }

  override onUpdate(dt: number): void {
    const step = Math.min(dt, 1 / 30);
    this.t += dt;
    this.now = this.t;
    this.fps += (1 / Math.max(dt, 1e-3) - this.fps) * 0.1;
    if (input.getKeyDown("KeyV")) QualitySettings.vSyncCount = QualitySettings.vSyncCount > 0 ? 0 : 1;

    // The camera drifts slowly, like a camera on a slow dolly in front of the glass.
    const cam = this.camera.transform;
    cam.position = new Vector3(Math.sin(this.t * 0.045) * 1.6, 0.4 + Math.sin(this.t * 0.07) * 0.35, 12 + Math.sin(this.t * 0.03) * 0.8);
    cam.rotation = Quaternion.lookRotation(new Vector3(Math.sin(this.t * 0.045) * 0.6, -0.2, 0).sub(cam.position));

    // The pointer on the plane through the middle of the tank (z = 0).
    const m = input.mousePosition;
    if (m.x !== this.lastMouse.x || m.y !== this.lastMouse.y) {
      if (this.lastMouse.x >= 0) this.hudUntil = this.t + 3;
      this.lastMouse = new Vector2(m.x, m.y);
    }
    const w = this.camera.screenToWorldPoint({ x: m.x, y: m.y, z: cam.position.z });
    this.mouse = w.y > FLOOR && w.y < TOP ? new Vector3(w.x, w.y, 0) : null;
    if (this.mouse && input.getMouseButtonDown(0)) this.drop(this.mouse);
    if (this.mouse && input.getMouseButtonDown(1)) this.addFish(SPECIES[Math.floor(Math.random() * SPECIES.length)]!, this.mouse);
    if (input.getKeyDown("Space")) {
      this.tapAt = this.t;
      this.tapFrom = new Vector3(0, 0, NEAR + 3);
    }

    for (const f of this.fish) f.step(step, this);

    for (const p of this.food) {
      p.age += dt;
      if (p.pos.y > FLOOR + 0.1) p.pos = new Vector3(p.pos.x + Math.sin(this.t * 2 + p.drift) * 0.1 * dt, p.pos.y - 0.32 * dt, p.pos.z);
      p.go.transform.position = p.pos;
    }
    const stale = this.food.filter((p) => p.age > 25);
    for (const p of stale) p.go.destroy();
    if (stale.length) this.food = this.food.filter((p) => p.age <= 25);

    for (const pl of this.plants) {
      const pts: Vector2[] = [];
      for (let j = 0; j <= 10; j++) {
        const f = j / 10;
        pts.push(new Vector2(pl.base.x + Math.sin(this.t * 0.7 + pl.phase + f * 2) * 0.45 * f * f, pl.base.y + f * pl.height));
      }
      pl.line.points = pts;
      pl.line.transform.position = new Vector3(0, 0, pl.base.z);
    }
    for (const c of this.caustics) {
      for (let k = 0; k < c.spline.count; k++) {
        const a = (k / c.spline.count) * Math.PI * 2;
        const r = c.size * (1 + 0.2 * Math.sin(this.t * 1.1 + c.phase + k * 1.7));
        c.spline.setPosition(k, new Vector3(c.centre.x + Math.cos(a) * r + Math.sin(this.t * 0.4 + c.phase) * 0.4, c.centre.y + Math.sin(a) * r * 0.6, 0));
      }
      c.line.points = c.spline.sample(5);
    }
    for (const s of this.snow) {
      s.pos = new Vector3(s.pos.x + Math.sin(this.t * 0.3 + s.phase) * 0.05 * dt, s.pos.y - 0.08 * dt, s.pos.z);
      if (s.pos.y < FLOOR) s.pos = new Vector3(s.pos.x, FLOOR + 8, s.pos.z);
      s.go.transform.position = s.pos;
    }
    for (const b of this.bubbles) {
      b.pos = new Vector3(b.pos.x, b.pos.y + b.speed * dt, b.pos.z);
      if (b.pos.y > TOP + 4) b.pos = new Vector3(b.pos.x, FLOOR + 0.2, b.pos.z);
      b.go.transform.position = new Vector3(b.pos.x + Math.sin(this.t * 3 + b.phase) * 0.1, b.pos.y, b.pos.z);
    }
    this.shafts.forEach((s, i) => {
      s.transform.position = new Vector3(-8 + i * 4 + Math.sin(this.t * 0.15 + i) * 0.9, 9, -6 + i * 1.2);
    });
    if (Math.floor(this.t * 4) !== Math.floor((this.t - dt) * 4)) notify();
  }
}

// ---- React: a vignette, and a readout that shows only while the mouse moves ----

let tank: Tank | null = null;
let eaten = 0;
const listeners = new Set<() => void>();
function notify() {
  for (const f of listeners) f();
}

addStyleSheet(`
  .hud { transition: opacity 0.6s; }
`);

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
  const shown = t.now < t.hudUntil;
  return (
    <box style={{ width: "100%", height: "100%", pointerEvents: "none" }}>
      <box style={{ position: "absolute", left: 0, top: 0, right: 0, bottom: 0, backgroundImage: "radial-gradient(rgba(0, 0, 0, 0) 55%, rgba(0, 8, 20, 0.6) 100%)" }} />
      <box className="hud" style={{ position: "absolute", left: 22, bottom: 18, flexDirection: "column", gap: 4, opacity: shown ? 1 : 0 }}>
        <text style={{ color: "#e8f6ff", fontSize: 18, fontWeight: 700 }}>{`${t.fish.length} fish · ${eaten} pellets eaten`}</text>
        <text style={{ color: "rgba(232, 246, 255, 0.7)", fontSize: 13 }}>Click: feed · Right-click: add a fish · Space: tap the glass · V: vsync</text>
      </box>
      <text className="hud" style={{ position: "absolute", right: 18, top: 14, color: "rgba(232, 246, 255, 0.5)", fontSize: 12, opacity: shown ? 1 : 0 }}>{`${Math.round(t.fps)} FPS`}</text>
    </box>
  );
}

register({ Tank });
createRoot().render(<Hud />);
