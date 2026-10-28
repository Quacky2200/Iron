// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

import { describe, expect, test } from "bun:test";
import { nearestIndex, sizeLabel, sizeList } from "./video_prompt_sizes";

const monitor = { width: 2560, height: 1440 };
const modes = [
  { width: 1280, height: 720, refreshRate: 60 },
  { width: 2560, height: 1440, refreshRate: 144 },
  { width: 1920, height: 1080, refreshRate: 60 },
  { width: 2560, height: 1440, refreshRate: 60 },
  { width: 1920, height: 1080, refreshRate: 144 },
  { width: 1600, height: 900, refreshRate: 59.94 },
  { width: 1920, height: 1080, refreshRate: 144 },
];

describe("video settings prompt: resolution list", () => {
  // M9-VSP-11
  test("sizes are de-duplicated and sorted largest first; a window only gets sizes below the monitor's", () => {
    expect(sizeList(modes, monitor, "Windowed")).toEqual([
      { w: 1920, h: 1080, hz: 0 },
      { w: 1600, h: 900, hz: 0 },
      { w: 1280, h: 720, hz: 0 },
    ]);
    expect(sizeList(modes, monitor, "ExclusiveFullScreen").map(sizeLabel)).toEqual([
      "2560 × 1440  144 Hz",
      "2560 × 1440  60 Hz",
      "1920 × 1080  144 Hz",
      "1920 × 1080  60 Hz",
      "1600 × 900  60 Hz",
      "1280 × 720  60 Hz",
    ]);
    // The monitor isn't known yet: nothing is left out.
    expect(sizeList(modes, { width: 0, height: 0 }, "Windowed")).toHaveLength(4);
  });

  // M9-VSP-11
  test("the saved size is selected, else the nearest one", () => {
    const windowed = sizeList(modes, monitor, "Windowed");
    expect(nearestIndex(windowed, { w: 1600, h: 900, hz: 0 })).toBe(1);
    // 1700 x 960 isn't offered: the nearest by area.
    expect(nearestIndex(windowed, { w: 1700, h: 960, hz: 0 })).toBe(1);
    expect(nearestIndex(windowed, { w: 4000, h: 3000, hz: 0 })).toBe(0);
    const exclusive = sizeList(modes, monitor, "ExclusiveFullScreen");
    expect(exclusive[nearestIndex(exclusive, { w: 1920, h: 1080, hz: 60 })]).toEqual({ w: 1920, h: 1080, hz: 60 });
    expect(nearestIndex([], { w: 1, h: 1, hz: 0 })).toBe(-1);
  });
});
