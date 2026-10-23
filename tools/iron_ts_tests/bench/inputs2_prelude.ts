// SPDX-License-Identifier: Apache-2.0

// Two frames: the first as in inputs_prelude.ts, then a key released, another pressed, a gamepad
// gone, and the window losing focus.
import { _resetInputForTests } from "../../../js/engine/src/input.ts";

const g = globalThis as any;
_resetInputForTests();
g.__iron_input("keydown", "KeyW", "w", false);
g.__iron_input("keydown", "Space", " ", false);
g.__iron_input("keydown", "MouseLeft", "", false);
g.__iron_input("mousemove", 10.5, 20.25);
g.__iron_input("mousedelta", 3, 4);
g.__iron_input("wheel", 0, 2);
g.__iron_input("gamepadconnected", 0, "Pad A");
g.__iron_input("gamepadbutton", 0, "South", 1);
g.__iron_input("gamepadconnected", 1, "Pad B");
g.__iron_input("gamepadaxis", 1, "LeftStickX", -0.9);
g.__iron_tick(0.016);
g.__iron_input("keyup", "Space", " ");
g.__iron_input("keyup", "MouseLeft", "");
g.__iron_input("keydown", "KeyD", "d", false);
g.__iron_input("mousedelta", 7, 7);
g.__iron_input("gamepaddisconnected", 1);
g.__iron_tick(0.016);
g.__iron_input("blur");

// The native API's gamepad shape over the original `input` (functions by id, no objects).
import { input as I } from "../../../js/engine/src/input.ts";
g.input = Object.create(I, {
  gamepadCount: { value: () => I.gamepads().length },
  gamepadId: { value: (i: number) => I.gamepads()[i]?.id ?? -1 },
  gamepadConnected: { value: (id: number) => !!I.gamepad(id) },
  gamepadName: { value: (id: number) => I.gamepad(id)?.name ?? "" },
  gamepadButton: { value: (id: number, b: string) => I.gamepad(id)?.buttons[b] ?? 0 },
  gamepadAxis: { value: (id: number, a: string) => I.gamepad(id)?.axes[a] ?? 0 },
});
