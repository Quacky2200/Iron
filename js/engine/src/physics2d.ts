// 2D physics the Unity way: `Rigidbody2D` and `BoxCollider2D` / `CircleCollider2D` /
// `CapsuleCollider2D` / `PolygonCollider2D` / `EdgeCollider2D`, with Vector2 APIs,
// angles in degrees about z, and `onCollisionEnter2D` / `onTriggerEnter2D` callbacks.
//
// They run in the same physics world as 3D: a Rigidbody2D's body can only move in x
// and y and turn about z, and 2D colliders are their outlines extruded into slabs
// `Physics2D.depth` deep, centred on the object, so 2D objects collide whatever their z
// (as in Unity, where z is only for drawing order).

import { Collider, type ObjectPhysics, _objectPhysics, _physicsFor, _releasePhysics } from "./physics-components.ts";
import { Quaternion, Vector2, type Vector2Like, Vector3 } from "./math.ts";
import { type PhysicsCollider, physics, type Shape } from "./physics.ts";
import { Component, type GameObject } from "./scene.ts";

/** How `Rigidbody2D.addForce` treats its vector (Unity's `ForceMode2D`). */
export enum ForceMode2D {
  /** A force in newtons over one fixed step (call it from `onFixedUpdate`). */
  Force = 0,
  /** An instant change in momentum: jumps, hits. */
  Impulse = 1,
}

let gravity2D = new Vector2(0, -9.81);

export const Physics2D = {
  /** How deep 2D colliders are along z (metres). */
  depth: 10,

  /** World gravity, shared with 3D physics (its z stays 0). */
  get gravity(): Vector2 {
    return gravity2D;
  },
  set gravity(g: Vector2Like) {
    gravity2D = new Vector2(g.x, g.y);
    physics.setGravity({ x: g.x, y: g.y, z: 0 });
  },

  /**
   * Casts a ray in the XY plane from `origin` along `direction` (after the next physics
   * tick) and resolves with the first collider hit within `distance`, or `null`.
   */
  async raycast(origin: Vector2Like, direction: Vector2Like, distance = 1000): Promise<RaycastHit2D | null> {
    const hit = await physics.raycast({ x: origin.x, y: origin.y, z: 0 }, { x: direction.x, y: direction.y, z: 0 }, distance);
    if (!hit) return null;
    return {
      collider: colliderFor(hit.collider),
      point: new Vector2(hit.point.x, hit.point.y),
      normal: new Vector2(hit.normal.x, hit.normal.y),
      distance: hit.distance,
    };
  },
};

/** What `Physics2D.raycast` hit (Unity's `RaycastHit2D`). */
export interface RaycastHit2D {
  /** The collider component, or null for colliders made without components. */
  collider: Collider | null;
  point: Vector2;
  normal: Vector2;
  distance: number;
}

const colliders2D = new Map<number, Collider2D>();

function colliderFor(c: PhysicsCollider): Collider | null {
  return colliders2D.get(c.id) ?? null;
}

/** What `onCollisionEnter2D` / `onCollisionExit2D` get (Unity's `Collision2D`). */
export interface Collision2D {
  readonly gameObject: GameObject;
  readonly collider: Collider;
  readonly rigidbody: Rigidbody2D | null;
  /** How fast they met (m/s); 0 on exit. */
  readonly relativeSpeed: number;
}

/** A 2D collider: an outline in the object's XY plane. */
export abstract class Collider2D extends Collider {
  /** The outline's offset from the object (Unity's `offset`). */
  get offset(): Vector2 {
    return new Vector2(this.center.x, this.center.y);
  }
  set offset(o: Vector2Like) {
    this.center = new Vector3(o.x, o.y, 0);
  }

  override get _is2D(): boolean {
    return true;
  }

  /** The Rigidbody2D moving it, if any. */
  get attachedRigidbody2D(): Rigidbody2D | null {
    const rb = _objectPhysics(this.gameObject)?.rigidbody;
    return rb instanceof Rigidbody2D ? rb : null;
  }

