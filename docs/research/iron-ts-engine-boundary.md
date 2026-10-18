# iron_ts: the engine boundary (design note, for review)

Status: **proposal, no code written.** This is the design note for `TODO.md` M9.b2 "Direct engine
calls" (a STOP item), set by the direction recorded in `iron-ts-classes.md` section 12: the engine is
native, TypeScript sees it through declaration files (`.d.ts`), scripts call native functions
directly, QuickJS stays an option. Questions that need a yes or no are in section 11.

## 1. What this has to achieve

1. Compiled gameplay code calls the engine (`transform.position = p`, `Physics.raycast(...)`,
   `Time.deltaTime`, `Mathf.sin`) as a plain native call: no `Value` conversion, no op queue, no
   string-named dispatch.
2. A user class can `extends Component` and be driven by the engine (`onUpdate(dt)` and the other
   hooks) with no name lookup and no JS engine.
3. The engine's API exists **once**: a native implementation, from which the declaration files, the
   compiler's knowledge of the engine, and (while it is kept) the QuickJS bindings are generated.
   Nothing is written twice.
4. The existing examples keep running while this is built, one module at a time.

## 2. What exists today (measured by reading the code, 2026-10-18)

- `js/engine/src` is 11,468 lines of TypeScript. Scripts reach Rust only through
  `__iron_op(name, ...args)`: about 85 distinct op names in these families: `render` 19, `physics`
  15, `audio` 15, `scene` 9, `prefs` 4, `app` 3, `cursor` 3, plus `asset`, `cloth`, `screen`,
  `loadFont`, style sheets. Arguments are the generic `Value` (null, bool, number, string, array),
  queued in an `OpBuffer` and applied by name once per frame. `iron_core/src/ops.rs` already says hot
  paths should move to a binary encoding.
- **The TypeScript keeps a mirror of the world.** `Transform` stores `_position`, `_rotation`,
  `_scale` in TS and sends `scene.local` (three arrays) on every set; Rust sends moves back in a
  batch, `__iron_scene(moved)`, as flat numbers. Physics contacts, raycast results, input and audio
  endings come back the same way, as batches into `__iron_physics`, `__iron_input`, `__iron_audio`.
- **Rust owns the component list, JS holds the instances.** `ComponentHost` creates components by
  class name (`__iron_create`), asks which hooks a class has (`__iron_hooks`), and calls
  `__iron_dispatch(id, "onUpdate", dt)` by name. A component that throws is reported and disabled.
- The measured cost of crossing: in the sine-circle comparison (`iron-ts-spike.md` section 8) the
  QuickJS frame spent about 0.12 ms of 1.26 ms in `__iron_op` plus draining it, for one op.
- Where the 11.5k lines go, roughly: thin wrappers over ops (physics, audio, rendering component
  classes, app, prefs, screen); and **real logic that lives only in TypeScript**: `math.ts` (1,025:
  vectors, quaternions, colours, matrices), `scenes.ts` (1,044: JSON scene load, `SceneManager`,
  `instantiate`), `animation.ts` (921: curves, clips, blend trees, `Animator`), `time.ts`/`clock.ts`
  (`DateTime`, `TimeSpan`, `Time`, `Task`), `input.ts`, `random.ts`, `lines.ts`/`gizmos.ts`, and the
  hot-reload and property reflection code (`hot.ts`, `property.ts`).

The consequence: the aim "none of the engine in TypeScript" is not a binding exercise. About half of
`js/engine` is logic that has to be rewritten in Rust (or stay as library source compiled with the
game). That is the bulk of the work in this item, and it is mostly mechanical.

## 3. The shape: declarations over a native API

```text
Rust engine functions (#[iron_api])  --generates-->  engine_api.json   (the manifest)
                                                       |-- iron.d.ts          (for tsc, editors, users)
                                                       |-- extern "C" shims   (the symbols to link)
                                                       |-- compiler extern table (iron_ts)
                                                       |-- QuickJS bindings   (while QuickJS is kept)
```

The Rust side is the source of truth, written once, annotated with what the generators cannot guess
(f32, handle versus value, nullable, doc comment). The compiler reads the **manifest**, not the
`.d.ts`: a `.d.ts` says what TypeScript needs (`readonly x: number`), not which symbol to call or how
a `Vector3` is passed. The `.d.ts` is generated, so it cannot drift, and carries the doc comments.

This is a proposal with a real alternative (section 11, question 1): hand-write the `.d.ts` and the
Rust, and test that they agree. That is simpler to start and drifts.

