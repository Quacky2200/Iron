import { afterEach, beforeEach, describe, expect, test } from "bun:test";

import { Camera, GameObject, LayerMask } from "../src/index.ts";
import { _resetCamerasForTests } from "../src/rendering.ts";
import { _resetSceneForTests } from "../src/scene.ts";

let ops: unknown[][] = [];

beforeEach(() => {
  _resetSceneForTests();
  _resetCamerasForTests();
  ops = [];
  (globalThis as { __iron_op?: unknown }).__iron_op = (...args: unknown[]) => ops.push(args);
});

afterEach(() => {
  delete (globalThis as { __iron_op?: unknown }).__iron_op;
  delete globalThis.__iron_info;
});

// M5-LAY-03
describe("layers", () => {
  test("LayerMask names Unity's built-ins and the manifest's layers", () => {
    expect(LayerMask.nameToLayer("UI")).toBe(5);
    expect(LayerMask.nameToLayer("Player")).toBe(-1);
    globalThis.__iron_info = {
      app: { name: "g", productName: "g", version: "1", companyName: "", identifier: "", engineVersion: "0", layers: { 8: "Player", 40: "Bad", 9: 3 } },
      system: {} as never,
    };
    expect(LayerMask.nameToLayer("Player")).toBe(8);
    expect(LayerMask.layerToName(8)).toBe("Player");
    expect(LayerMask.layerToName(9)).toBe("");
    expect(LayerMask.nameToLayer("Bad")).toBe(-1);
    expect(LayerMask.getMask("Default", "UI", "Nope")).toBe(0b100001);
    expect(LayerMask.contains(LayerMask.everything, 31)).toBe(true);
    expect(LayerMask.contains(LayerMask.getMask("UI"), 0)).toBe(false);
  });

  test("objects send their layer and refuse ones out of range", () => {
    const go = new GameObject("hud");
    expect(go.layer).toBe(0);
    go.layer = 5;
    expect(go.layer).toBe(5);
    expect(ops.at(-1)).toEqual(["scene.layer", go.id, 5]);
    expect(() => {
      go.layer = 32;
    }).toThrow(RangeError);
    expect(go.layer).toBe(5);
  });

  test("field of view: clamped, and converted between vertical and horizontal", () => {
    const cam = new GameObject("cam").addComponent(Camera);
    cam.onStart();
    expect(Camera.verticalToHorizontalFieldOfView(90, 1)).toBeCloseTo(90, 6);
    // tan(30) * 16 / 9 -> about 91.5 degrees across.
    expect(Camera.verticalToHorizontalFieldOfView(60, 16 / 9)).toBeCloseTo(91.4928, 3);
    expect(Camera.horizontalToVerticalFieldOfView(91.4928, 16 / 9)).toBeCloseTo(60, 3);
    cam.horizontalFieldOfView = Camera.verticalToHorizontalFieldOfView(40, cam.aspect);
    expect(cam.fieldOfView).toBeCloseTo(40, 6);
    expect(ops.filter((o) => o[0] === "render.camera").at(-1)![3]).toBeCloseTo(40, 6);
    cam.fieldOfView = 500;
    expect(cam.fieldOfView).toBe(179);
  });

  test("cameras send their culling mask and cull distances", () => {
    const cam = new GameObject("cam").addComponent(Camera);
    cam.onStart();
    const last = () => ops.filter((o) => o[0] === "render.camera").at(-1)!;
    expect(cam.cullingMask).toBe(0xffffffff);
    expect(last().slice(12)).toEqual([0xffffffff, null, false]);
    // Signed masks (Unity's ~) come out unsigned.
    cam.cullingMask &= ~LayerMask.getMask("UI");
    expect(cam.cullingMask).toBe(0xffffffdf);
    const far = cam.layerCullDistances;
    far[8] = 30;
    // A copy: nothing sent until it's set back.
    expect(cam.layerCullDistances[8]).toBe(0);
    cam.layerCullDistances = far;
    cam.layerCullSpherical = true;
    const [mask, distances, spherical] = last().slice(12) as [number, number[], boolean];
    expect(mask).toBe(0xffffffdf);
    expect(distances).toHaveLength(32);
    expect(distances[8]).toBe(30);
    expect(spherical).toBe(true);
  });
});
