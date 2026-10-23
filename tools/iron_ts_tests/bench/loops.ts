// SPDX-License-Identifier: Apache-2.0

// for(;;), do...while, labelled break/continue and labelled blocks.
export interface Item {
  v: number;
}

/** 0 + 1 + ... + (n - 1). */
export function sumTo(n: number): number {
  let s = 0;
  for (let i = 0; i < n; i++) {
    s += i;
  }
  return s;
}

/** `continue` must run the update clause, or this never ends. Sums the odd numbers below n. */
export function sumOdd(n: number): number {
  let s = 0;
  for (let i = 0; i < n; i++) {
    if (i % 2 === 0) continue;
    s += i;
  }
  return s;
}

/** Two declarators, a comma update and a break: pairs (i, j) meet in the middle. */
export function converge(): number {
  let steps = 0;
  for (let i = 0, j = 10; ; i++, j--) {
    if (i >= j) break;
    steps += j - i;
  }
  return steps;
}

/** No test and no update. */
export function firstSquareOver(limit: number): number {
  let n = 1;
  for (;;) {
    if (n * n > limit) break;
    n += 1;
  }
  return n;
}

/** The variable declared before the loop and a missing initialiser. */
export function countDown(from: number): number {
  let i = from;
  let ticks = 0;
  for (; i > 0; i -= 3) {
    ticks += 1;
  }
  return ticks * 100 + i;
}

/** The body runs once even though the test is false at the start. */
export function atLeastOnce(): number {
  let n = 10;
  let runs = 0;
  do {
    runs += 1;
    n += 1;
  } while (n < 5);
  return runs * 100 + n;
}

/** `continue` in a do...while goes to the test: counts 1..6 but adds only the even ones. */
export function doContinue(): number {
  let i = 0;
  let s = 0;
  do {
    i += 1;
    if (i % 2 === 1) continue;
    s += i;
  } while (i < 6);
  return s * 100 + i;
}

/** `continue outer` skips the rest of the inner loop and still runs outer's update. */
export function labelledContinue(): number {
  let count = 0;
  outer: for (let i = 0; i < 5; i++) {
    for (let j = 0; j < 5; j++) {
      if (j > i) continue outer;
      count += 1;
    }
  }
  return count;
}

/** `break outer` leaves both loops: the first pair with i * j = 12 is (3, 4). */
export function labelledBreak(): number {
  let found = 0;
  outer: for (let i = 1; i < 10; i++) {
    for (let j = 1; j < 10; j++) {
      if (i * j === 12) {
        found = i * 10 + j;
        break outer;
      }
    }
  }
  return found;
}

/** A labelled while and a labelled for-of. */
export function labelledWhileAndForOf(xs: number[]): number {
  let total = 0;
  let k = 0;
  loop: while (k < 100) {
    k += 1;
    for (const x of xs) {
      if (x < 0) break loop;
      if (x === 0) continue loop;
      total += x;
    }
  }
  return total * 1000 + k;
}

/** `name: { ... break name; ... }` skips the rest of the block. */
export function labelledBlock(x: number): number {
  let r = 1;
  found: {
    if (x > 5) {
      r = 2;
      break found;
    }
    r = 3;
  }
  return r;
}

/** Each iteration gets its own `i`: the closures see 0, 1 and 2, not 3, 3, 3. */
export function closuresSeeEachIteration(): number {
  const fns: (() => number)[] = [];
  for (let i = 0; i < 3; i++) {
    fns.push(() => i);
  }
  return fns[0]!() * 100 + fns[1]!() * 10 + fns[2]!();
}

/** The body changes a captured loop variable; the next iteration starts from the changed value. */
export function closureChangesTheLoopVariable(): number {
  let seen = 0;
  for (let i = 0; i < 10; i++) {
    const bump = () => {
      i += 3;
    };
    bump();
    seen += 1;
  }
  return seen;
}

/** Allocates in the head, the body and the update, so a collection can land anywhere in the loop. */
export function allocatingLoop(n: number): number {
  let total = 0;
  for (let item: Item = { v: 1 }, k = 0; k < n; item = { v: item.v + 1 }, k++) {
    const scratch: Item = { v: item.v * 2 };
    total += scratch.v - item.v;
  }
  return total;
}
