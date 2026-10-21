import { afterEach, describe, expect, test } from "bun:test";

import { GraphicsSettings } from "../src/index.ts";

const setPipeline = (p: unknown) => {
  (globalThis as { __iron_info?: unknown }).__iron_info = { app: { renderPipeline: p } };
};

afterEach(() => {
  delete (globalThis as { __iron_info?: unknown }).__iron_info;
});

// M7-RP-01
describe("GraphicsSettings", () => {
  test("the project's render pipeline and what it runs", () => {
    expect(GraphicsSettings.renderPipeline).toBe("builtin");
    expect(GraphicsSettings.supportsAutoExposure).toBe(true);
    setPipeline("universal");
    expect(GraphicsSettings.renderPipeline).toBe("universal");
    expect(GraphicsSettings.supportsVolumetricFog).toBe(false);
    expect(GraphicsSettings.supportsAutoExposure).toBe(false);
    expect(GraphicsSettings.maxAdditionalLights).toBe(32);
    setPipeline("hd");
    expect(GraphicsSettings.renderPipeline).toBe("hd");
    expect(GraphicsSettings.supportsVolumetricFog).toBe(true);
    setPipeline("nonsense");
    expect(GraphicsSettings.renderPipeline).toBe("builtin");
  });
});
