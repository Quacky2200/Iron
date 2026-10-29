// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// The project browser's paths (was js/editor-ui/test/project.test.ts). The file operations
// themselves are the host's (`crates/platypus_engine_api/src/editor`, M5-DND-12, M5-EDT-27).

import { baseName, canMoveInto, crumbs, formatSize, joinPath, parentPath, projectFolder } from "../src/project.ts";
import { eq, report, same, test, yes } from "./check.ts";

// M5-DND-08
test("project paths join, split into breadcrumbs and refuse moves into themselves", () => {
  same(joinPath("", "a.png"), "a.png", "join at the root");
  same(joinPath("assets/ui", "a.png"), "assets/ui/a.png", "join");
  same(parentPath("assets/ui/a.png"), "assets/ui", "parent");
  same(parentPath("a.png"), "", "parent at the root");
  same(baseName("C:\\Games\\Arcade"), "Arcade", "base name");
  eq(
    crumbs("assets/ui", "Arcade"),
    [
      { name: "Arcade", path: "" },
      { name: "assets", path: "assets" },
      { name: "ui", path: "assets/ui" },
    ],
    "crumbs",
  );
  yes(canMoveInto("assets/a.png", ""), "up to the root");
  yes(!canMoveInto("assets/a.png", "assets"), "where it is");
  yes(!canMoveInto("assets", "assets/ui"), "into itself");
  yes(!canMoveInto("assets", "assets"), "onto itself");
  yes(canMoveInto("assets", "assets2"), "a sibling with the same start");
  same(formatSize(512), "512 B", "bytes");
  same(formatSize(1536), "1.5 KB", "KB");
  same(formatSize(12 * 1024 * 1024), "12 MB", "MB");
});

// M5-DND-08
test("without a project the file calls fail and say why", () => {
  yes(projectFolder() === null, "no project folder");
  eq(Editor.listDir("").length, 0, "nothing listed");
  yes(Editor.lastError.includes("--project"), `the error: ${Editor.lastError}`);
});

export function run(): string {
  return report();
}
