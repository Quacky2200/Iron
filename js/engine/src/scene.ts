// GameObjects and components, the s&box / Unity way. Rust owns the objects (crates/
// iron_scene); scripts hold `GameObject` handles with numeric IDs and a mirror of each
// object's name, parent and local transform, so reads need no round trip. Changes go
// to the engine as `scene.*` ops. When the engine moves an object (physics), Rust
// sends its new local transform back with `__iron_scene(moved)` after the tick.
//
// Components are script classes attached to GameObjects. The engine keeps the list
// and calls `onStart`/`onUpdate`/`onDestroy` by (component id, method name) through
// the host functions at the end of this file, skipping components on inactive objects.

import { checkLayer } from "./layers.ts";
import type { Scene } from "./scenes.ts";
import { Quaternion, type QuaternionLike, Vector3, type Vector3Like } from "./math.ts";

export abstract class Component {
  /** Set by the engine when the component is created. */
  readonly id!: number;
  /** The object this component is attached to. */
  readonly gameObject!: GameObject;

  /** The object's transform. */
  get transform(): Transform {
    return this.gameObject.transform;
  }

  /**
   * Called as soon as the component is added (its `addComponent` props already set),
   * before `onStart`: set up things other components may look for right away.
   */
  onAwake?(): void;

  /** Called once, before any other update. */
  onStart?(): void;

  /**
   * Called every frame while the object is active. `dt` is the frame's length in seconds
   * (`Time.deltaTime`, scaled by `Time.timeScale`): use it for movement, input and UI.
   */
  onUpdate?(dt: number): void;

  /** Called every frame after every component's `onUpdate` (cameras that follow things). */
  onLateUpdate?(dt: number): void;

  /** Called on each fixed 60 Hz tick (`Time.fixedDeltaTime`), with physics: forces and steady simulation. */
  onFixedUpdate?(dt: number): void;

  /** Called once per frame just before it's drawn, after layout: last-moment visual changes. */
  onPreRender?(): void;

  /** Called when the component or its object is removed. */
  onDestroy?(): void;

  /**
   * Called instead of `onAwake` / `onStart` when hot reload brings the component back
   * with its saved fields: re-attach event listeners and callbacks here.
   */
  onReload?(): void;

  /** Another component on the same object. */
  getComponent<T extends Component>(cls: ComponentClass<T>): T | undefined {
    return this.gameObject.getComponent(cls);
  }

  /** Removes this component from its object. */
  destroy(): void {
    this.gameObject.removeComponent(this);
  }
}

export type ComponentClass<T extends Component = Component> = new () => T;

const classes = new Map<string, ComponentClass>();
const names = new Map<ComponentClass, string>();
/** Registered classes the engine starts one of each (`spawn`, the default). */
const starting = new Set<string>();
/** The engine's own component classes, by name (they aren't starting components). */
const builtIns = new Map<string, ComponentClass>();
const instances = new Map<number, Component>();
const objects = new Map<number, GameObject>();
const roots: GameObject[] = [];

// Script-chosen ids. The engine's own (components it spawns, objects it creates for
// them) start at 2^30, so the two never meet.
let nextObject = 1;
let nextComponent = 1 << 24;

/**
 * Registers component classes by name, so scene files can use them. Using an object
 * (`register({ Hello })`) keeps names stable even if the bundle is minified. The engine
 * also starts one of each on its own object, unless `spawn` is false (components that
 * only live in scenes).
 */
export function register(components: Record<string, ComponentClass>, options: { spawn?: boolean } = {}): void {
  for (const [name, cls] of Object.entries(components)) {
    classes.set(name, cls);
    names.set(cls, name);
    if (options.spawn === false) starting.delete(name);
    else starting.add(name);
  }
}

/** @internal Names the engine's own components, so scenes can find them by name. */
export function _registerBuiltIns(components: Record<string, ComponentClass>): void {
  for (const [name, cls] of Object.entries(components)) {
    builtIns.set(name, cls);
    if (!names.has(cls)) names.set(cls, name);
  }
}

