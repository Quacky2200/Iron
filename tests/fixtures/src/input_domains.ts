// SPDX-License-Identifier: Apache-2.0

// M9-API-1500: `input` follows the event domain whose code is running. The Rust test drives
// `__iron_input` per domain and probes with `__probe()` from main and from an isolated window's `run`.
import { input } from "@iron/engine";

const listened: string[] = [];
input.on("keydown", (e) => listened.push(e.code));

const g = globalThis as {
  __probe?: () => string;
  __listened?: () => string;
};
g.__probe = () =>
  `A ${input.isKeyDown("KeyA")}, B ${input.isKeyDown("KeyB")}, A! ${input.getKeyDown("KeyA")}, x ${input.mousePosition.x}`;
g.__listened = () => listened.join(",");
