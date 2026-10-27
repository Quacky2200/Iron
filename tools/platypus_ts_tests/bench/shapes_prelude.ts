// SPDX-License-Identifier: Apache-2.0

// Three-argument hypot as the native math does it, and NaN where the original answers null.
import "./hypot_patch.ts";
import { Plane, Vector3 } from "../../../js/engine/src/math.ts";

const raycast = Plane.prototype.raycast;
const intersect = Plane.prototype.intersect;
Plane.prototype.raycast = function (this: Plane, ray) {
  return raycast.call(this, ray) ?? Number.NaN;
};
Plane.prototype.intersect = function (this: Plane, ray) {
  return intersect.call(this, ray) ?? new Vector3(Number.NaN, Number.NaN, Number.NaN);
};
