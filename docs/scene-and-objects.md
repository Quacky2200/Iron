# Scene and objects

Crates and packages: [iron_scene](../crates/iron_scene/src) (Rust),
[iron_script/components.rs](../crates/iron_script/src/components.rs) (component lifecycle),
[js/engine](../js/engine/src) (`@iron/engine`), plus the wiring in
[iron_app/main.rs](../crates/iron_app/src/main.rs). Design background:
[DESIGN.md](../DESIGN.md) section 7.5. Milestone M5.a. How cameras and planes reach the screen is in
[world-rendering.md](world-rendering.md).

## GameObjects in Rust

[lib.rs](../crates/iron_scene/src/lib.rs) has `Scene`, a generational arena of GameObjects. Each has a
name, an enabled flag, a parent, children and a local `Transform` (position, rotation, scale).
There is no ECS: parent and child links plus a dirty flag per object are enough for small scenes.

- `ObjectId` carries a generation, so the id of a destroyed object misses instead of finding whatever
  reused the slot.
- `create`, `destroy` (returns the object and its descendants, parent first), `set_parent(id, parent,
  keep_world)` (returns `false` for a cycle), `set_local`, `set_world`, `set_enabled`, `find`, `walk`
  (depth-first, in order).
- `active(id)` is true when the object and all its ancestors are enabled.
- World transforms are worked out lazily from the hierarchy. `update_world` refreshes the dirty ones
  (the app calls it before drawing); `world_matrix(id)` composes through the parents.
- `take_moved` returns the objects whose world transform changed since the last call, including new
  ones.

Components do not live in the scene. Script components are in `ComponentHost`, physics bodies in
`iron_physics`, cameras, meshes, lights, sprites, particles and panels in the app. They refer to
objects by id. Bodies are tied to their objects by `physics.attach` (see below).

## Script ids and engine ids

Scripts hold numbers, never engine references. [script.rs](../crates/iron_scene/src/script.rs) has
`ScriptScene`, which wraps a `Scene` and maps the script's object ids (`ScriptObject`, a `u32`) to
`ObjectId`s, like UI nodes and physics bodies.

| Range | Who picks the id |
|---|---|
| `1 ..` | Scripts (`new GameObject(...)` counts up from 1) |
| `1 << 30` (`ENGINE_IDS`) and up | The engine, for objects it creates for scripts (`create_engine_object`) |

Component ids split too: `ComponentHost` counts the engine's own up from 0, and scripts allocate
`addComponent` ids from `1 << 24`. Ids go through `f64` in `Args::id`, because `f32`
can't hold integers past 2^24.

