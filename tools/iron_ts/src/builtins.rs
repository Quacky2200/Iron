//! What the compiler itself gives a program, beyond the engine manifests: the component base class,
//! `register` and `@property`, timers, the generic `GameObject` members, the typed window
//! messages, `Random.pick<T>`, and a few members whose script type is wider than the native one.
//! One entry per declaration (its TypeScript signature and doc; decision 1 of
//! docs/research/iron-generated-dts-plan.md: a raw signature in this table counts as generated
//! from Rust). `types_dts.rs` prints them into `types/iron.d.ts`: a member inside the generated
//! class or namespace of its owner (it replaces a native member of the same name), a top-level
//! declaration after the engine's. Each entry's comment names the code that lowers it.

use crate::lower::components::{CONTACT_HOOKS, HOOKS, hook_param};

/// Where a built-in declaration goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Place {
    /// A top-level declaration of this name (its text is the whole declaration, `declare`
    /// included); `@iron/engine` re-exports it.
    Top(&'static str),
    /// A static member of a generated class, or a member of a generated namespace. The signature
    /// is written as a class member (`pick<T>(..): T;`, `readonly roots: T;`); inside a namespace it
    /// becomes `function ..` / `const ..`.
    Static(&'static str),
    /// An instance member of a generated class.
    Member(&'static str),
    /// A constructor of a generated class (they replace the generated ones).
    Ctor(&'static str),
}

/// One declaration.
#[derive(Clone, Copy, Debug)]
pub struct Builtin {
    pub place: Place,
    pub sig: &'static str,
    pub doc: &'static str,
}

const fn top(name: &'static str, sig: &'static str, doc: &'static str) -> Builtin {
    Builtin {
        place: Place::Top(name),
        sig,
        doc,
    }
}

const fn stat(owner: &'static str, sig: &'static str, doc: &'static str) -> Builtin {
    Builtin {
        place: Place::Static(owner),
        sig,
        doc,
    }
}

const fn member(owner: &'static str, sig: &'static str, doc: &'static str) -> Builtin {
    Builtin {
        place: Place::Member(owner),
        sig,
        doc,
    }
}

const fn ctor(owner: &'static str, sig: &'static str) -> Builtin {
    Builtin {
        place: Place::Ctor(owner),
        sig,
        doc: "",
    }
}

pub const BUILTINS: &[Builtin] = &[
    // ---- Timers (`lower/prelude.rs` `TIMERS`, over the runtime's timer queue).
    top(
        "setTimeout",
        "declare function setTimeout(callback: (arg?: number) => void, ms?: number): number;",
        "Calls `callback` once after `ms` milliseconds of engine time (0 by default).",
    ),
    top(
        "setInterval",
        "declare function setInterval(callback: (arg?: number) => void, ms?: number): number;",
        "Calls `callback` every `ms` milliseconds of engine time.",
    ),
    top(
        "clearTimeout",
        "declare function clearTimeout(id: number): void;",
        "Cancels a `setTimeout`.",
    ),
    top(
        "clearInterval",
        "declare function clearInterval(id: number): void;",
        "Cancels a `setInterval`.",
    ),
    top(
        "queueMicrotask",
        "declare function queueMicrotask(callback: () => void): void;",
        "Runs `callback` after the current script turn, before timers.",
    ),
    // ---- Components (`lower/components.rs`: `register` is erased into exported thunks, the base
    // class is its prelude; `engine_objs.rs` lowers the generic members).
    top(
        "ComponentClass",
        "type ComponentClass<T = Component> = new () => T;",
        "A component class (what `register` takes).",
    ),
    top(
        "ClassOf",
        "type ClassOf<T> = { prototype: T };",
        "A class, engine or script, by its instances' type (what `getComponent` takes).",
    ),
    top(
        "register",
        "declare function register(components: Record<string, ComponentClass>, options?: { spawn?: boolean }): void;",
        "Registers component classes by name (scene files use the names).",
    ),
    top(
        "property",
        "declare function property(value: unknown, context: ClassFieldDecoratorContext | ClassGetterDecoratorContext): void;",
        "Marks a field as saved in scenes and kept across hot reloads.",
    ),
    // ---- GameObject (`lower/engine_objs.rs`: options, components, `findObjectOfType`;
    // `components.rs` `children_helpers` / `tree_helpers`; `facade.rs` `lower_create_primitive`).
    top(
        "GameObjectOptions",
        "interface GameObjectOptions {\n  parent?: GameObject | null;\n  position?: Vector3Like;\n  rotation?: QuaternionLike;\n  scale?: Vector3Like;\n}",
        "Where a new object goes: its parent, and its position, rotation and scale (local to the parent).",
    ),
    top(
        "InstantiateObjectOptions",
        "interface InstantiateObjectOptions {\n  parent?: GameObject | null;\n  position?: Vector3Like;\n  rotation?: QuaternionLike;\n}",
        "Where a copy goes (`GameObject.instantiate`).",
    ),
    ctor(
        "GameObject",
        "constructor(name?: string, options?: GameObjectOptions);",
    ),
    member(
        "GameObject",
        "readonly components: Component[];",
        "The components of this program on the object.",
    ),
    member(
        "GameObject",
        "readonly children: GameObject[];",
        "Its children, in order.",
    ),
    member(
        "GameObject",
        "readonly scene: Scene;",
        "The scene it is in.",
    ),
    member(
        "GameObject",
        "addComponent<T>(cls: ClassOf<T>, props?: Partial<T>): T;",
        "Adds a component of a class (engine or script), with some of its properties set.",
    ),
    member(
        "GameObject",
        "getComponent<T>(cls: ClassOf<T>): T | undefined;",
        "The first component of a class on the object.",
    ),
    member(
        "GameObject",
        "getComponents<T>(cls: ClassOf<T>): T[];",
        "Every component of a class on the object.",
    ),
    member(
        "GameObject",
        "getComponentInChildren<T>(cls: ClassOf<T>): T | undefined;",
        "The first component of a class on the object or below it.",
    ),
    member(
        "GameObject",
        "removeComponent(component: Component): void;",
        "Removes a component of this program from the object.",
    ),
    stat(
        "GameObject",
        "findObjectOfType<T>(cls: ClassOf<T>): T | undefined;",
        "The first component of a class on any object.",
    ),
    stat(
        "GameObject",
        "readonly roots: readonly GameObject[];",
        "The objects without a parent.",
    ),
    stat(
        "GameObject",
        "dontDestroyOnLoad(go: GameObject): void;",
        "Keeps an object (and what is under it) when a single-mode scene load replaces the others.",
    ),
    stat(
        "GameObject",
        "instantiate(original: GameObject | Prefab, options?: InstantiateObjectOptions): GameObject;",
        "A copy of an object or a prefab, with its components.",
    ),
    stat(
        "GameObject",
        "createPrimitive(type: PrimitiveType, options?: GameObjectOptions & { name?: string }): GameObject;",
        "An object with a mesh renderer of a built-in mesh.",
    ),
    // ---- Window messages (`lower/messages.rs` over the JSON natives; `facade.rs` picks them).
    member(
        "Window",
        "postMessage(data: unknown): boolean;",
        "Queues `data` (structured values only: no handles, functions, Map or Set) for this window's domain; delivered at the start of that domain's next dispatch, in order per sender. False when the data is over 1 MiB as JSON or the window is closed.",
    ),
    member(
        "Window",
        "onMessage<T = unknown>(f: (from: Window, data: T) => void): number;",
        "A message posted to this window. Runs in this window's domain. Give `T` (or annotate the listener's `data`).",
    ),
    member(
        "Window",
        "onRequest<T = unknown, R = unknown>(f: (from: Window, data: T, reply: (answer: R) => void) => void): number;",
        "A request posted to this window; call `reply` once (a second call is ignored).",
    ),
    member(
        "Window",
        "request<R = unknown>(data: unknown, timeoutMs?: number): Promise<R>;",
        "Posts `data` as a request and resolves with the reply. Rejects with an `Error` when the window closes, no `onRequest` listener exists in its domain when it is delivered, or nobody replies within `timeoutMs` (default 10000).",
    ),
    member(
        "Window",
        "emit(name: string, payload?: unknown): boolean;",
        "Named events on top of the raw messages: sends event `name` (with an optional payload) to this window. Same limits as `postMessage`; false when refused. Throws on an empty name.",
    ),
    member(
        "Window",
        "on<T = unknown>(name: string, f: (payload: T, from: Window) => void): number;",
        "Runs `f(payload, from)` for each event `name` posted to this window, in its domain; other names are ignored. The handler's parameter type decodes the payload (a handler without parameters takes none).",
    ),
    member(
        "Window",
        "ask<R = unknown>(name: string, payload?: unknown, timeoutMs?: number): Promise<R>;",
        "Asks event `name`; resolves with what `answer(name, ..)` returned. Rejects like `request`, or at once with `no answer for \"name\"` when the window has no `answer` for it.",
    ),
    member(
        "Window",
        "answer<T = unknown, R = unknown>(name: string, f: (payload: T, from: Window) => R): number;",
        "Answers the asks of event `name` with the handler's return value.",
    ),
    stat(
        "Window",
        "onMessage<T = unknown>(f: (from: Window, data: T) => void): number;",
        "Messages to any window of the current domain (after the per-window listeners of the target).",
    ),
    stat(
        "Window",
        "onRequest<T = unknown, R = unknown>(f: (from: Window, data: T, reply: (answer: R) => void) => void): number;",
        "Requests to any window of the current domain; `reply` answers the sender's promise (once).",
    ),
    stat(
        "Window",
        "broadcast(data: unknown): number;",
        "Posts `data` to the root window of every other family. Returns how many were queued.",
    ),
    stat(
        "Window",
        "emitAll(name: string, payload?: unknown): number;",
        "`broadcast` of a named event (see `Window.emit`).",
    ),
    // ---- Types `#[ts_param]` names that no manifest declares (`Mesh.createFlat`'s lists; the
    // compiler reads a typed array or a plain one, `expr.rs` `coerce_ext`).
    top(
        "NumberList",
        "type NumberList = readonly number[] | Float32Array | Float64Array | Uint32Array | Uint16Array | Int32Array;",
        "Numbers as a plain array or a typed array (for meshes made in bulk).",
    ),
    // ---- `Random.pick<T>` (`lower/facade.rs` `lower_random_pick`).
    stat(
        "Random",
        "pick<T>(items: readonly T[]): T | undefined;",
        "A random item from a list (`undefined` when empty).",
    ),
    // ---- Wider than the native types: a string where a handle with `fromString` goes
    // (`lower/expr.rs` `coerce_ext`), and `Color`'s optional alpha (the value class's fields).
    member(
        "MeshRenderer",
        "mesh: Mesh | PrimitiveType;",
        "A mesh, or the name of a primitive.",
    ),
    member(
        "AudioSource",
        "playOneShot(clip: Sound | string, volumeScale?: number): void;",
        "Plays `clip` once over whatever else plays (footsteps, hits), at `volumeScale` x `volume`.",
    ),
    ctor(
        "Color",
        "constructor(r: number, g: number, b: number, a?: number);",
    ),
];

/// The component base class, from the hooks the host calls (`components.rs`): the frame hooks
/// with what their thunks pass, then the members every component has.
fn component_class() -> String {
    let mut s = String::from(
        "/**\n * The base of a script component: `class Player extends Component { .. }`, then `register({ Player })`.\n * The contact hooks (",
    );
    let contacts: Vec<String> = CONTACT_HOOKS.iter().map(|h| format!("`{h}`")).collect();
    s += &contacts.join(", ");
    s += ", and their `..2D` forms) take a `Collision` (`Collision2D`), a trigger's the other `Collider` (`Collider2D`); they are not\n * declared, so a game writes them without `override`.\n */\ndeclare abstract class Component {\n  /** Set by the engine when the component is created. */\n  readonly id: number;\n  /** The object this component is attached to. */\n  readonly gameObject: GameObject;\n  /** The object's transform. */\n  readonly transform: Transform;\n";
    for h in HOOKS {
        let param = hook_param(h).map_or("", |(_, p)| p);
        s += &format!("  {h}?({param}): void;\n");
    }
    s += "  /** After a hot reload, for a component restored from the old code (instead of `onAwake`). */\n  onReload?(): void;\n  /** Another component on the same object. */\n  getComponent<T>(cls: ClassOf<T>): T | undefined;\n  /** Removes this component from its object. */\n  destroy(): void;\n}\n";
    s
}

/// The name a member signature declares (`pick<T>(..)` is `pick`, `readonly roots: T` is `roots`).
pub fn member_name(sig: &str) -> &str {
    let s = sig.trim_start();
    let s = s.strip_prefix("readonly ").unwrap_or(s);
    let end = s
        .find(|c: char| !(c.is_alphanumeric() || c == '_' || c == '$'))
        .unwrap_or(s.len());
    &s[..end]
}

fn of(place: impl Fn(Place) -> bool) -> impl Iterator<Item = &'static Builtin> {
    BUILTINS.iter().filter(move |b| place(b.place))
}

/// The built-ins of an owner's static side.
pub fn statics(owner: &str) -> impl Iterator<Item = &'static Builtin> + '_ {
    of(move |p| matches!(p, Place::Static(o) if o == owner))
}

/// The built-in instance members of a class.
pub fn members(owner: &str) -> impl Iterator<Item = &'static Builtin> + '_ {
    of(move |p| matches!(p, Place::Member(o) if o == owner))
}

/// The built-in constructors of a class.
pub fn ctors(owner: &str) -> impl Iterator<Item = &'static Builtin> + '_ {
    of(move |p| matches!(p, Place::Ctor(o) if o == owner))
}

/// The top-level declarations (the component class first), each with its doc.
pub fn top_level() -> String {
    let mut s = component_class();
    for b in of(|p| matches!(p, Place::Top(_))) {
        if !b.doc.is_empty() {
            s += &format!("/** {} */\n", b.doc);
        }
        s += b.sig;
        s += "\n";
    }
    s
}

/// The names the top-level declarations give (`@iron/engine` re-exports them).
pub fn top_names() -> Vec<&'static str> {
    let mut names = vec!["Component"];
    names.extend(of(|p| matches!(p, Place::Top(_))).map(|b| match b.place {
        Place::Top(n) => n,
        _ => unreachable!("filtered to top-level entries"),
    }));
    names
}

/// A class member signature as a namespace member (`function pick<T>(..)`, `const roots: T`).
pub fn as_namespace_member(sig: &str) -> String {
    match sig.strip_prefix("readonly ") {
        Some(rest) => format!("const {rest}"),
        None if member_is_property(sig) => format!("let {sig}"),
        None => format!("function {sig}"),
    }
}

fn member_is_property(sig: &str) -> bool {
    let name = member_name(sig);
    sig.trim_start()[name.len()..].starts_with(':')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn member_signatures_become_namespace_members() {
        assert_eq!(member_name("pick<T>(a: T[]): T;"), "pick");
        assert_eq!(member_name("readonly roots: X[];"), "roots");
        assert_eq!(
            as_namespace_member("pick<T>(a: T[]): T;"),
            "function pick<T>(a: T[]): T;"
        );
        assert_eq!(
            as_namespace_member("readonly roots: X[];"),
            "const roots: X[];"
        );
        assert_eq!(as_namespace_member("count: number;"), "let count: number;");
    }

    // M9-TS-3360
    #[test]
    fn the_component_class_has_every_hook_the_host_calls() {
        let c = component_class();
        for h in HOOKS {
            assert!(c.contains(&format!("  {h}?(")), "{h}: {c}");
        }
        assert!(c.contains("  onUpdate?(dt: number): void;"), "{c}");
        assert!(
            c.contains("  onApplicationFocus?(hasFocus: boolean): void;"),
            "{c}"
        );
        assert!(c.contains("`onTriggerStay`"), "{c}");
        // Every top-level entry names what it declares.
        for b in of(|p| matches!(p, Place::Top(_))) {
            let Place::Top(n) = b.place else {
                unreachable!()
            };
            assert!(b.sig.contains(n), "{n}: {}", b.sig);
        }
    }
}
