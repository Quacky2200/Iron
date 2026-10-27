// SPDX-License-Identifier: Apache-2.0

// The engine's rendering components (M5.a): cameras and planes, plus camera helpers.
// Like Unity's, they're added to GameObjects (`go.addComponent(Camera)`) and follow
// the object's transform. Each sends its settings to the engine as `render.*` ops when
// it starts and whenever a setting changes.

import { property } from "./property.ts";
import { QualitySettings, _setPostHook } from "./quality.ts";
import { physics } from "./physics.ts";
import {
  Color,
  Mathf,
  Matrix4x4,
  Quaternion,
  Ray,
  Rect,
  Vector2,
  type Vector2Like,
  Vector3,
  type Vector3Like,
} from "./math.ts";
import { Component, type GameObject, HideFlags } from "./scene.ts";
import { Screen } from "./screen.ts";
import type { RenderTexture, Texture } from "./textures.ts";

const rgba = (c: Color) => [c.r, c.g, c.b, c.a];

const cameras: Camera[] = [];

/**
 * What the player sees: a view from the object looking down its forward (-z) axis,
 * perspective or orthographic (2D). Several cameras draw in `depth` order (lower
 * first), each into its `rect` of the screen: split screen, picture-in-picture, a
 * minimap. With no camera, only screen UI is drawn.
 *
 * Screen points are in UI pixels from the top-left (like pointer events); viewport
 * points are 0 to 1 across the camera's `rect`, also from the top-left, with z the
 * distance in front of the camera.
 */
export class Camera extends Component {
  private _fov = 60;
  private _near = 0.1;
  private _far = 1000;
  private _background = new Color(0.05, 0.06, 0.09, 1);
  private _depth = 0;
  private _orthographic = false;
  private _orthographicSize = 5;
  private _rect = new Rect(0, 0, 1, 1);
  private _cullingMask = 0xffffffff;
  private _layerCullDistances: number[] = new Array(32).fill(0);
  private _layerCullSpherical = false;
  private _antialiasing: "none" | "fxaa" = "none";
  private _sceneGrid = false;
  private _fog = true;
  private _targetTexture: RenderTexture | null = null;
  private started = false;
  /** @internal The exposure last read back (`__platypus_exposure`). */
  _exposure = Number.NaN;
  /** @internal How bright it last metered. */
  _sceneBrightness = Number.NaN;

  /** Vertical field of view in degrees (perspective), 1 to 179. */
  @property
  get fieldOfView(): number {
    return this._fov;
  }
  set fieldOfView(v: number) {
    this._fov = Math.min(179, Math.max(1, v));
    this.send();
  }

  /** The horizontal field of view in degrees, from the vertical one and the aspect. */
  get horizontalFieldOfView(): number {
    return Camera.verticalToHorizontalFieldOfView(this._fov, this.aspect);
  }
  set horizontalFieldOfView(v: number) {
    this.fieldOfView = Camera.horizontalToVerticalFieldOfView(v, this.aspect);
  }

  /** A vertical field of view (degrees) as a horizontal one for a width / height aspect. */
  static verticalToHorizontalFieldOfView(vertical: number, aspect: number): number {
    const half = (vertical * Math.PI) / 360;
    return (Math.atan(Math.tan(half) * aspect) * 360) / Math.PI;
  }

  static horizontalToVerticalFieldOfView(horizontal: number, aspect: number): number {
    const half = (horizontal * Math.PI) / 360;
    return (Math.atan(Math.tan(half) / aspect) * 360) / Math.PI;
  }

  @property
  get nearClipPlane(): number {
    return this._near;
  }
  set nearClipPlane(v: number) {
    this._near = v;
    this.send();
  }

  @property
  get farClipPlane(): number {
    return this._far;
  }
  set farClipPlane(v: number) {
    this._far = v;
    this.send();
  }

  /** The colour behind everything in this camera's view (linear RGBA). */
  @property
  get backgroundColor(): Color {
    return this._background;
  }
  set backgroundColor(c: Color) {
    this._background = c;
    this.send();
  }

  private _clearFlags: "skybox" | "solidColor" = "solidColor";
  /**
   * What's behind everything (Unity's `clearFlags`): `skybox` shows
   * `RenderSettings.skybox` (the background colour without one), `solidColor` the
   * background colour.
   */
  @property
  get clearFlags(): "skybox" | "solidColor" {
    return this._clearFlags;
  }
  set clearFlags(f: "skybox" | "solidColor") {
    this._clearFlags = f;
    this.send();
  }

  /** Draw order: cameras with a higher depth draw over lower ones. */
  @property
  get depth(): number {
    return this._depth;
  }
  set depth(d: number) {
    this._depth = d;
    this.send();
  }

  /** An orthographic (2D, no perspective) view instead of a perspective one. */
  @property
  get orthographic(): boolean {
    return this._orthographic;
  }
  set orthographic(on: boolean) {
    this._orthographic = on;
    this.send();
  }

  /** Half the view's height in world units when orthographic (Unity's `orthographicSize`). */
  @property
  get orthographicSize(): number {
    return this._orthographicSize;
  }
  set orthographicSize(s: number) {
    this._orthographicSize = s;
    this.send();
  }

  /** The part of the screen it draws into: x, y, width, height from the top-left, 0 to 1. */
  @property
  get rect(): Rect {
    return this._rect;
  }
  set rect(r: Rect) {
    this._rect = r;
    this.send();
  }

  /**
   * The layers it draws, a bit per layer (all by default). Signed masks work too:
   * `cam.cullingMask &= ~LayerMask.getMask("UI")`.
   */
  @property
  get cullingMask(): number {
    return this._cullingMask;
  }
  set cullingMask(mask: number) {
    this._cullingMask = mask >>> 0;
    this.send();
  }

  /**
   * Per layer, how far away things on it are still drawn (0: out to `farClipPlane`).
   * Set the whole array: changing an item of the one you got does nothing.
   */
  @property
  get layerCullDistances(): number[] {
    return [...this._layerCullDistances];
  }
  set layerCullDistances(distances: number[]) {
    this._layerCullDistances = Array.from({ length: 32 }, (_, i) => Math.max(0, distances[i] ?? 0));
    this.send();
  }

  /** Measure cull distances straight from the camera rather than along its view. */
  @property
  get layerCullSpherical(): boolean {
    return this._layerCullSpherical;
  }
  set layerCullSpherical(on: boolean) {
    this._layerCullSpherical = on;
    this.send();
  }

  /**
   * Post anti-aliasing (Unity's camera `antialiasing`): `fxaa` smooths edges the cheap
   * way, after tonemapping, including ones MSAA can't (textures, shading). The main
   * (biggest) camera's choice applies to the screen.
   */
  @property
  get antialiasing(): "none" | "fxaa" {
    return this._antialiasing;
  }
  set antialiasing(mode: "none" | "fxaa") {
    this._antialiasing = mode;
    this.send();
  }

  /**
   * Draws into this texture instead of the screen (Unity's `targetTexture`); its `rect`
   * is then within the texture. `null`: the screen.
   */
  @property
  get targetTexture(): RenderTexture | null {
    return this._targetTexture;
  }
  set targetTexture(t: RenderTexture | null) {
    this._targetTexture = t;
    this.send();
  }

  /**
   * Draws the editor's ground grid on y = 0 (1 m and 10 m lines, the X axis red and the
   * Z axis blue) over the opaque meshes. For editor views; not saved with scenes.
   */
  get sceneGrid(): boolean {
    return this._sceneGrid;
  }
  set sceneGrid(on: boolean) {
    this._sceneGrid = on;
    this.send();
  }

  /** Shows `RenderSettings.fog` (on by default): turn it off for a minimap or a clear view. */
  @property
  get fog(): boolean {
    return this._fog;
  }
  set fog(on: boolean) {
    this._fog = on;
    this.send();
  }

  /** The camera's area of the screen in UI pixels. */
  get pixelRect(): Rect {
    return new Rect(this._rect.x * Screen.width, this._rect.y * Screen.height, this._rect.width * Screen.width, this._rect.height * Screen.height);
  }

  get aspect(): number {
    const r = this.pixelRect;
    return r.width / Math.max(1e-6, r.height);
  }

  /**
   * The main camera: the first active one added that draws to the screen (like Unity's
   * tagged `Camera.main`). Hidden ones (the editor's scene camera) don't count.
   */
  static get main(): Camera | undefined {
    return cameras.find(
      (c) =>
        c.gameObject.activeInHierarchy &&
        !c.gameObject.destroyed &&
        !c._targetTexture &&
        (c.gameObject.hideFlags & HideFlags.HideInHierarchy) === 0,
    );
  }

  /**
   * The exposure this camera shows its view at, in EV100 (higher is darker, as a camera's
   * EV setting): auto exposure's as it adapts, or the manual one. Read back from the GPU a
   * frame or two late; NaN until the first.
   */
  get exposure(): number {
    return this._exposure;
  }

  /** How bright auto exposure metered this camera's view, in EV100 (NaN until it has). */
  get sceneBrightness(): number {
    return this._sceneBrightness;
  }

  /**
   * Makes auto exposure jump straight to what this camera sees next frame instead of
   * easing there: after a cut, a teleport or a scene load.
   */
  resetExposure(): void {
    __platypus_op("render.exposureReset", this.id);
  }

