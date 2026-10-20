// Contact hooks: the physics step reports contacts and the host calls onCollisionEnter / Exit /
// Stay and onTriggerEnter / Exit / Stay by name on the components of both objects.

let log = "";
let ball: Body | null = null;

class Probe extends Component {
  onCollisionEnter(c: Collision): void {
    log += `${this.gameObject!.name}:E(${c.other!.name},${c.speed > 0.1 ? "fast" : "slow"})`;
  }
  onCollisionExit(c: Collision): void {
    log += `${this.gameObject!.name}:X(${c.other!.name})`;
  }
  onCollisionStay(c: Collision): void {
    log += `${this.gameObject!.name}:S`;
  }
  onTriggerEnter(c: Collision): void {
    log += `${this.gameObject!.name}:TE(${c.other!.name},${c.sensor})`;
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

register({ Probe, Child, Quitter, Thrower });

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
