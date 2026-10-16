import { beforeEach, describe, expect, spyOn, test } from "bun:test";

import { input } from "../src/index.ts";
import { _resetInputForTests } from "../src/input.ts";

const send = (type: string, ...args: unknown[]) => globalThis.__iron_input(type, ...args);

beforeEach(() => _resetInputForTests());

describe("game input", () => {
  test("keys are tracked by code and reported to listeners", () => {
    const seen: string[] = [];
    const stop = input.on("keydown", (e) => seen.push(`${e.code}/${e.key}/${e.repeat}`));
    send("keydown", "KeyW", "w", false);
    expect(input.isKeyDown("KeyW")).toBe(true);
    send("keydown", "KeyW", "w", true);
    send("keyup", "KeyW", "w");
    expect(input.isKeyDown("KeyW")).toBe(false);
    expect(seen).toEqual(["KeyW/w/false", "KeyW/w/true"]);
    stop();
    send("keydown", "KeyA", "a", false);
    expect(seen).toHaveLength(2);
  });

  test("losing window focus releases everything", () => {
    send("keydown", "KeyW", "w", false);
    send("gamepadconnected", 0, "Pad");
    send("gamepadbutton", 0, "South", 1);
    send("blur");
    expect(input.isKeyDown("KeyW")).toBe(false);
    expect(input.isButtonDown("South")).toBe(false);
  });

  // M3-PAD-01
  test("gamepad buttons and axes reach scripts", () => {
    const log: string[] = [];
    input.on("gamepadconnected", (e) => log.push(`connected ${e.gamepad.id} ${e.gamepad.name}`));
    input.on("gamepadbuttondown", (e) => log.push(`down ${e.button}`));
    input.on("gamepadbuttonup", (e) => log.push(`up ${e.button}`));
    input.on("gamepadaxis", (e) => log.push(`${e.axis} ${e.value}`));
    input.on("gamepaddisconnected", (e) => log.push(`gone ${e.gamepad.id}`));

    send("gamepadconnected", 1, "Xbox Controller");
    send("gamepadbutton", 1, "South", 1);
    expect(input.isButtonDown("South")).toBe(true);
    send("gamepadbutton", 1, "RightTrigger", 0.3);
    send("gamepadbutton", 1, "RightTrigger", 0.8);
    send("gamepadbutton", 1, "South", 0);
    send("gamepadaxis", 1, "LeftStickX", -0.5);
    expect(input.gamepad(1)?.axes.LeftStickX).toBe(-0.5);
    expect(input.gamepad(1)?.buttons.RightTrigger).toBe(0.8);
    send("gamepaddisconnected", 1);
    expect(input.gamepads()).toEqual([]);
    expect(log).toEqual([
      "connected 1 Xbox Controller",
      "down South",
      "down RightTrigger",
      "up South",
      "LeftStickX -0.5",
      "gone 1",
    ]);
  });

  test("a throwing listener is logged and the others still run", () => {
    const error = spyOn(console, "error").mockImplementation(() => {});
    const seen: string[] = [];
    input.on("keydown", () => {
      throw new Error("boom");
    });
    input.on("keydown", (e) => seen.push(e.code));
    send("keydown", "Space", " ", false);
    expect(seen).toEqual(["Space"]);
    expect(error.mock.calls[0]![0]).toContain("boom");
    error.mockRestore();
  });
});
