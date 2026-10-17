// The console panel: the engine's and scripts' log with level colours, filter buttons
// with counts, collapsing repeats, search, auto-scroll that pauses while you look
// further up, and code links (`src/main.ts:12:5`) that open in your code editor.

import { Input, type IronPointerEvent, Text } from "@iron/react";
import { useEffect, useMemo, useRef, useState, useSyncExternalStore } from "react";

import { type ConsoleFilter, consoleRows, countLevels, isLink, type LogEntry, parseLinks } from "./console.ts";
import { clearLog, logEntries, openInEditor, subscribeLog } from "./log.ts";
import { VirtualList, type VirtualListHandle } from "./VirtualList.tsx";

export interface ConsoleProps {
  width: number;
  height: number;
  /** Entries to show instead of the engine's log (for tests and other feeds). */
  entries?: readonly LogEntry[];
  rowHeight?: number;
}

const TOOLBAR = 28;

function Message({ text }: { text: string }) {
  const segments = useMemo(() => parseLinks(text), [text]);
  // Separate text elements, not spans: the link must be its own hit target.
  return (
    <box className="iron-console-message">
      {segments.map((s, i) =>
        isLink(s) ? (
          <Text
            key={i}
            className="iron-console-link"
            onClick={(e: IronPointerEvent) => {
              e.stopPropagation();
              openInEditor(s.file, s.line, s.col);
            }}
          >
            {s.text}
          </Text>
        ) : (
          <Text key={i}>{s.text}</Text>
        ),
      )}
    </box>
  );
}

function FilterButton({ label, count, on, level, toggle }: { label: string; count?: number; on: boolean; level?: string; toggle: () => void }) {
  return (
    <box className={on ? "iron-console-filter on" : "iron-console-filter"} data-level={level ?? ""} onClick={toggle}>
      {level ? <box className="iron-console-dot" /> : null}
      <Text>{count === undefined ? label : `${label} ${count > 999 ? "999+" : count}`}</Text>
    </box>
  );
}

export function Console({ width, height, entries: given, rowHeight = 20 }: ConsoleProps) {
  const live = useSyncExternalStore(subscribeLog, logEntries);
  const entries = given ?? live;
  const [filter, setFilter] = useState<ConsoleFilter>({ info: true, warn: true, error: true, search: "", collapse: false });
  const [selected, setSelected] = useState(-1);
  // Follows new lines until scrolled away from the bottom, and again once back there.
  const [follow, setFollow] = useState(true);
  const list = useRef<VirtualListHandle>(null);

  const counts = useMemo(() => countLevels(entries), [entries]);
  const rows = useMemo(() => consoleRows(entries, filter), [entries, filter]);
  const listHeight = Math.max(0, height - TOOLBAR);

  useEffect(() => {
    if (follow) list.current?.scrollToEnd();
  }, [rows.length, follow, listHeight]);

  const flip = (key: "info" | "warn" | "error" | "collapse") => setFilter((f) => ({ ...f, [key]: !f[key] }));

  return (
    <box className="iron-console" style={{ width, height }}>
      <box className="iron-console-toolbar" style={{ height: TOOLBAR }}>
        <box className="iron-console-button" onClick={() => clearLog()}>
          <Text>Clear</Text>
        </box>
        <FilterButton label="Collapse" on={filter.collapse} toggle={() => flip("collapse")} />
        <Input
          className="iron-console-search"
          placeholder="Search"
          value={filter.search}
          onChange={(e) => setFilter((f) => ({ ...f, search: e.value }))}
        />
        <box style={{ flexGrow: 1 }} />
        <FilterButton label="Info" level="info" count={counts.info} on={filter.info} toggle={() => flip("info")} />
        <FilterButton label="Warn" level="warn" count={counts.warn} on={filter.warn} toggle={() => flip("warn")} />
        <FilterButton label="Error" level="error" count={counts.error} on={filter.error} toggle={() => flip("error")} />
      </box>
      <VirtualList
        ref={list}
        className="iron-console-list"
        count={rows.length}
        rowHeight={rowHeight}
        height={listHeight}
        selected={selected}
        onSelect={setSelected}
        onScroll={(_top, atEnd) => setFollow(atEnd)}
        empty="No messages"
        renderRow={(i) => {
          const { entry, count } = rows[i]!;
          return (
            <box className="iron-console-row" data-level={entry.level}>
              <box className="iron-console-dot" />
              <Text className="iron-console-time">{(entry.time / 1000).toFixed(2)}</Text>
              <Message text={entry.message.split("\n")[0]!} />
              {count > 1 ? <Text className="iron-console-count">{String(count)}</Text> : null}
            </box>
          );
        }}
      />
    </box>
  );
}
