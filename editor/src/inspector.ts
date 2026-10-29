// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// The Inspector's logic (Inspector.tsx draws it): numbers as people type them, field values as the
// JSON `EditorScene.setField` takes, a change of one vector component, and field names as labels.
// Setting a field is an undoable command that finds its component again by kind and place (undo
// can rebuild the object, with new handles).

import { type Command } from "./history.ts";
import { find, type Loc, locate } from "./scene-edit.ts";

/** A number typed in a field (`-1.5`, `2e3`); NaN when it isn't one. */
export function parseNumber(text: string): number {
  const t = text.trim();
  if (t === "" || !/^[-+]?(\d+\.?\d*|\.\d+)([eE][-+]?\d+)?$/.test(t)) return NaN;
  return Number(t);
}

/** A number as a field shows it: at most 4 decimals, no trailing zeros, no `-0`. */
export function formatNumber(n: number): string {
  if (!Number.isFinite(n)) return `${n}`;
  const r = Math.round(n * 10000) / 10000;
  return `${r === 0 ? 0 : r}`;
}

/** `fieldOfView` -> `Field Of View`, `m_x` -> `M X` (Unity's labels). */
export function labelOf(name: string): string {
  const words = name
    .replace(/_/g, " ")
    .replace(/([a-z0-9])([A-Z])/g, "$1 $2")
    .split(" ")
    .filter((w) => w !== "");
  return words.map((w) => w.charAt(0).toUpperCase() + w.slice(1)).join(" ");
}

/** A vector with component `i` set to `v`. */
export function withComponent(vec: readonly number[], i: number, v: number): number[] {
  const out = [...vec];
  out[i] = v;
  return out;
}

export const numberJson = (n: number): string => JSON.stringify(n);
export const boolJson = (b: boolean): string => JSON.stringify(b);
export const stringJson = (s: string): string => JSON.stringify(s);
export const vectorJson = (v: readonly number[]): string => JSON.stringify(v);

/** Which of an object's components of `kind` this one is (0 for the first). */
export function placeOf(list: readonly ComponentInfo[], kind: string, handle: number): number {
  let n = 0;
  for (const c of list) {
    if (c.handle === handle && c.kind === kind) return n;
    if (c.kind === kind) n++;
  }
  return -1;
}

/** The `place`-th component of `kind` on an object, if there is one. */
export function componentAt(list: readonly ComponentInfo[], kind: string, place: number): ComponentInfo | null {
  const of = list.filter((c) => c.kind === kind);
  return of[place] ?? null;
}

function setAt(loc: Loc, kind: string, place: number, name: string, json: string): void {
  const go = find(loc);
  if (!go) return;
  const c = componentAt(EditorScene.components(go), kind, place);
  if (c) EditorScene.setField(go, c.kind, c.handle, name, json);
}

/** Sets a component's field; undo puts the old JSON back. */
export function fieldCommand(go: GameObject, c: ComponentInfo, name: string, before: string, after: string): Command {
  const loc = locate(go);
  const place = placeOf(EditorScene.components(go), c.kind, c.handle);
  return {
    label: `Set ${c.kind}.${name}`,
    run: () => setAt(loc, c.kind, place, name, after),
    undo: () => setAt(loc, c.kind, place, name, before),
  };
}

/** Adds a component; undo removes it again. */
export function addComponentCommand(go: GameObject, kind: string): Command {
  const loc = locate(go);
  return {
    label: `Add ${kind}`,
    run: () => {
      const o = find(loc);
      if (o) EditorScene.addComponent(o, kind);
    },
    undo: () => {
      const o = find(loc);
      if (!o) return;
      const of = EditorScene.components(o).filter((c) => c.kind === kind);
      const last = of[of.length - 1];
      if (last) EditorScene.removeComponent(o, last.kind, last.handle);
    },
  };
}

/** Removes a component; undo builds the object again from a snapshot taken before (its other parts with it). */
export function removeComponentCommand(go: GameObject, c: ComponentInfo): Command {
  const loc = locate(go);
  const place = placeOf(EditorScene.components(go), c.kind, c.handle);
  const snapshot = EditorScene.snapshot(go);
  return {
    label: `Remove ${c.kind}`,
    run: () => {
      const o = find(loc);
      if (!o) return;
      const at = componentAt(EditorScene.components(o), c.kind, place);
      if (at) EditorScene.removeComponent(o, at.kind, at.handle);
    },
    undo: () => {
      // The object as it was, in its place (children and references inside it included).
      const o = find(loc);
      if (!o || snapshot === "") return;
      const parent = o.parent;
      const index = o.transform.siblingIndex;
      o.destroy();
      EditorScene.restore(snapshot, parent ?? GameObject.byId(0), index);
    },
  };
}

/** The local transform as the Inspector edits it: position, Euler angles in degrees, scale. */
export interface LocalPose {
  position: Vector3;
  euler: Vector3;
  scale: Vector3;
}

export function localPose(go: GameObject): LocalPose {
  const t = go.transform;
  return { position: t.localPosition, euler: t.localEulerAngles, scale: t.localScale };
}

function applyLocal(go: GameObject, p: LocalPose): void {
  const t = go.transform;
  t.localPosition = p.position;
  t.localEulerAngles = p.euler;
  t.localScale = p.scale;
}

/** A change of the local transform from the Inspector. */
export function localPoseCommand(go: GameObject, before: LocalPose, after: LocalPose, what: string): Command {
  const loc = locate(go);
  return {
    label: `Set ${what} of ${go.name}`,
    run: () => {
      const o = find(loc);
      if (o) applyLocal(o, after);
    },
    undo: () => {
      const o = find(loc);
      if (o) applyLocal(o, before);
    },
  };
}
