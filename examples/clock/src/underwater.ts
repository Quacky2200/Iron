// SPDX-License-Identifier: Apache-2.0

// The underwater face: a brass porthole clock lying on the sea bed, slightly tilted, its second
// hand dragging as if through water. Light shafts drift down through blue water, kelp sways,
// bubbles wobble up to the surface, and fish (spline bodies) swim past, turning at the edges.

import { Face, label, mix, obj, shape, two, under } from "./common.ts";

const SURFACE = Color.hex("#1f6fa8");
const DEEP = Color.hex("#061a33");
const SAND = Color.hex("#b89a63");
const SAND_DARK = Color.hex("#8d7448");
const BRASS = Color.hex("#b8893a");
const BRASS_DARK = Color.hex("#6e5020");
const CENTRE = new Vector3(0.6, 0.3, 0);

/** A fish pointing right, about one unit long: a closed spline body and a tail. */
export function fishBody(): Vector2[] {
  const s = new Spline(
    [new Vector3(0.5, 0, 0), new Vector3(0.18, 0.2, 0), new Vector3(-0.25, 0.13, 0), new Vector3(-0.42, 0, 0), new Vector3(-0.25, -0.13, 0), new Vector3(0.18, -0.2, 0)],
    { closed: true },
  );
  return s.sample(8).map((p) => new Vector2(p.x, p.y));
}
export const FISH_TAIL = [new Vector2(-0.38, 0), new Vector2(-0.72, 0.22), new Vector2(-0.64, 0), new Vector2(-0.72, -0.22)];

interface Swimmer {
  go: GameObject;
  tail: GameObject;
  y: number;
  speed: number;
  dir: number;
  x: number;
  phase: number;
}

export class UnderwaterFace extends Face {
  readonly name = "Underwater";
  readonly root: GameObject = new GameObject("Underwater face");
  private shafts: GameObject[] = [];
  private kelp: { line: LineRenderer2D; base: Vector2; height: number; phase: number }[] = [];
  private bubbles: { go: GameObject; x: number; y: number; speed: number; phase: number }[] = [];
  private fish: Swimmer[] = [];
  private hourHand!: GameObject;
  private minuteHand!: GameObject;
  private secondHand!: GameObject;
  private secondAngle: number = Number.NaN;
  private time!: TextMesh;
  private shown = "";
  private background: Color | null = null;

  constructor() {
    super();
    under(this.root, () => this.build());
  }

