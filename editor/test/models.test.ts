// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// The editor's models without React: the virtual list, the tree, docking, the menubar and the
// console (was js/editor-ui/test/models.test.ts under bun).

import { collapse, consoleRows, countLevels, type LogEntry, type LogLevel, parseLinks, type Segment } from "../src/console.ts";
import {
  activate,
  computeRects,
  type DockNode,
  type DockSplit,
  dockTab,
  dropTarget,
  group,
  groupOf,
  parseLayout,
  removeTab,
  resize,
  SPLITTER,
  serializeLayout,
  split,
  TAB_BAR,
} from "../src/dock.ts";
import { acceleratorLabel, CLOSED, findAccelerator, type Menu, type MenuState, menuKey, nextIndex, parseAccelerator } from "../src/menu.ts";
import { clickSelect, emptyTreeState, flatten, reveal, type TreeNode, type TreeState, treeKey } from "../src/tree.ts";
import { keyIndex, scrollForIndex, visibleRange } from "../src/virtual.ts";
import { close, eq, report, same, test, yes } from "./check.ts";

const NO_MODS = { ctrl: false, shift: false, alt: false, meta: false };

// M5-EDT-06
test("only the rows in view plus the overscan are mounted", () => {
  eq(visibleRange(0, 100, 20, 1000, 0), { start: 0, end: 5 }, "top");
  eq(visibleRange(0, 100, 20, 1000, 3), { start: 0, end: 8 }, "overscan");
  // Half a row scrolled: the partly visible rows on both ends count.
  eq(visibleRange(210, 100, 20, 1000, 0), { start: 10, end: 16 }, "half a row");
  eq(visibleRange(19900, 100, 20, 1000, 3), { start: 992, end: 1000 }, "end");
  eq(visibleRange(0, 100, 20, 0), { start: 0, end: 0 }, "empty");
  // A million rows still mount a handful.
  const r = visibleRange(10_000_000, 400, 20, 1_000_000, 4);
  eq(r.end - r.start, 28, "a million rows");
});

// M5-EDT-06
test("scrolling to an index moves only as far as needed, or aligns it", () => {
  // Row 10 is 200..220; the view shows 0..100.
  eq(scrollForIndex(10, 20, 100, 0, 1000), 120, "down");
  eq(scrollForIndex(2, 20, 100, 120, 1000), 40, "up");
  eq(scrollForIndex(7, 20, 100, 120, 1000), 120, "in view");
  eq(scrollForIndex(10, 20, 100, 0, 1000, "start"), 200, "start");
  eq(scrollForIndex(10, 20, 100, 0, 1000, "center"), 160, "center");
  // Clamped to the ends.
  eq(scrollForIndex(999, 20, 100, 0, 1000, "start"), 19900, "clamped end");
  eq(scrollForIndex(0, 20, 100, 500, 1000, "end"), 0, "clamped start");
});

// M5-EDT-07
test("keys move the selection within the list", () => {
  eq(keyIndex("ArrowDown", -1, 10, 5), 0, "down from none");
  eq(keyIndex("ArrowDown", 9, 10, 5), 9, "down at the end");
  eq(keyIndex("ArrowUp", 0, 10, 5), 0, "up at the top");
  eq(keyIndex("Home", 5, 10, 5), 0, "home");
  eq(keyIndex("End", 5, 10, 5), 9, "end");
  eq(keyIndex("PageDown", 0, 10, 5), 4, "page down");
  eq(keyIndex("PageUp", 9, 10, 5), 5, "page up");
  yes(keyIndex("a", 3, 10, 5) === null, "not a key");
  yes(keyIndex("ArrowDown", 0, 0, 5) === null, "no rows");
});

function T(id: string, children?: TreeNode<number>[]): TreeNode<number> {
  return { id, label: id.toUpperCase(), children };
}
const ROOTS: TreeNode<number>[] = [T("a", [T("a1"), T("a2", [T("a2x")])]), T("b"), T("c", [T("c1")])];
const ids = (state: TreeState): string[] => [...state.selected].sort();

