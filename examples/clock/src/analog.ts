// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// The analog face: three rings sweep round with the seconds, minutes and hours (`Shapes.arc`,
// round-capped strokes) and unwind when they come full circle; the hands are closed splines
// (leaf shapes, filled) turned by their objects; the second hand ticks with a little overshoot,
// the current second's tick flares, and an aura (a closed spline) pulses each second.

import { BG, DIM, easeOutBack, easeOutCubic, Face, INK, label, mix, obj, shape, under } from "./common.ts";

const FACE = Color.hex("#161e2a");
const SECONDS = Color.hex("#ff6b6b");
const MINUTES = Color.hex("#ffc04d");
const HOURS = Color.hex("#4fd1c5");
const CENTRE = new Vector3(0, 0.45, 0);
const R = 3.3;

/** A leaf-shaped hand pointing up, `length` long: a closed spline through six knots. */
function handShape(length: number, width: number): Vector2[] {
  const leaf = new Spline(
    [
      new Vector3(0, -length * 0.16, 0),
      new Vector3(width, 0, 0),
      new Vector3(width * 0.45, length * 0.72, 0),
      new Vector3(0, length, 0),
      new Vector3(-width * 0.45, length * 0.72, 0),
      new Vector3(-width, 0, 0),
    ],
    { closed: true },
  );
  return leaf.sample(10).map((p) => new Vector2(p.x, p.y));
}

/** A ring that sweeps clockwise from twelve and, when it comes full circle, unwinds. */
class Ring {
  private line: LineRenderer2D;
  private last = -1;
  private wrapAt: number = -Infinity;
  constructor(
    name: string,
    private readonly radius: number,
    colour: Color,
    width: number,
    private readonly unwind: number,
  ) {
    shape(`${name} track`, { points: Shapes.circle(Vector2.zero, radius, 96), loop: true, width, color: mix(FACE, colour, 0.16) }, CENTRE);
    this.line = shape(name, { width, color: colour }, CENTRE);
  }

  /** `fraction` (0 to 1) of the way round; `now` in seconds. */
  update(fraction: number, now: number): void {
    if (fraction < this.last - 0.5) this.wrapAt = now;
    this.last = fraction;
    let head = fraction * 360;
    let tail = 0;
    const since = (now - this.wrapAt) / this.unwind;
    if (since < 1) {
      // The old lap's tail races round to the new head.
      head += 360;
      tail = easeOutCubic(since) * 360;
    }
    const sweep = Math.max(0.01, head - tail);
    this.line.points = Shapes.arc(Vector2.zero, this.radius, this.radius, 90 - tail, -sweep, Math.max(2, Math.ceil(sweep / 4)));
  }
}

export class AnalogFace extends Face {
  readonly name = "Analog";
  readonly root: GameObject = new GameObject("Analog face");
  private seconds!: Ring;
  private minutes!: Ring;
  private hours!: Ring;
  private hourHand!: GameObject;
  private minuteHand!: GameObject;
  private secondHand!: GameObject;
  private ticks: LineRenderer2D[] = [];
  private tickGlow: number[] = [];
  private aura!: LineRenderer2D;
  private auraSpline!: Spline;
  private time!: TextMesh;
  private date!: TextMesh;
  private lastSecond = -1;
  private secondAt = 0;
  private shownTime = "";

  constructor() {
    super();
    under(this.root, () => this.build());
  }

