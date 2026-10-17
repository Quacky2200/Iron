// A list that mounts only the rows in view (plus a few each side), at a fixed row
// height, so lists of a million rows cost the same as ten. The engine scrolls the
// container; rows are placed absolutely inside a spacer as tall as the whole list.

import { type IronElement, type IronKeyboardEvent, type IronScrollEvent, type IronStyle, Text } from "@iron/react";
import { type ReactNode, type Ref, useImperativeHandle, useRef, useState } from "react";

import { keyIndex, maxScroll, scrollForIndex, visibleRange } from "./virtual.ts";

export interface VirtualListHandle {
  /** Scrolls so row `index` shows (`auto`: only as far as needed). */
  scrollToIndex(index: number, align?: "auto" | "start" | "center" | "end"): void;
  /** Scrolls to the last row. */
  scrollToEnd(): void;
  /** Focuses the list (keys go to it). */
  focus(): void;
  readonly scrollTop: number;
}

export interface VirtualListProps {
  count: number;
  rowHeight: number;
  /** The visible height in UI pixels (the list doesn't measure itself). */
  height: number;
  renderRow: (index: number) => ReactNode;
  /** The selected row: arrows, Page Up/Down, Home and End move it (unless `keyboard` is false). */
  selected?: number;
  onSelect?: (index: number) => void;
  /** Whether the list moves the selection with keys itself (a tree view handles them instead). */
  keyboard?: boolean;
  overscan?: number;
  /** After every scroll: the offset and whether the end is in view. */
  onScroll?: (scrollTop: number, atEnd: boolean) => void;
  onKeyDown?: (event: IronKeyboardEvent) => void;
  className?: string;
  style?: IronStyle;
  /** Shown when there are no rows. */
  empty?: ReactNode;
  ref?: Ref<VirtualListHandle>;
}

export function VirtualList({
  count,
  rowHeight,
  height,
  renderRow,
  selected = -1,
  onSelect,
  keyboard = true,
  overscan = 4,
  onScroll,
  onKeyDown,
  className,
  style,
  empty,
  ref,
}: VirtualListProps) {
  const box = useRef<IronElement>(null);
  const [scrollTop, setScrollTop] = useState(0);
  // The latest offset, also between renders (scrollTo's event arrives a frame later).
  const top = useRef(0);

  const scrollTo = (y: number) => {
    const clamped = Math.max(0, Math.min(maxScroll(count, rowHeight, height), y));
    if (Math.abs(clamped - top.current) < 0.5) return;
    top.current = clamped;
    setScrollTop(clamped);
    box.current?.scrollTo(0, clamped);
  };
  const scrollToIndex = (index: number, align: "auto" | "start" | "center" | "end" = "auto") =>
    scrollTo(scrollForIndex(index, rowHeight, height, top.current, count, align));

  useImperativeHandle(
    ref,
    () => ({
      scrollToIndex,
      scrollToEnd: () => scrollTo(maxScroll(count, rowHeight, height)),
      focus: () => box.current?.focus(),
      get scrollTop() {
        return top.current;
      },
    }),
    [count, rowHeight, height],
  );

  const { start, end } = visibleRange(scrollTop, height, rowHeight, count, overscan);
  const rows: ReactNode[] = [];
  for (let i = start; i < end; i++) {
    rows.push(
      <box
        key={i}
        className={i === selected ? "iron-vrow selected" : "iron-vrow"}
        style={{ position: "absolute", left: 0, right: 0, top: i * rowHeight, height: rowHeight }}
        onPointerDown={() => onSelect?.(i)}
      >
        {renderRow(i)}
      </box>,
    );
  }

  return (
    <box
      ref={box}
      className={className ? `iron-vlist ${className}` : "iron-vlist"}
      tabIndex={0}
      style={{ overflowY: "auto", overflowX: "hidden", height, ...style }}
      onScroll={(e: IronScrollEvent) => {
        top.current = e.scrollTop;
        setScrollTop(e.scrollTop);
        onScroll?.(e.scrollTop, e.scrollTop >= maxScroll(count, rowHeight, height) - 1);
      }}
      onKeyDown={(e: IronKeyboardEvent) => {
        onKeyDown?.(e);
        if (e.defaultPrevented || !keyboard) return;
        const to = keyIndex(e.key, selected, count, Math.floor(height / rowHeight));
        if (to === null) return;
        e.preventDefault();
        onSelect?.(to);
        scrollToIndex(to);
      }}
    >
      <box style={{ position: "relative", height: count * rowHeight, flexShrink: 0, width: "100%" }}>{rows}</box>
      {count === 0 && empty ? (
        <box className="iron-vlist-empty" style={{ position: "absolute", left: 0, right: 0, top: 0 }}>
          {typeof empty === "string" ? <Text>{empty}</Text> : empty}
        </box>
      ) : null}
    </box>
  );
}
