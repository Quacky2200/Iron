// Module-level `let` variables: assigned from functions and closures, holding pointers, nullable
// values and closures. A `let` is never folded into its uses, even with a literal initialiser.
export interface Item {
  v: number;
}
export interface Node {
  v: number;
  next: Node | null;
}

let counter = 0;
let current: Item = { v: 1 };
let last: Item | null = null;
let best: number | null = null;
let pending: Item | undefined;
let unsetCount: number | undefined;
let handler: (n: number) => number = (n) => n;
let chain: Node | null = null;
const base = 5;

function bump(): number {
  counter += 1;
  return counter;
}

/** 1, 2, ++ to 3, a closure adds 10, forEach's callback adds 1+2+3: 19. Each call continues from the last. */
export function counterDemo(): number {
  bump();
  bump();
  counter++;
  const add = (n: number) => {
    counter += n;
  };
  add(10);
  [1, 2, 3].forEach((x) => {
    counter += x;
  });
  return counter;
}

export function counterValue(): number {
  return counter;
}

function sum(a: Item, b: number): number {
  return a.v + b;
}

/** Replaces the global, then allocates again so the old item is garbage unless someone holds it. */
function swapCurrent(v: number): number {
  current = { v };
  const scratch: Item = { v: 0 };
  return v + scratch.v;
}

/**
 * JS evaluates `current` first (the item with v = 1), then the call, which replaces `current`.
 * `sum` still gets the old item: 1 + 100 = 101.
 */
export function globalReplacedByACall(): number {
  return sum(current, swapCurrent(100));
}

/**
 * The same with the assignment written in the argument itself and no call in between:
 * `sum` gets the old item (v = 1) and the second argument is 7: 8.
 */
export function globalReplacedInTheArgument(): number {
  return sum(current, ((current = { v: 50 }), { v: 7 }.v));
}

export function currentValue(): number {
  return current.v;
}

export function remember(v: number): void {
  last = { v };
}

export function lastOr(d: number): number {
  const l = last;
  return l === null ? d : l.v;
}

export function forget(): void {
  last = null;
}

export function consider(x: number): void {
  if (best === null || x > best) {
    best = x;
  }
}

export function bestOr(d: number): number {
  const b = best;
  return b === null ? d : b;
}

/** `let pending: Item | undefined;` with no initialiser starts as null (undefined). */
export function pendingOr(d: number): number {
  const p = pending;
  if (p === undefined) {
    pending = { v: d };
    return -1;
  }
  return p.v;
}

/** `let unsetCount: number | undefined;` starts as null. */
export function unsetPlusOne(): number {
  const before = unsetCount ?? 0;
  unsetCount = before + 1;
  return before + 1;
}

/** A closure stored in a global, capturing a local of the function that made it. */
export function setHandler(k: number): void {
  handler = (n) => n * k + base;
}

export function callHandler(n: number): number {
  return handler(n);
}

/** A list that lives only in a global, grown across many allocations. */
export function growChain(n: number): number {
  let i = 0;
  while (i < n) {
    chain = { v: i, next: chain };
    i++;
  }
  let total = 0;
  let p = chain;
  while (p !== null) {
    total += p.v;
    p = p.next;
  }
  return total;
}
