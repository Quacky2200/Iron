// SPDX-License-Identifier: Apache-2.0

// `@property`: marks what a component saves with its scene (s&box's `[Property]`, Unity's
// serialized fields). Works on fields and on getters (or setters) of get/set pairs, with
// standard decorators. Subclasses inherit their parents' properties.
//
//   class Health extends Component {
//     @property max = 100;
//     @property target: GameObject | null = null;
//   }

const PROPERTIES = Symbol("iron.properties");

// Bun's decorator helpers put metadata under `Symbol.metadata` when it exists, else
// under `Symbol.for("Symbol.metadata")`.
const metadataKey: symbol = (Symbol as { metadata?: symbol }).metadata ?? Symbol.for("Symbol.metadata");

type Context =
  | ClassFieldDecoratorContext
  | ClassGetterDecoratorContext
  | ClassSetterDecoratorContext
  | ClassAccessorDecoratorContext;

/** Saves this field or property with the scene. */
export function property(_value: unknown, context: Context): void {
  const meta = context.metadata as Record<symbol, string[] | undefined>;
  // Each class gets its own list, starting from its parent's (metadata inherits).
  if (!Object.hasOwn(meta, PROPERTIES)) meta[PROPERTIES] = [...(meta[PROPERTIES] ?? [])];
  const list = meta[PROPERTIES]!;
  const name = String(context.name);
  if (!list.includes(name)) list.push(name);
}

/** The `@property` names of a class and its parents, parents' first. */
export function propertiesOf(cls: abstract new (...args: never[]) => unknown): readonly string[] {
  const meta = (cls as unknown as Record<symbol, Record<symbol, string[] | undefined> | undefined>)[metadataKey];
  return meta?.[PROPERTIES] ?? [];
}
