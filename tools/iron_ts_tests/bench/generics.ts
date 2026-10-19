// Generics by monomorphisation: functions, classes, interfaces and aliases with type parameters.
// Every exported function without parameters returns a string; `gen_generics_expected.ts` runs
// this in bun.

function showN(n: number | null): string {
  if (n === null) return "null";
  return String(n);
}

function showS(s: string | null): string {
  if (s === null) return "null";
  return s;
}

function identity<T>(x: T): T {
  return x;
}

function first<T>(xs: T[]): T | null {
  return xs.length > 0 ? xs[0] : null;
}

function mapAll<T, U>(xs: T[], f: (x: T) => U): U[] {
  const out: U[] = [];
  for (const x of xs) out.push(f(x));
  return out;
}

function pair<A, B>(a: A, b: B): Pair<A, B> {
  return { first: a, second: b };
}

function fill<T>(n: number, value: T): T[] {
  const out: T[] = [];
  for (let i = 0; i < n; i++) out.push(value);
  return out;
}

function reduce<T, A>(xs: T[], init: A, f: (acc: A, x: T) => A): A {
  let acc = init;
  for (const x of xs) acc = f(acc, x);
  return acc;
}

interface Pair<A, B> {
  first: A;
  second: B;
}

type Maybe<T> = T | null;

class Box<T> {
  value: T;
  constructor(value: T) {
    this.value = value;
  }
  get(): T {
    return this.value;
  }
  set(v: T): void {
    this.value = v;
  }
  map<U>(f: (x: T) => U): Box<U> {
    return new Box<U>(f(this.value));
  }
}

class Stack<T> {
  items: T[] = [];
  push(x: T): void {
    this.items.push(x);
  }
  pop(): T | null {
    return this.items.length > 0 ? (this.items.pop() as T) : null;
  }
  get size(): number {
    return this.items.length;
  }
}

class Node<T> {
  value: T;
  next: Node<T> | null = null;
  constructor(value: T) {
    this.value = value;
  }
}

class Named<T> extends Box<T> {
  name: string;
  constructor(name: string, value: T) {
    super(value);
    this.name = name;
  }
  describe(): string {
    return this.name + "=" + String(this.get());
  }
}

export function identityTypes(): string {
  const a = identity(5);
  const b = identity("hello");
  const c = identity<boolean>(true);
  const d = identity([1, 2, 3]);
  return [String(a + 1), String(b.length), String(c ? "y" : "n"), String(d.length)].join(",");
}

export function firstOf(): string {
  const a = first([4, 5, 6]);
  const b = first(["x", "y"]);
  const c = first<number>([]);
  return [showN(a), showS(b), showN(c)].join(",");
}

export function mapping(): string {
  const a = mapAll([1, 2, 3], (x) => x * 2);
  const b = mapAll(["a", "bb", "ccc"], (s) => s.length);
  const c = mapAll([1, 2], (n) => "n" + n);
  return [String(a.join("|")), String(b.join("|")), String(c.join("|"))].join(";");
}

export function pairs(): string {
  const p = pair(1, "one");
  const q = pair("k", true);
  const r: Pair<number, number> = { first: 3, second: 4 };
  return [String(p.first), String(p.second), String(q.first), String(q.second ? "t" : "f"), String(r.first + r.second)].join(",");
}

export function fills(): string {
  return [String(fill(3, 7).join("")), String(fill(2, "ab").join("-")), String(fill(2, true).length)].join(",");
}

export function reducing(): string {
  const sum = reduce([1, 2, 3, 4], 0, (a, x) => a + x);
  const text = reduce(["a", "b", "c"], "", (a, x) => a + x);
  const lens = reduce(["aa", "b"], 0, (a, x) => a + x.length);
  return [String(sum), String(text), String(lens)].join(",");
}

export function boxes(): string {
  const a = new Box(41);
  a.set(a.get() + 1);
  const b = new Box<string>("s");
  b.set(b.get() + "!");
  const c = a.map((x) => x * 2);
  const d = b.map((s) => s.length);
  return [String(a.get()), String(b.get()), String(c.get()), String(d.get())].join(",");
}

export function stacks(): string {
  const s = new Stack<number>();
  s.push(1);
  s.push(2);
  s.push(3);
  const t = new Stack<string>();
  t.push("x");
  const top = s.pop();
  const none = new Stack<number>().pop();
  return [String(s.size), showN(top), String(t.size), showN(none)].join(",");
}

export function lists(): string {
  const head = new Node(1);
  head.next = new Node(2);
  head.next.next = new Node(3);
  let n: Node<number> | null = head;
  let sum = 0;
  while (n !== null) {
    sum += n.value;
    n = n.next;
  }
  const words = new Node("a");
  words.next = new Node("b");
  return [String(sum), String(words.value + (words.next === null ? "" : words.next.value))].join(",");
}

export function subclass(): string {
  const a = new Named("x", 5);
  const b = new Named<string>("y", "z");
  a.set(a.get() + 1);
  return [String(a.describe()), String(b.describe()), String(a instanceof Box ? "box" : "no")].join(",");
}

export function aliases(): string {
  const a: Maybe<number> = 5;
  const b: Maybe<string> = null;
  return [showN(a), showS(b)].join(",");
}

export function nested(): string {
  const bb = new Box(new Box(3));
  const inner = bb.get();
  const arr = new Box([1, 2, 3]);
  arr.get().push(4);
  return [String(inner.get()), String(arr.get().length), String(identity(identity(9)))].join(",");
}
