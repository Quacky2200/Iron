// Physics as components on GameObjects, the Unity way: a `Rigidbody` makes an object
// simulated; `BoxCollider`, `SphereCollider`, `CapsuleCollider` and `MeshCollider`
// give it shape (an object with colliders and no Rigidbody is static). The engine moves
// the GameObject with its body each fixed tick, and moving the GameObject from a script
// teleports the body (or, for a kinematic one, carries it there, pushing things).
//
// Components on the object get Unity's callbacks: `onCollisionEnter(collision)` /
// `onCollisionExit`, and `onTriggerEnter(other)` / `onTriggerExit` for triggers.
//
// Built on the lower-level `physics` handles (physics.ts). Collider sizes take the
// object's scale when the collider is added; changing the scale later doesn't resize it.

import { Time } from "./clock.ts";
import { Color, Mathf, Quaternion, Vector3, type Vector3Like } from "./math.ts";
import { contactListeners, type PhysicsBody, type PhysicsCollider, type PhysicsJoint, physics, type Shape } from "./physics.ts";
import { Material, MeshRenderer } from "./rendering.ts";
import { Component, GameObject } from "./scene.ts";

/** How `Rigidbody.addForce` treats its vector (Unity's `ForceMode`). */
export enum ForceMode {
  /** A force in newtons, over one fixed step (call it from `onFixedUpdate`). */
  Force = 0,
  /** An acceleration in m/s², over one fixed step, whatever the mass. */
  Acceleration = 5,
  /** An instant change in momentum (N·s): hits, jumps. */
  Impulse = 1,
  /** An instant change in velocity (m/s), whatever the mass. */
  VelocityChange = 2,
}

/** Axes a Rigidbody can't move along or turn about (Unity's flags; combine with `|`). */
export const RigidbodyConstraints = {
  None: 0,
  FreezePositionX: 2,
  FreezePositionY: 4,
  FreezePositionZ: 8,
  FreezeRotationX: 16,
  FreezeRotationY: 32,
  FreezeRotationZ: 64,
  FreezePosition: 14,
  FreezeRotation: 112,
  FreezeAll: 126,
} as const;

/** Surface properties of a collider (Unity's `PhysicsMaterial`). */
export class PhysicsMaterial {
  constructor(
    /** 0 is ice; 1 grips. Default 0.6. */
    readonly dynamicFriction = 0.6,
    /** 0 doesn't bounce; 1 bounces back as fast. */
    readonly bounciness = 0,
  ) {}
}

/** What `onCollisionEnter` / `onCollisionExit` get: the other side of the contact. */
export interface Collision {
  readonly gameObject: GameObject;
  readonly collider: Collider;
  readonly rigidbody: Rigidbody | null;
  /** How fast the two were moving relative to each other going in (m/s); 0 on exit. */
  readonly relativeSpeed: number;
}

/** What simulates an object's body: a `Rigidbody` or a `Rigidbody2D`. */
export interface BodyOwner {
  /** Sends its mass again (colliders changed, which share it out). */
  _sendMass(): void;
}

/** One physics body per GameObject, shared by its Rigidbody and colliders. */
export interface ObjectPhysics {
  body: PhysicsBody;
  rigidbody: BodyOwner | null;
  colliders: Set<Collider>;
}

const objectPhysics = new Map<GameObject, ObjectPhysics>();
const colliderOwners = new Map<number, Collider>();

/** @internal The object's physics, if it has any (physics2d.ts). */
export function _objectPhysics(go: GameObject): ObjectPhysics | undefined {
  return objectPhysics.get(go);
}

/** @internal The object's body, made static at the object's pose the first time. */
export function _physicsFor(go: GameObject): ObjectPhysics {
  return physicsFor(go);
}

/** @internal Drops the body once nothing on the object needs it. */
export function _releasePhysics(go: GameObject): void {
  releasePhysics(go);
}

/** The object's body, made static at the object's pose the first time. */
function physicsFor(go: GameObject): ObjectPhysics {
  let p = objectPhysics.get(go);
  if (!p) {
    const t = go.transform;
    const body = physics.createBody({ kind: "fixed", position: t.position, rotation: t.rotation });
    body.attachTo(go.id);
    p = { body, rigidbody: null, colliders: new Set() };
    objectPhysics.set(go, p);
  }
  return p;
}