/** @internal A component class by its saved name (`undefined`: unknown). */
export function _classByName(name: string): ComponentClass | undefined {
  return classes.get(name) ?? builtIns.get(name);
}

/** @internal Whether a class is one of the engine's own components. */
export function _isBuiltIn(cls: ComponentClass): boolean {
  const name = names.get(cls);
  return name !== undefined && builtIns.get(name) === cls;
}

/** Components brought back by a reload: they've started already. */
const restored = new Set<number>();

/** @internal Marks a component as restored by a reload (no `onStart`). */
export function _markRestored(component: Component): void {
  restored.add(component.id);
}

/** @internal A component's saved name. */
export function _nameOf(cls: ComponentClass): string {
  return nameOf(cls);
}

/** @internal How scenes.ts tells new root objects which scene they're in. */
export interface SceneHooks {
  /** The scene new objects go into (the active one). */
  active(): Scene;
  dontDestroyOnLoad(go: GameObject): void;
  instantiate(original: GameObject, options: InstantiateObjectOptions): GameObject;
  reset(): void;
}

export interface InstantiateObjectOptions {
  /** Put the copy under this object (keeping the original's local transform). */
  parent?: GameObject | null;
  position?: Vector3Like;
  rotation?: QuaternionLike;
}
let sceneHooks: SceneHooks | null = null;

/** @internal */
export function _setSceneHooks(hooks: SceneHooks): void {
  sceneHooks = hooks;
}

/**
 * The registered name of a class. Unregistered classes (like the engine's own
 * `Camera`) go by their class name; they aren't starting components, so they stay
 * out of `__iron_registered`.
 */
function nameOf(cls: ComponentClass): string {
  let name = names.get(cls);
  if (!name) {
    name = cls.name;
    if (!name) throw new Error("component classes need a name; register them with register({ ... })");
    names.set(cls, name);
  }
  return name;
}

function instantiate(cls: ComponentClass, id: number, object: GameObject): Component {
  const instance = new cls();
  Object.assign(instance, { id, gameObject: object });
  instances.set(id, instance);
  object.components.push(instance);
  return instance;
}

const vec = (v: Vector3Like) => [v.x, v.y, v.z];
const quat = (q: QuaternionLike) => [q.x, q.y, q.z, q.w];

/**
 * @internal Told when an object's world scale may have changed (its `localScale` was set,
 * or it moved to another parent): colliders resize with it.
 */
export const scaleListeners: ((go: GameObject) => void)[] = [];

/** Position, rotation and scale of a GameObject, like Unity's `Transform`. */
export class Transform {
  /** @internal */
  _position = Vector3.zero;
  /** @internal */
  _rotation = Quaternion.identity;
  /** @internal */
  _scale = Vector3.one;

  constructor(readonly gameObject: GameObject) {}

  private send(): void {
    __iron_op("scene.local", this.gameObject.id, vec(this._position), quat(this._rotation), vec(this._scale));
  }

  /** @internal Sets the local transform without telling the engine (it told us). */
  _set(position: Vector3, rotation: Quaternion, scale: Vector3): void {
    this._position = position;
    this._rotation = rotation;
    this._scale = scale;
  }

  /** Position relative to the parent (the world, for a root object). */
  get localPosition(): Vector3 {
    return this._position;
  }
  set localPosition(v: Vector3Like) {
    this._position = Vector3.from(v);
    this.send();
  }

  get localRotation(): Quaternion {
    return this._rotation;
  }
  set localRotation(q: QuaternionLike) {
    this._rotation = Quaternion.from(q).normalized;
    this.send();
  }

  get localScale(): Vector3 {
    return this._scale;
  }
  set localScale(v: Vector3Like) {
    const old = this._scale;
    this._scale = Vector3.from(v);
    this.send();
    if (!old.equals(this._scale)) for (const f of scaleListeners) f(this.gameObject);
  }

