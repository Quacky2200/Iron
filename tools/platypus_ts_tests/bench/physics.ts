// SPDX-License-Identifier: Apache-2.0

// Physics from compiled code: bodies, colliders, a ray and attaching an object.

export function fall(steps: number): number {
  Physics.addBox(new Vector3(10, 0.5, 10));
  const body = Physics.createBody("dynamic");
  body.position = new Vector3(0, 5, 0);
  body.addBall(0.5);
  for (let i = 0; i < steps; i++) Physics.step();
  return body.position.y;
}

export function restSpeed(steps: number): number {
  Physics.addBox(new Vector3(10, 0.5, 10));
  const body = Physics.createBody("dynamic");
  body.position = new Vector3(0, 3, 0);
  body.addBall(0.5);
  for (let i = 0; i < steps; i++) Physics.step();
  return body.velocity.magnitude;
}

export function rayDown(): number {
  Physics.addBox(new Vector3(10, 0.5, 10));
  const hit = Physics.raycast(new Vector3(0, 5, 0), new Vector3(0, -2, 0));
  return hit.distance * 10 + hit.point.y + hit.normal.y * 1000;
}

export function rayMiss(): number {
  Physics.addBox(new Vector3(1, 0.5, 1));
  const hit = Physics.raycast(new Vector3(50, 5, 0), new Vector3(0, -1, 0), 100);
  return hit.distance;
}

export function followed(go: GameObject, steps: number): number {
  Physics.addBox(new Vector3(10, 0.5, 10));
  const body = Physics.createBody("dynamic");
  body.position = new Vector3(0, 4, 0);
  body.addBall(0.5);
  body.attachTo(go);
  for (let i = 0; i < steps; i++) Physics.step();
  return go.transform.position.y;
}

export function gravityY(): number {
  Physics.gravity = new Vector3(0, -3, 0);
  return Physics.gravity.y;
}

export function impulse(): number {
  Physics.gravity = new Vector3(0, 0, 0);
  const body = Physics.createBody("dynamic");
  body.addBall(0.5);
  body.setMass(2);
  Physics.step();
  body.applyImpulse(new Vector3(4, 0, 0));
  Physics.step();
  return body.velocity.x;
}

export function dropAndKind(): string {
  const b = Physics.createBody("kinematic");
  b.kind = "fixed";
  return b.kind;
}
