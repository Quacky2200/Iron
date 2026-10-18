// Transitions, ported from MiniGameEngine's `Transition` classes: a value eases from one
// target to the next over a duration. `loop` starts a new pass when one ends, `pingPong`
// runs it back to the start (a whole cycle), and `pick` chooses each new target (the
// random transitions). `halfDuration` makes each leg take half the duration, so a
// ping-pong cycle takes one `duration`.

export type Easing = (t: number) => number;

/** A CSS cubic-bezier easing curve, solved by bisection on x. */
export function cubicBezier(x1: number, y1: number, x2: number, y2: number): Easing {
  const bx = (s: number) => 3 * (1 - s) * (1 - s) * s * x1 + 3 * (1 - s) * s * s * x2 + s * s * s;
  const by = (s: number) => 3 * (1 - s) * (1 - s) * s * y1 + 3 * (1 - s) * s * s * y2 + s * s * s;
  return (t) => {
    let lo = 0;
    let hi = 1;
    for (let i = 0; i < 40; i++) {
      const mid = (lo + hi) / 2;
      if (bx(mid) < t) lo = mid;
      else hi = mid;
    }
    return by((lo + hi) / 2);
  };
}

/** The easings the original used (W3C CSS curves). */
export const Easings = {
  linear: (t: number) => t,
  easeIn: cubicBezier(0.42, 0, 1, 1),
  easeInOut: cubicBezier(1, 0.5, 0, 0),
};

export interface TweenOptions<T> {
  start: T;
  /** Seconds. */
  duration?: number;
  lerp: (a: T, b: T, t: number) => T;
  /** Picks the next target. Without it the target stays put. */
  pick?: () => T;
  easing?: Easing;
  loop?: boolean;
  pingPong?: boolean;
  halfDuration?: boolean;
  enabled?: boolean;
}

export class Tween<T> {
  value: T;
  duration: number;
  easing: Easing;
  loop: boolean;
  pingPong: boolean;
  halfDuration: boolean;
  enabled: boolean;
  pick: (() => T) | null;
  private a: T;
  private b: T;
  private t = 0;
  private forward = true;
  private readonly lerp: (a: T, b: T, t: number) => T;

  constructor(o: TweenOptions<T>) {
    this.a = o.start;
    this.b = o.pick ? o.pick() : o.start;
    this.value = o.start;
    this.lerp = o.lerp;
    this.duration = o.duration ?? 1;
    this.easing = o.easing ?? Easings.linear;
    this.loop = o.loop ?? false;
    this.pingPong = o.pingPong ?? false;
    this.halfDuration = o.halfDuration ?? false;
    this.enabled = o.enabled ?? false;
    this.pick = o.pick ?? null;
  }

  /** The value the current pass is heading to. */
  get target(): T {
    return this.b;
  }

  update(dt: number): void {
    if (!this.enabled) return;
    const length = this.duration * (this.halfDuration ? 0.5 : 1);
    this.t = Math.min(1, this.t + dt / Math.max(length, 0.001));
    const e = this.easing(this.forward ? this.t : 1 - this.t);
    this.value = this.lerp(this.a, this.b, e);
    if (this.t < 1) return;

    this.t = 0;
    if (this.pingPong) {
      this.forward = !this.forward;
      // A whole cycle is done once the value is back at the start: pick a new end.
      if (this.forward && this.pick) this.b = this.pick();
    } else if (this.loop) {
      this.a = this.b;
      if (this.pick) this.b = this.pick();
    } else {
      this.enabled = false;
    }
  }
}

/** Linear interpolation for numbers. */
export const lerpNumber = (a: number, b: number, t: number) => a + (b - a) * t;