  /** Local rotation as Euler angles in degrees. */
  get localEulerAngles(): Vector3 {
    return this._rotation.eulerAngles;
  }
  set localEulerAngles(v: Vector3Like) {
    this.localRotation = Quaternion.euler(v);
  }

  private parentWorld(): { position: Vector3; rotation: Quaternion; scale: Vector3 } {
    const p = this.gameObject.parent?.transform;
    return p
      ? { position: p.position, rotation: p.rotation, scale: p.lossyScale }
      : { position: Vector3.zero, rotation: Quaternion.identity, scale: Vector3.one };
  }

  /** World position. */
  get position(): Vector3 {
    const p = this.parentWorld();
    return p.position.add(p.rotation.rotate(p.scale.mul(this._position)));
  }
  set position(v: Vector3Like) {
    const p = this.parentWorld();
    this._position = p.rotation.inverse().rotate(Vector3.from(v).sub(p.position)).div(p.scale);
    this.send();
  }

  /** World rotation. */
  get rotation(): Quaternion {
    return this.parentWorld().rotation.mul(this._rotation);
  }
  set rotation(q: QuaternionLike) {
    this._rotation = this.parentWorld().rotation.inverse().mul(q).normalized;
    this.send();
  }

  /** World rotation as Euler angles in degrees. */
  get eulerAngles(): Vector3 {
    return this.rotation.eulerAngles;
  }
  set eulerAngles(v: Vector3Like) {
    this.rotation = Quaternion.euler(v);
  }

  /** World scale (approximate under rotated, non-uniformly scaled parents, like Unity's). */
  get lossyScale(): Vector3 {
    return this.parentWorld().scale.mul(this._scale);
  }

  /** The object's forward (-z), right (+x) and up (+y) directions in the world. */
  get forward(): Vector3 {
    return this.rotation.rotate(Vector3.forward);
  }
  get right(): Vector3 {
    return this.rotation.rotate(Vector3.right);
  }
  get up(): Vector3 {
    return this.rotation.rotate(Vector3.up);
  }

  /** Moves by `delta`, in world space (or along the object's own axes with `"self"`). */
  translate(delta: Vector3Like, space: "world" | "self" = "world"): void {
    const d = space === "self" ? this.rotation.rotate(delta) : Vector3.from(delta);
    this.position = this.position.add(d);
  }

  /** Rotates by Euler angles in degrees, about the object's own axes (or the world's with `"world"`). */
  rotate(euler: Vector3Like, space: "self" | "world" = "self"): void {
    const q = Quaternion.euler(euler);
    this.rotation = space === "self" ? this.rotation.mul(q) : q.mul(this.rotation);
  }

  /** Turns the object so forward points at `target`. */
  lookAt(target: Vector3Like, up: Vector3Like = Vector3.up): void {
    this.rotation = Quaternion.lookRotation(Vector3.from(target).sub(this.position), up);
  }

  /** A point from the object's local space to the world. */
  transformPoint(p: Vector3Like): Vector3 {
    return this.position.add(this.rotation.rotate(this.lossyScale.mul(p)));
  }

  /** A point from the world to the object's local space. */
  inverseTransformPoint(p: Vector3Like): Vector3 {
    return this.rotation.inverse().rotate(Vector3.from(p).sub(this.position)).div(this.lossyScale);
  }

  /** A direction from local space to the world (rotation only). */
  transformDirection(d: Vector3Like): Vector3 {
    return this.rotation.rotate(d);
  }

  get parent(): Transform | null {
    return this.gameObject.parent?.transform ?? null;
  }
}

export interface GameObjectOptions {
  parent?: GameObject | null;
  position?: Vector3Like;
  rotation?: QuaternionLike;
  scale?: Vector3Like;
}

export class GameObject {
  readonly id: number;
  readonly transform: Transform;
  readonly components: Component[] = [];
  readonly children: GameObject[] = [];
  private _name: string;
  private _parent: GameObject | null = null;
  private _enabled = true;
  private _destroyed = false;
  private _layer = 0;
  /** @internal The scene a root object is in (children are in their root's). */
  _scene: Scene | null = null;
  /** @internal On a model's copy (`Model.instantiate`): what to load it from when saved. */
  _model: { src: string; colliders?: string } | null = null;
  /** @internal Built from a model's nodes: saved scenes rebuild these from the model. */
  _modelPart = false;

