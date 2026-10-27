// SPDX-License-Identifier: Apache-2.0

// Three-argument hypot as the native math does it, and the native name for building a matrix
// from columns (the original takes an array).
import "./hypot_patch.ts";
import { Matrix4x4 } from "../../../js/engine/src/math.ts";

(Matrix4x4 as any).fromColumns = (a: any, b: any, c: any, d: any) =>
  new Matrix4x4([a.x, a.y, a.z, a.w, b.x, b.y, b.z, b.w, c.x, c.y, c.z, c.w, d.x, d.y, d.z, d.w]);