/** Drops the body once nothing on the object needs it. */
function releasePhysics(go: GameObject) {
  const p = objectPhysics.get(go);
  if (!p || p.rigidbody || p.colliders.size > 0) return;
  p.body.remove();
  objectPhysics.delete(go);
}

/** A collider of any shape: Unity's `Collider`. */
export abstract class Collider extends Component {
  private _center = Vector3.zero;
  private _isTrigger = false;
  private _material = new PhysicsMaterial();
  /** The underlying physics collider (after the component is added). */
  handle: PhysicsCollider | null = null;

  /** The shape's offset from the object, in its local space. */
  get center(): Vector3 {
    return this._center;
  }
  set center(c: Vector3Like) {
    this._center = Vector3.from(c);
    this.handle?.setOffset(this._center.mul(this.gameObject.transform.lossyScale));
  }

  /** A trigger reports overlaps (`onTriggerEnter`) instead of colliding. */
  get isTrigger(): boolean {
    return this._isTrigger;
  }
  set isTrigger(on: boolean) {
    this._isTrigger = on;
    this.handle?.setSensor(on);
  }

  get material(): PhysicsMaterial {
    return this._material;
  }
  set material(m: PhysicsMaterial) {
    this._material = m;
    this.handle?.setFriction(m.dynamicFriction);
    this.handle?.setRestitution(m.bounciness);
  }

  /** The Rigidbody moving this collider, if any. */
  get attachedRigidbody(): Rigidbody | null {
    const rb = objectPhysics.get(this.gameObject)?.rigidbody;
    return rb instanceof Rigidbody ? rb : null;
  }

  /** @internal 2D colliders get the `...2D` callbacks. */
  get _is2D(): boolean {
    return false;
  }

  /** The physics shape at the object's current scale. */
  protected abstract shape(scale: Vector3): Shape;

  /** How the shape is turned on the object (2D circles lie in the XY plane). */
  protected shapeRotation(): Quaternion | undefined {
    return undefined;
  }

  /** Rebuilds the physics collider (after a size change). */
  protected rebuild(): void {
    if (!this.handle) return;
    this.handle.remove();
    colliderOwners.delete(this.handle.id);
    this.create();
  }

  protected create(): void {
    const p = physicsFor(this.gameObject);
    const scale = this.gameObject.transform.lossyScale;
    this.handle = p.body.addCollider({
      shape: this.shape(scale),
      position: this._center.mul(scale),
      rotation: this.shapeRotation(),
      sensor: this._isTrigger,
      friction: this._material.dynamicFriction,
      restitution: this._material.bounciness,
    });
    colliderOwners.set(this.handle.id, this);
    p.colliders.add(this);
    // The body's mass is shared by its colliders: share it again.
    p.rigidbody?._sendMass();
  }

  override onAwake(): void {
    this.create();
  }

  override onDestroy(): void {
    const p = objectPhysics.get(this.gameObject);
    p?.colliders.delete(this);
    if (this.handle) {
      colliderOwners.delete(this.handle.id);
      this.handle.remove();
      this.handle = null;
    }
    p?.rigidbody?._sendMass();
    releasePhysics(this.gameObject);
  }
}

/** A box, `size` across (in the object's local units). */
export class BoxCollider extends Collider {
  private _size = Vector3.one;
  get size(): Vector3 {
    return this._size;
  }
  set size(s: Vector3Like) {
    this._size = Vector3.from(s);
    this.rebuild();
  }
  protected shape(scale: Vector3): Shape {
    const h = this._size.mul(scale).scale(0.5);
    return { box: { x: Math.abs(h.x), y: Math.abs(h.y), z: Math.abs(h.z) } };
  }
}

/** A ball of `radius` (scaled by the object's largest axis). */
export class SphereCollider extends Collider {
  private _radius = 0.5;
  get radius(): number {
    return this._radius;
  }
  set radius(r: number) {
    this._radius = r;
    this.rebuild();
  }
  protected shape(scale: Vector3): Shape {
    return { ball: this._radius * Math.max(Math.abs(scale.x), Math.abs(scale.y), Math.abs(scale.z)) };
  }
}

