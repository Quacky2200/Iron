// The virtual list's arithmetic: which rows to mount for a scroll position, where to
// scroll to show a row, and where keys move the selection. Rows have a fixed height.

/** Rows `start` (inclusive) to `end` (exclusive) are mounted. */
export interface Range {
  start: number;
  end: number;
}

/**
 * The rows that intersect the viewport, plus `overscan` rows on each side so a fast
 * scroll doesn't show gaps before the next render.
 */
export function visibleRange(scrollTop: number, viewport: number, rowHeight: number, count: number, overscan = 3): Range {
  if (count <= 0 || rowHeight <= 0) return { start: 0, end: 0 };
  const first = Math.floor(Math.max(0, scrollTop) / rowHeight);
  const last = Math.ceil((Math.max(0, scrollTop) + Math.max(0, viewport)) / rowHeight);
  return {
    start: Math.max(0, Math.min(count, first - overscan)),
    end: Math.max(0, Math.min(count, last + overscan)),
  };
}

/** The biggest scroll offset for `count` rows. */
export const maxScroll = (count: number, rowHeight: number, viewport: number): number =>
  Math.max(0, count * rowHeight - viewport);

/**
 * The scroll offset that shows row `index`: `"auto"` scrolls only as far as needed (like
 * `scrollIntoView({ block: "nearest" })`), `"start"`/`"center"`/`"end"` place it.
 */
export function scrollForIndex(
  index: number,
  rowHeight: number,
  viewport: number,
  scrollTop: number,
  count: number,
  align: "auto" | "start" | "center" | "end" = "auto",
): number {
  const top = index * rowHeight;
  const bottom = top + rowHeight;
  let target = scrollTop;
  if (align === "start") target = top;
  else if (align === "end") target = bottom - viewport;
  else if (align === "center") target = top - (viewport - rowHeight) / 2;
  else if (top < scrollTop) target = top;
  else if (bottom > scrollTop + viewport) target = bottom - viewport;
  return Math.max(0, Math.min(maxScroll(count, rowHeight, viewport), target));
}

/** Where a navigation key moves the selected index (null: not a navigation key). */
export function keyIndex(key: string, index: number, count: number, pageRows: number): number | null {
  if (count <= 0) return null;
  const page = Math.max(1, pageRows - 1);
  const clamp = (i: number) => Math.max(0, Math.min(count - 1, i));
  switch (key) {
    case "ArrowDown":
      return clamp(index < 0 ? 0 : index + 1);
    case "ArrowUp":
      return clamp(index < 0 ? count - 1 : index - 1);
    case "Home":
      return 0;
    case "End":
      return count - 1;
    case "PageDown":
      return clamp(index + page);
    case "PageUp":
      return clamp(index - page);
  }
  return null;
}
