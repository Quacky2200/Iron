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

import { Cloth } from "./cloth.ts";
import { Assets } from "./assets.ts";
import { Animation, AnimationClip, AnimationCurve, Animator, AnimatorController } from "./animation.ts";
import { AudioListener, AudioSource } from "./audio-components.ts";
import { Sound } from "./audio.ts";
import { LineRenderer } from "./lines.ts";
import { Color, Matrix4x4, Quaternion, Rect, RectInt, Vector2, Vector2Int, Vector3, Vector3Int, Vector4 } from "./math.ts";
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
import { _restoreQuality, _qualityState } from "./quality.ts";
import { propertiesOf } from "./property.ts";
import { _restoreTime, _timeState } from "./clock.ts";
import {
  Camera,
  CameraFollow,
  CameraOrbit,
  Light,
  LODGroup,
  Material,
  Mesh,
  MeshRenderer,
  SkinnedMeshRenderer,
  Mirror,
  PlaneRenderer,
  ReflectionProbe,
  RenderSettings,
  WorldPanel,
} from "./rendering.ts";
import {
  Component,
  GameObject,
  HideFlags,
  _classByName,
  _isBuiltIn,
  _markRestored,
  _nameOf,
  _registerBuiltIns,
  _setSceneHooks,
  type ComponentClass,
  type InstantiateObjectOptions,
} from "./scene.ts";
import { RenderTexture, Sprite, SpriteRenderer, Texture } from "./textures.ts";

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
  Animation,
  Animator,
  CharacterController,
  CharacterJoint,
  Cloth,
  CircleCollider2D,
  EdgeCollider2D,
  FixedJoint,
  HingeJoint,
  Light,
  LineRenderer,
  LODGroup,
  MeshCollider,
  MeshRenderer,
  SkinnedMeshRenderer,
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
  /** Local position, rotation (x, y, z, w) and scale; left out when 0, identity and 1. */
  position?: [number, number, number];
  rotation?: [number, number, number, number];
  scale?: [number, number, number];
  /** A model's copy: its nodes are rebuilt from the model, not saved. */
  model?: { src: string; colliders?: string; guid?: string };
  /** The prefab file it was placed from (kept for tools; the object is saved in full). */
  prefab?: string;
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

/** A saved asset's file: its GUID's current path when known, else the saved path. */
function fromGuid(guid: unknown, src: unknown): string {
  return (typeof guid === "string" && Assets.guidToPath(guid)) || String(src);
}

/** Bytes as a hex string: half the size of a JSON number array, and quick to read back. */
function toHex(bytes: readonly number[]): string {
  let out = "";
  for (const b of bytes) out += (b < 16 ? "0" : "") + b.toString(16);
  return out;
}

function fromHex(hex: string): number[] {
  const out: number[] = new Array(hex.length >> 1);
  for (let i = 0; i < out.length; i++) out[i] = parseInt(hex.slice(i * 2, i * 2 + 2), 16);
  return out;
}

const warn = (message: string) => console.warn(`scene: ${message}`);

/** What `Saver.value` gives for values it can't save, in reload mode (the field is left out). */
const UNSAVED = Symbol("unsaved");
/** Material settings saved only when they differ from a new material's. */
const MATERIAL_EXTRAS = [
  "normalMap",
  "normalScale",
  "metallicGlossMap",
  "specularColor",
  "occlusionMap",
  "occlusionStrength",
  "emissionMap",
  "detailAlbedoMap",
  "detailAlbedoMapScale",
  "detailAlbedoMapOffset",
  "renderingMode",
  "alphaCutoff",
  "doubleSided",
  "gltfPacking",
] as const;
const DEFAULT_MATERIAL = new Material();

/** Saving: file ids for objects and components, and shared assets. */
class Saver {
  readonly ids = new Map<GameObject | Component, number>();
  readonly assets: Record<string, Record<string, unknown>> = {};
  private assetIds = new Map<object, string>();
  /**
   * `keepExternal`: objects outside what's saved stay live references (`instantiate`),
   * else null. `reload`: also saves script components' plain fields, skipping quietly
   * what can't be saved.
   */
  constructor(
    private readonly keepExternal: boolean,
    readonly reload = false,
  ) {}

