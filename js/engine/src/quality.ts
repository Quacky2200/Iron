// Rendering quality (Unity's `QualitySettings`): named levels (Low / Medium / High /
// Ultra) that set anti-aliasing, render scale and shadows together, and each setting on
// its own. A game's `package.json` can choose the default level and change the levels:
//
//   "iron": { "quality": { "default": "Medium", "levels": { "Low": { "renderScale": 0.6 } } } }
//
// The default level is applied when the engine loads.

import { _loadJson, _saveJson } from "./prefs.ts";

/** Which shadows are drawn (Unity's `ShadowQuality`). */
export type ShadowQualityMode = "disable" | "hardOnly" | "all";

export interface QualityLevel {
  name: string;
  /** Samples per pixel for anti-aliasing (MSAA): 1 (off), 2, 4 or 8. */
  antiAliasing: number;
  /** The world's resolution relative to the window (0.25 to 2); UI stays sharp. */
  renderScale: number;
  shadows: ShadowQualityMode;
  /** Soft shadows' filter: `low` (3 × 3 samples) or `high` (5 × 5). */
  softShadowQuality: "low" | "high";
  /** The shadow map's size in texels. */
  shadowResolution: number;
  /** How far shadows reach from the camera (metres). */
  shadowDistance: number;
  /** The sun's shadow cascades: 1, 2 or 4 (more keeps near shadows sharp far out). */
  shadowCascades: number;
  /** LOD groups switch as if things were this many times taller on screen (more detail above 1). */
  lodBias: number;
  /** 1: frames wait for the display's refresh (no tearing); 0: they don't (`targetFrameRate` caps them). */
  vSyncCount: number;
  /** Sharper textures at grazing angles: never, as each texture asks (`anisoLevel`), or at least 9x everywhere. */
  anisotropicFiltering: AnisotropicFiltering;
  /** Top mip levels skipped (Unity's texture quality): 0 full size, 1 half, 2 quarter, 3 eighth. */
  globalTextureMipmapLimit: number;
  /**
   * Lets the game's auto exposure (`RenderSettings.autoExposure`) run: a player's option.
   * Off, it blends to the game's manual exposure as if the game had turned it off.
   */
  autoExposure: boolean;
}

/** Unity's `AnisotropicFiltering`. */
export type AnisotropicFiltering = "disable" | "enable" | "forceEnable";

const DEFAULT_LEVELS: QualityLevel[] = [
  { name: "Low", antiAliasing: 1, renderScale: 0.75, shadows: "hardOnly", softShadowQuality: "low", shadowResolution: 1024, shadowDistance: 20, shadowCascades: 1, lodBias: 0.4, vSyncCount: 1, anisotropicFiltering: "disable", globalTextureMipmapLimit: 1, autoExposure: true },
  { name: "Medium", antiAliasing: 2, renderScale: 1, shadows: "all", softShadowQuality: "low", shadowResolution: 2048, shadowDistance: 25, shadowCascades: 2, lodBias: 0.7, vSyncCount: 1, anisotropicFiltering: "enable", globalTextureMipmapLimit: 0, autoExposure: true },
  { name: "High", antiAliasing: 4, renderScale: 1, shadows: "all", softShadowQuality: "low", shadowResolution: 2048, shadowDistance: 30, shadowCascades: 4, lodBias: 1, vSyncCount: 1, anisotropicFiltering: "enable", globalTextureMipmapLimit: 0, autoExposure: true },
  { name: "Ultra", antiAliasing: 8, renderScale: 1, shadows: "all", softShadowQuality: "high", shadowResolution: 4096, shadowDistance: 40, shadowCascades: 4, lodBias: 2, vSyncCount: 1, anisotropicFiltering: "forceEnable", globalTextureMipmapLimit: 0, autoExposure: true },
];

interface ManifestQuality {
  default?: string;
  levels?: Record<string, Partial<Omit<QualityLevel, "name">>>;
}

