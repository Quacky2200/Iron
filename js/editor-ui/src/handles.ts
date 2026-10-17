// Unity's transform handles (W move, E rotate, R scale) as plain math: where each part
// is on screen, which part the pointer is over, and what a drag does to the object.
// The view is abstract (`HandleView`: world to panel pixels and back), so the math is
// tested without a camera; SceneView.tsx draws the parts as SVG and feeds it drags.

import { Quaternion, type Ray, Vector3 } from "@iron/engine";

export type HandleTool = "move" | "rotate" | "scale";
/** A part of a handle: an axis, or the centre (free move in the view, uniform scale). */
export type HandlePart = "x" | "y" | "z" | "center";

/** What the handles need from the view. Pixels are the panel's, from its top-left. */
export interface HandleView {
  /** A world point in panel pixels; `z` is its distance in front of the eye. */
  toPanel(p: Vector3): Vector3;
  /** The ray through a panel point. */
  ray(x: number, y: number): Ray;
  /** World units per pixel at `p` (the size of things at its distance). */
  unitsPerPixel(p: Vector3): number;
  /** The way the view looks. */
  forward: Vector3;
}

/** How long the handles are on screen, in pixels. */
export const HANDLE_PIXELS = 90;
/** How close (pixels) the pointer must be to a part to grab it. */
export const GRAB_PIXELS = 8;

const AXES: Record<Exclude<HandlePart, "center">, Vector3> = {
  x: Vector3.right,
  y: Vector3.up,
  z: Vector3.back,
};

/** The object's place and axes (local: turned with the object). */
export interface HandleFrame {
  position: Vector3;
  rotation: Quaternion;
}

/** An axis's direction in the world. */
export function axisDirection(frame: HandleFrame, axis: Exclude<HandlePart, "center">): Vector3 {
  return frame.rotation.rotate(AXES[axis]);
}

/** The handles' length in world units at the object (a constant size on screen). */
export function handleLength(view: HandleView, frame: HandleFrame): number {
  return view.unitsPerPixel(frame.position) * HANDLE_PIXELS;
}

/** A part as drawn: a polyline in panel pixels (front-facing bits only, for rings). */
export interface HandleShape {
  part: HandlePart;
  /** Polylines: an axis line, or a ring's front arcs. */
  lines: { x: number; y: number }[][];
  /** Back arcs of a ring (drawn faint, not grabbable). */
  back: { x: number; y: number }[][];
  /** How far away it is, for the drawing order (farthest first). */
  depth: number;
}

const RING_SEGMENTS = 64;

/** Every part of `tool`'s handles as the view sees them, farthest first. */
export function handleShapes(tool: HandleTool, view: HandleView, frame: HandleFrame): HandleShape[] {
  const p = frame.position;
  const length = handleLength(view, frame);
  const at = (q: Vector3) => {
    const s = view.toPanel(q);
    return { x: s.x, y: s.y };
  };
  const out: HandleShape[] = [];
  for (const axis of ["x", "y", "z"] as const) {
    const dir = axisDirection(frame, axis);
    if (tool === "rotate") {
      const [u, v] = perpendicular(dir);
      const front: { x: number; y: number }[][] = [];
      const back: { x: number; y: number }[][] = [];
      let run: { x: number; y: number }[] = [];
      let runFront: boolean | null = null;
      for (let i = 0; i <= RING_SEGMENTS; i++) {
        const a = (i / RING_SEGMENTS) * Math.PI * 2;
        const offset = u.scale(Math.cos(a) * length).add(v.scale(Math.sin(a) * length));
        // The half facing the viewer (a hair of slack so edge-on rings still show).
        const isFront = offset.dot(view.forward) <= length * 0.02;
        const point = at(p.add(offset));
        if (runFront !== null && isFront !== runFront) {
          run.push(point);
          (runFront ? front : back).push(run);
          run = [];
        }
        run.push(point);
        runFront = isFront;
      }
      if (run.length > 1) (runFront ? front : back).push(run);
      out.push({ part: axis, lines: front, back, depth: 0 });
    } else {
      const end = p.add(dir.scale(length));
      out.push({ part: axis, lines: [[at(p), at(end)]], back: [], depth: view.toPanel(end).z });
    }
  }
  out.sort((a, b) => b.depth - a.depth);
  if (tool !== "rotate") {
    const c = at(p);
    out.push({ part: "center", lines: [[c]], back: [], depth: -Infinity });
  }
  return out;
}

/** Two unit directions at right angles to `n` and each other. */
function perpendicular(n: Vector3): [Vector3, Vector3] {
  const u = n.cross(Math.abs(n.y) < 0.9 ? Vector3.up : Vector3.right).normalized;
  return [u, n.cross(u).normalized];
}

/** The distance from `(x, y)` to the segment `a`–`b`. */
function toSegment(x: number, y: number, a: { x: number; y: number }, b: { x: number; y: number }): number {
  const dx = b.x - a.x;
  const dy = b.y - a.y;
  const len = dx * dx + dy * dy;
  const t = len > 0 ? Math.max(0, Math.min(1, ((x - a.x) * dx + (y - a.y) * dy) / len)) : 0;
  return Math.hypot(x - (a.x + t * dx), y - (a.y + t * dy));
}

