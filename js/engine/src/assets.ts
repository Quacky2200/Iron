// Asset files in the game folder. While the game runs from a folder (not in screenshot
// runs), the engine watches it: textures, models and sounds loaded from a file that
// changes load again in place (same ids, so everything using them updates), then
// `Assets.onChanged` listeners hear the changed paths (game-relative, forward
// slashes) to refresh anything else, such as scene or data files they read.

const listeners = new Set<(paths: string[]) => void>();

export const Assets = {
  /** Calls `fn` with the paths of asset files that changed; returns a function that stops it. */
  onChanged(fn: (paths: string[]) => void): () => void {
    listeners.add(fn);
    return () => listeners.delete(fn);
  },
};

globalThis.__iron_assets_changed = (paths: string[]) => {
  for (const f of [...listeners]) {
    try {
      f(paths);
    } catch (e) {
      console.error(`Assets.onChanged: ${e}`);
    }
  }
};

/** Forgets listeners. Only for tests. */
export function _resetAssetsForTests(): void {
  listeners.clear();
}
