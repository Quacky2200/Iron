//! Opcodes for UI commits, shared with `@iron/react` (`js/react/src/opcodes.ts`
//! must match). Each op is an array `[opcode, ...args]`; node IDs are numbers.
//!
//! The root container is node [`crate::ROOT`] (`0`); the reconciler's own nodes
//! start at `1`. Removing a node also destroys its whole subtree: React never
//! reinserts a removed instance (moves use `APPEND_CHILD`/`INSERT_BEFORE` directly).

/// `[CREATE, id, type]`: creates a detached node of element `type` (e.g. `"box"`).
pub const CREATE: u32 = 0;

/// `[SET_STYLE, id, prop, value, prop, value, ...]`: sets style properties.
/// A `null` value resets the property to its default.
pub const SET_STYLE: u32 = 1;

/// `[APPEND_CHILD, parent, child]`: appends (or moves) `child` to the end of `parent`.
pub const APPEND_CHILD: u32 = 2;

/// `[INSERT_BEFORE, parent, child, before]`: inserts (or moves) `child` before `before`.
pub const INSERT_BEFORE: u32 = 3;

/// `[REMOVE_CHILD, parent, child]`: detaches `child` and destroys its subtree.
pub const REMOVE_CHILD: u32 = 4;