// M5-EDT-08
test("flattening walks only into expanded nodes", () => {
  const rows = flatten(ROOTS, new Set(["a", "a2"]));
  eq(
    rows.map((r) => `${r.node.id}@${r.depth}`),
    ["a@0", "a1@1", "a2@1", "a2x@2", "b@0", "c@0"],
    "rows",
  );
  yes(rows[0]!.expanded, "a is open");
  yes(rows[5]!.hasChildren && !rows[5]!.expanded, "c is closed");
  eq(rows[3]!.parent, "a2", "parent");
  // Deep trees don't overflow the stack (flattening is a loop over a stack of its own; 5000 levels
  // keep the collect-at-every-allocation run short, bun's test had 20000).
  let deep: TreeNode<number> = T("leaf");
  const open = new Set<string>();
  for (let i = 0; i < 5000; i++) {
    deep = T(`n${i}`, [deep]);
    open.add(`n${i}`);
  }
  eq(flatten([deep], open).length, 5001, "deep");
});

// M5-EDT-08
test("clicks select one, Ctrl toggles and Shift selects a range from the anchor", () => {
  const rows = flatten(ROOTS, new Set(["a"]));
  let s = clickSelect(rows, emptyTreeState(), "a1", {});
  eq(ids(s), ["a1"], "click");
  s = clickSelect(rows, s, "c", { shift: true });
  eq(ids(s), ["a1", "a2", "b", "c"], "shift");
  eq(s.anchor, "a1", "anchor");
  // A second Shift-click re-ranges from the same anchor.
  s = clickSelect(rows, s, "a", { shift: true });
  eq(ids(s), ["a", "a1"], "re-range");
  s = clickSelect(rows, s, "c", { ctrl: true });
  eq(ids(s), ["a", "a1", "c"], "ctrl adds");
  s = clickSelect(rows, s, "a", { ctrl: true });
  eq(ids(s), ["a1", "c"], "ctrl removes");
  // Ctrl+Shift adds a range to what's selected.
  s = clickSelect(rows, { ...s, anchor: "b" }, "c", { ctrl: true, shift: true });
  eq(ids(s), ["a1", "b", "c"], "ctrl+shift");
});

// M5-EDT-09
test("arrows, Home/End and left/right navigate, expand and collapse", () => {
  let s: TreeState = { ...emptyTreeState(), focus: "a" };
  const key = (k: string, mods: { shift?: boolean; ctrl?: boolean } = {}): string | null => {
    const next = treeKey(flatten(ROOTS, s.expanded), s, k, mods, 4);
    yes(next !== null, `key ${k}`);
    if (next !== null) s = next;
    return s.focus;
  };
  eq(key("ArrowRight"), "a", "right opens");
  yes(s.expanded.has("a"), "a open");
  eq(key("ArrowRight"), "a1", "right enters");
  eq(key("ArrowDown"), "a2", "down");
  eq(key("ArrowRight"), "a2", "right opens a2");
  eq(key("ArrowDown"), "a2x", "down into a2");
  // Left on a leaf goes to its parent; on an open node, closes it.
  eq(key("ArrowLeft"), "a2", "left to the parent");
  eq(key("ArrowLeft"), "a2", "left closes");
  yes(!s.expanded.has("a2"), "a2 closed");
  eq(key("ArrowLeft"), "a", "left to a");
  eq(key("End"), "c", "end");
  eq(key("Home"), "a", "home");
  eq(key("PageDown"), "b", "page down");
  eq(ids(s), ["b"], "selected b");
  // Shift extends from the anchor; Ctrl moves focus only, Ctrl+Space toggles.
  eq(key("ArrowUp", { shift: true }), "a2", "shift up");
  eq(ids(s), ["a2", "b"], "extended");
  eq(key("ArrowUp", { ctrl: true }), "a1", "ctrl up");
  eq(ids(s), ["a2", "b"], "focus only");
  key(" ", { ctrl: true });
  eq(ids(s), ["a1", "a2", "b"], "ctrl+space");
  yes(treeKey(flatten(ROOTS, s.expanded), s, "x") === null, "not a key");
});

// M5-EDT-09
test("revealing a node expands its ancestors", () => {
  const s = reveal(ROOTS, emptyTreeState(), "a2x");
  eq([...s.expanded].sort(), ["a", "a2"], "ancestors");
  yes(reveal(ROOTS, s, "missing") === s, "missing leaves it as it is");
});