  /** The slab's half depth: `Physics2D.depth` deep at any z scale (sprites are often flat). */
  protected halfDepth(): number {
    return Physics2D.depth / 2;
  }

  /** An outline (counter-clockwise or not) as a convex slab. */
  protected slab(points: Vector2Like[], scale: Vector3): Shape {
    const d = this.halfDepth();
    const pts: Vector3[] = [];
    for (const p of points) {
      pts.push(new Vector3(p.x * scale.x, p.y * scale.y, -d), new Vector3(p.x * scale.x, p.y * scale.y, d));
    }
    return { convex: pts };
  }

  protected override create(): void {
    // A rebuild replaces the handle.
    if (this.handle) colliders2D.delete(this.handle.id);
    super.create();
    if (this.handle) colliders2D.set(this.handle.id, this);
  }

  override onDestroy(): void {
    if (this.handle) colliders2D.delete(this.handle.id);
    super.onDestroy();
  }
}

/** A rectangle `size` across. */
export class BoxCollider2D extends Collider2D {
  private _size = Vector2.one;
  get size(): Vector2 {
    return this._size;
  }
  set size(s: Vector2Like) {
    this._size = new Vector2(s.x, s.y);
    this.rebuild();
  }
  protected shape(scale: Vector3): Shape {
    return {
      box: {
        x: Math.abs((this._size.x * scale.x) / 2),
        y: Math.abs((this._size.y * scale.y) / 2),
        z: this.halfDepth(),
      },
    };
  }
}

/** A circle of `radius` (scaled by the object's larger of x and y). */
export class CircleCollider2D extends Collider2D {
  private _radius = 0.5;
  get radius(): number {
    return this._radius;
  }
  set radius(r: number) {
    this._radius = r;
    this.rebuild();
  }
  protected shape(scale: Vector3): Shape {
    return {
      cylinder: {
        halfHeight: this.halfDepth(),
        radius: this._radius * Math.max(Math.abs(scale.x), Math.abs(scale.y)),
      },
    };
  }
  /** Cylinders stand along y; this one's axis runs along z. */
  protected override shapeRotation(): Quaternion {
    return Quaternion.euler(90, 0, 0);
  }
}

/** A capsule outline `size` across, rounded along its `direction`. */
export class CapsuleCollider2D extends Collider2D {
  private _size = new Vector2(1, 2);
  private _direction: "vertical" | "horizontal" = "vertical";
  get size(): Vector2 {
    return this._size;
  }
  set size(s: Vector2Like) {
    this._size = new Vector2(s.x, s.y);
    this.rebuild();
  }
  get direction(): "vertical" | "horizontal" {
    return this._direction;
  }
  set direction(d: "vertical" | "horizontal") {
    this._direction = d;
    this.rebuild();
  }
  protected shape(scale: Vector3): Shape {
    const vertical = this._direction === "vertical";
    const [w, h] = [this._size.x, this._size.y];
    const r = (vertical ? w : h) / 2;
    const half = Math.max(0, (vertical ? h : w) / 2 - r);
    // Two half circles joined: 8 segments each.
    const pts: Vector2[] = [];
    for (let end = 0; end < 2; end++) {
      for (let i = 0; i <= 8; i++) {
        const a = Math.PI * (i / 8) + (end ? Math.PI : 0);
        const along = end ? -half : half;
        pts.push(
          vertical
            ? new Vector2(Math.cos(a) * r, along + Math.sin(a) * r)
            : new Vector2(along + Math.sin(a) * r, -Math.cos(a) * r),
        );
      }
    }
    return this.slab(pts, scale);
  }
}

/**
 * A polygon outline: `points` in the object's space. Concave outlines are filled in to
 * their convex hull.
 */
