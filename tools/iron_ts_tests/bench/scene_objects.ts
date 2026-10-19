// GameObject and Transform members (hierarchy, world and local transforms, directions, points),
// compiled against the native API and run in bun against js/engine/src/scene.ts. The scene stores
// floats, so numbers are printed to four decimals.

function f(x: number): string {
  return x.toFixed(3);
}
function v(p: Vector3): string {
  return `(${f(p.x)},${f(p.y)},${f(p.z)})`;
}
function q(r: Quaternion): string {
  return `(${f(r.x)},${f(r.y)},${f(r.z)},${f(r.w)})`;
}

function tree(): GameObject {
  const root = new GameObject("Root");
  root.transform.localPosition = new Vector3(1, 2, 3);
  root.transform.localRotation = Quaternion.euler(0, 90, 0);
  root.transform.localScale = new Vector3(2, 2, 2);
  const arm = new GameObject("Arm");
  arm.setParent(root, false);
  arm.transform.localPosition = new Vector3(1, 0, 0);
  arm.transform.localScale = new Vector3(1, 0.5, 1);
  const hand = new GameObject("Hand");
  hand.setParent(arm, false);
  hand.transform.localPosition = new Vector3(0, 1, 0);
  hand.transform.localRotation = Quaternion.euler(30, 0, 0);
  return root;
}

export function hierarchy(): string {
  const root = tree();
  const arm = root.findChild("Arm");
  const hand = arm.findChild("Hand");
  let s = `${root.childCount},${arm.childCount},${hand.childCount};`;
  s += `${root.findChild("Hand").destroyed},${root.findChild("Nope").destroyed},${arm.findChild("Hand").name};`;
  s += `${hand.root.name},${arm.root.name},${root.root.name},${hand.parent.name},${arm.parent.name};`;
  s += `${arm.childAt(0).name},${root.childAt(0).name},${GameObject.find("Hand").name};`;
  const other = new GameObject("Other");
  s += `${GameObject.rootCount()},${GameObject.rootAt(0).name},${GameObject.rootAt(1).name};`;
  hand.setParent(other, true);
  s += `${arm.childCount},${other.childCount},${hand.parent.name},${hand.root.name};`;
  other.destroy();
  s += `${other.destroyed},${hand.destroyed},${GameObject.rootCount()};`;
  s += `${GameObject.byId(arm.id).name}`;
  arm.hideFlags = HideFlags.DontSave;
  s += `,${arm.hideFlags},${root.hideFlags}`;
  return s;
}

export function worldTransform(): string {
  const root = tree();
  const hand = root.findChild("Arm").findChild("Hand");
  const t = hand.transform;
  let s = `${v(t.position)}${q(t.rotation)}${v(t.lossyScale)}${v(t.eulerAngles)}${v(t.localEulerAngles)};`;
  s += `${v(t.forward)}${v(t.right)}${v(t.up)};`;
  t.position = new Vector3(5, 5, 5);
  s += `${v(t.position)}${v(t.localPosition)};`;
  t.rotation = Quaternion.euler(0, 0, 45);
  s += `${q(t.rotation)}${q(t.localRotation)};`;
  t.eulerAngles = new Vector3(10, 20, 30);
  s += `${v(t.eulerAngles)}${q(t.localRotation)};`;
  t.localEulerAngles = new Vector3(0, 45, 0);
  s += `${v(t.localEulerAngles)}${v(t.eulerAngles)};`;
  s += `${t.parent.gameObject.name}`;
  return s;
}

export function moving(): string {
  const root = tree();
  const hand = root.findChild("Arm").findChild("Hand");
  const t = hand.transform;
  const start = t.position;
  t.translate(new Vector3(1, 0, 0));
  const a = v(t.position.sub(start));
  const b = t.position;
  t.translate(new Vector3(0, 0, 1), "self");
  const c = v(t.position.sub(b));
  const d = t.rotation;
  t.rotate(new Vector3(0, 90, 0));
  const e = q(d.inverse().mul(t.rotation));
  t.rotate(new Vector3(0, 0, 45), "world");
  const g = q(t.rotation);
  t.lookAt(new Vector3(0, 0, 0));
  const h = v(t.forward);
  t.lookAtUp(new Vector3(10, 0, 0), new Vector3(0, 0, 1));
  const i = v(t.forward) + v(t.up);
  return `${a}|${c}|${e}|${g}|${h}|${i}`;
}

export function points(): string {
  const root = tree();
  const hand = root.findChild("Arm").findChild("Hand");
  const t = hand.transform;
  const p = new Vector3(1, 2, 3);
  const w = t.transformPoint(p);
  const back = t.inverseTransformPoint(w);
  return `${v(w)}${v(back)}${v(t.transformDirection(new Vector3(1, 0, 0)))}${v(t.inverseTransformPoint(t.position))}`;
}

export function keepingWorld(): string {
  const root = tree();
  const hand = root.findChild("Arm").findChild("Hand");
  const before = hand.transform.position;
  const rb = hand.transform.rotation;
  const other = new GameObject("Elsewhere", { position: new Vector3(10, 0, 0) });
  hand.setParent(other);
  const a = `${v(before)}${v(hand.transform.position)}${q(rb)}${q(hand.transform.rotation)}${v(hand.transform.localPosition)}`;
  hand.setParent(root, false);
  const b = `${v(hand.transform.position)}${v(hand.transform.localPosition)}`;
  hand.setParent(GameObject.byId(0));
  const c = `${v(hand.transform.position)}${hand.root.name}`;
  return `${a}|${b}|${c}`;
}
