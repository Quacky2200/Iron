// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// Undo and redo: a list of commands with a position in it. A command knows how to do and undo
// itself; consecutive commands with the same merge key (one handle drag, one slider scrub) fold
// into one entry. The saved position says whether the document has unsaved changes. Plain
// functions over a `History` record, tested without React.

export interface Command {
  label: string;
  run: () => void;
  undo: () => void;
  /** Folds into the previous command when both have this key (the first's undo, this one's run). */
  merge?: string;
}

export interface History {
  done: Command[];
  undone: Command[];
  /** How many entries `done` had when last saved (-1: a state no entry leads back to). */
  saved: number;
  limit: number;
  /** Bumped on every change, for listeners. */
  version: number;
}

/** The most entries kept by default; older ones drop off. */
export const HISTORY_LIMIT = 200;

export function createHistory(limit: number = HISTORY_LIMIT): History {
  return { done: [], undone: [], saved: 0, limit, version: 0 };
}

/** Runs a command and records it (dropping what was undone). */
export function perform(h: History, command: Command): void {
  command.run();
  record(h, command);
}

/** Records a command that has already happened (a finished handle drag). */
export function record(h: History, command: Command): void {
  h.undone = [];
  const last = h.done[h.done.length - 1];
  // Merging into the saved entry would lose the saved state.
  if (last && command.merge !== undefined && last.merge === command.merge && h.saved !== h.done.length) {
    const first = last;
    h.done[h.done.length - 1] = { label: first.label, run: command.run, undo: first.undo, merge: command.merge };
  } else {
    h.done.push(command);
    if (h.done.length > h.limit) {
      h.done.shift();
      // The saved state fell off the end: nothing leads back to it.
      h.saved = h.saved > 0 ? h.saved - 1 : -1;
    }
  }
  h.version++;
}

/** Undoes the last command; its label, or "" when there was nothing to undo. */
export function undo(h: History): string {
  const c = h.done.pop();
  if (!c) return "";
  c.undo();
  h.undone.push(c);
  h.version++;
  return c.label;
}

/** Does the last undone command again; its label, or "". */
export function redo(h: History): string {
  const c = h.undone.pop();
  if (!c) return "";
  c.run();
  h.done.push(c);
  h.version++;
  return c.label;
}

export const canUndo = (h: History): boolean => h.done.length > 0;
export const canRedo = (h: History): boolean => h.undone.length > 0;
export const undoLabel = (h: History): string => h.done[h.done.length - 1]?.label ?? "";
export const redoLabel = (h: History): string => h.undone[h.undone.length - 1]?.label ?? "";

/** Whether there are changes since the last save. */
export const isDirty = (h: History): boolean => h.saved !== h.done.length;

/** The document was saved as it is now. */
export function markSaved(h: History): void {
  h.saved = h.done.length;
  h.version++;
}

/** Forgets everything (a new or opened document), clean. */
export function clearHistory(h: History): void {
  h.done = [];
  h.undone = [];
  h.saved = 0;
  h.version++;
}

/** Several commands as one entry: run in order, undone in reverse. */
export function group(label: string, commands: Command[]): Command {
  return {
    label,
    run: () => {
      for (const c of commands) c.run();
    },
    undo: () => {
      for (let i = commands.length - 1; i >= 0; i--) commands[i]!.undo();
    },
  };
}
