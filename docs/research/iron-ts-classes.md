# iron_ts: classes (design note, for review)

Status: **proposal, no code written.** `TODO.md` M9.b2 "Classes" is a STOP item: this note is what
the user reviews before the compiler grows classes. Decisions that need a yes or no are in
section 11. Background: `iron-ts-spike.md` (IR, GC, closures), section 11 there has the counts.

## 1. Why classes, and what "done" means

Gameplay code is classes. In `js/engine/src` there are 87 `class` and 58 `extends`, 1,416 uses of
`this`, 212 getters/setters and 187 `@property` decorators; every example game's behaviour is a
class that `extends Component`. The zombie game's `sim.ts` is plain functions only because its
author separated it from `main.tsx` on purpose.

Three levels, each shippable alone:

1. **Plain classes**: fields, constructor, methods, `this`, `new`. Enough for value-like helpers,
   AI state machines, inventories.
2. **Inheritance**: `extends`, `super`, overrides, abstract classes, `instanceof`, accessors,
   statics, private members.
3. **Components**: classes that `extends Component` are created and driven by the engine (lifecycle
   hooks), carry `@property`, and talk to the engine. This level depends on "Direct engine calls"
   (section 8) and cannot be finished by classes work alone.

## 2. Object layout

A class is a struct, as an interface already is. Rules:

- **Fields in declaration order, base class fields first.** A subclass object starts with a valid
  base object, so an upcast is free (same pointer) and a base method works on a subclass instance.
- All slots stay 8 bytes; the header word is unchanged: `mark(1) | heap id(31) | kind(32)`.
- **`kind` is the class id** (what `struct_base + id + 1` already is for structs). The pointer
  masks the collector uses are registered per class and include the inherited prefix.
- **Class ids are assigned in depth-first order of the hierarchy**, so a class and all its
  descendants occupy one contiguous range `[id, id + count)`. Then `x instanceof C` is
  `lo <= kind - base < hi`: two compares on a header load, no tables, no walking.
- A field initialiser (`speed = 6`) becomes a store in the constructor. Nullable-number fields start
  as null, others as zero, as for object literals today.

No per-object vtable word (see 3): objects stay as small as they are now.

## 3. Methods and calls

A method is an ordinary IR function whose first parameter is `this`. `this.x` is a `Field` read.
`this` is a parameter, so it is already a GC root in the callee's frame.

- **Direct call when the target is known.** The compiler sees the whole program. For `a.m()` it
  computes the set of classes that can be the dynamic type of `a` (the static class and its
  descendants, class-hierarchy analysis). If only one implementation of `m` is reachable the call
  is a plain `Call`, which also lets later inlining happen. Almost all calls in gameplay code are
  like this.
- **Virtual call otherwise**: one dispatch table per overridden method, indexed by class id:
  `table_m[kind - base]` gives the function. Cost: load the header, subtract, one load, an indirect
  call. The tables are static data, built at link time. Small override sets (two or three) can use
  a compare chain instead.
- `super.m(...)` is a direct call to the base implementation; `super(...)` in a constructor is a call
  to the base constructor body.
- Closed world: a class that exists only in a module the compiler never sees (a plugin, a
  QuickJS-side subclass) cannot be dispatched to. The hybrid plan (module partition) has to treat
  an AOT class as final to the JS side or refuse such a mix; see 8.
- Method values (`const f = obj.method`, `obj.method.bind(obj)`, callbacks passed as `this.onHit`):
  allowed as a closure that captures `this` and calls the method. An arrow function inside a method
  captures `this` the same way. `this` is never reassigned, so this is the "capture by value" item in
  the closures list and is cheap (no cell).

## 4. Constructors

`new C(args)` lowers to a call of `C$new(args)`:

1. `NewObj(C)`, the pointer held in a local (so it is rooted),
2. base constructor body with `super(...)`'s arguments, or nothing for a class with no base,
3. this class's field initialisers in source order, then the rest of the constructor body.

