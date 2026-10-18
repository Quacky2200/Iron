// The bedside alarm clock: red seven-segment digits glowing through bloom. Lit segments are
// brighter than white (linear colour above 1), so only they bloom; unlit ones stay faintly
// visible, as on a real display. The colon blinks with the seconds.
import { type Camera, Color, type DateTime, GameObject, type LineRenderer2D, RenderSettings, Shapes, type TextMesh, Vector2, Vector3 } from "@iron/engine";

import { type Face, label, shape, two, under } from "./common.ts";

const LIT = new Color(4, 0.16, 0.08, 1);
const UNLIT = new Color(0.07, 0.012, 0.01, 1);
const BLACK = Color.hex("#050405");

/** Segments a to g: [from, to] in a 1 x 2 box (y up). */
const SEGMENTS: [Vector2, Vector2][] = [
  [new Vector2(0.16, 2), new Vector2(0.84, 2)], // a: top
  [new Vector2(1, 1.84), new Vector2(1, 1.16)], // b: top right
  [new Vector2(1, 0.84), new Vector2(1, 0.16)], // c: bottom right
  [new Vector2(0.16, 0), new Vector2(0.84, 0)], // d: bottom
  [new Vector2(0, 0.84), new Vector2(0, 0.16)], // e: bottom left
  [new Vector2(0, 1.84), new Vector2(0, 1.16)], // f: top left
  [new Vector2(0.16, 1), new Vector2(0.84, 1)], // g: middle
];
/** Which segments each digit lights. */
const LIGHTS = ["abcdef", "bc", "abged", "abgcd", "fgbc", "afgcd", "afgedc", "abc", "abcdefg", "abcdfg"];
/** The display leans forward a little. */
const lean = (p: Vector2) => new Vector2(p.x + p.y * 0.12, p.y);

class Digit {
  private segments: LineRenderer2D[];
  private shown = -1;
  constructor(name: string, at: Vector3, size: number) {
    this.segments = SEGMENTS.map(([a, b], i) =>
      shape(`${name} ${"abcdefg"[i]}`, { points: [lean(a).scale(size), lean(b).scale(size)], width: 0.2 * size, color: UNLIT }, at),
    );
  }

  /** Lights `digit` (-1: blank). */
  set(digit: number): void {
    if (digit === this.shown) return;
    this.shown = digit;
    const on = LIGHTS[digit] ?? "";
    this.segments.forEach((s, i) => {
      s.color = on.includes("abcdefg"[i]!) ? LIT : UNLIT;
    });
  }
}

export class SegmentFace implements Face {
  readonly name = "Alarm clock";
  readonly root = new GameObject("Alarm clock face");
  private digits: Digit[] = [];
  private seconds: Digit[] = [];
  private colon: LineRenderer2D[] = [];
  private pm!: TextMesh;
  private background: Color | null = null;

  constructor() {
    under(this.root, () => this.build());
  }

  private build(): void {
    const size = 1.6;
    const w = size;
    const gap = 0.55;
    const total = 4 * w + 3 * gap + 0.6;
    let x = -total / 2;
    const y = -1.3;
    for (let i = 0; i < 4; i++) {
      this.digits.push(new Digit(`digit ${i}`, new Vector3(x, y, 0), size));
      x += w + gap + (i === 1 ? 0.6 : 0);
    }
    // Halfway between the hours and the minutes, leaning with the digits.
    const colonX = -total / 2 + 2 * w + gap + (gap + 0.6) / 2;
    for (const dy of [0.85, 2.35]) {
      this.colon.push(shape("colon", { points: Shapes.circle(Vector2.zero, 0.16, 20), loop: true, width: 0.02, color: LIT, fillColor: LIT }, new Vector3(colonX + dy * 0.12, y + dy, 0)));
    }
    // Small seconds, bottom right.
    for (let i = 0; i < 2; i++) this.seconds.push(new Digit(`second ${i}`, new Vector3(total / 2 + 0.65 + i * 0.75, y, 0), 0.55));
    this.pm = label("pm", { text: "PM", fontSize: 26, fontWeight: 700, color: LIT }, new Vector3(total / 2 + 0.95, y + 2.85, 0));
  }

  show(camera: Camera): void {
    this.background = camera.backgroundColor;
    camera.backgroundColor = BLACK;
    RenderSettings.bloom = { threshold: 1.0, intensity: 1.4, scatter: 0.72 };
  }

  hide(camera: Camera): void {
    if (this.background) camera.backgroundColor = this.background;
    RenderSettings.bloom = null;
  }

  update(now: DateTime): void {
    const hour12 = now.hour % 12 === 0 ? 12 : now.hour % 12;
    const text = `${String(hour12).padStart(2, " ")}${two(now.minute)}`;
    this.digits.forEach((d, i) => {
      // A blank leading hour digit lights nothing.
      const ch = text[i]!;
      d.set(ch === " " ? -1 : Number(ch));
    });
    const sec = two(now.second);
    this.seconds.forEach((d, i) => d.set(Number(sec[i])));
    const on = now.millisecond < 500;
    for (const c of this.colon) {
      c.color = on ? LIT : UNLIT;
      c.fillColor = on ? LIT : UNLIT;
    }
    this.pm.color = now.hour >= 12 ? LIT : new Color(0.12, 0.02, 0.02, 1);
  }
}
