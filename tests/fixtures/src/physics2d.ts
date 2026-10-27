// SPDX-License-Identifier: Apache-2.0

// M5-RB2D-01: 2D physics components. The Rust test runs the app's physics sync and
// reads the results back.
import {
  BoxCollider2D,
  CapsuleCollider2D,
  CircleCollider2D,
  type Collider,
  type Collision2D,
  Component,
  EdgeCollider2D,
  GameObject,
  Physics2D,
  PolygonCollider2D,
  Rigidbody2D,
  register,
  Vector2,
  Vector3,
} from "@platypus/engine";

const log: string[] = [];
let box: GameObject;
let ball: GameObject;
let pill: GameObject;
let tri: GameObject;
let ray = "none";
let frames = 0;

class Logger extends Component {
  onCollisionEnter2D(c: Collision2D) {
    log.push(`${this.gameObject.name} hit ${c.gameObject.name}`);
  }
  onTriggerEnter2D(other: Collider) {
    log.push(`trigger ${other.gameObject.name}`);
  }
}

class Scene2D extends Component {
  override onStart() {
    const ground = new GameObject("ground");
    ground.addComponent(EdgeCollider2D, { points: [new Vector2(-20, 0), new Vector2(20, 0)] });
    // A slope from (3, 2) down to (9, 0).
    const slope = new GameObject("slope");
    slope.addComponent(EdgeCollider2D, { points: [new Vector2(3, 2), new Vector2(9, 0)] });

    // A box at z = 2 (2D ignores z) through a trigger onto the ground.
    box = new GameObject("box", { position: new Vector3(0, 3, 2) });
    box.addComponent(Rigidbody2D, { mass: 2 });
    box.addComponent(BoxCollider2D);
    box.addComponent(Logger);
    const zone = new GameObject("zone", { position: new Vector3(0, 1.8, 0) });
    zone.addComponent(BoxCollider2D, { size: new Vector2(2, 0.4), isTrigger: true });

    ball = new GameObject("ball", { position: new Vector3(3.6, 2.6, 0) });
    ball.addComponent(Rigidbody2D);
    ball.addComponent(CircleCollider2D, { radius: 0.3 });

    pill = new GameObject("pill", { position: new Vector3(-5, 2, -1) });
    pill.addComponent(Rigidbody2D, { freezeRotation: true });
    pill.addComponent(CapsuleCollider2D, { size: new Vector2(0.6, 2) });

    tri = new GameObject("tri", { position: new Vector3(-8, 2, 0) });
    tri.addComponent(Rigidbody2D);
    tri.addComponent(PolygonCollider2D, { points: [new Vector2(-0.5, 0), new Vector2(0.5, 0), new Vector2(0, 0.8)] });
  }

  override onUpdate() {
    frames++;
    // Once things have settled, look down at the box (triggers count, as in Unity).
    if (frames === 100) {
      Physics2D.raycast(new Vector2(0, 5), Vector2.down, 10).then((hit) => {
        ray = hit ? `${hit.collider?.gameObject.name} ${hit.distance.toFixed(2)}` : "miss";
      });
    }
  }
}

register({ Scene2D });

const g = globalThis as Record<string, unknown>;
g.__log2d = () => log.join(", ");
g.__box = () => {
  const p = box.transform.position;
  const e = box.transform.eulerAngles;
  return [p.x, p.y, p.z, e.x, e.y];
};
g.__ball = () => [ball.transform.position.x, ball.getComponent(Rigidbody2D)!.angularVelocity];
g.__pill = () => [pill.transform.position.y, pill.getComponent(Rigidbody2D)!.rotation];
g.__tri_y = () => tri.transform.position.y;
g.__ray = () => ray;
