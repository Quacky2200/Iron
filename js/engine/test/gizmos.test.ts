// SPDX-License-Identifier: Apache-2.0

import { afterEach, beforeEach, describe, expect, test } from "bun:test";

import { Camera, Color, GameObject, Gizmos, Matrix4x4, Quaternion, Vector3 } from "../src/index.ts";
import { _resetSceneForTests } from "../src/scene.ts";

let ops: unknown[][] = [];

beforeEach(() => {
  _resetSceneForTests();
  ops = [];
  (globalThis as { __iron_op?: unknown }).__iron_op = (...args: unknown[]) => ops.push(args);
  Gizmos.clear();
  Gizmos.color = Color.white;
  Gizmos.matrix = Matrix4x4.identity;
});

afterEach(() => {
  delete (globalThis as { __iron_op?: unknown }).__iron_op;
});

/** The presented lines as [a, b, colour] triples. */
function presented(): [number[], number[], number[]][] {
  const op = ops.findLast((o) => o[0] === "render.gizmos")!;
  const flat = op[2] as number[];
  const out: [number[], number[], number[]][] = [];
  for (let i = 0; i < flat.length; i += 10) out.push([flat.slice(i, i + 3), flat.slice(i + 3, i + 6), flat.slice(i + 6, i + 10)]);
  return out;
}

// M5-RND-23
describe("Gizmos", () => {
  test("lines go through the matrix in the current colour and reach one camera on present", () => {
    const camera = new GameObject("cam").addComponent(Camera);
    Gizmos.color = new Color(1, 0, 0, 0.5);
    Gizmos.matrix = Matrix4x4.trs(new Vector3(10, 0, 0), Quaternion.identity);
    Gizmos.drawLine(Vector3.zero, Vector3.up);
    Gizmos.drawRay(Vector3.zero, new Vector3(0, 0, -2));
    expect(Gizmos.lineCount).toBe(2);
    Gizmos.present(camera);
    expect(ops.at(-1)![1]).toBe(camera.id);
    expect(presented()).toEqual([
      [[10, 0, 0], [10, 1, 0], [1, 0, 0, 0.5]],
      [[10, 0, 0], [10, 0, -2], [1, 0, 0, 0.5]],
    ]);
    // Presenting starts over: nothing drawn clears the camera's lines.
    Gizmos.present(camera);
    expect(presented()).toEqual([]);
  });

  test("shapes: a cube's 12 edges, a sphere's three circles, a frustum's 12 edges along -z", () => {
    const camera = new GameObject("cam").addComponent(Camera);
    Gizmos.drawWireCube(Vector3.zero, new Vector3(2, 4, 6));
    Gizmos.present(camera);
    const cube = presented();
    expect(cube.length).toBe(12);
    for (const [a, b] of cube) {
      for (const p of [a, b]) expect(p.map(Math.abs)).toEqual([1, 2, 3]);
    }
    Gizmos.drawWireSphere(new Vector3(0, 1, 0), 2);
    Gizmos.present(camera);
    const sphere = presented();
    expect(sphere.length).toBe(3 * 48);
    for (const [a] of sphere) expect(Math.hypot(a[0]!, a[1]! - 1, a[2]!)).toBeCloseTo(2);
    Gizmos.drawFrustum(Vector3.zero, 90, 10, 1, 2);
    Gizmos.present(camera);
    const frustum = presented();
    expect(frustum.length).toBe(12);
    const zs = new Set(frustum.flatMap(([a, b]) => [a[2], b[2]]));
    expect([...zs].sort((x, y) => x! - y!)).toEqual([-10, -1]);
    // The far rectangle: 10 tall each way at 90 degrees, twice as wide.
    const far = frustum.flatMap(([a, b]) => [a, b]).filter((p) => p[2] === -10);
    expect(Math.max(...far.map((p) => p[0]!))).toBeCloseTo(20);
    expect(Math.max(...far.map((p) => p[1]!))).toBeCloseTo(10);
  });
});
