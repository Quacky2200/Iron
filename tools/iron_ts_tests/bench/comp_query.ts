// Looking up and removing components of the program own classes: getComponent (subclasses count),
// getComponents, getComponentInChildren, GameObject.findObjectOfType, removeComponent, destroy(),
// transform and gameObject on a component. Compiled with a NativeHost and run in bun against
// js/engine/src/scene.ts.

class Base extends Component {
  hp: number = 10;
  heal(n: number): number {
    this.hp += n;
    return this.hp;
  }
}

class Derived extends Base {
  bonus: number = 5;
}

class Other extends Component {
  tag: number = 1;
}

register({ Base, Derived, Other });

function hp(c: Base | undefined): number {
  return c ? c.hp : -1;
}

export function lookups(): string {
  const go = new GameObject("A");
  const b = go.addComponent(Base);
  const d = go.addComponent(Derived, { hp: 20, bonus: 7 });
  const o = go.addComponent(Other);
  let s = `${hp(go.getComponent(Base))},${hp(go.getComponent(Derived))};`;
  s += `${go.getComponent(Base) === b},${go.getComponent(Derived) === d},${go.getComponent(Other) === o};`;
  const bases = go.getComponents(Base);
  const derived = go.getComponents(Derived);
  const others = go.getComponents(Other);
  s += `${bases.length},${derived.length},${others.length},${bases[0] === b},${bases[1] === d};`;
  const empty = new GameObject("Empty");
  s += `${empty.getComponents(Base).length},${hp(empty.getComponent(Base))},${empty.getComponent(Other) === undefined};`;
  s += `${d.heal(5)},${b.heal(1)},${hp(go.getComponent(Base))},${d.gameObject!.name},${o.gameObject === go};`;
  return s;
}

export function children(): string {
  const parent = new GameObject("Parent");
  const mid = new GameObject("Mid");
  const leaf = new GameObject("Leaf");
  mid.setParent(parent, false);
  leaf.setParent(mid, false);
  parent.addComponent(Other, { tag: 1 });
  const deep = leaf.addComponent(Derived, { hp: 33 });
  const lower = leaf.addComponent(Other, { tag: 2 });
  let s = `${hp(parent.getComponentInChildren(Base))},${parent.getComponentInChildren(Base) === deep},`;
  s += `${parent.getComponentInChildren(Other) === lower},${mid.getComponentInChildren(Other) === lower},`;
  s += `${parent.getComponentInChildren(Other)!.tag},${mid.getComponentInChildren(Other)!.tag},`;
  s += `${mid.getComponentInChildren(Derived) === deep},${leaf.getComponentInChildren(Base) === deep}`;
  return s;
}

export function finding(): string {
  const none = GameObject.findObjectOfType(Derived);
  const first = new GameObject("First");
  const second = new GameObject("Second");
  const a = first.addComponent(Other, { tag: 7 });
  const b = second.addComponent(Derived, { hp: 40 });
  const c = first.addComponent(Derived, { hp: 41 });
  return `${none === undefined},${GameObject.findObjectOfType(Other) === a},${GameObject.findObjectOfType(Derived) === b},${GameObject.findObjectOfType(Base) === b},${c.gameObject === first}`;
}

export function removing(): string {
  const go = new GameObject("R");
  const b = go.addComponent(Base);
  const d = go.addComponent(Derived);
  const o = go.addComponent(Other);
  let s = `${go.getComponents(Base).length};`;
  go.removeComponent(d);
  s += `${go.getComponents(Base).length},${go.getComponent(Derived) === undefined},${go.getComponent(Base) === b};`;
  b.destroy();
  s += `${go.getComponents(Base).length},${go.getComponent(Base) === undefined},${go.getComponent(Other) === o};`;
  go.removeComponent(o);
  go.removeComponent(o);
  s += `${go.getComponents(Other).length}`;
  return s;
}