const LAYOUT = (): DockNode =>
  split(
    "root",
    "row",
    [group("left", ["hierarchy"]), split("mid", "column", [group("scene", ["scene", "game"]), group("bottom", ["console"])], [0.7, 0.3]), group("right", ["inspector"])],
    [0.2, 0.6, 0.2],
  );

/** The node as a split (the tests know it is one). */
function asSplit(n: DockNode | undefined): DockSplit {
  if (n === undefined || n.type !== "split") throw new Error("not a split");
  return n;
}

function tabsOf(n: DockNode): string[] {
  return n.type === "group" ? n.tabs : [];
}

// M5-EDT-10
test("docking a tab on a group's middle adds it there and empties collapse", () => {
  const l = dockTab(LAYOUT(), "hierarchy", "scene", "center");
  eq(groupOf(l, "hierarchy")?.id ?? "", "scene", "group");
  eq(groupOf(l, "hierarchy")?.active ?? "", "hierarchy", "active");
  // The emptied left group is gone; the others share its space.
  const root = asSplit(l);
  eq(
    root.children.map((c) => c.id),
    ["mid", "right"],
    "children",
  );
  close(root.sizes[0]! + root.sizes[1]!, 1, "sizes add up");
  close(root.sizes[0]! / root.sizes[1]!, 3, "ratio kept");
});

// M5-EDT-10
test("docking on an edge splits the group, merging with a split the same way", () => {
  // Below the inspector: a new column split replaces it.
  let l = dockTab(LAYOUT(), "game", "right", "bottom");
  const right = asSplit(asSplit(l).children[2]);
  eq(right.direction, "column", "direction");
  eq(
    right.children.map((c) => tabsOf(c)),
    [["inspector"], ["game"]],
    "split tabs",
  );
  eq(groupOf(l, "scene")?.tabs ?? [], ["scene"], "scene kept");
  // Left of the scene group, inside a row: no new split, the root row gains a column.
  l = dockTab(LAYOUT(), "console", "left", "right");
  const root = asSplit(l);
  eq(root.children.length, 4, "four columns");
  eq(tabsOf(root.children[1]!), ["console"], "console column");
  close(root.sizes[0]!, 0.1, "left size");
  close(root.sizes[1]!, 0.1, "console size");
  // `mid` lost its only other child, so the scene group replaced it.
  eq(root.children[2]?.id ?? "", "scene", "scene replaced mid");
  // Splitting a group by its only tab does nothing.
  same(serializeLayout(dockTab(LAYOUT(), "inspector", "right", "left")), serializeLayout(LAYOUT()), "only tab");
});

// M5-EDT-10
test("tabs reorder, activate and close", () => {
  let l = dockTab(LAYOUT(), "game", "scene", "center", 0);
  eq(groupOf(l, "game")?.tabs ?? [], ["game", "scene"], "reordered");
  l = activate(l, "scene");
  eq(groupOf(l, "scene")?.active ?? "", "scene", "activated");
  l = removeTab(l, "scene");
  eq(groupOf(l, "game")?.active ?? "", "game", "active after close");
  // Removing everything leaves one empty group.
  for (const t of ["game", "hierarchy", "console", "inspector"]) l = removeTab(l, t);
  yes(l.type === "group" && tabsOf(l).length === 0, "one empty group");
});

// M5-EDT-11
test("splitters resize neighbours within limits", () => {
  let l = resize(LAYOUT(), "root", 0, 0.1);
  let root = asSplit(l);
  close(root.sizes[0]!, 0.3, "grown");
  close(root.sizes[1]!, 0.5, "shrunk");
  l = resize(l, "root", 0, -1);
  root = asSplit(l);
  close(root.sizes[0]!, 0.05, "minimum");
  close(root.sizes[1]!, 0.75, "rest");
});

