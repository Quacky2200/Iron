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

// ---- several objects at once ----

/** A component on every selected object: its kind and place (the n-th of that kind). */
export interface SharedComponent {
  kind: string;
  place: number;
}

/** The components every list has (by kind and place), in the first list's order. */
export function sharedComponents(lists: readonly (readonly ComponentInfo[])[]): SharedComponent[] {
  const first = lists[0];
  if (!first) return [];
  const out: SharedComponent[] = [];
  for (const c of first) {
    const place = placeOf(first, c.kind, c.handle);
    if (lists.every((l) => componentAt(l, c.kind, place) !== null)) out.push({ kind: c.kind, place });
  }
  return out;
}

/** Whether values differ ("mixed" in the Inspector). */
export const mixed = (values: readonly string[]): boolean => values.some((v) => v !== values[0]);

/** Which parts of a vector differ between `values` (each a vector of the same length). */
export function mixedParts(values: readonly (readonly number[])[]): boolean[] {
  const first = values[0] ?? [];
  return first.map((v, i) => values.some((o) => o[i] !== v));
}

/** The parts of `after` that differ from `before` (a vector edited in one place). */
export function changedParts(before: readonly number[], after: readonly number[]): number[] {
  const out: number[] = [];
  after.forEach((v, i) => {
    if (v !== before[i]) out.push(i);
  });
  return out;
}

/** `own` with the parts `parts` taken from `edited` (each object keeps its other parts). */
export function withParts(own: readonly number[], edited: readonly number[], parts: readonly number[]): number[] {
  const out = [...own];
  for (const i of parts) out[i] = edited[i] ?? out[i]!;
  return out;
}

// ---- references and colours ----

/** A reference field's JSON for an object (`null` for none). */
export const objectJson = (go: GameObject | null): string => (go ? `{"$ref":"GameObject","object":${go.id}}` : "null");

/** A reference field's JSON for a component (`null` for none). */
export const componentJson = (c: ComponentInfo | null): string =>
  c ? `{"$ref":"Component","kind":${JSON.stringify(c.kind)},"handle":${c.handle}}` : "null";

/** A sound field's JSON for a project file. */
export const soundJson = (path: string): string => `{"$type":"Sound","src":${JSON.stringify(path)}}`;

/** The file extensions an asset kind is made from (none: it can't be chosen from a file). */
export function assetExtensions(kind: string): string[] {
  if (kind === "Texture") return ["png", "jpg", "jpeg"];
  if (kind === "Sound") return ["wav", "ogg", "mp3", "flac"];
  return [];
}

/** Whether a project file can be an asset of `kind`. */
export function fitsAsset(kind: string, path: string): boolean {
  const dot = path.lastIndexOf(".");
  return dot >= 0 && assetExtensions(kind).includes(path.slice(dot + 1).toLowerCase());
}

/** A colour as `#rrggbb` (alpha left out). */
export function colorHex(c: readonly number[]): string {
  const part = (v: number): string => Math.round(Math.min(1, Math.max(0, v)) * 255).toString(16).padStart(2, "0");
  return `#${part(c[0] ?? 0)}${part(c[1] ?? 0)}${part(c[2] ?? 0)}`;
}

/** `#rgb` or `#rrggbb` (with or without `#`) as channels 0..1 with `alpha`; null when it isn't one. */
export function parseHex(text: string, alpha: number): number[] | null {
  let t = text.trim().replace(/^#/, "");
  if (/^[0-9a-fA-F]{3}$/.test(t)) t = t.split("").map((ch) => ch + ch).join("");
  if (!/^[0-9a-fA-F]{6}$/.test(t)) return null;
  const at = (i: number): number => parseInt(t.slice(i, i + 2), 16) / 255;
  return [at(0), at(2), at(4), alpha];
}

/** A colour from hue (degrees), saturation and value (0..1), as channels 0..1. */
export function hsv(h: number, s: number, v: number): number[] {
  const f = (n: number): number => {
    const k = (n + h / 60) % 6;
    return v - v * s * Math.max(0, Math.min(k, 4 - k, 1));
  };
  return [f(5), f(3), f(1)];
}

/** The colour picker's palette: rows of 12 hues from light to dark, then greys from white to black. */
export function palette(): number[][][] {
  const rows: number[][][] = [];
  const tones = [
    { s: 0.25, v: 1 },
    { s: 0.5, v: 1 },
    { s: 0.8, v: 1 },
    { s: 1, v: 0.8 },
    { s: 1, v: 0.55 },
    { s: 1, v: 0.3 },
  ];
  for (const t of tones) {
    const row: number[][] = [];
    for (let i = 0; i < 12; i++) row.push(hsv(i * 30, t.s, t.v));
    rows.push(row);
  }
  const greys: number[][] = [];
  for (let i = 0; i < 12; i++) {
    const g = 1 - i / 11;
    greys.push([g, g, g]);
  }
  rows.push(greys);
  return rows;
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
