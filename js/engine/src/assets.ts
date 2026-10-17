// Asset files in the game folder.
//
// - The asset database: each asset's `.meta` file (made by `scripts/assets.ts`, which
//   `bun run dev` runs) gives it a GUID and importer settings. `Assets.guidToPath` /
//   `pathToGuid` look them up; anywhere a file path is taken (`Texture.load`,
//   `Model.load`, `audio.load`), `"guid:<guid>"` works too, so references survive
//   renames. Scenes save assets with their GUIDs.
// - Hot reload: while the game runs from a folder (not in screenshot runs), the engine
//   watches it: textures, models and sounds loaded from a file that changes load again
//   in place (same ids, so everything using them updates), then `Assets.onChanged`
//   listeners hear the changed paths (game-relative, forward slashes) to refresh
//   anything else, such as scene or data files they read.

const listeners = new Set<(paths: string[]) => void>();

/** One asset's database entry. */
export interface AssetEntry {
  path: string;
  guid: string;
  /** Its importer settings (`type`, and e.g. a texture's `filterMode`). */
  importer: Record<string, unknown>;
}

const entries = (): AssetEntry[] => ((globalThis.__iron_info as { assets?: AssetEntry[] } | undefined)?.assets ?? []);
const clean = (path: string) => path.replace(/\\/g, "/").replace(/^\.\//, "");

export const Assets = {
  /** Every asset with a `.meta` file. */
  get all(): readonly AssetEntry[] {
    return entries();
  },

  /** The game path of the asset with this GUID (`undefined`: none). */
  guidToPath(guid: string): string | undefined {
    return entries().find((e) => e.guid === guid)?.path;
  },

  /** The GUID of the asset at a game path (`undefined`: no `.meta`). */
  pathToGuid(path: string): string | undefined {
    const p = clean(path);
    return entries().find((e) => e.path === p)?.guid;
  },

  /** An asset's importer settings from its `.meta` (`{}` without one). */
  importer(path: string): Record<string, unknown> {
    const p = clean(Assets.resolve(path));
    return entries().find((e) => e.path === p)?.importer ?? {};
  },

  /** A path for `"guid:<guid>"` sources (others unchanged). Unknown GUIDs stay as given. */
  resolve(src: string): string {
    if (!src.startsWith("guid:")) return src;
    return Assets.guidToPath(src.slice(5)) ?? src;
  },

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
