// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// Awaitable waits on engine time: Task.delay, Task.delaySeconds and Task.frame over the runtime's
// timers. The test advances the timers like the host does each frame.

let log = "";

async function worker(): Promise<void> {
  log += "a";
  await Task.delay(100);
  log += "b";
  await Task.delaySeconds(0.5);
  log += "c";
  await Task.frame();
  log += "d";
}

export function start(): void {
  log = "";
  worker();
}

export function text(): string {
  return log;
}
