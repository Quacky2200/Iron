// SPDX-License-Identifier: Apache-2.0

// GDI-style shape outlines for `LineRenderer2D.points`: rectangles, ellipses, arcs, pies,
// polygons, Béziers and cardinal splines. The maths is native (`platypus_math::outlines`, the same
// functions compiled games call); the app installs it as the global `Shapes`. y is up, angles are
// degrees counter-clockwise from +x; closed shapes don't repeat their first point (set the
// renderer's `loop`; a `fillColor` always closes the shape).

import type { Vector2, Vector2Like } from "./math.ts";

export interface ShapesApi {
  /** A rectangle from its bottom-left corner (GDI's `DrawRectangle`). */
  rectangle(x: number, y: number, width: number, height: number): Vector2[];
  /** A rectangle with corners rounded to `radius` (at most half the shorter side). */
  roundedRectangle(x: number, y: number, width: number, height: number, radius: number, segments?: number): Vector2[];
  /** An ellipse around `center` with radii `rx` and `ry` (GDI's `DrawEllipse`). */
  ellipse(center: Vector2Like, rx: number, ry: number, segments?: number): Vector2[];
  circle(center: Vector2Like, radius: number, segments?: number): Vector2[];
  /** Part of an ellipse from `startAngle`, `sweepAngle` degrees round (negative goes clockwise), GDI's `DrawArc`. */
  arc(center: Vector2Like, rx: number, ry: number, startAngle: number, sweepAngle: number, segments?: number): Vector2[];
  /** A pie slice: the centre, then the arc (GDI's `DrawPie` / `FillPie`). */
  pie(center: Vector2Like, rx: number, ry: number, startAngle: number, sweepAngle: number, segments?: number): Vector2[];
  /** `sides` corners evenly round a circle, the first at `rotation` degrees (90: on top). */
  regularPolygon(center: Vector2Like, radius: number, sides: number, rotation?: number): Vector2[];
  /** The points as given (GDI's `DrawPolygon`). */
  polygon(...points: Vector2Like[]): Vector2[];
  triangle(a: Vector2Like, b: Vector2Like, c: Vector2Like): Vector2[];
  /** A cubic Bézier from `p0` to `p3`, pulled toward `p1` and `p2` (GDI's `DrawBezier`). */
  bezier(p0: Vector2Like, p1: Vector2Like, p2: Vector2Like, p3: Vector2Like, segments?: number): Vector2[];
  /** A smooth curve through every point (GDI's `DrawCurve`): `tension` 0 is straight lines, 0.5 a Catmull-Rom spline. */
  curve(points: readonly Vector2Like[], tension?: number, segments?: number): Vector2[];
  /** The same, closed back to the first point (GDI's `DrawClosedCurve` / `FillClosedCurve`). */
  closedCurve(points: readonly Vector2Like[], tension?: number, segments?: number): Vector2[];
}

export const Shapes: ShapesApi = (globalThis as unknown as { Shapes: ShapesApi }).Shapes;