  /**
   * A physical camera's settings (HDRP's), for the manual exposure: the aperture's
   * f-number, the shutter's time in seconds and the sensor's ISO. Their exposure is
   * `physicalExposure`; use it as `RenderSettings.fixedExposure`. HDRP's defaults (f/16,
   * 1/200 s, ISO 200) suit a sunny day in real-world light units (`platypus.lightUnits: "physical"`).
   */
  aperture = 16;
  shutterSpeed = 1 / 200;
  iso = 200;

  /** The EV100 the physical settings give: log2(N² / t) - log2(ISO / 100). */
  get physicalExposure(): number {
    return Math.log2((this.aperture * this.aperture) / Math.max(this.shutterSpeed, 1e-6)) - Math.log2(Math.max(this.iso, 1) / 100);
  }

  /** @internal By id, for `__platypus_exposure`. */
  static byId(id: number): Camera | undefined {
    return cameras.find((c) => c.id === id);
  }

  /** Every active camera, in draw order. */
  static get allCameras(): Camera[] {
    return cameras.filter((c) => c.gameObject.activeInHierarchy && !c.gameObject.destroyed).sort((a, b) => a._depth - b._depth);
  }

  /** The projection, as the renderer uses it (right-handed, depth 0 to 1). */
  get projectionMatrix(): Matrix4x4 {
    if (this._orthographic) {
      const h = this._orthographicSize;
      const w = h * this.aspect;
      return Matrix4x4.orthographic(-w, w, -h, h, this._near, this._far);
    }
    return Matrix4x4.perspective(this._fov, this.aspect, this._near, this._far);
  }

  /** World to camera space (the view matrix; scale is ignored). */
  get worldToCameraMatrix(): Matrix4x4 {
    return Matrix4x4.trs(this.transform.position, this.transform.rotation).inverse;
  }

  /** World to clip space. */
  get viewProjectionMatrix(): Matrix4x4 {
    return this.projectionMatrix.mul(this.worldToCameraMatrix);
  }

  /** A viewport point (0 to 1 from the top-left) to normalized device coordinates. */
  private static ndc(v: Vector2Like): Vector2 {
    return new Vector2(v.x * 2 - 1, 1 - v.y * 2);
  }

  /** Screen point (UI pixels) to viewport point (0 to 1 across `rect`). */
  screenToViewportPoint(p: Vector3Like): Vector3 {
    const r = this.pixelRect;
    return new Vector3((p.x - r.x) / r.width, (p.y - r.y) / r.height, p.z);
  }

  viewportToScreenPoint(v: Vector3Like): Vector3 {
    const r = this.pixelRect;
    return new Vector3(r.x + v.x * r.width, r.y + v.y * r.height, v.z);
  }

  /** The ray from the camera through a viewport point. */
  viewportPointToRay(v: Vector2Like): Ray {
    const inv = this.viewProjectionMatrix.inverse;
    const n = Camera.ndc(v);
    const near = inv.multiplyPoint(new Vector3(n.x, n.y, 0));
    const far = inv.multiplyPoint(new Vector3(n.x, n.y, 1));
    return new Ray(near, far.sub(near));
  }

  /** The ray from the camera through a screen point (UI pixels): what's under the pointer. */
  screenPointToRay(p: Vector2Like): Ray {
    return this.viewportPointToRay(this.screenToViewportPoint({ x: p.x, y: p.y, z: 0 }));
  }

  /** A world point to the viewport (0 to 1, top-left), z being its distance in front of the camera. */
  worldToViewportPoint(p: Vector3Like): Vector3 {
    const n = this.viewProjectionMatrix.multiplyPoint(p);
    const depth = Vector3.from(p).sub(this.transform.position).dot(this.transform.forward);
    return new Vector3((n.x + 1) / 2, (1 - n.y) / 2, depth);
  }

  /** A world point to the screen (UI pixels), z being its distance in front of the camera. */
  worldToScreenPoint(p: Vector3Like): Vector3 {
    return this.viewportToScreenPoint(this.worldToViewportPoint(p));
  }

  /** The world point at a viewport point, `z` units in front of the camera. */
  viewportToWorldPoint(v: Vector3Like): Vector3 {
    // The ray starts on the near plane; find where it's `z` along the camera's forward.
    const ray = this.viewportPointToRay(v);
    const forward = this.transform.forward;
    const along = ray.direction.dot(forward);
    const start = ray.origin.sub(this.transform.position).dot(forward);
    return ray.getPoint(along > 1e-6 ? (v.z - start) / along : 0);
  }

  /** The world point at a screen point, `z` units in front of the camera. */
  screenToWorldPoint(p: Vector3Like): Vector3 {
    return this.viewportToWorldPoint(this.screenToViewportPoint(p));
  }

  private send(): void {
    if (!this.started) return;
    const r = this._rect;
    __platypus_op(
      "render.camera",
      this.id,
      this.gameObject.id,
      this._fov,
      this._near,
      this._far,
      rgba(this._background),
      this._depth,
      this._orthographic,
      this._orthographicSize,
      [r.x, r.y, r.width, r.height],
      this._clearFlags === "skybox" ? "skybox" : "color",
      this._cullingMask,
      this._layerCullDistances.some((d) => d > 0) ? this._layerCullDistances : null,
      this._layerCullSpherical,
      this._antialiasing,
      this._targetTexture?.id ?? null,
      this._sceneGrid,
      this._fog,
    );
  }

  override onStart(): void {
    this.started = true;
    cameras.push(this);
    this.send();
  }

  override onDestroy(): void {
    cameras.splice(cameras.indexOf(this), 1);
    __platypus_op("render.remove", this.id);
  }
}

/** Clears the camera list. Only for tests. */
export function _resetCamerasForTests() {
  cameras.length = 0;
}

/**
 * A flat rectangle in the object's XZ plane (facing up, like Unity's Plane), `size`
 * metres across, in one colour. Rotate the object to stand it up.
 */
export class PlaneRenderer extends Component {
  private _size = new Vector2(1, 1);
  private _color = Color.white;
  private started = false;

  @property
  get size(): Vector2 {
    return this._size;
  }
  set size(v: Vector2) {
    this._size = Vector2.from(v);
    this.send();
  }

  @property
  get color(): Color {
    return this._color;
  }
  set color(c: Color) {
    this._color = c;
    this.send();
  }

  private send(): void {
    if (!this.started) return;
    __platypus_op("render.plane", this.id, this.gameObject.id, this._size.toArray(), rgba(this._color));
  }

  override onStart(): void {
    this.started = true;
    this.send();
  }

  override onDestroy(): void {
    __platypus_op("render.remove", this.id);
  }
}

/**
 * Follows a target from an offset (in the target's space, or the world's), easing
 * into place, and optionally looks at it. Runs in `onLateUpdate`, after things moved.
 */
export class CameraFollow extends Component {
  @property
  target: GameObject | null = null;
  @property
  offset = new Vector3(0, 3, 6);
  /** Rotate the offset with the target (behind a car), or keep it in world space. */
  @property
  offsetInTargetSpace = false;
  /** About how long catching up takes, in seconds (0: instantly). */
  @property
  smoothTime = 0.25;
  @property
  lookAtTarget = true;
  private velocity = [0, 0, 0];

  override onLateUpdate(dt: number): void {
    const t = this.target?.transform;
    if (!t || this.target?.destroyed) return;
    const offset = this.offsetInTargetSpace ? t.rotation.rotate(this.offset) : this.offset;
    const goal = t.position.add(offset);
    const at = this.transform.position;
    if (this.smoothTime <= 0) {
      this.transform.position = goal;
    } else {
      const axis = (i: 0 | 1 | 2, a: number, b: number) => {
        const [v, vel] = Mathf.smoothDamp(a, b, this.velocity[i]!, this.smoothTime, Math.max(dt, 1e-4));
        this.velocity[i] = vel;
        return v;
      };
      this.transform.position = new Vector3(axis(0, at.x, goal.x), axis(1, at.y, goal.y), axis(2, at.z, goal.z));
    }
    if (this.lookAtTarget) this.transform.lookAt(t.position);
  }
}

/**
 * Orbits a target (or point) at a distance: set `yaw` / `pitch` (degrees) from input,
 * or let it turn by itself with `autoRotate` (degrees per second).
 */
export class CameraOrbit extends Component {
  @property
  target: GameObject | null = null;
  /** Orbits this point when there's no target. */
  @property
  center = Vector3.zero;
  @property
  distance = 8;
  @property
  yaw = 0;
  @property
  pitch = 20;
  @property
  minPitch = -80;
  @property
  maxPitch = 80;
  @property
  autoRotate = 0;

  override onLateUpdate(dt: number): void {
    this.yaw = Mathf.repeat(this.yaw + this.autoRotate * dt, 360);
    this.pitch = Mathf.clamp(this.pitch, this.minPitch, this.maxPitch);
    const center = this.target && !this.target.destroyed ? this.target.transform.position : this.center;
    const rotation = Quaternion.euler(-this.pitch, this.yaw, 0);
    this.transform.position = center.add(rotation.rotate(Vector3.back).scale(this.distance));
    this.transform.lookAt(center);
  }
}

/**
 * Screen-style UI on a plane in the world, like s&box's `WorldPanel`: render React into
 * it with `createRoot(panel)` from `@platypus/react`. The panel stands in its object's XY
 * plane facing +z (towards a camera looking down -z), `width`×`height` UI pixels at
 * `pixelsPerMeter`, centred on the object. Pointer events reach it through the camera
 * (screen UI over it wins), and keys go to it after it's clicked.
 */
