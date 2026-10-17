// The game and the machine it runs on (Unity's `Application` and `SystemInfo`). The
// engine installs `globalThis.__iron_info` before the game's code runs: the game's
// manifest (its `package.json`: `name`, `version` and an `iron` section with `title`,
// `company`, `identifier` and `icon`) and what the OS and GPU report.

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
}

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
