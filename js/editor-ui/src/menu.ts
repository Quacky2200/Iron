// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// The menubar's model: menus, accelerators and keyboard movement, without React, so
// it can be tested on its own (MenuBar.tsx draws it).

/** One entry in a menu. */
export interface MenuItem {
  /** Shown text. Leave out for a separator. */
  label?: string;
  /** A line between groups (no other fields needed). */
  separator?: boolean;
  /** Shown on the right and run from anywhere: `"Ctrl+S"`, `"Ctrl+Shift+Z"`, `"F5"`, `"Delete"`. */
  accelerator?: string;
  /** A tick in front of the label. */
  checked?: boolean;
  disabled?: boolean;
  /** Runs when the item is chosen (by click, Enter or its accelerator). */
  action?: () => void;
  /** A submenu that opens to the right. */
  submenu?: MenuItem[];
}

/** A top-level menu: `File`, `Edit`, ... */
export interface Menu {
  label: string;
  items: MenuItem[];
}

/** An accelerator split into the physical key (`KeyboardEvent.code`) and modifiers. */
export interface Accelerator {
  code: string;
  ctrl: boolean;
  shift: boolean;
  alt: boolean;
  meta: boolean;
}

const NAMED_CODES: Record<string, string> = {
  del: "Delete",
  delete: "Delete",
  esc: "Escape",
  escape: "Escape",
  enter: "Enter",
  return: "Enter",
  tab: "Tab",
  space: "Space",
  backspace: "Backspace",
  insert: "Insert",
  home: "Home",
  end: "End",
  pageup: "PageUp",
  pagedown: "PageDown",
  up: "ArrowUp",
  down: "ArrowDown",
  left: "ArrowLeft",
  right: "ArrowRight",
  plus: "Equal",
  "=": "Equal",
  "-": "Minus",
  ",": "Comma",
  ".": "Period",
  "/": "Slash",
  "`": "Backquote",
  "[": "BracketLeft",
  "]": "BracketRight",
  ";": "Semicolon",
};

/** Parses `"Ctrl+Shift+S"` (case doesn't matter; `Cmd`/`Meta`/`Super` are the same key). */
export function parseAccelerator(text: string): Accelerator | null {
  const acc: Accelerator = { code: "", ctrl: false, shift: false, alt: false, meta: false };
  // "Ctrl++" ends with an empty part for the plus key.
  const parts = text.split("+").map((p) => p.trim());
  if (text.endsWith("++")) parts.splice(parts.length - 2, 2, "plus");
  for (const raw of parts) {
    const part = raw.toLowerCase();
    if (part === "ctrl" || part === "control") acc.ctrl = true;
    else if (part === "shift") acc.shift = true;
    else if (part === "alt" || part === "option") acc.alt = true;
    else if (part === "cmd" || part === "meta" || part === "super" || part === "win") acc.meta = true;
    else if (acc.code) return null;
    else acc.code = keyCode(raw);
  }
  return acc.code ? acc : null;
}

function keyCode(key: string): string {
  const lower = key.toLowerCase();
  if (/^[a-z]$/.test(lower)) return `Key${lower.toUpperCase()}`;
  if (/^[0-9]$/.test(lower)) return `Digit${lower}`;
  if (/^f([1-9]|1[0-9]|2[0-4])$/.test(lower)) return lower.toUpperCase();
  return NAMED_CODES[lower] ?? key;
}

/** Whether a key press (by code, with its modifiers) is the accelerator. */
export function matchesAccelerator(acc: Accelerator, code: string, mods: Omit<Accelerator, "code">): boolean {
  return (
    acc.code === code && acc.ctrl === mods.ctrl && acc.shift === mods.shift && acc.alt === mods.alt && acc.meta === mods.meta
  );
}

/** The enabled item whose accelerator a key press is, searching submenus too. */
export function findAccelerator(menus: readonly Menu[], code: string, mods: Omit<Accelerator, "code">): MenuItem | null {
  const search = (items: readonly MenuItem[]): MenuItem | null => {
    for (const item of items) {
      if (item.disabled || item.separator) continue;
      if (item.accelerator && item.action) {
        const acc = parseAccelerator(item.accelerator);
        if (acc && matchesAccelerator(acc, code, mods)) return item;
      }
      if (item.submenu) {
        const found = search(item.submenu);
        if (found) return found;
      }
    }
    return null;
  };
  for (const menu of menus) {
    const found = search(menu.items);
    if (found) return found;
  }
  return null;
}

/** Whether keyboard movement can stop on an item. */
export const selectable = (item: MenuItem | undefined): boolean => !!item && !item.separator && !item.disabled;

