import { beforeEach, expect, test } from "bun:test";

import { dispatchUiEvent } from "../../react/src/events.ts";
import { instances, resetForTests } from "../../react/src/host.ts";
import { _resetRootForTests, createRoot } from "../../react/src/index.ts";
import type { Menu } from "../src/menu.ts";
import { MenuBar } from "../src/MenuBar.tsx";

beforeEach(() => {
  (globalThis as { __iron_commit?: unknown }).__iron_commit = () => {};
  (globalThis as { __iron_op?: unknown }).__iron_op = () => {};
  resetForTests();
  _resetRootForTests();
});
// Hover updates render a tick later (continuous priority).
const tick = () => new Promise((resolve) => setTimeout(resolve, 5));

/** Live element ids with a class, in creation order. */
const byClass = (name: string) =>
  [...instances.values()].filter((i) => String((i.props as { className?: string }).className ?? "").split(" ").includes(name)).map((i) => i.id);
const POINTER = [0, 0, 0, 1, 1, 0, 0, 0];

// M5-EDT-14
test("the menubar opens on press, switches on hover, runs items and closes on Escape", async () => {
  const ran: string[] = [];
  const menus: Menu[] = [
    { label: "File", items: [{ label: "New", action: () => ran.push("new") }, { label: "Open", action: () => ran.push("open") }] },
    { label: "Edit", items: [{ label: "Undo", action: () => ran.push("undo") }] },
  ];
  const root = createRoot();
  root.render(<MenuBar menus={menus} />);
  const [bar] = byClass("iron-menubar");
  const [row] = byClass("iron-menubar-items");
  const [file, edit] = byClass("iron-menubar-item");
  expect(byClass("iron-menu-item")).toEqual([]);

  dispatchUiEvent("pointerdown", [file!, row!, bar!], POINTER);
  expect(byClass("iron-menu-item").length).toBe(2);
  expect(byClass("iron-menu-backdrop").length).toBe(1);

  // Hovering the next title while open moves there.
  dispatchUiEvent("pointerenter", [edit!, row!, bar!], POINTER);
  await tick();
  const items = byClass("iron-menu-item");
  expect(items.length).toBe(1);
  const menu = byClass("iron-menu")[0]!;
  dispatchUiEvent("click", [items[0]!, menu, edit!, row!, bar!], POINTER);
  expect(ran).toEqual(["undo"]);
  expect(byClass("iron-menu-item")).toEqual([]);

  // Keys: Down on a highlighted bar opens, Enter chooses, Escape closes.
  dispatchUiEvent("pointerdown", [file!, row!, bar!], POINTER);
  dispatchUiEvent("keydown", [bar!], [0, 0, 0, 0, 0, 0, 0, 0, 0], ["ArrowDown", "ArrowDown"]);
  dispatchUiEvent("keydown", [bar!], [0, 0, 0, 0, 0, 0, 0, 0, 0], ["Enter", "Enter"]);
  expect(ran).toEqual(["undo", "open"]);
  dispatchUiEvent("pointerdown", [file!, row!, bar!], POINTER);
  dispatchUiEvent("keydown", [bar!], [0, 0, 0, 0, 0, 0, 0, 0, 0], ["Escape", "Escape"]);
  expect(byClass("iron-menu-item")).toEqual([]);

  // A press on the backdrop (anywhere else) closes.
  dispatchUiEvent("pointerdown", [file!, row!, bar!], POINTER);
  const [backdrop] = byClass("iron-menu-backdrop");
  dispatchUiEvent("pointerdown", [backdrop!, bar!], POINTER);
  expect(byClass("iron-menu-item")).toEqual([]);
  root.unmount();
});
