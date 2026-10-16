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

/// `[CREATE_TEXT, id, text]`: creates a text node (a run of characters, element
/// type `#text`). Inside `text`/`span` elements it is inline content; elsewhere it
/// is laid out as its own anonymous text block.
pub const CREATE_TEXT: u32 = 5;

/// `[SET_TEXT, id, text]`: replaces a text node's characters.
pub const SET_TEXT: u32 = 6;

/// `[SET_ATTRS, id, name, value, name, value, ...]`: element attributes used by
/// CSS selectors: `className`, `id`, `disabled`, `checked`, `tabIndex` and `data-*`.
/// A `null` value removes the attribute.
pub const SET_ATTRS: u32 = 7;
