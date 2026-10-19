// The order in which real React runs effects and cleanups, for the native reconciler to match
// (crates/iron_ui_native/tests/effects.rs holds the same scenarios and expected logs).
import { beforeEach, expect, test } from "bun:test";
import { useEffect, useState } from "react";

import { resetForTests } from "../src/host.ts";
import { _resetRootForTests, createRoot } from "../src/index.ts";

let log: string[] = [];
beforeEach(() => {
  log = [];
  (globalThis as { __iron_commit?: unknown }).__iron_commit = () => {};
  resetForTests();
  _resetRootForTests();
});

function Eff({ name, n }: { name: string; n: number }) {
  useEffect(() => {
    log.push(`${name}+${n}`);
    return () => log.push(`${name}-${n}`);
  }, [n]);
  return null;
}

function P({ n, kids }: { n: number; kids: string[] }) {
  useEffect(() => {
    log.push(`P+${n}`);
    return () => log.push(`P-${n}`);
  }, [n]);
  return (
    <>
      {kids.map((k) => (
        <Eff key={k} name={k} n={n} />
      ))}
    </>
  );
}

function Nest({ n }: { n: number }) {
  useEffect(() => {
    log.push(`outer+${n}`);
    return () => log.push(`outer-${n}`);
  }, [n]);
  return <Inner n={n} />;
}

function Inner({ n }: { n: number }) {
  useEffect(() => {
    log.push(`inner+${n}`);
    return () => log.push(`inner-${n}`);
  }, [n]);
  return null;
}

test("effect order across mount, update, removal and unmount", () => {
  const root = createRoot();
  const steps: string[][] = [];
  const step = (f: () => void) => {
    log = [];
    f();
    steps.push(log);
  };
  step(() => root.render(<P n={0} kids={["a", "b"]} />));
  step(() => root.render(<P n={1} kids={["a", "b"]} />));
  step(() => root.render(<P n={1} kids={["a"]} />));
  step(() => root.render(<Nest n={0} />));
  step(() => root.render(<Nest n={1} />));
  step(() => root.render(<P n={2} kids={[]} />));
  step(() => root.unmount());
  expect(steps).toEqual(EXPECTED);
});

const EXPECTED: string[][] = [
  ["a+0", "b+0", "P+0"],
  ["a-0", "b-0", "P-0", "a+1", "b+1", "P+1"],
  ["b-1"],
  ["P-1", "a-1", "inner+0", "outer+0"],
  ["inner-0", "outer-0", "inner+1", "outer+1"],
  ["outer-1", "inner-1", "P+2"],
  ["P-2"],
];