export class PolygonCollider2D extends Collider2D {
  private _points: Vector2[] = [new Vector2(-0.5, -0.5), new Vector2(0.5, -0.5), new Vector2(0, 0.5)];
  get points(): Vector2[] {
    return this._points;
  }
  set points(p: Vector2Like[]) {
    this._points = p.map((v) => new Vector2(v.x, v.y));
    this.rebuild();
  }
  protected shape(scale: Vector3): Shape {
    return this.slab(this._points, scale);
  }
}

/**
 * A line through `points` (ground, walls, slopes). Lines don't enclose anything, so
 * they're for static scenery and kinematic platforms, not falling bodies.
 */
export class EdgeCollider2D extends Collider2D {
  private _points: Vector2[] = [new Vector2(-0.5, 0), new Vector2(0.5, 0)];
  get points(): Vector2[] {
    return this._points;
  }
  set points(p: Vector2Like[]) {
    this._points = p.map((v) => new Vector2(v.x, v.y));
    this.rebuild();
  }
  protected shape(scale: Vector3): Shape {
    const d = this.halfDepth();
    const pts: Vector3[] = [];
    const indices: number[] = [];
    this._points.forEach((p, i) => {
      pts.push(new Vector3(p.x * scale.x, p.y * scale.y, -d), new Vector3(p.x * scale.x, p.y * scale.y, d));
      if (i > 0) {
        const [a, b, c, e] = [(i - 1) * 2, (i - 1) * 2 + 1, i * 2, i * 2 + 1];
        indices.push(a, c, b, b, c, e);
      }
    });
    return { mesh: { points: pts, indices } };
  }
}

export type RigidbodyType2D = "dynamic" | "kinematic" | "static";

/**
 * Makes the GameObject simulated in 2D (Unity's `Rigidbody2D`): it moves in x and y and
 * turns about z. Its 2D colliders give it shape.
 */
export class Rigidbody2D extends Component {
  private _mass = 1;
  private _drag = 0;
  private _angularDrag = 0.05;
  private _gravityScale = 1;
  private _bodyType: RigidbodyType2D = "dynamic";
  private _freezeRotation = false;
  private _continuous = false;
  private p: ObjectPhysics | null = null;

  /** In kg. */
  get mass(): number {
    return this._mass;
  }
  set mass(kg: number) {
    this._mass = Math.max(1e-6, kg);
    this._sendMass();
  }

  /** Slows movement (linear damping). */
  get drag(): number {
    return this._drag;
  }
  set drag(d: number) {
    this._drag = d;
    this.p?.body.setDamping(this._drag, this._angularDrag);
  }

  get angularDrag(): number {
    return this._angularDrag;
  }
  set angularDrag(d: number) {
    this._angularDrag = d;
    this.p?.body.setDamping(this._drag, this._angularDrag);
  }

  /** Multiplies gravity (0: floats). */
  get gravityScale(): number {
    return this._gravityScale;
  }
  set gravityScale(s: number) {
    this._gravityScale = s;
    this.p?.body.setGravityScale(s);
  }

  /** `dynamic` (simulated), `kinematic` (moved by scripts, pushes others) or `static`. */
  get bodyType(): RigidbodyType2D {
    return this._bodyType;
  }
  set bodyType(t: RigidbodyType2D) {
    this._bodyType = t;
    this.p?.body.setKind(t === "static" ? "fixed" : t);
  }

  get isKinematic(): boolean {
    return this._bodyType === "kinematic";
  }
  set isKinematic(on: boolean) {
    this.bodyType = on ? "kinematic" : "dynamic";
  }

  /** Keeps it upright: no turning from hits. */
  get freezeRotation(): boolean {
    return this._freezeRotation;
  }
  set freezeRotation(on: boolean) {
    this._freezeRotation = on;
    this.sendAxes();
  }

