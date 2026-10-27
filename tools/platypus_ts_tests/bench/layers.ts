// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// LayerMask: names, masks and the game's own layer names, compiled against the native API and run
// in bun against js/engine/src/layers.ts. The game names layers 8, 9 and 4 (renaming "Water").

export function names(): string {
  return `${LayerMask.layerToName(0)}|${LayerMask.layerToName(5)}|${LayerMask.layerToName(3)}|${LayerMask.layerToName(8)}|${LayerMask.layerToName(4)}`;
}

export function numbers(): string {
  return `${LayerMask.nameToLayer("UI")},${LayerMask.nameToLayer("Player")},${LayerMask.nameToLayer("Minimap")},${LayerMask.nameToLayer("Water")},${LayerMask.nameToLayer("Nope")}`;
}

export function masks(): string {
  const m = LayerMask.getMask("Default", "Player");
  const none = LayerMask.getMask();
  const unknown = LayerMask.getMask("Nope", "Minimap");
  return `${m},${none},${unknown},${LayerMask.everything},${LayerMask.nothing}`;
}

export function contains(): string {
  const m = LayerMask.getMask("Default", "Player", "UI");
  let s = "";
  for (let i = 0; i < 12; i++) s += LayerMask.contains(m, i) ? "1" : "0";
  s += LayerMask.contains(LayerMask.everything, 31) ? "1" : "0";
  s += LayerMask.contains(LayerMask.nothing, 0) ? "1" : "0";
  return s;
}
