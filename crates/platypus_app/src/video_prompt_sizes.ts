// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// The video-settings prompt's resolution list (pure, tested by `video_prompt_sizes.test.ts`). The
// host puts this text in front of `video_prompt.tsx`.

export type Size = { w: number; h: number; hz: number };

/** The sizes a mode offers, largest first (by area, then width, then rate), without repeats:
 * the monitor's modes, with their refresh rates for exclusive fullscreen; for a window only
 * the sizes smaller than the monitor (a window as big as the screen has no room for its frame). */
export function sizeList(
  modes: readonly { width: number; height: number; refreshRate: number }[],
  monitor: { width: number; height: number },
  mode: string,
): Size[] {
  const exclusive = mode === "ExclusiveFullScreen";
  const out: Size[] = [];
  for (const r of modes) {
    const s: Size = { w: r.width, h: r.height, hz: exclusive ? Math.round(r.refreshRate) : 0 };
    const fits = exclusive || monitor.width === 0 || (s.w < monitor.width && s.h < monitor.height);
    if (fits && !out.some((o) => o.w === s.w && o.h === s.h && o.hz === s.hz)) out.push(s);
  }
  out.sort((a, b) => b.w * b.h - a.w * a.h || b.w - a.w || b.hz - a.hz);
  return out;
}

/** The entry of `list` nearest to `want`: the same size if it's there (the closest rate),
 * else the closest area; -1 for an empty list. */
export function nearestIndex(list: readonly Size[], want: Size): number {
  let best = -1;
  let score = Infinity;
  for (let i = 0; i < list.length; i++) {
    const s = list[i]!;
    const d = Math.abs(s.w * s.h - want.w * want.h) * 1000 + Math.abs(s.w - want.w) * 10 + Math.abs(s.hz - want.hz);
    if (d < score) {
      score = d;
      best = i;
    }
  }
  return best;
}

export function sizeLabel(s: Size): string {
  return s.hz > 0 ? `${s.w} × ${s.h}  ${s.hz} Hz` : `${s.w} × ${s.h}`;
}
