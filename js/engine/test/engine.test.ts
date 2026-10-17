import { describe, expect, test } from "bun:test";
import { Component, register } from "../src/index.ts";

class Counter extends Component {
  starts = 0;
  total = 0;
  override onStart() {
    this.starts++;
  }
  override onUpdate(dt: number) {
    this.total += dt;
  }
  totals() {
    return [this.starts, this.total];
  }
}

describe("component host", () => {
  test("registers classes by name", () => {
    register({ Counter });
    expect(globalThis.__iron_registered()).toContain("Counter");
  });

  test("dispatches by id and method name", () => {
    register({ Counter });
    globalThis.__iron_create("Counter", 100);
    globalThis.__iron_dispatch(100, "onStart");
    globalThis.__iron_dispatch(100, "onUpdate", 0.5);
    globalThis.__iron_dispatch(100, "onUpdate", 0.25);
    expect(globalThis.__iron_dispatch(100, "totals")).toEqual([1, 0.75]);
    // Missing methods are ignored.
    expect(globalThis.__iron_dispatch(100, "onNothing")).toBeUndefined();
  });

  test("unknown classes and ids throw", () => {
    expect(() => globalThis.__iron_create("Missing", 1)).toThrow("unknown component class");
    expect(() => globalThis.__iron_dispatch(9999, "onUpdate")).toThrow("no component");
  });

  test("destroy removes the instance", () => {
    register({ Counter });
    globalThis.__iron_create("Counter", 200);
    globalThis.__iron_destroy(200);
    expect(() => globalThis.__iron_dispatch(200, "onUpdate", 1)).toThrow("no component");
  });
});

// M3-GAME-02
describe("setUiScale", () => {
  test("sends the uiScale op for each form", async () => {
    const { setUiScale } = await import("../src/index.ts");
    const ops: unknown[][] = [];
    (globalThis as { __iron_op?: unknown }).__iron_op = (...args: unknown[]) => ops.push(args);
    setUiScale({ height: 1080 });
    setUiScale({ width: 1920, height: 1080 });
    setUiScale(1.5);
    setUiScale(null);
    expect(ops).toEqual([
      ["uiScale", "height", 1080],
      ["uiScale", "fit", 1920, 1080],
      ["uiScale", "fixed", 1.5],
      ["uiScale", "none"],
    ]);
    delete (globalThis as { __iron_op?: unknown }).__iron_op;
  });
});
