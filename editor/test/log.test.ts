// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// The console's log feed (was js/editor-ui/test/log.test.ts): records from `Editor.logSince`
// are added once, in order, and clearing empties the console.

import { addRecords, clearLog, listenLog, logEntries, resetLogForTests } from "../src/log.ts";
import { eq, report, test } from "./check.ts";

function rec(seq: number, level: number, target: string, message: string): LogRecord {
  return { seq, level, target, message, timeMs: seq * 10 };
}

// M5-EDT-20
test("the log feed adds records once, in order, and clearing empties it", () => {
  resetLogForTests();
  let changes = 0;
  // The feed (`startLog`, `Editor.logSince` each frame) needs the host's frame callbacks.
  const stop = listenLog(() => changes++);
  addRecords([rec(1, 2, "script", "hello"), rec(2, 3, "platypus_app", "careful")]);
  eq(
    logEntries().map((e) => `${e.seq}:${e.level}:${e.target}:${e.message}:${e.time}`),
    ["1:info:script:hello:10", "2:warn:platypus_app:careful:20"],
    "entries",
  );
  // Records seen already are skipped (a feed read again from the start).
  addRecords([rec(1, 2, "script", "hello"), rec(2, 3, "platypus_app", "careful"), rec(3, 4, "script", "bad")]);
  eq(
    logEntries().map((e) => e.level),
    ["info", "warn", "error"],
    "levels",
  );
  eq(changes, 2, "listener calls");
  clearLog();
  eq(logEntries().length, 0, "cleared");
  addRecords([rec(3, 4, "script", "bad")]);
  eq(logEntries().length, 0, "old records stay out after a clear");
  stop();
  addRecords([rec(4, 0, "script", "trace")]);
  eq(changes, 3, "the clear told the listener; after stop, nothing");
  eq(logEntries()[0]?.level ?? "", "debug", "trace is debug");
});

export function run(): string {
  return report();
}
