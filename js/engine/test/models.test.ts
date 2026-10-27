// SPDX-License-Identifier: Apache-2.0

import { afterEach, beforeEach, describe, expect, test } from "bun:test";

import { Animation, GameObject, Matrix4x4, Mesh, MeshCollider, MeshRenderer, Model, Quaternion, SkinnedMeshRenderer, Vector3 } from "../src/index.ts";
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
  (globalThis as { __platypus_op?: unknown }).__platypus_op = (...args: unknown[]) => ops.push(args);
});

afterEach(() => {
  delete (globalThis as { __platypus_op?: unknown }).__platypus_op;
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
    globalThis.__platypus_models([m.id, JSON.stringify(house)]);
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
    globalThis.__platypus_models([m.id, null]);
    await expect(ready).rejects.toThrow("missing.glb");
    expect(go.children.length).toBe(0);
    expect(go).toBeInstanceOf(GameObject);
  });
});

// M5-SKN-02
describe("skinned models and animations", () => {
  test("skinned parts get a SkinnedMeshRenderer with their bones; animations become clips", () => {
    const ibm = (y: number) => [1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, -y, 0, 1];
    const worm = {
      roots: [0],
      materials: [],
      nodes: [
        { name: "Worm", position: [0, 0, 0], rotation: [0, 0, 0, 1], scale: [1, 1, 1], children: [1, 3], primitives: [] },
        { name: "Bone0", position: [0, 0, 0], rotation: [0, 0, 0, 1], scale: [1, 1, 1], children: [2], primitives: [] },
        { name: "Bone1", position: [0, 1, 0], rotation: [0, 0, 0, 1], scale: [1, 1, 1], children: [], primitives: [] },
        { name: "Body", position: [0, 0, 0], rotation: [0, 0, 0, 1], scale: [1, 1, 1], children: [], skin: 0, primitives: [{ mesh: E + 1, material: null, bounds: [[0, 0, 0], [1, 1, 1]] }] },
      ],
      skins: [{ joints: [1, 2], inverseBind: [...ibm(0), ...ibm(1)] }],
      animations: [
        {
          name: "bend",
          channels: [
            { node: 2, path: "rotation", interpolation: "linear", times: [0, 1], values: [0, 0, 0, 1, 0, 0, 0.2588, 0.9659] },
            { node: 0, path: "translation", interpolation: "step", times: [0, 1], values: [0, 0, 0, 2, 0, 0] },
          ],
        },
      ],
    };
    const m = Model.load("worm.glb");
    const root = m.instantiate();
    globalThis.__platypus_models([m.id, JSON.stringify(worm)]);
    const body = root.findChild("Worm")!.findChild("Body")!;
    const skinned = body.getComponent(SkinnedMeshRenderer)!;
    const bones = skinned.bones.map((b) => b.name);
    expect(bones).toEqual(["Bone0", "Bone1"]);
    const op = ops.filter((o) => o[0] === "render.meshRenderer").at(-1)!;
    expect(op[13]).toEqual(skinned.bones.map((b) => b.id));
    expect((op[14] as number[]).length).toBe(32);
    expect((op[14] as number[])[16 + 13]).toBe(-1);
    // The animation: a clip with paths from the copy's root, playing on its own.
    expect(m.animations.map((c) => c.name)).toEqual(["bend"]);
    const clip = m.animations[0]!;
    expect(clip.curves.map((c) => `${c.path}:${c.property}`)).toContain("Worm/Bone0/Bone1:localRotation.z");
    const anim = root.getComponent(Animation)!;
    expect(anim.clip).toBe(clip);
    anim.play();
    globalThis.__platypus_animate(0.5);
    const bone1 = root.findChild("Worm")!.findChild("Bone0")!.findChild("Bone1")!;
    expect(bone1.transform.localRotation.z).toBeCloseTo(0.2588 / 2, 2);
    // Step keys hold until the next.
    expect(root.findChild("Worm")!.transform.localPosition.x).toBe(0);
    clip.wrapMode = "clampForever";
    clip.sampleAnimation(root, 1);
    expect(root.findChild("Worm")!.transform.localPosition.x).toBe(2);
  });

  // M5-SKN-03
  test("rotation keys on opposite sides (q and -q) are flipped to agree, so poses don't fly", () => {
    // A quarter turn about z, then the same turn stored negated, then a little further.
    const s = Math.SQRT1_2;
    const arm = {
      roots: [0],
      materials: [],
      nodes: [{ name: "Arm", position: [0, 0, 0], rotation: [0, 0, 0, 1], scale: [1, 1, 1], children: [], primitives: [] }],
      animations: [
        {
          name: "swing",
          channels: [{ node: 0, path: "rotation", interpolation: "linear", times: [0, 1, 2], values: [0, 0, s, s, 0, 0, -s, -s, 0, 0, -0.7934, -0.6088] }],
        },
      ],
    };
    const m = Model.load("arm.glb");
    const root = m.instantiate();
    globalThis.__platypus_models([m.id, JSON.stringify(arm)]);
    const clip = m.animations[0]!;
    clip.wrapMode = "clampForever";
    // Halfway between the first two keys: still the quarter turn (it summed to zero before,
    // and normalizing nothing gave any rotation at all).
    clip.sampleAnimation(root, 0.5);
    const q = root.findChild("Arm")!.transform.localRotation;
    expect(Math.abs(q.z)).toBeCloseTo(s, 3);
    expect(Math.abs(q.w)).toBeCloseTo(s, 3);
    // And on towards the third key, the same way round.
    clip.sampleAnimation(root, 1.5);
    const r = root.findChild("Arm")!.transform.localEulerAngles.z;
    expect(r).toBeGreaterThan(90);
    expect(r).toBeLessThan(110);
  });

  test("script meshes carry bone weights and bind poses", () => {
    const mesh = Mesh.create({
      vertices: [Vector3.zero, Vector3.right, Vector3.up],
      triangles: [0, 1, 2],
      boneWeights: [
        { indices: [0, 1, 0, 0], weights: [1, 1, 0, 0] },
        { indices: [0, 0, 0, 0], weights: [1, 0, 0, 0] },
        { indices: [1, 0, 0, 0], weights: [2, 0, 0, 0] },
      ],
      bindposes: [Matrix4x4.identity, Matrix4x4.trs(new Vector3(0, -1, 0), Quaternion.identity)],
    });
    const op = ops.find((o) => o[0] === "render.mesh")!;
    expect(op[6]).toEqual([0, 1, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0]);
    // Weights sum to 1.
    expect(op[7]).toEqual([0.5, 0.5, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0]);
    expect(mesh.bindposes.length).toBe(2);
    const a = new GameObject("A");
    const b = new GameObject("B");
    const r = new GameObject("Skin").addComponent(SkinnedMeshRenderer, { mesh, bones: [a, b] });
    expect(ops.filter((o) => o[0] === "render.meshRenderer").at(-1)![13]).toEqual([a.id, b.id]);
    expect(r.bones.length).toBe(2);
  });
});
