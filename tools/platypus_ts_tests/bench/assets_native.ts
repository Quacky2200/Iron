// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// Assets.exists / list / packOf from compiled code. The Rust test (tests/engine_api/assets_native.rs)
// fills the index the host would, and checks what the program gets.

export function files(): string {
  const wavs: string[] = Assets.list("assets/sfx/*.wav");
  const boss = Assets.packOf("assets/sfx/b.wav") === "scene-boss" ? "boss" : Assets.packOf("assets/sfx/b.wav");
  const none = Assets.packOf("assets/none.wav") === "" ? "none" : "?";
  const empty = Assets.list("nothing/**").length === 0 ? "empty" : "?";
  return `${wavs.length}|${wavs.join(",")}|${Assets.exists("assets/sfx/a.wav")}|${Assets.exists("assets/sfx/none.wav")}|${Assets.exists("data:,x")}|${boss}|${none}|${empty}`;
}
