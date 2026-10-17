// Physics for gameplay code: rigid bodies, colliders, a character controller and
// raycasts, simulated by rapier in Rust (crates/iron_physics). Scripts hold handles
// with numeric IDs; changes go to the engine as `physics.*` ops and apply before the
// next fixed tick. After each tick Rust calls `__iron_physics(...)` with what moved,
// what collided and query answers, so `position` and friends are always last tick's
// values. Units are metres, seconds and kilograms; y is up.

import { Quaternion, type QuaternionLike as Quat, Vector3, type Vector3Like as Vec3 } from "./math.ts";

export type BodyKind = "dynamic" | "fixed" | "kinematic";

export interface BodyOptions {
  /** `dynamic` (simulated, the default), `fixed` (never moves) or `kinematic` (moved by script). */
  kind?: BodyKind;
  position?: Vec3;
  rotation?: Quat;
  velocity?: Vec3;
  /** Multiplies gravity for this body (0: floats). Default 1. */
  gravityScale?: number;
  linearDamping?: number;
  angularDamping?: number;
  /** Continuous collision detection, for fast bodies that would pass through thin walls. */
  ccd?: boolean;
  /** Keeps the body upright (characters, pickups). */
  lockRotations?: boolean;
}

export type Shape =
  | { box: Vec3 } // half extents
  | { ball: number } // radius
  | { capsule: { halfHeight: number; radius: number } } // along y
  | { cylinder: { halfHeight: number; radius: number } } // along y
  | { convex: Vec3[] } // the convex hull of these points
  | { mesh: { points: Vec3[]; indices: number[] } }; // triangles, for static level geometry

export interface ColliderOptions {
  shape: Shape;
  /** Offset from the body. */
  position?: Vec3;
  rotation?: Quat;
  /** Default 0.5. */
  friction?: number;
  /** Bounciness, 0 to 1. Default 0. */
  restitution?: number;
  /** kg/m³; sets the body's mass with the shape's volume. Default 1. */
  density?: number;
  /** A trigger: reports overlaps through collision events but doesn't push anything. */
  sensor?: boolean;
}

export interface CollisionEvent {
  /** This body's collider that collided. */
  collider: PhysicsCollider;
  /** What it collided with. */
  other: PhysicsCollider;
  /** Whether either collider is a sensor (an overlap, not a contact). */
  sensor: boolean;
  /** How fast they were moving relative to each other going in (m/s); 0 on end. */
  speed: number;
}

export interface RayHit {
  collider: PhysicsCollider;
  distance: number;
  point: Vector3;
  normal: Vector3;
}

export interface CharacterOptions {
  /** Steepest slope it walks up, in degrees. Default 45. */
  maxSlope?: number;
  /** Keeps it on the ground going down slopes and steps up to this far (metres). */
  snapToGround?: number;
  /** Steps up ledges this high (metres) automatically. */
  stepHeight?: number;
}

const vec = (v: Vec3 | undefined): number[] | null => (v ? [v.x, v.y, v.z] : null);
const quat = (q: Quat | undefined): number[] | null => (q ? [q.x, q.y, q.z, q.w] : null);

// IDs are chosen here, like UI node IDs, so creating a body needs no round trip.
let nextBody = 1;
let nextCollider = 1;
let nextQuery = 1;
const bodies = new Map<number, PhysicsBody>();
const colliders = new Map<number, PhysicsCollider>();
const queries = new Map<number, { origin: Vec3; dir: Vec3; resolve: (hit: RayHit | null) => void }>();

export class PhysicsCollider {
  readonly id: number;
  /** The body it's attached to; `null` for a static collider attached to the world. */
  readonly body: PhysicsBody | null;
  readonly sensor: boolean;

