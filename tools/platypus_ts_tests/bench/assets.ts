// SPDX-License-Identifier: Apache-2.0

// Assets: the asset database lookups, compiled against the native API and run in bun against
// js/engine/src/assets.ts. The game has two assets with .meta files.

export function lookups(): string {
  return [
    Assets.guidToPath("guid-a"),
    Assets.guidToPath("nope") === "" ? "none" : "?",
    Assets.pathToGuid("textures/a.png"),
    Assets.pathToGuid("./textures\\a.png"),
    Assets.pathToGuid("missing.png") === "" ? "none" : "?",
  ].join(",");
}

export function sources(): string {
  return [Assets.resolve("guid:guid-b"), Assets.resolve("guid:unknown"), Assets.resolve("plain/path.png")].join(",");
}

export function importers(): string {
  return [Assets.importer("textures/a.png"), Assets.importer("guid:guid-b"), Assets.importer("nothing")].join(";");
}

export function enumerate(): string {
  let s = "";
  for (let i = 0; i < Assets.count(); i++) s += Assets.pathAt(i) + "=" + Assets.guidAt(i) + ";";
  return s;
}
