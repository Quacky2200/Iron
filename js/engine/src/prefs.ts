// The player's saved settings (Unity's `PlayerPrefs`): numbers and strings by key, kept
// in a file in the user's settings folder (the engine picks it per OS; see
// crates/iron_app/src/prefs.rs). Reads come from a copy made at startup; writes go to the
// engine, which writes the file on `save()` and when the game closes. Keys starting
// with `iron.` are the engine's own (window mode, quality level).

const values = new Map<string, unknown>(Object.entries((globalThis.__iron_info as { prefs?: Record<string, unknown> } | undefined)?.prefs ?? {}));

function set(key: string, value: unknown): void {
  values.set(key, value);
  __iron_op("prefs.set", key, value as never);
}

export const PlayerPrefs = {
  /** A whole number (`fallback` when there's none). */
  getInt(key: string, fallback = 0): number {
    const v = values.get(key);
    return typeof v === "number" ? Math.trunc(v) : fallback;
  },
  getFloat(key: string, fallback = 0): number {
    const v = values.get(key);
    return typeof v === "number" ? v : fallback;
  },
  getString(key: string, fallback = ""): string {
    const v = values.get(key);
    return typeof v === "string" ? v : fallback;
  },
  setInt(key: string, value: number): void {
    set(key, Math.trunc(value));
  },
  setFloat(key: string, value: number): void {
    set(key, value);
  },
  setString(key: string, value: string): void {
    set(key, value);
  },
  hasKey(key: string): boolean {
    return values.has(key);
  },
  deleteKey(key: string): void {
    values.delete(key);
    __iron_op("prefs.delete", key);
  },
  /** Every key, the engine's settings too. */
  deleteAll(): void {
    values.clear();
    __iron_op("prefs.deleteAll");
  },
  /** Writes the file now (it's also written when the game closes). */
  save(): void {
    __iron_op("prefs.save");
  },
};

/** @internal A JSON value under an engine key (`iron.quality`), or null. */
export function _loadJson(key: string): unknown {
  const v = values.get(key);
  if (typeof v !== "string") return null;
  try {
    return JSON.parse(v);
  } catch {
    return null;
  }
}

/** @internal Saves a JSON value under an engine key. */
export function _saveJson(key: string, value: unknown): void {
  set(key, JSON.stringify(value));
}

/** Reads the engine's copy again (`__iron_info.prefs`). Only for tests. */
export function _resetPrefsForTests(): void {
  values.clear();
  const saved = (globalThis.__iron_info as { prefs?: Record<string, unknown> } | undefined)?.prefs ?? {};
  for (const [k, v] of Object.entries(saved)) values.set(k, v);
}