  /**
   * Creates an object in the world (at the origin, or as given). `position` and
   * `rotation` are local to `parent`.
   */
  constructor(name = "GameObject", options: GameObjectOptions = {}) {
    this.id = nextObject++;
    this._name = name;
    this.transform = new Transform(this);
    this.transform._set(
      Vector3.from(options.position ?? Vector3.zero),
      Quaternion.from(options.rotation ?? Quaternion.identity).normalized,
      Vector3.from(options.scale ?? Vector3.one),
    );
    objects.set(this.id, this);
    const parent = options.parent ?? null;
    this._parent = parent;
    if (!parent) this._scene = sceneHooks?.active() ?? null;
    (parent ? parent.children : roots).push(this);
    const t = this.transform;
    __iron_op("scene.create", this.id, name, parent?.id ?? null, vec(t._position), quat(t._rotation), vec(t._scale));
  }

  /** @internal An object the engine created (it has the id already). */
  static _adopt(id: number, name: string): GameObject {
    const go = Object.create(GameObject.prototype) as GameObject;
    Object.assign(go, {
      id,
      components: [],
      children: [],
      _name: name,
      _parent: null,
      _enabled: true,
      _destroyed: false,
      _layer: 0,
      _scene: sceneHooks?.active() ?? null,
      _model: null,
      _modelPart: false,
    });
    Object.assign(go, { transform: new Transform(go) });
    objects.set(id, go);
    roots.push(go);
    return go;
  }

  get name(): string {
    return this._name;
  }
  set name(name: string) {
    this._name = name;
    __iron_op("scene.name", this.id, name);
  }

  /**
   * The render group (0 to 31) it's in: cameras draw the layers in their `cullingMask`.
   * Children keep their own layers, like Unity's.
   */
  get layer(): number {
    return this._layer;
  }
  set layer(layer: number) {
    this._layer = checkLayer(layer);
    __iron_op("scene.layer", this.id, layer);
  }

  get parent(): GameObject | null {
    return this._parent;
  }

  /** The scene it's in (its root's). */
  get scene(): Scene {
    let root: GameObject = this;
    while (root._parent) root = root._parent;
    return root._scene!;
  }

  /** The topmost object above it (itself at the root). */
  get root(): GameObject {
    let root: GameObject = this;
    while (root._parent) root = root._parent;
    return root;
  }

  /**
   * Moves the object under `parent` (`null`: to the world root). It keeps its place in
   * the world unless `keepWorld` is false, in which case it keeps its local transform.
   */
  setParent(parent: GameObject | null, keepWorld = true): void {
    for (let p = parent; p; p = p._parent) {
      if (p === this) throw new Error(`can't parent ${this._name} under itself`);
    }
    const t = this.transform;
    const position = t.position;
    const rotation = t.rotation;
    // A new root stays in the scene it was in.
    if (!parent) this._scene = this.scene;
    const siblings = this._parent ? this._parent.children : roots;
    siblings.splice(siblings.indexOf(this), 1);
    this._parent = parent;
    (parent ? parent.children : roots).push(this);
    __iron_op("scene.parent", this.id, parent?.id ?? null);
    if (keepWorld) {
      t.position = position;
      t.rotation = rotation;
    }
    // A new parent can mean a new world scale.
    for (const f of scaleListeners) f(this);
  }

  /** Whether this object is enabled (its children may still be off through their own flag). */
  get enabled(): boolean {
    return this._enabled;
  }
  set enabled(on: boolean) {
    this._enabled = on;
    __iron_op("scene.enabled", this.id, on);
  }

  /** Enabled, along with every parent: inactive objects' components don't update. */
  get activeInHierarchy(): boolean {
    for (let o: GameObject | null = this; o; o = o._parent) if (!o._enabled) return false;
    return true;
  }