`spawn_starting_components` in `main.rs` makes one GameObject per **starting** component class
(registered without `spawn: false`), named after it, holding that component. Games build the rest from
`onStart`, or load a scene file ([Scenes](#scenes)).

## `scene.*` ops

Scripts change the scene with ops. `App::tick` matches the `scene.` prefix and calls
`ScriptScene::apply(name, args)`. A bad op logs `scene.<name>: ...` and changes nothing.

| Op | Arguments | Effect |
|---|---|---|
| `scene.create` | `id name parent\|null [p] [q] [s]` | New object. Reusing an id destroys the old object first. |
| `scene.destroy` | `id` | Destroys the subtree. Unknown ids are ignored. |
| `scene.parent` | `id parent\|null` | Re-parents and keeps the **local** transform. Scripts that keep the world position send `scene.local` after. A cycle is an error. |
| `scene.local` | `id [p] [q] [s]` | Sets the local transform. Missing parts are identity. A zero quaternion is an error. |
| `scene.locals` | flat array, 11 numbers per object | Sets many local transforms: `id px py pz qx qy qz qw sx sy sz` each, local. Unknown or destroyed ids are skipped, the quaternion is normalized, and a degenerate one becomes identity. |
| `scene.enabled` | `id bool` | |
| `scene.name` | `id name` | |
| `scene.layer` | `id 0..31` | Sets the render layer (an error outside 0 to 31). Stored on the object, default 0; `ScriptScene::layer(id)` reads it. Cameras, lights, mirrors and probes filter on it (see [world-rendering.md](world-rendering.md#layers-and-culling)). |
| `scene.addComponent` | `object component class` | Returns `SceneEvent::ComponentAdded`. |
| `scene.removeComponent` | `component` | Returns `SceneEvent::ComponentRemoved`. |

`apply` returns an optional `SceneEvent` for what the rest of the engine must do, and `App::scene_op`
handles it:

- `ComponentAdded` calls `ComponentHost::adopt` (the script already created the instance).
- `ComponentRemoved` calls `ComponentHost::remove` (`onDestroy`, then `__iron_destroy`).
- `Destroyed(objects)` calls `remove_on_objects`, then `WorldViews::forget_objects` and
  `Panels::forget_objects`, so cameras, planes, panels and components on those objects go too.

## The move batch

Scripts keep a mirror of every object's name, parent and local transform, so reads need no round
trip. Only changes the **engine** makes (physics moving a body) go back. `set_world_from_engine`
records the object, and `take_batch` returns a flat array, 11 numbers per object:

```
id, px, py, pz, qx, qy, qz, qw, sx, sy, sz      (local transform)
```

`App::send_scene_moves` calls `__iron_scene(moved)` once per fixed tick, after physics steps. Physics is
the usual source: `Attachments::step` ([physics_sync.rs](../crates/iron_app/src/physics_sync.rs)) calls
`set_world_from_engine` for each attached body that moved. It
skips the call when the batch is empty or the bundle has no `__iron_scene` (probed at startup and
after a reload as `scene_events`). Script-made changes are never echoed back. The JS side updates the
mirror without queueing ops.

The other direction is `ScriptScene::take_script_moved`. Every `scene.local`, `scene.locals`, `scene.parent` and
`scene.create` with a transform records the object in `script_moved`. The call returns those objects and
all their descendants (as script ids, once each) and empties the list. Physics calls it before each step
so attached bodies follow objects that scripts moved
([physics-and-audio.md](physics-and-audio.md#components-on-gameobjects-m5a2)).

## Transform batching

`Transform` setters do not send an op each. They queue the transform in `pendingTransforms`, and
`flushTransforms()` sends all of them in one `scene.locals` op. Every other `scene.*` op calls
`flushTransforms()` first, so op order holds. It is exposed as `__iron_flush`; the runtime calls it at the end
of `run_jobs()` (after `load_bundle`, `eval` and `call_global`). World `position` and `rotation` getters and
setters skip the parent maths when the parent's world transform is identity, and `parentWorld()` is one
recursive `world()` walk.

## Components and the lifecycle

`ComponentHost` keeps `Entry { id, class, object, hooks, started, enabled }`.

| Call | Purpose |
|---|---|
| `spawn_on(rt, class, object, name)` | Creates the instance with `__iron_create(class, id, object, name)`. `spawn` is the same without an object. |
| `adopt(id, class, object)` | Takes over a component a script created with `addComponent`. |
| `remove(rt, id)` | `onDestroy` if it had started and has the hook, then `__iron_destroy`. |
| `remove_on_objects(rt, objects)` | `remove` for every component on destroyed objects. |
| `fixed_update`, `update_active`, `pre_render` | The per-tick and per-frame hooks below. Each takes an `active(object)` closure. |

**Hooks are looked up once.** On a component's first run the host calls `__iron_hooks(id)`, which
returns the names of its lifecycle methods, and stores them. A missing hook costs nothing. If the
bundle has no `__iron_hooks`, every hook is assumed present. Components on inactive objects are
skipped. A component that throws is logged, disabled and destroyed; the others keep running. Hooks run in batches through `__iron_run` ([scripting.md](scripting.md#componenthost-and-ironengine)).

**Order.** `App::tick` runs, per frame:

1. For each fixed 60 Hz tick (at most 5 catch-up ticks): `onStart` for new components, then
   `onFixedUpdate(dt)` for all; `apply_script_output`; `step_physics`; `send_scene_moves`.
2. `ScriptRuntime::tick` (timers and microtasks).
3. `apply_script_output`, then `update_active`: `__iron_tick(dt)` advances script `Time` and returns the
   scaled `dt`; then `onStart` for new components and `onUpdate(dt)` for all; then `__iron_animate(dt)`
   (animation players pose their objects, see [animation-3d.md](animation-3d.md#when-it-runs)); then
   `onLateUpdate(dt)` for all. `dt` is the frame length, capped at `MAX_FRAME_DT` (0.1 s).
4. `apply_script_output`, `update_audio`, `Input::Refresh` for the screen and every panel.
5. Drawing: `pre_render` runs `onPreRender()`, then `apply_script_output`, then
   `__iron_screen(width, height, scale)` if the size changed, then the world and UI are drawn.

`onAwake` is not called by the host's loop. `GameObject.addComponent` calls it right after creating the
instance and setting its props, and `__iron_create` does too (for `spawn_on`), both before any `onStart`.
So components can set up things other components look for right away (a physics component makes its
body there).

`onDestroy` runs when the component or its object is removed. `onStart` always runs before the first
other hook, in whichever step reaches the component first.

`onApplicationPause(paused)` and `onApplicationFocus(focused)` are optional too. `__iron_focus` calls them
on components of active objects when the window loses or gains focus, or the game pauses in the background
(pause first, then focus, as in Unity); see [platform.md](platform.md#focus-and-pausing).

`onReload` is optional and only runs after a hot reload that kept state. It replaces `onAwake` and
`onStart` for script components brought back from the saved scenes, and runs once all objects are
built. Re-attach listeners and callbacks there ([scripting.md](scripting.md#hot-reload)).

## TypeScript API overview

All exported from [js/engine/src/index.ts](../js/engine/src/index.ts). The names follow Unity and
s&box.

**Scene** ([scene.ts](../js/engine/src/scene.ts)).

- `GameObject`: `new GameObject(name, { parent, position, rotation, scale })`, `name`, `layer` (0 to 31,
  range-checked, sends `scene.layer`; see `LayerMask` in [layers.ts](../js/engine/src/layers.ts)), `parent`,
  `children`, `components`, `setParent(parent, keepWorld = true)`, `addComponent(Class, props)`,
  `getComponent`, `removeComponent`, `destroy`, `findChild`. Statics: `find(name)`, `roots`, `byId`,
  `findObjectOfType`, `createPrimitive(type, options)` (an object with the mesh and collider of a built-in
  primitive; [primitives.ts](../js/engine/src/primitives.ts) registers the factory).
- `Transform`: `position`, `rotation`, `scale` (world) and local versions, `forward`, `right`, `up`,
  `eulerAngles`, `translate`, `rotate`, `lookAt`, `transformPoint`, `inverseTransformPoint`,
  `transformDirection`.
- `Component`: `id`, `gameObject`, `transform`, `getComponent`, `destroy`, and the optional hooks
  `onStart`, `onUpdate(dt)`, `onLateUpdate(dt)`, `onFixedUpdate(dt)`, `onPreRender()`, `onDestroy()`.
- Optional hooks `onAwake()` and `onReload()` (above) too.
- `register({ Name: Class }, { spawn })`. Registering names a class for scene files. Unless `spawn` is
  `false`, it is also a starting component (the engine makes one on its own object); `__iron_registered`
  lists only those. Engine classes such as `Camera` are not registered by games: scenes find them by
  class name (`_registerBuiltIns`, called from `scenes.ts`).
- `@property` ([property.ts](../js/engine/src/property.ts)): a standard decorator for fields and
  getter/setter pairs that marks what a component saves with its scene. See [Scenes](#scenes).
- `GameObject` also has `scene`, `root`, `GameObject.dontDestroyOnLoad(go)` and
  `GameObject.instantiate(original, { parent, position, rotation })`.

**Math** ([math.ts](../js/engine/src/math.ts)). Immutable `Vector2`, `Vector3`, `Vector4`,
`Vector2Int`, `Vector3Int`, `Quaternion` (Euler degrees in Unity's order, `lookRotation`, `slerp`),
`Matrix4x4`, `Ray`, `Plane`, `Rect`, `RectInt`, `Bounds`, `Pose`, `Color` (`Color.hex` converts sRGB to
linear), and `Mathf`. Coordinates are right-handed, with -z as `Vector3.forward` for cameras
([world-rendering.md](world-rendering.md#coordinates)).

**Dates and spans** ([time.ts](../js/engine/src/time.ts)). C#-style `DateTime` and `TimeSpan` on JS
`Date`. **Resolution is one millisecond.** There are no ticks, microseconds or nanoseconds, a
`TimeSpan` rounds to whole milliseconds, and `f` format digits past three print as zeros. Don't use
them to profile.

**Time and timers** ([clock.ts](../js/engine/src/clock.ts)). `Time` (`time`, `deltaTime`,
`unscaledDeltaTime`, `fixedDeltaTime`, `frameCount`, `timeScale`, `realtimeSinceStartup`) advances in
`__iron_tick`. `timeScale` scales the `dt` components get; physics is not scaled yet. `TimeSince`
(`now`, `ago`, `seconds`, `over`), `TimeUntil` (`in`, `seconds`, `done`, `fraction`) and awaitable
`Task.delay`, `delaySeconds` and `frame` (resolves at the next tick). `Task.delay` uses `setTimeout`,
so it runs on engine time.

**Logging** ([log.ts](../js/engine/src/log.ts)). `Debug`, `Log` and `Console` all end in `console.*`,
so they reach the app's log sink. Values print readably: vectors via `toString`, errors with their
stack, other objects as JSON (cycles marked). `Log.trace` is `console.debug`.

**Random** ([random.ts](../js/engine/src/random.ts)). `Random` (`value`, `range`, `rangeInt`, `bool`,
`insideUnitCircle`, `insideUnitSphere`, `onUnitSphere`, `rotation`, `initState`) and a seedable `Rng`
(mulberry32, C#'s `Next` and `NextDouble`, `pick`, `shuffle`). Not for security.

**Input** ([input.ts](../js/engine/src/input.ts)). On top of the keyboard and gamepad state
([input-and-events.md](input-and-events.md#gamepads-and-game-input-m3c-step-6)): `getKey`,
`getKeyDown` and `getKeyUp` (true for one tick; key repeats are not presses), `getAxis("Horizontal" |
"Vertical")` (keys and left stick, the larger wins), `getButton` and `getButtonDown` (`Jump`, `Fire1`,
`Fire2`, `Submit`, `Cancel`; `getButtonDown` is keys only for now). The mouse: `mousePosition`
(UI pixels), `mouseDelta`, `mouseScrollDelta`, `getMouseButton`/`Down`/`Up` and the axes `"Mouse X"`,
`"Mouse Y"` and `"Mouse ScrollWheel"` ([details](input-and-events.md#the-mouse-for-gameplay-m5a2)).
Press sets roll over in an `onTick` listener, so they follow `Time`.

**Screen** ([screen.ts](../js/engine/src/screen.ts)). `Screen.width`, `height` (UI pixels, what
pointer events use), `scale` and `aspect`, set by `__iron_screen`. Window mode, resolutions and `setResolution` are in [platform.md](platform.md).

**Application and system** ([app.ts](../js/engine/src/app.ts)). `Application` (`name`, `productName`,
`version`, `companyName`, `identifier`, `engineVersion`, `platform`, `systemLanguage`, `isEditor`,
`isMobilePlatform`, `targetFrameRate`, `runInBackground`, `isFocused`, `isPaused`, `onFocusChanged(fn)`,
`quit()`; the last four are in [platform.md](platform.md#focus-and-pausing)) and `SystemInfo` (OS, CPU, memory, architecture, GPU name, vendor, backend
and driver, `screen` size, refresh rate and DPI). The app builds `globalThis.__iron_info` from the
game's `package.json` (`name`, `version`, and an `iron` section with `title`, `company`, `identifier`
`icon`, a PNG used for the window icon, and `display`, the default window mode and size; the title is the window title) and the machine
([info.rs](../crates/iron_app/src/info.rs)). It evaluates it **before the bundle's top level runs**
(and again for a reloaded runtime). GPU and screen fields are added once the window and renderer exist,
and the script is evaluated again; `prefs` and `display` sections are added too (see [platform.md](platform.md)). `quit()` sends `app.quit`, which sets the app's `done` flag.

**Cursor** ([cursor.ts](../js/engine/src/cursor.ts)). `Cursor.visible`, `lockState`
(`CursorLockMode.None`, `Locked`, `Confined`) and `setCursor(image, hotspot)`; see
[input-and-events.md](input-and-events.md#script-cursor-m5a2).

**Rendering, physics and audio components** are in [world-rendering.md](world-rendering.md),
[assets-and-models.md](assets-and-models.md), [particles.md](particles.md) and
[physics-and-audio.md](physics-and-audio.md#components-on-gameobjects-m5a2): `Camera`, `PlaneRenderer`,
`MeshRenderer`, `Light`, `SpriteRenderer`, `ParticleSystem`, `WorldPanel`, `Rigidbody`, colliders, joints,
2D physics, `AudioSource` and `AudioListener`.

## Scenes

Code: [scenes.ts](../js/engine/src/scenes.ts) and [property.ts](../js/engine/src/property.ts). This is
all TypeScript; Rust only reads files for it (`asset.readText`). Rust's `Scene` is still one flat
world of objects. A "scene" here is a set of root GameObjects that scripts keep together.

**Scenes and roots.** Every root GameObject has `_scene`. Children belong to their root's scene. New
roots go into the **active** scene. A game starts with one empty scene named `Main`.

**File format.** `SceneData` is plain JSON (`format: "iron.scene"`, `version: 1`). Objects nest through
`children`. Ids are unique in the file and are what references point at (components share the
numbering with objects).

```json
{
  "format": "iron.scene", "version": 1, "name": "Level 1",
  "renderSettings": { "ambientLight": { "$type": "Color", "value": [0.1, 0.1, 0.1, 1] } },
  "assets": { "mesh1": { "type": "Mesh", "vertices": [], "triangles": [], "normals": [], "uv": [] } },
  "objects": [{
    "id": 1, "name": "Door", "position": [0, 0, -5], "rotation": [0, 0, 0, 1], "scale": [1, 1, 1],
    "layer": 2, "enabled": false,
    "components": [{ "id": 2, "type": "Door", "properties": {
      "speed": 2, "target": { "$ref": "GameObject", "id": 3 } } }],
    "children": [{ "id": 3, "name": "Handle" }]
  }]
}
```

`layer` (default 0) and `enabled` (default true) are only written when they differ. `model` marks a
model copy (below). `type` is the registered name or an engine class name.

**What is saved.** Each object's name, local transform, layer, enabled flag, children and components.
For a component, only its `@property` values. The engine's own components have `@property` on all
their settings, so a `MeshRenderer` or `Rigidbody` round-trips. Subclasses inherit their parents'
properties (the list lives in the class's decorator metadata).

**Value encoding.** Numbers, strings, booleans, `null`, arrays and plain objects go as they are. The
rest are tagged:

| Value | Saved as |
|---|---|
| `Vector2/3/4`, `Vector2Int/3Int`, `Quaternion`, `Color`, `Rect`, `RectInt` | `{ "$type": "Vector3", "value": [x, y, z] }` |
| A `GameObject` or `Component` in the file | `{ "$ref": "GameObject" or "Component", "id": n }` |
| `Material`, `Sprite`, `Sound` | `{ "$type": ... }` with their fields |
| A `Mesh` from `Mesh.create`, a `Texture` loaded from a file | once in `assets`, used as `{ "$asset": "mesh1" }`, so users share one copy |

Also saved: the active scene's `RenderSettings` (ambient light and mode, reflection intensity, skybox,
bloom), applied when that scene becomes active.

**Load order.** `loadObjects` runs in three passes, like Unity: (1) build every object and add every
component *asleep* (`_addComponentAsleep`, no `onAwake`); (2) set every property, resolving
references through the file ids; (3) call `onAwake` on all components, then apply `enabled: false`.
This lets a component refer to one that comes later in the file, and `onAwake` sees finished values.
A component type that isn't registered is skipped with a warning.

**SceneManager.** Unity's API:

- `loadScene(data, "single" | "additive")` and `loadSceneAsync(dataOrPath, mode)`. `single` (the
  default) unloads every other scene first, except `DontDestroyOnLoad` objects, and makes the new one
  active. Bad data (wrong `format` or `version`) throws; the async version rejects.
- `createScene(name)`, `unloadScene(scene)` (never the last one), `getActiveScene`, `setActiveScene`,
  `sceneCount`, `getSceneAt`, `getSceneByName`, `moveGameObjectToScene(root, scene)`,
  `mergeScenes(source, destination)`.
- `onSceneLoaded(f)`, `onSceneUnloaded(f)`, `onActiveSceneChanged(f)`; each returns a function that
  removes the listener.
- `Scene`: `name`, `handle`, `path`, `isLoaded`, `getRootGameObjects()`, `rootCount`, and
  `serialize()` (returns `SceneData`; nothing writes files yet, so scripts keep or log it).

**Reading files.** `loadSceneAsync("scenes/level.scene.json")` sends `asset.readText id path`. The app
resolves the path inside the game folder (`resolve_asset`, so escaping it is refused), reads it and
answers with `__iron_asset_text([id, text | null, ...])` after the op batch. `null` (missing,
unreadable or outside the folder) rejects the promise. Bundled data
(`import level from "./level.scene.json"`) needs no read.

**DontDestroyOnLoad.** `GameObject.dontDestroyOnLoad(go)` moves the root of `go` to a scene of that
name. A `single` load leaves it alone, and it isn't counted in `sceneCount`.

**Instantiate.** `GameObject.instantiate(original, { parent, position, rotation })` saves the object
with the same code, then loads it again. References between objects inside the copy point at the
copies. References to things outside it stay pointing at the originals (saved as `live` ids, which
only `instantiate` uses). With no parent the copy goes into the original's scene at the original's
**world** placement; with a parent it keeps the local transform. `position` and `rotation` override.

**Prefabs.** `Prefab` (Unity's prefab asset) is one object tree in the scene file's format, kept for
placing copies. `PrefabData` is `{ format: "iron.prefab", version: 1, name, root, assets? }`, where
`root` is a `GameObjectData` and `assets` holds the meshes and textures it shares by id.

- `Prefab.fromObject(go)` captures an object and everything under it as it is now (the same `Saver`
  as scenes, so the same limits). References to objects outside the tree are dropped. The root's own
  position and rotation are removed, so a copy starts from the origin. The data is copied, so later
  changes to the original don't leak in. A model part can't be captured on its own.
- `Prefab.fromData(data)` reads saved data (`toJSON()`'s output, or a bundled `.prefab.json`); other data
  is refused. `Prefab.load(path)` reads a file from the game folder through `asset.readText`, once per
  path.
- `prefab.instantiate({ parent, position, rotation })`, or `GameObject.instantiate(prefab, options)`,
  loads a fresh copy of the tree (like a scene load, into the active scene or under `parent`). References
  inside it point at the copy's objects. A copy from a file remembers its path: `GameObject._prefab` is
  saved as `GameObjectData.prefab`, for tools. Saved scenes still hold the object in full.

**Start scene.** `iron.startScene` in the game's `package.json` reaches scripts as
`__iron_info.app.startScene`. When `scenes.ts` is imported it loads that file additively, makes it
active and drops `Main` if nothing is in it. Starting components still spawn next to it. A failure
is logged as `start scene <path>: ...`.

**Model copies.** An object from `Model.instantiate` has `_model` (`src`, `colliders`) and its
node objects are `_modelPart`. Saving writes `model` and skips the parts. Loading calls
`Model._buildInto`, so the copy is rebuilt from the model file (waiting for the model to arrive, as
`instantiate` does).

**Hide flags.** `GameObject.hideFlags` holds `HideFlags` bits (Unity's values: `HideInHierarchy` 1,
`HideInInspector` 2, `DontSaveInEditor` 4, `NotEditable` 8, `DontSaveInBuild` 16,
`DontUnloadUnusedAsset` 32, and the combinations `DontSave` 52 and `HideAndDontSave` 61). The engine
reads two: scene saving skips an object with any `DontSave` bits (and its children, with no file id for it
or its components), and `Camera.main` skips a `HideInHierarchy` camera. Tools read the rest: the editor's
Hierarchy and Scene view list `visibleObjects()`, which leaves out `HideInHierarchy` objects and what is under them. The editor's own scene camera and frame host are
`HideAndDontSave`. The flags are not saved, and a hot reload restores only saved objects.

**Limits of scenes.**

- Nothing writes files yet (M6 Script IO), so a saved scene comes from `serialize()`'s output.
- Because model copies are rebuilt from the model, objects or components a user added **under a
  model's nodes** are lost when saved. Components on the model's root object are kept. A part can't be
  copied with `instantiate` on its own; copy the root.
- A value the encoder can't handle (a class instance, a texture made from pixels, a model's mesh) is
  saved as `null` with a `scene: ...` warning. Only `Mesh.create` meshes and file textures are kept.
- A reference to an object outside what is saved becomes `null` (except in `instantiate`).
- Hot reload keeps scenes by saving and loading them again; see [scripting.md](scripting.md#hot-reload). It adds `onReload` and plain fields to what is saved.

## Limits

- Hot reload saves every scene before the swap and rebuilds them in the new runtime
  ([scripting.md](scripting.md#hot-reload)). `reload_scripts` still builds a new `ScriptScene` and
  `WorldViews` and drops all panels. Restored script components get `onReload` instead of `onAwake` and
  `onStart`; the bootstrap closure (`spawn_starting_components`) only runs when nothing could be restored.
- Only local transforms cross to scripts, one batch per fixed tick.
- Physics bodies are separate from GameObjects; the `Rigidbody` and collider components link them with
  `physics.attach`, and a body keeps its object's pose only while attached. Scale is not synced to
  bodies.
- `Time.timeScale` doesn't scale physics.

## Tests

`M5-OBJ-01..03` are in [tests/scene.rs](../crates/iron_scene/tests/scene.rs) (hierarchy, world
transforms, moved sets). `M5-SCN-01..04` cover scenes: `js/engine/test/scenes.test.ts` (round trips, `SceneManager`,
`instantiate`) and `M5-SCN-02` in [tests/scripts.rs](../crates/iron_scene/tests/scripts.rs) (save and
load in QuickJS with fixture `tests/fixtures/src/scene.ts`). `M5-PRE-01` (prefabs: capture, copies,
references, JSON, files) is in `scenes.test.ts` too. `examples/scenes` is the manual check.
`M5-OBJ-06..08` are in [tests/scripts.rs](../crates/iron_scene/tests/scripts.rs)
(ops, move batch, lifecycle order, running `tests/fixtures/src/scene.ts` in QuickJS). JS units are
`js/engine/test/{scene,math,time,helpers,camera}.test.ts`. See [testing.md](testing.md).
