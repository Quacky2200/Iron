import { afterEach, beforeEach, expect, test } from "bun:test";
import { existsSync, mkdirSync, mkdtempSync, renameSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { readMeta, scan, update } from "./assets.ts";

let dir = "";
beforeEach(() => {
  dir = mkdtempSync(join(tmpdir(), "iron-meta-"));
});
afterEach(() => {
  rmSync(dir, { recursive: true, force: true });
});

// M5-AST-03
test("every asset gets a .meta with a GUID and importer settings; moves keep the GUID", () => {
  mkdirSync(join(dir, "assets/textures"), { recursive: true });
  mkdirSync(join(dir, "src"));
  mkdirSync(join(dir, "dist"));
  writeFileSync(join(dir, "assets/textures/crate.png"), "png");
  writeFileSync(join(dir, "assets/worm.glb"), "glb");
  writeFileSync(join(dir, "src/main.tsx"), "code");
  writeFileSync(join(dir, "dist/game.js"), "build");
  writeFileSync(join(dir, "package.json"), "{}");
  expect(scan(dir).assets).toEqual(["assets/textures/crate.png", "assets/worm.glb"]);

  const first = update(dir);
  expect(first.created).toEqual(["assets/textures/crate.png", "assets/worm.glb"]);
  const crate = readMeta(join(dir, "assets/textures/crate.png.meta"))!;
  expect(crate.guid).toMatch(/^[0-9a-f]{32}$/);
  expect(crate.importer).toMatchObject({ type: "texture", filterMode: "bilinear", mipmaps: true });
  expect(readMeta(join(dir, "assets/worm.glb.meta"))!.importer.type).toBe("model");
  // Running again changes nothing.
  expect(update(dir)).toEqual({ created: [], moved: [], orphans: [] });

  // The file moves without its meta: the meta follows, keeping the GUID.
  mkdirSync(join(dir, "assets/props"));
  renameSync(join(dir, "assets/textures/crate.png"), join(dir, "assets/props/crate.png"));
  const moved = update(dir);
  expect(moved.moved).toEqual([["assets/textures/crate.png", "assets/props/crate.png"]]);
  expect(readMeta(join(dir, "assets/props/crate.png.meta"))!.guid).toBe(crate.guid);

  // A deleted asset leaves an orphan, kept unless pruning.
  rmSync(join(dir, "assets/worm.glb"));
  expect(update(dir).orphans).toEqual(["assets/worm.glb.meta"]);
  expect(existsSync(join(dir, "assets/worm.glb.meta"))).toBe(true);
  update(dir, { prune: true });
  expect(existsSync(join(dir, "assets/worm.glb.meta"))).toBe(false);
});
