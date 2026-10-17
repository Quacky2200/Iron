// The engine's rendering components: `Camera` and `PlaneRenderer` (M5.a). Like
// Unity's, they're added to GameObjects (`go.addComponent(Camera)`) and follow the
// object's transform. Each sends its settings to the engine as `render.*` ops when it
// starts and whenever a setting changes.

import { Color, Vector2 } from "./math.ts";
import { Component } from "./scene.ts";

const rgba = (c: Color) => [c.r, c.g, c.b, c.a];

/**
 * What the player sees: a perspective view from the object, looking down its forward
 * (-z) axis. The first active camera is the main one; with none, only screen UI is
 * drawn.
 */
export class Camera extends Component {
  private _fov = 60;
  private _near = 0.1;
  private _far = 1000;
  private _background = new Color(0.05, 0.06, 0.09, 1);
  private started = false;

  /** Vertical field of view in degrees. */
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

  /** The colour behind everything (linear RGBA). */
  get backgroundColor(): Color {
    return this._background;
  }
  set backgroundColor(c: Color) {
    this._background = c;
    this.send();
  }

  /** The active camera added first, like Unity's `Camera.main`. */
  static get main(): Camera | undefined {
    return cameras.find((c) => c.gameObject.activeInHierarchy && !c.gameObject.destroyed);
  }

  private send(): void {
    if (!this.started) return;
    __iron_op("render.camera", this.id, this.gameObject.id, this._fov, this._near, this._far, rgba(this._background));
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

const cameras: Camera[] = [];

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
