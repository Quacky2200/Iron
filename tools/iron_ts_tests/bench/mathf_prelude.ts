// SPDX-License-Identifier: Apache-2.0

import "./hypot_patch.ts";
// The native API's `smoothDamp` answers a Vector2 (value, velocity) where the original returns an array.
import { Mathf as M, Vector2 } from "../../../js/engine/src/math.ts";

(globalThis as any).Mathf = {
  ...M,
  smoothDamp: (...a: Parameters<typeof M.smoothDamp>) => {
    const [out, v] = M.smoothDamp(...a);
    return new Vector2(out, v);
  },
};
