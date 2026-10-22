// The TypeScript-side shapes of the engine API that the manifests do not carry: what the compiler
// expands over the native API (`facade.ts`, `engine_objs.rs`, `components.rs`, `intl`). Hand-written;
// `iron-build types` puts it after the manifest declarations in types/iron.d.ts.
//
// Format (read by `types_dts.rs`): an `interface N { .. }` block replaces the native instance members
// of the same names, a `declare namespace N { .. }` block the native statics of the same names, and
// `//@ctor N(params)` lines replace N's constructors. Members are indented two spaces.

declare function setTimeout(fn: () => void, ms?: number): number;
declare function clearTimeout(id: number): void;

type Vector2Like = { x: number; y: number };
type Vector3Like = { x: number; y: number; z: number };
type QuaternionLike = { x: number; y: number; z: number; w: number };

// ---- Components --------------------------------------------------------------------------------

/** What `onCollisionEnter` and friends get: the other side of the contact. */
interface Collider {
  readonly gameObject: GameObject;
}

type ComponentClass<T = Component> = new () => T;
type ClassOf<T> = { prototype: T };

// The contact hooks (`onCollisionEnter(c: Collision)`, `onTriggerEnter(other: Collider)`, `onCollisionStay`,
// the `..2D` ones) are not declared: a game writes them without `override`.
declare abstract class Component {
  /** Set by the engine when the component is created. */
  readonly id: number;
  /** The object this component is attached to. */
  readonly gameObject: GameObject;
  /** The object's transform. */
  readonly transform: Transform;
  onAwake?(): void;
  onStart?(): void;
  onUpdate?(dt: number): void;
  onLateUpdate?(dt: number): void;
  onFixedUpdate?(dt: number): void;
  onPreRender?(): void;
  onDestroy?(): void;
  onReload?(): void;
  onApplicationFocus?(hasFocus: boolean): void;
  onApplicationPause?(paused: boolean): void;
  /** Another component on the same object. */
  getComponent<T>(cls: ClassOf<T>): T | undefined;
  /** Removes this component from its object. */
  destroy(): void;
}

/** Registers component classes by name (scene files use the names). */
declare function register(components: Record<string, ComponentClass>, options?: { spawn?: boolean }): void;

/** Marks a field as saved in scenes and kept across hot reloads. */
declare function property(value: unknown, context: ClassFieldDecoratorContext | ClassGetterDecoratorContext): void;

// ---- GameObject --------------------------------------------------------------------------------

interface GameObjectOptions {
  parent?: GameObject | null;
  position?: Vector3Like;
  rotation?: QuaternionLike;
  scale?: Vector3Like;
}
interface InstantiateObjectOptions {
  parent?: GameObject | null;
  position?: Vector3Like;
  rotation?: QuaternionLike;
}

//@ctor GameObject(name?: string, options?: GameObjectOptions)
interface GameObject {
  /** The components of this program on the object. */
  readonly components: Component[];
  readonly children: GameObject[];
  readonly scene: Scene;
  addComponent<T>(cls: ClassOf<T>, props?: Partial<T>): T;
  getComponent<T>(cls: ClassOf<T>): T | undefined;
  getComponents<T>(cls: ClassOf<T>): T[];
  getComponentInChildren<T>(cls: ClassOf<T>): T | undefined;
  removeComponent(component: Component): void;
}
declare namespace GameObject {
  function findObjectOfType<T>(cls: ClassOf<T>): T | undefined;
  const roots: readonly GameObject[];
  function dontDestroyOnLoad(go: GameObject): void;
  function instantiate(original: GameObject | Prefab, options?: InstantiateObjectOptions): GameObject;
  function createPrimitive(type: PrimitiveType, options?: GameObjectOptions & { name?: string }): GameObject;
}

// ---- Rendering ---------------------------------------------------------------------------------


// ---- Windows -----------------------------------------------------------------------------------

