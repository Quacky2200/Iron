// The rainbow face: lots of colour. Seven bands chase each other round the outside, sixty
// ticks run round the hue wheel and light up as the seconds pass, the hands slowly cycle
// through the colours, confetti orbits the dial, and each character of the time below has a
// colour of its own.

import { Face, label, obj, shape, two, under } from "./common.ts";

const CENTRE = new Vector3(0, 0.4, 0);
const BANDS = ["#ff3b30", "#ff9500", "#ffcc00", "#34c759", "#00c7be", "#007aff", "#af52de"].map((h) => Color.hex(h));

/** A colour round the hue wheel (`h` 0 to 1). */
function hue(h: number, s = 0.9, l = 0.6): Color {
  const k = (n: number) => (n + h * 12) % 12;
  const a = s * Math.min(l, 1 - l);
  const f = (n: number) => l - a * Math.max(-1, Math.min(k(n) - 3, 9 - k(n), 1));
  return new Color(f(0), f(8), f(4), 1);
}

export class RainbowFace extends Face {
  readonly name = "Rainbow";
  readonly root: GameObject = new GameObject("Rainbow face");
  private bands: LineRenderer2D[] = [];
  private ticks: LineRenderer2D[] = [];
  private tickState: number[] = [];
  private hourHand!: LineRenderer2D;
  private minuteHand!: LineRenderer2D;
  private secondHand!: GameObject;
  private confetti: { go: GameObject; radius: number; speed: number; phase: number }[] = [];
  private chars: TextMesh[] = [];
  private shown = "";

  constructor() {
    super();
    under(this.root, () => this.build());
  }

  private build(): void {
    BANDS.forEach((c, i) => {
      this.bands.push(shape(`band ${i}`, { width: 0.11, color: c }, CENTRE));
    });
    for (let i = 0; i < 60; i++) {
      this.ticks.push(shape(`tick ${i}`, { width: 0.07, color: hue(i / 60), sortingOrder: 1 }, CENTRE));
      this.tickState.push(-1);
    }
    for (let i = 0; i < 28; i++) {
      const go = obj(`confetti ${i}`, CENTRE);
      const r = 0.05 + (i % 4) * 0.018;
      const c = hue((i * 0.137) % 1, 0.95, 0.62);
      shape(`confetti dot ${i}`, { points: Shapes.circle(Vector2.zero, r, 10), loop: true, width: 0.01, color: c, fillColor: c, sortingOrder: 2 }, Vector3.zero, go);
      this.confetti.push({ go, radius: 0.9 + (i % 7) * 0.22, speed: (i % 2 ? 1 : -1) * (0.12 + (i % 5) * 0.05), phase: i * 2.3 });
    }
    this.hourHand = shape("hour hand", { width: 0.26, sortingOrder: 3 }, CENTRE);
    this.minuteHand = shape("minute hand", { width: 0.16, sortingOrder: 4 }, CENTRE);
    this.secondHand = obj("second hand", CENTRE);
    shape("second needle", { points: [new Vector2(0, -0.4), new Vector2(0, 2.65)], width: 0.035, color: Color.white, sortingOrder: 5 }, Vector3.zero, this.secondHand);
    shape("hub", { points: Shapes.circle(Vector2.zero, 0.16, 20), loop: true, width: 0.04, color: Color.white, fillColor: Color.hex("#16121f"), sortingOrder: 6 }, CENTRE);
    // HH:MM:SS, one TextMesh per character so each gets its own colour.
    for (let i = 0; i < 8; i++) {
      this.chars.push(label(`char ${i}`, { fontSize: 44, fontWeight: 800 }, new Vector3(-1.75 + i * 0.5, -3.55, 0)));
    }
  }

  update(now: DateTime, t: number): void {
    const s = now.second + now.millisecond / 1000;
    const m = now.minute + s / 60;
    const h = (now.hour % 12) + m / 60;

    // The bands chase round, each a little behind the one outside it.
    this.bands.forEach((band, i) => {
      const r = 3.95 - i * 0.14;
      const start = 90 - (t * (24 + i * 3) + i * 18);
      band.points = Shapes.arc(Vector2.zero, r, r, start, -(250 + 40 * Math.sin(t * 0.5 + i * 0.4)), 72);
    });

    // Ticks: the ones the seconds have passed are long and bright; the current one pulses.
    this.ticks.forEach((tick, i) => {
      const passed = i <= now.second;
      const pulse = i === now.second ? 1 - Math.min(1, (s - now.second) * 2) : 0;
      const state = (passed ? 1 : 0) + pulse;
      if (Math.abs(state - this.tickState[i]!) < 0.02) return;
      this.tickState[i] = state;
      const a = (90 - i * 6) * (Math.PI / 180);
      const d = new Vector2(Math.cos(a), Math.sin(a));
      const inner = passed ? 2.7 - pulse * 0.25 : 2.95;
      tick.points = [d.scale(inner), d.scale(3.15)];
      tick.color = passed ? hue(i / 60, 0.95, 0.6 + pulse * 0.2) : hue(i / 60, 0.35, 0.28);
      tick.width = (i % 5 === 0 ? 0.1 : 0.06) + pulse * 0.05;
    });

    const hand = (line: LineRenderer2D, degrees: number, length: number, shift: number) => {
      const a = (90 - degrees) * (Math.PI / 180);
      line.points = [new Vector2(0, 0), new Vector2(Math.cos(a) * length, Math.sin(a) * length)];
      line.color = hue((t * 0.03 + shift) % 1, 0.95, 0.62);
    };
    hand(this.hourHand, h * 30, 1.5, 0);
    hand(this.minuteHand, m * 6, 2.35, 0.33);
    this.secondHand.transform.rotation = Quaternion.euler(0, 0, -s * 6);

    for (const c of this.confetti) {
      const a = t * c.speed + c.phase;
      const r = c.radius + 0.08 * Math.sin(t * 1.3 + c.phase);
      c.go.transform.position = CENTRE.add(new Vector3(Math.cos(a) * r, Math.sin(a) * r, 0));
    }

    const text = `${two(now.hour)}:${two(now.minute)}:${two(now.second)}`;
    if (text !== this.shown) {
      this.shown = text;
      this.chars.forEach((c, i) => {
        c.text = text.charAt(i);
        c.color = text.charAt(i) === ":" ? Color.hex("#8a8fa3") : hue((i / 8 + t * 0.02) % 1, 0.95, 0.62);
      });
    }
  }
}