That is the TypeScript/JS order (initialisers run after `super()` returns; arguments of `super()`
are evaluated before it, so they cannot use `this`). Parameter properties
(`constructor(readonly x = 0)`, used by `Vector3`) are stores at the top of the body. A class with no
constructor gets the implicit one. Calling a constructor without `new` and `new.target` are errors.

## 5. Members

- **Getters/setters** are methods called where the property is read or written, resolved statically
  from the class type. `obj.transform` where `transform` is an accessor is a `Call`, not a `Field`.
  An accessor in a hierarchy is virtual by the same rules as a method.
- **Statics**: static fields are module globals (name `Class.field`), initialised in `ts_init` in
  source order; static methods are plain functions. `static readonly zero = new Vector3(0, 0, 0)`
  is a global holding an object.
- **Private** (`private`, `#x`, `protected`): TypeScript already checked access; the field is an
  ordinary field, mangled with the class name so a subclass field of the same name cannot collide.
- **Abstract** classes and methods: the class cannot be `new`ed; abstract methods are virtual with
  no implementation of their own.
- `instanceof` against a class is the range test in 2. `typeof x === "number"` and friends are
  decided by static types; a union where it matters is a tagged union already.
- **Class expressions, mixins, `Symbol` members, computed members, `Object.defineProperty`,
  prototype edits, `this` rebinding with `call/apply/bind` on user code**: not supported; a clear
  error with a line number.

## 6. Classes against the type system

This is the largest hidden cost. TypeScript is structural, our structs are nominal:

- A class instance can be passed as its class or any base class: free (prefix layout).
- A class instance passed where an *interface* is expected (`Vector3Like`, which the engine uses
  everywhere, or any `{ x: number; y: number }`) needs the structural-typing item
  (`TODO.md` "Structural typing", STOP). Until that is designed, such a call is an error. The
  fall-back that works today for one case: when the target interface's fields are a prefix of the
  class's fields, the same pointer works; otherwise a copy is needed.
- Generics (`getComponent<T extends Component>(cls: ComponentClass<T>): T`) need monomorphisation
  (blocked on the type sidecar). Plain classes do not.

So plain classes (level 1) can ship before structural typing if the code that uses them passes
classes as classes.

## 7. Value classes and allocation

`Vector3`, `Quaternion` and `Color` are classes whose methods allocate a new object per operation
(`a.add(b)`). In our heap each is a 24-40 byte object and a collection candidate. Options:

