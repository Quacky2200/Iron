// The sand timer: an hourglass that empties over each minute and turns over when the minute
// changes. The sand in each bulb is a filled shape cut to the glass at its level (the top one
// dips in the middle as it drains, the bottom one heaps into a mound), a thin stream falls
// through the neck with grains in it (they speed up as they fall, then roll off the mound;
// a few slide down the top's funnel into the neck), and the whole timer turns over with an
// ease. Turning a full bottom over gives a full top, so the swap is seamless.
import { Color, type DateTime, GameObject, type LineRenderer2D, Quaternion, Shapes, type TextMesh, Vector2, Vector3 } from "@iron/engine";

import { DIM, easeInOutCubic, type Face, INK, label, obj, shape, two, under } from "./common.ts";

/** Half the glass's height (centre to rim). */
const H = 2.9;
const FILL = 0.84;
const SAND = Color.hex("#e5b46a");
const SAND_DARK = Color.hex("#c9934a");
const GLASS = Color.hex("#a9c3d6");
const WOOD = Color.hex("#6b4527");
const WOOD_LIGHT = Color.hex("#8a5c35");
const FLIP = 1.1;
const GRAINS = 14;
const FUNNEL = 5;
/** Each grain's outline (set only while it shows, so hidden grains draw nothing). */
const grainShapes = new Map<LineRenderer2D, Vector2[]>();

/** The glass's half-width at height `y` (0 at the neck). */
const width = (y: number) => {
  const a = Math.min(1, Math.abs(y) / H);
  return 0.1 + 1.28 * (1 - (1 - a) ** 2) ** 1.25;
};

/** The glass's outline: down the right side, up the left. */
function glassOutline(): Vector2[] {
  const right: Vector2[] = [];
  for (let i = 0; i <= 60; i++) {
    const y = H - (2 * H * i) / 60;
    right.push(new Vector2(width(y), y));
  }
  return [...right, ...right.slice().reverse().map((p) => new Vector2(-p.x, p.y))];
}

/** Sand filling the glass from `bottom` up to `top` (both on the same side of the neck). */
function sandBetween(bottom: number, top: number, dip: number, mound: number): Vector2[] {
  const steps = 24;
  const right: Vector2[] = [];
  for (let i = 0; i <= steps; i++) {
    const y = bottom + ((top - bottom) * i) / steps;
    right.push(new Vector2(width(y) - 0.03, y));
  }
  const edge = right[right.length - 1]!;
  // The surface: a dip as the top drains, a mound as the bottom heaps.
  const surface: Vector2[] = [];
  for (let i = 1; i < 12; i++) {
    const x = edge.x - (2 * edge.x * i) / 12;
    const k = 1 - (x / edge.x) ** 2;
    surface.push(new Vector2(x, top - dip * k + mound * k));
  }
  const left = right.slice().reverse().map((p) => new Vector2(-p.x, p.y));
  return [...right, ...surface, ...left];
}

export class HourglassFace implements Face {
  readonly name = "Sand timer";
  readonly root = new GameObject("Sand timer face");
  private timer!: GameObject;
  private topSand!: LineRenderer2D;
  private bottomSand!: LineRenderer2D;
  private stream!: LineRenderer2D;
  /** Grains in the stream (they fall, then roll off the mound) and in the top's funnel. */
  private grains: LineRenderer2D[] = [];
  private funnel: LineRenderer2D[] = [];
  private grainShown: boolean[] = [];
  private time!: TextMesh;
  private left!: TextMesh;
  private lastMinute = -1;
  private flipAt = -Infinity;
  private shown = "";

  constructor() {
    under(this.root, () => this.build());
  }

  private build(): void {
    this.timer = obj("timer", new Vector3(0, 0.55, 0));
    const inside = this.timer;
    this.topSand = shape("top sand", { loop: true, width: 0, fillColor: SAND, sortingOrder: 1 }, Vector3.zero, inside);
    this.bottomSand = shape("bottom sand", { loop: true, width: 0, fillColor: SAND, sortingOrder: 1 }, Vector3.zero, inside);
    this.stream = shape("stream", { width: 0.035, color: SAND_DARK, sortingOrder: 2 }, Vector3.zero, inside);
    for (let i = 0; i < GRAINS + FUNNEL; i++) {
      const size = 0.022 + (i % 3) * 0.006;
      const g = shape(`grain ${i}`, { loop: true, width: 0.008, color: SAND_DARK, fillColor: i % 2 ? SAND : SAND_DARK, sortingOrder: 2 }, Vector3.zero, inside);
      g.points = [];
      grainShapes.set(g, Shapes.circle(Vector2.zero, size, 7));
      (i < GRAINS ? this.grains : this.funnel).push(g);
      this.grainShown.push(false);
    }
    shape("glass", { points: glassOutline(), loop: true, width: 0.06, color: GLASS, sortingOrder: 3 }, Vector3.zero, inside);
    // A glint down the glass.
    shape("glint", { points: Shapes.curve([new Vector2(0.72, 2.3), new Vector2(0.82, 1.7), new Vector2(0.55, 1.0)]), width: 0.045, color: Color.hex("#e6f1fa"), sortingOrder: 4 }, Vector3.zero, inside);
    // The frame: two end plates and three posts.
    for (const y of [H + 0.18, -H - 0.18]) {
      shape(`plate ${y > 0 ? "top" : "bottom"}`, { points: Shapes.roundedRectangle(-1.75, -0.18, 3.5, 0.36, 0.12), loop: true, width: 0.04, color: WOOD_LIGHT, fillColor: WOOD, sortingOrder: 5 }, new Vector3(0, y, 0), inside);
    }
    for (const x of [-1.55, 1.55]) {
      shape(`post ${x}`, { points: Shapes.roundedRectangle(-0.07, -H, 0.14, 2 * H, 0.06), loop: true, width: 0.03, color: WOOD_LIGHT, fillColor: WOOD, sortingOrder: 5 }, new Vector3(x, 0, 0), inside);
    }
    this.time = label("time", { fontSize: 48, fontWeight: 700, color: INK }, new Vector3(0, -3.55, 0));
    this.left = label("left", { fontSize: 20, color: DIM }, new Vector3(0, -4.1, 0));
  }

