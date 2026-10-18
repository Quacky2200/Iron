// The meter face: a mileage counter with the case taken away. Each digit sits on an invisible
// drum and rolls forward to the next, carrying 9 to 0 like the real thing; the last one creeps
// round all the time, as on a water meter. Digits fade as they turn away from the front, and
// they're brighter than white, so they glow through bloom. Each drum rests a couple of degrees
// off true. Seen through a perspective camera.
import { type Camera, Color, type DateTime, GameObject, Quaternion, RenderSettings, type TextMesh, Vector3 } from "@iron/engine";

import { type Face, label, obj, perspective, two, under } from "./common.ts";

const R = 1.15;
const WIDTH = 0.9;
const STEP = 36;
const ROLL = 0.55;
/** Brighter than white (linear), so the digits bloom. */
const WARM = new Color(2.4, 2.0, 1.45, 1);
const RED = new Color(3.2, 0.42, 0.3, 1);
const BLACK = Color.hex("#07090d");

const easeInOut = (x: number) => (x < 0.5 ? 2 * x * x : 1 - (-2 * x + 2) ** 2 / 2);

/** One drum: the digits 0 to 9 round an invisible cylinder, rolled by turning its pivot. */
class Drum {
  readonly pivot: GameObject;
  private digits: TextMesh[] = [];
  /** Each digit's last fade, so only changes are sent. */
  private fades: number[] = [];
  private shown = Number.NaN;
  private from = 0;
  private to = 0;
  private at = -Infinity;

  constructor(
    name: string,
    x: number,
    private readonly colour: Color,
    /** Where it rests, a little off true, like a worn counter. */
    private readonly slop: number,
  ) {
    this.pivot = obj(name, new Vector3(x, 0, 0));
    for (let k = 0; k < 10; k++) {
      // Each digit on a holder turned k steps round the axis, out at the drum's radius.
      const holder = obj(`${name} ${k}`, Vector3.zero, this.pivot);
      holder.transform.localRotation = Quaternion.euler(k * STEP, 0, 0);
      this.digits.push(label(`${name} digit ${k}`, { text: String(k), fontSize: 66, fontWeight: 600, color: colour, resolution: 3 }, new Vector3(0, 0, R), holder));
      this.fades.push(-1);
    }
  }

  /** Rolls forward to `digit` (0 to 9; with `smooth`, fractions creep between digits). */
  set(digit: number, now: number, smooth = false): void {
    if (Number.isNaN(this.shown)) {
      this.shown = digit;
      this.from = this.to = digit * STEP;
    } else if (smooth) {
      let to = digit * STEP;
      while (to < this.to - 1e-6) to += 360;
      this.from = this.to = to;
      this.shown = digit;
    } else if (digit !== this.shown) {
      const steps = (((digit - this.shown) % 10) + 10) % 10;
      this.from = this.current(now);
      this.to += steps * STEP;
      this.shown = digit;
      this.at = now;
    }
    const angle = this.current(now);
    // Rolling up: the next digit comes from below, as on a real counter.
    this.pivot.transform.localRotation = Quaternion.euler(-angle + this.slop, 0, 0);
    // Digits fade as they turn away from the front; the far side isn't seen at all.
    this.digits.forEach((d, k) => {
      const facing = Math.cos(((k * STEP - angle + this.slop) * Math.PI) / 180);
      const fade = Math.round(Math.max(0, Math.min(1, (facing - 0.55) / 0.45)) ** 1.6 * 48) / 48;
      if (fade === this.fades[k]) return;
      this.fades[k] = fade;
      const c = this.colour;
      d.color = new Color(c.r, c.g, c.b, fade);
    });
  }

  private current(now: number): number {
    const p = Math.min(1, (now - this.at) / ROLL);
    return this.from + (this.to - this.from) * easeInOut(p);
  }
}

export class MeterFace implements Face {
  readonly name = "Meter";
  readonly root = new GameObject("Meter face");
  private drums: Drum[] = [];
  private restore: (() => void) | null = null;
  private background: Color | null = null;

  constructor() {
    under(this.root, () => this.build());
  }

  private build(): void {
    // Six drums, HH MM SS; the seconds glow red.
    const gap = 0.07;
    const pairGap = 0.55;
    const total = 6 * WIDTH + 3 * gap + 2 * pairGap;
    let x = -total / 2 + WIDTH / 2;
    const slops = [1.8, -2.4, 0.9, -1.5, 2.6, -0.8];
    for (let i = 0; i < 6; i++) {
      this.drums.push(new Drum(`drum ${i}`, x, i >= 4 ? RED : WARM, slops[i]!));
      x += WIDTH + (i % 2 === 1 ? pairGap : gap);
    }
  }

  show(camera: Camera): void {
    this.restore = perspective(camera);
    this.background = camera.backgroundColor;
    camera.backgroundColor = BLACK;
    RenderSettings.bloom = { threshold: 1.0, intensity: 1.1, scatter: 0.7 };
  }

  hide(camera: Camera): void {
    this.restore?.();
    this.restore = null;
    if (this.background) camera.backgroundColor = this.background;
    RenderSettings.bloom = null;
  }

  update(now: DateTime, t: number): void {
    const hh = two(now.hour);
    const mm = two(now.minute);
    const ss = two(now.second);
    this.drums[0]!.set(Number(hh[0]), t);
    this.drums[1]!.set(Number(hh[1]), t);
    this.drums[2]!.set(Number(mm[0]), t);
    this.drums[3]!.set(Number(mm[1]), t);
    this.drums[4]!.set(Number(ss[0]), t);
    // The last drum creeps round all the time, like a water meter's.
    this.drums[5]!.set((now.second % 10) + now.millisecond / 1000, t, true);
  }
}