const manifest = (): ManifestQuality => {
  const q = (globalThis as { __iron_info?: { app?: { quality?: unknown } } }).__iron_info?.app?.quality;
  return q && typeof q === "object" ? (q as ManifestQuality) : {};
};

/** The levels with the game's overrides applied. */
function buildLevels(): QualityLevel[] {
  const overrides = manifest().levels ?? {};
  return DEFAULT_LEVELS.map((l) => ({ ...l, ...(overrides[l.name] ?? {}), name: l.name }));
}

let levels = buildLevels();
let levelIndex = Math.max(0, levels.findIndex((l) => l.name === (manifest().default ?? "High")));
let current: QualityLevel = { ...levels[levelIndex]! };

function sendQuality(): void {
  const c = current;
  __iron_op("render.quality", 0, c.antiAliasing, c.renderScale, c.shadows, c.softShadowQuality === "high" ? 2 : 1, c.lodBias, c.vSyncCount, c.anisotropicFiltering, c.globalTextureMipmapLimit);
  remember();
}

function sendShadows(): void {
  __iron_op("render.shadowSettings", 0, current.shadowDistance, current.shadowResolution, current.shadowCascades);
  remember();
}

/** The player's choice is saved (`iron.quality` in PlayerPrefs) and comes back next time. */
function remember(): void {
  if (restoring) return;
  _saveJson("iron.quality", { level: levels[levelIndex]?.name, current });
}
let restoring = false;

const listeners = new Set<(level: number) => void>();

/** Resends `render.post` when the player's auto exposure option changes (set by rendering.ts). */
let postHook: (() => void) | null = null;
/** @internal */
export function _setPostHook(f: () => void): void {
  postHook = f;
}