/**
 * The next selectable index from `from` going `step` (+1 down, -1 up), wrapping; `from`
 * -1 starts before the first item. -1 when nothing is selectable.
 */
export function nextIndex(items: readonly MenuItem[], from: number, step: 1 | -1): number {
  const n = items.length;
  if (n === 0) return -1;
  let i = from < 0 ? (step > 0 ? -1 : n) : from;
  for (let tries = 0; tries < n; tries++) {
    i = (i + step + n) % n;
    if (selectable(items[i])) return i;
  }
  return -1;
}

/** Where the menubar is: which top menu is open and the highlighted item at each level. */
export interface MenuState {
  /** The open (or, with nothing open, keyboard-highlighted) top menu; -1 for none. */
  top: number;
  /** Whether the top menu's list is showing. */
  open: boolean;
  /** Highlighted item per open level: `[i]` in the top menu, `[i, j]` in i's submenu... */
  path: number[];
}

export const CLOSED: MenuState = { top: -1, open: false, path: [] };

/** The items shown at `level` of the open menu (0: the top menu's list). */
export function itemsAt(menus: readonly Menu[], state: MenuState, level: number): MenuItem[] | null {
  let items = menus[state.top]?.items;
  for (let l = 0; l < level; l++) {
    items = items?.[state.path[l] ?? -1]?.submenu;
  }
  return items ?? null;
}

/** What a key does to the menubar: the new state and, for Enter, the item chosen. */
export function menuKey(
  menus: readonly Menu[],
  state: MenuState,
  key: string,
): { state: MenuState; choose?: MenuItem; handled: boolean } {
  if (state.top < 0) return { state, handled: false };
  const n = menus.length;
  const depth = state.path.length;
  const level = depth - 1;
  const items = level >= 0 ? itemsAt(menus, state, level) : null;
  const current = level >= 0 ? items?.[state.path[level]!] : undefined;
  const openTop = (top: number): MenuState => {
    const first = nextIndex(menus[top]?.items ?? [], -1, 1);
    return { top, open: true, path: first >= 0 ? [first] : [] };
  };
  switch (key) {
    case "Escape":
      if (!state.open) return { state: CLOSED, handled: true };
      // Close one submenu level, then the top menu (keeping the bar highlighted).
      if (depth > 1) return { state: { ...state, path: state.path.slice(0, -1) }, handled: true };
      return { state: { top: state.top, open: false, path: [] }, handled: true };
    case "ArrowLeft":
      if (depth > 1) return { state: { ...state, path: state.path.slice(0, -1) }, handled: true };
      return { state: state.open ? openTop((state.top - 1 + n) % n) : { ...state, top: (state.top - 1 + n) % n }, handled: true };
    case "ArrowRight":
      if (current?.submenu && selectable(current)) {
        const first = nextIndex(current.submenu, -1, 1);
        if (first >= 0) return { state: { ...state, path: [...state.path, first] }, handled: true };
      }
      return { state: state.open ? openTop((state.top + 1) % n) : { ...state, top: (state.top + 1) % n }, handled: true };
    case "ArrowDown":
    case "ArrowUp": {
      if (!state.open) return { state: openTop(state.top), handled: true };
      if (!items) return { state, handled: true };
      const i = nextIndex(items, state.path[level] ?? -1, key === "ArrowDown" ? 1 : -1);
      return { state: { ...state, path: [...state.path.slice(0, -1), i] }, handled: true };
    }
    case "Home":
    case "End": {
      if (!items) return { state, handled: true };
      const i = nextIndex(items, -1, key === "Home" ? 1 : -1);
      return { state: { ...state, path: [...state.path.slice(0, -1), i] }, handled: true };
    }
    case "Enter":
    case " ": {
      if (!state.open) return { state: openTop(state.top), handled: true };
      if (!current || !selectable(current)) return { state, handled: true };
      if (current.submenu) {
        const first = nextIndex(current.submenu, -1, 1);
        return { state: { ...state, path: [...state.path, first] }, handled: true };
      }
      return { state: CLOSED, choose: current, handled: true };
    }
  }
  return { state, handled: false };
}

/** The accelerator as shown in menus (`Ctrl+Shift+S`), or the text as written if it doesn't parse. */
export function acceleratorLabel(text: string): string {
  const acc = parseAccelerator(text);
  if (!acc) return text;
  const key = acc.code.replace(/^Key/, "").replace(/^Digit/, "");
  const names: Record<string, string> = { Equal: "+", Minus: "-", Comma: ",", Period: ".", Slash: "/" };
  return [acc.ctrl && "Ctrl", acc.alt && "Alt", acc.shift && "Shift", acc.meta && "Meta", names[key] ?? key]
    .filter(Boolean)
    .join("+");
}
