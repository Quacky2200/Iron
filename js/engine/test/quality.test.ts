// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

import { afterEach, beforeEach, describe, expect, test } from "bun:test";

import { QualitySettings } from "../src/index.ts";
import { _resetQualityForTests } from "../src/quality.ts";

let ops: unknown[][] = [];

beforeEach(() => {
  ops = [];
  (globalThis as { __platypus_op?: unknown }).__platypus_op = (...args: unknown[]) => ops.push(args);
});

afterEach(() => {
  delete (globalThis as { __platypus_op?: unknown }).__platypus_op;
  delete globalThis.__platypus_info;
  _resetQualityForTests();
});

// M5-QUA-01
describe("QualitySettings", () => {
  test("levels set anti-aliasing, scale and shadows together; settings change on their own", () => {
    expect(QualitySettings.names).toEqual(["Low", "Medium", "High", "Ultra"]);
    expect(QualitySettings.getQualityLevel()).toBe(2);
    let heard = -1;
    QualitySettings.onLevelChanged((l) => (heard = l));
    QualitySettings.setQualityLevel("Low");
    // The choice is also saved (prefs.set), checked in display.test.ts.
    ops = ops.filter((o) => o[0] !== "prefs.set");
    expect(ops).toEqual([
      ["render.quality", 0, 1, 0.75, "hardOnly", 1, 0.4, 1, "disable", 1, false, true, 8, false],
      ["render.shadowSettings", 0, 20, 1024, 1],
    ]);
    expect(heard).toBe(0);
    ops = [];
    QualitySettings.antiAliasing = 3;
    QualitySettings.renderScale = 5;
    ops = ops.filter((o) => o[0] !== "prefs.set");
    expect(ops).toEqual([
      ["render.quality", 0, 4, 0.75, "hardOnly", 1, 0.4, 1, "disable", 1, false, true, 8, false],
      ["render.quality", 0, 4, 2, "hardOnly", 1, 0.4, 1, "disable", 1, false, true, 8, false],
    ]);
    QualitySettings.lodBias = 3;
    expect(ops.filter((o) => o[0] === "render.quality").at(-1)![6]).toBe(3);
    // Texture quality (M5-QUA-02): anisotropic filtering and the mip limit.
    QualitySettings.anisotropicFiltering = "forceEnable";
    QualitySettings.globalTextureMipmapLimit = 7;
    expect(ops.filter((o) => o[0] === "render.quality").at(-1)!.slice(8, 10)).toEqual(["forceEnable", 3]);
    expect(() => QualitySettings.setQualityLevel("Potato")).toThrow("Potato");
  });

  test("the game's manifest picks the default level and overrides levels", () => {
    globalThis.__platypus_info = {
      app: {
        name: "g",
        productName: "g",
        version: "1",
        companyName: "",
        identifier: "",
        engineVersion: "0",
        quality: { default: "Low", levels: { Low: { renderScale: 0.5 } } },
      },
      system: {} as never,
    };
    _resetQualityForTests();
    expect(QualitySettings.getQualityLevel()).toBe(0);
    expect(QualitySettings.renderScale).toBe(0.5);
    expect(QualitySettings.levels[0]!.renderScale).toBe(0.5);
  });

  // M7-RP-03
  test("each level carries the render pipeline's settings, and the default can go by platform", () => {
    // Low is the phones' tier; High has everything.
    expect(QualitySettings.volumetricFog).toBe(true);
    expect(QualitySettings.additionalLights).toBe(-1);
    QualitySettings.setQualityLevel("Low");
    expect(QualitySettings.volumetricFog).toBe(false);
    expect(QualitySettings.additionalLights).toBe(8);
    expect(QualitySettings.additionalLightShadows).toBe(false);
    QualitySettings.additionalLights = 500;
    QualitySettings.hdr = false;
    expect(ops.filter((o) => o[0] === "render.quality").at(-1)!.slice(10)).toEqual([false, false, 256, false]);
    const on = (platform: string) => {
      globalThis.__platypus_info = {
        app: { quality: { default: { Android: "Low", iOS: "Medium", default: "Ultra" } } } as never,
        system: { operatingSystemFamily: platform } as never,
      };
      _resetQualityForTests();
      return QualitySettings.names[QualitySettings.getQualityLevel()];
    };
    expect(on("Android")).toBe("Low");
    expect(on("iOS")).toBe("Medium");
    expect(on("Windows")).toBe("Ultra");
  });
});
