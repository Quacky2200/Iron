// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// The saved file, installed the way the engine does (and read again: the module read it at import).
import { _resetPrefsForTests } from "../../../js/engine/src/prefs.ts";

(globalThis as any).__platypus_op = () => {};
(globalThis as any).__platypus_info = { prefs: { "saved.n": 7.9, "saved.s": "hello" } };
_resetPrefsForTests();
