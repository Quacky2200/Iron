import { afterEach, beforeEach, describe, expect, test } from "bun:test";

import {
  Camera,
  Color,
  Component,
  GameObject,
  Light,
  Material,
  Mesh,
  MeshRenderer,
  Prefab,
  property,
  register,
  RenderSettings,
  type SceneData,
  SceneManager,
  Texture,
  Vector3,
} from "../src/index.ts";
import { _resetSceneForTests } from "../src/scene.ts";

let ops: unknown[][] = [];

beforeEach(() => {
  _resetSceneForTests();
  ops = [];
  (globalThis as { __iron_op?: unknown }).__iron_op = (...args: unknown[]) => ops.push(args);
  register({ Health, Turret });
});

afterEach(() => {
  delete (globalThis as { __iron_op?: unknown }).__iron_op;
});

class Health extends Component {
  @property max = 100;
  @property target: GameObject | null = null;
  @property gun: Turret | null = null;
  @property tags: { team: string; spawn: Vector3 } = { team: "red", spawn: Vector3.zero };
  /** Not a property: not saved. */
  current = 100;
  /** What `target` was when it woke: properties are set before `onAwake`. */
  targetAtAwake: GameObject | null | undefined;
  override onAwake() {
    this.targetAtAwake = this.target;
  }
}

class Turret extends Component {
  @property range = 10;
}

/** A scene with a bit of everything: hierarchy, layers, engine and script components, references, shared assets. */
function buildScene() {
  const mesh = Mesh.create({ vertices: [Vector3.zero, Vector3.right, Vector3.up], triangles: [0, 1, 2] });
  const brick = Texture.load("textures/brick.png", { filterMode: "point" });
  const player = new GameObject("Player", { position: new Vector3(1, 2, 3) });
  player.layer = 9;
  const gunObject = new GameObject("Gun", { parent: player, position: new Vector3(0, 1, 0), scale: new Vector3(2, 2, 2) });
  const turret = gunObject.addComponent(Turret, { range: 25 });
  const hidden = new GameObject("Hidden", { parent: player });
  hidden.enabled = false;
  const enemy = new GameObject("Enemy");
  player.addComponent(MeshRenderer, { mesh, material: Material.create({ color: new Color(1, 0, 0, 1), metallic: 1, mainTexture: brick }) });
  const bumps = Texture.load("textures/bumps.png", { linear: true, anisoLevel: 8 });
  enemy.addComponent(MeshRenderer, {
    mesh,
    material: Material.create({ normalMap: bumps, normalScale: 2, renderingMode: "transparent", specularColor: new Color(0.1, 0.2, 0.3, 1) }),
  });
  player.addComponent(Health, { max: 150, target: enemy, gun: turret, tags: { team: "blue", spawn: new Vector3(4, 5, 6) } });
  const cam = new GameObject("Camera").addComponent(Camera, { fieldOfView: 40, cullingMask: 0xffff });
  new GameObject("Sun").addComponent(Light, { type: "directional", intensity: 2, shadows: "soft" });
  RenderSettings.ambientLight = new Color(0.1, 0.2, 0.3, 1);
  return { player, enemy, cam };
}

