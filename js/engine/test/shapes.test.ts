import { describe, expect, test } from "bun:test";

import { Shapes, Vector2 } from "../src/index.ts";

const close = (a: Vector2, x: number, y: number) => {
  expect(a.x).toBeCloseTo(x, 5);
  expect(a.y).toBeCloseTo(y, 5);
};

// M5-RND-25
describe("Shapes", () => {
  test("rectangles: four corners from the bottom-left, rounded ones inside their box", () => {
    const r = Shapes.rectangle(1, 2, 3, 4);
    expect(r.map((p) => [p.x, p.y])).toEqual([[1, 2], [4, 2], [4, 6], [1, 6]]);
    const rr = Shapes.roundedRectangle(0, 0, 4, 2, 5, 4);
    // The radius is clamped to half the shorter side; four corners of five points.
    expect(rr.length).toBe(20);
    for (const p of rr) {
      expect(p.x).toBeGreaterThanOrEqual(-1e-6);
      expect(p.x).toBeLessThanOrEqual(4 + 1e-6);
      expect(p.y).toBeGreaterThanOrEqual(-1e-6);
      expect(p.y).toBeLessThanOrEqual(2 + 1e-6);
    }
  });

  test("circles and ellipses: points on the curve, 64 by default", () => {
    const c = Shapes.circle(new Vector2(1, 1), 2);
    expect(c.length).toBe(64);
    for (const p of c) expect(p.distance(new Vector2(1, 1))).toBeCloseTo(2, 5);
    const e = Shapes.ellipse(Vector2.zero, 3, 1, 4);
    close(e[0]!, 3, 0);
    close(e[1]!, 0, 1);
  });

  test("arcs run from the start angle through the sweep; pies start at the centre", () => {
    const a = Shapes.arc(Vector2.zero, 1, 1, 0, 90);
    close(a[0]!, 1, 0);
    close(a.at(-1)!, 0, 1);
    const back = Shapes.arc(Vector2.zero, 1, 1, 0, -90, 2);
    expect(back.length).toBe(3);
    close(back.at(-1)!, 0, -1);
    const pie = Shapes.pie(new Vector2(5, 5), 1, 1, 0, 180);
    close(pie[0]!, 5, 5);
    close(pie.at(-1)!, 4, 5);
  });

  test("regular polygons start on top; triangles keep their points", () => {
    const t = Shapes.regularPolygon(Vector2.zero, 2, 3);
    expect(t.length).toBe(3);
    close(t[0]!, 0, 2);
    expect(Shapes.triangle({ x: 0, y: 0 }, { x: 1, y: 0 }, { x: 0, y: 1 }).map((p) => [p.x, p.y])).toEqual([[0, 0], [1, 0], [0, 1]]);
  });

  test("Béziers run from the first point to the last, halfway at the control average", () => {
    const b = Shapes.bezier({ x: 0, y: 0 }, { x: 0, y: 2 }, { x: 2, y: 2 }, { x: 2, y: 0 }, 4);
    expect(b.length).toBe(5);
    close(b[0]!, 0, 0);
    close(b[2]!, 1, 1.5);
    close(b[4]!, 2, 0);
  });

  test("curves pass through every point; tension 0 is straight; closed curves come back round", () => {
    const pts = [new Vector2(0, 0), new Vector2(1, 2), new Vector2(3, 1), new Vector2(4, 3)];
    const c = Shapes.curve(pts, 0.5, 4);
    expect(c.length).toBe(3 * 4 + 1);
    pts.forEach((p, i) => close(c[i * 4]!, p.x, p.y));
    const straight = Shapes.curve(pts, 0, 2);
    close(straight[1]!, 0.5, 1);
    const closed = Shapes.closedCurve(pts, 0.5, 4);
    expect(closed.length).toBe(4 * 4);
    pts.forEach((p, i) => close(closed[i * 4]!, p.x, p.y));
  });
});