  get destroyed(): boolean {
    return this._destroyed;
  }

  /** Attaches a new component, with fields set from `props`. It starts on the next tick. */
  addComponent<T extends Component>(cls: ComponentClass<T>, props?: Partial<T>): T {
    const instance = this._addComponentAsleep(cls, props);
    instance.onAwake?.();
    return instance;
  }

  /** @internal `addComponent` without `onAwake` (scenes wake components once all are loaded). */
  _addComponentAsleep<T extends Component>(cls: ComponentClass<T>, props?: Partial<T>): T {
    const name = nameOf(cls);
    const id = nextComponent++;
    const instance = instantiate(cls, id, this) as T;
    if (props) Object.assign(instance, props);
    __iron_op("scene.addComponent", this.id, id, name);
    return instance;
  }

  /** The first component of this class (or a subclass) on the object. */
  getComponent<T extends Component>(cls: ComponentClass<T>): T | undefined {
    return this.components.find((c): c is T => c instanceof cls);
  }

  getComponents<T extends Component>(cls: ComponentClass<T>): T[] {
    return this.components.filter((c): c is T => c instanceof cls);
  }

  /** The first matching component here or on any child, depth-first. */
  getComponentInChildren<T extends Component>(cls: ComponentClass<T>): T | undefined {
    return this.getComponent(cls) ?? this.children.map((c) => c.getComponentInChildren(cls)).find((c) => c);
  }

  /** Removes a component (its `onDestroy` runs on the engine's next pass). */
  removeComponent(component: Component): void {
    const i = this.components.indexOf(component);
    if (i < 0) return;
    this.components.splice(i, 1);
    __iron_op("scene.removeComponent", component.id);
  }

  /** Destroys the object, its children and all their components. */
  destroy(): void {
    if (this._destroyed) return;
    __iron_op("scene.destroy", this.id);
    forget(this);
  }

  /**
   * Keeps a root object (and its children) when `SceneManager.loadScene` replaces the
   * scenes: it moves to the `DontDestroyOnLoad` scene (Unity's `DontDestroyOnLoad`).
   */
  static dontDestroyOnLoad(go: GameObject): void {
    if (!sceneHooks) throw new Error("dontDestroyOnLoad needs @iron/engine's scenes.ts loaded");
    sceneHooks.dontDestroyOnLoad(go);
  }

  /**
   * A copy of an object with its children and components (Unity's `Instantiate`):
   * references inside the copy point at the copies, others at what they pointed at.
   * Without a parent the copy starts where the original is in the world.
   */
  static instantiate(original: GameObject, options: InstantiateObjectOptions = {}): GameObject {
    if (!sceneHooks) throw new Error("instantiate needs @iron/engine's scenes.ts loaded");
    return sceneHooks.instantiate(original, options);
  }

  /** A child by name (direct children only). */
  findChild(name: string): GameObject | undefined {
    return this.children.find((c) => c._name === name);
  }

  /** The first object with this name, depth-first through the world. */
  static find(name: string): GameObject | undefined {
    const walk = (list: GameObject[]): GameObject | undefined => {
      for (const o of list) {
        if (o._name === name) return o;
        const found = walk(o.children);
        if (found) return found;
      }
      return undefined;
    };
    return walk(roots);
  }

  /** Objects at the world root, in order. */
  static get roots(): readonly GameObject[] {
    return roots;
  }

  /** An object by id. */
  static byId(id: number): GameObject | undefined {
    return objects.get(id);
  }

  /**
   * An object with a built-in mesh and the matching collider, like Unity's:
   * `GameObject.createPrimitive("cube")` (or `PrimitiveType.Cube`).
   */
  static createPrimitive(
    type: "cube" | "sphere" | "capsule" | "cylinder" | "plane" | "quad",
    options?: GameObjectOptions & { name?: string },
  ): GameObject {
    if (!primitiveFactory) throw new Error("createPrimitive needs @iron/engine's index (primitives.ts) loaded");
    return primitiveFactory(type, options);
  }