/** A capsule standing along y: `height` end to end (caps included), `radius` round. */
export class CapsuleCollider extends Collider {
  private _radius = 0.5;
  private _height = 2;
  get radius(): number {
    return this._radius;
  }
  set radius(r: number) {
    this._radius = r;
    this.rebuild();
  }
  get height(): number {
    return this._height;
  }
  set height(h: number) {
    this._height = h;
    this.rebuild();
  }
  protected shape(scale: Vector3): Shape {
    const radius = this._radius * Math.max(Math.abs(scale.x), Math.abs(scale.z));
    const halfHeight = Math.max(0, (this._height * Math.abs(scale.y)) / 2 - radius);
    return { capsule: { halfHeight, radius } };
  }
}

/**
 * A triangle mesh (static level geometry), or with `convex` its convex hull (which can
 * move with a Rigidbody). Set `points` and `indices` when adding it.
 */
export class MeshCollider extends Collider {
  points: Vector3Like[] = [];
  /** Three per triangle, into `points`. */
  indices: number[] = [];
  convex = false;
  protected shape(scale: Vector3): Shape {
    const pts = this.points.map((p) => Vector3.from(p).mul(scale));
    return this.convex ? { convex: pts } : { mesh: { points: pts, indices: this.indices } };
  }
  /** Rebuilds it from new `points` / `indices`. */
  setMesh(points: Vector3Like[], indices: number[], convex = this.convex): void {
    this.points = points;
    this.indices = indices;
    this.convex = convex;
    this.rebuild();
  }
}

/**
 * Makes the GameObject simulated: gravity, forces, collisions. Its colliders give it
 * shape (add at least one). Unity's `Rigidbody`.
 */
export class Rigidbody extends Component {
  private _mass = 1;
  private _drag = 0;
  private _angularDrag = 0.05;
  private _useGravity = true;
  private _isKinematic = false;
  private _constraints: number = RigidbodyConstraints.None;
  private _continuous = false;
  private p: ObjectPhysics | null = null;

  /** The underlying physics body. */
  get body(): PhysicsBody | null {
    return this.p?.body ?? null;
  }

  /** In kg. */
  get mass(): number {
    return this._mass;
  }
  set mass(kg: number) {
    this._mass = Math.max(1e-6, kg);
    this._sendMass();
  }

  /** Slows movement through the air (linear damping). */
  get drag(): number {
    return this._drag;
  }
  set drag(d: number) {
    this._drag = d;
    this.p?.body.setDamping(this._drag, this._angularDrag);
  }

  /** Slows spinning. */
  get angularDrag(): number {
    return this._angularDrag;
  }
  set angularDrag(d: number) {
    this._angularDrag = d;
    this.p?.body.setDamping(this._drag, this._angularDrag);
  }

  get useGravity(): boolean {
    return this._useGravity;
  }
  set useGravity(on: boolean) {
    this._useGravity = on;
    this.p?.body.setGravityScale(on ? 1 : 0);
  }

  /** Moved only by scripts (setting the transform), pushing simulated bodies out of the way. */
  get isKinematic(): boolean {
    return this._isKinematic;
  }
  set isKinematic(on: boolean) {
    this._isKinematic = on;
    this.p?.body.setKind(on ? "kinematic" : "dynamic");
  }

  /** `RigidbodyConstraints` flags, e.g. `FreezeRotation` for characters. */
  get constraints(): number {
    return this._constraints;
  }
  set constraints(flags: number) {
    this._constraints = flags;
    this.sendConstraints();
  }

  /** Continuous collision detection: fast things don't pass through thin walls. */
  get collisionDetection(): "discrete" | "continuous" {
    return this._continuous ? "continuous" : "discrete";
  }
  set collisionDetection(mode: "discrete" | "continuous") {
    this._continuous = mode === "continuous";
    this.p?.body.setCcd(this._continuous);
  }

  /** Velocity in m/s (as of the last physics step). */
  get velocity(): Vector3 {
    return this.p ? Vector3.from(this.p.body.velocity) : Vector3.zero;
  }
  set velocity(v: Vector3Like) {
    this.p?.body.setVelocity(v);
  }

