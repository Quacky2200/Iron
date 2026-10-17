// Scenes (Unity's `SceneManager`, s&box's scene files): every root GameObject belongs to
// a scene. Scenes save to plain JSON (`SceneData`): objects with their transforms,
// children and components, each component with its `@property` values, plus the
// render settings and the meshes and textures they share. Loading builds the objects,
// sets every property, then wakes the components (`onAwake`), so components can refer
// to each other in any order.
//
// A scene file is usually bundled (`import level from "./level.scene.json"`) and loaded
// with `SceneManager.loadScene(level)`; `loadSceneAsync("scenes/level.scene.json")`
// reads one from the game folder instead.

import { AudioListener, AudioSource } from "./audio-components.ts";
import { Sound } from "./audio.ts";
import { LineRenderer } from "./lines.ts";
import { Color, Quaternion, Rect, RectInt, Vector2, Vector2Int, Vector3, Vector3Int, Vector4 } from "./math.ts";
import { Model } from "./models.ts";
import { ParticleSystem } from "./particles.ts";
import {
  BoxCollider,
  CapsuleCollider,
  CharacterController,
  CharacterJoint,
  FixedJoint,
  HingeJoint,
  MeshCollider,
  Rigidbody,
  Rope,
  RopeJoint,
  SphereCollider,
  SpringJoint,
} from "./physics-components.ts";
import { BoxCollider2D, CapsuleCollider2D, CircleCollider2D, EdgeCollider2D, PolygonCollider2D, Rigidbody2D } from "./physics2d.ts";
import { propertiesOf } from "./property.ts";
import {
  Camera,
  CameraFollow,
  CameraOrbit,
  Light,
  LODGroup,
  Material,
  Mesh,
  MeshRenderer,
  Mirror,
  PlaneRenderer,
  ReflectionProbe,
  RenderSettings,
  WorldPanel,
} from "./rendering.ts";
import {
  Component,
  GameObject,
  _classByName,
  _nameOf,
  _registerBuiltIns,
  _setSceneHooks,
  type ComponentClass,
  type InstantiateObjectOptions,
} from "./scene.ts";
import { Sprite, SpriteRenderer, Texture } from "./textures.ts";

_registerBuiltIns({
  AudioListener,
  AudioSource,
  BoxCollider,
  BoxCollider2D,
  Camera,
  CameraFollow,
  CameraOrbit,
  CapsuleCollider,
  CapsuleCollider2D,
  CharacterController,
  CharacterJoint,
  CircleCollider2D,
  EdgeCollider2D,
  FixedJoint,
  HingeJoint,
  Light,
  LineRenderer,
  LODGroup,
  MeshCollider,
  MeshRenderer,
  Mirror,
  ParticleSystem,
  PlaneRenderer,
  PolygonCollider2D,
  ReflectionProbe,
  Rigidbody,
  Rigidbody2D,
  Rope,
  RopeJoint,
  SphereCollider,
  SpringJoint,
  SpriteRenderer,
  WorldPanel,
});

// ---- The file format ----

/** A saved scene. */
export interface SceneData {
  format: "iron.scene";
  version: 1;
  name: string;
  /** The lighting it was saved with (applied when it's the active scene). */
  renderSettings?: Record<string, unknown>;
  /** Meshes and textures, shared by id (`{ "$asset": id }`). */
  assets?: Record<string, Record<string, unknown>>;
  objects: GameObjectData[];
}

export interface GameObjectData {
  /** Unique in the file: references point at it. */
  id: number;
  name: string;
  /** Default true. */
  enabled?: boolean;
  /** Default 0. */
  layer?: number;
  /** Local position, rotation (x, y, z, w) and scale. */
  position?: [number, number, number];
  rotation?: [number, number, number, number];
  scale?: [number, number, number];
  /** A model's copy: its nodes are rebuilt from the model, not saved. */
  model?: { src: string; colliders?: string };
  components?: ComponentData[];
  children?: GameObjectData[];
}

export interface ComponentData {
  /** Unique in the file (shared with objects' ids). */
  id: number;
  /** The registered name (`register({ Health })`) or the engine's (`MeshRenderer`). */
  type: string;
  properties?: Record<string, unknown>;
}

// ---- Values ----

const warn = (message: string) => console.warn(`scene: ${message}`);

/** Saving: file ids for objects and components, and shared assets. */
class Saver {
  readonly ids = new Map<GameObject | Component, number>();
  readonly assets: Record<string, Record<string, unknown>> = {};
  private assetIds = new Map<object, string>();
  /** Objects outside what's saved: kept as live references (`instantiate`), else null. */
  constructor(private readonly keepExternal: boolean) {}

