// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// Modal dialogs drawn by the editor's own UI (no OS dialogs): a backdrop that takes the pointer,
// a centred box with a title, Escape to cancel and Enter for the default button. `ModalHost`
// draws the open ones (modal.ts); `confirmDialog` and `textDialog` are the common ones, as
// promises of the answer.

import { closeModal, modals, pushModal, subscribeModals } from "./modal.ts";

export interface ModalProps {
  title: string;
  width?: number;
  children?: ReactNode;
  /** Escape, or the close box. */
  onCancel: () => void;
  /** Enter outside a multi-line field. */
  onEnter?: () => void;
}

/** The dialog's shell. Focuses itself when it opens, so keys come to it. */
export function Modal({ title, width = 420, children, onCancel, onEnter }: ModalProps) {
  const box = useRef<PlatypusElement>(null);
  useEffect(() => {
    box.current?.focus();
  }, []);
  return (
    <box
      className="platypus-modal-backdrop"
      onPointerDown={(e: PlatypusPointerEvent) => {
        e.stopPropagation();
        e.preventDefault();
      }}
    >
      <box
        ref={box}
        className="platypus-modal"
        style={{ width }}
        tabIndex={-1}
        onPointerDown={(e: PlatypusPointerEvent) => e.stopPropagation()}
        onKeyDown={(e: PlatypusKeyboardEvent) => {
          if (e.key === "Escape") {
            e.preventDefault();
            e.stopPropagation();
            onCancel();
          } else if (e.key === "Enter" && onEnter) {
            e.preventDefault();
            e.stopPropagation();
            onEnter();
          }
        }}
      >
        <box className="platypus-modal-title">
          <Text>{title}</Text>
          <box style={{ flexGrow: 1 }} />
          <box className="platypus-modal-close" onClick={onCancel}>
            <Text>×</Text>
          </box>
        </box>
        <box className="platypus-modal-body">{children}</box>
      </box>
    </box>
  );
}

export interface ButtonProps {
  label: string;
  onClick: () => void;
  primary?: boolean;
  disabled?: boolean;
}

export function Button({ label, onClick, primary = false, disabled = false }: ButtonProps) {
  const classes = primary ? "platypus-button primary" : "platypus-button";
  return (
    <box className={disabled ? `${classes} disabled` : classes} onClick={() => !disabled && onClick()}>
      <Text>{label}</Text>
    </box>
  );
}

/** The open dialogs, over everything else. */
export function ModalHost() {
  const open = useSyncExternalStore(subscribeModals, modals);
  const screen = { width: Screen.width, height: Screen.height };
  if (open.length === 0) return null;
  return (
    <box className="platypus-modal-layer" style={{ width: screen.width, height: screen.height }}>
      {open.map((m) => (
        <box key={m.id} className="platypus-modal-slot" style={{ width: screen.width, height: screen.height }}>
          {m.render()}
        </box>
      ))}
    </box>
  );
}

export interface ConfirmOptions {
  title: string;
  message: string;
  /** Button labels, left to right; the first is the default (Enter). */
  buttons: string[];
  /** What Escape answers ("" by default). */
  cancel?: string;
}

/** Asks a question; the answer is the label of the button pressed (`cancel` for Escape). */
export function confirmDialog(options: ConfirmOptions): Promise<string> {
  return new Promise<string>((resolve) => {
    let id = 0;
    const answer = (label: string) => {
      closeModal(id);
      resolve(label);
    };
    const cancel = options.cancel ?? "";
    id = pushModal(() => (
      <Modal title={options.title} onCancel={() => answer(cancel)} onEnter={() => answer(options.buttons[0] ?? cancel)}>
        <Text className="platypus-modal-message">{options.message}</Text>
        <box className="platypus-modal-buttons">
          {options.buttons.map((b, i) => (
            <Button key={b} label={b} primary={i === 0} onClick={() => answer(b)} />
          ))}
        </box>
      </Modal>
    ));
  });
}

export interface TextOptions {
  title: string;
  label: string;
  initial?: string;
  ok?: string;
  /** Why a value can't be used ("" when it can). */
  check?: (value: string) => string;
}

function TextForm({ options, done }: { options: TextOptions; done: (value: string) => void }) {
  const [value, setValue] = useState(options.initial ?? "");
  const field = useRef<PlatypusElement>(null);
  useEffect(() => {
    field.current?.focus();
  }, []);
  const problem = options.check ? options.check(value) : value.trim() === "" ? "A name is needed" : "";
  const ok = () => {
    if (problem === "") done(value);
  };
  return (
    <Modal title={options.title} onCancel={() => done("")} onEnter={ok}>
      <Text className="platypus-modal-label">{options.label}</Text>
      <Input ref={field} className="platypus-field" value={value} onChange={(e) => setValue(e.value)} />
      <Text className="platypus-modal-problem">{problem}</Text>
      <box className="platypus-modal-buttons">
        <Button label={options.ok ?? "OK"} primary disabled={problem !== ""} onClick={ok} />
        <Button label="Cancel" onClick={() => done("")} />
      </box>
    </Modal>
  );
}

/** Asks for a line of text; "" when cancelled. */
export function textDialog(options: TextOptions): Promise<string> {
  return new Promise<string>((resolve) => {
    let id = 0;
    const done = (value: string) => {
      closeModal(id);
      resolve(value);
    };
    id = pushModal(() => <TextForm options={options} done={done} />);
  });
}