## 4. Four kinds of declared type

The compiler needs to know, per declared type, how its values are represented. Four kinds cover
`js/engine`:

### 4.1 Handle classes (identity objects owned by the engine)

`GameObject`, `Transform`, `Rigidbody`, `Collider`, `AudioSource`, `Camera`, `Light`, `MeshRenderer`,
`Animator`, ... A value is the engine's **generational id**, a 64-bit integer (`ObjectId` in `iron_scene` is an index plus a
generation already; today scripts use their own numeric ids that the engine maps to it, and a handle
would be the engine id itself, which removes that map). Proposal: a new IR type `Handle(class)`, an `i64` in a register. It is not a heap object, so it
costs no allocation and no GC root; `===` compares ids; zero is `null`/`undefined`.

Methods and accessors become direct calls: `t.position` is `ts_eng_Transform_position(h, &out)`,
`t.position = v` is `ts_eng_Transform_set_position(h, x, y, z)`. A stale handle (the object was
destroyed) must not crash: the engine checks the generation and the call does nothing / returns the
default. The arena already behaves like this: a stale id misses.

Because the engine owns the data, the TypeScript mirror goes away: there is one copy of a transform,
in Rust, and `transform.position` reads it. (The `moved` batch exists only to keep the mirror true.)

### 4.2 Value classes (immutable data)

`Vector2/3/4`, `Quaternion`, `Color`, `Matrix`, `Rect`, `Bounds`, `Ray`, `Vector3Int`. Properties of
these are `readonly`, methods return new values, `equals` exists for comparison, so identity is not
meaningful in correct code. Where the spec says "float", the engine's glam types are `f32`.

Representation (question 2, section 11):

- **Phase 1 (recommended): ordinary compiled structs.** The `.d.ts` is `declare class Vector3 { readonly
  x: number; ... }` and the compiler gives it the struct layout it already has (a header and f64
  slots). Methods are **intrinsics**: `a.add(b)` is inlined arithmetic on the fields, `a.normalized`
  an inline sequence, `Quaternion.slerp` a call to a Rust function taking scalars. Engine calls pass
  and return scalars (`set_position(h, x, y, z)`; results written through an out-pointer into a
  caller-allocated object). Allocation per vector operation is the known cost, as in section 7 of the
  classes note.
- **Phase 2: scalar replacement.** A value class that does not escape a function is replaced by its
  fields (three locals). This is the "escape analysis" item under Performance; the contract does not
  change.
- Not recommended: passing aggregates by value through `extern "C"`. Win64 and SysV disagree about
  small structs and `f32x3` is not a register type; scalarising at the boundary avoids both.

**f32 consequence.** Fields stay f64 in the script heap (the IR has one number type), and are narrowed
to f32 when they cross into the engine. So `t.position = v; t.position.x` now reads back the
f32-rounded value (`0.1` becomes `0.10000000149011612`), where today it returns exactly what was set
because the TypeScript mirror stores f64. That is what Unity does, and it is the correct reading of
"the engine is native", but it is a visible change; it needs a yes (question 3).

### 4.3 Singletons and namespaces

`Time`, `Input`, `Physics`, `Debug`, `Application`, `Screen`, `Random`, `Mathf`, `PlayerPrefs`,
`QualitySettings`, `SceneManager`. Static members are direct calls with no receiver: `Time.deltaTime`
is `ts_eng_Time_deltaTime()`; static properties with setters likewise. Constants are inlined from
the manifest (`Mathf.PI`, enum members).

### 4.4 Extern base classes (`Component`)

`Component` is declared, not compiled. A user class `class Player extends Component` gets the prefix
layout of the classes note with the **native component header first**: the compiler reserves a fixed
prefix (the component's `id` and its `gameObject` handle, 16 bytes after the object header) that the
engine fills when it creates the component. Everything after the prefix is the user's fields.
Inherited members (`this.transform`, `this.gameObject`, `this.getComponent(T)`) are extern calls on
the handle in the prefix.

## 5. Component creation and hooks

Today: class name string in, instance by id, hooks by name. Native:

- Each compiled module exports a **class table** (static data), one entry per `register()`ed class:
  `{ name, class_id, instance_size, create: fn(handle_prefix) -> object, hooks: [onAwake, onStart,
  onUpdate, onLateUpdate, onFixedUpdate, onPreRender, onDestroy, ...] }`, each hook a function
  address or null. Which hooks a class has is known at compile time, so the `__iron_hooks` probe
  disappears and a missing hook costs nothing.
- Rust's `ComponentHost` keeps one **host root** per component (a pointer into the script heap,
  `ts_rt_root_add`), and calls `hook(this, dt)` directly. Order, enabled/active checks and `onStart`
  once are unchanged: only the call changes.