  private build(): void {
    // The aura: 36 knots round the face, pushed out on each second.
    this.auraSpline = new Spline(
      Array.from({ length: 36 }, (_, i) => {
        const a = (i / 36) * Math.PI * 2;
        return new Vector3(Math.cos(a) * (R + 0.35), Math.sin(a) * (R + 0.35), 0);
      }),
      { closed: true },
    );
    this.aura = shape("aura", { loop: true, width: 0.035, color: mix(BG, SECONDS, 0.6) }, CENTRE);
    shape("face", { points: Shapes.circle(Vector2.zero, R, 128), loop: true, width: 0.04, color: Color.hex("#2a3546"), fillColor: FACE }, CENTRE);

    // Ticks: every minute, longer and thicker on the hours.
    for (let i = 0; i < 60; i++) {
      const hour = i % 5 === 0;
      const a = (90 - i * 6) * (Math.PI / 180);
      const dir = new Vector2(Math.cos(a), Math.sin(a));
      this.ticks.push(
        shape(`tick ${i}`, { points: [dir.scale(R - (hour ? 0.42 : 0.25)), dir.scale(R - 0.1)], width: hour ? 0.07 : 0.03, color: hour ? INK : DIM }, CENTRE),
      );
      this.tickGlow.push(0);
    }
    for (let h = 1; h <= 12; h++) {
      const a = (90 - h * 30) * (Math.PI / 180);
      label(`numeral ${h}`, { text: String(h), fontSize: 30 }, CENTRE.add(new Vector3(Math.cos(a), Math.sin(a), 0).scale(R - 0.72)));
    }

    this.hours = new Ring("hours ring", R - 1.45, HOURS, 0.13, 2.4);
    this.minutes = new Ring("minutes ring", R - 1.7, MINUTES, 0.13, 1.6);
    this.seconds = new Ring("seconds ring", R - 1.95, SECONDS, 0.13, 0.7);

    // Hands, pointing up; their objects turn them. Later shapes draw over earlier ones.
    this.hourHand = shape("hour hand", { points: handShape(1.75, 0.16), loop: true, width: 0.03, color: HOURS, fillColor: INK }, CENTRE).gameObject;
    this.minuteHand = shape("minute hand", { points: handShape(2.3, 0.12), loop: true, width: 0.03, color: MINUTES, fillColor: INK }, CENTRE).gameObject;
    this.secondHand = obj("second hand", CENTRE);
    shape("second needle", { points: [new Vector2(0, -0.7), new Vector2(0, R - 0.2)], width: 0.035, color: SECONDS }, Vector3.zero, this.secondHand);
    shape("second weight", { points: Shapes.circle(new Vector2(0, -0.55), 0.13, 24), loop: true, width: 0.035, color: SECONDS, fillColor: SECONDS }, Vector3.zero, this.secondHand);
    shape("cap", { points: Shapes.circle(Vector2.zero, 0.12, 24), loop: true, width: 0.03, color: SECONDS, fillColor: BG }, CENTRE);

    this.time = label("time", { fontSize: 52, fontWeight: 700 }, new Vector3(0, -3.5, 0));
    this.date = label("date", { fontSize: 20, color: DIM }, new Vector3(0, -4.08, 0));
  }

  update(now: DateTime, t: number, dt: number): void {
    const s = now.second + now.millisecond / 1000;
    const m = now.minute + s / 60;
    const h = (now.hour % 12) + m / 60;
    if (now.second !== this.lastSecond) {
      this.lastSecond = now.second;
      this.secondAt = t;
      this.tickGlow[now.second] = 1;
    }
    const sinceTick = t - this.secondAt;

    this.seconds.update(s / 60, t);
    this.minutes.update(m / 60, t);
    this.hours.update(h / 12, t);

    const turn = (go: GameObject, degrees: number) => {
      go.transform.rotation = Quaternion.euler(0, 0, -degrees);
    };
    turn(this.hourHand, h * 30);
    turn(this.minuteHand, m * 6);
    // The second hand jumps each second and overshoots a little before it settles.
    turn(this.secondHand, (now.second - 1 + easeOutBack(Math.min(1, sinceTick / 0.35))) * 6);

    // The current second's tick flares, then fades back.
    this.ticks.forEach((tick, i) => {
      const before = this.tickGlow[i]!;
      if (before <= 0) return;
      const glow = Math.max(0, before - dt * 1.4);
      this.tickGlow[i] = glow;
      const hour = i % 5 === 0;
      tick.color = mix(hour ? INK : DIM, SECONDS, glow);
      tick.width = (hour ? 0.07 : 0.03) + glow * 0.06;
    });

    // The aura: a wave round the face, kicked outward on each second.
    const kick = Math.exp(-sinceTick * 5);
    for (let i = 0; i < this.auraSpline.count; i++) {
      const a = (i / this.auraSpline.count) * Math.PI * 2;
      const r = R + 0.35 + 0.05 * Math.sin(t * 1.3 + i * 0.7) + kick * 0.16 * (0.5 + 0.5 * Math.sin(i * 2.4 + now.second));
      this.auraSpline.setPosition(i, new Vector3(Math.cos(a) * r, Math.sin(a) * r, 0));
    }
    this.aura.points = this.auraSpline.sample(6);

    const text = now.toString("HH:mm:ss");
    if (text !== this.shownTime) {
      this.shownTime = text;
      this.time.text = text;
      this.date.text = now.toString("dddd d MMMM yyyy");
    }
  }
}
