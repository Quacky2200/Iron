// The morphing face, after Android's Material clock: each digit is one thick, round-capped
// stroke, and when it changes it morphs into the next (a 1 bends into a 2). Every glyph is a
// spline through a few knots in a 1 x 2 box (corner knots are `linear`), resampled by distance
// to the same number of points, so any digit can turn into any other point by point.

import { DIM, easeInOutCubic, Face, label, shape, two, under } from "./common.ts";

const POINTS = 64;
const MORPH = 0.6;

/** x, y in a 1 x 2 box (y up); a third number (1) makes a corner. */
interface Glyph {
  knots: number[][];
  closed: boolean;
}
const GLYPHS: Glyph[] = [
  { knots: [[0.5, 2], [0.04, 1.4], [0.04, 0.6], [0.5, 0], [0.96, 0.6], [0.96, 1.4]], closed: true },
  { knots: [[0.18, 1.55, 1], [0.6, 2, 1], [0.6, 0, 1]], closed: false },
  { knots: [[0.06, 1.5], [0.5, 2], [0.94, 1.5], [0.7, 0.95], [0.06, 0, 1], [0.96, 0, 1]], closed: false },
  { knots: [[0.08, 1.72], [0.5, 2], [0.9, 1.55], [0.42, 1.05, 1], [0.94, 0.52], [0.5, 0], [0.06, 0.28]], closed: false },
  { knots: [[0.72, 0, 1], [0.72, 2, 1], [0.04, 0.62, 1], [0.98, 0.62, 1]], closed: false },
  { knots: [[0.92, 2, 1], [0.14, 2, 1], [0.08, 1.08, 1], [0.55, 1.25], [0.95, 0.68], [0.55, 0], [0.06, 0.22]], closed: false },
  { knots: [[0.86, 1.88], [0.45, 2], [0.07, 1.25], [0.1, 0.45], [0.5, 0], [0.93, 0.45], [0.55, 1], [0.09, 0.62]], closed: false },
  { knots: [[0.04, 2, 1], [0.96, 2, 1], [0.36, 0, 1]], closed: false },
  { knots: [[0.5, 1.08], [0.1, 1.55], [0.5, 2], [0.9, 1.55], [0.5, 1.08], [0.06, 0.52], [0.5, 0], [0.94, 0.52]], closed: true },
  { knots: [[0.91, 1.38], [0.5, 0.98], [0.09, 1.42], [0.5, 2], [0.93, 1.5], [0.88, 0.6], [0.5, 0], [0.12, 0.2]], closed: false },
];

/** Each digit's outline, `POINTS` points evenly along it. */
function makeShapes(): Vector2[][] {
  const all: Vector2[][] = [];
  for (let g = 0; g < GLYPHS.length; g++) {
    const glyph = GLYPHS[g]!;
    const s = new Spline([], { closed: glyph.closed });
    for (let k = 0; k < glyph.knots.length; k++) {
      const knot = glyph.knots[k]!;
      s.addKnot(new Vector3(knot[0]!, knot[1]!, 0), knot.length > 2 ? "linear" : "auto");
    }
    const points: Vector2[] = [];
    for (let i = 0; i < POINTS; i++) {
      const p = s.evaluate(i / (POINTS - 1));
      points.push(new Vector2(p.x, p.y));
    }
    all.push(points);
  }
  return all;
}
const SHAPES: Vector2[][] = makeShapes();

/** One digit on screen: it morphs from what it shows to the next digit it's given. */
class Slot {
  private line: LineRenderer2D;
  private digit = -1;
  private from: Vector2[] = SHAPES[0]!;
  private to: Vector2[] = SHAPES[0]!;
  private startedAt: number = -Infinity;
  private shown: Vector2[] = SHAPES[0]!;
  constructor(
    name: string,
    at: Vector3,
    private readonly size: number,
    colour: Color,
  ) {
    this.line = shape(name, { width: 0.16 * size, color: colour }, at);
  }

  update(digit: number, t: number): void {
    if (digit !== this.digit) {
      // The first time it just appears; after that it morphs from wherever it is.
      this.from = this.digit < 0 ? SHAPES[digit]! : this.shown;
      this.to = SHAPES[digit]!;
      this.digit = digit;
      this.startedAt = t;
    }
    const p = Math.min(1, (t - this.startedAt) / MORPH);
    if (p >= 1 && this.shown === this.to) return;
    const e = easeInOutCubic(p);
    const s = this.size;
    this.shown = p >= 1 ? this.to : this.from.map((a, i) => a.lerp(this.to[i]!, e));
    this.line.points = this.shown.map((q) => new Vector2(q.x * s, q.y * s));
  }
}

export class MorphFace extends Face {
  readonly name = "Morphing";
  readonly root: GameObject = new GameObject("Morphing face");
  private slots: Slot[] = [];
  private dots: LineRenderer2D[] = [];
  private date!: TextMesh;
  private shownDate = "";

  constructor() {
    super();
    under(this.root, () => this.build());
  }

  private build(): void {
    const big = 1.5;
    const small = 0.75;
    const hours = Color.hex("#a8c7fa");
    const minutes = Color.hex("#d0bcff");
    const seconds = Color.hex("#ffb4ab");
    // HH:MM across the middle, the seconds smaller under the minutes.
    const w = big;
    const gap = 0.45;
    const colon = 1.0;
    const total = 4 * w + 2 * gap + colon;
    let x = -total / 2;
    const y = -0.6;
    const place = (name: string, colour: Color) => {
      this.slots.push(new Slot(name, new Vector3(x, y, 0), big, colour));
      x += w;
    };
    place("hour tens", hours);
    x += gap;
    place("hour units", hours);
    const colonX = x + colon / 2;
    x += colon;
    place("minute tens", minutes);
    x += gap;
    place("minute units", minutes);
    for (const dy of [0.85, 2.15]) {
      this.dots.push(shape("colon dot", { points: Shapes.circle(Vector2.zero, 0.13, 20), loop: true, width: 0.02, color: DIM, fillColor: DIM }, new Vector3(colonX, y + dy, 0)));
    }
    const sx = total / 2 - 2 * small - 0.3;
    this.slots.push(new Slot("second tens", new Vector3(sx, y - 2.15, 0), small, seconds));
    this.slots.push(new Slot("second units", new Vector3(sx + small + 0.3, y - 2.15, 0), small, seconds));
    this.date = label("date", { fontSize: 24, color: DIM }, new Vector3(0, 3.6, 0));
  }

  update(now: DateTime, t: number): void {
    const digits = `${two(now.hour)}${two(now.minute)}${two(now.second)}`;
    this.slots.forEach((slot, i) => slot.update(Number(digits.charAt(i)), t));
    // The colon breathes with the seconds.
    const glow = 0.5 + 0.5 * Math.cos(((now.millisecond / 1000) * Math.PI * 2));
    const c = new Color(0.36 + glow * 0.4, 0.42 + glow * 0.35, 0.5 + glow * 0.4, 1);
    for (const d of this.dots) {
      d.color = c;
      d.fillColor = c;
    }
    const date = now.toString("dddd d MMMM");
    if (date !== this.shownDate) {
      this.shownDate = date;
      this.date.text = date;
    }
  }
}
