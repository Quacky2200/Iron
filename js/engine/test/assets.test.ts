import { expect, test } from "bun:test";

import { Assets } from "../src/index.ts";

// M5-AST-02
test("Assets.onChanged hears changed files until stopped; a throwing listener doesn't stop the others", () => {
  const heard: string[][] = [];
  const stop = Assets.onChanged((p) => heard.push(p));
  Assets.onChanged(() => {
    throw new Error("boom");
  });
  const error = console.error;
  console.error = () => {};
  try {
    globalThis.__iron_assets_changed(["assets/crate.png"]);
    stop();
    globalThis.__iron_assets_changed(["assets/other.png"]);
  } finally {
    console.error = error;
  }
  expect(heard).toEqual([["assets/crate.png"]]);
});
