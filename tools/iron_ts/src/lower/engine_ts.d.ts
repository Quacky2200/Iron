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
/** How fog thickens with distance (Unity's `FogMode`). */
type FogMode = "linear" | "exponential" | "exponentialSquared";
/** The light-shaft volume's detail levels. */
type VolumeDetail = "low" | "medium" | "high" | "ultra";
/** Fog lying low (Unreal's exponential height fog), in `fogColor`. */
interface HeightFogSettings {
  /** Per metre at `baseHeight`. */
  density: number;
  baseHeight: number;
  /** Per metre (larger: a thinner layer). */
  falloff: number;
}
/** Light shafts (volumetric fog): lights shining through the fog light it up, cut by their shadows. Every field is optional. */
interface VolumetricFogSettings {
  /** Scales the scattered light (1). */
  intensity?: number;
  /** -1 to 1: positive scatters forward (0.4). */
  anisotropy?: number;
  /** Samples per depth slice, 1 to 16 (4). */
  steps?: number;
  /** How far the volume reaches in metres (50). */
  distance?: number;
  /** How much of each colour of light the fog scatters (white). */
  albedo?: Color;
  /** God rays: within this many metres of the camera the sun's shafts are worked out per pixel (0: off). */
  sharpDistance?: number;
  /** Samples per pixel for the sharp part, 4 to 128 (32). */
  sharpSamples?: number;
  /** How fine the light-shaft volume is (`high`). */
  resolution?: VolumeDetail;
  /** How the volume's sampling noise is cleaned up (`both`). */
  denoising?: "none" | "reprojection" | "gaussian" | "both";
  /** With reprojection, how much of each cell comes from earlier frames (0.9). */
  historyWeight?: number;
  /** Shadow samples per cell each frame, 1 to 8 (1). */
  samples?: number;
  /** Samples for cells whose view moved this frame (2). */
  motionSamples?: number;
}
/** Auto exposure (HDRP's Automatic mode). Every field is optional. */
interface AutoExposureSettings {
  /** The darkest average brightness it adapts to, in EV100 (-10). */
  minEV?: number;
  /** The brightest, in EV100 (20). */
  maxEV?: number;
  /** Stops added after metering (0; positive is brighter). */
  compensation?: number;
  /** How fast it adapts when the view gets brighter (3, per second). */
  speedDarkToLight?: number;
  /** How fast it adapts when the view gets darker (3, per second). */
  speedLightToDark?: number;
  /** Which part of the view counts (`centerWeighted`). */
  metering?: "average" | "centerWeighted" | "spot";
  /** The part of the brightness histogram averaged, in percent from the darkest (0 and 100: everything). */
  lowPercent?: number;
  highPercent?: number;
  /** Below `darkBelow` (EV100, 5) the eyes adapt only this much (0..1, 1) of the way. */
  darkAdaptation?: number;
  darkBelow?: number;
  /** More stops by how bright the view is: up to 8 points of [metered EV100, stops]. */
  curve?: [number, number][];
  /** Stops adapting: the exposure holds where it is. */
  hold?: boolean;
}
declare namespace RenderSettings {
  let skybox: ProceduralSky | null;
  let bloom: BloomSettings | null;
  let fogMode: FogMode;
  /** Height fog on top of the distance fog, while `fog` is on (`null`: none). */
  let heightFog: HeightFogSettings | null;
  /** Light shafts in the fog, while `fog` is on (`null`: none). */
  let volumetricFog: VolumetricFogSettings | null;
  /** Adapts the exposure to the view (`null`: off, the default). */
  let autoExposure: AutoExposureSettings | null;
  /** The manual exposure, in EV100 (higher is darker), used without auto exposure (`null`: light as it is). */
  let fixedExposure: number | null;
}

interface Camera {
  layerCullDistances: number[];
  readonly horizontalFieldOfView: number;
  /** The world point at a screen point (UI pixels from the top-left), `z` units in front of the camera. */
  screenToWorldPoint(p: Vector3Like): Vector3;
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
  /** Fills the shape in this colour (null: no fill). */
  fillColor: Color | null;
}

