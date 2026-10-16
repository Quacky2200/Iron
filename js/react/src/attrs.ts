// Element attributes used by CSS selectors (crates/iron_ui/src/attrs.rs).

type AttrValue = string | number | boolean | null;

/** Props that are attributes: `className`, `id`, `disabled`, `checked`, `data-*`. */
export function isAttr(name: string): boolean {
  return name === "className" || name === "id" || name === "disabled" || name === "checked" || name.startsWith("data-");
}

/** Attribute props as `[name, value, ...]`, skipping `undefined`. */
export function attrEntries(props: Record<string, unknown>): AttrValue[] {
  const out: AttrValue[] = [];
  for (const [name, value] of Object.entries(props)) {
    if (isAttr(name) && value !== undefined) out.push(name, value as AttrValue);
  }
  return out;
}

/** Changed attributes as `[name, value, ...]`; removed ones are sent as `null`. */
export function attrDiff(prev: Record<string, unknown>, next: Record<string, unknown>): AttrValue[] {
  const out: AttrValue[] = [];
  for (const [name, value] of Object.entries(next)) {
    if (isAttr(name) && value !== undefined && value !== prev[name]) out.push(name, value as AttrValue);
  }
  for (const [name, value] of Object.entries(prev)) {
    if (isAttr(name) && value !== undefined && next[name] === undefined) out.push(name, null);
  }
  return out;
}
