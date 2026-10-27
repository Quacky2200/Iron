// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

import { afterEach, beforeEach, describe, expect, test } from "bun:test";

import { Camera, Color, FogVolume, GameObject, Light, RenderSettings, RenderStats, SceneManager, Vector3 } from "../src/index.ts";
import { _resetCamerasForTests } from "../src/rendering.ts";
import { _resetSceneForTests } from "../src/scene.ts";
import type { SceneData } from "../src/scenes.ts";

let ops: unknown[][] = [];

beforeEach(() => {
  _resetSceneForTests();
  _resetCamerasForTests();
  ops = [];
  (globalThis as { __platypus_op?: unknown }).__platypus_op = (...args: unknown[]) => ops.push(args);
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
  RenderSettings.fogCulling = true;
  delete (globalThis as { __platypus_op?: unknown }).__platypus_op;
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
    expect(fogOps().at(-1)).toEqual(["render.fog", 0, "linear", [0.2, 0.3, 0.4], 0.01, 0, 300, null, null, true]);
    RenderSettings.fogStartDistance = 10;
    RenderSettings.fogEndDistance = 50;
    expect(fogOps().at(-1)).toEqual(["render.fog", 0, "linear", [0.2, 0.3, 0.4], 0.01, 10, 50, null, null, true]);
    RenderSettings.fogMode = "exponential";
    RenderSettings.fogDensity = -1;
    expect(RenderSettings.fogDensity).toBe(0);
    expect(fogOps().at(-1)!.slice(2, 5)).toEqual(["exponential", [0.2, 0.3, 0.4], 0]);
    // Height fog rides along (M7-FOG-04).
    RenderSettings.heightFog = { density: 0.1, baseHeight: -2, falloff: 0.5 };
    expect(fogOps().at(-1)![7]).toEqual([0.1, -2, 0.5]);
    // Light shafts too, with defaults for what's left out (M7-FOG-05).
    RenderSettings.volumetricFog = { steps: 8 };
    expect(fogOps().at(-1)![8]).toEqual([1, 0.4, 8, 50, 1, 1, 1, 0, 32, 2, 3, 0.9, 1, 2]);
    // M7-FOG-11
    RenderSettings.volumetricFog = { resolution: "ultra", denoising: "reprojection", historyWeight: 0.95, samples: 4, motionSamples: 6 };
    expect(fogOps().at(-1)![8]!.slice(-5)).toEqual([3, 1, 0.95, 4, 6]);
    // Culling past the fog (M7-FOG-07).
    RenderSettings.fogCulling = false;
    expect(fogOps().at(-1)!.at(-1)).toBe(false);
    RenderSettings.fog = false;
    expect(fogOps().at(-1)).toEqual(["render.fog", 0, null]);
  });

  // M7-FOG-07
  test("RenderStats follow what the engine reports", () => {
    globalThis.__platypus_render_stats(10, 4, 2, 1, 3, 5, 0, 2);
    expect(RenderStats.meshes).toBe(10);
    expect(RenderStats.culledMeshes).toBe(4);
    expect(RenderStats.fogCulled).toBe(3);
    expect(RenderStats.shadowMaps).toBe(2);
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
    expect(ops.filter((o) => o[0] === "render.light").at(-1)!.at(-3)).toBe(1);
    l.volumetricIntensity = -2;
    expect(l.volumetricIntensity).toBe(0);
    expect(ops.filter((o) => o[0] === "render.light").at(-1)!.at(-3)).toBe(0);
  });

  // M7-CLOUD-03
  test("the cloud layer goes out with its defaults filled in", () => {
    const last = () => ops.filter((o) => o[0] === "render.clouds").at(-1)!;
    RenderSettings.cloudLayer = { altitude: 60, size: 40, tint: new Color(0.5, 0.5, 0.55), windSpeed: 2, windDirection: 90 };
    expect(last()).toEqual(["render.clouds", 0, [60, 40, 0.5, 1, 0.3, 0.5, 0.5, 0.55, 2, 90, 20000]]);
    RenderSettings.cloudLayer = null;
    expect(last()).toEqual(["render.clouds", 0, null]);
  });

  // M7-SPOT-02
  test("spot lights can be pyramids, an aspect ratio wide", () => {
    const l = new GameObject("scanner").addComponent(Light, { type: "spot" });
    const last = () => ops.filter((o) => o[0] === "render.light").at(-1)!;
    expect(last().slice(-2)).toEqual(["cone", 1]);
    l.spotShape = "pyramid";
    l.aspectRatio = 40;
    expect(last().slice(-2)).toEqual(["pyramid", 40]);
    l.aspectRatio = 0;
    expect(l.aspectRatio).toBeGreaterThan(0);
  });

  // M7-FOG-06
  test("fog volumes send their box, density, colour and soft edge", () => {
    const go = new GameObject("mist");
    const v = go.addComponent(FogVolume, { size: new Vector3(6, 1, 4), density: 0.8 });
    const last = () => ops.filter((o) => o[0] === "render.fogVolume").at(-1)!;
    expect(last()).toEqual(["render.fogVolume", v.id, go.id, [6, 1, 4], 0.8, [0.75, 0.78, 0.82], 1, "box"]);
    v.blendDistance = -1;
    expect(last()[6]).toBe(0);
    v.shape = "ellipsoid";
    expect(last().at(-1)).toBe("ellipsoid");
    v.color = new Color(0.1, 0.2, 0.3);
    expect(last()[5]).toEqual([0.1, 0.2, 0.3]);
    // The engine runs onDestroy when the object goes.
    v.onDestroy();
    expect(ops.at(-1)).toEqual(["render.remove", v.id]);
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
    expect(fogOps().at(-1)).toEqual(["render.fog", 0, "linear", [0.1, 0.2, 0.3], 0.01, 5, 25, [0.2, 1, 0.3], null, true]);
  });
});