// M5-EDT-12
test("the layout saves to JSON and loads back the same", () => {
  const l = dockTab(LAYOUT(), "game", "right", "top");
  const json = serializeLayout(l);
  const back = parseLayout(json);
  yes(back !== null, "parses");
  if (back !== null) same(serializeLayout(back), json, "round trip");
  // Unknown tabs are dropped, and their empty group with them.
  const known = parseLayout(json, ["scene", "game", "console", "inspector"]);
  yes(known !== null && groupOf(known, "hierarchy") === null, "unknown tab dropped");
  if (known !== null) eq(groupOf(known, "game")?.id ?? "", groupOf(l, "game")?.id ?? "-", "same group");
  // Broken input gives null rather than a half layout.
  yes(parseLayout("{") === null, "broken JSON");
  yes(parseLayout('{"type":"split","id":"x","direction":"diagonal","children":[]}') === null, "bad direction");
  yes(parseLayout('{"type":"group","id":"g","tabs":[1]}') === null, "tab not a string");
});

// M5-EDT-11
test("rects match the flex layout and drops find group and zone", () => {
  const rects = computeRects(LAYOUT(), { x: 0, y: 30, width: 1000 + 2 * SPLITTER, height: 600 });
  eq(rects.get("left") ?? null, { x: 0, y: 30, width: 200, height: 600 }, "left");
  close(rects.get("mid")?.x ?? -1, 200 + SPLITTER, "mid x");
  close(rects.get("scene")?.height ?? -1, (600 - SPLITTER) * 0.7, "scene height");
  const l = LAYOUT();
  eq(dropTarget(l, rects, 100, 30 + TAB_BAR / 2), { group: "left", zone: "center" }, "tab strip");
  eq(dropTarget(l, rects, 100, 300), { group: "left", zone: "center" }, "middle");
  eq(dropTarget(l, rects, 10, 300), { group: "left", zone: "left" }, "left edge");
  eq(dropTarget(l, rects, 190, 300), { group: "left", zone: "right" }, "right edge");
  eq(dropTarget(l, rects, 100, 625), { group: "left", zone: "bottom" }, "bottom edge");
  yes(dropTarget(l, rects, 500, 1000) === null, "outside");
});

const nothing = (): void => {};
const MENUS: Menu[] = [
  {
    label: "File",
    items: [
      { label: "New", accelerator: "Ctrl+N", action: nothing },
      { separator: true },
      { label: "Recent", submenu: [{ label: "one", action: nothing }, { label: "two", disabled: true }, { label: "three" }] },
      { label: "Gone", disabled: true },
      { label: "Save As", accelerator: "Ctrl+Shift+S", action: nothing },
    ],
  },
  {
    label: "Edit",
    items: [
      { label: "Undo", accelerator: "ctrl+z", action: nothing },
      { label: "Delete", accelerator: "Del", action: nothing },
    ],
  },
];

// M5-EDT-13
test("accelerators parse, show and match key presses", () => {
  eq(parseAccelerator("Ctrl+Shift+S"), { code: "KeyS", ctrl: true, shift: true, alt: false, meta: false }, "parse");
  eq(parseAccelerator("F5")?.code ?? "", "F5", "F5");
  eq(parseAccelerator("Ctrl++")?.code ?? "", "Equal", "plus");
  eq(parseAccelerator("Ctrl+1")?.code ?? "", "Digit1", "digit");
  yes(parseAccelerator("Ctrl+A+B") === null, "two keys");
  same(acceleratorLabel("ctrl+shift+s"), "Ctrl+Shift+S", "label");
  eq(findAccelerator(MENUS, "KeyZ", { ...NO_MODS, ctrl: true })?.label ?? "", "Undo", "undo");
  eq(findAccelerator(MENUS, "KeyS", { ...NO_MODS, ctrl: true, shift: true })?.label ?? "", "Save As", "save as");
  yes(findAccelerator(MENUS, "KeyS", { ...NO_MODS, ctrl: true }) === null, "no Ctrl+S");
  eq(findAccelerator(MENUS, "Delete", NO_MODS)?.label ?? "", "Delete", "delete");
});

