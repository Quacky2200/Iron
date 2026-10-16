// UI commit opcodes. Must match crates/iron_ui/src/opcodes.rs.
// Each op is `[opcode, ...args]`; node 0 is the root container.

/** `[CREATE, id, type]` */
export const CREATE = 0;
/** `[SET_STYLE, id, prop, value, ...]`; a `null` value resets the prop. */
export const SET_STYLE = 1;
/** `[APPEND_CHILD, parent, child]` (also moves an attached child) */
export const APPEND_CHILD = 2;
/** `[INSERT_BEFORE, parent, child, before]` */
export const INSERT_BEFORE = 3;
/** `[REMOVE_CHILD, parent, child]`; destroys the child's subtree. */
export const REMOVE_CHILD = 4;

export const ROOT_ID = 0;

export type Op = (string | number | boolean | null)[];
/** `[CREATE_TEXT, id, text]` */
export const CREATE_TEXT = 5;
/** `[SET_TEXT, id, text]` */
export const SET_TEXT = 6;
/** `[SET_ATTRS, id, name, value, ...]`; `null` removes. */
export const SET_ATTRS = 7;
