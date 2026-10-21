// GDI-style shape outlines for `LineRenderer2D.points`: rectangles, ellipses, arcs, pies,
// polygons, Béziers and cardinal splines (`DrawCurve` / `DrawClosedCurve`). Each returns the
// outline's points on the XY plane: y up, angles in degrees counter-clockwise from +x (GDI's
// y points down, so its clockwise angles come out mirrored). Closed shapes don't repeat their
// first point: set the renderer's `loop` (a `fillColor` always closes the shape).

import { Vector2, type Vector2Like } from "./math.ts";

const DEG = Math.PI / 180;
/** Points around a whole ellipse unless asked otherwise. */
const ROUND = 64;

const at = (center: Vector2Like, rx: number, ry: number, degrees: number) =>
  new Vector2(center.x + Math.cos(degrees * DEG) * rx, center.y + Math.sin(degrees * DEG) * ry);

/** A cardinal spline through `points`, `segments` points per span (Hermite form). */
function cardinal(points: readonly Vector2Like[], tension: number, segments: number, closed: boolean): Vector2[] {
  const n = points.length;
  if (n < 2) return points.map((p) => Vector2.from(p));
  const point = (i: number) => (closed ? points[((i % n) + n) % n]! : points[Math.max(0, Math.min(n - 1, i))]!);
  const spans = closed ? n : n - 1;
  const out: Vector2[] = [];
  for (let i = 0; i < spans; i++) {
    const p0 = point(i - 1);
    const p1 = point(i);
    const p2 = point(i + 1);
    const p3 = point(i + 2);
    // GDI+'s tangents: `tension` times the chord across each point (0.5 is Catmull-Rom).
    const m1x = tension * (p2.x - p0.x);
    const m1y = tension * (p2.y - p0.y);
    const m2x = tension * (p3.x - p1.x);
    const m2y = tension * (p3.y - p1.y);
    for (let k = 0; k < segments; k++) {
      const t = k / segments;
      const t2 = t * t;
      const t3 = t2 * t;
      const h00 = 2 * t3 - 3 * t2 + 1;
      const h10 = t3 - 2 * t2 + t;
      const h01 = -2 * t3 + 3 * t2;
      const h11 = t3 - t2;
      out.push(new Vector2(h00 * p1.x + h10 * m1x + h01 * p2.x + h11 * m2x, h00 * p1.y + h10 * m1y + h01 * p2.y + h11 * m2y));
    }
  }
  if (!closed) out.push(Vector2.from(points[n - 1]!));
  return out;
}

export const Shapes = {
  /** A rectangle from its bottom-left corner (GDI's `DrawRectangle`). */
  rectangle(x: number, y: number, width: number, height: number): Vector2[] {
    return [new Vector2(x, y), new Vector2(x + width, y), new Vector2(x + width, y + height), new Vector2(x, y + height)];
  },

  /** A rectangle with corners rounded to `radius` (at most half the shorter side). */
  roundedRectangle(x: number, y: number, width: number, height: number, radius: number, segments = 8): Vector2[] {
    const r = Math.max(0, Math.min(radius, width / 2, height / 2));
    if (r === 0) return Shapes.rectangle(x, y, width, height);
    const out: Vector2[] = [];
    const corner = (cx: number, cy: number, start: number) => {
      for (let i = 0; i <= segments; i++) out.push(at({ x: cx, y: cy }, r, r, start + (90 * i) / segments));
    };
    corner(x + width - r, y + r, -90);
    corner(x + width - r, y + height - r, 0);
    corner(x + r, y + height - r, 90);
    corner(x + r, y + r, 180);
    return out;
  },

  /** An ellipse around `center` with radii `rx` and `ry` (GDI's `DrawEllipse`). */
  ellipse(center: Vector2Like, rx: number, ry: number, segments = ROUND): Vector2[] {
    return Array.from({ length: segments }, (_, i) => at(center, rx, ry, (360 * i) / segments));
  },

  circle(center: Vector2Like, radius: number, segments = ROUND): Vector2[] {
    return Shapes.ellipse(center, radius, radius, segments);
  },

  /**
   * Part of an ellipse from `startAngle`, `sweepAngle` degrees round (negative goes clockwise),
   * GDI's `DrawArc`. Open: both ends included.
   */
  arc(center: Vector2Like, rx: number, ry: number, startAngle: number, sweepAngle: number, segments?: number): Vector2[] {
    const n = Math.max(1, segments ?? Math.ceil((Math.abs(sweepAngle) / 360) * ROUND));
    return Array.from({ length: n + 1 }, (_, i) => at(center, rx, ry, startAngle + (sweepAngle * i) / n));
  },

  /** A pie slice: the centre, then the arc (GDI's `DrawPie` / `FillPie`). */
  pie(center: Vector2Like, rx: number, ry: number, startAngle: number, sweepAngle: number, segments?: number): Vector2[] {
    return [Vector2.from(center), ...Shapes.arc(center, rx, ry, startAngle, sweepAngle, segments)];
  },

  /** `sides` corners evenly round a circle, the first at `rotation` degrees (90: on top). */
  regularPolygon(center: Vector2Like, radius: number, sides: number, rotation = 90): Vector2[] {
    const n = Math.max(3, Math.round(sides));
    return Array.from({ length: n }, (_, i) => at(center, radius, radius, rotation + (360 * i) / n));
  },

  /** The points as given (GDI's `DrawPolygon`). */
  polygon(...points: Vector2Like[]): Vector2[] {
    return points.map((p) => Vector2.from(p));
  },

  triangle(a: Vector2Like, b: Vector2Like, c: Vector2Like): Vector2[] {
    return Shapes.polygon(a, b, c);
  },

  /** A cubic Bézier from `p0` to `p3`, pulled toward `p1` and `p2` (GDI's `DrawBezier`). */
  bezier(p0: Vector2Like, p1: Vector2Like, p2: Vector2Like, p3: Vector2Like, segments = 32): Vector2[] {
    return Array.from({ length: segments + 1 }, (_, i) => {
      const t = i / segments;
      const u = 1 - t;
      const a = u * u * u;
      const b = 3 * u * u * t;
      const c = 3 * u * t * t;
      const d = t * t * t;
      return new Vector2(a * p0.x + b * p1.x + c * p2.x + d * p3.x, a * p0.y + b * p1.y + c * p2.y + d * p3.y);
    });
  },

  /**
   * A smooth curve through every point (GDI's `DrawCurve`): `tension` 0 is straight lines,
   * 0.5 (GDI's default) a Catmull-Rom spline; `segments` points per span.
   */
  curve(points: readonly Vector2Like[], tension = 0.5, segments = 8): Vector2[] {
    return cardinal(points, tension, segments, false);
  },

  /** The same, closed back to the first point (GDI's `DrawClosedCurve` / `FillClosedCurve`). */
  closedCurve(points: readonly Vector2Like[], tension = 0.5, segments = 8): Vector2[] {
    return cardinal(points, tension, segments, true);
  },
};
