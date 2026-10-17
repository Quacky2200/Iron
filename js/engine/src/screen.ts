// The window as scripts see it (Unity's `Screen`). Sizes are in UI pixels, the units
// pointer events (`clientX`/`clientY`) and screen UI use: the window's pixels divided
// by its DPI scale and the UI scale (`setUiScale`). The engine reports changes with
// `__iron_screen(width, height, scale)`.

let width = 1280;
let height = 720;
let scale = 1;

/** How the window covers the screen (Unity's `FullScreenMode`). */
export type FullScreenMode = "ExclusiveFullScreen" | "FullScreenWindow" | "MaximizedWindow" | "Windowed";

/** A display mode: physical pixels and Hz (0 when unknown). */
export interface Resolution {
  width: number;
  height: number;
  refreshRate: number;
}

interface DisplayInfo {
  mode: FullScreenMode;
  width: number;
  height: number;
  refreshRate: number;
  windowWidth: number;
  windowHeight: number;
  supportedModes: FullScreenMode[];
  resolutions: Resolution[];
  currentResolution: Resolution | null;
}

/** What the engine says about the window and monitor (updated after each change). */
const display = (): Partial<DisplayInfo> => (globalThis.__iron_info as { display?: Partial<DisplayInfo> } | undefined)?.display ?? {};

const mode = (): FullScreenMode => display().mode ?? "Windowed";
const isFullScreen = (m: FullScreenMode) => m === "FullScreenWindow" || m === "ExclusiveFullScreen";
const windowSize = () => ({
  width: display().windowWidth ?? display().width ?? 1280,
  height: display().windowHeight ?? display().height ?? 720,
});

function setResolution(w: number, h: number, m: FullScreenMode | boolean = mode(), refreshRate = 0): void {
  const chosen: FullScreenMode = typeof m === "boolean" ? (m ? "FullScreenWindow" : "Windowed") : m;
  __iron_op("screen.set", chosen, Math.round(w), Math.round(h), refreshRate);
}

export const Screen = {
  /** Width in UI pixels. */
  get width(): number {
    return width;
  },
  /** Height in UI pixels. */
  get height(): number {
    return height;
  },
  /** Physical pixels per UI pixel (DPI scale × UI scale). */
  get scale(): number {
    return scale;
  },
  get aspect(): number {
    return width / Math.max(1, height);
  },

  /**
   * The window's mode. Setting it asks the engine; modes this platform doesn't have
   * fall back to the nearest (no exclusive fullscreen on Wayland, only fullscreen on
   * phones). The choice is saved for next time.
   */
  get fullScreenMode(): FullScreenMode {
    return mode();
  },
  set fullScreenMode(m: FullScreenMode) {
    // No size: exclusive takes the monitor's current resolution, windowed goes back to
    // the last window size.
    __iron_op("screen.set", m, 0, 0, 0);
  },

  /** Fullscreen (borderless, or exclusive when that's the mode) or not. */
  get fullScreen(): boolean {
    return isFullScreen(mode());
  },
  set fullScreen(on: boolean) {
    if (on === isFullScreen(mode())) return;
    __iron_op("screen.set", on ? "FullScreenWindow" : "Windowed", 0, 0, 0);
  },

  /**
   * Sets the window's size in physical pixels (windowed), or the display's mode
   * (exclusive fullscreen: the closest the monitor has), and optionally the mode and
   * refresh rate. Borderless and maximized windows ignore the size.
   */
  setResolution,

  /** The window's size when windowed (kept through fullscreen), in physical pixels. */
  get windowSize(): { width: number; height: number } {
    return windowSize();
  },

  /** The monitor's display modes, largest and fastest first (empty before the window opens). */
  get resolutions(): Resolution[] {
    return [...(display().resolutions ?? [])];
  },

  /** What the monitor runs at now. */
  get currentResolution(): Resolution {
    return display().currentResolution ?? { width: 0, height: 0, refreshRate: 0 };
  },

  /** The modes this platform offers, most fullscreen first. */
  get supportedFullScreenModes(): FullScreenMode[] {
    return [...(display().supportedModes ?? ["ExclusiveFullScreen", "FullScreenWindow", "MaximizedWindow", "Windowed"])];
  },
};

globalThis.__iron_screen = (w: number, h: number, s: number) => {
  width = w;
  height = h;
  scale = s;
};

/** Sets the screen size. Only for tests. */
export function _setScreenForTests(w: number, h: number, s = 1) {
  globalThis.__iron_screen(w, h, s);
}
