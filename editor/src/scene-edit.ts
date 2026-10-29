// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// Edits of the scene as undoable commands (history.ts). Undoing a delete builds the objects
// again, with new ids, so commands don't keep `GameObject`s: they keep where an object is (its
// path of indices among the shown objects, `Loc`), which undo and redo restore exactly.
// Deleted and copied objects are snapshots (`EditorScene.snapshot`, components and children
// included); references from outside a deleted object to it don't come back.

import { type Command, group } from "./history.ts";

/** Where an object is: its index among the shown siblings at each level, from the root. */
export type Loc = number[];

/** Shown in the Hierarchy (and so taking part in edits). */
export const shown = (go: GameObject): boolean => !go.destroyed && (go.hideFlags & HideFlags.HideInHierarchy) === 0;

/** The shown children of `parent` (the shown roots for none). */
export function shownChildren(parent: GameObject | null): GameObject[] {
  return parent ? parent.children.filter(shown) : GameObject.roots.filter(shown);
}

/** Where an object is now. */
export function locate(go: GameObject): Loc {
  const loc: Loc = [];
  let at: GameObject | null = go;
  while (at) {
    const parent: GameObject | null = at.parent;
    const self: GameObject = at;
    loc.unshift(shownChildren(parent).findIndex((c) => c.id === self.id));
    at = parent;
  }
  return loc;
}

/** The object at a place, if there is one. */
export function find(loc: Loc): GameObject | null {
  let at: GameObject | null = null;
  for (const i of loc) {
    const next: GameObject | undefined = shownChildren(at)[i];
    if (!next) return null;
    at = next;
  }
  return at;
}

/** The sibling index (counting hidden objects) that puts an object at shown place `index`. */
function rawIndex(parent: GameObject | null, index: number): number {
  const kids = shownChildren(parent);
  const before = kids[index];
  if (!before) return 1_000_000;
  return before.transform.siblingIndex;
}

/** Builds a snapshot at a place: under the object at `parentLoc` (none: a root) at shown `index`. */
export function restoreAt(data: string, parentLoc: Loc | null, index: number): GameObject | null {
  const parent = parentLoc ? find(parentLoc) : null;
  if (parentLoc && !parent) return null;
  const made = EditorScene.restore(data, parent ?? noObject(), rawIndex(parent, index));
  return made.exists() ? made : null;
}

/** "No object", where the engine takes one (the world root for a parent). */
export const noObject = (): GameObject => GameObject.byId(0);

const parentLocOf = (loc: Loc): Loc | null => (loc.length > 1 ? loc.slice(0, loc.length - 1) : null);
const indexOf = (loc: Loc): number => loc[loc.length - 1] ?? 0;

/** A local or world pose. */
export interface Pose {
  position: Vector3;
  rotation: Quaternion;
  scale: Vector3;
}

function setPose(go: GameObject, p: Pose): void {
  go.transform.position = p.position;
  go.transform.rotation = p.rotation;
  go.transform.localScale = p.scale;
}

/** A finished handle drag (world position and rotation, local scale). */
export function poseCommand(go: GameObject, before: Pose, after: Pose, label = "Move"): Command {
  const loc = locate(go);
  return {
    label: `${label} ${go.name}`,
    run: () => {
      const o = find(loc);
      if (o) setPose(o, after);
    },
    undo: () => {
      const o = find(loc);
      if (o) setPose(o, before);
    },
  };
}

export function renameCommand(go: GameObject, name: string): Command {
  const loc = locate(go);
  const before = go.name;
  return {
    label: `Rename ${before}`,
    run: () => {
      const o = find(loc);
      if (o) o.name = name;
    },
    undo: () => {
      const o = find(loc);
      if (o) o.name = before;
    },
  };
}

export function enableCommand(go: GameObject, enabled: boolean): Command {
  const loc = locate(go);
  const before = go.enabled;
  return {
    label: `${enabled ? "Enable" : "Disable"} ${go.name}`,
    run: () => {
      const o = find(loc);
      if (o) o.enabled = enabled;
    },
    undo: () => {
      const o = find(loc);
      if (o) o.enabled = before;
    },
  };
}

/** Builds a snapshot at a place; undo destroys it again. */
export function createCommand(label: string, data: string, parentLoc: Loc | null, index: number): Command {
  const loc: Loc = parentLoc ? [...parentLoc, index] : [index];
  return {
    label,
    run: () => {
      restoreAt(data, parentLoc, index);
    },
    undo: () => {
      find(loc)?.destroy();
    },
  };
}

/** Makes a new root object with `make` (again on redo, as the last root); undo destroys it. */
export function makeCommand(label: string, make: () => GameObject): Command {
  let loc: Loc = [];
  return {
    label,
    run: () => {
      const go = make();
      loc = locate(go);
    },
    undo: () => {
      find(loc)?.destroy();
    },
  };
}

/**
 * Makes a new object under `parent` (none: at the root) with `make` (again on redo); undo destroys
 * it. For things placed from files: a model, a prefab.
 */
export function placeCommand(label: string, parent: GameObject | null, make: (parent: GameObject | null) => GameObject): Command {
  const parentLoc = parent ? locate(parent) : null;
  let loc: Loc = [];
  return {
    label,
    run: () => {
      const p = parentLoc ? find(parentLoc) : null;
      if (parentLoc && !p) return;
      const go = make(p);
      if (go.exists()) loc = locate(go);
    },
    undo: () => {
      find(loc)?.destroy();
    },
  };
}