- `@property` is the reflection table of the classes note (name, type, offset), which Rust uses to
  save, load and edit components with no JS. This replaces the decorator metadata and the scene
  JSON code in `scenes.ts`.
- **Errors.** Today a component that throws is reported and disabled. Native code has no exceptions
  (`try`/`throw` is an open design item): a trap (null `this`, an out-of-range index is already
  null/NaN) cannot unwind. Until that design exists a native script fault is a fault in the game
  process. This is a real regression in robustness for development, and the dev loop note (JIT, with
  checks on) should address it.

## 6. Callbacks and events

- **Closures to Rust** (event listeners, `Task` continuations, `onCollision` registered at run time):
  a function value is already a `Closure{func, env}`; the engine receives it as an opaque pointer,
  roots it (`ts_rt_root_add`) for as long as it holds it, and calls `func(env, args...)`. The
  signature is the manifest's, so Rust knows the argument types.
- **Batches become direct calls.** Contacts and ended sounds are delivered as calls to the owning
  component's hook (`onCollisionEnter(other)`) when the engine steps; raycasts return their hit
  through an out object; input state is read by getters. The `__iron_physics(moved, contacts, rays,
  characters)` style batch disappears with the mirror.
- **Timers and `async`** (`Task`, `setTimeout`) need the event-loop contract of the open `async`
  item. Nothing here decides it.

## 7. Strings, arrays, nullable, generics

- **Strings** cross the boundary in many places (object names, tags, asset paths, scene names,
  `PlayerPrefs` keys). The strings item (STOP: UTF-16 or UTF-8) must come first for these. Until then
  the boundary can only pass numbers, booleans, handles, enums and arrays of those; string-taking
  engine functions are not callable from compiled code. This is the main ordering constraint on this
  item.
- **Arrays**: a script array is a runtime `Array` (len, cap, 8-byte slots). Engine functions that
  return lists (`getComponents`, raycast hits) fill an array the caller allocated and passed in, or
  call back per element. Engine functions never allocate in the script heap behind the collector's
  back except through the non-collecting `ts_rt_*` constructors.
- **Nullable** handles are 0; nullable numbers use the reserved NaN; nullable value classes are the
  null pointer. The manifest says which a function returns.
- **Generics**: `getComponent<T extends Component>(cls: ComponentClass<T>): T | undefined` takes a
  class and returns an instance. With generics blocked on the type sidecar, the compiler treats
  `getComponent(Player)` and `addComponent(Player, { ... })` as **intrinsics**: the class argument
  becomes the class id, the result type is the class. The same for `findObjectOfType`. These are the
  only generic engine calls in the examples.

## 8. Where each part of `js/engine/src` goes

| Module | Lines | Becomes |
|---|---|---|
| `math.ts` | 1,025 | Declared value classes (4.2); the arithmetic is intrinsics, the rest Rust (glam). `Mathf` is a singleton |
| `scene.ts` | 785 | `GameObject`, `Transform`, `Component` as handles / extern base over `iron_scene`; the mirror is deleted |
| `scenes.ts` | 1,044 | Rust: scene JSON load/save through the `@property` reflection table, `SceneManager`, `instantiate`, prefabs |
| `physics.ts`, `physics2d.ts`, `physics-components.ts` | 1,862 | Declared handle classes over `iron_physics`; the logic that wraps ops is deleted |
| `rendering.ts`, `models.ts`, `textures.ts`, `particles.ts`, `lines.ts`, `gizmos.ts`, `cloth.ts` | about 2,700 | Declared component classes whose fields are engine properties (setting `material.color` calls the engine) |
| `audio.ts`, `audio-components.ts` | 500 | Declared classes over `iron_audio` |
| `animation.ts` | 921 | **Rust port** (curves, clips, blend trees, transitions); the largest single port |
| `time.ts`, `clock.ts` | 629 | `DateTime`/`TimeSpan` become value classes with native methods; `Time` a singleton; `Task` waits for the async design |
| `input.ts`, `app.ts`, `screen.ts`, `quality.ts`, `cursor.ts`, `random.ts` | about 1,100 | Singletons over existing Rust state; `Random` needs a seedable API |
| `hot.ts`, `property.ts`, `assets.ts`, `index.ts` | rest | Replaced by the class table and reflection table, or by the dev loop note |

