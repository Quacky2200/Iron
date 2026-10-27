// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// In-place reloads of the game's code (dev builds, where React and the engine live in a
// vendor bundle that stays loaded). The engine wraps each load of the game's code in
// `__platypus_hot_begin()` / `__platypus_hot_end()`:
//
// - What the game's top level set up last time is undone first (its style sheets), so
//   running the new top level doesn't add things twice.
// - Afterwards, live components of registered classes are moved onto the new classes
//   (their fields kept), and React re-renders what changed (`@platypus/react` hooks in).

import { _patchClasses } from "./scene.ts";

let loading = false;
let undo: (() => void)[] = [];
const afterLoad: (() => void)[] = [];

/** True while the game's code is loading (`createRoot` then reuses its root). */
export function _loadingGame(): boolean {
  return loading;
}

/** @internal Undoes something the game's top level did, before its code loads again. */
export function _undoOnReload(f: () => void): void {
  if (loading) undo.push(f);
}

/** @internal Runs after each load of the game's code. */
export function _onHotEnd(f: () => void): void {
  afterLoad.push(f);
}

declare global {
  var __platypus_hot_begin: (() => void) | undefined;
  var __platypus_hot_end: (() => string[]) | undefined;
}

globalThis.__platypus_hot_begin = () => {
  const old = undo;
  undo = [];
  for (const f of old) {
    try {
      f();
    } catch (e) {
      console.error(`reload: ${e}`);
    }
  }
  loading = true;
};

/** Returns the names of component classes whose live instances kept the old code. */
globalThis.__platypus_hot_end = () => {
  loading = false;
  const stale = _patchClasses();
  if (stale.length) {
    console.warn(
      `reload: ${stale.join(", ")} ${stale.length === 1 ? "isn't" : "aren't"} registered, so ` +
        `${stale.length === 1 ? "its" : "their"} components keep the old code (register({ ${stale.join(", ")} }, { spawn: false }))`,
    );
  }
  for (const f of afterLoad) {
    try {
      f();
    } catch (e) {
      console.error(`reload: ${e}`);
    }
  }
  return stale;
};
