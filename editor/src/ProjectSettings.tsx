// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// Edit > Project Settings: the project's `project.json` as a form, grouped (Identity, Game,
// Build). Each change rewrites only that key's text in the file (`Editor.setProjectField`), so
// unknown keys, key order and `${templates}` stay as written. The dialog keeps its own undo
// history (its Undo / Redo buttons), apart from the scene's.

import { canRedo, canUndo, type Command, createHistory, type History, perform, redo, undo } from "./history.ts";
import { Checkbox, Choice, TextInput } from "./Inspector.tsx";
import { closeModal, pushModal } from "./modal.ts";
import { Button, Modal } from "./Modal.tsx";

/** The JSON a field's value is written as. */
function jsonOf(f: ProjectField, value: string): string {
  return f.ty === "bool" ? (value === "true" ? "true" : "false") : JSON.stringify(value);
}

/** Sets a setting; undo writes the old value back, or removes the key when the file had none. */
function setCommand(f: ProjectField, value: string): Command {
  const before = f.set ? jsonOf(f, f.value) : "";
  return {
    label: `Set ${f.key}`,
    run: () => {
      Editor.setProjectField(f.key, jsonOf(f, value));
    },
    undo: () => {
      if (before === "") Editor.removeProjectField(f.key);
      else Editor.setProjectField(f.key, before);
    },
  };
}

function SettingsForm({ done }: { done: () => void }) {
  const [history] = useState<History>(() => createHistory());
  const [, setVersion] = useState(0);
  const [problem, setProblem] = useState("");
  const fields = Editor.projectFields();
  const run = (c: Command): void => {
    perform(history, c);
    setProblem(Editor.lastError);
    setVersion(history.version);
  };
  const groups: string[] = [];
  for (const f of fields) if (!groups.includes(f.group)) groups.push(f.group);
  const editor = (f: ProjectField): ReactNode => {
    switch (f.ty) {
      case "bool":
        return <Checkbox value={f.value === "true"} onChange={(b) => run(setCommand(f, b ? "true" : "false"))} />;
      case "enum":
        return <Choice value={f.value} options={f.options.split("|")} onChange={(v) => run(setCommand(f, v))} />;
      default:
        return <TextInput value={f.value} onCommit={(v) => run(setCommand(f, v))} />;
    }
  };
  return (
    <Modal title="Project Settings" width={620} onCancel={done}>
      <box className="platypus-settings">
        {groups.map((g) => (
          <box key={g} className="platypus-inspector-section">
            <box className="platypus-inspector-title">
              <Text>{g}</Text>
            </box>
            <box className="platypus-inspector-fields">
              {fields
                .filter((f) => f.group === g)
                .map((f) => (
                  <box key={f.key} className="platypus-inspector-row">
                    <Text className="platypus-inspector-label">{f.key}</Text>
                    <box className="platypus-inspector-value">{editor(f)}</box>
                  </box>
                ))}
            </box>
          </box>
        ))}
      </box>
      <Text className="platypus-modal-problem">{problem}</Text>
      <box className="platypus-modal-buttons">
        <Button
          label="Undo"
          disabled={!canUndo(history)}
          onClick={() => {
            const label = undo(history);
            console.log(`Project Settings: undid ${label}`);
            setProblem(Editor.lastError);
            setVersion(history.version);
          }}
        />
        <Button
          label="Redo"
          disabled={!canRedo(history)}
          onClick={() => {
            const label = redo(history);
            console.log(`Project Settings: redid ${label}`);
            setProblem(Editor.lastError);
            setVersion(history.version);
          }}
        />
        <box style={{ flexGrow: 1 }} />
        <Button label="Close" primary onClick={done} />
      </box>
    </Modal>
  );
}

/** Shows Project Settings until closed. */
export function projectSettings(): void {
  let id = 0;
  id = pushModal(() => <SettingsForm done={() => closeModal(id)} />);
}
