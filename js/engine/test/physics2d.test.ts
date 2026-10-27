// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

import { afterEach, beforeEach, describe, expect, test } from "bun:test";

import {
  BoxCollider2D,
  CircleCollider2D,
  EdgeCollider2D,
  ForceMode2D,
  GameObject,
  Physics2D,
  PolygonCollider2D,
  Rigidbody2D,
  Vector2,
  Vector3,
} from "../src/index.ts";
import { _resetPhysicsComponentsForTests } from "../src/physics-components.ts";
import { _resetPhysicsForTests } from "../src/physics.ts";
import { _resetPhysics2DForTests } from "../src/physics2d.ts";
import { _resetSceneForTests } from "../src/scene.ts";

let ops: unknown[][] = [];

beforeEach(() => {
  _resetSceneForTests();
  _resetPhysicsForTests();
  _resetPhysicsComponentsForTests();
  _resetPhysics2DForTests();
  ops = [];
  (globalThis as { __platypus_op?: unknown }).__platypus_op = (...args: unknown[]) => ops.push(args);
});

afterEach(() => {
  delete (globalThis as { __platypus_op?: unknown }).__platypus_op;
});

const colliderOps = () => ops.filter((o) => o[0] === "physics.collider");
const sets = () => ops.filter((o) => o[0] === "physics.set").map((o) => o.slice(2));

// M5-RB2D-02
describe("2D physics components", () => {
  test("2D colliders are slabs Physics2D.depth deep; circles turn to face z", () => {
    const go = new GameObject("thing", { scale: new Vector3(2, 1, 0) });
    go.addComponent(BoxCollider2D, { size: new Vector2(1, 3), offset: new Vector2(0.5, 0) });
    go.addComponent(CircleCollider2D, { radius: 0.25 });
    const [box, circle] = colliderOps();
    expect(box!.slice(3, 6)).toEqual(["box", [1, 1.5, 5], [1, 0, 0]]);
    expect(circle!.slice(3, 5)).toEqual(["cylinder", [5, 0.5]]);
    const q = circle![6] as number[];
    expect(Math.abs(q[0]!)).toBeCloseTo(Math.SQRT1_2, 5);
  });

  test("polygons become convex slabs and edges static meshes", () => {
    const go = new GameObject("shapes");
    go.addComponent(PolygonCollider2D, { points: [new Vector2(0, 0), new Vector2(1, 0), new Vector2(0, 1)] });
    go.addComponent(EdgeCollider2D, { points: [new Vector2(0, 0), new Vector2(2, 1), new Vector2(4, 1)] });
    const [poly, edge] = colliderOps();
    expect(poly![3]).toBe("convex");
    expect((poly![4] as number[]).length).toBe(3 * 2 * 3);
    expect(edge![3]).toBe("mesh");
    expect((edge![7] as unknown[])[5]).toEqual([0, 2, 1, 1, 2, 3, 2, 4, 3, 3, 4, 5]);
  });

  test("Rigidbody2D keeps to the plane and speaks Vector2 and degrees", () => {
    const go = new GameObject("ball");
    go.addComponent(CircleCollider2D);
    const rb = go.addComponent(Rigidbody2D, { gravityScale: 0.5, freezeRotation: true });
    expect(sets()).toContainEqual(["kind", "dynamic"]);
    expect(sets()).toContainEqual(["gravityScale", 0.5]);
    expect(sets()).toContainEqual(["constraints", [true, true, false, false, false, false]]);
    ops = [];
    rb.velocity = new Vector2(3, 4);
    rb.angularVelocity = 180;
    rb.addForce(new Vector2(0, 2), ForceMode2D.Impulse);
    rb.bodyType = "static";
    expect(ops).toEqual([
      ["physics.set", 1, "velocity", [3, 4, 0]],
      ["physics.set", 1, "angularVelocity", [0, 0, Math.PI]],
      ["physics.force", 1, "impulse", [0, 2, 0]],
      ["physics.set", 1, "kind", "fixed"],
    ]);
    // The engine's report: spin about z in degrees.
    globalThis.__platypus_physics([1, 0, 0, 0, 0, 0, 0, 1, 1, 2, 0, 0, 0, -Math.PI / 2], [], [], []);
    expect(rb.velocity).toEqual(new Vector2(1, 2));
    expect(rb.angularVelocity).toBeCloseTo(-90, 5);
  });

  test("Physics2D.gravity sets world gravity in the plane", () => {
    Physics2D.gravity = new Vector2(0, -20);
    expect(ops.at(-1)).toEqual(["physics.gravity", [0, -20, 0]]);
  });
});
