// Differential oracle for the native `Shapes`: the TypeScript the compiled facade used
// (`tools/iron_ts/src/lower/facade.ts` before they became native Rust, renamed `Ref*`), run next to
// the native API on the same inputs. Every point must be the same double (bit for bit). Test
// code only; the engine has one implementation (iron_math). tests/all/outlines.rs reads the results.

function __rshapes_at(cx: number, cy: number, rx: number, ry: number, degrees: number): Vector2 {
  return new Vector2(cx + Math.cos(degrees * 0.017453292519943295) * rx, cy + Math.sin(degrees * 0.017453292519943295) * ry);
}
function __rshapes_pick(points: Vector2[], i: number, closed: boolean): Vector2 {
  const n = points.length;
  if (closed) return points[((i % n) + n) % n]!;
  return points[Math.max(0, Math.min(n - 1, i))]!;
}
/** A cardinal spline through `points`, `segments` points per span (Hermite form). */
function __rshapes_cardinal(points: Vector2[], tension: number, segments: number, closed: boolean): Vector2[] {
  const n = points.length;
  const out: Vector2[] = [];
  if (n < 2) {
    for (let i = 0; i < n; i++) out.push(new Vector2(points[i]!.x, points[i]!.y));
    return out;
  }
  const spans = closed ? n : n - 1;
  for (let i = 0; i < spans; i++) {
    const p0 = __rshapes_pick(points, i - 1, closed);
    const p1 = __rshapes_pick(points, i, closed);
    const p2 = __rshapes_pick(points, i + 1, closed);
    const p3 = __rshapes_pick(points, i + 2, closed);
    // GDI+ tangents: `tension` times the chord across each point (0.5 is Catmull-Rom).
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
  if (!closed) out.push(new Vector2(points[n - 1]!.x, points[n - 1]!.y));
  return out;
}
/** GDI-style shape outlines for `LineRenderer2D.points`. */
class RefShapes {
  /** A rectangle from its bottom-left corner (GDI DrawRectangle). */
  static rectangle(x: number, y: number, width: number, height: number): Vector2[] {
    return [new Vector2(x, y), new Vector2(x + width, y), new Vector2(x + width, y + height), new Vector2(x, y + height)];
  }

  /** A rectangle with corners rounded to `radius` (at most half the shorter side). */
  static roundedRectangle(x: number, y: number, width: number, height: number, radius: number, segments: number = 8): Vector2[] {
    const r = Math.max(0, Math.min(radius, Math.min(width / 2, height / 2)));
    if (r === 0) return RefShapes.rectangle(x, y, width, height);
    const out: Vector2[] = [];
    const corners: number[][] = [
      [x + width - r, y + r, -90],
      [x + width - r, y + height - r, 0],
      [x + r, y + height - r, 90],
      [x + r, y + r, 180],
    ];
    for (let c = 0; c < 4; c++) {
      const k = corners[c]!;
      for (let i = 0; i <= segments; i++) out.push(__rshapes_at(k[0]!, k[1]!, r, r, k[2]! + (90 * i) / segments));
    }
    return out;
  }

  /** An ellipse around `center` with radii `rx` and `ry` (GDI DrawEllipse). */
  static ellipse(center: Vector2, rx: number, ry: number, segments: number = 64): Vector2[] {
    const out: Vector2[] = [];
    for (let i = 0; i < segments; i++) out.push(__rshapes_at(center.x, center.y, rx, ry, (360 * i) / segments));
    return out;
  }

  static circle(center: Vector2, radius: number, segments: number = 64): Vector2[] {
    return RefShapes.ellipse(center, radius, radius, segments);
  }

  /** Part of an ellipse from `startAngle`, `sweepAngle` degrees round (negative goes clockwise), GDI DrawArc. */
  static arc(center: Vector2, rx: number, ry: number, startAngle: number, sweepAngle: number, segments?: number): Vector2[] {
    const n = Math.max(1, segments !== undefined ? segments : Math.ceil((Math.abs(sweepAngle) / 360) * 64));
    const out: Vector2[] = [];
    for (let i = 0; i <= n; i++) out.push(__rshapes_at(center.x, center.y, rx, ry, startAngle + (sweepAngle * i) / n));
    return out;
  }

  /** A pie slice: the centre, then the arc (GDI DrawPie / FillPie). */
  static pie(center: Vector2, rx: number, ry: number, startAngle: number, sweepAngle: number, segments?: number): Vector2[] {
    const out: Vector2[] = [new Vector2(center.x, center.y)];
    const arc = RefShapes.arc(center, rx, ry, startAngle, sweepAngle, segments);
    for (let i = 0; i < arc.length; i++) out.push(arc[i]!);
    return out;
  }

  /** `sides` corners evenly round a circle, the first at `rotation` degrees (90: on top). */
  static regularPolygon(center: Vector2, radius: number, sides: number, rotation: number = 90): Vector2[] {
    const n = Math.max(3, Math.round(sides));
    const out: Vector2[] = [];
    for (let i = 0; i < n; i++) out.push(__rshapes_at(center.x, center.y, radius, radius, rotation + (360 * i) / n));
    return out;
  }

  /** The points as given (GDI DrawPolygon). */
  static polygon(...points: Vector2[]): Vector2[] {
    const out: Vector2[] = [];
    for (let i = 0; i < points.length; i++) out.push(new Vector2(points[i]!.x, points[i]!.y));
    return out;
  }

  static triangle(a: Vector2, b: Vector2, c: Vector2): Vector2[] {
    return [new Vector2(a.x, a.y), new Vector2(b.x, b.y), new Vector2(c.x, c.y)];
  }

  /** A cubic Bezier from `p0` to `p3`, pulled toward `p1` and `p2` (GDI DrawBezier). */
  static bezier(p0: Vector2, p1: Vector2, p2: Vector2, p3: Vector2, segments: number = 32): Vector2[] {
    const out: Vector2[] = [];
    for (let i = 0; i <= segments; i++) {
      const t = i / segments;
      const u = 1 - t;
      const a = u * u * u;
      const b = 3 * u * u * t;
      const c = 3 * u * t * t;
      const d = t * t * t;
      out.push(new Vector2(a * p0.x + b * p1.x + c * p2.x + d * p3.x, a * p0.y + b * p1.y + c * p2.y + d * p3.y));
    }
    return out;
  }

  /** A smooth curve through every point (GDI DrawCurve): `tension` 0 is straight lines, 0.5 a Catmull-Rom spline. */
  static curve(points: Vector2[], tension: number = 0.5, segments: number = 8): Vector2[] {
    return __rshapes_cardinal(points, tension, segments, false);
  }

  /** The same, closed back to the first point (GDI DrawClosedCurve / FillClosedCurve). */
  static closedCurve(points: Vector2[], tension: number = 0.5, segments: number = 8): Vector2[] {
    return __rshapes_cardinal(points, tension, segments, true);
  }
}


const same = (a: number, b: number): boolean => a === b || (a !== a && b !== b);

function samePoints(name: string, a: Vector2[], b: Vector2[]): string {
  if (a.length !== b.length) return `${name}: ${a.length} points, expected ${b.length}`;
  for (let i = 0; i < a.length; i++) {
    if (!same(a[i]!.x, b[i]!.x) || !same(a[i]!.y, b[i]!.y)) {
      return `${name}[${i}]: (${a[i]!.x}, ${a[i]!.y}) expected (${b[i]!.x}, ${b[i]!.y})`;
    }
  }
  return "";
}

const sample: Vector2[] = [new Vector2(0, 0), new Vector2(1, 2), new Vector2(3, 1), new Vector2(4, 4), new Vector2(-2, 3), new Vector2(0.5, -1.5)];

/** Runs every shape on edge cases; "" when all agree, else the first difference. */
export function shapesMatch(): string {
  const o = new Vector2(1.5, -2.5);
  const checks: string[] = [];
  for (const r of [[1, 2, 3, 4], [0, 0, 0, 0], [-1, -2, -3, -4], [0.1, 0.2, 1e-7, 1e9]]) {
    checks.push(samePoints("rectangle", Shapes.rectangle(r[0]!, r[1]!, r[2]!, r[3]!), RefShapes.rectangle(r[0]!, r[1]!, r[2]!, r[3]!)));
  }
  for (const r of [[0, 0, 4, 2, 5], [1, 2, 10, 6, 1.5], [0, 0, 4, 2, 0], [0, 0, 0, 0, 1], [0, 0, 4, 2, -3], [-3, 5, 7, 7, 3.5]]) {
    checks.push(samePoints("roundedRectangle", Shapes.roundedRectangle(r[0]!, r[1]!, r[2]!, r[3]!, r[4]!), RefShapes.roundedRectangle(r[0]!, r[1]!, r[2]!, r[3]!, r[4]!)));
    for (const seg of [1, 3, 8, 12]) {
      checks.push(samePoints("roundedRectangle", Shapes.roundedRectangle(r[0]!, r[1]!, r[2]!, r[3]!, r[4]!, seg), RefShapes.roundedRectangle(r[0]!, r[1]!, r[2]!, r[3]!, r[4]!, seg)));
    }
  }
  checks.push(samePoints("ellipse", Shapes.ellipse(o, 2, 1), RefShapes.ellipse(o, 2, 1)));
  checks.push(samePoints("circle", Shapes.circle(o, 3), RefShapes.circle(o, 3)));
  for (const seg of [0, 1, 2, 7, 64, 5.5, 200]) {
    checks.push(samePoints("ellipse", Shapes.ellipse(o, 2.5, 0.5, seg), RefShapes.ellipse(o, 2.5, 0.5, seg)));
    checks.push(samePoints("circle", Shapes.circle(o, 0, seg), RefShapes.circle(o, 0, seg)));
    checks.push(samePoints("circle", Shapes.circle(o, -2, seg), RefShapes.circle(o, -2, seg)));
  }
  for (const a of [[0, 90], [0, -90], [30, 0], [10, 720], [-45, 1e-9], [0, 360], [100, -250.5]]) {
    checks.push(samePoints("arc", Shapes.arc(o, 2, 3, a[0]!, a[1]!), RefShapes.arc(o, 2, 3, a[0]!, a[1]!)));
    checks.push(samePoints("pie", Shapes.pie(o, 2, 3, a[0]!, a[1]!), RefShapes.pie(o, 2, 3, a[0]!, a[1]!)));
    for (const seg of [1, 2, 13, 0, 4.5]) {
      checks.push(samePoints("arc", Shapes.arc(o, 2, 3, a[0]!, a[1]!, seg), RefShapes.arc(o, 2, 3, a[0]!, a[1]!, seg)));
      checks.push(samePoints("pie", Shapes.pie(o, 2, 3, a[0]!, a[1]!, seg), RefShapes.pie(o, 2, 3, a[0]!, a[1]!, seg)));
    }
  }
  for (const sides of [3, 4, 5, 6.4, 6.5, 2, 0, -1, 12]) {
    checks.push(samePoints("regularPolygon", Shapes.regularPolygon(o, 2, sides), RefShapes.regularPolygon(o, 2, sides)));
    for (const rot of [0, 45, -90, 90]) {
      checks.push(samePoints("regularPolygon", Shapes.regularPolygon(o, 2, sides, rot), RefShapes.regularPolygon(o, 2, sides, rot)));
    }
  }
  checks.push(samePoints("polygon0", Shapes.polygon(), RefShapes.polygon()));
  checks.push(samePoints("polygon1", Shapes.polygon(sample[1]!), RefShapes.polygon(sample[1]!)));
  checks.push(samePoints("polygon3", Shapes.polygon(sample[0]!, sample[1]!, { x: 9, y: 8 }), RefShapes.polygon(sample[0]!, sample[1]!, { x: 9, y: 8 })));
  checks.push(samePoints("polygon spread", Shapes.polygon(...sample), RefShapes.polygon(...sample)));
  checks.push(samePoints("triangle", Shapes.triangle(sample[0]!, sample[2]!, { x: -1, y: 7 }), RefShapes.triangle(sample[0]!, sample[2]!, { x: -1, y: 7 })));
  for (const seg of [32, 1, 2, 4, 5.5, 100]) {
    checks.push(samePoints("bezier", Shapes.bezier(sample[0]!, sample[1]!, sample[2]!, sample[3]!, seg), RefShapes.bezier(sample[0]!, sample[1]!, sample[2]!, sample[3]!, seg)));
  }
  checks.push(samePoints("bezier", Shapes.bezier(sample[0]!, sample[1]!, sample[2]!, sample[3]!), RefShapes.bezier(sample[0]!, sample[1]!, sample[2]!, sample[3]!)));
  for (let n = 0; n <= 6; n++) {
    const pts = sample.slice(0, n);
    checks.push(samePoints("curve", Shapes.curve(pts), RefShapes.curve(pts)));
    checks.push(samePoints("closedCurve", Shapes.closedCurve(pts), RefShapes.closedCurve(pts)));
    for (const tension of [0, 0.5, 1, -0.25, 2]) {
      for (const seg of [1, 4, 8, 3.5]) {
        checks.push(samePoints("curve", Shapes.curve(pts, tension, seg), RefShapes.curve(pts, tension, seg)));
        checks.push(samePoints("closedCurve", Shapes.closedCurve(pts, tension, seg), RefShapes.closedCurve(pts, tension, seg)));
      }
    }
  }
  for (let i = 0; i < checks.length; i++) {
    if (checks[i] !== "") return checks[i]!;
  }
  return "";
}

/** The shapes are `Vector2` objects the program can use like any other. */
export function shapePointsAreVectors(): string {
  const c = Shapes.circle(new Vector2(1, 1), 2, 4);
  const d = c[1]!.add(c[2]!).scale(0.5);
  const poly = Shapes.polygon({ x: 0, y: 0 }, { x: 2, y: 0 });
  return `${c.length}|${Math.round(c[0]!.x)}|${Math.round(d.y * 100)}|${poly.length}|${poly[1]!.x}`;
}