  /** Spin in radians per second about each axis (as of the last physics step). */
  get angularVelocity(): Vector3 {
    return this.p ? Vector3.from(this.p.body.angularVelocity) : Vector3.zero;
  }
  set angularVelocity(v: Vector3Like) {
    this.p?.body.setAngularVelocity(v);
  }

  /** Pushes the body (Unity's `AddForce`): see `ForceMode`. */
  addForce(force: Vector3Like, mode: ForceMode = ForceMode.Force): void {
    this.p?.body.applyImpulse(this.impulse(force, mode));
  }

  /** Spins the body. `Acceleration` and `VelocityChange` are treated as `Force` and `Impulse`. */
  addTorque(torque: Vector3Like, mode: ForceMode = ForceMode.Force): void {
    const dt = 1 / 60;
    const t = Vector3.from(torque);
    const instant = mode === ForceMode.Impulse || mode === ForceMode.VelocityChange;
    this.p?.body.applyTorqueImpulse(instant ? t : t.scale(dt));
  }

  /** Pushes at a world point, so it also spins (Unity's `AddForceAtPosition`). */
  addForceAtPosition(force: Vector3Like, position: Vector3Like, mode: ForceMode = ForceMode.Force): void {
    this.p?.body.applyImpulseAtPoint(this.impulse(force, mode), position);
  }

  /** Upward push from an explosion at `position`, fading to nothing at `radius`. */
  addExplosionForce(force: number, position: Vector3Like, radius: number, upwards = 0, mode: ForceMode = ForceMode.Impulse): void {
    const from = Vector3.from(position).sub(new Vector3(0, upwards, 0));
    const offset = this.transform.position.sub(from);
    const distance = offset.magnitude;
    if (distance > radius) return;
    const falloff = 1 - Mathf.clamp01(distance / Math.max(radius, 1e-6));
    this.addForce(offset.normalized.scale(force * falloff), mode);
  }

  /** Moves a kinematic body to `position` over the next step (Unity's `MovePosition`). */
  movePosition(position: Vector3Like): void {
    this.transform.position = position;
  }

  moveRotation(rotation: { x: number; y: number; z: number; w: number }): void {
    this.transform.rotation = rotation;
  }

  sleep(): void {
    this.p?.body.sleep();
  }

  wakeUp(): void {
    this.p?.body.wakeUp();
  }

  private impulse(force: Vector3Like, mode: ForceMode): Vector3 {
    const f = Vector3.from(force);
    const dt = 1 / 60;
    switch (mode) {
      case ForceMode.Force:
        return f.scale(dt);
      case ForceMode.Acceleration:
        return f.scale(this._mass * dt);
      case ForceMode.Impulse:
        return f;
      case ForceMode.VelocityChange:
        return f.scale(this._mass);
    }
  }

  /** @internal */
  _sendMass(): void {
    this.p?.body.setMass(this._mass);
  }

  private sendConstraints(): void {
    const c = this._constraints;
    const free = (flag: number) => (c & flag) === 0;
    this.p?.body.setAllowedAxes(
      [free(RigidbodyConstraints.FreezePositionX), free(RigidbodyConstraints.FreezePositionY), free(RigidbodyConstraints.FreezePositionZ)],
      [free(RigidbodyConstraints.FreezeRotationX), free(RigidbodyConstraints.FreezeRotationY), free(RigidbodyConstraints.FreezeRotationZ)],
    );
  }

  override onAwake(): void {
    const p = physicsFor(this.gameObject);
    p.rigidbody = this;
    this.p = p;
    p.body.setKind(this._isKinematic ? "kinematic" : "dynamic");
    this._sendMass();
    p.body.setDamping(this._drag, this._angularDrag);
    p.body.setGravityScale(this._useGravity ? 1 : 0);
    if (this._continuous) p.body.setCcd(true);
    if (this._constraints) this.sendConstraints();
  }

  override onDestroy(): void {
    const p = this.p;
    if (!p) return;
    p.rigidbody = null;
    this.p = null;
    // The colliders stay, now static.
    p.body.setKind("fixed");
    releasePhysics(this.gameObject);
  }
}

type Hooks = {
  onCollisionEnter?(c: Collision): void;
  onCollisionExit?(c: Collision): void;
  onTriggerEnter?(other: Collider): void;
  onTriggerExit?(other: Collider): void;
  onCollisionEnter2D?(c: Collision2DLike): void;
  onCollisionExit2D?(c: Collision2DLike): void;
  onTriggerEnter2D?(other: Collider): void;
  onTriggerExit2D?(other: Collider): void;
};

