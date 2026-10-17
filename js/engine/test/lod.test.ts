import { afterEach, beforeEach, describe, expect, test } from "bun:test";

import { GameObject, LODGroup, MeshRenderer } from "../src/index.ts";
import { _resetSceneForTests } from "../src/scene.ts";

let ops: unknown[][] = [];

beforeEach(() => {
  _resetSceneForTests();
  ops = [];
  (globalThis as { __iron_op?: unknown }).__iron_op = (...args: unknown[]) => ops.push(args);
});

afterEach(() => {
  delete (globalThis as { __iron_op?: unknown }).__iron_op;
});

// M5-LOD-02
describe("LODGroup", () => {
  test("sends its levels as renderer ids and transition heights", () => {
    const tree = new GameObject("tree");
    const detailed = new GameObject("detailed", { parent: tree }).addComponent(MeshRenderer, { mesh: "sphere" });
    const simple = new GameObject("simple", { parent: tree }).addComponent(MeshRenderer, { mesh: "cube" });
    const group = tree.addComponent(LODGroup, { size: 4 });
    const last = () => ops.filter((o) => o[0] === "render.lodGroup").at(-1)!;
    group.setLODs([
      { screenRelativeTransitionHeight: 0.4, renderers: [detailed] },
      { screenRelativeTransitionHeight: 0.05, renderers: [simple] },
    ]);
    expect(last()).toEqual(["render.lodGroup", group.id, tree.id, 4, [0.4, 0.05], [[detailed.id], [simple.id]], -1]);
    expect(group.lodCount).toBe(2);
    // Copies: changing them changes nothing.
    group.getLODs()[0]!.renderers.length = 0;
    expect(group.getLODs()[0]!.renderers).toEqual([detailed]);
    group.forceLOD(1);
    expect(last().at(-1)).toBe(1);
    expect(() => group.setLODs([
      { screenRelativeTransitionHeight: 0.1, renderers: [] },
      { screenRelativeTransitionHeight: 0.5, renderers: [] },
    ])).toThrow("go down");
  });
});
