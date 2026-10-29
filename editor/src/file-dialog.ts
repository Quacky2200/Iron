// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// The open / save dialog's logic (FileDialog.tsx draws it): which entries show under a filter,
// the extension a saved name gets, and typed paths made into project paths. Paths are relative
// to the project, `/` separated, "" for its root.

export type DialogMode = "open" | "save" | "folder";

/** A choice in the dialog's type list: file names ending in one of `extensions` (none: all files). */
export interface FileFilter {
  label: string;
  extensions: string[];
}

export const SCENE_FILTER: FileFilter = { label: "Scenes (*.scene.json)", extensions: [".scene.json"] };
export const PREFAB_FILTER: FileFilter = { label: "Prefabs (*.prefab.json)", extensions: [".prefab.json"] };
export const MODEL_FILTER: FileFilter = { label: "Models (*.glb, *.gltf, *.fbx)", extensions: [".glb", ".gltf", ".fbx"] };
export const ALL_FILES: FileFilter = { label: "All files", extensions: [] };

/** Whether a file name passes a filter (case-insensitive). */
export function matches(name: string, filter: FileFilter): boolean {
  if (filter.extensions.length === 0) return true;
  const lower = name.toLowerCase();
  return filter.extensions.some((e) => lower.endsWith(e));
}

/** The entries a dialog lists: folders always, files that pass the filter (none when picking a folder). */
export function shownEntries(entries: readonly ProjectEntry[], filter: FileFilter, mode: DialogMode): ProjectEntry[] {
  return entries.filter((e) => e.dir || (mode !== "folder" && matches(e.name, filter)));
}

/** A name to save as: given the filter's first extension when it has none of them. */
export function withExtension(name: string, filter: FileFilter): string {
  const trimmed = name.trim();
  if (trimmed === "" || filter.extensions.length === 0 || matches(trimmed, filter)) return trimmed;
  // `level.json` under scenes becomes `level.scene.json`, not `level.json.scene.json`.
  const first = filter.extensions[0]!;
  const last = first.lastIndexOf(".");
  const tail = first.slice(last);
  const base = last > 0 && trimmed.toLowerCase().endsWith(tail) ? trimmed.slice(0, trimmed.length - tail.length) : trimmed;
  return `${base}${first}`;
}

/**
 * A path typed in the dialog as a project path: relative to `dir`, or to the project's root when
 * it starts with `/`; `.` and `..` are worked out. "" for the root, or null when it would leave
 * the project or names a drive.
 */
export function resolveTyped(dir: string, typed: string): string | null {
  const text = typed.trim().split("\\").join("/");
  if (/^[A-Za-z]:/.test(text)) return null;
  const parts: string[] = text.startsWith("/") ? [] : dir.split("/").filter((p) => p !== "");
  for (const part of text.split("/")) {
    if (part === "" || part === ".") continue;
    if (part === "..") {
      if (parts.length === 0) return null;
      parts.pop();
    } else parts.push(part);
  }
  return parts.join("/");
}

/** A file name without the filter's extension, for the name field (`main.scene.json` -> `main`). */
export function stemOf(name: string, filter: FileFilter): string {
  const lower = name.toLowerCase();
  for (const e of filter.extensions) if (lower.endsWith(e)) return name.slice(0, name.length - e.length);
  return name;
}
