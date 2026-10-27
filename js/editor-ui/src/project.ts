// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// The project folder for the editor: `platypus_editor [project]` answers `editor.listDir`,
// `editor.copyIn` and `editor.move` through `__platypus_editor_fs`, a frame later. Paths
// are relative to the project, `/` separated, "" for its root.

import { op } from "@platypus/engine";

export interface ProjectEntry {
  name: string;
  dir: boolean;
  /** Bytes (0 for folders). */
  size: number;
}

export interface Listing {
  path: string;
  entries: ProjectEntry[];
}

/** The data type a project item carries while dragged inside the editor. */
export const PROJECT_PATH_TYPE = "application/x-platypus-project-path";

let nextId = 1;
const pending = new Map<number, { resolve: (v: unknown) => void; reject: (e: Error) => void }>();

declare global {
  /** Answers to `editor.*` file ops, flat: `[id, ok, json or error, ...]` (platypus_editor only). */
  var __platypus_editor_fs: (flat: unknown[]) => void;
}

globalThis.__platypus_editor_fs = (flat: unknown[]) => {
  for (let i = 0; i + 2 < flat.length; i += 3) {
    const id = Number(flat[i]);
    const waiting = pending.get(id);
    if (!waiting) continue;
    pending.delete(id);
    const text = String(flat[i + 2]);
    if (flat[i + 1] === true) {
      try {
        waiting.resolve(JSON.parse(text));
      } catch (e) {
        waiting.reject(e instanceof Error ? e : new Error(String(e)));
      }
    } else {
      waiting.reject(new Error(text));
    }
  }
};

function request<T>(name: string, ...args: unknown[]): Promise<T> {
  const id = nextId++;
  return new Promise<T>((resolve, reject) => {
    pending.set(id, { resolve: resolve as (v: unknown) => void, reject });
    op(`editor.${name}`, id, ...args);
  });
}

/** A folder's entries, folders first. */
export const listDir = (path: string): Promise<Listing> => request("listDir", path);

/** Copies files dropped on the window into a project folder (renaming instead of overwriting). */
export const copyIn = (files: readonly string[], dir: string): Promise<{ created: string[] }> =>
  request("copyIn", [...files], dir);

/** Moves a project file or folder into another project folder. */
export const moveInto = (path: string, dir: string): Promise<{ path: string }> => request("move", path, dir);

/** The project folder's full path, if the editor has one. */
export function projectFolder(): string | null {
  const app = (globalThis.__platypus_info as { app?: { project?: string | null } } | undefined)?.app;
  return app?.project ?? null;
}

/** `dir/name`, or `name` at the root. */
export const joinPath = (dir: string, name: string): string => (dir ? `${dir}/${name}` : name);

/** The folder holding `path` ("" for top-level entries). */
export function parentPath(path: string): string {
  const i = path.lastIndexOf("/");
  return i < 0 ? "" : path.slice(0, i);
}

/** The last part of a path (`\` or `/` separated). */
export const baseName = (path: string): string => path.split(/[\\/]/).filter(Boolean).pop() ?? path;

/** Breadcrumbs from the root (named `rootName`) to `path`. */
export function crumbs(path: string, rootName: string): { name: string; path: string }[] {
  const out = [{ name: rootName, path: "" }];
  let at = "";
  for (const part of path.split("/").filter(Boolean)) {
    at = joinPath(at, part);
    out.push({ name: part, path: at });
  }
  return out;
}

/**
 * Whether a project item at `from` can be moved into folder `dir`: not into itself or
 * a folder inside it, and not where it already is.
 */
export function canMoveInto(from: string, dir: string): boolean {
  if (!from) return false;
  if (dir === from || dir.startsWith(`${from}/`)) return false;
  return parentPath(from) !== dir;
}

/** A file size as people read it: `512 B`, `1.5 KB`, `12 MB`. */
export function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  const units = ["KB", "MB", "GB", "TB"];
  let v = bytes / 1024;
  let i = 0;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i++;
  }
  return `${v < 10 ? v.toFixed(1) : Math.round(v)} ${units[i]}`;
}

/** For tests. */
export function _resetProjectForTests(): void {
  pending.clear();
  nextId = 1;
}