  id(thing: GameObject | Component): number {
    let id = this.ids.get(thing);
    if (id === undefined) {
      id = this.ids.size + 1;
      this.ids.set(thing, id);
    }
    return id;
  }

  private asset(key: object, prefix: string, make: () => Record<string, unknown>): { $asset: string } {
    let id = this.assetIds.get(key);
    if (!id) {
      id = `${prefix}${this.assetIds.size + 1}`;
      this.assetIds.set(key, id);
      this.assets[id] = make();
    }
    return { $asset: id };
  }

  value(v: unknown, where: string): unknown {
    if (v === null || v === undefined || typeof v === "number" || typeof v === "string" || typeof v === "boolean") {
      return v ?? null;
    }
    if (Array.isArray(v)) return v.map((x, i) => this.value(x, `${where}[${i}]`));
    if (v instanceof Vector2Int) return { $type: "Vector2Int", value: [v.x, v.y] };
    if (v instanceof Vector3Int) return { $type: "Vector3Int", value: [v.x, v.y, v.z] };
    if (v instanceof Vector2) return { $type: "Vector2", value: [v.x, v.y] };
    if (v instanceof Vector3) return { $type: "Vector3", value: [v.x, v.y, v.z] };
    if (v instanceof Vector4) return { $type: "Vector4", value: [v.x, v.y, v.z, v.w] };
    if (v instanceof Quaternion) return { $type: "Quaternion", value: [v.x, v.y, v.z, v.w] };
    if (v instanceof Color) return { $type: "Color", value: [v.r, v.g, v.b, v.a] };
    if (v instanceof RectInt) return { $type: "RectInt", value: [v.x, v.y, v.width, v.height] };
    if (v instanceof Rect) return { $type: "Rect", value: [v.x, v.y, v.width, v.height] };
    if (v instanceof GameObject || v instanceof Component) {
      const id = this.ids.get(v);
      const kind = v instanceof GameObject ? "GameObject" : "Component";
      if (id !== undefined) return { $ref: kind, id };
      if (this.keepExternal) return { $ref: kind, live: v instanceof GameObject ? v.id : v.id };
      return null;
    }
    if (v instanceof Material) {
      return {
        $type: "Material",
        color: this.value(v.color, where),
        emission: this.value(v.emission, where),
        smoothness: v.smoothness,
        metallic: v.metallic,
        unlit: v.unlit,
        mainTexture: this.value(v.mainTexture, where),
        mainTextureScale: this.value(v.mainTextureScale, where),
        mainTextureOffset: this.value(v.mainTextureOffset, where),
      };
    }
    if (v instanceof Texture) {
      if (!v.src) {
        warn(`${where}: only textures loaded from files are saved`);
        return null;
      }
      const t = v as unknown as { _mipmaps: boolean };
      return this.asset(v, "texture", () => ({ type: "Texture", src: v.src, filterMode: v.filterMode, wrapMode: v.wrapMode, mipmaps: t._mipmaps }));
    }
    if (v instanceof Sprite) {
      return { $type: "Sprite", texture: this.value(v.texture, where), rect: this.value(v.rect, where), pivot: this.value(v.pivot, where), pixelsPerUnit: v.pixelsPerUnit };
    }
    if (v instanceof Mesh) {
      const data = v._data;
      if (!data) {
        warn(`${where}: only meshes made with Mesh.create are saved (model meshes come with their model)`);
        return null;
      }
      return this.asset(v, "mesh", () => ({ type: "Mesh", ...data }));
    }
    if (v instanceof Sound) return { $type: "Sound", src: v.src };
    const proto = Object.getPrototypeOf(v);
    if (proto === Object.prototype || proto === null) {
      const out: Record<string, unknown> = {};
      for (const [k, x] of Object.entries(v as Record<string, unknown>)) {
        if (x !== undefined) out[k] = this.value(x, `${where}.${k}`);
      }
      return out;
    }
    warn(`${where}: can't save a ${proto?.constructor?.name ?? "value"}`);
    return null;
  }
}

/** Loading: file ids to what they became, and shared assets. */
class Loader {
  readonly things = new Map<number, GameObject | Component>();
  private assets = new Map<string, unknown>();
  constructor(private readonly data: Record<string, Record<string, unknown>> = {}) {}

