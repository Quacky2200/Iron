import { afterEach, beforeEach, describe, expect, test } from "bun:test";

import { Application, PlayerPrefs, QualitySettings, Screen } from "../src/index.ts";
import { _resetPrefsForTests } from "../src/prefs.ts";
import { _applyDefaultQuality, _resetQualityForTests } from "../src/quality.ts";

let ops: unknown[][] = [];

const info = (extra: Record<string, unknown>) => {
  globalThis.__iron_info = {
    app: { name: "g", productName: "g", version: "1", companyName: "", identifier: "", engineVersion: "0" },
    system: {} as never,
    ...extra,
  } as never;
};

beforeEach(() => {
  ops = [];
  (globalThis as { __iron_op?: unknown }).__iron_op = (...args: unknown[]) => ops.push(args);
});

afterEach(() => {
  delete (globalThis as { __iron_op?: unknown }).__iron_op;
  delete globalThis.__iron_info;
  _resetPrefsForTests();
  _resetQualityForTests();
});

// M5-PRF-02
describe("PlayerPrefs", () => {
  test("reads what the engine loaded and sends changes", () => {
    info({ prefs: { volume: 0.5, name: "Ada", level: 3.7 } });
    _resetPrefsForTests();
    expect(PlayerPrefs.getFloat("volume")).toBe(0.5);
    expect(PlayerPrefs.getString("name")).toBe("Ada");
    expect(PlayerPrefs.getInt("level")).toBe(3);
    // Wrong types and missing keys give the fallback.
    expect(PlayerPrefs.getInt("name", 7)).toBe(7);
    expect(PlayerPrefs.getString("volume", "none")).toBe("none");
    expect(PlayerPrefs.hasKey("missing")).toBe(false);
    PlayerPrefs.setInt("level", 9.9);
    PlayerPrefs.deleteKey("name");
    PlayerPrefs.save();
    expect(PlayerPrefs.getInt("level")).toBe(9);
    expect(PlayerPrefs.hasKey("name")).toBe(false);
    expect(ops).toEqual([["prefs.set", "level", 9], ["prefs.delete", "name"], ["prefs.save"]]);
    PlayerPrefs.deleteAll();
    expect(PlayerPrefs.hasKey("volume")).toBe(false);
  });
});

// M5-DSP-03
describe("display settings", () => {
  test("Screen reads the engine's display state and asks for changes", () => {
    info({
      display: {
        mode: "Windowed",
        width: 1600,
        height: 900,
        refreshRate: 0,
        supportedModes: ["FullScreenWindow", "MaximizedWindow", "Windowed"],
        resolutions: [{ width: 2560, height: 1440, refreshRate: 144 }],
        currentResolution: { width: 2560, height: 1440, refreshRate: 144 },
      },
    });
    expect(Screen.fullScreenMode).toBe("Windowed");
    expect(Screen.fullScreen).toBe(false);
    expect(Screen.windowSize).toEqual({ width: 1600, height: 900 });
    expect(Screen.supportedFullScreenModes).not.toContain("ExclusiveFullScreen");
    expect(Screen.currentResolution.refreshRate).toBe(144);
    Screen.fullScreen = true;
    Screen.setResolution(1280.4, 720, "ExclusiveFullScreen", 60);
    Screen.setResolution(800, 600, false);
    expect(ops).toEqual([
      ["screen.set", "FullScreenWindow", 1600, 900, 0],
      ["screen.set", "ExclusiveFullScreen", 1280, 720, 60],
      ["screen.set", "Windowed", 800, 600, 0],
    ]);
  });

  test("vsync and the frame cap go to the engine; quality choices are saved and come back", () => {
    Application.targetFrameRate = 144;
    Application.targetFrameRate = 0;
    expect(Application.targetFrameRate).toBe(-1);
    QualitySettings.vSyncCount = 0;
    QualitySettings.setQualityLevel("Medium");
    QualitySettings.vSyncCount = 0;
    expect(ops.filter((o) => o[0] === "app.targetFrameRate")).toEqual([["app.targetFrameRate", 144], ["app.targetFrameRate", -1]]);
    expect(ops.filter((o) => o[0] === "render.quality").at(-1)!.at(-1)).toBe(0);
    const saved = ops.filter((o) => o[0] === "prefs.set" && o[1] === "iron.quality").at(-1)![2] as string;

    // Next launch: the engine hands the saved choice back and it's applied, not saved again.
    info({ prefs: { "iron.quality": saved } });
    _resetPrefsForTests();
    _resetQualityForTests();
    ops = [];
    _applyDefaultQuality();
    expect(QualitySettings.names[QualitySettings.getQualityLevel()]).toBe("Medium");
    expect(QualitySettings.vSyncCount).toBe(0);
    expect(ops.some((o) => o[0] === "prefs.set")).toBe(false);
    expect(ops.find((o) => o[0] === "render.quality")!.at(-1)).toBe(0);
  });
});
