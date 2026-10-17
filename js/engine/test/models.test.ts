import { afterEach, beforeEach, describe, expect, test } from "bun:test";

import { GameObject, MeshCollider, MeshRenderer, Model } from "../src/index.ts";
import { _resetModelsForTests } from "../src/models.ts";
import { _resetPhysicsComponentsForTests } from "../src/physics-components.ts";
import { _resetPhysicsForTests } from "../src/physics.ts";
import { _resetSceneForTests } from "../src/scene.ts";

let ops: unknown[][] = [];

beforeEach(() => {
  _resetSceneForTests();
  _resetPhysicsForTests();
  _resetPhysicsComponentsForTests();
  _resetModelsForTests();
  ops = [];
  (globalThis as { __iron_op?: unknown }).__iron_op = (...args: unknown[]) => ops.push(args);
});

afterEach(() => {
  delete (globalThis as { __iron_op?: unknown }).__iron_op;
});

const E = 2 ** 30;
const house = {
  roots: [0],
  materials: [
    { name: "Red", color: [0.8, 0.2, 0.1, 1], emission: [0, 0, 0], smoothness: 0.6, unlit: false, texture: null },
    { name: "Checker", color: [1, 1, 1, 1], emission: [0, 0, 0], smoothness: 0, unlit: false, texture: [E + 4, 4, 4] },
  ],
  nodes: [
    { name: "House", position: [0, 0, 0], rotation: [0, 0, 0, 1], scale: [1, 1, 1], children: [1], primitives: [{ mesh: E + 1, material: 1, bounds: [[-0.5, -0.5, -0.5], [0.5, 0.5, 0.5]] }] },
    {
      name: "Roof",
      position: [0, 0.5, 0],
      rotation: [0, 0, 0, 1],
      scale: [1, 1, 1],
      children: [],
      primitives: [
        { mesh: E + 2, material: 0, bounds: [[0, 0, 0], [1, 1, 1]] },
        { mesh: E + 3, material: null, bounds: [[0, 0, 0], [1, 1, 1]] },
      ],
    },
  ],
};

// M5-GLB-02
describe("Model", () => {
  test("loads once per path and builds its nodes as GameObjects, even when placed early", async () => {
    const m = Model.load("assets/house.glb");
    expect(Model.load("assets/house.glb")).toBe(m);
    expect(ops.filter((o) => o[0] === "render.model")).toEqual([["render.model", 1, "assets/house.glb"]]);
    // Placed before the engine answers: the nodes arrive with the answer.
    const early = m.instantiate({ colliders: "mesh" });
    expect(early.children.length).toBe(0);
    const ready = m.whenLoaded();
    globalThis.__iron_models([m.id, JSON.stringify(house)]);
    expect(await ready).toBe(m);
    const top = early.children[0]!;
    expect(top.name).toBe("House");
    const roof = top.children[0]!;
    expect(roof.transform.localPosition.y).toBe(0.5);
    // Two parts: one child object each, with its material (none: the default).
    expect(roof.children.map((c) => c.name)).toEqual(["Roof 0", "Roof 1"]);
    const r0 = roof.children[0]!.getComponent(MeshRenderer)!;
    expect(r0.material.color.r).toBeCloseTo(0.8);
    const r = top.getComponent(MeshRenderer)!;
    expect(r.material.mainTexture!.id).toBe(E + 4);
    expect(r.material.mainTexture!.width).toBe(4);
    // Mesh colliders take the render mesh, which the engine resolves.
    expect(top.getComponent(MeshCollider)!.sharedMesh!.id).toBe(E + 1);
    const collider = ops.find((o) => o[0] === "physics.collider" && o[3] === "renderMesh")!;
    expect(collider[4]).toEqual([E + 1, 0, 1, 1, 1]);
  });

  test("a failed load rejects and places nothing", async () => {
    const m = Model.load("assets/missing.glb");
    const go = m.instantiate();
    const ready = m.whenLoaded();
    globalThis.__iron_models([m.id, null]);
    await expect(ready).rejects.toThrow("missing.glb");
    expect(go.children.length).toBe(0);
    expect(go).toBeInstanceOf(GameObject);
  });
});
