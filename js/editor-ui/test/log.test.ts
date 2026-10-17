import { beforeEach, expect, test } from "bun:test";

import { _resetLogForTests, clearLog, logEntries, openInEditor, subscribeLog } from "../src/log.ts";

let ops: unknown[][] = [];
beforeEach(() => {
  ops = [];
  (globalThis as { __iron_op?: unknown }).__iron_op = (...args: unknown[]) => ops.push(args);
  _resetLogForTests();
});

// M5-EDT-20
test("the log feed asks for history once and skips records it has seen", () => {
  let changes = 0;
  const stop = subscribeLog(() => changes++);
  subscribeLog(() => {});
  expect(ops).toEqual([["editor.logHistory"]]);
  globalThis.__iron_log([1, 2, "script", "hello", 5, 2, 3, "iron_app", "careful", 6]);
  expect(logEntries().map((e) => `${e.seq}:${e.level}:${e.target}:${e.message}`)).toEqual(["1:info:script:hello", "2:warn:iron_app:careful"]);
  // History sent again after a reload: only the new record is added.
  globalThis.__iron_log([1, 2, "script", "hello", 5, 2, 3, "iron_app", "careful", 6, 3, 4, "script", "bad", 7]);
  expect(logEntries().map((e) => e.level)).toEqual(["info", "warn", "error"]);
  expect(changes).toBe(2);
  clearLog();
  expect(logEntries()).toEqual([]);
  globalThis.__iron_log([3, 4, "script", "bad", 7]);
  expect(logEntries()).toEqual([]);
  stop();
  openInEditor("src/main.ts", 12, 5);
  expect(ops[1]).toEqual(["editor.openFile", "src/main.ts", 12, 5]);
});
