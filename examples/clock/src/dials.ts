// SPDX-License-Identifier: Apache-2.0

// The dials face: three rings (hours, minutes, seconds) carry their numbers round and turn to
// bring the time under a window at twelve o'clock, like a car's odometer laid flat. Seconds
// step with a small overshoot, minutes ease round when they change, hours glide.

import { DIM, easeInOutCubic, easeOutBack, Face, label, mix, obj, shape, two, under } from "./common.ts";

const CENTRE = new Vector3(0, -0.35, 0);
const ACCENT = Color.hex("#ffc04d");
const PANEL = Color.hex("#141b26");

interface DialSpec {
  name: string;
  radius: number;
  colour: Color;
  /** Ticks round the dial, and a number on every `every`th. */
  ticks: number;
  every: number;
  text: (i: number) => string;
}

const DIALS: DialSpec[] = [
  { name: "hours", radius: 1.5, colour: Color.hex("#4fd1c5"), ticks: 12, every: 1, text: (i) => String(i === 0 ? 12 : i) },
  { name: "minutes", radius: 2.4, colour: Color.hex("#ffc04d"), ticks: 60, every: 5, text: (i) => two(i) },
  { name: "seconds", radius: 3.3, colour: Color.hex("#ff6b6b"), ticks: 60, every: 5, text: (i) => two(i) },
];

/** A dial's turn: continuous degrees, eased from where it was to where it's going. */
class Turn {
  current = 0;
  private from = 0;
  private to: number = Number.NaN;
  private at = 0;

  /** Heads for `target` degrees the short way round, over `seconds` with `ease`. */
  step(target: number, t: number, seconds: number, ease: (x: number) => number): number {
    if (Number.isNaN(this.to)) {
      this.current = this.from = this.to = target;
      return target;
    }
    let to = target;
    while (to < this.to - 180) to += 360;
    while (to > this.to + 180) to -= 360;
    if (to !== this.to) {
      this.from = this.current;
      this.to = to;
      this.at = t;
    }
    const p = Math.min(1, (t - this.at) / seconds);
    this.current = this.from + (this.to - this.from) * ease(p);
    return this.current;
  }
}

export class DialsFace extends Face {
  readonly name = "Dials";
  readonly root: GameObject = new GameObject("Dials face");
  private pivots: GameObject[] = [];
  private turns: Turn[] = DIALS.map(() => new Turn());
  private readout!: TextMesh;
  private shown = "";

  constructor() {
    super();
    under(this.root, () => this.build());
    // Seen flat, the rings fill the view: a little smaller leaves room for the readout.
    this.root.transform.localScale = new Vector3(0.84, 0.84, 1);
  }

  private build(): void {
    for (const spec of DIALS) {
      const pivot = obj(`${spec.name} dial`, CENTRE);
      this.pivots.push(pivot);
      // The band, then its ticks and numbers, all turning with the pivot.
      shape(`${spec.name} band`, { points: Shapes.circle(Vector2.zero, spec.radius, 128), loop: true, width: 0.78, color: mix(PANEL, spec.colour, 0.08) }, Vector3.zero, pivot);
      for (let i = 0; i < spec.ticks; i++) {
        const deg = (i / spec.ticks) * 360;
        const a = (deg * Math.PI) / 180;
        // Clockwise from twelve.
        const dir = new Vector2(Math.sin(a), Math.cos(a));
        const numbered = i % spec.every === 0;
        if (!numbered) {
          shape(`${spec.name} tick ${i}`, { points: [dir.scale(spec.radius - 0.08), dir.scale(spec.radius + 0.08)], width: 0.025, color: DIM }, Vector3.zero, pivot);
          continue;
        }
        const n = label(
          `${spec.name} ${i}`,
          { text: spec.text(spec.ticks === 12 ? i : i), fontSize: spec.ticks === 12 ? 34 : 26, fontWeight: 600, color: spec.colour },
          new Vector3(dir.x * spec.radius, dir.y * spec.radius, 0),
          pivot,
        );
        // Its top points outward, so the one under the window stands upright.
        n.transform.localRotation = Quaternion.euler(0, 0, -deg);
      }
    }
    // The window at twelve: an outline over every dial, and a pointer.
    const top = DIALS[DIALS.length - 1]!.radius + 0.5;
    const bottom = DIALS[0]!.radius - 0.5;
    shape("window", { points: Shapes.roundedRectangle(-0.5, bottom, 1, top - bottom, 0.22), loop: true, width: 0.06, color: ACCENT, sortingOrder: 10 }, CENTRE);
    shape(
      "pointer",
      { points: Shapes.triangle({ x: -0.22, y: top + 0.36 }, { x: 0.22, y: top + 0.36 }, { x: 0, y: top + 0.06 }), loop: true, width: 0.03, color: ACCENT, fillColor: ACCENT, sortingOrder: 10 },
      CENTRE,
    );
    this.readout = label("readout", { fontSize: 26, color: DIM }, new Vector3(0, -4.55, 0));
  }

  update(now: DateTime, t: number): void {
    const hours = (now.hour % 12) + now.minute / 60;
    const angles = [
      this.turns[0]!.step(hours * 30, t, 0.8, easeInOutCubic),
      this.turns[1]!.step(now.minute * 6, t, 0.9, easeInOutCubic),
      this.turns[2]!.step(now.second * 6, t, 0.4, easeOutBack),
    ];
    // Each turns in its own plane to bring the time to the top.
    angles.forEach((a, i) => {
      this.pivots[i]!.transform.rotation = Quaternion.euler(0, 0, a);
    });
    const text = now.toString("HH:mm:ss");
    if (text !== this.shown) {
      this.shown = text;
      this.readout.text = text;
    }
  }
}