export const QualitySettings = {
  /** The levels' names, lowest first. */
  get names(): string[] {
    return levels.map((l) => l.name);
  },

  /** Each level's settings (copies). */
  get levels(): QualityLevel[] {
    return levels.map((l) => ({ ...l }));
  },

  /** The current level's index; changing a setting on its own keeps it. */
  getQualityLevel(): number {
    return levelIndex;
  },

  /** Switches to a level, by index or name, setting everything it covers. */
  setQualityLevel(level: number | string): void {
    const i = typeof level === "number" ? level : levels.findIndex((l) => l.name === level);
    const l = levels[i];
    if (!l) throw new Error(`no quality level ${level}`);
    const autoWas = current.autoExposure ?? true;
    levelIndex = i;
    current = { ...l };
    sendQuality();
    sendShadows();
    if ((current.autoExposure ?? true) !== autoWas) postHook?.();
    for (const f of listeners) f(i);
  },

  /** Called after `setQualityLevel`; returns a function that stops it. */
  onLevelChanged(f: (level: number) => void): () => void {
    listeners.add(f);
    return () => listeners.delete(f);
  },

  /** Samples per pixel (1, 2, 4 or 8; lowered to what the GPU supports). */
  get antiAliasing(): number {
    return current.antiAliasing;
  },
  set antiAliasing(n: number) {
    current.antiAliasing = n <= 1 ? 1 : n <= 2 ? 2 : n <= 4 ? 4 : 8;
    sendQuality();
  },

  get renderScale(): number {
    return current.renderScale;
  },
  set renderScale(s: number) {
    current.renderScale = Math.min(2, Math.max(0.25, s));
    sendQuality();
  },

  get shadows(): ShadowQualityMode {
    return current.shadows;
  },
  set shadows(m: ShadowQualityMode) {
    current.shadows = m;
    sendQuality();
  },

  get softShadowQuality(): "low" | "high" {
    return current.softShadowQuality;
  },
  set softShadowQuality(q: "low" | "high") {
    current.softShadowQuality = q;
    sendQuality();
  },

  /** How far from the camera shadows reach, in metres: shorter is sharper. */
  get shadowDistance(): number {
    return current.shadowDistance;
  },
  set shadowDistance(d: number) {
    current.shadowDistance = d;
    sendShadows();
  },

  /** 1: frames wait for the display's refresh; 0: they don't (`Application.targetFrameRate` caps them). */
  get vSyncCount(): number {
    return current.vSyncCount;
  },
  set vSyncCount(n: number) {
    current.vSyncCount = n > 0 ? 1 : 0;
    sendQuality();
  },

  /** LOD groups switch as if things were this many times taller on screen: above 1 keeps detail farther away. */
  get lodBias(): number {
    return current.lodBias;
  },
  set lodBias(b: number) {
    current.lodBias = Math.max(0.01, b);
    sendQuality();
  },

  /** Sharper textures at grazing angles (Unity's `anisotropicFiltering`). */
  get anisotropicFiltering(): AnisotropicFiltering {
    return current.anisotropicFiltering;
  },
  set anisotropicFiltering(a: AnisotropicFiltering) {
    current.anisotropicFiltering = a;
    sendQuality();
  },

  /** Top mip levels skipped, 0 to 3 (Unity's texture quality: full, half, quarter, eighth). */
  get globalTextureMipmapLimit(): number {
    return current.globalTextureMipmapLimit;
  },
  set globalTextureMipmapLimit(n: number) {
    current.globalTextureMipmapLimit = Math.min(3, Math.max(0, Math.round(n)));
    sendQuality();
  },

  /**
   * The player's switch for auto exposure (on by default): off, the game's auto exposure
   * blends out to its manual exposure (over `RenderSettings.exposureTransition`).
   */
  get autoExposure(): boolean {
    return current.autoExposure ?? true;
  },
  set autoExposure(on: boolean) {
    current.autoExposure = on;
    remember();
    postHook?.();
  },

  /** The sun's shadow cascades: 1, 2 or 4 (Unity's `shadowCascades`). */
  get shadowCascades(): number {
    return current.shadowCascades;
  },
  set shadowCascades(n: number) {
    current.shadowCascades = n >= 4 ? 4 : n >= 2 ? 2 : 1;
    sendShadows();
  },

  /** The shadow map's size in texels (256 to 8192). */
  get shadowResolution(): number {
    return current.shadowResolution;
  },
  set shadowResolution(r: number) {
    current.shadowResolution = r;
    sendShadows();
  },
};

/** @internal The level and settings, for hot reload. */
export function _qualityState(): unknown {
  return { level: levelIndex, current };
}

/** @internal Puts back what `_qualityState` saved and sends it. */
export function _restoreQuality(state: unknown): void {
  const s = state as { level?: number; current?: Partial<QualityLevel> } | null;
  if (!s?.current) return;
  levelIndex = Math.min(levels.length - 1, Math.max(0, s.level ?? levelIndex));
  current = { ...levels[levelIndex]!, ...s.current };
  sendQuality();
  sendShadows();
}

/** @internal Applies the player's saved choice, else the default level (done when the engine loads). */
export function _applyDefaultQuality(): void {
  const saved = _loadJson("iron.quality") as { level?: string; current?: Partial<QualityLevel> } | null;
  if (saved?.current) {
    const i = levels.findIndex((l) => l.name === saved.level);
    if (i >= 0) levelIndex = i;
    current = { ...levels[levelIndex]!, ...saved.current, name: levels[levelIndex]!.name };
  }
  // The starting state isn't a choice to save.
  restoring = true;
  try {
    sendQuality();
    sendShadows();
  } finally {
    restoring = false;
  }
}

/** Resets to the default levels and level. Only for tests. */
export function _resetQualityForTests() {
  levels = buildLevels();
  levelIndex = Math.max(0, levels.findIndex((l) => l.name === (manifest().default ?? "High")));
  current = { ...levels[levelIndex]! };
  listeners.clear();
}

// The engine starts at the default level.
if (typeof __iron_op === "function") _applyDefaultQuality();
