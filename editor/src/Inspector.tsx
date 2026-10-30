// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// The Inspector: the selected objects' name, enabled flag, tag and layer, their Transform, then a
// foldout per component they all have with an editor per field (number, checkbox, text, choice,
// vector, colour with a palette, references to objects, components and assets; nested data is
// shown, not edited). Values that differ between the objects show "—"; an edit goes to all of them
// as one undoable command (inspector.ts). Values follow the world each frame (a key of what is
// shown, as the Scene view).

import { cx } from "./classes.ts";
import { type Command, group } from "./history.ts";
import {
  addComponentCommand,
  boolJson,
  changedParts,
  componentAt,
  fieldCommand,
  formatNumber,
  labelOf,
  localPose,
  localPoseCommand,
  mixed,
  mixedParts,
  numberJson,
  parseNumber,
  removeComponentCommand,
  sharedComponents,
  stringJson,
  vectorJson,
  withComponent,
  withParts,
} from "./inspector.ts";
import { ColorInput, RefInput } from "./InspectorInputs.tsx";
import { pickDialog } from "./Pickers.tsx";
import { onEditorFrame } from "./frame.ts";
import { enableCommand, layerCommand, renameCommand, tagCommand } from "./scene-edit.ts";

export interface InspectorProps {
  objects: GameObject[];
  width: number;
  height: number;
  /** Runs an edit as an undoable command. */
  exec: (c: Command | null) => void;
}

/** A number field: typing changes nothing until Enter or leaving it; Escape puts the value back. */
export function NumberInput({ value, onCommit, mixed = false }: { value: number; onCommit: (n: number) => void; mixed?: boolean }) {
  // Several objects with different values show "—" until a number is typed.
  const shown = mixed ? MIXED : formatNumber(value);
  const [text, setText] = useState(shown);
  const editing = useRef(false);
  useEffect(() => {
    if (!editing.current) setText(shown);
  }, [shown]);
  const commit = (): void => {
    editing.current = false;
    const n = parseNumber(text);
    if (Number.isNaN(n)) setText(shown);
    else if (n !== value || mixed) onCommit(n);
  };
  return (
    <Input
      className="platypus-field platypus-num"
      value={text}
      onChange={(e) => {
        editing.current = true;
        setText(e.value);
      }}
      onKeyDown={(e: PlatypusKeyboardEvent) => {
        if (e.key === "Enter") commit();
        else if (e.key === "Escape") {
          editing.current = false;
          setText(shown);
        }
      }}
      onBlur={commit}
    />
  );
}

/** What a field shows when the selected objects' values differ. */
const MIXED = "—";

/** A text field, committed like the number field. */
export function TextInput({ value, onCommit, mixed = false }: { value: string; onCommit: (s: string) => void; mixed?: boolean }) {
  const shown = mixed ? MIXED : value;
  const [text, setText] = useState(shown);
  const editing = useRef(false);
  useEffect(() => {
    if (!editing.current) setText(shown);
  }, [shown]);
  const commit = (): void => {
    editing.current = false;
    if (text !== shown) onCommit(text);
  };
  return (
    <Input
      className="platypus-field"
      value={text}
      onChange={(e) => {
        editing.current = true;
        setText(e.value);
      }}
      onKeyDown={(e: PlatypusKeyboardEvent) => {
        if (e.key === "Enter") commit();
        else if (e.key === "Escape") {
          editing.current = false;
          setText(shown);
        }
      }}
      onBlur={commit}
    />
  );
}

export function Checkbox({ value, onChange, mixed = false }: { value: boolean; onChange: (v: boolean) => void; mixed?: boolean }) {
  return (
    <box className={value && !mixed ? "platypus-check on" : "platypus-check"} onClick={() => onChange(mixed ? true : !value)}>
      <Text>{mixed ? "–" : value ? "✓" : ""}</Text>
    </box>
  );
}

/** A choice among `options` (shown by `labels` when given): the value, and the list while open. */
export function Choice({
  value,
  options,
  onChange,
  mixed = false,
  labels,
  title = "Choose",
}: {
  value: string;
  options: string[];
  onChange: (v: string) => void;
  mixed?: boolean;
  labels?: string[];
  title?: string;
}) {
  const label = (o: string): string => {
    const i = options.indexOf(o);
    return labels && i >= 0 ? (labels[i] ?? o) : o;
  };
  // The choices open in a picker dialog: a list dropped down inside a row would paint under the
  // rows after it (z-index orders siblings only here, docs/differences.md).
  const open = (): void => {
    void pickDialog(
      title,
      options.map((o) => ({ key: o, label: o === value && !mixed ? `${label(o) || "—"}  ✓` : label(o) || "—" })),
    ).then((o) => {
      if (o !== null && (o !== value || mixed)) onChange(o);
    });
  };
  return (
    <box className="platypus-choice" onClick={open}>
      <Text className="platypus-choice-value">{mixed ? MIXED : label(value) || "—"}</Text>
      <box className="platypus-chevron open" />
    </box>
  );
}