  /** @internal Use `body.addCollider()` or `physics.addCollider()`. */
  constructor(body: PhysicsBody | null, options: ColliderOptions) {
    this.id = nextCollider++;
    this.body = body;
    this.sensor = !!options.sensor;
    const s = options.shape;
    let shape: string;
    let dims: number[];
    let indices: number[] | null = null;
    if ("box" in s) [shape, dims] = ["box", [s.box.x, s.box.y, s.box.z]];
    else if ("ball" in s) [shape, dims] = ["ball", [s.ball]];
    else if ("capsule" in s) [shape, dims] = ["capsule", [s.capsule.halfHeight, s.capsule.radius]];
    else if ("cylinder" in s) [shape, dims] = ["cylinder", [s.cylinder.halfHeight, s.cylinder.radius]];
    else if ("convex" in s) [shape, dims] = ["convex", s.convex.flatMap((p) => [p.x, p.y, p.z])];
    else {
      [shape, dims] = ["mesh", s.mesh.points.flatMap((p) => [p.x, p.y, p.z])];
      indices = s.mesh.indices;
    }
    colliders.set(this.id, this);
    // Collision events are on for every collider; bodies' handlers pick what they need.
    __iron_op("physics.collider", this.id, body?.id ?? null, shape, dims, vec(options.position), quat(options.rotation), [
      options.friction ?? 0.5,
      options.restitution ?? 0,
      options.density ?? 1,
      this.sensor,
      true,
      indices,
    ]);
  }

  setFriction(friction: number): void {
    __iron_op("physics.colliderSet", this.id, "friction", friction);
  }

  setRestitution(restitution: number): void {
    __iron_op("physics.colliderSet", this.id, "restitution", restitution);
  }

  setSensor(sensor: boolean): void {
    (this as { sensor: boolean }).sensor = sensor;
    __iron_op("physics.colliderSet", this.id, "sensor", sensor);
  }

  /** Where it sits on its body. */
  setOffset(position: Vec3, rotation?: Quat): void {
    __iron_op("physics.colliderSet", this.id, "offset", vec(position), quat(rotation));
  }

  /** Removes the collider from its body (or the world). */
  remove(): void {
    if (!colliders.delete(this.id)) return;
    __iron_op("physics.removeCollider", this.id);
  }
}

export class PhysicsBody {
  readonly id: number;
  readonly kind: BodyKind;
  /** Where the body was after the last tick (or where it was created / last set). */
  position: Vector3;
  rotation: Quaternion;
  velocity: Vector3;
  /** A character controller's ground contact after its last `move`. */
  grounded = false;
  /** Called when one of this body's colliders starts touching (or overlapping) another. */
  onCollisionStart?: (event: CollisionEvent) => void;
  /** Called when they stop. */
  onCollisionEnd?: (event: CollisionEvent) => void;

  /** @internal Use `physics.createBody()`. */
  constructor(options: BodyOptions = {}) {
    this.id = nextBody++;
    this.kind = options.kind ?? "dynamic";
    this.position = Vector3.from(options.position ?? Vector3.zero);
    this.rotation = Quaternion.from(options.rotation ?? Quaternion.identity);
    this.velocity = Vector3.from(options.velocity ?? Vector3.zero);
    bodies.set(this.id, this);
    __iron_op("physics.body", this.id, this.kind, vec(this.position), quat(this.rotation), vec(this.velocity), [
      options.gravityScale ?? 1,
      options.linearDamping ?? 0,
      options.angularDamping ?? 0,
      !!options.ccd,
      !!options.lockRotations,
    ]);
  }

  addCollider(options: ColliderOptions): PhysicsCollider {
    return new PhysicsCollider(this, options);
  }

  /** Teleports the body. */
  setPosition(p: Vec3): void {
    this.position = Vector3.from(p);
    __iron_op("physics.set", this.id, "position", vec(p));
  }

  setRotation(q: Quat): void {
    this.rotation = Quaternion.from(q);
    __iron_op("physics.set", this.id, "rotation", quat(q));
  }

  setVelocity(v: Vec3): void {
    this.velocity = Vector3.from(v);
    __iron_op("physics.set", this.id, "velocity", vec(v));
  }

  /** Radians per second around each axis. */
  setAngularVelocity(v: Vec3): void {
    __iron_op("physics.set", this.id, "angularVelocity", vec(v));
  }

  /** A kinematic body's position at the end of the next tick (it pushes things on the way). */
  moveTo(p: Vec3): void {
    __iron_op("physics.set", this.id, "target", vec(p));
  }

  /** An instant change in momentum (N·s): jumps, hits, explosions. */
  applyImpulse(impulse: Vec3): void {
    __iron_op("physics.force", this.id, "impulse", vec(impulse));
  }

  applyTorqueImpulse(impulse: Vec3): void {
    __iron_op("physics.force", this.id, "torqueImpulse", vec(impulse));
  }

  /** A force (N) applied every tick until `clearForces()`. */
  addForce(force: Vec3): void {
    __iron_op("physics.force", this.id, "force", vec(force));
  }

