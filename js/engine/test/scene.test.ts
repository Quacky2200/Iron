import { afterEach, beforeEach, describe, expect, test } from "bun:test";

import { Component, GameObject, Quaternion, register, Vector3 } from "../src/index.ts";
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

const near = (a: Vector3, b: Vector3) => a.distance(b) < 1e-5;

class Spinner extends Component {
  speed = 90;
  override onUpdate(dt: number) {
    this.transform.rotate(new Vector3(0, this.speed * dt, 0));
  }
}

class Health extends Component {
  hp = 100;
}

// M5-OBJ-04
describe("GameObjects", () => {
  test("creating and parenting become ops; the hierarchy mirrors them", () => {
    const ship = new GameObject("Ship", { position: new Vector3(0, 1, 0) });
    const gun = new GameObject("Gun", { parent: ship, position: new Vector3(1, 0, 0) });
    expect(ops[0]).toEqual(["scene.create", 1, "Ship", null, [0, 1, 0], [0, 0, 0, 1], [1, 1, 1]]);
    expect(ops[1]).toEqual(["scene.create", 2, "Gun", 1, [1, 0, 0], [0, 0, 0, 1], [1, 1, 1]]);
    expect(ship.children).toEqual([gun]);
    expect(GameObject.find("Gun")).toBe(gun);
    expect(GameObject.roots).toEqual([ship]);
    expect(() => ship.setParent(gun)).toThrow("under itself");

    // Re-parenting to the root keeps the world position.
    ops = [];
    gun.setParent(null);
    expect(near(gun.transform.localPosition, new Vector3(1, 1, 0))).toBe(true);
    expect(ops.map((o) => o[0])).toEqual(["scene.parent", "scene.local", "scene.local"]);
    expect(GameObject.roots).toEqual([ship, gun]);

    ship.destroy();
    expect(ship.destroyed).toBe(true);
    expect(GameObject.find("Ship")).toBeUndefined();
    expect(ops.at(-1)).toEqual(["scene.destroy", 1]);
  });

  test("world transforms compose through parents", () => {
    const parent = new GameObject("p", {
      position: new Vector3(10, 0, 0),
      rotation: Quaternion.euler(0, 90, 0),
      scale: new Vector3(2, 2, 2),
    });
    const child = new GameObject("c", { parent, position: new Vector3(0, 0, -1) });
    // Local forward 1 m, scaled by 2, turned 90° left: 2 m along -x.
    expect(near(child.transform.position, new Vector3(8, 0, 0))).toBe(true);
    expect(near(child.transform.forward, new Vector3(-1, 0, 0))).toBe(true);
    child.transform.position = new Vector3(10, 4, 0);
    expect(near(child.transform.localPosition, new Vector3(0, 2, 0))).toBe(true);
    expect(near(parent.transform.inverseTransformPoint(new Vector3(8, 0, 0)), new Vector3(0, 0, -1))).toBe(true);

    child.transform.lookAt(new Vector3(10, 4, 10));
    expect(near(child.transform.forward, new Vector3(0, 0, 1))).toBe(true);
    child.transform.eulerAngles = new Vector3(0, 180, 0);
    expect(child.transform.eulerAngles.y).toBeCloseTo(180);

    parent.enabled = false;
    expect(child.activeInHierarchy).toBe(false);
  });

  test("engine moves update the mirror without echoing ops", () => {
    const box = new GameObject("box");
    ops = [];
    globalThis.__iron_scene([box.id, 1, 2, 3, 0, 0, 0, 1, 1, 1, 1, 99, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1]);
    expect(box.transform.position).toEqual(new Vector3(1, 2, 3));
    expect(ops).toEqual([]);
  });
});

// M5-OBJ-05
describe("components", () => {
  test("addComponent attaches, sets props and tells the engine", () => {
    const go = new GameObject("turret");
    const spin = go.addComponent(Spinner, { speed: 45 });
    expect(spin.speed).toBe(45);
    expect(spin.gameObject).toBe(go);
    expect(spin.transform).toBe(go.transform);
    expect(ops.at(-1)).toEqual(["scene.addComponent", go.id, spin.id, "Spinner"]);
    expect(go.getComponent(Spinner)).toBe(spin);
    expect(spin.getComponent(Health)).toBeUndefined();
    const hp = go.addComponent(Health);
    expect(GameObject.findObjectOfType(Health)).toBe(hp);

    // The engine drives it by id.
    globalThis.__iron_dispatch(spin.id, "onUpdate", 1);
    expect(go.transform.eulerAngles.y).toBeCloseTo(45);

    hp.destroy();
    expect(go.getComponent(Health)).toBeUndefined();
    expect(ops.at(-1)).toEqual(["scene.removeComponent", hp.id]);
    globalThis.__iron_destroy(hp.id);
  });

  test("engine-spawned components get the object the engine made", () => {
    register({ Spinner });
    globalThis.__iron_create("Spinner", 7, 1 << 30, "Spinner");
    const go = GameObject.byId(1 << 30)!;
    expect(go.name).toBe("Spinner");
    expect(go.getComponent(Spinner)?.id).toBe(7);
    expect(globalThis.__iron_registered()).toEqual(["Spinner"]);
    globalThis.__iron_destroy(7);
    expect(go.components).toEqual([]);
  });
});
