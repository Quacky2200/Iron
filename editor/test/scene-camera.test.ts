// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// The Scene view's camera controls and axis gizmo (was js/editor-ui/test/scene-camera.test.ts).

import {
  defaultSceneCamera,
  eyePosition,
  fly,
  frame,
  gizmoAxes,
  look,
  lookAlong,
  orbit,
  pan,
  type SceneCameraState,
  sceneRotation,
  viewDirection,
  zoom,
} from "../src/scene-camera.ts";
import { close, eq, report, test, yes } from "./check.ts";

const near = (a: Vector3, b: Vector3, eps = 1e-6): boolean => a.sub(b).magnitude < eps;

/** The default camera with a yaw, a pitch and a distance. */
function at(yaw: number, pitch: number, distance: number = defaultSceneCamera().distance): SceneCameraState {
  return { ...defaultSceneCamera(), yaw, pitch, distance };
}

// M5-EDT-22
test("yaw and pitch: 0 looks forward, 90 yaw looks left, 90 pitch looks down; the rotation agrees", () => {
  yes(near(viewDirection(at(0, 0)), Vector3.forward), "forward");
  yes(near(viewDirection(at(90, 0)), Vector3.left), "left");
  yes(near(viewDirection(at(0, 90)), Vector3.down), "down");
  const d = defaultSceneCamera();
  for (const s of [d, at(120, -30), at(d.yaw, 90)]) {
    yes(near(sceneRotation(s).rotate(Vector3.forward), viewDirection(s)), `rotation at ${s.yaw}/${s.pitch}`);
  }
  const s: SceneCameraState = { ...defaultSceneCamera(), pivot: new Vector3(1, 2, 3), distance: 5 };
  close(eyePosition(s).sub(s.pivot).magnitude, 5, "eye distance");
});

// M5-EDT-22
test("look turns about the eye, orbit about the pivot", () => {
  const s = defaultSceneCamera();
  const looked = look(s, 40, -20);
  yes(near(eyePosition(looked), eyePosition(s), 1e-4), "eye kept");
  close(looked.yaw, s.yaw - 10, "yaw");
  close(looked.pitch, s.pitch - 5, "pitch");
  const orbited = orbit(s, 40, 1000);
  yes(near(orbited.pivot, s.pivot), "pivot kept");
  eq(orbited.pitch, 90, "pitch clamped");
  close(eyePosition(orbited).sub(s.pivot).magnitude, s.distance, "distance kept");
});

// M5-EDT-22
test("pan keeps the pivot's depth and moves a view's height per view height dragged", () => {
  const s = at(0, 0, 10);
  const height = 2 * 10 * Math.tan(Math.PI / 6);
  const panned = pan(s, 0, 500, 500);
  yes(near(panned.pivot, new Vector3(0, height, 0), 1e-6), "a view height");
  // Dragging right moves the view left.
  yes(pan(s, 100, 0, 500).pivot.x < 0, "right drags left");
});

// M5-EDT-22
test("zoom scales the distance, clamped; fly moves along the view, faster with Shift and distance", () => {
  const s = at(0, 0, 10);
  close(zoom(s, 40).distance, 11, "out");
  close(zoom(s, -40).distance, 10 / 1.1, "in");
  eq(zoom(s, -1e6).distance, 0.05, "clamped");
  const ahead = fly(s, new Vector3(0, 0, 1), 1);
  yes(near(ahead.pivot, new Vector3(0, 0, -7.5)), "forward");
  close(fly(s, new Vector3(0, 0, 1), 1, true).pivot.z, -30, "shift");
  close(fly(s, new Vector3(1, 1, 0), 0.5).pivot.x, 3.75, "right");
});

// M5-EDT-22
test("frame centres a sphere so it fits; lookAlong snaps to an axis about the pivot", () => {
  const framed = frame(defaultSceneCamera(), new Vector3(5, 1, 0), 2);
  yes(near(framed.pivot, new Vector3(5, 1, 0)), "centred");
  close(framed.distance, 4.4, "fits");
  const side = lookAlong(at(10, defaultSceneCamera().pitch), Vector3.left);
  yes(near(viewDirection(side), Vector3.left), "along left");
  const top = lookAlong(at(33, defaultSceneCamera().pitch), Vector3.down);
  eq([top.yaw, top.pitch], [33, 90], "from the top keeps the yaw");
});

// M5-EDT-22
test("the gizmo's axes, farthest first: from the front, +z points out and +x right", () => {
  const axes = gizmoAxes(at(0, 0));
  eq(axes[axes.length - 1]!.name, "z", "nearest");
  eq(axes[0]!.name, "-z", "farthest");
  const x = axes.find((a) => a.name === "x")!;
  close(x.x, 1, "x right");
  close(x.y, 0, "x level");
  const y = axes.find((a) => a.name === "y")!;
  close(y.y, -1, "y up");
});

export function run(): string {
  return report();
}
