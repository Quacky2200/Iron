// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// Modal dialogs drawn by the editor's own UI (no OS dialogs), as HTML's modal `<dialog>`: in the
// top layer over a dimmed `::backdrop`, everything outside inert (Tab, d-pad and the pointer stay
// in the dialog), focus moved in when it opens and given back when it closes, Escape to cancel
// and Enter for the default button. `ModalHost` draws the open ones (modal.ts); `confirmDialog`
// and `textDialog` are the common ones, as promises of the answer.

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

/** The dialog's shell: an open modal `dialog` (the engine keeps focus inside it). */
export function Modal({ title, width = 420, children, onCancel, onEnter }: ModalProps) {
  return (
    <dialog
      open
      modal
      className="platypus-modal"
      style={{ width }}
      tabIndex={-1}
      onCancel={(e: PlatypusEvent) => {
        // Escape: the dialog goes when the editor drops it from its stack, not by itself.
        e.preventDefault();
        onCancel();
      }}
      onKeyDown={(e: PlatypusKeyboardEvent) => {
        if (e.key === "Enter" && onEnter) {
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
    </dialog>
  );
}

export interface ButtonProps {
  label: string;
  onClick: () => void;
  primary?: boolean;
  disabled?: boolean;
}

/** A dialog button: clicked, or Enter / Space while focused (Tab reaches it). */
export function Button({ label, onClick, primary = false, disabled = false }: ButtonProps) {
  const classes = primary ? "platypus-button primary" : "platypus-button";
  return (
    <box
      className={disabled ? `${classes} disabled` : classes}
      tabIndex={disabled ? -1 : 0}
      onClick={() => !disabled && onClick()}
      onKeyDown={(e: PlatypusKeyboardEvent) => {
        if (e.key !== "Enter" && e.key !== " ") return;
        // Not also the dialog's default button.
        e.preventDefault();
        e.stopPropagation();
        if (!disabled) onClick();
      }}
    >
      <Text>{label}</Text>
    </box>
  );
}

/** The open dialogs, over everything else (each a modal `dialog` in the top layer). */
export function ModalHost() {
  const open = useSyncExternalStore(subscribeModals, modals);
  if (open.length === 0) return null;
  return (
    <box className="platypus-modal-layer">
      {open.map((m) => (
        <box key={m.id} className="platypus-modal-slot">
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