/** GDI-style shape outlines for `LineRenderer2D.points`: y up, angles in degrees counter-clockwise from +x. Closed shapes do not repeat their first point (set `loop`, or a `fillColor`). */
declare class Shapes {
  static rectangle(x: number, y: number, width: number, height: number): Vector2[];
  static roundedRectangle(x: number, y: number, width: number, height: number, radius: number, segments?: number): Vector2[];
  static ellipse(center: Vector2Like, rx: number, ry: number, segments?: number): Vector2[];
  static circle(center: Vector2Like, radius: number, segments?: number): Vector2[];
  static arc(center: Vector2Like, rx: number, ry: number, startAngle: number, sweepAngle: number, segments?: number): Vector2[];
  static pie(center: Vector2Like, rx: number, ry: number, startAngle: number, sweepAngle: number, segments?: number): Vector2[];
  static regularPolygon(center: Vector2Like, radius: number, sides: number, rotation?: number): Vector2[];
  static polygon(...points: Vector2Like[]): Vector2[];
  static triangle(a: Vector2Like, b: Vector2Like, c: Vector2Like): Vector2[];
  static bezier(p0: Vector2Like, p1: Vector2Like, p2: Vector2Like, p3: Vector2Like, segments?: number): Vector2[];
  static curve(points: readonly Vector2Like[], tension?: number, segments?: number): Vector2[];
  static closedCurve(points: readonly Vector2Like[], tension?: number, segments?: number): Vector2[];
}

/** `auto`: smooth through the neighbours; `bezier`: the knot's own handles; `linear`: none. */
type TangentMode = "auto" | "bezier" | "linear";
interface SplineKnot {
  position: Vector3;
  /** The handle before the knot, relative to it (used in `bezier` mode). */
  tangentIn: Vector3;
  /** The handle after the knot, relative to it (used in `bezier` mode). */
  tangentOut: Vector3;
  mode: TangentMode;
}
/** A path through knots, every segment a cubic Bezier (Unity's Splines package, the core of it). */
declare class Spline {
  constructor(points?: readonly Vector3Like[], options?: { closed?: boolean; tension?: number; mode?: TangentMode });
  readonly knots: SplineKnot[];
  readonly count: number;
  closed: boolean;
  tension: number;
  readonly segmentCount: number;
  /** Goes up on every change. */
  version: number;
  readonly length: number;
  addKnot(position: Vector3Like, mode?: TangentMode, tangentIn?: Vector3Like, tangentOut?: Vector3Like): number;
  insertKnot(index: number, position: Vector3Like, mode?: TangentMode, tangentIn?: Vector3Like, tangentOut?: Vector3Like): number;
  removeKnot(index: number): void;
  setPosition(index: number, position: Vector3Like): void;
  setTangents(index: number, tangentIn: Vector3Like, tangentOut: Vector3Like): void;
  setMode(index: number, mode: TangentMode): void;
  handleOut(index: number): Vector3;
  handleIn(index: number): Vector3;
  segmentPoint(i: number, u: number): Vector3;
  segmentDerivative(i: number, u: number): Vector3;
  locate(t: number): { segment: number; u: number };
  /** The point `t` of the way along (0 to 1, by distance). */
  evaluate(t: number): Vector3;
  tangent(t: number): Vector3;
  sample(perSegment?: number): Vector3[];
  nearest(p: Vector3Like): { t: number; point: Vector3; distance: number };
}
/** Moves its object along a spline (Unity's `SplineAnimate`). */
declare class SplineAnimate extends Component {
  spline: Spline | null;
  container: GameObject | null;
  duration: number;
  loopMode: "loop" | "pingPong" | "once";
  alignment: "none" | "path";
  playing: boolean;
  elapsed: number;
  readonly t: number;
  place(): void;
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

/**
 * Keyboard, mouse and gamepads. Input belongs to the window's event domain: it reads the keys of the window
 * family whose code is running; main-game components always read the main window's, an isolated window's keys
 * never reach it (send a message), and focus loss clears held keys. See docs/windows.md.
 */
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
type NumberList = readonly number[] | Float32Array | Float64Array | Uint32Array | Uint16Array | Int32Array;
/** A mesh as flat number lists (x, y, z per vertex; u, v per vertex in Unity's bottom-up space), for meshes made in bulk. */
interface FlatMeshData {
  positions: NumberList;
  triangles: NumberList;
  normals?: NumberList;
  uv?: NumberList;
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
  /** A mesh from flat lists (see `FlatMeshData`). In compiled games give all four lists (`normals` and `uv` may be empty). */
  function createFlat(data: FlatMeshData): Mesh;
}
interface Mesh {
  /** Replaces the mesh's data from flat lists. */
  setFlat(data: FlatMeshData): void;
}
type PrimitiveMesh = PrimitiveType;
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
