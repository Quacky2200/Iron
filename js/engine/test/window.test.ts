// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

import { afterEach, describe, expect, test } from "bun:test";

describe("Window", () => {
  afterEach(() => {
    delete (globalThis as { Window?: unknown }).Window;
  });

  // M9-TS-2822
  test("@platypus/engine re-exports the global Window", async () => {
    const stub = { main: { handle: 1 }, count: 1, isSupported: true, create: () => ({ handle: 2 }) };
    (globalThis as { Window?: unknown }).Window = stub;
    // A fresh copy of the module, so it reads the stub.
    const mod = await import("../src/window.ts?stubbed");
    expect(mod.Window).toBe(stub as unknown as typeof mod.Window);
    expect(mod.Window.main.handle).toBe(1);
    expect(mod.Window.create({ title: "T" }).handle).toBe(2);
    const index = await import("../src/index.ts");
    expect("Window" in index).toBe(true);
  });
});