// M5-SCN-01
describe("scene serialization", () => {
  test("save then load round-trips objects, components and properties exactly", () => {
    buildScene();
    const saved = SceneManager.getActiveScene().serialize();
    const json = JSON.parse(JSON.stringify(saved)) as SceneData;
    expect(json.objects.map((o) => o.name)).toEqual(["Player", "Enemy", "Camera", "Sun"]);
    expect(Object.values(json.assets ?? {}).map((a) => a.type).sort()).toEqual(["Mesh", "Texture", "Texture"]);

    RenderSettings.ambientLight = Color.black;
    const scene = SceneManager.loadScene(json);
    expect(SceneManager.getActiveScene()).toBe(scene);
    expect(RenderSettings.ambientLight.b).toBeCloseTo(0.3);
    // The old objects went; the new ones save to the same data.
    expect(GameObject.roots.map((g) => g.name)).toEqual(["Player", "Enemy", "Camera", "Sun"]);
    expect(scene.serialize()).toEqual(saved);

    const player = GameObject.find("Player")!;
    const health = player.getComponent(Health)!;
    expect(player.layer).toBe(9);
    expect(player.transform.localPosition.equals(new Vector3(1, 2, 3))).toBe(true);
    expect(player.findChild("Hidden")!.enabled).toBe(false);
    expect(health.max).toBe(150);
    expect(health.current).toBe(100);
    expect(health.target).toBe(GameObject.find("Enemy")!);
    expect(health.targetAtAwake).toBe(health.target);
    expect(health.gun).toBe(player.findChild("Gun")!.getComponent(Turret)!);
    expect(health.gun!.range).toBe(25);
    expect(health.tags.spawn.equals(new Vector3(4, 5, 6))).toBe(true);
    // One mesh and one texture, shared as they were.
    const meshes = [player, GameObject.find("Enemy")!].map((g) => g.getComponent(MeshRenderer)!.mesh);
    expect(meshes[0]).toBeInstanceOf(Mesh);
    expect(meshes[0]).toBe(meshes[1]!);
    const material = player.getComponent(MeshRenderer)!.material;
    expect(material.mainTexture!.src).toBe("textures/brick.png");
    expect(material.mainTexture!.filterMode).toBe("point");
    expect(GameObject.find("Camera")!.getComponent(Camera)!.cullingMask).toBe(0xffff);
    // Maps and modes come back; defaults aren't written.
    const glass = GameObject.find("Enemy")!.getComponent(MeshRenderer)!.material;
    expect(glass.normalMap!.linear).toBe(true);
    expect(glass.normalMap!.anisoLevel).toBe(8);
    expect(glass.normalScale).toBe(2);
    expect(glass.renderingMode).toBe("transparent");
    expect(glass.specularColor!.b).toBeCloseTo(0.3);
    const savedMaterial = json.objects[0]!.components!.find((c) => c.type === "MeshRenderer")!.properties!.material as Record<string, unknown>;
    expect("renderingMode" in savedMaterial).toBe(false);
  });

  test("unknown components and values are skipped with a warning", () => {
    const warnings: string[] = [];
    const warn = console.warn;
    console.warn = (m: string) => warnings.push(m);
    try {
      const go = new GameObject("Odd");
      go.addComponent(Health, { tags: { team: "x", spawn: new Date() as never } });
      const data = SceneManager.getActiveScene().serialize();
      expect(data.objects[0]!.components![0]!.properties!.tags).toEqual({ team: "x", spawn: null });
      data.objects[0]!.components!.push({ id: 99, type: "Nope" });
      SceneManager.loadScene(data);
      expect(GameObject.find("Odd")!.components.length).toBe(1);
    } finally {
      console.warn = warn;
    }
    expect(warnings.some((w) => w.includes("Date"))).toBe(true);
    expect(warnings.some((w) => w.includes("Nope"))).toBe(true);
  });
});

// M5-SCN-03
describe("SceneManager", () => {
  test("scenes: active, additive, moving objects, unloading, events", () => {
    const main = SceneManager.getActiveScene();
    expect(main.name).toBe("Main");
    const a = new GameObject("A");
    expect(a.scene).toBe(main);
    const child = new GameObject("Child", { parent: a });
    expect(child.scene).toBe(main);

    const heard: string[] = [];
    SceneManager.onSceneLoaded((s, mode) => heard.push(`loaded ${s.name} ${mode}`));
    SceneManager.onSceneUnloaded((s) => heard.push(`unloaded ${s.name}`));
    SceneManager.onActiveSceneChanged((p, n) => heard.push(`active ${p.name} -> ${n.name}`));

    const level: SceneData = { format: "iron.scene", version: 1, name: "Level", objects: [{ id: 1, name: "Door" }] };
    const added = SceneManager.loadScene(level, "additive");
    expect(SceneManager.sceneCount).toBe(2);
    expect(SceneManager.getActiveScene()).toBe(main);
    expect(added.getRootGameObjects().map((g) => g.name)).toEqual(["Door"]);
    expect(main.getRootGameObjects().map((g) => g.name)).toEqual(["A"]);

    expect(SceneManager.setActiveScene(added)).toBe(true);
    expect(new GameObject("B").scene).toBe(added);
    SceneManager.moveGameObjectToScene(a, added);
    expect(child.scene).toBe(added);
    expect(() => SceneManager.moveGameObjectToScene(child, main)).toThrow("root");

    expect(SceneManager.unloadScene(main)).toBe(true);
    expect(SceneManager.unloadScene(added)).toBe(false);
    expect(main.isLoaded).toBe(false);
    expect(heard).toEqual(["loaded Level additive", "active Main -> Level", "unloaded Main"]);
  });

  test("single loads replace every scene but objects kept with dontDestroyOnLoad", () => {
    const music = new GameObject("Music");
    GameObject.dontDestroyOnLoad(music);
    new GameObject("Level 1 stuff");
    SceneManager.loadScene({ format: "iron.scene", version: 1, name: "Level 2", objects: [] });
    expect(GameObject.roots.map((g) => g.name)).toEqual(["Music"]);
    expect(music.scene.name).toBe("DontDestroyOnLoad");
    expect(SceneManager.sceneCount).toBe(1);
    expect(SceneManager.getActiveScene().name).toBe("Level 2");
    expect(ops.some((o) => o[0] === "scene.destroy")).toBe(true);
  });

  test("scene-only components are registered without being started", () => {
    register({ Turret }, { spawn: false });
    expect(globalThis.__iron_registered!()).toEqual(["Health"]);
  });

  test("loadSceneAsync reads scene files from the game folder", async () => {
    const loading = SceneManager.loadSceneAsync("scenes/level.scene.json", "additive");
    const read = ops.find((o) => o[0] === "asset.readText")!;
    expect(read[2]).toBe("scenes/level.scene.json");
    globalThis.__iron_asset_text!([read[1] as number, JSON.stringify({ format: "iron.scene", version: 1, name: "Level", objects: [{ id: 1, name: "Rock" }] })]);
    const scene = await loading;
    expect(scene.path).toBe("scenes/level.scene.json");
    expect(scene.getRootGameObjects()[0]!.name).toBe("Rock");

    const missing = SceneManager.loadSceneAsync("nope.json");
    const id = ops.filter((o) => o[0] === "asset.readText").at(-1)![1] as number;
    globalThis.__iron_asset_text!([id, null]);
    await expect(missing).rejects.toThrow("nope.json");
    expect(() => SceneManager.loadScene({ name: "x" } as never)).toThrow("iron.scene");
  });
});

