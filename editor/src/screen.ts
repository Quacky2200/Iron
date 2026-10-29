// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// The window's size for panels that lay themselves out (the dock, the project browser).

let size = { width: Screen.width, height: Screen.height };
const screenSize = () => {
  if (size.width !== Screen.width || size.height !== Screen.height) size = { width: Screen.width, height: Screen.height };
  return size;
};

/** The window's size in UI pixels, re-rendering when it changes. */
export function useScreenSize(): { width: number; height: number } {
  return useSyncExternalStore(Screen.onChange, screenSize);
}
