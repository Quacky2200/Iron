// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// Edit > Preferences: the editor's own settings, kept in PlayerPrefs (`editor.pref.*`), apart
// from any project. Changes apply at once.

import { Checkbox } from "./Inspector.tsx";
import { closeModal, pushModal } from "./modal.ts";
import { Button, Modal } from "./Modal.tsx";

const KEY = "editor.pref.";

/** A preference (true or false), `fallback` when never set. */
export function getPref(name: string, fallback: boolean): boolean {
  return PlayerPrefs.getInt(KEY + name, fallback ? 1 : 0) !== 0;
}

export function setPref(name: string, value: boolean): void {
  PlayerPrefs.setInt(KEY + name, value ? 1 : 0);
  PlayerPrefs.save();
}

export interface PrefItem {
  name: string;
  label: string;
  fallback: boolean;
}

/** What Preferences shows. */
export const PREFS: PrefItem[] = [
  { name: "grid", label: "Show the grid in the Scene view", fallback: true },
  { name: "gizmos", label: "Show gizmos and handles in the Scene view", fallback: true },
  { name: "reopenLastScene", label: "Open the last scene when the editor starts", fallback: true },
];

function PreferencesForm({ done, changed }: { done: () => void; changed: (name: string, value: boolean) => void }) {
  const [, setVersion] = useState(0);
  return (
    <Modal title="Preferences" width={460} onCancel={done} onEnter={done}>
      <box className="platypus-inspector-fields">
        {PREFS.map((p) => (
          <box key={p.name} className="platypus-inspector-row">
            <Checkbox
              value={getPref(p.name, p.fallback)}
              onChange={(v) => {
                setPref(p.name, v);
                changed(p.name, v);
                setVersion((n) => n + 1);
              }}
            />
            <Text>{p.label}</Text>
          </box>
        ))}
      </box>
      <Text className="platypus-modal-label">The code editor for Console links is set with PLATYPUS_EDITOR_OPEN (docs/editor.md).</Text>
      <box className="platypus-modal-buttons">
        <Button label="Close" primary onClick={done} />
      </box>
    </Modal>
  );
}

/** Shows Preferences; `changed` hears each change. */
export function preferencesDialog(changed: (name: string, value: boolean) => void): void {
  let id = 0;
  id = pushModal(() => <PreferencesForm done={() => closeModal(id)} changed={changed} />);
}