  get collisionDetectionMode(): "discrete" | "continuous" {
    return this._continuous ? "continuous" : "discrete";
  }
  set collisionDetectionMode(m: "discrete" | "continuous") {
    this._continuous = m === "continuous";
    this.p?.body.setCcd(this._continuous);
  }

  /** Velocity in m/s (as of the last physics step). */
  get velocity(): Vector2 {
    const v = this.p?.body.velocity;
    return v ? new Vector2(v.x, v.y) : Vector2.zero;
  }
  set velocity(v: Vector2Like) {
    this.p?.body.setVelocity({ x: v.x, y: v.y, z: 0 });
  }

  /** Spin in degrees per second, anticlockwise (as of the last physics step). */
  get angularVelocity(): number {
    return ((this.p?.body.angularVelocity.z ?? 0) * 180) / Math.PI;
  }
  set angularVelocity(degrees: number) {
    this.p?.body.setAngularVelocity({ x: 0, y: 0, z: (degrees * Math.PI) / 180 });
  }

  /** Where it is in the XY plane. */
  get position(): Vector2 {
    const p = this.transform.position;
    return new Vector2(p.x, p.y);
  }
  set position(v: Vector2Like) {
    this.transform.position = new Vector3(v.x, v.y, this.transform.position.z);
  }

  /** Its angle about z in degrees, anticlockwise. */
  get rotation(): number {
    return this.transform.eulerAngles.z;
  }
  set rotation(degrees: number) {
    this.transform.rotation = Quaternion.euler(0, 0, degrees);
  }

  addForce(force: Vector2Like, mode: ForceMode2D = ForceMode2D.Force): void {
    const k = mode === ForceMode2D.Force ? 1 / 60 : 1;
    this.p?.body.applyImpulse({ x: force.x * k, y: force.y * k, z: 0 });
  }

  /** Pushes at a world point, so it also spins. */
  addForceAtPosition(force: Vector2Like, position: Vector2Like, mode: ForceMode2D = ForceMode2D.Force): void {
    const k = mode === ForceMode2D.Force ? 1 / 60 : 1;
    this.p?.body.applyImpulseAtPoint({ x: force.x * k, y: force.y * k, z: 0 }, { x: position.x, y: position.y, z: this.transform.position.z });
  }

  /** Spins it (N·m anticlockwise). */
  addTorque(torque: number, mode: ForceMode2D = ForceMode2D.Force): void {
    const k = mode === ForceMode2D.Force ? 1 / 60 : 1;
    this.p?.body.applyTorqueImpulse({ x: 0, y: 0, z: torque * k });
  }

  /** Moves a kinematic body to `position` over the next step, pushing things. */
  movePosition(position: Vector2Like): void {
    this.position = position;
  }

  moveRotation(degrees: number): void {
    this.rotation = degrees;
  }

  sleep(): void {
    this.p?.body.sleep();
  }

  wakeUp(): void {
    this.p?.body.wakeUp();
  }

  /** @internal */
  _sendMass(): void {
    this.p?.body.setMass(this._mass);
  }

  private sendAxes(): void {
    this.p?.body.setAllowedAxes([true, true, false], [false, false, !this._freezeRotation]);
  }

  override onAwake(): void {
    const p = _physicsFor(this.gameObject);
    p.rigidbody = this;
    this.p = p;
    p.body.setKind(this._bodyType === "static" ? "fixed" : this._bodyType);
    this._sendMass();
    p.body.setDamping(this._drag, this._angularDrag);
    p.body.setGravityScale(this._gravityScale);
    if (this._continuous) p.body.setCcd(true);
    this.sendAxes();
  }

  override onDestroy(): void {
    const p = this.p;
    if (!p) return;
    p.rigidbody = null;
    this.p = null;
    p.body.setKind("fixed");
    _releasePhysics(this.gameObject);
  }
}

/** Clears the 2D bookkeeping. Only for tests. */
export function _resetPhysics2DForTests() {
  colliders2D.clear();
  gravity2D = new Vector2(0, -9.81);
}
