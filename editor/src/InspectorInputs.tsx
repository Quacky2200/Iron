// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// The Inspector's reference and colour inputs. A reference (an object, a component, an asset) is
// chosen in a picker (Pickers.tsx) or dropped on the field: a Hierarchy row for objects and
// components, a Project file for assets. A colour's swatch opens the colour picker.

import { assetExtensions, componentAt, componentJson, fitsAsset, objectJson, soundJson } from "./inspector.ts";
import { colorDialog, type PickItem, pickDialog } from "./Pickers.tsx";
import { OBJECT_TYPE, PROJECT_PATH_TYPE, projectFiles } from "./project.ts";
import { shownChildren } from "./scene-edit.ts";

/** The shown objects, depth first, with their paths (`Shapes/Cube`). */
function shownObjects(): { object: GameObject; path: string }[] {
  const out: { object: GameObject; path: string }[] = [];
  const walk = (parent: GameObject | null, above: string): void => {
    for (const c of shownChildren(parent)) {
      const path = above === "" ? c.name : `${above}/${c.name}`;
      out.push({ object: c, path });
      walk(c, path);
    }
  };
  walk(null, "");
  return out;
}

/** What a reference field can be set to ("" is None): objects by id, components by their object's id, assets by project path. */
function choices(f: FieldInfo): PickItem[] {
  const none: PickItem = { key: "", label: "None" };
  if (f.ty === "object") return [none, ...shownObjects().map((o) => ({ key: `${o.object.id}`, label: o.path }))];
  if (f.ty === "component") {
    const kind = f.refKind;
    const on = shownObjects().filter((o) => kind !== "" && EditorScene.components(o.object).some((c) => c.kind === kind));
    return [none, ...on.map((o) => ({ key: `${o.object.id}`, label: `${kind} on ${o.path}` }))];
  }
  if (assetExtensions(f.refKind).length === 0) return [none];
  return [none, ...projectFiles((p) => fitsAsset(f.refKind, p)).map((p) => ({ key: p, label: p }))];
}

/** The JSON a reference field takes for a picker key ("null" for None; "" when it can't be made). */
function refJson(f: FieldInfo, key: string): string {
  if (key === "") return "null";
  if (f.ty === "object") {
    const go = GameObject.byId(Number(key));
    return go.exists() ? objectJson(go) : "";
  }
  if (f.ty === "component") {
    const go = GameObject.byId(Number(key));
    return go.exists() ? componentJson(componentAt(EditorScene.components(go), f.refKind, 0)) : "";
  }
  if (f.refKind === "Sound") return soundJson(key);
  return EditorScene.assetFromFile(f.refKind, key);
}

/** The picker key a drag carries for this field, if it fits ("" when it doesn't). */
function dropKey(f: FieldInfo, e: PlatypusDragEvent): string {
  if (f.ty === "asset") {
    const path = e.dataTransfer.getData(PROJECT_PATH_TYPE);
    return path && fitsAsset(f.refKind, path) ? path : "";
  }
  const id = e.dataTransfer.getData(OBJECT_TYPE);
  if (!id) return "";
  if (f.ty === "component") {
    const go = GameObject.byId(Number(id));
    return go.exists() && componentAt(EditorScene.components(go), f.refKind, 0) ? id : "";
  }
  return id;
}

/** A reference field: its target's name (a click opens the picker, as ◉ does) and a drop target. */
export function RefInput({ f, mixed, set }: { f: FieldInfo; mixed: boolean; set: (json: string) => void }) {
  const [over, setOver] = useState(false);
  const take = (key: string): void => {
    const json = refJson(f, key);
    if (json !== "") set(json);
    else console.warn(`Can't use ${key}: ${Editor.lastError}`);
  };
  const pick = (): void => {
    void pickDialog(`Select ${f.refKind || f.ty}`, choices(f)).then((key) => {
      if (key !== null) take(key);
    });
  };
  return (
    <box
      className={over ? "platypus-ref over" : "platypus-ref"}
      onDragOver={(e: PlatypusDragEvent) => {
        if (dropKey(f, e) === "") return;
        e.preventDefault();
        e.stopPropagation();
        e.dataTransfer.dropEffect = "link";
        setOver(true);
      }}
      onDragLeave={() => setOver(false)}
      onDrop={(e: PlatypusDragEvent) => {
        setOver(false);
        const key = dropKey(f, e);
        if (key === "") return;
        e.stopPropagation();
        take(key);
      }}
    >
      <Text className="platypus-ref-value" onClick={pick}>
        {mixed ? "—" : f.text || `None (${f.refKind || f.ty})`}
      </Text>
      <box className="platypus-ref-pick" onClick={pick}>
        <Text>◉</Text>
      </box>
    </box>
  );
}

/** A colour: its swatch (a click opens the colour picker), then the children (the channels as numbers). */
export function ColorInput({ value, mixed, set, children }: { value: number[]; mixed: boolean; set: (c: number[]) => void; children?: ReactNode }) {
  const css = `rgba(${Math.round(value[0]! * 255)}, ${Math.round(value[1]! * 255)}, ${Math.round(value[2]! * 255)}, ${value[3] ?? 1})`;
  return (
    <box className="platypus-color">
      <box
        className={mixed ? "platypus-color-swatch mixed" : "platypus-color-swatch"}
        style={{ backgroundColor: css }}
        onClick={() => {
          void colorDialog(value).then((c) => {
            if (c) set(c);
          });
        }}
      />
      {children}
    </box>
  );
}
