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
