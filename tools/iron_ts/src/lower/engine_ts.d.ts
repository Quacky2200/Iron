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
type PrimitiveType = "cube" | "sphere" | "capsule" | "cylinder" | "plane" | "quad";

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

interface SceneObjectData {
  name: string;
}
interface SceneData {
  name: string;
  objects: SceneObjectData[];
}
interface Scene {
  getRootGameObjects(): GameObject[];
  serialize(): SceneData;
}
declare namespace SceneManager {
  function loadSceneAsync(path: string, mode?: string): Promise<Scene>;
  function onSceneLoaded(fn: (scene: Scene, mode: string) => void): () => void;
  function onSceneUnloaded(fn: (scene: Scene) => void): () => void;
  function onActiveSceneChanged(fn: (from: Scene, to: Scene) => void): () => void;
}

// ---- Rendering ---------------------------------------------------------------------------------

interface ProceduralSky {
  zenith?: Color;
  horizon?: Color;
  ground?: Color;
  /** Towards the sun; `null` for no sun disc. */
  sunDirection?: Vector3Like | null;
  sunColor?: Color;
  /** The disc's size in degrees. */
  sunSize?: number;
}
interface BloomSettings {
  threshold: number;
  intensity: number;
  scatter: number;
}
declare namespace RenderSettings {
  let skybox: ProceduralSky | null;
  let bloom: BloomSettings | null;
}

interface Camera {
  layerCullDistances: number[];
  readonly horizontalFieldOfView: number;
}
declare namespace Camera {
  const main: Camera | undefined;
}

type FilterMode = "point" | "bilinear" | "trilinear";
type TextureWrapMode = "repeat" | "clamp" | "mirror";
interface TextureOptions {
  filterMode?: FilterMode;
  wrapMode?: TextureWrapMode;
  mipmaps?: boolean;
  anisoLevel?: number;
  linear?: boolean;
}
interface Texture {
  whenLoaded(): Promise<Texture>;
}
declare namespace Texture {
  function load(src: string, options?: TextureOptions): Texture;
  function create(width: number, height: number, colors: readonly Color[], options?: TextureOptions): Texture;
}
interface Material {
  withTexture(texture: Texture): Material;
}
interface Model {
  whenLoaded(): Promise<Model>;
}
declare namespace Sprite {
  function grid(texture: Texture, columns: number, rows: number, pivot?: Vector2, pixelsPerUnit?: number): Sprite[];
  function create(texture: Texture, rect?: Rect | null, pivot?: Vector2, pixelsPerUnit?: number): Sprite;
}
interface LodLevel {
  screenRelativeTransitionHeight: number;
  renderers: MeshRenderer[];
}
interface LODGroup {
  setLODs(levels: LodLevel[]): LODGroup;
}
interface LineRenderer2D {
  points: Vector2Like[];
}

declare namespace QualitySettings {
  let shadows: ShadowQualityMode;
  const names: string[];
  function onLevelChanged(fn: (level: number) => void): () => void;
}

interface Resolution {
  width: number;
  height: number;
  refreshRate: number;
}
declare namespace Screen {
  const windowSize: { width: number; height: number };
  const currentResolution: Resolution;
  const resolutions: Resolution[];
  const supportedFullScreenModes: FullScreenMode[];
  function onChange(fn: () => void): () => void;
}

// ---- Windows -----------------------------------------------------------------------------------

type WindowStyle = "normal" | "tool" | "dialog" | "popup" | "borderless" | "child";
interface WindowOptions {
  /** The title bar text. Default "". */
  title?: string;
  /** Client area size in UI pixels. Default 400 x 300. */
  width?: number;
  height?: number;
  /** Top-left of the outer frame in desktop UI pixels. Left out: centred on the parent, cascading. */
  x?: number;
  y?: number;
  minWidth?: number;
  minHeight?: number;
  /** Default "normal". */
  style?: WindowStyle;
  /** The window this one belongs to. Tool, dialog, popup and child windows default to the main window. */
  parent?: Window | null;
  resizable?: boolean;
  minimizable?: boolean;
  maximizable?: boolean;
  /** Whether the title bar has a close button. */
  closable?: boolean;
  alwaysOnTop?: boolean;
  visible?: boolean;
  /** Takes keyboard focus when shown; popups never do. */
  focus?: boolean;
  /** A name that keeps the window across a hot reload and finds it again (`Window.find`). */
  key?: string;
  /** Root windows of isolated families only (a "normal" or "borderless" window without a parent): keep the
   *  domain's timers running while the family is unfocused. Default true. */
  runInBackground?: boolean;
}
interface Window {
  /** Changing it re-applies decorations, taskbar and z-order rules; "child" cannot be set or unset. */
  style: WindowStyle;
  /** The owner or embedding parent; undefined for the main window and unowned windows. */
  readonly parent: Window | undefined;
  /** Why the window could not be opened ("Cannot open Window due to platform restrictions (platform: android)"),
   *  null when it could. `isOpen` is false then, and `onOpenFailed` has run. */
  readonly openError: string | null;
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
}
declare namespace Window {
  /** Messages to any window of the current domain (after the per-window listeners of the target). */
  function onMessage<T = unknown>(f: (from: Window, data: T) => void): number;
  /** Requests to any window of the current domain; `reply` answers the sender's promise (once). */
  function onRequest<T = unknown, R = unknown>(f: (from: Window, data: T, reply: (answer: R) => void) => void): number;
  /** Posts `data` to the root window of every other family. Returns how many were queued. */
  function broadcast(data: unknown): number;
  /** The window with keyboard focus, if one of the game's has it. */
  const focused: Window | undefined;
  /** Opens a window. It exists (and has a handle) at once; the OS window appears at the end of the frame. */
  function create(options?: WindowOptions): Window;
  /** The `index`-th open window, the main one first, then in creation order. */
  function at(index: number): Window | undefined;
  /** The open window made with this `key`. */
  function find(key: string): Window | undefined;
  /** The open window with this `handle` (for handles received in messages). */
  function byHandle(handle: number): Window | undefined;
}

