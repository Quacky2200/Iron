// SPDX-License-Identifier: Apache-2.0

import { afterEach, beforeEach, describe, expect, test } from "bun:test";

import { resetForTests } from "../src/host.ts";
import { _resetRootForTests, createRoot } from "../src/index.ts";
import type { Op } from "../src/opcodes.ts";

let commits: { ops: Op[]; target: number | undefined }[] = [];

beforeEach(() => {
  commits = [];
  (globalThis as { __platypus_commit?: unknown }).__platypus_commit = (ops: Op[], target?: number) => {
    commits.push({ ops, target });
  };
  resetForTests();
  _resetRootForTests();
});
afterEach(() => {
  delete (globalThis as { __platypus_commit?: unknown }).__platypus_commit;
});

describe("createRoot of a window", () => {
  // M9-TS-2820
  test("createRoot of a window commits to the window target", () => {
    const win = { handle: 3 };
    createRoot(win).render(<box />);
    expect(commits.length).toBe(1);
    expect(commits[0].target).toBe(0x4000_0003);
    // A second root for the same window names it in the error; another window is fine.
    expect(() => createRoot(win)).toThrow("window 3 already has a root");
    createRoot({ handle: 4 }).render(<box />);
    expect(commits[1].target).toBe(0x4000_0004);
  });

  // M9-TS-2821
  test("createRoot of the main window is the screen root", () => {
    createRoot({ handle: 1 }).render(<box />);
    expect(commits.length).toBe(1);
    // The screen's commits carry no target.
    expect(commits[0].target).toBeUndefined();
    expect(() => createRoot()).toThrow("only be called once");
  });

  test("world panels still take an object with an id or a number", () => {
    createRoot({ id: 7 }).render(<box />);
    createRoot(8).render(<box />);
    expect(commits.map((c) => c.target)).toEqual([7, 8]);
  });
});
