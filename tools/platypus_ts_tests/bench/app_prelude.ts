// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// The manifest and the machine as the engine installs them, and the native API's flat screen
// properties over the original `SystemInfo.screen`.
import { SystemInfo as S } from "../../../js/engine/src/app.ts";

(globalThis as any).__platypus_op = () => {};
(globalThis as any).__platypus_info = {
  app: { name: "demo", productName: "Demo Game", version: "1.2.3", companyName: "Acme", identifier: "com.acme.demo", engineVersion: "0.9.1", isEditor: false, targetFrameRate: 144, runInBackground: false },
  system: { operatingSystem: "Windows 11 Pro", operatingSystemFamily: "Windows", operatingSystemVersion: "10.0.22631", deviceType: "Desktop", deviceName: "PC-1", processorType: "Ryzen", processorCount: 16, systemMemorySize: 32768, architecture: "x86_64", language: "en-GB", touchSupported: false, graphicsDeviceName: "RTX", graphicsDeviceVendorID: 4318, graphicsDeviceType: "Vulkan", graphicsDeviceKind: "DiscreteGpu", graphicsDriver: "555.1", screenWidth: 2560, screenHeight: 1440, screenRefreshRate: 165, dpi: 109 },
};
(globalThis as any).SystemInfo = Object.create(S, {
  screenWidth: { get: () => S.screen.width },
  screenHeight: { get: () => S.screen.height },
  screenRefreshRate: { get: () => S.screen.refreshRate },
  screenDpi: { get: () => S.screen.dpi },
});
