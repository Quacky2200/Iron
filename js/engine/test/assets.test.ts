// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

import { expect, test } from "bun:test";

import { Assets, GameObject, Material, MeshRenderer, SceneManager, Texture, audio } from "../src/index.ts";

// M5-AST-02
test("Assets.onChanged hears changed files until stopped; a throwing listener doesn't stop the others", () => {
  const heard: string[][] = [];
  const stop = Assets.onChanged((p) => heard.push(p));
  Assets.onChanged(() => {
    throw new Error("boom");
  });
  const error = console.error;
  console.error = () => {};
  try {
    globalThis.__platypus_assets_changed(["assets/crate.png"]);
    stop();
    globalThis.__platypus_assets_changed(["assets/other.png"]);
  } finally {
    console.error = error;
  }
  expect(heard).toEqual([["assets/crate.png"]]);
});

// M9-API-1619
test("Assets.exists / list / packOf hand over to the engine's Assets namespace, and answer as an empty game without it", () => {
  const g = globalThis as { Assets?: unknown };
  const before = g.Assets;
  try {
    delete g.Assets;
    expect([Assets.exists("a.png"), Assets.list("**"), Assets.packOf("a.png")]).toEqual([false, [], ""]);
    g.Assets = {
      exists: (p: string) => p === "assets/a.png",
      list: (glob: string) => (glob === "assets/*" ? ["assets/a.png", "assets/b.png"] : []),
      packOf: () => "assets",
    };
    expect(Assets.exists("assets/a.png")).toBe(true);
    expect(Assets.exists("nope")).toBe(false);
    expect(Assets.list("assets/*")).toEqual(["assets/a.png", "assets/b.png"]);
    expect(Assets.packOf("assets/a.png")).toBe("assets");
  } finally {
    if (before === undefined) delete g.Assets;
    else g.Assets = before;
  }
});

// M5-AST-05
test("GUIDs stand in for paths, importers give texture defaults, and scenes find renamed files by GUID", () => {
  const ops: unknown[][] = [];
  (globalThis as { __platypus_op?: unknown }).__platypus_op = (...args: unknown[]) => ops.push(args);
  const info = globalThis.__platypus_info as Record<string, unknown> | undefined;
  const setDb = (assets: unknown[]) => {
    (globalThis as { __platypus_info?: unknown }).__platypus_info = { ...(info ?? {}), assets };
  };
  try {
    setDb([
      { path: "assets/crate.png", guid: "c0ffee", importer: { type: "texture", filterMode: "point", wrapMode: "clamp" } },
      { path: "assets/theme.ogg", guid: "50da", importer: { type: "audio", stream: true } },
    ]);
    expect(Assets.guidToPath("c0ffee")).toBe("assets/crate.png");
    expect(Assets.pathToGuid("./assets/crate.png")).toBe("c0ffee");
    expect(Assets.pathToGuid("nope.png")).toBeUndefined();
    // By GUID, with the importer's settings; given options win.
    const t = Texture.load("guid:c0ffee");
    expect(t.src).toBe("assets/crate.png");
    expect([t.filterMode, t.wrapMode]).toEqual(["point", "clamp"]);
    expect(Texture.load("assets/crate.png", { filterMode: "trilinear" }).filterMode).toBe("trilinear");
    // The importer can ask for a streamed sound.
    audio.load("guid:50da");
    expect(ops.find((o) => o[0] === "audio.load")!.slice(2)).toEqual(["assets/theme.ogg", true]);
    // Scenes keep the GUID; a renamed file is still found.
    const go = new GameObject("Crate");
    go.addComponent(MeshRenderer, { mesh: "cube", material: Material.create({ mainTexture: t }) });
    const data = JSON.parse(JSON.stringify(SceneManager.getActiveScene().serialize()));
    expect(Object.values(data.assets).some((a) => (a as { guid?: string }).guid === "c0ffee")).toBe(true);
    setDb([{ path: "assets/props/crate.png", guid: "c0ffee", importer: {} }]);
    SceneManager.loadScene(data);
    const material = GameObject.find("Crate")!.getComponent(MeshRenderer)!.material;
    expect(material.mainTexture!.src).toBe("assets/props/crate.png");
  } finally {
    (globalThis as { __platypus_info?: unknown }).__platypus_info = info;
    delete (globalThis as { __platypus_op?: unknown }).__platypus_op;
  }
});
