// SPDX-License-Identifier: Apache-2.0

// M3-PAD-01: gameplay code listening to gamepads and keys through @platypus/engine's
// `input`. The Rust test reads the log back with `__input_log()`.
import { input } from "@platypus/engine";

const log: string[] = [];
input.on("gamepadconnected", (e) => log.push(`connected ${e.gamepad.name}`));
input.on("gamepadbuttondown", (e) => log.push(`down ${e.button}`));
input.on("gamepadaxis", (e) => log.push(`${e.axis} ${e.value}`));
input.on("keydown", (e) => log.push(`key ${e.code}`));

(globalThis as { __input_log?: () => string }).__input_log = () =>
  [...log, `held ${input.isButtonDown("South")}`].join(", ");
