// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// The Scene view's transform handles (was js/editor-ui/test/handles.test.ts).

import { beginDrag, dragTo, type HandleFrame, type HandleView, handleShapes, pickHandle } from "../src/handles.ts";
import { close, eq, report, same, test, yes } from "./check.ts";

// An orthographic view down -z from z = 10: 100 pixels a unit, the origin at (200, 200).
const view: HandleView = {
  toPanel: (p) => new Vector3(200 + p.x * 100, 200 - p.y * 100, 10 - p.z),
  ray: (x, y) => Ray.create(new Vector3((x - 200) / 100, (200 - y) / 100, 10), new Vector3(0, 0, -1)),
  unitsPerPixel: () => 0.01,
  forward: new Vector3(0, 0, -1),
};

const at = (rotation: Quaternion = Quaternion.identity): HandleFrame => ({ position: Vector3.zero, rotation });
const object = { position: Vector3.zero, rotation: Quaternion.identity, scale: Vector3.one };
const near = (a: Vector3, b: Vector3): boolean => a.sub(b).magnitude < 1e-6;

// M5-EDT-24
test("move handles: axes a constant 90 pixels long; the pointer picks the centre or the nearest axis", () => {
  const shapes = handleShapes("move", view, at());
  const x = shapes.find((s) => s.part === "x")!;
  eq(
    x.lines[0]!,
    [
      { x: 200, y: 200 },
      { x: 290, y: 200 },
    ],
    "x axis",
  );
  same(shapes[shapes.length - 1]!.part, "center", "centre drawn last");
  eq(pickHandle(shapes, 201, 199), "center", "centre");
  eq(pickHandle(shapes, 250, 205), "x", "x");
  eq(pickHandle(shapes, 203, 150), "y", "y");
  yes(pickHandle(shapes, 300, 300) === null, "nothing");
});

// M5-EDT-24
test("moving along an axis follows the pointer along it (local axes), snapping to whole units", () => {
  const drag = beginDrag("move", "x", view, object, at(), 250, 200);
  yes(near(dragTo(drag, view, 350, 260).position, new Vector3(1, 0, 0)), "along x");
  yes(near(dragTo(drag, view, 335, 200, true).position, new Vector3(1, 0, 0)), "snapped");
  // Turned a quarter about z, the object's x axis points up.
  const turned = at(Quaternion.angleAxis(90, Vector3.back));
  const up = beginDrag("move", "x", view, object, turned, 200, 150);
  yes(near(dragTo(up, view, 220, 100).position, new Vector3(0, 0.5, 0)), "local axis");
  // The centre moves in the view's plane.
  const free = beginDrag("move", "center", view, object, at(), 200, 200);
  yes(near(dragTo(free, view, 230, 180).position, new Vector3(0.3, 0.2, 0)), "view plane");
});

// M5-EDT-24
test("rotate rings: the ring facing the viewer is whole, others half; dragging round turns about the axis", () => {
  const shapes = handleShapes("rotate", view, at());
  const z = shapes.find((s) => s.part === "z")!;
  eq(z.back.length, 0, "z ring whole");
  const x = shapes.find((s) => s.part === "x")!;
  yes(x.back.length > 0, "x ring has a back");
  eq(pickHandle(shapes, 290, 201), "z", "pick the ring");
  const drag = beginDrag("rotate", "z", view, object, at(), 290, 200);
  const turned = dragTo(drag, view, 200, 110).rotation;
  yes(near(turned.rotate(Vector3.right), Vector3.up), "a quarter turn");
  // Snapped to 15 degrees.
  const a = Math.atan2(0.3, 0.9);
  const snapped = dragTo(drag, view, 200 + Math.cos(a) * 90, 200 - Math.sin(a) * 90, true).rotation;
  close(snapped.rotate(Vector3.right).y, Math.sin(Math.PI / 12), "snapped");
});

// M5-EDT-24
test("scale: an axis by how far along it the pointer is from the start; the centre uniformly", () => {
  const drag = beginDrag("scale", "x", view, { ...object, scale: new Vector3(1, 2, 3) }, at(), 245, 200);
  yes(near(dragTo(drag, view, 290, 200).scale, new Vector3(2, 2, 3)), "along x");
  const all = beginDrag("scale", "center", view, object, at(), 200, 200);
  yes(near(dragTo(all, view, 250, 150).scale, new Vector3(2, 2, 2)), "uniform");
  eq(dragTo(all, view, 0, 400).scale.x, 0.01, "never below 0.01");
});

export function run(): string {
  return report();
}
