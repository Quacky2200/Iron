// SPDX-License-Identifier: Apache-2.0

// The original scene with no engine behind it; each function starts with an empty scene.
import { _resetSceneForTests } from "../../../js/engine/src/scene.ts";

(globalThis as any).__platypus_op = () => {};
_resetSceneForTests();
(globalThis as any).__bench_reset = () => _resetSceneForTests();
