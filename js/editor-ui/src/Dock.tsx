// SPDX-License-Identifier: Apache-2.0

// Docking panels: splits with draggable splitters, tab groups, and tabs dragged onto
// another group's middle (as a tab) or edges (a new split). The layout is data
// (dock.ts) the parent keeps, so it can be saved and loaded as JSON. Drags use pointer
// capture; where things are comes from `computeRects`, matching the flex layout below.

import { type IronElement, type IronPointerEvent, Text } from "@iron/react";
import { type ReactNode, useMemo, useRef, useState } from "react";

import {
  activate,
  computeRects,
  type DockGroup,
  type DockNode,
  type DockSplit,
  type DockZone,
  dockTab,
  dropTarget,
  type Rect,
  resize,
  SPLITTER,
  TAB_BAR,
  zoneRect,
} from "./dock.ts";

/** A dockable panel. */
export interface DockPanel {
  title: string;
  /**
   * The panel's content, given its box below the tab strip: the size, and where it is
   * on screen in UI pixels (for a camera drawing into it).
   */
  render: (box: Rect) => ReactNode;
  /** No background: what's drawn under the UI (a camera's view) shows through. */
  seeThrough?: boolean;
}

export interface DockProps {
  layout: DockNode;
  onLayoutChange: (layout: DockNode) => void;
  panels: Record<string, DockPanel>;
  /** Where the dock is on screen, in UI pixels (it fills this rect). */
  rect: Rect;
}

/** Pixels a tab must move before a press becomes a drag. */
const DRAG_START = 5;

interface TabDrag {
  tab: string;
  x0: number;
  y0: number;
  dragging: boolean;
  x: number;
  y: number;
  target: { group: string; zone: DockZone } | null;
}

export function Dock({ layout, onLayoutChange, panels, rect }: DockProps) {
  const rects = useMemo(() => computeRects(layout, rect), [layout, rect.x, rect.y, rect.width, rect.height]);
  const [drag, setDragState] = useState<TabDrag | null>(null);
  // The drag as of the last event (moves render a tick later; the release needs it now).
  const dragRef = useRef<TabDrag | null>(null);
  const setDrag = (d: TabDrag | null) => {
    dragRef.current = d;
    setDragState(d);
  };
  // Splitter drags measure from where they started.
  const splitDrag = useRef<{ layout: DockNode; split: string; index: number; at: number; free: number } | null>(null);

  const renderNode = (node: DockNode): ReactNode =>
    node.type === "split" ? renderSplit(node) : renderGroup(node);

  const renderSplit = (node: DockSplit) => {
    const row = node.direction === "row";
    const r = rects.get(node.id)!;
    const free = Math.max(1, (row ? r.width : r.height) - SPLITTER * (node.children.length - 1));
    const out: ReactNode[] = [];
    node.children.forEach((child, i) => {
      if (i > 0) {
        out.push(
          <Splitter
            key={`split-${child.id}`}
            row={row}
            onStart={(at) => {
              splitDrag.current = { layout, split: node.id, index: i - 1, at, free };
            }}
            onMove={(at) => {
              const d = splitDrag.current;
              if (!d) return;
              onLayoutChange(resize(d.layout, d.split, d.index, (at - d.at) / d.free));
            }}
            onEnd={() => {
              splitDrag.current = null;
            }}
          />,
        );
      }
      out.push(
        <box
          key={child.id}
          className="iron-dock-cell"
          style={{ flexGrow: node.sizes[i], flexShrink: 1, flexBasis: 0, minWidth: 0, minHeight: 0 }}
        >
          {renderNode(child)}
        </box>,
      );
    });
    return (
      <box className="iron-dock-split" style={{ flexDirection: row ? "row" : "column", flexGrow: 1, minWidth: 0, minHeight: 0 }}>
        {out}
      </box>
    );
  };

  const renderGroup = (g: DockGroup) => {
    const r = rects.get(g.id)!;
    const box = { x: r.x, y: r.y + TAB_BAR, width: r.width, height: Math.max(0, r.height - TAB_BAR) };
    const panel = g.active ? panels[g.active] : undefined;
    return (
      <box className={panel?.seeThrough ? "iron-dock-group see-through" : "iron-dock-group"}>
        <box className="iron-dock-tabs" style={{ height: TAB_BAR }}>
          {g.tabs.map((tab) => (
            <DockTab
              key={tab}
              title={panels[tab]?.title ?? tab}
              active={tab === g.active}
              dragging={drag?.dragging === true && drag.tab === tab}
              onPress={(x, y) => {
                if (tab !== g.active) onLayoutChange(activate(layout, tab));
                setDrag({ tab, x0: x, y0: y, x, y, dragging: false, target: null });
              }}
              onMove={(x, y) => {
                const d = dragRef.current;
                if (!d) return;
                const dragging = d.dragging || Math.hypot(x - d.x0, y - d.y0) > DRAG_START;
                setDrag({ ...d, x, y, dragging, target: dragging ? dropTarget(layout, rects, x, y) : null });
              }}
              onRelease={() => {
                const d = dragRef.current;
                if (d?.dragging && d.target) {
                  onLayoutChange(dockTab(layout, d.tab, d.target.group, d.target.zone));
                }
                setDrag(null);
              }}
            />
          ))}
        </box>
        <box className="iron-dock-content" style={{ flexGrow: 1, minHeight: 0, overflow: "hidden" }}>
          {panel ? panel.render(box) : null}
        </box>
      </box>
    );
  };

  // The drop preview and the dragged tab's ghost.
  let preview: ReactNode = null;
  if (drag?.dragging) {
    const target = drag.target;
    const g = target ? rects.get(target.group) : undefined;
    const zone = g && target ? zoneRect(g, target.zone) : null;
    preview = (
      <>
        {zone ? (
          <box
            className="iron-dock-drop"
            style={{ left: zone.x - rect.x, top: zone.y - rect.y, width: zone.width, height: zone.height }}
          />
        ) : null}
        <box className="iron-dock-ghost" style={{ left: drag.x - rect.x + 12, top: drag.y - rect.y + 8 }}>
          <Text>{panels[drag.tab]?.title ?? drag.tab}</Text>
        </box>
      </>
    );
  }

  return (
    <box className="iron-dock" style={{ width: rect.width, height: rect.height, position: "relative" }}>
      {renderNode(layout)}
      {preview}
    </box>
  );
}

