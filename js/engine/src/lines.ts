// `LineRenderer`: a smooth round tube through a list of points (ropes, cables, trails,
// laser beams). Unlike Unity's flat, camera-facing ribbon it's real geometry, lit like any
// mesh and the same from every side. It rebuilds its mesh when its points change.

import { Color, type Vector3Like, Vector3 } from "./math.ts";
import { Material, Mesh, MeshRenderer } from "./rendering.ts";
import { Component } from "./scene.ts";

/** Catmull-Rom point between b and c (t 0..1), with a and d the neighbours. */
function catmullRom(a: Vector3, b: Vector3, c: Vector3, d: Vector3, t: number): Vector3 {
  const t2 = t * t;
  const t3 = t2 * t;
  const k = (p0: number, p1: number, p2: number, p3: number) =>
    0.5 * (2 * p1 + (-p0 + p2) * t + (2 * p0 - 5 * p1 + 4 * p2 - p3) * t2 + (-p0 + 3 * p1 - 3 * p2 + p3) * t3);
  return new Vector3(k(a.x, b.x, c.x, d.x), k(a.y, b.y, c.y, d.y), k(a.z, b.z, c.z, d.z));
}

/** Tube geometry through `points` (world space): rings of `sides` vertices. */
export function tubeMesh(points: readonly Vector3Like[], radius: number, sides: number, smoothing: number) {
  const src = points.map((p) => Vector3.from(p));
  // Smooth the path: `smoothing` extra points between each pair.
  const path: Vector3[] = [];
  for (let i = 0; i < src.length - 1; i++) {
    const a = src[Math.max(0, i - 1)]!;
    const b = src[i]!;
    const c = src[i + 1]!;
    const d = src[Math.min(src.length - 1, i + 2)]!;
    for (let s = 0; s <= smoothing; s++) path.push(catmullRom(a, b, c, d, s / (smoothing + 1)));
  }
  path.push(src[src.length - 1]!);
  const n = path.length;
  const tangents = path.map((_, i) => path[Math.min(n - 1, i + 1)]!.sub(path[Math.max(0, i - 1)]!).normalized);
  // A frame carried along the path (parallel transport), so rings don't twist.
  let normal = Math.abs(tangents[0]!.y) < 0.9 ? Vector3.up : Vector3.right;
  normal = normal.sub(tangents[0]!.scale(normal.dot(tangents[0]!))).normalized;
  const vertices: Vector3[] = [];
  const normals: Vector3[] = [];
  const uv: { x: number; y: number }[] = [];
  let along = 0;
  for (let i = 0; i < n; i++) {
    const t = tangents[i]!;
    if (i > 0) {
      along += path[i]!.distance(path[i - 1]!);
      // Remove the part of the normal along the new tangent.
      const fixed = normal.sub(t.scale(normal.dot(t)));
      if (fixed.magnitude > 1e-6) normal = fixed.normalized;
    }
    const binormal = t.cross(normal);
    for (let s = 0; s <= sides; s++) {
      const a = (s / sides) * Math.PI * 2;
      const dir = normal.scale(Math.cos(a)).add(binormal.scale(Math.sin(a)));
      vertices.push(path[i]!.add(dir.scale(radius)));
      normals.push(dir);
      uv.push({ x: s / sides, y: along });
    }
  }
  const triangles: number[] = [];
  const ring = sides + 1;
  for (let i = 0; i < n - 1; i++) {
    for (let s = 0; s < sides; s++) {
      const a = i * ring + s;
      const b = a + ring;
      triangles.push(a, a + 1, b, a + 1, b + 1, b);
    }
  }
  return { vertices, normals, uv, triangles };
}

/** Draws a round tube through `positions` (world space), Unity's `LineRenderer` as a tube. */
export class LineRenderer extends Component {
  /** The points it goes through, in world space. Set a new array to redraw. */
  positions: Vector3Like[] = [];
  /** Tube thickness (diameter, metres). */
  width = 0.05;
  /** Points around the tube. */
  sides = 8;
  /** Extra points between each pair of positions, for curves. */
  smoothing = 3;
  material = new Material(Color.white, Color.black, 0.3);
  private mesh: Mesh | null = null;
  private renderer: MeshRenderer | null = null;
  private drawn = "";

  override onAwake(): void {
    this.mesh = Mesh.create({ vertices: [Vector3.zero, Vector3.zero, Vector3.zero], triangles: [0, 1, 2] });
    // The tube is in world space: the renderer sits on this object, which stays put.
    this.renderer = this.gameObject.addComponent(MeshRenderer, { mesh: this.mesh, material: this.material });
    this.redraw();
  }

  override onPreRender(): void {
    this.redraw();
  }

  /** Rebuilds the tube if the points or its look changed. */
  redraw(): void {
    if (!this.mesh) return;
    if (this.renderer && this.renderer.material !== this.material) this.renderer.material = this.material;
    const key = `${this.width},${this.sides},${this.smoothing},${this.positions.map((p) => `${p.x},${p.y},${p.z}`).join(";")}`;
    if (key === this.drawn || this.positions.length < 2) return;
    this.drawn = key;
    // World-space points into this object's space (usually it stays at the origin).
    const local = this.positions.map((p) => this.transform.inverseTransformPoint(p));
    this.mesh.set(tubeMesh(local, this.width / 2, Math.max(3, Math.round(this.sides)), Math.max(0, Math.round(this.smoothing))));
  }

  override onDestroy(): void {
    this.mesh?.destroy();
    this.mesh = null;
  }
}