  update(now: DateTime, t: number): void {
    const s = now.second + now.millisecond / 1000;
    if (now.minute !== this.lastMinute) {
      if (this.lastMinute >= 0) this.flipAt = t;
      this.lastMinute = now.minute;
    }
    const flipping = t - this.flipAt < FLIP;
    // While it turns over, the sand is the old minute's, all run down.
    const left = flipping ? 0 : 1 - s / 60;
    const p = flipping ? easeInOutCubic((t - this.flipAt) / FLIP) : 0;
    this.timer.transform.rotation = Quaternion.euler(0, 0, p * 180);

    const top = 0.08 + left * FILL * H;
    this.topSand.points = left > 0.004 ? sandBetween(0.08, top, 0.18 * (1 - left) * Math.min(1, left * 8), 0) : [];
    const fallen = 1 - left;
    const pile = -H + 0.05 + fallen * FILL * H * 0.92;
    this.bottomSand.points = fallen > 0.004 ? sandBetween(-H + 0.05, pile, 0, 0.32 * Math.min(1, fallen * 3)) : [];

    // The stream and its grains, while there's sand to fall.
    const falling = !flipping && left > 0.004;
    const landing = pile + 0.32 * Math.min(1, fallen * 3);
    this.stream.points = falling ? [new Vector2(0, 0.02), new Vector2(0, landing)] : [];
    this.stream.width = 0.03 + 0.012 * Math.sin(t * 40);
    const show = (g: LineRenderer2D, i: number, on: boolean, x: number, y: number) => {
      if (on !== this.grainShown[i]) {
        this.grainShown[i] = on;
        g.points = on ? grainShapes.get(g)! : [];
      }
      if (on) g.transform.localPosition = new Vector3(x, y, 0);
    };
    // Falling: from the neck to the mound's top, speeding up (u²); then rolling down its side.
    const drop = 0.02 - landing;
    const edge = width(pile) - 0.05;
    const mound = 0.32 * Math.min(1, fallen * 3);
    this.grains.forEach((g, i) => {
      const p = (t * 0.9 + (i * 0.618) % 1) % 1;
      const side = i % 2 ? 1 : -1;
      const jitter = Math.sin(i * 12.9898) * 0.5;
      if (p < 0.7) {
        const u = p / 0.7;
        show(g, i, falling, jitter * 0.05 * u + Math.sin(t * 11 + i) * 0.008, 0.02 - drop * u * u);
      } else {
        // Rolling off: out along the mound, slowing, then settled (hidden in the pile).
        const r = (p - 0.7) / 0.3;
        const x = side * edge * (0.15 + 0.6 * (1 - (1 - r) ** 2)) * (0.6 + 0.4 * Math.abs(jitter));
        const k = 1 - (x / edge) ** 2;
        show(g, i, falling && r < 0.9, x, pile + mound * k + 0.015);
      }
    });
    // The top's funnel: grains slide down the dip toward the neck.
    const topEdge = width(top) - 0.05;
    const dip = 0.18 * (1 - left) * Math.min(1, left * 8);
    this.funnel.forEach((g, i) => {
      const p = (t * 0.45 + i / FUNNEL) % 1;
      const side = i % 2 ? 1 : -1;
      const x = side * topEdge * 0.55 * (1 - p) ** 1.5;
      const k = 1 - (x / topEdge) ** 2;
      show(g, GRAINS + i, falling && left > 0.03, x, Math.max(0.06, top - dip * k + 0.012 - p * p * 0.1));
    });

    const text = `${two(now.hour)}:${two(now.minute)}`;
    if (text !== this.shown) {
      this.shown = text;
      this.time.text = text;
    }
    const remaining = `${Math.ceil(60 - s)} seconds of sand left`;
    if (this.left.text !== remaining) this.left.text = remaining;
  }
}
