// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// Pickers as dialogs, like Unity's object picker window: a filtered list to choose a reference
// from, and a colour palette with a hex field. Each answers with a promise.

import { colorHex, palette, parseHex } from "./inspector.ts";
import { closeModal, pushModal } from "./modal.ts";
import { Button, Modal } from "./Modal.tsx";

export interface PickItem {
  key: string;
  label: string;
}

function PickForm({ title, items, done }: { title: string; items: PickItem[]; done: (key: string | null) => void }) {
  const [filter, setFilter] = useState("");
  const field = useRef<PlatypusElement>(null);
  useEffect(() => {
    field.current?.focus();
  }, []);
  const shown = items.filter((i) => i.label.toLowerCase().includes(filter.toLowerCase()));
  return (
    <Modal title={title} width={380} onCancel={() => done(null)} onEnter={() => shown[0] && done(shown[0].key)}>
      <Input ref={field} className="platypus-field" value={filter} onChange={(e) => setFilter(e.value)} />
      <box className="platypus-pick-list">
        {shown.map((i) => (
          <box key={i.key} className="platypus-choice-item" onClick={() => done(i.key)}>
            <Text>{i.label}</Text>
          </box>
        ))}
      </box>
    </Modal>
  );
}

/** Asks for one of `items` (typing filters them; Enter takes the first shown); null when cancelled. */
export function pickDialog(title: string, items: PickItem[]): Promise<string | null> {
  return new Promise<string | null>((resolve) => {
    let id = 0;
    const done = (key: string | null) => {
      closeModal(id);
      resolve(key);
    };
    id = pushModal(() => <PickForm title={title} items={items} done={done} />);
  });
}

function ColorForm({ initial, done }: { initial: number[]; done: (c: number[] | null) => void }) {
  const alpha = initial[3] ?? 1;
  const [hex, setHex] = useState(colorHex(initial));
  const parsed = parseHex(hex, alpha);
  const ok = () => {
    if (parsed) done(parsed);
  };
  const css = (c: number[]): string => `rgb(${Math.round(c[0]! * 255)}, ${Math.round(c[1]! * 255)}, ${Math.round(c[2]! * 255)})`;
  return (
    <Modal title="Colour" width={300} onCancel={() => done(null)} onEnter={ok}>
      <box className="platypus-color-picker">
        {palette().map((row, r) => (
          <box key={r} className="platypus-color-row">
            {row.map((c, i) => (
              <box key={i} className="platypus-color-cell" style={{ backgroundColor: css(c) }} onClick={() => setHex(colorHex(c))} />
            ))}
          </box>
        ))}
      </box>
      <box className="platypus-color-hex">
        <box className="platypus-color-swatch" style={{ backgroundColor: parsed ? css(parsed) : "transparent" }} />
        <Text className="platypus-vector-axis">Hex</Text>
        <Input className="platypus-field" value={hex} onChange={(e) => setHex(e.value)} />
      </box>
      <box className="platypus-modal-buttons">
        <Button label="OK" primary disabled={parsed === null} onClick={ok} />
        <Button label="Cancel" onClick={() => done(null)} />
      </box>
    </Modal>
  );
}

/** Asks for a colour, starting at `initial` (alpha kept); null when cancelled. */
export function colorDialog(initial: number[]): Promise<number[] | null> {
  return new Promise<number[] | null>((resolve) => {
    let id = 0;
    const done = (c: number[] | null) => {
      closeModal(id);
      resolve(c);
    };
    id = pushModal(() => <ColorForm initial={initial} done={done} />);
  });
}