const AXES = ["X", "Y", "Z", "W"];

function VectorInput({ values, labels, onCommit, mixed = [] }: { values: number[]; labels: string[]; onCommit: (v: number[]) => void; mixed?: boolean[] }) {
  return (
    <box className="platypus-vector">
      {values.map((v, i) => (
        <box key={i} className="platypus-vector-part">
          <Text className="platypus-vector-axis">{labels[i] ?? ""}</Text>
          <NumberInput value={v} mixed={mixed[i] ?? false} onCommit={(n) => onCommit(withComponent(values, i, n))} />
        </box>
      ))}
    </box>
  );
}

function Row({ label, children, title }: { label: string; children?: ReactNode; title?: string }) {
  return (
    <box className="platypus-inspector-row">
      <Text className="platypus-inspector-label">{label}</Text>
      <box className="platypus-inspector-value">{children}</box>
      {title ? <Text className="platypus-inspector-doc">{title}</Text> : null}
    </box>
  );
}

/**
 * One field's editor. `mixed`: the selected objects' values differ (`parts`: which parts of a
 * vector); `set` gives every object the same JSON, `setVector` the edited parts of a vector.
 */
function FieldEditor({
  f,
  mixed,
  parts,
  set,
  setVector,
}: {
  f: FieldInfo;
  mixed: boolean;
  parts: boolean[];
  set: (json: string) => void;
  setVector: (v: number[]) => void;
}) {
  if (f.readOnly) return <Text className="platypus-inspector-readonly">{f.text || f.json}</Text>;
  switch (f.ty) {
    case "number":
      return <NumberInput value={f.num} mixed={mixed} onCommit={(n) => set(numberJson(n))} />;
    case "bool":
      return <Checkbox value={f.flag} mixed={mixed} onChange={(b) => set(boolJson(b))} />;
    case "string":
      return <TextInput value={f.text} mixed={mixed} onCommit={(s) => set(stringJson(s))} />;
    case "enum":
      return <Choice value={f.text} mixed={mixed} title={labelOf(f.name)} options={f.options.split("|")} onChange={(s) => set(stringJson(s))} />;
    case "color": {
      const v = f.vec.length >= 4 ? f.vec : [1, 1, 1, 1];
      return (
        <ColorInput value={v} mixed={mixed} set={(c) => set(vectorJson(c))}>
          <VectorInput values={v} labels={["R", "G", "B", "A"]} mixed={parts} onCommit={setVector} />
        </ColorInput>
      );
    }
    case "vector2":
    case "vector3":
    case "vector4":
      return <VectorInput values={f.vec} labels={AXES} mixed={parts} onCommit={setVector} />;
    case "object":
    case "component":
    case "asset":
      return <RefInput f={f} mixed={mixed} set={set} />;
    default:
      return <Text className="platypus-inspector-readonly">{f.text || f.json}</Text>;
  }
}

/** A key of everything shown for an object: re-render only when it changes. */
function inspectKey(go: GameObject): string {
  if (go.destroyed) return "gone";
  const t = go.transform;
  const p = t.localPosition;
  const r = t.localEulerAngles;
  const s = t.localScale;
  let key = `${go.id}|${go.name}|${go.enabled}|${go.layer}|${go.tag}|${p.x},${p.y},${p.z}|${r.x},${r.y},${r.z}|${s.x},${s.y},${s.z}`;
  for (const c of EditorScene.components(go)) {
    key += `|${c.kind}#${c.handle}`;
    for (const f of EditorScene.fields(go, c.kind, c.handle)) key += `:${f.json}`;
  }
  return key;
}

