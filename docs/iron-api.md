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
- **QuickJS.** A class is a JS class whose instances are opaque handle objects; see "Handles in QuickJS".

**Static properties.** Inside an `impl`, `#[property] pub fn main() -> T` (no `self`, no parameters) is a static getter and
`pub fn set_main(v: T)` next to it (same type) the static setter: `Window.main` reads like a property in both backends. The
manifest has a `get` / `set` function of the class without a `this` parameter (the compiler keeps a static getter with the
class statics, so `ext_static_prop` reads it, and the setter in `static_setters`). The `.d.ts` says `static readonly main: T;`
(`static get` / `static set` for a writable one), and `iron-build types` the same. In QuickJS the property is an accessor on the
class object (configurable, not enumerable), so strict-mode assignment to a read-only one throws `TypeError`. A static function
and a static property of one class cannot share a script name (compile error). A static and an instance member may (`Window.focused`
and `win.focused`): the instance one lives on the prototype. Tests: `crates/iron_api/tests/static_props.rs` (`M9-API-1490`, `1491`).

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
| declared handle | `number & { readonly __handle: "Name" }` (a class: `Name`) | `u64` (`Handle::to_raw`) | an opaque `Name` object, `null` for 0 |
| declared value class | a class with the declared fields (`new Vector3(1, 2, 3)`) | one scalar per field as parameters (`v_0, v_1, v_2`); a result through a trailing out pointer to room for the fields | a `Vector3` instance (an object with the fields; arrays are accepted as arguments) |

| `&str` / `String` | `string` | a parameter: `*const u8` to the script string object; a result: `*const u8` plus a trailing `out_len: *mut i64` | a JS string |
| `&[f64]` / `&[V]` | `number[]` / `Vector3[]` | `*const u8` to the script array | a JS array (elements: instances, objects or arrays) |
| `&[String]` / `Vec<String>` | `string[]` | `*const u8` to the script array of strings; the shim copies them (`iron_api::script_strs`), so the function may keep them | a JS array of strings |

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
static functions and operators on value classes are not built.

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
(`GameObject.name`, `transform.position`) are now in the QuickJS bindings, see "Handles in QuickJS"; the namespace functions
(`Scene.setLocalPosition(go, v)`) still work with handle objects. `iron_scene`'s `Scene` namespace has `quickjs` on, so the same API
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
- `#[options]` (with `#[optional(..)]`) lets QuickJS pass the optional parameters as one plain object, by camelCase name:
  `Window.create({ title: "Palette", style: "tool", minWidth: 200 })`. It applies when the call has exactly one argument past
  the required ones and that argument is a plain object (`Object.prototype`, not an array, function, handle or value-class
  instance: `iron_api::qjs::options_object`); a key that is missing, `undefined` or `null` is left out and takes the declared
  default. Positional calls keep working. The manifest and the `.d.ts` do not change: compiled code already maps a trailing
  object literal onto the optional parameters by name (`call_ext_args`). A function with `#[options]` cannot have optional
  value-class parameters (a value is itself a plain object). Tests `M9-API-1430..1434`.
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
  newline-separated string to poll) instead of `all`, an object and `onChanged(fn)`. It also has `exists(path)`,
  `list(glob)` (a `string[]` of the sorted paths matching a glob, `*` / `?` inside one folder name, `**` across folders,
  case-sensitive; a `Vec<String>` result of the macro, table below) and
  `packOf(path)` (`assets`, `scene-level1`, `""` for a loose file). They answer from the index the host fills with
  `iron_engine_api::assets::set_asset_index` (every path of the mounted packs or game folder, `asset_db::index`), so they see assets
  without a `.meta`;
- `SystemInfo.screenWidth` / `screenHeight` / `screenRefreshRate` / `screenDpi` replace `SystemInfo.screen`;
- `Application.onFocusChanged(fn)`, `Task` and the vector-valued `Random` members wait for closures and the
  Rust math types.

### Input, TimeSpan and DateTime

- **Input is per event domain** (window family): `input` reads the keys of the family whose code is running; main-game components always read the main window's, an isolated window's keys never reach it, focus loss clears a domain's keys, and QuickJS promise jobs read the main domain's. QuickJS and compiled code agree. See `docs/windows.md` ("Input belongs to the window's event domain").
- `input` follows `js/engine/src/input.ts`: events (`key_down`, `key_up_event`, `mouse_move`, `mouse_delta`, `wheel`,
  `gamepad_*`, `blur`) arrive as they happen and become "this frame" at `input::tick()`, which `time::advance` calls (the engine
  tick). Mouse buttons are keys (`MouseLeft`...). Gamepads are read by id (`gamepadCount`, `gamepadId(i)`, `gamepadName(id)`,
  `gamepadButton(id, name)`, `gamepadAxis(id, name)`); `input.gamepads()` gives `{ id, name, connected, buttons, axes }` (the standard
  buttons and axes by name, `0` when not reported; written in TypeScript over the readers above by the facade) and
  `input.on("gamepadconnected" | "gamepaddisconnected", fn)` calls `fn({ gamepad })` and returns the unsubscribe function.
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

## Physics components (`iron_physics_api`)

`crates/iron_physics_api` declares physics for compiled scripts: the low-level `Physics` namespace (`createBody`,
world colliders, `raycast` that answers at once, `createJoint`, `step`) and the components of `physics-components.ts` and
`physics2d.ts` as handle classes over the GameObject (`go.addComponent(Rigidbody, { mass: 2 })`,
`go.getComponent(BoxCollider)` through the static `findOn`). Layout: `comp.rs` (component data, the one body per
GameObject shared by its Rigidbody and colliders, joints, rope, per-step sync), `components.rs`, `joints.rs`,
`physics2d.rs` (the generated-looking class impls: same members repeated per class), `kinds.rs` (`register_kinds()`
puts the 18 classes into `iron_kinds` for scene files and `instantiate`).

- **Every property change reaches the rapier world at once.** A collider is remade when its shape changes.
- **`step()`** (`Physics.step()`): builds pending joints and ropes (`start_pending`), moves bodies to where scripts put their
  objects (pose differs by more than 1e-4 m), remakes colliders whose object was scaled, steps the world, writes the moved
  bodies back into the scene and keeps the contacts. Objects that are gone from the scene lose their physics. Then it runs the
  closures `Physics.afterNextStep(f)` was given, once each (what `Light.measureIlluminance` waits for).
- **For the host:** `take_contacts()` (collider numbers, `speed`, `sensor`), `collider_info(collider)` (the object, the
  collider class name, its component handle, `two_d`, the Rigidbody component) to dispatch `onCollisionEnter` /
  `onTriggerEnter` (and the `2D` names), `object_destroyed(object)` for destroyed GameObjects, `register_kinds()`.
- Differences to the TypeScript are listed in `tools/iron_ts/TODO.md` ("Physics components"). Tests `M9-API-200..209`,
  `M9-TS-1100..1105`.
## Audio (`audio`, `AudioSource`, `AudioListener`)

`audio.rs` is `js/engine/src/audio.ts`: `audio.load(path, stream?)`, `audio.play(path, {..})`, `Sound.play({ volume, rate, pan, loop,
fadeIn, start, bus, emitter })`, `Playback` (`setVolume/Rate/Pan(v, fade?)`, `pause/resume/stop(fade?)`, `seek`, `playing`, `paused`,
`onEnded(f)`), `Bus` (`audio.createBus({ volume, parent })`), `Emitter` (`audio.createEmitter(position, { minDistance, maxDistance, bus })`),
`audio.setListener`, `audio.setMasterVolume`. Each change is an `AudioCommand`; the host takes them with `audio::take_commands()` and
runs `command.to_op()` on `iron_audio::Mixer::apply` (a `Load` carries the path and stream flag: the host reads the file and calls
`Mixer::load`). When `Mixer::poll` reports finished playbacks the host calls `audio::ended(&[PlaybackRef])` (runs `onEnded`).

`audio_components.rs` is `audio-components.ts`: `AudioSource` and `AudioListener` are engine components (static `attach` / `findOn`, so
`go.addComponent(AudioSource, { clip, spatialBlend: 1 })` and `getComponent` work), with scene-file kinds `AudioSource`
(`clip` as `{"$type":"Sound","src","guid"}` or a path, `playOnAwake`, `spatialBlend`, `minDistance`, `maxDistance`, `volume`, `pitch`,
`panStereo`, `mute`, `loop`) and `AudioListener`; registered by `iron_scenes` (`audio_components::register_kinds`). The host calls
`audio_components::pre_render()` once a frame after transforms are final (emitters follow their objects, the listener follows its object;
only real moves are sent). `objects::process_destroyed` removes the components of destroyed objects.

