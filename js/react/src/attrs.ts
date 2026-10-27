// SPDX-License-Identifier: Apache-2.0

// Element attributes used by CSS selectors (crates/platypus_ui/src/attrs.rs).

type AttrValue = string | number | boolean | null;

const ATTRS: ReadonlySet<string> = new Set([
  "className",
  "id",
  "disabled",
  "checked",
  "draggable",
  "tabIndex",
  "dir",
  // Text fields.
  "value",
  "defaultValue",
  "placeholder",
  "readOnly",
  "rows",
  // Images.
  "src",
  // Dialogs.
  "open",
  "modal",
  "closedBy",
]);

/**
 * SVG element types. Their props (other than `style`, `children` and handlers) are all
 * attributes, sent as written (`strokeWidth`, `viewBox`); `text` inside an `svg` is SVG
 * text, so it passes its props through too.
 */
export const SVG_TYPES: ReadonlySet<string> = new Set([
  "svg",
  "g",
  "rect",
  "circle",
  "ellipse",
  "line",
  "polyline",
  "polygon",
  "path",
  "tspan",
  "defs",
  "use",
  "symbol",
  "linearGradient",
  "radialGradient",
  "stop",
  "clipPath",
  "mask",
  "pattern",
  "title",
]);

const NOT_ATTRS: ReadonlySet<string> = new Set(["children", "style", "key", "ref"]);

/** Whether `name` is sent as an attribute of an element of `type`. */
function isAttrOf(name: string, value: unknown, type: string): boolean {
  if (isAttr(name)) return true;
  if (!SVG_TYPES.has(type) && type !== "text") return false;
  if (NOT_ATTRS.has(name) || /^on[A-Z]/.test(name)) return false;
  return value === null || ["string", "number", "boolean"].includes(typeof value);
}

/** Props that are attributes: `className`, `id`, `disabled`, `checked`, `tabIndex`, `dir`, text field props, `data-*`. */
export function isAttr(name: string): boolean {
  return ATTRS.has(name) || name.startsWith("data-");
}

/** Attribute props as `[name, value, ...]`, skipping `undefined`. */
export function attrEntries(props: Record<string, unknown>, type = "box"): AttrValue[] {
  const out: AttrValue[] = [];
  for (const [name, value] of Object.entries(props)) {
    if (value !== undefined && isAttrOf(name, value, type)) out.push(name, value as AttrValue);
  }
  return out;
}

/** Changed attributes as `[name, value, ...]`; removed ones are sent as `null`. */
export function attrDiff(prev: Record<string, unknown>, next: Record<string, unknown>, type = "box"): AttrValue[] {
  const out: AttrValue[] = [];
  for (const [name, value] of Object.entries(next)) {
    if (value !== undefined && value !== prev[name] && isAttrOf(name, value, type)) {
      out.push(name, value as AttrValue);
    }
  }
  for (const [name, value] of Object.entries(prev)) {
    if (value !== undefined && next[name] === undefined && isAttrOf(name, value, type)) out.push(name, null);
  }
  return out;
}
