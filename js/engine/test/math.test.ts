import { describe, expect, test } from "bun:test";

import {
  Bounds,
  Color,
  Mathf,
  Plane,
  Pose,
  Quaternion,
  Ray,
  Rect,
  RectInt,
  Vector2,
  Vector2Int,
  Vector3,
  Vector4,
} from "../src/index.ts";

const near = (a: { x: number; y: number; z?: number }, b: { x: number; y: number; z?: number }, eps = 1e-5) =>
  Math.abs(a.x - b.x) < eps && Math.abs(a.y - b.y) < eps && Math.abs((a.z ?? 0) - (b.z ?? 0)) < eps;

// M5-MTH-01
describe("vectors", () => {
  test("Vector3 basics, Unity names", () => {
    const v = new Vector3(3, 4, 0);
    expect(v.magnitude).toBe(5);
    expect(v.normalized.equals(new Vector3(0.6, 0.8, 0))).toBe(true);
    expect(Vector3.forward).toEqual(new Vector3(0, 0, -1));
    expect(Vector3.right.cross(Vector3.up)).toEqual(new Vector3(0, 0, 1));
    expect(Vector3.angle(Vector3.right, Vector3.up)).toBeCloseTo(90);
    expect(new Vector3(1, -1, 0).reflect(Vector3.up)).toEqual(new Vector3(1, 1, 0));
    expect(new Vector3(2, 3, 4).projectOnPlane(Vector3.up)).toEqual(new Vector3(2, 0, 4));
    expect(Vector3.zero.moveTowards(new Vector3(10, 0, 0), 3)).toEqual(new Vector3(3, 0, 0));
    expect(new Vector3(10, 0, 0).clampMagnitude(2)).toEqual(new Vector3(2, 0, 0));
    // Immutable: methods return new values.
    const a = new Vector3(1, 2, 3);
    a.add(Vector3.one);
    expect(a).toEqual(new Vector3(1, 2, 3));
  });

  test("Vector2 and integer points", () => {
    expect(Vector2.right.rotate(90).equals(Vector2.up)).toBe(true);
    expect(Vector2.signedAngle(Vector2.right, Vector2.up)).toBeCloseTo(90);
    expect(Vector2.signedAngle(Vector2.up, Vector2.right)).toBeCloseTo(-90);
    expect(new Vector2Int(2.9, -1.2)).toEqual(new Vector2Int(2, -1));
    expect(Vector2Int.floor({ x: -0.5, y: 1.5 })).toEqual(new Vector2Int(-1, 1));
    expect(new Vector2Int(1, 1).manhattan({ x: 4, y: -1 })).toBe(5);
    expect([...new RectInt(0, 0, 2, 2).positions()].map(String)).toEqual(["(0, 0)", "(1, 0)", "(0, 1)", "(1, 1)"]);
  });

  test("Mathf", () => {
    expect(Mathf.lerp(0, 10, 2)).toBe(10);
    expect(Mathf.inverseLerp(10, 20, 15)).toBe(0.5);
    expect(Mathf.deltaAngle(350, 10)).toBe(20);
    expect(Mathf.deltaAngle(10, 350)).toBe(-20);
    expect(Mathf.moveTowards(0, 1, 0.25)).toBe(0.25);
    expect(Mathf.pingPong(3, 2)).toBe(1);
    expect(Mathf.repeat(-1, 360)).toBe(359);
    expect(Mathf.lerpAngle(350, 10, 0.5)).toBe(360);
    expect(Mathf.moveTowardsAngle(10, 350, 5)).toBe(5);
    expect(Mathf.sign(0)).toBe(1);
    expect([Mathf.isPowerOfTwo(64), Mathf.nextPowerOfTwo(65), Mathf.closestPowerOfTwo(90)]).toEqual([true, 128, 64]);
    expect(Mathf.linearToGammaSpace(Mathf.gammaToLinearSpace(0.5))).toBeCloseTo(0.5);
    // Perlin noise: repeatable, smooth, in range, and 0.5 on grid points.
    expect(Mathf.perlinNoise(1.3, 2.7)).toBe(Mathf.perlinNoise(1.3, 2.7));
    expect(Mathf.perlinNoise(3, 4)).toBe(0.5);
    expect(Math.abs(Mathf.perlinNoise(1.3, 2.7) - Mathf.perlinNoise(1.31, 2.7))).toBeLessThan(0.05);
    for (let i = 0; i < 200; i++) {
      const n = Mathf.perlinNoise(i * 0.37, i * 0.11);
      expect(n >= 0 && n <= 1).toBe(true);
    }
    // smoothDamp approaches without overshooting.
    let [x, v] = [0, 0];
    for (let i = 0; i < 120; i++) {
      [x, v] = Mathf.smoothDamp(x, 10, v, 0.3, 1 / 60);
      expect(x).toBeLessThanOrEqual(10);
    }
    expect(x).toBeCloseTo(10, 1);
  });
});

