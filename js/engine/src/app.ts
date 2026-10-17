// The game and the machine it runs on (Unity's `Application` and `SystemInfo`). The
// engine installs `globalThis.__iron_info` before the game's code runs: the game's
// manifest (its `package.json`: `name`, `version` and an `iron` section with `title`,
// `company`, `identifier` and `icon`) and what the OS and GPU report.
//
// Focus: Rust calls `__iron_focus(focused, paused)` when the window gains or loses
// focus (or is minimised) and when that pauses the game (`runInBackground` off).

import { _forEachActiveComponent } from "./scene.ts";

interface InfoApp {
  name: string;
  productName: string;
  version: string;
  companyName: string;
  identifier: string;
  engineVersion: string;
  /** The manifest's `iron.quality` (QualitySettings reads it). */
  quality?: unknown;
  /** The manifest's `iron.layers`: names by layer number (LayerMask reads it). */
  layers?: unknown;
  /** The manifest's `iron.startScene`: loaded when the game starts. */
  startScene?: string | null;
  /** The manifest's `iron.runInBackground`, or what scripts last set (kept over reloads). */
  runInBackground?: boolean;
  /** What scripts last set (kept over reloads). */
  targetFrameRate?: number;
}

interface InfoSystem {
  operatingSystem: string;
  operatingSystemFamily: string;
  operatingSystemVersion: string;
  deviceType: string;
  deviceName: string;
  processorType: string;
  processorCount: number;
  systemMemorySize: number;
  architecture: string;
  language: string;
  touchSupported: boolean;
  graphicsDeviceName?: string;
  graphicsDeviceVendorID?: number;
  graphicsDeviceType?: string;
  graphicsDeviceKind?: string;
  graphicsDriver?: string;
  screenWidth?: number;
  screenHeight?: number;
  screenRefreshRate?: number;
  dpi?: number;
}

declare global {
  var __iron_info: { app: InfoApp; system: InfoSystem } | undefined;
  var __iron_focus: (focused: boolean, paused: boolean) => void;
}

// Set by scripts; until then the engine's (or the manifest's) value.
let targetFrameRate: number | undefined;
let runInBackground: boolean | undefined;
let focused = true;
let paused = false;
const focusListeners = new Set<(focused: boolean) => void>();

const app = (): Partial<InfoApp> => globalThis.__iron_info?.app ?? {};
const sys = (): Partial<InfoSystem> => globalThis.__iron_info?.system ?? {};

/** Which OS family (Unity's `OperatingSystemFamily`, plus the mobile ones). */
export type OperatingSystemFamily = "Windows" | "MacOSX" | "Linux" | "Android" | "iOS" | "Other";

/** What kind of device (Unity's `DeviceType`). */
export type DeviceType = "Desktop" | "Handheld" | "Console" | "Unknown";

export const Application = {
  /** The game's `package.json` name. */
  get name(): string {
    return app().name ?? "";
  },
  /** The title it shows (`iron.title`, else the name). Unity's `productName`. */
  get productName(): string {
    return app().productName ?? "";
  },
  get version(): string {
    return app().version ?? "0.0.0";
  },
  get companyName(): string {
    return app().companyName ?? "";
  },
  /** Reverse-DNS id (`iron.identifier`). */
  get identifier(): string {
    return app().identifier ?? "";
  },
  /** Iron's version (Unity's `unityVersion`). */
  get engineVersion(): string {
    return app().engineVersion ?? "";
  },
  /** The OS family it runs on. */
  get platform(): OperatingSystemFamily {
    return (sys().operatingSystemFamily as OperatingSystemFamily) ?? "Other";
  },
  /** The user's language as a BCP 47 tag (`en-GB`). Unity's `systemLanguage`. */
  get systemLanguage(): string {
    return sys().language ?? "en-US";
  },
  /** Always false: there's no editor yet. */
  get isEditor(): boolean {
    return false;
  },
  get isMobilePlatform(): boolean {
    return sys().deviceType === "Handheld";
  },
  /**
   * Frames per second to aim for (-1: as fast as presenting allows). Like Unity's, it
   * only counts with vsync off (`QualitySettings.vSyncCount = 0`): vsync paces frames
   * to the display. Not saved: games keep it in `PlayerPrefs` if players choose it.
   */
  get targetFrameRate(): number {
    return targetFrameRate ?? app().targetFrameRate ?? -1;
  },
  set targetFrameRate(fps: number) {
    targetFrameRate = fps > 0 ? Math.round(fps) : -1;
    __iron_op("app.targetFrameRate", targetFrameRate);
  },
  /**
   * Keep running while the window is in the background (unfocused or minimised).
   * Off by default (`iron.runInBackground` in `package.json` changes that): then the
   * game pauses — no updates, fixed ticks or timers, sounds muted — until it's back.
   */
  get runInBackground(): boolean {
    return runInBackground ?? app().runInBackground ?? false;
  },
  set runInBackground(on: boolean) {
    runInBackground = !!on;
    __iron_op("app.runInBackground", runInBackground);
  },
  /** Whether the window has focus (and isn't minimised). */
  get isFocused(): boolean {
    return focused;
  },
  /** Whether the game is paused in the background. */
  get isPaused(): boolean {
    return paused;
  },
  /** Calls `f` when the window gains or loses focus; returns a function that stops it. */
  onFocusChanged(f: (focused: boolean) => void): () => void {
    focusListeners.add(f);
    return () => {
      focusListeners.delete(f);
    };
  },
  /** Closes the game after this frame. */
  quit(): void {
    __iron_op("app.quit");
  },
};

