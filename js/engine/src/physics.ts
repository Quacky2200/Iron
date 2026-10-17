// Physics for gameplay code: rigid bodies, colliders, a character controller and
// raycasts, simulated by rapier in Rust (crates/iron_physics). Scripts hold handles
// with numeric IDs; changes go to the engine as `physics.*` ops and apply before the
// next fixed tick. After each tick Rust calls `__iron_physics(...)` with what moved,
// what collided and query answers, so `position` and friends are always last tick's
// values. Units are metres, seconds and kilograms; y is up.

/** A vector in world space. */
export interface Vec3 {
  x: number;
  y: number;
  z: number;
}

/** A rotation as a unit quaternion. */
export interface Quat {
  x: number;
  y: number;
  z: number;
  w: number;
}

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
  collider: Collider;
  /** What it collided with. */
  other: Collider;
  /** Whether either collider is a sensor (an overlap, not a contact). */
  sensor: boolean;
  /** How fast they were moving relative to each other going in (m/s); 0 on end. */
  speed: number;
}

export interface RayHit {
  collider: Collider;
  distance: number;
  point: Vec3;
  normal: Vec3;
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
const bodies = new Map<number, RigidBody>();
const colliders = new Map<number, Collider>();
const queries = new Map<number, { origin: Vec3; dir: Vec3; resolve: (hit: RayHit | null) => void }>();

export class Collider {
  readonly id: number;
  /** The body it's attached to; `null` for a static collider attached to the world. */
  readonly body: RigidBody | null;
  readonly sensor: boolean;

  /** @internal Use `body.addCollider()` or `physics.addCollider()`. */
  constructor(body: RigidBody | null, options: ColliderOptions) {
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

  /** Removes the collider from its body (or the world). */
  remove(): void {
    if (!colliders.delete(this.id)) return;
    __iron_op("physics.removeCollider", this.id);
  }
}

export class RigidBody {
  readonly id: number;
  readonly kind: BodyKind;
  /** Where the body was after the last tick (or where it was created / last set). */
  readonly position: Vec3;
  readonly rotation: Quat;
  readonly velocity: Vec3;
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
    this.position = { x: 0, y: 0, z: 0, ...options.position };
    this.rotation = { x: 0, y: 0, z: 0, w: 1, ...options.rotation };
    this.velocity = { x: 0, y: 0, z: 0, ...options.velocity };
    bodies.set(this.id, this);
    __iron_op("physics.body", this.id, this.kind, vec(this.position), quat(this.rotation), vec(this.velocity), [
      options.gravityScale ?? 1,
      options.linearDamping ?? 0,
      options.angularDamping ?? 0,
      !!options.ccd,
      !!options.lockRotations,
    ]);
  }

  addCollider(options: ColliderOptions): Collider {
    return new Collider(this, options);
  }

  /** Teleports the body. */
  setPosition(p: Vec3): void {
    Object.assign(this.position, { x: p.x, y: p.y, z: p.z });
    __iron_op("physics.set", this.id, "position", vec(p));
  }

  setRotation(q: Quat): void {
    Object.assign(this.rotation, { x: q.x, y: q.y, z: q.z, w: q.w });
    __iron_op("physics.set", this.id, "rotation", quat(q));
  }

  setVelocity(v: Vec3): void {
    Object.assign(this.velocity, { x: v.x, y: v.y, z: v.z });
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

export const physics = {
  createBody(options?: BodyOptions): RigidBody {
    return new RigidBody(options);
  },

  /** A collider on the world itself (floors, walls): it never moves. */
  addCollider(options: ColliderOptions): Collider {
    return new Collider(null, options);
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
  body(id: number): RigidBody | undefined {
    return bodies.get(id);
  },
};

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
  nextBody = nextCollider = nextQuery = 1;
}

// Flat arrays, as documented on `Batch::to_args` (crates/iron_physics).
globalThis.__iron_physics = (moved: number[], contacts: number[], rays: number[], characters: number[]) => {
  for (let i = 0; i + 10 < moved.length; i += 11) {
    const b = bodies.get(moved[i]!);
    if (!b) continue;
    Object.assign(b.position, { x: moved[i + 1], y: moved[i + 2], z: moved[i + 3] });
    Object.assign(b.rotation, { x: moved[i + 4], y: moved[i + 5], z: moved[i + 6], w: moved[i + 7] });
    Object.assign(b.velocity, { x: moved[i + 8], y: moved[i + 9], z: moved[i + 10] });
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
      point: {
        x: q.origin.x + q.dir.x * distance,
        y: q.origin.y + q.dir.y * distance,
        z: q.origin.z + q.dir.z * distance,
      },
      normal: { x: rays[i + 3]!, y: rays[i + 4]!, z: rays[i + 5]! },
    });
  }
};
