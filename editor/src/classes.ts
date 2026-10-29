// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

/** Class names joined by spaces, empty ones left out: `cx("row", selected ? "selected" : "")`. */
export function cx(...names: string[]): string {
  return names.filter((n) => n !== "").join(" ");
}