/** The machine (Unity's `SystemInfo`). GPU and screen details arrive once the window opens. */
export const SystemInfo = {
  /** OS name and version, e.g. `Windows 11 Pro`. */
  get operatingSystem(): string {
    return sys().operatingSystem ?? "";
  },
  get operatingSystemFamily(): OperatingSystemFamily {
    return (sys().operatingSystemFamily as OperatingSystemFamily) ?? "Other";
  },
  get operatingSystemVersion(): string {
    return sys().operatingSystemVersion ?? "";
  },
  get deviceType(): DeviceType {
    return (sys().deviceType as DeviceType) ?? "Unknown";
  },
  /** The computer's network name. */
  get deviceName(): string {
    return sys().deviceName ?? "";
  },
  /** CPU model. */
  get processorType(): string {
    return sys().processorType ?? "";
  },
  /** Logical CPU cores. */
  get processorCount(): number {
    return sys().processorCount ?? 1;
  },
  /** Memory in MB. */
  get systemMemorySize(): number {
    return sys().systemMemorySize ?? 0;
  },
  /** CPU architecture: `x86_64`, `aarch64`. */
  get architecture(): string {
    return sys().architecture ?? "";
  },
  get graphicsDeviceName(): string {
    return sys().graphicsDeviceName ?? "";
  },
  /** PCI vendor id (0x10DE NVIDIA, 0x1002 AMD, 0x8086 Intel). */
  get graphicsDeviceVendorID(): number {
    return sys().graphicsDeviceVendorID ?? 0;
  },
  /** The graphics API in use: `Vulkan`, `Dx12`, `Metal`, `Gl`. */
  get graphicsDeviceType(): string {
    return sys().graphicsDeviceType ?? "";
  },
  /** `DiscreteGpu`, `IntegratedGpu`, `VirtualGpu`, `Cpu`, `Other`. */
  get graphicsDeviceKind(): string {
    return sys().graphicsDeviceKind ?? "";
  },
  get graphicsDriver(): string {
    return sys().graphicsDriver ?? "";
  },
  get touchSupported(): boolean {
    return sys().touchSupported ?? false;
  },
  /** The monitor the window is on, in physical pixels, and its refresh rate (Hz, 0 if unknown). */
  get screen(): { width: number; height: number; refreshRate: number; dpi: number } {
    const s = sys();
    return { width: s.screenWidth ?? 0, height: s.screenHeight ?? 0, refreshRate: s.screenRefreshRate ?? 0, dpi: s.dpi ?? 96 };
  },
};

globalThis.__iron_focus = (hasFocus: boolean, isPaused: boolean) => {
  const focusChanged = hasFocus !== focused;
  const pauseChanged = isPaused !== paused;
  focused = hasFocus;
  paused = isPaused;
  // Unity's order: pausing, then focus.
  if (pauseChanged) {
    _forEachActiveComponent("onApplicationPause", (c) => c.onApplicationPause?.(isPaused));
  }
  if (focusChanged) {
    for (const f of [...focusListeners]) {
      try {
        f(hasFocus);
      } catch (e) {
        console.error(`Application.onFocusChanged: ${e}`);
      }
    }
    _forEachActiveComponent("onApplicationFocus", (c) => c.onApplicationFocus?.(hasFocus));
  }
};

/** Back to a focused, running game with no listeners. Only for tests. */
export function _resetApplicationForTests() {
  targetFrameRate = undefined;
  runInBackground = undefined;
  focused = true;
  paused = false;
  focusListeners.clear();
}
