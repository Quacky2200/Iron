// SPDX-License-Identifier: Apache-2.0

import { afterEach, beforeEach, describe, expect, test } from "bun:test";

import { Color, GameObject, LineRenderer, LineRenderer2D, MeshRenderer, Vector2, Vector3 } from "../src/index.ts";
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

// M5-RND-24
describe("LineRenderer2D", () => {
  test("sends its points flat with width, colour and loop, once per frame, and goes on destroy", () => {
    const go = new GameObject("curve");
    const line = go.addComponent(LineRenderer2D, {
      points: [new Vector2(0, 0), new Vector2(1, 2), new Vector2(3, 4)],
      width: 0.25,
      color: new Color(1, 0.5, 0, 1),
      loop: true,
    });
    const strokes = () => ops.filter((o) => o[0] === "render.stroke");
    expect(strokes().at(-1)).toEqual(["render.stroke", line.id, go.id, 0.25, [1, 0.5, 0, 1], true, [0, 0, 1, 2, 3, 4], null, "evenodd", 0]);
    // Nothing changed: nothing sent before drawing.
    const sent = strokes().length;
    line.onPreRender();
    expect(strokes().length).toBe(sent);
    // Several changes in a frame go as one op.
    line.points = [new Vector2(5, 6), new Vector2(7, 8)];
    line.width = 0.5;
    line.loop = false;
    expect(strokes().length).toBe(sent);
    line.onPreRender();
    expect(strokes().length).toBe(sent + 1);
    expect(strokes().at(-1)).toEqual(["render.stroke", line.id, go.id, 0.5, [1, 0.5, 0, 1], false, [5, 6, 7, 8], null, "evenodd", 0]);
    // The engine runs onDestroy when the object goes.
    line.onDestroy();
    expect(ops.at(-1)).toEqual(["render.remove", line.id]);
  });
});

// M5-RND-25
describe("LineRenderer2D fills", () => {
  test("a fill colour and rule go with the points", () => {
    const go = new GameObject("star");
    const line = go.addComponent(LineRenderer2D, {
      points: [new Vector2(0, 0), new Vector2(1, 0), new Vector2(0, 1)],
      width: 0,
      fillColor: new Color(0, 1, 0, 1),
      fillRule: "nonzero",
    });
    const op = ops.filter((o) => o[0] === "render.stroke").at(-1)!;
    expect(op.slice(3)).toEqual([0, [1, 1, 1, 1], false, [0, 0, 1, 0, 0, 1], [0, 1, 0, 1], "nonzero", 0]);
    line.fillColor = null;
    line.onPreRender();
    expect(ops.filter((o) => o[0] === "render.stroke").at(-1)!.at(-3)).toBe(null);
    line.sortingOrder = 3.7;
    line.onPreRender();
    expect(ops.filter((o) => o[0] === "render.stroke").at(-1)!.at(-1)).toBe(3);
  });
});
