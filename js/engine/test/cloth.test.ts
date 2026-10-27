// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

import { afterEach, beforeEach, expect, test } from "bun:test";

import { Cloth, GameObject, Mesh, MeshRenderer, SphereCollider, Vector3 } from "../src/index.ts";
import { _resetPhysicsComponentsForTests } from "../src/physics-components.ts";
import { _resetPhysicsForTests } from "../src/physics.ts";
import { _resetSceneForTests } from "../src/scene.ts";

let ops: unknown[][] = [];

beforeEach(() => {
  _resetSceneForTests();
  _resetPhysicsForTests();
  _resetPhysicsComponentsForTests();
  ops = [];
  (globalThis as { __platypus_op?: unknown }).__platypus_op = (...args: unknown[]) => ops.push(args);
});

afterEach(() => {
  delete (globalThis as { __platypus_op?: unknown }).__platypus_op;
});

// M5-CLO-03
test("Cloth simulates a copy of its renderer's mesh with its settings, pins and colliders", () => {
  const ball = new GameObject("Ball", { position: new Vector3(0, 1, 0) });
  ball.addComponent(SphereCollider, { radius: 0.4 });
  const sheet = new GameObject("Sheet", { position: new Vector3(0, 2, 0) });
  const renderer = sheet.addComponent(MeshRenderer, { mesh: "plane" });
  const cloth = sheet.addComponent(Cloth, {
    bendingStiffness: 0.1,
    externalAcceleration: new Vector3(2, 0, 0),
    pinBox: { min: new Vector3(-5, -1, -5), max: new Vector3(-4.9, 1, 5) },
    sphereColliders: [ball],
  });
  cloth.onStart!();
  const op = ops.find((o) => o[0] === "cloth.cloth")!;
  expect(op.slice(1, 4)).toEqual([cloth.id, sheet.id, "plane"]);
  const simulated = renderer.mesh as Mesh;
  expect(simulated).toBeInstanceOf(Mesh);
  expect(op[4]).toBe(simulated.id);
  expect(op[5]).toEqual([1, 0.1, 0.1, 0.5, 8]);
  expect(op[6]).toEqual([2, 0, 0]);
  expect(op[11]).toEqual([-5, -1, -5, -4.9, 1, 5]);
  expect(op[12]).toEqual([["sphere", ball.id, 0.4, [0, 0, 0]]]);
  // Removing it gives the renderer its mesh back and frees the copy.
  sheet.removeComponent(cloth);
  // The engine calls onDestroy for removed components.
  cloth.onDestroy!();
  expect(renderer.mesh).toBe("plane");
  expect(ops.some((o) => o[0] === "cloth.remove" && o[1] === cloth.id)).toBe(true);
  expect(ops.some((o) => o[0] === "render.mesh" && o[1] === simulated.id && o[2] === null)).toBe(true);
});
