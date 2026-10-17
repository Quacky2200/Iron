// The engine's rendering components (M5.a): cameras and planes, plus camera helpers.
// Like Unity's, they're added to GameObjects (`go.addComponent(Camera)`) and follow
// the object's transform. Each sends its settings to the engine as `render.*` ops when
// it starts and whenever a setting changes.

import { property } from "./property.ts";
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
import { Component, type GameObject } from "./scene.ts";
import { Screen } from "./screen.ts";
import type { Texture } from "./textures.ts";

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
  private started = false;

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

  /** The camera's area of the screen in UI pixels. */
  get pixelRect(): Rect {
    return new Rect(this._rect.x * Screen.width, this._rect.y * Screen.height, this._rect.width * Screen.width, this._rect.height * Screen.height);
  }

  get aspect(): number {
    const r = this.pixelRect;
    return r.width / Math.max(1e-6, r.height);
  }

  /** The main camera: the first active one added (like Unity's tagged `Camera.main`). */
  static get main(): Camera | undefined {
    return cameras.find((c) => c.gameObject.activeInHierarchy && !c.gameObject.destroyed);
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
    __iron_op(
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
    );
  }

  override onStart(): void {
    this.started = true;
    cameras.push(this);
    this.send();
  }

  override onDestroy(): void {
    cameras.splice(cameras.indexOf(this), 1);
    __iron_op("render.remove", this.id);
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
    __iron_op("render.plane", this.id, this.gameObject.id, this._size.toArray(), rgba(this._color));
  }

  override onStart(): void {
    this.started = true;
    this.send();
  }

  override onDestroy(): void {
    __iron_op("render.remove", this.id);
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
 * it with `createRoot(panel)` from `@iron/react`. The panel stands in its object's XY
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
    __iron_op(
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
    __iron_op("render.remove", this.id);
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
export class Mesh {
  readonly id: number;
  /** @internal Its data as flat arrays (script meshes), for saving with scenes. */
  _data: { vertices: number[]; triangles: number[]; normals: number[]; uv: number[] } | null = null;
  private constructor(id: number) {
    this.id = id;
  }

  /** @internal A mesh the engine made (a model's), by its id. */
  static _engine(id: number): Mesh {
    return new Mesh(id);
  }

  static create(data: {
    vertices: readonly Vector3Like[];
    triangles: readonly number[];
    normals?: readonly Vector3Like[];
    uv?: readonly Vector2Like[];
  }): Mesh {
    const mesh = new Mesh(nextMesh++);
    mesh.set(data);
    return mesh;
  }

  /** Replaces the mesh's data (everything drawing it updates). */
  set(data: {
    vertices: readonly Vector3Like[];
    triangles: readonly number[];
    normals?: readonly Vector3Like[];
    uv?: readonly Vector2Like[];
  }): void {
    this._data = {
      vertices: data.vertices.flatMap((v) => [v.x, v.y, v.z]),
      triangles: [...data.triangles],
      normals: (data.normals ?? []).flatMap((v) => [v.x, v.y, v.z]),
      uv: (data.uv ?? []).flatMap((v) => [v.x, v.y]),
    };
    __iron_op(
      "render.mesh",
      this.id,
      data.vertices.flatMap((v) => [v.x, v.y, v.z]),
      (data.normals ?? []).flatMap((v) => [v.x, v.y, v.z]),
      // Unity's UVs run bottom-up; the engine's top-down.
      (data.uv ?? []).flatMap((v) => [v.x, 1 - v.y]),
      [...data.triangles],
    );
  }

  /** Frees it (renderers still using it draw nothing). */
  destroy(): void {
    __iron_op("render.mesh", this.id, null);
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

  private send(): void {
    if (!this.started) return;
    const m = this._material;
    __iron_op(
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
    );
  }

  override onAwake(): void {
    this.started = true;
    this.send();
  }

  override onDestroy(): void {
    __iron_op("render.remove", this.id);
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

  private send(): void {
    if (!this.started) return;
    const c = this._color;
    __iron_op(
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
    );
  }

  override onAwake(): void {
    this.started = true;
    this.send();
  }

  override onDestroy(): void {
    __iron_op("render.remove", this.id);
  }
}

let ambient = new Color(0.2, 0.2, 0.22);

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

let skybox: ProceduralSky | null = null;
let ambientMode: "skybox" | "color" = "skybox";
let reflectionIntensity = 1;

function sendSky(): void {
  if (!skybox) {
    __iron_op("render.sky", 0, null);
    return;
  }
  const s = skybox;
  const rgb = (c: Color | undefined) => (c ? [c.r, c.g, c.b] : null);
  const sun = s.sunDirection === null ? null : s.sunDirection ? [s.sunDirection.x, s.sunDirection.y, s.sunDirection.z] : [0.4, 0.6, 0.3];
  __iron_op(
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
    __iron_op("render.ambient", 0, [c.r, c.g, c.b]);
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
    if (b) __iron_op("render.bloom", 0, b.threshold, b.intensity, b.scatter);
    else __iron_op("render.bloom", 0, null);
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
    __iron_op("render.mirror", this.id, this.gameObject.id, [n.x, n.y, n.z], this.cullingMask >>> 0);
  }

  override onAwake(): void {
    this.send();
  }

  override onDestroy(): void {
    __iron_op("render.remove", this.id);
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
    __iron_op("render.probe", this.id, this.gameObject.id, [s.x, s.y, s.z], this.resolution, this.boxProjection, this.refreshMode === "everyFrame", this.cullingMask >>> 0);
  }

  /** Captures the scene again on the next frame (after moving things, say). */
  renderProbe(): void {
    __iron_op("render.probeRender", this.id);
  }

  override onAwake(): void {
    this.send();
  }

  override onDestroy(): void {
    __iron_op("render.remove", this.id);
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
    __iron_op(
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
    __iron_op("render.remove", this.id);
  }
}

