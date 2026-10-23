// SPDX-License-Identifier: Apache-2.0

// Layers: 32 render groups a GameObject belongs to one of (Unity's `layer`). Cameras
// draw the layers in their `cullingMask`, a bit per layer. Unity's built-in names come
// first; the game names the rest in its `package.json`: `"iron": { "layers": { "8":
// "Player", "9": "Minimap" } }`.

const BUILT_IN: Record<number, string> = {
  0: "Default",
  1: "TransparentFX",
  2: "Ignore Raycast",
  4: "Water",
  5: "UI",
};

/** Layer names by number: the built-ins, then the manifest's. */
function names(): Map<number, string> {
  const out = new Map<number, string>(Object.entries(BUILT_IN).map(([k, v]) => [Number(k), v]));
  const game = globalThis.__iron_info?.app.layers;
  if (game && typeof game === "object") {
    for (const [k, v] of Object.entries(game)) {
      const layer = Number(k);
      if (Number.isInteger(layer) && layer >= 0 && layer < 32 && typeof v === "string" && v) out.set(layer, v);
    }
  }
  return out;
}

export const LayerMask = {
  /** Every layer (Unity's `~0`). */
  everything: 0xffffffff,
  nothing: 0,
  /** A layer's number by name, or -1. */
  nameToLayer(name: string): number {
    for (const [layer, n] of names()) if (n === name) return layer;
    return -1;
  },
  /** A layer's name, or "" if it has none. */
  layerToName(layer: number): string {
    return names().get(layer) ?? "";
  },
  /** A mask with the named layers' bits set (unknown names are skipped). */
  getMask(...layerNames: string[]): number {
    let mask = 0;
    for (const name of layerNames) {
      const layer = LayerMask.nameToLayer(name);
      if (layer >= 0) mask |= 1 << layer;
    }
    return mask >>> 0;
  },
  /** Whether a mask includes a layer. */
  contains(mask: number, layer: number): boolean {
    return ((mask >>> 0) & ((1 << layer) >>> 0)) !== 0;
  },
};

/** @internal Checks a layer number (0 to 31). */
export function checkLayer(layer: number): number {
  if (!Number.isInteger(layer) || layer < 0 || layer > 31) throw new RangeError(`layer must be 0 to 31, got ${layer}`);
  return layer;
}