  private asset(id: string): unknown {
    if (this.assets.has(id)) return this.assets.get(id);
    const a = this.data[id];
    let made: unknown = null;
    if (a?.type === "Texture") {
      made = Texture.load(String(a.src), {
        filterMode: a.filterMode as never,
        wrapMode: a.wrapMode as never,
        mipmaps: a.mipmaps as boolean | undefined,
      });
    } else if (a?.type === "Mesh") {
      const flat = (k: string) => (a[k] as number[] | undefined) ?? [];
      const v3 = (f: number[]) => Array.from({ length: f.length / 3 }, (_, i) => new Vector3(f[i * 3], f[i * 3 + 1], f[i * 3 + 2]));
      const uv = flat("uv");
      made = Mesh.create({
        vertices: v3(flat("vertices")),
        triangles: flat("triangles"),
        normals: v3(flat("normals")),
        uv: Array.from({ length: uv.length / 2 }, (_, i) => new Vector2(uv[i * 2], uv[i * 2 + 1])),
      });
    } else {
      warn(`unknown asset ${id}`);
    }
    this.assets.set(id, made);
    return made;
  }

  value(v: unknown): unknown {
    if (v === null || typeof v !== "object") return v;
    if (Array.isArray(v)) return v.map((x) => this.value(x));
    const o = v as Record<string, unknown>;
    if (typeof o.$asset === "string") return this.asset(o.$asset);
    if (typeof o.$ref === "string") {
      if (typeof o.live === "number") {
        return o.$ref === "GameObject" ? (GameObject.byId(o.live) ?? null) : (liveComponent(o.live) ?? null);
      }
      const found = this.things.get(o.id as number);
      return found ?? null;
    }
    if (typeof o.$type === "string") {
      const n = (o.value as number[] | undefined) ?? [];
      switch (o.$type) {
        case "Vector2":
          return new Vector2(n[0], n[1]);
        case "Vector2Int":
          return new Vector2Int(n[0], n[1]);
        case "Vector3":
          return new Vector3(n[0], n[1], n[2]);
        case "Vector3Int":
          return new Vector3Int(n[0], n[1], n[2]);
        case "Vector4":
          return new Vector4(n[0], n[1], n[2], n[3]);
        case "Quaternion":
          return new Quaternion(n[0], n[1], n[2], n[3]);
        case "Color":
          return new Color(n[0], n[1], n[2], n[3]);
        case "Rect":
          return new Rect(n[0], n[1], n[2], n[3]);
        case "RectInt":
          return new RectInt(n[0], n[1], n[2], n[3]);
        case "Material":
          return Material.create({
            color: this.value(o.color) as Color,
            emission: this.value(o.emission) as Color,
            smoothness: o.smoothness as number,
            metallic: o.metallic as number,
            unlit: o.unlit as boolean,
            mainTexture: this.value(o.mainTexture) as Texture | null,
            mainTextureScale: this.value(o.mainTextureScale) as Vector2,
            mainTextureOffset: this.value(o.mainTextureOffset) as Vector2,
          });
        case "Sprite":
          return Sprite.create(this.value(o.texture) as Texture, this.value(o.rect) as Rect | null, this.value(o.pivot) as Vector2, o.pixelsPerUnit as number);
        case "Sound":
          return new Sound(String(o.src));
        default:
          warn(`unknown type ${o.$type}`);
          return null;
      }
    }
    const out: Record<string, unknown> = {};
    for (const [k, x] of Object.entries(o)) out[k] = this.value(x);
    return out;
  }
}

/** A live component by its id (for `instantiate`'s references outside the copy). */
function liveComponent(id: number): Component | undefined {
  for (const root of GameObject.roots) {
    const stack = [root];
    while (stack.length) {
      const o = stack.pop()!;
      const c = o.components.find((c) => c.id === id);
      if (c) return c;
      stack.push(...o.children);
    }
  }
  return undefined;
}

// ---- Objects ----

/** Gives every object and component under `roots` a file id (so references work both ways). */
function number(saver: Saver, roots: readonly GameObject[]): void {
  const walk = (go: GameObject) => {
    if (go._modelPart) return;
    saver.id(go);
    for (const c of go.components) saver.id(c);
    for (const child of go.children) walk(child);
  };
  for (const r of roots) walk(r);
}

