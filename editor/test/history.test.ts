// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// Undo and redo (editor/src/history.ts).

import {
  canRedo,
  canUndo,
  clearHistory,
  type Command,
  createHistory,
  group,
  isDirty,
  markSaved,
  perform,
  record,
  redo,
  redoLabel,
  undo,
  undoLabel,
} from "../src/history.ts";
import { eq, report, same, test, yes } from "./check.ts";

/** A command that sets `state.value` and remembers the old one. */
function setter(state: { value: number }, to: number, merge?: string): Command {
  const before = state.value;
  return {
    label: `Set ${to}`,
    run: () => {
      state.value = to;
    },
    undo: () => {
      state.value = before;
    },
    merge,
  };
}

// M5-UNDO-02
test("commands run, undo and redo in order", () => {
  const h = createHistory();
  const s = { value: 0 };
  perform(h, setter(s, 1));
  perform(h, setter(s, 2));
  eq(s.value, 2, "after two");
  same(undoLabel(h), "Set 2", "undo label");
  same(undo(h), "Set 2", "undone");
  eq(s.value, 1, "after undo");
  same(redoLabel(h), "Set 2", "redo label");
  same(undo(h), "Set 1", "undone again");
  eq(s.value, 0, "back to the start");
  same(undo(h), "", "nothing left");
  yes(!canUndo(h) && canRedo(h), "only redo");
  redo(h);
  redo(h);
  eq(s.value, 2, "redone");
  same(redo(h), "", "nothing to redo");
  // A new command drops what was undone.
  undo(h);
  perform(h, setter(s, 5));
  yes(!canRedo(h), "redo dropped");
  eq(s.value, 5, "the new value");
});

// M5-UNDO-03
test("a drag's commands merge into one entry; recorded commands don't run again", () => {
  const h = createHistory();
  const s = { value: 0 };
  perform(h, setter(s, 1, "drag"));
  perform(h, setter(s, 2, "drag"));
  perform(h, setter(s, 3, "drag"));
  eq(h.done.length, 1, "one entry");
  undo(h);
  eq(s.value, 0, "undone to before the drag");
  redo(h);
  eq(s.value, 3, "redone to its end");
  // `record` keeps a command that already happened.
  s.value = 7;
  let runs = 0;
  record(h, {
    label: "Moved",
    run: () => {
      runs++;
      s.value = 7;
    },
    undo: () => {
      s.value = 3;
    },
  });
  eq(runs, 0, "not run when recorded");
  undo(h);
  eq(s.value, 3, "undone");
});

// M5-UNDO-04
test("the saved position says whether there are changes, and the size is capped", () => {
  const h = createHistory(3);
  const s = { value: 0 };
  yes(!isDirty(h), "clean at first");
  perform(h, setter(s, 1));
  yes(isDirty(h), "dirty after a change");
  markSaved(h);
  yes(!isDirty(h), "clean after saving");
  // Merging doesn't fold into the saved entry.
  perform(h, setter(s, 2, "m"));
  perform(h, setter(s, 3, "m"));
  eq(h.done.length, 2, "merged after the save");
  undo(h);
  yes(!isDirty(h), "clean again at the saved point");
  redo(h);
  perform(h, setter(s, 4));
  perform(h, setter(s, 5));
  eq(h.done.length, 3, "capped");
  // The first entry fell off; the saved state is now the oldest one kept.
  for (let i = 0; i < 3; i++) undo(h);
  eq(s.value, 1, "back to the saved value");
  yes(!isDirty(h), "clean there");
  // One more change past the cap drops the saved state itself.
  for (let i = 0; i < 3; i++) redo(h);
  perform(h, setter(s, 6));
  for (let i = 0; i < 3; i++) undo(h);
  yes(isDirty(h), "dirty with the saved state gone");
  clearHistory(h);
  yes(!isDirty(h) && !canUndo(h) && !canRedo(h), "cleared");
});

// M5-UNDO-05
test("a group runs in order and undoes in reverse", () => {
  const h = createHistory();
  const log: string[] = [];
  const step = (name: string): Command => ({
    label: name,
    run: () => {
      log.push(`+${name}`);
    },
    undo: () => {
      log.push(`-${name}`);
    },
  });
  perform(h, group("Both", [step("a"), step("b")]));
  undo(h);
  eq(log, ["+a", "+b", "-b", "-a"], "order");
  same(redo(h), "Both", "label");
});

export function run(): string {
  return report();
}
