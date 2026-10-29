// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// The Inspector: the selected object's name, enabled flag, layer and tag, its Transform, then a
// foldout per component with an editor per field (number, checkbox, text, choice, vector, colour;
// references, assets and nested data are shown, not edited). Every edit is an undoable command
// (inspector.ts). Values follow the world each frame (a key of what is shown, as the Scene view).

import { cx } from "./classes.ts";
import { type Command } from "./history.ts";
import {
  addComponentCommand,
  boolJson,
  fieldCommand,
  formatNumber,
  labelOf,
  localPose,
  localPoseCommand,
  numberJson,
  parseNumber,
  removeComponentCommand,
  stringJson,
  vectorJson,
  withComponent,
} from "./inspector.ts";
import { onEditorFrame } from "./frame.ts";
import { renameCommand, enableCommand } from "./scene-edit.ts";

export interface InspectorProps {
  objects: GameObject[];
  width: number;
  height: number;
  /** Runs an edit as an undoable command. */
  exec: (c: Command | null) => void;
}

/** A number field: typing changes nothing until Enter or leaving it; Escape puts the value back. */
export function NumberInput({ value, onCommit }: { value: number; onCommit: (n: number) => void }) {
  const [text, setText] = useState(formatNumber(value));
  const editing = useRef(false);
  useEffect(() => {
    if (!editing.current) setText(formatNumber(value));
  }, [value]);
  const commit = (): void => {
    editing.current = false;
    const n = parseNumber(text);
    if (Number.isNaN(n)) setText(formatNumber(value));
    else if (n !== value) onCommit(n);
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
          setText(formatNumber(value));
        }
      }}
      onBlur={commit}
    />
  );
}

/** A text field, committed like the number field. */
export function TextInput({ value, onCommit }: { value: string; onCommit: (s: string) => void }) {
  const [text, setText] = useState(value);
  const editing = useRef(false);
  useEffect(() => {
    if (!editing.current) setText(value);
  }, [value]);
  const commit = (): void => {
    editing.current = false;
    if (text !== value) onCommit(text);
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
          setText(value);
        }
      }}
      onBlur={commit}
    />
  );
}

export function Checkbox({ value, onChange }: { value: boolean; onChange: (v: boolean) => void }) {
  return (
    <box className={value ? "platypus-check on" : "platypus-check"} onClick={() => onChange(!value)}>
      <Text>{value ? "✓" : ""}</Text>
    </box>
  );
}

/** A choice among `options`: the value, and the list while open. */
export function Choice({ value, options, onChange }: { value: string; options: string[]; onChange: (v: string) => void }) {
  const [open, setOpen] = useState(false);
  return (
    <box className="platypus-choice" onClick={() => setOpen(!open)}>
      <Text className="platypus-choice-value">{value || "—"}</Text>
      <box className="platypus-chevron open" />
      {open ? (
        <box className="platypus-choice-list">
          {options.map((o) => (
            <box
              key={o}
              className={o === value ? "platypus-choice-item on" : "platypus-choice-item"}
              onClick={(e: PlatypusPointerEvent) => {
                e.stopPropagation();
                setOpen(false);
                if (o !== value) onChange(o);
              }}
            >
              <Text>{o}</Text>
            </box>
          ))}
        </box>
      ) : null}
    </box>
  );
}

const AXES = ["X", "Y", "Z", "W"];

