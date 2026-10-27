// SPDX-License-Identifier: Apache-2.0

import { beforeEach, describe, expect, spyOn, test } from "bun:test";

import { Cursor, CursorLockMode, input, Vector2 } from "../src/index.ts";
import { _resetInputForTests } from "../src/input.ts";

const send = (type: string, ...args: unknown[]) => globalThis.__platypus_input(type, ...args);

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

// M5-CUR-01
describe("mouse and cursor", () => {
  const frame = () => globalThis.__platypus_tick(1 / 60);

  test("position, buttons, raw movement and the wheel, Unity style", () => {
    send("mousemove", 120, 80);
    expect(input.mousePosition).toEqual(new Vector2(120, 80));
    send("keydown", "MouseLeft", "", false);
    send("mousedelta", 3, 4);
    send("mousedelta", 2, -1);
    send("wheel", 0, 2);
    frame();
    expect(input.getMouseButton(0)).toBe(true);
    expect(input.getMouseButtonDown(0)).toBe(true);
    expect(input.getButton("Fire1")).toBe(true);
    // Raw movement adds up within a frame; y is up positive.
    expect(input.mouseDelta).toEqual(new Vector2(5, -3));
    expect(input.getAxis("Mouse X")).toBeCloseTo(0.5);
    expect(input.getAxis("Mouse Y")).toBeCloseTo(-0.3);
    expect(input.getAxis("Mouse ScrollWheel")).toBeCloseTo(0.2);
    send("keyup", "MouseLeft", "");
    frame();
    expect(input.getMouseButtonUp(0)).toBe(true);
    expect(input.mouseDelta).toEqual(new Vector2(0, 0));
  });

  test("Cursor settings become ops", () => {
    const ops: unknown[][] = [];
    (globalThis as { __platypus_op?: unknown }).__platypus_op = (...args: unknown[]) => ops.push(args);
    Cursor.visible = false;
    Cursor.lockState = CursorLockMode.Locked;
    Cursor.setCursor("assets/hand.png", new Vector2(4, 2));
    Cursor.setCursor(null);
    expect(ops).toEqual([
      ["cursor.visible", false],
      ["cursor.lock", "locked"],
      ["cursor.image", "assets/hand.png", [4, 2]],
      ["cursor.image", null],
    ]);
    expect([Cursor.visible, Cursor.lockState]).toEqual([false, "locked"]);
    delete (globalThis as { __platypus_op?: unknown }).__platypus_op;
  });
});
