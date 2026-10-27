// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// The engine ticks the clock three times before the game's code runs.
const g = globalThis as any;
g.__platypus_tick(0.25);
g.__platypus_tick(0.25);
g.__platypus_tick(0.25);