  /** A plain field's value for a reload, or `UNSAVED` (functions, engine handles, ...). */
  field(v: unknown, where: string): unknown {
    if (typeof v === "function" || typeof v === "symbol" || typeof v === "bigint") return UNSAVED;
    this.unsaved = false;
    const out = this.value(v, where);
    return this.unsaved ? UNSAVED : out;
  }
  private unsaved = false;

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
    if (v instanceof AnimationCurve) return { $type: "AnimationCurve", data: v._toJSON() };
    if (v instanceof AnimationClip) return { $type: "AnimationClip", data: v.toJSON() };
    if (v instanceof AnimatorController) return { $type: "AnimatorController", data: v.toJSON() };
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
        // The rest only where it differs from the default, keeping files short.
        ...Object.fromEntries(
          MATERIAL_EXTRAS.filter((k) => {
            const d = (DEFAULT_MATERIAL as unknown as Record<string, unknown>)[k];
            const x = (v as unknown as Record<string, unknown>)[k];
            return !(x === d || (x instanceof Vector2 && d instanceof Vector2 && x.equals(d)));
          }).map((k) => [k, this.value((v as unknown as Record<string, unknown>)[k], where)]),
        ),
      };
    }
    if (v instanceof RenderTexture) {
      return this.asset(v, "texture", () => ({ type: "RenderTexture", width: v.width, height: v.height }));
    }
    if (v instanceof Texture) {
      const bytes = v._bytes;
      if (!v.src && !bytes) return this.cannot(`${where}: only textures loaded from files or made with Texture.create are saved`);
      const t = v as unknown as { _mipmaps: boolean };
      const guid = v.src ? Assets.pathToGuid(v.src) : null;
      return this.asset(v, "texture", () => ({
        type: "Texture",
        // From a file, or its pixels (sRGB bytes, top row first, as hex) when made with `Texture.create`.
        ...(v.src ? { src: v.src } : { width: v.width, height: v.height, pixels: toHex(bytes!) }),
        ...(guid ? { guid } : {}),
        filterMode: v.filterMode,
        wrapMode: v.wrapMode,
        mipmaps: t._mipmaps,
        ...(v.anisoLevel !== 1 ? { anisoLevel: v.anisoLevel } : {}),
        ...(v.linear ? { linear: true } : {}),
      }));
    }
    if (v instanceof Sprite) {
      return { $type: "Sprite", texture: this.value(v.texture, where), rect: this.value(v.rect, where), pivot: this.value(v.pivot, where), pixelsPerUnit: v.pixelsPerUnit };
    }
    if (v instanceof Mesh) {
      const data = v._data;
      if (!data) return this.cannot(`${where}: only meshes made with Mesh.create are saved (model meshes come with their model)`);
      return this.asset(v, "mesh", () => ({
        type: "Mesh",
        ...data,
        ...(v.bindposes.length ? { bindposes: v.bindposes.flatMap((m) => [...m.m]) } : {}),
      }));
    }
    if (v instanceof Sound) {
      const guid = Assets.pathToGuid(v.src);
      return { $type: "Sound", src: v.src, ...(guid ? { guid } : {}) };
    }
    const proto = Object.getPrototypeOf(v);
    if (proto === Object.prototype || proto === null) {
      const out: Record<string, unknown> = {};
      for (const [k, x] of Object.entries(v as Record<string, unknown>)) {
        if (x !== undefined) out[k] = this.value(x, `${where}.${k}`);
      }
      return out;
    }
    return this.cannot(`${where}: can't save a ${proto?.constructor?.name ?? "value"}`);
  }

  /** A value it can't save: null with a warning (or, for reload fields, quietly left out). */
  private cannot(message: string): null {
    if (this.reload) this.unsaved = true;
    else warn(message);
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
    if (a?.type === "RenderTexture") {
      made = new RenderTexture(Number(a.width), Number(a.height));
    } else if (a?.type === "Texture") {
      const options = {
        filterMode: a.filterMode as never,
        wrapMode: a.wrapMode as never,
        mipmaps: a.mipmaps as boolean | undefined,
        anisoLevel: a.anisoLevel as number | undefined,
        linear: a.linear as boolean | undefined,
      };
      made =
        typeof a.pixels === "string"
          ? Texture._fromBytes(Number(a.width), Number(a.height), fromHex(a.pixels), options)
          : // By GUID when its file is known, so renamed files are still found.
            Texture.load(fromGuid(a.guid, a.src), options);
    } else if (a?.type === "Mesh") {
      const flat = (k: string) => (a[k] as number[] | undefined) ?? [];
      const v3 = (f: number[]) => Array.from({ length: f.length / 3 }, (_, i) => new Vector3(f[i * 3], f[i * 3 + 1], f[i * 3 + 2]));
      const uv = flat("uv");
      const joints = flat("joints");
      const weights = flat("weights");
      const four = (f: number[], i: number) => [f[i * 4], f[i * 4 + 1], f[i * 4 + 2], f[i * 4 + 3]] as [number, number, number, number];
      const poses = flat("bindposes");
      made = Mesh.create({
        vertices: v3(flat("vertices")),
        triangles: flat("triangles"),
        normals: v3(flat("normals")),
        uv: Array.from({ length: uv.length / 2 }, (_, i) => new Vector2(uv[i * 2], uv[i * 2 + 1])),
        boneWeights: Array.from({ length: joints.length / 4 }, (_, i) => ({ indices: four(joints, i), weights: four(weights, i) })),
        bindposes: Array.from({ length: poses.length / 16 }, (_, i) => new Matrix4x4(poses.slice(i * 16, i * 16 + 16))),
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
        case "AnimationCurve":
          return AnimationCurve._fromJSON(o.data);
        case "AnimationClip":
          return AnimationClip.fromJSON(o.data);
        case "AnimatorController":
          return AnimatorController.fromJSON(o.data);
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
            ...Object.fromEntries(MATERIAL_EXTRAS.filter((k) => k in o).map((k) => [k, this.value(o[k])])),
          });
        case "Sprite":
          return Sprite.create(this.value(o.texture) as Texture, this.value(o.rect) as Rect | null, this.value(o.pivot) as Vector2, o.pixelsPerUnit as number);
        case "Sound":
          return new Sound(fromGuid(o.guid, o.src));
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
    if (go._modelPart || unsaved(go)) return;
    saver.id(go);
    for (const c of go.components) saver.id(c);
    for (const child of go.children) walk(child);
  };
  for (const r of roots) walk(r);
}

