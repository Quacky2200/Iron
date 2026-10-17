// M4-PHY-02: a component drops a ball onto a floor through @iron/engine's `physics`
// and counts collisions. The Rust test runs ops, steps and sends the batches back,
// then reads `__physics_log()`.
import { Component, physics, type RigidBody, register } from "@iron/engine";

const log: string[] = [];

class Dropper extends Component {
  ball!: RigidBody;

  override onStart() {
    physics.addCollider({ shape: { box: { x: 10, y: 0.5, z: 10 } }, position: { x: 0, y: -0.5, z: 0 } });
    this.ball = physics.createBody({ position: { x: 0, y: 2, z: 0 } });
    this.ball.addCollider({ shape: { ball: 0.5 } });
    this.ball.onCollisionStart = (e) => {
      log.push(`hit ${e.other.body === null ? "world" : "body"} at y=${this.ball.position.y.toFixed(1)}`);
      physics.raycast({ x: 0, y: 5, z: 0 }, { x: 0, y: -1, z: 0 }).then((hit) => {
        log.push(`ray ${hit?.collider.id} ${hit?.distance.toFixed(2)}`);
      });
    };
  }
}

register({ Dropper });

(globalThis as { __physics_log?: () => string }).__physics_log = () => log.join(", ");
