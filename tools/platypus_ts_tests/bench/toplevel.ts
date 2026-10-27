// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// Top-level control-flow statements (`if`, `for`, `while`, `switch`, `try`, blocks, labels) run in
// source order with the declarations around them. Every export returns a string.
import { depLog, depTotal } from "./toplevel_dep.ts";

const log: string[] = [];
let counter = 0;
const base = 10;

function mark(s: string): void {
  log.push(s);
}

mark("start");
if (base > 5) {
  mark("big");
} else {
  mark("small");
}
const squares: number[] = [];
for (let i = 0; i < 5; i++) {
  squares.push(i * i);
}
let walked = 0;
let n = 0;
while (n < 4) {
  walked += n;
  n++;
}
let k = 0;
do {
  k += 3;
} while (k < 10);
switch (base) {
  case 5:
    mark("five");
    break;
  case 10:
    mark("ten");
  // falls through
  default:
    mark("default");
}
try {
  throw new Error("boom");
} catch (e) {
  mark("caught");
} finally {
  mark("finally");
}
{
  const x = 3;
  let y = x + 1;
  counter += y;
}
{
  const x = 100;
  counter += x;
}
if (later(2) === 4) mark("later");
mark("after-later");
const afterBlocks = counter + 1;
for (const s of ["a", "b"]) {
  mark(s);
}
let found = -1;
outer: for (let i = 0; i < 5; i++) {
  for (let j = 0; j < 5; j++) {
    if (i * j === 6) {
      found = i * 10 + j;
      break outer;
    }
  }
}
let late = 0;
if (afterBlocks > 100) {
  late = afterBlocks;
}
mark("end");

function later(v: number): number {
  return v * 2;
}

export function topLog(): string {
  return log.join(",");
}
export function topSquares(): string {
  return squares.join(",");
}
export function topNumbers(): string {
  return [walked, k, counter, afterBlocks, found, late].join(",");
}
export function topImported(): string {
  return depLog.join(",") + ":" + depTotal;
}
