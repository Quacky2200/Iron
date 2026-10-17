// M5-OBJ-06: GameObjects and components through @iron/engine in QuickJS. The Rust
// test applies the scene ops, drives the host and reads `__scene_log()`.
import { Component, GameObject, Quaternion, register, Time, Vector3 } from "@iron/engine";

const log: string[] = [];

class Spinner extends Component {
  speed = 90;
  override onStart() {
    log.push(`spinner start on ${this.gameObject.name}`);
  }
  override onUpdate(dt: number) {
    this.transform.rotate(new Vector3(0, this.speed * dt, 0));
  }
  override onDestroy() {
    log.push(`spinner destroyed at ${Math.round(this.transform.eulerAngles.y)}°`);
  }
}

class Game extends Component {
  turret!: GameObject;
  override onStart() {
    log.push(`game start on ${this.gameObject.name}`);
    this.turret = new GameObject("Turret", { position: new Vector3(0, 2, 0) });
    const barrel = new GameObject("Barrel", { parent: this.turret, position: new Vector3(0, 0, -1) });
    this.turret.addComponent(Spinner, { speed: 90 });
    barrel.transform.localRotation = Quaternion.identity;
  }
  override onUpdate() {
    if (Time.frameCount === 31) this.turret.enabled = false;
    if (Time.frameCount === 61) this.turret.destroy();
  }
}

register({ Game });

(globalThis as { __scene_log?: () => string }).__scene_log = () => log.join(", ");