// M5-SCN-04
describe("GameObject.instantiate", () => {
  test("copies objects with references inside pointing at the copies", () => {
    const { player, enemy } = buildScene();
    const copy = GameObject.instantiate(player, { position: new Vector3(10, 0, 0) });
    expect(copy).not.toBe(player);
    expect(copy.name).toBe("Player");
    expect(copy.transform.position.equals(new Vector3(10, 0, 0))).toBe(true);
    const health = copy.getComponent(Health)!;
    // Inside the copy: the copy's gun. Outside: the same enemy.
    expect(health.gun).toBe(copy.findChild("Gun")!.getComponent(Turret)!);
    expect(health.gun).not.toBe(player.getComponent(Health)!.gun);
    expect(health.target).toBe(enemy);
    expect(copy.findChild("Gun")!.transform.localScale.equals(new Vector3(2, 2, 2))).toBe(true);
    // Under a parent it keeps its local transform.
    const holder = new GameObject("Holder", { position: new Vector3(0, 5, 0) });
    const gun = GameObject.instantiate(player.findChild("Gun")!, { parent: holder });
    expect(gun.parent).toBe(holder);
    expect(gun.transform.position.equals(new Vector3(0, 6, 0))).toBe(true);
  });
});

// M5-PRE-01
describe("Prefab", () => {
  test("captures an object tree, places independent copies with inner references, and round-trips as data", () => {
    const enemy = new GameObject("Enemy", { position: new Vector3(9, 9, 9) });
    const gunObject = new GameObject("Gun", { parent: enemy, position: new Vector3(0, 1, 0) });
    const turret = gunObject.addComponent(Turret, { range: 30 });
    const outsider = new GameObject("Outsider");
    enemy.addComponent(Health, { max: 80, gun: turret, target: outsider });
    const prefab = Prefab.fromObject(enemy);
    expect(prefab.name).toBe("Enemy");
    // Placed copies: at the given spot, their own gun, the outside reference dropped.
    const a = GameObject.instantiate(prefab, { position: new Vector3(1, 0, 0) });
    const b = prefab.instantiate({ position: new Vector3(-1, 0, 0) });
    expect(a).not.toBe(b);
    expect(a.transform.localPosition.x).toBe(1);
    const ha = a.getComponent(Health)!;
    expect(ha.max).toBe(80);
    expect(ha.gun).toBe(a.findChild("Gun")!.getComponent(Turret)!);
    expect(ha.gun).not.toBe(turret);
    expect(ha.target).toBeNull();
    // Changing the original later doesn't change the prefab.
    enemy.getComponent(Health)!.max = 5;
    expect(prefab.instantiate().getComponent(Health)!.max).toBe(80);
    // As data (a .prefab.json file), and placed copies from a file keep its path when saved.
    const again = Prefab.fromData(JSON.parse(JSON.stringify(prefab.toJSON())), "prefabs/enemy.prefab.json");
    const c = again.instantiate({ parent: outsider });
    expect(c.parent).toBe(outsider);
    expect(c.findChild("Gun")!.getComponent(Turret)!.range).toBe(30);
    const saved = SceneManager.getActiveScene().serialize();
    const out = saved.objects.find((o) => o.name === "Outsider")!.children![0]!;
    expect(out.prefab).toBe("prefabs/enemy.prefab.json");
    expect(() => Prefab.fromData({ format: "iron.scene" })).toThrow("not prefab data");
  });
});
