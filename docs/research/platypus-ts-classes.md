# platypus_ts: classes (design note, for review)

Status: **proposal, no code written.** `TODO.md` M9.b2 "Classes" is a STOP item: this note is what
the user reviews before the compiler grows classes. Decisions that need a yes or no are in
section 11. Background: `platypus-ts-spike.md` (IR, GC, closures), section 11 there has the counts.

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
`ComponentHost` creates instances by class name (`__platypus_create`), lists the hooks a class has
(`__platypus_hooks`) and calls `__platypus_dispatch(id, "onUpdate", dt)`, which looks the method up by name.
`gameObject` and `transform` are JS objects that send ops (`__platypus_op`) to Rust.

Native plan:

- **Create and dispatch**: a compiled module exports a class table. For each registered class: a
  create function (`C$new` plus setting `id` and `gameObject`) and, per lifecycle hook, the function
  address (or null). The host keeps one handle (a host GC root) per component and calls
  `hook(this, dt)` directly, with no name lookup. `Time`/`dt` stays an argument.
- **Component base fields** (`id`, `gameObject`) are a native struct prefix that Rust fills in.
- **The engine API** (`this.transform.position = ...`, `getComponent`, `Input`, physics) is where this
  becomes real work: those are TypeScript classes (`GameObject`, `Transform`) that call
  `__platypus_op`. Compiled code needs either (a) these classes compiled natively too, with `__platypus_op`
  replaced by `extern "C"` calls, or (b) a thin native binding layer written by hand over ids. That
  is the "Direct engine calls" item, which is its own STOP, and the Component level waits for it.
- **Mixed worlds** (UI in React stays on QuickJS): a native component may call JS only through the
  hybrid bridge (also a STOP item); until then a game is all native for gameplay classes or all JS.
- **`@property`**: the decorator is read at compile time (only `property` imported from
  `@platypus/engine` is accepted; others are errors). It adds the field to the class's *reflection
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

## 12. Answers and the direction they set (from the user, 2026-10-18)

**Answers to section 11.** 1 (closed world), 2 (per-method tables), 5 (structural typing note first)
and 6 (scope cut): as recommended. 3 and 4 are changed by the direction below.

**Direction.** The aim is that most of the engine's TypeScript disappears: the engine is native
(Rust), and what TypeScript sees of it is **declaration files** (`.d.ts`: interfaces and `declare
class`es) plus the type files for tooling. Scripts call native functions directly, including maths
and `Date`. QuickJS stays available when needed (untyped code, mods, a dev fallback) but is not the
path the common case takes. The UI is not React: a small component/effect library (Preact, or our
own remake of components and effects) over a native UI tree, written in TypeScript the same way as
gameplay code, with no JS engine involved in the common case. The Bun watcher and the React
fast-refresh machinery go away; what stays is something that type-checks (a code server or the
TypeScript language service) and the type files.

What that changes in this note:

- **Section 8 (`Component` and the engine) is reversed.** `Component`, `GameObject`, `Transform`,
  `Vector3`, `Quaternion` and the rest are *not* compiled from TypeScript. They are declared in
  `.d.ts` and implemented in Rust. The compiler treats a `declare class` as an **extern class**:
  a known native layout (or an opaque handle id), methods and accessors that become direct calls
  or inline intrinsics, and a base class a user class can `extends` (`class Player extends
  Component`). User classes get the prefix layout of section 2 *after* the native base's fields.
  `@property` still compiles to the reflection table, and Rust reads it directly.
- **Answer 3 (order).** The engine boundary comes first as a design note ("Direct engine calls",
  rewritten around declaration files), because it fixes how an extern base class is laid out and
  how hooks are entered. Plain classes (sub-tasks 1-3) do not depend on it and can start in
  parallel; inheritance from an extern class (sub-task 9) waits for it.
