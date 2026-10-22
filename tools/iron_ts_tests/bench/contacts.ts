// Contact hooks: the physics step reports contacts and the host calls onCollisionEnter / Exit /
// Stay and onTriggerEnter / Exit / Stay by name on the components of both objects.

let log = "";
let ball: PhysicsBody | null = null;

class Probe extends Component {
  onCollisionEnter(c: Collision): void {
    log += `${this.gameObject!.name}:E(${c.gameObject.name},${c.relativeVelocity.magnitude > 0.1 ? "fast" : "slow"})`;
  }
  onCollisionExit(c: Collision): void {
    log += `${this.gameObject!.name}:X(${c.gameObject.name},${c.contactCount})`;
  }
  onCollisionStay(c: Collision): void {
    log += `${this.gameObject!.name}:S`;
  }
  // A trigger hook may ask for the `Collision` rather than the other `Collider`.
  onTriggerEnter(c: Collision): void {
    log += `${this.gameObject!.name}:TE(${c.gameObject.name},${c.contactCount})`;
  }
  onTriggerExit(c: Collision): void {
    log += `${this.gameObject!.name}:TX`;
  }
  onTriggerStay(c: Collision): void {
    log += `${this.gameObject!.name}:TS`;
  }
}

// Only a subclass method: inherited hooks are found too.
class Child extends Probe {
  onUpdate(dt: number): void {}
}

// A hook that destroys its own object's component and one that throws.
class Quitter extends Component {
  onCollisionEnter(c: Collision): void {
    log += "Q";
    this.destroy();
  }
}

class Thrower extends Component {
  onCollisionEnter(c: Collision): void {
    log += "T";
    throw new Error("boom");
  }
}

/** Writes down what Unity's `Collision` members say on entry (and the other `Collider` of a trigger). */
class Recorder extends Component {
  onCollisionEnter(c: Collision): void {
    const n = c.contactCount;
    let same = c.contacts.length === n;
    let y = 0;
    let ny = 0;
    for (let i = 0; i < n; i++) {
      const p = c.getContact(i);
      y += p.point.y / n;
      ny += p.normal.y / n;
      if (p.thisCollider.gameObject.name !== this.gameObject!.name) same = false;
      if (p.otherCollider.gameObject.name !== c.gameObject.name) same = false;
      if (p.separation > 0.1) same = false;
    }
    const rb = c.rigidbody;
    log +=
      `${this.gameObject!.name}:${c.gameObject.name} collider=${c.collider.gameObject.name}` +
      ` trigger=${c.collider.isTrigger} rigidbody=${rb ? rb.gameObject.name : "none"}` +
      ` ty=${c.transform.position.y}` +
      ` vy=${c.relativeVelocity.y} n=${n} y=${y} ny=${ny} iy=${c.impulse.y} same=${same};`;
  }
  onTriggerEnter(other: Collider): void {
    const rb = other.attachedRigidbody;
    log += `${this.gameObject!.name}:T(${other.gameObject.name},${other.isTrigger},${rb ? "rb" : "none"},${other.bounds.size.x});`;
  }
}

/** The same for `Collision2D` and a 2D trigger's `Collider2D`. */
class Recorder2D extends Component {
  onCollisionEnter2D(c: Collision2D): void {
    const n = c.contactCount;
    let ny = 0;
    let impulse = 0;
    for (const p of c.contacts) {
      ny += p.normal.y / n;
      impulse += p.normalImpulse;
      if (p.collider.gameObject.name !== c.gameObject.name) ny = 99;
      if (p.otherCollider.gameObject.name !== this.gameObject!.name) ny = 99;
    }
    const rb = c.rigidbody;
    const own = c.otherRigidbody;
    log +=
      `${this.gameObject!.name}:${c.gameObject.name} collider=${c.collider.gameObject.name}` +
      ` other=${c.otherCollider.gameObject.name} rigidbody=${rb ? rb.gameObject.name : "none"}` +
      ` own=${own ? own.gameObject.name : "none"} vy=${c.relativeVelocity.y} n=${n}` +
      ` y=${c.getContact(0).point.y} ny=${ny} impulse=${impulse} enabled=${c.enabled};`;
  }
  onTriggerEnter2D(other: Collider2D): void {
    log += `${this.gameObject!.name}:T2(${other.gameObject.name},${other.isTrigger});`;
  }
}

register({ Probe, Child, Quitter, Thrower, Recorder, Recorder2D });

/** A floor (BoxCollider, no Rigidbody) and a ball (SphereCollider and Rigidbody) 1.5 m above it, and
 * a trigger box (BoxCollider 2 m across, `isTrigger`) the ball falls through on the way. */
export function buildComponents(floor: GameObject, thing: GameObject, zone: GameObject): void {
  Physics.gravity = new Vector3(0, -9.81, 0);
  floor.transform.position = new Vector3(0, -0.5, 0);
  floor.addComponent(BoxCollider, { size: new Vector3(20, 1, 20) });
  zone.transform.position = new Vector3(0, 1.5, 0);
  zone.addComponent(BoxCollider, { size: new Vector3(2, 0.2, 2), isTrigger: true });
  thing.transform.position = new Vector3(0, 2, 0);
  thing.addComponent(SphereCollider, { radius: 0.5 });
  thing.addComponent(Rigidbody, { mass: 1 });
}

/** The same in 2D, and a 2D trigger. */
export function buildComponents2D(floor: GameObject, thing: GameObject, zone: GameObject): void {
  Physics2D.gravity = new Vector2(0, -9.81);
  floor.transform.position = new Vector3(0, -0.5, 0);
  floor.addComponent(BoxCollider2D, { size: new Vector2(20, 1) });
  zone.transform.position = new Vector3(0, 1.5, 0);
  zone.addComponent(BoxCollider2D, { size: new Vector2(2, 0.2), isTrigger: true });
  thing.transform.position = new Vector3(0, 2, 0);
  thing.addComponent(CircleCollider2D, { radius: 0.5 });
  thing.addComponent(Rigidbody2D);
}

export function logText(): string {
  return log;
}

export function clearLog(): void {
  log = "";
}

/** A fixed floor on `floor` and a ball body on `ball`, `height` above it. */
export function build(floor: GameObject, thing: GameObject, height: number, sensor: boolean): void {
  Physics.gravity = new Vector3(0, sensor ? 0 : -9.81, 0);
  const f = Physics.createBody("fixed");
  f.addBox(new Vector3(10, 0.5, 10));
  f.attachTo(floor);
  const b = Physics.createBody("dynamic");
  b.position = new Vector3(0, height, 0);
  const col = b.addBall(0.5);
  if (sensor) col.setSensor(true);
  b.attachTo(thing);
  ball = b;
}

export function steps(n: number): void {
  for (let i = 0; i < n; i++) Physics.step();
}

export function lift(): void {
  ball!.position = new Vector3(0, 50, 0);
  ball!.velocity = new Vector3(0, 0, 0);
}
