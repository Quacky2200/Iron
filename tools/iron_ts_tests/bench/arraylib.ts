// The array library: pop shift unshift slice splice concat reverse sort fill indexOf lastIndexOf
// findIndex at reduce, `length = n`, Array.from / of / isArray and `new Array(n)`.
export interface Item {
  v: number;
  order: number;
}

/** The length, then the digits: pack([4, 5]) is 245. */
function pack(xs: number[]): number {
  let s = xs.length;
  for (const x of xs) s = s * 10 + x;
  return s;
}

function item(v: number, order: number): Item {
  return { v, order };
}

export function popAndShift(): number {
  const a = [1, 2, 3, 4];
  const last = a.pop();
  const first = a.shift();
  const e: number[] = [];
  const none = e.pop();
  const noneShift = e.shift();
  return (
    pack(a) * 10000 + (last ?? 0) * 1000 + (first ?? 0) * 100 + (none === undefined ? 10 : 0) + (noneShift === undefined ? 1 : 0)
  );
}

export function unshifting(): number {
  const a = [5, 6];
  const n = a.unshift(7, 8);
  return pack(a) * 10 + n;
}

export function slicing(): number {
  const a = [1, 2, 3, 4, 5];
  return (
    pack(a.slice(1, 3)) * 1000000 +
    pack(a.slice(-2)) * 10000 +
    pack(a.slice(0, -1)) * 10 +
    pack(a.slice(7)) +
    pack(a.slice()) * 100000000
  );
}

export function splicing(): number {
  const a = [1, 2, 3, 4, 5];
  const removed = a.splice(1, 2);
  const r1 = pack(a) * 100 + pack(removed);
  a.splice(1, 0, 9, 9);
  const r2 = pack(a);
  a.splice(0, 1, 7);
  const r3 = pack(a);
  const tail = a.splice(-2);
  return r1 * 1000000000 + r2 * 1000000 + r3 * 1000 + pack(tail) * 10 + a.length;
}

export function concatenating(): number {
  const a = [1, 2];
  const b = a.concat([3], 4, [5, 6]);
  a.push(0);
  return pack(b) * 100 + pack(a);
}

export function reversing(): number {
  const a = [1, 2, 3, 4];
  const b = a.reverse();
  b.push(0);
  return pack(a);
}

export function sortingNumbers(): number {
  const a = [5, 3, 9, 1, 7, 3];
  a.sort((x, y) => x - y);
  const up = pack(a);
  a.sort((x, y) => y - x);
  return up * 10000000 + pack(a);
}

/** Stable: items with equal v keep their order. Allocates in the comparator. */
export function sortingObjects(): number {
  const xs: Item[] = [item(3, 0), item(1, 1), item(3, 2), item(2, 3), item(1, 4), item(3, 5)];
  xs.sort((a, b) => {
    const diff: Item = item(a.v - b.v, 0);
    return diff.v;
  });
  let s = 0;
  for (const x of xs) s = s * 10 + x.order;
  let t = 0;
  for (const x of xs) t = t * 10 + x.v;
  return s * 1000000 + t;
}

/** A longer array, made with a linear congruential generator, ends up in order. */
export function sortingMany(n: number): number {
  const xs: Item[] = [];
  let seed = 12345;
  for (let i = 0; i < n; i++) {
    seed = (seed * 1103515245 + 12345) % 2147483648;
    xs.push(item(seed % 1000, i));
  }
  xs.sort((a, b) => a.v - b.v);
  let bad = 0;
  for (let i = 1; i < xs.length; i++) {
    const p = xs[i - 1]!;
    const q = xs[i]!;
    if (p.v > q.v || (p.v === q.v && p.order > q.order)) bad += 1;
  }
  return bad * 100000 + xs[0]!.v * 1000 + xs[xs.length - 1]!.v;
}

export function filling(): number {
  const a = new Array<number>(5).fill(2);
  const b = [0, 0, 0, 0, 0];
  b.fill(9, 1, 3);
  const c = [1, 2, 3];
  c.fill(4, -1);
  return pack(a) * 1000000 + pack(b) * 1000 + pack(c);
}

export function searching(): number {
  const a = [3, 1, 4, 1, 5, 9, 2, 6];
  const items = [item(1, 0), item(5, 1), item(7, 2)];
  return (
    a.indexOf(1) * 100000 +
    (a.lastIndexOf(1) + 1) * 10000 +
    (a.indexOf(8) + 1) * 1000 +
    (a.findIndex((x) => x > 4) + 1) * 100 +
    (items.findIndex((it) => it.v === 7) + 1) * 10 +
    (a.includes(9) ? 1 : 0)
  );
}

export function atIndex(): number {
  const a = [10, 20, 30];
  const e: number[] = [];
  return (a.at(0) ?? 0) + (a.at(-1) ?? 0) * 100 + (a.at(-3) ?? 0) * 10000 + (a.at(5) === undefined ? 1000000 : 0) + (e.at(-1) === undefined ? 1 : 0);
}

export function reducing(): number {
  const a = [1, 2, 3, 4];
  const items = [item(2, 0), item(5, 1), item(3, 2)];
  const sum = a.reduce((acc, x) => acc + x, 0);
  const best = items.reduce((acc, it) => (it.v > acc ? it.v : acc), 0);
  const withIndex = a.reduce((acc, x, i) => acc + x * i, 0);
  const count = items.reduce((acc, it) => acc + (it.v > 2 ? 1 : 0), 0);
  return sum * 10000 + best * 1000 + withIndex * 10 + count;
}

export function lengthAssign(): number {
  const a = [1, 2, 3, 4, 5];
  a.length = 3;
  const first = pack(a);
  a.length = 0;
  a.push(8);
  return first * 100 + pack(a);
}

export function statics(): number {
  const squares = Array.from({ length: 5 }, (_, i) => i * i);
  const copy = Array.from(squares);
  copy[0] = 99;
  const of = Array.of(7, 8, 9);
  return pack(squares) * 100000000 + (Array.isArray(squares) ? 1 : 0) * 10000000 + pack(of) * 1000 + squares[0]! + copy[0]! * 0 + copy.length;
}

/** Array operations on arrays of objects, in a loop, under allocation pressure. */
export function allocating(n: number): number {
  let total = 0;
  for (let i = 0; i < n; i++) {
    const xs = [item(i % 5, 0), item(3, 1), item(i % 3, 2), item(1, 3)];
    xs.sort((a, b) => a.v - b.v);
    xs.splice(1, 1, item(7, 4), item(8, 5));
    const ys = xs.slice(1).concat([item(2, 6)]);
    ys.reverse();
    ys.unshift(item(9, 7));
    const top = ys.pop();
    total += ys.reduce((acc, it) => acc + it.v, 0) + (top === undefined ? 0 : top.v) + xs.length;
  }
  return total;
}

/** The comparator captures variables (an environment object only the closure holds) and allocates. */
export function sortingWithCapture(): number {
  const xs: Item[] = [item(3, 0), item(1, 1), item(2, 2), item(1, 3)];
  let calls = 0;
  const bonus = item(0, 9);
  xs.sort((a, b) => {
    calls += 1;
    const d = item(a.v - b.v + bonus.v, 0);
    return d.v;
  });
  return (calls > 0 ? 1 : 0) * 10000 + xs[0]!.order * 1000 + xs[1]!.order * 100 + xs[2]!.order * 10 + xs[3]!.order;
}
