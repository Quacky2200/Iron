import { afterEach, beforeEach, describe, expect, test } from "bun:test";

import {
  BoxCollider,
  CapsuleCollider,
  type Collider,
  type Collision,
  Component,
  ForceMode,
  GameObject,
  PhysicsMaterial,
  Rigidbody,
  RigidbodyConstraints,
  SphereCollider,
  Vector3,
} from "../src/index.ts";
import { _resetPhysicsComponentsForTests } from "../src/physics-components.ts";
import { _resetPhysicsForTests } from "../src/physics.ts";
import { _resetSceneForTests } from "../src/scene.ts";

let ops: unknown[][] = [];

beforeEach(() => {
  _resetSceneForTests();
  _resetPhysicsForTests();
  _resetPhysicsComponentsForTests();
  ops = [];
  (globalThis as { __iron_op?: unknown }).__iron_op = (...args: unknown[]) => ops.push(args);
});

afterEach(() => {
  delete (globalThis as { __iron_op?: unknown }).__iron_op;
});

const physicsOps = () => ops.filter((o) => String(o[0]).startsWith("physics."));

// M5-RB-02
describe("physics components", () => {
  test("colliders make a static body at the object; a Rigidbody makes it dynamic", () => {
    const go = new GameObject("crate", { position: new Vector3(1, 2, 3), scale: new Vector3(2, 2, 2) });
    const box = go.addComponent(BoxCollider, { size: new Vector3(1, 0.5, 1), material: new PhysicsMaterial(0.2, 0.5) });
    expect(physicsOps().slice(0, 3)).toEqual([
      ["physics.body", 1, "fixed", [1, 2, 3], [0, 0, 0, 1], [0, 0, 0], [1, 0, 0, false, false]],
      ["physics.attach", 1, go.id],
      // Half extents, scaled by the object's scale.
      ["physics.collider", 1, 1, "box", [1, 0.5, 1], [0, 0, 0], null, [0.2, 0.5, 1, false, true, null]],
    ]);
    ops = [];
    const rb = go.addComponent(Rigidbody, { mass: 10, constraints: RigidbodyConstraints.FreezeRotation });
    expect(physicsOps()).toContainEqual(["physics.set", 1, "kind", "dynamic"]);
    expect(physicsOps()).toContainEqual(["physics.set", 1, "mass", 10]);
    expect(physicsOps()).toContainEqual(["physics.set", 1, "constraints", [true, true, true, false, false, false]]);
    expect(box.attachedRigidbody).toBe(rb);

    // A second collider shares the body (and the mass is shared again).
    ops = [];
    go.addComponent(SphereCollider, { radius: 0.25, center: new Vector3(0, 1, 0) });
    expect(physicsOps()[0]).toEqual(["physics.collider", 2, 1, "ball", [0.5], [0, 2, 0], null, [0.6, 0, 1, false, true, null]]);
    expect(physicsOps().at(-1)).toEqual(["physics.set", 1, "mass", 10]);

    // Removing the Rigidbody leaves static colliders; removing those drops the body.
    ops = [];
    rb.destroy();
    globalThis.__iron_dispatch(rb.id, "onDestroy");
    expect(physicsOps()).toEqual([["physics.set", 1, "kind", "fixed"]]);
  });

  test("ForceMode turns into the right impulse", () => {
    const go = new GameObject("ball");
    go.addComponent(SphereCollider);
    const rb = go.addComponent(Rigidbody, { mass: 2 });
    ops = [];
    rb.addForce(new Vector3(60, 0, 0)); // Force: N over 1/60 s
    rb.addForce(new Vector3(60, 0, 0), ForceMode.Acceleration); // × mass
    rb.addForce(new Vector3(1, 0, 0), ForceMode.Impulse);
    rb.addForce(new Vector3(1, 0, 0), ForceMode.VelocityChange); // × mass
    const x = physicsOps().map((o) => (o[3] as number[])[0]);
    expect(x).toEqual([1, 2, 1, 2]);
    rb.isKinematic = true;
    expect(physicsOps().at(-1)).toEqual(["physics.set", 1, "kind", "kinematic"]);
  });

  test("capsules use Unity's height (caps included)", () => {
    const go = new GameObject("player");
    go.addComponent(CapsuleCollider, { radius: 0.5, height: 2 });
    expect(physicsOps().find((o) => o[0] === "physics.collider")?.[4]).toEqual([0.5, 0.5]);
  });

  test("contacts reach onCollisionEnter / onTriggerEnter on both objects", () => {
    const log: string[] = [];
    class Listener extends Component {
      onCollisionEnter(c: Collision) {
        log.push(`${this.gameObject.name} hit ${c.gameObject.name} at ${c.relativeSpeed}`);
      }
      onCollisionExit(c: Collision) {
        log.push(`${this.gameObject.name} left ${c.gameObject.name}`);
      }
      onTriggerEnter(other: Collider) {
        log.push(`${this.gameObject.name} entered ${other.gameObject.name}`);
      }
    }
    const a = new GameObject("a");
    const ca = a.addComponent(BoxCollider);
    a.addComponent(Rigidbody);
    a.addComponent(Listener);
    const b = new GameObject("b");
    const cb = b.addComponent(BoxCollider);
    b.addComponent(Listener);
    const zone = new GameObject("zone");
    const cz = zone.addComponent(BoxCollider, { isTrigger: true });
    const id = (c: Collider) => c.handle!.id;
    globalThis.__iron_physics([], [1, id(ca), id(cb), 0, 4, 0, id(ca), id(cb), 0, 0, 1, id(ca), id(cz), 1, 0], [], []);
    expect(log).toEqual(["a hit b at 4", "b hit a at 4", "a left b", "b left a", "a entered zone"]);
  });
});

// M5-RB-04
describe("colliders and scale", () => {
  test("colliders resize when their object or a parent is scaled, and when reparented", () => {
    const parent = new GameObject("parent");
    const go = new GameObject("crate", { parent });
    go.addComponent(BoxCollider, { size: new Vector3(1, 2, 1) });
    const shapes = () => ops.filter((o) => o[0] === "physics.collider").map((o) => o[4]);
    expect(shapes()).toEqual([[0.5, 1, 0.5]]);
    go.transform.localScale = new Vector3(2, 2, 2);
    expect(shapes().at(-1)).toEqual([1, 2, 1]);
    // The old shape went.
    expect(ops.some((o) => o[0] === "physics.removeCollider")).toBe(true);
    parent.transform.localScale = new Vector3(3, 1, 1);
    expect(shapes().at(-1)).toEqual([3, 2, 1]);
    // Same scale again: nothing rebuilt.
    const count = shapes().length;
    parent.transform.localScale = new Vector3(3, 1, 1);
    go.transform.localScale = new Vector3(2, 2, 2);
    expect(shapes().length).toBe(count);
    go.setParent(null, true);
    expect(shapes().at(-1)).toEqual([1, 2, 1]);
  });
});