export class WorldPanel extends Component {
  private _width = 800;
  private _height = 600;
  private _pixelsPerMeter = 500;
  private _resolution = 2;
  private _tint = Color.white;
  private _interactive = true;
  private started = false;

  /** Width in UI pixels. */
  @property
  get width(): number {
    return this._width;
  }
  set width(v: number) {
    this._width = v;
    this.send();
  }

  /** Height in UI pixels. */
  @property
  get height(): number {
    return this._height;
  }
  set height(v: number) {
    this._height = v;
    this.send();
  }

  /** UI pixels per metre: 500 makes an 800×600 panel 1.6 × 1.2 m. */
  @property
  get pixelsPerMeter(): number {
    return this._pixelsPerMeter;
  }
  set pixelsPerMeter(v: number) {
    this._pixelsPerMeter = v;
    this.send();
  }

  /** Texture pixels per UI pixel: higher keeps text sharp up close (costs memory). */
  @property
  get resolution(): number {
    return this._resolution;
  }
  set resolution(v: number) {
    this._resolution = v;
    this.send();
  }

  /** A colour multiplied over the panel (alpha fades it). */
  @property
  get tint(): Color {
    return this._tint;
  }
  set tint(c: Color) {
    this._tint = c;
    this.send();
  }

  /** Whether the pointer can use it. */
  @property
  get interactive(): boolean {
    return this._interactive;
  }
  set interactive(on: boolean) {
    this._interactive = on;
    this.send();
  }

  /** Its size in metres. */
  get worldSize(): Vector2 {
    return new Vector2(this._width / this._pixelsPerMeter, this._height / this._pixelsPerMeter);
  }

  private send(): void {
    if (!this.started) return;
    __platypus_op(
      "render.panel",
      this.id,
      this.gameObject.id,
      [this._width, this._height],
      this._pixelsPerMeter,
      this._resolution,
      rgba(this._tint),
      this._interactive,
    );
  }

  override onStart(): void {
    this.started = true;
    this.send();
  }

  override onDestroy(): void {
    __platypus_op("render.remove", this.id);
  }
}

/** How a surface covers what's behind it (Unity's Standard shader rendering modes). */
export type RenderingMode = "opaque" | "cutout" | "fade" | "transparent";

/** Settings for `Material.create` / `with`; anything left out keeps its default (or value). */
export interface MaterialOptions {
  color?: Color;
  emission?: Color;
  smoothness?: number;
  metallic?: number;
  unlit?: boolean;
  mainTexture?: Texture | null;
  mainTextureScale?: Vector2Like;
  mainTextureOffset?: Vector2Like;
  normalMap?: Texture | null;
  normalScale?: number;
  metallicGlossMap?: Texture | null;
  specularColor?: Color | null;
  occlusionMap?: Texture | null;
  occlusionStrength?: number;
  emissionMap?: Texture | null;
  detailAlbedoMap?: Texture | null;
  detailAlbedoMapScale?: Vector2Like;
  detailAlbedoMapOffset?: Vector2Like;
  renderingMode?: RenderingMode;
  alphaCutoff?: number;
  doubleSided?: boolean;
  /** @internal glTF's packing of the metallic map (roughness green, metallic blue, occlusion red). */
  gltfPacking?: boolean;
}

/** The settings past the constructor's, with their defaults. */
const EXTRAS = {
  normalMap: null as Texture | null,
  normalScale: 1,
  metallicGlossMap: null as Texture | null,
  specularColor: null as Color | null,
  occlusionMap: null as Texture | null,
  occlusionStrength: 1,
  emissionMap: null as Texture | null,
  detailAlbedoMap: null as Texture | null,
  detailAlbedoMapScale: Vector2.one,
  detailAlbedoMapOffset: Vector2.zero,
  renderingMode: "opaque" as RenderingMode,
  alphaCutoff: 0.5,
  doubleSided: false,
  gltfPacking: false,
};
type Extras = typeof EXTRAS;

/**
 * How a mesh looks: Unity's Standard material. Physically based: `smoothness` 0 is rough
 * (matte), 1 a polished mirror; `metallic` 1 makes it a metal (coloured reflections, no
 * diffuse), or `specularColor` sets the reflectance directly (the specular workflow).
 * Reflections show the sky or the nearest reflection probe. Maps (normal, metallic,
 * occlusion, emission, detail) use the main texture's tiling, except the detail map.
 */
export class Material {
  /** Bumps from a tangent-space normal map (load it with `{ linear: true }`). */
  readonly normalMap: Texture | null = EXTRAS.normalMap;
  /** How strong the normal map's bumps are (Unity's `bumpScale`). */
  readonly normalScale: number = EXTRAS.normalScale;
  /**
   * Metallic in red and smoothness in alpha (Unity's `_MetallicGlossMap`): with it,
   * `metallic` comes from the map and `smoothness` scales the map's. With
   * `specularColor`, it's the specular map instead (colour in rgb, smoothness in alpha).
   */
  readonly metallicGlossMap: Texture | null = EXTRAS.metallicGlossMap;
  /** The specular workflow (Unity's "Standard (Specular setup)"): reflectance head-on; `metallic` is then ignored. */
  readonly specularColor: Color | null = EXTRAS.specularColor;
  /** Ambient occlusion (green channel): darkens sky light and reflections in creases. */
  readonly occlusionMap: Texture | null = EXTRAS.occlusionMap;
  readonly occlusionStrength: number = EXTRAS.occlusionStrength;
  /** Multiplies `emission` (set `emission` to white to see the map as it is). */
  readonly emissionMap: Texture | null = EXTRAS.emissionMap;
  /** Fine detail multiplied over the colour (x2 in gamma space, so mid grey changes nothing), tiled on its own. */
  readonly detailAlbedoMap: Texture | null = EXTRAS.detailAlbedoMap;
  readonly detailAlbedoMapScale: Vector2 = EXTRAS.detailAlbedoMapScale;
  readonly detailAlbedoMapOffset: Vector2 = EXTRAS.detailAlbedoMapOffset;
  /**
   * `opaque` (default; a colour alpha below 1 still fades it), `cutout` (holes where alpha
   * < `alphaCutoff`, shadows too), `fade` (everything fades) or `transparent` (glass:
   * highlights and reflections stay).
   */
  readonly renderingMode: RenderingMode = EXTRAS.renderingMode;
  /** With `cutout`: alpha (texture x colour) below this is a hole. */
  readonly alphaCutoff: number = EXTRAS.alphaCutoff;
  /** Draws back faces too, lit from that side (leaves, cloth, open meshes). */
  readonly doubleSided: boolean = EXTRAS.doubleSided;
  /** @internal */
  readonly gltfPacking: boolean = EXTRAS.gltfPacking;

  constructor(
    /** Base colour (linear; alpha below 1 makes it see-through). For metals, the reflection's tint. */
    readonly color: Color = Color.white,
    /** Light it gives off by itself (linear, can exceed 1 for glow). */
    readonly emission: Color = Color.black,
    /** 0 (rough, matte) to 1 (polished, mirror-like): 1 - roughness. */
    readonly smoothness = 0.5,
    /** Ignore lights: just the colour plus emission. */
    readonly unlit = false,
    /** Multiplies the colour (Unity's `mainTexture`); its alpha counts when `color.a` < 1. */
    readonly mainTexture: Texture | null = null,
    /** How many times the texture repeats across the mesh's UVs. */
    readonly mainTextureScale: Vector2 = Vector2.one,
    /** Where the texture starts (in its 0..1 space). */
    readonly mainTextureOffset: Vector2 = Vector2.zero,
    /** 0 (plastic, stone, wood, skin) to 1 (metal). */
    readonly metallic = 0,
  ) {}

  /** A material from named settings: `Material.create({ color, metallic: 1, smoothness: 0.9 })`. */
  static create(o: MaterialOptions = {}): Material {
    return new Material().with(o);
  }

  /** A copy with some settings changed. */
  with(o: MaterialOptions): Material {
    const v2 = (v: Vector2Like | undefined, d: Vector2) => (v ? new Vector2(v.x, v.y) : d);
    const m = new Material(
      o.color ?? this.color,
      o.emission ?? this.emission,
      o.smoothness ?? this.smoothness,
      o.unlit ?? this.unlit,
      o.mainTexture === undefined ? this.mainTexture : o.mainTexture,
      v2(o.mainTextureScale, this.mainTextureScale),
      v2(o.mainTextureOffset, this.mainTextureOffset),
      o.metallic ?? this.metallic,
    );
    const out = m as unknown as Record<string, unknown>;
    for (const key of Object.keys(EXTRAS) as (keyof Extras)[]) {
      const given = (o as Record<string, unknown>)[key];
      out[key] = given === undefined ? this[key] : given;
    }
    out.detailAlbedoMapScale = v2(o.detailAlbedoMapScale, this.detailAlbedoMapScale);
    out.detailAlbedoMapOffset = v2(o.detailAlbedoMapOffset, this.detailAlbedoMapOffset);
    return m;
  }

  /** A copy with another colour. */
  withColor(color: Color): Material {
    return this.with({ color });
  }