// ---- Input, UI resources -----------------------------------------------------------------------

declare namespace input {
  function gamepads(): Gamepad[];
  function on(type: "gamepadconnected" | "gamepaddisconnected", fn: () => void): () => void;
}
declare namespace Assets {
  function onChanged(fn: (path: string) => void): () => void;
}
declare namespace Application {
  function onFocusChanged(fn: (focused: boolean) => void): () => void;
}

interface FontFace {
  family: string;
  src: string;
  weight?: number;
  style?: string;
}
declare function loadFont(face: FontFace): void;
declare function addStyleSheet(css: string): number;
declare function removeStyleSheet(id: number): void;

// ---- Animation ---------------------------------------------------------------------------------

type AnimatorController = AnimationController;
declare const AnimatorController: typeof AnimationController;

interface AnimatorStateInfo {
  name: string;
  normalizedTime: number;
  length: number;
}
interface Animator {
  getCurrentAnimatorStateInfo(): AnimatorStateInfo;
}
interface BlendChild {
  motion: AnimationClip;
  threshold: number;
  timeScale?: number;
}
interface BlendSpec {
  blendParameter: string;
  children: BlendChild[];
}
interface StateOpts {
  speed?: number;
  speedParameter?: string;
}
interface ConditionSpec {
  parameter: string;
  mode: string;
  threshold?: number;
}
interface TransitionSpec {
  from: string;
  to: string;
  conditions?: ConditionSpec[];
  hasExitTime?: boolean;
  exitTime?: number;
  duration?: number;
  offset?: number;
}
interface AnimationController {
  addParameter(name: string, type: string, defaultValue?: number): AnimationController;
  addState(name: string, motion: AnimationClip | BlendSpec, opts?: StateOpts): AnimationController;
  addTransition(t: TransitionSpec): AnimationController;
}
declare namespace AnimationCurve {
  function fromPoints(points: number[][], mode?: string): AnimationCurve;
}

// ---- More rendering shapes ---------------------------------------------------------------------

//@ctor Material(color?: Color, emission?: Color, smoothness?: number, unlit?: boolean, mainTexture?: Texture | null, mainTextureScale?: Vector2, mainTextureOffset?: Vector2, metallic?: number)
//@ctor Material(options?: $OPTIONS)
//@ctor Color(r: number, g: number, b: number, a?: number)
interface MeshRenderer {
  /** A mesh, or the name of a primitive. */
  mesh: Mesh | PrimitiveType;
}
interface MeshData {
  vertices: Vector3[];
  triangles: number[];
  normals?: Vector3[];
  uv?: Vector2[];
  boneWeights?: { indices: number[]; weights: number[] }[];
  bindposes?: Matrix4x4[];
}
declare namespace Mesh {
  function create(data: MeshData): Mesh;
}
interface SkinnedMeshRenderer {
  bones: GameObject[];
}
interface ClothBox {
  min: Vector3;
  max: Vector3;
}
interface Cloth {
  pinBox: ClothBox;
  pins: Vector3[];
  sphereColliders: GameObject[];
  capsuleColliders: GameObject[];
}
interface Camera {
  rect: Rect;
}
interface ParticleBurst {
  time: number;
  count: number;
  cycles?: number;
  interval?: number;
}
interface ParticleShapeSpec {
  type: string;
  angle?: number;
  radius?: number;
  size?: Vector3Like;
}
interface ColorKey {
  time: number;
  color: Color;
}
interface SizeKey {
  time: number;
  value: number;
}
interface ParticleSystem {
  /** A number, or `[min, max]`. */
  startLifetime: number | number[];
  startSpeed: number | number[];
  startSize: number | number[];
  startRotation: number | number[];
  angularVelocity: number | number[];
  /** A colour, or `[a, b]`. */
  startColor: Color | Color[];
  bursts: ParticleBurst[];
  shape: ParticleShapeSpec;
  colorOverLifetime: ColorKey[];
  sizeOverLifetime: SizeKey[];
}
interface EdgeCollider2D {
  points: Vector2Like[];
}
interface LineRenderer {
  positions: Vector3Like[];
}
interface AudioSource {
  playOneShot(clip: Sound | string, volumeScale?: number): void;
}
type ShadowQualityMode = "disable" | "hardOnly" | "all";
