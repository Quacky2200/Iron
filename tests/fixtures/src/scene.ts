// M5-OBJ-06: GameObjects and components through @iron/engine in QuickJS. The Rust
// test applies the scene ops, drives the host and reads `__scene_log()`.
import { Component, GameObject, Light, property, Quaternion, register, SceneManager, Time, Vector3 } from "@iron/engine";

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

/** Logs each lifecycle call (M5-OBJ-08). */
const hooks: string[] = [];
class Lifecycle extends Component {
  override onStart() {
    hooks.push("start");
  }
  override onFixedUpdate() {
    hooks.push("fixed");
  }
  override onUpdate() {
    hooks.push("update");
  }
  override onLateUpdate() {
    hooks.push("late");
  }
  override onPreRender() {
    hooks.push("prerender");
  }
  override onDestroy() {
    hooks.push("destroy");
  }
}

/** M5-SCN-02: saves a scene and loads it back in QuickJS (decorators included). */
let trip = "";
class Door extends Component {
  @property opensTo: GameObject | null = null;
  @property speed = 1;
}
class SceneTrip extends Component {
  override onStart() {
    GameObject.dontDestroyOnLoad(this.gameObject);
    const room = new GameObject("Room", { position: new Vector3(0, 0, -5) });
    const door = new GameObject("Door", { parent: room });
    door.addComponent(Door, { opensTo: room, speed: 3 });
    new GameObject("Lamp").addComponent(Light, { type: "point", range: 7 });
    const json = JSON.stringify(SceneManager.getActiveScene().serialize());
    const scene = SceneManager.loadScene(JSON.parse(json));
    const loaded = GameObject.find("Door")!.getComponent(Door)!;
    trip = [
      scene.getRootGameObjects().map((g) => g.name).join("+"),
      loaded.speed,
      loaded.opensTo === GameObject.find("Room"),
      GameObject.find("Lamp")!.getComponent(Light)!.range,
      this.gameObject.scene.name,
    ].join(" ");
  }
}

/** M4-HOT-02/03/05: state that should survive a reload. */
let starts = 0;
let reloads = 0;
class HotCounter extends Component {
  @property speed = 1;
  @property label = "fresh";
  count = 0;
  child: GameObject | null = null;
  override onStart() {
    starts++;
    this.speed = 42;
    this.label = "started";
    this.child = new GameObject("HotChild", { parent: this.gameObject, position: new Vector3(1, 2, 3) });
  }
  override onUpdate() {
    this.count++;
  }
  override onReload() {
    reloads++;
  }
}

register({ Game, Lifecycle, Door, SceneTrip, HotCounter });
/** M4-HOT-07: lots of components to carry over a reload. */
(globalThis as { __hot_spawn?: (n: number) => void }).__hot_spawn = (n) => {
  for (let i = 0; i < n; i++) {
    const go = new GameObject(`Counter ${i}`, { position: new Vector3(i, 0, 0) });
    const c = go._addComponentAsleep(HotCounter, { speed: i, count: i });
    c.child = new GameObject(`Child ${i}`, { parent: go });
  }
};
(globalThis as { __hot_log?: () => string }).__hot_log = () => {
  const c = GameObject.findObjectOfType(HotCounter)!;
  return [c.speed, c.label, c.count, c.child === GameObject.find("HotChild"), starts, reloads, Time.frameCount].join(" ");
};
(globalThis as { __scene_trip?: () => string }).__scene_trip = () => trip;
(globalThis as { __hooks_log?: () => string }).__hooks_log = () => hooks.join(" ");

(globalThis as { __scene_log?: () => string }).__scene_log = () => log.join(", ");
