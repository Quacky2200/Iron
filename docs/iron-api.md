# iron_api: the script API declared once in Rust

Crates: [iron_api](../crates/iron_api/src/lib.rs) (manifest types and generators) and
[iron_api_macros](../crates/iron_api_macros/src/lib.rs) (the `#[iron_api]` attribute). Tests:
[api.rs](../crates/iron_api/tests/api.rs) (`M9-API-01`..`06`),
[context.rs](../crates/iron_api/tests/context.rs) (`M9-API-07`..`09`) and
[values.rs](../crates/iron_api/tests/values.rs) (`M9-API-10`, `11`) and
[classes.rs](../crates/iron_api/tests/classes.rs) (`M9-API-16`..`18`). Design and the road ahead:
[iron-ts-engine-boundary.md](research/iron-ts-engine-boundary.md).

One module of ordinary Rust is the single source for a namespace of the script API: the manifest, the
TypeScript declarations, the `extern "C"` shims that compiled scripts link to, and typed QuickJS
bindings. They are all generated from the same functions, so they cannot drift apart.

## Usage

```rust
use iron_api::{Handle, iron_api};

#[derive(Clone, Copy)]
pub struct ObjectId(pub u64);
impl Handle for ObjectId {
    fn to_raw(self) -> u64 { self.0 }
    fn from_raw(raw: u64) -> Self { ObjectId(raw) }
}

#[iron_api(namespace = "Demo", quickjs, handles(GameObject = ObjectId))]
pub mod demo {
    use super::ObjectId;

    /// Adds two numbers.
    pub fn add(a: f64, b: f64) -> f64 { a + b }

    /// The parent of an object.
    pub fn parent_of(object: ObjectId) -> ObjectId { ObjectId(object.0 + 1) }
}
```

Arguments: `namespace = ".."` (required), `quickjs` (also generate `register_quickjs`) and
`handles(TsName = RustType, ..)` (Rust types that implement `iron_api::Handle`) and
`values(TsName = RustType [field: scalar, ..], ..)` (value classes, below). The module must be
inline (`pub mod x { .. }`).

### Value classes

A small Rust type that scripts see as a class of numbers (`glam::Vec3` as `Vector3`). The type
implements `iron_api::ValueType<N>` (`N` fields; the parts travel as `f64`):

```rust
impl ValueType<3> for Vec3 {
    fn from_parts(p: [f64; 3]) -> Self { Vec3 { x: p[0] as f32, y: p[1] as f32, z: p[2] as f32 } }
    fn to_parts(self) -> [f64; 3] { [self.x as f64, self.y as f64, self.z as f64] }
}

#[iron_api(namespace = "Demo", values(Vector3 = Vec3 [x: f32, y: f32, z: f32]))]
pub mod demo {
    pub fn cross(a: Vec3, b: Vec3) -> Vec3 { /* .. */ }
}
```

The `glam` feature (`iron_api = { features = ["glam"] }`) implements `ValueType` for `Vec2`, `Vec3`,
`Vec4` and `Quat`, so `iron_scene` declares `Vector3 = Vec3 [x: f32, y: f32, z: f32]` and
`Quaternion = Quat [..]` without writing the impls.

Each field is `f32`, `f64`, `i32` or `u32`, and all fields of one class have the same type. Functions
may take and return the Rust type; scripts write `Demo.cross(new Vector3(1, 0, 0), v)`. `to_dts`
declares `declare class Vector3 { constructor(x: number, ..); x: number; .. }` and `to_json` lists
the class under `values` (fields with their types), with `value:Vector3` as the parameter and result
type.