function Splitter({ row, onStart, onMove, onEnd }: { row: boolean; onStart: (at: number) => void; onMove: (at: number) => void; onEnd: () => void }) {
  const el = useRef<IronElement>(null);
  const active = useRef(false);
  return (
    <box
      ref={el}
      className={row ? "iron-splitter vertical" : "iron-splitter horizontal"}
      style={row ? { width: SPLITTER, cursor: "col-resize" } : { height: SPLITTER, cursor: "row-resize" }}
      onPointerDown={(e: IronPointerEvent) => {
        if (e.button !== 0) return;
        e.preventDefault();
        active.current = true;
        el.current?.setPointerCapture();
        onStart(row ? e.clientX : e.clientY);
      }}
      onPointerMove={(e: IronPointerEvent) => {
        if (active.current) onMove(row ? e.clientX : e.clientY);
      }}
      onPointerUp={() => {
        active.current = false;
        el.current?.releasePointerCapture();
        onEnd();
      }}
      onLostPointerCapture={() => {
        if (!active.current) return;
        active.current = false;
        onEnd();
      }}
    />
  );
}

function DockTab({
  title,
  active,
  dragging,
  onPress,
  onMove,
  onRelease,
}: {
  title: string;
  active: boolean;
  dragging: boolean;
  onPress: (x: number, y: number) => void;
  onMove: (x: number, y: number) => void;
  onRelease: () => void;
}) {
  const el = useRef<IronElement>(null);
  const pressed = useRef(false);
  const classes = ["iron-dock-tab", active && "active", dragging && "dragging"].filter(Boolean).join(" ");
  return (
    <box
      ref={el}
      className={classes}
      onPointerDown={(e: IronPointerEvent) => {
        if (e.button !== 0) return;
        pressed.current = true;
        // Moves keep coming to the tab wherever the pointer goes.
        el.current?.setPointerCapture();
        onPress(e.clientX, e.clientY);
      }}
      onPointerMove={(e: IronPointerEvent) => {
        if (pressed.current) onMove(e.clientX, e.clientY);
      }}
      onPointerUp={() => {
        if (!pressed.current) return;
        pressed.current = false;
        el.current?.releasePointerCapture();
        onRelease();
      }}
    >
      <Text>{title}</Text>
    </box>
  );
}