  /** A copy with a texture, repeated `scale` times from `offset`. */
  withTexture(texture: Texture | null, scale: Vector2Like = Vector2.one, offset: Vector2Like = Vector2.zero): Material {
    return this.with({ mainTexture: texture, mainTextureScale: scale, mainTextureOffset: offset });
  }

  /** @internal The maps and modes, as `render.meshRenderer`'s last argument. */
  _extras(): unknown[] {
    const s = this.specularColor;
    return [
      this.normalMap?.id ?? null,
      this.normalScale,
      this.metallicGlossMap?.id ?? null,
      this.gltfPacking,
      this.occlusionMap?.id ?? null,
      this.occlusionStrength,
      this.emissionMap?.id ?? null,
      this.detailAlbedoMap?.id ?? null,
      [this.detailAlbedoMapScale.x, this.detailAlbedoMapScale.y, this.detailAlbedoMapOffset.x, this.detailAlbedoMapOffset.y],
      s ? [s.r, s.g, s.b] : null,
      this.renderingMode,
      this.alphaCutoff,
      this.doubleSided,
    ];
  }
}

let nextMesh = 1;

/**
 * A triangle mesh made by a script (Unity's `Mesh`): vertices, optional normals and
 * texture coordinates (made up when missing), and triangles as index triples,
 * counter-clockwise seen from the front.
 */
/** Up to four bones moving a vertex, by index into a skinned renderer's `bones`, with weights (Unity's `BoneWeight`). */
export interface BoneWeight {
  indices: readonly [number, number, number, number];
  weights: readonly [number, number, number, number];
}

/** A mesh's data, for `Mesh.create` and `set`. */
export interface MeshData {
  vertices: readonly Vector3Like[];
  triangles: readonly number[];
  normals?: readonly Vector3Like[];
  uv?: readonly Vector2Like[];
  /** Skinned meshes: each vertex's bones and weights (weights are made to sum to 1). */
  boneWeights?: readonly BoneWeight[];
  /** Skinned meshes: each bone's inverse world matrix in the bind pose (Unity's `bindposes`). */
  bindposes?: readonly Matrix4x4[];
}

/** Numbers as a plain array or a typed array. */
export type NumberList = readonly number[] | Float32Array | Float64Array | Uint32Array | Uint16Array | Int32Array;

/**
 * A mesh as flat number lists (x, y, z per vertex; u, v per vertex in Unity's bottom-up
 * space), for meshes made in bulk (terrain chunks): no object per vertex. Typed arrays
 * (`Float32Array`, `Uint32Array`) cross to the engine fastest; don't change them after
 * passing them.
 */
export interface FlatMeshData {
  positions: NumberList;
  triangles: NumberList;
  normals?: NumberList;
  uv?: NumberList;
}

export class Mesh {
  readonly id: number;
  private _saved: { vertices: number[]; triangles: number[]; normals: number[]; uv: number[]; joints?: number[]; weights?: number[] } | null = null;
  /** Flat data not yet turned into plain arrays (only saving needs them). */
  private _flat: FlatMeshData | null = null;
  /** @internal Its data as flat arrays (script meshes), for saving with scenes. */
  get _data(): { vertices: number[]; triangles: number[]; normals: number[]; uv: number[]; joints?: number[]; weights?: number[] } | null {
    if (this._flat) {
      const f = this._flat;
      this._saved = { vertices: Array.from(f.positions), triangles: Array.from(f.triangles), normals: Array.from(f.normals ?? []), uv: Array.from(f.uv ?? []) };
      this._flat = null;
    }
    return this._saved;
  }
  set _data(d) {
    this._saved = d;
    this._flat = null;
  }
  /**
   * Each bone's inverse bind pose, for `SkinnedMeshRenderer`s (a model's skinned meshes
   * come with theirs).
   */
  bindposes: Matrix4x4[] = [];
  private constructor(id: number) {
    this.id = id;
  }

  /** @internal A mesh the engine made (a model's), by its id. */
  static _engine(id: number): Mesh {
    return new Mesh(id);
  }

  /** @internal A new script mesh id whose data the engine fills in (a cloth's copy). */
  static _reserve(): Mesh {
    return new Mesh(nextMesh++);
  }

  static create(data: MeshData): Mesh {
    const mesh = new Mesh(nextMesh++);
    mesh.set(data);
    return mesh;
  }

  /** Replaces the mesh's data (everything drawing it updates). */
  set(data: MeshData): void {
    const joints = (data.boneWeights ?? []).flatMap((b) => [...b.indices]);
    const weights = (data.boneWeights ?? []).flatMap((b) => {
      const sum = b.weights.reduce((s, w) => s + w, 0);
      return sum > 0 ? b.weights.map((w) => w / sum) : [1, 0, 0, 0];
    });
    this._data = {
      vertices: data.vertices.flatMap((v) => [v.x, v.y, v.z]),
      triangles: [...data.triangles],
      normals: (data.normals ?? []).flatMap((v) => [v.x, v.y, v.z]),
      uv: (data.uv ?? []).flatMap((v) => [v.x, v.y]),
      ...(joints.length ? { joints, weights } : {}),
    };
    if (data.bindposes) this.bindposes = [...data.bindposes];
    __platypus_op(
      "render.mesh",
      this.id,
      data.vertices.flatMap((v) => [v.x, v.y, v.z]),
      (data.normals ?? []).flatMap((v) => [v.x, v.y, v.z]),
      // Unity's UVs run bottom-up; the engine's top-down.
      (data.uv ?? []).flatMap((v) => [v.x, 1 - v.y]),
      [...data.triangles],
      ...(joints.length ? [joints, weights] : []),
    );
  }

  /** A mesh from flat arrays (see `FlatMeshData`). */
  static createFlat(data: FlatMeshData): Mesh {
    const mesh = new Mesh(nextMesh++);
    mesh.setFlat(data);
    return mesh;
  }

  /** Replaces the mesh's data from flat lists: the fast way for big, rebuilt meshes. */
  setFlat(data: FlatMeshData): void {
    const uv = data.uv ?? [];
    // Unity's UVs run bottom-up; the engine's top-down.
    const flipped = new Float32Array(uv.length);
    for (let i = 0; i < uv.length; i += 2) {
      flipped[i] = uv[i]!;
      flipped[i + 1] = 1 - uv[i + 1]!;
    }
    this._flat = data;
    __platypus_op("render.mesh", this.id, data.positions, data.normals ?? [], flipped, data.triangles);
  }

  /** Frees it (renderers still using it draw nothing). */
  destroy(): void {
    __platypus_op("render.mesh", this.id, null);
  }
}

/** Unity's built-in meshes. */
export type PrimitiveMesh = "cube" | "sphere" | "capsule" | "cylinder" | "plane" | "quad";

/**
 * Draws a mesh at the object (Unity's `MeshFilter` + `MeshRenderer` in one): a built-in
 * primitive by name, or a script `Mesh`, with a `Material`.
 */
export class MeshRenderer extends Component {
  private _mesh: PrimitiveMesh | Mesh = "cube";
  private _material = new Material();
  private _shadowCastingMode: "on" | "off" = "on";
  private started = false;

  /** Whether it casts shadows (Unity's `shadowCastingMode`: `on` or `off`). */
  @property
  get shadowCastingMode(): "on" | "off" {
    return this._shadowCastingMode;
  }
  set shadowCastingMode(m: "on" | "off") {
    this._shadowCastingMode = m;
    this.send();
  }

  @property
  get mesh(): PrimitiveMesh | Mesh {
    return this._mesh;
  }
  set mesh(m: PrimitiveMesh | Mesh) {
    this._mesh = m;
    this.send();
  }

  @property
  get material(): Material {
    return this._material;
  }
  set material(m: Material) {
    this._material = m;
    this.send();
  }

  /** @internal Extra arguments a subclass sends (a skinned renderer's bones). */
  protected _skinArgs(): unknown[] {
    return [];
  }

  /** @internal */
  protected send(): void {
    if (!this.started) return;
    const m = this._material;
    __platypus_op(
      "render.meshRenderer",
      this.id,
      this.gameObject.id,
      typeof this._mesh === "string" ? this._mesh : this._mesh.id,
      rgba(m.color),
      [m.emission.r, m.emission.g, m.emission.b],
      m.smoothness,
      m.unlit,
      m.mainTexture?.id ?? null,
      [m.mainTextureScale.x, m.mainTextureScale.y, m.mainTextureOffset.x, m.mainTextureOffset.y],
      this._shadowCastingMode !== "off",
      m.metallic,
      m._extras(),
      ...this._skinArgs(),
    );
  }

  override onAwake(): void {
    this.started = true;
    this.send();
  }

  override onDestroy(): void {
    __platypus_op("render.remove", this.id);
  }
}

/**
 * Draws a skinned mesh (Unity's `SkinnedMeshRenderer`): each vertex moves with up to four
 * of `bones` by its weights, from the mesh's bind poses. Animating the bones (an
 * `Animator` or `Animation`) bends the mesh. The renderer's own transform doesn't
 * apply: bones are in the world.
 */
export class SkinnedMeshRenderer extends MeshRenderer {
  private _bones: GameObject[] = [];
  /** The skeleton's top bone (Unity's `rootBone`). */
  @property rootBone: GameObject | null = null;

  /** The objects the mesh's bones follow, in the order its bone weights and bind poses use. */
  @property
  get bones(): GameObject[] {
    return this._bones;
  }
  set bones(b: GameObject[]) {
    this._bones = [...b];
    this.send();
  }