interface Window {
  /** Queues `data` (structured values only: no handles, functions, Map or Set) for this window's domain; delivered at
   *  the start of that domain's next dispatch, in order per sender. False when the data is over 1 MiB as JSON or the
   *  window is closed. */
  postMessage(data: unknown): boolean;
  /** A message posted to this window. Runs in this window's domain. Give `T` (or annotate the listener's `data`). */
  onMessage<T = unknown>(f: (from: Window, data: T) => void): number;
  /** A request posted to this window; call `reply` once (a second call is ignored). */
  onRequest<T = unknown, R = unknown>(f: (from: Window, data: T, reply: (answer: R) => void) => void): number;
  /** Posts `data` as a request and resolves with the reply. Rejects with an `Error` when the window closes, no
   *  `onRequest` listener exists in its domain when it is delivered, or nobody replies within `timeoutMs` (default 10000). */
  request<R = unknown>(data: unknown, timeoutMs?: number): Promise<R>;
  /** Named events on top of the raw messages: sends event `name` (with an optional payload) to this window. Same
   *  limits as `postMessage`; false when refused. Throws on an empty name. */
  emit(name: string, payload?: unknown): boolean;
  /** Runs `f(payload, from)` for each event `name` posted to this window, in its domain; other names are ignored.
   *  The handler's parameter type decodes the payload (a handler without parameters takes none). */
  on<T = unknown>(name: string, f: (payload: T, from: Window) => void): number;
  /** Asks event `name`; resolves with what `answer(name, ..)` returned. Rejects like `request`, or at once with `no answer for "name"` when the window has no `answer` for it. */
  ask<R = unknown>(name: string, payload?: unknown, timeoutMs?: number): Promise<R>;
  /** Answers the asks of event `name` with the handler's return value. */
  answer<T = unknown, R = unknown>(name: string, f: (payload: T, from: Window) => R): number;
}
declare namespace Window {
  /** Messages to any window of the current domain (after the per-window listeners of the target). */
  function onMessage<T = unknown>(f: (from: Window, data: T) => void): number;
  /** Requests to any window of the current domain; `reply` answers the sender's promise (once). */
  function onRequest<T = unknown, R = unknown>(f: (from: Window, data: T, reply: (answer: R) => void) => void): number;
  /** Posts `data` to the root window of every other family. Returns how many were queued. */
  function broadcast(data: unknown): number;
  /** `broadcast` of a named event (see `Window.emit`). */
  function emitAll(name: string, payload?: unknown): number;
}

// ---- Input, UI resources -----------------------------------------------------------------------

/**
 * Keyboard, mouse and gamepads. Input belongs to the window's event domain: it reads the keys of the window
 * family whose code is running; main-game components always read the main window's, an isolated window's keys
 * never reach it (send a message), and focus loss clears held keys. See docs/windows.md.
 */
declare namespace input {
  function gamepads(): Gamepad[];
  function on(type: "gamepadconnected" | "gamepaddisconnected", fn: (e: { gamepad: Gamepad }) => void): () => void;
}

// ---- More rendering shapes ---------------------------------------------------------------------

//@ctor Color(r: number, g: number, b: number, a?: number)
interface MeshRenderer {
  /** A mesh, or the name of a primitive. */
  mesh: Mesh | PrimitiveType;
}
declare namespace Random {
  /** A random item from a list (`undefined` when empty). */
  function pick<T>(items: readonly T[]): T | undefined;
}
declare namespace Light {
  /** A light meter: how much light falls on `point` from every light (lux in physical light units), facing `normal` if given. Shadows are not counted. */
  function illuminanceAt(point: Vector3Like, normal?: Vector3Like): number;
  /** A light meter that counts shadows: each light only if a physics ray from `point` reaches it (walls need colliders). */
  function measureIlluminance(point: Vector3Like, normal?: Vector3Like): Promise<number>;
}
interface AudioSource {
  playOneShot(clip: Sound | string, volumeScale?: number): void;
}
