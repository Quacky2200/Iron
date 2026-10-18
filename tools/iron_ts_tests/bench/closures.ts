// Closures: functions created in script, capturing variables. Every exported function returns a
// number so the test can compare it with what bun computes from this same file.
export interface Counter {
  inc: () => number;
  get: () => number;
}

export interface Item {
  v: number;
}

/** One variable, shared and mutated by a closure and the function that made it. */
export function counterDemo(): number {
  let count = 0;
  const inc = (): number => {
    count += 1;
    return count;
  };
  inc();
  inc();
  return inc() * 10 + count;
}

function makeAdder(n: number): (x: number) => number {
  return (x: number) => x + n;
}

/** Two closures from one function keep separate captured values. */
export function addersDemo(): number {
  const a = makeAdder(10);
  const b = makeAdder(100);
  return a(1) + b(1) * 1000;
}

function makeCounter(start: number): Counter {
  let n = start;
  return {
    inc: () => {
      n += 1;
      return n;
    },
    get: () => n,
  };
}

/** Two closures over one variable see each other's writes; another counter is independent. */
export function counterObjectDemo(): number {
  const c = makeCounter(5);
  c.inc();
  c.inc();
  const d = makeCounter(100);
  d.inc();
  return c.get() * 1000 + d.get();
}

/** A `let`/`const` declared inside a loop is a new variable every iteration. */
export function perIterationDemo(): number {
  const fs: Array<() => number> = [];
  for (const k of [1, 2, 3]) {
    fs.push(() => k * k);
  }
  let total = 0;
  for (const f of fs) total = total * 100 + f();
  return total;
}

export function whileDemo(): number {
  const fs: Array<() => number> = [];
  let i = 0;
  while (i < 3) {
    const j = i * 10;
    fs.push(() => j + 1);
    i += 1;
  }
  return fs[0]!() + fs[1]!() * 100 + fs[2]!() * 10000;
}

function double(x: number): number {
  return x * 2;
}

function apply(f: (x: number) => number, x: number): number {
  return f(x);
}

/** A top-level function as a value, as a callback, next to an inlined arrow callback. */
export function functionValueDemo(): number {
  const xs = [1, 2, 3];
  const ys = xs.map(double);
  const k = 5;
  const zs = xs.map((x) => x + k);
  return apply(double, 21) * 1000000 + ys[2]! * 10000 + zs[0]! * 100 + zs[2]!;
}

/** A closure that calls itself. */
export function recursiveDemo(): number {
  const fact: (n: number) => number = (n) => (n <= 1 ? 1 : n * fact(n - 1));
  return fact(6);
}

function makeBump(item: Item): () => number {
  return () => {
    item.v += 1;
    return item.v;
  };
}

/** A closure over an object changes the object everyone else holds. */
export function captureObjectDemo(): number {
  const it: Item = { v: 1 };
  const f = makeBump(it);
  f();
  f();
  return it.v * 10 + f();
}

/** A closure made inside an inlined array callback captures the callback's parameter. */
export function nestedDemo(): number {
  const adders = [1, 2, 3].map((x) => (y: number) => x + y);
  return adders[0]!(10) + adders[1]!(20) * 100 + adders[2]!(30) * 10000;
}

/** Immediately called. */
export function iifeDemo(): number {
  const base = 40;
  return (() => base + 2)();
}

function compose(f: (x: number) => number, g: (x: number) => number): (x: number) => number {
  return (x) => g(f(x));
}

/** Closures as arguments, returned, and called through a call result: two fresh closures wait for each other. */
export function composeDemo(): number {
  return compose(makeAdder(1), makeAdder(2))(10);
}

// ---- for the host, and for the collector ----

export function makeAccumulator(start: number): () => number {
  let total = start;
  return () => {
    total += 1;
    return total;
  };
}

export function makeItemReader(v: number): () => number {
  const it: Item = { v };
  return () => it.v;
}

/** Allocates garbage so the collector has work between calls of a closure. */
export function churn(n: number): number {
  let s = 0;
  let i = 0;
  while (i < n) {
    const o: Item = { v: i };
    const fs: Array<() => number> = [() => o.v];
    s += fs[0]!();
    i += 1;
  }
  return s;
}
