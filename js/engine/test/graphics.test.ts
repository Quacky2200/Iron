import { afterEach, describe, expect, test } from "bun:test";

import { GraphicsSettings } from "../src/index.ts";

afterEach(() => {
  delete (globalThis as { __iron_info?: unknown }).__iron_info;
});

// M7-RP-01
describe("GraphicsSettings", () => {
  test("the project's light units", () => {
    expect(GraphicsSettings.lightUnits).toBe("legacy");
    (globalThis as { __iron_info?: unknown }).__iron_info = { app: { lightUnits: "physical" } };
    expect(GraphicsSettings.lightUnits).toBe("physical");
    (globalThis as { __iron_info?: unknown }).__iron_info = { app: { lightUnits: "candles" } };
    expect(GraphicsSettings.lightUnits).toBe("legacy");
  });
});