The `.d.ts` that replaces all of this is generated, and most of the 11.5k lines are deleted rather
than translated: the Rust that implements them mostly exists (`iron_scene`, `iron_physics`,
`iron_audio`), because the TypeScript was already a thin client of it. The genuinely new Rust is
`math` (glam is there; the API surface is not), `scenes`, `animation`, `time`.

## 9. QuickJS next to it

QuickJS stays for code the compiler cannot or should not compile (untyped code, mods, a fallback
while a module is being ported). It reaches the engine through bindings **generated from the same
manifest**: `Transform.position` in QuickJS calls the same native function. That has two uses:

- the migration path: `js/engine` shrinks module by module into `.d.ts` plus generated glue, and the
  examples keep running throughout;
- one behaviour: a native script and a QuickJS script see the same engine, with the f32 rounding.

Calls between native modules and QuickJS code (a QuickJS class extending a native component, the
reverse) belong to the hybrid-builds note; this note only requires that neither side blocks the
other.

## 10. The UI and the dev loop

Out of scope here, but they constrain this note:

- The UI tree is already native (`iron_ui`, driven by `__iron_commit` opcodes). A UI library in the
  compiled subset would call `ts_eng_ui_*` functions; the React reconciler is not needed for that.
  What it needs from the boundary: typed props and style objects (styles as `Record<string, ...>` are
  the hard case: they need structural typing and string keys), and event callbacks (6, closures to
  Rust).
- A JIT dev loop (Cranelift in-process) links the same extern table at run time instead of at link
  time; the manifest makes that a table of function addresses. Nothing in this note blocks it.

## 11. Decisions I need from you

1. **Source of truth**: Rust annotated with `#[iron_api]` generating the manifest, `.d.ts`, shims and
   QuickJS bindings (recommended); or hand-written `.d.ts` and Rust checked against each other.
2. **Value classes**: phase 1 ordinary structs with scalarised engine calls, scalar replacement
   later (recommended); or build register-passed vectors into the IR from the start.
3. **f32 readback**: accept that `position.x` reads back f32-rounded (as in Unity)?
4. **Strings first**: is it right that string-taking engine functions wait for the strings item, and
   that the first engine calls from compiled code are the numeric ones (`Transform`, `Time`, `Mathf`,
   `Rigidbody`, `Input` axes)?
5. **Robustness**: native faults take the process down until `try`/`throw` is designed. Acceptable
   for the spike, with the dev loop checks as the answer later?
6. **Generics**: treat `getComponent`/`addComponent`/`findObjectOfType` as compiler intrinsics until
   generics exist?
7. **What stays TypeScript**: anything, or is the rule "no engine logic in TypeScript at all", with
   `animation.ts`, `scenes.ts` and `math.ts` ported to Rust? (Compiling them with `iron_ts` as
   library source is the cheaper alternative and is allowed by the design above.)

## 12. Proposed sub-tasks

| # | Sub-task | Size | Needs |
|---|---|---|---|
| 1 | The manifest format and `#[iron_api]` (or the hand-written equivalent), generating `.d.ts` | M | decision 1 |
| 2 | Compiler: extern table, `Handle` type, singleton calls, direct calls with scalar arguments | M | 1 |
| 3 | First vertical slice: `Time`, `Mathf`, `Vector3` intrinsics, `Transform` position/rotation, `Input` axes, compiled `Component` subclass with `onUpdate`, driven by `ComponentHost` | M | 1, 2, plain classes |
| 4 | Class table and component creation/hooks without names | M | 3 |
| 5 | Port: delete the transform mirror; physics, audio, rendering component classes as declarations | L | 3 |
| 6 | Rust ports: `scenes`, `animation`, `time`, `random` | L | 5 |
| 7 | QuickJS bindings generated from the manifest; `js/engine` reduced to glue | M | 1 |
| 8 | Strings and arrays across the boundary | M | strings item |
| 9 | Closures to Rust (listeners), batches replaced by hooks | M | 4 |
| 10 | The arcade or platformer example running with its gameplay compiled natively | M | 4-6 |

A useful first milestone is sub-task 3: one example component, compiled, moving a `Transform` every
frame with no JS engine in the loop. It shows the whole boundary working, and it gives a number to
set against the 0.12 ms crossing above.

## 13. Answers (from the user, 2026-10-18)

