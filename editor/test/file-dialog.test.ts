// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// The open / save dialog's logic (editor/src/file-dialog.ts).

import { ALL_FILES, MODEL_FILTER, matches, resolveTyped, SCENE_FILTER, shownEntries, stemOf, withExtension } from "../src/file-dialog.ts";
import { eq, report, same, test, yes } from "./check.ts";

// M5-EDT-31
test("filters pick files by their extensions and keep folders", () => {
  yes(matches("main.scene.json", SCENE_FILTER), "a scene");
  yes(matches("Main.SCENE.json", SCENE_FILTER), "any case");
  yes(!matches("main.json", SCENE_FILTER), "not a scene");
  yes(matches("tree.FBX", MODEL_FILTER) && matches("a.glb", MODEL_FILTER), "models");
  yes(matches("anything", ALL_FILES), "all files");
  const entries: ProjectEntry[] = [
    { name: "scenes", dir: true, size: 0 },
    { name: "a.scene.json", dir: false, size: 3 },
    { name: "b.png", dir: false, size: 4 },
  ];
  eq(
    shownEntries(entries, SCENE_FILTER, "open").map((e) => e.name),
    ["scenes", "a.scene.json"],
    "open",
  );
  eq(
    shownEntries(entries, ALL_FILES, "folder").map((e) => e.name),
    ["scenes"],
    "folders only",
  );
});

// M5-EDT-32
test("saved names get the filter's extension once", () => {
  same(withExtension("level", SCENE_FILTER), "level.scene.json", "added");
  same(withExtension("level.scene.json", SCENE_FILTER), "level.scene.json", "kept");
  same(withExtension("level.json", SCENE_FILTER), "level.scene.json", "not doubled");
  same(withExtension(" level ", SCENE_FILTER), "level.scene.json", "trimmed");
  same(withExtension("notes.txt", ALL_FILES), "notes.txt", "all files");
  same(withExtension("", SCENE_FILTER), "", "nothing");
  same(stemOf("main.scene.json", SCENE_FILTER), "main", "stem");
});

// M5-EDT-33
test("typed paths become project paths and can't leave the project", () => {
  same(resolveTyped("scenes", "level.scene.json") ?? "null", "scenes/level.scene.json", "relative");
  same(resolveTyped("scenes", "../prefabs/a.json") ?? "null", "prefabs/a.json", "up");
  same(resolveTyped("scenes/sub", "/top.json") ?? "null", "top.json", "from the root");
  same(resolveTyped("scenes", "a\\b.json") ?? "null", "scenes/a/b.json", "backslashes");
  same(resolveTyped("scenes", "./x/./y") ?? "null", "scenes/x/y", "dots");
  same(resolveTyped("", "..") ?? "null", "null", "above the root");
  same(resolveTyped("scenes", "../..") ?? "null", "null", "two up");
  same(resolveTyped("", "C:/Windows") ?? "null", "null", "a drive");
  same(resolveTyped("scenes", "..") ?? "null", "", "the root");
});

export function run(): string {
  return report();
}
