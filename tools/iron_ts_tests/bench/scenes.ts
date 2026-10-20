// Scene files, `instantiate` and prefabs for the Rust host: scenes are loaded from JSON text,
// components are made by kind (the classes below), and the program copies objects.
import { property } from "./property_stub";

let log = "";

class Spinner extends Component {
  @property speed: number = 45;
  @property axis: Vector3 = Vector3.up;
  @property buddy: Spinner | null = null;
  @property target: GameObject | null = null;
  ticks: number = 0;

  onAwake(): void {
    log += "a";
  }
  onStart(): void {
    log += "s";
  }
  onUpdate(dt: number): void {
    this.ticks += 1;
  }
}

class Bobber extends Component {
  @property height: number = 0.3;
  @property speed: number = 1;
}

register({ Spinner, Bobber });

export function logText(): string {
  return log;
}

export function clearLog(): void {
  log = "";
}

export function loadScene(text: string): number {
  return SceneManager.loadScene(text).handle;
}

export function loadAdditive(text: string): number {
  return SceneManager.loadScene(text, "additive").handle;
}

export function activeName(): string {
  return SceneManager.getActiveScene().name;
}

export function sceneCount(): number {
  return SceneManager.sceneCount;
}

export function saved(): string {
  return SceneManager.getActiveScene().serialize$n();
}

export function rootCount(): number {
  return SceneManager.getActiveScene().rootCount;
}

export function rootAt(i: number): GameObject {
  return SceneManager.getActiveScene().rootAt(i);
}

export function speedOf(go: GameObject): number {
  const s = go.getComponent(Spinner);
  return s ? s.speed : -1;
}

export function axisY(go: GameObject): number {
  const s = go.getComponent(Spinner);
  return s ? s.axis.y : -1;
}

export function setAxis(go: GameObject, x: number, y: number, z: number): void {
  const s = go.getComponent(Spinner);
  if (s) s.axis = new Vector3(x, y, z);
}

export function buddyObject(go: GameObject): GameObject | undefined {
  const s = go.getComponent(Spinner);
  if (s && s.buddy) return s.buddy.gameObject;
  return undefined;
}

export function targetObject(go: GameObject): GameObject | undefined {
  const s = go.getComponent(Spinner);
  if (s && s.target) return s.target;
  return undefined;
}

export function link(go: GameObject, other: GameObject): void {
  const s = go.getComponent(Spinner);
  const o = other.getComponent(Spinner);
  if (s && o) {
    s.buddy = o;
    s.target = other;
  }
}

export function bobHeight(go: GameObject): number {
  const b = go.getComponent(Bobber);
  return b ? b.height : -1;
}

export function copy(go: GameObject): GameObject {
  return GameObject.instantiate(go);
}

export function copyAt(go: GameObject, x: number, y: number, z: number): GameObject {
  return GameObject.instantiate(go, { position: new Vector3(x, y, z) });
}

export function copyUnder(go: GameObject, parent: GameObject): GameObject {
  return GameObject.instantiate(go, { parent: parent });
}

export function prefabOf(go: GameObject): number {
  return Prefab.fromObject(go).name.length;
}

export function placePrefab(go: GameObject, x: number, y: number, z: number): GameObject {
  const p = Prefab.fromObject(go);
  return p.instantiate({ position: new Vector3(x, y, z) });
}

export function keep(go: GameObject): void {
  GameObject.dontDestroyOnLoad(go);
}

export function sceneOfName(go: GameObject): string {
  return SceneManager.sceneOf(go).name;
}

export function jsonOfHandle(go: GameObject): string {
  return JSON.stringify({ object: go, n: 1 });
}

export function jsonOfVector(x: number, y: number, z: number): string {
  return JSON.stringify({ v: new Vector3(x, y, z), c: new Color(0.5, 0.25, 1, 1), q: Quaternion.identity });
}

export function parseVector(text: string): number {
  const v = JSON.parse(text) as Vector3;
  return v.x + v.y * 10 + v.z * 100;
}

export function parseColorAlpha(text: string): number {
  const c = JSON.parse(text) as Color;
  return c.a;
}

export function curveFromPoints(mode: string, t: number): number {
  const pts: Vector2[] = [new Vector2(0, 0), new Vector2(1, 10), new Vector2(2, 0)];
  const c = AnimationCurve.fromPoints(pts, mode);
  return c.evaluate(t);
}

export function curveDefaultMode(t: number): number {
  const pts: Vector2[] = [new Vector2(0, 0), new Vector2(2, 4)];
  return AnimationCurve.fromPoints(pts).evaluate(t);
}
