// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// Preloaded by bun so `@platypus/engine` can be imported outside the engine (it calls `__platypus_op`).
(globalThis as { __platypus_op?: () => void }).__platypus_op = () => {};
(globalThis as { __platypus_commit?: () => void }).__platypus_commit = () => {};
