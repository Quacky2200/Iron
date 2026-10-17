import { describe, expect, test } from "bun:test";

import { collapse, consoleRows, countLevels, type LogEntry, parseLinks } from "../src/console.ts";
import {
  activate,
  computeRects,
  type DockNode,
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
import { acceleratorLabel, CLOSED, findAccelerator, type Menu, menuKey, nextIndex, parseAccelerator } from "../src/menu.ts";
import { clickSelect, emptyTreeState, flatten, reveal, type TreeNode, type TreeState, treeKey } from "../src/tree.ts";
import { keyIndex, scrollForIndex, visibleRange } from "../src/virtual.ts";

const NO_MODS = { ctrl: false, shift: false, alt: false, meta: false };

describe("virtual list", () => {
  // M5-EDT-06
  test("only the rows in view plus the overscan are mounted", () => {
    expect(visibleRange(0, 100, 20, 1000, 0)).toEqual({ start: 0, end: 5 });
    expect(visibleRange(0, 100, 20, 1000, 3)).toEqual({ start: 0, end: 8 });
    // Half a row scrolled: the partly visible rows on both ends count.
    expect(visibleRange(210, 100, 20, 1000, 0)).toEqual({ start: 10, end: 16 });
    expect(visibleRange(19900, 100, 20, 1000, 3)).toEqual({ start: 992, end: 1000 });
    expect(visibleRange(0, 100, 20, 0)).toEqual({ start: 0, end: 0 });
    // A million rows still mount a handful.
    const r = visibleRange(10_000_000, 400, 20, 1_000_000, 4);
    expect(r.end - r.start).toBe(28);
  });

  // M5-EDT-06
  test("scrolling to an index moves only as far as needed, or aligns it", () => {
    // Row 10 is 200..220; the view shows 0..100.
    expect(scrollForIndex(10, 20, 100, 0, 1000)).toBe(120);
    expect(scrollForIndex(2, 20, 100, 120, 1000)).toBe(40);
    expect(scrollForIndex(7, 20, 100, 120, 1000)).toBe(120);
    expect(scrollForIndex(10, 20, 100, 0, 1000, "start")).toBe(200);
    expect(scrollForIndex(10, 20, 100, 0, 1000, "center")).toBe(160);
    // Clamped to the ends.
    expect(scrollForIndex(999, 20, 100, 0, 1000, "start")).toBe(19900);
    expect(scrollForIndex(0, 20, 100, 500, 1000, "end")).toBe(0);
  });

  // M5-EDT-07
  test("keys move the selection within the list", () => {
    expect(keyIndex("ArrowDown", -1, 10, 5)).toBe(0);
    expect(keyIndex("ArrowDown", 9, 10, 5)).toBe(9);
    expect(keyIndex("ArrowUp", 0, 10, 5)).toBe(0);
    expect(keyIndex("Home", 5, 10, 5)).toBe(0);
    expect(keyIndex("End", 5, 10, 5)).toBe(9);
    expect(keyIndex("PageDown", 0, 10, 5)).toBe(4);
    expect(keyIndex("PageUp", 9, 10, 5)).toBe(5);
    expect(keyIndex("a", 3, 10, 5)).toBeNull();
    expect(keyIndex("ArrowDown", 0, 0, 5)).toBeNull();
  });
});

const T = (id: string, children?: TreeNode[]): TreeNode => ({ id, label: id.toUpperCase(), children });
const ROOTS = [T("a", [T("a1"), T("a2", [T("a2x")])]), T("b"), T("c", [T("c1")])];
const ids = (state: TreeState) => [...state.selected].sort();

describe("tree view", () => {
  // M5-EDT-08
  test("flattening walks only into expanded nodes", () => {
    const rows = flatten(ROOTS, new Set(["a", "a2"]));
    expect(rows.map((r) => `${r.node.id}@${r.depth}`)).toEqual(["a@0", "a1@1", "a2@1", "a2x@2", "b@0", "c@0"]);
    expect(rows[0]!.expanded).toBe(true);
    expect(rows[5]!.hasChildren && !rows[5]!.expanded).toBe(true);
    expect(rows[3]!.parent).toBe("a2");
    // Deep trees don't overflow the stack.
    let deep: TreeNode = T("leaf");
    const open = new Set<string>();
    for (let i = 0; i < 20000; i++) {
      deep = T(`n${i}`, [deep]);
      open.add(`n${i}`);
    }
    expect(flatten([deep], open).length).toBe(20001);
  });

  // M5-EDT-08
  test("clicks select one, Ctrl toggles and Shift selects a range from the anchor", () => {
    const rows = flatten(ROOTS, new Set(["a"]));
    let s = clickSelect(rows, emptyTreeState(), "a1", {});
    expect(ids(s)).toEqual(["a1"]);
    s = clickSelect(rows, s, "c", { shift: true });
    expect(ids(s)).toEqual(["a1", "a2", "b", "c"]);
    expect(s.anchor).toBe("a1");
    // A second Shift-click re-ranges from the same anchor.
    s = clickSelect(rows, s, "a", { shift: true });
    expect(ids(s)).toEqual(["a", "a1"]);
    s = clickSelect(rows, s, "c", { ctrl: true });
    expect(ids(s)).toEqual(["a", "a1", "c"]);
    s = clickSelect(rows, s, "a", { ctrl: true });
    expect(ids(s)).toEqual(["a1", "c"]);
    // Ctrl+Shift adds a range to what's selected.
    s = clickSelect(rows, { ...s, anchor: "b" }, "c", { ctrl: true, shift: true });
    expect(ids(s)).toEqual(["a1", "b", "c"]);
  });

  // M5-EDT-09
  test("arrows, Home/End and left/right navigate, expand and collapse", () => {
    let s: TreeState = { ...emptyTreeState(), focus: "a" };
    const key = (k: string, mods = {}) => {
      const next = treeKey(flatten(ROOTS, s.expanded), s, k, mods, 4);
      expect(next).not.toBeNull();
      s = next!;
      return s.focus;
    };
    expect(key("ArrowRight")).toBe("a");
    expect(s.expanded.has("a")).toBe(true);
    expect(key("ArrowRight")).toBe("a1");
    expect(key("ArrowDown")).toBe("a2");
    expect(key("ArrowRight")).toBe("a2");
    expect(key("ArrowDown")).toBe("a2x");
    // Left on a leaf goes to its parent; on an open node, closes it.
    expect(key("ArrowLeft")).toBe("a2");
    expect(key("ArrowLeft")).toBe("a2");
    expect(s.expanded.has("a2")).toBe(false);
    expect(key("ArrowLeft")).toBe("a");
    expect(key("End")).toBe("c");
    expect(key("Home")).toBe("a");
    expect(key("PageDown")).toBe("b");
    expect(ids(s)).toEqual(["b"]);
    // Shift extends from the anchor; Ctrl moves focus only, Ctrl+Space toggles.
    expect(key("ArrowUp", { shift: true })).toBe("a2");
    expect(ids(s)).toEqual(["a2", "b"]);
    expect(key("ArrowUp", { ctrl: true })).toBe("a1");
    expect(ids(s)).toEqual(["a2", "b"]);
    key(" ", { ctrl: true });
    expect(ids(s)).toEqual(["a1", "a2", "b"]);
    expect(treeKey(flatten(ROOTS, s.expanded), s, "x")).toBeNull();
  });

  // M5-EDT-09
  test("revealing a node expands its ancestors", () => {
    const s = reveal(ROOTS, emptyTreeState(), "a2x");
    expect([...s.expanded].sort()).toEqual(["a", "a2"]);
    expect(reveal(ROOTS, s, "missing")).toBe(s);
  });
});

const LAYOUT = (): DockNode =>
  split("root", "row", [group("left", ["hierarchy"]), split("mid", "column", [group("scene", ["scene", "game"]), group("bottom", ["console"])], [0.7, 0.3]), group("right", ["inspector"])], [0.2, 0.6, 0.2]);

describe("docking", () => {
  // M5-EDT-10
  test("docking a tab on a group's middle adds it there and empties collapse", () => {
    const l = dockTab(LAYOUT(), "hierarchy", "scene", "center");
    expect(groupOf(l, "hierarchy")?.id).toBe("scene");
    expect(groupOf(l, "hierarchy")?.active).toBe("hierarchy");
    // The emptied left group is gone; the others share its space.
    const root = l as Extract<DockNode, { type: "split" }>;
    expect(root.children.map((c) => c.id)).toEqual(["mid", "right"]);
    expect(root.sizes[0]! + root.sizes[1]!).toBeCloseTo(1);
    expect(root.sizes[0]! / root.sizes[1]!).toBeCloseTo(3);
  });

  // M5-EDT-10
  test("docking on an edge splits the group, merging with a split the same way", () => {
    // Below the inspector: a new column split replaces it.
    let l = dockTab(LAYOUT(), "game", "right", "bottom");
    const right = (l as Extract<DockNode, { type: "split" }>).children[2]!;
    expect(right.type === "split" && right.direction).toBe("column");
    expect(right.type === "split" && right.children.map((c) => (c.type === "group" ? c.tabs : []))).toEqual([["inspector"], ["game"]]);
    expect(groupOf(l, "scene")?.tabs).toEqual(["scene"]);
    // Left of the scene group, inside a row: no new split, the root row gains a column.
    l = dockTab(LAYOUT(), "console", "left", "right");
    const root = l as Extract<DockNode, { type: "split" }>;
    expect(root.children.length).toBe(4);
    expect(root.children[1]?.type === "group" && root.children[1].tabs).toEqual(["console"]);
    expect(root.sizes[0]).toBeCloseTo(0.1);
    expect(root.sizes[1]).toBeCloseTo(0.1);
    // `mid` lost its only other child, so the scene group replaced it.
    expect(root.children[2]?.id).toBe("scene");
    // Splitting a group by its only tab does nothing.
    expect(dockTab(LAYOUT(), "inspector", "right", "left")).toEqual(LAYOUT());
  });

  // M5-EDT-10
  test("tabs reorder, activate and close", () => {
    let l = dockTab(LAYOUT(), "game", "scene", "center", 0);
    expect(groupOf(l, "game")?.tabs).toEqual(["game", "scene"]);
    l = activate(l, "scene");
    expect(groupOf(l, "scene")?.active).toBe("scene");
    l = removeTab(l, "scene");
    expect(groupOf(l, "game")?.active).toBe("game");
    // Removing everything leaves one empty group.
    for (const t of ["game", "hierarchy", "console", "inspector"]) l = removeTab(l, t);
    expect(l.type === "group" && l.tabs).toEqual([]);
  });

  // M5-EDT-11
  test("splitters resize neighbours within limits", () => {
    let l = resize(LAYOUT(), "root", 0, 0.1);
    let root = l as Extract<DockNode, { type: "split" }>;
    expect(root.sizes[0]).toBeCloseTo(0.3);
    expect(root.sizes[1]).toBeCloseTo(0.5);
    l = resize(l, "root", 0, -1);
    root = l as Extract<DockNode, { type: "split" }>;
    expect(root.sizes[0]).toBeCloseTo(0.05);
    expect(root.sizes[1]).toBeCloseTo(0.75);
  });

  // M5-EDT-12
  test("the layout saves to JSON and loads back the same", () => {
    const l = dockTab(LAYOUT(), "game", "right", "top");
    const json = serializeLayout(l);
    expect(parseLayout(json)).toEqual(l);
    expect(serializeLayout(parseLayout(json)!)).toBe(json);
    // Unknown tabs are dropped, and their empty group with them.
    const known = parseLayout(json, ["scene", "game", "console", "inspector"]);
    expect(groupOf(known!, "hierarchy")).toBeNull();
    expect(known && groupOf(known, "game")?.id).toBe(groupOf(l, "game")?.id);
    // Broken input gives null rather than a half layout.
    expect(parseLayout("{")).toBeNull();
    expect(parseLayout(JSON.stringify({ type: "split", id: "x", direction: "diagonal", children: [] }))).toBeNull();
    expect(parseLayout(JSON.stringify({ type: "group", id: "g", tabs: [1] }))).toBeNull();
  });

  // M5-EDT-11
  test("rects match the flex layout and drops find group and zone", () => {
    const rects = computeRects(LAYOUT(), { x: 0, y: 30, width: 1000 + 2 * SPLITTER, height: 600 });
    expect(rects.get("left")).toEqual({ x: 0, y: 30, width: 200, height: 600 });
    expect(rects.get("mid")?.x).toBeCloseTo(200 + SPLITTER);
    expect(rects.get("scene")?.height).toBeCloseTo((600 - SPLITTER) * 0.7);
    const l = LAYOUT();
    expect(dropTarget(l, rects, 100, 30 + TAB_BAR / 2)).toEqual({ group: "left", zone: "center" });
    expect(dropTarget(l, rects, 100, 300)).toEqual({ group: "left", zone: "center" });
    expect(dropTarget(l, rects, 10, 300)).toEqual({ group: "left", zone: "left" });
    expect(dropTarget(l, rects, 190, 300)).toEqual({ group: "left", zone: "right" });
    expect(dropTarget(l, rects, 100, 625)).toEqual({ group: "left", zone: "bottom" });
    expect(dropTarget(l, rects, 500, 1000)).toBeNull();
  });
});

const MENUS: Menu[] = [
  {
    label: "File",
    items: [
      { label: "New", accelerator: "Ctrl+N", action: () => {} },
      { separator: true },
      { label: "Recent", submenu: [{ label: "one", action: () => {} }, { label: "two", disabled: true }, { label: "three" }] },
      { label: "Gone", disabled: true },
      { label: "Save As", accelerator: "Ctrl+Shift+S", action: () => {} },
    ],
  },
  { label: "Edit", items: [{ label: "Undo", accelerator: "ctrl+z", action: () => {} }, { label: "Delete", accelerator: "Del", action: () => {} }] },
];

describe("menubar", () => {
  // M5-EDT-13
  test("accelerators parse, show and match key presses", () => {
    expect(parseAccelerator("Ctrl+Shift+S")).toEqual({ code: "KeyS", ctrl: true, shift: true, alt: false, meta: false });
    expect(parseAccelerator("F5")?.code).toBe("F5");
    expect(parseAccelerator("Ctrl++")?.code).toBe("Equal");
    expect(parseAccelerator("Ctrl+1")?.code).toBe("Digit1");
    expect(parseAccelerator("Ctrl+A+B")).toBeNull();
    expect(acceleratorLabel("ctrl+shift+s")).toBe("Ctrl+Shift+S");
    expect(findAccelerator(MENUS, "KeyZ", { ...NO_MODS, ctrl: true })?.label).toBe("Undo");
    expect(findAccelerator(MENUS, "KeyS", { ...NO_MODS, ctrl: true, shift: true })?.label).toBe("Save As");
    expect(findAccelerator(MENUS, "KeyS", { ...NO_MODS, ctrl: true })).toBeNull();
    expect(findAccelerator(MENUS, "Delete", NO_MODS)?.label).toBe("Delete");
  });

  // M5-EDT-14
  test("keys move through menus, skipping separators and disabled items", () => {
    const items = MENUS[0]!.items;
    expect(nextIndex(items, 0, 1)).toBe(2);
    expect(nextIndex(items, 2, 1)).toBe(4);
    expect(nextIndex(items, 4, 1)).toBe(0);
    expect(nextIndex(items, 0, -1)).toBe(4);
    let s = { top: 0, open: true, path: [0] };
    const key = (k: string) => {
      const r = menuKey(MENUS, s, k);
      s = r.state;
      return r;
    };
    key("ArrowDown");
    expect(s.path).toEqual([2]);
    // Right opens the submenu, Down skips its disabled item, Left goes back.
    key("ArrowRight");
    expect(s.path).toEqual([2, 0]);
    key("ArrowDown");
    expect(s.path).toEqual([2, 2]);
    key("ArrowLeft");
    expect(s.path).toEqual([2]);
    // Right on a plain item moves to the next menu; Left wraps back.
    key("ArrowDown");
    key("ArrowRight");
    expect(s).toEqual({ top: 1, open: true, path: [0] });
    key("ArrowLeft");
    expect(s.top).toBe(0);
    key("End");
    expect(s.path).toEqual([4]);
    const chosen = key("Enter");
    expect(chosen.choose?.label).toBe("Save As");
    expect(s).toEqual(CLOSED);
  });

  // M5-EDT-14
  test("Escape closes a level at a time", () => {
    let s = { top: 0, open: true, path: [2, 0] };
    s = menuKey(MENUS, s, "Escape").state;
    expect(s.path).toEqual([2]);
    s = menuKey(MENUS, s, "Escape").state;
    expect(s).toEqual({ top: 0, open: false, path: [] });
    // With the bar highlighted, Down opens the menu and Escape leaves the bar.
    expect(menuKey(MENUS, s, "ArrowDown").state).toEqual({ top: 0, open: true, path: [0] });
    expect(menuKey(MENUS, s, "Escape").state).toEqual(CLOSED);
    expect(menuKey(MENUS, CLOSED, "ArrowDown").handled).toBe(false);
  });
});

const entry = (seq: number, level: LogEntry["level"], message: string, target = "script"): LogEntry => ({ seq, level, message, target, time: seq });

describe("console", () => {
  // M5-EDT-18
  test("code links are found in messages", () => {
    expect(parseLinks("at main (src/main.ts:12:5) and src\\lib.rs:40")).toEqual([
      { text: "at main (" },
      { text: "src/main.ts:12:5", file: "src/main.ts", line: 12, col: 5 },
      { text: ") and " },
      { text: "src\\lib.rs:40", file: "src\\lib.rs", line: 40, col: 1 },
    ]);
    expect(parseLinks("C:\\Game\\a.tsx:3:1 failed")[0]).toEqual({ text: "C:\\Game\\a.tsx:3:1", file: "C:\\Game\\a.tsx", line: 3, col: 1 });
    expect(parseLinks("/home/me/game/x.js:7")[0]).toEqual({ text: "/home/me/game/x.js:7", file: "/home/me/game/x.js", line: 7, col: 1 });
    // Not links: URLs with ports, times, plain words.
    expect(parseLinks("see http://example.com:8080 at 12:30")).toEqual([{ text: "see http://example.com:8080 at 12:30" }]);
    expect(parseLinks("")).toEqual([]);
  });

  // M5-EDT-19
  test("repeats collapse with counts, and filters count and search", () => {
    const log = [entry(1, "info", "a"), entry(2, "warn", "b"), entry(3, "info", "a"), entry(4, "info", "a"), entry(5, "error", "a"), entry(6, "debug", "d", "iron_app")];
    expect(collapse(log).map((r) => `${r.entry.message}x${r.count}@${r.entry.seq}`)).toEqual(["ax3@1", "bx1@2", "ax1@5", "dx1@6"]);
    expect(countLevels(log)).toEqual({ info: 4, warn: 1, error: 1 });
    const all = { info: true, warn: true, error: true, search: "", collapse: false };
    expect(consoleRows(log, all).length).toBe(6);
    expect(consoleRows(log, { ...all, info: false }).map((r) => r.entry.seq)).toEqual([2, 5]);
    expect(consoleRows(log, { ...all, search: "B" }).map((r) => r.entry.seq)).toEqual([2]);
    expect(consoleRows(log, { ...all, search: "iron_" }).map((r) => r.entry.seq)).toEqual([6]);
    expect(consoleRows(log, { ...all, collapse: true }).map((r) => r.count)).toEqual([3, 1, 1, 1]);
  });
});