// M5-MTH-02
describe("rotations", () => {
  test("Euler angles in degrees, Unity's order, round trip", () => {
    // Yaw 90° turns forward (-z) to -x.
    expect(near(Quaternion.euler(0, 90, 0).rotate(Vector3.forward), new Vector3(-1, 0, 0))).toBe(true);
    // Pitch +90° about +x tips forward up (right-handed, unlike Unity's left-handed
    // world where positive pitch looks down).
    expect(near(Quaternion.euler(90, 0, 0).rotate(Vector3.forward), new Vector3(0, 1, 0))).toBe(true);
    for (const e of [new Vector3(10, 20, 30), new Vector3(300, 45, 5), new Vector3(0, 270, 0), new Vector3(80, 10, 350)]) {
      const back = Quaternion.euler(e).eulerAngles;
      expect(Quaternion.angle(Quaternion.euler(back), Quaternion.euler(e))).toBeLessThan(1e-3);
      expect(near(back, e, 1e-3)).toBe(true);
    }
  });

  test("lookRotation, fromToRotation, slerp and rotateTowards", () => {
    const dir = new Vector3(1, 1, 0).normalized;
    expect(near(Quaternion.lookRotation(dir).rotate(Vector3.forward), dir)).toBe(true);
    // Up stays as up as it can.
    expect(Quaternion.lookRotation(Vector3.right).rotate(Vector3.up).y).toBeCloseTo(1);
    const q = Quaternion.fromToRotation(Vector3.up, Vector3.right);
    expect(near(q.rotate(Vector3.up), Vector3.right)).toBe(true);
    expect(near(Quaternion.fromToRotation(Vector3.up, Vector3.down).rotate(Vector3.up), Vector3.down)).toBe(true);
    const half = Quaternion.identity.slerp(Quaternion.euler(0, 90, 0), 0.5);
    expect(Quaternion.angle(half, Quaternion.euler(0, 45, 0))).toBeLessThan(1e-3);
    const step = Quaternion.identity.rotateTowards(Quaternion.euler(0, 90, 0), 30);
    expect(Quaternion.angle(Quaternion.identity, step)).toBeCloseTo(30, 3);
    expect(near(q.mul(q.inverse()).rotate(Vector3.right), Vector3.right)).toBe(true);
  });
});

// M5-MTH-03
describe("shapes and colours", () => {
  test("Plane, Ray, Bounds, Rect", () => {
    const floor = Plane.fromPoint(Vector3.up, new Vector3(0, 2, 0));
    expect(floor.getDistanceToPoint(new Vector3(5, 5, 5))).toBe(3);
    const ray = new Ray(new Vector3(0, 10, 0), new Vector3(0, -2, 0));
    expect(floor.raycast(ray)).toBe(8);
    expect(near(ray.getPoint(8), new Vector3(0, 2, 0))).toBe(true);
    expect(floor.raycast(new Ray(Vector3.zero, Vector3.down))).toBeNull();
    expect(floor.raycast(new Ray(Vector3.zero, Vector3.right))).toBeNull();

    const box = Bounds.minMax(Vector3.zero, new Vector3(2, 2, 2));
    expect(box.center).toEqual(Vector3.one);
    expect(box.contains(new Vector3(1, 2, 0))).toBe(true);
    expect(box.encapsulate(new Vector3(-1, 0, 0)).min).toEqual(new Vector3(-1, 0, 0));
    expect(box.intersects(new Bounds(new Vector3(3, 1, 1), new Vector3(2.2, 1, 1)))).toBe(true);

    const r = new Rect(0, 0, 10, 5);
    expect(r.contains({ x: 9, y: 4 })).toBe(true);
    expect(r.contains({ x: 10, y: 0 })).toBe(false);
    expect(r.overlaps(new Rect(9, 4, 2, 2))).toBe(true);
  });

  test("Pose and Vector4", () => {
    const parent = new Pose(new Vector3(10, 0, 0), Quaternion.euler(0, 90, 0));
    const child = new Pose(new Vector3(0, 0, -1));
    const world = child.transformedBy(parent);
    expect(near(world.position, new Vector3(9, 0, 0))).toBe(true);
    expect(near(world.forward, new Vector3(-1, 0, 0))).toBe(true);
    expect(near(parent.inverseTransformPoint(parent.transformPoint(new Vector3(1, 2, 3))), new Vector3(1, 2, 3))).toBe(true);
    const v = Vector4.fromVector3(new Vector3(1, 2, 2), 0);
    expect(v.magnitude).toBe(3);
    expect(v.normalized.equals(new Vector4(1 / 3, 2 / 3, 2 / 3, 0))).toBe(true);
    expect(Vector4.zero.lerp(Vector4.one, 0.5)).toEqual(new Vector4(0.5, 0.5, 0.5, 0.5));
  });

  test("Color", () => {
    // Hex is sRGB, stored linear.
    const orange = Color.hex("#ff8000");
    expect([orange.r, orange.b, orange.a]).toEqual([1, 0, 1]);
    expect(orange.g).toBeCloseTo(0.2159, 4);
    expect(orange.toHex()).toBe("#ff8000ff");
    expect(Color.hex("#0f08").a).toBeCloseTo(0x88 / 255);
    expect(Color.fromSrgb(0.5, 0.5, 0.5).r).toBeCloseTo(0.214, 3);
    expect(Color.black.lerp(Color.white, 0.5)).toEqual(new Color(0.5, 0.5, 0.5, 1));
  });
});