// M5-EDT-14
test("keys move through menus, skipping separators and disabled items", () => {
  const items = MENUS[0]!.items;
  eq(nextIndex(items, 0, 1), 2, "skip separator");
  eq(nextIndex(items, 2, 1), 4, "skip disabled");
  eq(nextIndex(items, 4, 1), 0, "wrap");
  eq(nextIndex(items, 0, -1), 4, "wrap back");
  let s: MenuState = { top: 0, open: true, path: [0] };
  const key = (k: string): string => {
    const r = menuKey(MENUS, s, k);
    s = r.state;
    return r.choose?.label ?? "";
  };
  key("ArrowDown");
  eq(s.path, [2], "down");
  // Right opens the submenu, Down skips its disabled item, Left goes back.
  key("ArrowRight");
  eq(s.path, [2, 0], "submenu");
  key("ArrowDown");
  eq(s.path, [2, 2], "skip disabled");
  key("ArrowLeft");
  eq(s.path, [2], "back");
  // Right on a plain item moves to the next menu; Left wraps back.
  key("ArrowDown");
  key("ArrowRight");
  eq(s, { top: 1, open: true, path: [0] }, "next menu");
  key("ArrowLeft");
  eq(s.top, 0, "previous menu");
  key("End");
  eq(s.path, [4], "end");
  same(key("Enter"), "Save As", "chosen");
  eq(s, CLOSED, "closed");
});

// M5-EDT-14
test("Escape closes a level at a time", () => {
  let s: MenuState = { top: 0, open: true, path: [2, 0] };
  s = menuKey(MENUS, s, "Escape").state;
  eq(s.path, [2], "one level");
  s = menuKey(MENUS, s, "Escape").state;
  eq(s, { top: 0, open: false, path: [] }, "bar highlighted");
  // With the bar highlighted, Down opens the menu and Escape leaves the bar.
  eq(menuKey(MENUS, s, "ArrowDown").state, { top: 0, open: true, path: [0] }, "down opens");
  eq(menuKey(MENUS, s, "Escape").state, CLOSED, "escape leaves");
  yes(!menuKey(MENUS, CLOSED, "ArrowDown").handled, "closed ignores keys");
});

function entry(seq: number, level: LogLevel, message: string, target = "script"): LogEntry {
  return { seq, level, message, target, time: seq };
}

/** A segment as text, `[file:line:col]` for a link. */
function seg(s: Segment): string {
  return "file" in s ? `[${s.text}|${s.file}|${s.line}|${s.col}]` : s.text;
}

// M5-EDT-18
test("code links are found in messages", () => {
  eq(
    parseLinks("at main (src/main.ts:12:5) and src\\lib.rs:40").map(seg),
    ["at main (", "[src/main.ts:12:5|src/main.ts|12|5]", ") and ", "[src\\lib.rs:40|src\\lib.rs|40|1]"],
    "links",
  );
  same(seg(parseLinks("C:\\Game\\a.tsx:3:1 failed")[0]!), "[C:\\Game\\a.tsx:3:1|C:\\Game\\a.tsx|3|1]", "drive");
  same(seg(parseLinks("/home/me/game/x.js:7")[0]!), "[/home/me/game/x.js:7|/home/me/game/x.js|7|1]", "absolute");
  // Not links: URLs with ports, times, plain words.
  eq(parseLinks("see http://example.com:8080 at 12:30").map(seg), ["see http://example.com:8080 at 12:30"], "not links");
  eq(parseLinks("").length, 0, "empty");
});

// M5-EDT-19
test("repeats collapse with counts, and filters count and search", () => {
  const log = [entry(1, "info", "a"), entry(2, "warn", "b"), entry(3, "info", "a"), entry(4, "info", "a"), entry(5, "error", "a"), entry(6, "debug", "d", "platypus_app")];
  eq(
    collapse(log).map((r) => `${r.entry.message}x${r.count}@${r.entry.seq}`),
    ["ax3@1", "bx1@2", "ax1@5", "dx1@6"],
    "collapsed",
  );
  eq(countLevels(log), { info: 4, warn: 1, error: 1 }, "counts");
  const all = { info: true, warn: true, error: true, search: "", collapse: false };
  eq(consoleRows(log, all).length, 6, "all");
  eq(
    consoleRows(log, { ...all, info: false }).map((r) => r.entry.seq),
    [2, 5],
    "no info",
  );
  eq(
    consoleRows(log, { ...all, search: "B" }).map((r) => r.entry.seq),
    [2],
    "search message",
  );
  eq(
    consoleRows(log, { ...all, search: "platypus_" }).map((r) => r.entry.seq),
    [6],
    "search target",
  );
  eq(
    consoleRows(log, { ...all, collapse: true }).map((r) => r.count),
    [3, 1, 1, 1],
    "collapsed counts",
  );
});

export function run(): string {
  return report();
}