  private build(): void {
    // The water: bands from the bright surface down to the deep.
    const bands = 10;
    for (let i = 0; i < bands; i++) {
      const top = 5.2 - (i * 10.4) / bands;
      shape(`water ${i}`, { points: Shapes.rectangle(-10, top - 10.4 / bands - 0.02, 20, 10.4 / bands + 0.04), loop: true, width: 0, fillColor: mix(SURFACE, DEEP, i / (bands - 1)), sortingOrder: -10 });
    }
    // Light shafts slanting down from the surface.
    for (let i = 0; i < 5; i++) {
      const go = obj(`shaft ${i}`, new Vector3(-6 + i * 3.1, 5.2, 0));
      const w = 0.5 + (i % 3) * 0.3;
      shape(`shaft quad ${i}`, { points: [new Vector2(-w, 0), new Vector2(w, 0), new Vector2(w * 2.4 + 1.6, -9), new Vector2(-w * 0.4 + 1.6, -9)], loop: true, width: 0, fillColor: mix(DEEP, Color.hex("#7fc3e8"), 0.16), sortingOrder: -9 }, Vector3.zero, go);
      this.shafts.push(go);
    }
    // The sea bed.
    const bed: Vector2[] = [new Vector2(-10, -5.5)];
    for (let i = 0; i <= 40; i++) {
      const x = -10 + i * 0.5;
      bed.push(new Vector2(x, -3.6 + 0.25 * Math.sin(x * 0.7) + 0.12 * Math.sin(x * 2.1)));
    }
    bed.push(new Vector2(10, -5.5));
    shape("sand", { points: bed, loop: true, width: 0.05, color: SAND_DARK, fillColor: SAND, sortingOrder: 2 });
    for (let i = 0; i < 14; i++) {
      const x = -8.5 + i * 1.3 + Math.sin(i * 7.3) * 0.4;
      shape(`pebble ${i}`, { points: Shapes.ellipse(Vector2.zero, 0.16 + (i % 3) * 0.06, 0.09, 12), loop: true, width: 0.02, color: SAND_DARK, fillColor: mix(SAND_DARK, Color.hex("#6d6a63"), 0.5), sortingOrder: 3 }, new Vector3(x, -4.05 - (i % 2) * 0.35, 0));
    }
    // Kelp, swaying from the bed.
    const greens = ["#2f7d4a", "#3f9a55", "#25663c", "#4cae63"].map((h) => Color.hex(h));
    for (let i = 0; i < 9; i++) {
      const base = new Vector2(-8.2 + i * 2.05 + (i % 2) * 0.5, -3.55);
      this.kelp.push({ line: shape(`kelp ${i}`, { width: 0.16 + (i % 3) * 0.05, color: greens[i % 4]!, sortingOrder: 1 }), base, height: 2.2 + (i % 4) * 0.9, phase: i * 1.3 });
    }
    // Fish, swimming both ways.
    const colours = ["#ff8a3d", "#ffd23f", "#4fd1c5", "#ff5d8f", "#a78bfa", "#7bdff2"].map((h) => Color.hex(h));
    for (let i = 0; i < 6; i++) {
      const go = obj(`fish ${i}`, Vector3.zero);
      const c = colours[i]!;
      shape(`fish body ${i}`, { points: fishBody(), loop: true, width: 0.03, color: mix(c, Color.black, 0.3), fillColor: c, sortingOrder: 5 }, Vector3.zero, go);
      const tail = obj(`fish tail ${i}`, Vector3.zero, go);
      shape(`fish tail line ${i}`, { points: FISH_TAIL, loop: true, width: 0.03, color: mix(c, Color.black, 0.3), fillColor: mix(c, Color.white, 0.15), sortingOrder: 5 }, Vector3.zero, tail);
      shape(`fish eye ${i}`, { points: Shapes.circle(new Vector2(0.27, 0.05), 0.035, 8), loop: true, width: 0.01, color: Color.black, fillColor: Color.black, sortingOrder: 6 }, Vector3.zero, go);
      const scale = 0.55 + (i % 3) * 0.25;
      go.transform.localScale = new Vector3(scale, scale, 1);
      this.fish.push({ go, tail, y: -2.4 + i * 1.05, speed: 0.6 + (i % 4) * 0.3, dir: i % 2 ? -1 : 1, x: -8 + i * 2.9, phase: i * 1.7 });
    }
    // The porthole clock, tilted on the bed.
    const clock = obj("porthole", CENTRE);
    clock.transform.rotation = Quaternion.euler(0, 0, -8);
    shape("porthole rim", { points: Shapes.circle(Vector2.zero, 2.15, 96), loop: true, width: 0.3, color: BRASS, fillColor: mix(DEEP, Color.hex("#2f8fc0"), 0.55), sortingOrder: 7 }, Vector3.zero, clock);
    shape("porthole inner", { points: Shapes.circle(Vector2.zero, 1.9, 96), loop: true, width: 0.05, color: BRASS_DARK, sortingOrder: 8 }, Vector3.zero, clock);
    for (let i = 0; i < 8; i++) {
      const a = (i / 8) * Math.PI * 2 + 0.2;
      shape(`bolt ${i}`, { points: Shapes.circle(Vector2.zero, 0.07, 10), loop: true, width: 0.02, color: BRASS_DARK, fillColor: mix(BRASS, Color.white, 0.25), sortingOrder: 9 }, new Vector3(Math.cos(a) * 2.15, Math.sin(a) * 2.15, 0), clock);
    }
    for (let i = 0; i < 12; i++) {
      const a = (90 - i * 30) * (Math.PI / 180);
      const d = new Vector2(Math.cos(a), Math.sin(a));
      shape(`mark ${i}`, { points: [d.scale(i % 3 === 0 ? 1.45 : 1.6), d.scale(1.78)], width: i % 3 === 0 ? 0.1 : 0.05, color: Color.hex("#e9f6ff"), sortingOrder: 9 }, Vector3.zero, clock);
    }
    this.hourHand = obj("hour hand", Vector3.zero, clock);
    shape("hour hand line", { points: [new Vector2(0, -0.2), new Vector2(0, 0.95)], width: 0.13, color: Color.hex("#e9f6ff"), sortingOrder: 10 }, Vector3.zero, this.hourHand);
    this.minuteHand = obj("minute hand", Vector3.zero, clock);
    shape("minute hand line", { points: [new Vector2(0, -0.25), new Vector2(0, 1.5)], width: 0.09, color: Color.hex("#e9f6ff"), sortingOrder: 11 }, Vector3.zero, this.minuteHand);
    this.secondHand = obj("second hand", Vector3.zero, clock);
    shape("second hand line", { points: [new Vector2(0, -0.35), new Vector2(0, 1.65)], width: 0.035, color: Color.hex("#ff9f68"), sortingOrder: 12 }, Vector3.zero, this.secondHand);
    shape("hub", { points: Shapes.circle(Vector2.zero, 0.1, 12), loop: true, width: 0.02, color: BRASS_DARK, fillColor: BRASS, sortingOrder: 13 }, Vector3.zero, clock);
    // Bubbles in front of everything.
    for (let i = 0; i < 18; i++) {
      const r = 0.05 + (i % 4) * 0.035;
      const go = obj(`bubble ${i}`, Vector3.zero);
      shape(`bubble ring ${i}`, { points: Shapes.circle(Vector2.zero, r, 14), loop: true, width: 0.02, color: Color.hex("#cfefff"), sortingOrder: 14 }, Vector3.zero, go);
      shape(`bubble shine ${i}`, { points: Shapes.arc(Vector2.zero, r * 0.6, r * 0.6, 100, 60, 6), width: 0.012, color: Color.white, sortingOrder: 15 }, Vector3.zero, go);
      this.bubbles.push({ go, x: -7 + ((i * 7.7) % 14), y: -3.5 - (i % 6) * 1.4, speed: 0.6 + (i % 5) * 0.18, phase: i * 1.1 });
    }
    this.time = label("time", { fontSize: 42, fontWeight: 700, color: Color.hex("#d8f3ff") }, new Vector3(-5.6, 3.6, 0));
  }