Differences from the TypeScript: `on: Bus | Emitter` is `bus` and `emitter`; `playback.onEnded = f` is `playback.onEnded(f)`; a clip given as a
path is `src.clipPath = "x"` and `src.playOneShotPath("x")` (calls are not chosen by argument type for handle classes); the static
`AudioListener.volume` is `AudioListener.getVolume()` / `setVolume(v)`; `outputBus` is none (0) for the main mix; `audio.masterVolume`
and `audio.soundCount` are read-only properties; `createEmitter` takes a `Vector3`.

## Rendering data (`iron_render_api`)

[iron_render_api](../crates/iron_render_api/src/lib.rs) is `rendering.ts`, `textures.ts` (and, below, `particles.ts` ...) as data: nothing
draws, the app reads per-thread registries and a command queue. **No change to `iron_render` or `iron_app` yet**: wiring is theirs.

- **Components** (engine components like `Camera`: static `attach` / `findOn`, `gameObject`, one property per field, and a scene-file
  kind each): `Light`, `PlaneRenderer`, `MeshRenderer`, `SkinnedMeshRenderer`, `Mirror`, `ReflectionProbe`, `LODGroup`, `WorldPanel`,
  `SpriteRenderer`, `LineRenderer2D`. The host reads them with `iron_render_api::lights()`, `mesh_renderers()`, ... (`Vec<(Ref, Data)>`)
  and removes the components of destroyed objects with `iron_render_api::destroyed(&objects)`. `Camera` gains `clearFlags`, `rect`,
  `cullingMask`, `layerCullDistances`, `layerCullSpherical`, `antialiasing`, `targetTexture`, `sceneGrid`.
- **Assets** (handles with asset kinds): `Texture` (`Texture.load(src, { filterMode, wrapMode, mipmaps, anisoLevel, linear })`,
  `Texture.create(w, h, colors)`, `Texture.createRenderTexture(w, h)`), `Mesh` (`Mesh.create({ vertices, triangles, normals?, uv? })`,
  built-in meshes by name), `Material` (`new Material({ color, metallic, ... })` or `Material.create({ .. })`, `copy()`, `withColor(c)`),
  `Sprite`. The host takes `take_commands()` (`Texture(t)` upload, `Mesh(m)`, ... removals), reads `texture_info(t)` / `mesh_info(m)`
  and reports file loads with `texture_loaded(t, w, h)`.
- **Generated code.** `build.rs` writes `generated.rs` (into `OUT_DIR`) from the field tables in `gen/components.rs` (type, default, doc, hooks); it runs with every build, so
  there is nothing to regenerate. Hand-written parts are `resources.rs`.
- **Scene files.** `iron_kinds::AssetKind` (shared kinds go into the file's `assets` table as `{"$asset":"texture1"}`, inline kinds
  such as `Material` / `Sprite` are `{"$type":"Material",..}`); components save assets as `{"$handle":"Texture","id":N}` placeholders
  and the saver / loader (`iron_scenes::data`) translate, one handle per table entry. `instantiate` and in-memory prefabs keep the
  handles. `mesh: "cube"` is saved as the string. The example `level1.scene.json` loads completely.
- **Compiler help.** A string where a handle is expected calls the class's `fromString` (`mesh: "sphere"` is `Mesh.fromString`); an
  optional array parameter left out is empty; `Class.create({ .. })` on a class whose `create` takes nothing is `new Class({ .. })`;
  an optional handle left out is a zero handle.
- **Effects.** `ParticleSystem` (main settings as properties, `startLifetime` etc. as a value plus `...Max` and
  `setStartLifetimeRange(min, max)`, `setStartColorRange`, `shapeType` / `shapeRadius` / `shapeAngle` / `shapeSize`, `addBurst`,
  `addSizeKey`, `addColorKey`, `setTextureSheet`, `play` / `stop(clear)` / `pause` / `clear` / `emit(n)`, `onStopped(f)`): the data is in
  `particle_systems()`; scripts' `play` / `emit` ... are queued for the host (`take_particle_controls()`), which reports back with
  `report_particles(system, count, alive)` (runs `onStopped`, and `stopAction: "destroy"` destroys the object). `LineRenderer`
  (`setPositions(points)`, `redraw()`; a tube through a `MeshRenderer` of its own that scene files leave out; the host calls
  `pre_render_lines()` before drawing). `Cloth` (`rebuild()` swaps the object's `MeshRenderer` mesh for an engine-filled copy that
  `mesh_info` marks `engine`; `destroy` puts the original back; the physics side reads `pins`, `pin_box` and the collider lists).
  `Primitives.create(kind, { name })` with `PrimitiveType.Cube` ... (`set_primitive_collider_hook` lets the physics side add the
  collider). Differences: `Cloth` rebuilds on a scene load or an explicit `rebuild()`, not on `onStart`; `LineRenderer.positions`
  and the other arrays are call-style (`setPositions`, `addPin`, `addSphereCollider`).
- **Models** (`models.rs`): `Model.load(path)` (once per file; queues `RenderCommand::Model`), `isLoaded` / `failed`, `materialCount` /
  `materialAt(i)`, `meshCount` / `meshAt(i)`, `instantiate({ position, rotation, scale, parent, colliders })`. The host reads the file,
  makes its meshes and textures with `engine_mesh()` / `engine_texture(w, h)` and reports the TypeScript's description JSON with
  `model_loaded(model, Some(json))` (`None`: it failed); copies asked for earlier are built then (a GameObject per node, a mesh renderer
  per primitive, or child objects when a node has several; skinned nodes get a `SkinnedMeshRenderer` with bones and bind poses; glTF
  materials). `colliders: "mesh" | "convex"` goes through `set_model_collider_hook`. Scene files carry a copy as the object's `"model"`
  (`iron_kinds::ObjectExtra`: the model's own nodes are left out and built again on load). Not built: glTF animations.
- **Screen, QualitySettings, Gizmos** (`iron_engine_api`): `Screen` has `scale`, `aspect`, `fullScreenMode`, `setResolution(w, h, { mode,
  refreshRate })`, `windowWidth` / `windowHeight`, `resolutionCount` + `resolutionWidth(i)` ..., `currentResolution...`,
  `supportedFullScreenModes` (a `FullScreenMode[]`, an enum list) and `onChange(f)` (token, `offChange`). The host feeds it with `screen::update(w, h,
  scale)` (runs the listeners) and `set_display(..)` and takes the window changes scripts asked for with `take_requests()`.
  `QualitySettings` has the named levels (`levelCount`, `levelName(i)`, `levelJson(i)`, `getQualityLevel()`, `setQualityLevel(i)` /
  `setQualityLevelByName(name)`, `onLevelChanged(f)`) and every setting with the TypeScript's validation (unknown names ignored,
  numbers clamped). The game's `iron.quality` manifest goes through `screen::configure_quality(&json)`, the player's saved choice
  (`iron.quality` in PlayerPrefs) through `apply_default_quality()`; the renderer reads `quality()` when `quality_version()` changed.
  `Gizmos` has `drawRay`, `drawWireCube`, `drawWireDisc`, `drawWireSphere`, `drawFrustum`, `matrix` and `lineCount`.
- **Settings and behaviours.** `RenderSettings` (namespace properties: `ambientLight`, `ambientMode`, `reflectionIntensity`,
  `skyboxEnabled` + `skyZenith` / `skyHorizon` / `skyGround` / `skySunDirection` / `skyHasSun` / `skySunColor` / `skySunSize`,
  `bloomEnabled` + `bloomThreshold` / `bloomIntensity` / `bloomScatter`, `tonemapping`, `postExposure`; the host reads
  `settings()` when `version()` changed). Scene files apply their `renderSettings` when the scene becomes active and the active scene
  saves the current ones; the keys are the ones `scenes.ts` saves (`ambientLight`, `ambientMode`, `reflectionIntensity`, `skybox`, `bloom`, and the fog's `fog`, `fogMode`, `fogColor`, `fogDensity`, `fogStartDistance`, `fogEndDistance`, `heightFog`, `volumetricFog`, `fogCulling`; the TypeScript tier saves no exposure keys either). `Cursor` (`visible`, `lockState`, `setCursor(path, { hotspotX, hotspotY })`, `clearCursor()`;
  host: `iron_engine_api::cursor::state()`) with `CursorLockMode.None/Locked/Confined` strings. `CameraFollow` and `CameraOrbit` are
  components whose update the host runs: `late_update_camera_follows(dt)` and `late_update_camera_orbits(dt)` once a frame.
