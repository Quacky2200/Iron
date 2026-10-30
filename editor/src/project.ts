// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// The project folder for the editor (`platypus-app editor --project <folder>`): the host answers
// `Editor.listDir`, `copyIn` and `move` at once, `Editor.lastError` saying why one failed; the
// promises keep the browser's code the same whichever way the answers come. Paths are relative
// to the project, `/` separated, "" for its root.

export interface Listing {
  path: string;
  entries: ProjectEntry[];
}

/** The data type a project item carries while dragged inside the editor. */
export const PROJECT_PATH_TYPE = "application/x-platypus-project-path";

/** The data type a Hierarchy row carries while dragged (its object's id). */
export const OBJECT_TYPE = "application/x-platypus-object";

/** Why the last call failed, as an error (null when it worked). */
function failure(): Error | null {
  const error = Editor.lastError;
  return error === "" ? null : new Error(error);
}

/** A folder's entries, folders first. */
export function listDir(path: string): Promise<Listing> {
  const entries = Editor.listDir(path);
  const e = failure();
  return e ? Promise.reject(e) : Promise.resolve({ path, entries });
}

/** Copies files dropped on the window into a project folder (renaming instead of overwriting). */
export function copyIn(files: readonly string[], dir: string): Promise<{ created: string[] }> {
  const created = Editor.copyIn(files, dir);
  const e = failure();
  return e ? Promise.reject(e) : Promise.resolve({ created });
}

/** Moves a project file or folder into another project folder. */
export function moveInto(path: string, dir: string): Promise<{ path: string }> {
  const moved = Editor.move(path, dir);
  const e = failure();
  return e ? Promise.reject(e) : Promise.resolve({ path: moved });
}

// Folders a file pick never looks in: build output, the editor's own data, generated types.
const SKIPPED = ["build", ".platypus", "types", "node_modules"];

/** The project's files that `fits` takes, folder by folder from the root (at most `limit`). */
export function projectFiles(fits: (path: string) => boolean, limit = 500): string[] {
  const out: string[] = [];
  const dirs = [""];
  while (dirs.length > 0 && out.length < limit) {
    const dir = dirs.shift()!;
    for (const e of Editor.listDir(dir)) {
      const path = joinPath(dir, e.name);
      if (e.dir) {
        if (!(dir === "" && SKIPPED.includes(e.name))) dirs.push(path);
      } else if (fits(path) && out.length < limit) out.push(path);
    }
  }
  return out;
}

/** The project folder's full path, if the editor has one. */
export function projectFolder(): string | null {
  return Editor.isAvailable ? Editor.project : null;
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