/** What 2D callbacks get (physics2d.ts types it fully as `Collision2D`). */
interface Collision2DLike {
  readonly gameObject: GameObject;
  readonly collider: Collider;
  readonly rigidbody: BodyOwner | null;
  readonly relativeSpeed: number;
}

// Contacts become Unity-style callbacks on every component of both objects.
contactListeners.push((a, b, started, sensor, speed) => {
  const ca = colliderOwners.get(a.id);
  const cb = colliderOwners.get(b.id);
  if (!ca || !cb) return;
  for (const [self, other] of [
    [ca, cb],
    [cb, ca],
  ] as const) {
    const go = self.gameObject;
    if (go.destroyed) continue;
    for (const component of [...go.components] as (Component & Hooks)[]) {
      if (self._is2D) {
        // Unity keeps 2D and 3D callbacks apart.
        if (sensor) {
          (started ? component.onTriggerEnter2D : component.onTriggerExit2D)?.call(component, other);
        } else {
          const collision: Collision2DLike = {
            gameObject: other.gameObject,
            collider: other,
            rigidbody: objectPhysics.get(other.gameObject)?.rigidbody ?? null,
            relativeSpeed: speed,
          };
          (started ? component.onCollisionEnter2D : component.onCollisionExit2D)?.call(component, collision);
        }
      } else if (sensor) {
        (started ? component.onTriggerEnter : component.onTriggerExit)?.call(component, other);
      } else {
        const collision: Collision = {
          gameObject: other.gameObject,
          collider: other,
          rigidbody: other.attachedRigidbody,
          relativeSpeed: speed,
        };
        (started ? component.onCollisionEnter : component.onCollisionExit)?.call(component, collision);
      }
    }
  }
});

/** Clears the component bookkeeping. Only for tests. */
export function _resetPhysicsComponentsForTests() {
  objectPhysics.clear();
  colliderOwners.clear();
}

/** What a `CharacterController.move` touched (Unity's flags). */
export const CollisionFlags = { None: 0, Sides: 1, Above: 2, Below: 4 } as const;

/**
 * Moves a character that walks rather than tumbles (Unity's `CharacterController`): a
 * capsule that slides along walls, climbs slopes up to `slopeLimit` and steps up to
 * `stepOffset`, moved only by `move` / `simpleMove`. Physics doesn't push it, and it
 * pushes nothing; use a Rigidbody for that.
 */
export class CharacterController extends Collider {
  private _radius = 0.5;
  private _height = 2;
  private _slopeLimit = 45;
  private _stepOffset = 0.3;
  private fallSpeed = 0;
  private last = Vector3.zero;
  private _velocity = Vector3.zero;

  get radius(): number {
    return this._radius;
  }
  set radius(r: number) {
    this._radius = r;
    this.rebuild();
  }
  get height(): number {
    return this._height;
  }
  set height(h: number) {
    this._height = h;
    this.rebuild();
  }
  /** The steepest slope it walks up, in degrees. */
  get slopeLimit(): number {
    return this._slopeLimit;
  }
  set slopeLimit(d: number) {
    this._slopeLimit = d;
    this.configure();
  }
  /** The highest step it climbs, in metres. */
  get stepOffset(): number {
    return this._stepOffset;
  }
  set stepOffset(h: number) {
    this._stepOffset = h;
    this.configure();
  }

  /** Whether it touched the ground on its last move (as of the last physics step). */
  get isGrounded(): boolean {
    return objectPhysics.get(this.gameObject)?.body.grounded ?? false;
  }

  /** How fast it actually moved last frame (m/s). */
  get velocity(): Vector3 {
    return this._velocity;
  }

  protected shape(scale: Vector3): Shape {
    const radius = this._radius * Math.max(Math.abs(scale.x), Math.abs(scale.z));
    const halfHeight = Math.max(0, (this._height * Math.abs(scale.y)) / 2 - radius);
    return { capsule: { halfHeight, radius } };
  }