/** Objects with none of their ancestors in the list (deleting a parent deletes its children). */
export function topmost(objects: readonly GameObject[]): GameObject[] {
  const ids = new Set(objects.map((o) => o.id));
  return objects.filter((o) => {
    for (let p = o.parent; p; p = p.parent) if (ids.has(p.id)) return false;
    return true;
  });
}

/** Compares places in tree order. */
export function compareLoc(a: Loc, b: Loc): number {
  for (let i = 0; i < Math.min(a.length, b.length); i++) if (a[i] !== b[i]) return a[i]! - b[i]!;
  return a.length - b.length;
}

/** Deletes objects (with what is under them); undo builds them again where they were. */
export function deleteCommand(objects: readonly GameObject[]): Command | null {
  const tops = topmost(objects.filter(shown));
  // Last first, so the places of the others stay right; undo builds them first first.
  const items = tops
    .map((o) => ({ loc: locate(o), data: EditorScene.snapshot(o), name: o.name }))
    .filter((i) => i.data !== "")
    .sort((a, b) => compareLoc(b.loc, a.loc));
  if (items.length === 0) return null;
  const parts: Command[] = items.map((i) => {
    const make = createCommand("", i.data, parentLocOf(i.loc), indexOf(i.loc));
    return { label: "", run: make.undo, undo: make.run };
  });
  return group(items.length === 1 ? `Delete ${items[0]!.name}` : `Delete ${items.length} objects`, parts);
}

/** Copies of objects right after each (Ctrl+D). */
export function duplicateCommand(objects: readonly GameObject[]): Command | null {
  const tops = topmost(objects.filter(shown));
  // Last first, so each copy's place is right when made.
  const items = tops
    .map((o) => ({ loc: locate(o), data: EditorScene.snapshot(o) }))
    .filter((i) => i.data !== "")
    .sort((a, b) => compareLoc(b.loc, a.loc));
  if (items.length === 0) return null;
  const parts = items.map((i) => createCommand("", i.data, parentLocOf(i.loc), indexOf(i.loc) + 1));
  return group(items.length === 1 ? `Duplicate ${tops[0]!.name}` : `Duplicate ${items.length} objects`, parts);
}

/** Snapshots for the clipboard, in tree order. */
export function copyObjects(objects: readonly GameObject[]): string[] {
  return topmost(objects.filter(shown))
    .map((o) => ({ loc: locate(o), data: EditorScene.snapshot(o) }))
    .filter((i) => i.data !== "")
    .sort((a, b) => compareLoc(a.loc, b.loc))
    .map((i) => i.data);
}

/** Pastes snapshots under `parent` (none: as roots), after its last child. */
export function pasteCommand(data: readonly string[], parent: GameObject | null): Command | null {
  if (data.length === 0) return null;
  const parentLoc = parent ? locate(parent) : null;
  const start = shownChildren(parent).length;
  const parts = data.map((d, i) => createCommand("", d, parentLoc, start + i));
  return group(data.length === 1 ? "Paste" : `Paste ${data.length} objects`, parts);
}

/** Moves an object under `parent` (none: to the root) at shown place `index`, keeping its place in the world. */
export function reparentCommand(go: GameObject, parent: GameObject | null, index: number): Command | null {
  if (parent && (parent.id === go.id || isUnder(parent, go))) return null;
  const from = locate(go);
  const fromParent = parentLocOf(from);
  const fromIndex = indexOf(from);
  const toParent = parent ? locate(parent) : null;
  // The place counted without the object itself, when it moves within the same parent.
  const sameParent = (go.parent?.id ?? -1) === (parent?.id ?? -1);
  const to = sameParent && index > fromIndex ? index - 1 : index;
  if (sameParent && to === fromIndex) return null;
  // Where the move put the object and its old parent: their places can shift with it.
  let movedTo: Loc = [];
  let oldParentNow: Loc | null = null;
  return {
    label: `Move ${go.name}`,
    run: () => {
      const o = find(from);
      if (!o) return;
      const oldParent = fromParent ? find(fromParent) : null;
      moveTo(o, toParent ? find(toParent) : null, to);
      movedTo = locate(o);
      oldParentNow = oldParent ? locate(oldParent) : null;
    },
    undo: () => {
      const o = find(movedTo);
      if (o) moveTo(o, oldParentNow ? find(oldParentNow) : null, fromIndex);
    },
  };
}

/** Puts `o` under `parent` (none: the root), keeping its place in the world, at shown place `at` among the others. */
function moveTo(o: GameObject, parent: GameObject | null, at: number): void {
  o.setParent(parent ?? noObject(), true);
  const before = shownChildren(parent).filter((c) => c.id !== o.id)[at];
  if (!before) {
    o.transform.siblingIndex = 1_000_000;
    return;
  }
  // Taking the object out first moves the ones after it up by one.
  const raw = before.transform.siblingIndex;
  o.transform.siblingIndex = o.transform.siblingIndex < raw ? raw - 1 : raw;
}

/** Whether `go` is somewhere under `ancestor`. */
export function isUnder(go: GameObject, ancestor: GameObject): boolean {
  for (let p = go.parent; p; p = p.parent) if (p.id === ancestor.id) return true;
  return false;
}
