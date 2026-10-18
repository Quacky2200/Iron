// Two programs a collector with a sloppy root analysis gets wrong. In JavaScript both are fine
// because an argument that has been evaluated stays alive until the call; the native code has to
// keep it alive too.
export interface Item {
  v: number;
}
export interface Holder {
  item: Item;
}

function sum(a: Item, b: number): number {
  return a.v + b;
}

function fresh(v: number): Item {
  return { v };
}

/** Overwrites `h.item` and then allocates again, so the old item is garbage unless someone holds it. */
function replaceItem(h: Holder, v: number): number {
  h.item = { v };
  const scratch: Item = { v: 0 };
  return v + scratch.v;
}

export function makeHolder(v: number): Holder {
  return { item: { v } };
}

/**
 * JS evaluates `h.item` first (the old item, say v = 5), then the call, which replaces `h.item`.
 * `sum` still gets the old item: 5 + 100 = 105.
 */
export function fieldHeldAcrossStore(h: Holder): number {
  return sum(h.item, replaceItem(h, 100));
}

/**
 * JS evaluates `a` first (the item with v = 5), then reassigns `a` while evaluating the second
 * argument. `sum` gets the item `a` had before: 5 + 1 = 6.
 */
export function localReassigned(): number {
  let a = fresh(5);
  return sum(a, (a = fresh(100), fresh(1).v));
}

/** Overwrites `h.item`, then allocates: the new object takes the freed old item's memory. */
function swapAndAlloc(h: Holder): Item {
  h.item = { v: 9 };
  const x: Item = { v: 0 };
  return x;
}

/**
 * JS: `h.item` is the old item and `swapAndAlloc` returns a different, new object, so `===` is
 * false. Natively the old item is freed and the new object may be given the same address.
 */
export function sameAsOld(h: Holder): boolean {
  return h.item === swapAndAlloc(h);
}
