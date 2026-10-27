// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// Application and SystemInfo, compiled against the native API and run in bun against
// js/engine/src/app.ts. The prelude sets the game's manifest and the machine.

export function manifest(): string {
  return `${Application.name}|${Application.productName}|${Application.version}|${Application.companyName}|${Application.identifier}|${Application.engineVersion}|${Application.platform}|${Application.systemLanguage}|${Application.isEditor}|${Application.isMobilePlatform}`;
}

export function machine(): string {
  return `${SystemInfo.operatingSystem}|${SystemInfo.operatingSystemFamily}|${SystemInfo.operatingSystemVersion}|${SystemInfo.deviceType}|${SystemInfo.deviceName}|${SystemInfo.processorType}|${SystemInfo.processorCount}|${SystemInfo.systemMemorySize}|${SystemInfo.architecture}|${SystemInfo.graphicsDeviceName}|${SystemInfo.graphicsDeviceVendorID}|${SystemInfo.graphicsDeviceType}|${SystemInfo.graphicsDeviceKind}|${SystemInfo.graphicsDriver}|${SystemInfo.touchSupported}|${SystemInfo.screenWidth}|${SystemInfo.screenHeight}|${SystemInfo.screenRefreshRate}|${SystemInfo.screenDpi}`;
}

export function settable(): string {
  const before = `${Application.targetFrameRate},${Application.runInBackground}`;
  Application.targetFrameRate = 59.6;
  Application.runInBackground = true;
  const mid = `${Application.targetFrameRate},${Application.runInBackground}`;
  Application.targetFrameRate = 0;
  return `${before};${mid};${Application.targetFrameRate}`;
}