  private configure(): void {
    const body = objectPhysics.get(this.gameObject)?.body;
    body?.makeCharacter({ maxSlope: this._slopeLimit, snapToGround: this._stepOffset, stepHeight: this._stepOffset });
  }

  override onAwake(): void {
    this.create();
    objectPhysics.get(this.gameObject)!.body.setKind("kinematic");
    this.configure();
    this.last = this.transform.position;
  }

  override onUpdate(dt: number): void {
    const now = this.transform.position;
    this._velocity = dt > 0 ? now.sub(this.last).scale(1 / dt) : Vector3.zero;
    this.last = now;
  }

  /**
   * Moves by `motion` (metres, world space) on the next physics step, sliding along what
   * it hits. Returns `CollisionFlags.Below` if it was on the ground after its last move.
   */
  move(motion: Vector3Like): number {
    objectPhysics.get(this.gameObject)?.body.move(motion);
    return this.isGrounded ? CollisionFlags.Below : CollisionFlags.None;
  }

  /**
   * Walks at `speed` (m/s, its y ignored) with gravity, for this frame (Unity's
   * `SimpleMove`). Returns whether it's grounded.
   */
  simpleMove(speed: Vector3Like): boolean {
    const dt = Time.deltaTime;
    // A little downward pull on the ground keeps it grounded on slopes.
    this.fallSpeed = this.isGrounded ? 1 : this.fallSpeed + 9.81 * dt;
    this.move(new Vector3(speed.x * dt, -this.fallSpeed * dt, speed.z * dt));
    return this.isGrounded;
  }
}

/**
 * Joins this object's Rigidbody to `connectedBody` (or to the world, when that's null),
 * like Unity's joints. `anchor` is in this object's space; `connectedAnchor` in the
 * connected body's (or the world's), worked out from where they are now unless
 * `autoConfigureConnectedAnchor` is off. Made when the component starts.
 */
export abstract class Joint extends Component {
  connectedBody: Rigidbody | null = null;
  anchor = Vector3.zero;
  connectedAnchor = Vector3.zero;
  autoConfigureConnectedAnchor = true;
  protected handle: PhysicsJoint | null = null;

  protected abstract kind(): "fixed" | "ball" | "hinge" | "spring" | "rope";
  protected params(): number[] {
    return [];
  }

  /** (Re)creates the joint with the current settings. */
  protected build(): void {
    this.handle?.remove();
    this.handle = null;
    const body = objectPhysics.get(this.gameObject)?.body;
    if (!body) {
      console.warn(`${this.constructor.name} on ${this.gameObject.name} needs a Rigidbody`);
      return;
    }
    const anchor = this.anchor.mul(this.transform.lossyScale);
    const other = this.connectedBody?.body ?? null;
    if (this.autoConfigureConnectedAnchor) {
      const world = this.transform.transformPoint(this.anchor);
      const t = this.connectedBody?.transform;
      // Physics bodies don't scale, so the connected anchor is in its unscaled space.
      this.connectedAnchor = t ? t.rotation.inverse().rotate(world.sub(t.position)) : world;
    }
    this.handle = physics.createJoint(this.kind(), body, other, anchor, this.connectedAnchor, this.params());
  }

  override onStart(): void {
    this.build();
  }

  override onDestroy(): void {
    this.handle?.remove();
    this.handle = null;
  }
}

/** Holds two bodies rigidly together (Unity's `FixedJoint`). */
export class FixedJoint extends Joint {
  protected kind() {
    return "fixed" as const;
  }
}

/** Lets the bodies turn freely about the anchor, ball-and-socket style (ragdoll limbs, chains). */
export class CharacterJoint extends Joint {
  protected kind() {
    return "ball" as const;
  }
}

/** Unity's `JointLimits`: the hinge's range in degrees. */
export interface JointLimits {
  min: number;
  max: number;
}

/** Unity's `JointMotor`: drive the hinge at `targetVelocity` degrees per second. */
export interface JointMotor {
  targetVelocity: number;
  /** How hard it pushes towards that speed. */
  force: number;
}

/** A hinge about `axis` (doors, wheels, levers), with optional limits and a motor. */
export class HingeJoint extends Joint {
  axis = new Vector3(1, 0, 0);
  useLimits = false;
  limits: JointLimits = { min: -90, max: 90 };
  useMotor = false;
  motor: JointMotor = { targetVelocity: 0, force: 1 };