- **Answer 4 (value classes).** `Vector3`, `Quaternion`, `Color` become native value types
  (glam), declared in `.d.ts` and lowered to intrinsics, not heap objects compiled from `math.ts`.
  This needs `f32` fields in the IR (today every number is f64) and a decision on value versus
  reference semantics for these types in the IR (JS gives them reference semantics; a value type
  that is copied on assignment is only safe when scripts cannot observe identity or mutate through
  two names, which `readonly` fields and the engine's own API largely guarantee). That decision
  goes in the engine-boundary note.
- **The dev loop.** Hot reload without Bun would be: the compiler is the watcher. In development
  the same IR could be compiled with Cranelift's JIT in-process and swapped in function by
  function, with state kept through the reflection table (which is also what scene save/restore
  needs); release stays ahead-of-time. That is a proposal for the hybrid/dev-loop note, not a
  result: nothing here has been tried.
- **Type checking still needs a checker.** The compiler trusts annotations today and the type
  sidecar item would use the TypeScript compiler API, which needs a JS runtime or the native port
  at build time. Dropping Bun from the *game's* run time is possible; dropping a type checker from
  the *dev* loop is not.

New design notes this creates (all **STOP** items, none started): the engine boundary ("Direct
engine calls" as declaration files plus native bindings), the UI without React, and the dev loop
without Bun. They are listed in `TODO.md` M9.b2.

## 13. Plain classes: what was built (2026-10-18)

Sub-tasks 1-3 of section 10 are done and tested (`bench/classes.ts`, `M9-TS-111`..`119`).

| Source | Lowering |
|---|---|
| `class C { x = 1; y: T; z?: U }` | a struct `C`; fields in declaration order; optional fields and `T \| null` fields start null, others zero |
| `constructor(readonly x = 0) {}` | parameter properties are fields too; the constructor is the function `C$new`: allocate, null the nullable fields, store the parameter properties, run the field initialisers in source order, run the body, return the object (a bare `return;` returns it too) |
| no constructor | an implicit empty one, from a leaked empty function parsed once |
| `new C(a, b)` | a direct call of `C$new`; defaults and rest parameters work as for functions |
| `m(a) {}` | the function `C.m(this, a)`; `this` is an ordinary first parameter, so it is a GC root like any other |
| `obj.m(a)` | a direct call, receiver first; the receiver is evaluated before the arguments (a temporary when a default has to be filled in) |
| `get x()` / `set x(v)` | functions `C.get x` / `C.set x`; a read of a property that is not a field calls the getter, an assignment calls the setter (and its value is the assigned value); `x += 1` calls both |
| `static f()` | a plain function, called as `C.f()` |
| `static x = e` | a module global `C.x`, initialised in `ts_init` in source order with the other top-level values (`static readonly` literals are folded); assignable unless `readonly` |
| `#x`, `#m()` | an ordinary field `_p_x` / method `C._p_m`; TypeScript has already checked who may use it |
| `private`, `protected`, `public`, `readonly`, `implements` | accepted and ignored |
| `this` in an arrow function | captured like a variable (the scan treats `this` as a name), so it lives in a cell |

Field types come from an annotation, or from a literal initialiser (`step = 1`), or from
`field = new Other()`. Anything else needs an annotation (`b: number = this.a + 1`), and the error
says so.

**The receiver is a pointer held across the argument list**, the same hazard as `M9-TS-27`:
`h.item.plus(swap(h))` evaluates `h.item` first, `swap` replaces it and allocates. Method calls go
through the same operand-spilling rule as every other call, and `M9-TS-116` fails when field reads
are trusted (checked by hand).

**Not built (each is an error naming the class):** `extends`/`super` and anything virtual
(sub-task 4-5), abstract classes, generic classes, decorators (`@property`, sub-task 7), static
accessors and static blocks, `accessor` fields, class expressions, methods used as values
(`const f = obj.m`; sub-task 6), arrow-function fields, `instanceof`, `typeof`.

Static fields are module variables, so a program that runs `ts_init` again starts them over, and
two threads running the same module share them (as for any module variable, section 4 of the spike
doc).

(Section 14 replaces the "not built" list above for `extends`, `super`, abstract classes and
`instanceof`.)

## 14. Inheritance: what was built (2026-10-18)

Sub-tasks 4 and 5 are done (`bench/inherit.ts`, `M9-TS-120`..`135`). Where this differs from the plan
in sections 2-4, the differences are listed last.

| Source | Lowering |
|---|---|
| `class B extends A` | the struct of `B` starts with the fields of `A` (the prefix layout), so a `B` pointer is an `A` pointer. A field `B` declares again is the same slot, and `B`'s initialiser wins |
| constructor | a function `B$init(this, ...)`; `new B(x)` allocates the object, nulls the nullable fields and calls it. `super(...)` calls `A$init` and then runs `B`'s parameter properties and field initialisers (TypeScript's order). A derived class with no constructor takes the base class's parameters, passes them on, then runs its own initialisers |
| `a.m()` | one implementation reachable from the static type of `a` (that class and every subclass): a direct call. Several: `CallVirtual`, a call through a table of function addresses indexed by the receiver's struct id (read from its header). The compiler sees the whole program, so this is exact |
| `super.m()` | a direct call of the base class's implementation on `this` |
| getters and setters | dispatched the same way (`Place::Accessor` carries the target) |
| `abstract` | the class cannot be created (`new` is an error); an abstract method is an empty stub that is never called and is left out of the tables |
| `x instanceof C` | `InstanceOf`: null is false, otherwise the header's struct id indexes a table of bytes (1 for `C` and its subclasses) |
| a member only a subclass has | after `if (x instanceof Sub)` or `x as Sub`, tsc has narrowed the type and the lowering has not, so it looks the name up in the subclasses; the topmost class that has it is used (the same pointer, seen as that class). Unrelated candidates are an error |
| subclass where a base class is expected | the same pointer: assignment, arguments, returns, arrays of them, `a ? new B() : new C()` (the common base class) and `===` between them |

Differences from section 2-4: **`instanceof` uses a byte table indexed by struct id, not a range of
class ids**, because struct ids are handed out in the order types are first mentioned, so a class
and its subclasses are not contiguous; tables cost one byte per struct per class that is tested.
Dispatch tables are indexed by struct id too and hold one 8-byte address per struct.

**Limits.** A subclass must be declared after its base class (an error says so). Parameter defaults
of an override are ignored at a virtual call: the call is lowered against the statically resolved
method. A base constructor that calls an overridden method sees the subclass's version with the
subclass's fields still zero or null (JavaScript sees `undefined`; arithmetic on those differs).
Extending an engine class (`Component`) needs the manifest work, not this.

**GC.** A virtual call's receiver is the first operand, spilled like any pointer that waits for a
later operand (`M9-TS-135` fails when field reads are trusted). `CallVirtual` and `InstanceOf` are
counted in the frame plan like `Call`.

**A bug found on the way** (`M9-TS-136`): codegen did not evaluate the value of a `Seq` whose type
is `void`, so a call that returns nothing and needed a default argument filled in (it becomes
`Seq(temporaries, call)`) never ran. Constructors, which used to return the object, hid it until
they became `void` `$init` functions.

## 15. Methods as values and arrow-function fields (2026-10-18)

Sub-task 6 is done (`M9-TS-137`, `138`).

- `obj.m` without a call is a **bound method**: a closure whose environment is a one-element array
  holding `obj`, over a small wrapper function that reads it back and calls the method (through the
  same table as a call would, so `a.sound` on a base-typed variable still reaches the subclass). The
  wrapper is made once per target. `xs.map(this.twice)` works through this.
- **Unlike JavaScript, a detached method keeps its `this`.** In JS `const f = obj.m; f()` loses the
  receiver; here it does not. A program that works in JS cannot depend on the difference (it would be
  using a lost `this`), so the tests only use methods that do not use `this` when comparing with bun.
- `handler: (k: number) => number = (k) => { this.n += k; ... }`: a field initialised with an arrow
  function that uses `this`. The constructor's scan of captured names now includes the field
  initialisers, so `this` is boxed in a cell and the closure captures it. The field needs an
  annotation (a function type is not inferred from the initialiser).

## 16. `@property` reflection tables (2026-10-18)

Sub-task 7 is done (`M9-TS-139`, `140`; the rejection of other decorators is in `M9-TS-119`).

- `@property` (or `@property()`) on an instance field marks it. The decorator is **compile-time
  metadata**: nothing is called, the `import { property } from "@platypus/engine"` is not read (the
  compiler recognises the name). Any other decorator, on a field, method or class, is an error.
- Each class with marked fields gets a table in the generated Rust bindings:
  `pub const PROPS_Player: &[(&str, usize, &str)]` = (name, byte offset in the object, type). A
  subclass's table lists its base classes' properties first, at the same offsets. The type is a short
  name a loader switches on: `number`, `boolean`, `literal`, `string`, `object:<Class>`, `array`,
  `function`, `other`, with `?` appended for a nullable field.
- A host reads and writes a property by adding the offset to the object's address: a scene loader needs
  no script to set `speed` on a `Player`, and the compiled code sees it (`tests/props.rs` does this
  for a `Player` and, through the base table, a `Hero`).
- The IR carries the tables as `Module::props` (`PropTable`, `PropEntry`).

Not done: static properties, `@property` on accessors, options (`@property({ min: 0 })`), and the
engine-side scene loader that would use the tables (engine-boundary note).