1. **Source of truth: Rust annotated with `#[iron_api]`**, generating the manifest, the shims and the
   QuickJS bindings, *instead of* `__iron_op`. The user's point, which this note had only half said:
   this speeds up QuickJS as well. The QuickJS bindings become ordinary native functions registered
   with typed arguments (numbers as `f64`, handles as integers), not strings plus a `Value` array in an
   op queue. A consequence to design for: ops are queued and applied once per frame today, so a script
   never sees its own write until the engine has applied it (hence the TypeScript mirror); a direct
   call applies immediately, so reads see writes at once and the mirror can go in QuickJS too.
2. **Value classes: phase 1 ordinary structs with scalarised engine calls**, scalar replacement later.
3. **f32 readback accepted**, as in Unity.
4. **String-taking engine functions wait for the strings item**; the first engine calls from compiled
   code are the numeric ones. ("For now, until implemented": recorded as a limit that lifts when
   strings land.)
5. **Native faults take the process down for now**; the dev loop's checks are the later answer.
6. **Everything in `js/engine` is ported to Rust.** Anything left in TypeScript is user-made, and may
   be ported if it is portable (a Bun library written in TypeScript is the example given). So the
   "compile engine modules with `iron_ts` as library source" alternative in question 7 is not taken
   for the engine; it remains how *user* libraries are compiled.

**Question 6 answered later: yes**, generics-as-intrinsics (`getComponent`, `addComponent`,
`findObjectOfType`) is the working default. The paragraph below describes the state before that.

**Not answered / assumed.** The reply numbered six items for seven questions; "6" matched question 7
(what stays TypeScript) in content. Question 6, generics (`getComponent`, `addComponent`,
`findObjectOfType` as compiler intrinsics until generics exist), is therefore **still open**; the
recommendation stands as the working default and is easy to change, because it is three functions.

**What this makes next.** Sub-task 1 of section 12: the manifest format and `#[iron_api]`, generating
the `.d.ts`, the `extern "C"` shims and the QuickJS bindings. It lives in the engine workspace (a
proc-macro crate and a generator), not in `tools/`, and the engine crates that carry ops today
(`iron_scene`, `iron_physics`, `iron_audio`, `iron_script`) are where the first annotated functions
go. `iron_render` and `iron_app` carry the `render.*`, `app.*`, `cursor.*` and `screen.*` ops and
belong to another session's area, so they are left for last, and for whoever owns them.

## 14. First slice of `#[iron_api]`: built (2026-10-18)

Sub-task 1 of section 12, in its smallest useful form: two new crates in the engine workspace,
additive (nothing else depends on them, `__iron_op` is untouched).

- `crates/iron_api_macros`: `#[iron_api(namespace = "Demo", quickjs, handles(GameObject = ObjectId))]`
  on an inline `pub mod`. It keeps the module's code and adds `MANIFEST`, one `extern "C"` shim per
  `pub fn` (`iron_Demo_add`), and with `quickjs` a `register_quickjs(ctx)`.
- `crates/iron_api`: the manifest types (`Namespace`, `Function`, `Param`, `Ty`), the `Handle` trait
  (the Rust id type implements `to_raw`/`from_raw`), and the generators `to_dts()` (the TypeScript
  declaration, doc comments as JSDoc) and `to_json()` (the table the compiler will read).
- Supported so far: functions over `f64`, `f32`, `i32`, `u32`, `bool`, `()` and declared handle
  types. Handles are `number & { __handle }` in TypeScript, a `u64` through the C ABI and a plain
  number in QuickJS. No registry or linker trick: everything the macro needs is in the one module it
  is attached to.
- **QuickJS gets typed functions**, not `__iron_op(name, ...args)`: `Demo.add(1, 2)` is a native
  closure taking `f64`s. Measured with `cargo test -p iron_api --release -- --ignored --nocapture`:
  1,000,000 calls take about 150 ns each through the typed function and about 245 ns through a
  simplified `__iron_op` (a name string, a rest array of numbers, a queue push; the real op also
  converts each argument to a `Value`, so the real gap is larger). Most of the remaining 150 ns is
  QuickJS's own call overhead; the large win is for compiled code, which calls the shim directly.
- Tests: `crates/iron_api/tests/api.rs` (`M9-API-01`..`06`).

**Not built yet** (next, in order): the compiler reading the JSON manifest (an extern call node, the
`Handle` type, f32/i32/u32/bool conversions at the call), value classes (`Vector3`) scalarised across
the boundary, strings, methods on handles (`transform.position`) and the engine context those need (the
shims are free functions, so anything that touches the world must reach it through state the host
sets up, for example a thread-local current world; that design is open), and annotating the real
engine crates. The crates that carry ops today are `iron_scene`, `iron_physics`, `iron_audio` and
`iron_script`; `render.*`, `app.*`, `cursor.*` and `screen.*` live in `iron_render` and `iron_app`
and are left for their owner.

