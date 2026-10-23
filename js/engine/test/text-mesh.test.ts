// SPDX-License-Identifier: Apache-2.0

import { afterEach, beforeEach, describe, expect, test } from "bun:test";

import { Color, GameObject, TextMesh } from "../src/index.ts";
import { _resetSceneForTests } from "../src/scene.ts";

let ops: unknown[][] = [];

beforeEach(() => {
  _resetSceneForTests();
  ops = [];
  (globalThis as { __iron_op?: unknown }).__iron_op = (...args: unknown[]) => ops.push(args);
});

afterEach(() => {
  delete (globalThis as { __iron_op?: unknown }).__iron_op;
});

// M5-TXT-01
describe("TextMesh", () => {
  test("sends its text and look, once per frame, and goes on destroy", () => {
    const go = new GameObject("label");
    const label = go.addComponent(TextMesh, { text: "Score", fontSize: 24, color: new Color(1, 0.5, 0, 1), alignment: "left" });
    const texts = () => ops.filter((o) => o[0] === "render.text");
    expect(texts().at(-1)).toEqual(["render.text", label.id, go.id, "Score", [1, 0.5, 0, 1], 24, null, 400, "left", 0, 100, 2]);
    // Nothing changed: nothing sent before drawing.
    const sent = texts().length;
    label.onPreRender();
    expect(texts().length).toBe(sent);
    // Several changes in a frame go as one op.
    label.text = "Score: 10\nLevel 2";
    label.font = "Charm";
    label.maxWidth = 300;
    label.fontSize = 0;
    label.onPreRender();
    expect(texts().length).toBe(sent + 1);
    expect(texts().at(-1)!.slice(3)).toEqual(["Score: 10\nLevel 2", [1, 0.5, 0, 1], 1, "Charm", 400, "left", 300, 100, 2]);
    label.onDestroy();
    expect(ops.at(-1)).toEqual(["render.remove", label.id]);
  });
});
