// SPDX-License-Identifier: Apache-2.0

import { describe, expect, test } from "bun:test";

import { Quaternion, Ray, Vector3 } from "@platypus/engine";

import { beginDrag, dragTo, type HandleFrame, type HandleView, handleShapes, pickHandle } from "../src/handles.ts";

// An orthographic view down -z from z = 10: 100 pixels a unit, the origin at (200, 200).
const view: HandleView = {
  toPanel: (p) => new Vector3(200 + p.x * 100, 200 - p.y * 100, 10 - p.z),
  ray: (x, y) => new Ray(new Vector3((x - 200) / 100, (200 - y) / 100, 10), new Vector3(0, 0, -1)),
  unitsPerPixel: () => 0.01,
  forward: new Vector3(0, 0, -1),
};

const at = (rotation = Quaternion.identity): HandleFrame => ({ position: Vector3.zero, rotation });
const object = { position: Vector3.zero, rotation: Quaternion.identity, scale: Vector3.one };
const near = (a: Vector3, b: Vector3) => a.sub(b).magnitude < 1e-6;

// M5-EDT-24
describe("transform handles", () => {
  test("move handles: axes a constant 90 pixels long; the pointer picks the centre or the nearest axis", () => {
    const shapes = handleShapes("move", view, at());
    const x = shapes.find((s) => s.part === "x")!;
    expect(x.lines[0]).toEqual([
      { x: 200, y: 200 },
      { x: 290, y: 200 },
    ]);
    expect(shapes.at(-1)!.part).toBe("center");
    expect(pickHandle(shapes, 201, 199)).toBe("center");
    expect(pickHandle(shapes, 250, 205)).toBe("x");
    expect(pickHandle(shapes, 203, 150)).toBe("y");
    expect(pickHandle(shapes, 300, 300)).toBeNull();
  });

  test("moving along an axis follows the pointer along it (local axes), snapping to whole units", () => {
    const drag = beginDrag("move", "x", view, object, at(), 250, 200);
    expect(near(dragTo(drag, view, 350, 260).position, new Vector3(1, 0, 0))).toBe(true);
    expect(near(dragTo(drag, view, 335, 200, true).position, new Vector3(1, 0, 0))).toBe(true);
    // Turned a quarter about z, the object's x axis points up.
    const turned = at(Quaternion.angleAxis(90, Vector3.back));
    const up = beginDrag("move", "x", view, object, turned, 200, 150);
    expect(near(dragTo(up, view, 220, 100).position, new Vector3(0, 0.5, 0))).toBe(true);
    // The centre moves in the view's plane.
    const free = beginDrag("move", "center", view, object, at(), 200, 200);
    expect(near(dragTo(free, view, 230, 180).position, new Vector3(0.3, 0.2, 0))).toBe(true);
  });

  test("rotate rings: the ring facing the viewer is whole, others half; dragging round turns about the axis", () => {
    const shapes = handleShapes("rotate", view, at());
    const z = shapes.find((s) => s.part === "z")!;
    expect(z.back).toEqual([]);
    const x = shapes.find((s) => s.part === "x")!;
    expect(x.back.length).toBeGreaterThan(0);
    expect(pickHandle(shapes, 290, 201)).toBe("z");
    const drag = beginDrag("rotate", "z", view, object, at(), 290, 200);
    const turned = dragTo(drag, view, 200, 110).rotation;
    expect(near(turned.rotate(Vector3.right), Vector3.up)).toBe(true);
    // Snapped to 15 degrees.
    const a = Math.atan2(0.3, 0.9);
    const snapped = dragTo(drag, view, 200 + Math.cos(a) * 90, 200 - Math.sin(a) * 90, true).rotation;
    expect(snapped.rotate(Vector3.right).y).toBeCloseTo(Math.sin(Math.PI / 12));
  });

  test("scale: an axis by how far along it the pointer is from the start; the centre uniformly", () => {
    const drag = beginDrag("scale", "x", view, { ...object, scale: new Vector3(1, 2, 3) }, at(), 245, 200);
    expect(near(dragTo(drag, view, 290, 200).scale, new Vector3(2, 2, 3))).toBe(true);
    const all = beginDrag("scale", "center", view, object, at(), 200, 200);
    expect(near(dragTo(all, view, 250, 150).scale, new Vector3(2, 2, 2))).toBe(true);
    expect(dragTo(all, view, 0, 400).scale.x).toBe(0.01);
  });
});