**Methods and getters.** A free function marked `#[method(Vector3)]` or `#[getter(Vector3)]` becomes a
member of the value class. `Vector3` must be declared in `values(..)` and the function's first
parameter must be that value (else a compile error); a getter takes only the value. Free functions
are used because the value type is a foreign type (glam's `Vec3`), so it cannot get an inherent
`impl`.

```rust
#[method(Vector3)]
pub fn scale(a: Vec3, s: f32) -> Vec3 { a * s }    // a.scale(2)
#[getter(Vector3)]
pub fn magnitude(a: Vec3) -> f32 { a.length() }    // v.magnitude
```

- **Manifest and `.d.ts`.** The functions go to `Namespace.classes` under the value's name, with the
  receiver first as `this` (type `value:Vector3`). `to_dts` declares them inside `declare class
  Vector3` (not as a second class, and no handle alias): `scale(s: number): Vector3; get magnitude():
  number;`. Without the attribute the function stays a namespace function.
- **Shims.** `iron_Vector3_scale(this_0, this_1, this_2, s, out)`: the receiver travels as the value's
  fields like any value parameter, and a value result goes through the trailing out pointer.
- **Not built.** Static functions on a value class (`Vector3.zero`), operators (`a + b`) and compound
  assignment on value properties.

### Handle classes

The methods, properties and statics of a handle type are ordinary `impl` blocks of its Rust type
inside the module. The type must be declared in `handles(..)`; the class gets the handle's TypeScript
name.

```rust
#[iron_api(namespace = "Scene", handles(GameObject = ObjectId, Transform = TransformRef))]
pub mod scene_api {
    impl ObjectId {
        pub fn create() -> ObjectId { /* .. */ }          // static: GameObject.create()
        pub fn destroy(self) { /* .. */ }                 // method: go.destroy()
        #[property]
        pub fn enabled(self) -> bool { /* .. */ }         // getter: go.enabled
        pub fn set_enabled(self, enabled: bool) { /* .. */ } // setter: go.enabled = false
        #[property]
        pub fn transform(self) -> TransformRef { TransformRef(self) }
    }
    impl TransformRef { /* .. */ }

    pub fn count() -> u32 { /* .. */ }                    // namespace function: Scene.count()
}
```

| Rust | Script |
|---|---|
| `#[property] pub fn x(self) -> T` | getter `obj.x` |
| `pub fn set_x(self, v: T)` next to a getter `x` of the same type | setter `obj.x = v` (a getter alone is read-only) |
| other `pub fn f(self, ..)` | method `obj.f(..)` |
| `pub fn f(..)` without `self` | static `Class.f(..)` |
| plain `pub fn` outside an `impl` | namespace function `Scene.count()` |

`self` is taken by value (handles are `Copy`). Two classes can sit on the same id, each with its own
Rust newtype: `Transform` is `TransformRef(ObjectId)` and shares the object's raw handle, so
`go.transform` returns the same number. A class replaces the plain handle alias in the declarations.

- **Manifest.** `Function.kind` is a `FnKind` (`Static`, `Method`, `Get`, `Set`); `Namespace.classes`
  holds a `ClassDef { name, functions }` per class. The JSON gains `"kind"` per function and
  `"classes":[{"name":..,"functions":[..]}]`. A class function lists its receiver first, as a
  parameter named `this` (`handle:Transform`). Parameter names are camelCase in the manifest and the
  declarations (`keep_world` is `keepWorld`).
- **`.d.ts`.** `declare class Transform { private constructor(); static f(): T; method(): T; get x():
  T; set x(value: T); }`. A handle without a class still emits `declare type X = number & { .. }`.
- **Shims.** `iron_<Class>_<jsName>` with `this: u64` first: `iron_Transform_setPosition(this, x, y, z)`
  (a value result still goes through a trailing out pointer). A setter is `set` plus the capitalised
  property name. Namespace functions keep `iron_<Namespace>_<jsName>`.
- **QuickJS.** Bindings are generated only for namespace functions; classes are not available there
  (a handle is a plain number).

## What the macro adds

The module's code is kept as it is. Inside the module the macro adds:

| Item | What |
|---|---|
| `MANIFEST` | A `const Namespace`: name, handle names, value classes, handle classes and, per function, `kind`, `name`, `rust_name`, `symbol`, `doc`, `params`, `ret` |
| `extern "C"` shims | One `#[no_mangle] pub extern "C" fn` per `pub fn` (including those in `impl` blocks of declared handles), named `iron_<Namespace>_<jsName>` (`iron_Demo_add`) or `iron_<Class>_<jsName>` for a class. Private functions are not exported |
| `register_quickjs(ctx)` | With `quickjs`: installs the global `Demo` object whose `add` is a native closure with typed arguments (no `Value` array, no op queue) |

Doc comments on the functions become the JSDoc of the declaration. Functions cannot be `async` or
generic; `self` is allowed only in a handle class's `impl`, by value.

## Types

| Rust | TypeScript | C ABI | QuickJS |
|---|---|---|---|
| `f64` | `number` | `f64` | `f64` |
| `f32` | `number` | `f32` (narrowed when it crosses) | `f32` |
| `i32` / `u32` | `number` | `i32` / `u32` | `i32` / `u32` |
| `bool` | `boolean` | `bool` | `bool` |
| `()` | `void` | `()` | `()` |
| declared handle | `number & { readonly __handle: "Name" }` | `u64` (`Handle::to_raw`) | `f64` number, cast to `u64` for `from_raw` |
| declared value class | a class with the declared fields (`new Vector3(1, 2, 3)`) | one scalar per field as parameters (`v_0, v_1, v_2`); a result through a trailing out pointer to room for the fields | not available: `quickjs` together with `values(..)` is a compile error |

| `&str` / `String` | `string` | a parameter: `*const u8` to the script string object; a result: `*const u8` plus a trailing `out_len: *mut i64` | not available: a string in a `quickjs` namespace is a compile error |

Any other type is a compile error that lists the supported ones. Names follow `snake_case` to
`camelCase` (`set_time_scale` is `setTimeScale`), done by `camel` in both crates.

### Strings

A function may take `&str` (or `String`) parameters and return `String`. The manifest, JSON and d.ts
type is `string` (`iron_api::Ty::Str`); `string | null` parameters are not supported.

- **In.** A string parameter is a `*const u8` pointing at the script string object: the header
  `iron_api::ScriptStr` (layout equal to `iron_ts_rt::Str`, a test in `iron_ts_tests` checks it) and
  the bytes inline after its 48 bytes. The shim reads it with `iron_api::script_str`, which
  borrows the bytes as `&str` with no copy when the string is well formed. Script strings are WTF-8,
  so one with a lone surrogate is copied with each lone surrogate replaced by one U+FFFD. A null
  pointer reads as `""`. The collector is non-moving and the caller roots the string, so it stays
  valid for the call. The engine must copy what it keeps (`set_name` stores an owned `String`).
- **Out.** A `String` result adds a trailing `out_len: *mut i64` to the shim, which returns
  `*const u8` into a per-thread buffer (`iron_api::return_string`). The compiled call copies it at
  once into a new script string (`ts_rt_str_new_gc`), before any other engine call.
- **Unsafe shims.** A shim with a string parameter is `unsafe extern "C"`, since it dereferences the
  pointer.

```rust
pub fn find(name: &str) -> ObjectId { /* name is borrowed: copy it to keep it */ }
pub fn name(self) -> String { /* the shim returns it through return_string */ }
```

Tests: `M9-API-21`..`22` (crates/iron_api/tests/strings.rs).

## Generating the files

```rust
let dts = demo::MANIFEST.to_dts(); // declare namespace Demo { function add(a: number, b: number): number; }
let json = demo::MANIFEST.to_json(); // {"namespace":"Demo","functions":[{"name":"add","symbol":"iron_Demo_add",..}]}
```

`to_dts` writes a `declare type` per handle without a class, a `declare class` per handle class and
value class, plus a `declare namespace` with the JSDoc. `to_json` is the table a compiler reads to call
each symbol directly: the `handles` list (handle type names), the `values` list, the `classes` list
and per function the name, symbol, kind, parameter names and types (`f64`, `bool`, `handle:GameObject`, ...)
and the result type.

**Handle rule.** Compiled scripts store a handle as a raw `u64` and use 0 as null in `T | null`, so
the id 0 means "no object": a handle type must never produce a live id 0 (start generations or
indices at 1).

## The engine context

A shim has no world argument (scripts have none), so the host makes the world current on the thread
that runs scripts and API functions reach it through `iron_api::context`. `enter` returns a scope
guard that restores the previous world on drop; `with` runs a closure on the current world, or
returns `None` when there is none. While the closure runs the world is not current, so a nested
call gets `None` instead of a second `&mut` to it.

```rust
thread_local! { static WORLD: Cell<*mut World> = const { Cell::new(std::ptr::null_mut()) }; }
let _scope = iron_api::context::enter(&WORLD, &mut world); // while scripts run
// inside an API function:
let pos = iron_api::context::with(&WORLD, |w| w.position(id)).unwrap_or_default();
// a nested `with(&WORLD, ..)` inside that closure returns None
```

## Status

Additive. `__iron_op` and the ops in [scripting.md](scripting.md) are untouched. `iron_scene` is the
first engine crate using `#[iron_api]` (`Scene`, in `iron_scene::api`; see
[scene-and-objects.md](scene-and-objects.md#the-typed-script-api-iron_sceneapi)), but nothing in the
app calls it yet. The `iron_ts` compiler reads the JSON manifest and calls the shims directly
(`--manifest`, see [iron-ts.md](iron-ts.md); the test namespace is `tools/iron_demo_api`). Value
classes (`Vector3`) with methods and getters (`a.add(b)`), handle classes (`GameObject`, `Transform`)
and the engine context exist, and so do strings across the boundary (`GameObject.name` and `find`);
static functions and operators on value classes, classes, values and strings in the QuickJS bindings and annotating the other engine crates (physics, audio) are not built.

Measured with `cargo test -p iron_api --release -- --ignored --nocapture` (1,000,000 calls): about
150 ns per call through the typed function and about 245 ns through a simplified `__iron_op` (a name,
a rest array of numbers and a queue push; the real op also converts each argument to a `Value`, so
the real gap is larger). Most of the remaining 150 ns is QuickJS's own call overhead. The large win
is for compiled code, which calls the shim directly.

## QuickJS bindings for values and strings

`#[iron_api(namespace = "X", quickjs, ...)]` now also generates QuickJS functions for namespace functions that take or return
a string or a value class (they used to be a macro error). A `&str` or `String` is a JS string; a value class (`Vector3`) is
a plain object with the fields' names (`{ x, y, z }`): an argument is read field by field (a missing field is a JS exception
the script can catch) and a result is a new object. Every generated function returns `rquickjs::Result`. Handle classes
(`GameObject.name`, `transform.position`) and value-class methods are still not in the QuickJS bindings; scripts there call the
namespace functions (`Scene.setLocalPosition(go, v)`). `iron_scene`'s `Scene` namespace has `quickjs` on, so the same API
runs from QuickJS and from compiled code. Tests `M9-API-23..26`.

## Namespace properties and value classes in QuickJS

- **Namespace properties.** In a namespace (no `impl`), `#[property] pub fn color() -> Color` is a getter and `pub fn set_color(c: Color)`
  next to it the setter: scripts write `Gizmos.color = c` and `Time.deltaTime`. The manifest has them as functions of kind `get` and
  `set` with no receiver, the `.d.ts` as `let color: Color;` (`const` when read only), the shims as `iron_Gizmos_color()` /
  `iron_Gizmos_setColor(..)`, QuickJS as accessors on the namespace object, and `iron_ts` calls the getter on a read and the
  setter on an assignment (`Place::NsProp`, also for `+=`). A value class's statics are the properties and functions of a namespace
  with the same name (`Color.white`, `Color.hex("#ff0000")`).
- **Value classes in QuickJS.** Each value type is a real JS class (made once, `globalThis.Vector3 ??= class ...`): the constructor takes
  the fields, methods (`#[method(Vector3)]`) and getters (`#[getter(Vector3)]`) are on the prototype and call hidden Rust functions
  (`__iron_vm_<Value>_<name>`), results are instances. Modules that mention the same value type share the class.

## The engine API beyond the scene

[iron_engine_api](../crates/iron_engine_api/src/lib.rs) declares, with `#[iron_api]`, what the sine-circle demo and most gameplay
code use besides `Scene`: `Color`, `Time`, `input`, `Gizmos`, `Camera`, `Screen` and `QualitySettings`, with the names and behaviour of
`js/engine`. Each keeps its state per thread and has plain Rust functions for the host: `time::advance(real_dt)`,
`input::{key_down_event, key_up_event, mouse_button_event, mouse_move, end_frame}`, `gizmos::take_frame()`, `camera::cameras()`,
`screen::{state, quality, set_size}`. `iron_app` is not wired to them yet (another session's area). Tests `M9-API-27..36`.

`iron_engine_api` also has `audio` (`load(path, stream)`, `Sound.play(loop)`, `Playback.pause/resume/stop`, recorded as
`AudioCommand`s for the host). A Rust parameter written `r#loop` is the script's `loop`. Tests `M9-TS-593..597`.

## Optional parameters and renamed members

- `#[optional(a, b)]` on a function marks trailing parameters a call may leave out. The manifest says
  `optional`, the declaration writes `seed?: number`, and the function sees a default: **NaN** for a
  number (so `seed.is_nan()` means "left out"), an empty string, `false` or a zero handle. A compiled
  call fills them in; QuickJS treats a missing or `undefined` argument the same way.
- `new Rng(seed)`: a handle class whose static `create` has parameters is constructed with them
  (`new Class(args)` calls `create(args)`).
- `#[js_name = "graphicsDeviceVendorID"]` gives the script name when `snake_case` to `camelCase` does not
  spell it. A Rust keyword is written raw (`r#in` for `TimeUntil.in(2)`).

## The first batch moved out of `js/engine` (2026-10-19)

`iron_engine_api` now also declares `Rng` / `Random`, `LayerMask`, `Debug` / `Log` / `Console`,
`PlayerPrefs`, `Assets`, `Application` / `SystemInfo`, `TimeSince` / `TimeUntil`. Each keeps its state per
thread and has host functions: `layers::set_game_layers`, `log::set_sink`, `prefs::{load, take_changes}`,
`assets::{set_assets, changed}`, `app::{set_app, set_system, set_focus, take_requests}`. Differences from
the TypeScript, all because a call has fixed typed parameters:

- logging functions take one string (compiled code writes a template literal); objects are not printed as JSON;
- `getMask` takes up to four names; a left-out lookup (`guidToPath`) returns `""` instead of `undefined`;
- `Assets` has `count()` / `pathAt(i)` / `guidAt(i)` / `importer(path)` as JSON text / `takeChanges()` (a
  newline-separated string to poll) instead of `all`, an object and `onChanged(fn)`;
- `SystemInfo.screenWidth` / `screenHeight` / `screenRefreshRate` / `screenDpi` replace `SystemInfo.screen`;
- `Application.onFocusChanged(fn)`, `Task` and the vector-valued `Random` members wait for closures and the
  Rust math types.

### Input, TimeSpan and DateTime

- `input` follows `js/engine/src/input.ts`: events (`key_down`, `key_up_event`, `mouse_move`, `mouse_delta`, `wheel`,
  `gamepad_*`, `blur`) arrive as they happen and become "this frame" at `input::tick()`, which `time::advance` calls (the engine
  tick). Mouse buttons are keys (`MouseLeft`...). Gamepads are read by id (`gamepadCount`, `gamepadId(i)`, `gamepadName(id)`,
  `gamepadButton(id, name)`, `gamepadAxis(id, name)`); `input.on(..)` listeners wait for closures.
- `TimeSpan` (field `totalMilliseconds`) and `DateTime` (fields `epochMilliseconds`, `utc`; local time through the OS zone or
  `datetime::set_zone`) are value classes. A static `create` makes `new TimeSpan(ms)` and `new DateTime(y, m, ...)` round and
  carry as TypeScript does (compiled code and QuickJS both call it; the engine builds its own results with the fields).
  Differences: `parse` logs an error and returns NaN instead of throwing, `DateTime.parse` knows ISO 8601 only (no whitespace),
  `subtract(TimeSpan)` is `subtractTimeSpan`, `DateTime.fromDate(Date)` and `Task` are not there.
- A function with more than four parameters reads its QuickJS arguments from the rest list (`new DateTime(..)` has eight).

### The math types (`iron_math`)

`crates/iron_math` declares `Mathf`, `Vector2/3/4`, `Vector2Int/3Int`, `Quaternion`, `Matrix4x4`, `Rect`, `RectInt`,
`Ray`, `Plane`, `Bounds` and `Pose` as `js/engine/src/math.ts` has them (the scene, input, gizmos and colour APIs use its
types, so they live below `iron_scene`). Values are **doubles** (the TypeScript's), computed in the same order, so
results are bit for bit the same as bun's for everything but `Math.hypot` of three or more numbers, where engines differ
in the last bit: the native version scales by the largest and sums the squares (two numbers use the C library's `hypot`,
as bun does), and the tests that compare with bun run the original with the same algorithm. The scene's `Vector3` /
`Quaternion` are now these f64 types too (positions are still stored as f32 in the scene), and `Color` is f64.

- `new Vector3(x, y, z)` calls the static `create` (a missing component is 0); the classes in QuickJS are made by it too.
- A name with several versions (`Quaternion.euler(x, y, z)` and `euler(vector)`) is chosen by argument count in QuickJS and
  by argument types in compiled code. A function of more than four parameters reads its QuickJS arguments from the rest list.
- A value class argument may be another value class with the same fields (a `Vector2Int` where a `Vector2` goes).
- Differences: the `...Like` parameters are the classes; `toArray()`, `from(v)`, `new Matrix4x4(array)` and `.m` are not
  there (`Matrix4x4.fromColumns` and `get(row, col)` are); `Mathf.smoothDamp` returns a `Vector2` (value, velocity);
  `Plane.raycast` / `intersect` answer NaN where the TypeScript returned `null`; `RectInt.positions()` is not there;
  `Ray`, `Plane`, `Bounds` and `Pose` keep flat numbers and give their vectors from getters; `Random.insideUnitSphere` and
  friends can now be added (the vectors exist).

### Defaults for left-out parameters

`#[optional(keep_world = true, fade = 0.3)]` gives a left-out parameter a value instead of NaN / false / empty: a compiled call and
QuickJS both pass it (`GameObject.setParent(parent, keepWorld = true)` is declared this way). The manifest says `"default": 1`
(a flag is 0 or 1).

## Scenes (`iron_scenes`, `iron_kinds`)

`iron_scenes` is `scenes.ts` in Rust: `SceneManager` (`loadScene(text, mode?)`, `getActiveScene`, `setActiveScene`,
`createScene`, `unloadScene`, `moveGameObjectToScene`, `mergeScenes`, `sceneOf`, `dontDestroyOnLoad`, `instantiate`), `Scene`
(`name`, `path`, `isLoaded`, `rootCount` / `rootAt(i)`, `serialize()`) and `Prefab` (`fromObject`, `fromData`, `instantiate`,
`toJSON`). The file format is `scenes.ts`'s `iron.scene` version 1, read and written with `serde_json`.

The loader does not know component classes. `iron_kinds` is a per-thread registry of *kinds* (`create`, `on`, `save`, `load`,
`awake`, `object`, `destroy` over raw `u64` handles). `iron_scenes` registers `Camera`, `Animation` and `Animator`; a compiled
program's `NativeHost` registers one kind per class it has (properties go through the `PROPS` table: numbers, booleans, strings,
component and `GameObject` references, and the value classes `Vector2/3/4`, `Quaternion`, `Color`, `Rect`, ... written in
place). A component whose kind is not registered is skipped with a warning (`take_warnings()`). References are placeholders for
kinds (`{"$ref":"GameObject","object":raw}`, `{"$ref":"Component","kind","handle"}`) that the loader and saver map to file ids.
`instantiate` is save plus load with external references kept live. The compiler lowers `GameObject.instantiate(go, { position,
rotation, parent })` and `prefab.instantiate({..})` onto the numeric `SceneManager.instantiate` / `Prefab.instantiate`.

Differences: the components of one object are saved kind by kind, not in the order they were added; `loadSceneAsync` is a host
request (`request_load` / `take_load_requests` / `finish_load`); scene events are read with `take_events()` until closure APIs
exist; compiled programs do not keep state across a hot reload (they restart); `$asset`, `Material` and `Texture` values are not
loaded yet.
