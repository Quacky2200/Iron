// Plain classes: fields, constructors, methods, getters and setters, statics, private members.
export class Vec2 {
  constructor(
    readonly x: number = 0,
    readonly y: number = 0,
  ) {}
  static readonly zero = new Vec2(0, 0);
  static created = 0;
  static of(x: number, y: number): Vec2 {
    Vec2.created += 1;
    return new Vec2(x, y);
  }
  add(o: Vec2): Vec2 {
    return new Vec2(this.x + o.x, this.y + o.y);
  }
  scale(k: number): Vec2 {
    return new Vec2(this.x * k, this.y * k);
  }
  dot(o: Vec2): number {
    return this.x * o.x + this.y * o.y;
  }
  get length(): number {
    return Math.sqrt(this.dot(this));
  }
}

export class Counter {
  #count = 0;
  step = 1;
  label?: number;
  private history: number[] = [];
  constructor(start: number) {
    this.#count = start;
    this.history.push(start);
  }
  increment(): Counter {
    this.#count += this.step;
    this.history.push(this.#count);
    return this;
  }
  #bump(n: number): void {
    this.#count += n;
  }
  bumpTwice(n: number): void {
    this.#bump(n);
    this.#bump(n);
  }
  get value(): number {
    return this.#count;
  }
  set value(v: number) {
    this.#count = v;
    this.history.push(v);
  }
  get historyLength(): number {
    return this.history.length;
  }
  /** `this` captured by an arrow function. */
  withArrow(): number {
    const add = (n: number) => this.#count + n;
    return add(10);
  }
  /** `this` inside an inlined array callback. */
  sumHistory(): number {
    let s = 0;
    this.history.forEach((h) => {
      s += h + this.step;
    });
    return s;
  }
}

class Cell {
  next: Cell | null = null;
  constructor(public v: number) {}
}

export class Stack {
  head: Cell | null = null;
  size = 0;
  push(v: number): void {
    const c = new Cell(v);
    c.next = this.head;
    this.head = c;
    this.size += 1;
  }
  pop(): number {
    const h = this.head;
    if (h === null) return -1;
    this.head = h.next;
    this.size -= 1;
    return h.v;
  }
  sum(): number {
    let s = 0;
    let p = this.head;
    while (p !== null) {
      s += p.v;
      p = p.next;
    }
    return s;
  }
}

/** Initialisers run in order, after the parameter properties, before the constructor body. */
class Order {
  a = 1;
  b: number = this.a + 1;
  c: number;
  constructor(c: number) {
    this.c = this.b + c;
  }
}

class Early {
  x = 0;
  constructor(flag: boolean) {
    this.x = 1;
    if (flag) return;
    this.x = 2;
  }
}

class Box {
  constructor(public v: number) {}
  plus(n: number): number {
    return this.v + n;
  }
}

class Holder {
  item: Box;
  constructor() {
    this.item = new Box(5);
  }
}

function swap(h: Holder): number {
  h.item = new Box(100);
  const s = new Box(0);
  return s.v + 1;
}

export function demoVec(): number {
  const a = new Vec2(3, 4);
  const b = Vec2.of(1, 2);
  const c = a.add(b).scale(2);
  return c.x * 1000 + c.y * 100 + a.length * 10 + Vec2.created + Vec2.zero.x;
}

export function demoDefaults(): number {
  return new Vec2().x + new Vec2(7).x * 10 + new Vec2(1, 2).y * 100;
}

export function demoCounter(): number {
  const c = new Counter(5);
  c.increment().increment();
  const afterIncrements = c.value;
  c.value = 20;
  c.bumpTwice(3);
  const label = c.label === undefined ? 1 : 0;
  c.label = 4;
  return (
    afterIncrements * 100000000 +
    c.value * 1000000 +
    c.withArrow() * 10000 +
    c.historyLength * 1000 +
    c.sumHistory() * 10 +
    label +
    (c.label ?? 0) * 0
  );
}

export function demoStack(n: number): number {
  const s = new Stack();
  for (let i = 1; i <= n; i++) s.push(i);
  const top = s.pop();
  return s.sum() * 10000 + top * 100 + s.size;
}

export function demoOrder(): number {
  const o = new Order(10);
  return o.a * 10000 + o.b * 1000 + o.c;
}

export function demoEarly(): number {
  return new Early(true).x * 10 + new Early(false).x;
}

/** JS evaluates `h.item` (the old box) first; the call replaces it and allocates. 5 + 1. */
export function receiverHeld(): number {
  const h = new Holder();
  return h.item.plus(swap(h));
}

/** The same object seen through two names, and identity. */
export function identity(): number {
  const a = new Box(1);
  const b = a;
  b.v = 9;
  const c = new Box(9);
  return a.v * 100 + (a === b ? 10 : 0) + (a === c ? 1 : 0);
}

export function allocating(n: number): number {
  let total = 0;
  const s = new Stack();
  for (let i = 0; i < n; i++) {
    const v = Vec2.of(i, i + 1).add(new Vec2(1, 1)).scale(2);
    s.push(v.x + v.y);
    if (i % 3 === 0) total += s.pop();
  }
  return total + s.sum() * 1000 + s.size;
}

export class Greeter {
  n = 0;
  add: (k: number) => number = (k) => {
    this.n += k;
    return this.n;
  };
  twice(k: number): number {
    return k * 2;
  }
  triple(k: number): number {
    return k * 3;
  }
  bindTwice(): (k: number) => number {
    return this.twice;
  }
  apply(xs: number[]): number[] {
    return xs.map(this.twice);
  }
}

/** Methods as values (they do not use `this`, so plain JavaScript agrees), and an arrow function field. */
export function demoBound(): number {
  const g = new Greeter();
  const f = g.bindTwice();
  const t = g.triple;
  g.add(3);
  const add = g.add;
  add(4);
  return f(5) * 1000 + t(2) * 100 + g.n * 10 + g.apply([1, 2, 3]).length;
}
