// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// The engine's log for the editor: once a console subscribes, each frame reads the records it
// hasn't seen with `Editor.logSince` (everything the host kept so far first).

import { type LogEntry, levelOf } from "./console.ts";
import { onEditorFrame } from "./frame.ts";

/** The most entries kept; older ones drop off. */
export const LOG_LIMIT = 10000;

let entries: LogEntry[] = [];
// The newest record seen: nothing is added twice.
let lastSeq = 0;
const listeners = new Set<() => void>();
let started = false;

/** Adds new log records (ones at or below the newest seen are skipped). */
export function addRecords(records: readonly LogRecord[]): void {
  const added: LogEntry[] = [];
  for (const r of records) {
    if (r.seq <= lastSeq) continue;
    lastSeq = r.seq;
    added.push({ seq: r.seq, level: levelOf(r.level), target: r.target, message: r.message, time: r.timeMs });
  }
  if (added.length > 0) {
    entries = entries.concat(added);
    if (entries.length > LOG_LIMIT) entries = entries.slice(entries.length - LOG_LIMIT);
    for (const fn of [...listeners]) fn();
  }
}

/** Starts reading the engine's log every frame (once). */
export function startLog(): void {
  if (started) return;
  started = true;
  onEditorFrame(() => addRecords(Editor.logSince(lastSeq)));
}

/** The entries so far (a new array after each change, for `useSyncExternalStore`). */
export const logEntries = (): LogEntry[] => entries;

/** Calls `fn` after each change, reading the engine's log from now on. */
export function subscribeLog(fn: () => void): () => void {
  startLog();
  return listenLog(fn);
}

/** Calls `fn` after each change (the feed itself is `startLog`'s). */
export function listenLog(fn: () => void): () => void {
  listeners.add(fn);
  return () => {
    listeners.delete(fn);
  };
}

/** Empties the console (the log file keeps everything). */
export function clearLog(): void {
  entries = [];
  for (const fn of [...listeners]) fn();
}

/** Opens a file at a line in the user's code editor (`code -g`, or `PLATYPUS_EDITOR_OPEN`). */
export function openInEditor(file: string, line = 1, col = 1): void {
  Editor.openFile(file, line, col);
}

/** For tests. */
export function resetLogForTests(): void {
  entries = [];
  lastSeq = 0;
  listeners.clear();
}
