// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// Where something dropped on the Scene view goes, and what a dropped project file is: the ground
// (y = 0) under the pointer, else a little in front of the camera when the ray misses it.

/** How far in front of the camera a drop lands when the ray doesn't meet the ground. */
export const DROP_DISTANCE = 10;

/** Where a ray from `origin` along `dir` meets the plane y = 0 in front of it, else `distance` along it. */
export function groundPoint(origin: Vector3, dir: Vector3, distance: number = DROP_DISTANCE): Vector3 {
  if (dir.y < -1e-4) {
    const t = -origin.y / dir.y;
    if (t > 0 && t < 10_000) return new Vector3(origin.x + dir.x * t, 0, origin.z + dir.z * t);
  }
  return new Vector3(origin.x + dir.x * distance, origin.y + dir.y * distance, origin.z + dir.z * distance);
}

export type AssetKind = "model" | "prefab" | "scene" | "other";

/** What a project file is, by its name. */
export function assetKind(path: string): AssetKind {
  const p = path.toLowerCase();
  if (p.endsWith(".prefab.json")) return "prefab";
  if (p.endsWith(".scene.json")) return "scene";
  if (p.endsWith(".glb") || p.endsWith(".gltf") || p.endsWith(".fbx")) return "model";
  return "other";
}

/** Whether a file can be placed in the scene (a model or a prefab). */
export const placeable = (path: string): boolean => {
  const k = assetKind(path);
  return k === "model" || k === "prefab";
};

/** Where a dropped system file is copied first: `assets/models` for models, else `assets`. */
export function importFolder(path: string): string {
  return assetKind(path) === "model" ? "assets/models" : "assets";
}
