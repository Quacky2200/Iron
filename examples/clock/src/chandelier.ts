// The chandelier face: a classic clock (white dial, Roman numerals, black spline hands, a gold
// seconds hand) hung in front of an ornament: two spirograph rosettes turning slowly against
// each other, curling gold arms ending in flickering candle flames, and crystal drops swinging
// on their chains. The gold and the flames are brighter than white, so they glow.

import { easeOutBack, Face, label, mix, obj, shape, under } from "./common.ts";

const CENTRE = new Vector3(0, 0.2, 0);
const GOLD = new Color(1.15, 0.82, 0.33, 1);
const GOLD_DIM = Color.hex("#7a5b25");
const FLAME = new Color(3.0, 1.7, 0.6, 1);
const CRYSTAL = new Color(1.2, 1.5, 1.9, 1);
const DIAL = Color.hex("#f4efe3");
const NAVY = Color.hex("#0a0e1d");
const ROMAN = ["XII", "I", "II", "III", "IV", "V", "VI", "VII", "VIII", "IX", "X", "XI"];

/** A spirograph curve (a hypotrochoid), scaled to `size`. */
function rosette(big: number, small: number, pen: number, size: number): Vector2[] {
  const turns = small / gcd(big, small);
  const out: Vector2[] = [];
  const n = 900;
  const k = (big - small) / small;
  const scale = size / (big - small + pen);
  for (let i = 0; i < n; i++) {
    const a = (i / n) * Math.PI * 2 * turns;
    out.push(new Vector2(((big - small) * Math.cos(a) + pen * Math.cos(k * a)) * scale, ((big - small) * Math.sin(a) - pen * Math.sin(k * a)) * scale));
  }
  return out;
}

function gcd(a: number, b: number): number {
  return b === 0 ? a : gcd(b, a % b);
}

/** A hand pointing up: a slim closed spline with a curl near its tip. */
function hand(length: number, width: number): Vector2[] {
  const s = new Spline(
    [
      new Vector3(0, -length * 0.18, 0),
      new Vector3(width, 0, 0),
      new Vector3(width * 0.5, length * 0.55, 0),
      new Vector3(width * 1.6, length * 0.72, 0),
      new Vector3(0, length, 0),
      new Vector3(-width * 1.6, length * 0.72, 0),
      new Vector3(-width * 0.5, length * 0.55, 0),
      new Vector3(-width, 0, 0),
    ],
    { closed: true },
  );
  return s.sample(8).map((p) => new Vector2(p.x, p.y));
}

export class ChandelierFace extends Face {
  readonly name = "Chandelier";
  readonly root: GameObject = new GameObject("Chandelier face");
  private rosettes: GameObject[] = [];
  private flames: { line: LineRenderer2D; phase: number }[] = [];
  private drops: { pivot: GameObject; phase: number; speed: number }[] = [];
  private hourHand!: GameObject;
  private minuteHand!: GameObject;
  private secondHand!: GameObject;
  private lastSecond = -1;
  private secondAt = 0;
  private background: Color | null = null;

  constructor() {
    super();
    under(this.root, () => this.build());
  }