- **Differences from the TypeScript.** `RenderSettings.skybox = { .. }` is the `sky...` properties; `Cursor.setCursor(texture)` takes a
  path only. `Material` is mutable (a shared handle, like Unity's `sharedMaterial`; `with(..)` is
  `copy()`); `Texture.load` mipmaps / linear given as their defaults let the importer decide; `RenderTexture` is a `Texture`
  (`createRenderTexture`, `isRenderTexture`); `whenLoaded()` is a facade promise (polled each frame; rejects when loading failed); `Sprite.create(texture, ppu?)`
  with `pivot` / `setRect(..)` setters and `Sprite.gridCell(texture, columns, rows, index)` instead of `grid`; a `SkinnedMeshRenderer` is not
  found by `getComponent(MeshRenderer)`; arrays of objects are `addBone(b)` / `addLod(..)` style calls (`bones`, `lods`, `points`);
  `Mirror.normal` has a `hasNormal` flag; `specularColor = null` is `clearSpecularColor()`; enum-like strings (`Light.type`) ignore
  unknown names instead of storing them.

## Closures given to the engine (`iron_api::callback`)

A parameter of type `Callback<(A, B)>` (elements `f64`, `i32`, `u32`, `bool`, `String` or `HandleArg<T>` for a declared handle) is a
script closure the engine keeps and calls later: `input.onKeyDown((code, repeat) => ..)`. Manifest type `fn(f64,string):void`
(`ExtTy::Fn(params, result)` in the compiler; the old `fn:void` form still reads), declaration `(a: number, b: string) => void`.
The closure returns nothing.

- **Rust side.** `cb.call((1.0, "x".into()))`, `Callback::from_fn(rust_closure)` (host code and tests), `Listeners<A>` (a table with
  `add` / `remove` / `snapshot`; tokens are unique across all events) and `emit(&snapshot, args)`. Emit after the state borrow is
  released, from a snapshot, so a listener may subscribe, unsubscribe or call the API.
- **Runtime.** The shim wraps the closure pointer with `Callback::from_script`, which roots it (a host root) until the last clone is
  dropped. The runtime tells `iron_api` how to root, make a script string and take a thrown exception through
  `callback::install_bridge`; `iron_ts_rt` installs it with its heap. A string argument is a new rooted script string for the call.
  An exception thrown by a callback is swallowed and listed by `callback::take_errors()`.
- **QuickJS.** A function with a `Callback` parameter is not generated for QuickJS yet.
- **Differences.** The TypeScript `on(..)` returned an unsubscribe function; here `onX(f)` returns a token (a number, never 0) and a
  matching `off(token)` removes it: `input.off`, `Assets.offChanged`, `Application.offFocusChanged`, `SceneManager.offSceneEvent`.
  `input.onKeyDown` gets `(code, repeat)` (no layout `key`); gamepad events get the id as a number. `Assets.onChanged` gets the paths as
  one string, one per line. Scene events get the `Scene` handle and the mode as `"single"` / `"additive"`. Listeners are called at once
  (`SceneManager` after the load or unload finished); `take_events()` still works.
## Handles in QuickJS

The decision (user, 2026-10-20, `research/iron-native-physics.md` section 7): whatever the compiled side treats as internal (engine
ids, handles) reaches QuickJS as an **opaque internal value**, like a V8 internal field. Plain data (numbers, strings, arrays,
value classes) is encoded normally.

- **The object.** A handle is an instance of a Rust class (`iron_api::qjs::HandleBox { raw, class }`) given the prototype of the
  handle's JS class (`GameObject.prototype`). The id is in the engine-side slot, not a property: it has no own properties,
  `JSON.stringify(go)` is `{}`, `Object.prototype.toString.call(go)` is `[object GameObject]` (`Symbol.toStringTag`), and only
  properties the script sets are own. Id 0 is `null` going out; `null` / `undefined` are 0 coming in.
- **The class.** `register_quickjs` makes, for each name in `handles(..)`, a global JS class once (`Scene.GameObject` is the same
  object). Modules that declare the same handle share it and each adds its members. Statics (`GameObject.create()`) are properties
  of the class (overloads by argument count work for them); methods and `#[property]` accessors (with the setter) are
  non-enumerable properties of the prototype; `new Class(args)` calls the static `create`; a class without `create` throws.
  A namespace named like its class (`AnimationCurve`) is the class itself. If a namespace object and a class have the same name
  (`Scene` helper functions and the scene class), they are merged in either order and the class wins (`install_namespace`).
- **Checks.** A handle argument must be an instance of exactly that class (`Transform` where `GameObject` goes is a
  `TypeError: expected a GameObject, got a Transform`); a method with a non-handle `this` throws.
