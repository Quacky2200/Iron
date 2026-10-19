// The game's database, and the native API's shape over the original `Assets` (no undefined, an
// importer as JSON text, a list by index).
import { Assets as A } from "../../../js/engine/src/assets.ts";

(globalThis as any).__iron_info = {
  assets: [
    { path: "textures/a.png", guid: "guid-a", importer: { type: "texture", filterMode: "point" } },
    { path: "sounds/b.ogg", guid: "guid-b", importer: { type: "audio" } },
  ],
};
(globalThis as any).Assets = {
  resolve: (s: string) => A.resolve(s),
  count: () => A.all.length,
  pathAt: (i: number) => A.all[i]?.path ?? "",
  guidAt: (i: number) => A.all[i]?.guid ?? "",
  guidToPath: (g: string) => A.guidToPath(g) ?? "",
  pathToGuid: (p: string) => A.pathToGuid(p) ?? "",
  importer: (p: string) => JSON.stringify(A.importer(p)),
};
