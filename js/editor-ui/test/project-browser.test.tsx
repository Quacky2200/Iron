// SPDX-License-Identifier: Apache-2.0

import { beforeEach, expect, test } from "bun:test";

import { dispatchUiEvent } from "../../react/src/events.ts";
import { instances, resetForTests } from "../../react/src/host.ts";
import { _resetRootForTests, createRoot } from "../../react/src/index.ts";
import { _resetProjectForTests, PROJECT_PATH_TYPE } from "../src/project.ts";
import { ProjectBrowser } from "../src/ProjectBrowser.tsx";

let ops: unknown[][] = [];
beforeEach(() => {
  ops = [];
  (globalThis as { __platypus_commit?: unknown }).__platypus_commit = () => {};
  (globalThis as { __platypus_op?: unknown }).__platypus_op = (...args: unknown[]) => ops.push(args);
  (globalThis as { __platypus_info?: unknown }).__platypus_info = { app: { project: "C:/Games/Arcade" }, system: {} };
  resetForTests();
  _resetRootForTests();
  _resetProjectForTests();
});

const tick = () => new Promise((resolve) => setTimeout(resolve, 5));
const byClass = (name: string) =>
  [...instances.values()].filter((i) => String((i.props as { className?: string }).className ?? "").split(" ").includes(name));
const POINTER = [0, 0, 0, 1, 1, 0, 0, 0];
/** An event path of just the row: its own handlers are what's checked. */
const pathOf = (id: number): number[] => [id];

// M5-DND-10
test("the project browser lists a folder, takes dropped files and moves items into folders", async () => {
  const root = createRoot();
  root.render(<ProjectBrowser width={400} height={300} />);
  await tick();
  expect(ops[0]).toEqual(["editor.listDir", 1, ""]);
  globalThis.__platypus_editor_fs([
    1,
    true,
    JSON.stringify({
      path: "",
      entries: [
        { name: "assets", dir: true, size: 0 },
        { name: "main.tsx", dir: false, size: 2048 },
      ],
    }),
  ]);
  await tick();
  const rows = byClass("platypus-project-row");
  expect(rows.map((r) => (r.props as { draggable?: boolean }).draggable)).toEqual([true, true]);
  const [folder, file] = rows;

  // Files from the system dropped on the folder row are copied into it.
  const files = ["C:\\in\\logo.png"];
  expect(dispatchUiEvent("dragover", pathOf(folder!.id), POINTER, files)).toBe(true);
  dispatchUiEvent("drop", pathOf(folder!.id), POINTER, files);
  expect(ops.at(-1)).toEqual(["editor.copyIn", 2, files, "assets"]);

  // A file dragged onto the folder moves there; onto itself it isn't accepted.
  dispatchUiEvent("dragstart", pathOf(file!.id), POINTER);
  expect(dispatchUiEvent("dragover", pathOf(file!.id), POINTER)).toBe(false);
  expect(dispatchUiEvent("dragover", pathOf(folder!.id), POINTER)).toBe(true);
  dispatchUiEvent("drop", pathOf(folder!.id), POINTER);
  dispatchUiEvent("dragend", pathOf(file!.id), [0, 0, 0, 0, 1, 0, 0, 0]);
  expect(ops.at(-1)).toEqual(["editor.move", 3, "main.tsx", "assets"]);
  expect(PROJECT_PATH_TYPE).toContain("platypus");
  root.unmount();
});
