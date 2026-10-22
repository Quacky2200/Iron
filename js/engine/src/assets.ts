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

/**
 * The members the engine itself gives the global `Assets` (`#[iron_api(namespace = "Assets")]`,
 * crates/iron_engine_api/src/assets.rs; `Assets.exists` / `list` / `packOf` there are the typed
 * declarations of the generated `types/iron.d.ts`). This module's `Assets` below adds the asset
 * database and hot reload and hands these three to the native namespace, which answers from the
 * index the host fills (packs or the game folder, also assets without a `.meta`). Without the
 * engine (unit tests) they answer as an empty game would.
 */
interface NativeAssets {
  exists(path: string): boolean;
  list(glob: string): string[];
  packOf(path: string): string;
}
const native = (): Partial<NativeAssets> => (globalThis as { Assets?: Partial<NativeAssets> }).Assets ?? {};

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

  /**
   * Whether the game can load this path: a file of a pack or of the game folder, a `data:` URL,
   * or a `guid:` source of a known asset.
   */
  exists(path: string): boolean {
    return native().exists?.(path) ?? false;
  },

  /**
   * The paths of every asset matching a glob (`*` and `?` inside one folder name, `**` across
   * folders, case-sensitive; a plain folder matches everything below it), sorted.
   */
  list(glob: string): string[] {
    return native().list?.(glob) ?? [];
  },

  /** The pack that holds the asset at this path (`""` for a loose file or an unknown path). */
  packOf(path: string): string {
    return native().packOf?.(path) ?? "";
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