  protected kind() {
    return "hinge" as const;
  }
  protected override params(): number[] {
    const a = this.axis;
    return [a.x, a.y, a.z, this.useLimits ? 1 : 0, this.limits.min, this.limits.max, this.useMotor ? 1 : 0, this.motor.targetVelocity, this.motor.force];
  }
  /** Applies changed limits or motor settings. */
  refresh(): void {
    if (this.handle) this.build();
  }
}

/** Pulls the bodies towards `restLength` apart like a spring (Unity's `SpringJoint`). */
export class SpringJoint extends Joint {
  /** Stiffness. */
  spring = 10;
  /** Damping. */
  damper = 0.2;
  /** The length it rests at; by default, the distance when it's made. */
  restLength: number | null = null;

  protected kind() {
    return "spring" as const;
  }
  protected override params(): number[] {
    let rest = this.restLength;
    if (rest === null) {
      const world = this.transform.transformPoint(this.anchor);
      const t = this.connectedBody?.transform;
      const other = t ? t.position.add(t.rotation.rotate(this.connectedAnchor)) : this.connectedAnchor;
      rest = world.distance(other);
    }
    return [rest, this.spring, this.damper];
  }
}

/** Keeps the bodies at most `maxDistance` apart, slack within it: a rope without segments. */
export class RopeJoint extends Joint {
  maxDistance = 2;
  protected kind() {
    return "rope" as const;
  }
  protected override params(): number[] {
    return [this.maxDistance];
  }
}

/**
 * A rope you can see and hit: a chain of capsule segments with Rigidbodies joined by
 * ball joints, hanging from this object (its Rigidbody, or the world where it is) to
 * `end` (a Rigidbody it holds, optional). Made when the component starts.
 */
export class Rope extends Component {
  /** Total length in metres. */
  length = 3;
  segments = 12;
  radius = 0.04;
  /** Each segment's mass in kg. */
  segmentMass = 0.05;
  color = new Color(0.55, 0.42, 0.28, 1);
  /** What the far end holds, if anything. */
  end: Rigidbody | null = null;
  /** The segment objects (they hang in the world, not under this object). */
  readonly links: GameObject[] = [];
  private joints: PhysicsJoint[] = [];

  override onStart(): void {
    const start = this.transform.position;
    const toEnd = this.end ? this.end.transform.position.sub(start) : Vector3.down.scale(this.length);
    const dir = toEnd.magnitude > 1e-6 ? toEnd.normalized : Vector3.down;
    const n = Math.max(1, Math.round(this.segments));
    const seg = this.length / n;
    // Segments' local -y points along the rope.
    const rotation = Quaternion.fromToRotation(Vector3.down, dir);
    const material = new Material(this.color, Color.black, 0.2);
    const own = objectPhysics.get(this.gameObject);
    let prev: PhysicsBody | null = own?.rigidbody ? own.body : null;
    let prevAnchor = prev ? Vector3.zero : start;
    for (let i = 0; i < n; i++) {
      const center = start.add(dir.scale(seg * (i + 0.5)));
      const link = new GameObject(`${this.gameObject.name} rope ${i}`, {
        position: center,
        rotation,
        // The capsule mesh is 1 m wide and 2 m tall.
        scale: new Vector3(this.radius * 2, seg / 2, this.radius * 2),
      });
      link.addComponent(MeshRenderer, { mesh: "capsule", material });
      link.addComponent(CapsuleCollider);
      const rb = link.addComponent(Rigidbody, { mass: this.segmentMass, angularDrag: 0.5, drag: 0.1 });
      const body = rb.body!;
      this.joints.push(physics.createJoint("ball", body, prev, new Vector3(0, seg / 2, 0), prevAnchor));
      prev = body;
      prevAnchor = new Vector3(0, -seg / 2, 0);
      this.links.push(link);
    }
    if (this.end?.body && prev) {
      this.joints.push(physics.createJoint("ball", this.end.body, prev, Vector3.zero, prevAnchor));
    }
  }

  override onDestroy(): void {
    for (const j of this.joints) j.remove();
    for (const l of this.links) l.destroy();
    this.joints = [];
    this.links.length = 0;
  }
}
