// Sparks, dust, speed lines and confetti: a pool of short LineRenderer2D capsules.
// A particle's length and width are set when it's emitted; after that it only moves and fades.
import { Color, GameObject, LineRenderer2D, Vector2, Vector3 } from "@iron/engine";

interface Particle {
  go: GameObject;
  line: LineRenderer2D;
  on: boolean;
  x: number;
  y: number;
  vx: number;
  vy: number;
  age: number;
  life: number;
  color: [number, number, number, number];
  gravity: number;
  drag: number;
}

export interface Emit {
  x: number;
  y: number;
  vx: number;
  vy: number;
  life: number;
  color: [number, number, number, number];
  /** Capsule length; near 0 makes a round dot. */
  len?: number;
  width?: number;
  /** Downwards, in units per second squared (negative rises). */
  gravity?: number;
  /** Fraction of speed lost per second. */
  drag?: number;
  /** Degrees, for a particle that starts still (speed lines). */
  angle?: number;
}

export class Fx {
  private pool: Particle[] = [];
  private next = 0;

  constructor(count: number) {
    for (let i = 0; i < count; i++) {
      const go = new GameObject(`Fx ${i}`, { position: new Vector3(0, 0, 0.3) });
      go.enabled = false;
      const line = go.addComponent(LineRenderer2D, { points: [new Vector2(-0.2, 0), Vector2.zero], width: 0.05, color: Color.white, loop: false });
      this.pool.push({ go, line, on: false, x: 0, y: 0, vx: 0, vy: 0, age: 0, life: 1, color: [1, 1, 1, 1], gravity: 0, drag: 0 });
    }
  }

  /** Starts a particle, reusing the oldest one when the pool is full. */
  emit(e: Emit): void {
    const p = this.pool[this.next]!;
    this.next = (this.next + 1) % this.pool.length;
    p.on = true;
    p.x = e.x;
    p.y = e.y;
    p.vx = e.vx;
    p.vy = e.vy;
    p.age = 0;
    p.life = e.life;
    p.color = e.color;
    p.gravity = e.gravity ?? 0;
    p.drag = e.drag ?? 0;
    p.line.points = [new Vector2(-(e.len ?? 0.2), 0), Vector2.zero];
    p.line.width = e.width ?? 0.05;
    p.line.color = new Color(...e.color);
    const heading = e.vx === 0 && e.vy === 0 ? (e.angle ?? 0) : (Math.atan2(e.vy, e.vx) * 180) / Math.PI;
    p.go.transform.localEulerAngles = new Vector3(0, 0, heading);
    p.go.transform.position = new Vector3(p.x, p.y, 0.3);
    p.go.enabled = true;
  }

  update(dt: number): void {
    for (const p of this.pool) {
      if (!p.on) continue;
      p.age += dt;
      if (p.age >= p.life) {
        p.on = false;
        p.go.enabled = false;
        continue;
      }
      const keep = Math.max(0, 1 - p.drag * dt);
      p.vx *= keep;
      p.vy = p.vy * keep - p.gravity * dt;
      p.x += p.vx * dt;
      p.y += p.vy * dt;
      p.go.transform.position = new Vector3(p.x, p.y, 0.3);
      if (p.gravity !== 0) p.go.transform.localEulerAngles = new Vector3(0, 0, (Math.atan2(p.vy, p.vx) * 180) / Math.PI);
      const [r, g, b, a] = p.color;
      p.line.color = new Color(r, g, b, a * (1 - p.age / p.life));
    }
  }
}
