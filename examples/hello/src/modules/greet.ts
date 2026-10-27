// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// A module of this game: imported by its bare name, `import { greeting } from "greet"`.
export function greeting(name: string, times: number): string {
  return `hello ${name}`.repeat(Math.max(1, times));
}