function AddComponent({ objects, exec }: { objects: GameObject[]; exec: (c: Command | null) => void }) {
  const [open, setOpen] = useState(false);
  const [filter, setFilter] = useState("");
  const kinds = open ? EditorScene.componentKinds().filter((k) => k.toLowerCase().includes(filter.toLowerCase())) : [];
  return (
    <box className="platypus-add-component">
      <box className="platypus-button" onClick={() => setOpen(!open)}>
        <Text>Add Component</Text>
      </box>
      {open ? (
        <box className="platypus-add-list">
          <Input className="platypus-field" value={filter} onChange={(e) => setFilter(e.value)} />
          {kinds.map((k) => (
            <box
              key={k}
              className="platypus-choice-item"
              onClick={() => {
                setOpen(false);
                setFilter("");
                exec(oneStep(`Add ${k} to ${objects.length} objects`, objects.map((o) => addComponentCommand(o, k))));
              }}
            >
              <Text>{k}</Text>
            </box>
          ))}
        </box>
      ) : null}
    </box>
  );
}

/** A Transform row. */
type Part = "position" | "euler" | "scale";

/** One undoable step for several objects' edits (the one edit itself when there is one). */
function oneStep(label: string, commands: readonly (Command | null)[]): Command | null {
  const list = commands.filter((c): c is Command => c !== null);
  if (list.length <= 1) return list[0] ?? null;
  return group(label, list);
}

/** An object's field `name` on its component of `kind` at `place`, with that component. */
function fieldOn(go: GameObject, kind: string, place: number, name: string): { c: ComponentInfo; f: FieldInfo } | null {
  const c = componentAt(EditorScene.components(go), kind, place);
  if (!c) return null;
  const f = EditorScene.fields(go, c.kind, c.handle).find((x) => x.name === name);
  return f ? { c, f } : null;
}

/** The layers a layer choice offers: the named ones, and the object's own if it has no name. */
function layerOptions(current: number): { options: string[]; labels: string[] } {
  const names = Editor.layerNames();
  const options: string[] = [];
  const labels: string[] = [];
  names.forEach((n, i) => {
    if (n === "" && i !== current) return;
    options.push(`${i}`);
    labels.push(n === "" ? `${i}` : `${i}: ${n}`);
  });
  return { options, labels };
}