function saveObject(saver: Saver, go: GameObject): GameObjectData | null {
  if (go._modelPart) return null;
  const t = go.transform;
  const p = t.localPosition;
  const r = t.localRotation;
  const s = t.localScale;
  const data: GameObjectData = {
    id: saver.id(go),
    name: go.name,
    position: [p.x, p.y, p.z],
    rotation: [r.x, r.y, r.z, r.w],
    scale: [s.x, s.y, s.z],
  };
  if (!go.enabled) data.enabled = false;
  if (go.layer !== 0) data.layer = go.layer;
  if (go._model) data.model = { ...go._model };
  const components = go.components.map((c): ComponentData => {
    const cls = c.constructor as ComponentClass;
    const properties: Record<string, unknown> = {};
    for (const name of propertiesOf(cls)) {
      const value = (c as unknown as Record<string, unknown>)[name];
      if (value !== undefined) properties[name] = saver.value(value, `${go.name}.${_nameOf(cls)}.${name}`);
    }
    return { id: saver.id(c), type: _nameOf(cls), properties };
  });
  if (components.length) data.components = components;
  const children = go.children.map((c) => saveObject(saver, c)).filter((c): c is GameObjectData => c !== null);
  if (children.length) data.children = children;
  return data;
}

/**
 * Builds objects (under `parent`, or as roots of the active scene), then sets their
 * components' properties, then wakes the components: Unity's order.
 */
function loadObjects(list: readonly GameObjectData[], assets: SceneData["assets"], parent: GameObject | null): GameObject[] {
  const loader = new Loader(assets);
  const pending: { component: Component; properties: Record<string, unknown> }[] = [];
  const disabled: GameObject[] = [];
  const build = (d: GameObjectData, under: GameObject | null): GameObject => {
    const go = new GameObject(d.name ?? "GameObject", {
      parent: under,
      position: d.position ? new Vector3(...d.position) : undefined,
      rotation: d.rotation ? new Quaternion(...d.rotation) : undefined,
      scale: d.scale ? new Vector3(...d.scale) : undefined,
    });
    loader.things.set(d.id, go);
    if (d.layer) go.layer = d.layer;
    if (d.enabled === false) disabled.push(go);
    if (d.model) Model.load(d.model.src)._buildInto(go, { colliders: d.model.colliders as never });
    for (const c of d.components ?? []) {
      const cls = _classByName(c.type);
      if (!cls) {
        warn(`${d.name}: unknown component ${c.type} (register it with register({ ${c.type} }))`);
        continue;
      }
      const component = go._addComponentAsleep(cls);
      loader.things.set(c.id, component);
      pending.push({ component, properties: c.properties ?? {} });
    }
    for (const child of d.children ?? []) build(child, go);
    return go;
  };
  const made = list.map((d) => build(d, parent));
  for (const { component, properties } of pending) {
    for (const [name, value] of Object.entries(properties)) {
      try {
        (component as unknown as Record<string, unknown>)[name] = loader.value(value);
      } catch (e) {
        warn(`${component.gameObject.name}: couldn't set ${name}: ${e}`);
      }
    }
  }
  for (const { component } of pending) component.onAwake?.();
  for (const go of disabled) go.enabled = false;
  return made;
}

// ---- Scenes ----

let nextHandle = 1;

/** A set of root objects loaded, saved and unloaded together (Unity's `Scene`). */
export class Scene {
  readonly handle: number;
  /** Where it was loaded from (`null`: made in code or from bundled data). */
  path: string | null = null;
  /** False once unloaded. */
  isLoaded = true;
  /** @internal The lighting it was loaded with, applied when it's made active. */
  _renderSettings: Record<string, unknown> | null = null;

  constructor(public name: string) {
    this.handle = nextHandle++;
  }

  /** Its root objects, in order. */
  getRootGameObjects(): GameObject[] {
    return GameObject.roots.filter((go) => go._scene === this);
  }

  get rootCount(): number {
    return this.getRootGameObjects().length;
  }

  /** Everything in it as `SceneData` (JSON-ready), with the render settings if it's active. */
  serialize(): SceneData {
    const saver = new Saver(false);
    const roots = this.getRootGameObjects();
    number(saver, roots);
    const objects = roots.map((go) => saveObject(saver, go)).filter((d): d is GameObjectData => d !== null);
    const data: SceneData = { format: "iron.scene", version: 1, name: this.name, objects };
    if (this === active) data.renderSettings = saveRenderSettings(saver);
    if (Object.keys(saver.assets).length) data.assets = saver.assets;
    return data;
  }
}

