/**
 * @iron/engine — the TypeScript side of Iron.
 *
 * Game code extends `Component` and calls `register({ ... })`. The engine (Rust)
 * owns the list of components and calls into them through the `__iron_*` host
 * functions installed below, always by (component id, method name).
 */

import "./globals";

export { type Gamepad, type InputEvents, input, type KeyEvent } from "./input.ts";

export abstract class Component {
  /** Set by the engine when the component is created. */
  readonly id!: number;

  /** Called once, before the first `onUpdate`. */
  onStart?(): void;

  /** Called every fixed tick. `dt` is in seconds. */
  onUpdate?(dt: number): void;
}

export type ComponentClass = new () => Component;

const classes = new Map<string, ComponentClass>();
const instances = new Map<number, Component>();

/**
 * Registers component classes by name. Using an object (`register({ Hello })`)
 * keeps names stable even if the bundle is minified.
 */
export function register(components: Record<string, ComponentClass>): void {
  for (const [name, cls] of Object.entries(components)) {
    classes.set(name, cls);
  }
}

/** Queues an op for the engine. Low level: prefer the typed APIs built on it. */
export function op(name: string, ...args: unknown[]): void {
  __iron_op(name, ...args);
}

export interface FontFace {
  /** The name to use in `fontFamily`. */
  family: string;
  /** A TTF/OTF file, relative to the game folder (like `url()` in CSS). */
  src: string;
  /** `@font-face` weight; defaults to what the file says. */
  weight?: number | "normal" | "bold";
  /** `@font-face` style; defaults to what the file says. */
  style?: "normal" | "italic";
}

/**
 * Loads a font for `fontFamily` to use (like CSS `@font-face`). Text that names the
 * family re-lays out once it has loaded; until then it uses fallback fonts.
 */
export function loadFont(face: FontFace): void {
  op("loadFont", face.family, face.src, face.weight ?? null, face.style ?? null);
}

let nextStyleSheet = 1;

/**
 * Adds a stylesheet from CSS text (CSS-in-JS). `url()`s are relative to the game
 * folder. Returns an id for `removeStyleSheet`. Stylesheets imported with
 * `import "./ui.css"` are bundled into `dist/game.css` and loaded automatically.
 */
export function addStyleSheet(css: string): number {
  const id = nextStyleSheet++;
  op("addStyleSheet", id, css);
  return id;
}

/** Removes a stylesheet added with `addStyleSheet`. */
export function removeStyleSheet(id: number): void {
  op("removeStyleSheet", id);
}

// ---- Host functions called by Rust (crates/iron_script/src/components.rs) ----

globalThis.__iron_registered = () => [...classes.keys()];

globalThis.__iron_create = (className: string, id: number) => {
  const cls = classes.get(className);
  if (!cls) throw new Error(`unknown component class '${className}'`);
  const instance = new cls();
  (instance as { id: number }).id = id;
  instances.set(id, instance);
};

globalThis.__iron_dispatch = (id: number, method: string, ...args: unknown[]) => {
  const instance = instances.get(id);
  if (!instance) throw new Error(`no component with id ${id}`);
  const fn = (instance as unknown as Record<string, unknown>)[method];
  if (typeof fn === "function") return fn.apply(instance, args);
  return undefined;
};

globalThis.__iron_destroy = (id: number) => {
  instances.delete(id);
};