- **Host side.** `iron_api::qjs::handle_new(ctx, "GameObject", raw)` makes the object and `handle_arg` reads the id back (for the
  app's own bindings, tests).
- **No identity.** Every read makes a new object, so `a === b` is false for the same id (listed in `tools/iron_ts/TODO.md`).
- **Value classes.** Each QuickJS value class has `toJSON()` (the compact array of numbers, as compiled code stringifies) and a static
  `X.fromJSON(v)` taking that array (exactly the fields' count) or the object form (left-out fields 0, `a` 1). A function
  parameter of a value type accepts an instance, the object form (all fields) or the array.
- **Arrays.** `&[String]` (or `Vec<String>`, owned) reads a JS array of strings (manifest type `strings`, `ExtTy::StrArr` in the compiled lowering; an optional one left out is empty; `M9-API-1621`, `M9-TS-3323`); `&[f64]` reads a JS array of numbers; `&[V]` an array of value instances, objects or arrays. Typed arrays and array
  results are not built.
- Code: `crates/iron_api/src/qjs.rs`, the macro's `handle_class` branch. Tests `M9-API-100..113`.

### Installing the APIs: one call

`iron_native::install(&mut quickjs_runtime)` (crate `crates/iron_native`) installs the whole native API: it runs
`iron_native::register_quickjs(ctx)` in the runtime's context and again in every context a `reset()` (hot reload) makes
(`QuickJsRuntime::add_native`). The order inside is fixed: `iron_engine_api::register_quickjs` (math, scene, animation, input,
time, audio, camera, ...), `iron_scenes::register_quickjs` (`SceneManager`, `Scene`, `Prefab`),
`iron_physics_api::register_quickjs`, `iron_render_api::register_quickjs`. `iron_native::manifests()` lists them all for the compiler.

**Classes and namespaces in QuickJS** (every `#[iron_api]` module of the engine has the `quickjs` flag; test `M9-API-120` checks
that every function of every manifest exists, `M9-API-144` that every property of every component round-trips):

- *Math (value classes):* `Mathf`, `Vector2/3/4`, `Vector2Int/3Int`, `Quaternion`, `Matrix4x4`, `Rect`, `RectInt`, `Ray`, `Plane`, `Bounds`, `Pose`, `Color`, `Keyframe`, `TimeSpan`, `DateTime`, `TimeSince`, `TimeUntil`.
- *Scene and objects:* `Scene` (helper functions; also the scene class), `GameObject`, `Transform`, `SceneManager`, `Prefab`, `HideFlags`, `LayerMask`.
- *Services:* `Time`, `input`, `Random` / `Rng`, `Debug`, `Log`, `Console`, `PlayerPrefs`, `Assets`, `Application`, `SystemInfo`, `Cursor` / `CursorLockMode`, `Screen`, `QualitySettings`, `Gizmos`.
- *Animation:* `AnimationCurve`, `AnimationClip`, `Animation`, `AnimationState`, `Animations`, `Animator`, `AnimationController` with its parameter / condition / transition / blend tree / node / builder classes.
- *Audio:* `audio` (`Sound`, `Playback`, `Bus`, `Emitter`), `AudioSource`, `AudioListener`.
- *Physics:* `Physics` (`PhysicsBody`, `PhysicsCollider`, `PhysicsJoint`, `RaycastHit`), `Rigidbody`, `BoxCollider`, `SphereCollider`, `CapsuleCollider`, `MeshCollider`, `CharacterController`, `FixedJoint`, `CharacterJoint`, `HingeJoint`, `SpringJoint`, `RopeJoint`, `Rope`, `ForceMode`, `RigidbodyConstraints`, `Physics2D` (`Rigidbody2D`, `BoxCollider2D`, `CircleCollider2D`, `CapsuleCollider2D`, `PolygonCollider2D`, `EdgeCollider2D`, `RaycastHit2D`).
- *Rendering data:* `Camera` (`Cameras`), `Light`, `MeshRenderer`, `SkinnedMeshRenderer`, `SpriteRenderer`, `LineRenderer`, `LineRenderer2D`, `LODGroup`, `Mirror`, `ReflectionProbe`, `PlaneRenderer`, `WorldPanel`, `CameraFollow`, `CameraOrbit`, `ParticleSystem`, `Cloth`, `Material`, `Sprite`, `Texture`, `Mesh`, `Model`, `Primitives` / `PrimitiveType`, `RenderSettings`.

**Callbacks.** A `Callback<(..)>` parameter (listener APIs: `input.onKeyDown(f)`, `Assets.onChanged`, `SceneManager.onSceneLoaded`,
`Screen.onChange`, `ParticleSystem.onStopped`, `Playback.onEnded`, ...) takes a JS function. The engine keeps it as a reference
counted `Function` (`iron_api::qjs::keep_function`), so the collector treats it as a root until the `Callback` is dropped, and
calls it with the same arguments the compiled side gives (numbers, booleans, strings, handle objects); the listener APIs return
the numeric token and `off(token)` removes it. An exception in a callback is not thrown into the engine: it is recorded for
`iron_api::callback::take_errors()` and the other callbacks still run. **Drop the callbacks before the runtime**: QuickJS asserts when
objects are alive at `JS_FreeRuntime`, so reset the engine state that holds listeners (`input::reset`, `assets::reset`,
`app::reset`, `screen::reset`, `audio::reset`, `iron_render_api::reset`, `iron_scenes::reset`) before dropping or replacing a
runtime (the `reset()` of a hot reload included).

**Optional parameters** of value classes are NaN-filled when left out (as in compiled code), optional arrays are empty, optional
handles `null`. Methods of one class with the same script name (`#[js_name]`) are chosen by argument count like namespace functions.

**Hand-over for `iron_app`** (not done here, the area is the lead's):
1. Where the app builds its `QuickJsRuntime`, call `iron_native::install(&mut rt)?` once before loading the bundle (add `iron_native` to
   `crates/iron_app/Cargo.toml`). It replaces calling the per-crate functions.
2. Wrap script execution (load, tick, event callbacks, the hot-reload `__iron_restore_state`) in `iron_scene::api::enter(&mut scene)`, so `GameObject.create()` and friends
   have a current scene, and call `iron_scenes::register_engine_kinds()`, `iron_render_api::register_kinds()`,
   `iron_engine_api::audio_components::register_kinds()` (and the physics kinds) as the compiled host does.
3. Feed the per-thread state each frame as in "The engine API beyond the scene": `time::advance`, `input::*` events,
   `Physics.step()` (or `iron_physics_api::step()` once per fixed tick), `gizmos::take_frame()`, `audio::take_commands()` for the mixer
   and `audio::ended(..)` back, `iron_render_api::take_commands()` / `texture_loaded`, `report_particles`, `assets::changed`, `app::set_focus`,
   `screen::set_size`.
4. On a runtime reset (hot reload) call the `reset()` functions listed under Callbacks first, then `rt.reset()` (the natives are
   installed again by `install`). Handle objects do not survive a reload; the Rust state does.
5. The old `__iron_op("scene.*" / "physics.*" / "audio.*")` paths and these native globals both exist: a game bundle should use the
   native globals **or** the `@iron/engine` classes, not hand ids between them.

- **Engine facade (2026-10-20):** the compiler maps the TypeScript object shapes onto the per-field API (`RenderSettings.skybox = { .. }`, `GameObject` option keys, `Camera.rect` / `layerCullDistances` / `main`, `Screen.windowSize` / `resolutions`, `QualitySettings.names`, `Sprite.grid`, `LODGroup.setLODs`, Cloth / SkinnedMeshRenderer array options, `new Material(color, emission, smoothness)`, the `AnimatorController` builder); see `tools/iron_ts/TODO.md` and `tools/iron_ts/src/lower/facade.rs`. `Camera.main()` is a native static (`CameraRef::main`).
### Native games in `iron_app` (compiled in process)

`iron_app` runs a game two ways. A folder with `dist/game.irbundle` (or a `.irbundle` / `.ts` / `.tsx` path) is a **native
game**: [native.rs](../crates/iron_app/src/native.rs) JIT-compiles the bundled TypeScript with `iron_ts::jit` and drives it with no
QuickJS; otherwise `dist/game.js` runs in QuickJS as before (`App` keeps both sets of fields and branches at the pump points).

- **Bundle** (written by `iron-build`): JSON `{"format":1,"title","entry","source":"<one TS/TSX text>","css":"game.css"|null,
  "assets":[..]}`. `Bundle::read` / `Bundle::parse`. The stylesheet is still `dist/game.css`, loaded by `load_game_css`.
- **Compile.** Manifests: `iron_native::manifests()` (all engine namespaces, which also registers their shim addresses in
  `iron_api::symbol`) plus `iron_ts_ui::manifest_json()` and `root_manifest_json()`; extra symbols: `iron_ts_ui::symbols()`; the
  runtime's come from `iron_ts_rt::symbols()` (a build script lists its `#[no_mangle]` exports). Each reload gets a new
  `struct_base` (100,000 apart).
- **Per frame and event** (`Native`): `fixed_tick` (`NativeHost::fixed_update`, `iron_physics_api::step`, `dispatch_physics`),
  `frame` (`Time` via `iron_engine_api::time::advance`, `ts_rt_timers_advance(now_ms)`, microtasks, `NativeHost::update`),
  `pre_render`, `ui_events` (a `UiEvent` becomes an `iron_ui_native::Event` for `Runtime::dispatch`), and after each of them the
  mounts (`iron_ts_ui::take_mounts`, `UiRoot.mount(target, element)`), `Runtime::flush`, script faults and UI errors to the log.
  Every call runs inside `iron_scene::api::enter(&mut app.scene.scene)`.
- **Console.** `console.log` goes to the `script` log target through `ts_rt_set_console`.
- **Host wiring** (`native.rs`, `native_pump.rs`, `game_input.rs`):
  - *Input.* Keys, mouse, wheel and gamepad events go to `iron_engine_api::input` (`game_input::apply_native`, the same argument lists as `__iron_input`), so `getKey*` and the `onKeyDown` closures work. UI events run through the compiled handlers *inside* the interaction layer's dispatch (`Native::ui_event`, with `Runtime::dispatch_handlers`; world panels go through `native_render::panel_event` the same way), so `preventDefault()` reaches the default actions; `Native::settle` commits afterwards. `iron_ts_ui::take_node_commands()` (`ref.focus()`, `blur()`, `scrollTo()`) is applied to the UI by `App::apply_node_commands`.
  - *Physics.* `fixed_tick` calls `iron_physics_api::step()` (it writes body poses into the scene) and `dispatch_physics`.
  - *Audio.* `AudioCommand`s go to the mixer through `audio_op` (`Load` reads the file, the others `to_op()`); the mixer's finished sounds go back as `audio::ended` (`onEnded`, `AudioSource` state); `AudioSource` / `AudioListener` follow their objects in `pre_render`.
  - *Window and settings.* The window, `Screen`, quality levels and cursor are `native_render.rs`. `Application` requests (target frame rate, run in background, quit) and focus / pause (`onApplicationFocus` / `Pause`, `Application.onFocusChanged`) are applied. `PlayerPrefs` start from the saved prefs and their changes are written back. The manifest's `Application` info, layer names, quality levels and asset GUIDs are set by `native_pump::configure_engine`, before the program's top level, again after a reload.
  - *Scenes and assets.* `SceneManager.loadSceneAsync` requests are read from the game folder (`Native::set_root`), scene events are dispatched after every call into the program, `iron.startScene` loads at start, `Assets.onChanged` runs when the asset watcher reloads files. `register(.., { spawn: false })` is read from the source text (the compiler erases `register`).
  - *Exit.* `App::drop` clears the panel reconcilers (`native_render::clear_state`) and runs `Native::shutdown` (drops the program's closures and resets the engine state) while the program's heap is alive; thread-local destructors run in no fixed order and used to panic at exit.
  - *Still missing:* `onReload`, component references between components across a reload, model completion callbacks.

**Hot reload of a native game** (`Native::reload`, triggered by a change of `dist/game.irbundle` or `dist/.iron-reload`,
`IRON_RELOAD_AT_FRAME=N` in screenshot runs; about 30 ms for hello): the new source is compiled first (a failure, or a
`{"ok":false,"error"}` note from `iron-build dev`, keeps the old game). Then the scenes are saved with `iron_scenes::serialize`
(compiled components through their `@property` tables), the old `NativeHost` is dropped without `onDestroy`, the engine's per-thread
state is reset (`input`, `assets`, `app`, `screen`, `audio`, `animation`, `camera`, `iron_render_api`, `iron_physics_api`,
`iron_scenes`) because it holds the old code's closures, the new program's top level runs, the objects it made are removed and the
saved scenes load again (`load_scene`: components come back on their objects with their fields, `onAwake` runs; classes new in this
program get a starting object). The UI keeps its hook state through `Runtime::refresh`, except components whose `__uiSignatures`
hook list changed. Differences to QuickJS: no `onReload` hook (the compiled runtime has no such hook), non-`@property` fields start over, module variables of objects the
top level made hold removed objects, component references between components are not restored.

**3D from a native game** (`crates/iron_app/src/render_sync.rs`): each frame `sync` takes the resource commands
(`iron_render_api::take_commands`: textures from files or pixels through `WorldViews::texture_op`, then `texture_loaded`; script meshes
through the `mesh` op), clears the renderer's cameras, lights and mesh renderers, and rebuilds them from `iron_engine_api::camera::cameras()`,
`iron_render_api::lights()` and `mesh_renderers()` (with `with_material`) by calling `WorldViews::apply` with the same arguments the
QuickJS `render.*` ops carry, so there is one renderer path. Native objects get script ids with `ScriptScene::adopt`. Cameras look along
-z (as in the QuickJS path, unlike Unity).

Round 2 (render side): `sync` also turns sprite renderers, `LineRenderer2D`, planes, mirrors, reflection probes (sent when new or
changed; `probeRender` on a new render request), LOD groups, skinned mesh renderers (ids from `1 << 24`; bind poses from the mesh or from
the model's skins), line renderers (`pre_render_lines` makes their tube meshes, which are plain mesh renderers), particle systems
(settings as the TypeScript's `[key, value, ...]` pairs when they change, then `take_particle_controls`; `world_view.update_particles`
results go back through `native_render::report_particles` into `report_particles`, so `onStopped` and `stopAction` work), models
(`RenderCommand::Model` -> `WorldViews::model_op` -> `model_loaded`; model meshes and textures keep the host's ids from `1 << 30`, which
`mesh_arg` passes straight through), render textures (`renderTexture` op), gizmos (`take_frame`, re-applied every frame after the cameras),
`RenderSettings` (ambient, sky, bloom, post) and `QualitySettings` (quality, shadow settings) whenever their version counters change.
Mesh renderer ids are the registry handles, so `LODGroup` renderer lists match. A cloth starts when the host first sees it with a mesh
renderer on its object (`rebuild()`), and its `cloth.*` ops go to `ClothSystem` through `Extras`.

The app side is `crates/iron_app/src/native_render.rs` (`impl App`, called from `frame()` through `sync_native_render`):
`CameraFollow` / `CameraOrbit` late updates and `iron_engine_api::animation::update`, the quality levels from `package.json`
(`configure_quality` and `apply_default_quality`, once), `UiResources` requests (`addStyleSheet`, `removeStyleSheet`, `loadFont` through
the same code as the QuickJS ops, on the screen and every panel), `Cursor` (version counter) and `Screen` (window requests, the display
state and size changes), and world panels: `Native::take_panel_mounts` hands over `createRoot(panel).render(..)` roots; each panel has its
own `UiTree` (`panels.rs`) and its own `iron_ui_native::Runtime` (flushed every frame); pointer and key events of a panel
(`App::native_panel_events`) go to that runtime. `WorldPanel` components give the panel's size, density and tint through `Panels::apply`.
Not done: glTF animation clips (`Model.animations`) are not turned into clips; animation events are not delivered to components;
colliders of models (`colliders: "mesh"`) need the physics hook (agent H).

## The FPS counter (`crates/iron_app/src/fps.rs`)

A raw frame-rate overlay drawn by the app for every game, QuickJS (`dist/game.js`) or native (`dist/game.irbundle`): lime green
(`#32ff32`), bold, 13px, `0 1px 2px #000` shadow, no background, in the top right corner 10px from the top and right edges,
right-aligned, text like `144 FPS`. `FpsCounter` counts frames over windows of 0.25 s of real time and shows `round(frames / elapsed)`
(no smoothing, no cap). `FpsOverlay` is a tiny `UiTree` (one absolutely placed box, `pointer-events: none`, so no hit testing or focus)
drawn with `render_overlay` after the world and the game's UI, in `frame()` and in `save_screenshot`, at window resolution and window
scale factor (not the UI scale, not the render scale).

On or off, in order: `--fps` / `--no-fps` on the command line; off with `IRON_FPS=0` (or `false`, `off`, `no`) or `"iron": { "fps": false }`
in `package.json`; otherwise on, except in `--screenshot` runs (off unless `--fps`, so goldens and `bun run screenshots` don't change).
`iron_editor` has it off unless `--fps`. A short screenshot run shows the partial window.

## Fog, light shafts and exposure in native games (2026-10-21)

The render features of `js/engine` that `docs/world-rendering.md` describes (fog, height fog, light shafts, auto exposure, light
units, soft shadow levels, fog volumes) are declared natively too, and `render_sync.rs` sends the renderer the same `render.fog`,
`render.post`, `render.light`, `render.fogVolume`, `render.exposureReset` and `render.quality` ops, so the pictures match.

- **`RenderSettings`** (`iron_render_api::render_settings`): `fog`, `fogMode`, `fogColor`, `fogDensity`, `fogStartDistance`,
  `fogEndDistance`, `fogCulling`; `heightFogEnabled` + `heightFogDensity` / `heightFogBaseHeight` / `heightFogFalloff`;
  `volumetricFogEnabled` + `volumetricFogIntensity` / `Anisotropy` / `Steps` / `Distance` / `Albedo` / `SharpDistance` / `SharpSamples` /
  `Resolution` / `Denoising` / `HistoryWeight` / `Samples` / `MotionSamples`; `autoExposureEnabled` + `autoExposureMinEv` / `MaxEv` /
  `Compensation` / `SpeedDarkToLight` / `SpeedLightToDark` / `Metering` / `LowPercent` / `HighPercent` / `DarkAdaptation` / `DarkBelow` /
  `Hold` + `clearAutoExposureCurve()` / `addAutoExposureCurvePoint(ev, stops)`; `fixedExposure` (`number | null`, an `Option<f64>`),
  `exposureTransition`, `freezeExposure()`. Setting a field of the height fog, light shafts or auto exposure turns it on with the
  defaults for the rest, as the sky does.
- **Object-valued properties** are native records: `skybox`, `bloom`, `heightFog`, `volumetricFog` and `autoExposure` take an object or
  `null` and read back as one (`Option<Record>`, JSON on the wire, see "Records as parameters" below); `fixedExposure` is native
  (`number | null`). `FogMode`, `VolumeDetail`, `VolumeDenoising` and `ExposureMetering` are enums (`#[iron_api]` `enums(..)`, string
  unions in the declarations). The per-field natives (`heightFogDensity`, `skyZenith`, ...) are still there.
- **`Camera`**: `fog` (show the fog, on), `exposure` / `sceneBrightness` (EV100, read back from the GPU a frame or two late, NaN
  before: the app calls `camera::set_exposure`), `resetExposure()`, `aperture` / `shutterSpeed` / `iso` and `physicalExposure`.
- **`Light`**: `volumetricIntensity`, `spotShape` (`cone` | `pyramid`), `aspectRatio`; `Light.illuminanceAt(point, normal?)` and
  `Light.measureIlluminance(point, normal?)` (native, `crates/iron_render_api/src/light_meter.rs`: the sum over every light, the
  shadowed form casting a physics ray to each, so `iron_render_api` depends on `iron_physics_api`; `measureIlluminance` is a
  `#[promise]` that resolves after the next physics step, through `Physics.afterNextStep`, as the TypeScript tier does;
  `M9-API-1645`). **`FogVolume`** is a component (`size`, `density`, `color`,
  `blendDistance`, `shape`).
- **`QualitySettings`** also has `autoExposure`, `volumetricFog`, `hdr`, `additionalLights`, `additionalLightShadows` (per level in
  `project.json`'s `quality.levels`; Low has no light shafts) and `softShadowQuality` `medium`. `RenderStats` (meshes, culled
  meshes, quads, ..., `fogCulled`, `lights`, `shadowMaps`) is filled by the app after each frame; `GraphicsSettings.lightUnits` is
  `physical` when `project.json` says `"lightUnits": "physical"`.
- **Meshes**: `Mesh.createFlat(data)` / `mesh.setFlat(data)` (positions, triangles, normals, uv: typed arrays are copied to
  `number[]` at the boundary; a list left out is empty) over `Mesh.createFlatArrays(..)`. Script meshes
  now flip the texture v coordinate on upload (Unity's bottom-up, as `js/engine`'s `Mesh.set` does). `Random.pick(list)`.
- **Compiler**: an optional object-typed option (`new GameObject(name, { rotation })` with `rotation?: Quaternion`) is skipped when
  missing; `x === false` on an optional boolean is false for a missing value; a typed array is copied where the engine takes
  `number[]`.
- **Scene files** now save and apply the fog keys (`fog`, `fogMode`, `fogColor`, `fogDensity`, `fogStartDistance`,
  `fogEndDistance`, `heightFog`, `volumetricFog`, `fogCulling`); `scenes.ts` saves no exposure keys, so neither do native scenes.

## 2D shapes, text in the world and splines in native games (2026-10-22)

`js/engine`'s `LineRenderer2D` fills, `TextMesh`, `Shapes`, `Spline` / `SplineAnimate` and `Camera.screenToWorldPoint` work in compiled
games too. The renderer-side parts send the same ops as the QuickJS classes (`render.stroke`, `render.text`), so the pictures match.

- **`LineRenderer2D`** (generated, `gen/components.rs`): besides `points`, `width`, `color`, `loop` it has `fillRule`
  (`evenodd` | `nonzero`), `sortingOrder` and `fillColor`. `fillColor` is a native `Option<Rgba>` (`Color | null`; the generator's `color?` field type): a `Color` fills the (closed) shape, `null` takes the fill away. Scene files
  save `fillColor` as a colour or `null`. `render_sync.rs` sends `stroke id object width color loop points fill rule order`; the
  renderer keeps a shape's triangles while its points and rule are unchanged (a native game sends every shape every frame).
- **`TextMesh`** (generated): `text`, `fontSize`, `color`, `font` (`""`: Inter), `fontWeight`, `alignment`, `maxWidth`,
  `pixelsPerUnit`, `resolution`. The host gives each one a panel (`panels.rs::apply_text`, ids from `render_sync::TEXT_BASE`) that
  goes with its component (`native_render::sync_texts`).
- **`Shapes`** (`rectangle`, `roundedRectangle`, `ellipse`, `circle`, `arc`, `pie`, `regularPolygon`, `polygon`, `triangle`,
  `bezier`, `curve`, `closedCurve`) is native Rust now (`iron_math::outlines`, see "Enums, records, lists, rest parameters and
  docs"). **`Spline`** (knots, tangent modes, `evaluate` / `tangent` by distance, `segmentPoint`,
  `sample`, `nearest`, `version`) and **`SplineAnimate`** are native Rust too (see "Enums, records, lists, rest parameters and
  docs"); the TypeScript copies in `facade.ts` are gone.
- **`Camera.screenToWorldPoint(p)`** is a facade method (frustum maths over `rect`, `Screen.width` / `height`, `fieldOfView` or
  `orthographicSize` and the transform). The other `viewportTo...` / `screenPointToRay` members are not there yet.
- **Facade chunk requirements.** A chunk header may end with `@ Name ..`: `//# when Spline @ Vector3`. The chunk is added only
  when the engine API has every name (class, value class, handle or namespace) and the program names a trigger, so a comment that
  mentions `LineRenderer2D` no longer pulls a chunk written over it into programs without that class.
- **Compiler changes this needed** (`tools/iron_ts`): interface and type-literal *method signatures* are function-valued
  properties (`at(t: number): Move;`, optional ones too); named tuple members (`[t: number, run: () => void]`); a method shorthand
  in an object literal (`{ at(t) { .. } }`) is a property whose closure takes its parameter types from the declared
  field; closures inside closures in a **top-level initialiser** now capture the outer closure's locals; errors in top-level
  declarations and object types carry their line; the JSX rewrite keeps the lines of what follows each multi-line element, so
  later error lines are right; `clipPath` is an attribute of `IronProps` (inline SVG `<g clipPath="url(#id)">`).
- **Still not there**: `Partial<Class>` (the examples use a small options interface and set the fields), a string-literal union
  local assigned from an optional field (compiler panic: `Spline` keeps `mode` a `string`), `Array.from({ length }, f)`,
  `.flatMap`, `.flat()`, `Record<string, V>` as a dictionary (use `Map`), `Map.get` over engine handles (box them), object
  accessors (`get x() {}`) in object literals, destructuring assignment, spread call arguments, classes assigned to interfaces
  that have function properties (use an abstract base class), `String(n)[i]` indexing (use `charAt`).

## Enums, records, lists, rest parameters and docs (2026-10-22, generated declarations P0/P1)

Part of making every declaration generated from Rust (plan: [iron-generated-dts-plan.md](research/iron-generated-dts-plan.md)):
`#[iron_api]` and the manifest got what `Shapes`, `Spline` and `SplineAnimate` needed, so their TypeScript copies
(`facade.ts`, `engine_ts.d.ts`) could go. Tests: [records_enums.rs](../crates/iron_api/tests/records_enums.rs)
(`M9-API-1610`..`1614`).

| Rust | Script | C ABI |
|---|---|---|
| `enums(TangentMode = Tangent [Auto, Bezier, Linear])` on the attribute, a Rust enum `Tangent` | the string union `"auto" \| "bezier" \| "linear"` (variant names with a lower-case first letter), plus the global `TangentMode.name(i)` / `TangentMode.parse(s)` | the index, `i32` (-1: not a name; the engine reads it as the first value) |
| `records(SplineHit = SplineHit [segment: f64, u: f64])`, a Rust struct with those public fields (`f64`, `bool`, `string` (a `String`), a declared value class or enum) | a plain object `{ segment, u }` (an `interface` in the d.ts) | a result only: the numbers go to `iron_api`'s per-thread buffer (`return_flat`) and the strings to its string list (`return_strings`, in field order, element after element for a list), the compiled call reads them with `iron_api_result(i)` / `iron_api_result_str(i)` and builds the object |
| `-> Vec<V>` / `-> Vec<Record>` | `V[]` / `Record[]` | the shim returns the element count; the numbers of every element are in the same buffer |
| `-> Vec<String>` | `string[]` | the shim returns the count and keeps the strings in `iron_api`'s per-thread list (`return_strings`); the compiled call copies them out with `iron_api_result_str(i)`, one new script string each (manifest `list:string`; QuickJS gets an array of strings; `Assets.list`, `M9-API-1620`) |
| `#[rest(points)]` on a function whose last parameter is `&[V]` | `...points: V[]` (QuickJS: every remaining argument; compiled: the arguments packed into an array, or one `...array`) | the array pointer, as for `&[V]` |
| `#[options(after = points)]` with `#[optional(points, closed = false, ..)]` | `new Spline(points?, { closed?, tension? })`: the object after `points` gives the parameters after it | the positional shim |
| `like` on the attribute | value *parameters* are declared `Vector3Like` (a plain `{ x, y, z }` does) | - |
| `#[nullable]` on a handle getter and its setter | `T \| null` in the declaration | a zero handle is `null` as before |
| `#[nullable(undefined)]` (on a handle result, a static property such as `Camera.main`) | `T \| undefined` | the same zero handle (manifest `undefined`) |
| `Option<f64>`, `Option<String>` (a result only), `Option<V>` for a value class | `number \| null`, `string \| null`, `Color \| null` (manifest `opt:f64`, `opt:string`, `opt:value:Color`) | the inner type with a none mark: NaN for a number, NaN in every field for a value (also as a parameter, so a setter takes `null`), and for a string an empty text plus the flag `iron_api_result_none()` the compiled call reads right after (`Window.openError`, `RenderSettings.fixedExposure`, `LineRenderer2D.fillColor`); QuickJS gets `null` |
| `enums(extern PrimitiveType = Primitive [..])` | an enum another namespace declares (no second set of converters, no second declaration); the module imports the declaring namespace's `primitive_type_*` helpers | the same index |
| `enums(FullScreenMode = Mode [ExclusiveFullScreen = "ExclusiveFullScreen", ..])` | a variant may name its string when it is not the variant with a lower-case first letter | the index |
| a parameter `Option<Enum>` | the enum, usually optional; a left-out or unknown name is `None` (compiled calls pass -1; QuickJS throws a `TypeError` for an unknown name and takes a left-out one as `None`): setters use it to ignore a name that is not one (`RenderSettings.fogMode = "nonsense"`), constructors to tell "left out" (the `.meta` importer's value) from the first value (`Texture.load(.., { filterMode })`) | the index (-1: none) |
| `-> Vec<Enum>` | `Enum[]` (manifest `list:enum:X`) | the count, and the indexes in the result buffer (`Screen.supportedFullScreenModes`) |
| `#[options(name = "TextureOptions", only)]` with `#[optional(..)]` and `/// key: text` lines in the doc | the options object after the leading required parameters has a declared type: `options?: TextureOptions` and `interface TextureOptions { /** text */ key?: T; .. }` (a handle key also takes `null`; one name over several functions merges their keys); `only` leaves out the positional form of the declaration (the compiler still takes it); QuickJS reads the object by key as for a bare `#[options]` (manifest `optionsName`, `optionsOnly`, parameter `doc`) | the positional shim |
| `&[V]`, `&[f64]`, `&[String]` parameters | declared `readonly V[]` (the engine only reads them; a `readonly` array of a game passes) | the array pointer |
| `aliases(AnimatorController = AnimationController)` | `type AnimatorController = AnimationController; declare const AnimatorController: typeof AnimationController;` for a class, only the `type` for an enum (`PrimitiveMesh = PrimitiveType`); the compiler renames the alias to its target in the source, QuickJS sets a second global | - |
| `#[ts_type = "\"low\" \| \"high\""]` on a `String` getter (and setter) | what the property reads as is that union; a class property is an accessor pair (`get x(): "low" \| "high"; set x(v: string)`: any string may still be written, an unknown name is ignored), a namespace property the union alone | - (manifest JSON `tsType`) |
| `///` on the module, an `impl` block (the class), a function, and inside `enums(..)` / `records(..)` (also on record fields) | JSDoc in the generated `.d.ts` (manifest JSON `doc`) | - |

**Enums on the wire.** A compiled call converts a string constant to its index at compile time and anything else with
`Enum.parse(s)`; a result goes through `Enum.name(i)`. The type name of an enum or record is known to the compiler, so
`let m: TangentMode = "auto"` works (the compiler sees a `string`; `tsc` sees the union). The native helpers for Rust code are
generated next to the shims (`tangent_mode_name`, `_parse`, `_index`, `_from_index`, `TANGENT_MODE_NAMES`). QuickJS passes and
returns the strings and throws a `TypeError` for an unknown name (a left-out optional enum is the first value in both).

**Enums of the engine API (2026-10-22, plan phase 2a).** `FogMode`, `VolumeDetail` (`RenderSettings`), `PrimitiveType` (with the
alias `PrimitiveMesh`; `PrimitiveType.Cube` is the enum value `"cube"`), `FilterMode`, `TextureWrapMode` (`Texture`), `FullScreenMode`
(`Screen`), `ShadowQualityMode` (`QualitySettings`), `WindowStyle` (`Window`) and `LoadSceneMode` (`SceneManager`) are
`#[iron_api]` enums: the string unions in the declarations are generated, `tsc` and the compiler's `Record<FogMode, V>` see the same
names, and a setter ignores a name that is not one. Differences from before: the natives
`Screen.supportedFullScreenModeCount` / `...At(i)`, `RenderSettings.fixedExposureEnabled` / `...Value`, `LineRenderer2D.hasFill` /
`fillValue` and `Window.openErrorText` are gone (the nullable property replaces each pair), `Camera.main` is a property of the
native `Camera` class in QuickJS too (it was a function there), and `Window.create({ style: "floating" })` throws a `TypeError` in
QuickJS (compiled code and the old string API fell back to `normal` with a warning). Tests: `crates/iron_api/tests/enums_nullables.rs`
(`M9-API-1622`..`1626`, `1630`..`1632`), `crates/iron_native/tests/enums.rs` (`M9-API-1627`..`1629`, `1633`), `tools/iron_ts_tests` `M9-TS-3340`, `3341`, `3343`, `tools/iron_ts` `M9-TS-3342`.

**Records as parameters (2026-10-22, plan phase 2b, second part).** A record declared in `records(..)` may have optional keys and number
lists, and `Option<R>` of a record is a parameter or a result (`opt:record:R` in the manifest). On the wire it is **JSON text**: the
shim parses the text with the record's generated `__from_json_R` (a missing required key, a wrong type or an unknown enum name is
reported on stderr and the argument is `None`) and a result is `__to_json_R`; QuickJS stringifies the object it is given (and
builds a real object with value-class instances for a result). The compiled side makes the text from the argument's static type
(`JSON.stringify` of the record's struct, so a game's own structurally matching `interface`, a variable or a spread work); an object
**literal** (also `null` and `c ? { .. } : null`) is written key by key instead, so a key that is written as `null`
(`RenderSettings.skybox = { sunDirection: null }`) differs from one that is left out (a compiled `T | null` field cannot tell
`undefined` from `null`). Record field types: `f64`, `bool`, `string`, a value class (`<Value>Like` when a plain `{ x, y, z }` does),
an enum, `pairs` (`[number, number][]`) and `Option<..>` of those (`Option<Option<T>>`: the key may also be `null`);
`name: T as "minEV"` names a key that is not the camelCase of the field. The declaration is `interface R { key?: T; .. }`. Records that
only have flat fields can still be plain results (the buffer form above); a record with an optional key or a list is JSON only.

**Records and option bags (2026-10-22, plan phase 2b, first part).** `Screen.windowSize`, `currentResolution` and `resolutions` are native records (`WindowSize`,
`Resolution`, `DisplayMode` in Rust), `Animator.getCurrentAnimatorStateInfo()` returns the record `AnimatorStateInfo` (a record with a string field),
`TextureOptions` and `WindowOptions` are generated from `Texture.load` / `create` and `Window.create` (`#[options(name = ..)]`). Natives that only
fed the facade are gone: `Screen.resolutionCount` / `resolutionWidth(i)` / `resolutionHeight(i)` / `resolutionRefreshRate(i)` and
`currentResolutionWidth` / `Height` / `RefreshRate`.

**Lists (2026-10-22, plan phase 2c, first part).** `Vec<f64>` and `Vec<Handle>` are results (`list:f64`, `list:handle:X`: the count, then the numbers in the
result buffer or the 64-bit ids behind `iron_api_result_id(i)`), `&[Handle]` is a parameter (`handles:X`, read with `script_handles`: the
script array of ids). A getter that returns a list pairs with the setter that takes the array (`pair_ty` in the macro), so it is one
property (`SkinnedMeshRenderer.bones`, `Cloth.pins` / `sphereColliders` / `capsuleColliders`, `Camera.layerCullDistances`,
`LineRenderer.positions`, `LineRenderer2D.points`, `EdgeCollider2D.points`, `PolygonCollider2D.points`, `QualitySettings.names`);
`Scene.getRootGameObjects()` and `Sprite.grid(..)` return handle lists. `#[like]` on a function declares its value parameters
`<Value>Like` (a plain `{ x, y }` object does) without making the whole namespace `like`. A compiled call that passes `undefined` for
an optional number, value, handle or array parameter leaves it out. Tests: `M9-API-1634`, `M9-API-1635`, `M9-TS-3350`.

**Subscriptions (2026-10-22, plan phase 2d, first part).** A function that takes one `Callback` and returns its `u32` token can say
`#[subscription(off = "off_change")]` (optionally `ts = "(scene: Scene, mode: LoadSceneMode) => void"` for a callback type the
wire types cannot spell). The C symbol and the manifest keep the token (`"subscription": { "off": "offChange", "ts": ".." }`); scripts
get a function that calls `off_change(token)`: QuickJS builds it in the binding, compiled code gets a small function written from the
manifest by `facade::extend_source` (`__xt_Screen_onChange` over `Screen.onChange$n` and `Screen.offChange`), and the declaration
returns `() => void`. `Screen.onChange`, `QualitySettings.onLevelChanged`, `Assets.onChanged`, `Application.onFocusChanged` and
`SceneManager.onSceneLoaded` / `onSceneUnloaded` / `onActiveSceneChanged` use it; the token forms (`input.onKeyDown`, window
listeners) are unchanged. Tests: `M9-API-1636`, `M9-TS-3351`.

**Promises (2026-10-22, plan phase 2d, second part).** `#[promise]` marks a function whose last two parameters are `resolve: Callback<(T,)>` and
`reject: Callback<(String,)>` (nothing else is returned): scripts call it without them and get a `Promise<T>`, `T` the resolve callback's
argument (`"promise": true` in the manifest; the declaration drops the two parameters). QuickJS makes a real promise and passes the two
functions of `Promise::new` as the callbacks (the reject text becomes an `Error`: `qjs::error_rejecter`); compiled code gets a small
function written from the manifest (`new Promise<T>((resolve, reject) => X.f$n(.., resolve, e => reject(new Error(e))))`). Optional
parameters the generated function passes on become `""` / `NaN` / `false` when left out. Used by `SceneManager.loadSceneAsync`,
`Prefab.load` (listeners the engine adds and removes itself, in `iron_scenes::manager`) and `Texture.whenLoaded()` / `Model.whenLoaded()`
(a waiter list settled by `texture_loaded` / `model_loaded`, so no polling timer). Tests: `M9-API-1637`, `M9-API-1638`, and the
existing `M9-API-1512`..`1517` and `M9-TS` promise tests.

**Lists of records and handles in records (2026-10-22, plan phase 2b/2c).** A record field may be a handle (`MeshRenderer`) or `Vec<Handle>`;
in JSON a handle is its id (a descriptor tag `9` in the compiled JSON blobs, set only when a call of the engine API stringifies or
parses a record: a game's own `JSON.stringify(handle)` is still `{}`; QuickJS walks the object with `qjs::value_to_json`, where a
handle object is its id). `&[R]` parameters and `Vec<R>` results of records that cannot travel flat travel as JSON text
(`records:R`, `ExtTy::RecordsJson`); a getter that returns such a list pairs with the setter that takes it. `#[ts_param(points = "..")]`
sets the declared TypeScript type of a parameter that is wider than the Rust one (`AnimationCurve.fromPoints` takes `Vector2` values
or `[time, value]` pairs: QuickJS reads both, compiled code converts `number[][]` rows). Native now: `LODGroup.setLODs`,
`ParticleSystem.bursts` / `colorOverLifetime` / `sizeOverLifetime` / `shape`. Tests: `M9-API-1639`, `M9-TS-3352`.

**Manifest JSON** (additive): top-level `doc`, `like`, `enums: [{ name, doc, values, nameSymbol, parseSymbol }]`,
`records: [{ name, doc, fields: [{ name, type, doc }] }]`; classes `doc`; functions `doc`, `optionsAfter`, `nullable`, `tsType`;
parameters `rest`; type strings `enum:X`, `record:X`, `list:value:V`, `list:record:R`, `list:string` (a result) and `strings` (a parameter). `tools/iron_ts/src/types_dts.rs` reads them
(`enum` becomes `type X = ..` and `namespace X { name, parse }`, `record` an `interface`).

**Guard tests** (`tools/iron_build/tests/generated_dts.rs`): `M9-TS-3100` no hand-written `.d.ts` file under `tools/` (strict:
`engine_ts.d.ts` and `facade.ts` are gone; what the compiler itself declares is the table `tools/iron_ts/src/builtins.rs`),
`M9-TS-3101` the built-ins declare no engine name again and every class they add members to exists, `M9-TS-3102` the generated
`types/iron.d.ts` equals a blessed API snapshot
(`tools/iron_build/tests/golden/api_surface.txt`; `IRON_BLESS=1 cargo test -p iron_build --test generated_dts` rewrites it, and the
diff shows exactly which declarations changed).

**Shapes** is native: `iron_math::outlines` (`crates/iron_math/src/outlines.rs`, declared as the namespace `Shapes`), ported
from the TypeScript in the same order of operations. Compiled code and QuickJS call the same functions. The compiled facade copy
is gone; `tools/iron_ts_tests/bench/outlines.ts` keeps it, renamed `RefShapes`, as a differential oracle (`M9-TS-3110`: every
point of every shape on edge cases equals the native one bit for bit, in every GC mode). `js/engine/src/shapes.ts` is a typed
alias of the global `Shapes`.

**QuickJS installs the native API.** `iron_app` now calls `iron_native::install` for bundled (JavaScript) games, where it only
called `Window`'s `register_quickjs`, including after a hot reload. The globals it adds (`Shapes`, `Spline`, `TangentMode`, ...)
are the ones compiled games call. `js/engine`'s own classes are still module values; they do not read these globals except the
ones a thin wrapper names (`Shapes`).

**Spline** is native too: `iron_math::spline` (`crates/iron_math/src/spline.rs`, plain maths: knots, `closed`, `tension`, the cached
control points and length table, `locate` / `evaluate` / `tangent` / `sample` / `nearest`) and `iron_engine_api::spline` (the
`Spline` handle class over a per-thread registry; `new Spline(points?, { closed?, tension?, mode? })` is `create` with
`#[options(after = points)]`). The d.ts comes from the manifest: `TangentMode` (an enum), `SplineKnot`, `SplineHit` and
`SplineNearest` (records), `Spline` (class). Differences from the TypeScript class: `knots` is a read-only snapshot (a fresh array of
fresh objects: edit with `setPosition` / `setTangents` / `setMode` / `insertKnot` / `removeKnot`), `version` is read-only, a handle
is not garbage collected (`Spline.destroy()` frees it), a knot index must be a whole number in range (anything else is ignored, as an
out-of-range one was) and a `NaN` `t` gives NaN points. The oracle is the old compiled TypeScript, kept renamed (`RefSpline`) in
`bench/outlines.ts`: `M9-TS-3112` builds splines of 0 to 6 knots (open and closed, three tensions, auto and linear), edits them
with every mutator and compares every number (`count`, `length`, handles, `segmentPoint`, `segmentDerivative`, `locate`,
`evaluate`, `tangent`, `sample`, `nearest`, the knots) bit for bit. In QuickJS `Spline` is the same global class
(`js/engine/src/spline.ts` types it; its bun tests became the QuickJS tests `M9-API-1603`..`1605`).

**SplineAnimate** is a native component (`iron_engine_api::spline`, the `SplineAnimate` handle class, kind `SplineAnimate`). Its
fields (`spline`, `container`, `duration`, `loopMode`, `alignment`, `playing`, `elapsed`) are data in the registry; `t` and `place()` are
methods; `SplineAnimate.attach(go)` / `findOn(go)` are what `go.addComponent(SplineAnimate, { duration: 2 })` and
`go.getComponent(SplineAnimate)` compile to. `loopMode` is the enum `SplineLoopMode` (`"loop" | "pingPong" | "once"`) and `alignment`
`SplineAlignment` (`"none" | "path"`). The host calls `iron_engine_api::spline::update(dt)` once a frame next to
`animation::update` (`native_render.rs::sync_native_render`); it advances every playing component whose object is active in the
hierarchy and puts the object on the path (`place`: the point and the direction from the spline, through the container's
`transformPoint` / `transformDirection` when there is one, then `Quaternion.euler(0, 0, atan2)` on a flat path or `lookRotation`).
Components of destroyed objects go with them (`objects::process_destroyed`).

**Scene files.** The kind saves every field: `loopMode` and `alignment` as names, `container` as an object reference, `spline` as the
inline asset `{"$type":"Spline","closed":..,"tension":..,"knots":[{"position","tangentIn","tangentOut","mode"}]}` (asset kind
`Spline`, `register_kinds()`); loading makes a new spline (`M9-API-1607`). `iron_scenes::register_engine_kinds` registers both.

**QuickJS.** A bundled game's `GameObject` is the op-based one in `js/engine/src/scene.ts`, so the native component cannot be attached
to it. `js/engine/src/spline.ts` keeps `SplineAnimate` as a thin script component that leaves the path maths to three native statics of
the same class: `SplineAnimate.pose(spline, elapsed, duration, loopMode)` (a `{ position, direction }` record),
`SplineAnimate.progress(..)` (`t`) and `SplineAnimate.facing(direction)` (the rotation); the loop, ping-pong and alignment logic
exists once, in Rust (`M9-API-1606`). Its container handling is the script `Transform`'s.

Differences from the TypeScript component: the update runs once a frame after the scripts' `onUpdate`s (in the old version in
component order), so a game that moves the spline in its own `onUpdate` sees the object placed in the same frame; a destroyed or
inactive object is skipped; `SplineAnimate` does not `extends Component` in the declarations (it is a handle class like `Camera`).
Tests: `M9-TS-3113` (`SplineAnimate.pose` equals the old component's `t` and direction for every loop mode, duration and elapsed time
in the oracle), `M9-TS-3115` (added with options, found again, the host update moves the object and pings back), `M9-TS-3116` (the
old `js/engine` test: positions, facing, loop modes, a container), `M9-API-1606`, `M9-API-1607`.