function saveRenderSettings(saver: Saver): Record<string, unknown> {
  return saver.value(
    {
      ambientLight: RenderSettings.ambientLight,
      ambientMode: RenderSettings.ambientMode,
      reflectionIntensity: RenderSettings.reflectionIntensity,
      skybox: RenderSettings.skybox,
      bloom: RenderSettings.bloom,
    },
    "renderSettings",
  ) as Record<string, unknown>;
}

function applyRenderSettings(saved: Record<string, unknown>): void {
  const s = new Loader().value(saved) as Record<string, unknown>;
  if (s.ambientLight instanceof Color) RenderSettings.ambientLight = s.ambientLight;
  if (s.ambientMode === "skybox" || s.ambientMode === "color") RenderSettings.ambientMode = s.ambientMode;
  if (typeof s.reflectionIntensity === "number") RenderSettings.reflectionIntensity = s.reflectionIntensity;
  if ("skybox" in s) RenderSettings.skybox = (s.skybox as never) ?? null;
  if ("bloom" in s) RenderSettings.bloom = (s.bloom as never) ?? null;
}

export type LoadSceneMode = "single" | "additive";

let scenes: Scene[] = [];
let active: Scene;
let keep: Scene | null = null;
const loadedListeners = new Set<(scene: Scene, mode: LoadSceneMode) => void>();
const unloadedListeners = new Set<(scene: Scene) => void>();
const activeListeners = new Set<(previous: Scene, next: Scene) => void>();

function reset(): void {
  scenes = [new Scene("Main")];
  active = scenes[0]!;
  keep = null;
  loadedListeners.clear();
  unloadedListeners.clear();
  activeListeners.clear();
}
reset();

function setActive(scene: Scene): void {
  if (scene === active) return;
  const previous = active;
  active = scene;
  if (scene._renderSettings) applyRenderSettings(scene._renderSettings);
  for (const f of activeListeners) f(previous, scene);
}

function unload(scene: Scene): void {
  for (const go of scene.getRootGameObjects()) go.destroy();
  scene.isLoaded = false;
  scenes = scenes.filter((s) => s !== scene);
  for (const f of unloadedListeners) f(scene);
}

function fromData(data: SceneData, mode: LoadSceneMode, path: string | null): Scene {
  if (data?.format !== "iron.scene") throw new Error("not a scene (format isn't iron.scene)");
  if (data.version !== 1) throw new Error(`scene version ${data.version} isn't supported`);
  const scene = new Scene(data.name);
  scene.path = path;
  scene._renderSettings = data.renderSettings ?? null;
  const old = mode === "single" ? [...scenes] : [];
  scenes.push(scene);
  for (const s of old) unload(s);
  // New objects go into the active scene: make it this one while building.
  const previous = active;
  active = scene;
  try {
    loadObjects(data.objects ?? [], data.assets, null);
  } finally {
    active = previous;
  }
  if (mode === "single") setActive(scene);
  for (const f of loadedListeners) f(scene, mode);
  return scene;
}

// Text files read from the game folder, for `loadSceneAsync(path)`.
let nextRead = 1;
const reads = new Map<number, { resolve: (text: string) => void; reject: (e: Error) => void; path: string }>();

declare global {
  var __iron_asset_text: ((results: (number | string | null)[]) => void) | undefined;
}

globalThis.__iron_asset_text = (results) => {
  for (let i = 0; i + 1 < results.length; i += 2) {
    const read = reads.get(results[i] as number);
    if (!read) continue;
    reads.delete(results[i] as number);
    const text = results[i + 1];
    if (typeof text === "string") read.resolve(text);
    else read.reject(new Error(`couldn't read ${read.path}`));
  }
};

function readText(path: string): Promise<string> {
  const id = nextRead++;
  return new Promise((resolve, reject) => {
    reads.set(id, { resolve, reject, path });
    __iron_op("asset.readText", id, path);
  });
}

