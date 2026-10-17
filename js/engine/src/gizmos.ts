// Unity's `Gizmos`: wire shapes for editor views (a camera's frustum, a light's range).
// Draw calls collect line segments in world space, through `Gizmos.matrix` and in
// `Gizmos.color`; `Gizmos.present(camera)` hands them to one camera, which shows them
// (thin, a constant width on screen, faint where hidden) until it's given new ones.

import { Color, Matrix4x4, Vector3, type Vector3Like } from "./math.ts";
import type { Camera } from "./rendering.ts";

const DEG = Math.PI / 180;
/** Segments in a full circle. */
const CIRCLE = 48;

let lines: number[] = [];

function push(a: Vector3, b: Vector3): void {
  const m = Gizmos.matrix;
  const p = m.multiplyPoint3x4(a);
  const q = m.multiplyPoint3x4(b);
  const c = Gizmos.color;
  lines.push(p.x, p.y, p.z, q.x, q.y, q.z, c.r, c.g, c.b, c.a);
}

/** Two directions at right angles to `normal` (and to each other). */
function basis(normal: Vector3): [Vector3, Vector3] {
  const n = normal.normalized;
  const u = n.cross(Math.abs(n.y) < 0.9 ? Vector3.up : Vector3.right).normalized;
  return [u, n.cross(u)];
}

export const Gizmos = {
  /** The colour of what's drawn next (straight alpha). */
  color: Color.white as Color,

  /** Places what's drawn next (`Matrix4x4.trs(...)` for a shape in an object's space). */
  matrix: Matrix4x4.identity as Matrix4x4,

  drawLine(from: Vector3Like, to: Vector3Like): void {
    push(Vector3.from(from), Vector3.from(to));
  },

  /** A line from `from` along `direction` (its length included). */
  drawRay(from: Vector3Like, direction: Vector3Like): void {
    const f = Vector3.from(from);
    push(f, f.add(direction));
  },

  /** The edges of a box. */
  drawWireCube(center: Vector3Like, size: Vector3Like): void {
    const c = Vector3.from(center);
    const h = Vector3.from(size).scale(0.5);
    const corner = (i: number) => c.add(new Vector3(i & 1 ? h.x : -h.x, i & 2 ? h.y : -h.y, i & 4 ? h.z : -h.z));
    for (let i = 0; i < 8; i++) {
      for (const bit of [1, 2, 4]) {
        if ((i & bit) === 0) push(corner(i), corner(i | bit));
      }
    }
  },

  /** A circle about `normal` (Unity's `Handles.DrawWireDisc`). */
  drawWireDisc(center: Vector3Like, normal: Vector3Like, radius: number): void {
    const c = Vector3.from(center);
    const [u, v] = basis(Vector3.from(normal));
    let prev = c.add(u.scale(radius));
    for (let i = 1; i <= CIRCLE; i++) {
      const a = (i / CIRCLE) * Math.PI * 2;
      const p = c.add(u.scale(Math.cos(a) * radius)).add(v.scale(Math.sin(a) * radius));
      push(prev, p);
      prev = p;
    }
  },

  /** A sphere as three circles, one about each axis. */
  drawWireSphere(center: Vector3Like, radius: number): void {
    for (const n of [Vector3.right, Vector3.up, Vector3.back]) Gizmos.drawWireDisc(center, n, radius);
  },

  /**
   * A view frustum from `center` looking along -z (forward), like a camera's: `fov`
   * degrees tall, `aspect` wide per tall, from `minRange` to `maxRange`.
   */
  drawFrustum(center: Vector3Like, fov: number, maxRange: number, minRange: number, aspect: number): void {
    const c = Vector3.from(center);
    const t = Math.tan((fov / 2) * DEG);
    const rect = (d: number) => {
      const h = d * t;
      const w = h * aspect;
      return [new Vector3(-w, -h, -d), new Vector3(w, -h, -d), new Vector3(w, h, -d), new Vector3(-w, h, -d)].map((p) => c.add(p));
    };
    const near = rect(minRange);
    const far = rect(maxRange);
    for (let i = 0; i < 4; i++) {
      push(near[i]!, near[(i + 1) % 4]!);
      push(far[i]!, far[(i + 1) % 4]!);
      push(near[i]!, far[i]!);
    }
  },

  /** How many lines are waiting to be presented. */
  get lineCount(): number {
    return lines.length / 10;
  },

  /** Drops what's been drawn since the last present. */
  clear(): void {
    lines = [];
  },

  /**
   * Gives what's been drawn to `camera` (it shows those lines until the next present,
   * so presenting nothing clears them) and starts over.
   */
  present(camera: Camera): void {
    __iron_op("render.gizmos", camera.id, lines);
    lines = [];
  },
};
