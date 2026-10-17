/**
 * @iron/engine — the TypeScript side of Iron.
 *
 * Game code extends `Component`, attaches components to `GameObject`s and calls
 * `register({ ... })`. The engine (Rust) owns the objects and the list of components
 * and calls into them through `__iron_*` host functions (scene.ts), always by
 * (component id, method name).
 */

import "./globals";

export {
  audio,
  Bus,
  Emitter,
  type EmitterOptions,
  type LoadOptions,
  Playback,
  type PlayOptions,
  Sound,
} from "./audio.ts";
export { AudioListener, AudioSource } from "./audio-components.ts";
export { type MinMax, type ParticleBurst, type ParticleShape, ParticleSystem } from "./particles.ts";
export {
  type FilterMode,
  Sprite,
  SpriteRenderer,
  Texture,
  type TextureOptions,
  type TextureWrapMode,
} from "./textures.ts";
export { type Gamepad, type InputEvents, input, type KeyEvent } from "./input.ts";
export {
  Bounds,
  Color,
  Mathf,
  Matrix4x4,
  Plane,
  Pose,
  Quaternion,
  type QuaternionLike,
  Ray,
  Rect,
  RectInt,
  Vector2,
  Vector2Int,
  type Vector2Like,
  Vector3,
  Vector3Int,
  type Vector3Like,
  Vector4,
} from "./math.ts";
export {
  type BodyKind,
  type BodyOptions,
  type CharacterOptions,
  PhysicsCollider,
  type ColliderOptions,
  type CollisionEvent,
  physics,
  type RayHit,
  PhysicsBody,
  PhysicsJoint,
  type JointKind,
  type Shape,
} from "./physics.ts";
export { DateTime, type DateTimeKind, TimeSpan } from "./time.ts";
export { Task, Time, TimeSince, TimeUntil } from "./clock.ts";
export { Console, Debug, Log } from "./log.ts";
export { Random, Rng } from "./random.ts";
export {
  BoxCollider,
  CapsuleCollider,
  CharacterController,
  CharacterJoint,
  Collider,
  CollisionFlags,
  FixedJoint,
  HingeJoint,
  Joint,
  type JointLimits,
  type JointMotor,
  Rope,
  RopeJoint,
  SpringJoint,
  type Collision,
  ForceMode,
  MeshCollider,
  PhysicsMaterial,
  Rigidbody,
  RigidbodyConstraints,
  SphereCollider,
} from "./physics-components.ts";
export {
  Camera,
  CameraFollow,
  CameraOrbit,
  Light,
  type LightType,
  Material,
  Mesh,
  MeshRenderer,
  PlaneRenderer,
  type PrimitiveMesh,
  type BloomSettings,
  RenderSettings,
  WorldPanel,
} from "./rendering.ts";
export { createPrimitive, PrimitiveType } from "./primitives.ts";
export { Screen } from "./screen.ts";
export {
  Component,
  type ComponentClass,
  GameObject,
  type GameObjectOptions,
  register,
  Transform,
} from "./scene.ts";

/** Queues an op for the engine. Low level: prefer the typed APIs built on it. */
export function op(name: string, ...args: unknown[]): void {
  __iron_op(name, ...args);
}

export interface FontFace {
  /** The name to use in `fontFamily`. */
  family: string;
  /** A TTF, OTF, WOFF or WOFF2 file, relative to the game folder (like `url()` in CSS). */
  src: string;
  /** `@font-face` weight; defaults to what the file says. */
  weight?: number | "normal" | "bold";
  /** `@font-face` style; defaults to what the file says. */
  style?: "normal" | "italic";
}

/**
 * Loads a font for `fontFamily` to use (like CSS `@font-face`). Text that names the
 * family re-lays out once it has loaded; until then it uses fallback fonts.
 */
export function loadFont(face: FontFace): void {
  op("loadFont", face.family, face.src, face.weight ?? null, face.style ?? null);
}

let nextStyleSheet = 1;

/**
 * Adds a stylesheet from CSS text (CSS-in-JS). `url()`s are relative to the game
 * folder. Returns an id for `removeStyleSheet`. Stylesheets imported with
 * `import "./ui.css"` are bundled into `dist/game.css` and loaded automatically.
 */
export function addStyleSheet(css: string): number {
  const id = nextStyleSheet++;
  op("addStyleSheet", id, css);
  return id;
}

/**
 * How screen UI scales with the window (s&box's ScreenPanel). UI is laid out in "UI
 * pixels"; the engine draws them bigger or smaller:
 * - `{ height: 1080 }`: the window is always 1080 UI pixels high (s&box's default target).
 * - `{ width: 1920, height: 1080 }`: that design always fits inside the window.
 * - a number: a fixed zoom (`1.5`).
 * - `null`: one UI pixel per logical pixel (the default).
 * The factor is kept between 0.25 and 8.
 */
export function setUiScale(scale: { height: number; width?: number } | number | null): void {
  if (scale === null) op("uiScale", "none");
  else if (typeof scale === "number") op("uiScale", "fixed", scale);
  else if (scale.width !== undefined) op("uiScale", "fit", scale.width, scale.height);
  else op("uiScale", "height", scale.height);
}

/** Removes a stylesheet added with `addStyleSheet`. */
export function removeStyleSheet(id: number): void {
  op("removeStyleSheet", id);
}