  addTorque(torque: Vec3): void {
    __iron_op("physics.force", this.id, "torque", vec(torque));
  }

  clearForces(): void {
    __iron_op("physics.clearForces", this.id);
  }

  /** An instant impulse at a world point (it also spins the body). */
  applyImpulseAtPoint(impulse: Vec3, point: Vec3): void {
    __iron_op("physics.force", this.id, "impulseAt", vec(impulse), vec(point));
  }

  /** Switches between simulated (`dynamic`), immovable (`fixed`) and script-moved (`kinematic`). */
  setKind(kind: BodyKind): void {
    (this as { kind: BodyKind }).kind = kind;
    __iron_op("physics.set", this.id, "kind", kind);
  }

  /** Total mass in kg (shared by its colliders). */
  setMass(kg: number): void {
    __iron_op("physics.set", this.id, "mass", kg);
  }

  setGravityScale(scale: number): void {
    __iron_op("physics.set", this.id, "gravityScale", scale);
  }

  setDamping(linear: number, angular: number): void {
    __iron_op("physics.set", this.id, "linearDamping", linear);
    __iron_op("physics.set", this.id, "angularDamping", angular);
  }

  setCcd(on: boolean): void {
    __iron_op("physics.set", this.id, "ccd", on);
  }

  /** Which axes it may move along and turn about: [x, y, z] translations, then rotations. */
  setAllowedAxes(translation: [boolean, boolean, boolean], rotation: [boolean, boolean, boolean]): void {
    __iron_op("physics.set", this.id, "constraints", [...translation, ...rotation]);
  }

  sleep(): void {
    __iron_op("physics.set", this.id, "sleep");
  }

  wakeUp(): void {
    __iron_op("physics.set", this.id, "wake");
  }

  /** Makes the body move a GameObject (and follow it when scripts move the object). */
  attachTo(objectId: number): void {
    __iron_op("physics.attach", this.id, objectId);
  }

  /** Disabled bodies are left out of the simulation (and keep their state). */
  setEnabled(enabled: boolean): void {
    __iron_op("physics.set", this.id, "enabled", enabled);
  }

  /**
   * Makes this body a character: `move()` then slides it along walls, climbs slopes and
   * steps, instead of it being pushed around. Use a `kinematic` body with a capsule
   * collider (its first collider is the character's shape).
   */
  makeCharacter(options: CharacterOptions = {}): void {
    __iron_op("physics.character", this.id, options.maxSlope ?? 45, options.snapToGround ?? null, options.stepHeight ?? null);
  }

  /**
   * Moves a character by `delta` metres this tick (moves in the same tick add up). Include
   * a small downward pull to keep it on the ground; `grounded` updates after the tick.
   */
  move(delta: Vec3): void {
    __iron_op("physics.move", this.id, vec(delta));
  }

  /** Removes the body and its colliders. */
  remove(): void {
    if (!bodies.delete(this.id)) return;
    for (const [id, c] of colliders) if (c.body === this) colliders.delete(id);
    __iron_op("physics.removeBody", this.id);
  }
}

export type JointKind = "fixed" | "ball" | "hinge" | "spring" | "rope";

let nextJoint = 1;

/** A joint between two bodies (or a body and the world). */
export class PhysicsJoint {
  readonly id: number;
  /** @internal Use `physics.createJoint()`. */
  constructor(
    readonly kind: JointKind,
    a: PhysicsBody,
    b: PhysicsBody | null,
    anchorA: Vec3,
    anchorB: Vec3,
    params: number[] = [],
  ) {
    this.id = nextJoint++;
    __iron_op("physics.joint", this.id, kind, a.id, b?.id ?? null, vec(anchorA), vec(anchorB), params);
  }

  remove(): void {
    __iron_op("physics.removeJoint", this.id);
  }
}