## 15. The compiler calls the engine API: built (2026-10-18)

Sub-task 2 of section 12, in its first form (`M9-TS-142`..`146`). A TypeScript module compiled with
the manifest of an `#[iron_api]` namespace calls it directly.

- `iron_ts --manifest api.json` (or `compile_with_manifests`) reads the JSON `Namespace::to_json`
  writes (`tools/iron_ts/src/manifest.rs`). `Demo.scale(3, 4)` becomes `CallExtern`, an import of the
  symbol `iron_Demo_scale` with the manifest's C signature: no op, no `Value`, no name lookup.
- Numbers stay `f64` in scripts and are converted **at the call**: `f32` demotes on the way in and
  promotes on the way out; `i32`/`u32` take a saturating, truncating conversion (`-5.7` becomes `-5`,
  as `as i32` does in Rust); `bool` is a byte.
- **`Ty::Handle(name)`** is the new IR type for an engine object id: a 64-bit integer, never a GC
  pointer (not traced, not rooted, `===` compares ids). It works in locals, parameters, object
  fields, arrays, `T | null` and closure cells. In a `T | null` slot, **0 is null, so no live
  handle may ever be 0** (the first version of the demo API handed out id 0 for its first entity and
  `if (target === null)` caught it; generations now start at 1). This is a rule for every engine
  handle type.
- An engine function is assumed not to call back into scripts and so not to allocate in the script
  heap: `CallExtern` is not an allocation point in the frame plan. The first engine function that runs
  a script hook changes that (closures to Rust, section 6).
- The end-to-end proof is `tools/iron_demo_api` (a small world of entities written with
  `#[iron_api]`) linked into `iron_ts_tests`, whose `build.rs` compiles `bench/engine.ts` with that
  crate's manifest. Every test runs normally, in stress GC mode and with collection off.

Not built yet: value classes across the boundary, strings, methods on handles (`transform.position`),
the engine context the real shims will reach the world through, and annotating `iron_scene` and the
other engine crates.

## 16. The engine context and value classes: built (2026-10-18)

**Decision: a thread-local current world.** Taken as the recommended default when the question in
section 12's follow-up went unanswered and the work was told to continue. A shim has no world argument
(scripts have none), so the host makes the world current on the scripting thread and API functions
reach it through `iron_api::context`: `enter(&SLOT, &mut world)` returns a scope guard that restores
the previous world when dropped, and `with(&SLOT, |w| ..)` runs a closure on it or answers `None` when
there is none (so a function called outside a frame returns its default instead of crashing). While
the closure runs the world is *not* current, so an API function called from inside it gets `None`
instead of a second `&mut` to the same world (`M9-API-07`..`09`). The alternative (an explicit
context pointer as every shim's first argument) costs an argument on every call and a parameter in
every signature; it is still possible later because the shims are generated.

**Value classes** (`M9-API-10`, `11`; `M9-TS-147`..`150`): `values(Vector3 = Vec3 [x: f32, y: f32,
z: f32])` declares a Rust type that implements `ValueType<N>` as a class scripts can construct and
read.

- Scripts: `new Vector3(1, 2, 3)` is an object with those fields (`v.x`), a struct in the script heap
  like any other; the generated `.d.ts` declares the class.
- Across the C ABI a value parameter is passed as one scalar per field (`v_0, v_1, v_2`); a value result
  is written through a trailing pointer to room for the fields. Scalars rather than a struct avoid the
  Win64/SysV disagreement about small structs, as the design said.
- The compiler flattens the parameter into `Field` reads of a temporary and gives the extern a
  trailing out-pointer; `CallExternInto` calls it with a stack slot, then widens the scalars into the
  fields of a freshly allocated object (allocated before the call, held in a rooted local). Every
  argument goes into a temporary first when a value is among them, so evaluation order is kept.
- All fields of one value class have the same scalar type. QuickJS bindings do not support value
  classes yet (the macro refuses `quickjs` together with `values(..)`).
- This is "phase 1" of the design: each vector is a heap object; scalar replacement later removes the
  allocation without changing the contract.

Not built yet: methods on value classes as inline intrinsics (`a.add(b)`), strings, methods on
handles (`transform.position`), annotating the real engine crates, and QuickJS support for values.
