// SPDX-License-Identifier: Apache-2.0

import { afterEach, beforeEach, describe, expect, spyOn, test } from "bun:test";

import { type CollisionEvent, physics } from "../src/index.ts";
import { _resetPhysicsForTests } from "../src/physics.ts";

let ops: unknown[][] = [];

beforeEach(() => {
  _resetPhysicsForTests();
  ops = [];
  (globalThis as { __iron_op?: unknown }).__iron_op = (...args: unknown[]) => ops.push(args);
});

afterEach(() => {
  delete (globalThis as { __iron_op?: unknown }).__iron_op;
});

const tick = (moved: number[], contacts: number[] = [], rays: number[] = [], characters: number[] = []) =>
  globalThis.__iron_physics(moved, contacts, rays, characters);

describe("physics", () => {
  test("bodies and colliders become ops with IDs chosen in script", () => {
    const body = physics.createBody({ position: { x: 0, y: 3, z: 0 }, lockRotations: true });
    const col = body.addCollider({ shape: { box: { x: 0.5, y: 0.5, z: 0.5 } }, restitution: 0.2 });
    physics.addCollider({ shape: { mesh: { points: [{ x: 0, y: 0, z: 0 }], indices: [0, 0, 0] } } });
    expect(ops).toEqual([
      ["physics.body", 1, "dynamic", [0, 3, 0], [0, 0, 0, 1], [0, 0, 0], [1, 0, 0, false, true]],
      ["physics.collider", 1, 1, "box", [0.5, 0.5, 0.5], null, null, [0.5, 0.2, 1, false, true, null]],
      ["physics.collider", 2, null, "mesh", [0, 0, 0], null, null, [0.5, 0, 1, false, true, [0, 0, 0]]],
    ]);
    expect(col.body).toBe(body);
    body.applyImpulse({ x: 0, y: 5, z: 0 });
    body.remove();
    body.remove();
    expect(ops.slice(3)).toEqual([["physics.force", 1, "impulse", [0, 5, 0]], ["physics.removeBody", 1]]);
  });

  test("each tick's batch updates poses and character grounding", () => {
    const a = physics.createBody();
    const b = physics.createBody({ kind: "kinematic" });
    tick([1, 1, 2, 3, 0, 0, 0, 1, 0, -9.8, 0, 0, 0, 2], [], [], [2, 1]);
    expect(a.position).toEqual({ x: 1, y: 2, z: 3 });
    expect(a.velocity.y).toBe(-9.8);
    expect(a.angularVelocity.z).toBe(2);
    expect(b.grounded).toBe(true);
    // Unknown ids (removed since) are skipped.
    tick([99, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0]);
  });

  // M4-PHY-02 (TS side)
  test("collision start and end reach both bodies' handlers", () => {
    const ball = physics.createBody();
    const ballCol = ball.addCollider({ shape: { ball: 0.25 } });
    const zone = physics.createBody({ kind: "fixed" });
    const zoneCol = zone.addCollider({ shape: { box: { x: 1, y: 1, z: 1 } }, sensor: true });
    const floor = physics.addCollider({ shape: { box: { x: 10, y: 0.5, z: 10 } } });
    const log: string[] = [];
    const name = (e: CollisionEvent) => `${e.collider.id}-${e.other.id}${e.sensor ? " sensor" : ""} ${e.speed}`;
    ball.onCollisionStart = (e) => log.push(`ball start ${name(e)}`);
    ball.onCollisionEnd = (e) => log.push(`ball end ${name(e)}`);
    zone.onCollisionStart = () => {
      throw new Error("boom");
    };
    const error = spyOn(console, "error").mockImplementation(() => {});
    tick([], [1, ballCol.id, zoneCol.id, 1, 3, 0, ballCol.id, zoneCol.id, 1, 0, 1, floor.id, ballCol.id, 0, 7.5]);
    expect(log).toEqual([`ball start 1-2 sensor 3`, `ball end 1-2 sensor 0`, `ball start 1-3 7.5`]);
    // A throwing handler is reported and the batch carries on.
    expect(error).toHaveBeenCalledTimes(1);
    error.mockRestore();
  });

  test("raycasts resolve after the tick with the hit point", async () => {
    const floor = physics.addCollider({ shape: { box: { x: 10, y: 0.5, z: 10 } } });
    const hit = physics.raycast({ x: 0, y: 5, z: 0 }, { x: 0, y: -2, z: 0 });
    const miss = physics.raycast({ x: 0, y: 5, z: 0 }, { x: 0, y: 1, z: 0 }, 10);
    expect(ops.at(-2)).toEqual(["physics.raycast", 1, [0, 5, 0], [0, -1, 0], 1000]);
    tick([], [], [1, floor.id, 4.5, 0, 1, 0, 2, -1, 0, 0, 0, 0]);
    const h = await hit;
    expect(h?.collider).toBe(floor);
    expect(h?.point).toEqual({ x: 0, y: 0.5, z: 0 });
    expect(await miss).toBeNull();
  });
});
