// Inheritance: extends, super, overriding, abstract classes, virtual calls, instanceof, downcasts.
export abstract class Shape {
  static created = 0;
  tag = 1;
  constructor(readonly sides: number) {
    Shape.created += 1;
  }
  abstract area(): number;
  /** Calls `area` on `this`: a virtual call. */
  describe(): number {
    return this.sides * 1000 + this.area();
  }
  get kind(): number {
    return 0;
  }
  scaled(k: number): number {
    return this.area() * k;
  }
}

export class Rect extends Shape {
  constructor(
    public w: number,
    public h: number,
  ) {
    super(4);
    this.tag = 2;
  }
  area(): number {
    return this.w * this.h;
  }
  override get kind(): number {
    return 1;
  }
}

export class Square extends Rect {
  extra = 5;
  constructor(s: number) {
    super(s, s);
  }
  override area(): number {
    return super.area() + 1;
  }
  override get kind(): number {
    return 2;
  }
  onlyOnSquare(): number {
    return this.extra * 100 + this.w;
  }
}

export class Circle extends Shape {
  constructor(public r: number) {
    super(0);
  }
  area(): number {
    return 3 * this.r * this.r;
  }
}

/** No constructor of its own: takes the base class's parameters. The initialiser runs after them. */
export class Tri extends Shape {
  corner = 3;
  area(): number {
    return this.sides + this.corner;
  }
}

class A {
  x = 1;
  y = 10;
}
class B extends A {
  x = 5;
  z = 100;
}

class Base {
  v = 1;
  constructor() {
    this.v = this.compute();
  }
  compute(): number {
    return 10;
  }
}
class Derived extends Base {
  k = 7;
  override compute(): number {
    return 20;
  }
}

class Node2 {
  child: Node2 | null = null;
  constructor(public depth: number) {}
  total(): number {
    return this.depth + (this.child === null ? 0 : this.child.total());
  }
}
class Node3 extends Node2 {
  override total(): number {
    return 1000 + super.total();
  }
}

export function demoVirtual(): number {
  const shapes: Shape[] = [new Rect(2, 3), new Square(4), new Circle(2), new Tri(3)];
  let s = 0;
  for (const sh of shapes) s = s * 100 + sh.area();
  return s;
}

export function demoThisDispatch(): number {
  const shapes: Shape[] = [new Rect(2, 3), new Square(4), new Circle(2), new Tri(3)];
  let s = 0;
  for (const sh of shapes) s += sh.describe();
  return s;
}

export function demoAccessors(): number {
  const shapes: Shape[] = [new Rect(1, 1), new Square(1), new Circle(1), new Tri(1)];
  let s = 0;
  for (const sh of shapes) s = s * 10 + sh.kind;
  return s;
}

export function demoSuper(): number {
  const sq = new Square(3);
  const r: Rect = sq;
  return sq.area() * 100 + r.area() + sq.scaled(2) * 10000;
}

export function demoInstanceof(): number {
  const shapes: (Shape | null)[] = [new Rect(2, 3), new Square(4), new Circle(2), new Tri(3), null];
  let rects = 0;
  let squares = 0;
  let circles = 0;
  let shapesN = 0;
  for (const sh of shapes) {
    if (sh instanceof Rect) rects += 1;
    if (sh instanceof Square) squares += 1;
    if (sh instanceof Circle) circles += 1;
    if (sh instanceof Shape) shapesN += 1;
  }
  return rects * 1000 + squares * 100 + circles * 10 + shapesN;
}

/** Narrowing by `instanceof` and by `as`: fields and methods only a subclass has. */
export function demoDowncast(): number {
  const shapes: Shape[] = [new Rect(2, 3), new Square(4), new Circle(2)];
  let s = 0;
  for (const sh of shapes) {
    if (sh instanceof Square) {
      s += sh.onlyOnSquare();
    } else if (sh instanceof Circle) {
      s += sh.r;
    }
  }
  const first = shapes[0] as Rect;
  return s * 1000 + first.w * 10 + first.h;
}

export function demoSubclassAsBase(): number {
  const pick = (flag: boolean): Shape => (flag ? new Rect(1, 2) : new Circle(1));
  const both = flag3();
  return pick(true).area() * 100 + pick(false).area() * 10 + both.length;
}
function flag3(): Shape[] {
  const out: Shape[] = [];
  out.push(new Square(1));
  out.push(new Tri(1));
  return out.concat([new Circle(1)]);
}

export function demoTags(): number {
  return new Rect(1, 1).tag * 100 + new Square(1).tag * 10 + new Circle(1).tag + Shape.created * 0;
}

export function demoFieldRedeclared(): number {
  const b = new B();
  return b.x * 1000 + b.y * 10 + b.z;
}

/** The base constructor's call of an overridden method reaches the subclass's version. */
export function demoConstructorDispatch(): number {
  return new Base().v * 100 + new Derived().v;
}

export function demoImplicitConstructor(): number {
  const t = new Tri(5);
  return t.sides * 100 + t.corner * 10 + t.tag;
}

export function demoRecursionThroughBase(): number {
  const top = new Node3(1);
  top.child = new Node3(2);
  top.child.child = new Node2(3);
  return top.total();
}

export function demoCreatedCount(): number {
  const before = Shape.created;
  const a = new Rect(1, 1);
  const b = new Square(1);
  const c = new Circle(1);
  return Shape.created - before + a.sides + b.sides + c.sides;
}

/** Many objects of several classes, virtual calls and `instanceof` under allocation pressure. */
export function allocating(n: number): number {
  let total = 0;
  const keep: Shape[] = [];
  for (let i = 0; i < n; i++) {
    const s: Shape = i % 3 === 0 ? new Rect(i, 2) : i % 3 === 1 ? new Square(i) : new Circle(i);
    total += s.describe() + s.scaled(2);
    if (s instanceof Rect) total += s.w;
    if (i % 5 === 0) keep.push(s);
  }
  for (const k of keep) total += k.area();
  return total;
}

class ShapeHolder {
  s: Shape = new Circle(1);
}

function swapShape(h: ShapeHolder): number {
  h.s = new Rect(9, 9);
  const scratch = new Circle(0);
  return scratch.r + 1;
}

/** JS evaluates `h.s` (the old Circle) first; `swapShape` replaces it and allocates. 3 * 1. */
export function demoVirtualReceiverHeld(): number {
  const h = new ShapeHolder();
  return h.s.scaled(swapShape(h));
}

class Animal {
  sound(): number {
    return 1;
  }
}
class Dog extends Animal {
  override sound(): number {
    return 2;
  }
}

/** A method taken as a value from a base-typed variable still dispatches on the real class. */
export function demoBoundVirtual(): number {
  const a: Animal = new Dog();
  const b: Animal = new Animal();
  const fa = a.sound;
  const fb = b.sound;
  return fa() * 10 + fb();
}
