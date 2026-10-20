// Physics components from compiled code: Rigidbody, colliders, joints, Rope, CharacterController and 2D.

function floorBox(): GameObject {
  const floor = new GameObject("floor");
  floor.addComponent(BoxCollider, { size: new Vector3(20, 1, 20) });
  return floor;
}

export function compFall(steps: number): number {
  floorBox();
  const ball = new GameObject("ball", { position: new Vector3(0, 5, 0) });
  ball.addComponent(SphereCollider, { radius: 0.5 });
  ball.addComponent(Rigidbody, { mass: 1 });
  for (let i = 0; i < steps; i++) Physics.step();
  return ball.transform.position.y;
}

export function colliderProps(): number {
  const go = new GameObject("c");
  go.addComponent(BoxCollider, { size: new Vector3(3, 1, 1), center: new Vector3(0, 2, 0), isTrigger: true, bounciness: 0.25 });
  const c = go.getComponent(BoxCollider);
  if (!c) return -1;
  return c.size.x + c.center.y * 10 + (c.isTrigger ? 100 : 0) + c.bounciness * 1000;
}

export function noComponent(): number {
  const go = new GameObject("n");
  go.addComponent(BoxCollider);
  return go.getComponent(Rigidbody) ? 1 : 0;
}

export function bodyProps(): number {
  const go = new GameObject("b");
  const rb = go.addComponent(Rigidbody, { mass: 4, drag: 0.5, useGravity: false, isKinematic: false, constraints: RigidbodyConstraints.FreezeRotation });
  const again = go.getComponent(Rigidbody);
  if (!again) return -1;
  return again.mass + again.drag * 10 + (again.useGravity ? 1000 : 0) + again.constraints * 10000 + rb.mass * 0;
}

export function impulseMode(): number {
  Physics.gravity = new Vector3(0, 0, 0);
  const go = new GameObject("p");
  go.addComponent(SphereCollider);
  const rb = go.addComponent(Rigidbody, { mass: 2 });
  Physics.step();
  rb.addForce(new Vector3(4, 0, 0), ForceMode.Impulse);
  Physics.step();
  return rb.velocity.x;
}

export function fixedJointHolds(steps: number): number {
  const a = new GameObject("a", { position: new Vector3(0, 5, 0) });
  a.addComponent(BoxCollider);
  const arb = a.addComponent(Rigidbody, { isKinematic: true });
  const b = new GameObject("b", { position: new Vector3(0, 3.5, 0) });
  b.addComponent(SphereCollider, { radius: 0.2 });
  b.addComponent(Rigidbody);
  b.addComponent(FixedJoint, { connectedBody: arb });
  for (let i = 0; i < steps; i++) Physics.step();
  return b.transform.position.y;
}

export function ropeHangs(): number {
  const o = new GameObject("rope", { position: new Vector3(0, 5, 0) });
  o.addComponent(Rope, { length: 2, segments: 4 });
  for (let i = 0; i < 60; i++) Physics.step();
  const r = o.getComponent(Rope);
  if (!r) return -1;
  const n = r.pointCount;
  return n * 1000 + r.pointAt(n - 1).y;
}

export function characterLands(): number {
  floorBox();
  const hero = new GameObject("hero", { position: new Vector3(0, 3, 0) });
  const cc = hero.addComponent(CharacterController);
  for (let i = 0; i < 90; i++) {
    cc.move(new Vector3(0.02, -0.1, 0));
    Physics.step();
  }
  return (cc.isGrounded ? 100 : 0) + hero.transform.position.y;
}

export function twoDFall(steps: number): number {
  const ground = new GameObject("ground");
  ground.addComponent(BoxCollider2D, { size: new Vector2(20, 1) });
  const ball = new GameObject("ball", { position: new Vector3(0, 5, 0.7) });
  ball.addComponent(CircleCollider2D, { radius: 0.5 });
  ball.addComponent(Rigidbody2D);
  for (let i = 0; i < steps; i++) Physics.step();
  return ball.transform.position.y * 100 + ball.transform.position.z;
}

export function ray2D(): string {
  const ground = new GameObject("ground2d");
  ground.addComponent(BoxCollider2D, { size: new Vector2(20, 1) });
  Physics.step();
  const hit = Physics2D.raycast(new Vector2(3, 5), new Vector2(0, -1));
  return hit.colliderObject.name + ":" + hit.distance;
}

export function gravity2D(): number {
  Physics2D.gravity = new Vector2(0, -2);
  return Physics.gravity.y;
}
