// SPDX-License-Identifier: Apache-2.0

import { beforeEach, expect, test } from "bun:test";

import {
  _resetProjectForTests,
  baseName,
  canMoveInto,
  copyIn,
  crumbs,
  formatSize,
  joinPath,
  listDir,
  moveInto,
  parentPath,
} from "../src/project.ts";

let ops: unknown[][] = [];
beforeEach(() => {
  ops = [];
  (globalThis as { __platypus_op?: unknown }).__platypus_op = (...args: unknown[]) => ops.push(args);
  _resetProjectForTests();
});

// M5-DND-08
test("project paths join, split into breadcrumbs and refuse moves into themselves", () => {
  expect(joinPath("", "a.png")).toBe("a.png");
  expect(joinPath("assets/ui", "a.png")).toBe("assets/ui/a.png");
  expect(parentPath("assets/ui/a.png")).toBe("assets/ui");
  expect(parentPath("a.png")).toBe("");
  expect(baseName("C:\\Games\\Arcade")).toBe("Arcade");
  expect(crumbs("assets/ui", "Arcade")).toEqual([
    { name: "Arcade", path: "" },
    { name: "assets", path: "assets" },
    { name: "ui", path: "assets/ui" },
  ]);
  expect(canMoveInto("assets/a.png", "")).toBe(true);
  expect(canMoveInto("assets/a.png", "assets")).toBe(false);
  expect(canMoveInto("assets", "assets/ui")).toBe(false);
  expect(canMoveInto("assets", "assets")).toBe(false);
  expect(canMoveInto("assets", "assets2")).toBe(true);
  expect(formatSize(512)).toBe("512 B");
  expect(formatSize(1536)).toBe("1.5 KB");
  expect(formatSize(12 * 1024 * 1024)).toBe("12 MB");
});

// M5-DND-08
test("file ops are requests answered through __platypus_editor_fs", async () => {
  const listing = listDir("assets");
  const copied = copyIn(["C:/in/a.png"], "assets");
  const moved = moveInto("a.png", "../x");
  expect(ops).toEqual([
    ["editor.listDir", 1, "assets"],
    ["editor.copyIn", 2, ["C:/in/a.png"], "assets"],
    ["editor.move", 3, "a.png", "../x"],
  ]);
  globalThis.__platypus_editor_fs([
    1,
    true,
    JSON.stringify({ path: "assets", entries: [{ name: "ui", dir: true, size: 0 }] }),
    2,
    true,
    JSON.stringify({ created: ["assets/a.png"] }),
    3,
    false,
    "\"../x\" is outside the project",
  ]);
  expect((await listing).entries[0]?.name).toBe("ui");
  expect((await copied).created).toEqual(["assets/a.png"]);
  await expect(moved).rejects.toThrow("outside the project");
});