- **Keep them heap objects** and rely on the collector (works now, costs an allocation per vector
  op; the busy sim's 0.9 µs/step already includes many).
- **Escape analysis / scalar replacement** (an item under Performance) removes most of the
  allocations later without any language change. Recommended.
- Do **not** add value semantics to the language: JS has reference semantics for these classes and
  a mutation visible through two names must stay visible.

## 8. Components, the engine and `@property`

How a Component works today: `Component` is a TypeScript class in `js/engine`; Rust's
`ComponentHost` creates instances by class name (`__iron_create`), lists the hooks a class has
(`__iron_hooks`) and calls `__iron_dispatch(id, "onUpdate", dt)`, which looks the method up by name.
`gameObject` and `transform` are JS objects that send ops (`__iron_op`) to Rust.

Native plan:

- **Create and dispatch**: a compiled module exports a class table. For each registered class: a
  create function (`C$new` plus setting `id` and `gameObject`) and, per lifecycle hook, the function
  address (or null). The host keeps one handle (a host GC root) per component and calls
  `hook(this, dt)` directly, with no name lookup. `Time`/`dt` stays an argument.
- **Component base fields** (`id`, `gameObject`) are a native struct prefix that Rust fills in.
- **The engine API** (`this.transform.position = ...`, `getComponent`, `Input`, physics) is where this
  becomes real work: those are TypeScript classes (`GameObject`, `Transform`) that call
  `__iron_op`. Compiled code needs either (a) these classes compiled natively too, with `__iron_op`
  replaced by `extern "C"` calls, or (b) a thin native binding layer written by hand over ids. That
  is the "Direct engine calls" item, which is its own STOP, and the Component level waits for it.
- **Mixed worlds** (UI in React stays on QuickJS): a native component may call JS only through the
  hybrid bridge (also a STOP item); until then a game is all native for gameplay classes or all JS.
- **`@property`**: the decorator is read at compile time (only `property` imported from
  `@iron/engine` is accepted; others are errors). It adds the field to the class's *reflection
  table*: name, type, byte offset. The scene loader in Rust then reads and writes the fields by
  offset, with no JS. This replaces the decorator metadata object. The table needs the field types
  the scene format uses today (numbers, strings, booleans, vectors, nested object refs); anything
  else is a compile error.
- **Hot reload and scene save/restore of component state** are development features. Release builds
  would not have them (declared in the hybrid item), except through the reflection table.

## 9. GC consequences

- Every class registers its pointer mask (including the inherited prefix) at `ts_init`. Class ids are
  inside the module's `struct_base` range, so several modules still do not collide.
- `this`, arguments and the new object are rooted like any parameter or local. The constructor's
  `NewObj` pointer stays in a local across the base constructor call (an allocating call), which the
  existing spill rules cover.
- Host handles for components are `ts_rt_root_add` roots, removed on destroy.
- Class tables (dispatch, reflection) are static data and need no tracing.

## 10. Proposed sub-tasks and sizes

| # | Sub-task | Size | Needs |
|---|---|---|---|
| 1 | Plain classes: fields, constructor, methods, `this`, `new`, parameter properties | M | none |
| 2 | Statics, private/protected, readonly, implicit constructors | S | 1 |
| 3 | Getters and setters (static dispatch) | S | 1 |
| 4 | Inheritance: prefix layout, class id ranges, `super`, `instanceof`, direct calls by CHA | M | 1 |
| 5 | Virtual calls: per-method tables, abstract classes, overridden accessors | M | 4 |
| 6 | Methods as values, `this` in arrows (by-value capture) | S | closures |
| 7 | `@property` reflection table, accepted decorators, compile errors for the rest | S | 1 |
| 8 | Class-to-interface passing | L, **STOP** | Structural typing |
| 9 | `Component` creation and lifecycle table, host handles | M | 4, 7, Direct engine calls |
| 10 | Differential tests (bun vs native) of the engine's math classes | M | 1-5 |

Tests for each step follow the existing pattern: a `bench/classes.ts` module run in bun for the
expected numbers, native tests in all three GC modes, and a hazard test written first for each new
way a pointer can be held (a constructor argument evaluated before `this` exists; a `super` call
that allocates; a method called on a field that the method itself overwrites).

## 11. Decisions I need from you

1. **Closed world**: is it acceptable that the compiler sees every class of the game and calls
   non-overridden methods directly (and refuses subclasses it cannot see)? This is what makes methods
   cheap; the cost is that an AOT class cannot be subclassed from QuickJS code.
2. **Vtable**: per-method tables indexed by class id (recommended, no object growth), or a vtable
   word in each object (simpler codegen, +8 bytes per object).
3. **Order**: plain classes (1-3) first, then inheritance (4-5), and leave Components for after the
   engine-calls design? Or design the engine boundary (Direct engine calls note) before any class
   code, since it shapes `Component`'s fields?
4. **Value classes**: keep `Vector3` and friends as heap objects for now (section 7)?
5. **Structural typing**: should its design note come before class-to-interface passing, so
   `Vector3Like` parameters are decided once? (Recommended: yes.)
6. **Scope cut**: the unsupported list in section 5 (mixins, class expressions, computed members,
   prototype tricks) stays unsupported for good?
