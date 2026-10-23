// SPDX-License-Identifier: Apache-2.0

import { describe, expect, test } from "bun:test";

import { Vector3 } from "@iron/engine";

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

const near = (a: Vector3, b: Vector3, eps = 1e-6) => a.sub(b).magnitude < eps;
const at = (over: Partial<SceneCameraState> = {}): SceneCameraState => ({ ...defaultSceneCamera(), ...over });

// M5-EDT-22
describe("scene camera", () => {
  test("yaw and pitch: 0 looks forward, 90 yaw looks left, 90 pitch looks down; the rotation agrees", () => {
    expect(near(viewDirection({ yaw: 0, pitch: 0 }), Vector3.forward)).toBe(true);
    expect(near(viewDirection({ yaw: 90, pitch: 0 }), Vector3.left)).toBe(true);
    expect(near(viewDirection({ yaw: 0, pitch: 90 }), Vector3.down)).toBe(true);
    for (const s of [at(), at({ yaw: 120, pitch: -30 }), at({ pitch: 90 })]) {
      expect(near(sceneRotation(s).rotate(Vector3.forward), viewDirection(s))).toBe(true);
    }
    const s = at({ pivot: new Vector3(1, 2, 3), distance: 5 });
    expect(eyePosition(s).sub(s.pivot).magnitude).toBeCloseTo(5);
  });

  test("look turns about the eye, orbit about the pivot", () => {
    const s = at();
    const looked = look(s, 40, -20);
    expect(near(eyePosition(looked), eyePosition(s), 1e-4)).toBe(true);
    expect(looked.yaw).toBeCloseTo(s.yaw - 10);
    expect(looked.pitch).toBeCloseTo(s.pitch - 5);
    const orbited = orbit(s, 40, 1000);
    expect(near(orbited.pivot, s.pivot)).toBe(true);
    expect(orbited.pitch).toBe(90);
    expect(eyePosition(orbited).sub(s.pivot).magnitude).toBeCloseTo(s.distance);
  });

  test("pan keeps the pivot's depth and moves a view's height per view height dragged", () => {
    const s = at({ yaw: 0, pitch: 0, distance: 10 });
    const height = 2 * 10 * Math.tan(Math.PI / 6);
    const panned = pan(s, 0, 500, 500);
    expect(near(panned.pivot, new Vector3(0, height, 0), 1e-6)).toBe(true);
    // Dragging right moves the view left.
    expect(pan(s, 100, 0, 500).pivot.x).toBeLessThan(0);
  });

  test("zoom scales the distance, clamped; fly moves along the view, faster with Shift and distance", () => {
    const s = at({ yaw: 0, pitch: 0, distance: 10 });
    expect(zoom(s, 40).distance).toBeCloseTo(11);
    expect(zoom(s, -40).distance).toBeCloseTo(10 / 1.1);
    expect(zoom(s, -1e6).distance).toBe(0.05);
    const ahead = fly(s, new Vector3(0, 0, 1), 1);
    expect(near(ahead.pivot, new Vector3(0, 0, -7.5))).toBe(true);
    expect(fly(s, new Vector3(0, 0, 1), 1, true).pivot.z).toBeCloseTo(-30);
    expect(fly(s, new Vector3(1, 1, 0), 0.5).pivot.x).toBeCloseTo(3.75);
  });

  test("frame centres a sphere so it fits; lookAlong snaps to an axis about the pivot", () => {
    const framed = frame(at(), new Vector3(5, 1, 0), 2);
    expect(near(framed.pivot, new Vector3(5, 1, 0))).toBe(true);
    expect(framed.distance).toBeCloseTo(4.4);
    const side = lookAlong(at({ yaw: 10 }), Vector3.left);
    expect(near(viewDirection(side), Vector3.left)).toBe(true);
    const top = lookAlong(at({ yaw: 33 }), Vector3.down);
    expect([top.yaw, top.pitch]).toEqual([33, 90]);
  });

  test("the gizmo's axes, farthest first: from the front, +z points out and +x right", () => {
    const axes = gizmoAxes(at({ yaw: 0, pitch: 0 }));
    expect(axes.at(-1)!.name).toBe("z");
    expect(axes[0]!.name).toBe("-z");
    const x = axes.find((a) => a.name === "x")!;
    expect(x.x).toBeCloseTo(1);
    expect(x.y).toBeCloseTo(0);
    const y = axes.find((a) => a.name === "y")!;
    expect(y.y).toBeCloseTo(-1);
  });
});
