// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// Generators, part 2: throw(), return() running finally blocks, return values.
// Every exported function without parameters returns a string; `gen_generators2_expected.ts` runs
// this file in bun.

let log = "";

function* guarded(): Generator<number> {
  try {
    yield 1;
    yield 2;
  } catch (e) {
    log += "caught:" + (e as Error).message + ";";
    yield 99;
  } finally {
    log += "fin;";
  }
  yield 3;
}

function* plain(): Generator<number> {
  try {
    yield 1;
    yield 2;
  } finally {
    log += "plain-fin;";
  }
}

function* swallower(): Generator<number> {
  try {
    yield 1;
    yield 2;
  } catch (e) {
    log += "swallowed;";
  } finally {
    log += "swallower-fin;";
  }
}

function* finallyYields(): Generator<number> {
  try {
    yield 1;
  } finally {
    yield 50;
    log += "after-50;";
  }
}

function* withReturn(): Generator<number, number> {
  yield 1;
  yield 2;
  return 42;
}

function* earlyReturn(n: number): Generator<number, number> {
  for (let i = 0; i < 10; i++) {
    if (i === n) return i * 100;
    yield i;
  }
  return -1;
}

function show(r: IteratorResult<number>): string {
  return (r.done ? "D" : "N") + r.value;
}

export function throwCaught(): string {
  log = "";
  const g = guarded();
  const a = show(g.next());
  const b = show(g.throw(new Error("boom")));
  const c = show(g.next());
  const d = String(g.next().done);
  return [a, b, c, d].join(",") + "|" + log;
}

export function throwEscapes(): string {
  log = "";
  const g = plain();
  g.next();
  let msg = "none";
  try {
    g.throw(new Error("out"));
  } catch (e) {
    msg = (e as Error).message;
  }
  const after = g.next();
  return msg + "|" + log + "|" + after.done;
}

export function throwBeforeStart(): string {
  log = "";
  const g = plain();
  let msg = "none";
  try {
    g.throw(new Error("early"));
  } catch (e) {
    msg = (e as Error).message;
  }
  const after = g.next();
  return msg + "|" + log + "|" + after.done;
}

export function returnRunsFinally(): string {
  log = "";
  const g = plain();
  const a = show(g.next());
  const r = g.return(7);
  const after = g.next();
  return a + "," + show(r) + "," + after.done + "|" + log;
}

export function returnBeforeStart(): string {
  log = "";
  const g = plain();
  const r = g.return(5);
  const after = g.next();
  return show(r) + "," + after.done + "|" + log;
}

export function returnNotSwallowed(): string {
  log = "";
  const g = swallower();
  g.next();
  const r = g.return(3);
  return show(r) + "|" + log;
}

export function returnFinallyYields(): string {
  log = "";
  const g = finallyYields();
  const a = show(g.next());
  const b = show(g.return(9));
  const c = show(g.next());
  return [a, b, c].join(",") + "|" + log;
}

export function returnValues(): string {
  const g = withReturn();
  const out: string[] = [];
  for (let i = 0; i < 3; i++) out.push(show(g.next()));
  out.push(String(g.next().done));
  return out.join(",");
}

export function earlyReturnValue(): string {
  const a = earlyReturn(2);
  const b = earlyReturn(20);
  const out: string[] = [];
  for (let i = 0; i < 3; i++) out.push(show(a.next()));
  for (let i = 0; i < 11; i++) out.push(show(b.next()));
  return out.join(",");
}