  protected override _skinArgs(): unknown[] {
    const mesh = this.mesh;
    const bindposes = typeof mesh === "string" ? [] : mesh.bindposes;
    const count = Math.min(this._bones.length, bindposes.length);
    return [this._bones.slice(0, count).map((b) => b.id), bindposes.slice(0, count).flatMap((m) => [...m.m])];
  }
}

export type LightType = "directional" | "point" | "spot";

/** Unity's `LightShadows`. */
export type LightShadows = "none" | "hard" | "soft";

/**
 * Lights meshes (Unity's `Light`). Directional lights shine along the object's forward
 * (-z) everywhere, like the sun; point lights shine from the object out to `range`;
 * spot lights shine a cone `spotAngle` degrees wide along forward.
 */
export class Light extends Component {
  private _type: LightType = "directional";
  private _color = Color.white;
  private _intensity = 1;
  private _range = 10;
  private _spotAngle = 30;
  private _innerSpotAngle = 21.8;
  private _haloSize = 0;
  private _beamIntensity = 0;
  private _shadows: LightShadows = "none";
  private _shadowStrength = 1;
  private _cullingMask = 0xffffffff;
  private _volumetricIntensity = 1;
  private _spotShape: "cone" | "pyramid" = "cone";
  private _aspectRatio = 1;
  private started = false;

  @property
  get type(): LightType {
    return this._type;
  }
  set type(t: LightType) {
    this._type = t;
    this.send();
  }
  @property
  get color(): Color {
    return this._color;
  }
  set color(c: Color) {
    this._color = c;
    this.send();
  }
  @property
  get intensity(): number {
    return this._intensity;
  }
  set intensity(i: number) {
    this._intensity = i;
    this.send();
  }
  /** How far point and spot lights reach (metres). */
  @property
  get range(): number {
    return this._range;
  }
  set range(r: number) {
    this._range = r;
    this.send();
  }
  /** A spot light's full cone angle in degrees. */
  @property
  get spotAngle(): number {
    return this._spotAngle;
  }
  set spotAngle(a: number) {
    this._spotAngle = a;
    this.send();
  }
  /** Where a spot light starts fading towards its edge (degrees, full angle). */
  @property
  get innerSpotAngle(): number {
    return this._innerSpotAngle;
  }
  set innerSpotAngle(a: number) {
    this._innerSpotAngle = a;
    this.send();
  }

  /** A glow around the light this many metres across, in its colour (0: none; Unity's halo). */
  @property
  get haloSize(): number {
    return this._haloSize;
  }
  set haloSize(s: number) {
    this._haloSize = s;
    this.send();
  }

  /**
   * A spot light's visible beam: a soft cone of light in the air, as long as `range`
   * (0: none; 0.3–1 looks like light through dust or haze).
   */
  @property
  get beamIntensity(): number {
    return this._beamIntensity;
  }
  set beamIntensity(b: number) {
    this._beamIntensity = b;
    this.send();
  }

  /**
   * Shadows it casts (Unity's `LightShadows`): `hard`, `soft` (filtered edges) or `none`.
   * Only the first directional light with shadows casts them, for now, within
   * `QualitySettings.shadowDistance` of the camera.
   */
  @property
  get shadows(): LightShadows {
    return this._shadows;
  }
  set shadows(s: LightShadows) {
    this._shadows = s;
    this.send();
  }
  /** How dark its shadows are, 0 to 1. */
  @property
  get shadowStrength(): number {
    return this._shadowStrength;
  }
  set shadowStrength(s: number) {
    this._shadowStrength = s;
    this.send();
  }

  /**
   * The layers it lights, a bit each (all by default). Objects on other layers ignore
   * it and don't cast its shadows.
   */
  @property
  get cullingMask(): number {
    return this._cullingMask;
  }
  set cullingMask(mask: number) {
    this._cullingMask = mask >>> 0;
    this.send();
  }

  /**
   * How strongly it lights the fog for light shafts (`RenderSettings.volumetricFog`): 1
   * as brightly as it lights surfaces, 0 not at all (HDRP's volumetric multiplier).
   */
  @property
  get volumetricIntensity(): number {
    return this._volumetricIntensity;
  }
  set volumetricIntensity(v: number) {
    this._volumetricIntensity = Math.max(0, v);
    this.send();
  }

  /**
   * A spot light's shape (HDRP's): a `cone`, or a `pyramid` `spotAngle` high and
   * `aspectRatio` times as wide (in tangent), its width along the object's x. A thin, wide
   * pyramid is a sheet of light: a scanner's, a laser level's, light through a slot.
   */
  get spotShape(): "cone" | "pyramid" {
    return this._spotShape;
  }
  set spotShape(s: "cone" | "pyramid") {
    this._spotShape = s;
    this.send();
  }
  /** A pyramid spot light's width over its height (1). */
  get aspectRatio(): number {
    return this._aspectRatio;
  }
  set aspectRatio(a: number) {
    this._aspectRatio = Math.max(0.001, a);
    this.send();
  }

  private send(): void {
    if (!this.started) return;
    const c = this._color;
    __platypus_op(
      "render.light",
      this.id,
      this.gameObject.id,
      this._type,
      [c.r, c.g, c.b],
      this._intensity,
      this._range,
      this._spotAngle,
      this._innerSpotAngle,
      this._haloSize,
      this._beamIntensity,
      this._shadows,
      this._shadowStrength,
      this._cullingMask,
      this._volumetricIntensity,
      this._spotShape,
      this._aspectRatio,
    );
  }

  override onAwake(): void {
    this.started = true;
    allLights.add(this);
    this.send();
  }

  override onDestroy(): void {
    allLights.delete(this);
    __platypus_op("render.remove", this.id);
  }

  /**
   * A light meter: how much light falls on `point` from every light, in lux in the HD
   * pipeline (where lights are in lux and lumens; elsewhere in Platypus's own units). Facing
   * `normal` if given (a surface's), else each light head-on. Shadows aren't counted: indoors
   * it counts the sun through the roof. `measureIlluminance` does count them.
   */
  static illuminanceAt(point: Vector3Like, normal?: Vector3Like): number {
    let sum = 0;
    for (const l of allLights) sum += l.illuminanceFrom(point, normal);
    return sum;
  }

  /**
   * A light meter that counts shadows: as `illuminanceAt`, but each light only if a physics
   * ray from `point` reaches it (so walls and roofs need colliders). Resolves after the next
   * physics tick.
   */
  static async measureIlluminance(point: Vector3Like, normal?: Vector3Like): Promise<number> {
    const parts = [...allLights].map(async (l) => {
      const lux = l.illuminanceFrom(point, normal);
      if (lux <= 0) return 0;
      let dir: Vector3Like;
      let distance: number;
      if (l._type === "directional") {
        const f = l.transform.forward;
        dir = { x: -f.x, y: -f.y, z: -f.z };
        distance = 10000;
      } else {
        const p = l.transform.position;
        dir = { x: p.x - point.x, y: p.y - point.y, z: p.z - point.z };
        distance = Math.hypot(dir.x, dir.y, dir.z) - 0.05;
      }
      // Start a little off the point, so the surface it's on doesn't block it.
      const len = Math.hypot(dir.x, dir.y, dir.z) || 1;
      const from = { x: point.x + (dir.x / len) * 0.02, y: point.y + (dir.y / len) * 0.02, z: point.z + (dir.z / len) * 0.02 };
      const hit = await physics.raycast(from, dir, distance);
      return hit ? 0 : lux;
    });
    return (await Promise.all(parts)).reduce((a, b) => a + b, 0);
  }

  /** The light this light alone sheds on `point` (facing `normal`), shadows aside. */
  private illuminanceFrom(point: Vector3Like, normal?: Vector3Like): number {
    const l = this;
    if (!l.gameObject.activeInHierarchy) return 0;
    const forward = l.transform.forward;
    if (l._type === "directional") {
      const cos = normal ? Math.max(0, -(normal.x * forward.x + normal.y * forward.y + normal.z * forward.z)) : 1;
      return l._intensity * cos;
    }
    const p = l.transform.position;
    const dx = point.x - p.x;
    const dy = point.y - p.y;
    const dz = point.z - p.z;
    const d = Math.hypot(dx, dy, dz);
    if (d >= l._range || d === 0) return 0;
    // As the renderer: a smooth fade to zero at the range.
    const x = d / l._range;
    const fade = (1 - x ** 4) ** 2;
    // Lumens to candela, all round (HDRP's spots without "reflector").
    let candela = l._intensity / (4 * Math.PI);
    if (l._type === "spot") {
      const c = (dx * forward.x + dy * forward.y + dz * forward.z) / d;
      const outer = Math.cos(((l._spotAngle / 2) * Math.PI) / 180);
      const inner = Math.cos(((l._innerSpotAngle / 2) * Math.PI) / 180);
      const t = Math.min(1, Math.max(0, (c - outer) / Math.max(inner - outer, 1e-4)));
      candela *= t * t * (3 - 2 * t);
    }
    const cos = normal ? Math.max(0, -(normal.x * dx + normal.y * dy + normal.z * dz) / d) : 1;
    return ((candela * fade) / (d * d)) * cos;
  }
}

/** Every awake light, for `Light.illuminanceAt`. */
const allLights = new Set<Light>();