export const SceneManager = {
  /** Loaded scenes (not counting `DontDestroyOnLoad`). */
  get sceneCount(): number {
    return scenes.length;
  },

  getSceneAt(index: number): Scene {
    const s = scenes[index];
    if (!s) throw new RangeError(`no scene at ${index}`);
    return s;
  },

  getSceneByName(name: string): Scene | undefined {
    return scenes.find((s) => s.name === name);
  },

  /** The scene new objects go into, whose lighting is used. */
  getActiveScene(): Scene {
    return active;
  },

  /** Makes a loaded scene active (false if it isn't loaded). */
  setActiveScene(scene: Scene): boolean {
    if (!scenes.includes(scene)) return false;
    setActive(scene);
    return true;
  },

  /** A new empty scene, loaded alongside the others. */
  createScene(name: string): Scene {
    if (scenes.some((s) => s.name === name)) throw new Error(`a scene named ${name} is already loaded`);
    const scene = new Scene(name);
    scenes.push(scene);
    return scene;
  },

  /**
   * Loads saved scene data. `single` (default) unloads every other scene first (but
   * not `DontDestroyOnLoad` objects) and makes it active; `additive` adds it.
   */
  loadScene(data: SceneData, mode: LoadSceneMode = "single"): Scene {
    return fromData(data, mode, null);
  },

  /** Loads a scene file from the game folder (or saved data) once it has been read. */
  async loadSceneAsync(source: string | SceneData, mode: LoadSceneMode = "single"): Promise<Scene> {
    if (typeof source !== "string") return fromData(source, mode, null);
    const data = JSON.parse(await readText(source)) as SceneData;
    return fromData(data, mode, source);
  },

  /** Destroys a scene's objects and drops it. The last scene can't be unloaded. */
  unloadScene(scene: Scene): boolean {
    if (!scenes.includes(scene) || scenes.length === 1) return false;
    unload(scene);
    if (active === scene) setActive(scenes[0]!);
    return true;
  },

  /** Moves a root object (with its children) to another loaded scene. */
  moveGameObjectToScene(go: GameObject, scene: Scene): void {
    if (go.parent) throw new Error("only root objects move between scenes");
    if (!scene.isLoaded) throw new Error(`scene ${scene.name} isn't loaded`);
    go._scene = scene;
  },

  /** Moves every object of `source` into `destination` and unloads `source`. */
  mergeScenes(source: Scene, destination: Scene): void {
    for (const go of source.getRootGameObjects()) go._scene = destination;
    SceneManager.unloadScene(source);
  },

  /** Called after a scene loads; returns a function that stops it. */
  onSceneLoaded(f: (scene: Scene, mode: LoadSceneMode) => void): () => void {
    loadedListeners.add(f);
    return () => loadedListeners.delete(f);
  },

  onSceneUnloaded(f: (scene: Scene) => void): () => void {
    unloadedListeners.add(f);
    return () => unloadedListeners.delete(f);
  },

  onActiveSceneChanged(f: (previous: Scene, next: Scene) => void): () => void {
    activeListeners.add(f);
    return () => activeListeners.delete(f);
  },
};

/**
 * Copies an object with its children and components (Unity's `Instantiate`):
 * references inside the copy point at the copies, others at the originals.
 */
function instantiate(original: GameObject, options: InstantiateObjectOptions = {}): GameObject {
  const saver = new Saver(true);
  number(saver, [original]);
  const data = saveObject(saver, original);
  if (!data) throw new Error("parts of a model can't be copied on their own; copy the model's root");
  if (!options.parent) {
    // Where the original is in the world.
    const t = original.transform;
    const [p, r, s] = [t.position, t.rotation, t.lossyScale];
    data.position = [p.x, p.y, p.z];
    data.rotation = [r.x, r.y, r.z, r.w];
    data.scale = [s.x, s.y, s.z];
  }
  if (options.position) data.position = [options.position.x, options.position.y, options.position.z];
  if (options.rotation) data.rotation = [options.rotation.x, options.rotation.y, options.rotation.z, options.rotation.w];
  const previous = active;
  // A root copy goes into the original's scene.
  if (!options.parent) active = original.scene;
  try {
    return loadObjects([data], saver.assets, options.parent ?? null)[0]!;
  } finally {
    active = previous;
  }
}

/**
 * The game's start scene (`iron.startScene` in its `package.json`), loaded once the
 * file has been read. It loads next to the objects of the starting components and
 * becomes active; the `Main` scene goes if nothing is in it.
 */
async function loadStartScene(path: string): Promise<void> {
  const main = active;
  try {
    const scene = await SceneManager.loadSceneAsync(path, "additive");
    SceneManager.setActiveScene(scene);
    if (main.isLoaded && main.rootCount === 0) SceneManager.unloadScene(main);
  } catch (e) {
    console.error(`start scene ${path}: ${e}`);
  }
}

const startScene = globalThis.__iron_info?.app.startScene;
if (startScene && typeof __iron_op === "function") void loadStartScene(startScene);

_setSceneHooks({
  active: () => active,
  dontDestroyOnLoad(go) {
    const root = go.root;
    keep ??= new Scene("DontDestroyOnLoad");
    root._scene = keep;
  },
  instantiate,
  reset,
});
