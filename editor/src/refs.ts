// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// References from other objects into deleted ones. Undoing a delete builds the objects again with
// new ids, so the reference fields that pointed at them (a CameraFollow's target, a joint's body)
// would stay empty: `refsInto` notes them by place before the delete, and `repoint` sets them
// to the rebuilt objects and components afterwards.

import { type Command } from "./history.ts";
import { componentAt, placeOf } from "./inspector.ts";
import { find, type Loc, locate, shownChildren } from "./scene-edit.ts";

/** A reference field as saved: `{"$ref":"GameObject","object":id}` or `{"$ref":"Component","kind":k,"handle":h}`. */
interface RefJson {
  $ref?: string;
  object?: number;
  kind?: string;
  handle?: number;
}

/** What a reference pointed at, by place: under the deleted object at `top`, down `path` (child indices); a component when `kind` is set. */
interface RefTarget {
  top: Loc;
  path: number[];
  kind: string;
  place: number;
}

/** A reference field outside the deleted objects that pointed into them. */
export interface HeldRef {
  holder: Loc;
  kind: string;
  place: number;
  name: string;
  target: RefTarget;
}

/** Every object under `go` (itself first) with its child-index path from it. */
function subtree(go: GameObject): { object: GameObject; path: number[] }[] {
  const out: { object: GameObject; path: number[] }[] = [];
  const walk = (o: GameObject, path: number[]): void => {
    out.push({ object: o, path });
    o.children.forEach((c, i) => walk(c, [...path, i]));
  };
  walk(go, []);
  return out;
}

/** The shown objects, depth first. */
function allShown(): GameObject[] {
  const out: GameObject[] = [];
  const walk = (parent: GameObject | null): void => {
    for (const c of shownChildren(parent)) {
      out.push(c);
      walk(c);
    }
  };
  walk(null);
  return out;
}

/** The reference fields of other objects that point at `tops` or anything on or under them. */
export function refsInto(tops: readonly GameObject[]): HeldRef[] {
  const objects = new Map<number, RefTarget>();
  const components = new Map<string, RefTarget>();
  for (const top of tops) {
    const at = locate(top);
    for (const { object, path } of subtree(top)) {
      objects.set(object.id, { top: at, path, kind: "", place: 0 });
      const list = EditorScene.components(object);
      for (const c of list) components.set(`${c.kind}:${c.handle}`, { top: at, path, kind: c.kind, place: placeOf(list, c.kind, c.handle) });
    }
  }
  const out: HeldRef[] = [];
  for (const holder of allShown()) {
    if (objects.has(holder.id)) continue;
    const list = EditorScene.components(holder);
    for (const c of list) {
      for (const f of EditorScene.fields(holder, c.kind, c.handle)) {
        // Empty references are `null`.
        if ((f.ty !== "object" && f.ty !== "component") || !f.json.startsWith("{")) continue;
        const ref: RefJson = JSON.parse(f.json);
        const target = ref.$ref === "GameObject" ? objects.get(ref.object ?? -1) : components.get(`${ref.kind ?? ""}:${ref.handle ?? -1}`);
        if (target) out.push({ holder: locate(holder), kind: c.kind, place: placeOf(list, c.kind, c.handle), name: f.name, target });
      }
    }
  }
  return out;
}

/** Sets the noted references to the objects and components now at their targets' places. */
export function repoint(refs: readonly HeldRef[]): void {
  for (const r of refs) {
    let object = find(r.target.top);
    for (const i of r.target.path) object = object ? (object.children[i] ?? null) : null;
    const holder = find(r.holder);
    if (!object || !holder) continue;
    let json = `{"$ref":"GameObject","object":${object.id}}`;
    if (r.target.kind !== "") {
      const c = componentAt(EditorScene.components(object), r.target.kind, r.target.place);
      if (!c) continue;
      json = `{"$ref":"Component","kind":${JSON.stringify(c.kind)},"handle":${c.handle}}`;
    }
    const h = componentAt(EditorScene.components(holder), r.kind, r.place);
    if (h && EditorScene.setField(holder, h.kind, h.handle, r.name, json)) console.log(`Restored ${holder.name}'s ${r.kind}.${r.name}`);
  }
}

/** A command whose undo points the references back (put first in a delete's group: undone last). */
export function repointOnUndo(refs: readonly HeldRef[]): Command {
  return { label: "", run: () => {}, undo: () => repoint(refs) };
}
