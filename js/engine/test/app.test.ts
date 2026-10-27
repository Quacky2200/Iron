// SPDX-License-Identifier: Apache-2.0

import { afterEach, describe, expect, test } from "bun:test";

import { _resetApplicationForTests } from "../src/app.ts";
import { Application, Component, GameObject, SystemInfo } from "../src/index.ts";
import { _resetSceneForTests } from "../src/scene.ts";

afterEach(() => {
  _resetApplicationForTests();
  delete globalThis.__platypus_info;
  delete (globalThis as { __platypus_op?: unknown }).__platypus_op;
});

// M5-APP-02
describe("Application and SystemInfo", () => {
  test("read what the engine installed, with defaults before it has", () => {
    expect(Application.version).toBe("0.0.0");
    expect(SystemInfo.processorCount).toBe(1);
    globalThis.__platypus_info = {
      app: {
        name: "fox-run",
        productName: "Fox Run",
        version: "1.2.0",
        companyName: "Platypus",
        identifier: "dev.platypus.fox",
        engineVersion: "0.1.0",
      },
      system: {
        operatingSystem: "Windows 11 Pro",
        operatingSystemFamily: "Windows",
        operatingSystemVersion: "11",
        deviceType: "Desktop",
        deviceName: "box",
        processorType: "CPU",
        processorCount: 16,
        systemMemorySize: 32768,
        architecture: "x86_64",
        language: "en-GB",
        touchSupported: false,
        graphicsDeviceName: "GPU",
        screenWidth: 2560,
        screenHeight: 1440,
        screenRefreshRate: 144,
        dpi: 144,
      },
    };
    expect([Application.productName, Application.version, Application.platform, Application.systemLanguage]).toEqual([
      "Fox Run",
      "1.2.0",
      "Windows",
      "en-GB",
    ]);
    expect(Application.isMobilePlatform).toBe(false);
    expect([SystemInfo.processorCount, SystemInfo.systemMemorySize, SystemInfo.graphicsDeviceName]).toEqual([16, 32768, "GPU"]);
    expect(SystemInfo.screen).toEqual({ width: 2560, height: 1440, refreshRate: 144, dpi: 144 });
  });

  // M5-APP-11
  test("args are the command line's arguments after --", () => {
    expect(Application.args).toEqual([]);
    globalThis.__platypus_info = { app: { args: ["--goto", "a.md#b"] }, system: {} } as never;
    const args = Application.args;
    expect(args).toEqual(["--goto", "a.md#b"]);
    args.push("changed");
    expect(Application.args).toEqual(["--goto", "a.md#b"]);
  });

  // M5-APP-12
  test("quit is an op, with an exit code", () => {
    const ops: unknown[][] = [];
    (globalThis as { __platypus_op?: unknown }).__platypus_op = (...args: unknown[]) => ops.push(args);
    Application.quit();
    Application.quit(2.7);
    expect(ops).toEqual([
      ["app.quit", 0],
      ["app.quit", 2],
    ]);
  });
});

class Watcher extends Component {
  events: string[] = [];
  override onApplicationFocus(hasFocus: boolean) {
    this.events.push(`focus ${hasFocus}`);
  }
  override onApplicationPause(paused: boolean) {
    this.events.push(`pause ${paused}`);
  }
}

// M5-APP-10
describe("Running in the background and focus", () => {
  test("runInBackground starts from the manifest and goes to the engine when set", () => {
    const ops: unknown[][] = [];
    (globalThis as { __platypus_op?: unknown }).__platypus_op = (...args: unknown[]) => ops.push(args);
    expect(Application.runInBackground).toBe(false);
    globalThis.__platypus_info = { app: { runInBackground: true, targetFrameRate: 30 }, system: {} } as never;
    // The engine's values (the manifest's, or kept over a reload).
    expect([Application.runInBackground, Application.targetFrameRate]).toEqual([true, 30]);
    Application.runInBackground = false;
    expect(Application.runInBackground).toBe(false);
    expect(ops).toEqual([["app.runInBackground", false]]);
  });

  test("focus and pause reach listeners and active components' hooks", () => {
    _resetSceneForTests();
    (globalThis as { __platypus_op?: unknown }).__platypus_op = () => {};
    const on = new GameObject("On").addComponent(Watcher);
    const offObject = new GameObject("Off");
    const off = offObject.addComponent(Watcher);
    offObject.enabled = false;

    const seen: boolean[] = [];
    const stop = Application.onFocusChanged((f) => seen.push(f));
    Application.onFocusChanged(() => {
      throw new Error("a broken listener doesn't stop the others");
    });
    expect([Application.isFocused, Application.isPaused]).toEqual([true, false]);

    const errors = console.error;
    console.error = () => {};
    try {
      globalThis.__platypus_focus(false, true);
      expect([Application.isFocused, Application.isPaused]).toEqual([false, true]);
      // Same state again: nothing new.
      globalThis.__platypus_focus(false, true);
      globalThis.__platypus_focus(true, false);
      stop();
      // Running in the background: focus changes, the pause doesn't.
      globalThis.__platypus_focus(false, false);
    } finally {
      console.error = errors;
    }
    expect(seen).toEqual([false, true]);
    expect(on.events).toEqual(["pause true", "focus false", "pause false", "focus true", "focus false"]);
    expect(off.events).toEqual([]);
  });
});