  /** The first component of this class on any object. */
  static findObjectOfType<T extends Component>(cls: ComponentClass<T>): T | undefined {
    for (const c of instances.values()) if (c instanceof cls) return c as T;
    return undefined;
  }
}

type PrimitiveFactory = (
  type: "cube" | "sphere" | "capsule" | "cylinder" | "plane" | "quad",
  options?: GameObjectOptions & { name?: string },
) => GameObject;
let primitiveFactory: PrimitiveFactory | null = null;

/** @internal primitives.ts provides `createPrimitive` (it needs the render and physics components). */
export function setPrimitiveFactory(f: PrimitiveFactory) {
  primitiveFactory = f;
}

/** Drops a destroyed object and its subtree from the mirror. */
function forget(go: GameObject): void {
  const siblings = go.parent ? go.parent.children : roots;
  const i = siblings.indexOf(go);
  if (i >= 0) siblings.splice(i, 1);
  const stack = [go];
  while (stack.length) {
    const o = stack.pop()!;
    (o as unknown as { _destroyed: boolean })._destroyed = true;
    objects.delete(o.id);
    stack.push(...o.children);
  }
}

/** Clears everything. Only for tests. */
export function _resetSceneForTests() {
  classes.clear();
  starting.clear();
  names.clear();
  for (const [name, cls] of builtIns) names.set(cls, name);
  instances.clear();
  restored.clear();
  objects.clear();
  roots.length = 0;
  sceneHooks?.reset();
  nextObject = 1;
  nextComponent = 1 << 24;
}

// ---- Host functions called by Rust (crates/iron_script/src/components.rs) ----

globalThis.__iron_registered = () => [...starting];

/** Creates an engine-spawned component, on the object the engine made for it. */
globalThis.__iron_create = (className: string, id: number, objectId?: number, objectName?: string) => {
  const cls = classes.get(className);
  if (!cls) throw new Error(`unknown component class '${className}'`);
  const object = objectId === undefined || objectId === null
    ? GameObject._adopt(-id - 1, className)
    : (objects.get(objectId) ?? GameObject._adopt(objectId, objectName ?? className));
  instantiate(cls, id, object).onAwake?.();
};

const HOOKS = ["onStart", "onUpdate", "onLateUpdate", "onFixedUpdate", "onPreRender", "onDestroy"] as const;

/** Which lifecycle methods a component has, so the engine only calls those. */
globalThis.__iron_hooks = (id: number) => {
  const instance = instances.get(id) as unknown as Record<string, unknown> | undefined;
  if (!instance) return [...HOOKS];
  return HOOKS.filter((h) => typeof instance[h] === "function" && !(h === "onStart" && restored.has(id)));
};

globalThis.__iron_dispatch = (id: number, method: string, ...args: unknown[]) => {
  const instance = instances.get(id);
  if (!instance) throw new Error(`no component with id ${id}`);
  const fn = (instance as unknown as Record<string, unknown>)[method];
  if (typeof fn === "function") return fn.apply(instance, args);
  return undefined;
};

globalThis.__iron_destroy = (id: number) => {
  const instance = instances.get(id);
  instances.delete(id);
  if (!instance) return;
  const list = instance.gameObject.components;
  const i = list.indexOf(instance);
  if (i >= 0) list.splice(i, 1);
};

/** Objects the engine moved: `id, px, py, pz, qx, qy, qz, qw, sx, sy, sz` each (local). */
globalThis.__iron_scene = (moved: number[]) => {
  for (let i = 0; i + 10 < moved.length; i += 11) {
    const go = objects.get(moved[i]!);
    if (!go) continue;
    go.transform._set(
      new Vector3(moved[i + 1], moved[i + 2], moved[i + 3]),
      new Quaternion(moved[i + 4], moved[i + 5], moved[i + 6], moved[i + 7]),
      new Vector3(moved[i + 8], moved[i + 9], moved[i + 10]),
    );
  }
};
