// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// M5-RB-01: Rigidbody and collider components on GameObjects. The Rust test runs the
// app's physics sync and reads the log back.
import {
  BoxCollider,
  CharacterController,
  HingeJoint,
  Rope,
  type Collider,
  type Collision,
  Component,
  GameObject,
  Rigidbody,
  register,
  SphereCollider,
  Vector3,
} from "@platypus/engine";

const log: string[] = [];
let crate: GameObject;
let paddle: GameObject;
let walker: GameObject;
let weight: GameObject;
let wheel: GameObject;

class Logger extends Component {
  onCollisionEnter(c: Collision) {
    log.push(`${this.gameObject.name} hit ${c.gameObject.name}`);
  }
  onTriggerEnter(other: Collider) {
    log.push(`trigger ${other.gameObject.name}`);
  }
}

class Drop extends Component {
  override onStart() {
    const floor = new GameObject("Floor", { position: new Vector3(0, -0.5, 0) });
    floor.addComponent(BoxCollider, { size: new Vector3(20, 1, 20) });

    crate = new GameObject("crate", { position: new Vector3(0, 3, 0) });
    crate.addComponent(Rigidbody, { mass: 5 });
    crate.addComponent(BoxCollider);
    crate.addComponent(Logger);

    // A trigger zone the crate falls through.
    const zone = new GameObject("zone", { position: new Vector3(0, 1.5, 0) });
    zone.addComponent(BoxCollider, { size: new Vector3(2, 0.5, 2), isTrigger: true });

    paddle = new GameObject("paddle", { position: new Vector3(5, 0.5, 0) });
    paddle.addComponent(Rigidbody, { isKinematic: true });
    paddle.addComponent(SphereCollider);

    // M5-RB-03: a character walking along +z on the floor.
    walker = new GameObject("walker", { position: new Vector3(-5, 1.05, 0) });
    walker.addComponent(CharacterController);

    // M5-JNT-02: a 2 m rope from a world point holding a weight, and a motorised wheel.
    weight = new GameObject("weight", { position: new Vector3(8, 4, 0) });
    const wrb = weight.addComponent(Rigidbody, { mass: 1 });
    weight.addComponent(SphereCollider, { radius: 0.2 });
    const anchor = new GameObject("anchor", { position: new Vector3(8, 6, 0) });
    anchor.addComponent(Rope, { length: 2, segments: 8, end: wrb });
    wheel = new GameObject("wheel", { position: new Vector3(-8, 3, 0) });
    wheel.addComponent(Rigidbody, { useGravity: false });
    wheel.addComponent(BoxCollider, { size: new Vector3(1, 0.2, 0.2) });
    wheel.addComponent(HingeJoint, { axis: new Vector3(0, 0, 1), useMotor: true, motor: { targetVelocity: 180, force: 10 } });
  }

  override onUpdate() {
    walker.getComponent(CharacterController)!.simpleMove(new Vector3(0, 0, 2));
  }

  override onFixedUpdate() {
    // Script-moved kinematic body.
    paddle.transform.position = new Vector3(2.5, 0.5, 0);
  }
}

register({ Drop });

const g = globalThis as Record<string, unknown>;
g.__rb_log = () => log.join(", ");
g.__crate_y = () => crate.transform.position.y;
g.__paddle_body_x = () => paddle.getComponent(Rigidbody)!.body!.position.x;
g.__walker = () => {
  const p = walker.transform.position;
  return [p.x, p.y, p.z, walker.getComponent(CharacterController)!.isGrounded ? 1 : 0];
};
g.__weight_y = () => weight.transform.position.y;
g.__wheel_angle = () => wheel.transform.eulerAngles.z;
