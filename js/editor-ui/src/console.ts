// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// The console's model: log entries, the level filter and counts, search, collapsing
// repeats, and finding code links (`src/main.ts:12:5`) in messages.

export type LogLevel = "debug" | "info" | "warn" | "error";

export interface LogEntry {
  /** Unique and increasing (the engine's record number). */
  seq: number;
  level: LogLevel;
  /** Where it came from: `script` for `console.*`, a Rust module path for the engine. */
  target: string;
  message: string;
  /** Milliseconds since the log started. */
  time: number;
}

/** The engine's level numbers (0 trace ... 4 error) as console levels. */
export const levelOf = (n: number): LogLevel => (n >= 4 ? "error" : n === 3 ? "warn" : n === 2 ? "info" : "debug");

/** A run of message text, or a link to a place in a file. */
export type Segment = { text: string } | { text: string; file: string; line: number; col: number };

// `path/to/file.ext:line[:col]`, with Windows drives and backslashes too.
const LINK = /((?:[A-Za-z]:[\\/]|[\\/])?(?:[\w.\-@~]+[\\/])*[\w\-@~][\w.\-@~]*\.[A-Za-z][A-Za-z0-9]*):(\d+)(?::(\d+))?/g;

/** Splits a message into text and code links. URLs (`http://host.com:80`) aren't links. */
export function parseLinks(message: string): Segment[] {
  const out: Segment[] = [];
  let last = 0;
  for (const m of message.matchAll(LINK)) {
    const at = m.index ?? 0;
    // Right after a URL scheme (`http://`): a host and port, not a file.
    if (/[A-Za-z][\w+.-]*:\/*$/.test(message.slice(Math.max(0, at - 16), at))) continue;
    if (at > last) out.push({ text: message.slice(last, at) });
    out.push({ text: m[0], file: m[1]!, line: Number(m[2]), col: m[3] ? Number(m[3]) : 1 });
    last = at + m[0].length;
  }
  if (last < message.length) out.push({ text: message.slice(last) });
  return out;
}

export const isLink = (s: Segment): s is Extract<Segment, { file: string }> => "file" in s;

/** An entry as shown, with how many identical entries it stands for. */
export interface ConsoleRow {
  entry: LogEntry;
  count: number;
}

/** Identical entries (same level and message) shown once with a count, at the first one's place. */
export function collapse(entries: readonly LogEntry[]): ConsoleRow[] {
  const rows: ConsoleRow[] = [];
  const index = new Map<string, number>();
  for (const entry of entries) {
    const key = `${entry.level}\u0000${entry.message}`;
    const at = index.get(key);
    if (at === undefined) {
      index.set(key, rows.length);
      rows.push({ entry, count: 1 });
    } else {
      rows[at] = { entry: rows[at]!.entry, count: rows[at]!.count + 1 };
    }
  }
  return rows;
}

export type LevelCounts = Record<"info" | "warn" | "error", number>;

/** Entries per filter button (debug counts as info). */
export function countLevels(entries: readonly LogEntry[]): LevelCounts {
  const counts: LevelCounts = { info: 0, warn: 0, error: 0 };
  for (const e of entries) counts[e.level === "debug" ? "info" : e.level]++;
  return counts;
}

export interface ConsoleFilter {
  info: boolean;
  warn: boolean;
  error: boolean;
  search: string;
  collapse: boolean;
}

/** The rows to show for a filter. */
export function consoleRows(entries: readonly LogEntry[], filter: ConsoleFilter): ConsoleRow[] {
  const needle = filter.search.trim().toLowerCase();
  const shown = entries.filter((e) => {
    const level = e.level === "debug" ? "info" : e.level;
    if (!filter[level]) return false;
    return !needle || e.message.toLowerCase().includes(needle) || e.target.toLowerCase().includes(needle);
  });
  return filter.collapse ? collapse(shown) : shown.map((entry) => ({ entry, count: 1 }));
}
