// The meter face: a water meter / mileage counter. Each digit is a drum (a lit cylinder with
// the ten digits round it) that rolls forward to the next, carrying 9 to 0 like the real thing;
// the last drum creeps round continuously, as on a water meter. The drums sit in a housing whose
// window hides all but the current digit and a sliver of its neighbours, each resting a couple
// of degrees off true. Seen through a perspective camera, lit by a light of its own.
import {
  type Camera,
  Color,
  type DateTime,
  GameObject,
  Light,
  Material,
  MeshRenderer,
  Quaternion,
  type TextMesh,
  Vector3,
} from "@iron/engine";

import { DIM, type Face, label, obj, perspective, two, under } from "./common.ts";

const R = 1.15;
const WIDTH = 0.9;
const STEP = 36;
const ROLL = 0.55;
const HOUSING = Material.create({ color: Color.hex("#2c323b"), smoothness: 0.55, metallic: 0.4 });
const BLACK_DRUM = Material.create({ color: Color.hex("#16181d"), smoothness: 0.3 });
const RED_DRUM = Material.create({ color: Color.hex("#7d1717"), smoothness: 0.3 });
const DIGIT = Color.hex("#f3efe4");

const easeInOut = (x: number) => (x < 0.5 ? 2 * x * x : 1 - (-2 * x + 2) ** 2 / 2);

/** One drum: a cylinder with the digits 0 to 9 round it, rolled by turning its pivot. */
class Drum {
  readonly pivot: GameObject;
  private shown = Number.NaN;
  private from = 0;
  private to = 0;
  private at = -Infinity;
  /** Where it rests, a little off true, like a worn meter. */
  private readonly slop: number;

  constructor(name: string, x: number, red: boolean, slop: number) {
    this.slop = slop;
    this.pivot = obj(name, new Vector3(x, 0, 0));
    // The cylinder's axis is along y: lay it along x, sized to the drum.
    const body = obj(`${name} body`, Vector3.zero, this.pivot);
    body.transform.localRotation = Quaternion.euler(0, 0, 90);
    body.transform.localScale = new Vector3(R * 2, WIDTH / 2, R * 2);
    body.addComponent(MeshRenderer, { mesh: "cylinder", material: red ? RED_DRUM : BLACK_DRUM });
    for (let k = 0; k < 10; k++) {
      // Each digit on a holder turned k steps round the axis, standing just off the surface.
      const holder = obj(`${name} ${k}`, Vector3.zero, this.pivot);
      holder.transform.localRotation = Quaternion.euler(k * STEP, 0, 0);
      label(`${name} digit ${k}`, { text: String(k), fontSize: 66, fontWeight: 600, color: DIGIT, resolution: 3 }, new Vector3(0, 0, R + 0.01), holder);
    }
  }

  /** Rolls forward to `digit` (0 to 9, fractions creep between digits); `now` in seconds. */
  set(digit: number, now: number, smooth = false): void {
    if (Number.isNaN(this.shown)) {
      this.shown = digit;
      this.from = this.to = digit * STEP;
    } else if (smooth) {
      // Continuous: forward from wherever it is, wrapping 9.x to 0.x.
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
    // Rolling up: the next digit comes from below, as on a real counter.
    this.pivot.transform.localRotation = Quaternion.euler(-this.current(now) + this.slop, 0, 0);
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
  private readout!: TextMesh;
  private shownTime = "";

  constructor() {
    under(this.root, () => this.build());
  }

  private build(): void {
    const sun = obj("meter light");
    sun.transform.rotation = Quaternion.lookRotation(new Vector3(0.25, -0.55, -1));
    sun.addComponent(Light, { type: "directional", intensity: 1.6 });

    // Six drums, HH MM SS; the seconds are red.
    const gap = 0.07;
    const pairGap = 0.55;
    const total = 6 * WIDTH + 3 * gap + 2 * pairGap;
    let x = -total / 2 + WIDTH / 2;
    const slops = [1.8, -2.4, 0.9, -1.5, 2.6, -0.8];
    for (let i = 0; i < 6; i++) {
      this.drums.push(new Drum(`drum ${i}`, x, i >= 4, slops[i]!));
      x += WIDTH + (i % 2 === 1 ? pairGap : gap);
    }

    // The housing: bars in front of the drums leave a window over the middle band.
    const front = R + 0.32;
    const window = 1.05;
    const wide = total + 1.4;
    const bar = (name: string, cx: number, cy: number, w: number, h: number) => {
      const b = obj(name, new Vector3(cx, cy, front));
      b.transform.localScale = new Vector3(w, h, 0.3);
      b.addComponent(MeshRenderer, { mesh: "cube", material: HOUSING });
    };
    bar("housing top", 0, window / 2 + 0.9, wide, 1.8);
    bar("housing bottom", 0, -window / 2 - 0.9, wide, 1.8);
    bar("housing left", -total / 2 - 0.4, 0, 0.8, window);
    bar("housing right", total / 2 + 0.4, 0, 0.8, window);
    // Posts between the pairs.
    let px = -total / 2 + 2 * WIDTH + gap + pairGap / 2;
    for (let i = 0; i < 2; i++) {
      bar(`housing post ${i}`, px, 0, pairGap * 0.7, window);
      px += 2 * WIDTH + gap + pairGap;
    }
    // Names on the housing under each pair.
    const names = ["HOURS", "MINUTES", "SECONDS"];
    names.forEach((n, i) => {
      const cx = -total / 2 + WIDTH + gap / 2 + i * (2 * WIDTH + gap + pairGap);
      label(`meter ${n}`, { text: n, fontSize: 18, fontWeight: 600, color: DIM }, new Vector3(cx, -window / 2 - 0.35, front + 0.16));
    });
    this.readout = label("meter readout", { fontSize: 22, color: DIM }, new Vector3(0, window / 2 + 0.45, front + 0.16));
  }

  show(camera: Camera): void {
    this.restore = perspective(camera);
  }

  hide(): void {
    this.restore?.();
    this.restore = null;
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
    const text = now.toString("dddd d MMMM");
    if (text !== this.shownTime) {
      this.shownTime = text;
      this.readout.text = text;
    }
  }
}
