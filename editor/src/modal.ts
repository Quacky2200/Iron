// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// The open modal dialogs, top last. `ModalHost` (Modal.tsx) draws them over the editor; while one
// is open the menubar's accelerators wait.

export interface ModalEntry {
  id: number;
  render: () => ReactNode;
}

let stack: ModalEntry[] = [];
let next = 1;
const listeners = new Set<() => void>();

function changed(): void {
  for (const fn of [...listeners]) fn();
}

/** Opens a dialog drawn by `render`; returns its id for `closeModal`. */
export function pushModal(render: () => ReactNode): number {
  const id = next++;
  stack = [...stack, { id, render }];
  changed();
  return id;
}

/** Closes a dialog (nothing happens for one already closed). */
export function closeModal(id: number): void {
  const left = stack.filter((m) => m.id !== id);
  if (left.length === stack.length) return;
  stack = left;
  changed();
}

/** The open dialogs, bottom first (a new array after each change). */
export const modals = (): ModalEntry[] => stack;

/** Whether a dialog is open. */
export const modalOpen = (): boolean => stack.length > 0;

export function subscribeModals(fn: () => void): () => void {
  listeners.add(fn);
  return () => {
    listeners.delete(fn);
  };
}
