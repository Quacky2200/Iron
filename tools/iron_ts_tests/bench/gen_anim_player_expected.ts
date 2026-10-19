// Runs every exported function of anim_player.ts in bun against the original engine TypeScript
// (animation.ts, scene.ts, math.ts, rendering.ts) and writes what they return.
//   bun bench/gen_anim_player_expected.ts
import * as e0 from "../../../js/engine/src/animation.ts";
import * as e1 from "../../../js/engine/src/scene.ts";
import * as e2 from "../../../js/engine/src/math.ts";
import * as e3 from "../../../js/engine/src/rendering.ts";

Object.assign(globalThis, e0);
Object.assign(globalThis, e1);
Object.assign(globalThis, e2);
Object.assign(globalThis, e3);
// What the engine sets up around the game (info, clock ticks), if the program needs any.
if (await Bun.file(import.meta.dir + "/anim_player_prelude.ts").exists()) await import("./anim_player_prelude.ts");
const m = await import("./anim_player.ts");

const out: Record<string, unknown> = {};
for (const [name, fn] of Object.entries(m)) {
  if (typeof fn !== "function") continue;
  const v = (fn as () => unknown)();
  out[name] = typeof v === "string" ? { s: Array.from({ length: v.length }, (_, i) => v.charCodeAt(i)) } : v;
}
await Bun.write(import.meta.dir + "/anim_player_expected.json", JSON.stringify(out));
console.log("wrote", Object.keys(out).length, "results");