function VectorInput({ values, labels, onCommit }: { values: number[]; labels: string[]; onCommit: (v: number[]) => void }) {
  return (
    <box className="platypus-vector">
      {values.map((v, i) => (
        <box key={i} className="platypus-vector-part">
          <Text className="platypus-vector-axis">{labels[i] ?? ""}</Text>
          <NumberInput value={v} onCommit={(n) => onCommit(withComponent(values, i, n))} />
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

/** One field's editor. */
function FieldEditor({ f, set }: { f: FieldInfo; set: (json: string) => void }) {
  if (f.readOnly) return <Text className="platypus-inspector-readonly">{f.text || f.json}</Text>;
  switch (f.ty) {
    case "number":
      return <NumberInput value={f.num} onCommit={(n) => set(numberJson(n))} />;
    case "bool":
      return <Checkbox value={f.flag} onChange={(b) => set(boolJson(b))} />;
    case "string":
      return <TextInput value={f.text} onCommit={(s) => set(stringJson(s))} />;
    case "enum":
      return <Choice value={f.text} options={f.options.split("|")} onChange={(s) => set(stringJson(s))} />;
    case "color": {
      const v = f.vec.length >= 4 ? f.vec : [1, 1, 1, 1];
      const css = `rgba(${Math.round(v[0]! * 255)}, ${Math.round(v[1]! * 255)}, ${Math.round(v[2]! * 255)}, ${v[3]!})`;
      return (
        <box className="platypus-color">
          <box className="platypus-color-swatch" style={{ backgroundColor: css }} />
          <VectorInput values={v} labels={["R", "G", "B", "A"]} onCommit={(c) => set(vectorJson(c))} />
        </box>
      );
    }
    case "vector2":
    case "vector3":
    case "vector4":
      return <VectorInput values={f.vec} labels={AXES} onCommit={(c) => set(vectorJson(c))} />;
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

function AddComponent({ go, exec }: { go: GameObject; exec: (c: Command | null) => void }) {
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
                exec(addComponentCommand(go, k));
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

export function Inspector({ objects, width, height, exec }: InspectorProps) {
  const [, setVersion] = useState(0);
  const [closed, setClosed] = useState<Set<string>>(new Set<string>());
  const go = objects[objects.length - 1];
  const goId = go ? go.id : -1;
  useEffect(() => {
    let last = "";
    return onEditorFrame(() => {
      const o = goId < 0 ? null : GameObject.byId(goId);
      const key = o && o.exists() ? inspectKey(o) : "";
      if (key !== last) {
        last = key;
        setVersion((v) => v + 1);
      }
    });
  }, [goId]);

  if (!go || go.destroyed) {
    return (
      <box className="platypus-inspector" style={{ width, height }}>
        <Text className="platypus-inspector-empty">Nothing selected</Text>
      </box>
    );
  }
  const pose = localPose(go);
  const setPose = (what: string, position: Vector3, euler: Vector3, scale: Vector3): void =>
    exec(localPoseCommand(go, pose, { position, euler, scale }, what));
  const v3 = (v: Vector3): number[] => [v.x, v.y, v.z];
  const toV3 = (a: number[]): Vector3 => new Vector3(a[0] ?? 0, a[1] ?? 0, a[2] ?? 0);
  const components = EditorScene.components(go);
  const toggle = (key: string): void =>
    setClosed((s) => {
      const next = new Set(s);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return next;
    });

  return (
    <box className="platypus-inspector" style={{ width, height }}>
      {objects.length > 1 ? <Text className="platypus-inspector-note">{`${objects.length} selected: showing ${go.name}`}</Text> : null}
      <box className="platypus-inspector-header">
        <Checkbox value={go.enabled} onChange={(b) => exec(enableCommand(go, b))} />
        <TextInput value={go.name} onCommit={(name) => exec(renameCommand(go, name))} />
      </box>
      <box className="platypus-inspector-fields">
        <Row label="Tag">
          <Text className="platypus-inspector-readonly">{go.tag}</Text>
        </Row>
        <Row label="Layer">
          <Text className="platypus-inspector-readonly">{`${go.layer}`}</Text>
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
              <VectorInput values={v3(pose.position)} labels={AXES} onCommit={(a) => setPose("Position", toV3(a), pose.euler, pose.scale)} />
            </Row>
            <Row label="Rotation">
              <VectorInput values={v3(pose.euler)} labels={AXES} onCommit={(a) => setPose("Rotation", pose.position, toV3(a), pose.scale)} />
            </Row>
            <Row label="Scale">
              <VectorInput values={v3(pose.scale)} labels={AXES} onCommit={(a) => setPose("Scale", pose.position, pose.euler, toV3(a))} />
            </Row>
          </box>
        )}
      </box>
      {components.map((c) => {
        const key = `${c.kind}#${c.handle}`;
        const fields = EditorScene.fields(go, c.kind, c.handle);
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
                  exec(removeComponentCommand(go, c));
                }}
              >
                <Text>×</Text>
              </box>
            </box>
            {closed.has(key) ? null : (
              <box className={cx("platypus-inspector-fields", c.opaque ? "opaque" : "")}>
                {fields.map((f) => (
                  <Row key={f.name} label={labelOf(f.name)}>
                    <FieldEditor f={f} set={(json) => exec(fieldCommand(go, c, f.name, f.json, json))} />
                  </Row>
                ))}
              </box>
            )}
          </box>
        );
      })}
      <AddComponent go={go} exec={exec} />
    </box>
  );
}