  override show(camera: Camera): void {
    this.background = camera.backgroundColor;
    camera.backgroundColor = DEEP;
  }

  override hide(camera: Camera): void {
    if (this.background) camera.backgroundColor = this.background;
  }

  update(now: DateTime, t: number, dt: number): void {
    this.shafts.forEach((s, i) => {
      s.transform.position = new Vector3(-6 + i * 3.1 + Math.sin(t * 0.15 + i) * 0.8, 5.2, 0);
    });
    for (const k of this.kelp) {
      const pts: Vector2[] = [];
      for (let j = 0; j <= 12; j++) {
        const f = j / 12;
        const sway = Math.sin(t * 0.9 + k.phase + f * 2.2) * 0.45 * f * f + Math.sin(t * 2.1 + k.phase) * 0.06 * f;
        pts.push(new Vector2(k.base.x + sway, k.base.y + f * k.height));
      }
      k.line.points = pts;
    }
    for (const b of this.bubbles) {
      b.y += b.speed * dt;
      if (b.y > 5.4) {
        b.y = -3.6;
        b.x = -7 + Math.random() * 14;
      }
      b.go.transform.position = new Vector3(b.x + Math.sin(t * 2.2 + b.phase) * 0.12, b.y, 0);
    }
    for (const f of this.fish) {
      f.x += f.speed * f.dir * dt;
      if (f.x > 9.5) f.dir = -1;
      if (f.x < -9.5) f.dir = 1;
      const s = Math.abs(f.go.transform.localScale.y);
      f.go.transform.localScale = new Vector3(s * f.dir, s, 1);
      f.go.transform.position = new Vector3(f.x, f.y + Math.sin(t * 0.8 + f.phase) * 0.25, 0);
      f.tail.transform.localRotation = Quaternion.euler(0, 0, Math.sin(t * 7 + f.phase) * 18);
    }

    const s = now.second + now.millisecond / 1000;
    const m = now.minute + s / 60;
    const h = (now.hour % 12) + m / 60;
    this.hourHand.transform.localRotation = Quaternion.euler(0, 0, -h * 30);
    this.minuteHand.transform.localRotation = Quaternion.euler(0, 0, -m * 6);
    // The second hand drags behind, as if through water, catching up smoothly.
    const target = now.second * 6;
    if (Number.isNaN(this.secondAngle)) this.secondAngle = target;
    // The short way round, so it glides on past twelve.
    const gap = ((((target - this.secondAngle) % 360) + 540) % 360) - 180;
    this.secondAngle += gap * Math.min(1, dt * 4);
    this.secondHand.transform.localRotation = Quaternion.euler(0, 0, -this.secondAngle);

    const text = `${two(now.hour)}:${two(now.minute)}`;
    if (text !== this.shown) {
      this.shown = text;
      this.time.text = text;
    }
  }
}
