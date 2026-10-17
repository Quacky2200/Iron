import { afterEach, beforeEach, describe, expect, test } from "bun:test";

import { GameObject, LineRenderer, MeshRenderer, Vector3 } from "../src/index.ts";
import { tubeMesh } from "../src/lines.ts";
import { _resetSceneForTests } from "../src/scene.ts";

let ops: unknown[][] = [];

beforeEach(() => {
  _resetSceneForTests();
  ops = [];
  (globalThis as { __iron_op?: unknown }).__iron_op = (...args: unknown[]) => ops.push(args);
});

afterEach(() => {
  delete (globalThis as { __iron_op?: unknown }).__iron_op;
});

// M5-LIN-01
describe("LineRenderer", () => {
  test("a tube has rings of vertices at its radius around a smoothed path", () => {
    const points = [new Vector3(0, 0, 0), new Vector3(0, -1, 0), new Vector3(1, -2, 0)];
    const t = tubeMesh(points, 0.1, 6, 3);
    // 2 spans × (3 + 1) points + the last, 7 vertices per ring (the seam is doubled).
    const rings = 2 * 4 + 1;
    expect(t.vertices.length).toBe(rings * 7);
    expect(t.triangles.length).toBe((rings - 1) * 6 * 6);
    // The first ring circles the first point at the radius, across the (curving) path.
    for (let s = 0; s < 7; s++) {
      const v = t.vertices[s]!;
      expect(v.distance(points[0]!)).toBeCloseTo(0.1, 5);
      expect(Math.abs(v.y)).toBeLessThan(0.02);
    }
    // It passes through every given point.
    const centre = (ring: number) =>
      t.vertices.slice(ring * 7, ring * 7 + 6).reduce((a, b) => a.add(b), Vector3.zero).scale(1 / 6);
    expect(centre(4).distance(points[1]!)).toBeLessThan(1e-5);
    expect(centre(8).distance(points[2]!)).toBeLessThan(1e-5);
  });

  test("redraws only when the points change", () => {
    const go = new GameObject("cable");
    const line = go.addComponent(LineRenderer, { positions: [Vector3.zero, Vector3.up], width: 0.2 });
    expect(go.getComponent(MeshRenderer)).toBeDefined();
    const meshOps = () => ops.filter((o) => o[0] === "render.mesh").length;
    const before = meshOps();
    line.onPreRender();
    expect(meshOps()).toBe(before);
    line.positions = [Vector3.zero, Vector3.right];
    line.onPreRender();
    expect(meshOps()).toBe(before + 1);
  });
});
