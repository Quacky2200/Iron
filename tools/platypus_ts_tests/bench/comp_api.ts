// SPDX-License-Identifier: Apache-2.0

// The rest of the Component / GameObject surface of scene.ts as compiled code sees it: the
// object tree (children, roots, scene), the components on an object, the component's own
// helpers and the find family.

class Mark extends Component {
  n: number = 0;
}

class Other extends Component {}

register({ Mark, Other });

export function tree(): string {
  const a = new GameObject("A");
  const b = new GameObject("B");
  const c = new GameObject("C");
  b.setParent(a, false);
  c.setParent(a, false);
  let s = `${a.children.length};`;
  for (const k of a.children) s += k.name;
  s += `;${a.children[1].name};${a.root === a},${b.root === a},${b.parent === a},${a.parent === null},${b.parent !== null};`;
  return s;
}

export function listing(): string {
  const go = new GameObject("Host");
  go.addComponent(Mark, { n: 1 });
  go.addComponent(Other);
  go.addComponent(Mark, { n: 2 });
  return `${go.components.length},${go.components[0] instanceof Mark},${go.components[1] instanceof Other}`;
}

export function roots(): string {
  const before = GameObject.roots.length;
  const r = new GameObject("RootX");
  const child = new GameObject("ChildX");
  child.setParent(r, false);
  let found = false;
  for (const o of GameObject.roots) if (o === r) found = true;
  return `${GameObject.roots.length - before},${found},${GameObject.find("ChildX") === child},${GameObject.find("Nope") === undefined},${GameObject.byId(r.id) === r}`;
}

export function sceneOfObject(): string {
  const go = new GameObject("InScene");
  return `${go.scene === SceneManager.getActiveScene()},${go.scene.name}`;
}
