// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// The engine's log for the editor: `platypus_editor` sends new records to `__platypus_log`
// each frame once `editor.logHistory` asked for them (everything kept so far first).

import { op } from "@platypus/engine";

import { type LogEntry, levelOf } from "./console.ts";

/** The most entries kept; older ones drop off. */
export const LOG_LIMIT = 10000;

let entries: LogEntry[] = [];
// The newest record seen: history sent again (after a reload) isn't added twice.
let lastSeq = 0;
const listeners = new Set<() => void>();
let started = false;

declare global {
  /** New log records, flat: `[seq, level, target, message, timeMs, ...]` (platypus_editor only). */
  var __platypus_log: (flat: unknown[]) => void;
}

globalThis.__platypus_log = (flat: unknown[]) => {
  const added: LogEntry[] = [];
  for (let i = 0; i + 4 < flat.length; i += 5) {
    const seq = Number(flat[i]);
    if (seq <= lastSeq) continue;
    lastSeq = seq;
    added.push({
      seq,
      level: levelOf(Number(flat[i + 1])),
      target: String(flat[i + 2]),
      message: String(flat[i + 3]),
      time: Number(flat[i + 4]),
    });
  }
  if (added.length) {
    entries = entries.concat(added);
    if (entries.length > LOG_LIMIT) entries = entries.slice(entries.length - LOG_LIMIT);
    for (const fn of listeners) fn();
  }
};

/** Asks the engine for the log (once). */
export function startLog(): void {
  if (started) return;
  started = true;
  op("editor.logHistory");
}

/** The entries so far (a new array after each change, for `useSyncExternalStore`). */
export const logEntries = (): LogEntry[] => entries;

export function subscribeLog(fn: () => void): () => void {
  startLog();
  listeners.add(fn);
  return () => {
    listeners.delete(fn);
  };
}

/** Empties the console (the log file keeps everything). */
export function clearLog(): void {
  entries = [];
  for (const fn of listeners) fn();
}

/** Opens a file at a line in the user's code editor (`code -g`, or `PLATYPUS_EDITOR_OPEN`). */
export function openInEditor(file: string, line = 1, col = 1): void {
  op("editor.openFile", file, line, col);
}

/** For tests. */
export function _resetLogForTests(): void {
  entries = [];
  lastSeq = 0;
  started = false;
  listeners.clear();
}