let ambient = new Color(0.2, 0.2, 0.22);

/**
 * What the world renderer drew and skipped in the last frame, over every camera, mirror and
 * reflection probe (like Unity's rendering statistics). Read-only; the engine updates it.
 */
export const RenderStats = {
  meshes: 0,
  /** Meshes left out: outside a camera's view or hidden by fog. */
  culledMeshes: 0,
  quads: 0,
  culledQuads: 0,
  /** Of the culled meshes and quads, those `RenderSettings.fogCulling` skipped. */
  fogCulled: 0,
  lights: 0,
  culledLights: 0,
  /** Shadow map layers drawn. */
  shadowMaps: 0,
};

globalThis.__platypus_render_stats = (...n: number[]) => {
  [RenderStats.meshes, RenderStats.culledMeshes, RenderStats.quads, RenderStats.culledQuads, RenderStats.fogCulled, RenderStats.lights, RenderStats.culledLights, RenderStats.shadowMaps] = n.map((v) => v ?? 0) as [number, number, number, number, number, number, number, number];
};

/** Unity's Bloom: bright things (emission, hot lights) glow onto their surroundings. */
export interface BloomSettings {
  /** Linear brightness where glowing starts (1: only what's brighter than white). */
  threshold: number;
  /** How strong the glow is. */
  intensity: number;
  /** 0..1: how far it spreads. */
  scatter: number;
}

let bloom: BloomSettings | null = null;

/** How HDR light becomes screen colours (Unity's `TonemappingMode`). */
export type TonemappingMode = "none" | "neutral" | "aces";
let tonemapping: TonemappingMode = "none";
let postExposure = 0;

/**
 * Auto exposure (HDRP's Automatic mode): the view is metered, weighted towards its middle,
 * and the exposure eases towards one that shows its average as mid grey, as eyes adapt. In
 * a dark tunnel the opening ahead burns white; stepping out into daylight blinds for a
 * moment, then settles. Pair it with bloom and tonemapping.
 */
export interface AutoExposureSettings {
  /** The darkest average brightness it adapts to, in EV100 (-10): darker stays dark. */
  minEV?: number;
  /** The brightest, in EV100 (20): brighter stays bright. */
  maxEV?: number;
  /** Stops added after metering (0; positive is brighter). */
  compensation?: number;
  /** How fast it adapts when the view gets brighter (3, per second). */
  speedDarkToLight?: number;
  /** How fast it adapts when the view gets darker (3, per second: about a second for a big change). */
  speedLightToDark?: number;
  /**
   * Which part of the view counts: `average` all alike, `centerWeighted` (the default) the
   * middle most, `spot` only a small circle in the middle.
   */
  metering?: "average" | "centerWeighted" | "spot";
  /**
   * The part of the brightness histogram averaged, in percent from the darkest (0 and 100:
   * everything, the default). HDRP suggests 40 and 90: then a little sun or a few black
   * pixels can't swing it.
   */
  lowPercent?: number;
  highPercent?: number;
  /**
   * A soft floor instead of `minEV`'s wall: below `darkBelow` (EV100, 5) the eyes adapt only
   * `darkAdaptation` (0..1, default 1: fully) of the way, so a dark room brightens some but
   * still reads as dark.
   */
  darkAdaptation?: number;
  darkBelow?: number;
  /**
   * More stops by how bright the view is (HDRP's curve mapping): up to 8 points of
   * [metered EV100, stops], joined by straight lines. [[-2, -1.5], [6, 0]] keeps dark
   * places darker without touching bright ones.
   */
  curve?: [number, number][];
  /** Stops adapting: the exposure holds where it is. */
  hold?: boolean;
}

/** `metering` as the op sends it. */
const METERING = { average: 0, centerWeighted: 1, spot: 2 } as const;
/** Manual exposure (EV100), and the seconds auto exposure takes to blend in and out. */
let fixedExposure: number | null = null;
let exposureTransition = 0;

/** @internal Each camera's exposure, read back: EV100 in use, and how bright it metered. */
globalThis.__platypus_exposure = (id: number, ev: number, scene: number | null) => {
  const c = Camera.byId(id);
  if (!c) return;
  c._exposure = ev;
  c._sceneBrightness = scene ?? Number.NaN;
};

let autoExposure: AutoExposureSettings | null = null;
const sendPost = () =>
  __platypus_op(
    "render.post",
    0,
    postExposure,
    tonemapping,
    autoExposure && QualitySettings.autoExposure
      ? [
          autoExposure.minEV ?? -10,
          autoExposure.maxEV ?? 20,
          autoExposure.compensation ?? 0,
          autoExposure.speedDarkToLight ?? 3,
          autoExposure.speedLightToDark ?? 3,
          METERING[autoExposure.metering ?? "centerWeighted"],
          autoExposure.lowPercent ?? 0,
          autoExposure.highPercent ?? 100,
          autoExposure.hold ? 1 : 0,
          autoExposure.darkAdaptation ?? 1,
          autoExposure.darkBelow ?? 5,
          Math.min(8, autoExposure.curve?.length ?? 0),
          ...(autoExposure.curve ?? []).slice(0, 8).flat(),
        ]
      : null,
    fixedExposure,
    exposureTransition,
  );
// The player's auto exposure option (QualitySettings) can turn it off.
_setPostHook(sendPost);

/**
 * A procedural sky (Unity's procedural skybox): gradients from the zenith to the horizon
 * to the ground, and a sun disc. Colours are linear and can exceed 1 (the sun usually
 * does, so it glows and shows in shiny things).
 */
export interface ProceduralSky {
  zenith?: Color;
  horizon?: Color;
  ground?: Color;
  /** Towards the sun; `null` for no sun disc. */
  sunDirection?: Vector3Like | null;
  sunColor?: Color;
  /** The disc's size in degrees. */
  sunSize?: number;
}

/**
 * How fog thickens with distance (Unity's `FogMode`): `linear` from `fogStartDistance`
 * to `fogEndDistance`; `exponential` by `fogDensity` per metre; `exponentialSquared`
 * clear nearby, then closing in.
 */
export type FogMode = "linear" | "exponential" | "exponentialSquared";

let fog = false;
let fogMode: FogMode = "exponentialSquared";
let fogColor = new Color(0.5, 0.5, 0.5);
let fogDensity = 0.01;
let fogStart = 0;
let fogEnd = 300;

/**
 * Fog lying low (Unreal's exponential height fog), in `fogColor`: `density` per metre at
 * `baseHeight`, thinning by a factor of e every `1 / falloff` metres above it and
 * thickening below. It covers the sky and background too, so the horizon sinks into it.
 */
export interface HeightFogSettings {
  density: number;
  baseHeight: number;
  /** Per metre (larger: a thinner layer). */
  falloff: number;
}

let heightFog: HeightFogSettings | null = null;

/**
 * A deck of cloud drawn with the background (HDRP's cloud layer): behind everything, so the
 * distance fog doesn't hide it. Each pixel of sky looks up to where its ray meets the deck,
 * so the clouds shift overhead as the camera moves under them; the wind carries them; they
 * thin out towards the horizon. Every field is optional.
 */
export interface CloudLayerSettings {
  /** The deck's height in metres (1500). */
  altitude?: number;
  /** About how wide a cloud is, metres (2000). */
  size?: number;
  /** How much of the sky is cloud, 0 to 1 (0.5). */
  coverage?: number;
  /** 0 to 1 (1). */
  opacity?: number;
  /** How soft the clouds' edges are, 0 (hard) to 1 (0.3). */
  softness?: number;
  /** Their colour (linear; thick parts are a little darker underneath). Light grey. */
  tint?: Color;
  /** The wind, m/s (0) and the way it blows in degrees from +x towards +z (0). */
  windSpeed?: number;
  windDirection?: number;
  /** Over about this distance along a ray they fade into the sky, metres (20000). */
  fadeDistance?: number;
}

let cloudLayer: CloudLayerSettings | null = null;

function sendClouds(): void {
  const c = cloudLayer;
  const t = c?.tint;
  __platypus_op(
    "render.clouds",
    0,
    c
      ? [c.altitude ?? 1500, c.size ?? 2000, c.coverage ?? 0.5, c.opacity ?? 1, c.softness ?? 0.3, t?.r ?? 0.8, t?.g ?? 0.8, t?.b ?? 0.82, c.windSpeed ?? 0, c.windDirection ?? 0, c.fadeDistance ?? 20000]
      : null,
  );
}
let fogCulling = true;

/**
 * Light shafts (volumetric fog): lights shining through the fog light it up, cut by their
 * shadows, so the sun shows in beams through gaps and spot lights as cones, tinted by
 * `albedo`. Every field is optional. A froxel volume over each camera's view (240 × 135
 * cells, 64 depth slices) is filled once a frame, with a shadow lookup per light per
 * sample; lights opt out with `Light.volumetricIntensity = 0`.
 */