export function Inspector({ objects, width, height, exec }: InspectorProps) {
  const [, setVersion] = useState(0);
  const [closed, setClosed] = useState<Set<string>>(new Set<string>());
  const all = objects.filter((o) => !o.destroyed);
  const go = all[all.length - 1];
  const ids = all.map((o) => `${o.id}`).join(",");
  useEffect(() => {
    let last = "";
    return onEditorFrame(() => {
      let key = "";
      for (const id of ids.split(",")) {
        if (id === "") continue;
        const o = GameObject.byId(Number(id));
        key += `${o.exists() ? inspectKey(o) : "gone"};`;
      }
      if (key !== last) {
        last = key;
        setVersion((v) => v + 1);
      }
    });
  }, [ids]);

  if (!go) {
    return (
      <box className="platypus-inspector" style={{ width, height }}>
        <Text className="platypus-inspector-empty">Nothing selected</Text>
      </box>
    );
  }
  const many = all.length > 1;
  const v3 = (v: Vector3): number[] => [v.x, v.y, v.z];
  const toV3 = (a: number[]): Vector3 => new Vector3(a[0] ?? 0, a[1] ?? 0, a[2] ?? 0);
  const poses = all.map(localPose);
  const pose = poses[poses.length - 1]!;
  const vectors = (part: Part): number[][] => poses.map((p) => v3(p[part]));
  /** A Transform row edited: each object takes the edited parts, keeping its others. */
  const setPart = (what: string, part: Part, edited: number[]): void => {
    const changed = changedParts(v3(pose[part]), edited);
    exec(
      oneStep(
        `Set ${what} of ${all.length} objects`,
        all.map((o, i) => {
          const before = poses[i]!;
          const after = { ...before };
          after[part] = toV3(withParts(v3(before[part]), edited, changed));
          return localPoseCommand(o, before, after, what);
        }),
      ),
    );
  };
  const lists = all.map((o) => EditorScene.components(o));
  const shared = sharedComponents(lists.reverse());
  const toggle = (key: string): void =>
    setClosed((s) => {
      const next = new Set(s);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return next;
    });
  const layers = layerOptions(go.layer);
  const differ = (get: (o: GameObject) => string): boolean => mixed(all.map(get));

  return (
    <box className="platypus-inspector" style={{ width, height }}>
      {many ? <Text className="platypus-inspector-note">{`${all.length} objects: their shared components; edits go to all`}</Text> : null}
      <box className="platypus-inspector-header">
        <Checkbox
          value={go.enabled}
          mixed={differ((o) => `${o.enabled}`)}
          onChange={(b) => exec(oneStep(`${b ? "Enable" : "Disable"} ${all.length} objects`, all.map((o) => enableCommand(o, b))))}
        />
        <TextInput
          value={go.name}
          mixed={differ((o) => o.name)}
          onCommit={(name) => exec(oneStep(`Rename ${all.length} objects`, all.map((o) => renameCommand(o, name))))}
        />
      </box>
      <box className="platypus-inspector-fields">
        <Row label="Tag">
          <TextInput
            value={go.tag}
            mixed={differ((o) => o.tag)}
            onCommit={(tag) => exec(oneStep(`Set Tag of ${all.length} objects`, all.map((o) => tagCommand(o, tag))))}
          />
        </Row>
        <Row label="Layer">
          <Choice
            value={`${go.layer}`}
            options={layers.options}
            labels={layers.labels}
            title="Layer"
            mixed={differ((o) => `${o.layer}`)}
            onChange={(l) => exec(oneStep(`Set Layer of ${all.length} objects`, all.map((o) => layerCommand(o, Number(l)))))}
          />
        </Row>
      </box>
      <box className="platypus-inspector-section">
        <box className="platypus-inspector-title" onClick={() => toggle("Transform")}>
          <box className={closed.has("Transform") ? "platypus-chevron" : "platypus-chevron open"} />
          <Text>Transform</Text>
        </box>
        {closed.has("Transform") ? null : (
          <box className="platypus-inspector-fields">
            <Row label="Position">
              <VectorInput values={v3(pose.position)} labels={AXES} mixed={mixedParts(vectors("position"))} onCommit={(a) => setPart("Position", "position", a)} />
            </Row>
            <Row label="Rotation">
              <VectorInput values={v3(pose.euler)} labels={AXES} mixed={mixedParts(vectors("euler"))} onCommit={(a) => setPart("Rotation", "euler", a)} />
            </Row>
            <Row label="Scale">
              <VectorInput values={v3(pose.scale)} labels={AXES} mixed={mixedParts(vectors("scale"))} onCommit={(a) => setPart("Scale", "scale", a)} />
            </Row>
          </box>
        )}
      </box>
      {shared.map((s) => {
        const c = componentAt(EditorScene.components(go), s.kind, s.place)!;
        const key = `${s.kind}#${s.place}`;
        const fields = EditorScene.fields(go, c.kind, c.handle);
        /** The field on every selected object (the shown one last). */
        const each = (name: string): { go: GameObject; c: ComponentInfo; f: FieldInfo }[] =>
          all.flatMap((o) => {
            const at = fieldOn(o, s.kind, s.place, name);
            return at ? [{ go: o, c: at.c, f: at.f }] : [];
          });
        const setAll = (name: string, json: (f: FieldInfo) => string): void => {
          const at = each(name);
          exec(oneStep(`Set ${s.kind}.${name} of ${at.length} objects`, at.map((x) => fieldCommand(x.go, x.c, name, x.f.json, json(x.f)))));
        };
        return (
          <box key={key} className="platypus-inspector-section">
            <box className="platypus-inspector-title" onClick={() => toggle(key)}>
              <box className={closed.has(key) ? "platypus-chevron" : "platypus-chevron open"} />
              <Text>{c.opaque ? `${c.kind} (script not loaded)` : c.kind}</Text>
              <box style={{ flexGrow: 1 }} />
              <box
                className="platypus-inspector-remove"
                onClick={(e: PlatypusPointerEvent) => {
                  e.stopPropagation();
                  exec(
                    oneStep(
                      `Remove ${s.kind} from ${all.length} objects`,
                      all.map((o) => {
                        const oc = componentAt(EditorScene.components(o), s.kind, s.place);
                        return oc ? removeComponentCommand(o, oc) : null;
                      }),
                    ),
                  );
                }}
              >
                <Text>×</Text>
              </box>
            </box>
            {closed.has(key) ? null : (
              <box className={cx("platypus-inspector-fields", c.opaque ? "opaque" : "")}>
                {fields.map((f) => {
                  const at = many ? each(f.name) : [];
                  return (
                    <Row key={f.name} label={labelOf(f.name)}>
                      <FieldEditor
                        f={f}
                        mixed={many && mixed(at.map((x) => x.f.json))}
                        parts={many ? mixedParts(at.map((x) => x.f.vec)) : []}
                        set={(json) => setAll(f.name, () => json)}
                        setVector={(edited) => setAll(f.name, (own) => vectorJson(withParts(own.vec, edited, changedParts(f.vec, edited))))}
                      />
                    </Row>
                  );
                })}
              </box>
            )}
          </box>
        );
      })}
      <AddComponent objects={all} exec={exec} />
    </box>
  );
}
