import { afterEach, beforeEach, describe, expect, test } from "bun:test";

import { Camera, Color, GameObject, Light, RenderSettings, SceneManager } from "../src/index.ts";
import { _resetCamerasForTests } from "../src/rendering.ts";
import { _resetSceneForTests } from "../src/scene.ts";
import type { SceneData } from "../src/scenes.ts";

let ops: unknown[][] = [];

beforeEach(() => {
  _resetSceneForTests();
  _resetCamerasForTests();
  ops = [];
  (globalThis as { __iron_op?: unknown }).__iron_op = (...args: unknown[]) => ops.push(args);
});

afterEach(() => {
  // RenderSettings is shared by every test file: put Unity's defaults back.
  RenderSettings.fog = false;
  RenderSettings.fogMode = "exponentialSquared";
  RenderSettings.fogColor = new Color(0.5, 0.5, 0.5);
  RenderSettings.fogDensity = 0.01;
  RenderSettings.fogStartDistance = 0;
  RenderSettings.fogEndDistance = 300;
  RenderSettings.heightFog = null;
  RenderSettings.volumetricFog = null;
  delete (globalThis as { __iron_op?: unknown }).__iron_op;
});

const fogOps = () => ops.filter((o) => o[0] === "render.fog");

// M7-FOG-03
describe("fog", () => {
  test("RenderSettings sends the fog while it's on, and clears it when turned off", () => {
    expect(RenderSettings.fog).toBe(false);
    // Off: changing settings sends nothing.
    RenderSettings.fogColor = new Color(0.2, 0.3, 0.4);
    RenderSettings.fogMode = "linear";
    expect(fogOps()).toEqual([]);
    RenderSettings.fog = true;
    expect(fogOps().at(-1)).toEqual(["render.fog", 0, "linear", [0.2, 0.3, 0.4], 0.01, 0, 300, null, null]);
    RenderSettings.fogStartDistance = 10;
    RenderSettings.fogEndDistance = 50;
    expect(fogOps().at(-1)).toEqual(["render.fog", 0, "linear", [0.2, 0.3, 0.4], 0.01, 10, 50, null, null]);
    RenderSettings.fogMode = "exponential";
    RenderSettings.fogDensity = -1;
    expect(RenderSettings.fogDensity).toBe(0);
    expect(fogOps().at(-1)!.slice(2, 5)).toEqual(["exponential", [0.2, 0.3, 0.4], 0]);
    // Height fog rides along (M7-FOG-04).
    RenderSettings.heightFog = { density: 0.1, baseHeight: -2, falloff: 0.5 };
    expect(fogOps().at(-1)![7]).toEqual([0.1, -2, 0.5]);
    // Light shafts too, with defaults for what's left out (M7-FOG-05).
    RenderSettings.volumetricFog = { steps: 24 };
    expect(fogOps().at(-1)![8]).toEqual([1, 0.4, 24, 50]);
    RenderSettings.fog = false;
    expect(fogOps().at(-1)).toEqual(["render.fog", 0, null]);
  });

  test("cameras show the fog unless they opt out", () => {
    const cam = new GameObject("cam").addComponent(Camera);
    cam.onStart();
    const last = () => ops.filter((o) => o[0] === "render.camera").at(-1)!;
    expect(cam.fog).toBe(true);
    expect(last().at(-1)).toBe(true);
    cam.fog = false;
    expect(last().at(-1)).toBe(false);
  });

  // M7-FOG-05
  test("lights say how strongly they light the fog", () => {
    const l = new GameObject("lamp").addComponent(Light, { type: "spot" });
    expect(ops.filter((o) => o[0] === "render.light").at(-1)!.at(-1)).toBe(1);
    l.volumetricIntensity = -2;
    expect(l.volumetricIntensity).toBe(0);
    expect(ops.filter((o) => o[0] === "render.light").at(-1)!.at(-1)).toBe(0);
  });

  test("scenes save the fog and bring it back", () => {
    RenderSettings.fogMode = "linear";
    RenderSettings.fogColor = new Color(0.1, 0.2, 0.3);
    RenderSettings.fogStartDistance = 5;
    RenderSettings.fogEndDistance = 25;
    RenderSettings.heightFog = { density: 0.2, baseHeight: 1, falloff: 0.3 };
    RenderSettings.fog = true;
    const json = JSON.parse(JSON.stringify(SceneManager.getActiveScene().serialize())) as SceneData;
    RenderSettings.fog = false;
    RenderSettings.fogMode = "exponential";
    RenderSettings.fogEndDistance = 300;
    RenderSettings.heightFog = null;
    ops = [];
    SceneManager.loadScene(json);
    expect(RenderSettings.fog).toBe(true);
    expect(RenderSettings.fogColor.g).toBeCloseTo(0.2);
    expect(fogOps().at(-1)).toEqual(["render.fog", 0, "linear", [0.1, 0.2, 0.3], 0.01, 5, 25, [0.2, 1, 0.3], null]);
  });
});