export interface VolumetricFogSettings {
  /** Scales the scattered light (1 by default). */
  intensity?: number;
  /** -1 to 1: positive scatters forward, so looking towards a light shows more (0.4). */
  anisotropy?: number;
  /** Samples per depth slice, 1 to 16 (4): more keep thin cones sampled. */
  steps?: number;
  /** How far the volume reaches in metres (50); fog beyond counts as lit by the sun. */
  distance?: number;
  /**
   * How much of each colour of light the fog scatters (white by default). It's separate
   * from `fogColor`, which is how the fog looks in the ambient light: a night fog is dark,
   * yet glows under a lamp.
   */
  albedo?: Color;
  /**
   * God rays: within this many metres of the camera the sun's shafts are worked out per
   * pixel from its shadow map, finding each beam's edges exactly (0, the default: off).
   * Suits large openings (a canopy, a roof hole); shadows thinner than its sample spacing,
   * such as window bars, alias into a hatch. For those, a finer `resolution` is the cleaner
   * way to sharpen beams. Costs `sharpSamples` shadow lookups per fogged pixel.
   */
  sharpDistance?: number;
  /** Samples per pixel for the sharp part, 4 to 128 (32). */
  sharpSamples?: number;
  /**
   * How fine the light-shaft volume is (`high` by default): `low` 160 × 90 cells by 48 depth
   * slices, `medium` 240 × 135 × 64, `high` 320 × 180 × 96, `ultra` 480 × 270 × 128. Finer
   * is sharper (beams through bars and gaps) and costs more to fill each frame.
   */
  resolution?: VolumeDetail;
  /**
   * How the light volume's sampling noise is cleaned up (HDRP's denoising mode): `both`
   * (the default) blends each cell with last frame's (`reprojection`) and blurs it over a
   * cell's width (`gaussian`), so a blind's slats or a grate come out as clean haze;
   * `reprojection` alone saves three texture reads a step (slower GPUs, phones); `none`
   * samples each frame afresh (beams through fine patterns step and shimmer).
   */
  denoising?: "none" | "reprojection" | "gaussian" | "both";
  /**
   * With reprojection, how much of each cell comes from earlier frames (0.9, Unreal's
   * history weight): up to 0.98 is smoother still, but slower to follow a moving sun or
   * camera.
   */
  historyWeight?: number;
  /** Shadow samples per cell each frame, 1 to 8 (1): more is less noise, at that much the cost. */
  samples?: number;
  /**
   * Samples for cells whose view moved this frame (2; adaptive): the camera moving is when the
   * history counts least, so this takes the noise out of turning and walking at a cost only
   * while it happens.
   */
  motionSamples?: number;
}

const DENOISING = { none: 0, reprojection: 1, gaussian: 2, both: 3 } as const;

/** The light-shaft volume's detail levels. */
export type VolumeDetail = "low" | "medium" | "high" | "ultra";
const VOLUME_DETAILS: VolumeDetail[] = ["low", "medium", "high", "ultra"];

let volumetricFog: VolumetricFogSettings | null = null;

function sendFog(): void {
  if (!fog) {
    __platypus_op("render.fog", 0, null);
    return;
  }
  const h = heightFog ? [heightFog.density, heightFog.baseHeight, heightFog.falloff] : null;
  const v = volumetricFog;
  const a = v?.albedo;
  const shafts = v ? [v.intensity ?? 1, v.anisotropy ?? 0.4, v.steps ?? 4, v.distance ?? 50, a?.r ?? 1, a?.g ?? 1, a?.b ?? 1, v.sharpDistance ?? 0, v.sharpSamples ?? 32, Math.max(0, VOLUME_DETAILS.indexOf(v.resolution ?? "high")), DENOISING[v.denoising ?? "both"], v.historyWeight ?? 0.9, v.samples ?? 1, v.motionSamples ?? 2] : null;
  __platypus_op("render.fog", 0, fogMode, [fogColor.r, fogColor.g, fogColor.b], fogDensity, fogStart, fogEnd, h, shafts, fogCulling);
}

let skybox: ProceduralSky | null = null;
let ambientMode: "skybox" | "color" = "skybox";
let reflectionIntensity = 1;

function sendSky(): void {
  if (!skybox) {
    __platypus_op("render.sky", 0, null);
    return;
  }
  const s = skybox;
  const rgb = (c: Color | undefined) => (c ? [c.r, c.g, c.b] : null);
  const sun = s.sunDirection === null ? null : s.sunDirection ? [s.sunDirection.x, s.sunDirection.y, s.sunDirection.z] : [0.4, 0.6, 0.3];
  __platypus_op(
    "render.sky",
    0,
    rgb(s.zenith) ?? [0.16, 0.32, 0.62],
    rgb(s.horizon) ?? [0.62, 0.72, 0.82],
    rgb(s.ground) ?? [0.22, 0.2, 0.18],
    sun,
    rgb(s.sunColor) ?? [20, 18, 15],
    s.sunSize ?? 1.5,
    reflectionIntensity,
    ambientMode === "skybox",
  );
}

/** Scene-wide lighting (Unity's `RenderSettings`). */
export const RenderSettings = {
  /** Light everything gets regardless of lights (linear). */
  get ambientLight(): Color {
    return ambient;
  },
  set ambientLight(c: Color) {
    ambient = c;
    __platypus_op("render.ambient", 0, [c.r, c.g, c.b]);
  },
  /**
   * The sky: what reflections show, where sky light comes from, and what cameras with
   * `clearFlags: "skybox"` show behind everything. `null` (the default) means none:
   * reflections then show the ambient colour.
   */
  get skybox(): ProceduralSky | null {
    return skybox;
  },
  set skybox(s: ProceduralSky | null) {
    skybox = s ? { ...s } : null;
    sendSky();
  },
  /** Where ambient light comes from: the sky (`skybox`, with one) or `ambientLight`. */
  get ambientMode(): "skybox" | "color" {
    return ambientMode;
  },
  set ambientMode(m: "skybox" | "color") {
    ambientMode = m;
    if (skybox) sendSky();
  },
  /** How bright the sky's reflections and light are. */
  get reflectionIntensity(): number {
    return reflectionIntensity;
  },
  set reflectionIntensity(i: number) {
    reflectionIntensity = i;
    if (skybox) sendSky();
  },
  /** Bloom over every camera (`null`: off, the default). */
  get bloom(): BloomSettings | null {
    return bloom;
  },
  set bloom(b: BloomSettings | null) {
    bloom = b ? { ...b } : null;
    if (b) __platypus_op("render.bloom", 0, b.threshold, b.intensity, b.scatter);
    else __platypus_op("render.bloom", 0, null);
  },
  /**
   * How HDR light (brighter than white) becomes screen colours: `none` clips it (the
   * default), `neutral` keeps colours and rolls highlights off softly, `aces` is filmic.
   */
  get tonemapping(): TonemappingMode {
    return tonemapping;
  },
  set tonemapping(t: TonemappingMode) {
    tonemapping = t;
    sendPost();
  },
  /** Brightens (positive) or darkens (negative) everything, in stops, before tonemapping. */
  get postExposure(): number {
    return postExposure;
  },
  set postExposure(ev: number) {
    postExposure = ev;
    sendPost();
  },
  /** Adapts the exposure to the view (off: null, the default). See `AutoExposureSettings`. */
  get autoExposure(): AutoExposureSettings | null {
    return autoExposure;
  },
  set autoExposure(a: AutoExposureSettings | null) {
    autoExposure = a ? { ...a, curve: a.curve?.map((p) => [p[0], p[1]] as [number, number]) } : null;
    sendPost();
  },
  /**
   * The manual exposure, in EV100 (HDRP's Fixed mode; higher is darker), used without auto
   * exposure and blended with it while it fades in or out. null (the default): light as it
   * is.
   */
  get fixedExposure(): number | null {
    return fixedExposure;
  },
  set fixedExposure(ev: number | null) {
    fixedExposure = ev;
    sendPost();
  },
  /**
   * Seconds auto exposure takes to blend in when it's turned on and out (to the manual
   * exposure) when it's turned off, and a change of `fixedExposure` takes to ease to the
   * new value (0, the default: at once).
   */
  get exposureTransition(): number {
    return exposureTransition;
  },
  set exposureTransition(s: number) {
    exposureTransition = Math.max(0, s);
    sendPost();
  },
  /**
   * Turns auto exposure off without anything changing on screen: the manual exposure is
   * set to where the main camera's eyes are now. Fade it elsewhere later with
   * `fixedExposure` (and `exposureTransition` for a slow, unnoticed blend).
   */
  freezeExposure(): void {
    const ev = Camera.main?.exposure;
    if (ev !== undefined && Number.isFinite(ev)) fixedExposure = ev;
    autoExposure = null;
    sendPost();
  },
  /**
   * Fog (off by default): things fade towards `fogColor` with distance from the camera;
   * glows fade out. Backgrounds and skies stay clear, so match the camera's background
   * colour to the fog for a seamless horizon. Cameras can opt out (`Camera.fog`).
   */
  get fog(): boolean {
    return fog;
  },
  set fog(on: boolean) {
    fog = on;
    sendFog();
  },
  get fogMode(): FogMode {
    return fogMode;
  },
  set fogMode(m: FogMode) {
    fogMode = m;
    if (fog) sendFog();
  },
  /** The fog's colour (linear). */
  get fogColor(): Color {
    return fogColor;
  },
  set fogColor(c: Color) {
    fogColor = c;
    if (fog) sendFog();
  },
  /** How thick the exponential modes are, per metre. */
  get fogDensity(): number {
    return fogDensity;
  },
  set fogDensity(d: number) {
    fogDensity = Math.max(0, d);
    if (fog) sendFog();
  },
  /** Where linear fog begins, in metres from the camera. */
  get fogStartDistance(): number {
    return fogStart;
  },
  set fogStartDistance(d: number) {
    fogStart = d;
    if (fog) sendFog();
  },
  /** Where linear fog hides everything. */
  get fogEndDistance(): number {
    return fogEnd;
  },
  set fogEndDistance(d: number) {
    fogEnd = d;
    if (fog) sendFog();
  },
  /**
   * Height fog on top of the distance fog, while `fog` is on (`null`: none, the default).
   * For height fog alone, use `fogMode = "exponential"` with `fogDensity = 0`.
   */
  get heightFog(): HeightFogSettings | null {
    return heightFog;
  },
  /** The cloud layer (`null`: none, the default). See `CloudLayerSettings`. */
  get cloudLayer(): CloudLayerSettings | null {
    return cloudLayer;
  },
  set cloudLayer(c: CloudLayerSettings | null) {
    cloudLayer = c ? { ...c } : null;
    sendClouds();
  },
  set heightFog(h: HeightFogSettings | null) {
    heightFog = h ? { ...h } : null;
    if (fog) sendFog();
  },
  /**
   * Skip drawing what lies wholly past where the distance fog hides everything (on by
   * default): linear fog's end, about 5.5 / density for exponential fog, 2.35 / density for
   * squared. With the camera's background in `fogColor` nothing changes on screen; with
   * another background, things past it would have shown as fog-coloured silhouettes.
   * `RenderStats.fogCulled` counts what it saved.
   */
  get fogCulling(): boolean {
    return fogCulling;
  },
  set fogCulling(on: boolean) {
    fogCulling = on;
    if (fog) sendFog();
  },
  /** Light shafts in the fog, while `fog` is on (`null`: none, the default). */
  get volumetricFog(): VolumetricFogSettings | null {
    return volumetricFog;
  },
  set volumetricFog(v: VolumetricFogSettings | null) {
    volumetricFog = v ? { ...v } : null;
    if (fog) sendFog();
  },
};

