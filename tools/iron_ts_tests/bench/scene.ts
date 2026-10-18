// Compiled scripts driving the real engine scene (`iron_scene`'s `Scene` API, written with
// `#[iron_api]`): GameObject handles and Vector3 / Quaternion values, no JavaScript engine.

/** A parent and a child: the child's world position includes the parent's. */
export function moveAround(): number {
  const before = Scene.count();
  const parent = Scene.create();
  const child = Scene.create();
  Scene.setLocalPosition(parent, new Vector3(10, 0, 0));
  Scene.setParent(child, parent, false);
  Scene.setLocalPosition(child, new Vector3(1, 2, 3));
  const p = Scene.position(child);
  const hasParent = Scene.parent(child) ? 1 : 0;
  const rootHasParent = Scene.parent(parent) ? 1 : 0;
  Scene.destroy(parent);
  const left = Scene.count() - before;
  return p.x * 10000000 + p.y * 100000 + p.z * 1000 + hasParent * 100 + rootHasParent * 10 + left;
}

/** A per-frame update loop: move an object round a circle and read it back. */
export function orbit(frames: number): number {
  const o = Scene.create();
  let sum = 0;
  for (let i = 0; i < frames; i++) {
    const a = i * 0.1;
    Scene.setPosition(o, new Vector3(Math.cos(a) * 5, 0, Math.sin(a) * 5));
    const p = Scene.position(o);
    sum += p.x + p.z;
  }
  Scene.destroy(o);
  return sum;
}

/** Destroyed objects stop answering; a new object reusing the slot does not revive the old handle. */
export function staleHandles(): number {
  const a = Scene.create();
  Scene.destroy(a);
  const gone = Scene.exists(a) ? 1 : 0;
  const b = Scene.create(); // takes a's slot with a new generation
  const reborn = Scene.exists(a) ? 1 : 0;
  const alive = Scene.exists(b) ? 1 : 0;
  const origin = Scene.position(a);
  Scene.destroy(b);
  return gone * 1000 + reborn * 100 + alive * 10 + origin.x + origin.y + origin.z;
}

/** A rotation round y, set from a not-yet-unit Quaternion: read back normalised. */
export function rotation(): number {
  const o = Scene.create();
  Scene.setLocalRotation(o, new Quaternion(0, 0.5, 0, 0.5));
  const q = Scene.localRotation(o);
  Scene.destroy(o);
  return q.y * 1000 + q.w;
}

/** Objects in an array and in a list of handles, between collections. */
export function crowd(n: number): number {
  const before = Scene.count();
  const all: GameObject[] = [];
  for (let i = 0; i < n; i++) {
    const o = Scene.create();
    Scene.setLocalPosition(o, new Vector3(i, i * 2, 0));
    all.push(o);
  }
  let sum = 0;
  for (const o of all) sum += Scene.localPosition(o).y;
  const made = Scene.count() - before;
  for (const o of all) Scene.destroy(o);
  return sum * 1000 + made * 1 + (Scene.count() - before) * 100;
}

/** The same hierarchy through handle classes: `go.transform.position = v` and methods on handles. */
export function classHierarchy(): number {
  const before = Scene.count();
  const parent = GameObject.create();
  const child = GameObject.create();
  parent.transform.localPosition = new Vector3(10, 0, 0);
  child.transform.setParent(parent.transform, false);
  child.transform.localPosition = new Vector3(1, 2, 3);
  const p = child.transform.position;
  const hasParent = child.transform.parent ? 1 : 0;
  const rootHasParent = parent.transform.parent ? 1 : 0;
  parent.destroy();
  const left = Scene.count() - before;
  return p.x * 10000000 + p.y * 100000 + p.z * 1000 + hasParent * 100 + rootHasParent * 10 + left;
}

/** Scalar, boolean and handle properties, and a getter that goes back to the object. */
export function classFlags(): number {
  const o = GameObject.create();
  o.layer = 5;
  o.enabled = false;
  const off = o.activeInHierarchy ? 1 : 0;
  const was = o.enabled ? 1 : 0;
  o.enabled = true;
  const on = o.activeInHierarchy ? 1 : 0;
  const same = o.transform.gameObject === o ? 1 : 0;
  const alive = o.exists() ? 1 : 0;
  const layer = o.layer;
  o.destroy();
  const gone = o.exists() ? 1 : 0;
  return layer * 10000 + off * 1000 + was * 100 + on * 10 + same + alive * 100000 + gone * 1000000;
}

/** A per-frame update loop through `transform.position`. */
export function classOrbit(frames: number): number {
  const o = GameObject.create();
  let sum = 0;
  for (let i = 0; i < frames; i++) {
    const a = i * 0.1;
    o.transform.position = new Vector3(Math.cos(a) * 5, 0, Math.sin(a) * 5);
    const p = o.transform.position;
    sum += p.x + p.z;
  }
  o.destroy();
  return sum;
}
