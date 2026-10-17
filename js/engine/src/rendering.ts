// The engine's rendering components (M5.a): cameras and planes, plus camera helpers.
// Like Unity's, they're added to GameObjects (`go.addComponent(Camera)`) and follow
// the object's transform. Each sends its settings to the engine as `render.*` ops when
// it starts and whenever a setting changes.

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
  private started = false;

  /** Vertical field of view in degrees (perspective). */
  get fieldOfView(): number {
    return this._fov;
  }
  set fieldOfView(v: number) {
    this._fov = v;
    this.send();
  }

  get nearClipPlane(): number {
    return this._near;
  }
  set nearClipPlane(v: number) {
    this._near = v;
    this.send();
  }

  get farClipPlane(): number {
    return this._far;
  }
  set farClipPlane(v: number) {
    this._far = v;
    this.send();
  }

  /** The colour behind everything in this camera's view (linear RGBA). */
  get backgroundColor(): Color {
    return this._background;
  }
  set backgroundColor(c: Color) {
    this._background = c;
    this.send();
  }

  /** Draw order: cameras with a higher depth draw over lower ones. */
  get depth(): number {
    return this._depth;
  }
  set depth(d: number) {
    this._depth = d;
    this.send();
  }

  /** An orthographic (2D, no perspective) view instead of a perspective one. */
  get orthographic(): boolean {
    return this._orthographic;
  }
  set orthographic(on: boolean) {
    this._orthographic = on;
    this.send();
  }

  /** Half the view's height in world units when orthographic (Unity's `orthographicSize`). */
  get orthographicSize(): number {
    return this._orthographicSize;
  }
  set orthographicSize(s: number) {
    this._orthographicSize = s;
    this.send();
  }

  /** The part of the screen it draws into: x, y, width, height from the top-left, 0 to 1. */
  get rect(): Rect {
    return this._rect;
  }
  set rect(r: Rect) {
    this._rect = r;
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

  get size(): Vector2 {
    return this._size;
  }
  set size(v: Vector2) {
    this._size = Vector2.from(v);
    this.send();
  }

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
  target: GameObject | null = null;
  offset = new Vector3(0, 3, 6);
  /** Rotate the offset with the target (behind a car), or keep it in world space. */
  offsetInTargetSpace = false;
  /** About how long catching up takes, in seconds (0: instantly). */
  smoothTime = 0.25;
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
  target: GameObject | null = null;
  /** Orbits this point when there's no target. */
  center = Vector3.zero;
  distance = 8;
  yaw = 0;
  pitch = 20;
  minPitch = -80;
  maxPitch = 80;
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
  get width(): number {
    return this._width;
  }
  set width(v: number) {
    this._width = v;
    this.send();
  }

  /** Height in UI pixels. */
  get height(): number {
    return this._height;
  }
  set height(v: number) {
    this._height = v;
    this.send();
  }

  /** UI pixels per metre: 500 makes an 800×600 panel 1.6 × 1.2 m. */
  get pixelsPerMeter(): number {
    return this._pixelsPerMeter;
  }
  set pixelsPerMeter(v: number) {
    this._pixelsPerMeter = v;
    this.send();
  }

  /** Texture pixels per UI pixel: higher keeps text sharp up close (costs memory). */
  get resolution(): number {
    return this._resolution;
  }
  set resolution(v: number) {
    this._resolution = v;
    this.send();
  }

  /** A colour multiplied over the panel (alpha fades it). */
  get tint(): Color {
    return this._tint;
  }
  set tint(c: Color) {
    this._tint = c;
    this.send();
  }

  /** Whether the pointer can use it. */
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
