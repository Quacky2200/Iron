// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// Where dropped files go (editor/src/placement.ts).

import { assetKind, groundPoint, importFolder, placeable } from "../src/placement.ts";
import { close, report, same, test, yes } from "./check.ts";

// M5-DND-21
test("a drop lands on the ground under the pointer, else in front of the camera", () => {
  const p = groundPoint(new Vector3(0, 10, 0), new Vector3(0, -1, 1).normalized);
  close(p.x, 0, "x");
  close(p.y, 0, "on the ground");
  close(p.z, 10, "where the ray meets it");
  const up = groundPoint(new Vector3(1, 2, 3), new Vector3(0, 1, 0), 5);
  close(up.y, 7, "looking up: 5 along the ray");
  const flat = groundPoint(new Vector3(0, 2, 0), new Vector3(1, 0, 0));
  close(flat.x, 10, "level: the default distance");
});

// M5-DND-21
test("files are told apart by their names", () => {
  same(assetKind("assets/Tree.FBX"), "model", "fbx");
  same(assetKind("a.glb"), "model", "glb");
  same(assetKind("p/Cube.prefab.json"), "prefab", "prefab");
  same(assetKind("scenes/main.scene.json"), "scene", "scene");
  same(assetKind("data.json"), "other", "json");
  yes(placeable("x.gltf") && placeable("x.prefab.json") && !placeable("x.scene.json"), "placeable");
  same(importFolder("C:/Downloads/tree.fbx"), "assets/models", "models");
  same(importFolder("C:/Downloads/a.png"), "assets", "others");
});

export function run(): string {
  return report();
}