  private build(): void {
    // Two rosettes behind everything, turning against each other.
    const a = obj("rosette a", CENTRE);
    shape("rosette a line", { points: rosette(7, 3, 4.6, 4.3), loop: true, width: 0.02, color: GOLD_DIM }, Vector3.zero, a);
    const b = obj("rosette b", CENTRE);
    shape("rosette b line", { points: rosette(9, 4, 3.4, 3.7), loop: true, width: 0.018, color: mix(GOLD_DIM, GOLD, 0.25) }, Vector3.zero, b);
    this.rosettes.push(a, b);

    // Eight curling arms, each ending in a candle.
    for (let i = 0; i < 8; i++) {
      const ang = (i / 8) * Math.PI * 2 + Math.PI / 8;
      const dir = new Vector2(Math.cos(ang), Math.sin(ang));
      const side = new Vector2(-dir.y, dir.x);
      const p0 = dir.scale(2.05);
      const tip = dir.scale(4.3);
      const arm = Shapes.bezier(p0, p0.add(dir.scale(1.2)).add(side.scale(0.9)), tip.add(side.scale(-0.8)), tip, 28);
      shape(`arm ${i}`, { points: arm, width: 0.07, color: GOLD, sortingOrder: 2 }, CENTRE);
      shape(`cup ${i}`, { points: Shapes.circle(Vector2.zero, 0.13, 16), loop: true, width: 0.03, color: GOLD, fillColor: GOLD_DIM, sortingOrder: 3 }, CENTRE.add(new Vector3(tip.x, tip.y, 0)));
      const flame = shape(`flame ${i}`, { loop: true, width: 0.02, color: FLAME, fillColor: FLAME, sortingOrder: 4 }, CENTRE.add(new Vector3(tip.x, tip.y + 0.12, 0)));
      this.flames.push({ line: flame, phase: i * 1.7 });
    }

    // Crystal drops on chains, between the arms, swinging a little.
    for (let i = 0; i < 16; i++) {
      const ang = (i / 16) * Math.PI * 2;
      const at = CENTRE.add(new Vector3(Math.cos(ang) * 3.25, Math.sin(ang) * 3.25, 0));
      const pivot = obj(`drop ${i}`, at);
      const len = 0.45 + (i % 2) * 0.25;
      shape(`chain ${i}`, { points: [Vector2.zero, new Vector2(0, -len)], width: 0.015, color: GOLD_DIM, sortingOrder: 1 }, Vector3.zero, pivot);
      shape(
        `crystal ${i}`,
        { points: Shapes.polygon({ x: 0, y: -len }, { x: 0.09, y: -len - 0.16 }, { x: 0, y: -len - 0.42 }, { x: -0.09, y: -len - 0.16 }), loop: true, width: 0.015, color: CRYSTAL, fillColor: mix(NAVY, CRYSTAL, 0.35), sortingOrder: 1 },
        Vector3.zero,
        pivot,
      );
      this.drops.push({ pivot, phase: i * 0.9, speed: 1.1 + (i % 3) * 0.25 });
    }

    // The clock in front.
    shape("rim", { points: Shapes.circle(Vector2.zero, 2.05, 128), loop: true, width: 0.14, color: GOLD, fillColor: DIAL, sortingOrder: 10 }, CENTRE);
    shape("inner ring", { points: Shapes.circle(Vector2.zero, 1.45, 96), loop: true, width: 0.015, color: GOLD_DIM, sortingOrder: 11 }, CENTRE);
    for (let i = 0; i < 60; i++) {
      const a2 = (90 - i * 6) * (Math.PI / 180);
      const d = new Vector2(Math.cos(a2), Math.sin(a2));
      shape(`tick ${i}`, { points: [d.scale(i % 5 === 0 ? 1.68 : 1.8), d.scale(1.9)], width: i % 5 === 0 ? 0.045 : 0.018, color: Color.hex("#2b2418"), sortingOrder: 11 }, CENTRE);
    }
    ROMAN.forEach((r, i) => {
      const a2 = (90 - i * 30) * (Math.PI / 180);
      label(`roman ${r}`, { text: r, fontSize: 30, fontWeight: 600, color: Color.hex("#2b2418") }, CENTRE.add(new Vector3(Math.cos(a2) * 1.62 * 0.88, Math.sin(a2) * 1.62 * 0.88, 0)));
    });
    this.hourHand = shape("hour hand", { points: hand(0.95, 0.07), loop: true, width: 0.02, color: Color.hex("#1a1610"), fillColor: Color.hex("#1a1610"), sortingOrder: 12 }, CENTRE).gameObject;
    this.minuteHand = shape("minute hand", { points: hand(1.45, 0.05), loop: true, width: 0.02, color: Color.hex("#1a1610"), fillColor: Color.hex("#1a1610"), sortingOrder: 13 }, CENTRE).gameObject;
    this.secondHand = obj("second hand", CENTRE);
    shape("second needle", { points: [new Vector2(0, -0.35), new Vector2(0, 1.75)], width: 0.02, color: GOLD, sortingOrder: 14 }, Vector3.zero, this.secondHand);
    shape("hub", { points: Shapes.circle(Vector2.zero, 0.07, 16), loop: true, width: 0.02, color: GOLD, fillColor: GOLD, sortingOrder: 15 }, CENTRE);
  }

  override show(camera: Camera): void {
    this.background = camera.backgroundColor;
    camera.backgroundColor = NAVY;
    RenderSettings.bloom = { threshold: 1.0, intensity: 0.8, scatter: 0.7 };
  }

  override hide(camera: Camera): void {
    if (this.background) camera.backgroundColor = this.background;
    RenderSettings.bloom = null;
  }

  update(now: DateTime, t: number): void {
    this.rosettes[0]!.transform.rotation = Quaternion.euler(0, 0, t * 4);
    this.rosettes[1]!.transform.rotation = Quaternion.euler(0, 0, -t * 6);
    for (const f of this.flames) {
      const flicker = 1 + 0.18 * Math.sin(t * 11 + f.phase) + 0.1 * Math.sin(t * 23 + f.phase * 2);
      const h = 0.22 * flicker;
      f.line.points = [new Vector2(0, 0), new Vector2(0.06, h * 0.35), new Vector2(0, h), new Vector2(-0.06, h * 0.35)];
    }
    for (const d of this.drops) {
      d.pivot.transform.rotation = Quaternion.euler(0, 0, Math.sin(t * d.speed + d.phase) * 7);
    }
    const s = now.second + now.millisecond / 1000;
    const m = now.minute + s / 60;
    const h = (now.hour % 12) + m / 60;
    if (now.second !== this.lastSecond) {
      this.lastSecond = now.second;
      this.secondAt = t;
    }
    const tick = easeOutBack(Math.min(1, (t - this.secondAt) / 0.3));
    this.hourHand.transform.rotation = Quaternion.euler(0, 0, -h * 30);
    this.minuteHand.transform.rotation = Quaternion.euler(0, 0, -m * 6);
    this.secondHand.transform.rotation = Quaternion.euler(0, 0, -(now.second - 1 + tick) * 6);
  }
}