/** Objects flagged `DontSave` (in the editor or a build) stay out of saved scenes. */
const unsaved = (go: GameObject) => (go.hideFlags & HideFlags.DontSave) !== 0;

function saveObject(saver: Saver, go: GameObject): GameObjectData | null {
  if (go._modelPart || unsaved(go)) return null;
  const t = go.transform;
  const p = t.localPosition;
  const r = t.localRotation;
  const s = t.localScale;
  const data: GameObjectData = { id: saver.id(go), name: go.name };
  // Defaults are left out: smaller files, faster loads.
  if (p.x !== 0 || p.y !== 0 || p.z !== 0) data.position = [p.x, p.y, p.z];
  if (r.x !== 0 || r.y !== 0 || r.z !== 0 || r.w !== 1) data.rotation = [r.x, r.y, r.z, r.w];
  if (s.x !== 1 || s.y !== 1 || s.z !== 1) data.scale = [s.x, s.y, s.z];
  if (!go.enabled) data.enabled = false;
  if (go.layer !== 0) data.layer = go.layer;
  if (go._model) {
    const guid = Assets.pathToGuid(go._model.src);
    data.model = { ...go._model, ...(guid ? { guid } : {}) };
  }
  if (go._prefab) data.prefab = go._prefab;
  const components = go.components.map((c): ComponentData => {
    const cls = c.constructor as ComponentClass;
    const properties: Record<string, unknown> = {};
    const own = c as unknown as Record<string, unknown>;
    const props = propertiesOf(cls);
    for (const name of props) {
      const value = own[name];
      if (value !== undefined) properties[name] = saver.value(value, `${go.name}.${_nameOf(cls)}.${name}`);
    }
    if (saver.reload && !_isBuiltIn(cls)) {
      // A script component's other fields too, like Unity's reload: counters, timers, state.
      for (const name of Object.keys(own)) {
        if (name === "id" || name === "gameObject" || props.includes(name) || own[name] === undefined) continue;
        const value = saver.field(own[name], `${go.name}.${_nameOf(cls)}.${name}`);
        if (value !== UNSAVED) properties[name] = value;
      }
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
function loadObjects(
  list: readonly GameObjectData[],
  assets: SceneData["assets"],
  parent: GameObject | null,
  reload = false,
): GameObject[] {
  const loader = new Loader(assets);
  const pending: { component: Component; properties: Record<string, unknown> }[] = [];
  const disabled: GameObject[] = [];
  const build = (d: GameObjectData, under: GameObject | null): GameObject => {
    const [p, r, s] = [d.position, d.rotation, d.scale];
    const go = new GameObject(d.name ?? "GameObject", {
      parent: under,
      position: p ? new Vector3(p[0], p[1], p[2]) : undefined,
      rotation: r ? new Quaternion(r[0], r[1], r[2], r[3]) : undefined,
      scale: s ? new Vector3(s[0], s[1], s[2]) : undefined,
    });
    loader.things.set(d.id, go);
    if (d.layer) go.layer = d.layer;
    if (d.enabled === false) disabled.push(go);
    if (d.model) Model.load(fromGuid(d.model.guid, d.model.src))._buildInto(go, { colliders: d.model.colliders as never });
    if (d.prefab) go._prefab = d.prefab;
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
    const own = component as unknown as Record<string, unknown>;
    const cls = component.constructor as ComponentClass;
    const props = propertiesOf(cls);
    for (const [name, value] of Object.entries(properties)) {
      // After a code change a property may be gone (renamed or removed): drop it.
      if (!props.includes(name) && !Object.hasOwn(own, name)) {
        warn(`${component.gameObject.name}: ${_nameOf(cls)} has no ${name} any more; dropped`);
        continue;
      }
      try {
        own[name] = loader.value(value);
      } catch (e) {
        warn(`${component.gameObject.name}: couldn't set ${name}: ${e}`);
      }
    }
  }
  // Script components carried over a reload have already woken and started: they get
  // `onReload` instead. The engine's are rebuilt like any load.
  const restored = new Set(
    reload ? pending.map((p) => p.component).filter((c) => !_isBuiltIn(c.constructor as ComponentClass)) : [],
  );
  for (const component of restored) _markRestored(component);
  for (const { component } of pending) if (!restored.has(component)) component.onAwake?.();
  for (const component of restored) {
    try {
      component.onReload?.();
    } catch (e) {
      console.error(`${component.gameObject.name}: onReload: ${e}`);
    }
  }
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
    return this._serialize(false);
  }

  /** @internal `reload`: script components' plain fields too. */
  _serialize(reload: boolean): SceneData {
    const saver = new Saver(false, reload);
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
      fog: RenderSettings.fog,
      fogMode: RenderSettings.fogMode,
      fogColor: RenderSettings.fogColor,
      fogDensity: RenderSettings.fogDensity,
      fogStartDistance: RenderSettings.fogStartDistance,
      fogEndDistance: RenderSettings.fogEndDistance,
      heightFog: RenderSettings.heightFog,
      cloudLayer: RenderSettings.cloudLayer,
      volumetricFog: RenderSettings.volumetricFog,
      fogCulling: RenderSettings.fogCulling,
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
  // The fog's settings before its switch, so turning it on sends them all.
  if (s.fogMode === "linear" || s.fogMode === "exponential" || s.fogMode === "exponentialSquared") RenderSettings.fogMode = s.fogMode;
  if (s.fogColor instanceof Color) RenderSettings.fogColor = s.fogColor;
  if (typeof s.fogDensity === "number") RenderSettings.fogDensity = s.fogDensity;
  if (typeof s.fogStartDistance === "number") RenderSettings.fogStartDistance = s.fogStartDistance;
  if (typeof s.fogEndDistance === "number") RenderSettings.fogEndDistance = s.fogEndDistance;
  if ("heightFog" in s) RenderSettings.heightFog = (s.heightFog as never) ?? null;
  if ("cloudLayer" in s) RenderSettings.cloudLayer = (s.cloudLayer as never) ?? null;
  if ("volumetricFog" in s) RenderSettings.volumetricFog = (s.volumetricFog as never) ?? null;
  if (typeof s.fogCulling === "boolean") RenderSettings.fogCulling = s.fogCulling;
  if (typeof s.fog === "boolean") RenderSettings.fog = s.fog;
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

/** A saved prefab (Unity's prefab asset): one object tree in the scene file's format. */
export interface PrefabData {
  format: "iron.prefab";
  version: 1;
  name: string;
  root: GameObjectData;
  /** Meshes and textures, shared by id (`{ "$asset": id }`). */
  assets?: Record<string, Record<string, unknown>>;
}

const prefabFiles = new Map<string, Promise<Prefab>>();

/**
 * An object tree to place copies of (Unity's prefab): made from a live object
 * (`Prefab.fromObject`), from data (`fromData`, e.g. a bundled `.prefab.json`), or read
 * from the game folder (`Prefab.load`). Copies (`instantiate`, or
 * `GameObject.instantiate(prefab)`) are independent objects; references inside the
 * prefab point at the copy's own objects.
 */
export class Prefab {
  private constructor(
    private readonly data: PrefabData,
    /** The file it came from, if any. */
    readonly src: string | null,
  ) {}

  /** Captures an object and everything under it as it is now. References to objects outside it are dropped. */
  static fromObject(go: GameObject): Prefab {
    const saver = new Saver(false);
    number(saver, [go]);
    const root = saveObject(saver, go);
    if (!root) throw new Error("parts of a model can't be prefabs on their own; use the model's root");
    // The prefab's own placement starts from the origin.
    delete root.position;
    delete root.rotation;
    const data: PrefabData = { format: "iron.prefab", version: 1, name: go.name, root };
    if (Object.keys(saver.assets).length) data.assets = saver.assets;
    return new Prefab(JSON.parse(JSON.stringify(data)) as PrefabData, null);
  }

  /** A prefab from saved data (`toJSON()`'s output). */
  static fromData(data: unknown, src: string | null = null): Prefab {
    const d = data as Partial<PrefabData>;
    if (d?.format !== "iron.prefab" || !d.root) throw new Error(`not prefab data${src ? `: ${src}` : ""}`);
    return new Prefab(d as PrefabData, src);
  }

  /** Reads a prefab file (JSON) from the game folder, once per path. */
  static load(path: string): Promise<Prefab> {
    let p = prefabFiles.get(path);
    if (!p) {
      p = readText(path).then((text) => Prefab.fromData(JSON.parse(text), path));
      prefabFiles.set(path, p);
      p.catch(() => prefabFiles.delete(path));
    }
    return p;
  }

  get name(): string {
    return this.data.name;
  }

  /** Places a copy: in the active scene (or under `parent`), at `position` / `rotation`. */
  instantiate(options: InstantiateObjectOptions = {}): GameObject {
    const root = JSON.parse(JSON.stringify(this.data.root)) as GameObjectData;
    if (options.position) root.position = [options.position.x, options.position.y, options.position.z];
    if (options.rotation) root.rotation = [options.rotation.x, options.rotation.y, options.rotation.z, options.rotation.w];
    if (this.src) root.prefab = this.src;
    return loadObjects([root], this.data.assets, options.parent ?? null)[0]!;
  }

  /** As JSON-ready data, for a `.prefab.json` file. */
  toJSON(): PrefabData {
    return JSON.parse(JSON.stringify(this.data)) as PrefabData;
  }
}

/**
 * Copies an object with its children and components (Unity's `Instantiate`):
 * references inside the copy point at the copies, others at the originals. A `Prefab`
 * places a copy of it.
 */
function instantiate(original: GameObject | Prefab, options: InstantiateObjectOptions = {}): GameObject {
  if (original instanceof Prefab) return original.instantiate(options);
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

// ---- Hot reload: the whole game's state, saved by the old code and restored by the new ----

interface ReloadState {
  version: 1;
  scenes: SceneData[];
  /** Index of the active scene in `scenes`. */
  active: number;
  keep: SceneData | null;
  quality: unknown;
  time: unknown;
}

function sceneNames(data: SceneData): string[] {
  const out: string[] = [];
  const walk = (o: GameObjectData) => {
    for (const c of o.components ?? []) out.push(c.type);
    for (const child of o.children ?? []) walk(child);
  };
  for (const o of data.objects) walk(o);
  return out;
}

/** Called by the engine on the old code before a reload: everything to carry over, as JSON. */
function saveState(): string {
  const state: ReloadState = {
    version: 1,
    scenes: scenes.map((s) => s._serialize(true)),
    active: scenes.indexOf(active),
    keep: keep && keep.rootCount > 0 ? keep._serialize(true) : null,
    quality: _qualityState(),
    time: _timeState(),
  };
  return JSON.stringify(state);
}

/**
 * Called by the engine on the new code instead of starting components: rebuilds the
 * saved scenes. False (nothing changed) when a saved component class isn't registered
 * any more, so the engine starts the game over instead.
 */
function restoreState(json: string): boolean {
  const state = JSON.parse(json) as ReloadState;
  const all = [...state.scenes, ...(state.keep ? [state.keep] : [])];
  const missing = [...new Set(all.flatMap(sceneNames))].filter((name) => !_classByName(name));
  if (missing.length) {
    warn(
      `can't keep state across the reload: ${missing.join(", ")} ${missing.length === 1 ? "isn't" : "aren't"} registered ` +
        `(register({ ${missing.join(", ")} }, { spawn: false })); starting over`,
    );
    // Starting over includes the start scene, held back while the reload was pending.
    if (startScene) void loadStartScene(startScene);
    return false;
  }
  _restoreQuality(state.quality);
  _restoreTime(state.time);
  // The fresh `Main` scene is replaced by the saved ones.
  for (const s of scenes) s.isLoaded = false;
  scenes = [];
  const build = (data: SceneData): Scene => {
    const scene = new Scene(data.name);
    scene._renderSettings = data.renderSettings ?? null;
    const previous = active;
    active = scene;
    try {
      loadObjects(data.objects ?? [], data.assets, null, true);
    } finally {
      active = previous;
    }
    return scene;
  };
  scenes = state.scenes.map(build);
  if (state.keep) keep = build(state.keep);
  if (!scenes.length) scenes = [new Scene("Main")];
  active = scenes[state.active] ?? scenes[0]!;
  if (active._renderSettings) applyRenderSettings(active._renderSettings);
  return true;
}

declare global {
  var __iron_save_state: (() => string) | undefined;
  var __iron_restore_state: ((json: string) => boolean) | undefined;
  /** Set by the engine while a reload with saved state loads the new bundle. */
  var __iron_reloading: boolean | undefined;
}
globalThis.__iron_save_state = saveState;
globalThis.__iron_restore_state = restoreState;

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
// Not after a reload that keeps state: the saved scenes come back instead.
if (startScene && typeof __iron_op === "function" && !globalThis.__iron_reloading) void loadStartScene(startScene);

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