export const physics = {
  /**
   * Joins body `a` to `b` (or the world, with `null`) at anchors in their own spaces (the
   * world's, for `b` null). `params`: hinge [axis x, y, z, use limits 0/1, min, max
   * (degrees), use motor 0/1, speed (degrees/s), strength]; spring [rest length,
   * stiffness, damping]; rope [max length].
   */
  createJoint(kind: JointKind, a: PhysicsBody, b: PhysicsBody | null, anchorA: Vec3, anchorB: Vec3, params?: number[]): PhysicsJoint {
    return new PhysicsJoint(kind, a, b, anchorA, anchorB, params);
  },

  createBody(options?: BodyOptions): PhysicsBody {
    return new PhysicsBody(options);
  },

  /** A collider on the world itself (floors, walls): it never moves. */
  addCollider(options: ColliderOptions): PhysicsCollider {
    return new PhysicsCollider(null, options);
  },

  setGravity(g: Vec3): void {
    __iron_op("physics.gravity", vec(g));
  },

  /**
   * Casts a ray (after the next tick) and resolves with the first collider it hits within
   * `maxDistance`, or `null`.
   */
  raycast(origin: Vec3, direction: Vec3, maxDistance = 1000): Promise<RayHit | null> {
    const id = nextQuery++;
    const len = Math.hypot(direction.x, direction.y, direction.z) || 1;
    const dir = { x: direction.x / len, y: direction.y / len, z: direction.z / len };
    return new Promise((resolve) => {
      queries.set(id, { origin: { ...origin }, dir, resolve });
      __iron_op("physics.raycast", id, vec(origin), vec(dir), maxDistance);
    });
  },

  /** A body by id, if it exists. */
  body(id: number): PhysicsBody | undefined {
    return bodies.get(id);
  },
};

/** @internal Called for every contact (the component layer turns them into Unity-style callbacks). */
export const contactListeners: ((a: PhysicsCollider, b: PhysicsCollider, started: boolean, sensor: boolean, speed: number) => void)[] = [];

function report(fn: ((e: CollisionEvent) => void) | undefined, event: CollisionEvent) {
  if (!fn) return;
  try {
    fn(event);
  } catch (e) {
    const err = e instanceof Error ? e : new Error(String(e));
    console.error(`[physics] error in collision handler: ${err.message}${err.stack ? `\n${err.stack}` : ""}`);
  }
}

/** Clears all handles. Only for tests. */
export function _resetPhysicsForTests() {
  bodies.clear();
  colliders.clear();
  queries.clear();
  nextBody = nextCollider = nextQuery = nextJoint = 1;
}

// Flat arrays, as documented on `Batch::to_args` (crates/iron_physics).
globalThis.__iron_physics = (moved: number[], contacts: number[], rays: number[], characters: number[]) => {
  for (let i = 0; i + 10 < moved.length; i += 11) {
    const b = bodies.get(moved[i]!);
    if (!b) continue;
    b.position = new Vector3(moved[i + 1], moved[i + 2], moved[i + 3]);
    b.rotation = new Quaternion(moved[i + 4], moved[i + 5], moved[i + 6], moved[i + 7]);
    b.velocity = new Vector3(moved[i + 8], moved[i + 9], moved[i + 10]);
  }
  for (let i = 0; i + 1 < characters.length; i += 2) {
    const b = bodies.get(characters[i]!);
    if (b) b.grounded = characters[i + 1] === 1;
  }
  for (let i = 0; i + 4 < contacts.length; i += 5) {
    const a = colliders.get(contacts[i + 1]!);
    const b = colliders.get(contacts[i + 2]!);
    if (!a || !b) continue;
    const started = contacts[i] === 1;
    const sensor = contacts[i + 3] === 1;
    const speed = contacts[i + 4]!;
    for (const listener of contactListeners) {
      try {
        listener(a, b, started, sensor, speed);
      } catch (e) {
        const err = e instanceof Error ? e : new Error(String(e));
        console.error(`[physics] error in collision callback: ${err.message}${err.stack ? `\n${err.stack}` : ""}`);
      }
    }
    for (const [self, other] of [
      [a, b],
      [b, a],
    ] as const) {
      const body = self.body;
      if (!body) continue;
      report(started ? body.onCollisionStart : body.onCollisionEnd, { collider: self, other, sensor, speed });
    }
  }
  for (let i = 0; i + 5 < rays.length; i += 6) {
    const q = queries.get(rays[i]!);
    if (!q) continue;
    queries.delete(rays[i]!);
    const collider = colliders.get(rays[i + 1]!);
    if (!collider) {
      q.resolve(null);
      continue;
    }
    const distance = rays[i + 2]!;
    q.resolve({
      collider,
      distance,
      point: new Vector3(q.origin.x + q.dir.x * distance, q.origin.y + q.dir.y * distance, q.origin.z + q.dir.z * distance),
      normal: new Vector3(rays[i + 3], rays[i + 4], rays[i + 5]),
    });
  }
};
