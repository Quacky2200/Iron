import { afterEach, beforeEach, describe, expect, test } from "bun:test";

import { Color, GameObject, Material, MeshRenderer, Rect, Sprite, SpriteRenderer, Texture, Vector2 } from "../src/index.ts";
import { _resetPhysicsComponentsForTests } from "../src/physics-components.ts";
import { _resetPhysicsForTests } from "../src/physics.ts";
import { _resetSceneForTests } from "../src/scene.ts";
import { _resetTexturesForTests } from "../src/textures.ts";

let ops: unknown[][] = [];

beforeEach(() => {
  _resetSceneForTests();
  _resetPhysicsForTests();
  _resetPhysicsComponentsForTests();
  _resetTexturesForTests();
  ops = [];
  (globalThis as { __iron_op?: unknown }).__iron_op = (...args: unknown[]) => ops.push(args);
});

afterEach(() => {
  delete (globalThis as { __iron_op?: unknown }).__iron_op;
});

const renderOps = () => ops.filter((o) => String(o[0]).startsWith("render."));

// M5-TEX-01
describe("textures", () => {
  test("load sends the file and settings; the engine's answer fills in the size", async () => {
    const t = Texture.load("assets/crate.png", { filterMode: "point" });
    expect(renderOps()).toEqual([["render.texture", 1, "assets/crate.png", "point", "repeat", true]]);
    expect(t.isLoaded).toBe(false);
    const ready = t.whenLoaded();
    globalThis.__iron_textures([[1, 64, 32]]);
    expect(await ready).toBe(t);
    expect([t.width, t.height, t.isLoaded]).toEqual([64, 32, true]);
    t.wrapMode = "clamp";
    expect(renderOps().at(-1)).toEqual(["render.texture", 1, "assets/crate.png", "point", "clamp", true]);
    t.destroy();
    expect(renderOps().at(-1)).toEqual(["render.textureRemove", 1]);
  });

  test("a failed load rejects whenLoaded", async () => {
    const t = Texture.load("assets/missing.png");
    const ready = t.whenLoaded();
    globalThis.__iron_textures([[t.id, 0, 0]]);
    expect(t.failed).toBe(true);
    await expect(ready).rejects.toThrow("missing.png");
  });

  test("create takes linear colours bottom row first and sends sRGB bytes top row first", () => {
    const red = new Color(1, 0, 0, 1);
    const grey = new Color(0.5, 0.5, 0.5, 0.5);
    Texture.create(1, 2, [red, grey], { mipmaps: false });
    // Linear 0.5 is sRGB 188.
    expect(renderOps()[0]).toEqual(["render.texture", 1, null, "bilinear", "repeat", false, 1, 2, [188, 188, 188, 128, 255, 0, 0, 255]]);
  });

  test("materials carry a texture and tiling to the MeshRenderer", () => {
    const t = Texture.load("assets/bricks.png");
    const go = new GameObject("wall");
    const r = go.addComponent(MeshRenderer, { mesh: "cube", material: new Material().withTexture(t, new Vector2(2, 3), new Vector2(0.5, 0)) });
    expect(renderOps().at(-1)).toEqual(["render.meshRenderer", r.id, go.id, "cube", [1, 1, 1, 1], [0, 0, 0], 0.5, false, t.id, [2, 3, 0.5, 0]]);
    // withColor keeps the texture.
    r.material = r.material.withColor(Color.white);
    expect(renderOps().at(-1)![8]).toBe(t.id);
  });
});

// M5-TEX-02
describe("sprites", () => {
  test("SpriteRenderer sends its sprite; clearing it removes the quad", () => {
    const sheet = Texture.load("assets/hero.png", { filterMode: "point" });
    const go = new GameObject("hero");
    const r = go.addComponent(SpriteRenderer, {
      sprite: Sprite.create(sheet, new Rect(16, 0, 16, 16), new Vector2(0.5, 0), 16),
      flipX: true,
    });
    expect(renderOps().at(-1)).toEqual(["render.sprite", r.id, go.id, sheet.id, [16, 0, 16, 16], [0.5, 0], 16, [1, 1, 1, 1], true, false]);
    r.sprite = null;
    expect(renderOps().at(-1)).toEqual(["render.remove", r.id]);
  });

  test("grid cuts a sheet left to right, top to bottom; size is in units", () => {
    const sheet = Texture.load("assets/sheet.png");
    globalThis.__iron_textures([[sheet.id, 64, 32]]);
    const cells = Sprite.grid(sheet, 4, 2, undefined, 32);
    expect(cells.length).toBe(8);
    // The first cell is the top-left: y counts up from the bottom.
    expect(cells[0]!.rect).toEqual(new Rect(0, 16, 16, 16));
    expect(cells[5]!.rect).toEqual(new Rect(16, 0, 16, 16));
    expect(cells[0]!.size).toEqual(new Vector2(0.5, 0.5));
    expect(Sprite.create(sheet).size).toEqual(new Vector2(0.64, 0.32));
  });
});