/**
 * Makes the object's `MeshRenderer` a flat mirror: it reflects the scene exactly, as seen
 * by the main camera, in the plane through the object's origin. Give the renderer a
 * metallic, smooth material for a clear mirror (`Material.create({ metallic: 1,
 * smoothness: 1 })`); a colour tints the reflection. Each mirror renders the scene once
 * more per frame. Other cameras see the sky in it.
 */
export class Mirror extends Component {
  /** The plane's normal in the object's space: up (+y) for planes, +z for quads. */
  @property
  normal: Vector3Like | null = null;
  /** The layers it reflects, a bit each (all by default; set before it starts). */
  @property
  cullingMask = 0xffffffff;

  private send(): void {
    const mesh = this.getComponent(MeshRenderer)?.mesh;
    // Quads face +z (`Vector3.back`: forward is -z here).
    const n = this.normal ?? (mesh === "quad" ? Vector3.back : Vector3.up);
    __platypus_op("render.mirror", this.id, this.gameObject.id, [n.x, n.y, n.z], this.cullingMask >>> 0);
  }

  override onAwake(): void {
    this.send();
  }

  override onDestroy(): void {
    __platypus_op("render.remove", this.id);
  }
}

/**
 * Captures the scene around its object into a cubemap that shiny things inside its box
 * reflect instead of the sky (Unity's `ReflectionProbe`). Rendering it costs six views of
 * the scene, so by default it renders once (`refreshMode: "onAwake"`); `everyFrame` keeps
 * it live, and `viaScripting` renders once when added and then only on `renderProbe()`.
 */
export class ReflectionProbe extends Component {
  /** The box of influence, centred on the object (metres). */
  @property
  size: Vector3Like = new Vector3(10, 10, 10);
  /** The cubemap's face size in texels (a power of two, 16 to 1024). */
  @property
  resolution = 128;
  /** Correct reflections for the box's walls (rooms) instead of treating them as far away. */
  @property
  boxProjection = false;
  /** The layers it captures, a bit each (all by default; set before it starts). */
  @property
  cullingMask = 0xffffffff;
  @property
  refreshMode: "onAwake" | "everyFrame" | "viaScripting" = "onAwake";

  private send(): void {
    const s = this.size;
    __platypus_op("render.probe", this.id, this.gameObject.id, [s.x, s.y, s.z], this.resolution, this.boxProjection, this.refreshMode === "everyFrame", this.cullingMask >>> 0);
  }

  /** Captures the scene again on the next frame (after moving things, say). */
  renderProbe(): void {
    __platypus_op("render.probeRender", this.id);
  }

  override onAwake(): void {
    this.send();
  }

  override onDestroy(): void {
    __platypus_op("render.remove", this.id);
  }
}

/**
 * A box of fog in its own colour (HDRP's local volumetric fog): mist over a pond, smoke in
 * a room, a cloud layer to fly through. It's centred on the object and lined up with the
 * world's axes (the object's scale stretches it; its rotation doesn't turn it), thick
 * inside and thinning over `blendDistance` from its faces. It works with or without
 * `RenderSettings.fog`, covers the sky behind it too, and light shafts light it. Each camera
 * draws the 8 nearest; `Camera.fog = false` hides them.
 */
export class FogVolume extends Component {
  private _size: Vector3Like = new Vector3(10, 2, 10);
  private _density = 0.5;
  private _color = new Color(0.75, 0.78, 0.82, 1);
  private _blend = 1;
  private _shape: "box" | "ellipsoid" = "box";
  private started = false;

  /** The box's size in metres, before the object's scale. */
  @property
  get size(): Vector3Like {
    return this._size;
  }
  set size(s: Vector3Like) {
    this._size = { x: s.x, y: s.y, z: s.z };
    this.send();
  }
  /** How thick it is inside, per metre (0.5: a metre of it hides 40% of what's behind). */
  @property
  get density(): number {
    return this._density;
  }
  set density(d: number) {
    this._density = Math.max(0, d);
    this.send();
  }
  /** Its colour (linear). */
  @property
  get color(): Color {
    return this._color;
  }
  set color(c: Color) {
    this._color = c;
    this.send();
  }
  /** How far in from its faces it fades in, in metres (0: hard edges). */
  @property
  get blendDistance(): number {
    return this._blend;
  }
  set blendDistance(b: number) {
    this._blend = Math.max(0, b);
    this.send();
  }
  /** `box`, or `ellipsoid`: a soft round blob inside the box (mist over a pond). */
  @property
  get shape(): "box" | "ellipsoid" {
    return this._shape;
  }
  set shape(s: "box" | "ellipsoid") {
    this._shape = s;
    this.send();
  }

  private send(): void {
    if (!this.started) return;
    const s = this._size;
    const c = this._color;
    __platypus_op("render.fogVolume", this.id, this.gameObject.id, [s.x, s.y, s.z], this._density, [c.r, c.g, c.b], this._blend, this._shape);
  }

  override onAwake(): void {
    this.started = true;
    this.send();
  }

  override onDestroy(): void {
    __platypus_op("render.remove", this.id);
  }
}

/** One level of a `LODGroup`: its renderers, shown down to this screen height. */
export interface LOD {
  /** The fraction of the view's height (0 to 1) below which the next level takes over. */
  screenRelativeTransitionHeight: number;
  renderers: MeshRenderer[];
}

/**
 * Levels of detail (Unity's `LODGroup`): each camera shows one level's renderers, the
 * most detailed first, by how tall the group looks on its screen (`size` metres,
 * times the object's scale, `QualitySettings.lodBias` scaling it); below the last
 * level's height nothing is drawn.
 */
export class LODGroup extends Component {
  private _lods: LOD[] = [];
  private _size = 1;
  private _forced = -1;
  private started = false;

  /** The group's size in metres before its object's scale (its height, say). */
  @property
  get size(): number {
    return this._size;
  }
  set size(s: number) {
    this._size = s;
    this.send();
  }

  get lodCount(): number {
    return this._lods.length;
  }

  /** The levels, as `getLODs` / `setLODs` (saved with the scene). */
  @property
  get lods(): LOD[] {
    return this.getLODs();
  }
  set lods(lods: LOD[]) {
    this.setLODs(lods);
  }

  /** The levels (copies). */
  getLODs(): LOD[] {
    return this._lods.map((l) => ({ ...l, renderers: [...l.renderers] }));
  }

  /** Sets the levels, most detailed first; transition heights must go down. */
  setLODs(lods: LOD[]): void {
    for (let i = 1; i < lods.length; i++) {
      if (lods[i]!.screenRelativeTransitionHeight > lods[i - 1]!.screenRelativeTransitionHeight) {
        throw new Error("LOD transition heights must go down from the first level");
      }
    }
    this._lods = lods.map((l) => ({ ...l, renderers: [...l.renderers] }));
    this.send();
  }

  /** Always shows this level (-1: back to choosing by distance). */
  forceLOD(index: number): void {
    this._forced = index;
    this.send();
  }

  private send(): void {
    if (!this.started) return;
    __platypus_op(
      "render.lodGroup",
      this.id,
      this.gameObject.id,
      this._size,
      this._lods.map((l) => l.screenRelativeTransitionHeight),
      this._lods.map((l) => l.renderers.map((r) => r.id)),
      this._forced,
    );
  }

  override onAwake(): void {
    this.started = true;
    this.send();
  }

  override onDestroy(): void {
    __platypus_op("render.remove", this.id);
  }
}