/** The part under the pointer, if any: the centre first, then the nearest line. */
export function pickHandle(shapes: readonly HandleShape[], x: number, y: number): HandlePart | null {
  const center = shapes.find((s) => s.part === "center");
  if (center) {
    const c = center.lines[0]![0]!;
    if (Math.abs(x - c.x) <= GRAB_PIXELS && Math.abs(y - c.y) <= GRAB_PIXELS) return "center";
  }
  let best: HandlePart | null = null;
  let bestDistance = GRAB_PIXELS;
  for (const s of shapes) {
    if (s.part === "center") continue;
    for (const line of s.lines) {
      for (let i = 1; i < line.length; i++) {
        const d = toSegment(x, y, line[i - 1]!, line[i]!);
        if (d <= bestDistance) {
          bestDistance = d;
          best = s.part;
        }
      }
    }
  }
  return best;
}

/** The point on the line through `origin` along `dir` closest to `ray` (its parameter along `dir`). */
export function closestOnLine(origin: Vector3, dir: Vector3, ray: Ray): number {
  // Closest points between two lines: solve for the parameters along each.
  const w = origin.sub(ray.origin);
  const a = dir.dot(dir);
  const b = dir.dot(ray.direction);
  const c = ray.direction.dot(ray.direction);
  const d = dir.dot(w);
  const e = ray.direction.dot(w);
  const denom = a * c - b * b;
  // Looking straight along the axis: no grip on it.
  if (Math.abs(denom) < 1e-9) return 0;
  return (b * e - c * d) / denom;
}

/** Where `ray` meets the plane through `point` facing `normal` (null when parallel or behind). */
export function onPlane(point: Vector3, normal: Vector3, ray: Ray): Vector3 | null {
  const along = ray.direction.dot(normal);
  if (Math.abs(along) < 1e-9) return null;
  const t = point.sub(ray.origin).dot(normal) / along;
  return t < 0 ? null : ray.getPoint(t);
}

/** A drag in progress: what it started from. */
export interface HandleDrag {
  tool: HandleTool;
  part: HandlePart;
  /** The object as it was. */
  start: { position: Vector3; rotation: Quaternion; scale: Vector3 };
  /** The frame the handles were in (local axes at the start). */
  frame: HandleFrame;
  /** Where the pointer started, in panel pixels. */
  x0: number;
  y0: number;
  /** Where the grab began: along the axis (move/scale) or on the ring's plane (rotate). */
  grab: number;
  grabPoint: Vector3 | null;
}

/** Starts a drag on `part` at panel point `(x, y)`. */
export function beginDrag(
  tool: HandleTool,
  part: HandlePart,
  view: HandleView,
  object: { position: Vector3; rotation: Quaternion; scale: Vector3 },
  frame: HandleFrame,
  x: number,
  y: number,
): HandleDrag {
  const ray = view.ray(x, y);
  let grab = 0;
  let grabPoint: Vector3 | null = null;
  if (part === "center") {
    grabPoint = onPlane(frame.position, view.forward, ray);
  } else if (tool === "rotate") {
    grabPoint = onPlane(frame.position, axisDirection(frame, part), ray);
  } else {
    grab = closestOnLine(frame.position, axisDirection(frame, part), ray);
  }
  return { tool, part, start: object, frame, x0: x, y0: y, grab, grabPoint };
}

/** What the object becomes with the pointer at `(x, y)`; `snap` rounds (Ctrl in Unity). */
export function dragTo(
  drag: HandleDrag,
  view: HandleView,
  x: number,
  y: number,
  snap = false,
): { position: Vector3; rotation: Quaternion; scale: Vector3 } {
  const { start, frame, part, tool } = drag;
  const ray = view.ray(x, y);
  const round = (v: number, step: number) => (snap ? Math.round(v / step) * step : v);
  if (tool === "move") {
    if (part === "center") {
      const now = onPlane(frame.position, view.forward, ray);
      if (!now || !drag.grabPoint) return start;
      return { ...start, position: start.position.add(now.sub(drag.grabPoint)) };
    }
    const dir = axisDirection(frame, part);
    const t = round(closestOnLine(frame.position, dir, ray) - drag.grab, 1);
    return { ...start, position: start.position.add(dir.scale(t)) };
  }
  if (tool === "rotate") {
    if (part === "center") return start;
    const axis = axisDirection(frame, part);
    const now = onPlane(frame.position, axis, ray);
    let degrees: number;
    if (now && drag.grabPoint) {
      const a = drag.grabPoint.sub(frame.position);
      const b = now.sub(frame.position);
      degrees = (Math.atan2(a.cross(b).dot(axis), a.dot(b)) * 180) / Math.PI;
    } else {
      // The ring seen edge-on: turn by the drag across the screen instead.
      degrees = x - drag.x0;
    }
    degrees = round(degrees, 15);
    return { ...start, rotation: Quaternion.angleAxis(degrees, axis).mul(start.rotation).normalized };
  }
  // Scale.
  if (part === "center") {
    const f = Math.max(0.01, 1 + round((x - drag.x0 - (y - drag.y0)) / 100, 0.1));
    return { ...start, scale: start.scale.scale(f) };
  }
  const dir = axisDirection(frame, part);
  const t = closestOnLine(frame.position, dir, ray);
  const factor = Math.abs(drag.grab) > 1e-6 ? t / drag.grab : 1;
  const f = round(factor, 0.1);
  const s = start.scale;
  const scale = part === "x" ? new Vector3(s.x * f, s.y, s.z) : part === "y" ? new Vector3(s.x, s.y * f, s.z) : new Vector3(s.x, s.y, s.z * f);
  return { ...start, scale };
}
