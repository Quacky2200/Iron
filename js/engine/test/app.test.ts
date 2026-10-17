import { afterEach, describe, expect, test } from "bun:test";

import { Application, SystemInfo } from "../src/index.ts";

afterEach(() => {
  delete globalThis.__iron_info;
  delete (globalThis as { __iron_op?: unknown }).__iron_op;
});

// M5-APP-02
describe("Application and SystemInfo", () => {
  test("read what the engine installed, with defaults before it has", () => {
    expect(Application.version).toBe("0.0.0");
    expect(SystemInfo.processorCount).toBe(1);
    globalThis.__iron_info = {
      app: {
        name: "fox-run",
        productName: "Fox Run",
        version: "1.2.0",
        companyName: "Iron",
        identifier: "dev.iron.fox",
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

  test("quit is an op", () => {
    const ops: unknown[][] = [];
    (globalThis as { __iron_op?: unknown }).__iron_op = (...args: unknown[]) => ops.push(args);
    Application.quit();
    expect(ops).toEqual([["app.quit"]]);
  });
});
