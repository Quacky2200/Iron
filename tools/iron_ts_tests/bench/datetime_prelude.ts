// New York time, and the native API's name for subtracting a span.
import { DateTime } from "../../../js/engine/src/time.ts";

process.env.TZ = "America/New_York";
(DateTime.prototype as any).subtractTimeSpan = function (s: unknown) {
  return (this as any).subtract(s);
};
