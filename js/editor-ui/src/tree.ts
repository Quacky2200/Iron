// SPDX-License-Identifier: Apache-2.0

// The tree view's model: flattening the expanded nodes into rows (which the virtual
// list draws), selection with Shift/Ctrl ranges, and keyboard movement.

/** One node of a tree. `children` undefined means a leaf; an empty array can still expand. */
export interface TreeNode<T = unknown> {
  id: string;
  label: string;
  children?: TreeNode<T>[];
  /** Anything the app wants to keep with the node (an object id, a path...). */
  data?: T;
}

/** A visible row: a node and where it sits. */
export interface TreeRow<T = unknown> {
  node: TreeNode<T>;
  depth: number;
  parent: string | null;
  hasChildren: boolean;
  expanded: boolean;
}

/** The expanded nodes' rows in display order. Only walks into expanded nodes, so huge collapsed trees are cheap. */
export function flatten<T>(roots: readonly TreeNode<T>[], expanded: ReadonlySet<string>): TreeRow<T>[] {
  const rows: TreeRow<T>[] = [];
  // An explicit stack: deep trees don't overflow the call stack.
  const stack: { node: TreeNode<T>; depth: number; parent: string | null }[] = [];
  for (let i = roots.length - 1; i >= 0; i--) stack.push({ node: roots[i]!, depth: 0, parent: null });
  while (stack.length) {
    const { node, depth, parent } = stack.pop()!;
    const hasChildren = node.children !== undefined && node.children.length > 0;
    const open = hasChildren && expanded.has(node.id);
    rows.push({ node, depth, parent, hasChildren, expanded: open });
    if (open) {
      const kids = node.children!;
      for (let i = kids.length - 1; i >= 0; i--) stack.push({ node: kids[i]!, depth: depth + 1, parent: node.id });
    }
  }
  return rows;
}

/** Selection and focus of a tree view. */
export interface TreeState {
  expanded: ReadonlySet<string>;
  selected: ReadonlySet<string>;
  /** The row keys act on (the "current" row, drawn with a focus ring). */
  focus: string | null;
  /** Where Shift ranges start. */
  anchor: string | null;
}

export const emptyTreeState = (): TreeState => ({ expanded: new Set(), selected: new Set(), focus: null, anchor: null });

const indexOf = (rows: readonly TreeRow[], id: string | null) => (id === null ? -1 : rows.findIndex((r) => r.node.id === id));

function range(rows: readonly TreeRow[], a: string | null, b: string): Set<string> {
  const i = Math.max(0, indexOf(rows, a));
  const j = indexOf(rows, b);
  const [lo, hi] = i < j ? [i, j] : [j, i];
  return new Set(rows.slice(lo, hi + 1).map((r) => r.node.id));
}

/** A click on a row: plain selects it, Ctrl toggles it, Shift selects from the anchor (Ctrl+Shift adds the range). */
export function clickSelect(
  rows: readonly TreeRow[],
  state: TreeState,
  id: string,
  mods: { shift?: boolean; ctrl?: boolean },
): TreeState {
  if (mods.shift) {
    const span = range(rows, state.anchor ?? id, id);
    const selected = mods.ctrl ? new Set([...state.selected, ...span]) : span;
    return { ...state, selected, focus: id, anchor: state.anchor ?? id };
  }
  if (mods.ctrl) {
    const selected = new Set(state.selected);
    if (selected.has(id)) selected.delete(id);
    else selected.add(id);
    return { ...state, selected, focus: id, anchor: id };
  }
  return { ...state, selected: new Set([id]), focus: id, anchor: id };
}

/** Opens or closes one node. */
export function toggle(state: TreeState, id: string, open?: boolean): TreeState {
  const expanded = new Set(state.expanded);
  const want = open ?? !expanded.has(id);
  if (want) expanded.add(id);
  else expanded.delete(id);
  return { ...state, expanded };
}

/**
 * What a key does: arrows move (Shift extends the selection, Ctrl moves focus only),
 * Home/End/PageUp/PageDown jump, Left closes or goes to the parent, Right opens or goes
 * to the first child, Space toggles the focused row's selection with Ctrl. `null` when
 * the key isn't one of these.
 */
export function treeKey(
  rows: readonly TreeRow[],
  state: TreeState,
  key: string,
  mods: { shift?: boolean; ctrl?: boolean } = {},
  pageRows = 10,
): TreeState | null {
  if (rows.length === 0) return null;
  const at = indexOf(rows, state.focus);
  const row = at >= 0 ? rows[at] : undefined;
  let to: number;
  switch (key) {
    case "ArrowDown":
      to = Math.min(rows.length - 1, at + 1);
      break;
    case "ArrowUp":
      to = at < 0 ? 0 : Math.max(0, at - 1);
      break;
    case "Home":
      to = 0;
      break;
    case "End":
      to = rows.length - 1;
      break;
    case "PageDown":
      to = Math.min(rows.length - 1, Math.max(0, at) + Math.max(1, pageRows - 1));
      break;
    case "PageUp":
      to = Math.max(0, at - Math.max(1, pageRows - 1));
      break;
    case "ArrowLeft":
      if (!row) return null;
      if (row.expanded) return toggle(state, row.node.id, false);
      if (row.parent === null) return state;
      to = indexOf(rows, row.parent);
      break;
    case "ArrowRight":
      if (!row || !row.hasChildren) return row ? state : null;
      if (!row.expanded) return toggle(state, row.node.id, true);
      to = at + 1;
      break;
    case " ":
      if (!row) return null;
      return mods.ctrl ? clickSelect(rows, state, row.node.id, { ctrl: true }) : clickSelect(rows, state, row.node.id, {});
    default:
      return null;
  }
  const id = rows[to]!.node.id;
  if (mods.shift) return clickSelect(rows, state, id, { shift: true });
  if (mods.ctrl) return { ...state, focus: id };
  return { ...state, selected: new Set([id]), focus: id, anchor: id };
}

/** Expands every ancestor of `id` so its row shows (ids along the way from `roots`). */
export function reveal<T>(roots: readonly TreeNode<T>[], state: TreeState, id: string): TreeState {
  const path: string[] = [];
  const find = (nodes: readonly TreeNode<T>[]): boolean => {
    for (const n of nodes) {
      if (n.id === id) return true;
      if (n.children && find(n.children)) {
        path.push(n.id);
        return true;
      }
    }
    return false;
  };
  if (!find(roots)) return state;
  return { ...state, expanded: new Set([...state.expanded, ...path]) };
}
