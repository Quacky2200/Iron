import { afterEach, beforeEach, describe, expect, test } from "bun:test";

import { GameObject, Shapes, Spline, SplineAnimate, Vector2, Vector3 } from "../src/index.ts";
import { _resetSceneForTests } from "../src/scene.ts";

const v = (x: number, y: number, z = 0) => new Vector3(x, y, z);
const close = (a: Vector3, b: Vector3, digits = 4) => {
  expect(a.x).toBeCloseTo(b.x, digits);
  expect(a.y).toBeCloseTo(b.y, digits);
  expect(a.z).toBeCloseTo(b.z, digits);
};

beforeEach(() => {
  _resetSceneForTests();
  (globalThis as { __iron_op?: unknown }).__iron_op = () => {};
});

afterEach(() => {
  delete (globalThis as { __iron_op?: unknown }).__iron_op;
});

// M5-SPL-01
describe("Spline", () => {
  test("passes through its knots; auto tangents match GDI's cardinal curve", () => {
    const pts = [v(0, 0), v(1, 2), v(3, 1), v(4, 3)];
    const s = new Spline(pts);
    expect(s.segmentCount).toBe(3);
    pts.forEach((p, i) => close(s.segmentPoint(Math.min(i, 2), i === 3 ? 1 : 0), p));
    const curve = Shapes.curve(pts.map((p) => new Vector2(p.x, p.y)), 0.5, 4);
    for (let i = 0; i < 3; i++) {
      for (let k = 0; k < 4; k++) {
        const c = curve[i * 4 + k]!;
        close(s.segmentPoint(i, k / 4), v(c.x, c.y));
      }
    }
  });

  test("linear knots make straight segments; bezier knots use their handles", () => {
    const line = new Spline([v(0, 0), v(2, 0), v(2, 2)], { mode: "linear" });
    close(line.segmentPoint(0, 0.5), v(1, 0));
    close(line.segmentPoint(1, 0.25), v(2, 0.5));
    const b = new Spline();
    b.addKnot(v(0, 0), "bezier", v(0, 0), v(0, 1));
    b.addKnot(v(2, 0), "bezier", v(0, 1), v(0, -1));
    // Halfway along a Bézier with both handles up by 1: three quarters of the way up.
    close(b.segmentPoint(0, 0.5), v(1, 0.75));
  });

  test("evaluate goes by distance, even when segments differ in length", () => {
    const s = new Spline([v(0, 0), v(1, 0), v(4, 0)], { mode: "linear" });
    expect(s.length).toBeCloseTo(4, 4);
    close(s.evaluate(0.25), v(1, 0));
    close(s.evaluate(0.5), v(2, 0));
    close(s.evaluate(1), v(4, 0));
    close(s.tangent(0.7), v(1, 0));
  });

  test("closed splines join back to the first knot", () => {
    const square = new Spline([v(0, 0), v(1, 0), v(1, 1), v(0, 1)], { closed: true, mode: "linear" });
    expect(square.segmentCount).toBe(4);
    expect(square.length).toBeCloseTo(4, 4);
    close(square.evaluate(0.875), v(0, 0.5));
    expect(square.sample(2).length).toBe(8);
    const open = new Spline([v(0, 0), v(1, 0), v(1, 1)]);
    expect(open.sample(2).length).toBe(5);
  });

  test("finds the nearest point; edits move, add, remove and re-mode knots", () => {
    const s = new Spline([v(0, 0), v(4, 0)], { mode: "linear" });
    const near = s.nearest(v(1, 3));
    expect(near.t).toBeCloseTo(0.25, 3);
    expect(near.distance).toBeCloseTo(3, 3);
    const before = s.version;
    s.insertKnot(1, v(2, 2));
    expect(s.count).toBe(3);
    expect(s.version).toBeGreaterThan(before);
    s.setPosition(1, v(2, 1));
    close(s.knots[1]!.position, v(2, 1));
    // Turning an auto knot to bezier keeps the shape it had.
    const auto = new Spline([v(0, 0), v(1, 1), v(2, 0)]);
    const mid = auto.segmentPoint(0, 0.5);
    auto.setMode(1, "bezier");
    close(auto.segmentPoint(0, 0.5), mid);
    auto.setTangents(1, v(-1, 0), v(1, 0));
    expect(auto.knots[1]!.mode).toBe("bezier");
    s.removeKnot(0);
    expect(s.count).toBe(2);
  });
});

// M5-SPL-01
describe("SplineAnimate", () => {
  test("moves its object along the path, looping, ping-ponging or once, facing along it", () => {
    const path = new Spline([v(0, 0), v(4, 0), v(4, 4)], { mode: "linear" });
    const go = new GameObject("car");
    const a = go.addComponent(SplineAnimate, { duration: 4 });
    a.spline = path;
    a.onUpdate(1);
    close(go.transform.position, v(2, 0));
    // Facing +x along the first leg.
    close(go.transform.rotation.rotate(Vector3.right), v(1, 0), 3);
    // 3.5 s of 4: seven units along the eight.
    a.onUpdate(2.5);
    close(go.transform.position, v(4, 3), 3);
    close(go.transform.rotation.rotate(Vector3.right), v(0, 1), 3);
    // Round again: 4.5 s is one unit along.
    a.onUpdate(1);
    close(go.transform.position, v(1, 0), 3);
    a.loopMode = "pingPong";
    a.elapsed = 5;
    a.place();
    close(go.transform.position, v(4, 2), 3);
    a.loopMode = "once";
    a.elapsed = 9;
    a.place();
    close(go.transform.position, v(4, 4), 3);
    // In a container's space.
    const container = new GameObject("track", { position: v(10, 0, 0) });
    a.container = container;
    a.elapsed = 0;
    a.loopMode = "loop";
    a.place();
    close(go.transform.position, v(10, 0, 0), 3);
  });
});
