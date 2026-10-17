// M5-RB-01: Rigidbody and collider components on GameObjects. The Rust test runs the
// app's physics sync and reads the log back.
import {
  BoxCollider,
  type Collider,
  type Collision,
  Component,
  GameObject,
  Rigidbody,
  register,
  SphereCollider,
  Vector3,
} from "@iron/engine";

const log: string[] = [];
let crate: GameObject;
let paddle: GameObject;

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
